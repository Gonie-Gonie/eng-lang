const vscode = require("vscode");
const { createArtifactOpeners } = require("./artifactOpeners");
const { createCommandHandlers } = require("./commandHandlers");
const { createDecorationController } = require("./decorations");
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
const { createSemanticLegend } = require("./lspSemanticTokens");

const LANGUAGE_ID = "englang";
const reviewCache = new Map();
let output;
let artifactOpeners;
let decorationController;
let lspRequests;
let commandHandlers;

const editorMetadata = loadEditorMetadata(__dirname);
const SEMANTIC_TOKEN_TYPES = editorMetadata.semanticTokenTypes;
const SEMANTIC_TOKEN_MODIFIERS = editorMetadata.semanticTokenModifiers;
const COMPLETION_SEED = editorMetadata.completionSeed;
const HTTP_RESPONSE_FIELDS = editorMetadata.syntaxCatalog.http_response_fields;
const SAMPLE_TABLE_FIELDS = editorMetadata.syntaxCatalog.sample_table_fields;
const CASE_TABLE_FIELDS = editorMetadata.syntaxCatalog.case_table_fields;
const CASE_OUTPUT_TABLE_FIELDS = editorMetadata.syntaxCatalog.case_output_table_fields;

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
  decorationController = createDecorationController({
    isEngDocument,
    reviewCache
  });
  commandHandlers = createCommandHandlers({
    output,
    reviewCache,
    artifactOpeners,
    lspRequests,
    isEngDocument,
    updateSemanticSymbolDecorations: decorationController.updateSemanticSymbolDecorations
  });
  const diagnostics = vscode.languages.createDiagnosticCollection("englang");
  const diagnosticController = new EngDiagnosticsController(context, diagnostics, {
    output,
    isEngDocument,
    clearSnapshotCache: lspRequests.clearSnapshotCache,
    diagnosticsBackend,
    diagnosticsBackendLabel,
    findLspRuntime,
    findRuntime,
    snapshotDocumentSource: lspRequests.snapshotDocumentSource,
    workspaceRoot,
    cacheReview: (document, review) => reviewCache.set(document.uri.fsPath, review),
    updateReviewRiskDecorations: decorationController.updateReviewRiskDecorations,
    updateSemanticSymbolDecorations: decorationController.updateSemanticSymbolDecorations
  });
  context.subscriptions.push(output, diagnostics);
  context.subscriptions.push(...decorationController.disposables);

  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((document) => diagnosticController.maybeCheck(document)),
    vscode.workspace.onDidChangeTextDocument((event) => diagnosticController.scheduleChangedCheck(event.document)),
    vscode.workspace.onDidSaveTextDocument((document) => diagnosticController.maybeCheck(document)),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration("englang.reviewRiskDecorations.enabled")) {
        decorationController.refreshVisibleReviewRiskDecorations();
      }
    }),
    vscode.workspace.onDidCloseTextDocument((document) => {
      diagnosticController.clearPendingCheck(document);
      lspRequests.clearSnapshotCache(document);
      diagnostics.delete(document.uri);
      decorationController.updateReviewRiskDecorations(document, undefined);
      decorationController.updateSemanticSymbolDecorations(document, undefined);
    }),
    vscode.window.onDidChangeActiveTextEditor((editor) => {
      if (editor && isEngDocument(editor.document)) {
        const cached = reviewCache.get(editor.document.uri.fsPath);
        decorationController.updateReviewRiskDecorations(editor.document, cached);
        decorationController.updateSemanticSymbolDecorations(editor.document, cached);
      }
    }),
    vscode.commands.registerCommand("englang.checkFile", () => diagnosticController.checkActiveFile()),
    vscode.commands.registerCommand("englang.runFile", () => commandHandlers.runActiveFile(context)),
    vscode.commands.registerCommand("englang.runExample", () => commandHandlers.runExample(context)),
    vscode.commands.registerCommand("englang.switchProfile", () => commandHandlers.switchExecutionProfile()),
    vscode.commands.registerCommand("englang.switchProblemsSource", () => commandHandlers.switchProblemsSource()),
    vscode.commands.registerCommand("englang.showToolingStatus", () => commandHandlers.showToolingStatus(context)),
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
        httpResponseFields: HTTP_RESPONSE_FIELDS,
        sampleTableFields: SAMPLE_TABLE_FIELDS,
        caseTableFields: CASE_TABLE_FIELDS,
        caseOutputTableFields: CASE_OUTPUT_TABLE_FIELDS,
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
        updateSemanticSymbolDecorations: decorationController.updateSemanticSymbolDecorations,
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
        codeActionsForDocumentSource: lspRequests.codeActionsForDocumentSource,
        completionSeed: COMPLETION_SEED
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

function appendOutputLine(message) {
  output?.appendLine(message);
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
