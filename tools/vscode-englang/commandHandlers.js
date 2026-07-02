const cp = require("child_process");
const crypto = require("crypto");
const fs = require("fs");
const path = require("path");
const vscode = require("vscode");
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
  semanticTokenDebugSample
} = require("./lspSemanticTokens");
const {
  renderReviewSummaryHtml,
  reviewPanelArtifacts
} = require("./reviewPanelRenderer");

const PROBLEMS_SOURCES = [
  {
    id: "file",
    label: "file",
    description: "Quieter saved-file checks",
    detail: "Problems update when an EngLang file opens, saves, or is checked manually."
  },
  {
    id: "live",
    label: "live",
    description: "Live buffer checks",
    detail: "Problems update from the current unsaved editor buffer after a short typing pause."
  }
];

function createCommandHandlers(options = {}) {
  const output = options.output;
  const reviewCache = options.reviewCache;
  const artifactOpeners = options.artifactOpeners;
  const lspRequests = options.lspRequests;
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

  async function switchProblemsSource() {
    const document = vscode.window.activeTextEditor?.document;
    const current = problemsSource(document);
    const picked = await vscode.window.showQuickPick(
      PROBLEMS_SOURCES.map((source) => ({
        label: source.label,
        description: source.id === current ? `${source.description} (current)` : source.description,
        detail: source.detail,
        source: source.id
      })),
      { placeHolder: `Current EngLang Problems source: ${current}` }
    );
    if (!picked) {
      return;
    }

    const target = vscode.workspace.workspaceFolders?.length
      ? vscode.ConfigurationTarget.Workspace
      : vscode.ConfigurationTarget.Global;
    await engConfig(document).update("problemsSource", picked.source, target);
    const suffix = picked.source === "live"
      ? "Problems will update while typing when englang.lintOnChange is enabled."
      : "Problems will use saved-file checks on open, save, and manual check.";
    vscode.window.showInformationMessage(`EngLang Problems source set to ${picked.source}. ${suffix}`);
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
      vscode.window.showWarningMessage("No highlight data is available. See the EngLang output panel.");
      return;
    }
    reviewCache.set(document.uri.fsPath, snapshot);
    updateSemanticSymbolDecorations(document, snapshot);
    const semanticTokens = snapshot.semantic_tokens ?? { legend: {}, tokens: [] };
    const tokenCounts = {};
    const modifierCounts = {};
    const tokenSamplesByType = {};
    const tokenSamplesByModifier = {};
    for (const token of semanticTokens.tokens ?? []) {
      tokenCounts[token.type] = (tokenCounts[token.type] ?? 0) + 1;
      const sample = semanticTokenDebugSample(document, token);
      addSemanticTokenDebugSample(tokenSamplesByType, token.type || "-", sample);
      for (const modifier of token.modifiers ?? []) {
        modifierCounts[modifier] = (modifierCounts[modifier] ?? 0) + 1;
        addSemanticTokenDebugSample(tokenSamplesByModifier, modifier || "-", sample);
      }
    }
    const payload = {
      source: document.uri.fsPath,
      highlight_count: semanticTokens.tokens?.length ?? 0,
      highlight_counts_by_category: tokenCounts,
      highlight_counts_by_detail: modifierCounts,
      highlight_samples_by_category: tokenSamplesByType,
      highlight_samples_by_detail: tokenSamplesByModifier,
      token_count: semanticTokens.tokens?.length ?? 0,
      token_counts_by_type: tokenCounts,
      token_counts_by_modifier: modifierCounts,
      token_samples_by_type: tokenSamplesByType,
      token_samples_by_modifier: tokenSamplesByModifier,
      highlight_data: semanticTokens,
      semantic_tokens: semanticTokens
    };
    const debugDocument = await vscode.workspace.openTextDocument({
      language: "json",
      content: JSON.stringify(payload, null, 2)
    });
    await vscode.window.showTextDocument(debugDocument, { preview: false });
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

  function problemsSource(document) {
    const config = engConfig(document);
    const configured = config.get("problemsSource", "file");
    if (configured === "file" || configured === "live") {
      return configured;
    }
    const legacyBackend = config.get("diagnosticsBackend", "eng-cli");
    return legacyBackend === "lsp-snapshot" ? "live" : "file";
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
    return {
      extension: {
        id: "englang.englang",
        version: context.extension?.packageJSON?.version ?? "unknown",
        path: context.extensionPath
      },
      workspace: {
        root: document ? workspaceRoot(document) : currentWorkspaceRoot() ?? null,
        active_document: vscode.window.activeTextEditor?.document?.uri?.fsPath ?? null
      },
      executables: {
        eng: executableStatus(runtime, config.get("runtimePath", "")),
        eng_lsp: executableStatus(lsp, config.get("lspPath", ""))
      },
      settings: {
        problems_source: problemsSource(document),
        lint_on_save: config.get("lintOnSave", true),
        lint_on_change: config.get("lintOnChange", true),
        semantic_highlighting: config.get("semanticHighlighting.enabled", true),
        review_risk_decorations: config.get("reviewRiskDecorations.enabled", true),
        execution_profile: executionProfile(document)
      },
      commands: {
        switch_problems_source: "EngLang: Switch Problems Source...",
        inspect_highlight_tokens: "EngLang: Inspect Highlight Tokens",
        check_current_file: "EngLang: Check Current File"
      }
    };
  }

  function executableStatus(resolvedPath, configuredPath) {
    const pathLike = /[\\/]/.test(resolvedPath);
    return {
      resolved_path: resolvedPath,
      configured_path: configuredPath || null,
      source: configuredPath
        ? "setting"
        : pathLike
          ? "bundled-or-workspace"
          : "PATH",
      exists: pathLike ? fs.existsSync(resolvedPath) : null
    };
  }

  return {
    runActiveFile,
    runExample,
    switchExecutionProfile,
    switchProblemsSource,
    showToolingStatus,
    reviewActiveFile,
    openReviewPanel,
    showSemanticTokensDebug
  };
}

module.exports = {
  createCommandHandlers
};
