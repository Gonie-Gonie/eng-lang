"use strict";

const assert = require("assert");
const Module = require("module");

class Range {
  constructor(startLine, startCharacter, endLine, endCharacter) {
    this.start = { line: startLine, character: startCharacter };
    this.end = { line: endLine, character: endCharacter };
  }
}

const vscodeMock = {
  EndOfLine: {
    LF: 1,
    CRLF: 2
  },
  Range,
  TextEdit: {
    replace(range, newText) {
      return { newText, range };
    }
  }
};

const originalLoad = Module._load;
let EngFormattingProvider;
try {
  Module._load = function loadWithVscodeMock(request, parent, isMain) {
    if (request === "vscode") {
      return vscodeMock;
    }
    return originalLoad.call(this, request, parent, isMain);
  };
  ({ EngFormattingProvider } = require("../formattingProvider"));
} finally {
  Module._load = originalLoad;
}

function documentFixture(lines, options = {}) {
  return {
    eol: options.eol ?? vscodeMock.EndOfLine.LF,
    languageId: options.languageId ?? "englang",
    version: options.version ?? 1,
    get lineCount() {
      return lines.length;
    },
    getText() {
      const newline = this.eol === vscodeMock.EndOfLine.CRLF ? "\r\n" : "\n";
      return lines.join(newline);
    },
    lineAt(index) {
      return { lineNumber: index, text: lines[index] };
    }
  };
}

function deferred() {
  let resolve;
  const promise = new Promise((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

async function documentAndRangeFormattingUseCompilerOutput() {
  const document = documentFixture([
    "fn main() {",
    "value=1",
    "}"
  ]);
  const formatted = [
    "fn main() {",
    "    value = 1",
    "}"
  ].join("\n");
  const provider = new EngFormattingProvider({}, {
    isEngDocument: (candidate) => candidate.languageId === "englang",
    async formatDocumentSource() {
      return { changed: true, formatted };
    }
  });

  const documentEdits = await provider.provideDocumentFormattingEdits(document, {}, {});
  assert.strictEqual(documentEdits.length, 1);
  assert.deepStrictEqual(documentEdits[0].range.start, { line: 0, character: 0 });
  assert.deepStrictEqual(documentEdits[0].range.end, { line: 2, character: 1 });
  assert.strictEqual(documentEdits[0].newText, formatted);

  const rangeEdits = await provider.provideDocumentRangeFormattingEdits(
    document,
    new Range(1, 0, 2, 0),
    {},
    {}
  );
  assert.strictEqual(rangeEdits.length, 1);
  assert.deepStrictEqual(rangeEdits[0].range.start, { line: 1, character: 0 });
  assert.deepStrictEqual(rangeEdits[0].range.end, { line: 1, character: 7 });
  assert.strictEqual(rangeEdits[0].newText, "    value = 1");
}

async function closingBraceFormatsOnlyItsLine() {
  const document = documentFixture([
    "fn main() {",
    "    if ready {",
    "        }",
    "}"
  ]);
  const formatted = [
    "fn main() {",
    "    if ready {",
    "    }",
    "}"
  ].join("\n");
  let requestCount = 0;
  const provider = new EngFormattingProvider({}, {
    isEngDocument: (candidate) => candidate.languageId === "englang",
    async formatDocumentSource() {
      requestCount += 1;
      return { changed: true, formatted };
    }
  });

  const edits = await provider.provideOnTypeFormattingEdits(
    document,
    { line: 2, character: 9 },
    "}",
    {},
    {}
  );
  assert.strictEqual(requestCount, 1);
  assert.strictEqual(edits.length, 1);
  assert.deepStrictEqual(edits[0].range.start, { line: 2, character: 0 });
  assert.deepStrictEqual(edits[0].range.end, { line: 2, character: 9 });
  assert.strictEqual(edits[0].newText, "    }");

  const ignoredCases = [
    { ch: "{", document, position: { line: 1, character: 16 } },
    {
      ch: "}",
      document: documentFixture(['label = "{Q}"']),
      position: { line: 0, character: 12 }
    },
    {
      ch: "}",
      document: documentFixture(["# closing } is documentation"]),
      position: { line: 0, character: 11 }
    },
    {
      ch: "}",
      document: documentFixture(["value = 1 // closing }"]),
      position: { line: 0, character: 22 }
    }
  ];
  for (const ignoredCase of ignoredCases) {
    const ignored = await provider.provideOnTypeFormattingEdits(
      ignoredCase.document,
      ignoredCase.position,
      ignoredCase.ch,
      {},
      {}
    );
    assert.deepStrictEqual(ignored, []);
  }
  assert.strictEqual(
    requestCount,
    1,
    "non-structural, string, and comment braces must not invoke the formatter"
  );

  const cancelled = await provider.provideOnTypeFormattingEdits(
    document,
    { line: 2, character: 9 },
    "}",
    {},
    { isCancellationRequested: true }
  );
  assert.deepStrictEqual(cancelled, []);
  assert.strictEqual(requestCount, 1, "cancelled formatting must not invoke the formatter");
}

async function staleOrUnsafeOnTypeResultsAreDiscarded() {
  const pending = deferred();
  const document = documentFixture(["fn main() {", "    }", "}"]);
  const provider = new EngFormattingProvider({}, {
    isEngDocument: () => true,
    formatDocumentSource() {
      return pending.promise;
    }
  });

  const editsPromise = provider.provideOnTypeFormattingEdits(
    document,
    { line: 1, character: 5 },
    "}",
    {},
    {}
  );
  document.version += 1;
  pending.resolve({ changed: true, formatted: "fn main() {\n}\n}" });
  assert.deepStrictEqual(await editsPromise, []);

  const mismatchedDocument = documentFixture(["fn main() {", "    }"]);
  const mismatchedProvider = new EngFormattingProvider({}, {
    isEngDocument: () => true,
    async formatDocumentSource() {
      return { changed: true, formatted: "fn main() {\n\n}\n" };
    }
  });
  assert.deepStrictEqual(
    await mismatchedProvider.provideOnTypeFormattingEdits(
      mismatchedDocument,
      { line: 1, character: 5 },
      "}",
      {},
      {}
    ),
    [],
    "on-type formatting must not guess line mappings after a structural rewrite"
  );
}

async function main() {
  await documentAndRangeFormattingUseCompilerOutput();
  await closingBraceFormatsOnlyItsLine();
  await staleOrUnsafeOnTypeResultsAreDiscarded();
  process.stdout.write("VS Code compiler-backed formatting provider smoke passed.\n");
}

main().catch((error) => {
  process.stderr.write(String(error.stack || error.message) + "\n");
  process.exitCode = 1;
});
