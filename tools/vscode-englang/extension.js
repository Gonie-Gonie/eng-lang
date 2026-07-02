const cp = require("child_process");
const crypto = require("crypto");
const fs = require("fs");
const path = require("path");
const vscode = require("vscode");
const { LAST_RUN_ARTIFACTS } = require("./artifactRegistry");
const { EngCompletionProvider } = require("./completionProvider");
const { EngDiagnosticsController } = require("./diagnosticsProvider");
const { EngCodeActionProvider } = require("./codeActionProvider");
const { EngFoldingRangeProvider } = require("./foldingRangeProvider");
const { EngFormattingProvider } = require("./formattingProvider");
const { EngHoverProvider } = require("./hoverProvider");
const {
  EngDefinitionProvider,
  EngDocumentSymbolProvider,
  EngWorkspaceSymbolProvider
} = require("./navigationProviders");
const { EngSemanticTokensProvider } = require("./semanticTokensProvider");
const { loadEditorMetadata } = require("./editorMetadata");
const { EXECUTION_PROFILES } = require("./executionProfiles");
const {
  currentWorkspaceRoot,
  engConfig,
  findLspRuntime,
  findLspRuntimeForRoot,
  findRuntime,
  workspaceRoot
} = require("./runtimeDiscovery");
const {
  addSemanticTokenDebugSample,
  createSemanticLegend,
  semanticTokenDebugSample,
  semanticTokenRange
} = require("./lspSemanticTokens");
const {
  firstReviewArray,
  lineValue,
  normalizedReviewDocument,
  renderReviewSummaryHtml,
  reviewPanelArtifacts,
  reviewValue
} = require("./reviewPanelRenderer");

const LANGUAGE_ID = "englang";
const reviewCache = new Map();
const snapshotPromiseCache = new Map();
let output;
let reviewRiskDecorations;
let semanticSymbolDecorations;

const editorMetadata = loadEditorMetadata(__dirname);
const SEMANTIC_TOKEN_TYPES = editorMetadata.semanticTokenTypes;
const SEMANTIC_TOKEN_MODIFIERS = editorMetadata.semanticTokenModifiers;
const COMPLETION_SEED = editorMetadata.completionSeed;

const semanticLegend = createSemanticLegend(
  SEMANTIC_TOKEN_TYPES,
  SEMANTIC_TOKEN_MODIFIERS
);

