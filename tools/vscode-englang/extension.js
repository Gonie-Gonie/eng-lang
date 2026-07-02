const vscode = require("vscode");
const { createArtifactOpeners } = require("./artifactOpeners");
const { createCommandHandlers } = require("./commandHandlers");
const { createLspRequests } = require("./lspRequests");
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
const {
  currentWorkspaceRoot,
  engConfig,
  findLspRuntime,
  findLspRuntimeForRoot,
  findRuntime,
  workspaceRoot
} = require("./runtimeDiscovery");
const {
  createSemanticLegend,
  semanticTokenRange
} = require("./lspSemanticTokens");
const {
  firstReviewArray,
  lineValue,
  normalizedReviewDocument,
  reviewValue
} = require("./reviewPanelRenderer");

const LANGUAGE_ID = "englang";
const reviewCache = new Map();
let output;
let reviewRiskDecorations;
let semanticSymbolDecorations;
let artifactOpeners;
let lspRequests;
let commandHandlers;

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
  artifactOpeners = createArtifactOpeners({ currentWorkspaceRoot, workspaceRoot });
  lspRequests = createLspRequests({
    isEngDocument,
    findLspRuntime,
    findLspRuntimeForRoot,
    workspaceRoot,
    appendOutputLine
  });
  commandHandlers = createCommandHandlers({
    output,
    reviewCache,
    artifactOpeners,
    lspRequests,
    isEngDocument,
    updateSemanticSymbolDecorations
  });
  const diagnostics = vscode.languages.createDiagnosticCollection("englang");
  reviewRiskDecorations = createReviewRiskDecorationTypes();
  semanticSymbolDecorations = createSemanticSymbolDecorationTypes();
  const diagnosticController = new EngDiagnosticsController(context, diagnostics, {
    output,
    isEngDocument,
    clearSnapshotCache: lspRequests.clearSnapshotCache,
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
      lspRequests.clearSnapshotCache(document);
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
    vscode.commands.registerCommand("englang.runFile", () => commandHandlers.runActiveFile(context)),
    vscode.commands.registerCommand("englang.runExample", () => commandHandlers.runExample(context)),
    vscode.commands.registerCommand("englang.switchProfile", () => commandHandlers.switchExecutionProfile()),
    vscode.commands.registerCommand("englang.reviewFile", () => commandHandlers.reviewActiveFile(context)),
    vscode.commands.registerCommand("englang.openReviewPanel", () => commandHandlers.openReviewPanel(context)),
    vscode.commands.registerCommand("englang.openReport", () => artifactOpeners.openLastRunArtifact("report")),
    vscode.commands.registerCommand("englang.openLastArtifact", () => artifactOpeners.openLastRunArtifactPicker()),
    vscode.commands.registerCommand("englang.openGeneratedOutput", () => artifactOpeners.openGeneratedOutputArtifactPicker()),
    vscode.commands.registerCommand("englang.openReviewJson", () => artifactOpeners.openLastRunArtifact("review")),
    vscode.commands.registerCommand("englang.openResultArtifact", () => artifactOpeners.openLastRunArtifact("result")),
    vscode.commands.registerCommand("englang.openReportSpec", () => artifactOpeners.openLastRunArtifact("reportSpec")),
    vscode.commands.registerCommand("englang.openOutputManifest", () => artifactOpeners.openLastRunArtifact("outputManifest")),
    vscode.commands.registerCommand("englang.openRunLog", () => artifactOpeners.openLastRunArtifact("runLog")),
    vscode.commands.registerCommand("englang.openStaticRunPlan", () => artifactOpeners.openLastRunArtifact("staticRunPlan")),
    vscode.commands.registerCommand("englang.openRunPlan", () => artifactOpeners.openLastRunArtifact("runPlan")),
    vscode.commands.registerCommand("englang.openRunLock", () => artifactOpeners.openLastRunArtifact("runLock")),
    vscode.commands.registerCommand("englang.openProcessResults", () => artifactOpeners.openLastRunArtifact("processResults")),
    vscode.commands.registerCommand("englang.openCacheManifest", () => artifactOpeners.openLastRunArtifact("cacheManifest")),
    vscode.commands.registerCommand("englang.openTestResults", () => artifactOpeners.openLastRunArtifact("testResults")),
    vscode.commands.registerCommand("englang.openPlotSpec", () => artifactOpeners.openLastRunArtifact("plotSpec")),
    vscode.commands.registerCommand("englang.openPlotManifest", () => artifactOpeners.openLastRunArtifact("plotManifest")),
    vscode.commands.registerCommand("englang.openPlotSvg", () => artifactOpeners.openLastRunArtifact("plotSvg")),
    vscode.commands.registerCommand("englang.showSemanticTokensDebug", () => commandHandlers.showSemanticTokensDebug(context)),
    vscode.languages.registerHoverProvider(
      LANGUAGE_ID,
      new EngHoverProvider(context, {
        isEngDocument,
        snapshotDocumentSource: lspRequests.snapshotDocumentSource,
        cachedSnapshotForDocument: (document) => reviewCache.get(document.uri.fsPath),
        cacheSnapshotForDocument: (document, snapshot) => reviewCache.set(document.uri.fsPath, snapshot)
      })
    ),
    vscode.languages.registerCompletionItemProvider(
      LANGUAGE_ID,
      new EngCompletionProvider(context, {
        completionSeed: COMPLETION_SEED,
        completionSnapshotForPosition: lspRequests.completionSnapshotForPosition,
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
        snapshotDocumentSource: lspRequests.snapshotDocumentSource,
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
        snapshotDocumentSource: lspRequests.snapshotDocumentSource,
        cacheSnapshotForDocument: (document, snapshot) => reviewCache.set(document.uri.fsPath, snapshot)
      })
    ),
    vscode.languages.registerWorkspaceSymbolProvider(
      new EngWorkspaceSymbolProvider(context, {
        workspaceSymbolsForQuery: lspRequests.workspaceSymbolsForQuery,
        appendOutputLine
      })
    ),
    vscode.languages.registerDefinitionProvider(
      LANGUAGE_ID,
      new EngDefinitionProvider(context, {
        isEngDocument,
        definitionSnapshotForPosition: lspRequests.definitionSnapshotForPosition,
        snapshotDocumentSource: lspRequests.snapshotDocumentSource,
        cachedSnapshotForDocument: (document) => reviewCache.get(document.uri.fsPath),
        cacheSnapshotForDocument: (document, snapshot) => reviewCache.set(document.uri.fsPath, snapshot),
        appendOutputLine
      })
    ),
    vscode.languages.registerFoldingRangeProvider(
      LANGUAGE_ID,
      new EngFoldingRangeProvider(context, {
        isEngDocument,
        snapshotDocumentSource: lspRequests.snapshotDocumentSource,
        cacheSnapshotForDocument: (document, snapshot) => reviewCache.set(document.uri.fsPath, snapshot)
      })
    ),
    vscode.languages.registerDocumentFormattingEditProvider(
      LANGUAGE_ID,
      new EngFormattingProvider(context, {
        isEngDocument,
        formatDocumentSource: lspRequests.formatDocumentSource
      })
    ),
    vscode.languages.registerCodeActionsProvider(
      LANGUAGE_ID,
      new EngCodeActionProvider(context, {
        codeActionsForDocumentSource: lspRequests.codeActionsForDocumentSource
      }),
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
