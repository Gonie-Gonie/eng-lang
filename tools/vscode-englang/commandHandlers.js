const cp = require("child_process");
const crypto = require("crypto");
const fs = require("fs");
const path = require("path");
const vscode = require("vscode");
const packageManifest = require("./package.json");
const { EXECUTION_PROFILES } = require("./executionProfiles");
const {
  currentWorkspaceRoot,
  engConfig,
  findLspRuntime,
  findRuntime,
  workspaceRoot
} = require("./runtimeDiscovery");
const {
  addSemanticTokenDebugSample,
  semanticTokenDebugSample,
  semanticTokenRange
} = require("./lspSemanticTokens");
const {
  renderReviewSummaryHtml,
  reviewPanelArtifacts
} = require("./reviewPanelRenderer");

const DIAGNOSTICS_MODES = [
  {
    id: "file",
    label: "File diagnostics",
    description: "file",
    detail: "Saved-file Problems diagnostics when an EngLang file opens, saves, or is checked manually."
  },
  {
    id: "live",
    label: "Live diagnostics",
    description: "live",
    detail: "Current-buffer Problems diagnostics after a short typing pause."
  }
];

const DEFAULT_SEMANTIC_TOKEN_SCOPE_MAP = semanticTokenScopeMapFromPackage(packageManifest);

function semanticTokenScopeMapFromPackage(manifest) {
  const rule = (manifest?.contributes?.semanticTokenScopes ?? [])
    .find((entry) => entry?.language === "englang");
  return rule?.scopes ?? {};
}

