"use strict";

const assert = require("assert");
const Module = require("module");
const completionCatalog =
  require("../generated/editor/englang-completions.json").completion_items;

const originalLoad = Module._load;
let samplingMethodCompletionPrefix;
let samplingMethodCompletionsForContext;
try {
  Module._load = function loadWithVscodeMock(request, parent, isMain) {
    if (request === "vscode") {
      return {};
    }
    return originalLoad.call(this, request, parent, isMain);
  };
  ({
    samplingMethodCompletionPrefix,
    samplingMethodCompletionsForContext
  } = require("../completionProvider"));
} finally {
  Module._load = originalLoad;
}

function context(line) {
  return [
    {
      lineAt() {
        return { text: line };
      }
    },
    { line: 0, character: line.length }
  ];
}

const canonical = samplingMethodCompletionsForContext(
  ...context("designs = sample "),
  completionCatalog
);
assert.deepStrictEqual(
  canonical.map((completion) => completion.label),
  ["grid", "random", "lhs"]
);

const prefixed = samplingMethodCompletionsForContext(
  ...context("designs = sample r"),
  completionCatalog
);
assert.deepStrictEqual(prefixed.map((completion) => completion.label), ["random"]);
assert.deepStrictEqual(
  samplingMethodCompletionsForContext(
    ...context("designs = sample latin"),
    completionCatalog
  ),
  []
);

for (const line of [
  "load = uniform(",
  'note = "sample "',
  "# sample ",
  "value = 1 // sample "
]) {
  assert.strictEqual(
    samplingMethodCompletionPrefix(...context(line)),
    undefined,
    `{line} must not enter sampling method completion`
  );
}

console.log("sampling completion tests passed");
