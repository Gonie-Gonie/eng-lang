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
const LAST_RUN_REPORT_SPEC_RELATIVE_PATH = ["build", "result", "report_spec.json"];

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
  const diagnosticsCollection = options.diagnosticsCollection;
  const semanticTokenScopeMap = options.semanticTokenScopeMap ?? DEFAULT_SEMANTIC_TOKEN_SCOPE_MAP;
  const semanticTokenTypes = options.semanticTokenTypes ?? [];
  const semanticTokenModifiers = options.semanticTokenModifiers ?? [];
  const syntaxCatalog = options.syntaxCatalog ?? {};
  const isEngDocument = options.isEngDocument ?? (() => true);
  const updateSemanticSymbolDecorations =
    options.updateSemanticSymbolDecorations ?? (() => undefined);
  const cacheTimeAlignmentReview =
    options.cacheTimeAlignmentReview ?? (() => undefined);
  const clearTimeAlignmentReview =
    options.clearTimeAlignmentReview ?? (() => undefined);
  const updateTimeAlignmentDecorations =
    options.updateTimeAlignmentDecorations ?? (() => undefined);
  const timeAlignmentReviewRevision =
    options.timeAlignmentReviewRevision ?? (() => 0);
  const timeAlignmentReviewRevisionIsCurrent =
    options.timeAlignmentReviewRevisionIsCurrent ?? (() => true);

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
    clearTimeAlignmentReview(document);
    updateTimeAlignmentDecorations(document, undefined);
    const reviewRevision = timeAlignmentReviewRevision(document);
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
          const report = loadLastRunTimeAlignmentReport(document, cwd);
          if (report && timeAlignmentReviewRevisionIsCurrent(document, reviewRevision)) {
            cacheTimeAlignmentReview(document, report);
            updateTimeAlignmentDecorations(document, report);
            const alignmentCount = Array.isArray(report.time_alignments)
              ? report.time_alignments.length
              : 0;
            output.appendLine(`TimeSeries alignment review: ${alignmentCount} record(s) from current source.`);
          } else if (!timeAlignmentReviewRevisionIsCurrent(document, reviewRevision)) {
            output.appendLine("TimeSeries alignment review discarded: an EngLang workspace source changed during the run.");
          } else {
            output.appendLine("TimeSeries alignment review unavailable: report source path/hash did not match the current editor.");
          }
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
    const currentFileHighlightProbe = await toolingStatusHighlightProbe(context);
    const currentFileProblemsProbe = toolingStatusProblemsProbe();
    const payload = toolingStatusPayload(context, document, config, currentFileHighlightProbe, currentFileProblemsProbe);
    const statusDocument = await vscode.workspace.openTextDocument({
      language: "json",
      content: JSON.stringify(payload, null, 2)
    });
    await vscode.window.showTextDocument(statusDocument, { preview: false });
  }

  async function showProblemAtCursor() {
    const target = activeEngEditorOrWarn();
    if (!target) {
      return;
    }
    const payload = problemCursorPayload(target.document, target.editor.selection.active);
    const debugDocument = await vscode.workspace.openTextDocument({
      language: "json",
      content: JSON.stringify(payload, null, 2)
    });
    await vscode.window.showTextDocument(debugDocument, { preview: false });
  }

  async function copyProblemAtCursor() {
    const target = activeEngEditorOrWarn();
    if (!target) {
      return;
    }
    const payload = problemCursorPayload(target.document, target.editor.selection.active);
    const copyReady = payload.summary.copy_ready;
    if (!copyReady) {
      vscode.window.showInformationMessage(payload.summary.status);
      return;
    }
    await vscode.env.clipboard.writeText(JSON.stringify(copyReady, null, 2));
    vscode.window.showInformationMessage("EngLang problem copied to clipboard.");
  }

  function activeEngEditorOrWarn() {
    const editor = vscode.window.activeTextEditor;
    const document = editor?.document;
    if (!editor || !document || !isEngDocument(document)) {
      vscode.window.showWarningMessage("Open an EngLang .eng file first.");
      return undefined;
    }
    return { editor, document };
  }

  function problemCursorPayload(document, cursor) {
    const diagnosticsAvailable = typeof diagnosticsCollection?.get === "function";
    const diagnosticRows = diagnosticsAvailable
      ? Array.from(diagnosticsCollection.get(document.uri) ?? [])
        .map((diagnostic, index) => ({
          diagnostic,
          row: toolingStatusProblemRow(document, diagnostic, index)
        }))
      : [];
    const allProblems = diagnosticRows.map((entry) => entry.row);
    const matchingProblems = diagnosticRows
      .filter((entry) => entry.diagnostic?.range?.contains(cursor))
      .map((entry) => entry.row);
    const lineProblems = diagnosticRows
      .filter((entry) => problemRangeTouchesLine(entry.diagnostic?.range, cursor.line))
      .map((entry) => entry.row);
    const nearestProblems = lineProblems
      .map((problem) => ({
        ...problem,
        cursor_distance: problemCursorDistance(problem, cursor)
      }))
      .sort((left, right) =>
        left.cursor_distance - right.cursor_distance || Number(left.zero_based_character ?? 0) - Number(right.zero_based_character ?? 0)
      )
      .slice(0, 3);
    return {
      source: document.uri.fsPath,
      diagnostics_collection_status: diagnosticsAvailable ? "available" : "unavailable",
      cursor: {
        line: cursor.line + 1,
        column: cursor.character + 1,
        zero_based_line: cursor.line,
        zero_based_character: cursor.character
      },
      summary: {
        status: cursorProblemStatus(matchingProblems, nearestProblems, allProblems.length, diagnosticsAvailable),
        matching_problem_count: matchingProblems.length,
        nearest_problem_count: nearestProblems.length,
        line_problem_count: lineProblems.length,
        file_problem_count: allProblems.length,
        diagnostic_range_status: toolingStatusProblemsRangeStatus(allProblems),
        severity_counts: toolingStatusCountBy(allProblems, "severity"),
        source_counts: toolingStatusCountBy(allProblems, "source"),
        copy_ready: problemCopyReady(matchingProblems[0] ?? nearestProblems[0] ?? null)
      },
      matching_problems: matchingProblems,
      nearest_problems: nearestProblems,
      line_problems: lineProblems,
      all_problems: allProblems.slice(0, 50),
      omitted_problem_count: Math.max(0, allProblems.length - 50)
    };
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
    const coverageSummary = highlightCoverageSummary(document, tokenRows);
    const coverageStatus = highlightCoverageStatus(coverageSummary);
    const tokenCount = semanticTokens.tokens?.length ?? 0;
    const scopeMapStatus = semanticScopeMapStatus(
      semanticTokenScopeMap,
      semanticTokenTypes,
      semanticTokenModifiers
    );
    const payload = {
      source: document.uri.fsPath,
      role_aware_highlighting_enabled: engConfig(document).get("semanticHighlighting.enabled", true),
      semantic_highlighting_enabled: engConfig(document).get("semanticHighlighting.enabled", true),
      summary: {
        status: highlightInspectionStatus(tokenCount, tokensWithoutFallbackScope, tokensWithUnmappedSelectors, rangeOverlaps.length),
        fallback_scope_status: highlightFallbackStatus(tokenCount, tokensWithoutFallbackScope),
        theme_fallback_scope_status: themeFallbackScopeStatus(tokenCount, tokensWithoutFallbackScope),
        direct_selector_status: highlightDirectSelectorStatus(tokenCount, tokensWithUnmappedSelectors),
        range_overlap_status: highlightRangeOverlapStatus(tokenCount, rangeOverlaps.length),
        highlight_coverage_status: coverageStatus,
        coverage_status: coverageStatus,
        coverage_summary: coverageSummary,
        token_count: tokenCount,
        scope_map_status: scopeMapStatus.status,
        counts_by_type: tokenCounts,
        counts_by_modifier: modifierCounts,
        counts_by_selector: selectorCounts,
        range_overlap_count: rangeOverlaps.length,
        scope_map_entry_count: Object.keys(semanticTokenScopeMap).length,
        tokens_without_fallback_scope: tokensWithoutFallbackScope,
        tokens_missing_theme_fallback_scope: tokensWithoutFallbackScope,
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
      highlight_coverage_status: coverageStatus,
      highlight_coverage: coverageSummary,
      highlight_coverage_summary: coverageSummary,
      advanced_highlight_data: {
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
    const message = `No highlight data is available. Role-aware highlighting is ${settingState}; run EngLang: Show Tooling Status to confirm the live editor tool path.`;
    output.appendLine(`highlight data unavailable: role-aware highlighting setting=${semanticHighlighting}; use EngLang: Show Tooling Status to inspect the live editor tool path.`);
    const picked = await vscode.window.showWarningMessage(message, "Show Tooling Status");
    if (picked === "Show Tooling Status") {
      await showToolingStatus(context);
    }
  }

  async function showSemanticTokenAtCursor(context) {
    const target = activeEngEditorOrWarn();
    if (!target) {
      return;
    }
    const payload = await semanticTokenCursorPayload(context, target.document, target.editor.selection.active);
    if (!payload) {
      await showHighlightUnavailableWarning(context, target.document);
      return;
    }
    const debugDocument = await vscode.workspace.openTextDocument({
      language: "json",
      content: JSON.stringify(payload, null, 2)
    });
    await vscode.window.showTextDocument(debugDocument, { preview: false });
  }

  async function copySemanticTokenAtCursor(context) {
    const target = activeEngEditorOrWarn();
    if (!target) {
      return;
    }
    const payload = await semanticTokenCursorPayload(context, target.document, target.editor.selection.active);
    if (!payload) {
      await showHighlightUnavailableWarning(context, target.document);
      return;
    }
    const copyReady = payload.summary.copy_ready;
    if (!copyReady) {
      vscode.window.showInformationMessage(payload.summary.status);
      return;
    }
    await vscode.env.clipboard.writeText(JSON.stringify(copyReady, null, 2));
    vscode.window.showInformationMessage("EngLang highlight token copied to clipboard.");
  }

  async function semanticTokenCursorPayload(context, document, cursor) {
    const snapshot = await lspRequests.snapshotDocumentSource(document, context);
    if (!snapshot) {
      return undefined;
    }
    reviewCache.set(document.uri.fsPath, snapshot);
    updateSemanticSymbolDecorations(document, snapshot);
    const semanticTokens = snapshot.semantic_tokens ?? { legend: {}, tokens: [] };
    const scopeMapStatus = semanticScopeMapStatus(
      semanticTokenScopeMap,
      semanticTokenTypes,
      semanticTokenModifiers
    );
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
    return {
      source: document.uri.fsPath,
      role_aware_highlighting_enabled: engConfig(document).get("semanticHighlighting.enabled", true),
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
        line_range_overlap_status: highlightRangeOverlapStatus(lineTokens.length, lineRangeOverlaps.length),
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
      advanced_highlight_data: {
        semantic_tokens: semanticTokens
      }
    };
  }
  function highlightInspectionStatus(tokenCount, tokensWithoutFallbackScope, tokensWithUnmappedSelectors, rangeOverlapCount = 0) {
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
    if (rangeOverlapCount > 0) {
      issues.push(`${rangeOverlapCount} overlapping highlight range${rangeOverlapCount === 1 ? "" : "s"} need source-range review`);
    }
    if (issues.length > 0) {
      return `${tokenLabel} returned; ${issues.join(" and ")}.`;
    }
    return `${tokenLabel} returned with theme fallback scope coverage, direct selector mappings, and no overlapping ranges.`;
  }

  function highlightRangeOverlapStatus(tokenCount, rangeOverlapCount) {
    if (tokenCount === 0) {
      return "no_tokens";
    }
    return rangeOverlapCount > 0 ? "overlapping_ranges" : "clear";
  }

  function highlightFallbackStatus(tokenCount, tokensWithoutFallbackScope) {
    if (tokenCount === 0) {
      return "no_tokens";
    }
    return tokensWithoutFallbackScope > 0 ? "missing_fallback_scopes" : "mapped";
  }

  function themeFallbackScopeStatus(tokenCount, tokensWithoutFallbackScope) {
    if (tokenCount === 0) {
      return "no_tokens";
    }
    return tokensWithoutFallbackScope > 0 ? "missing_theme_fallback_scopes" : "covered";
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
    const themeCoverageStatus = fallbackScopes.length > 0 ? "covered" : "missing_theme_fallback_scope";
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
      theme_coverage_status: themeCoverageStatus,
      fallback_status: fallbackScopes.length > 0 ? "mapped" : "missing_fallback_scope",
      direct_selector_status: unmappedSelectors.length > 0 ? "missing_direct_scope" : "mapped",
      fallback_scope_count: fallbackScopes.length,
      theme_fallback_scope_count: fallbackScopes.length,
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
      theme_coverage_status: row?.theme_coverage_status ?? "-",
      fallback_status: row?.fallback_status ?? "-",
      direct_selector_status: row?.direct_selector_status ?? "-",
      fallback_scope_count: row?.fallback_scope_count ?? 0,
      theme_fallback_scope_count: row?.theme_fallback_scope_count ?? row?.fallback_scope_count ?? 0,
      inspector_panels: row?.inspector_panels ?? [],
      panel_hint: row?.panel_hint ?? null,
      copy_text: row?.copy_text ?? row?.text ?? "",
      copy_range: row?.copy_range ?? row?.range_text ?? null,
      copy_selector: row?.copy_selector ?? row?.primary_selector ?? row?.type ?? "-"
    };
  }

  function highlightCoverageSummary(document, tokenRows) {
    const tokenCounts = semanticTokenTextCounts(tokenRows, (line) =>
      line > 0 && line <= document.lineCount ? document.lineAt(line - 1).text : ""
    );
    return highlightCoverageCatalog().map((domain) => {
      const catalogWords = uniqueStrings(domain.words);
      const sourceWords = sourceCatalogWords(document, catalogWords, { allowNumericPrefix: domain.key === "unit" });
      const matchedSourceWords = sourceWords.filter((word) => (tokenCounts.get(normalizedCatalogWord(word)) || 0) > 0);
      const unmatchedSourceWords = sourceWords.filter((word) => (tokenCounts.get(normalizedCatalogWord(word)) || 0) === 0);
      const highlightedRangeCount = catalogWords.reduce((total, word) => total + (tokenCounts.get(normalizedCatalogWord(word)) || 0), 0);
      const status = unmatchedSourceWords.length ? "unmatched" : sourceWords.length ? "covered" : "not_used";
      return {
        domain: domain.key,
        label: domain.label,
        status,
        filter_query: domain.filter,
        catalog_word_count: catalogWords.length,
        source_word_count: sourceWords.length,
        highlighted_range_count: highlightedRangeCount,
        matched_source_words: matchedSourceWords,
        unmatched_source_words: unmatchedSourceWords
      };
    });
  }

  function highlightCoverageStatus(rows) {
    const items = Array.isArray(rows) ? rows : [];
    const unmatchedCount = items.reduce((total, row) => total + (row.unmatched_source_words?.length ?? 0), 0);
    if (unmatchedCount > 0) {
      return `unmatched_source_words:${unmatchedCount}`;
    }
    if (items.some((row) => (row.source_word_count ?? 0) > 0)) {
      return "covered";
    }
    return "not_used";
  }

  function highlightCoverageCatalog() {
    const keywordGroups = syntaxCatalog.keyword_groups ?? {};
    const keywordGroupWords = Object.values(keywordGroups).flatMap((items) => Array.isArray(items) ? items : []);
    const publicFieldWords = [
      ...catalogItemLabels(syntaxCatalog.table_fields),
      ...catalogItemLabels(syntaxCatalog.sample_table_fields),
      ...catalogItemLabels(syntaxCatalog.http_response_fields),
      ...catalogItemLabels(syntaxCatalog.coverage_result_fields),
      ...catalogItemLabels(syntaxCatalog.time_alignment_result_fields),
      ...catalogItemLabels(syntaxCatalog.db_connection_fields),
      ...catalogItemLabels(syntaxCatalog.case_table_fields),
      ...catalogItemLabels(syntaxCatalog.case_output_table_fields),
      ...catalogItemLabels(syntaxCatalog.case_run_result_table_fields),
      ...catalogItemLabels(syntaxCatalog.case_result_collection_table_fields),
      ...catalogItemLabels(syntaxCatalog.model_fields),
      ...catalogItemLabels(syntaxCatalog.prediction_table_fields)
    ];
    return [
      {
        key: "keyword",
        label: "Keywords",
        filter: "keyword",
        words: [...arrayOrEmpty(syntaxCatalog.keywords), ...keywordGroupWords]
      },
      {
        key: "type",
        label: "Types",
        filter: "type",
        words: arrayOrEmpty(syntaxCatalog.public_types)
      },
      {
        key: "quantity",
        label: "Quantities",
        filter: "quantity",
        words: catalogItemLabels(syntaxCatalog.quantities)
      },
      {
        key: "workflow",
        label: "Workflow",
        filter: "workflow",
        words: [
          ...arrayOrEmpty(syntaxCatalog.workflow_builtins),
          ...arrayOrEmpty(syntaxCatalog.hyphenated_workflow_builtins),
          ...arrayOrEmpty(syntaxCatalog.legacy_workflow_builtin_aliases),
          ...arrayOrEmpty(syntaxCatalog.workflow_status_literals)
        ]
      },
      {
        key: "option",
        label: "Options",
        filter: "option",
        words: [
          ...catalogItemLabels(syntaxCatalog.workflow_options),
          ...arrayOrEmpty(syntaxCatalog.legacy_workflow_option_aliases)
        ]
      },
      {
        key: "unit",
        label: "Units",
        filter: "unit",
        words: [
          ...catalogItemLabels(syntaxCatalog.units),
          ...arrayOrEmpty(syntaxCatalog.legacy_unit_aliases)
        ]
      },
      {
        key: "field",
        label: "Public fields",
        filter: "property",
        words: publicFieldWords
      },
      {
        key: "constant",
        label: "Constants",
        filter: "constant",
        words: [...arrayOrEmpty(syntaxCatalog.constants), ...arrayOrEmpty(syntaxCatalog.workflow_status_literals)]
      },
      {
        key: "operator",
        label: "Operators",
        filter: "operator",
        words: arrayOrEmpty(syntaxCatalog.operator_words)
      }
    ];
  }

  function semanticTokenTextCounts(tokenRows, lineTextForRow = () => "") {
    const counts = new Map();
    for (const row of Array.isArray(tokenRows) ? tokenRows : []) {
      const key = normalizedCatalogWord(row?.text);
      if (!key) continue;
      counts.set(key, (counts.get(key) || 0) + 1);
    }
    addSemanticTokenPhraseCounts(counts, tokenRows, lineTextForRow);
    return counts;
  }

  function addSemanticTokenPhraseCounts(counts, tokenRows, lineTextForRow) {
    const rowsByLine = new Map();
    for (const row of Array.isArray(tokenRows) ? tokenRows : []) {
      const line = Number(row?.line);
      const start = Number(row?.start);
      const length = Number(row?.length);
      const key = normalizedCatalogWord(row?.text);
      if (!key || /\s/.test(key) || !Number.isFinite(line) || line < 1 || !Number.isFinite(start) || !Number.isFinite(length) || length <= 0) {
        continue;
      }
      if (!rowsByLine.has(line)) rowsByLine.set(line, []);
      rowsByLine.get(line).push({ key, start, end: start + length });
    }
    for (const [line, rows] of rowsByLine.entries()) {
      const sourceLine = String(lineTextForRow(line) || "");
      const ordered = rows.sort((left, right) => left.start - right.start || left.end - right.end);
      for (let index = 0; index < ordered.length; index += 1) {
        let phrase = ordered[index].key;
        let currentEnd = ordered[index].end;
        for (let nextIndex = index + 1; nextIndex < Math.min(index + 4, ordered.length); nextIndex += 1) {
          const next = ordered[nextIndex];
          const gap = sourceLine.slice(currentEnd, next.start);
          if (!/^\s+$/.test(gap)) break;
          phrase = `${phrase} ${next.key}`;
          counts.set(phrase, (counts.get(phrase) || 0) + 1);
          currentEnd = next.end;
        }
      }
    }
  }

  function sourceCatalogWords(document, words, options = {}) {
    const source = document?.getText?.() ?? "";
    if (!source) return [];
    return uniqueStrings(words)
      .filter((word) => sourceContainsCatalogWord(source, word, options))
      .sort((left, right) => left.localeCompare(right));
  }

  function sourceContainsCatalogWord(source, word, options = {}) {
    const value = String(word || "");
    if (!value) return false;
    let index = source.indexOf(value);
    while (index >= 0) {
      if (catalogWordBoundaryOk(source, index, value.length, options)) return true;
      index = source.indexOf(value, index + value.length);
    }
    return false;
  }

  function catalogWordBoundaryOk(source, index, length, options = {}) {
    const left = index > 0 ? source[index - 1] : "";
    const right = index + length < source.length ? source[index + length] : "";
    const leftOk = !isCatalogWordChar(left) || (options.allowNumericPrefix && /[0-9.]/.test(left));
    const rightOk = !isCatalogWordChar(right);
    return leftOk && rightOk;
  }

  function isCatalogWordChar(char) {
    return /[A-Za-z0-9_-]/.test(String(char || ""));
  }

  function catalogItemLabels(items) {
    return (Array.isArray(items) ? items : [])
      .map((item) => {
        if (typeof item === "string") return item;
        return typeof item?.label === "string" ? item.label : undefined;
      })
      .filter((label) => typeof label === "string" && label.length > 0);
  }

  function uniqueStrings(items) {
    return [...new Set((Array.isArray(items) ? items : [])
      .map((item) => String(item || "").trim())
      .filter(Boolean))];
  }

  function arrayOrEmpty(value) {
    return Array.isArray(value) ? value : [];
  }

  function normalizedCatalogWord(value) {
    return String(value || "").trim().toLowerCase();
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
      theme_coverage_status: row.theme_coverage_status ?? "-",
      fallback_status: row.fallback_status ?? "-",
      theme_fallback_scope_count: row.theme_fallback_scope_count ?? row.fallback_scope_count ?? 0,
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

  function toolingStatusPayload(context, document, config, currentFileHighlightProbe = null, currentFileProblemsProbe = null) {
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
      ? "Checked-code role-aware colors are enabled for the current editor."
      : "Checked-code role-aware colors are disabled; VS Code will use first-pass syntax colors only.";
    const scopeMapStatus = semanticScopeMapStatus(
      semanticTokenScopeMap,
      semanticTokenTypes,
      semanticTokenModifiers
    );
    const highlightingSummary = highlightingStatusSummary(semanticHighlighting, scopeMapStatus);
    const nativeWorkflowProbe = toolingStatusNativeWorkflowProbe(document);
    const localPackageStatus = toolingStatusLocalPackageStatus(context, document);
    return {
      summary: {
        check_and_run_tool: toolStatusSummary(checkAndRunTool, "saved-file checks and program runs"),
        live_editor_tool: toolStatusSummary(liveEditorTool, "live editor checks"),
        diagnostics: diagnosticsSummary,
        role_aware_colors: roleAwareColorSummary,
        highlighting: highlightingSummary,
        local_extension_package: localPackageStatus.summary,
        package_freshness: localPackageStatus.package_freshness.summary,
        install_freshness: localPackageStatus.install_freshness.summary,
        install_preflight: localPackageStatus.install_preflight.summary,
        current_file_highlights: currentFileHighlightProbe?.summary ?? "No current EngLang highlight probe was run.",
        current_file_problems: currentFileProblemsProbe?.summary ?? "No current EngLang Problems probe was run.",
        native_workflows: nativeWorkflowProbe.summary
      },
      extension: {
        id: "englang.englang",
        version: context.extension?.packageJSON?.version ?? "unknown",
        path: context.extensionPath,
        local_package: localPackageStatus
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
        highlighting_model: "First-pass syntax colors plus checked-code role-aware colors",
        status_note: "Live editor features read the current buffer for hover, completion, symbols, highlights, formatting, quick fixes, and live Problems updates."
      },
      features: {
        problems: {
          source: problemsSource,
          source_label: diagnosticsProblemsSource(mode),
          mode: problemsSource === "live" ? "live buffer" : "saved file",
          summary: diagnosticsSummary,
          updates_while_typing: problemsSource === "live" && lintOnChange,
          tool: problemsSource === "live" ? "live_editor" : "check_and_run",
          current_file_count: currentFileProblemsProbe?.diagnostic_count ?? 0,
          current_file_range_status: currentFileProblemsProbe?.diagnostic_range_status ?? "unknown",
          current_file_probe: currentFileProblemsProbe,
          inspection_commands: {
            cursor: "EngLang: Inspect Problem at Cursor",
            copy_cursor: "EngLang: Copy Problem at Cursor",
            status: "EngLang: Show Tooling Status"
          }
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
        },
        native_workflows: nativeWorkflowProbe
      },
      highlighting: {
        model: "First-pass syntax colors plus checked-code role-aware colors",
        summary: highlightingSummary,
        textmate_first_paint: {
          enabled: true,
          source: "generated TextMate grammar",
          purpose: "Immediate syntax colors before checked-code roles arrive."
        },
        semantic_tokens: {
          enabled: semanticHighlighting,
          source: "live_editor",
          request_model: "on-demand live editor checks",
          token_type_count: semanticTokenTypes.length,
          token_modifier_count: semanticTokenModifiers.length
        },
        current_file_probe: currentFileHighlightProbe,
        fallback_scope_map: scopeMapStatus,
        inspection_commands: {
          current_file: "EngLang: Inspect Highlight Tokens",
          cursor: "EngLang: Inspect Highlight Token at Cursor",
          copy_cursor: "EngLang: Copy Highlight Token at Cursor"
        }
      },
      problems: {
        current_file_probe: currentFileProblemsProbe
      },
      native_workflows: nativeWorkflowProbe,
      local_extension_package: localPackageStatus,
      settings: {
        diagnostics_mode: mode,
        saved_file_diagnostics_on_open_save: config.get("lintOnSave", true),
        live_typing_diagnostics_enabled: problemsSource === "live" && lintOnChange,
        lint_on_save: config.get("lintOnSave", true),
        lint_on_change: lintOnChange,
        role_aware_highlighting: semanticHighlighting,
        semantic_highlighting: semanticHighlighting,
        review_risk_decorations: config.get("reviewRiskDecorations.enabled", true),
        validation_decorations: config.get("validationDecorations.enabled", true),
        time_alignment_decorations: config.get("timeAlignmentDecorations.enabled", true),
        execution_profile: executionProfile(document)
      },
      commands: {
        switch_diagnostics_mode: "EngLang: Switch Diagnostics Mode...",
        inspect_problem_at_cursor: "EngLang: Inspect Problem at Cursor",
        copy_problem_at_cursor: "EngLang: Copy Problem at Cursor",
        inspect_highlight_tokens: "EngLang: Inspect Highlight Tokens",
        inspect_highlight_token_at_cursor: "EngLang: Inspect Highlight Token at Cursor",
        copy_highlight_token_at_cursor: "EngLang: Copy Highlight Token at Cursor",
        check_current_file: "EngLang: Check Current File"
      }
    };
  }

  function toolingStatusLocalPackageStatus(context, document) {
    const root = document ? workspaceRoot(document) : currentWorkspaceRoot();
    const version = context.extension?.packageJSON?.version ?? packageManifest.version ?? "unknown";
    const checkout = englangSourceCheckoutStatus(root);
    if (checkout.status !== "source_checkout_detected") {
      const notChecked = localPackageNotChecked("Source checkout not detected; run .\\dev.bat vscode-status from an EngLang repository checkout for CLI install/package status.");
      return {
        status: checkout.status,
        summary: "Local VSIX freshness is unavailable because the active workspace is not an EngLang source checkout.",
        workspace_root: root ?? null,
        expected_files: checkout.expected_files,
        missing_files: checkout.missing_files,
        built_vsix: null,
        package_freshness: notChecked,
        install_freshness: notChecked,
        install_preflight: notChecked,
        commands: localPackageCommands()
      };
    }

    const vsixPath = localVscodeVsixPath(root, version);
    const builtVsix = localPackageFileSummary(vsixPath);
    const latestInput = latestVscodePackageInput(root);
    const packageFreshness = vscodePackageFreshness(builtVsix, latestInput);
    const installFreshness = vscodeInstallFreshness(context.extensionPath, builtVsix);
    const installPreflight = vscodeInstallPreflight(installFreshness);
    return {
      status: "source_checkout_detected",
      summary: localPackageStatusSummary(packageFreshness, installFreshness, installPreflight),
      workspace_root: root,
      expected_vsix_path: vsixPath,
      built_vsix: builtVsix,
      package_inputs: latestInput,
      package_freshness: packageFreshness,
      install_freshness: installFreshness,
      install_preflight: installPreflight,
      commands: localPackageCommands()
    };
  }

  function englangSourceCheckoutStatus(root) {
    const expectedFiles = root
      ? [
          path.join(root, "dev.bat"),
          path.join(root, "scripts", "dev.ps1"),
          path.join(root, "tools", "vscode-englang", "package.json")
        ]
      : [];
    const missingFiles = expectedFiles.filter((filePath) => !fs.existsSync(filePath));
    return {
      status: root && missingFiles.length === 0 ? "source_checkout_detected" : "source_checkout_not_detected",
      expected_files: expectedFiles,
      missing_files: missingFiles
    };
  }

  function localVscodeVsixPath(root, version) {
    return path.join(root, "dist", "local-vscode", "tools", localVscodeVsixFileName(version));
  }

  function localVscodeVsixFileName(version) {
    const value = String(version || "unknown");
    return /(?:^|[-.])preview(?:[-.]|$)/i.test(value)
      ? `englang-vscode-preview-${value}.vsix`
      : `englang-vscode-${value}.vsix`;
  }

  function localPackageCommands() {
    return {
      status: ".\\dev.bat vscode-status",
      package: ".\\dev.bat vscode-package",
      install: ".\\dev.bat vscode-install"
    };
  }

  function localPackageNotChecked(summary) {
    return {
      status: "not_checked",
      summary
    };
  }

  function localPackageStatusSummary(packageFreshness, installFreshness, installPreflight) {
    return [
      packageFreshness.summary,
      installFreshness.summary,
      installPreflight.summary
    ].filter(Boolean).join(" ");
  }

  function latestVscodePackageInput(root) {
    const inputPaths = [
      path.join(root, "tools", "vscode-englang"),
      path.join(root, "target", "release", "eng.exe"),
      path.join(root, "target", "release", "eng-lsp.exe")
    ];
    const inputs = inputPaths.map((inputPath) => localPackageFileSummary(inputPath));
    const existingInputs = inputs.filter((input) => input.exists && input.updated_ms !== null);
    const newest = existingInputs
      .slice()
      .sort((left, right) => right.updated_ms - left.updated_ms)[0] ?? null;
    return {
      paths: inputs,
      newest
    };
  }

  function vscodePackageFreshness(builtVsix, latestInput) {
    if (!builtVsix?.exists) {
      return {
        status: "missing",
        summary: "Package freshness: missing - run .\\dev.bat vscode-package."
      };
    }
    const newestInput = latestInput?.newest;
    if (!newestInput?.updated_ms) {
      return {
        status: "unknown",
        summary: "Package freshness: unknown - VS Code package input timestamps could not be read."
      };
    }
    if (newestInput.updated_ms > builtVsix.updated_ms + 1000) {
      return {
        status: "rebuild_available",
        summary: `Package freshness: rebuild available - VS Code extension source or release binaries are newer than the built VSIX (inputs ${newestInput.updated}, VSIX ${builtVsix.updated}); run .\\dev.bat vscode-package.`
      };
    }
    return {
      status: "current",
      summary: "Package freshness: current - built VSIX is at least as new as VS Code extension source and release binaries."
    };
  }

  function vscodeInstallFreshness(extensionPath, builtVsix) {
    if (!builtVsix?.exists) {
      return {
        status: "unknown",
        summary: "Install freshness: unknown - build the VSIX with .\\dev.bat vscode-package."
      };
    }
    const installed = localPackageFileSummary(extensionPath);
    if (!installed.exists || installed.updated_ms === null) {
      return {
        status: "unknown",
        summary: "Install freshness: unknown - installed EngLang extension timestamp could not be read."
      };
    }
    if (builtVsix.updated_ms > installed.updated_ms + 1000) {
      return {
        status: "update_available",
        summary: `Install freshness: update available - built VSIX is newer than the running EngLang extension (VSIX ${builtVsix.updated}, extension ${installed.updated}); close all VS Code windows and run .\\dev.bat vscode-install.`
      };
    }
    return {
      status: "current",
      summary: "Install freshness: current - running EngLang extension is at least as new as the built VSIX."
    };
  }

  function vscodeInstallPreflight(installFreshness) {
    if (installFreshness?.status === "update_available") {
      return {
        status: "blocked_while_vscode_is_running",
        summary: "Install preflight: blocked while this VS Code window is running; close all VS Code windows before reinstalling EngLang."
      };
    }
    return {
      status: "ready_or_not_needed",
      summary: "Install preflight: ready or not needed for the running extension."
    };
  }

  function localPackageFileSummary(targetPath) {
    if (!targetPath) {
      return {
        path: null,
        exists: false,
        kind: "missing",
        updated: null,
        updated_ms: null,
        size: null,
        size_label: null
      };
    }
    try {
      if (!fs.existsSync(targetPath)) {
        return {
          path: targetPath,
          exists: false,
          kind: "missing",
          updated: null,
          updated_ms: null,
          size: null,
          size_label: null
        };
      }
      const stats = fs.statSync(targetPath);
      if (stats.isDirectory()) {
        const newest = newestFileSummary(targetPath);
        return {
          path: targetPath,
          exists: true,
          kind: "directory",
          updated: newest ? timestampLabel(newest.updated_ms) : timestampLabel(stats.mtimeMs),
          updated_ms: newest?.updated_ms ?? stats.mtimeMs,
          size: null,
          size_label: null
        };
      }
      return {
        path: targetPath,
        exists: true,
        kind: "file",
        updated: timestampLabel(stats.mtimeMs),
        updated_ms: stats.mtimeMs,
        size: stats.size,
        size_label: byteSizeLabel(stats.size)
      };
    } catch (error) {
      return {
        path: targetPath,
        exists: false,
        kind: "unreadable",
        updated: null,
        updated_ms: null,
        size: null,
        size_label: null,
        error: error.message
      };
    }
  }

  function newestFileSummary(directoryPath) {
    let newest = null;
    const pending = [directoryPath];
    while (pending.length > 0) {
      const current = pending.pop();
      let entries = [];
      try {
        entries = fs.readdirSync(current, { withFileTypes: true });
      } catch {
        continue;
      }
      for (const entry of entries) {
        const entryPath = path.join(current, entry.name);
        try {
          const stats = fs.statSync(entryPath);
          if (entry.isDirectory()) {
            pending.push(entryPath);
          } else if (!newest || stats.mtimeMs > newest.updated_ms) {
            newest = {
              path: entryPath,
              updated_ms: stats.mtimeMs
            };
          }
        } catch {
          // Ignore unreadable files in status reporting; the surrounding summary remains best-effort.
        }
      }
    }
    return newest;
  }

  function timestampLabel(ms) {
    return Number.isFinite(ms) ? new Date(ms).toISOString() : null;
  }

  function byteSizeLabel(bytes) {
    if (!Number.isFinite(bytes)) {
      return null;
    }
    if (bytes >= 1024 * 1024 * 1024) {
      return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
    }
    if (bytes >= 1024 * 1024) {
      return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    }
    if (bytes >= 1024) {
      return `${(bytes / 1024).toFixed(1)} KB`;
    }
    return `${bytes} B`;
  }
  function toolingStatusProblemsProbe() {
    const document = vscode.window.activeTextEditor?.document;
    if (!document || !isEngDocument(document)) {
      return {
        status: "no_active_englang_document",
        summary: "No active EngLang file is open for current-file Problems probing.",
        diagnostic_count: 0,
        diagnostic_range_status: "no_diagnostics",
        diagnostics: []
      };
    }
    if (typeof diagnosticsCollection?.get !== "function") {
      return {
        source: document.uri.fsPath,
        status: "unavailable",
        summary: "Current-file Problems probing is unavailable because the VS Code diagnostics collection is not configured.",
        diagnostic_count: 0,
        diagnostic_range_status: "no_diagnostics",
        diagnostics: []
      };
    }

    const diagnostics = Array.from(diagnosticsCollection.get(document.uri) ?? []);
    const rows = diagnostics.map((diagnostic, index) => toolingStatusProblemRow(document, diagnostic, index));
    const severityCounts = toolingStatusCountBy(rows, "severity");
    const sourceCounts = toolingStatusCountBy(rows, "source");
    const rangeCounts = toolingStatusCountBy(rows, "diagnostic_range_status");
    const diagnosticRangeStatus = toolingStatusProblemsRangeStatus(rows);
    return {
      source: document.uri.fsPath,
      status: rows.length > 0 ? "diagnostics_present" : "clean",
      summary: toolingStatusProblemsSummary(rows.length, diagnosticRangeStatus, severityCounts),
      diagnostic_count: rows.length,
      severity_counts: severityCounts,
      source_counts: sourceCounts,
      diagnostic_range_status: diagnosticRangeStatus,
      range_status_counts: rangeCounts,
      diagnostics: rows.slice(0, 20),
      omitted_diagnostic_count: Math.max(0, rows.length - 20)
    };
  }

  function toolingStatusProblemRow(document, diagnostic, index) {
    const range = diagnostic?.range;
    return {
      index,
      line: range ? range.start.line + 1 : null,
      column: range ? range.start.character + 1 : null,
      end_line: range ? range.end.line + 1 : null,
      end_column: range ? range.end.character + 1 : null,
      zero_based_line: range?.start.line ?? null,
      zero_based_character: range?.start.character ?? null,
      zero_based_end_line: range?.end.line ?? null,
      zero_based_end_character: range?.end.character ?? null,
      severity: toolingStatusProblemSeverity(diagnostic?.severity),
      source: diagnostic?.source || "unknown",
      code: toolingStatusDiagnosticCode(diagnostic?.code),
      message: toolingStatusProblemMessage(diagnostic?.message),
      diagnostic_range_text: toolingStatusProblemRangeText(range),
      diagnostic_source_text: toolingStatusProblemSourceText(document, range),
      source_line_text: toolingStatusProblemLineText(document, range),
      diagnostic_range_status: toolingStatusProblemRangeStatus(document, range)
    };
  }

  function toolingStatusProblemSeverity(severity) {
    switch (severity) {
      case vscode.DiagnosticSeverity.Error:
        return "error";
      case vscode.DiagnosticSeverity.Warning:
        return "warning";
      case vscode.DiagnosticSeverity.Information:
        return "information";
      case vscode.DiagnosticSeverity.Hint:
        return "hint";
      default:
        return severity === undefined || severity === null ? "unknown" : String(severity);
    }
  }

  function toolingStatusDiagnosticCode(code) {
    if (code === undefined || code === null) {
      return null;
    }
    if (typeof code === "object") {
      return code.value === undefined || code.value === null ? JSON.stringify(code) : String(code.value);
    }
    return String(code);
  }

  function toolingStatusProblemMessage(message) {
    return String(message ?? "").split(/\r?\n/)[0].slice(0, 240);
  }

  function toolingStatusProblemRangeText(range) {
    if (!range) {
      return null;
    }
    const startLine = range.start.line + 1;
    const startColumn = range.start.character + 1;
    const endLine = range.end.line + 1;
    const endColumn = range.end.character + 1;
    return startLine === endLine
      ? `L${startLine}:C${startColumn}-C${endColumn}`
      : `L${startLine}:C${startColumn}-L${endLine}:C${endColumn}`;
  }

  function toolingStatusProblemSourceText(document, range) {
    if (!range) {
      return null;
    }
    try {
      return toolingStatusProblemTruncatedText(document.getText(range));
    } catch {
      return null;
    }
  }

  function toolingStatusProblemLineText(document, range) {
    if (!range) {
      return null;
    }
    try {
      return toolingStatusProblemTruncatedText(document.lineAt(range.start.line).text);
    } catch {
      return null;
    }
  }

  function toolingStatusProblemTruncatedText(text) {
    const value = String(text ?? "");
    return value.length > 240 ? `${value.slice(0, 240)}...` : value;
  }

  function toolingStatusProblemRangeStatus(document, range) {
    if (!range) {
      return "missing";
    }
    if (range.start.line !== range.end.line) {
      return "multi_line";
    }
    if (range.end.character <= range.start.character) {
      return "point";
    }
    const lineLength = toolingStatusLineLength(document, range.start.line);
    if (range.start.character === 0 && lineLength !== undefined && range.end.character >= lineLength) {
      return "line";
    }
    return "precise";
  }

  function toolingStatusLineLength(document, zeroBasedLine) {
    try {
      if (zeroBasedLine < 0 || zeroBasedLine >= document.lineCount) {
        return undefined;
      }
      return document.lineAt(zeroBasedLine).text.length;
    } catch {
      return undefined;
    }
  }

  function toolingStatusProblemsRangeStatus(rows) {
    if (rows.length === 0) {
      return "no_diagnostics";
    }
    const statuses = new Set(rows.map((row) => row.diagnostic_range_status));
    if (statuses.size === 1) {
      return statuses.values().next().value;
    }
    if ([...statuses].every((status) => status === "precise" || status === "multi_line")) {
      return "precise_or_multi_line";
    }
    return "mixed";
  }

  function toolingStatusProblemsSummary(count, diagnosticRangeStatus, severityCounts) {
    if (count === 0) {
      return "Current EngLang file has no VS Code Problems diagnostics.";
    }
    const severitySummary = ["error", "warning", "information", "hint"]
      .filter((severity) => severityCounts[severity] > 0)
      .map((severity) => `${severityCounts[severity]} ${severity}`)
      .join(", ");
    const countLabel = `${count} diagnostic${count === 1 ? "" : "s"}`;
    const rangeLabel = diagnosticRangeStatus.replace(/_/g, " ");
    return `Current EngLang file has ${countLabel}${severitySummary ? ` (${severitySummary})` : ""}; Problems ranges are ${rangeLabel}.`;
  }

  function toolingStatusCountBy(rows, key) {
    const counts = {};
    for (const row of rows) {
      const value = row?.[key] || "unknown";
      counts[value] = (counts[value] ?? 0) + 1;
    }
    return counts;
  }

  function problemRangeTouchesLine(range, zeroBasedLine) {
    if (!range) {
      return false;
    }
    return range.start.line <= zeroBasedLine && range.end.line >= zeroBasedLine;
  }

  function problemCursorDistance(problem, cursor) {
    const startLine = Number(problem?.zero_based_line);
    const startCharacter = Number(problem?.zero_based_character);
    const endLine = Number(problem?.zero_based_end_line);
    const endCharacter = Number(problem?.zero_based_end_character);
    if (!Number.isFinite(startLine) || !Number.isFinite(startCharacter) || !Number.isFinite(endLine) || !Number.isFinite(endCharacter)) {
      return Number.MAX_SAFE_INTEGER;
    }
    const lineDistance = cursor.line < startLine
      ? startLine - cursor.line
      : cursor.line > endLine
        ? cursor.line - endLine
        : 0;
    if (lineDistance > 0) {
      return lineDistance * 100000 + Math.min(startCharacter, endCharacter);
    }
    if (cursor.line === startLine && cursor.character < startCharacter) {
      return startCharacter - cursor.character;
    }
    if (cursor.line === endLine && cursor.character > endCharacter) {
      return cursor.character - endCharacter;
    }
    return 0;
  }

  function cursorProblemStatus(matchingProblems, nearestProblems, fileProblemCount, diagnosticsAvailable = true) {
    if (!diagnosticsAvailable) {
      return "Current Problems data is unavailable because the VS Code diagnostics collection is not configured.";
    }
    if (matchingProblems.length > 0) {
      return `Caret is inside ${matchingProblems.length} VS Code Problems diagnostic${matchingProblems.length === 1 ? "" : "s"}.`;
    }
    if (nearestProblems.length > 0) {
      return "No Problems diagnostic covers the caret; nearest diagnostics on this line are listed.";
    }
    if (fileProblemCount > 0) {
      return `No Problems diagnostics are on this line; current file has ${fileProblemCount} diagnostic${fileProblemCount === 1 ? "" : "s"}.`;
    }
    return "Current EngLang file has no VS Code Problems diagnostics.";
  }

  function problemCopyReady(problem) {
    if (!problem) {
      return null;
    }
    return {
      code: problem.code,
      source: problem.source,
      severity: problem.severity,
      range: problem.diagnostic_range_text,
      text: problem.diagnostic_source_text,
      line_text: problem.source_line_text,
      message: problem.message
    };
  }

  function toolingStatusNativeWorkflowProbe(document) {
    const root = document ? workspaceRoot(document) : currentWorkspaceRoot();
    if (!root) {
      return {
        status: "no_workspace",
        summary: "No workspace root is open for native workflow status probing.",
        source_file_count: 0,
        public_doc_count: 0
      };
    }

    const workflowRoot = path.join(root, "examples", "workflows");
    const requiredSources = [
      "examples/workflows/01_weather_api_to_standard_file/main.eng",
      "examples/workflows/02_native_surrogate_case_workflow/main.eng",
      "examples/workflows/03_uncertain_sensor_report/main.eng"
    ];
    const requiredSourcePaths = requiredSources.map((relativePath) => path.join(root, ...relativePath.split("/")));
    const missingSources = requiredSourcePaths
      .filter((sourcePath) => !fs.existsSync(sourcePath))
      .map((sourcePath) => `missing required workflow source ${toolingStatusRelativePath(root, sourcePath)}`);
    const sourceFiles = requiredSourcePaths
      .filter((sourcePath) => fs.existsSync(sourcePath))
      .flatMap((sourcePath) => toolingStatusRecursiveFiles(path.dirname(sourcePath), [".eng"]))
      .filter((sourcePath, index, all) => all.indexOf(sourcePath) === index)
      .sort();
    const publicDocFiles = [
      ...toolingStatusRecursiveFiles(workflowRoot, [".md", ".txt"]),
      ...toolingStatusDirectoryFiles(path.join(root, "docs", "workflows"), [".md"]),
      ...[
        "examples/README.md",
        "docs/user/tutorial/12_composite_workflow.md",
        "docs/current/workflow_modules.md",
        "docs/current/test_ci_gates.md"
      ].map((relativePath) => path.join(root, ...relativePath.split("/"))).filter((docPath) => fs.existsSync(docPath))
    ].filter((docPath, index, all) => all.indexOf(docPath) === index).sort();

    const sourcePatterns = toolingStatusNativeWorkflowForbiddenPatterns();
    const docPatterns = sourcePatterns.filter((pattern) => pattern.label !== "requests" && pattern.label !== "urllib");
    const staleDocPhrases = [
      "files produced by an external process",
      "external-simulator adapter pattern",
      "native surrogate half",
      "external simulator adapter could feed later",
      "Python process:",
      "created by Python",
      "Python-created",
      "generated by Python",
      "Python-generated",
      "Python-made",
      "Python-backed",
      "Python-side",
      "CSV fixture",
      "02_external_simulation_surrogate",
      "external_simulation_surrogate.md"
    ];
    const sourceIssues = toolingStatusNativeWorkflowTextIssues(root, sourceFiles, sourcePatterns, "source");
    const docIssues = toolingStatusNativeWorkflowTextIssues(root, publicDocFiles, docPatterns, "public_doc", staleDocPhrases);
    const primitiveEvidence = toolingStatusNativeWorkflowPrimitiveEvidence(root);
    const processArtifact = toolingStatusNativeWorkflowProcessArtifact(root);
    const runGraphArtifacts = toolingStatusNativeWorkflowRunGraphArtifacts(root, sourcePatterns);
    const issues = [
      ...missingSources,
      ...sourceIssues,
      ...docIssues,
      ...primitiveEvidence.issues,
      ...processArtifact.issues,
      ...runGraphArtifacts.issues
    ];
    const status = issues.length > 0 ? "issues" : "passed";
    const artifactSummary = processArtifact.status === "present"
      ? `latest process_count=${processArtifact.process_count}; ${processArtifact.external_process_summary}; run graphs=${runGraphArtifacts.artifact_count}`
      : "latest process artifact missing";
    return {
      status,
      summary: issues.length > 0
        ? `Native workflow status found ${issues.length} issue${issues.length === 1 ? "" : "s"}.`
        : `Native workflow source/docs guard passed (${sourceFiles.length} source file${sourceFiles.length === 1 ? "" : "s"}, ${publicDocFiles.length} public doc${publicDocFiles.length === 1 ? "" : "s"}); ${primitiveEvidence.summary}; ${artifactSummary}.`,
      workspace_root: root,
      required_sources: requiredSources,
      source_file_count: sourceFiles.length,
      public_doc_count: publicDocFiles.length,
      native_primitive_evidence: primitiveEvidence,
      latest_process_artifact: processArtifact,
      latest_run_graph_artifacts: runGraphArtifacts,
      issue_count: issues.length,
      issues: issues.slice(0, 20),
      omitted_issue_count: Math.max(0, issues.length - 20),
      full_evidence_gate: ".\\dev.bat workflows-test"
    };
  }

  function toolingStatusNativeWorkflowForbiddenPatterns() {
    return [
      { label: "run command", regex: /\brun\s+command\b/im },
      { label: "python", regex: /\bpython(?:\d+(?:\.\d+)*)?(?:\.exe)?\b/i },
      { label: "py launcher", regex: /\bpy(?:\.exe)?\b/i },
      { label: ".py", regex: /\.py\b/i },
      { label: ".pyw", regex: /\.pyw\b/i },
      { label: ".ipynb", regex: /\.ipynb\b/i },
      { label: "pip", regex: /\bpip(?:3)?\b/i },
      { label: "conda", regex: /\bconda\b/i },
      { label: "poetry", regex: /\bpoetry\b/i },
      { label: "pyenv", regex: /\bpyenv\b/i },
      { label: "virtualenv", regex: /\bvirtualenv\b/i },
      { label: "venv", regex: /\bvenv\b/i },
      { label: "ipython", regex: /\bipython\b/i },
      { label: "pytest", regex: /\bpytest\b/i },
      { label: "tox", regex: /\btox\b/i },
      { label: "nox", regex: /\bnox\b/i },
      { label: "mypy", regex: /\bmypy\b/i },
      { label: "ruff", regex: /\bruff\b/i },
      { label: "subprocess", regex: /\bsubprocess\b/i },
      { label: "pandas", regex: /\bpandas\b/i },
      { label: "numpy", regex: /\bnumpy\b/i },
      { label: "scipy", regex: /\bscipy\b/i },
      { label: "sklearn", regex: /\bsklearn\b/i },
      { label: "statsmodels", regex: /\bstatsmodels\b/i },
      { label: "polars", regex: /\bpolars\b/i },
      { label: "matplotlib", regex: /\bmatplotlib\b/i },
      { label: "requests", regex: /\brequests\b/i },
      { label: "urllib", regex: /\burllib\b/i },
      { label: "pyarrow", regex: /\bpyarrow\b/i },
      { label: "xarray", regex: /\bxarray\b/i },
      { label: "tensorflow", regex: /\btensorflow\b/i },
      { label: "pytorch", regex: /\bpytorch\b/i },
      { label: "torch", regex: /\btorch\b/i },
      { label: "jupyter", regex: /\bjupyter\b/i },
      { label: "jupyterlab", regex: /\bjupyterlab\b/i },
      { label: "notebook", regex: /\bnotebook\b/i },
      { label: "select_first_row", regex: /\bselect_first_row\s*\(/i }
    ];
  }

  function toolingStatusNativeWorkflowTextIssues(root, files, patterns, fileKind, forbiddenPhrases = []) {
    const issues = [];
    for (const filePath of files) {
      const text = toolingStatusReadText(filePath);
      if (text === undefined) {
        issues.push(`${fileKind} unreadable: ${toolingStatusRelativePath(root, filePath)}`);
        continue;
      }
      for (const pattern of patterns) {
        if (pattern.regex.test(text)) {
          issues.push(`${fileKind} contains ${pattern.label}: ${toolingStatusRelativePath(root, filePath)}`);
        }
      }
      for (const phrase of forbiddenPhrases) {
        if (text.includes(phrase)) {
          issues.push(`${fileKind} contains stale wording '${phrase}': ${toolingStatusRelativePath(root, filePath)}`);
        }
      }
    }
    return issues;
  }

  function toolingStatusNativeWorkflowPrimitiveEvidence(root) {
    const specs = [
      {
        name: "01 weather API",
        source: "examples/workflows/01_weather_api_to_standard_file/main.eng",
        required: [
          ["http get", /\bhttp\s+get\b/i],
          ["offline_response", /\boffline_response\s*=/i],
          ["promote json", /\bpromote\s+json\b/i],
          ["check coverage", /\bcheck\s+coverage\b/i],
          ["write standard_text", /\bwrite\s+standard_text\b/i]
        ]
      },
      {
        name: "02 surrogate case",
        source: "examples/workflows/02_native_surrogate_case_workflow/main.eng",
        required: [
          ["sample lhs", /\bsample\s+lhs\b/i],
          ["derive", /\bderive\b/i],
          ["materialize cases", /\bmaterialize\s+cases\b/i],
          ["apply over", /\bapply\b[\s\S]*\bover\b/i],
          ["collect results", /\bcollect\s+results\b/i],
          ["train regression", /\btrain\s+regression\b/i],
          ["predict using", /\bpredict\b[\s\S]*\busing\b/i],
          ["open sqlite", /\bopen\s+sqlite\b/i],
          ["read sqlite", /\bread\s+sqlite\b/i]
        ]
      },
      {
        name: "03 uncertain sensor",
        source: "examples/workflows/03_uncertain_sensor_report/main.eng",
        required: [
          ["promote csv", /\bpromote\s+csv\b/i],
          ["sensor_std", /\bsensor_std\s*=/i],
          ["integrate", /\bintegrate\s*\(/i],
          ["mean", /\bmean\s*\(/i],
          ["max", /\bmax\s*\(/i],
          ["check coverage", /\bcheck\s+coverage\b/i],
          ["export summary", /\bexport\s+summary\s+to\s+csv\b/i],
          ["confidence_band", /\bconfidence_band\s*=/i]
        ]
      }
    ];
    const issues = [];
    const workflows = specs.map((spec) => {
      const sourcePath = path.join(root, ...spec.source.split("/"));
      const text = fs.existsSync(sourcePath) ? fs.readFileSync(sourcePath, "utf8") : "";
      const matched = [];
      const missing = [];
      if (!text) {
        issues.push(`native primitive evidence source missing: ${spec.source}`);
      }
      for (const [label, regex] of spec.required) {
        if (regex.test(text)) {
          matched.push(label);
        } else {
          missing.push(label);
          issues.push(`native primitive evidence missing ${label}: ${spec.name}`);
        }
      }
      return {
        name: spec.name,
        source: spec.source,
        matched,
        missing,
        matched_count: matched.length,
        required_count: spec.required.length,
        summary: `${spec.name}: ${matched.join(", ")}`
      };
    });
    return {
      status: issues.length > 0 ? "issues" : "passed",
      summary: workflows.map((workflow) => workflow.summary).join("; "),
      workflows,
      issues
    };
  }

  function toolingStatusNativeWorkflowProcessArtifact(root) {
    const artifactPath = path.join(root, "build", "result", "process_results.json");
    if (!fs.existsSync(artifactPath)) {
      return {
        status: "missing",
        summary: "No latest process_results.json artifact was found; run .\\dev.bat workflows-test for fresh evidence.",
        path: toolingStatusRelativePath(root, artifactPath),
        issues: []
      };
    }
    const issues = [];
    try {
      const processResults = JSON.parse(fs.readFileSync(artifactPath, "utf8"));
      const processCount = Number(processResults.process_count ?? 0);
      const processListCount = Array.isArray(processResults.processes) ? processResults.processes.length : 0;
      if (processResults.format !== "eng-process-results-v1") {
        issues.push(`latest process_results.json has unexpected format ${processResults.format}`);
      }
      if (processResults.execution_profile !== "normal") {
        issues.push(`latest process_results.json has unexpected execution_profile ${processResults.execution_profile}`);
      }
      if (processCount !== 0 || processListCount !== 0) {
        issues.push("latest process_results.json records external processes");
      }
      const externalProcessSummary = processCount === 0 && processListCount === 0
        ? "no external processes"
        : `external processes recorded (process_count=${processCount}, processes=${processListCount})`;
      return {
        status: "present",
        summary: `Latest process_results.json has process_count=${processCount} and processes=${processListCount}; ${externalProcessSummary}.`,
        path: toolingStatusRelativePath(root, artifactPath),
        format: processResults.format ?? null,
        execution_profile: processResults.execution_profile ?? null,
        process_count: processCount,
        processes_count: processListCount,
        external_process_summary: externalProcessSummary,
        issues
      };
    } catch (error) {
      return {
        status: "error",
        summary: "Latest process_results.json could not be parsed.",
        path: toolingStatusRelativePath(root, artifactPath),
        error: error.message,
        issues: [`could not parse latest process_results.json: ${error.message}`]
      };
    }
  }

  function toolingStatusNativeWorkflowRunGraphArtifacts(root, sourcePatterns) {
    const artifactPaths = [
      path.join(root, "build", "result", "static_run_plan.json"),
      path.join(root, "build", "result", "run_plan.json")
    ];
    const existingArtifactPaths = artifactPaths.filter((artifactPath) => fs.existsSync(artifactPath));
    const issues = [];
    for (const artifactPath of existingArtifactPaths) {
      try {
        const runGraph = JSON.parse(fs.readFileSync(artifactPath, "utf8"));
        const fields = [];
        for (const node of runGraph?.graph?.nodes ?? []) {
          fields.push(String(node?.id ?? ""), String(node?.kind ?? ""), String(node?.label ?? ""));
        }
        for (const edge of runGraph?.graph?.edges ?? []) {
          fields.push(String(edge?.from ?? ""), String(edge?.to ?? ""), String(edge?.kind ?? ""));
        }
        for (const field of fields) {
          if (/^process:/i.test(field) || /\brun\s+command\b/i.test(field)) {
            issues.push(`run graph contains process/run-command metadata '${field}': ${toolingStatusRelativePath(root, artifactPath)}`);
          }
          for (const pattern of sourcePatterns) {
            if (pattern.label !== "run command" && pattern.regex.test(field)) {
              issues.push(`run graph contains ${pattern.label} marker '${field}': ${toolingStatusRelativePath(root, artifactPath)}`);
            }
          }
        }
      } catch (error) {
        issues.push(`could not parse run graph ${toolingStatusRelativePath(root, artifactPath)}: ${error.message}`);
      }
    }
    return {
      status: existingArtifactPaths.length > 0 ? "present" : "missing",
      summary: existingArtifactPaths.length > 0
        ? `Checked ${existingArtifactPaths.length} latest run graph artifact${existingArtifactPaths.length === 1 ? "" : "s"}.`
        : "No latest run graph artifacts were found; run .\\dev.bat workflows-test for fresh evidence.",
      artifacts: existingArtifactPaths.map((artifactPath) => toolingStatusRelativePath(root, artifactPath)),
      artifact_count: existingArtifactPaths.length,
      issues
    };
  }

  function toolingStatusDirectoryFiles(directoryPath, extensions) {
    if (!fs.existsSync(directoryPath)) {
      return [];
    }
    try {
      return fs.readdirSync(directoryPath, { withFileTypes: true })
        .filter((entry) => entry.isFile() && extensions.includes(path.extname(entry.name).toLowerCase()))
        .map((entry) => path.join(directoryPath, entry.name));
    } catch {
      return [];
    }
  }

  function toolingStatusRecursiveFiles(directoryPath, extensions) {
    if (!fs.existsSync(directoryPath)) {
      return [];
    }
    const files = [];
    const stack = [directoryPath];
    while (stack.length > 0) {
      const current = stack.pop();
      let entries = [];
      try {
        entries = fs.readdirSync(current, { withFileTypes: true });
      } catch {
        continue;
      }
      for (const entry of entries) {
        const childPath = path.join(current, entry.name);
        if (entry.isDirectory()) {
          stack.push(childPath);
        } else if (entry.isFile() && extensions.includes(path.extname(entry.name).toLowerCase())) {
          files.push(childPath);
        }
      }
    }
    return files;
  }

  function toolingStatusReadText(filePath) {
    try {
      return fs.readFileSync(filePath, "utf8");
    } catch {
      return undefined;
    }
  }

  function toolingStatusRelativePath(root, targetPath) {
    return path.relative(root, targetPath).replace(/\\/g, "/");
  }
  async function toolingStatusHighlightProbe(context) {
    const document = vscode.window.activeTextEditor?.document;
    if (!document || !isEngDocument(document)) {
      return {
        status: "no_active_englang_document",
        summary: "No active EngLang file is open for current-file highlight probing.",
        token_count: 0,
        range_overlap_count: 0,
        range_overlap_status: "no_tokens"
      };
    }
    if (typeof lspRequests?.snapshotDocumentSource !== "function") {
      return {
        source: document.uri.fsPath,
        status: "unavailable",
        summary: "Current-file highlight probing is unavailable because live editor checks are not configured.",
        token_count: 0,
        range_overlap_count: 0,
        range_overlap_status: "no_tokens"
      };
    }
    try {
      const snapshot = await lspRequests.snapshotDocumentSource(document, context);
      if (!snapshot) {
        return {
          source: document.uri.fsPath,
          status: "unavailable",
          summary: "Current-file highlight data is unavailable; use EngLang: Inspect Highlight Tokens for details.",
          token_count: 0,
          range_overlap_count: 0,
          range_overlap_status: "no_tokens"
        };
      }
      const semanticTokens = snapshot.semantic_tokens ?? { legend: {}, tokens: [] };
      const tokenRows = (semanticTokens.tokens ?? [])
        .map((token) => semanticTokenDebugRow(document, token, semanticTokenScopeMap));
      const rangeOverlaps = semanticTokenRangeOverlaps(document, tokenRows);
      const coverageSummary = highlightCoverageSummary(document, tokenRows);
      const coverageStatus = highlightCoverageStatus(coverageSummary);
      const tokensMissingThemeFallbackScope = tokenRows.filter((row) => row.theme_coverage_status === "missing_theme_fallback_scope").length;
      return {
        source: document.uri.fsPath,
        status: highlightRangeOverlapStatus(tokenRows.length, rangeOverlaps.length),
        summary: toolingHighlightProbeSummary(tokenRows.length, rangeOverlaps.length),
        token_count: tokenRows.length,
        range_overlap_count: rangeOverlaps.length,
        range_overlap_status: highlightRangeOverlapStatus(tokenRows.length, rangeOverlaps.length),
        theme_fallback_scope_status: themeFallbackScopeStatus(tokenRows.length, tokensMissingThemeFallbackScope),
        tokens_missing_theme_fallback_scope: tokensMissingThemeFallbackScope,
        coverage_status: coverageStatus,
        highlight_coverage_status: coverageStatus,
        coverage_summary: coverageSummary,
        highlight_coverage: coverageSummary,
        inspection_commands: {
          current_file: "EngLang: Inspect Highlight Tokens",
          cursor: "EngLang: Inspect Highlight Token at Cursor",
          copy_cursor: "EngLang: Copy Highlight Token at Cursor"
        }
      };
    } catch (error) {
      output.appendLine(`Unable to probe current-file highlight status: ${error.message}`);
      return {
        source: document.uri.fsPath,
        status: "error",
        summary: "Current-file highlight probe failed; see the EngLang output panel.",
        token_count: 0,
        range_overlap_count: 0,
        range_overlap_status: "no_tokens",
        error: error.message
      };
    }
  }

  function toolingHighlightProbeSummary(tokenCount, rangeOverlapCount) {
    if (tokenCount === 0) {
      return "Current file returned no role-aware highlight tokens.";
    }
    if (rangeOverlapCount > 0) {
      return `Current file returned ${tokenCount} role-aware highlight token${tokenCount === 1 ? "" : "s"} with ${rangeOverlapCount} overlapping range${rangeOverlapCount === 1 ? "" : "s"}.`;
    }
    return `Current file returned ${tokenCount} role-aware highlight token${tokenCount === 1 ? "" : "s"} with no overlapping ranges.`;
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
      ? `Role-aware highlighting is enabled; ${mapLabel}.`
      : `Role-aware highlighting is disabled; ${mapLabel}.`;
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
        ? (pathLike ? "Configured path not found; using bundled or workspace executable" : "Configured path not found; using PATH command")
        : pathLike
          ? "Bundled or workspace executable"
          : "Resolved from PATH when invoked";
    return {
      resolved_path: resolvedPath,
      configured_path: trimmedConfiguredPath || null,
      configured_path_status: trimmedConfiguredPath
        ? configuredSelected
          ? "selected"
          : "configured_path_not_found_using_discovered_tool"
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
    showProblemAtCursor,
    copyProblemAtCursor,
    reviewActiveFile,
    openReviewPanel,
    showSemanticTokensDebug,
    showSemanticTokenAtCursor,
    copySemanticTokenAtCursor
  };
}

function loadLastRunTimeAlignmentReport(document, root) {
  if (!document?.uri?.fsPath || !root) {
    return undefined;
  }
  const reportPath = path.join(root, ...LAST_RUN_REPORT_SPEC_RELATIVE_PATH);
  try {
    const report = JSON.parse(fs.readFileSync(reportPath, "utf8"));
    return timeAlignmentReportMatchesDocument(report, document, root) ? report : undefined;
  } catch (_error) {
    return undefined;
  }
}

function timeAlignmentReportMatchesDocument(report, document, root) {
  if (!report || typeof report !== "object" || !document?.uri?.fsPath) {
    return false;
  }
  const sourcePath = report.source_path ?? report.sourcePath;
  if (typeof sourcePath !== "string" || sourcePath.trim().length === 0) {
    return false;
  }
  if (!path.isAbsolute(sourcePath) && !root) {
    return false;
  }
  const resolvedSource = path.isAbsolute(sourcePath)
    ? path.normalize(sourcePath)
    : path.resolve(root, sourcePath);
  if (pathComparisonKey(resolvedSource) !== pathComparisonKey(document.uri.fsPath)) {
    return false;
  }
  const sourceHash = report.source_hash ?? report.sourceHash;
  if (
    typeof sourceHash !== "string"
    || sourceHash.length === 0
    || typeof document.getText !== "function"
    || fnv1a64(document.getText()) !== sourceHash.toLowerCase()
  ) {
    return false;
  }
  return Array.isArray(report.time_alignments ?? report.timeAlignments);
}

function pathComparisonKey(value) {
  const normalized = path.resolve(String(value));
  return process.platform === "win32" ? normalized.toLowerCase() : normalized;
}

function fnv1a64(value) {
  let hash = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  for (const byte of Buffer.from(String(value), "utf8")) {
    hash ^= BigInt(byte);
    hash = BigInt.asUintN(64, hash * prime);
  }
  return hash.toString(16).padStart(16, "0");
}

module.exports = {
  createCommandHandlers,
  fnv1a64,
  loadLastRunTimeAlignmentReport,
  timeAlignmentReportMatchesDocument
};
