"use strict";

const assert = require("assert");
const Module = require("module");

const originalLoad = Module._load;
let localMemberCompletionsForContext;
let workflowBindingFieldCompletionsFromSource;
try {
  Module._load = function loadWithVscodeMock(request, parent, isMain) {
    if (request === "vscode") {
      return {};
    }
    return originalLoad.call(this, request, parent, isMain);
  };
  ({
    localMemberCompletionsForContext,
    workflowBindingFieldCompletionsFromSource
  } = require("../completionProvider"));
} finally {
  Module._load = originalLoad;
}

const catalogs = {
  timeAlignmentResultFields: [
    { label: "materialization_status", detail: "output status" },
    { label: "output_count", detail: "output point count" }
  ]
};
const source = [
  "series_on_grid = align measured.T_zone with simulated.T_zone",
  "regular_series = resample measured.T_zone by 30 min"
].join("\n");
const fieldsByBinding = workflowBindingFieldCompletionsFromSource(source, catalogs);

for (const binding of ["series_on_grid", "regular_series"]) {
  assert.deepStrictEqual(
    fieldsByBinding[binding].map((field) => field.label),
    ["materialization_status", "output_count"],
    `${binding} should use TimeSeries alignment result fields`
  );
}

const line = "count = regular_series.";
const localItems = localMemberCompletionsForContext(
  { lineAt: () => ({ text: line }) },
  { line: 0, character: line.length },
  {
    ...catalogs,
    workflowBindingFields: fieldsByBinding
  }
);
assert.deepStrictEqual(
  localItems.map((item) => item.label),
  ["materialization_status", "output_count"]
);

const fallbackLine = "status = aligned_preview.";
const fallbackItems = localMemberCompletionsForContext(
  { lineAt: () => ({ text: fallbackLine }) },
  { line: 0, character: fallbackLine.length },
  catalogs
);
assert.deepStrictEqual(
  fallbackItems.map((item) => item.label),
  ["materialization_status", "output_count"]
);

console.log("VS Code TimeSeries alignment completion smoke passed.");
