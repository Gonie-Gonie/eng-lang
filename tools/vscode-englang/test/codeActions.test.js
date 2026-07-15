"use strict";

const assert = require("assert");
const childProcess = require("child_process");
const Module = require("module");

class Range {
  constructor(startLine, startCharacter, endLine, endCharacter) {
    this.start = { line: startLine, character: startCharacter };
    this.end = { line: endLine, character: endCharacter };
  }
}

class WorkspaceEdit {
  constructor() {
    this.replacements = [];
  }

  replace(uri, range, newText) {
    this.replacements.push({ uri, range, newText });
  }
}

class CodeAction {
  constructor(title, kind) {
    this.title = title;
    this.kind = kind;
  }
}

const vscodeMock = {
  CodeAction,
  CodeActionKind: {
    Empty: { value: "" },
    QuickFix: { value: "quickfix" }
  },
  Range,
  WorkspaceEdit,
  workspace: { workspaceFolders: [] }
};

const originalLoad = Module._load;
let createLspRequests;
let EngCodeActionProvider;
let lspCodeActionsFromPayload;
try {
  Module._load = function loadWithVscodeMock(request, parent, isMain) {
    if (request === "vscode") {
      return vscodeMock;
    }
    return originalLoad.call(this, request, parent, isMain);
  };
  ({ createLspRequests } = require("../lspRequests"));
  ({ EngCodeActionProvider } = require("../codeActionProvider"));
  ({ lspCodeActionsFromPayload } = require("../lspCodeActions"));
} finally {
  Module._load = originalLoad;
}

