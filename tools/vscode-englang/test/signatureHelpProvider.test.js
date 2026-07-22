"use strict";

const assert = require("assert");
const Module = require("module");

class MarkdownString {
  constructor(value = "") {
    this.value = value;
    this.isTrusted = false;
  }
}

class SignatureHelp {
  constructor() {
    this.signatures = [];
    this.activeSignature = 0;
    this.activeParameter = 0;
  }
}

class SignatureInformation {
  constructor(label, documentation) {
    this.label = label;
    this.documentation = documentation;
    this.parameters = [];
  }
}

class ParameterInformation {
  constructor(label, documentation) {
    this.label = label;
    this.documentation = documentation;
  }
}

const originalLoad = Module._load;
let EngSignatureHelpProvider;
let parameterLabelFromLsp;
let signatureHelpFromLsp;
try {
  Module._load = function loadWithVscodeMock(request, parent, isMain) {
    if (request === "vscode") {
      return {
        MarkdownString,
        ParameterInformation,
        SignatureHelp,
        SignatureInformation
      };
    }
    return originalLoad.call(this, request, parent, isMain);
  };
  ({
    EngSignatureHelpProvider,
    parameterLabelFromLsp,
    signatureHelpFromLsp
  } = require("../signatureHelpProvider"));
} finally {
  Module._load = originalLoad;
}

const payload = {
  signatures: [{
    label: "combine(left: Length [m], right: Length [m]) -> Length [m]",
    documentation: { kind: "markdown", value: "Returns `Length [m]`." },
    parameters: [
      { label: "left: Length [m]", documentation: "First length." },
      {
        label: [34, 51],
        documentation: { kind: "markdown", value: "Second length." }
      }
    ]
  }],
  activeSignature: 4,
  activeParameter: 1
};

const converted = signatureHelpFromLsp(payload);
assert.ok(converted instanceof SignatureHelp);
assert.strictEqual(converted.activeSignature, 0);
assert.strictEqual(converted.activeParameter, 1);
assert.strictEqual(converted.signatures[0].documentation.value, "Returns `Length [m]`.");
assert.strictEqual(converted.signatures[0].documentation.isTrusted, false);
assert.deepStrictEqual(converted.signatures[0].parameters[1].label, [34, 51]);
assert.strictEqual(converted.signatures[0].parameters[1].documentation.value, "Second length.");
assert.strictEqual(signatureHelpFromLsp({ signatures: [] }), undefined);
assert.strictEqual(parameterLabelFromLsp([-1, 2]), undefined);

const overloadedBuiltin = signatureHelpFromLsp({
  signatures: [
    {
      label: "measured(value: Quantity, relative_error: Ratio) -> Uncertain[Quantity]",
      parameters: [
        { label: "value: Quantity" },
        { label: "relative_error: Ratio" }
      ]
    },
    {
      label: "measured(value: Quantity, std: Quantity) -> Uncertain[Quantity]",
      parameters: [
        { label: "value: Quantity" },
        { label: "std: Quantity" }
      ]
    }
  ],
  activeSignature: 1,
  activeParameter: 1
});
assert.strictEqual(overloadedBuiltin.signatures.length, 2);
assert.strictEqual(overloadedBuiltin.activeSignature, 1);
assert.strictEqual(overloadedBuiltin.activeParameter, 1);
assert.strictEqual(
  overloadedBuiltin.signatures[1].parameters[1].label,
  "std: Quantity"
);

async function main() {
  const document = { languageId: "englang", version: 1 };
  const provider = new EngSignatureHelpProvider({
    isEngDocument: (candidate) => candidate.languageId === "englang",
    signatureHelpForPosition: async () => payload
  });
  const help = await provider.provideSignatureHelp(
    document,
    { line: 3, character: 24 },
    { isCancellationRequested: false }
  );
  assert.ok(help instanceof SignatureHelp);
  assert.strictEqual(help.activeParameter, 1);

  const staleProvider = new EngSignatureHelpProvider({
    signatureHelpForPosition: async () => {
      document.version += 1;
      return payload;
    }
  });
  assert.strictEqual(
    await staleProvider.provideSignatureHelp(
      document,
      { line: 3, character: 24 },
      { isCancellationRequested: false }
    ),
    undefined
  );
  console.log("VS Code signature help provider smoke passed.");
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
