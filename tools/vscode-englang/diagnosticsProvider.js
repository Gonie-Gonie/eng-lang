const cp = require("child_process");
const vscode = require("vscode");

const CHECK_DEBOUNCE_MS = 350;
const DIAGNOSTIC_DOC_ROOT = "https://github.com/Gonie-Gonie/eng-lang/blob/main/docs";
const DIAGNOSTIC_CODE_DOC = `${DIAGNOSTIC_DOC_ROOT}/reference/cli/spec.md#diagnostic-codes`;
const DIAGNOSTIC_DOC_TARGETS = new Map([
  ["E-SCRIPT-001", `${DIAGNOSTIC_DOC_ROOT}/reference/language/top_level_execution_policy.md#execution-model`],
  ["E-STRUCT-ARGS-001", `${DIAGNOSTIC_DOC_ROOT}/reference/language/top_level_execution_policy.md#args`],
  [
    "W-TABLE-LEGACY-SELECT-FIRST-ROW",
    `${DIAGNOSTIC_DOC_ROOT}/reference/artifacts/report_review.md#promoted-table-selection-and-transform-metadata`
  ],
  [
    "W-ML-TRAIN-ALIAS",
    `${DIAGNOSTIC_DOC_ROOT}/reference/artifacts/report_review.md#data-driven-modeling-metadata`
  ],
  [
    "W-ML-ANN-ALIAS",
    `${DIAGNOSTIC_DOC_ROOT}/reference/artifacts/report_review.md#data-driven-modeling-metadata`
  ]
]);

class EngDiagnosticsController {
  constructor(context, diagnostics, options = {}) {
    this.context = context;
    this.diagnostics = diagnostics;
    this.output = options.output;
    this.isEngDocument = options.isEngDocument ?? (() => true);
    this.clearSnapshotCache = options.clearSnapshotCache ?? (() => undefined);
    this.diagnosticsRuntime = options.diagnosticsRuntime;
    this.persistentDiagnostics = options.persistentDiagnostics === true;
    this.diagnosticsRuntimeLabel = options.diagnosticsRuntimeLabel ?? ((runtimeMode) => runtimeMode);
    this.findLspRuntime = options.findLspRuntime;
    this.findRuntime = options.findRuntime;
    this.snapshotDocumentSource = options.snapshotDocumentSource;
    this.workspaceRoot = options.workspaceRoot;
    this.workspaceRootKey = options.workspaceRootKey ?? ((root) => String(root ?? ""));
    this.cacheReview = options.cacheReview ?? (() => undefined);
    this.clearCachedReview = options.clearCachedReview ?? (() => undefined);
    this.updateReviewRiskDecorations = options.updateReviewRiskDecorations ?? (() => undefined);
    this.updateReviewValidationDecorations =
      options.updateReviewValidationDecorations ?? (() => undefined);
    this.updateSemanticSymbolDecorations = options.updateSemanticSymbolDecorations ?? (() => undefined);
    this.changeTimers = new Map();
    this.checkDebounceMs = options.checkDebounceMs ?? CHECK_DEBOUNCE_MS;
    this.checkSequence = 0;
    this.activeDocumentChecks = new WeakMap();
    this.activeDocumentProcesses = new Map();
    this.disposed = false;
  }

  maybeCheck(document) {
    if (!this.isEngDocument(document)) {
      return;
    }
    if (this.persistentDiagnostics && this.diagnosticsRuntime?.(document) === "lsp-snapshot") {
      return;
    }
    const config = vscode.workspace.getConfiguration("englang", document.uri);
    if (!config.get("lintOnSave", true) || document.isDirty) {
      return;
    }
    this.clearPendingCheck(document);
    this.checkDocument(document);
  }

  scheduleChangedCheck(document) {
    if (!this.isEngDocument(document)) {
      return;
    }
    this.clearSnapshotCache(document);
    this.invalidateDocumentCheck(document);
    if (this.diagnosticsRuntime?.(document) !== "lsp-snapshot") {
      return;
    }
    if (this.persistentDiagnostics) {
      return;
    }
    const config = vscode.workspace.getConfiguration("englang", document.uri);
    if (!config.get("lintOnChange", true)) {
      return;
    }
    this.scheduleDebouncedCheck(
      document,
      () => this.checkDocumentSource(document),
      this.liveDiagnosticsDelayMs(document)
    );
  }

  scheduleWorkspaceChangedChecks(document, includeChangedDocument = true) {
    const changedRoot = this.workspaceRootKey(this.workspaceRoot(document));
    const changedUri = document?.uri?.toString?.();
    const candidates = vscode.workspace.textDocuments ?? [];
    let changedDocumentScheduled = false;
    for (const candidate of candidates) {
      if (!this.isEngDocument(candidate)) {
        continue;
      }
      const candidateRoot = this.workspaceRootKey(this.workspaceRoot(candidate));
      if (candidateRoot !== changedRoot) {
        continue;
      }
      const candidateIsChanged = candidate === document
        || (changedUri && candidate?.uri?.toString?.() === changedUri);
      if (!includeChangedDocument && candidateIsChanged) {
        continue;
      }
      this.scheduleChangedCheck(candidate);
      changedDocumentScheduled ||= candidateIsChanged;
    }
    if (includeChangedDocument && !changedDocumentScheduled) {
      this.scheduleChangedCheck(document);
    }
  }

  scheduleWorkspaceFileChangedChecks(document) {
    const changedRoot = this.workspaceRootKey(this.workspaceRoot(document));
    const changedUri = document?.uri?.toString?.();
    for (const candidate of vscode.workspace.textDocuments ?? []) {
      if (!this.isEngDocument(candidate)) {
        continue;
      }
      const candidateRoot = this.workspaceRootKey(this.workspaceRoot(candidate));
      if (candidateRoot !== changedRoot || candidate.uri.toString() === changedUri) {
        continue;
      }
      this.scheduleDependencyFileCheck(candidate);
    }
  }

  scheduleDependencyFileCheck(document) {
    if (!this.isEngDocument(document)) {
      return;
    }
    this.clearSnapshotCache(document);
    this.invalidateDocumentCheck(document);
    if (!this.dependencyFileCheckKind(document)) {
      this.clearPendingCheck(document);
      return;
    }
    this.scheduleDebouncedCheck(
      document,
      () => {
        const kind = this.dependencyFileCheckKind(document);
        if (kind === "source") {
          this.checkDocumentSource(document);
        } else if (kind === "file") {
          this.checkDocument(document);
        }
      },
      this.liveDiagnosticsDelayMs(document)
    );
  }

