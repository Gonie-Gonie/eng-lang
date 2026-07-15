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
  caseOutputTableFields: [{ label: "rendered_count", detail: "rendered outputs" }],
  caseRunResultTableFields: [
    { label: "succeeded_count", detail: "succeeded native case runs" }
  ]
};
const source = [
  "case_inputs = apply case_input_template over cases",
  "case_runs = apply run_case over case_inputs",
  "function_case_runs = apply(run_case, over=case_inputs)"
].join("\n");
const fieldsByBinding = workflowBindingFieldCompletionsFromSource(source, catalogs);

assert.deepStrictEqual(
  fieldsByBinding.case_inputs.map((field) => field.label),
  ["rendered_count"]
);
for (const binding of ["case_runs", "function_case_runs"]) {
  assert.deepStrictEqual(
    fieldsByBinding[binding].map((field) => field.label),
    ["succeeded_count"],
    `${binding} should use native case run result fields`
  );
}

const line = "status = case_runs.";
const localItems = localMemberCompletionsForContext(
  { lineAt: () => ({ text: line }) },
  { line: 0, character: line.length },
  catalogs
);
assert.deepStrictEqual(
  localItems.map((item) => item.label),
  ["succeeded_count"],
  "case_runs name fallback should not offer CaseOutput fields"
);

console.log("VS Code native case run completion smoke passed.");
