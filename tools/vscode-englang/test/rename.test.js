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

const vscodeMock = {
  Range,
  Uri: {
    parse(value) {
      if (!value.startsWith("file:")) {
        throw new Error("expected a file URI");
      }
      return { value, toString: () => value };
    }
  },
  WorkspaceEdit,
  workspace: { workspaceFolders: [] }
};

const originalLoad = Module._load;
let createLspRequests;
let EngRenameProvider;
let prepareRenameFromLsp;
let workspaceEditFromLsp;
try {
  Module._load = function loadWithVscodeMock(request, parent, isMain) {
    if (request === "vscode") {
      return vscodeMock;
    }
    return originalLoad.call(this, request, parent, isMain);
  };
  ({ createLspRequests } = require("../lspRequests"));
  ({ EngRenameProvider } = require("../navigationProviders"));
  ({ prepareRenameFromLsp, workspaceEditFromLsp } = require("../lspNavigation"));
} finally {
  Module._load = originalLoad;
}

const preparePayload = {
  range: {
    start: { line: 2, character: 28 },
    end: { line: 2, character: 38 }
  },
  placeholder: "left_power"
};
const renamePayload = {
  changes: {
    "file:///C:/workspace/main.eng": [
      {
        range: {
          start: { line: 0, character: 0 },
          end: { line: 0, character: 10 }
        },
        newText: "input_power"
      },
      {
        range: {
          start: { line: 2, character: 28 },
          end: { line: 2, character: 38 }
        },
        newText: "input_power"
      }
    ]
  }
};

function deferred() {
  let resolve;
  const promise = new Promise((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

function documentFixture() {
  const source = "left_power = 5 kW\ntotal = left_power\n";
  return {
    languageId: "englang",
    version: 7,
    uri: {
      fsPath: "C:\\workspace\\main.eng",
      toString: () => "file:///C:/workspace/main.eng"
    },
    getText: () => source
  };
}

function convertersAreStrict() {
  const prepared = prepareRenameFromLsp(preparePayload);
  assert.strictEqual(prepared.placeholder, "left_power");
  assert.deepStrictEqual(prepared.range.end, { line: 2, character: 38 });

  const workspaceEdit = workspaceEditFromLsp(renamePayload);
  assert.strictEqual(workspaceEdit.replacements.length, 2);
  assert.strictEqual(workspaceEdit.replacements[1].newText, "input_power");

  const messages = [];
  const malformed = workspaceEditFromLsp(
    {
      changes: {
        "file:///C:/workspace/main.eng": [
          renamePayload.changes["file:///C:/workspace/main.eng"][0],
          { range: {}, newText: "partial" }
        ]
      }
    },
    (message) => messages.push(message)
  );
  assert.strictEqual(malformed, undefined, "malformed edits must fail as a whole");
  assert.strictEqual(messages.length, 1);
}

async function providerUsesCurrentBufferRequests() {
  const calls = [];
  const document = documentFixture();
  const position = { line: 1, character: 15 };
  const provider = new EngRenameProvider({}, {
    isEngDocument: () => true,
    prepareRenameForPosition(requestDocument, requestPosition) {
      calls.push({ kind: "prepare", requestDocument, requestPosition });
      return preparePayload;
    },
    renameForPosition(requestDocument, requestPosition, newName) {
      calls.push({ kind: "rename", requestDocument, requestPosition, newName });
      return renamePayload;
    }
  });

  const prepared = await provider.prepareRename(document, position, {});
  assert.strictEqual(prepared.placeholder, "left_power");
  const edit = await provider.provideRenameEdits(document, position, "input_power", {});
  assert.strictEqual(edit.replacements.length, 2);
  assert.deepStrictEqual(calls.map((call) => call.kind), ["prepare", "rename"]);
  assert.strictEqual(calls[1].newName, "input_power");
}

async function providerRejectsStaleAndBackendErrors() {
  const request = deferred();
  const document = documentFixture();
  const provider = new EngRenameProvider({}, {
    isEngDocument: () => true,
    renameForPosition: () => request.promise
  });
  const pending = provider.provideRenameEdits(document, { line: 1, character: 10 }, "new_name", {});
  document.version += 1;
  request.resolve(renamePayload);
  assert.strictEqual(await pending, undefined);

  const rejected = new EngRenameProvider({}, {
    isEngDocument: () => true,
    renameForPosition: () => ({ error: "`report` is reserved by EngLang." })
  });
  await assert.rejects(
    rejected.provideRenameEdits(document, { line: 1, character: 10 }, "report", {}),
    /reserved by EngLang/
  );
}

async function requestsUseRenameStdinCommands() {
  const originalExecFile = childProcess.execFile;
  const calls = [];
  childProcess.execFile = (_runtime, args, _options, callback) => {
    calls.push(args);
    const payload = args[0] === "--prepare-rename-stdin" ? preparePayload : renamePayload;
    setImmediate(() => callback(null, JSON.stringify(payload), ""));
    return { kill() {}, stdin: { end() {} } };
  };
  try {
    const requests = createLspRequests({
      findLspRuntime: () => "eng-lsp.exe",
      isEngDocument: () => true,
      workspaceRoot: () => "C:\\workspace"
    });
    const document = documentFixture();
    const position = { line: 1, character: 15 };
    assert.deepStrictEqual(
      await requests.prepareRenameForPosition(document, position, {}),
      preparePayload
    );
    assert.deepStrictEqual(
      await requests.renameForPosition(document, position, "input_power", {}),
      renamePayload
    );
    assert.deepStrictEqual(calls[0], [
      "--prepare-rename-stdin",
      document.uri.fsPath,
      "1",
      "15"
    ]);
    assert.deepStrictEqual(calls[1], [
      "--rename-stdin",
      document.uri.fsPath,
      "1",
      "15",
      "input_power"
    ]);
  } finally {
    childProcess.execFile = originalExecFile;
  }
}

async function main() {
  convertersAreStrict();
  await providerUsesCurrentBufferRequests();
  await providerRejectsStaleAndBackendErrors();
  await requestsUseRenameStdinCommands();
  process.stdout.write("VS Code semantic rename smoke passed.\n");
}

main().catch((error) => {
  process.stderr.write(String(error.stack || error.message) + "\n");
  process.exitCode = 1;
});