  dependencyFileCheckKind(document) {
    const config = vscode.workspace.getConfiguration("englang", document.uri);
    const runtimeMode = this.diagnosticsRuntime?.(document);
    if (runtimeMode === "lsp-snapshot") {
      if (this.persistentDiagnostics) {
        return undefined;
      }
      const setting = document.isDirty ? "lintOnChange" : "lintOnSave";
      return config.get(setting, true) ? "source" : undefined;
    }
    if (runtimeMode === "eng-cli" && !document.isDirty && config.get("lintOnSave", true)) {
      return "file";
    }
    return undefined;
  }

  liveDiagnosticsDelayMs(document) {
    const config = vscode.workspace.getConfiguration("englang", document.uri);
    const configured = Number(config.get("liveDiagnosticsDelayMs"));
    if (!Number.isFinite(configured)) {
      return this.checkDebounceMs;
    }
    return Math.max(100, Math.min(5000, Math.trunc(configured)));
  }

  scheduleDebouncedCheck(document, check, delayMs = this.checkDebounceMs) {
    this.clearPendingCheck(document);
    const key = document.uri.toString();
    const timer = setTimeout(() => {
      this.changeTimers.delete(key);
      check();
    }, delayMs);
    this.changeTimers.set(key, timer);
  }

  clearPendingCheck(document) {
    const key = document.uri.toString();
    const timer = this.changeTimers.get(key);
    if (timer) {
      clearTimeout(timer);
      this.changeTimers.delete(key);
    }
  }

  dispose() {
    this.disposed = true;
    this.clearSnapshotCache();
    for (const timer of this.changeTimers.values()) {
      clearTimeout(timer);
    }
    this.changeTimers.clear();
    for (const process of this.activeDocumentProcesses.values()) {
      process.cancelled = true;
      process.child?.kill?.();
    }
    this.activeDocumentProcesses.clear();
  }

  beginDocumentCheck(document) {
    const revision = ++this.checkSequence;
    this.activeDocumentChecks.set(document, revision);
    this.cancelDocumentProcess(document);
    return revision;
  }

  invalidateDocumentCheck(document) {
    if (!document) {
      return;
    }
    this.activeDocumentChecks.set(document, ++this.checkSequence);
    this.cancelDocumentProcess(document);
  }

  beginDocumentProcess(document) {
    const key = document.uri.toString();
    const process = {
      cancelled: false,
      child: undefined
    };
    this.activeDocumentProcesses.set(key, process);
    return process;
  }

  attachDocumentProcess(process, child) {
    process.child = child;
    if (process.cancelled) {
      child?.kill?.();
    }
  }

  finishDocumentProcess(document, process) {
    const key = document.uri.toString();
    if (this.activeDocumentProcesses.get(key) === process) {
      this.activeDocumentProcesses.delete(key);
    }
  }

  cancelDocumentProcess(document) {
    const key = document?.uri?.toString?.();
    if (!key) {
      return;
    }
    const process = this.activeDocumentProcesses.get(key);
    if (!process) {
      return;
    }
    this.activeDocumentProcesses.delete(key);
    process.cancelled = true;
    process.child?.kill?.();
  }

  isCurrentDocumentCheck(document, documentVersion, checkRevision) {
    return !this.disposed
      && document.version === documentVersion
      && this.activeDocumentChecks.get(document) === checkRevision;
  }

  clearDocumentDiagnostics(document, reason = "diagnostics mode changed") {
    if (!document || !this.isEngDocument(document)) {
      return;
    }
    this.clearPendingCheck(document);
    this.clearSnapshotCache(document);
    this.invalidateDocumentCheck(document);
    this.clearCachedReview(document);
    this.diagnostics.delete(document.uri);
    this.updateReviewRiskDecorations(document, undefined);
    this.updateReviewValidationDecorations(document, undefined);
    this.updateSemanticSymbolDecorations(document, undefined);
    this.appendLine(`Problems cleared for ${document.uri.fsPath}: ${reason}`);
  }

  async checkActiveFile() {
    const document = vscode.window.activeTextEditor?.document;
    if (!document || !this.isEngDocument(document)) {
      vscode.window.showWarningMessage("Open an EngLang .eng file first.");
      return;
    }
    const runtimeMode = this.diagnosticsRuntime?.(document);
    if (document.isDirty) {
      if (runtimeMode === "lsp-snapshot") {
        this.checkDocumentSource(document);
        return;
      }
      this.clearDocumentDiagnostics(
        document,
        "file mode uses saved-file checks; save the file to refresh Problems"
      );
      vscode.window.showInformationMessage(
        "EngLang file diagnostics use saved files. Save the file, or switch Diagnostics Mode to live for unsaved Problems."
      );
      return;
    }
    await this.checkDocument(document);
  }

  checkDocument(document) {
    if (this.persistentDiagnostics && this.diagnosticsRuntime?.(document) === "lsp-snapshot") {
      return this.checkDocumentSource(document);
    }
    const checkRevision = this.beginDocumentCheck(document);
    const runtimeMode = this.diagnosticsRuntime(document);
    const runtime = runtimeMode === "lsp-snapshot"
      ? this.findLspRuntime(this.context, document)
      : this.findRuntime(this.context, document);
    const args = runtimeMode === "lsp-snapshot"
      ? ["--snapshot", document.uri.fsPath]
      : ["ide-check", document.uri.fsPath];
    const cwd = this.workspaceRoot(document);
    const documentVersion = document.version;
    const runtimeLabel = this.diagnosticsRuntimeLabel(runtimeMode);
    this.appendLine(`${runtimeLabel} check ${document.uri.fsPath}`);
    this.appendLine(`Problems source: ${diagnosticSource(runtimeLabel)}; diagnostics: ${runtimeLabel}; tool: ${runtime}`);

    const process = this.beginDocumentProcess(document);
    const child = cp.execFile(
      runtime,
      args,
      { cwd, maxBuffer: 10 * 1024 * 1024 },
      (error, stdout, stderr) => {
        this.finishDocumentProcess(document, process);
        this.finishDocumentCheck(
          document,
          runtimeLabel,
          documentVersion,
          checkRevision,
          error,
          stdout,
          stderr
        );
      }
    );
    this.attachDocumentProcess(process, child);
  }

