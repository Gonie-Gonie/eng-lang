"use strict";

const assert = require("assert");
const childProcess = require("child_process");
const Module = require("module");

function uri(fsPath, owner = "workspace") {
  return {
    fsPath,
    owner,
    scheme: "file",
    toString() {
      return `file:///${fsPath.replace(/\\/g, "/")}`;
    }
  };
}

function documentFixture(fsPath, source, options = {}) {
  return {
    isDirty: options.isDirty ?? true,
    languageId: options.languageId ?? "englang",
    uri: uri(fsPath, options.owner ?? "workspace"),
    version: options.version ?? 1,
    getText() {
      return source;
    }
  };
}

const workspaceFolder = {
  uri: uri("C:\\workspace", "workspace")
};
const otherFolder = {
  uri: uri("C:\\other", "other")
};
const vscodeMock = {
  workspace: {
    textDocuments: [],
    workspaceFolders: [workspaceFolder],
    getWorkspaceFolder(documentUri) {
      if (documentUri.owner === "workspace") return workspaceFolder;
      if (documentUri.owner === "other") return otherFolder;
      return undefined;
    }
  }
};

const originalLoad = Module._load;
let createLspRequests;
try {
  Module._load = function loadWithVscodeMock(request, parent, isMain) {
    if (request === "vscode") return vscodeMock;
    return originalLoad.call(this, request, parent, isMain);
  };
  ({ createLspRequests } = require("../lspRequests"));
} finally {
  Module._load = originalLoad;
}

function requestsFixture(logs = []) {
  return createLspRequests({
    appendOutputLine(message) {
      logs.push(message);
    },
    findLspRuntimeForRoot() {
      return "eng-lsp.exe";
    },
    isEngDocument(document) {
      return document.languageId === "englang";
    }
  });
}

function installExecFixture() {
  const calls = [];
  const originalExecFile = childProcess.execFile;
  childProcess.execFile = (runtime, args, options, callback) => {
    const call = {
      args,
      callback,
      killed: false,
      options,
      runtime,
      stdin: ""
    };
    calls.push(call);
    return {
      kill() {
        call.killed = true;
      },
      stdin: {
        end(value) {
          call.stdin = value;
        }
      }
    };
  };
  return {
    calls,
    restore() {
      childProcess.execFile = originalExecFile;
    }
  };
}

function symbolPayload(name = "UnsavedThing") {
  return JSON.stringify({
    format: "eng-lsp-snapshot-v1",
    symbols: [{
      name,
      kind: 5,
      location: {
        uri: "file:///C:/workspace/main.eng",
        range: {
          start: { line: 0, character: 7 },
          end: { line: 0, character: 19 }
        }
      },
      containerName: "schema"
    }]
  });
}

async function dirtyWorkspaceBuffersReachCompilerStdin() {
  const dirty = documentFixture("C:\\workspace\\main.eng", "schema UnsavedThing {}\n");
  const clean = documentFixture("C:\\workspace\\clean.eng", "clean_value = 1\n", {
    isDirty: false
  });
  const wrongLanguage = documentFixture("C:\\workspace\\notes.txt", "not EngLang", {
    languageId: "plaintext"
  });
  const other = documentFixture("C:\\other\\other.eng", "other_value = 1\n", {
    owner: "other"
  });
  vscodeMock.workspace.textDocuments = [dirty, clean, wrongLanguage, other];
  const exec = installExecFixture();
  try {
    const promise = requestsFixture().workspaceSymbolsForQuery("Unsaved", {}, undefined);
    assert.strictEqual(exec.calls.length, 1);
    const call = exec.calls[0];
    assert.deepStrictEqual(call.args, [
      "--workspace-symbols-stdin",
      "C:\\workspace",
      "Unsaved"
    ]);
    assert.strictEqual(call.options.cwd, "C:\\workspace");
    const payload = JSON.parse(call.stdin);
    assert.strictEqual(payload.format, "eng-lsp-open-documents-v1");
    assert.deepStrictEqual(payload.documents, [{
      path: "C:\\workspace\\main.eng",
      source: "schema UnsavedThing {}\n"
    }]);

    call.callback(null, symbolPayload(), "");
    const symbols = await promise;
    assert.strictEqual(symbols.length, 1);
    assert.strictEqual(symbols[0].name, "UnsavedThing");
  } finally {
    exec.restore();
  }
}

async function changedBufferInvalidatesWorkspaceSymbols() {
  const dirty = documentFixture("C:\\workspace\\main.eng", "schema Before {}\n");
  vscodeMock.workspace.textDocuments = [dirty];
  const exec = installExecFixture();
  try {
    const promise = requestsFixture().workspaceSymbolsForQuery("Before", {}, undefined);
    dirty.version += 1;
    exec.calls[0].callback(null, symbolPayload("Before"), "");
    assert.deepStrictEqual(await promise, []);
  } finally {
    exec.restore();
  }
}

async function changedOpenDocumentSetInvalidatesWorkspaceSymbols() {
  vscodeMock.workspace.textDocuments = [
    documentFixture("C:\\workspace\\main.eng", "schema Before {}\n")
  ];
  const exec = installExecFixture();
  try {
    const promise = requestsFixture().workspaceSymbolsForQuery("Before", {}, undefined);
    vscodeMock.workspace.textDocuments.push(
      documentFixture("C:\\workspace\\new.eng", "schema NewlyOpened {}\n")
    );
    exec.calls[0].callback(null, symbolPayload("Before"), "");
    assert.deepStrictEqual(await promise, []);
  } finally {
    exec.restore();
  }
}

async function cancellationKillsWorkspaceSymbolProcess() {
  vscodeMock.workspace.textDocuments = [
    documentFixture("C:\\workspace\\main.eng", "schema Pending {}\n")
  ];
  const exec = installExecFixture();
  let cancel;
  let disposed = 0;
  const token = {
    isCancellationRequested: false,
    onCancellationRequested(listener) {
      cancel = listener;
      return {
        dispose() {
          disposed += 1;
        }
      };
    }
  };
  try {
    const promise = requestsFixture().workspaceSymbolsForQuery("Pending", {}, token);
    token.isCancellationRequested = true;
    cancel();
    assert.deepStrictEqual(await promise, []);
    assert.strictEqual(exec.calls[0].killed, true);
    assert.strictEqual(disposed, 1);
  } finally {
    exec.restore();
  }
}

async function malformedCompilerSnapshotIsRejected() {
  vscodeMock.workspace.textDocuments = [];
  const logs = [];
  const exec = installExecFixture();
  try {
    const promise = requestsFixture(logs).workspaceSymbolsForQuery("Thing", {}, undefined);
    exec.calls[0].callback(null, JSON.stringify({ symbols: [] }), "");
    assert.deepStrictEqual(await promise, []);
    assert.ok(logs.some((line) => line.includes("did not contain a workspace symbol snapshot")));
  } finally {
    exec.restore();
  }
}

async function main() {
  await dirtyWorkspaceBuffersReachCompilerStdin();
  await changedBufferInvalidatesWorkspaceSymbols();
  await changedOpenDocumentSetInvalidatesWorkspaceSymbols();
  await cancellationKillsWorkspaceSymbolProcess();
  await malformedCompilerSnapshotIsRejected();
  process.stdout.write("VS Code unsaved workspace symbol smoke passed.\n");
}

main().catch((error) => {
  process.stderr.write(String(error.stack || error.message) + "\n");
  process.exitCode = 1;
});
