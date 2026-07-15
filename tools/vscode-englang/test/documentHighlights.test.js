"use strict";

const assert = require("assert");
const Module = require("module");

class Range {
  constructor(startLine, startCharacter, endLine, endCharacter) {
    this.start = { line: startLine, character: startCharacter };
    this.end = { line: endLine, character: endCharacter };
  }
}

class DocumentHighlight {
  constructor(range, kind) {
    this.range = range;
    this.kind = kind;
  }
}

const vscodeMock = {
  DocumentHighlight,
  DocumentHighlightKind: {
    Text: 0,
    Read: 1,
    Write: 2
  },
  Range
};

const originalLoad = Module._load;
let documentHighlightsFromLsp;
let EngDocumentHighlightProvider;
try {
  Module._load = function loadWithVscodeMock(request, parent, isMain) {
    if (request === "vscode") {
      return vscodeMock;
    }
    return originalLoad.call(this, request, parent, isMain);
  };
  ({ documentHighlightsFromLsp } = require("../lspNavigation"));
  ({ EngDocumentHighlightProvider } = require("../navigationProviders"));
} finally {
  Module._load = originalLoad;
}

const payload = [
  {
    range: {
      start: { line: 1, character: 4 },
      end: { line: 1, character: 10 }
    },
    kind: 3
  },
  {
    range: {
      start: { line: 4, character: 12 },
      end: { line: 4, character: 18 }
    },
    kind: 2
  },
  { range: { start: { line: 8 } }, kind: 2 }
];

const converted = documentHighlightsFromLsp(payload);
assert.strictEqual(converted.length, 2);
assert.strictEqual(converted[0].kind, vscodeMock.DocumentHighlightKind.Write);
assert.strictEqual(converted[1].kind, vscodeMock.DocumentHighlightKind.Read);
assert.deepStrictEqual(converted[1].range.end, { line: 4, character: 18 });

async function providerUsesCurrentBufferRequest() {
  const calls = [];
  const document = { languageId: "englang", version: 7 };
  const position = { line: 4, character: 15 };
  const provider = new EngDocumentHighlightProvider({}, {
    isEngDocument: () => true,
    documentHighlightsForPosition(requestDocument, requestPosition) {
      calls.push({ requestDocument, requestPosition });
      return payload;
    }
  });
  const highlights = await provider.provideDocumentHighlights(document, position, {});
  assert.strictEqual(calls.length, 1);
  assert.strictEqual(calls[0].requestDocument, document);
  assert.strictEqual(calls[0].requestPosition, position);
  assert.strictEqual(highlights.length, 2);
}

providerUsesCurrentBufferRequest()
  .then(() => process.stdout.write("VS Code semantic document highlight smoke passed.\n"))
  .catch((error) => {
    process.stderr.write(String(error.stack || error.message) + "\n");
    process.exitCode = 1;
  });