  checkDocumentSource(document) {
    const checkRevision = this.beginDocumentCheck(document);
    if (this.snapshotDocumentSource) {
      const documentVersion = document.version;
      const runtime = this.findLspRuntime?.(this.context, document) ?? "eng-lsp.exe";
      this.appendLine(`live buffer check ${document.uri.fsPath}`);
      this.appendLine(`Problems source: ${diagnosticSource("live buffer")}; diagnostics: live buffer; tool: ${runtime}`);
      this.snapshotDocumentSource(document, this.context)
        .then((review) => {
          if (!this.isCurrentDocumentCheck(document, documentVersion, checkRevision)) {
            return;
          }
          if (!review) {
            this.applyUnavailableSnapshotDiagnostic(
              document,
              "live buffer",
              diagnosticsFailureDetail({ parseError: new Error("empty snapshot response") })
            );
            return;
          }
          this.finishParsedDocumentCheck(
            document,
            "live buffer",
            documentVersion,
            checkRevision,
            review
          );
        })
        .catch((error) => {
          if (!this.isCurrentDocumentCheck(document, documentVersion, checkRevision)) {
            return;
          }
          this.appendLine(`live buffer check failed: ${error.message}`);
          this.applyUnavailableSnapshotDiagnostic(
            document,
            "live buffer",
            diagnosticsFailureDetail({ error })
          );
        });
      return;
    }

    const runtime = this.findLspRuntime(this.context, document);
    const cwd = this.workspaceRoot(document);
    const documentVersion = document.version;
    this.appendLine(`live buffer check ${document.uri.fsPath}`);
    this.appendLine(`Problems source: ${diagnosticSource("live buffer")}; diagnostics: live buffer; tool: ${runtime}`);

    const process = this.beginDocumentProcess(document);
    const child = cp.execFile(
      runtime,
      ["--snapshot-stdin", document.uri.fsPath],
      { cwd, maxBuffer: 10 * 1024 * 1024 },
      (error, stdout, stderr) => {
        this.finishDocumentProcess(document, process);
        this.finishDocumentCheck(
          document,
          "live buffer",
          documentVersion,
          checkRevision,
          error,
          stdout,
          stderr
        );
      }
    );
    this.attachDocumentProcess(process, child);
    if (child.stdin) {
      child.stdin.end(document.getText());
    }
  }

  finishDocumentCheck(
    document,
    runtimeLabel,
    documentVersion,
    checkRevision,
    error,
    stdout,
    stderr
  ) {
    if (!this.isCurrentDocumentCheck(document, documentVersion, checkRevision)) {
      return;
    }
    if (stderr && stderr.trim().length > 0) {
      this.appendLine(stderr.trim());
    }

    let review;
    try {
      review = JSON.parse(stdout);
    } catch (parseError) {
      const failure = diagnosticsFailureDetail({ error, stderr, stdout, parseError });
      this.appendLine(`Unable to parse EngLang ${runtimeLabel} output: ${parseError.message}`);
      if (failure.problemMessage) {
        this.appendLine(`diagnostics failure detail: ${failure.problemMessage}`);
      }
      if (error) {
        this.appendLine(error.message);
      }
      this.applyUnavailableSnapshotDiagnostic(document, runtimeLabel, failure);
      return;
    }
    this.finishParsedDocumentCheck(
      document,
      runtimeLabel,
      documentVersion,
      checkRevision,
      review
    );
  }

  finishParsedDocumentCheck(
    document,
    runtimeLabel,
    documentVersion,
    checkRevision,
    review
  ) {
    if (!this.isCurrentDocumentCheck(document, documentVersion, checkRevision)) {
      return;
    }
    this.cacheReview(document, review);
    this.diagnostics.set(document.uri, toDiagnostics(document, review, {
      source: diagnosticSource(runtimeLabel)
    }));
    this.updateReviewRiskDecorations(document, review);
    this.updateReviewValidationDecorations(document, review);
    this.updateSemanticSymbolDecorations(document, review);
    const errors = review.diagnostics?.filter((item) => severityName(item.severity) === "error").length ?? 0;
    const warnings = review.diagnostics?.filter((item) => severityName(item.severity) === "warning").length ?? 0;
    this.appendLine(`diagnostics (${diagnosticSource(runtimeLabel)}): ${errors} error(s), ${warnings} warning(s)`);
  }

  applyPublishedDiagnostics(document, params) {
    if (
      !document
      || !this.isEngDocument(document)
      || this.diagnosticsRuntime?.(document) !== "lsp-snapshot"
      || !Array.isArray(params?.diagnostics)
    ) {
      return false;
    }
    if (Number.isInteger(params.version) && params.version !== document.version) {
      this.appendLine(
        `Ignored stale live diagnostics for ${document.uri.fsPath}: server version ${params.version}, editor version ${document.version}`
      );
      return false;
    }
    const config = vscode.workspace.getConfiguration("englang", document.uri);
    const enabled = document.isDirty
      ? config.get("lintOnChange", true)
      : config.get("lintOnSave", true);
    if (!enabled) {
      return false;
    }

    this.clearPendingCheck(document);
    this.invalidateDocumentCheck(document);
    const converted = toDiagnostics(document, { diagnostics: params.diagnostics }, {
      source: diagnosticSource("live editor")
    });
    this.diagnostics.set(document.uri, converted);
    const errors = params.diagnostics.filter((item) => severityName(item.severity) === "error").length;
    const warnings = params.diagnostics.filter((item) => severityName(item.severity) === "warning").length;
    this.appendLine(`diagnostics (eng/live): ${errors} error(s), ${warnings} warning(s)`);
    return true;
  }

  applyReviewSnapshot(document, review, documentVersion = document?.version) {
    if (
      !document
      || !this.isEngDocument(document)
      || document.version !== documentVersion
      || this.diagnosticsRuntime?.(document) !== "lsp-snapshot"
      || review?.format !== "eng-lsp-snapshot-v1"
    ) {
      return false;
    }
    this.cacheReview(document, review);
    this.updateReviewRiskDecorations(document, review);
    this.updateReviewValidationDecorations(document, review);
    this.updateSemanticSymbolDecorations(document, review);
    return true;
  }

