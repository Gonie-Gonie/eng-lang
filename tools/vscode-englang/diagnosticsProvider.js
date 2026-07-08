const cp = require("child_process");
const vscode = require("vscode");

const CHECK_DEBOUNCE_MS = 350;

class EngDiagnosticsController {
  constructor(context, diagnostics, options = {}) {
    this.context = context;
    this.diagnostics = diagnostics;
    this.output = options.output;
    this.isEngDocument = options.isEngDocument ?? (() => true);
    this.clearSnapshotCache = options.clearSnapshotCache ?? (() => undefined);
    this.diagnosticsRuntime = options.diagnosticsRuntime;
    this.diagnosticsRuntimeLabel = options.diagnosticsRuntimeLabel ?? ((runtimeMode) => runtimeMode);
    this.findLspRuntime = options.findLspRuntime;
    this.findRuntime = options.findRuntime;
    this.snapshotDocumentSource = options.snapshotDocumentSource;
    this.workspaceRoot = options.workspaceRoot;
    this.cacheReview = options.cacheReview ?? (() => undefined);
    this.updateReviewRiskDecorations = options.updateReviewRiskDecorations ?? (() => undefined);
    this.updateSemanticSymbolDecorations = options.updateSemanticSymbolDecorations ?? (() => undefined);
    this.changeTimers = new Map();
  }

  maybeCheck(document) {
    if (!this.isEngDocument(document)) {
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
    if (this.diagnosticsRuntime?.(document) !== "lsp-snapshot") {
      return;
    }
    const config = vscode.workspace.getConfiguration("englang", document.uri);
    if (!config.get("lintOnChange", true)) {
      return;
    }
    this.clearPendingCheck(document);
    const key = document.uri.toString();
    const timer = setTimeout(() => {
      this.changeTimers.delete(key);
      this.checkDocumentSource(document);
    }, CHECK_DEBOUNCE_MS);
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

  async checkActiveFile() {
    const document = vscode.window.activeTextEditor?.document;
    if (!document || !this.isEngDocument(document)) {
      vscode.window.showWarningMessage("Open an EngLang .eng file first.");
      return;
    }
    if (document.isDirty) {
      this.checkDocumentSource(document);
      return;
    }
    await this.checkDocument(document);
  }

  checkDocument(document) {
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

    cp.execFile(
      runtime,
      args,
      { cwd, maxBuffer: 10 * 1024 * 1024 },
      (error, stdout, stderr) => {
        this.finishDocumentCheck(document, runtimeLabel, documentVersion, error, stdout, stderr);
      }
    );
  }

  checkDocumentSource(document) {
    if (this.snapshotDocumentSource) {
      const documentVersion = document.version;
      this.appendLine(`live buffer check ${document.uri.fsPath}`);
      this.snapshotDocumentSource(document, this.context)
        .then((review) => {
          if (document.version !== documentVersion) {
            return;
          }
          if (!review) {
            this.applyUnavailableSnapshotDiagnostic(document);
            return;
          }
          this.finishParsedDocumentCheck(document, "live buffer", documentVersion, review);
        })
        .catch((error) => {
          this.appendLine(`live buffer check failed: ${error.message}`);
          this.applyUnavailableSnapshotDiagnostic(document);
        });
      return;
    }

    const runtime = this.findLspRuntime(this.context, document);
    const cwd = this.workspaceRoot(document);
    const documentVersion = document.version;
    this.appendLine(`live buffer check ${document.uri.fsPath}`);

    const child = cp.execFile(
      runtime,
      ["--snapshot-stdin", document.uri.fsPath],
      { cwd, maxBuffer: 10 * 1024 * 1024 },
      (error, stdout, stderr) => {
        this.finishDocumentCheck(document, "live buffer", documentVersion, error, stdout, stderr);
      }
    );
    if (child.stdin) {
      child.stdin.end(document.getText());
    }
  }

  finishDocumentCheck(document, runtimeLabel, documentVersion, error, stdout, stderr) {
    if (document.version !== documentVersion) {
      return;
    }
    if (stderr && stderr.trim().length > 0) {
      this.appendLine(stderr.trim());
    }

    let review;
    try {
      review = JSON.parse(stdout);
    } catch (parseError) {
      this.appendLine(`Unable to parse EngLang ${runtimeLabel} output: ${parseError.message}`);
      if (error) {
        this.appendLine(error.message);
      }
      this.applyUnavailableSnapshotDiagnostic(document);
      return;
    }

    this.finishParsedDocumentCheck(document, runtimeLabel, documentVersion, review);
  }

  finishParsedDocumentCheck(document, runtimeLabel, documentVersion, review) {
    if (document.version !== documentVersion) {
      return;
    }
    this.cacheReview(document, review);
    this.diagnostics.set(document.uri, toDiagnostics(document, review));
    this.updateReviewRiskDecorations(document, review);
    this.updateSemanticSymbolDecorations(document, review);
    const errors = review.diagnostics?.filter((item) => severityName(item.severity) === "error").length ?? 0;
    const warnings = review.diagnostics?.filter((item) => severityName(item.severity) === "warning").length ?? 0;
    this.appendLine(`diagnostics: ${errors} error(s), ${warnings} warning(s)`);
  }

  applyUnavailableSnapshotDiagnostic(document) {
    this.diagnostics.set(document.uri, [
      new vscode.Diagnostic(
        firstLineRange(document),
        "EngLang runtime did not return editor JSON. Check englang.runtimePath or englang.lspPath.",
        vscode.DiagnosticSeverity.Error
      )
    ]);
    this.updateReviewRiskDecorations(document, undefined);
    this.updateSemanticSymbolDecorations(document, undefined);
  }

  appendLine(message) {
    this.output?.appendLine(message);
  }
}

function toDiagnostics(document, review) {
  return (review.diagnostics ?? []).map((item) => {
    const sourceLine = item.range?.start?.line ?? Math.max(0, (item.line ?? 1) - 1);
    const line = Math.max(0, Math.min(sourceLine, document.lineCount - 1));
    const textLine = document.lineAt(line);
    const maxCharacter = Math.max(1, textLine.text.length);
    const startCharacter = Math.max(
      0,
      Math.min(item.range?.start?.character ?? 0, maxCharacter - 1)
    );
    const endCharacter = Math.max(
      startCharacter + 1,
      Math.min(item.range?.end?.character ?? maxCharacter, maxCharacter)
    );
    const range = new vscode.Range(line, startCharacter, line, endCharacter);
    const severity = toVscodeSeverity(item.severity);
    const diagnostic = new vscode.Diagnostic(range, item.message, severity);
    diagnostic.code = item.code;
    diagnostic.source = "eng";
    if (item.help) {
      diagnostic.message = `${item.message}\n${item.help}`;
    }
    return diagnostic;
  });
}

function severityName(severity) {
  if (severity === 1 || severity === "error") {
    return "error";
  }
  if (severity === 2 || severity === "warning") {
    return "warning";
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
  return vscode.DiagnosticSeverity.Information;
}

function firstLineRange(document) {
  const line = document.lineAt(0);
  return new vscode.Range(0, 0, 0, Math.max(1, line.text.length));
}

module.exports = {
  EngDiagnosticsController,
  severityName,
  toDiagnostics
};