function activate(context) {
  output = vscode.window.createOutputChannel("EngLang");
  const diagnostics = vscode.languages.createDiagnosticCollection("englang");
  reviewRiskDecorations = createReviewRiskDecorationTypes();
  semanticSymbolDecorations = createSemanticSymbolDecorationTypes();
  const diagnosticController = new EngDiagnosticsController(context, diagnostics, {
    output,
    isEngDocument,
    clearSnapshotCache,
    diagnosticsBackend,
    diagnosticsBackendLabel,
    findLspRuntime,
    findRuntime,
    workspaceRoot,
    cacheReview: (document, review) => reviewCache.set(document.uri.fsPath, review),
    updateReviewRiskDecorations,
    updateSemanticSymbolDecorations
  });
  context.subscriptions.push(output, diagnostics);
  context.subscriptions.push(
    reviewRiskDecorations.high,
    reviewRiskDecorations.medium,
    semanticSymbolDecorations.internal,
    semanticSymbolDecorations.planned
  );

  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((document) => diagnosticController.maybeCheck(document)),
    vscode.workspace.onDidChangeTextDocument((event) => diagnosticController.scheduleChangedCheck(event.document)),
    vscode.workspace.onDidSaveTextDocument((document) => diagnosticController.maybeCheck(document)),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration("englang.reviewRiskDecorations.enabled")) {
        refreshVisibleReviewRiskDecorations();
      }
    }),
    vscode.workspace.onDidCloseTextDocument((document) => {
      diagnosticController.clearPendingCheck(document);
      clearSnapshotCache(document);
      diagnostics.delete(document.uri);
      updateReviewRiskDecorations(document, undefined);
      updateSemanticSymbolDecorations(document, undefined);
    }),
    vscode.window.onDidChangeActiveTextEditor((editor) => {
      if (editor && isEngDocument(editor.document)) {
        const cached = reviewCache.get(editor.document.uri.fsPath);
        updateReviewRiskDecorations(
          editor.document,
          cached
        );
        updateSemanticSymbolDecorations(editor.document, cached);
      }
    }),
    vscode.commands.registerCommand("englang.checkFile", () => diagnosticController.checkActiveFile()),
    vscode.commands.registerCommand("englang.runFile", () => runActiveFile(context)),
    vscode.commands.registerCommand("englang.runExample", () => runExample(context)),
    vscode.commands.registerCommand("englang.switchProfile", () => switchExecutionProfile()),
    vscode.commands.registerCommand("englang.reviewFile", () => reviewActiveFile(context)),
    vscode.commands.registerCommand("englang.openReviewPanel", () => openReviewPanel(context)),
    vscode.commands.registerCommand("englang.openReport", () => openLastRunArtifact("report")),
    vscode.commands.registerCommand("englang.openLastArtifact", openLastRunArtifactPicker),
    vscode.commands.registerCommand("englang.openGeneratedOutput", openGeneratedOutputArtifactPicker),
    vscode.commands.registerCommand("englang.openReviewJson", () => openLastRunArtifact("review")),
    vscode.commands.registerCommand("englang.openResultArtifact", () => openLastRunArtifact("result")),
    vscode.commands.registerCommand("englang.openReportSpec", () => openLastRunArtifact("reportSpec")),
    vscode.commands.registerCommand("englang.openOutputManifest", () => openLastRunArtifact("outputManifest")),
    vscode.commands.registerCommand("englang.openRunLog", () => openLastRunArtifact("runLog")),
    vscode.commands.registerCommand("englang.openStaticRunPlan", () => openLastRunArtifact("staticRunPlan")),
    vscode.commands.registerCommand("englang.openRunPlan", () => openLastRunArtifact("runPlan")),
    vscode.commands.registerCommand("englang.openRunLock", () => openLastRunArtifact("runLock")),
    vscode.commands.registerCommand("englang.openProcessResults", () => openLastRunArtifact("processResults")),
    vscode.commands.registerCommand("englang.openCacheManifest", () => openLastRunArtifact("cacheManifest")),
    vscode.commands.registerCommand("englang.openTestResults", () => openLastRunArtifact("testResults")),
    vscode.commands.registerCommand("englang.openPlotSpec", () => openLastRunArtifact("plotSpec")),
    vscode.commands.registerCommand("englang.openPlotManifest", () => openLastRunArtifact("plotManifest")),
    vscode.commands.registerCommand("englang.openPlotSvg", () => openLastRunArtifact("plotSvg")),
    vscode.commands.registerCommand("englang.showSemanticTokensDebug", () => showSemanticTokensDebug(context)),
    vscode.languages.registerHoverProvider(
      LANGUAGE_ID,
      new EngHoverProvider(context, {
        isEngDocument,
        snapshotDocumentSource,
        cachedSnapshotForDocument: (document) => reviewCache.get(document.uri.fsPath),
        cacheSnapshotForDocument: (document, snapshot) => reviewCache.set(document.uri.fsPath, snapshot)
      })
    ),
    vscode.languages.registerCompletionItemProvider(
      LANGUAGE_ID,
      new EngCompletionProvider(context, {
        completionSeed: COMPLETION_SEED,
        completionSnapshotForPosition,
        cachedSnapshotForDocument: (document) => reviewCache.get(document.uri.fsPath)
      }),
      ":",
      " ",
      "[",
      "."
    ),
    vscode.languages.registerDocumentSemanticTokensProvider(
      LANGUAGE_ID,
      new EngSemanticTokensProvider(context, {
        isEngDocument,
        snapshotDocumentSource,
        cacheSnapshotForDocument: (document, snapshot) => reviewCache.set(document.uri.fsPath, snapshot),
        updateSemanticSymbolDecorations,
        semanticLegend,
        semanticTokenTypes: SEMANTIC_TOKEN_TYPES,
        semanticTokenModifiers: SEMANTIC_TOKEN_MODIFIERS
      }),
      semanticLegend
    ),
    vscode.languages.registerDocumentSymbolProvider(
      LANGUAGE_ID,
      new EngDocumentSymbolProvider(context, {
        isEngDocument,
        snapshotDocumentSource,
        cacheSnapshotForDocument: (document, snapshot) => reviewCache.set(document.uri.fsPath, snapshot)
      })
    ),
    vscode.languages.registerWorkspaceSymbolProvider(
      new EngWorkspaceSymbolProvider(context, {
        workspaceSymbolsForQuery,
        appendOutputLine
      })
    ),
    vscode.languages.registerDefinitionProvider(
      LANGUAGE_ID,
      new EngDefinitionProvider(context, {
        isEngDocument,
        definitionSnapshotForPosition,
        snapshotDocumentSource,
        cachedSnapshotForDocument: (document) => reviewCache.get(document.uri.fsPath),
        cacheSnapshotForDocument: (document, snapshot) => reviewCache.set(document.uri.fsPath, snapshot),
        appendOutputLine
      })
    ),
    vscode.languages.registerFoldingRangeProvider(
      LANGUAGE_ID,
      new EngFoldingRangeProvider(context, {
        isEngDocument,
        snapshotDocumentSource,
        cacheSnapshotForDocument: (document, snapshot) => reviewCache.set(document.uri.fsPath, snapshot)
      })
    ),
    vscode.languages.registerDocumentFormattingEditProvider(
      LANGUAGE_ID,
      new EngFormattingProvider(context, {
        isEngDocument,
        formatDocumentSource
      })
    ),
    vscode.languages.registerCodeActionsProvider(
      LANGUAGE_ID,
      new EngCodeActionProvider(context, { codeActionsForDocumentSource }),
      {
        providedCodeActionKinds: [vscode.CodeActionKind.QuickFix]
      }
    )
  );

  for (const document of vscode.workspace.textDocuments) {
    diagnosticController.maybeCheck(document);
  }
}