  applyUnavailableSnapshotDiagnostic(document, runtimeLabel = "editor", failure = undefined) {
    const settingHint = diagnosticsSettingHint(runtimeLabel);
    const failureMessage = failure?.problemMessage
      ? ` Tool failure: ${failure.problemMessage}.`
      : "";
    const outputHint = failure?.hasOutput
      ? " See the EngLang output channel for stderr/stdout details."
      : "";
    const diagnostic = new vscode.Diagnostic(
      firstLineRange(document),
      `EngLang ${runtimeLabel} diagnostics did not return editor JSON.${failureMessage} Run EngLang: Show Tooling Status to confirm selected tool paths, or check ${settingHint}.${outputHint}`,
      vscode.DiagnosticSeverity.Error
    );
    diagnostic.source = diagnosticSource(runtimeLabel);
    this.clearCachedReview(document);
    this.diagnostics.set(document.uri, [diagnostic]);
    this.updateReviewRiskDecorations(document, undefined);
    this.updateReviewValidationDecorations(document, undefined);
    this.updateSemanticSymbolDecorations(document, undefined);
  }

  appendLine(message) {
    this.output?.appendLine(message);
  }
}

function toDiagnostics(document, review, options = {}) {
  const source = typeof options.source === "string" && options.source.length > 0
    ? options.source
    : "eng";
  return (review.diagnostics ?? []).map((item) => {
    const range = diagnosticRange(document, item);
    const severity = toVscodeSeverity(item.severity);
    const diagnostic = new vscode.Diagnostic(range, item.message, severity);
    const code = diagnosticCode(item);
    if (code !== undefined) {
      diagnostic.code = code;
    }
    diagnostic.source = source;
    const tags = diagnosticTags(item);
    if (tags.length > 0) {
      diagnostic.tags = tags;
    }
    if (item.help) {
      diagnostic.message = `${item.message}\n${item.help}`;
    }
    return diagnostic;
  });
}

function diagnosticRange(document, item) {
  const sourceLine = integerOrUndefined(item?.range?.start?.line)
    ?? Math.max(0, (sourceLineNumber(item) ?? 1) - 1);
  const line = Math.max(0, Math.min(sourceLine, document.lineCount - 1));
  const textLine = document.lineAt(line);
  const lineText = String(textLine.text || "");
  const maxCharacter = Math.max(1, lineText.length);
  const rangeStartCharacter = integerOrUndefined(item?.range?.start?.character);
  const rangeEndCharacter = integerOrUndefined(item?.range?.end?.character);
  const sourceColumn = sourceColumnNumber(item);
  const hasSourceColumn = sourceColumn !== undefined;
  const hasExplicitRange = rangeStartCharacter !== undefined && rangeEndCharacter !== undefined;
  const tokenRange = hasExplicitRange
    ? undefined
    : diagnosticFallbackRangeForCode(lineText, item, sourceColumn);
  const fallbackStartCharacter = tokenRange?.start ?? (hasSourceColumn
    ? sourceColumnCharacter(lineText, sourceColumn)
    : 0);
  const startCharacter = Math.max(
    0,
    Math.min(rangeStartCharacter ?? fallbackStartCharacter, maxCharacter - 1)
  );
  const fallbackEndCharacter = tokenRange?.end ?? (hasSourceColumn
    ? diagnosticTokenEndCharacter(lineText, startCharacter, maxCharacter)
    : maxCharacter);
  const endCharacter = Math.max(
    startCharacter + 1,
    Math.min(rangeEndCharacter ?? fallbackEndCharacter, maxCharacter)
  );
  return new vscode.Range(line, startCharacter, line, endCharacter);
}

function diagnosticFallbackRangeForCode(lineText, item, sourceColumn) {
  const code = diagnosticCodeValue(item);
  const searchStart = sourceColumn !== undefined
    ? sourceColumnCharacter(lineText, sourceColumn)
    : 0;
  const optionNames = diagnosticOptionNames(code);
  if (optionNames) {
    const optionRange = optionValueRange(lineText, optionNames, searchStart);
    if (optionRange) {
      return optionRange;
    }
  }
  if (code === "E-SYNTAX-DECL-001") {
    return firstNeedleRange(lineText, [":="], searchStart);
  }
  if (code === "E-EQ-BOOL-001") {
    return firstNeedleRange(lineText, ["=="], searchStart);
  }
  if (code === "E-STRUCT-ARGS-001") {
    return firstNeedleRange(lineText, ["struct Args", "struct"], 0);
  }
  if (code === "E-SCRIPT-001") {
    return firstNeedleRange(lineText, ["script"], searchStart);
  }
  if (code === "W-NET-FIXTURE-ALIAS") {
    return optionKeyRange(lineText, "fixture");
  }
  if (code === "W-NET-RESPONSE-HASH-ALIAS") {
    return memberFieldRange(lineText, "hash", searchStart);
  }
  if (code === "W-NET-RESPONSE-STATUS-ALIAS") {
    return memberFieldRange(lineText, "status", searchStart);
  }
  if (code === "W-ML-TRAIN-ALIAS") {
    return firstNeedleRange(lineText, ["regression_table", "train_regression"], searchStart);
  }
  if (code === "W-ML-ANN-ALIAS") {
    return functionCallNameRange(lineText, "ann", searchStart);
  }
  if (code === "W-STATS-SUM-001") {
    return functionCallNameRange(lineText, "sum", searchStart);
  }
  if (code === "E-PUBLIC-ANNOTATION-001") {
    return firstNeedleRange(lineText, ["="], searchStart);
  }
  if (code === "E-FS-CONFIRM-001") {
    return firstNeedleRange(lineText, ["delete", "move"], searchStart);
  }
  if (code === "E-FS-DELETE-001") {
    return firstNeedleRange(lineText, ["delete"], searchStart);
  }
  if (code === "E-LOG-LEVEL-001") {
    return logLevelRange(lineText);
  }
  if (code === "E-NET-INVALID-URL") {
    return netUrlLiteralRange(lineText, searchStart);
  }
  if (code === "E-UNC-ARGS-002") {
    const uncertaintyRange = uncertaintyArgumentDiagnosticRange(lineText, item);
    if (uncertaintyRange) {
      return uncertaintyRange;
    }
  }
  const formatRange = formatInterpolationDiagnosticRange(lineText, item);
  if (formatRange) {
    return formatRange;
  }
  return diagnosticBacktickRange(lineText, item);
}