function createCommandHandlers(options = {}) {
  const output = options.output;
  const reviewCache = options.reviewCache;
  const artifactOpeners = options.artifactOpeners;
  const lspRequests = options.lspRequests;
  const semanticTokenScopeMap = options.semanticTokenScopeMap ?? DEFAULT_SEMANTIC_TOKEN_SCOPE_MAP;
  const semanticTokenTypes = options.semanticTokenTypes ?? [];
  const semanticTokenModifiers = options.semanticTokenModifiers ?? [];
  const isEngDocument = options.isEngDocument ?? (() => true);
  const updateSemanticSymbolDecorations =
    options.updateSemanticSymbolDecorations ?? (() => undefined);

  async function runActiveFile(context) {
    const document = vscode.window.activeTextEditor?.document;
    if (!document || !isEngDocument(document)) {
      vscode.window.showWarningMessage("Open an EngLang .eng file first.");
      return;
    }
    await runDocumentFile(context, document);
  }

  async function runDocumentFile(context, document) {
    if (document.isDirty) {
      await document.save();
    }

    const runtime = findRuntime(context, document);
    const cwd = workspaceRoot(document);
    const profile = executionProfile(document);
    const args = ["run", document.uri.fsPath, "--profile", profile, "--save-artifacts"];
    output.show(true);
    output.appendLine(`run ${document.uri.fsPath} --profile ${profile}`);
    cp.execFile(
      runtime,
      args,
      { cwd, maxBuffer: 10 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (stdout) {
          output.appendLine(stdout.trim());
        }
        if (stderr) {
          output.appendLine(stderr.trim());
        }
        if (error) {
          vscode.window.showErrorMessage(`EngLang run failed in ${profile} profile. See the EngLang output panel.`);
        } else {
          vscode.window.showInformationMessage(`EngLang run completed (${profile}).`);
        }
      }
    );
  }

  async function runExample(context) {
    const root = currentWorkspaceRoot();
    if (!root) {
      vscode.window.showWarningMessage("Open an EngLang workspace first.");
      return;
    }

    const examples = findExampleFiles(root);
    if (examples.length === 0) {
      vscode.window.showWarningMessage("No EngLang examples found under examples/official or examples/workflows.");
      return;
    }

    const picked = await vscode.window.showQuickPick(
      examples.map((example) => ({
        label: example.label,
        description: example.kind,
        detail: example.relativePath,
        path: example.path
      })),
      { placeHolder: "Select an EngLang example to run" }
    );
    if (!picked) {
      return;
    }

    const document = await vscode.workspace.openTextDocument(vscode.Uri.file(picked.path));
    await vscode.window.showTextDocument(document, { preview: false });
    await runDocumentFile(context, document);
  }

  async function switchExecutionProfile() {
    const document = vscode.window.activeTextEditor?.document;
    const current = executionProfile(document);
    const picked = await vscode.window.showQuickPick(
      EXECUTION_PROFILES.map((profile) => ({
        label: profile.id,
        description: profile.description,
        detail: profile.detail,
        profile: profile.id
      })),
      { placeHolder: `Current EngLang execution profile: ${current}` }
    );
    if (!picked) {
      return;
    }

    const target = vscode.workspace.workspaceFolders?.length
      ? vscode.ConfigurationTarget.Workspace
      : vscode.ConfigurationTarget.Global;
    await engConfig(document).update("executionProfile", picked.profile, target);
    vscode.window.showInformationMessage(`EngLang execution profile set to ${picked.profile}.`);
  }

  async function switchDiagnosticsMode() {
    const document = vscode.window.activeTextEditor?.document;
    const current = diagnosticsMode(document);
    const picked = await vscode.window.showQuickPick(
      DIAGNOSTICS_MODES.map((mode) => ({
        label: mode.label,
        description: mode.id === current ? `${mode.description} (current)` : mode.description,
        detail: mode.detail,
        mode: mode.id
      })),
      { placeHolder: `Current EngLang diagnostics mode: ${current}` }
    );
    if (!picked) {
      return;
    }

    const target = vscode.workspace.workspaceFolders?.length
      ? vscode.ConfigurationTarget.Workspace
      : vscode.ConfigurationTarget.Global;
    await engConfig(document).update("diagnosticsMode", picked.mode, target);
    const lintOnChange = engConfig(document).get("lintOnChange", true);
    const suffix = diagnosticsModeChangeSummary(picked.mode, lintOnChange);
    vscode.window.showInformationMessage(`EngLang diagnostics mode set to ${picked.mode}. ${suffix}`);
    return picked.mode;
  }

  async function showToolingStatus(context) {
    const document = toolingStatusDocument();
    const config = engConfig(document);
    const payload = toolingStatusPayload(context, document, config);
    const statusDocument = await vscode.workspace.openTextDocument({
      language: "json",
      content: JSON.stringify(payload, null, 2)
    });
    await vscode.window.showTextDocument(statusDocument, { preview: false });
  }

  async function reviewActiveFile(context) {
    const result = await runReviewForActiveDocument(context);
    if (!result) {
      return;
    }

    const reviewDocument = await vscode.workspace.openTextDocument({
      language: "json",
      content: JSON.stringify(result.review, null, 2)
    });
    await vscode.window.showTextDocument(reviewDocument, { preview: false });
    announceReviewResult(
      result,
      "EngLang review JSON opened.",
      "EngLang review JSON opened with diagnostics. See the EngLang output panel."
    );
  }

  async function openReviewPanel(context) {
    const result = await runReviewForActiveDocument(context);
    if (!result) {
      return;
    }

    const panel = vscode.window.createWebviewPanel(
      "englangReviewPanel",
      "EngLang Review",
      vscode.ViewColumn.Beside,
      {
        enableScripts: true,
        retainContextWhenHidden: true
      }
    );
    panel.webview.html = renderReviewSummaryHtml(
      result.review,
      result.document.uri.fsPath,
      reviewPanelNonce(),
      reviewPanelArtifacts(workspaceRoot(result.document))
    );
    panel.webview.onDidReceiveMessage((message) => {
      if (message?.type === "openSourceLine") {
        openSourceLine(result.document.uri, message.line, message.column).catch((error) => {
          output.appendLine(`Unable to open EngLang source line: ${error.message}`);
        });
      }
      if (message?.type === "openArtifact") {
        artifactOpeners.openLastRunArtifact(message.artifactId, result.document).catch((error) => {
          output.appendLine(`Unable to open EngLang artifact: ${error.message}`);
        });
      }
    });
    announceReviewResult(
      result,
      "EngLang review panel opened.",
      "EngLang review panel opened with diagnostics. See the EngLang output panel."
    );
  }

  async function runReviewForActiveDocument(context) {
    const document = vscode.window.activeTextEditor?.document;
    if (!document || !isEngDocument(document)) {
      vscode.window.showWarningMessage("Open an EngLang .eng file first.");
      return undefined;
    }
    if (document.isDirty) {
      await document.save();
    }

    return runReviewForDocument(context, document);
  }

  function runReviewForDocument(context, document) {
    const runtime = findRuntime(context, document);
    const cwd = workspaceRoot(document);
    output.show(true);
    output.appendLine(`review ${document.uri.fsPath}`);
    return new Promise((resolve) => {
      cp.execFile(
        runtime,
        ["review", document.uri.fsPath, "--json"],
        { cwd, maxBuffer: 20 * 1024 * 1024 },
        (error, stdout, stderr) => {
          if (stderr && stderr.trim().length > 0) {
            output.appendLine(stderr.trim());
          }

          let review;
          try {
            review = JSON.parse(stdout);
          } catch (parseError) {
            output.appendLine(`Unable to parse EngLang review output: ${parseError.message}`);
            if (error) {
              output.appendLine(error.message);
            }
            vscode.window.showErrorMessage("EngLang review failed. See the EngLang output panel.");
            resolve(undefined);
            return;
          }

          reviewCache.set(document.uri.fsPath, review);
          resolve({ document, review, error });
        }
      );
    });
  }

  function announceReviewResult(result, successMessage, warningMessage) {
    if (result.error) {
      output.appendLine(result.error.message);
      vscode.window.showWarningMessage(warningMessage);
      return;
    }
    vscode.window.showInformationMessage(successMessage);
  }

  async function openSourceLine(uri, line, column = 1) {
    const lineNumber = Number(line);
    if (!Number.isFinite(lineNumber) || lineNumber < 1) {
      return;
    }
    const document = await vscode.workspace.openTextDocument(uri);
    const editor = await vscode.window.showTextDocument(document, {
      preview: false,
      viewColumn: vscode.ViewColumn.One
    });
    const targetLine = Math.min(Math.max(0, Math.trunc(lineNumber) - 1), document.lineCount - 1);
    const textLine = document.lineAt(targetLine);
    const targetCharacter = sourceColumnCharacter(textLine.text, column);
    const position = new vscode.Position(targetLine, targetCharacter);
    const range = new vscode.Range(
      targetLine,
      targetCharacter,
      targetLine,
      Math.max(targetCharacter, textLine.text.length)
    );
    editor.selection = targetCharacter > 0
      ? new vscode.Selection(position, range.end)
      : new vscode.Selection(position, position);
    editor.revealRange(range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
  }

  function sourceColumnCharacter(lineText, column) {
    const columnNumber = Number(column);
    if (!Number.isFinite(columnNumber) || columnNumber <= 1) {
      return 0;
    }
    const targetByte = Math.max(0, Math.trunc(columnNumber) - 1);
    const text = String(lineText || "");
    let byteOffset = 0;
    let characterOffset = 0;
    for (const character of text) {
      const characterBytes = Buffer.byteLength(character, "utf8");
      if (byteOffset + characterBytes > targetByte) {
        break;
      }
      byteOffset += characterBytes;
      characterOffset += character.length;
    }
    return Math.min(characterOffset, text.length);
  }

  function reviewPanelNonce() {
    return crypto.randomBytes(16).toString("base64");
  }

  async function showSemanticTokensDebug(context) {
    const document = vscode.window.activeTextEditor?.document;
    if (!document || !isEngDocument(document)) {
      vscode.window.showWarningMessage("Open an EngLang .eng file first.");
      return;
    }
    const snapshot = await lspRequests.snapshotDocumentSource(document, context);
    if (!snapshot) {
      await showHighlightUnavailableWarning(context, document);
      return;
    }
    reviewCache.set(document.uri.fsPath, snapshot);
    updateSemanticSymbolDecorations(document, snapshot);
    const semanticTokens = snapshot.semantic_tokens ?? { legend: {}, tokens: [] };
    const tokenCounts = {};
    const modifierCounts = {};
    const selectorCounts = {};
    const tokenSamplesByType = {};
    const tokenSamplesByModifier = {};
    const tokenSamplesBySelector = {};
    const missingScopeSelectors = {};
    const unmappedSelectorCounts = {};
    let tokensWithoutFallbackScope = 0;
    let tokensWithUnmappedSelectors = 0;
    for (const token of semanticTokens.tokens ?? []) {
      tokenCounts[token.type] = (tokenCounts[token.type] ?? 0) + 1;
      const sample = semanticTokenDebugSample(document, token, semanticTokenScopeMap);
      for (const selector of sample.semantic_selectors ?? []) {
        selectorCounts[selector] = (selectorCounts[selector] ?? 0) + 1;
        addSemanticTokenDebugSample(tokenSamplesBySelector, selector || "-", sample);
      }
      const unmappedSelectors = sample.unmapped_semantic_selectors ?? [];
      if (unmappedSelectors.length > 0) {
        tokensWithUnmappedSelectors += 1;
        for (const selector of unmappedSelectors) {
          unmappedSelectorCounts[selector] = (unmappedSelectorCounts[selector] ?? 0) + 1;
        }
      }
      if ((sample.fallback_scopes ?? []).length === 0) {
        tokensWithoutFallbackScope += 1;
        for (const selector of sample.semantic_selectors ?? []) {
          missingScopeSelectors[selector] = (missingScopeSelectors[selector] ?? 0) + 1;
        }
      }
      addSemanticTokenDebugSample(tokenSamplesByType, token.type || "-", sample);
      for (const modifier of token.modifiers ?? []) {
        modifierCounts[modifier] = (modifierCounts[modifier] ?? 0) + 1;
        addSemanticTokenDebugSample(tokenSamplesByModifier, modifier || "-", sample);
      }
    }
    const tokenRows = (semanticTokens.tokens ?? [])
      .map((token) => semanticTokenDebugRow(document, token, semanticTokenScopeMap));
    const rangeOverlaps = semanticTokenRangeOverlaps(document, tokenRows);
    const tokenCount = semanticTokens.tokens?.length ?? 0;
    const scopeMapStatus = semanticScopeMapStatus(
      semanticTokenScopeMap,
      semanticTokenTypes,
      semanticTokenModifiers
    );
    const payload = {
      source: document.uri.fsPath,
      semantic_highlighting_enabled: engConfig(document).get("semanticHighlighting.enabled", true),
      summary: {
        status: highlightInspectionStatus(tokenCount, tokensWithoutFallbackScope, tokensWithUnmappedSelectors),
        fallback_scope_status: highlightFallbackStatus(tokenCount, tokensWithoutFallbackScope),
        direct_selector_status: highlightDirectSelectorStatus(tokenCount, tokensWithUnmappedSelectors),
        token_count: tokenCount,
        scope_map_status: scopeMapStatus.status,
        counts_by_type: tokenCounts,
        counts_by_modifier: modifierCounts,
        counts_by_selector: selectorCounts,
        range_overlap_count: rangeOverlaps.length,
        scope_map_entry_count: Object.keys(semanticTokenScopeMap).length,
        tokens_without_fallback_scope: tokensWithoutFallbackScope,
        tokens_with_unmapped_selectors: tokensWithUnmappedSelectors,
        missing_scope_selectors: missingScopeSelectors,
        unmapped_selector_counts: unmappedSelectorCounts
      },
      semantic_scope_map: scopeMapStatus,
      legend: semanticTokens.legend ?? {},
      samples: {
        by_type: tokenSamplesByType,
        by_modifier: tokenSamplesByModifier,
        by_selector: tokenSamplesBySelector
      },
      tokens: tokenRows,
      range_overlaps: rangeOverlaps,
      raw: {
        semantic_tokens: semanticTokens
      },
      highlight_count: tokenCount,
      highlight_range_overlap_count: rangeOverlaps.length,
      highlight_range_overlaps: rangeOverlaps,
      highlight_counts_by_category: tokenCounts,
      highlight_counts_by_detail: modifierCounts,
      highlight_counts_by_selector: selectorCounts,
      highlight_samples_by_category: tokenSamplesByType,
      highlight_samples_by_detail: tokenSamplesByModifier,
      highlight_samples_by_selector: tokenSamplesBySelector,
      token_count: tokenCount,
      token_counts_by_type: tokenCounts,
      token_counts_by_modifier: modifierCounts,
      token_counts_by_selector: selectorCounts,
      token_samples_by_type: tokenSamplesByType,
      token_samples_by_modifier: tokenSamplesByModifier,
      token_samples_by_selector: tokenSamplesBySelector,
      highlight_data: semanticTokens,
      semantic_tokens: semanticTokens
    };
    const debugDocument = await vscode.workspace.openTextDocument({
      language: "json",
      content: JSON.stringify(payload, null, 2)
    });
    await vscode.window.showTextDocument(debugDocument, { preview: false });
  }


  async function showHighlightUnavailableWarning(context, document) {
    const semanticHighlighting = engConfig(document).get("semanticHighlighting.enabled", true);
    const settingState = semanticHighlighting ? "enabled" : "disabled";
    const message = `No highlight data is available. Semantic highlighting is ${settingState}; run EngLang: Show Tooling Status to confirm the live editor tool path.`;
    output.appendLine(`highlight data unavailable: semanticHighlighting.enabled=${semanticHighlighting}; use EngLang: Show Tooling Status to inspect the live editor tool path.`);
    const picked = await vscode.window.showWarningMessage(message, "Show Tooling Status");
    if (picked === "Show Tooling Status") {
      await showToolingStatus(context);
    }
  }

  async function showSemanticTokenAtCursor(context) {
    const editor = vscode.window.activeTextEditor;
    const document = editor?.document;
    if (!editor || !document || !isEngDocument(document)) {
      vscode.window.showWarningMessage("Open an EngLang .eng file first.");
      return;
    }
    const snapshot = await lspRequests.snapshotDocumentSource(document, context);
    if (!snapshot) {
      await showHighlightUnavailableWarning(context, document);
      return;
    }
    reviewCache.set(document.uri.fsPath, snapshot);
    updateSemanticSymbolDecorations(document, snapshot);
    const semanticTokens = snapshot.semantic_tokens ?? { legend: {}, tokens: [] };
    const scopeMapStatus = semanticScopeMapStatus(
      semanticTokenScopeMap,
      semanticTokenTypes,
      semanticTokenModifiers
    );
    const cursor = editor.selection.active;
    const matchingTokens = (semanticTokens.tokens ?? [])
      .filter((token) => semanticTokenRange(document, token)?.contains(cursor))
      .map((token) => semanticTokenDebugRow(document, token, semanticTokenScopeMap));
    const lineTokens = (semanticTokens.tokens ?? [])
      .filter((token) => Number(token.line) === cursor.line)
      .map((token) => semanticTokenDebugRow(document, token, semanticTokenScopeMap));
    const lineRangeOverlaps = semanticTokenRangeOverlaps(document, lineTokens);
    const nearestTokens = lineTokens
      .map((token) => ({
        ...token,
        cursor_distance: semanticTokenCursorDistance(token, cursor.character)
      }))
      .sort((left, right) =>
        left.cursor_distance - right.cursor_distance || Number(left.start ?? 0) - Number(right.start ?? 0)
      )
      .slice(0, 3);
    const cursorTokenHint = matchingTokens.length > 0
      ? "matching_token"
      : nearestTokens.length > 0
        ? "nearest_token_on_line"
        : "no_semantic_tokens_on_line";
    const payload = {
      source: document.uri.fsPath,
      semantic_highlighting_enabled: engConfig(document).get("semanticHighlighting.enabled", true),
      cursor: {
        line: cursor.line + 1,
        column: cursor.character + 1,
        zero_based_line: cursor.line,
        zero_based_character: cursor.character
      },
      summary: {
        status: cursorHighlightStatus(matchingTokens, nearestTokens),
        matching_token_count: matchingTokens.length,
        nearest_token_count: nearestTokens.length,
        line_token_count: lineTokens.length,
        line_range_overlap_count: lineRangeOverlaps.length,
        cursor_token_hint: cursorTokenHint,
        scope_map_status: scopeMapStatus.status,
        scope_map_entry_count: Object.keys(semanticTokenScopeMap).length,
        copy_ready: semanticTokenCopyReady(matchingTokens[0] ?? nearestTokens[0] ?? null)
      },
      matching_tokens: matchingTokens,
      nearest_tokens: nearestTokens,
      line_tokens: lineTokens,
      line_range_overlaps: lineRangeOverlaps,
      semantic_scope_map: scopeMapStatus,
      legend: semanticTokens.legend ?? {},
      raw: {
        semantic_tokens: semanticTokens
      }
    };
    const debugDocument = await vscode.workspace.openTextDocument({
      language: "json",
      content: JSON.stringify(payload, null, 2)
    });
    await vscode.window.showTextDocument(debugDocument, { preview: false });
  }

  function highlightInspectionStatus(tokenCount, tokensWithoutFallbackScope, tokensWithUnmappedSelectors) {
    if (tokenCount === 0) {
      return "No role-aware highlight tokens were returned for this file.";
    }
    const tokenLabel = `${tokenCount} role-aware highlight token${tokenCount === 1 ? "" : "s"}`;
    const issues = [];
    if (tokensWithoutFallbackScope > 0) {
      issues.push(`${tokensWithoutFallbackScope} token${tokensWithoutFallbackScope === 1 ? "" : "s"} need theme fallback scope coverage`);
    }
    if (tokensWithUnmappedSelectors > 0) {
      issues.push(`${tokensWithUnmappedSelectors} token${tokensWithUnmappedSelectors === 1 ? "" : "s"} need direct selector mapping`);
    }
    if (issues.length > 0) {
      return `${tokenLabel} returned; ${issues.join(" and ")}.`;
    }
    return `${tokenLabel} returned with theme fallback scope coverage and direct selector mappings.`;
  }

  function highlightFallbackStatus(tokenCount, tokensWithoutFallbackScope) {
    if (tokenCount === 0) {
      return "no_tokens";
    }
    return tokensWithoutFallbackScope > 0 ? "missing_fallback_scopes" : "mapped";
  }

  function highlightDirectSelectorStatus(tokenCount, tokensWithUnmappedSelectors) {
    if (tokenCount === 0) {
      return "no_tokens";
    }
    return tokensWithUnmappedSelectors > 0 ? "missing_direct_selector_scopes" : "mapped";
  }

  function cursorHighlightStatus(matchingTokens, nearestTokens) {
    if (matchingTokens.length > 0) {
      return `Caret is inside ${matchingTokens.length} role-aware highlight token${matchingTokens.length === 1 ? "" : "s"}.`;
    }
    if (nearestTokens.length > 0) {
      return "No highlight token covers the caret; nearest tokens on this line are listed.";
    }
    return "No role-aware highlight tokens were returned for the current line.";
  }

  function semanticTokenCursorDistance(row, character) {
    const start = Number(row?.start);
    const length = Number(row?.length);
    if (!Number.isFinite(start) || !Number.isFinite(length) || length <= 0) {
      return Number.MAX_SAFE_INTEGER;
    }
    const end = start + length;
    if (character >= start && character < end) {
      return 0;
    }
    return character < start ? start - character : character - end;
  }

  function semanticTokenDebugRow(document, token, semanticScopeMap) {
    const sample = semanticTokenDebugSample(document, token, semanticScopeMap);
    const start = Number(sample.start);
    const length = Number(sample.length);
    const line = Number(sample.line);
    const column = Number.isFinite(start) ? start + 1 : null;
    const semanticSelectors = sample.semantic_selectors ?? [];
    const fallbackScopes = sample.fallback_scopes ?? [];
    const unmappedSelectors = sample.unmapped_semantic_selectors ?? [];
    const rangeText = semanticTokenRangeText(line, column, length);
    const primarySelector = semanticSelectors[0] ?? sample.type;
    const inspectorPanels = semanticTokenInspectorPanels(sample, semanticSelectors);
    return {
      line: sample.line,
      column,
      start: sample.start,
      length: sample.length,
      end: Number.isFinite(start) && Number.isFinite(length) ? start + length : null,
      range_text: rangeText,
      text: sample.text,
      type: sample.type,
      modifiers: sample.modifiers,
      primary_selector: primarySelector,
      fallback_status: fallbackScopes.length > 0 ? "mapped" : "missing_fallback_scope",
      direct_selector_status: unmappedSelectors.length > 0 ? "missing_direct_scope" : "mapped",
      fallback_scope_count: fallbackScopes.length,
      semantic_selectors: semanticSelectors,
      unmapped_semantic_selectors: unmappedSelectors,
      fallback_scopes: fallbackScopes,
      inspector_panels: inspectorPanels,
      panel_hint: inspectorPanels.length > 0 ? inspectorPanels.join(", ") : null,
      copy_text: sample.text,
      copy_range: rangeText,
      copy_selector: primarySelector
    };
  }

  function semanticTokenRangeOverlaps(document, rows) {
    const tokensByLine = new Map();
    for (const row of Array.isArray(rows) ? rows : []) {
      const line = Number(row?.line);
      if (!Number.isFinite(line) || line < 1) {
        continue;
      }
      if (!tokensByLine.has(line)) {
        tokensByLine.set(line, []);
      }
      tokensByLine.get(line).push(row);
    }

    const overlaps = [];
    for (const [line, lineRows] of tokensByLine.entries()) {
      const textLine = line <= document.lineCount ? document.lineAt(line - 1).text : "";
      const ranges = lineRows
        .map((row) => {
          const start = Number(row?.start);
          const length = Number(row?.length);
          const rowEnd = Number(row?.end);
          const end = Number.isFinite(rowEnd) ? rowEnd : start + length;
          if (!Number.isFinite(start) || !Number.isFinite(length) || !Number.isFinite(end) || length <= 0 || end <= start) {
            return null;
          }
          return { row, start, end };
        })
        .filter(Boolean)
        .sort((left, right) => left.start - right.start || left.end - right.end);
      let previous = null;
      for (const range of ranges) {
        if (previous && range.start < previous.end) {
          const start = Math.max(previous.start, range.start);
          const end = Math.min(previous.end, range.end);
          overlaps.push({
            line,
            column: start + 1,
            start,
            length: end - start,
            end,
            range_text: semanticTokenRangeText(line, start + 1, end - start),
            text: textLine.slice(start, end),
            left: semanticTokenOverlapSide(previous.row),
            right: semanticTokenOverlapSide(range.row)
          });
          if (range.end > previous.end) {
            previous = range;
          }
          continue;
        }
        previous = range;
      }
    }
    return overlaps;
  }

  function semanticTokenOverlapSide(row) {
    return {
      line: row?.line ?? null,
      column: row?.column ?? null,
      start: row?.start ?? null,
      length: row?.length ?? null,
      end: row?.end ?? null,
      range_text: row?.range_text ?? null,
      text: row?.text ?? "",
      type: row?.type ?? "-",
      modifiers: row?.modifiers ?? [],
      primary_selector: row?.primary_selector ?? row?.type ?? "-",
      semantic_selectors: row?.semantic_selectors ?? [],
      fallback_status: row?.fallback_status ?? "-",
      direct_selector_status: row?.direct_selector_status ?? "-",
      fallback_scope_count: row?.fallback_scope_count ?? 0,
      inspector_panels: row?.inspector_panels ?? [],
      panel_hint: row?.panel_hint ?? null,
      copy_text: row?.copy_text ?? row?.text ?? "",
      copy_range: row?.copy_range ?? row?.range_text ?? null,
      copy_selector: row?.copy_selector ?? row?.primary_selector ?? row?.type ?? "-"
    };
  }

  function semanticTokenRangeText(line, column, length) {
    if (!Number.isFinite(line) || !Number.isFinite(column) || !Number.isFinite(length) || length <= 0) {
      return null;
    }
    return `L${line}:C${column}-C${column + length}`;
  }

  function semanticTokenCopyReady(row) {
    if (!row) {
      return null;
    }
    return {
      text: row.copy_text ?? row.text ?? "",
      range: row.copy_range ?? row.range_text ?? null,
      selector: row.copy_selector ?? row.primary_selector ?? row.type ?? "-",
      inspector_panels: row.inspector_panels ?? [],
      panel_hint: row.panel_hint ?? null,
      fallback_status: row.fallback_status ?? "-",
      direct_selector_status: row.direct_selector_status ?? "-"
    };
  }

  function semanticTokenInspectorPanels(row, semanticSelectors = []) {
    const modifiers = Array.isArray(row?.modifiers) ? row.modifiers.map((modifier) => String(modifier)) : [];
    const modifierText = modifiers.join(" ").toLowerCase();
    const tokenType = String(row?.type || "");
    const detailText = [
      row?.text,
      tokenType,
      modifierText,
      ...semanticSelectors
    ].map((value) => String(value || "").toLowerCase()).join(" ");
    const panels = [];
    const add = (panel) => {
      if (!panels.includes(panel)) panels.push(panel);
    };

    if (detailText.includes("schema")) add("schema");
    if (modifiers.includes("timeseries") || modifiers.includes("axis") || detailText.includes("timeseries") || detailText.includes("time axis")) add("time");
    if (modifiers.includes("validation")) add("checks");
    if (modifiers.includes("workflowStep")) add("workflow");
    if (modifiers.includes("workflowStep") && /case|materialize|collect|apply/.test(detailText)) add("case");
    if (modifiers.includes("sideEffect")) add("effects");
    if (modifiers.includes("external")) {
      add("effects");
      if (/http|network|cache|response|download|url/.test(detailText)) add("network");
    }
    if (modifiers.includes("cache") || /cache|cache_key|cachekey|offline_response/.test(detailText)) add("network");
    if (tokenType === "namespace" || modifiers.includes("defaultLibrary") || modifiers.includes("imported") || modifiers.includes("internal") || modifiers.includes("planned") || /\beng\./.test(detailText)) add("modules");
    if (modifiers.includes("db") || /sqlite|database|db_/.test(detailText)) add("db");
    if (modifiers.includes("model") || detailText.includes("model") || detailText.includes("prediction")) add("model");
    if (modifiers.includes("report") || /report|plot|artifact/.test(detailText)) add("review");
    if (modifiers.includes("unit") || modifiers.includes("quantity")) add("units");
    if (["variable", "parameter", "property"].includes(tokenType)) add("variables");

    return panels.slice(0, 4);
  }

  function findExampleFiles(root) {
    const groups = [
      { kind: "official", dir: path.join(root, "examples", "official") },
      { kind: "workflow", dir: path.join(root, "examples", "workflows") }
    ];
    const examples = [];
    for (const group of groups) {
      collectExampleMainFiles(group.dir, root, group.kind, examples);
    }
    return examples.sort((left, right) => left.relativePath.localeCompare(right.relativePath));
  }

  function collectExampleMainFiles(dir, root, kind, examples) {
    let entries;
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {
      return;
    }

    for (const entry of entries) {
      const entryPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        if (!entry.name.startsWith(".") && entry.name !== "build" && entry.name !== "target") {
          collectExampleMainFiles(entryPath, root, kind, examples);
        }
        continue;
      }
      if (!entry.isFile() || entry.name !== "main.eng") {
        continue;
      }
      const relativePath = path.relative(root, entryPath).replace(/[\\/]/g, "/");
      examples.push({
        kind,
        path: entryPath,
        relativePath,
        label: relativePath.replace(/^examples\//, "").replace(/\/main\.eng$/, "")
      });
    }
  }

  function executionProfile(document) {
    const configured = engConfig(document).get("executionProfile", "normal");
    return EXECUTION_PROFILES.some((profile) => profile.id === configured)
      ? configured
      : "normal";
  }

  function diagnosticsMode(document) {
    const config = engConfig(document);
    const configuredMode = explicitlyConfiguredEngValue(config, "diagnosticsMode");
    if (configuredMode === "file" || configuredMode === "live") {
      return configuredMode;
    }
    const legacySource = explicitlyConfiguredEngValue(config, "problemsSource");
    if (legacySource === "file" || legacySource === "live") {
      return legacySource;
    }
    const legacyBackend = config.get("diagnosticsBackend", "eng-cli");
    return legacyBackend === "lsp-snapshot" ? "live" : "file";
  }

  function explicitlyConfiguredEngValue(config, key) {
    const inspection = config.inspect(key);
    if (!inspection) {
      return undefined;
    }
    for (const scope of [
      "workspaceFolderValue",
      "workspaceValue",
      "globalValue"
    ]) {
      const value = inspection[scope];
      if (value !== undefined) {
        return value;
      }
    }
    return undefined;
  }

  function toolingStatusDocument() {
    const document = vscode.window.activeTextEditor?.document;
    if (document?.uri?.scheme === "file") {
      return document;
    }
    const root = currentWorkspaceRoot();
    if (!root) {
      return undefined;
    }
    const probePath = path.join(root, "workspace.eng");
    return {
      uri: vscode.Uri.file(probePath),
      fileName: probePath
    };
  }

  function toolingStatusPayload(context, document, config) {
    const runtime = document ? findRuntime(context, document) : "eng.exe";
    const lsp = document ? findLspRuntime(context, document) : "eng-lsp.exe";
    const checkAndRunTool = executableStatus(runtime, config.get("runtimePath", ""));
    const liveEditorTool = executableStatus(lsp, config.get("lspPath", ""));
    const mode = diagnosticsMode(document);
    const problemsSource = mode;
    const lintOnChange = config.get("lintOnChange", true);
    const semanticHighlighting = config.get("semanticHighlighting.enabled", true);
    const diagnosticsSummary = diagnosticsStatusSummary(problemsSource, lintOnChange);
    const roleAwareColorSummary = semanticHighlighting
      ? "Compiler-backed role-aware colors are enabled for the current editor."
      : "Compiler-backed role-aware colors are disabled; VS Code will use TextMate syntax colors only.";
    const scopeMapStatus = semanticScopeMapStatus(
      semanticTokenScopeMap,
      semanticTokenTypes,
      semanticTokenModifiers
    );
    const highlightingSummary = highlightingStatusSummary(semanticHighlighting, scopeMapStatus);
    return {
      summary: {
        check_and_run_tool: toolStatusSummary(checkAndRunTool, "saved-file checks and program runs"),
        live_editor_tool: toolStatusSummary(liveEditorTool, "live editor checks"),
        diagnostics: diagnosticsSummary,
        role_aware_colors: roleAwareColorSummary,
        highlighting: highlightingSummary
      },
      extension: {
        id: "englang.englang",
        version: context.extension?.packageJSON?.version ?? "unknown",
        path: context.extensionPath
      },
      workspace: {
        root: document ? workspaceRoot(document) : currentWorkspaceRoot() ?? null,
        active_document: vscode.window.activeTextEditor?.document?.uri?.fsPath ?? null
      },
      tools: {
        check_and_run: checkAndRunTool,
        live_editor: liveEditorTool
      },
      executables: {
        eng: checkAndRunTool,
        eng_lsp: liveEditorTool
      },
      editor_client: {
        request_model: "on-demand live editor checks",
        long_running_language_server: false,
        live_buffer_tool: "live_editor",
        file_check_tool: "check_and_run",
        highlighting_model: "TextMate first paint plus compiler-backed semantic token refinement",
        status_note: "Live editor features read the current buffer for hover, completion, symbols, highlights, formatting, quick fixes, and live Problems updates."
      },
      features: {
        problems: {
          source: problemsSource,
          source_label: diagnosticsProblemsSource(mode),
          mode: problemsSource === "live" ? "live buffer" : "saved file",
          summary: diagnosticsSummary,
          updates_while_typing: problemsSource === "live" && lintOnChange,
          tool: problemsSource === "live" ? "live_editor" : "check_and_run"
        },
        hover: liveEditorFeature("live_editor"),
        completion: liveEditorFeature("live_editor"),
        definition: liveEditorFeature("live_editor"),
        document_symbols: liveEditorFeature("live_editor"),
        workspace_symbols: liveEditorFeature("live_editor"),
        folding: liveEditorFeature("live_editor"),
        formatting: liveEditorFeature("live_editor"),
        quick_fixes: liveEditorFeature("live_editor"),
        role_aware_colors: {
          enabled: semanticHighlighting,
          summary: roleAwareColorSummary,
          tool: "live_editor",
          request_model: "on-demand live editor checks",
          fallback_scope_status: scopeMapStatus.status,
          scope_map_entry_count: scopeMapStatus.selector_count
        }
      },
      highlighting: {
        model: "TextMate first paint plus compiler-backed semantic token refinement",
        summary: highlightingSummary,
        textmate_first_paint: {
          enabled: true,
          source: "generated TextMate grammar",
          purpose: "Immediate lexical colors before compiler-backed roles arrive."
        },
        semantic_tokens: {
          enabled: semanticHighlighting,
          source: "live_editor",
          request_model: "on-demand live editor checks",
          token_type_count: semanticTokenTypes.length,
          token_modifier_count: semanticTokenModifiers.length
        },
        fallback_scope_map: scopeMapStatus,
        inspection_commands: {
          current_file: "EngLang: Inspect Highlight Tokens",
          cursor: "EngLang: Inspect Highlight Token at Cursor"
        }
      },
      settings: {
        diagnostics_mode: mode,
        saved_file_diagnostics_on_open_save: config.get("lintOnSave", true),
        live_typing_diagnostics_enabled: problemsSource === "live" && lintOnChange,
        lint_on_save: config.get("lintOnSave", true),
        lint_on_change: lintOnChange,
        role_aware_highlighting: semanticHighlighting,
        semantic_highlighting: semanticHighlighting,
        review_risk_decorations: config.get("reviewRiskDecorations.enabled", true),
        execution_profile: executionProfile(document)
      },
      commands: {
        switch_diagnostics_mode: "EngLang: Switch Diagnostics Mode...",
        inspect_highlight_tokens: "EngLang: Inspect Highlight Tokens",
        inspect_highlight_token_at_cursor: "EngLang: Inspect Highlight Token at Cursor",
        check_current_file: "EngLang: Check Current File"
      }
    };
  }

  function diagnosticsProblemsSource(mode) {
    return mode === "live" ? "eng/live" : "eng/file";
  }
  function diagnosticsModeChangeSummary(mode, lintOnChange) {
    const sourceLabel = diagnosticsProblemsSource(mode);
    if (mode !== "live") {
      return `VS Code Problems will use saved-file diagnostics on open, save, and manual check with source ${sourceLabel}.`;
    }
    if (!lintOnChange) {
      return `Live diagnostics mode is selected with source ${sourceLabel}, but typing updates remain off because englang.lintOnChange is false.`;
    }
    return `VS Code Problems will update from the current unsaved buffer while typing with source ${sourceLabel}.`;
  }

  function diagnosticsStatusSummary(mode, lintOnChange) {
    const sourceLabel = diagnosticsProblemsSource(mode);
    if (mode !== "live") {
      return `VS Code Problems use source ${sourceLabel} for saved-file diagnostics when a file opens, saves, or is checked manually.`;
    }
    if (!lintOnChange) {
      return `Live diagnostics mode is selected with source ${sourceLabel}, but typing updates are off because englang.lintOnChange is false.`;
    }
    return `VS Code Problems use source ${sourceLabel} and update from the current unsaved editor buffer after a short typing pause.`;
  }

  function toolStatusSummary(status, purpose) {
    const name = path.basename(status.resolved_path);
    return `${name} is used for ${purpose}; ${status.availability}.`;
  }

  function liveEditorFeature(tool) {
    return {
      enabled: true,
      tool,
      request_model: "on-demand live editor checks"
    };
  }

  function highlightingStatusSummary(semanticHighlighting, scopeMapStatus) {
    const mapLabel = scopeMapStatus.status === "mapped"
      ? "all generated token types and modifiers have fallback scopes"
      : `${scopeMapStatus.missing_token_types.length} token type(s) and ${scopeMapStatus.missing_modifiers.length} modifier(s) need fallback scopes`;
    return semanticHighlighting
      ? `Role-aware semantic highlighting is enabled; ${mapLabel}.`
      : `Role-aware semantic highlighting is disabled; ${mapLabel}.`;
  }

  function semanticScopeMapStatus(scopeMap, tokenTypes, tokenModifiers) {
    const selectors = Object.keys(scopeMap ?? {}).sort();
    const mappedTokenTypes = [];
    const missingTokenTypes = [];
    for (const tokenType of tokenTypes ?? []) {
      if (selectorHasMappedScopes(scopeMap, tokenType)) {
        mappedTokenTypes.push(tokenType);
      } else {
        missingTokenTypes.push(tokenType);
      }
    }

    const mappedModifiers = [];
    const missingModifiers = [];
    const selectors_by_modifier = {};
    for (const modifier of tokenModifiers ?? []) {
      const modifierSelectors = selectors.filter((selector) =>
        semanticSelectorHasModifier(selector, modifier) && selectorHasMappedScopes(scopeMap, selector)
      );
      selectors_by_modifier[modifier] = modifierSelectors;
      if (modifierSelectors.length > 0) {
        mappedModifiers.push(modifier);
      } else {
        missingModifiers.push(modifier);
      }
    }

    const status = selectors.length === 0
      ? "missing_scope_map"
      : missingTokenTypes.length > 0 || missingModifiers.length > 0
        ? "partial"
        : "mapped";
    return {
      status,
      selector_count: selectors.length,
      token_type_count: (tokenTypes ?? []).length,
      mapped_token_type_count: mappedTokenTypes.length,
      missing_token_types: missingTokenTypes,
      token_modifier_count: (tokenModifiers ?? []).length,
      mapped_modifier_count: mappedModifiers.length,
      missing_modifiers: missingModifiers,
      selectors_by_modifier
    };
  }

  function selectorHasMappedScopes(scopeMap, selector) {
    const mappedScopes = scopeMap?.[selector];
    const values = Array.isArray(mappedScopes)
      ? mappedScopes
      : typeof mappedScopes === "string"
        ? [mappedScopes]
        : [];
    return values.some((scope) => typeof scope === "string" && scope.length > 0);
  }

  function semanticSelectorHasModifier(selector, modifier) {
    return selector.split(".").slice(1).includes(modifier);
  }

  function executablePathKey(value) {
    const normalized = path.normalize(value);
    return process.platform === "win32" ? normalized.toLowerCase() : normalized;
  }

  function executableStatus(resolvedPath, configuredPath) {
    const trimmedConfiguredPath = typeof configuredPath === "string" ? configuredPath.trim() : "";
    const pathLike = /[\\/]/.test(resolvedPath);
    const exists = pathLike ? fs.existsSync(resolvedPath) : null;
    const configuredSelected = trimmedConfiguredPath.length > 0
      && executablePathKey(resolvedPath) === executablePathKey(trimmedConfiguredPath);
    const configuredFallback = trimmedConfiguredPath.length > 0 && !configuredSelected;
    const source = configuredSelected
      ? "setting"
      : pathLike
        ? "bundled-or-workspace"
        : "PATH";
    const sourceLabel = configuredSelected
      ? "Configured in settings"
      : configuredFallback
        ? "Fallback because the configured path was not found"
        : pathLike
          ? "Bundled or workspace executable"
          : "Resolved from PATH when invoked";
    return {
      resolved_path: resolvedPath,
      configured_path: trimmedConfiguredPath || null,
      configured_path_status: trimmedConfiguredPath
        ? configuredSelected
          ? "selected"
          : "not found; using fallback"
        : "unset",
      source,
      source_label: sourceLabel,
      exists,
      availability: pathLike
        ? exists
          ? "the file exists"
          : "the selected path is missing"
        : "the command will be resolved from PATH when invoked",
      launch_hint: pathLike
        ? "VS Code will launch this exact executable path."
        : "VS Code will ask the operating system to find this command on PATH."
    };
  }

  return {
    runActiveFile,
    runExample,
    switchExecutionProfile,
    switchDiagnosticsMode,
    switchProblemsSource: switchDiagnosticsMode,
    showToolingStatus,
    reviewActiveFile,
    openReviewPanel,
    showSemanticTokensDebug,
    showSemanticTokenAtCursor
  };
}

module.exports = {
  createCommandHandlers
};