function deactivate() {}

function createReviewRiskDecorationTypes() {
  return {
    high: vscode.window.createTextEditorDecorationType({
      isWholeLine: true,
      borderWidth: "0 0 0 2px",
      borderStyle: "solid",
      borderColor: new vscode.ThemeColor("editorError.foreground"),
      overviewRulerColor: new vscode.ThemeColor("editorError.foreground"),
      overviewRulerLane: vscode.OverviewRulerLane.Right
    }),
    medium: vscode.window.createTextEditorDecorationType({
      isWholeLine: true,
      borderWidth: "0 0 0 2px",
      borderStyle: "solid",
      borderColor: new vscode.ThemeColor("editorWarning.foreground"),
      overviewRulerColor: new vscode.ThemeColor("editorWarning.foreground"),
      overviewRulerLane: vscode.OverviewRulerLane.Right
    })
  };
}

function createSemanticSymbolDecorationTypes() {
  return {
    internal: vscode.window.createTextEditorDecorationType({
      textDecoration: "underline dotted",
      opacity: "0.85"
    }),
    planned: vscode.window.createTextEditorDecorationType({
      textDecoration: "underline dotted",
      opacity: "0.75"
    })
  };
}

function clearSnapshotCache(document) {
  snapshotPromiseCache.delete(snapshotCacheKey(document));
}

function updateReviewRiskDecorations(document, review) {
  if (!reviewRiskDecorations || !isEngDocument(document)) {
    return;
  }
  const editors = vscode.window.visibleTextEditors.filter(
    (editor) => editor.document.uri.toString() === document.uri.toString()
  );
  if (editors.length === 0) {
    return;
  }
  const config = vscode.workspace.getConfiguration("englang", document.uri);
  const decorations = config.get("reviewRiskDecorations.enabled", true)
    ? reviewRiskDecorationOptions(document, review)
    : { high: [], medium: [] };
  for (const editor of editors) {
    editor.setDecorations(reviewRiskDecorations.high, decorations.high);
    editor.setDecorations(reviewRiskDecorations.medium, decorations.medium);
  }
}