function diagnosticOptionNames(code) {
  switch (code) {
    case "E-NET-RETRY-POLICY":
    case "E-PROCESS-RETRY-POLICY":
      return ["retry"];
    case "E-NET-TIMEOUT":
    case "E-PROCESS-TIMEOUT":
      return ["timeout"];
    case "E-NET-BODY-SIZE-LIMIT":
      return ["body_size_limit", "response_body_limit"];
    case "E-NET-BODY-METHOD":
    case "E-NET-BODY-POLICY":
      return ["body"];
    case "E-PROCESS-ALLOW-FAILURE":
      return ["allow_failure"];
    case "E-PROCESS-CWD-001":
      return ["cwd"];
    case "E-PROCESS-ENV-001":
      return ["env"];
    case "E-SAMPLING-COUNT-INVALID":
      return ["count"];
    case "E-SAMPLING-SEED-INVALID":
      return ["seed"];
    case "E-ML-ARGS-001":
      return ["target", "y", "features", "x", "test", "test_fraction", "hidden", "layers", "epochs"];
    case "E-ML-ARGS-002":
      return ["test", "test_fraction", "seed", "hidden", "layers", "epochs"];
    case "E-ML-ARGS-003":
      return ["algorithm"];
    case "E-CACHE-KEY-NONDETERMINISTIC":
      return ["cache_key"];
    case "E-CACHE-DIR":
      return ["cache_dir"];
    case "E-CACHE-TTL":
      return ["cache_ttl"];
    case "E-WITH-UNIT-001":
      return ["unit y", "unit x", "display_unit", "unit"];
    case "E-WITH-UNCERTAINTY-POLICY-001":
      return ["uncertainty"];
    case "E-WITH-UNCERTAINTY-SAMPLES-001":
      return ["samples"];
    case "E-WITH-UNCERTAINTY-SEED-001":
      return ["seed"];
    case "E-SIM-TIMESTEP-INVALID":
    case "E-SOLVE-TIMESTEP-INVALID":
      return ["timestep"];
    case "E-SIM-DURATION-INVALID":
    case "E-SOLVE-DURATION-INVALID":
      return ["duration"];
    case "E-SIM-TOLERANCE-INVALID":
    case "E-SOLVE-TOLERANCE-INVALID":
      return ["tolerance"];
    case "E-SIM-SOLVER-UNSUPPORTED":
    case "E-SOLVE-SOLVER-UNSUPPORTED":
      return ["solver"];
    case "E-SOLVE-RELAXATION-INVALID":
      return ["relaxation"];
    case "E-SOLVE-FD-STEP-INVALID":
      return ["finite_difference_step"];
    case "E-SOLVE-DAMPING-INVALID":
      return ["damping"];
    case "E-SOLVE-CONSISTENCY-TOLERANCE-INVALID":
      return ["consistency_tolerance"];
    case "E-SOLVE-MAX-ITER-INVALID":
      return ["max_iter"];
    case "E-SOLVE-LINE-SEARCH-STEPS-INVALID":
      return ["line_search_steps"];
    case "E-SOLVE-INITIAL-INVALID":
      return ["initial", "initial_derivative", "initial_algebraic"];
    case "E-SOLVE-VARIABLE-SCALE-INVALID":
      return ["variable_scale", "variable_scales"];
    case "E-SOLVE-MASS-MATRIX-INVALID":
      return ["mass_matrix"];
    case "E-SOLVE-JACOBIAN-UNSUPPORTED":
      return ["jacobian"];
    case "E-SOLVE-ALGEBRAIC-INITIALIZATION-UNSUPPORTED":
      return ["algebraic_initialization"];
    default:
      return undefined;
  }
}

function optionValueRange(lineText, optionNames, startCharacter = 0) {
  const code = stripLineComment(lineText);
  for (const optionName of optionNames) {
    const range = optionValueRangeFrom(code, optionName, startCharacter)
      ?? optionValueRangeFrom(code, optionName, 0);
    if (range) {
      return range;
    }
  }
  return undefined;
}

function optionValueRangeFrom(lineText, optionName, startCharacter = 0) {
  const text = String(lineText || "");
  let cursor = Math.max(0, Math.min(startCharacter, text.length));
  while (cursor < text.length) {
    const nameStart = text.indexOf(optionName, cursor);
    if (nameStart < 0) {
      return undefined;
    }
    const nameEnd = nameStart + optionName.length;
    if (!isIdentifierPart(text[nameStart - 1]) && !isIdentifierPart(text[nameEnd])) {
      let equals = nameEnd;
      while (equals < text.length && /\s/.test(text[equals])) {
        equals += 1;
      }
      if (text[equals] === "=") {
        let valueStart = equals + 1;
        while (valueStart < text.length && /\s/.test(text[valueStart])) {
          valueStart += 1;
        }
        let valueEnd = text.length;
        while (valueEnd > valueStart && /\s/.test(text[valueEnd - 1])) {
          valueEnd -= 1;
        }
        return valueStart < valueEnd
          ? { start: valueStart, end: valueEnd }
          : { start: nameStart, end: nameEnd };
      }
    }
    cursor = nameEnd;
  }
  return undefined;
}

function stripLineComment(text) {
  const index = lineCommentStart(text);
  return index >= 0 ? String(text || "").slice(0, index) : String(text || "");
}

function lineCommentStart(text) {
  const source = String(text || "");
  let inString = false;
  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (character === "\\" && inString) {
      index += 1;
      continue;
    }
    if (character === '"') {
      inString = !inString;
      continue;
    }
    if (!inString && character === "#") {
      return index;
    }
    if (!inString && character === "/" && source[index + 1] === "/") {
      return index;
    }
  }
  return -1;
}

function firstNeedleRange(lineText, needles, startCharacter = 0) {
  const text = String(lineText || "");
  const searchStart = Math.max(0, Math.min(startCharacter, text.length));
  for (const needle of needles) {
    const index = text.indexOf(needle, searchStart);
    if (index >= 0) {
      return { start: index, end: index + needle.length };
    }
  }
  for (const needle of needles) {
    const index = text.indexOf(needle);
    if (index >= 0) {
      return { start: index, end: index + needle.length };
    }
  }
  return undefined;
}

function optionKeyRange(lineText, key) {
  const text = String(lineText || "");
  const start = lineIndentLength(text);
  if (text.slice(start, start + key.length) !== key) {
    return undefined;
  }
  const afterKey = text.slice(start + key.length);
  if (!afterKey.charAt(0).match(/\s|=/)) {
    return undefined;
  }
  const equalsIndex = afterKey.indexOf("=");
  if (equalsIndex < 0 || afterKey.slice(0, equalsIndex).trim().length > 0) {
    return undefined;
  }
  return { start, end: start + key.length };
}

