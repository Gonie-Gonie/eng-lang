"use strict";

const assert = require("assert");
const childProcess = require("child_process");
const Module = require("module");
const path = require("path");

class Range {
  constructor(startLine, startCharacter, endLine, endCharacter) {
    this.start = { line: startLine, character: startCharacter };
    this.end = { line: endLine, character: endCharacter };
  }
}

class Diagnostic {
  constructor(range, message, severity) {
    this.range = range;
    this.message = message;
    this.severity = severity;
  }
}

class EventEmitter {
  constructor() {
    this.event = () => ({ dispose() {} });
  }

  fire() {}

  dispose() {}
}

const vscodeMock = {
  Diagnostic,
  DiagnosticSeverity: {
    Error: 0,
    Warning: 1,
    Information: 2
  },
  DiagnosticTag: {
    Deprecated: 1,
    Unnecessary: 2
  },
  EventEmitter,
  Range,
  window: {
    activeTextEditor: undefined,
    showInformationMessage() {},
    showWarningMessage() {}
  },
  workspace: {
    textDocuments: [],
    getConfiguration(_section, uri) {
      return {
        get(name, fallback) {
          return uri?.configuration?.[name] ?? fallback;
        }
      };
    },
    getWorkspaceFolder() {
      return undefined;
    },
    workspaceFolders: []
  }
};

const originalLoad = Module._load;
let EngDiagnosticsController;
let EngSemanticTokensProvider;
let createLspRequests;
let isWorkspaceEngSourceUri;
let workspaceRootKey;
try {
  Module._load = function loadWithVscodeMock(request, parent, isMain) {
    if (request === "vscode") {
      return vscodeMock;
    }
    return originalLoad.call(this, request, parent, isMain);
  };
  ({ EngDiagnosticsController } = require("../diagnosticsProvider"));
  ({ EngSemanticTokensProvider } = require("../semanticTokensProvider"));
  ({ createLspRequests } = require("../lspRequests"));
  ({ isWorkspaceEngSourceUri, workspaceRootKey } = require("../runtimeDiscovery"));
} finally {
  Module._load = originalLoad;
}

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, reject, resolve };
}

function flushPromises() {
  return new Promise((resolve) => setImmediate(resolve));
}

