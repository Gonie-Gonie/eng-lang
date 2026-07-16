"use strict";

const assert = require("assert");
const fs = require("fs");
const Module = require("module");
const path = require("path");

class SemanticTokensLegend {
  constructor(tokenTypes, tokenModifiers) {
    this.tokenTypes = tokenTypes;
    this.tokenModifiers = tokenModifiers;
  }
}

const originalLoad = Module._load;
let MAX_SEMANTIC_TOKEN_MODIFIERS;
let createSemanticLegend;
let semanticModifierBits;
try {
  Module._load = function loadWithVscodeMock(request, parent, isMain) {
    if (request === "vscode") {
      return { SemanticTokensLegend };
    }
    return originalLoad.call(this, request, parent, isMain);
  };
  ({
    MAX_SEMANTIC_TOKEN_MODIFIERS,
    createSemanticLegend,
    semanticModifierBits
  } = require("../lspSemanticTokens"));
} finally {
  Module._load = originalLoad;
}

const metadataPath = path.join(
  __dirname,
  "..",
  "generated",
  "editor",
  "englang-semantic-legend.json"
);
const metadata = JSON.parse(fs.readFileSync(metadataPath, "utf8"));
const legend = metadata.semantic_token_legend;
const modifiers = legend.token_modifiers;

assert.strictEqual(MAX_SEMANTIC_TOKEN_MODIFIERS, 31);
assert.ok(modifiers.length <= MAX_SEMANTIC_TOKEN_MODIFIERS);
assert.ok(modifiers.includes("definition"));
assert.ok(!modifiers.includes("static"));
assert.strictEqual(modifiers.indexOf("declaration"), 0);
assert.strictEqual(modifiers.indexOf("temporal"), 30);

const temporalBit = 2 ** modifiers.indexOf("temporal");
const declarationBit = 2 ** modifiers.indexOf("declaration");
assert.strictEqual(
  semanticModifierBits(["declaration", "temporal"], modifiers),
  declarationBit + temporalBit
);
assert.notStrictEqual(temporalBit, declarationBit);
assert.throws(
  () => createSemanticLegend(legend.token_types, [...modifiers, "overflow"]),
  /at most 31 modifiers/
);

console.log("VS Code semantic modifier bitset smoke passed.");
