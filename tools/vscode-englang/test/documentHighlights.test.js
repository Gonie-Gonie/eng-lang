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

class Location {
  constructor(uri, range) {
    this.uri = uri;
    this.range = range;
  }
}

const Uri = {
  parse(value) {
    return { value, fsPath: value.replace("file:///", "") };
  }
};

const vscodeMock = {
  DocumentHighlight,
  DocumentHighlightKind: {
    Text: 0,
    Read: 1,
    Write: 2
  },
  Location,
  Uri,
  Range,
  workspace: { textDocuments: [] }
};

const originalLoad = Module._load;
let documentHighlightsFromLsp;
let referenceLocationsFromLsp;
let EngDocumentHighlightProvider;
let EngReferenceProvider;
let createLspRequests;
try {
  Module._load = function loadWithVscodeMock(request, parent, isMain) {
    if (request === "vscode") {
      return vscodeMock;
    }
    return originalLoad.call(this, request, parent, isMain);
  };
  ({ documentHighlightsFromLsp, referenceLocationsFromLsp } = require("../lspNavigation"));
  ({ EngDocumentHighlightProvider, EngReferenceProvider } = require("../navigationProviders"));
  ({ createLspRequests } = require("../lspRequests"));
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

const referencePayload = payload.slice(0, 2).map((highlight) => ({
  uri: "file:///C:/workspace/main.eng",
  range: highlight.range
}));
const convertedReferences = referenceLocationsFromLsp(referencePayload);
assert.strictEqual(convertedReferences.length, 2);
assert.strictEqual(convertedReferences[0].uri.value, "file:///C:/workspace/main.eng");
assert.deepStrictEqual(convertedReferences[1].range.start, { line: 4, character: 12 });

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

async function referenceProviderUsesCurrentBufferAndDeclarationContext() {
  const calls = [];
  const document = {
    languageId: "englang",
    version: 9,
    uri: Uri.parse("file:///C:/workspace/main.eng")
  };
  const position = { line: 4, character: 15 };
  const provider = new EngReferenceProvider({}, {
    isEngDocument: () => true,
    referencesForPosition(requestDocument, requestPosition, includeDeclaration) {
      calls.push({ requestDocument, requestPosition, includeDeclaration });
      return referencePayload;
    }
  });
  const references = await provider.provideReferences(
    document,
    position,
    { includeDeclaration: false },
    {}
  );
  assert.strictEqual(calls.length, 1);
  assert.strictEqual(calls[0].requestDocument, document);
  assert.strictEqual(calls[0].requestPosition, position);
  assert.strictEqual(calls[0].includeDeclaration, false);
  assert.strictEqual(references.length, 2);
}

async function referenceProviderRejectsStaleBufferResults() {
  let resolveRequest;
  const request = new Promise((resolve) => {
    resolveRequest = resolve;
  });
  const document = {
    languageId: "englang",
    version: 11,
    uri: Uri.parse("file:///C:/workspace/stale.eng")
  };
  const provider = new EngReferenceProvider({}, {
    referencesForPosition: () => request
  });
  const pending = provider.provideReferences(
    document,
    { line: 1, character: 3 },
    { includeDeclaration: true },
    {}
  );
  document.version = 12;
  resolveRequest(referencePayload);
  assert.deepStrictEqual(await pending, []);
}

async function navigationRequestsUseAllDirtyWorkspaceBuffers() {
  const childProcess = require("child_process");
  const originalExecFile = childProcess.execFile;
  const invocations = [];
  const definitionPayload = {
    uri: "file:///C:/workspace/module.eng",
    range: {
      start: { line: 2, character: 6 },
      end: { line: 2, character: 17 }
    }
  };
  childProcess.execFile = function execFile(runtime, args, options, callback) {
    const invocation = { runtime, args, options, stdinText: "" };
    invocations.push(invocation);
    const response = args[0] === "--workspace-definition-stdin"
      ? definitionPayload
      : args[0] === "--workspace-snapshot-stdin"
        ? { format: "eng-lsp-snapshot-v1", diagnostics: [], completions: [], hovers: [] }
        : args[0] === "--workspace-completion-stdin"
          ? { format: "eng-lsp-snapshot-v1", completions: [{ label: "SHARED_GAIN" }] }
          : referencePayload;
    setImmediate(() => callback(null, JSON.stringify(response), ""));
    return {
      kill() {},
      stdin: {
        end(value) {
          invocation.stdinText = value;
        }
      }
    };
  };
  try {
    const requests = createLspRequests({
      findLspRuntime: () => "C:/tools/eng-lsp.exe",
      workspaceRoot: () => "C:/workspace",
      isEngDocument: () => true
    });
    const document = {
      isDirty: true,
      languageId: "englang",
      version: 13,
      uri: {
        fsPath: "C:/workspace/main.eng",
        scheme: "file",
        toString: () => "file:///C:/workspace/main.eng"
      },
      getText: () => "Q = 5 kW\nE = Q\n"
    };
    const otherDocument = {
      isDirty: true,
      languageId: "englang",
      version: 4,
      uri: {
        fsPath: "C:/workspace/other.eng",
        scheme: "file",
        toString: () => "file:///C:/workspace/other.eng"
      },
      getText: () => "other = Q\n"
    };
    vscodeMock.workspace.textDocuments = [document, otherDocument];
    const references = await requests.referencesForPosition(
      document,
      { line: 1, character: 4 },
      false,
      {},
      undefined
    );
    assert.deepStrictEqual(references, referencePayload);
    assert.deepStrictEqual(invocations[0].args, [
      "--workspace-references-stdin",
      "C:/workspace",
      "C:/workspace/main.eng",
      "1",
      "4",
      "false"
    ]);
    const expectedDocuments = {
      format: "eng-lsp-open-documents-v1",
      documents: [
        { path: "C:/workspace/main.eng", source: document.getText() },
        { path: "C:/workspace/other.eng", source: otherDocument.getText() }
      ]
    };
    assert.deepStrictEqual(JSON.parse(invocations[0].stdinText), expectedDocuments);

    const definition = await requests.definitionSnapshotForPosition(
      document,
      { line: 1, character: 4 },
      {},
      undefined
    );
    assert.deepStrictEqual(definition, definitionPayload);
    assert.deepStrictEqual(invocations[1].args, [
      "--workspace-definition-stdin",
      "C:/workspace",
      "C:/workspace/main.eng",
      "1",
      "4"
    ]);
    assert.deepStrictEqual(JSON.parse(invocations[1].stdinText), expectedDocuments);

    const snapshot = await requests.snapshotDocumentSource(document, {}, undefined);
    assert.strictEqual(snapshot.format, "eng-lsp-snapshot-v1");
    assert.deepStrictEqual(invocations[2].args, [
      "--workspace-snapshot-stdin",
      "C:/workspace",
      "C:/workspace/main.eng"
    ]);
    assert.deepStrictEqual(JSON.parse(invocations[2].stdinText), expectedDocuments);

    const completion = await requests.completionSnapshotForPosition(
      document,
      { line: 1, character: 4 },
      {},
      undefined
    );
    assert.strictEqual(completion.completions[0].label, "SHARED_GAIN");
    assert.deepStrictEqual(invocations[3].args, [
      "--workspace-completion-stdin",
      "C:/workspace",
      "C:/workspace/main.eng",
      "1",
      "4"
    ]);
    assert.deepStrictEqual(JSON.parse(invocations[3].stdinText), expectedDocuments);
  } finally {
    vscodeMock.workspace.textDocuments = [];
    childProcess.execFile = originalExecFile;
  }
}

Promise.all([
  providerUsesCurrentBufferRequest(),
  referenceProviderUsesCurrentBufferAndDeclarationContext(),
  referenceProviderRejectsStaleBufferResults(),
  navigationRequestsUseAllDirtyWorkspaceBuffers()
])
  .then(() => process.stdout.write("VS Code semantic document highlight and reference smoke passed.\n"))
  .catch((error) => {
    process.stderr.write(String(error.stack || error.message) + "\n");
    process.exitCode = 1;
  });
