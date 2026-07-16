"use strict";

const assert = require("assert");
const Module = require("module");

class MarkdownString {
  constructor(value = "") {
    this.value = value;
    this.isTrusted = false;
  }

  appendMarkdown(value) {
    this.value += value;
  }
}

class Range {
  constructor(startLine, startCharacter, endLine, endCharacter) {
    this.start = { line: startLine, character: startCharacter };
    this.end = { line: endLine, character: endCharacter };
  }
}

class Hover {
  constructor(contents, range) {
    this.contents = contents;
    this.range = range;
  }
}

const originalLoad = Module._load;
let findHoverForWord;
let hoverKindLabel;
let hoverFromSnapshot;
let hoverMarkdown;
try {
  Module._load = function loadWithVscodeMock(request, parent, isMain) {
    if (request === "vscode") {
      return { Hover, MarkdownString, Range };
    }
    return originalLoad.call(this, request, parent, isMain);
  };
  ({ findHoverForWord, hoverFromSnapshot, hoverKindLabel, hoverMarkdown } = require("../hoverProvider"));
} finally {
  Module._load = originalLoad;
}

for (const [kind, label] of [
  ["unit", "Unit"],
  ["quantity", "Quantity"],
  ["schema_field", "Schema field"],
  ["timeseries_axis", "TimeSeries axis"],
  ["timeseries", "TimeSeries"],
  ["side_effect", "Side effect"],
  ["external_boundary", "External boundary"],
  ["uncertainty", "Uncertainty"],
  ["validation", "Validation"],
  ["case_run_result_table_field", "Case run result field"]
]) {
  assert.strictEqual(hoverKindLabel(kind), label);
}

const sideEffect = hoverMarkdown(
  {
    name: "write",
    kind: "side_effect",
    detail: "Operation that can mutate files.",
    quantity_kind: "",
    display_unit: "-",
    status: "high_review_risk"
  },
  "write"
);
assert.ok(sideEffect.value.includes("Kind: Side effect"));
assert.ok(sideEffect.value.includes("Status: High review risk"));
assert.ok(!sideEffect.value.includes("Quantity:"));
assert.ok(!sideEffect.value.includes("Display unit:"));

const unit = hoverMarkdown(
  {
    name: "degC",
    kind: "unit",
    detail: "Compiler-recognized unit spelling.",
    quantity_kind: "",
    display_unit: "degC",
    status: null
  },
  "degC"
);
assert.ok(unit.value.includes("Kind: Unit"));
assert.ok(unit.value.includes("Display unit: `degC`"));
assert.ok(!unit.value.includes("Quantity:"));

const source = "irradiance: Irradiance [W/m2] = 300 W/m2";
const document = {
  lineAt() {
    return { text: source };
  },
  getText(range) {
    return source.slice(range.start.character, range.end.character);
  },
  getWordRangeAtPosition() {
    return undefined;
  }
};
const compositeStart = source.indexOf("W/m2");
const compositeHover = hoverFromSnapshot(
  document,
  { line: 0, character: compositeStart + 1 },
  {
    hovers: [{
      name: "W/m2",
      kind: "unit",
      line: 1,
      contents: { kind: "markdown", value: "**W/m2**\n\nKind: Unit" }
    }],
    semantic_tokens: {
      tokens: [{ line: 0, start: compositeStart, length: "W/m2".length, type: "type", modifiers: ["unit"] }]
    }
  }
);
assert.ok(compositeHover instanceof Hover);
assert.strictEqual(compositeHover.contents.value, "**W/m2**\n\nKind: Unit");
assert.strictEqual(compositeHover.range.start.character, compositeStart);
assert.strictEqual(compositeHover.range.end.character, compositeStart + "W/m2".length);

const structuredHover = findHoverForWord(
  {
    hovers: [
      { name: "Q_for_energy", kind: "timeseries", line: 29 },
      { name: "where.Q_for_energy", kind: "where_local", line: 31 }
    ]
  },
  ["Q_for_energy"],
  29
);
assert.strictEqual(structuredHover.kind, "where_local");

console.log("VS Code semantic role hover smoke passed.");
