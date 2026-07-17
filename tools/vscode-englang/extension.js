const vscode = require("vscode");
const { createArtifactOpeners } = require("./artifactOpeners");
const { createCommandHandlers } = require("./commandHandlers");
const { createDecorationController } = require("./decorations");
const { createLspRequests } = require("./lspRequests");
const {
  PersistentLspClient,
  createPersistentLspRequests
} = require("./persistentLspClient");
const { EngCompletionProvider } = require("./completionProvider");
const { EngDiagnosticsController } = require("./diagnosticsProvider");
const { EngCodeActionProvider } = require("./codeActionProvider");
const { EngFoldingRangeProvider } = require("./foldingRangeProvider");
const { EngFormattingProvider } = require("./formattingProvider");
const { EngHoverProvider } = require("./hoverProvider");
const {
  EngDefinitionProvider,
  EngDocumentHighlightProvider,
  EngDocumentSymbolProvider,
  EngReferenceProvider,
  EngRenameProvider,
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
  isWorkspaceEngSourceUri,
  workspaceRoot,
  workspaceRootKey
} = require("./runtimeDiscovery");
const { createSemanticLegend } = require("./lspSemanticTokens");

const LANGUAGE_ID = "englang";
const reviewCache = new Map();
const timeAlignmentReviewCache = new Map();
const fallbackReviewCache = new Map();
const workspaceReviewRevisions = new Map();
let output;
let artifactOpeners;
let decorationController;
let lspRequests;
let languageClient;
let commandHandlers;