function memberFieldRange(lineText, field, startCharacter = 0) {
  const text = String(lineText || "");
  const searchStart = Math.max(0, Math.min(startCharacter, text.length));
  const range = memberFieldRangeFrom(text, field, searchStart)
    ?? memberFieldRangeFrom(text, field, 0);
  return range;
}

function memberFieldRangeFrom(text, field, searchStart) {
  const needle = `.${field}`;
  let cursor = searchStart;
  while (cursor < text.length) {
    const dotStart = text.indexOf(needle, cursor);
    if (dotStart < 0) {
      return undefined;
    }
    const fieldStart = dotStart + 1;
    const fieldEnd = fieldStart + field.length;
    if (dotStart > 0 && isIdentifierPart(text[dotStart - 1]) && !isIdentifierPart(text[fieldEnd])) {
      return { start: fieldStart, end: fieldEnd };
    }
    cursor = fieldEnd;
  }
  return undefined;
}

function functionCallNameRange(lineText, name, startCharacter = 0) {
  const text = String(lineText || "");
  const searchStart = Math.max(0, Math.min(startCharacter, text.length));
  return functionCallNameRangeFrom(text, name, searchStart)
    ?? functionCallNameRangeFrom(text, name, 0);
}

function functionCallNameRangeFrom(text, name, searchStart) {
  let cursor = searchStart;
  while (cursor < text.length) {
    const start = text.indexOf(name, cursor);
    if (start < 0) {
      return undefined;
    }
    const end = start + name.length;
    let next = end;
    while (next < text.length && /\s/.test(text[next])) {
      next += 1;
    }
    if (!isIdentifierPart(text[start - 1]) && !isIdentifierPart(text[end]) && text[next] === "(") {
      return { start, end };
    }
    cursor = end;
  }
  return undefined;
}

function logLevelRange(lineText) {
  const text = String(lineText || "");
  const logStart = lineIndentLength(text);
  if (text.slice(logStart, logStart + 3) !== "log" || !/\s/.test(text[logStart + 3] || "")) {
    return undefined;
  }
  let levelStart = logStart + 3;
  while (levelStart < text.length && /\s/.test(text[levelStart])) {
    levelStart += 1;
  }
  const first = text[levelStart];
  if (first === '"' || first === "#") {
    return { start: logStart, end: logStart + 3 };
  }
  let levelEnd = levelStart;
  while (levelEnd < text.length && !/\s|#/.test(text[levelEnd])) {
    levelEnd += 1;
  }
  return levelEnd > levelStart ? { start: levelStart, end: levelEnd } : undefined;
}

function netUrlLiteralRange(lineText, startCharacter = 0) {
  const text = String(lineText || "");
  const searchStart = Math.max(0, Math.min(startCharacter, text.length));
  return callStringArgumentRange(text, "url", searchStart)
    ?? callStringArgumentRange(text, "url", 0)
    ?? stringLiteralRange(text, searchStart)
    ?? stringLiteralRange(text, 0);
}

function callStringArgumentRange(text, functionName, startCharacter = 0) {
  let cursor = startCharacter;
  while (cursor < text.length) {
    const start = text.indexOf(functionName, cursor);
    if (start < 0) {
      return undefined;
    }
    const end = start + functionName.length;
    let open = end;
    while (open < text.length && /\s/.test(text[open])) {
      open += 1;
    }
    if (!isIdentifierPart(text[start - 1]) && !isIdentifierPart(text[end]) && text[open] === "(") {
      let argumentStart = open + 1;
      while (argumentStart < text.length && /\s/.test(text[argumentStart])) {
        argumentStart += 1;
      }
      const range = stringLiteralRangeAt(text, argumentStart);
      if (range) {
        return range;
      }
    }
    cursor = end;
  }
  return undefined;
}

function stringLiteralRange(text, startCharacter = 0) {
  const quoteStart = text.indexOf('"', Math.max(0, startCharacter));
  return quoteStart >= 0 ? stringLiteralRangeAt(text, quoteStart) : undefined;
}

function stringLiteralRangeAt(text, quoteStart) {
  if (text[quoteStart] !== '"') {
    return undefined;
  }
  let escaped = false;
  for (let cursor = quoteStart + 1; cursor < text.length; cursor += 1) {
    const character = text[cursor];
    if (escaped) {
      escaped = false;
      continue;
    }
    if (character === "\\") {
      escaped = true;
      continue;
    }
    if (character === '"') {
      return { start: quoteStart, end: cursor + 1 };
    }
  }
  return undefined;
}

function diagnosticBacktickRange(lineText, item) {
  return backtickPayloadRange(lineText, item?.message)
    ?? backtickPayloadRange(lineText, item?.help);
}

function uncertaintyArgumentDiagnosticRange(lineText, item) {
  const message = String(item?.message ?? "");
  if (message.includes("sample count")) {
    return namedArgumentValueRange(lineText, ["samples", "n"]);
  }
  if (message.includes("standard deviation")) {
    return namedArgumentValueRange(lineText, ["std", "sigma", "uncertainty"]);
  }
  if (message.includes("relative error")) {
    return namedArgumentValueRange(lineText, ["error", "relative_error"]);
  }
  if (message.includes("scale/gain")) {
    return namedArgumentValueRange(lineText, ["scale", "gain"]);
  }
  if (message.includes("offset/bias")) {
    return namedArgumentValueRange(lineText, ["offset", "bias"]);
  }
  if (message.includes("lower bound") && message.includes("upper bound")) {
    return namedArgumentValueRange(lineText, ["lower", "min"])
      ?? namedArgumentValueRange(lineText, ["upper", "max"]);
  }
  return undefined;
}

function namedArgumentValueRange(lineText, names) {
  const code = stripLineComment(lineText);
  for (const name of names) {
    const pattern = new RegExp(`(^|[^A-Za-z0-9_])(${escapeRegExp(name)})(\\s*=\\s*)`, "g");
    let match;
    while ((match = pattern.exec(code)) !== null) {
      const nameStart = match.index + match[1].length;
      const valueStart = nameStart + name.length + match[3].length;
      let valueEnd = valueStart;
      while (valueEnd < code.length && code[valueEnd] !== "," && code[valueEnd] !== ")") {
        valueEnd += 1;
      }
      while (valueEnd > valueStart && /\s/.test(code[valueEnd - 1])) {
        valueEnd -= 1;
      }
      if (valueEnd > valueStart) {
        return { start: valueStart, end: valueEnd };
      }
    }
  }
  return undefined;
}

