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
        openSourceLine(result.document.uri, message.line).catch((error) => {
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

  async function openSourceLine(uri, line) {
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
    const position = new vscode.Position(targetLine, 0);
    const range = new vscode.Range(
      targetLine,
      0,
      targetLine,
      Math.max(1, textLine.text.length)
    );
    editor.selection = new vscode.Selection(position, position);
    editor.revealRange(range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
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
    let tokensWithoutFallbackScope = 0;
    for (const token of semanticTokens.tokens ?? []) {
      tokenCounts[token.type] = (tokenCounts[token.type] ?? 0) + 1;
      const sample = semanticTokenDebugSample(document, token, semanticTokenScopeMap);
      for (const selector of sample.semantic_selectors ?? []) {
        selectorCounts[selector] = (selectorCounts[selector] ?? 0) + 1;
        addSemanticTokenDebugSample(tokenSamplesBySelector, selector || "-", sample);
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
    const tokenCount = semanticTokens.tokens?.length ?? 0;
    const payload = {
      source: document.uri.fsPath,
      semantic_highlighting_enabled: engConfig(document).get("semanticHighlighting.enabled", true),
      summary: {
        status: highlightInspectionStatus(tokenCount, tokensWithoutFallbackScope),
        fallback_scope_status: highlightFallbackStatus(tokenCount, tokensWithoutFallbackScope),
        token_count: tokenCount,
        counts_by_type: tokenCounts,
        counts_by_modifier: modifierCounts,
        counts_by_selector: selectorCounts,
        scope_map_entry_count: Object.keys(semanticTokenScopeMap).length,
        tokens_without_fallback_scope: tokensWithoutFallbackScope,
        missing_scope_selectors: missingScopeSelectors
      },
      legend: semanticTokens.legend ?? {},
      samples: {
        by_type: tokenSamplesByType,
        by_modifier: tokenSamplesByModifier,
        by_selector: tokenSamplesBySelector
      },
      tokens: tokenRows,
      raw: {
        semantic_tokens: semanticTokens
      },
      highlight_count: tokenCount,
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
    const cursor = editor.selection.active;
    const matchingTokens = (semanticTokens.tokens ?? [])
      .filter((token) => semanticTokenRange(document, token)?.contains(cursor))
      .map((token) => semanticTokenDebugRow(document, token, semanticTokenScopeMap));
    const lineTokens = (semanticTokens.tokens ?? [])
      .filter((token) => Number(token.line) === cursor.line)
      .map((token) => semanticTokenDebugRow(document, token, semanticTokenScopeMap));
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
        cursor_token_hint: cursorTokenHint,
        scope_map_entry_count: Object.keys(semanticTokenScopeMap).length
      },
      matching_tokens: matchingTokens,
      nearest_tokens: nearestTokens,
      line_tokens: lineTokens,
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

  function highlightInspectionStatus(tokenCount, tokensWithoutFallbackScope) {
    if (tokenCount === 0) {
      return "No role-aware highlight tokens were returned for this file.";
    }
    if (tokensWithoutFallbackScope > 0) {
      return `${tokenCount} role-aware highlight token${tokenCount === 1 ? "" : "s"} returned; ${tokensWithoutFallbackScope} token${tokensWithoutFallbackScope === 1 ? "" : "s"} need theme fallback scope coverage.`;
    }
    return `${tokenCount} role-aware highlight token${tokenCount === 1 ? "" : "s"} returned with theme fallback scope coverage.`;
  }

  function highlightFallbackStatus(tokenCount, tokensWithoutFallbackScope) {
    if (tokenCount === 0) {
      return "no_tokens";
    }
    return tokensWithoutFallbackScope > 0 ? "missing_fallback_scopes" : "mapped";
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
    const semanticSelectors = sample.semantic_selectors ?? [];
    const fallbackScopes = sample.fallback_scopes ?? [];
    return {
      line: sample.line,
      column: Number.isFinite(start) ? start + 1 : null,
      start: sample.start,
      length: sample.length,
      text: sample.text,
      type: sample.type,
      modifiers: sample.modifiers,
      primary_selector: semanticSelectors[0] ?? sample.type,
      fallback_status: fallbackScopes.length > 0 ? "mapped" : "missing_fallback_scope",
      fallback_scope_count: fallbackScopes.length,
      semantic_selectors: semanticSelectors,
      fallback_scopes: fallbackScopes
    };
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
    return {
      summary: {
        check_and_run_tool: toolStatusSummary(checkAndRunTool, "saved-file checks and program runs"),
        live_editor_tool: toolStatusSummary(liveEditorTool, "live editor requests"),
        diagnostics: diagnosticsSummary,
        role_aware_colors: roleAwareColorSummary
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
        status_note: "Live editor features read the current buffer for hover, completion, symbols, highlights, formatting, quick fixes, and live Problems updates."
      },
      features: {
        problems: {
          source: problemsSource,
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
          request_model: "on-demand live editor check"
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

  function diagnosticsModeChangeSummary(mode, lintOnChange) {
    if (mode !== "live") {
      return "VS Code Problems will use saved-file diagnostics on open, save, and manual check.";
    }
    if (!lintOnChange) {
      return "Live diagnostics mode is selected, but typing updates remain off because englang.lintOnChange is false.";
    }
    return "VS Code Problems will update from the current unsaved buffer while typing.";
  }

  function diagnosticsStatusSummary(mode, lintOnChange) {
    if (mode !== "live") {
      return "VS Code Problems use saved-file diagnostics when a file opens, saves, or is checked manually.";
    }
    if (!lintOnChange) {
      return "Live diagnostics mode is selected, but typing updates are off because englang.lintOnChange is false.";
    }
    return "VS Code Problems update from the current unsaved editor buffer after a short typing pause.";
  }

  function toolStatusSummary(status, purpose) {
    const name = path.basename(status.resolved_path);
    return `${name} is used for ${purpose}; ${status.availability}.`;
  }

  function liveEditorFeature(tool) {
    return {
      enabled: true,
      tool,
      request_model: "on-demand live editor check"
    };
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
