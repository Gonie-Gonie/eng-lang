"use strict";

const assert = require("assert");
const Module = require("module");
const path = require("path");

class Range {
  constructor(startLine, startCharacter, endLine, endCharacter) {
    this.start = { line: startLine, character: startCharacter };
    this.end = { line: endLine, character: endCharacter };
  }
}

class MarkdownString {
  constructor() {
    this.value = "";
    this.isTrusted = false;
  }

  appendMarkdown(value) {
    this.value += value;
    return this;
  }

  appendText(value) {
    this.value += value;
    return this;
  }
}

class ThemeColor {
  constructor(id) {
    this.id = id;
  }
}

const createdDecorationTypes = [];
const vscodeMock = {
  MarkdownString,
  OverviewRulerLane: { Right: 4 },
  Range,
  ThemeColor,
  window: {
    visibleTextEditors: [],
    createTextEditorDecorationType(options) {
      const type = { options, dispose() {} };
      createdDecorationTypes.push(type);
      return type;
    }
  },
  workspace: {
    getConfiguration() {
      return { get(_key, fallback) { return fallback; } };
    }
  }
};

const originalLoad = Module._load;
let createDecorationController;
let reviewValidationDecorationOptions;
let timeAlignmentDecorationOptions;
let fnv1a64;
let timeAlignmentReportMatchesDocument;
let renderReviewSummaryHtml;
try {
  Module._load = function loadWithVscodeMock(request, parent, isMain) {
    if (request === "vscode") {
      return vscodeMock;
    }
    return originalLoad.call(this, request, parent, isMain);
  };
  ({
    createDecorationController,
    reviewValidationDecorationOptions,
    timeAlignmentDecorationOptions
  } = require("../decorations"));
  ({ fnv1a64, timeAlignmentReportMatchesDocument } = require("../commandHandlers"));
  ({ renderReviewSummaryHtml } = require("../reviewPanelRenderer"));
} finally {
  Module._load = originalLoad;
}

function testDocument() {
  const lines = [
    "class Construction {",
    "good = Construction {}",
    "middle = 1",
    "bad = Construction {}",
    "other = 2",
    "validate value > 0",
    "aligned = align measured.T with simulated.T",
    "resampled = resample measured.T to simulated.T",
    "missing = align absent.T with simulated.T",
    "done = 3"
  ];
  return {
    lineCount: lines.length,
    lineAt(index) {
      return { text: lines[index] };
    }
  };
}

function objectValidation(target, line, status, expression, leftValue, rightValue) {
  return {
    kind: "class_object_validation",
    target,
    class_name: "Construction",
    expression,
    operator: ">",
    left_value: leftValue,
    right_value: rightValue,
    status,
    source_span: { line, column: 1 },
    rule_source_span: { line: 1, column: 1 }
  };
}

const document = testDocument();
const review = {
  review_document: {
    validations: [
      objectValidation("good", 2, "pass", "u_value > 0 W/K", "10 W/K", "0 W/K"),
      objectValidation("good", 2, "pass", "thickness > 0 m", "0.2 m", "0 m"),
      objectValidation("bad", 4, "pass", "thickness > 0 m", "0.2 m", "0 m"),
      objectValidation("bad", 4, "fail", "u_value > 0 W/K", "0 W/K", "0 W/K"),
      { kind: "class_validation", status: "declared", line: 1 },
      { kind: "command_validation", status: "pending_runtime", line: 6 },
      objectValidation("outside", 99, "fail", "x > 0", "0", "0")
    ]
  }
};

const options = reviewValidationDecorationOptions(document, review);
assert.strictEqual(options.pass.length, 1);
assert.strictEqual(options.fail.length, 1);
assert.deepStrictEqual(options.pass[0].range.start, {
  line: 1,
  character: document.lineAt(1).text.length
});
assert.deepStrictEqual(options.fail[0].range.start, {
  line: 3,
  character: document.lineAt(3).text.length
});
assert.match(options.pass[0].hoverMessage.value, /validation passed/);
assert.match(options.pass[0].hoverMessage.value, /good \(Construction\)/);
assert.match(options.pass[0].hoverMessage.value, /good \(Construction\) - passed/);
assert.match(options.pass[0].hoverMessage.value, /thickness > 0 m/);
assert.match(options.fail[0].hoverMessage.value, /validation failed/);
assert.match(options.fail[0].hoverMessage.value, /bad \(Construction\) - failed/);
assert.match(options.fail[0].hoverMessage.value, /Observed: 0 W\/K > 0 W\/K/);

