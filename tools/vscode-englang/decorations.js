const vscode = require("vscode");
const { semanticTokenRange } = require("./lspSemanticTokens");
const {
  firstReviewArray,
  lineValue,
  normalizedReviewDocument,
  reviewValue
} = require("./reviewPanelRenderer");

function createDecorationController(options = {}) {
  const {
    isEngDocument = () => false,
    reviewCache = new Map(),
    timeAlignmentReviewCache = new Map()
  } = options;
  const reviewRiskDecorations = createReviewRiskDecorationTypes();
  const reviewValidationDecorations = createReviewValidationDecorationTypes();
  const semanticSymbolDecorations = createSemanticSymbolDecorationTypes();
  const timeAlignmentDecorations = createTimeAlignmentDecorationTypes();

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
        updateReviewRiskDecorations(editor.document, cached);
        updateSemanticSymbolDecorations(editor.document, cached);
      }
    }
  }

  function refreshVisibleReviewValidationDecorations() {
    for (const editor of vscode.window.visibleTextEditors) {
      if (isEngDocument(editor.document)) {
        const cached = reviewCache.get(editor.document.uri.fsPath);
        updateReviewValidationDecorations(editor.document, cached);
      }
    }
  }

  function refreshVisibleTimeAlignmentDecorations() {
    for (const editor of vscode.window.visibleTextEditors) {
      if (isEngDocument(editor.document)) {
        const cached = timeAlignmentReviewCache.get(editor.document.uri.fsPath);
        updateTimeAlignmentDecorations(editor.document, cached);
      }
    }
  }

  function refreshVisibleSemanticSymbolDecorations() {
    for (const editor of vscode.window.visibleTextEditors) {
      if (isEngDocument(editor.document)) {
        const cached = reviewCache.get(editor.document.uri.fsPath);
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
    const config = vscode.workspace.getConfiguration("englang", document.uri);
    const decorations = config.get("semanticHighlighting.enabled", true)
      ? semanticSymbolDecorationOptions(document, snapshot)
      : { internal: [], planned: [] };
    for (const editor of editors) {
      editor.setDecorations(semanticSymbolDecorations.internal, decorations.internal);
      editor.setDecorations(semanticSymbolDecorations.planned, decorations.planned);
    }
  }

  function updateReviewValidationDecorations(document, review) {
    if (!reviewValidationDecorations || !isEngDocument(document)) {
      return;
    }
    const editors = vscode.window.visibleTextEditors.filter(
      (editor) => editor.document.uri.toString() === document.uri.toString()
    );
    if (editors.length === 0) {
      return;
    }
    const config = vscode.workspace.getConfiguration("englang", document.uri);
    const decorations = config.get("validationDecorations.enabled", true)
      ? reviewValidationDecorationOptions(document, review)
      : { pass: [], fail: [] };
    for (const editor of editors) {
      editor.setDecorations(reviewValidationDecorations.pass, decorations.pass);
      editor.setDecorations(reviewValidationDecorations.fail, decorations.fail);
    }
  }

  function updateTimeAlignmentDecorations(document, review) {
    if (!timeAlignmentDecorations || !isEngDocument(document)) {
      return;
    }
    const editors = vscode.window.visibleTextEditors.filter(
      (editor) => editor.document.uri.toString() === document.uri.toString()
    );
    if (editors.length === 0) {
      return;
    }
    const config = vscode.workspace.getConfiguration("englang", document.uri);
    const decorations = config.get("timeAlignmentDecorations.enabled", true)
      ? timeAlignmentDecorationOptions(document, review)
      : [];
    for (const editor of editors) {
      editor.setDecorations(timeAlignmentDecorations.warning, decorations);
    }
  }

  return {
    disposables: [
      reviewRiskDecorations.high,
      reviewRiskDecorations.medium,
      reviewValidationDecorations.pass,
      reviewValidationDecorations.fail,
      semanticSymbolDecorations.internal,
      semanticSymbolDecorations.planned,
      timeAlignmentDecorations.warning
    ],
    refreshVisibleReviewRiskDecorations,
    refreshVisibleReviewValidationDecorations,
    refreshVisibleSemanticSymbolDecorations,
    refreshVisibleTimeAlignmentDecorations,
    updateReviewRiskDecorations,
    updateReviewValidationDecorations,
    updateSemanticSymbolDecorations,
    updateTimeAlignmentDecorations
  };
}

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

function createReviewValidationDecorationTypes() {
  return {
    pass: vscode.window.createTextEditorDecorationType({
      after: {
        contentText: "  validation passed",
        color: new vscode.ThemeColor("testing.iconPassed"),
        fontStyle: "italic",
        margin: "0 0 0 0.75rem"
      }
    }),
    fail: vscode.window.createTextEditorDecorationType({
      after: {
        contentText: "  validation failed",
        color: new vscode.ThemeColor("testing.iconFailed"),
        fontStyle: "italic",
        margin: "0 0 0 0.75rem"
      },
      overviewRulerColor: new vscode.ThemeColor("testing.iconFailed"),
      overviewRulerLane: vscode.OverviewRulerLane.Right
    })
  };
}

function createTimeAlignmentDecorationTypes() {
  return {
    warning: vscode.window.createTextEditorDecorationType({
      after: {
        color: new vscode.ThemeColor("editorWarning.foreground"),
        fontStyle: "italic",
        margin: "0 0 0 0.75rem"
      },
      overviewRulerColor: new vscode.ThemeColor("editorWarning.foreground"),
      overviewRulerLane: vscode.OverviewRulerLane.Right
    })
  };
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
    markdown.appendMarkdown("**EngLang planned stdlib import**\n\nThis import names a documented EngLang module that is not yet executable as a public stdlib API.");
  } else {
    markdown.appendMarkdown("**EngLang internal stdlib import**\n\nThis import names compiler/runtime vocabulary outside the public stdlib API.");
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

function reviewValidationDecorationOptions(document, review) {
  const doc = normalizedReviewDocument(review);
  const records = firstReviewArray(doc, review, "validations");
  const byLine = new Map();
  for (const record of records) {
    const status = String(record?.status ?? "").toLowerCase();
    if (status !== "pass" && status !== "fail") {
      continue;
    }
    const lineNumber = Number(lineValue(record));
    if (!Number.isFinite(lineNumber) || lineNumber < 1 || lineNumber > document.lineCount) {
      continue;
    }
    const safeLine = Math.trunc(lineNumber);
    const item = byLine.get(safeLine) ?? { status: "pass", records: [] };
    item.records.push(record);
    if (status === "fail") {
      item.status = "fail";
    }
    byLine.set(safeLine, item);
  }

  const pass = [];
  const fail = [];
  for (const [lineNumber, item] of [...byLine.entries()].sort((left, right) => left[0] - right[0])) {
    const option = {
      range: lineEndRange(document, lineNumber - 1),
      hoverMessage: reviewValidationHoverMessage(item.status, item.records)
    };
    if (item.status === "fail") {
      fail.push(option);
    } else {
      pass.push(option);
    }
  }
  return { pass, fail };
}

function timeAlignmentDecorationOptions(document, review) {
  const doc = normalizedReviewDocument(review);
  const records = firstReviewArray(doc, review, "time_alignments", "timeAlignments");
  const warnings = [];
  for (const record of records) {
    const warning = timeAlignmentWarning(record);
    const lineNumber = Number(lineValue(record));
    if (
      !warning
      || !Number.isFinite(lineNumber)
      || lineNumber < 1
      || lineNumber > document.lineCount
    ) {
      continue;
    }
    warnings.push({
      range: lineEndRange(document, Math.trunc(lineNumber) - 1),
      hoverMessage: timeAlignmentHoverMessage(record, warning),
      renderOptions: {
        after: { contentText: `  ${warning.label}` }
      }
    });
  }
  return warnings;
}

function timeAlignmentWarning(record) {
  const strategy = String(reviewValue(record, "strategy", "strategy", "")).toLowerCase();
  const materializationStatus = String(
    reviewValue(record, "materialization_status", "materializationStatus", "")
  ).toLowerCase();
  if (strategy === "auto_pairwise" || materializationStatus === "not_requested") {
    return undefined;
  }
  if (materializationStatus === "materialized") {
    return undefined;
  }
  if (materializationStatus === "unavailable") {
    return { kind: "unavailable", label: "alignment unavailable" };
  }
  if (materializationStatus === "partial") {
    const outputCount = Number(reviewValue(record, "output_count", "outputCount", 0));
    const targetCount = Number(reviewValue(record, "target_count", "targetCount", 0));
    const countLabel = targetCount > 0
      ? ` ${Math.max(0, outputCount)}/${targetCount}`
      : "";
    return { kind: "partial", label: `alignment partial${countLabel}` };
  }
  const alignmentStatus = String(
    reviewValue(record, "alignment_status", "status", "")
  ).toLowerCase();
  if (alignmentStatus === "mismatch") {
    return { kind: "mismatch", label: "alignment mismatch" };
  }
  const stepStatus = String(reviewValue(record, "step_status", "stepStatus", "")).toLowerCase();
  if (stepStatus === "mismatch" || stepStatus === "irregular") {
    return { kind: "step", label: "alignment step mismatch" };
  }
  return undefined;
}

function timeAlignmentHoverMessage(record, warning) {
  const markdown = new vscode.MarkdownString();
  markdown.isTrusted = false;
  markdown.appendMarkdown("**EngLang TimeSeries alignment warning**");
  const binding = reviewValue(record, "binding", "binding", "alignment output");
  const left = reviewValue(record, "left", "left", "?");
  const right = reviewValue(record, "right", "right", "?");
  const strategy = reviewValue(record, "strategy", "strategy", "?");
  const method = reviewValue(record, "method", "method", "?");
  const materializationStatus = reviewValue(
    record,
    "materialization_status",
    "materializationStatus",
    warning.kind
  );
  const outputCount = reviewValue(record, "output_count", "outputCount", 0);
  const targetCount = reviewValue(record, "target_count", "targetCount", 0);
  const reason = reviewValue(
    record,
    "materialization_reason",
    "materializationReason",
    warning.label
  );
  markdown.appendText(`\n\nLatest saved run: ${binding}`);
  markdown.appendText(`\nSource/target: ${left} -> ${right}`);
  markdown.appendText(`\nStrategy: ${strategy}; method: ${method}`);
  markdown.appendText(`\nMaterialization: ${materializationStatus} (${outputCount}/${targetCount} points)`);
  markdown.appendText(`\nReason: ${reason}`);
  return markdown;
}

function reviewValidationHoverMessage(status, records) {
  const markdown = new vscode.MarkdownString();
  markdown.isTrusted = false;
  markdown.appendMarkdown(
    status === "fail"
      ? "**EngLang validation failed**"
      : "**EngLang validation passed**"
  );
  for (const record of records) {
    const target = reviewValue(record, "target", "target", "validation");
    const className = reviewValue(record, "class_name", "className", "");
    const expression = reviewValue(record, "expression", "expression", "");
    const leftValue = reviewValue(record, "left_value", "leftValue", "");
    const rightValue = reviewValue(record, "right_value", "rightValue", "");
    const operator = reviewValue(record, "operator", "operator", "");
    const recordStatus = String(reviewValue(record, "status", "status", "")).toLowerCase();
    const resultLabel = recordStatus === "fail" ? "failed" : "passed";
    markdown.appendMarkdown("\n\n");
    markdown.appendText(
      `${className ? `${target} (${className})` : String(target)} - ${resultLabel}`
    );
    if (expression) {
      markdown.appendText(`\nRule: ${expression}`);
    }
    if (leftValue || rightValue) {
      markdown.appendText(`\nObserved: ${leftValue || "?"} ${operator || "?"} ${rightValue || "?"}`);
    }
  }
  return markdown;
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

function fullLineRange(document, lineNumber) {
  const line = document.lineAt(lineNumber);
  if (lineNumber + 1 < document.lineCount) {
    return new vscode.Range(lineNumber, 0, lineNumber + 1, 0);
  }
  return new vscode.Range(lineNumber, 0, lineNumber, line.text.length);
}

function lineEndRange(document, lineNumber) {
  const line = document.lineAt(lineNumber);
  return new vscode.Range(lineNumber, line.text.length, lineNumber, line.text.length);
}

module.exports = {
  createDecorationController,
  reviewValidationDecorationOptions,
  timeAlignmentDecorationOptions,
  timeAlignmentWarning
};