function deferred() {
  let resolve;
  const promise = new Promise((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

function documentFixture() {
  const lines = ["😀 := 1", "next = 2"];
  return {
    fileName: "C:\\workspace\\main.eng",
    isDirty: true,
    languageId: "englang",
    lineCount: lines.length,
    uri: {
      fsPath: "C:\\workspace\\main.eng",
      toString: () => "file:///C:/workspace/main.eng"
    },
    version: 4,
    getText: () => lines.join("\n"),
    lineAt(line) {
      if (!Number.isInteger(line) || line < 0 || line >= lines.length) {
        throw new RangeError("line outside document");
      }
      return { lineNumber: line, text: lines[line] };
    }
  };
}

function diagnosticFixture(code = "E-SYNTAX-DECL-001") {
  return {
    code,
    message: "Use = for declarations.",
    range: new Range(0, 3, 0, 5)
  };
}

function actionFixture(diagnostic = diagnosticFixture()) {
  return {
    title: "Replace := with =",
    kind: "quickfix",
    isPreferred: true,
    diagnostics: [
      {
        code: diagnostic.code,
        message: diagnostic.message,
        range: {
          start: { line: 0, character: 3 },
          end: { line: 0, character: 5 }
        }
      }
    ],
    edit: {
      changes: {
        "file:///C:/workspace/main.eng": [
          {
            range: {
              start: { line: 0, character: 3 },
              end: { line: 0, character: 5 }
            },
            newText: "="
          },
          {
            range: {
              start: { line: 1, character: 8 },
              end: { line: 1, character: 8 }
            },
            newText: " # checked"
          }
        ]
      }
    }
  };
}

function bridgeAcceptsOnlyCompleteCurrentDocumentActions() {
  const document = documentFixture();
  const diagnostic = diagnosticFixture();
  const action = actionFixture(diagnostic);
  const converted = lspCodeActionsFromPayload(document, { actions: [action] }, [diagnostic]);

  assert.strictEqual(converted.length, 1);
  assert.strictEqual(converted[0].title, action.title);
  assert.strictEqual(converted[0].kind, vscodeMock.CodeActionKind.QuickFix);
  assert.strictEqual(converted[0].isPreferred, true);
  assert.deepStrictEqual(converted[0].diagnostics, [diagnostic]);
  assert.strictEqual(converted[0].edit.replacements.length, 2);
  assert.deepStrictEqual(converted[0].edit.replacements[0].range.start, {
    line: 0,
    character: 3
  });
  assert.deepStrictEqual(
    lspCodeActionsFromPayload(document, { actions: [action] }, []),
    [],
    "quick fixes must stay bound to an exact editor diagnostic"
  );

  const wrongUri = actionFixture(diagnostic);
  wrongUri.edit.changes = {
    "file:///C:/workspace/other.eng": wrongUri.edit.changes[document.uri.toString()]
  };
  assert.deepStrictEqual(lspCodeActionsFromPayload(document, [wrongUri], [diagnostic]), []);

  const multiFile = actionFixture(diagnostic);
  multiFile.edit.changes["file:///C:/workspace/other.eng"] = [];
  assert.deepStrictEqual(lspCodeActionsFromPayload(document, [multiFile], [diagnostic]), []);

  const partial = actionFixture(diagnostic);
  partial.edit.changes[document.uri.toString()].push({ range: {}, newText: "partial" });
  assert.deepStrictEqual(lspCodeActionsFromPayload(document, [partial], [diagnostic]), []);

  const reversed = actionFixture(diagnostic);
  reversed.edit.changes[document.uri.toString()][0].range = {
    start: { line: 0, character: 5 },
    end: { line: 0, character: 3 }
  };
  assert.deepStrictEqual(lspCodeActionsFromPayload(document, [reversed], [diagnostic]), []);

  const negative = actionFixture(diagnostic);
  negative.edit.changes[document.uri.toString()][0].range.start.character = -1;
  assert.deepStrictEqual(lspCodeActionsFromPayload(document, [negative], [diagnostic]), []);

  const overlapping = actionFixture(diagnostic);
  overlapping.edit.changes[document.uri.toString()][1].range = {
    start: { line: 0, character: 4 },
    end: { line: 0, character: 6 }
  };
  assert.deepStrictEqual(lspCodeActionsFromPayload(document, [overlapping], [diagnostic]), []);

  const unrelated = actionFixture(diagnosticFixture("E-OTHER"));
  assert.deepStrictEqual(lspCodeActionsFromPayload(document, [unrelated], [diagnostic]), []);

  const wrongKind = actionFixture(diagnostic);
  wrongKind.kind = "refactor.rewrite";
  assert.deepStrictEqual(lspCodeActionsFromPayload(document, [wrongKind], [diagnostic]), []);
}

async function providerUsesOnlyCompilerActions() {
  const document = documentFixture();
  const diagnostic = diagnosticFixture();
  const calls = [];
  const provider = new EngCodeActionProvider({ marker: "context" }, {
    codeActionsForDocumentSource(requestDocument, requestContext, cancellationToken) {
      calls.push({ requestDocument, requestContext, cancellationToken });
      return { actions: [actionFixture(diagnostic)] };
    }
  });
  const token = { isCancellationRequested: false };
  const actions = await provider.provideCodeActions(
    document,
    diagnostic.range,
    { diagnostics: [diagnostic] },
    token
  );
  assert.strictEqual(actions.length, 1);
  assert.strictEqual(calls.length, 1);
  assert.strictEqual(calls[0].requestDocument, document);
  assert.strictEqual(calls[0].cancellationToken, token);

  const compilerHasNoAction = new EngCodeActionProvider({}, {
    codeActionsForDocumentSource: () => ({ actions: [] })
  });
  assert.deepStrictEqual(
    await compilerHasNoAction.provideCodeActions(
      document,
      diagnostic.range,
      { diagnostics: [diagnostic] },
      token
    ),
    [],
    "the extension must not synthesize a local fix when the compiler returns none"
  );

  let filteredRequests = 0;
  const filtered = new EngCodeActionProvider({}, {
    codeActionsForDocumentSource: () => {
      filteredRequests += 1;
      return { actions: [actionFixture(diagnostic)] };
    }
  });
  assert.deepStrictEqual(
    await filtered.provideCodeActions(
      document,
      diagnostic.range,
      { diagnostics: [diagnostic], only: { value: "refactor" } },
      token
    ),
    []
  );
  assert.strictEqual(filteredRequests, 0);
}

async function providerRejectsCancelledStaleAndFailedRequests() {
  const document = documentFixture();
  const diagnostic = diagnosticFixture();
  let requestCount = 0;
  const cancelled = new EngCodeActionProvider({}, {
    codeActionsForDocumentSource: () => {
      requestCount += 1;
      return { actions: [actionFixture(diagnostic)] };
    }
  });
  assert.deepStrictEqual(
    await cancelled.provideCodeActions(
      document,
      diagnostic.range,
      { diagnostics: [diagnostic] },
      { isCancellationRequested: true }
    ),
    []
  );
  assert.strictEqual(requestCount, 0);

  const pending = deferred();
  const stale = new EngCodeActionProvider({}, {
    codeActionsForDocumentSource: () => pending.promise
  });
  const staleResult = stale.provideCodeActions(
    document,
    diagnostic.range,
    { diagnostics: [diagnostic] },
    { isCancellationRequested: false }
  );
  document.version += 1;
  pending.resolve({ actions: [actionFixture(diagnostic)] });
  assert.deepStrictEqual(await staleResult, []);

  const messages = [];
  const failed = new EngCodeActionProvider({}, {
    appendOutputLine: (message) => messages.push(message),
    codeActionsForDocumentSource: () => {
      throw new Error("tool unavailable");
    }
  });
  assert.deepStrictEqual(
    await failed.provideCodeActions(
      document,
      diagnostic.range,
      { diagnostics: [diagnostic] },
      { isCancellationRequested: false }
    ),
    []
  );
  assert.deepStrictEqual(messages, ["Code action lookup failed: tool unavailable"]);
}

async function lspRequestUsesCurrentUnsavedBuffer() {
  const originalExecFile = childProcess.execFile;
  const calls = [];
  childProcess.execFile = (_runtime, args, options, callback) => {
    const call = { args, options, stdin: undefined };
    calls.push(call);
    return {
      kill() {},
      stdin: {
        end(value) {
          call.stdin = value;
          setImmediate(() => callback(null, JSON.stringify({ actions: [] }), ""));
        }
      }
    };
  };
  try {
    const document = documentFixture();
    const requests = createLspRequests({
      findLspRuntime: () => "eng-lsp.exe",
      workspaceRoot: () => "C:\\workspace"
    });
    assert.deepStrictEqual(
      await requests.codeActionsForDocumentSource(document, {}),
      { actions: [] }
    );
    assert.deepStrictEqual(calls[0].args, [
      "--code-actions-stdin",
      document.uri.fsPath
    ]);
    assert.strictEqual(calls[0].stdin, document.getText());
  } finally {
    childProcess.execFile = originalExecFile;
  }
}

async function main() {
  bridgeAcceptsOnlyCompleteCurrentDocumentActions();
  await providerUsesOnlyCompilerActions();
  await providerRejectsCancelledStaleAndFailedRequests();
  await lspRequestUsesCurrentUnsavedBuffer();
  process.stdout.write("VS Code compiler code action smoke passed.\n");
}

main().catch((error) => {
  process.stderr.write(String(error.stack || error.message) + "\n");
  process.exitCode = 1;
});