const editorMetadata = loadEditorMetadata(__dirname);
const SEMANTIC_TOKEN_TYPES = editorMetadata.semanticTokenTypes;
const SEMANTIC_TOKEN_MODIFIERS = editorMetadata.semanticTokenModifiers;
const COMPLETION_ITEMS = editorMetadata.completionItems;
const HTTP_RESPONSE_FIELDS = editorMetadata.syntaxCatalog.http_response_fields;
const COVERAGE_RESULT_FIELDS = editorMetadata.syntaxCatalog.coverage_result_fields;
const TIME_ALIGNMENT_RESULT_FIELDS = editorMetadata.syntaxCatalog.time_alignment_result_fields;
const TABLE_FIELDS = editorMetadata.syntaxCatalog.table_fields;
const SAMPLE_TABLE_FIELDS = editorMetadata.syntaxCatalog.sample_table_fields;
const DB_CONNECTION_FIELDS = editorMetadata.syntaxCatalog.db_connection_fields;
const CASE_TABLE_FIELDS = editorMetadata.syntaxCatalog.case_table_fields;
const CASE_OUTPUT_TABLE_FIELDS = editorMetadata.syntaxCatalog.case_output_table_fields;
const CASE_RUN_RESULT_TABLE_FIELDS = editorMetadata.syntaxCatalog.case_run_result_table_fields;
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
  const compatibilityLspRequests = createLspRequests({
    isEngDocument,
    findLspRuntime,
    findLspRuntimeForRoot,
    workspaceRoot,
    appendOutputLine
  });
  languageClient = new PersistentLspClient(context, {
    isEngDocument,
    findLspRuntime,
    findLspRuntimeForRoot,
    workspaceRoot,
    appendOutputLine,
    semanticTokenTypes: SEMANTIC_TOKEN_TYPES,
    semanticTokenModifiers: SEMANTIC_TOKEN_MODIFIERS
  });
  lspRequests = createPersistentLspRequests({
    client: languageClient,
    fallback: compatibilityLspRequests,
    appendOutputLine
  });
  decorationController = createDecorationController({
    isEngDocument,
    reviewCache,
    timeAlignmentReviewCache,
    fallbackReviewCache
  });
  const diagnostics = vscode.languages.createDiagnosticCollection("englang");
  const diagnosticsStatusBar = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Left,
    80
  );
  diagnosticsStatusBar.name = "EngLang Problems";
  diagnosticsStatusBar.command = "englang.showToolingStatus";
  commandHandlers = createCommandHandlers({
    output,
    reviewCache,
    artifactOpeners,
    lspRequests,
    diagnosticsCollection: diagnostics,
    isEngDocument,
    semanticTokenTypes: SEMANTIC_TOKEN_TYPES,
    semanticTokenModifiers: SEMANTIC_TOKEN_MODIFIERS,
    syntaxCatalog: editorMetadata.syntaxCatalog,
    updateSemanticSymbolDecorations: decorationController.updateSemanticSymbolDecorations,
    cacheTimeAlignmentReview: (document, review) => {
      timeAlignmentReviewCache.set(document.uri.fsPath, review);
    },
    clearTimeAlignmentReview: (document) => {
      clearWorkspaceRunArtifactReview(
        document,
        timeAlignmentReviewCache,
        decorationController.updateTimeAlignmentDecorations
      );
    },
    updateTimeAlignmentDecorations: decorationController.updateTimeAlignmentDecorations,
    cacheFallbackReview: (document, review) => {
      fallbackReviewCache.set(document.uri.fsPath, review);
    },
    clearFallbackReview: (document) => {
      clearWorkspaceRunArtifactReview(
        document,
        fallbackReviewCache,
        decorationController.updateFallbackDecorations
      );
    },
    updateFallbackDecorations: decorationController.updateFallbackDecorations,
    runArtifactRevision: workspaceReviewRevision,
    runArtifactRevisionIsCurrent: (document, revision) => (
      workspaceReviewRevision(document) === revision
    )
  });
  const diagnosticController = new EngDiagnosticsController(context, diagnostics, {
    output,
    isEngDocument,
    clearSnapshotCache: lspRequests.clearSnapshotCache,
    diagnosticsRuntime,
    persistentDiagnostics: true,
    diagnosticsRuntimeLabel,
    findLspRuntime,
    findRuntime,
    snapshotDocumentSource: lspRequests.snapshotDocumentSource,
    workspaceRoot,
    workspaceRootKey,
    cacheReview: (document, review) => reviewCache.set(document.uri.fsPath, review),
    clearCachedReview: (document) => reviewCache.delete(document.uri.fsPath),
    updateReviewRiskDecorations: decorationController.updateReviewRiskDecorations,
    updateReviewValidationDecorations: decorationController.updateReviewValidationDecorations,
    updateSemanticSymbolDecorations: decorationController.updateSemanticSymbolDecorations
  });
  const semanticTokensProvider = new EngSemanticTokensProvider(context, {
    isEngDocument,
    semanticTokensForDocument: lspRequests.semanticTokensForDocument,
    snapshotDocumentSource: lspRequests.snapshotDocumentSource,
    cacheSnapshotForDocument: (document, snapshot) => reviewCache.set(document.uri.fsPath, snapshot),
    updateReviewValidationDecorations: decorationController.updateReviewValidationDecorations,
    updateSemanticSymbolDecorations: decorationController.updateSemanticSymbolDecorations,
    semanticLegend,
    semanticTokenTypes: SEMANTIC_TOKEN_TYPES,
    semanticTokenModifiers: SEMANTIC_TOKEN_MODIFIERS
  });
  const formattingProvider = new EngFormattingProvider(context, {
    isEngDocument,
    formatDocumentSource: lspRequests.formatDocumentSource,
    formattingEditsForDocument: lspRequests.formattingEditsForDocument,
    rangeFormattingEditsForDocument: lspRequests.rangeFormattingEditsForDocument
  });
  const engSourceWatcher = vscode.workspace.createFileSystemWatcher("**/*.eng");
  const diagnosticsSubscription = languageClient.onDiagnostics((params) => {
    void applyPublishedLanguageServerDiagnostics(params);
  });

  async function applyPublishedLanguageServerDiagnostics(params) {
    const document = documentForLanguageServerUri(params?.uri);
    if (!document || !diagnosticController.applyPublishedDiagnostics(document, params)) {
      return;
    }
    const documentVersion = document.version;
    try {
      const review = await lspRequests.snapshotDocumentSource(document, context);
      diagnosticController.applyReviewSnapshot(document, review, documentVersion);
    } catch (error) {
      appendOutputLine(`Unable to refresh EngLang review decorations: ${error.message}`);
    }
  }

  function documentForLanguageServerUri(uriText) {
    if (typeof uriText !== "string") {
      return undefined;
    }
    return (vscode.workspace.textDocuments ?? []).find((document) => (
      document.uri.toString() === uriText || document.uri.toString(true) === uriText
    ));
  }

  function runLanguageClientOperation(label, operation) {
    Promise.resolve(operation).catch((error) => {
      appendOutputLine(`EngLang language client ${label} failed: ${error.message}`);
    });
  }

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

  function clearCachedWorkspaceEditorSnapshots(document) {
    if (!document || !isEngDocument(document)) {
      return;
    }
    const changedRoot = workspaceRootKey(workspaceRoot(document));
    workspaceReviewRevisions.set(changedRoot, workspaceReviewRevision(document) + 1);
    const candidates = [document, ...(vscode.workspace.textDocuments ?? [])];
    const seen = new Set();
    for (const candidate of candidates) {
      if (!isEngDocument(candidate)) {
        continue;
      }
      const candidateRoot = workspaceRootKey(workspaceRoot(candidate));
      const candidateUri = candidate.uri.toString();
      if (candidateRoot !== changedRoot || seen.has(candidateUri)) {
        continue;
      }
      seen.add(candidateUri);
      reviewCache.delete(candidate.uri.fsPath);
      timeAlignmentReviewCache.delete(candidate.uri.fsPath);
      fallbackReviewCache.delete(candidate.uri.fsPath);
      decorationController.updateReviewRiskDecorations(candidate, undefined);
      decorationController.updateReviewValidationDecorations(candidate, undefined);
      decorationController.updateSemanticSymbolDecorations(candidate, undefined);
      decorationController.updateTimeAlignmentDecorations(candidate, undefined);
      decorationController.updateFallbackDecorations(candidate, undefined);
    }
  }

  function clearWorkspaceRunArtifactReview(document, cache, updateDecorations) {
    if (!document || !isEngDocument(document)) {
      return;
    }
    const root = workspaceRootKey(workspaceRoot(document));
    const candidates = [document, ...(vscode.workspace.textDocuments ?? [])];
    const seen = new Set();
    for (const candidate of candidates) {
      if (!isEngDocument(candidate) || workspaceRootKey(workspaceRoot(candidate)) !== root) {
        continue;
      }
      const candidateUri = candidate.uri.toString();
      if (seen.has(candidateUri)) {
        continue;
      }
      seen.add(candidateUri);
      cache.delete(candidate.uri.fsPath);
      updateDecorations(candidate, undefined);
    }
  }

  function workspaceReviewRevision(document) {
    const root = workspaceRootKey(workspaceRoot(document));
    return workspaceReviewRevisions.get(root) ?? 0;
  }

  function refreshWorkspaceAfterEngSourceSave(document) {
    if (!isEngDocument(document)) {
      return;
    }
    lspRequests.clearSnapshotCache(document);
    clearCachedWorkspaceEditorSnapshots(document);
    semanticTokensProvider.scheduleRefresh();
    diagnosticController.scheduleWorkspaceFileChangedChecks(document);
  }

  function refreshWorkspaceAfterClosedEngSourceChange(uri, changeType = 2) {
    if (!isWorkspaceEngSourceUri(uri)) {
      return;
    }
    const openDocument = (vscode.workspace.textDocuments ?? []).some(
      (document) => document.uri.toString() === uri.toString()
    );
    if (openDocument) {
      return;
    }
    runLanguageClientOperation(
      "watched-file notification",
      languageClient.watchedFileChanged(uri, changeType)
    );
    refreshWorkspaceAfterEngSourceSave({
      fileName: uri.fsPath,
      languageId: LANGUAGE_ID,
      uri
    });
  }

  function updateDiagnosticsStatusBar(document = vscode.window.activeTextEditor?.document) {
    if (!document || !isEngDocument(document)) {
      diagnosticsStatusBar.hide();
      return;
    }
    const config = engConfig(document);
    const mode = diagnosticsMode(document);
    const source = diagnosticsRuntimeLabel(diagnosticsRuntime(document));
    const problems = Array.from(diagnostics.get(document.uri) ?? []);
    const counts = diagnosticSeverityCounts(problems);
    const countText = diagnosticsStatusBarCountText(counts);
    const updateState = diagnosticsStatusBarUpdateState(document, mode, config);
    diagnosticsStatusBar.text = `${diagnosticsStatusBarIcon(counts, updateState)} EngLang ${mode} ${countText}`;
    diagnosticsStatusBar.tooltip = [
      `EngLang Problems: ${countText}`,
      `Source: ${source}`,
      `Mode: ${mode}`,
      updateState,
      "Click to open EngLang: Show Tooling Status."
    ].filter(Boolean).join("\n");
    diagnosticsStatusBar.show();
  }

  function updateDiagnosticsStatusBarForDocument(document) {
    const activeDocument = vscode.window.activeTextEditor?.document;
    if (activeDocument && document?.uri.toString() !== activeDocument.uri.toString()) {
      updateDiagnosticsStatusBar(activeDocument);
      return;
    }
    updateDiagnosticsStatusBar(document);
  }

  context.subscriptions.push(
    output,
    diagnostics,
    diagnosticsStatusBar,
    diagnosticController,
    semanticTokensProvider,
    engSourceWatcher,
    languageClient,
    diagnosticsSubscription
  );
  context.subscriptions.push(...decorationController.disposables);

  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((document) => {
      runLanguageClientOperation("document open", languageClient.openDocument(document));
      diagnosticController.maybeCheck(document);
      updateDiagnosticsStatusBarForDocument(document);
    }),
    vscode.workspace.onDidChangeTextDocument((event) => {
      runLanguageClientOperation("document change", languageClient.changeDocument(event.document));
      clearCachedWorkspaceEditorSnapshots(event.document);
      semanticTokensProvider.scheduleRefresh();
      diagnosticController.scheduleWorkspaceChangedChecks(event.document);
      updateDiagnosticsStatusBarForDocument(event.document);
    }),
    vscode.workspace.onDidSaveTextDocument((document) => {
      runLanguageClientOperation("document save", languageClient.saveDocument(document));
      refreshWorkspaceAfterEngSourceSave(document);
      diagnosticController.maybeCheck(document);
      updateDiagnosticsStatusBarForDocument(document);
    }),
    engSourceWatcher.onDidCreate((uri) => refreshWorkspaceAfterClosedEngSourceChange(uri, 1)),
    engSourceWatcher.onDidChange((uri) => refreshWorkspaceAfterClosedEngSourceChange(uri, 2)),
    engSourceWatcher.onDidDelete((uri) => refreshWorkspaceAfterClosedEngSourceChange(uri, 3)),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (
        event.affectsConfiguration("englang.diagnosticsMode") ||
        event.affectsConfiguration("englang.lintOnSave") ||
        event.affectsConfiguration("englang.lintOnChange") ||
        event.affectsConfiguration("englang.liveDiagnosticsDelayMs")
      ) {
        refreshActiveDiagnosticsForSettings("diagnostics configuration changed");
      }
      if (
        event.affectsConfiguration("englang.lspPath")
        || event.affectsConfiguration("englang.liveDiagnosticsDelayMs")
      ) {
        runLanguageClientOperation(
          "configuration restart",
          languageClient.restart(vscode.window.activeTextEditor?.document)
        );
      }
      if (event.affectsConfiguration("englang.reviewRiskDecorations.enabled")) {
        decorationController.refreshVisibleReviewRiskDecorations();
      }
      if (event.affectsConfiguration("englang.validationDecorations.enabled")) {
        decorationController.refreshVisibleReviewValidationDecorations();
      }
      if (event.affectsConfiguration("englang.timeAlignmentDecorations.enabled")) {
        decorationController.refreshVisibleTimeAlignmentDecorations();
      }
      if (event.affectsConfiguration("englang.fallbackDecorations.enabled")) {
        decorationController.refreshVisibleFallbackDecorations();
      }
      if (event.affectsConfiguration("englang.semanticHighlighting.enabled")) {
        semanticTokensProvider.refresh();
        decorationController.refreshVisibleSemanticSymbolDecorations();
      }
      updateDiagnosticsStatusBar();
    }),
    vscode.workspace.onDidChangeWorkspaceFolders(() => {
      runLanguageClientOperation(
        "workspace-folder restart",
        languageClient.restart(vscode.window.activeTextEditor?.document)
      );
    }),
    vscode.workspace.onDidCloseTextDocument((document) => {
      runLanguageClientOperation("document close", languageClient.closeDocument(document));
      diagnosticController.clearPendingCheck(document);
      diagnosticController.invalidateDocumentCheck(document);
      lspRequests.clearSnapshotCache(document);
      diagnostics.delete(document.uri);
      clearCachedWorkspaceEditorSnapshots(document);
      semanticTokensProvider.scheduleRefresh();
      diagnosticController.scheduleWorkspaceChangedChecks(document, false);
      updateDiagnosticsStatusBar();
    }),
    vscode.window.onDidChangeActiveTextEditor((editor) => {
      if (editor && isEngDocument(editor.document)) {
        const cached = reviewCache.get(editor.document.uri.fsPath);
        decorationController.updateReviewRiskDecorations(editor.document, cached);
        decorationController.updateReviewValidationDecorations(editor.document, cached);
        decorationController.updateSemanticSymbolDecorations(editor.document, cached);
        decorationController.updateTimeAlignmentDecorations(
          editor.document,
          timeAlignmentReviewCache.get(editor.document.uri.fsPath)
        );
        decorationController.updateFallbackDecorations(
          editor.document,
          fallbackReviewCache.get(editor.document.uri.fsPath)
        );
      }
      updateDiagnosticsStatusBar(editor?.document);
    }),
    vscode.languages.onDidChangeDiagnostics((event) => {
      const document = vscode.window.activeTextEditor?.document;
      if (!document || !isEngDocument(document)) {
        updateDiagnosticsStatusBar(document);
        return;
      }
      if (event.uris.some((uri) => uri.toString() === document.uri.toString())) {
        updateDiagnosticsStatusBar(document);
      }
    }),
    vscode.commands.registerCommand("englang.checkFile", () => diagnosticController.checkActiveFile()),
    vscode.commands.registerCommand("englang.refreshProblems", () => diagnosticController.checkActiveFile()),
    vscode.commands.registerCommand("englang.runFile", () => commandHandlers.runActiveFile(context)),
    vscode.commands.registerCommand("englang.runExample", () => commandHandlers.runExample(context)),
    vscode.commands.registerCommand("englang.switchProfile", () => commandHandlers.switchExecutionProfile()),
    vscode.commands.registerCommand("englang.switchDiagnosticsMode", refreshAfterDiagnosticsModeCommand),
    vscode.commands.registerCommand("englang.switchProblemsSource", refreshAfterDiagnosticsModeCommand),
    vscode.commands.registerCommand("englang.showToolingStatus", () => commandHandlers.showToolingStatus(context)),
    vscode.commands.registerCommand("englang.showProblemAtCursor", () => commandHandlers.showProblemAtCursor()),
    vscode.commands.registerCommand("englang.copyProblemAtCursor", () => commandHandlers.copyProblemAtCursor()),
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
    vscode.commands.registerCommand("englang.copySemanticTokenAtCursor", () => commandHandlers.copySemanticTokenAtCursor(context)),
    vscode.languages.registerHoverProvider(
      LANGUAGE_ID,
      new EngHoverProvider(context, {
        isEngDocument,
        hoverForPosition: lspRequests.hoverForPosition,
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
        coverageResultFields: COVERAGE_RESULT_FIELDS,
        timeAlignmentResultFields: TIME_ALIGNMENT_RESULT_FIELDS,
        tableFields: TABLE_FIELDS,
        sampleTableFields: SAMPLE_TABLE_FIELDS,
        dbConnectionFields: DB_CONNECTION_FIELDS,
        caseTableFields: CASE_TABLE_FIELDS,
        caseOutputTableFields: CASE_OUTPUT_TABLE_FIELDS,
        caseRunResultTableFields: CASE_RUN_RESULT_TABLE_FIELDS,
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
        documentSymbolsForDocument: lspRequests.documentSymbolsForDocument,
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
    vscode.languages.registerDocumentHighlightProvider(
      LANGUAGE_ID,
      new EngDocumentHighlightProvider(context, {
        isEngDocument,
        documentHighlightsForPosition: lspRequests.documentHighlightsForPosition
      })
    ),
    vscode.languages.registerReferenceProvider(
      LANGUAGE_ID,
      new EngReferenceProvider(context, {
        isEngDocument,
        referencesForPosition: lspRequests.referencesForPosition,
        appendOutputLine
      })
    ),
    vscode.languages.registerRenameProvider(
      LANGUAGE_ID,
      new EngRenameProvider(context, {
        isEngDocument,
        prepareRenameForPosition: lspRequests.prepareRenameForPosition,
        renameForPosition: lspRequests.renameForPosition,
        appendOutputLine
      })
    ),
    vscode.languages.registerFoldingRangeProvider(
      LANGUAGE_ID,
      new EngFoldingRangeProvider(context, {
        isEngDocument,
        foldingRangesForDocument: lspRequests.foldingRangesForDocument,
        snapshotDocumentSource: lspRequests.snapshotDocumentSource,
        cacheSnapshotForDocument: (document, snapshot) => reviewCache.set(document.uri.fsPath, snapshot)
      })
    ),
    vscode.languages.registerDocumentFormattingEditProvider(
      LANGUAGE_ID,
      formattingProvider
    ),
    vscode.languages.registerDocumentRangeFormattingEditProvider(
      LANGUAGE_ID,
      formattingProvider
    ),
    vscode.languages.registerOnTypeFormattingEditProvider(
      LANGUAGE_ID,
      formattingProvider,
      "}"
    ),
    vscode.languages.registerCodeActionsProvider(
      LANGUAGE_ID,
      new EngCodeActionProvider(context, {
        codeActionsForDocumentRange: lspRequests.codeActionsForDocumentRange,
        codeActionsForDocumentSource: lspRequests.codeActionsForDocumentSource,
        appendOutputLine
      }),
      {
        providedCodeActionKinds: [vscode.CodeActionKind.QuickFix]
      }
    )
  );

  for (const document of vscode.workspace.textDocuments) {
    diagnosticController.maybeCheck(document);
  }
  const startupDocument = vscode.window.activeTextEditor?.document
    ?? (vscode.workspace.textDocuments ?? []).find(isEngDocument);
  runLanguageClientOperation("startup", languageClient.start(startupDocument));
  updateDiagnosticsStatusBar();
}

async function deactivate() {
  await languageClient?.stop();
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

function diagnosticSeverityCounts(problems) {
  const counts = { errors: 0, warnings: 0, infos: 0, hints: 0 };
  for (const problem of problems) {
    switch (problem.severity) {
      case vscode.DiagnosticSeverity.Error:
        counts.errors += 1;
        break;
      case vscode.DiagnosticSeverity.Warning:
        counts.warnings += 1;
        break;
      case vscode.DiagnosticSeverity.Information:
        counts.infos += 1;
        break;
      case vscode.DiagnosticSeverity.Hint:
        counts.hints += 1;
        break;
      default:
        break;
    }
  }
  return counts;
}

function diagnosticsStatusBarCountText(counts) {
  const parts = [];
  if (counts.errors > 0) parts.push(`${counts.errors}E`);
  if (counts.warnings > 0) parts.push(`${counts.warnings}W`);
  if (counts.infos > 0) parts.push(`${counts.infos}I`);
  if (counts.hints > 0) parts.push(`${counts.hints}H`);
  return parts.length > 0 ? parts.join(" ") : "clean";
}

function diagnosticsStatusBarIcon(counts, updateState) {
  if (/off|disabled/.test(updateState)) {
    return "$(circle-slash)";
  }
  if (/last saved/.test(updateState)) {
    return "$(circle-large-outline)";
  }
  if (counts.errors > 0) {
    return "$(error)";
  }
  if (counts.warnings > 0) {
    return "$(warning)";
  }
  return "$(check)";
}

function diagnosticsStatusBarUpdateState(document, mode, config) {
  const lintOnSave = config.get("lintOnSave", true);
  const lintOnChange = config.get("lintOnChange", true);
  if (mode === "live") {
    if (document.isDirty && !lintOnChange) {
      return "Live typing diagnostics are off; run EngLang: Refresh Problems for an unsaved-buffer check.";
    }
    if (!document.isDirty && !lintOnSave) {
      return "Saved-file diagnostics are off; run EngLang: Refresh Problems for a manual check.";
    }
    return lintOnChange
      ? "Live typing diagnostics update after a short pause."
      : "Live diagnostics are selected; saved files refresh on open, save, or manual check.";
  }
  if (!lintOnSave) {
    return "Saved-file diagnostics are off; run EngLang: Refresh Problems for a manual check.";
  }
  if (document.isDirty) {
    return "File diagnostics use the saved file; save before refreshing, or switch Diagnostics Mode to live for unsaved Problems.";
  }
  return "File diagnostics refresh on open, save, or manual check.";
}
module.exports = {
  activate,
  deactivate
};