function wait(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function documentFixture() {
  const source = "value = 1";
  return {
    fileName: "C:\\workspace\\main.eng",
    isDirty: true,
    languageId: "englang",
    lineCount: 1,
    uri: {
      fsPath: "C:\\workspace\\main.eng",
      toString() {
        return "file:///C:/workspace/main.eng";
      }
    },
    version: 1,
    getText() {
      return source;
    },
    lineAt() {
      return { text: source };
    }
  };
}

function reviewFixture(message) {
  return {
    format: "eng-lsp-snapshot-v1",
    diagnostics: [
      {
        message,
        range: {
          start: { line: 0, character: 0 },
          end: { line: 0, character: 5 }
        },
        severity: "warning"
      }
    ]
  };
}

function diagnosticsFixture() {
  const calls = [];
  return {
    calls,
    delete(uri) {
      calls.push({ kind: "delete", uri: uri.toString() });
    },
    set(uri, diagnostics) {
      calls.push({
        kind: "set",
        messages: diagnostics.map((diagnostic) => diagnostic.message),
        uri: uri.toString()
      });
    }
  };
}

function controllerFixture(requests, diagnostics, cachedReviews = [], clearedReviews = []) {
  return new EngDiagnosticsController({}, diagnostics, {
    cacheReview(_document, review) {
      cachedReviews.push(review);
    },
    clearCachedReview(document) {
      clearedReviews.push(document.uri.toString());
    },
    diagnosticsRuntime() {
      return "lsp-snapshot";
    },
    findLspRuntime() {
      return "eng-lsp.exe";
    },
    output: {
      appendLine() {}
    },
    snapshotDocumentSource() {
      const request = requests.shift();
      assert.ok(request, "unexpected editor snapshot request");
      return request.promise;
    },
    updateReviewRiskDecorations() {},
    updateReviewValidationDecorations() {},
    updateSemanticSymbolDecorations() {}
  });
}

async function latestDocumentCheckWins() {
  const oldRequest = deferred();
  const latestRequest = deferred();
  const document = documentFixture();
  const diagnostics = diagnosticsFixture();
  const cachedReviews = [];
  const controller = controllerFixture(
    [oldRequest, latestRequest],
    diagnostics,
    cachedReviews
  );

  controller.checkDocumentSource(document);
  controller.checkDocumentSource(document);
  latestRequest.resolve(reviewFixture("latest"));
  await flushPromises();

  assert.deepStrictEqual(
    diagnostics.calls.filter((call) => call.kind === "set").map((call) => call.messages),
    [["latest"]]
  );
  assert.strictEqual(cachedReviews.at(-1).diagnostics[0].message, "latest");

  oldRequest.resolve(reviewFixture("stale"));
  await flushPromises();
  assert.deepStrictEqual(
    diagnostics.calls.filter((call) => call.kind === "set").map((call) => call.messages),
    [["latest"]],
    "an older same-version check must not replace current Problems"
  );
  assert.strictEqual(cachedReviews.at(-1).diagnostics[0].message, "latest");
}

async function staleFailureDoesNotReplaceProblems() {
  const oldRequest = deferred();
  const latestRequest = deferred();
  const document = documentFixture();
  const diagnostics = diagnosticsFixture();
  const controller = controllerFixture([oldRequest, latestRequest], diagnostics);

  controller.checkDocumentSource(document);
  controller.checkDocumentSource(document);
  oldRequest.reject(new Error("stale failure"));
  await flushPromises();
  assert.deepStrictEqual(
    diagnostics.calls.filter((call) => call.kind === "set"),
    [],
    "a stale rejected check must not publish a tool failure"
  );

  latestRequest.resolve(reviewFixture("current"));
  await flushPromises();
  assert.deepStrictEqual(
    diagnostics.calls.filter((call) => call.kind === "set").map((call) => call.messages),
    [["current"]]
  );
}

async function currentFailureClearsCachedReview() {
  const request = deferred();
  const document = documentFixture();
  const diagnostics = diagnosticsFixture();
  const clearedReviews = [];
  const controller = controllerFixture([request], diagnostics, [], clearedReviews);

  controller.checkDocumentSource(document);
  request.reject(new Error("current failure"));
  await flushPromises();

  assert.deepStrictEqual(clearedReviews, [document.uri.toString()]);
  assert.strictEqual(
    diagnostics.calls.filter((call) => call.kind === "set").length,
    1,
    "a current tool failure must replace stale Problems and cached review data"
  );
}

async function clearingProblemsInvalidatesInFlightCheck() {
  const request = deferred();
  const document = documentFixture();
  const diagnostics = diagnosticsFixture();
  const controller = controllerFixture([request], diagnostics);

  controller.checkDocumentSource(document);
  controller.clearDocumentDiagnostics(document, "test clear");
  request.resolve(reviewFixture("late"));
  await flushPromises();

  assert.deepStrictEqual(
    diagnostics.calls.map((call) => call.kind),
    ["delete"],
    "cleared Problems must stay clear when an earlier request completes"
  );
}

async function newerFileCheckStopsOlderProcess() {
  const originalExecFile = childProcess.execFile;
  const callbacks = [];
  const children = [];
  childProcess.execFile = (_runtime, _args, _options, complete) => {
    const child = {
      killCount: 0,
      kill() {
        this.killCount += 1;
      }
    };
    callbacks.push(complete);
    children.push(child);
    return child;
  };

  try {
    const document = documentFixture();
    document.isDirty = false;
    const diagnostics = diagnosticsFixture();
    let snapshotClearCount = 0;
    const controller = new EngDiagnosticsController({}, diagnostics, {
      clearSnapshotCache() {
        snapshotClearCount += 1;
      },
      diagnosticsRuntime() {
        return "eng-cli";
      },
      diagnosticsRuntimeLabel() {
        return "saved file";
      },
      findRuntime() {
        return "eng.exe";
      },
      output: {
        appendLine() {}
      },
      workspaceRoot() {
        return "C:\\workspace";
      }
    });

    controller.checkDocument(document);
    controller.checkDocument(document);
    assert.strictEqual(children[0].killCount, 1, "a newer file check must stop older work");

    callbacks[0](null, JSON.stringify(reviewFixture("stale file result")), "");
    callbacks[1](null, JSON.stringify(reviewFixture("current file result")), "");
    await flushPromises();
    assert.deepStrictEqual(
      diagnostics.calls.filter((call) => call.kind === "set").map((call) => call.messages),
      [["current file result"]]
    );

    controller.checkDocument(document);
    controller.dispose();
    assert.strictEqual(snapshotClearCount, 1, "disposing diagnostics must cancel shared snapshots");
    assert.strictEqual(children[2].killCount, 1, "disposing diagnostics must stop active checks");
    callbacks[2](null, JSON.stringify(reviewFixture("result after dispose")), "");
    assert.deepStrictEqual(
      diagnostics.calls.filter((call) => call.kind === "set").map((call) => call.messages),
      [["current file result"]],
      "a disposed controller must ignore child-process callbacks"
    );
  } finally {
    childProcess.execFile = originalExecFile;
  }
}

async function callerCancellationDoesNotKillSharedSnapshot() {
  const originalExecFile = childProcess.execFile;
  let callback;
  let cancel;
  let disposed = 0;
  let execCount = 0;
  let killCount = 0;
  let stdinText = "";
  let requestArgs = [];
  childProcess.execFile = (_runtime, args, _options, complete) => {
    execCount += 1;
    requestArgs = args;
    callback = complete;
    return {
      kill() {
        killCount += 1;
      },
      stdin: {
        end(value) {
          stdinText = value;
        }
      }
    };
  };

  try {
    const document = documentFixture();
    let importedSource = "const SHARED_GAIN: Ratio = 0.9\n";
    const importedDocument = {
      isDirty: true,
      languageId: "englang",
      uri: {
        fsPath: "C:\\workspace\\module.eng",
        scheme: "file",
        toString() {
          return "file:///C:/workspace/module.eng";
        }
      },
      version: 4,
      getText() {
        return importedSource;
      }
    };
    vscodeMock.workspace.textDocuments = [document, importedDocument];
    const requests = createLspRequests({
      appendOutputLine() {},
      findLspRuntime() {
        return "eng-lsp.exe";
      },
      workspaceRoot() {
        return "C:\\workspace";
      }
    });
    const cancellationToken = {
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

    const cancelledCaller = requests.snapshotDocumentSource(
      document,
      {},
      cancellationToken
    );
    const sharedCaller = requests.snapshotDocumentSource(document, {});
    assert.strictEqual(execCount, 1, "same-version callers must share one snapshot process");
    assert.deepStrictEqual(requestArgs, [
      "--workspace-snapshot-stdin",
      "C:\\workspace",
      "C:\\workspace\\main.eng"
    ]);
    assert.deepStrictEqual(JSON.parse(stdinText), {
      format: "eng-lsp-open-documents-v1",
      documents: [
        { path: "C:\\workspace\\main.eng", source: document.getText() },
        { path: "C:\\workspace\\module.eng", source: importedSource }
      ]
    });

    cancellationToken.isCancellationRequested = true;
    cancel();
    assert.strictEqual(await cancelledCaller, undefined);
    assert.strictEqual(killCount, 0, "one caller must not kill a shared snapshot process");

    callback(null, JSON.stringify(reviewFixture("shared")), "");
    const sharedResult = await sharedCaller;
    assert.strictEqual(sharedResult.diagnostics[0].message, "shared");
    assert.strictEqual(disposed, 1);

    importedSource = "const CHANGED_GAIN: Ratio = 0.7\n";
    importedDocument.version += 1;
    const refreshed = requests.snapshotDocumentSource(document, {});
    assert.strictEqual(execCount, 2, "an imported buffer version must invalidate the snapshot cache");
    assert.strictEqual(
      JSON.parse(stdinText).documents[1].source,
      importedSource,
      "the refreshed snapshot must send the latest imported buffer"
    );
    callback(null, JSON.stringify(reviewFixture("refreshed")), "");
    assert.strictEqual((await refreshed).diagnostics[0].message, "refreshed");

    importedSource = "const STALE_GAIN: Ratio = 0.5\n";
    importedDocument.version += 1;
    const staleSnapshot = requests.snapshotDocumentSource(document, {});
    assert.strictEqual(execCount, 3);
    requests.clearSnapshotCache(document);
    assert.strictEqual(killCount, 1, "cache invalidation must stop stale snapshot work");
    assert.strictEqual(await staleSnapshot, undefined);

    const currentSnapshot = requests.snapshotDocumentSource(document, {});
    assert.strictEqual(execCount, 4, "a new revision must start after stale work is cancelled");
    callback(null, JSON.stringify(reviewFixture("current after cancel")), "");
    assert.strictEqual(
      (await currentSnapshot).diagnostics[0].message,
      "current after cancel"
    );
  } finally {
    vscodeMock.workspace.textDocuments = [];
    childProcess.execFile = originalExecFile;
  }
}

function changedImportSchedulesOpenWorkspaceDiagnostics() {
  const scheduled = [];
  const mainDocument = { languageId: "englang", root: "C:\\workspace", name: "main" };
  const moduleDocument = { languageId: "englang", root: "C:\\workspace", name: "module" };
  const otherDocument = { languageId: "englang", root: "C:\\other", name: "other" };
  const ignoredDocument = { languageId: "plaintext", root: "C:\\workspace", name: "notes" };
  vscodeMock.workspace.textDocuments = [mainDocument, moduleDocument, otherDocument, ignoredDocument];
  const controller = new EngDiagnosticsController({}, {}, {
    isEngDocument: (document) => document.languageId === "englang",
    workspaceRoot: (document) => document.root
  });
  controller.scheduleChangedCheck = (document) => scheduled.push(document.name);

  controller.scheduleWorkspaceChangedChecks(moduleDocument);

  assert.deepStrictEqual(scheduled, ["main", "module"]);
  scheduled.length = 0;
  controller.scheduleWorkspaceChangedChecks(moduleDocument, false);
  assert.deepStrictEqual(scheduled, ["main"]);
  vscodeMock.workspace.textDocuments = [];
}

async function diskImportChangeUsesSelectedDiagnosticsMode() {
  const checks = [];
  const root = path.resolve("workspace-disk-refresh");
  function dependencyDocument(name, options = {}) {
    const fsPath = path.join(options.root ?? root, `${name}.eng`);
    return {
      fileName: fsPath,
      isDirty: options.isDirty ?? false,
      languageId: "englang",
      mode: options.mode ?? "lsp-snapshot",
      root: options.root ?? root,
      uri: {
        configuration: options.configuration ?? {},
        fsPath,
        scheme: "file",
        toString() {
          return `file://${fsPath.replace(/\\/g, "/")}`;
        }
      },
      version: 1
    };
  }

  const changed = dependencyDocument("module");
  const liveSaved = dependencyDocument("live-saved");
  const liveDirty = dependencyDocument("live-dirty", { isDirty: true });
  const fileSaved = dependencyDocument("file-saved", { mode: "eng-cli" });
  const fileDirty = dependencyDocument("file-dirty", { isDirty: true, mode: "eng-cli" });
  const disabled = dependencyDocument("disabled", {
    configuration: { lintOnSave: false }
  });
  const otherRoot = dependencyDocument("other", {
    root: path.resolve("other-workspace")
  });
  vscodeMock.workspace.textDocuments = [
    changed,
    liveSaved,
    liveDirty,
    fileSaved,
    fileDirty,
    disabled,
    otherRoot
  ];
  const controller = new EngDiagnosticsController({}, {}, {
    checkDebounceMs: 5,
    diagnosticsRuntime: (document) => document.mode,
    isEngDocument: (document) => document.languageId === "englang",
    workspaceRoot: (document) => document.root,
    workspaceRootKey
  });
  liveSaved.uri.configuration.liveDiagnosticsDelayMs = 175;
  assert.strictEqual(controller.liveDiagnosticsDelayMs(liveSaved), 175);
  liveSaved.uri.configuration.liveDiagnosticsDelayMs = 1;
  assert.strictEqual(controller.liveDiagnosticsDelayMs(liveSaved), 100);
  liveSaved.uri.configuration.liveDiagnosticsDelayMs = 9000;
  assert.strictEqual(controller.liveDiagnosticsDelayMs(liveSaved), 5000);
  delete liveSaved.uri.configuration.liveDiagnosticsDelayMs;
  controller.checkDocumentSource = (document) => checks.push(`source:${path.basename(document.fileName)}`);
  controller.checkDocument = (document) => checks.push(`file:${path.basename(document.fileName)}`);

  controller.scheduleWorkspaceFileChangedChecks(changed);
  await wait(20);
  assert.deepStrictEqual(checks.sort(), [
    "file:file-saved.eng",
    "source:live-dirty.eng",
    "source:live-saved.eng"
  ]);

  checks.length = 0;
  controller.checkDebounceMs = 20;
  controller.scheduleDependencyFileCheck(liveSaved);
  controller.dispose();
  await wait(30);
  assert.deepStrictEqual(checks, [], "disposing diagnostics must cancel pending dependency checks");
  vscodeMock.workspace.textDocuments = [];
}

function workspaceSourceWatcherIgnoresGeneratedTrees() {
  const root = path.resolve("workspace-source-watcher");
  const originalGetWorkspaceFolder = vscodeMock.workspace.getWorkspaceFolder;
  vscodeMock.workspace.getWorkspaceFolder = (uri) => {
    const relative = path.relative(root, uri.fsPath);
    if (path.isAbsolute(relative) || relative === ".." || relative.startsWith(`..${path.sep}`)) {
      return undefined;
    }
    return { uri: { fsPath: root } };
  };
  const uri = (relativePath, scheme = "file") => ({
    fsPath: path.join(root, relativePath),
    scheme
  });
  try {
    assert.strictEqual(isWorkspaceEngSourceUri(uri(path.join("src", "model.eng"))), true);
    assert.strictEqual(isWorkspaceEngSourceUri(uri(path.join("build", "copy.eng"))), false);
    assert.strictEqual(isWorkspaceEngSourceUri(uri(path.join("nested", "target", "copy.eng"))), false);
    assert.strictEqual(isWorkspaceEngSourceUri(uri(path.join("src", "notes.txt"))), false);
    assert.strictEqual(isWorkspaceEngSourceUri(uri(path.join("src", "model.eng"), "untitled")), false);
  } finally {
    vscodeMock.workspace.getWorkspaceFolder = originalGetWorkspaceFolder;
  }
}

async function semanticRefreshIsDebouncedAndDisposed() {
  const provider = new EngSemanticTokensProvider({}, {});
  let refreshCount = 0;
  provider._onDidChangeSemanticTokens.fire = () => {
    refreshCount += 1;
  };

  provider.scheduleRefresh(5);
  provider.scheduleRefresh(5);
  await wait(20);
  assert.strictEqual(refreshCount, 1, "rapid imported-buffer changes should request one color refresh");

  provider.scheduleRefresh(20);
  provider.dispose();
  await wait(30);
  assert.strictEqual(refreshCount, 1, "disposing the provider must cancel a pending color refresh");
}

async function main() {
  await latestDocumentCheckWins();
  await staleFailureDoesNotReplaceProblems();
  await currentFailureClearsCachedReview();
  await clearingProblemsInvalidatesInFlightCheck();
  await newerFileCheckStopsOlderProcess();
  await callerCancellationDoesNotKillSharedSnapshot();
  changedImportSchedulesOpenWorkspaceDiagnostics();
  await diskImportChangeUsesSelectedDiagnosticsMode();
  workspaceSourceWatcherIgnoresGeneratedTrees();
  await semanticRefreshIsDebouncedAndDisposed();
  process.stdout.write("VS Code editor request race smoke passed.\n");
}

main().catch((error) => {
  process.stderr.write(String(error.stack || error.message) + "\n");
  process.exitCode = 1;
});