function escapeRegExp(text) {
  return String(text).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function formatInterpolationDiagnosticRange(lineText, item) {
  const code = diagnosticCodeValue(item);
  if (code === "E-PRINT-FMT-003" || code === "E-WRITE-FMT-003") {
    const payload = lastBacktickPayload(item?.message);
    return payload ? formatInterpolationPayloadRange(lineText, payload, "unit") : undefined;
  }
  if (code === "E-PRINT-FMT-004" || code === "E-WRITE-FMT-004") {
    const payload = firstBacktickPayload(item?.message);
    return payload ? formatInterpolationPayloadRange(lineText, payload, "expression") : undefined;
  }
  if (code === "E-PRINT-FMT-002" || code === "E-WRITE-FMT-002") {
    return emptyFormatInterpolationRange(lineText);
  }
  if (code === "E-PRINT-FMT-001" || code === "E-WRITE-FMT-001") {
    return unterminatedFormatInterpolationRange(lineText);
  }
  return undefined;
}

function formatInterpolationPayloadRange(lineText, payload, payloadKind) {
  const text = String(lineText || "");
  for (const literal of stringLiteralRanges(text)) {
    const contentStart = literal.start + 1;
    const contentEnd = Math.max(contentStart, literal.end - 1);
    const content = text.slice(contentStart, contentEnd);
    let cursor = 0;
    while (cursor < content.length) {
      const openOffset = content.indexOf("{", cursor);
      if (openOffset < 0) break;
      const fieldStart = openOffset + 1;
      const closeOffset = content.indexOf("}", fieldStart);
      if (closeOffset < 0) break;
      const field = content.slice(fieldStart, closeOffset);
      const fieldOffset = contentStart + fieldStart;
      const range = payloadKind === "unit"
        ? formatUnitRangeInField(field, fieldOffset, payload)
        : formatExpressionRangeInField(field, fieldOffset, payload);
      if (range) return range;
      cursor = closeOffset + 1;
    }
  }
  return undefined;
}

function formatExpressionRangeInField(field, fieldOffset, payload) {
  const colon = field.indexOf(":");
  const expression = colon >= 0 ? field.slice(0, colon) : field;
  const range = trimmedRange(expression, fieldOffset);
  return range && range.text === payload ? { start: range.start, end: range.end } : undefined;
}

function formatUnitRangeInField(field, fieldOffset, payload) {
  const colon = field.indexOf(":");
  if (colon < 0) return undefined;
  const specStart = colon + 1;
  const spec = field.slice(specStart);
  let cursor = leadingWhitespaceLength(spec);
  const afterLeading = spec.slice(cursor);
  if (afterLeading.startsWith(".")) {
    cursor += 1;
    while (cursor < spec.length && /[0-9]/.test(spec[cursor])) {
      cursor += 1;
    }
  }
  const range = trimmedRange(spec.slice(cursor), fieldOffset + specStart + cursor);
  return range && range.text === payload ? { start: range.start, end: range.end } : undefined;
}

function emptyFormatInterpolationRange(lineText) {
  const text = String(lineText || "");
  for (const literal of stringLiteralRanges(text)) {
    const contentStart = literal.start + 1;
    const contentEnd = Math.max(contentStart, literal.end - 1);
    const content = text.slice(contentStart, contentEnd);
    let cursor = 0;
    while (cursor < content.length) {
      const openOffset = content.indexOf("{", cursor);
      if (openOffset < 0) break;
      const fieldStart = openOffset + 1;
      const closeOffset = content.indexOf("}", fieldStart);
      if (closeOffset < 0) break;
      if (content.slice(fieldStart, closeOffset).trim() === "") {
        return { start: contentStart + openOffset, end: contentStart + closeOffset + 1 };
      }
      cursor = closeOffset + 1;
    }
  }
  return undefined;
}

function unterminatedFormatInterpolationRange(lineText) {
  const text = String(lineText || "");
  for (const literal of stringLiteralRanges(text)) {
    const contentStart = literal.start + 1;
    const contentEnd = Math.max(contentStart, literal.end - 1);
    const content = text.slice(contentStart, contentEnd);
    let cursor = 0;
    while (cursor < content.length) {
      const openOffset = content.indexOf("{", cursor);
      if (openOffset < 0) break;
      const fieldStart = openOffset + 1;
      const closeOffset = content.indexOf("}", fieldStart);
      if (closeOffset < 0) {
        const start = contentStart + openOffset;
        return { start, end: start + 1 };
      }
      cursor = closeOffset + 1;
    }
  }
  return undefined;
}

function stringLiteralRanges(lineText) {
  const text = String(lineText || "");
  const ranges = [];
  let cursor = 0;
  while (cursor < text.length) {
    const quoteStart = text.indexOf('"', cursor);
    if (quoteStart < 0) break;
    const range = stringLiteralRangeAt(text, quoteStart);
    if (!range) break;
    ranges.push(range);
    cursor = range.end;
  }
  return ranges;
}

function trimmedRange(value, offset) {
  const raw = String(value || "");
  const leading = leadingWhitespaceLength(raw);
  const trimmed = raw.trim();
  if (!trimmed) return undefined;
  const start = offset + leading;
  return { start, end: start + trimmed.length, text: trimmed };
}

function leadingWhitespaceLength(value) {
  const match = String(value || "").match(/^\s*/);
  return match ? match[0].length : 0;
}

function firstBacktickPayload(text) {
  const match = /`([^`]+)`/.exec(String(text || ""));
  return match ? match[1].trim() : undefined;
}

function lastBacktickPayload(text) {
  let last;
  for (const match of String(text || "").matchAll(/`([^`]+)`/g)) {
    const payload = match[1].trim();
    if (payload) last = payload;
  }
  return last;
}

