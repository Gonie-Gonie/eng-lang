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
    this.diagnosticsRuntimeLabel = options.diagnosticsRuntimeLabel ?? ((runtimeMode) => runtimeMode);
    this.findLspRuntime = options.findLspRuntime;
    this.findRuntime = options.findRuntime;
    this.snapshotDocumentSource = options.snapshotDocumentSource;
    this.workspaceRoot = options.workspaceRoot;
    this.cacheReview = options.cacheReview ?? (() => undefined);
    this.clearCachedReview = options.clearCachedReview ?? (() => undefined);
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

  clearDocumentDiagnostics(document, reason = "diagnostics mode changed") {
    if (!document || !this.isEngDocument(document)) {
      return;
    }
    this.clearPendingCheck(document);
    this.clearCachedReview(document);
    this.diagnostics.delete(document.uri);
    this.updateReviewRiskDecorations(document, undefined);
    this.updateSemanticSymbolDecorations(document, undefined);
    this.appendLine(`Problems cleared for ${document.uri.fsPath}: ${reason}`);
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
    this.appendLine(`Problems source: ${diagnosticSource(runtimeLabel)}; diagnostics: ${runtimeLabel}; tool: ${runtime}`);

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
      const runtime = this.findLspRuntime?.(this.context, document) ?? "eng-lsp.exe";
      this.appendLine(`live buffer check ${document.uri.fsPath}`);
      this.appendLine(`Problems source: ${diagnosticSource("live buffer")}; diagnostics: live buffer; tool: ${runtime}`);
      this.snapshotDocumentSource(document, this.context)
        .then((review) => {
          if (document.version !== documentVersion) {
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
          this.finishParsedDocumentCheck(document, "live buffer", documentVersion, review);
        })
        .catch((error) => {
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
    this.finishParsedDocumentCheck(document, runtimeLabel, documentVersion, review);
  }

  finishParsedDocumentCheck(document, runtimeLabel, documentVersion, review) {
    if (document.version !== documentVersion) {
      return;
    }
    this.cacheReview(document, review);
    this.diagnostics.set(document.uri, toDiagnostics(document, review, {
      source: diagnosticSource(runtimeLabel)
    }));
    this.updateReviewRiskDecorations(document, review);
    this.updateSemanticSymbolDecorations(document, review);
    const errors = review.diagnostics?.filter((item) => severityName(item.severity) === "error").length ?? 0;
    const warnings = review.diagnostics?.filter((item) => severityName(item.severity) === "warning").length ?? 0;
    this.appendLine(`diagnostics (${diagnosticSource(runtimeLabel)}): ${errors} error(s), ${warnings} warning(s)`);
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
    this.diagnostics.set(document.uri, [diagnostic]);
    this.updateReviewRiskDecorations(document, undefined);
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
  return undefined;
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
  const code = diagnosticCodeValue(item) ?? "";
  const message = String(item?.message ?? "").toLowerCase();
  if (
    code === "W-TABLE-LEGACY-SELECT-FIRST-ROW" ||
    code === "E-SCRIPT-001" ||
    code === "E-STRUCT-ARGS-001" ||
    message.includes("legacy") ||
    message.includes("deprecated")
  ) {
    return [vscode.DiagnosticTag.Deprecated];
  }
  return [];
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