function refreshVisibleReviewRiskDecorations() {
  for (const editor of vscode.window.visibleTextEditors) {
    if (isEngDocument(editor.document)) {
      const cached = reviewCache.get(editor.document.uri.fsPath);
      updateReviewRiskDecorations(
        editor.document,
        cached
      );
      updateSemanticSymbolDecorations(editor.document, cached);
    }
  }
}

function updateSemanticSymbolDecorations(document, snapshot) {
  if (!semanticSymbolDecorations || !isEngDocument(document)) {
    return;
  }
  const editors = vscode.window.visibleTextEditors.filter(
    (editor) => editor.document.uri.toString() === document.uri.toString()
  );
  if (editors.length === 0) {
    return;
  }
  const decorations = semanticSymbolDecorationOptions(document, snapshot);
  for (const editor of editors) {
    editor.setDecorations(semanticSymbolDecorations.internal, decorations.internal);
    editor.setDecorations(semanticSymbolDecorations.planned, decorations.planned);
  }
}

function semanticSymbolDecorationOptions(document, snapshot) {
  const internal = [];
  const planned = [];
  for (const token of snapshot?.semantic_tokens?.tokens ?? snapshot?.semanticTokens?.tokens ?? []) {
    const modifiers = token.modifiers ?? [];
    const isInternal = modifiers.includes("internal");
    const isPlanned = modifiers.includes("planned");
    if (!isInternal && !isPlanned) {
      continue;
    }
    const range = semanticTokenRange(document, token);
    if (!range) {
      continue;
    }
    const option = {
      range,
      hoverMessage: semanticSymbolHoverMessage(isPlanned ? "planned" : "internal")
    };
    if (isPlanned) {
      planned.push(option);
    } else {
      internal.push(option);
    }
  }
  return { internal, planned };
}

function semanticSymbolHoverMessage(kind) {
  const markdown = new vscode.MarkdownString();
  markdown.isTrusted = false;
  if (kind === "planned") {
    markdown.appendMarkdown("**EngLang planned symbol**\n\nThis symbol is reserved for a planned workflow surface.");
  } else {
    markdown.appendMarkdown("**EngLang internal symbol**\n\nThis symbol belongs to an internal runtime or bundled stdlib boundary.");
  }
  return markdown;
}

function reviewRiskDecorationOptions(document, review) {
  const doc = normalizedReviewDocument(review);
  const records = [
    ...firstReviewArray(doc, review, "risks"),
    ...firstReviewArray(doc, review, "fallbacks")
  ];
  const byLine = new Map();
  for (const record of records) {
    const lineNumber = reviewRiskLineNumber(record);
    setReviewRiskDecorationLine(byLine, document, lineNumber, reviewRiskLevel(record), record);
  }
  for (const token of review?.semantic_tokens?.tokens ?? review?.semanticTokens?.tokens ?? []) {
    const modifiers = token.modifiers ?? [];
    let level = "";
    if (modifiers.includes("riskHigh")) {
      level = "high";
    } else if (modifiers.includes("riskMedium")) {
      level = "medium";
    }
    setReviewRiskDecorationLine(byLine, document, Number(token.line) + 1, level, {
      category: "semantic token",
      summary: `Compiler semantic metadata marked this line as ${level} review risk.`
    });
  }

  const high = [];
  const medium = [];
  for (const [lineNumber, item] of byLine.entries()) {
    const option = {
      range: fullLineRange(document, lineNumber - 1),
      hoverMessage: reviewRiskHoverMessage(item.level, item.record)
    };
    if (item.level === "high") {
      high.push(option);
    } else {
      medium.push(option);
    }
  }
  return { high, medium };
}

function setReviewRiskDecorationLine(byLine, document, lineNumber, level, record) {
  if (!Number.isFinite(lineNumber) || lineNumber < 1 || lineNumber > document.lineCount) {
    return;
  }
  if (level !== "high" && level !== "medium") {
    return;
  }
  const safeLine = Math.trunc(lineNumber);
  const existing = byLine.get(safeLine);
  if (existing?.level === "high" && level === "medium") {
    return;
  }
  byLine.set(safeLine, { level, record });
}

function reviewRiskLineNumber(record) {
  const raw = lineValue(record);
  const lineNumber = Number(raw);
  return Number.isFinite(lineNumber) ? Math.trunc(lineNumber) : NaN;
}

