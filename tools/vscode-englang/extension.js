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
const COMPLETION_ITEMS = editorMetadata.completionItems;
const UNIT_LABELS = catalogItemLabels(editorMetadata.syntaxCatalog.units);
const WORKFLOW_OPTION_LABELS = catalogItemLabels(editorMetadata.syntaxCatalog.workflow_options);
const HTTP_RESPONSE_FIELDS = editorMetadata.syntaxCatalog.http_response_fields;
const SAMPLE_TABLE_FIELDS = editorMetadata.syntaxCatalog.sample_table_fields;
const DB_CONNECTION_FIELDS = editorMetadata.syntaxCatalog.db_connection_fields;
const CASE_TABLE_FIELDS = editorMetadata.syntaxCatalog.case_table_fields;
const CASE_OUTPUT_TABLE_FIELDS = editorMetadata.syntaxCatalog.case_output_table_fields;
const CASE_RESULT_COLLECTION_TABLE_FIELDS = editorMetadata.syntaxCatalog.case_result_collection_table_fields;
const MODEL_FIELDS = editorMetadata.syntaxCatalog.model_fields;
const PREDICTION_TABLE_FIELDS = editorMetadata.syntaxCatalog.prediction_table_fields;

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
    semanticTokenTypes: SEMANTIC_TOKEN_TYPES,
    semanticTokenModifiers: SEMANTIC_TOKEN_MODIFIERS,
    updateSemanticSymbolDecorations: decorationController.updateSemanticSymbolDecorations
  });
  const diagnostics = vscode.languages.createDiagnosticCollection("englang");
  const diagnosticController = new EngDiagnosticsController(context, diagnostics, {
    output,
    isEngDocument,
    clearSnapshotCache: lspRequests.clearSnapshotCache,
    diagnosticsRuntime,
    diagnosticsRuntimeLabel,
    findLspRuntime,
    findRuntime,
    snapshotDocumentSource: lspRequests.snapshotDocumentSource,
    workspaceRoot,
    cacheReview: (document, review) => reviewCache.set(document.uri.fsPath, review),
    clearCachedReview: (document) => reviewCache.delete(document.uri.fsPath),
    updateReviewRiskDecorations: decorationController.updateReviewRiskDecorations,
    updateSemanticSymbolDecorations: decorationController.updateSemanticSymbolDecorations
  });
  const semanticTokensProvider = new EngSemanticTokensProvider(context, {
    isEngDocument,
    snapshotDocumentSource: lspRequests.snapshotDocumentSource,
    cacheSnapshotForDocument: (document, snapshot) => reviewCache.set(document.uri.fsPath, snapshot),
    updateSemanticSymbolDecorations: decorationController.updateSemanticSymbolDecorations,
    semanticLegend,
    semanticTokenTypes: SEMANTIC_TOKEN_TYPES,
    semanticTokenModifiers: SEMANTIC_TOKEN_MODIFIERS
  });
  function refreshActiveDiagnosticsForSettings(reason = "diagnostics settings changed") {
    const document = vscode.window.activeTextEditor?.document;
    if (!document || !isEngDocument(document)) {
      return;
    }
    const config = engConfig(document);
    const mode = diagnosticsMode(document);
    const lintOnSave = config.get("lintOnSave", true);
    const lintOnChange = config.get("lintOnChange", true);
    if (mode === "live") {
      if (document.isDirty) {
        if (lintOnChange) {
          diagnosticController.checkActiveFile();
        } else {
          diagnosticController.clearDocumentDiagnostics(
            document,
            "live typing diagnostics are disabled; save the file or run EngLang: Check Current File to refresh Problems"
          );
        }
      } else if (lintOnSave) {
        diagnosticController.checkDocument(document);
      } else {
        diagnosticController.clearDocumentDiagnostics(
          document,
          "saved-file diagnostics are disabled for the active EngLang editor"
        );
      }
    } else if (!lintOnSave) {
      diagnosticController.clearDocumentDiagnostics(
        document,
        "saved-file diagnostics are disabled for file mode"
      );
    } else if (document.isDirty) {
      diagnosticController.clearDocumentDiagnostics(
        document,
        "file mode uses saved-file checks; save the file to refresh Problems"
      );
    } else {
      diagnosticController.checkDocument(document);
    }
    output.appendLine(`Diagnostics settings refresh: ${reason}`);
  }

  async function refreshAfterDiagnosticsModeCommand() {
    const mode = await commandHandlers.switchDiagnosticsMode();
    if (!mode) {
      return;
    }
    refreshActiveDiagnosticsForSettings("diagnostics mode command");
  }

  function clearCachedEditorSnapshot(document) {
    if (!document || !isEngDocument(document)) {
      return;
    }
    reviewCache.delete(document.uri.fsPath);
    decorationController.updateReviewRiskDecorations(document, undefined);
    decorationController.updateSemanticSymbolDecorations(document, undefined);
  }

  context.subscriptions.push(output, diagnostics, semanticTokensProvider);
  context.subscriptions.push(...decorationController.disposables);

  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((document) => diagnosticController.maybeCheck(document)),
    vscode.workspace.onDidChangeTextDocument((event) => {
      clearCachedEditorSnapshot(event.document);
      diagnosticController.scheduleChangedCheck(event.document);
    }),
    vscode.workspace.onDidSaveTextDocument((document) => diagnosticController.maybeCheck(document)),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (
        event.affectsConfiguration("englang.diagnosticsMode") ||
        event.affectsConfiguration("englang.lintOnSave") ||
        event.affectsConfiguration("englang.lintOnChange")
      ) {
        refreshActiveDiagnosticsForSettings("diagnostics configuration changed");
      }
      if (event.affectsConfiguration("englang.reviewRiskDecorations.enabled")) {
        decorationController.refreshVisibleReviewRiskDecorations();
      }
      if (event.affectsConfiguration("englang.semanticHighlighting.enabled")) {
        semanticTokensProvider.refresh();
        decorationController.refreshVisibleSemanticSymbolDecorations();
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
    vscode.commands.registerCommand("englang.switchDiagnosticsMode", refreshAfterDiagnosticsModeCommand),
    vscode.commands.registerCommand("englang.switchProblemsSource", refreshAfterDiagnosticsModeCommand),
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
    vscode.commands.registerCommand("englang.showSemanticTokenAtCursor", () => commandHandlers.showSemanticTokenAtCursor(context)),
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
        completionItems: COMPLETION_ITEMS,
        httpResponseFields: HTTP_RESPONSE_FIELDS,
        sampleTableFields: SAMPLE_TABLE_FIELDS,
        dbConnectionFields: DB_CONNECTION_FIELDS,
        caseTableFields: CASE_TABLE_FIELDS,
        caseOutputTableFields: CASE_OUTPUT_TABLE_FIELDS,
        caseResultCollectionTableFields: CASE_RESULT_COLLECTION_TABLE_FIELDS,
        modelFields: MODEL_FIELDS,
        predictionTableFields: PREDICTION_TABLE_FIELDS,
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
      semanticTokensProvider,
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
    vscode.languages.registerDocumentRangeFormattingEditProvider(
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
        completionItems: COMPLETION_ITEMS,
        unitLabels: UNIT_LABELS,
        workflowOptionLabels: WORKFLOW_OPTION_LABELS
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

function catalogItemLabels(items) {
  return (Array.isArray(items) ? items : [])
    .map((item) => {
      if (typeof item === "string") {
        return item;
      }
      return typeof item?.label === "string" ? item.label : undefined;
    })
    .filter((label) => typeof label === "string" && label.length > 0);
}

function appendOutputLine(message) {
  output?.appendLine(message);
}

function isEngDocument(document) {
  return document.languageId === LANGUAGE_ID || document.fileName.endsWith(".eng");
}

function diagnosticsRuntime(document) {
  const mode = diagnosticsMode(document);
  return mode === "live" ? "lsp-snapshot" : "eng-cli";
}

function diagnosticsRuntimeLabel(runtimeMode) {
  return runtimeMode === "lsp-snapshot" ? "live editor" : "file";
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