const snapshotOptions = reviewValidationDecorationOptions(document, {
  validations: [objectValidation("live", 2, "pass", "u_value > 0 W/K", "5 W/K", "0 W/K")]
});
assert.strictEqual(snapshotOptions.pass.length, 1);
assert.strictEqual(snapshotOptions.fail.length, 0);

const alignmentOptions = timeAlignmentDecorationOptions(document, {
  time_alignments: [
    {
      binding: "aligned",
      left: "measured.T",
      right: "simulated.T",
      strategy: "align",
      method: "exact",
      materialization_status: "partial",
      materialization_reason: "exact matching omitted target timestamps without source samples",
      output_count: 2,
      target_count: 4,
      status: "overlap",
      step_status: "mismatch",
      line: 7
    },
    {
      binding: "resampled",
      left: "measured.T",
      right: "simulated.T",
      strategy: "resample",
      method: "linear",
      materialization_status: "materialized",
      output_count: 4,
      target_count: 4,
      status: "overlap",
      step_status: "mismatch",
      line: 8
    },
    {
      binding: "missing",
      left: "absent.T",
      right: "simulated.T",
      strategy: "align",
      method: "exact",
      materialization_status: "unavailable",
      materialization_reason: "source TimeSeries `absent.T` is unavailable",
      output_count: 0,
      target_count: 0,
      status: "mismatch",
      line: 9
    },
    {
      strategy: "auto_pairwise",
      materialization_status: "not_requested",
      status: "mismatch",
      line: 7
    }
  ]
});
assert.strictEqual(alignmentOptions.length, 2);
assert.strictEqual(alignmentOptions[0].renderOptions.after.contentText, "  alignment partial 2/4");
assert.strictEqual(alignmentOptions[1].renderOptions.after.contentText, "  alignment unavailable");
assert.deepStrictEqual(alignmentOptions[0].range.start, {
  line: 6,
  character: document.lineAt(6).text.length
});
assert.match(alignmentOptions[0].hoverMessage.value, /Latest saved run: aligned/);
assert.match(alignmentOptions[0].hoverMessage.value, /Materialization: partial \(2\/4 points\)/);
assert.match(alignmentOptions[1].hoverMessage.value, /absent\.T.*simulated\.T/);
assert.ok(!alignmentOptions.some((option) => option.hoverMessage.value.includes("resampled")));

const workspaceRoot = path.resolve("workspace");
const sourcePath = path.join(workspaceRoot, "main.eng");
const sourceText = "aligned = align measured.T with simulated.T\n";
assert.strictEqual(fnv1a64("hello"), "a430d84680aabd0b");
const matchingDocument = {
  uri: { fsPath: sourcePath },
  getText: () => sourceText
};
const matchingReport = {
  source_path: sourcePath,
  source_hash: fnv1a64(sourceText),
  time_alignments: []
};
assert.strictEqual(
  timeAlignmentReportMatchesDocument(matchingReport, matchingDocument, workspaceRoot),
  true
);
assert.strictEqual(
  timeAlignmentReportMatchesDocument(
    matchingReport,
    { ...matchingDocument, getText: () => `${sourceText}changed = 1\n` },
    workspaceRoot
  ),
  false
);
assert.strictEqual(
  timeAlignmentReportMatchesDocument(
    { ...matchingReport, source_path: path.join(workspaceRoot, "other.eng") },
    matchingDocument,
    workspaceRoot
  ),
  false
);

createDecorationController({ isEngDocument: () => true });
const validationPassType = createdDecorationTypes[2].options;
const validationFailType = createdDecorationTypes[3].options;
assert.strictEqual(validationPassType.after.contentText, "  validation passed");
assert.strictEqual(validationPassType.after.color.id, "testing.iconPassed");
assert.strictEqual(validationFailType.after.contentText, "  validation failed");
assert.strictEqual(validationFailType.after.color.id, "testing.iconFailed");
const timeAlignmentType = createdDecorationTypes[6].options;
assert.strictEqual(timeAlignmentType.after.color.id, "editorWarning.foreground");
assert.strictEqual(timeAlignmentType.overviewRulerLane, 4);

const reviewHtml = renderReviewSummaryHtml(review, "C:/workspace/main.eng", "nonce", []);
assert.match(reviewHtml, /<th>Target<\/th><th>Expression<\/th><th>Kind<\/th><th>Phase<\/th>/);
assert.match(reviewHtml, /<strong>good<\/strong>/);
assert.match(reviewHtml, /u_value &gt; 0 W\/K/);
assert.match(reviewHtml, /pill good">pass<\/span>/);

process.stdout.write("VS Code validation and TimeSeries alignment decoration smoke passed.\n");