function reviewRiskLevel(record) {
  return String(record?.level ?? record?.risk_level ?? record?.riskLevel ?? "").toLowerCase();
}

function reviewRiskHoverMessage(level, record) {
  const title = level === "high" ? "High review risk" : "Medium review risk";
  const summary =
    reviewValue(record, "summary", "summary", "") ||
    reviewValue(record, "assumption", "assumption", "") ||
    reviewValue(record, "reason", "reason", "") ||
    reviewValue(record, "method", "method", "");
  const category =
    reviewValue(record, "category", "category", "") ||
    reviewValue(record, "kind", "kind", "") ||
    "review";
  const markdown = new vscode.MarkdownString();
  markdown.isTrusted = false;
  markdown.appendMarkdown(`**EngLang ${title}**\n\n`);
  markdown.appendMarkdown(`Category: \`${category}\``);
  if (summary) {
    markdown.appendMarkdown(`\n\n${summary}`);
  }
  return markdown;
}

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
      openLastRunArtifact(message.artifactId, result.document).catch((error) => {
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

async function openLastRunArtifactPicker() {
  const picked = await vscode.window.showQuickPick(
    LAST_RUN_ARTIFACTS.map((artifact) => ({
      label: artifact.label,
      description: artifact.description,
      artifact
    })),
    { placeHolder: "Open an artifact from build/result" }
  );
  if (picked) {
    await openLastRunArtifact(picked.artifact.id);
  }
}

async function openGeneratedOutputArtifactPicker() {
  const root = currentWorkspaceRoot();
  if (!root) {
    vscode.window.showWarningMessage("Open an EngLang workspace folder first.");
    return;
  }
  const manifestPath = path.join(root, "build", "result", "output_manifest.json");
  if (!fs.existsSync(manifestPath)) {
    vscode.window.showWarningMessage("No build/result/output_manifest.json found yet. Run the current file first.");
    return;
  }

  let manifest;
  try {
    manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  } catch (error) {
    vscode.window.showWarningMessage(`Could not read output_manifest.json: ${error.message}`);
    return;
  }

  const artifacts = outputManifestArtifactItems(manifest, root);
  if (artifacts.length === 0) {
    vscode.window.showWarningMessage("The last output_manifest.json does not list any existing generated files.");
    return;
  }
  const picked = await vscode.window.showQuickPick(artifacts, {
    placeHolder: "Open a generated file from the last run"
  });
  if (!picked) {
    return;
  }
  const uri = vscode.Uri.file(picked.filePath);
  if (picked.external) {
    await vscode.env.openExternal(uri);
    return;
  }
  const document = await vscode.workspace.openTextDocument(uri);
  await vscode.window.showTextDocument(document, { preview: false });
}

async function openLastRunArtifact(artifactId, sourceDocument = undefined) {
  const artifact = LAST_RUN_ARTIFACTS.find((item) => item.id === artifactId);
  if (!artifact) {
    vscode.window.showWarningMessage(`Unknown EngLang artifact: ${artifactId}`);
    return;
  }
  const root = sourceDocument ? workspaceRoot(sourceDocument) : currentWorkspaceRoot();
  if (!root) {
    vscode.window.showWarningMessage("Open an EngLang workspace folder first.");
    return;
  }
  const artifactPath = path.join(root, ...artifact.relativePath);
  if (!fs.existsSync(artifactPath)) {
    vscode.window.showWarningMessage(`No ${artifact.description} found yet. Run the current file first.`);
    return;
  }
  const uri = vscode.Uri.file(artifactPath);
  if (artifact.external) {
    await vscode.env.openExternal(uri);
    return;
  }
  const document = await vscode.workspace.openTextDocument(uri);
  await vscode.window.showTextDocument(document, { preview: false });
}

function outputManifestArtifactItems(manifest, root) {
  const outputDir = outputManifestOutputDir(manifest, root);
  const artifacts = Array.isArray(manifest?.artifacts) ? manifest.artifacts : [];
  const seen = new Set();
  const items = [];
  for (const artifact of artifacts) {
    if (!artifact || typeof artifact !== "object") {
      continue;
    }
    const manifestPath = String(artifact.path ?? "").trim();
    if (!manifestPath) {
      continue;
    }
    const filePath = resolveOutputManifestPath(manifestPath, outputDir, root);
    if (!filePath || seen.has(filePath) || !fs.existsSync(filePath)) {
      continue;
    }
    seen.add(filePath);
    const kind = String(artifact.kind ?? "artifact");
    const artifactClass = String(artifact.class ?? "").trim();
    const status = String(artifact.status ?? "").trim();
    items.push({
      label: outputManifestArtifactLabel(kind),
      description: relativeDisplayPath(root, filePath),
      detail: [artifactClass, status].filter(Boolean).join(" | "),
      filePath,
      external: shouldOpenArtifactExternally(filePath)
    });
  }
  return items.sort((left, right) => {
    const pathOrder = left.description.localeCompare(right.description);
    return pathOrder !== 0 ? pathOrder : left.label.localeCompare(right.label);
  });
}

function outputManifestOutputDir(manifest, root) {
  const outputDir = String(manifest?.output_dir ?? "").trim();
  if (!outputDir) {
    return path.join(root, "build", "result");
  }
  if (path.isAbsolute(outputDir)) {
    return outputDir;
  }
  return path.join(root, outputDir);
}

function resolveOutputManifestPath(manifestPath, outputDir, root) {
  if (path.isAbsolute(manifestPath)) {
    return manifestPath;
  }
  const normalized = manifestPath.replaceAll("\\", "/");
  if (normalized === "build" || normalized.startsWith("build/")) {
    return path.join(root, ...normalized.split("/"));
  }
  return path.join(outputDir, ...normalized.split("/"));
}

function outputManifestArtifactLabel(kind) {
  return kind
    .split("_")
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function relativeDisplayPath(root, filePath) {
  const relative = path.relative(root, filePath);
  return relative && !relative.startsWith("..") ? relative : filePath;
}

function shouldOpenArtifactExternally(filePath) {
  const extension = path.extname(filePath).toLowerCase();
  return extension === ".html" || extension === ".svg";
}

async function showSemanticTokensDebug(context) {
  const document = vscode.window.activeTextEditor?.document;
  if (!document || !isEngDocument(document)) {
    vscode.window.showWarningMessage("Open an EngLang .eng file first.");
    return;
  }
  const snapshot = await snapshotDocumentSource(document, context);
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

function snapshotDocumentSource(document, context, cancellationToken) {
  const key = snapshotCacheKey(document);
  const cached = snapshotPromiseCache.get(key);
  if (cached) {
    return cached;
  }

  const promise = new Promise((resolve) => {
    const runtime = findLspRuntime(context, document);
    const cwd = workspaceRoot(document);
    const documentVersion = document.version;
    let settled = false;
    const finish = (value) => {
      if (settled) {
        return;
      }
      settled = true;
      if (document.version !== documentVersion) {
        snapshotPromiseCache.delete(key);
        resolve(undefined);
        return;
      }
      resolve(value);
    };

    const child = cp.execFile(
      runtime,
      ["--snapshot-stdin", document.uri.fsPath],
      { cwd, maxBuffer: 10 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (stderr && stderr.trim().length > 0) {
          output.appendLine(stderr.trim());
        }
        if (error) {
          output.appendLine(`Live editor check failed: ${error.message}`);
          finish(undefined);
          return;
        }
        try {
          finish(JSON.parse(stdout));
        } catch (parseError) {
          output.appendLine(`Unable to parse EngLang live editor data: ${parseError.message}`);
          finish(undefined);
        }
      }
    );

    if (child.stdin) {
      child.stdin.end(document.getText());
    }
  });
  snapshotPromiseCache.set(key, promise);
  promise.finally(() => {
    if (snapshotPromiseCache.get(key) === promise) {
      snapshotPromiseCache.delete(key);
    }
  });
  return promise;
}

function snapshotCacheKey(document) {
  return `${document.uri.toString()}@${document.version}`;
}

async function workspaceSymbolsForQuery(query, context, cancellationToken) {
  const folders = (vscode.workspace.workspaceFolders ?? [])
    .filter((folder) => folder.uri.scheme === "file");
  if (folders.length === 0 || cancellationToken?.isCancellationRequested) {
    return [];
  }

  const results = await Promise.all(
    folders.map((folder) => workspaceSymbolsForFolder(folder, query, context, cancellationToken))
  );
  const seen = new Set();
  const symbols = [];
  for (const symbol of results.flat()) {
    const uri = symbol?.location?.uri ?? "";
    const line = symbol?.location?.range?.start?.line ?? 0;
    const key = `${symbol?.name ?? ""}\n${uri}\n${line}`;
    if (!symbol?.name || seen.has(key)) {
      continue;
    }
    seen.add(key);
    symbols.push(symbol);
  }
  return symbols;
}

function workspaceSymbolsForFolder(folder, query, context, cancellationToken) {
  return new Promise((resolve) => {
    if (cancellationToken?.isCancellationRequested) {
      resolve([]);
      return;
    }

    const root = folder.uri.fsPath;
    const runtime = findLspRuntimeForRoot(context, root);
    let settled = false;
    const finish = (value) => {
      if (settled) {
        return;
      }
      settled = true;
      resolve(value);
    };

    const child = cp.execFile(
      runtime,
      ["--workspace-symbols", root, query ?? ""],
      { cwd: root, maxBuffer: 10 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (settled) {
          return;
        }
        if (stderr && stderr.trim().length > 0) {
          output.appendLine(stderr.trim());
        }
        if (error) {
          output.appendLine(`workspace symbol lookup failed: ${error.message}`);
          finish([]);
          return;
        }
        try {
          const payload = JSON.parse(stdout);
          const symbols = Array.isArray(payload)
            ? payload
            : (Array.isArray(payload.symbols) ? payload.symbols : []);
          finish(symbols);
        } catch (parseError) {
          output.appendLine(`Unable to parse EngLang workspace symbols: ${parseError.message}`);
          finish([]);
        }
      }
    );

    cancellationToken?.onCancellationRequested(() => {
      child.kill();
      finish([]);
    });
  });
}

function completionSnapshotForPosition(document, position, context, cancellationToken) {
  return new Promise((resolve) => {
    if (!isEngDocument(document)) {
      resolve(undefined);
      return;
    }

    const runtime = findLspRuntime(context, document);
    const cwd = workspaceRoot(document);
    let settled = false;
    const finish = (value) => {
      if (settled) {
        return;
      }
      settled = true;
      resolve(value);
    };

    const child = cp.execFile(
      runtime,
      [
        "--completion-stdin",
        document.uri.fsPath,
        String(position.line),
        String(position.character)
      ],
      { cwd, maxBuffer: 10 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (stderr && stderr.trim().length > 0) {
          output.appendLine(stderr.trim());
        }
        if (error) {
          output.appendLine(`Completion lookup failed: ${error.message}`);
          finish(undefined);
          return;
        }
        try {
          const payload = JSON.parse(stdout);
          if (Array.isArray(payload)) {
            finish({ completions: payload });
            return;
          }
          finish(payload);
        } catch (parseError) {
          output.appendLine(`Unable to parse EngLang completion data: ${parseError.message}`);
          finish(undefined);
        }
      }
    );

    cancellationToken?.onCancellationRequested(() => {
      child.kill();
      finish(undefined);
    });

    if (child.stdin) {
      child.stdin.end(document.getText());
    }
  });
}

function definitionSnapshotForPosition(document, position, context, cancellationToken) {
  return new Promise((resolve) => {
    if (!isEngDocument(document)) {
      resolve(undefined);
      return;
    }

    const runtime = findLspRuntime(context, document);
    const cwd = workspaceRoot(document);
    let settled = false;
    const finish = (value) => {
      if (settled) {
        return;
      }
      settled = true;
      resolve(value);
    };

    const child = cp.execFile(
      runtime,
      [
        "--definition-stdin",
        document.uri.fsPath,
        String(position.line),
        String(position.character)
      ],
      { cwd, maxBuffer: 10 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (stderr && stderr.trim().length > 0) {
          output.appendLine(stderr.trim());
        }
        if (error) {
          output.appendLine(`Definition lookup failed: ${error.message}`);
          finish(undefined);
          return;
        }
        try {
          finish(JSON.parse(stdout));
        } catch (parseError) {
          output.appendLine(`Unable to parse EngLang definition data: ${parseError.message}`);
          finish(undefined);
        }
      }
    );

    cancellationToken?.onCancellationRequested(() => {
      child.kill();
      finish(undefined);
    });

    if (child.stdin) {
      child.stdin.end(document.getText());
    }
  });
}

function formatDocumentSource(document, context, cancellationToken) {
  return new Promise((resolve) => {
    if (!isEngDocument(document)) {
      resolve(undefined);
      return;
    }

    const runtime = findLspRuntime(context, document);
    const cwd = workspaceRoot(document);
    let settled = false;
    const finish = (value) => {
      if (settled) {
        return;
      }
      settled = true;
      resolve(value);
    };

    const child = cp.execFile(
      runtime,
      ["--format-stdin", document.uri.fsPath],
      { cwd, maxBuffer: 10 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (stderr && stderr.trim().length > 0) {
          output.appendLine(stderr.trim());
        }
        if (error) {
          output.appendLine(`formatting failed: ${error.message}`);
          finish(undefined);
          return;
        }
        try {
          finish(JSON.parse(stdout));
        } catch (parseError) {
          output.appendLine(`Unable to parse EngLang formatting result: ${parseError.message}`);
          finish(undefined);
        }
      }
    );

    cancellationToken?.onCancellationRequested(() => {
      child.kill();
      finish(undefined);
    });

    if (child.stdin) {
      child.stdin.end(document.getText());
    }
  });
}

function codeActionsForDocumentSource(document, context, cancellationToken) {
  return new Promise((resolve) => {
    if (!isEngDocument(document)) {
      resolve(undefined);
      return;
    }

    const runtime = findLspRuntime(context, document);
    const cwd = workspaceRoot(document);
    let settled = false;
    const finish = (value) => {
      if (settled) {
        return;
      }
      settled = true;
      resolve(value);
    };

    const child = cp.execFile(
      runtime,
      ["--code-actions-stdin", document.uri.fsPath],
      { cwd, maxBuffer: 10 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (stderr && stderr.trim().length > 0) {
          output.appendLine(stderr.trim());
        }
        if (error) {
          output.appendLine(`code action lookup failed: ${error.message}`);
          finish(undefined);
          return;
        }
        try {
          finish(JSON.parse(stdout));
        } catch (parseError) {
          output.appendLine(`Unable to parse EngLang code actions: ${parseError.message}`);
          finish(undefined);
        }
      }
    );

    cancellationToken?.onCancellationRequested(() => {
      child.kill();
      finish(undefined);
    });

    if (child.stdin) {
      child.stdin.end(document.getText());
    }
  });
}

function appendOutputLine(message) {
  output?.appendLine(message);
}

function fullLineRange(document, lineNumber) {
  const line = document.lineAt(lineNumber);
  if (lineNumber + 1 < document.lineCount) {
    return new vscode.Range(lineNumber, 0, lineNumber + 1, 0);
  }
  return new vscode.Range(lineNumber, 0, lineNumber, line.text.length);
}

function isEngDocument(document) {
  return document.languageId === LANGUAGE_ID || document.fileName.endsWith(".eng");
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

function diagnosticsBackend(document) {
  const source = problemsSource(document);
  return source === "live" ? "lsp-snapshot" : "eng-cli";
}

function diagnosticsBackendLabel(backend) {
  return backend === "lsp-snapshot" ? "live editor" : "file";
}

function problemsSource(document) {
  const config = engConfig(document);
  const configuredSource = explicitlyConfiguredEngValue(config, "problemsSource");
  if (configuredSource === "file" || configuredSource === "live") {
    return configuredSource;
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
    "workspaceFolderLanguageValue",
    "workspaceFolderValue",
    "workspaceLanguageValue",
    "workspaceValue",
    "globalLanguageValue",
    "globalValue"
  ]) {
    if (inspection[scope] !== undefined) {
      return inspection[scope];
    }
  }
  return undefined;
}

module.exports = {
  activate,
  deactivate
};