function backtickPayloadRange(lineText, text) {
  for (const match of String(text || "").matchAll(/`([^`]+)`/g)) {
    const payload = match[1].trim();
    if (!payload) {
      continue;
    }
    const range = firstNeedleRange(lineText, [payload], 0);
    if (range) {
      return range;
    }
  }
  return undefined;
}

function isIdentifierPart(character) {
  return typeof character === "string" && /^[A-Za-z0-9_]$/.test(character);
}

function lineIndentLength(lineText) {
  const match = String(lineText || "").match(/^\s*/);
  return match ? match[0].length : 0;
}

function sourceLineNumber(item) {
  return positiveIntegerOrUndefined(item?.source_span?.line)
    ?? positiveIntegerOrUndefined(item?.sourceSpan?.line)
    ?? positiveIntegerOrUndefined(item?.source_line)
    ?? positiveIntegerOrUndefined(item?.sourceLine)
    ?? positiveIntegerOrUndefined(item?.line);
}

function sourceColumnNumber(item) {
  return positiveIntegerOrUndefined(item?.source_span?.column)
    ?? positiveIntegerOrUndefined(item?.sourceSpan?.column)
    ?? positiveIntegerOrUndefined(item?.source_column)
    ?? positiveIntegerOrUndefined(item?.sourceColumn)
    ?? positiveIntegerOrUndefined(item?.column);
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

function diagnosticTokenEndCharacter(lineText, startCharacter, maxCharacter) {
  const text = String(lineText || "");
  let cursor = Math.max(0, Math.min(startCharacter, text.length));
  while (cursor < text.length && /\s/.test(text[cursor])) {
    cursor += 1;
  }
  const tokenStart = cursor;
  for (const character of text.slice(cursor)) {
    if (/\s/.test(character) || /[()[\]{},;:]/.test(character)) {
      break;
    }
    cursor += character.length;
  }
  if (cursor === tokenStart && cursor < text.length) {
    const firstCharacter = Array.from(text.slice(cursor))[0];
    cursor += firstCharacter?.length ?? 1;
  }
  return Math.max(startCharacter + 1, Math.min(cursor, maxCharacter));
}

function integerOrUndefined(value) {
  const number = Number(value);
  return Number.isInteger(number) ? number : undefined;
}

function positiveIntegerOrUndefined(value) {
  const number = integerOrUndefined(value);
  return number !== undefined && number > 0 ? number : undefined;
}

function diagnosticCode(item) {
  const value = diagnosticCodeValue(item);
  if (!value) {
    return undefined;
  }
  const target = diagnosticCodeTarget(value);
  return target ? { value, target } : value;
}

function diagnosticCodeValue(item) {
  const code = item?.code;
  if (typeof code === "string" && code.length > 0) {
    return code;
  }
  const value = code?.value;
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

function diagnosticCodeTarget(code) {
  if (!/^[EW]-/.test(code)) {
    return undefined;
  }
  return vscode.Uri.parse(DIAGNOSTIC_DOC_TARGETS.get(code) ?? DIAGNOSTIC_CODE_DOC);
}

function diagnosticTags(item) {
  const protocolTags = Array.isArray(item?.tags) ? item.tags : [];
  const tags = new Set();
  if (protocolTags.includes(1)) {
    tags.add(vscode.DiagnosticTag.Unnecessary);
  }
  if (protocolTags.includes(2)) {
    tags.add(vscode.DiagnosticTag.Deprecated);
  }
  const code = diagnosticCodeValue(item) ?? "";
  const message = String(item?.message ?? "").toLowerCase();
  if (
    code === "W-TABLE-LEGACY-SELECT-FIRST-ROW" ||
    code === "W-ML-TRAIN-ALIAS" ||
    code === "W-ML-ANN-ALIAS" ||
    code === "E-SCRIPT-001" ||
    code === "E-STRUCT-ARGS-001" ||
    message.includes("legacy") ||
    message.includes("deprecated")
  ) {
    tags.add(vscode.DiagnosticTag.Deprecated);
  }
  return Array.from(tags);
}

function diagnosticsFailureDetail(details = {}) {
  const problemParts = [];
  const error = details.error;
  if (error?.code !== undefined) {
    problemParts.push(`exit code ${error.code}`);
  } else if (error?.signal) {
    problemParts.push(`signal ${error.signal}`);
  }
  if (details.parseError?.message) {
    problemParts.push(`invalid JSON: ${compactDiagnosticText(details.parseError.message, 120)}`);
  }

  const stderrText = compactDiagnosticText(details.stderr);
  const stdoutText = compactDiagnosticText(details.stdout);
  if (stderrText) {
    problemParts.push(`stderr: ${stderrText}`);
  } else if (stdoutText) {
    problemParts.push(`stdout: ${stdoutText}`);
  } else if (error?.message) {
    problemParts.push(compactDiagnosticText(error.message));
  } else if (!stdoutText) {
    problemParts.push("empty stdout");
  }

  return {
    problemMessage: problemParts.filter(Boolean).join("; "),
    hasOutput: Boolean(stderrText || stdoutText)
  };
}

function compactDiagnosticText(value, maxLength = 220) {
  const text = String(value ?? "").replace(/\s+/g, " ").trim();
  if (text.length <= maxLength) {
    return text;
  }
  return `${text.slice(0, Math.max(0, maxLength - 1))}...`;
}

function diagnosticSource(runtimeLabel) {
  const label = String(runtimeLabel ?? "").toLowerCase();
  if (label.includes("live")) {
    return "eng/live";
  }
  if (label.includes("file")) {
    return "eng/file";
  }
  return "eng";
}

function diagnosticsSettingHint(runtimeLabel) {
  const label = String(runtimeLabel ?? "").toLowerCase();
  if (label.includes("live")) {
    return "englang.lspPath";
  }
  if (label.includes("file")) {
    return "englang.runtimePath";
  }
  return "englang.runtimePath or englang.lspPath";
}

function severityName(severity) {
  if (severity === 1 || severity === "error") {
    return "error";
  }
  if (severity === 2 || severity === "warning") {
    return "warning";
  }
  if (severity === 4 || severity === "hint") {
    return "hint";
  }
  return "info";
}

function toVscodeSeverity(severity) {
  const name = severityName(severity);
  if (name === "error") {
    return vscode.DiagnosticSeverity.Error;
  }
  if (name === "warning") {
    return vscode.DiagnosticSeverity.Warning;
  }
  if (name === "hint") {
    return vscode.DiagnosticSeverity.Hint ?? vscode.DiagnosticSeverity.Information;
  }
  return vscode.DiagnosticSeverity.Information;
}

function firstLineRange(document) {
  const line = document.lineAt(0);
  return new vscode.Range(0, 0, 0, Math.max(1, line.text.length));
}

module.exports = {
  EngDiagnosticsController,
  diagnosticCode,
  diagnosticCodeTarget,
  diagnosticFallbackRangeForCode,
  diagnosticRange,
  diagnosticSource,
  diagnosticTags,
  diagnosticsFailureDetail,
  diagnosticsSettingHint,
  severityName,
  sourceColumnCharacter,
  toDiagnostics
};
