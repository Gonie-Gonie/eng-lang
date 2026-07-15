"use strict";

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const vm = require("vm");

const appPath = path.join(__dirname, "..", "ui", "app.js");
const source = fs.readFileSync(appPath, "utf8");
const invokeCalls = [];
const openFileSources = new Map();
let saveFailurePath = null;
let saveFilesPromise = null;
let codeActionPayload = null;
let definitionPromise = null;
let prepareRenamePayload = null;
let workspaceSymbolPayload = null;
let workspaceSymbolPromise = null;
const pendingBootstrap = new Promise(() => {});
const nativeWindowState = {
  closeCallback: null,
  destroyCalls: 0
};
const nativeWindow = {
  destroy() {
    nativeWindowState.destroyCalls += 1;
    return Promise.resolve();
  },
  onCloseRequested(callback) {
    nativeWindowState.closeCallback = callback;
    return Promise.resolve(() => {});
  }
};
const context = vm.createContext({
  Blob,
  TextDecoder,
  TextEncoder,
  URL,
  clearTimeout,
  console,
  document: {
    body: {
      appendChild() {}
    },
    createElement() {
      return {};
    },
    getElementById() {
      return null;
    },
    querySelector() {
      return null;
    },
    querySelectorAll() {
      return [];
    }
  },
  navigator: {},
  setTimeout,
  window: {
    __TAURI__: {
      core: {
        invoke(command, args) {
          if (command === "ide_bootstrap") {
            return pendingBootstrap;
          }
          invokeCalls.push({ args, command });
          if (command === "ide_save_file") {
            if (args.path === saveFailurePath) {
              return Promise.reject(new Error(`Save conflict: ${args.path}; no files were written`));
            }
            return Promise.resolve({ path: args.path, source: args.source });
          }
          if (command === "ide_save_files") {
            const failed = args.files.find((file) => file.path === saveFailurePath);
            if (failed) {
              return Promise.reject(new Error(`Save conflict: ${failed.path}; no files were written`));
            }
            if (saveFilesPromise) return saveFilesPromise;
            return Promise.resolve(args.files.map((file) => ({
              path: file.path,
              source: file.source
            })));
          }
          if (command === "ide_open_file") {
            if (!openFileSources.has(args.path)) {
              return Promise.reject(new Error(`cannot open ${args.path}`));
            }
            return Promise.resolve({ path: args.path, source: openFileSources.get(args.path) });
          }
          if (command === "ide_code_actions") {
            return Promise.resolve(codeActionPayload || { uri: "file:///C:/Repo/main.eng", actions: [] });
          }
          if (command === "ide_definition") {
            return definitionPromise || Promise.resolve(null);
          }
          if (command === "ide_prepare_rename") {
            return Promise.resolve(prepareRenamePayload || {});
          }
          if (command === "ide_workspace_symbols") {
            if (workspaceSymbolPromise) return workspaceSymbolPromise;
            return Promise.resolve(workspaceSymbolPayload || {
              format: "eng-lsp-snapshot-v1",
              symbols: []
            });
          }
          if (command === "ide_run" || command === "ide_terminal") {
            return Promise.resolve({
              ok: true,
              runtimeUpdated: true,
              terminal: "run ok",
              check: {
                diagnostics: [],
                symbols: [],
                status: "ok",
                semanticTokens: { legend: {}, tokens: [] },
                hovers: [],
                documentSymbols: []
              },
              variables: [],
              args: [],
              artifacts: [],
              plotSpec: null,
              reportTitle: "",
              inspectors: {}
            });
          }
          return Promise.resolve({});
        }
      },
      event: {},
      window: {
        getCurrentWindow() {
          return nativeWindow;
        }
      }
    },
    addEventListener() {},
    localStorage: undefined
  }
});

vm.runInContext(source, context, { filename: appPath });

function run(code) {
  return vm.runInContext(code, context);
}

async function dirtyTabRequiresDecision() {
  run(`
    state.tabs = [
      { path: "dirty.eng", source: "changed", dirty: true },
      { path: "clean.eng", source: "clean", dirty: false }
    ];
    state.currentPath = "dirty.eng";
    state.source = "changed";
    state.dirty = true;
    globalThis.openedClosePath = null;
    globalThis.realOpenUnsavedChangesDialog = openUnsavedChangesDialog;
    openUnsavedChangesDialog = (path) => {
      globalThis.openedClosePath = path;
      state.pendingTabClose = path;
    };
  `);

  await run(`closeTab("dirty.eng")`);
  assert.strictEqual(run("globalThis.openedClosePath"), "dirty.eng");
  assert.strictEqual(run("state.tabs.length"), 2);
  assert.strictEqual(run("state.pendingTabClose"), "dirty.eng");
  run("openUnsavedChangesDialog = globalThis.realOpenUnsavedChangesDialog");
}

async function reopeningDirtyTabPreservesTheOpenBuffer() {
  invokeCalls.length = 0;
  run(`
    state.root = "C:/Repo";
    state.tabs = [{
      path: "main.eng",
      source: "editor changed",
      savedSource: "disk baseline",
      dirty: true
    }];
    state.currentPath = "main.eng";
    state.source = "editor changed";
    state.savedSource = "disk baseline";
    state.dirty = true;
  `);

  await run('openFile("main.eng")');
  assert.strictEqual(invokeCalls.some((item) => item.command === "ide_open_file"), false);
  assert.strictEqual(run("state.source"), "editor changed");
  assert.strictEqual(run("state.savedSource"), "disk baseline");
  assert.strictEqual(run("state.dirty"), true);
  run('state.root = ""');
}

async function saveDecisionPersistsThenCloses() {
  invokeCalls.length = 0;
  run(`
    state.tabs = [
      { path: "current.eng", source: "current", savedSource: "current", dirty: false },
      { path: "dirty.eng", source: "changed", savedSource: "original", dirty: true }
    ];
    state.currentPath = "current.eng";
    state.source = "current";
    state.savedSource = "current";
    state.dirty = false;
    state.pendingTabClose = "dirty.eng";
    globalThis.renderCount = 0;
    render = () => {
      globalThis.renderCount += 1;
    };
  `);

  await run("savePendingTabAndClose()");
  assert.strictEqual(invokeCalls.length, 1);
  assert.strictEqual(invokeCalls[0].command, "ide_save_file");
  assert.strictEqual(invokeCalls[0].args.path, "dirty.eng");
  assert.strictEqual(invokeCalls[0].args.source, "changed");
  assert.strictEqual(invokeCalls[0].args.expectedSource, "original");
  assert.deepStrictEqual(
    Array.from(run("state.tabs.map((tab) => tab.path)")),
    ["current.eng"]
  );
  assert.strictEqual(run("state.pendingTabClose"), null);
  assert.strictEqual(run("globalThis.renderCount"), 1);
}

async function runSafelySavesBeforeExecuting() {
  invokeCalls.length = 0;
  run(`
    state.root = "C:/Repo";
    state.currentPath = "main.eng";
    state.source = "value = 2";
    state.savedSource = "value = 1";
    state.dirty = true;
    state.tabs = [
      { path: "main.eng", source: "value = 2", savedSource: "value = 1", dirty: true },
      { path: "module.eng", source: "gain = 0.9", savedSource: "gain = 0.8", dirty: true },
      { path: "data/input.csv", source: "x,y\\n1,2\\n", savedSource: "x,y\\n1,1\\n", dirty: true },
      { path: "clean.eng", source: "clean = 1", savedSource: "clean = 1", dirty: false },
      { path: "C:/Else/external.eng", source: "external = 2", savedSource: "external = 1", dirty: true }
    ];
    state.profile = "normal";
  `);

  await run("runCurrent()");
  assert.deepStrictEqual(invokeCalls.map((item) => item.command), ["ide_save_files", "ide_run"]);
  assert.deepStrictEqual(JSON.parse(JSON.stringify(invokeCalls[0].args.files)), [
    { path: "main.eng", source: "value = 2", expectedSource: "value = 1" },
    { path: "module.eng", source: "gain = 0.9", expectedSource: "gain = 0.8" },
    { path: "data/input.csv", source: "x,y\n1,2\n", expectedSource: "x,y\n1,1\n" },
    { path: "clean.eng", source: "clean = 1", expectedSource: "clean = 1" }
  ]);
  assert.strictEqual(run("state.savedSource"), "value = 2");
  assert.strictEqual(run("state.dirty"), false);
  assert.deepStrictEqual(
    Array.from(run("state.tabs.slice(0, 4).map((tab) => tab.dirty)")),
    [false, false, false, false]
  );
  assert.strictEqual(run("state.tabs[4].dirty"), true);
  run('state.root = ""');
}

async function terminalRunSafelySavesBeforeExecuting() {
  invokeCalls.length = 0;
  run(`
    state.root = "C:/Repo";
    state.currentPath = "main.eng";
    state.source = "value = 3";
    state.savedSource = "value = 2";
    state.dirty = true;
    state.tabs = [
      { path: "main.eng", source: "value = 3", savedSource: "value = 2", dirty: true },
      { path: "module.eng", source: "gain = 1.0", savedSource: "gain = 0.9", dirty: true }
    ];
    state.runDir = ".";
    state.profile = "normal";
  `);

  await run('sendTerminalCommand("run")');
  assert.deepStrictEqual(invokeCalls.map((item) => item.command), ["ide_save_files", "ide_terminal"]);
  assert.deepStrictEqual(JSON.parse(JSON.stringify(invokeCalls[0].args.files)), [
    { path: "main.eng", source: "value = 3", expectedSource: "value = 2" },
    { path: "module.eng", source: "gain = 1.0", expectedSource: "gain = 0.9" }
  ]);
  assert.strictEqual(invokeCalls[1].args.command, "run");
  assert.strictEqual(run("state.savedSource"), "value = 3");
  assert.strictEqual(run("state.dirty"), false);
  run('state.root = ""');
}

async function runConflictKeepsEveryBufferUnsaved() {
  invokeCalls.length = 0;
  saveFailurePath = "module.eng";
  run(`
    state.root = "C:/Repo";
    state.currentPath = "main.eng";
    state.source = "value = 4";
    state.savedSource = "value = 3";
    state.dirty = true;
    state.tabs = [
      { path: "main.eng", source: "value = 4", savedSource: "value = 3", dirty: true },
      { path: "module.eng", source: "gain = 1.1", savedSource: "gain = 1.0", dirty: true }
    ];
  `);

  await run("runCurrent()");
  saveFailurePath = null;
  assert.deepStrictEqual(invokeCalls.map((item) => item.command), ["ide_save_files"]);
  assert.deepStrictEqual(Array.from(run("state.tabs.map((tab) => tab.dirty)")), [true, true]);
  assert.match(run("state.status"), /Run failed: (?:Error: )?Save conflict: module\.eng/);
  run('state.root = ""');
}

async function runChangedImportDuringSaveCancelsExecution() {
  invokeCalls.length = 0;
  let resolveSave;
  saveFilesPromise = new Promise((resolve) => {
    resolveSave = resolve;
  });
  run(`
    state.root = "C:/Repo";
    state.currentPath = "main.eng";
    state.source = "value = 5";
    state.savedSource = "value = 4";
    state.dirty = true;
    state.tabs = [
      { path: "main.eng", source: "value = 5", savedSource: "value = 4", dirty: true },
      { path: "module.eng", source: "gain = 1.2", savedSource: "gain = 1.1", dirty: true }
    ];
  `);

  const pending = run("runCurrent()");
  run(`
    state.tabs[1].source = "gain = 1.3";
    state.tabs[1].dirty = true;
  `);
  resolveSave(invokeCalls[0].args.files.map((file) => ({
    path: file.path,
    source: file.source
  })));
  await pending;
  saveFilesPromise = null;

  assert.deepStrictEqual(invokeCalls.map((item) => item.command), ["ide_save_files"]);
  assert.deepStrictEqual(Array.from(run("state.tabs.map((tab) => tab.dirty)")), [false, true]);
  assert.strictEqual(run("state.tabs[1].savedSource"), "gain = 1.2");
  assert.strictEqual(run("state.tabs[1].source"), "gain = 1.3");
  assert.strictEqual(run("state.status"), "Run failed; buffer changed");
  run('state.root = ""');
}

function saveShortcutUsesCurrentAction() {
  run(`
    state.pendingTabClose = null;
    globalThis.saveShortcutCalls = 0;
    saveCurrent = async () => {
      globalThis.saveShortcutCalls += 1;
    };
    globalThis.saveShortcutEvent = {
      altKey: false,
      ctrlKey: true,
      key: "s",
      metaKey: false,
      prevented: false,
      shiftKey: false,
      preventDefault() {
        this.prevented = true;
      }
    };
    handleGlobalKeyDown(globalThis.saveShortcutEvent);
  `);

  assert.strictEqual(run("globalThis.saveShortcutEvent.prevented"), true);
  assert.strictEqual(run("globalThis.saveShortcutCalls"), 1);
}

function definitionPathsNormalizeWorkspaceTargets() {
  run('state.root = "C:/Repo"');
  assert.strictEqual(
    run('definitionPathFromUri("file:///C:/Repo/stdlib/eng/path.eng")'),
    "C:/Repo/stdlib/eng/path.eng"
  );
  assert.strictEqual(
    run('definitionWorkspacePath("C:/Repo/stdlib/eng/path.eng")'),
    "stdlib/eng/path.eng"
  );
  assert.strictEqual(
    run('sameDefinitionPath("stdlib/Eng/Path.eng", "STDLIB/eng/path.eng")'),
    true
  );
  assert.strictEqual(run('definitionPathFromUri("https://example.com/main.eng")'), "");
  run('state.root = ""');
}

function definitionRequestUsesUtf16Caret() {
  run(`
    state.currentPath = "main.eng";
    globalThis.definitionEditor = {
      value: "head\\n  \\uD83D\\uDE00alpha",
      selectionStart: "head\\n  \\uD83D\\uDE00alpha".indexOf("alpha")
    };
    globalThis.definitionRequest = editorDefinitionRequest(globalThis.definitionEditor);
  `);
  assert.strictEqual(run("globalThis.definitionRequest.path"), "main.eng");
  assert.strictEqual(run("globalThis.definitionRequest.line"), 1);
  assert.strictEqual(run("globalThis.definitionRequest.character"), 4);
}

async function definitionNavigationPreservesDirtyOpenTab() {
  run(`
    state.root = "C:/Repo";
    state.currentPath = "main.eng";
    state.tabs = [
      { path: "main.eng", source: "main", dirty: false },
      { path: "lib.eng", source: "unsaved", dirty: true }
    ];
    globalThis.definitionSwitchPath = null;
    globalThis.definitionOpenCalls = 0;
    globalThis.realDefinitionSwitchTab = switchTab;
    globalThis.realDefinitionOpenFile = openFile;
    switchTab = async (path) => {
      globalThis.definitionSwitchPath = path;
      state.currentPath = path;
    };
    openFile = async () => {
      globalThis.definitionOpenCalls += 1;
    };
  `);

  assert.strictEqual(await run('openDefinitionTarget("C:/Repo/lib.eng")'), true);
  assert.strictEqual(run("globalThis.definitionSwitchPath"), "lib.eng");
  assert.strictEqual(run("globalThis.definitionOpenCalls"), 0);
  assert.strictEqual(run('state.tabs.find((tab) => tab.path === "lib.eng").dirty'), true);
  run(`
    switchTab = globalThis.realDefinitionSwitchTab;
    openFile = globalThis.realDefinitionOpenFile;
    state.root = "";
  `);
}

async function definitionNavigationUsesAndGuardsAllDirtyWorkspaceBuffers() {
  invokeCalls.length = 0;
  let resolveDefinition;
  definitionPromise = new Promise((resolve) => {
    resolveDefinition = resolve;
  });
  run(`
    state.root = "C:/Repo";
    state.currentPath = "main.eng";
    state.source = "use \\\"lib.eng\\\"\\nvalue = SHARED_GAIN\\n";
    state.dirty = true;
    state.tabs = [
      { path: "main.eng", source: state.source, dirty: true },
      { path: "lib.eng", source: "const SHARED_GAIN: Ratio = 0.9\\n", dirty: true },
      { path: "clean.eng", source: "clean = 1\\n", dirty: false }
    ];
    globalThis.realDefinitionById = byId;
    globalThis.definitionRequestEditor = {
      value: state.source,
      selectionStart: state.source.indexOf("SHARED_GAIN")
    };
    byId = (id) => id === "editor" ? globalThis.definitionRequestEditor : null;
  `);

  const pending = run("goToDefinitionAtCaret()");
  assert.strictEqual(invokeCalls.length, 1);
  assert.strictEqual(invokeCalls[0].command, "ide_definition");
  assert.deepStrictEqual(
    Array.from(invokeCalls[0].args.documents, (document) => ({ ...document })),
    [{ path: "lib.eng", source: "const SHARED_GAIN: Ratio = 0.9\n" }]
  );
  run('state.tabs.find((tab) => tab.path === "lib.eng").source = "const SHARED_GAIN: Ratio = 1.0\\n"');
  resolveDefinition({
    uri: "file:///C:/Repo/lib.eng",
    range: {
      start: { line: 0, character: 6 },
      end: { line: 0, character: 17 }
    }
  });
  assert.strictEqual(await pending, false);
  assert.strictEqual(run("state.status"), "Definition cancelled; another modified buffer changed");
  run(`
    byId = globalThis.realDefinitionById;
    state.root = "";
  `);
  definitionPromise = null;
}

function definitionShortcutUsesCurrentAction() {
  run(`
    state.pendingTabClose = null;
    state.pendingWindowClose = false;
    globalThis.definitionShortcutCalls = 0;
    globalThis.realGoToDefinitionAtCaret = goToDefinitionAtCaret;
    goToDefinitionAtCaret = async () => {
      globalThis.definitionShortcutCalls += 1;
    };
    globalThis.definitionShortcutEvent = {
      altKey: false,
      ctrlKey: false,
      key: "F12",
      metaKey: false,
      prevented: false,
      shiftKey: false,
      preventDefault() {
        this.prevented = true;
      }
    };
    handleGlobalKeyDown(globalThis.definitionShortcutEvent);
    goToDefinitionAtCaret = globalThis.realGoToDefinitionAtCaret;
  `);

  assert.strictEqual(run("globalThis.definitionShortcutEvent.prevented"), true);
  assert.strictEqual(run("globalThis.definitionShortcutCalls"), 1);
}

function workspaceSymbolShortcutOpensCompilerSearch() {
  run(`
    state.pendingQuickFix = null;
    state.pendingRename = null;
    state.pendingWorkspaceSymbols = null;
    state.pendingTabClose = null;
    state.pendingWindowClose = false;
    globalThis.workspaceSymbolShortcutCalls = 0;
    globalThis.realOpenWorkspaceSymbolSearch = openWorkspaceSymbolSearch;
    openWorkspaceSymbolSearch = () => {
      globalThis.workspaceSymbolShortcutCalls += 1;
      return true;
    };
    globalThis.workspaceSymbolShortcutEvent = {
      altKey: false,
      ctrlKey: true,
      key: "t",
      metaKey: false,
      prevented: false,
      shiftKey: false,
      preventDefault() {
        this.prevented = true;
      }
    };
    handleGlobalKeyDown(globalThis.workspaceSymbolShortcutEvent);
    openWorkspaceSymbolSearch = globalThis.realOpenWorkspaceSymbolSearch;
  `);

  assert.strictEqual(run("globalThis.workspaceSymbolShortcutEvent.prevented"), true);
  assert.strictEqual(run("globalThis.workspaceSymbolShortcutCalls"), 1);
}

async function workspaceSymbolSearchUsesDirtyBuffersAndCompilerLocations() {
  invokeCalls.length = 0;
  workspaceSymbolPromise = null;
  workspaceSymbolPayload = {
    format: "eng-lsp-snapshot-v1",
    symbols: [
      {
        name: "WorkspaceThingFactory",
        kind: 12,
        location: {
          uri: "file:///C:/Repo/lib.eng",
          range: {
            start: { line: 4, character: 0 },
            end: { line: 4, character: 21 }
          }
        },
        containerName: "function"
      },
      {
        name: "WorkspaceThing",
        kind: 5,
        location: {
          uri: "file:///C:/Repo/main.eng",
          range: {
            start: { line: 0, character: 7 },
            end: { line: 0, character: 21 }
          }
        },
        containerName: "schema"
      }
    ]
  };
  run(`
    state.root = "C:/Repo";
    state.currentPath = "main.eng";
    state.source = "schema WorkspaceThing {}";
    state.dirty = true;
    state.tabs = [
      { path: "main.eng", source: state.source, dirty: true },
      { path: "lib.eng", source: "fn WorkspaceThingFactory() {}", dirty: false },
      { path: "C:/Else/outside.eng", source: "outside = 1", dirty: true }
    ];
    state.pendingWorkspaceSymbols = {
      busy: true,
      error: "",
      items: [],
      query: "WorkspaceThing",
      revision: 0,
      selectedIndex: 0
    };
  `);

  assert.strictEqual(await run("requestWorkspaceSymbols(state.pendingWorkspaceSymbols)"), true);
  const call = invokeCalls.find((item) => item.command === "ide_workspace_symbols");
  assert.ok(call, "workspace symbol search should call the native compiler bridge");
  assert.strictEqual(call.args.query, "WorkspaceThing");
  assert.deepStrictEqual(JSON.parse(JSON.stringify(call.args.documents)), [{
    path: "main.eng",
    source: "schema WorkspaceThing {}"
  }]);
  assert.deepStrictEqual(
    Array.from(run("state.pendingWorkspaceSymbols.items.map((item) => item.name)")),
    ["WorkspaceThing", "WorkspaceThingFactory"]
  );
  assert.strictEqual(run("state.pendingWorkspaceSymbols.items[0].path"), "main.eng");
  assert.strictEqual(run('workspaceSymbolRelativePath("C:/Else/outside.eng")'), "");
  run(`
    state.pendingWorkspaceSymbols = null;
    state.root = "";
  `);
  workspaceSymbolPayload = null;
}

async function closedWorkspaceSymbolSearchRejectsLateResults() {
  let resolveWorkspaceSymbols;
  workspaceSymbolPromise = new Promise((resolve) => {
    resolveWorkspaceSymbols = resolve;
  });
  run(`
    state.root = "C:/Repo";
    state.currentPath = "main.eng";
    state.source = "value = 1";
    state.dirty = false;
    state.tabs = [{ path: "main.eng", source: state.source, dirty: false }];
    state.pendingWorkspaceSymbols = {
      busy: true,
      error: "",
      items: [],
      query: "Late",
      revision: 0,
      selectedIndex: 0
    };
  `);
  const request = run("requestWorkspaceSymbols(state.pendingWorkspaceSymbols)");
  run(`
    state.pendingWorkspaceSymbols = null;
    workspaceSymbolRequestRevision += 1;
  `);
  resolveWorkspaceSymbols({
    format: "eng-lsp-snapshot-v1",
    symbols: []
  });
  assert.strictEqual(await request, false);
  workspaceSymbolPromise = null;
  run('state.root = ""');
}

async function workspaceSymbolNavigationSelectsUtf16Range() {
  run(`
    state.pendingWorkspaceSymbols = {
      items: [{
        absolutePath: "C:/Repo/lib.eng",
        detail: "schema",
        kind: 5,
        name: "WorkspaceThing",
        path: "lib.eng",
        range: {
          start: { line: 0, character: 2 },
          end: { line: 0, character: 16 }
        },
        uri: "file:///C:/Repo/lib.eng"
      }],
      selectedIndex: 0
    };
    globalThis.workspaceSymbolEditor = {
      value: "\uD83D\uDE00WorkspaceThing",
      selectionStart: 0,
      selectionEnd: 0,
      scrollTop: 0,
      focus() {}
    };
    globalThis.workspaceSymbolOpenedPath = "";
    globalThis.realWorkspaceSymbolById = byId;
    globalThis.realCloseWorkspaceSymbolSearch = closeWorkspaceSymbolSearch;
    globalThis.realWorkspaceSymbolOpenDefinitionTarget = openDefinitionTarget;
    byId = (id) => id === "editor" ? globalThis.workspaceSymbolEditor : null;
    closeWorkspaceSymbolSearch = () => {
      state.pendingWorkspaceSymbols = null;
    };
    openDefinitionTarget = async (path) => {
      globalThis.workspaceSymbolOpenedPath = path;
      return true;
    };
  `);

  assert.strictEqual(await run("openWorkspaceSymbolItem(0)"), true);
  assert.strictEqual(run("globalThis.workspaceSymbolOpenedPath"), "C:/Repo/lib.eng");
  assert.strictEqual(run("globalThis.workspaceSymbolEditor.selectionStart"), 2);
  assert.strictEqual(run("globalThis.workspaceSymbolEditor.selectionEnd"), 16);
  run(`
    byId = globalThis.realWorkspaceSymbolById;
    closeWorkspaceSymbolSearch = globalThis.realCloseWorkspaceSymbolSearch;
    openDefinitionTarget = globalThis.realWorkspaceSymbolOpenDefinitionTarget;
  `);
}

function documentHighlightShortcutUsesCurrentAction() {
  run(`
    state.pendingTabClose = null;
    state.pendingWindowClose = false;
    globalThis.documentHighlightShortcutCalls = 0;
    globalThis.realShowDocumentHighlightsAtCaret = showDocumentHighlightsAtCaret;
    showDocumentHighlightsAtCaret = async () => {
      globalThis.documentHighlightShortcutCalls += 1;
    };
    globalThis.documentHighlightShortcutEvent = {
      altKey: false,
      ctrlKey: false,
      key: "F12",
      metaKey: false,
      prevented: false,
      shiftKey: true,
      preventDefault() {
        this.prevented = true;
      }
    };
    handleGlobalKeyDown(globalThis.documentHighlightShortcutEvent);
    showDocumentHighlightsAtCaret = globalThis.realShowDocumentHighlightsAtCaret;
  `);

  assert.strictEqual(run("globalThis.documentHighlightShortcutEvent.prevented"), true);
  assert.strictEqual(run("globalThis.documentHighlightShortcutCalls"), 1);
}

function quickFixShortcutUsesCurrentProblemAction() {
  run(`
    state.pendingQuickFix = null;
    state.pendingRename = null;
    state.pendingTabClose = null;
    state.pendingWindowClose = false;
    globalThis.quickFixShortcutCalls = 0;
    globalThis.realRequestCursorProblemQuickFix = requestCursorProblemQuickFix;
    requestCursorProblemQuickFix = async () => {
      globalThis.quickFixShortcutCalls += 1;
    };
    globalThis.quickFixShortcutEvent = {
      altKey: false,
      ctrlKey: true,
      key: ".",
      metaKey: false,
      prevented: false,
      shiftKey: false,
      preventDefault() {
        this.prevented = true;
      }
    };
    handleGlobalKeyDown(globalThis.quickFixShortcutEvent);
    requestCursorProblemQuickFix = globalThis.realRequestCursorProblemQuickFix;
  `);

  assert.strictEqual(run("globalThis.quickFixShortcutEvent.prevented"), true);
  assert.strictEqual(run("globalThis.quickFixShortcutCalls"), 1);
}

function problemNavigationUsesFilteredUtf16RangesAndWraps() {
  run(`
    globalThis.realByIdForProblemNavigation = byId;
    globalThis.realRenderForProblemNavigation = render;
    globalThis.realScheduleLiveCheckForProblemNavigation = scheduleLiveCheck;
    globalThis.realProblemQuerySelector = document.querySelector;
    globalThis.realProblemQuerySelectorAll = document.querySelectorAll;
    state.currentPath = "main.eng";
    state.source = "ok\\nwarn \\u{1F600}value\\nerror target\\n";
    state.highlightSource = state.source;
    state.problemSeverity = "all";
    state.problemCode = "all";
    state.problemQuery = "";
    state.bottomTab = "terminal";
    state.check = {
      status: "checked",
      diagnostics: [
        { line: 3, column: 7, startCharacter: 6, endCharacter: 12, severity: "error", code: "E-TARGET", message: "target" },
        { line: 2, column: 8, startCharacter: 7, endCharacter: 12, severity: "warning", code: "W-VALUE", message: "value" }
      ],
      documentSymbols: [],
      hovers: [],
      semanticTokens: { legend: {}, tokens: [] },
      symbols: []
    };
    state.tabs = [{ path: "main.eng", source: state.source, savedSource: state.source, dirty: false }];
    globalThis.problemNavigationEditor = {
      value: state.source,
      selectionStart: 0,
      selectionEnd: 0,
      scrollTop: 0,
      focused: false,
      focus() { this.focused = true; }
    };
    globalThis.problemNavigationRows = [
      { active: false, dataset: { problemIndex: "0" }, scrolled: false },
      { active: false, dataset: { problemIndex: "1" }, scrolled: false }
    ];
    globalThis.problemNavigationRows.forEach((row) => {
      row.classList = {
        add(name) { if (name === "active") row.active = true; },
        remove(name) { if (name === "active") row.active = false; }
      };
      row.scrollIntoView = () => { row.scrolled = true; };
    });
    document.querySelectorAll = (selector) => selector === ".problem-row.active"
      ? globalThis.problemNavigationRows.filter((row) => row.active)
      : [];
    document.querySelector = (selector) => {
      if (selector === ".problem-row.active") {
        return globalThis.problemNavigationRows.find((row) => row.active) || null;
      }
      const match = String(selector).match(/data-problem-index="(\\d+)"/);
      return match ? globalThis.problemNavigationRows[Number(match[1])] : null;
    };
    byId = (id) => id === "editor" ? globalThis.problemNavigationEditor : null;
    globalThis.problemNavigationRenderCalls = 0;
    render = () => { globalThis.problemNavigationRenderCalls += 1; };
    globalThis.problemNavigationCheckCalls = 0;
    scheduleLiveCheck = () => { globalThis.problemNavigationCheckCalls += 1; };
  `);

  assert.strictEqual(run("navigateProblem(1)"), true);
  assert.strictEqual(
    run("globalThis.problemNavigationEditor.value.slice(globalThis.problemNavigationEditor.selectionStart, globalThis.problemNavigationEditor.selectionEnd)"),
    "value"
  );
  assert.strictEqual(run("globalThis.problemNavigationRows[1].active"), true);
  assert.strictEqual(run("state.status"), "Problem 1 of 2: W-VALUE at L2");

  assert.strictEqual(run("navigateProblem(1)"), true);
  assert.strictEqual(
    run("globalThis.problemNavigationEditor.value.slice(globalThis.problemNavigationEditor.selectionStart, globalThis.problemNavigationEditor.selectionEnd)"),
    "target"
  );
  assert.strictEqual(run("globalThis.problemNavigationRows[0].active"), true);
  assert.strictEqual(run("state.status"), "Problem 2 of 2: E-TARGET at L3");

  assert.strictEqual(run("navigateProblem(1)"), true);
  assert.strictEqual(
    run("globalThis.problemNavigationEditor.value.slice(globalThis.problemNavigationEditor.selectionStart, globalThis.problemNavigationEditor.selectionEnd)"),
    "value"
  );
  assert.strictEqual(run("navigateProblem(-1)"), true);
  assert.strictEqual(
    run("globalThis.problemNavigationEditor.value.slice(globalThis.problemNavigationEditor.selectionStart, globalThis.problemNavigationEditor.selectionEnd)"),
    "target"
  );

  run(`
    state.problemSeverity = "warning";
    globalThis.problemNavigationEditor.selectionStart = 0;
    globalThis.problemNavigationEditor.selectionEnd = 0;
  `);
  assert.strictEqual(run("navigateProblem(1)"), true);
  assert.strictEqual(run("state.status"), "Problem 1 of 1: W-VALUE at L2");
  assert.ok(run('renderProblems().includes("previousProblemBtn")'));
  assert.ok(run('renderProblems().includes("nextProblemBtn")'));

  run(`
    state.highlightSource = null;
    state.check.status = "idle";
  `);
  assert.strictEqual(run("navigateProblem(1)"), false);
  assert.strictEqual(run("globalThis.problemNavigationCheckCalls"), 1);
  assert.strictEqual(run("state.status"), "Analyze the current buffer before navigating problems");

  run(`
    byId = globalThis.realByIdForProblemNavigation;
    render = globalThis.realRenderForProblemNavigation;
    scheduleLiveCheck = globalThis.realScheduleLiveCheckForProblemNavigation;
    document.querySelector = globalThis.realProblemQuerySelector;
    document.querySelectorAll = globalThis.realProblemQuerySelectorAll;
    state.currentPath = "";
    state.source = "";
    state.highlightSource = null;
    state.tabs = [];
    state.problemSeverity = "all";
    state.problemCode = "all";
    state.problemQuery = "";
    state.bottomTab = "terminal";
    state.check = { diagnostics: [], symbols: [], status: "", semanticTokens: { legend: {}, tokens: [] }, hovers: [], documentSymbols: [] };
  `);
}

function problemNavigationShortcutUsesBothDirections() {
  run(`
    state.pendingQuickFix = null;
    state.pendingRename = null;
    state.pendingWorkspaceSymbols = null;
    state.pendingTabClose = null;
    state.pendingWindowClose = false;
    globalThis.problemShortcutDirections = [];
    globalThis.realNavigateProblem = navigateProblem;
    navigateProblem = (direction) => {
      globalThis.problemShortcutDirections.push(direction);
      return true;
    };
    globalThis.nextProblemShortcutEvent = {
      altKey: false,
      ctrlKey: false,
      key: "F8",
      metaKey: false,
      prevented: false,
      shiftKey: false,
      preventDefault() { this.prevented = true; }
    };
    globalThis.previousProblemShortcutEvent = {
      ...globalThis.nextProblemShortcutEvent,
      prevented: false,
      shiftKey: true
    };
    handleGlobalKeyDown(globalThis.nextProblemShortcutEvent);
    handleGlobalKeyDown(globalThis.previousProblemShortcutEvent);
    navigateProblem = globalThis.realNavigateProblem;
  `);

  assert.strictEqual(run("globalThis.nextProblemShortcutEvent.prevented"), true);
  assert.strictEqual(run("globalThis.previousProblemShortcutEvent.prevented"), true);
  assert.deepStrictEqual(Array.from(run("globalThis.problemShortcutDirections")), [1, -1]);
}

function renameShortcutUsesCurrentAction() {
  run(`
    state.pendingRename = null;
    state.pendingTabClose = null;
    state.pendingWindowClose = false;
    globalThis.renameShortcutCalls = 0;
    globalThis.realStartSemanticRename = startSemanticRename;
    startSemanticRename = async () => {
      globalThis.renameShortcutCalls += 1;
    };
    globalThis.renameShortcutEvent = {
      altKey: false,
      ctrlKey: false,
      key: "F2",
      metaKey: false,
      prevented: false,
      shiftKey: false,
      preventDefault() {
        this.prevented = true;
      }
    };
    handleGlobalKeyDown(globalThis.renameShortcutEvent);
    startSemanticRename = globalThis.realStartSemanticRename;
  `);

  assert.strictEqual(run("globalThis.renameShortcutEvent.prevented"), true);
  assert.strictEqual(run("globalThis.renameShortcutCalls"), 1);
}

function busyRenameCanBeCancelledSafely() {
  run(`
    globalThis.renameRevisionBeforeCancel = renameRequestRevision;
    state.pendingRename = { busy: true };
    cancelSemanticRename();
  `);
  assert.strictEqual(run("state.pendingRename"), null);
  assert.strictEqual(
    run("renameRequestRevision"),
    run("globalThis.renameRevisionBeforeCancel + 1")
  );
}

async function renamePreparationAllowsOtherDirtyEngLangTabs() {
  invokeCalls.length = 0;
  prepareRenamePayload = {
    range: {
      start: { line: 0, character: 8 },
      end: { line: 0, character: 19 }
    },
    placeholder: "SHARED_RATE"
  };
  run(`
    state.pendingRename = null;
    state.currentPath = "main.eng";
    state.source = "value = SHARED_RATE";
    state.dirty = true;
    state.tabs = [
      { path: "main.eng", source: state.source, dirty: true },
      { path: "other.eng", source: "other = SHARED_RATE", dirty: true }
    ];
    globalThis.realEditorDefinitionRequest = editorDefinitionRequest;
    editorDefinitionRequest = () => ({
      path: "main.eng",
      source: state.source,
      line: 0,
      character: 10
    });
    globalThis.realOpenSemanticRenameDialog = openSemanticRenameDialog;
    globalThis.realSelectEditorUtf16Range = selectEditorUtf16Range;
    selectEditorUtf16Range = () => true;
    openSemanticRenameDialog = (pending) => {
      globalThis.preparedRename = pending;
      state.pendingRename = pending;
    };
  `);
  assert.strictEqual(await run("startSemanticRename()"), true);
  const preparationCall = invokeCalls.find((item) => item.command === "ide_prepare_rename");
  assert.deepStrictEqual(JSON.parse(JSON.stringify(preparationCall.args)), {
    path: "main.eng",
    source: "value = SHARED_RATE",
    line: 0,
    character: 10,
    documents: [{ path: "other.eng", source: "other = SHARED_RATE" }]
  });
  assert.deepStrictEqual(
    JSON.parse(run("JSON.stringify(globalThis.preparedRename.documents)")),
    [{ path: "other.eng", source: "other = SHARED_RATE" }]
  );
  run(`
    editorDefinitionRequest = globalThis.realEditorDefinitionRequest;
    openSemanticRenameDialog = globalThis.realOpenSemanticRenameDialog;
    selectEditorUtf16Range = globalThis.realSelectEditorUtf16Range;
    state.pendingRename = null;
  `);
  prepareRenamePayload = null;
}

function semanticTokenAndReferenceRangesUseUtf16Coordinates() {
  assert.strictEqual(
    run(`JSON.stringify(semanticTokenRange("\uD83D\uDE00alpha", { line: 0, start: 2, length: 5 }))`),
    '{"start":2,"end":7,"token":{"line":0,"start":2,"length":5}}'
  );
  assert.strictEqual(
    run(`"\uD83D\uDE00alpha".slice(...(() => {
      const range = semanticTokenRange("\uD83D\uDE00alpha", { line: 0, start: 2, length: 5 });
      return [range.start, range.end];
    })())`),
    "alpha"
  );
  run(`
    state.currentPath = "unicode.eng";
    state.source = "\uD83D\uDE00alpha";
    state.highlightSource = state.source;
    state.documentHighlights = {
      path: state.currentPath,
      source: state.source,
      items: [{
        range: {
          start: { line: 0, character: 2 },
          end: { line: 0, character: 7 }
        },
        kind: 2
      }]
    };
  `);
  assert.strictEqual(
    run("documentHighlightKindForToken({ line: 0, start: 2, length: 5 }, 0)"),
    2
  );
  run("state.documentHighlights.source = 'stale'");
  assert.strictEqual(run("currentDocumentHighlights().length"), 0);
}

function workspaceReferencesTrackAllDirtyOpenBuffers() {
  run(`
    state.root = "C:/Repo";
    state.currentPath = "main.eng";
    state.source = "shared = SHARED_GAIN";
    state.tabs = [
      { path: "main.eng", source: state.source, dirty: true },
      { path: "other.eng", source: "other = SHARED_GAIN", dirty: true }
    ];
    state.documentHighlights = {
      path: "main.eng",
      source: state.source,
      items: [{
        range: {
          start: { line: 0, character: 9 },
          end: { line: 0, character: 20 }
        },
        kind: 2
      }]
    };
    state.workspaceReferences = {
      path: "main.eng",
      source: state.source,
      documents: [{ path: "other.eng", source: "other = SHARED_GAIN" }],
      label: "SHARED_GAIN",
      notice: "",
      items: [
        {
          uri: "file:///C:/Repo/main.eng",
          range: {
            start: { line: 0, character: 9 },
            end: { line: 0, character: 20 }
          }
        },
        {
          uri: "file:///C:/Repo/module.eng",
          range: {
            start: { line: 0, character: 6 },
            end: { line: 0, character: 17 }
          }
        }
      ]
    };
  `);

  assert.deepStrictEqual(
    JSON.parse(run("JSON.stringify(dirtyWorkspaceDocuments('main.eng'))")),
    [{ path: "other.eng", source: "other = SHARED_GAIN" }]
  );
  assert.strictEqual(run("currentWorkspaceReferences().length"), 2);
  assert.strictEqual(
    run("documentHighlightForWorkspaceReference(state.workspaceReferences.items[0]).kind"),
    2
  );
  assert.strictEqual(
    run("JSON.stringify(workspaceReferenceRange(state.workspaceReferences.items[1]))"),
    '{"start":{"line":0,"character":6},"end":{"line":0,"character":17}}'
  );
  assert.strictEqual(
    run("workspaceReferenceRange({ range: { start: { line: 1, character: 4 }, end: { line: 1, character: 4 } } })"),
    null
  );

  run(`
    state.currentPath = "other.eng";
    state.source = state.tabs[1].source;
  `);
  assert.strictEqual(run("currentWorkspaceReferences().length"), 2);
  run("state.tabs[0].source = 'changed after lookup'");
  assert.strictEqual(run("currentWorkspaceReferences().length"), 0);
  run("clearReferenceResults(); state.root = ''");
  assert.strictEqual(run("state.workspaceReferences.items.length"), 0);
}

async function workspaceRenameStagesVerifiedUtf16Buffers() {
  invokeCalls.length = 0;
  openFileSources.clear();
  run(`
    state.root = "C:/Repo";
    state.currentPath = "main.eng";
    state.source = "\uD83D\uDE00 SHARED_RATE\\nagain = SHARED_RATE\\n";
    state.dirty = true;
    state.tabs = [
      { path: "main.eng", source: state.source, dirty: true },
      { path: "module.eng", source: "const SHARED_RATE: Ratio = 0.8\\n", dirty: true },
      { path: "notes.csv", source: "changed", dirty: true }
    ];
    globalThis.renamePending = {
      request: {
        path: "main.eng",
        source: state.source,
        line: 0,
        character: 3
      },
      range: {
        start: { line: 0, character: 3 },
        end: { line: 0, character: 14 }
      },
      placeholder: "SHARED_RATE"
    };
    globalThis.renamePayload = {
      changes: {
        "file:///C:/Repo/main.eng": [
          {
            range: {
              start: { line: 0, character: 3 },
              end: { line: 0, character: 14 }
            },
            newText: "RENAMED_RATE"
          },
          {
            range: {
              start: { line: 1, character: 8 },
              end: { line: 1, character: 19 }
            },
            newText: "RENAMED_RATE"
          }
        ],
        "file:///C:/Repo/module.eng": [{
          range: {
            start: { line: 0, character: 6 },
            end: { line: 0, character: 17 }
          },
          newText: "RENAMED_RATE"
        }]
      }
    };
    globalThis.renameDocuments = [{
      path: "module.eng",
      source: "const SHARED_RATE: Ratio = 0.8\\n"
    }];
  `);

  assert.strictEqual(run("workspaceDocumentsAreCurrent(globalThis.renameDocuments, 'main.eng')"), true);
  assert.strictEqual(run("sourceUtf16Offset(state.source, { line: 0, character: 3 })"), 3);
  await run(`(async () => {
    globalThis.stagedRename = await stageWorkspaceRename(
      globalThis.renamePending,
      globalThis.renamePayload,
      "RENAMED_RATE",
      globalThis.renameDocuments
    );
  })()`);
  assert.strictEqual(run("globalThis.stagedRename.editCount"), 3);
  assert.strictEqual(run("globalThis.stagedRename.focus.start"), 3);
  assert.strictEqual(run("globalThis.stagedRename.focus.end"), 15);
  assert.deepStrictEqual(invokeCalls, []);

  const originalSource = run("state.source");
  const originalTabs = run("JSON.stringify(state.tabs)");
  await assert.rejects(
    run(`stageWorkspaceRename(
      globalThis.renamePending,
      globalThis.renamePayload,
      "RENAMED_RATE",
      [{ path: "module.eng", source: "const CHANGED_RATE: Ratio = 0.8\\n" }]
    )`),
    /changed before all edits could be verified/
  );
  assert.strictEqual(run("state.source"), originalSource);
  assert.strictEqual(run("JSON.stringify(state.tabs)"), originalTabs);

  run("commitWorkspaceRename(globalThis.renamePending, globalThis.stagedRename, globalThis.renameDocuments)");
  assert.strictEqual(
    run("state.source"),
    "\uD83D\uDE00 RENAMED_RATE\nagain = RENAMED_RATE\n"
  );
  assert.strictEqual(
    run("state.tabs.find((tab) => tab.path === 'module.eng').source"),
    "const RENAMED_RATE: Ratio = 0.8\n"
  );
  assert.deepStrictEqual(
    Array.from(run("state.tabs.filter((tab) => /\\.eng$/i.test(tab.path)).map((tab) => tab.dirty)")),
    [true, true]
  );

  assert.throws(
    () => run(`workspaceRenamePlan({ changes: {
      "file:///C:/outside/other.eng": [{
        range: { start: { line: 0, character: 0 }, end: { line: 0, character: 4 } },
        newText: "next"
      }]
    } }, "next", "main.eng")`),
    /outside the EngLang workspace/
  );
  assert.throws(
    () => run(`workspaceRenamePlan({ changes: {
      "file:///C:/Repo/module.eng": [{
        range: { start: { line: 0, character: 6 }, end: { line: 0, character: 17 } },
        newText: "next"
      }]
    } }, "next", "main.eng")`),
    /did not edit the selected EngLang file/
  );
  assert.throws(
    () => run(`applyWorkspaceTextEdits("aaaa", [
      { range: { start: { line: 0, character: 0 }, end: { line: 0, character: 2 } }, newText: "next" },
      { range: { start: { line: 0, character: 1 }, end: { line: 0, character: 3 } }, newText: "next" }
    ], "aa")`),
    /overlapping source edits/
  );
  assert.throws(
    () => run(`applyWorkspaceTextEdits("old", [{
      range: { start: { line: 0, character: 0 }, end: { line: 0, character: 3 } },
      newText: "next"
    }], "name")`),
    /changed before all edits could be verified/
  );
  openFileSources.clear();
  run("state.root = ''; state.pendingRename = null");
}

async function compilerQuickFixAppliesUnsavedUtf16Edits() {
  invokeCalls.length = 0;
  const source = "\uD83D\uDE00 power = 10\n";
  codeActionPayload = {
    format: "eng-lsp-snapshot-v1",
    uri: "file:///C:/Repo/main.eng",
    actions: [{
      title: "Annotate power and add its unit",
      kind: "quickfix",
      isPreferred: true,
      diagnostics: [{
        code: "W-QTY-AMBIG-001",
        range: {
          start: { line: 0, character: 3 },
          end: { line: 0, character: 8 }
        }
      }],
      edit: {
        changes: {
          "file:///C:/Repo/main.eng": [
            {
              range: {
                start: { line: 0, character: 8 },
                end: { line: 0, character: 8 }
              },
              newText: ": HeatRate [kW]"
            },
            {
              range: {
                start: { line: 0, character: 11 },
                end: { line: 0, character: 13 }
              },
              newText: "12 kW"
            }
          ]
        }
      }
    }]
  };
  run(`
    state.root = "C:/Repo";
    state.currentPath = "main.eng";
    state.source = ${JSON.stringify(source)};
    state.highlightSource = state.source;
    state.dirty = true;
    state.tabs = [{ path: "main.eng", source: state.source, dirty: true }];
    state.pendingQuickFix = null;
    globalThis.quickFixDiagnostic = {
      line: 1,
      startCharacter: 3,
      endCharacter: 8,
      code: "W-QTY-AMBIG-001",
      message: "Add an explicit quantity annotation."
    };
  `);

  assert.deepStrictEqual(
    JSON.parse(run("JSON.stringify(problemDiagnosticLspRange(globalThis.quickFixDiagnostic))")),
    {
      start: { line: 0, character: 3 },
      end: { line: 0, character: 8 }
    }
  );
  assert.strictEqual(await run("requestProblemQuickFix(globalThis.quickFixDiagnostic)"), true);
  const request = invokeCalls.find((item) => item.command === "ide_code_actions");
  assert.deepStrictEqual(JSON.parse(JSON.stringify(request.args)), { path: "main.eng", source });
  assert.strictEqual(run("state.source"), "\uD83D\uDE00 power: HeatRate [kW] = 12 kW\n");
  assert.strictEqual(run("state.tabs[0].source"), run("state.source"));
  assert.strictEqual(run("state.dirty"), true);
  assert.strictEqual(run("state.tabs[0].dirty"), true);

  const changedSource = run("state.source");
  const changedTabs = run("JSON.stringify(state.tabs)");
  assert.throws(
    () => run(`applyProblemQuickFix({
      title: "Stale fix",
      edits: [{
        range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } },
        newText: "x"
      }]
    }, { path: "main.eng", source: ${JSON.stringify(source)} })`),
    /current buffer changed/
  );
  assert.strictEqual(run("state.source"), changedSource);
  assert.strictEqual(run("JSON.stringify(state.tabs)"), changedTabs);

  assert.throws(
    () => run(`codeActionPlan({
      title: "Outside edit",
      kind: "quickfix",
      edit: { changes: {
        "file:///C:/Repo/other.eng": [{
          range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } },
          newText: "x"
        }]
      } }
    }, { path: "main.eng", source: "value" })`),
    /different file/
  );
  assert.throws(
    () => run(`applyCodeActionTextEdits("abcd", [
      { range: { start: { line: 0, character: 0 }, end: { line: 0, character: 2 } }, newText: "x" },
      { range: { start: { line: 0, character: 1 }, end: { line: 0, character: 3 } }, newText: "y" }
    ])`),
    /overlapping source edits/
  );
  assert.throws(
    () => run(`applyCodeActionTextEdits("abcd", [
      { range: { start: { line: 0, character: 2 }, end: { line: 0, character: 2 } }, newText: "x" },
      { range: { start: { line: 0, character: 2 }, end: { line: 0, character: 2 } }, newText: "y" }
    ])`),
    /overlapping source edits/
  );
  run("invalidateLiveCheck()");
  codeActionPayload = null;
  run("state.root = ''; state.pendingQuickFix = null");
}

function documentSymbolsNormalizeAndFilter() {
  const flattened = run(`JSON.stringify(flattenDocumentSymbols(normalizeCheck({
    document_symbols: [{
      name: "RoomThermal",
      detail: "system",
      kind: 5,
      selectionRange: {
        start: { line: 3, character: 7 },
        end: { line: 3, character: 18 }
      },
      children: [{
        name: "T_room",
        detail: "state",
        kind: 8,
        selectionRange: {
          start: { line: 4, character: 10 },
          end: { line: 4, character: 16 }
        },
        children: []
      }]
    }]
  }).documentSymbols).map((item) => ({
    name: item.name,
    detail: item.detail,
    kind: item.kind,
    depth: item.depth,
    line: item.line,
    character: item.character,
    endCharacter: item.endCharacter
  })))`);
  assert.strictEqual(
    flattened,
    '[{"name":"RoomThermal","detail":"system","kind":5,"depth":0,"line":3,"character":7,"endCharacter":18},{"name":"T_room","detail":"state","kind":8,"depth":1,"line":4,"character":10,"endCharacter":16}]'
  );
  assert.strictEqual(
    run(`JSON.stringify(filteredOutlineItems(flattenDocumentSymbols([{
      name: "RoomThermal",
      detail: "system",
      kind: 5,
      selectionRange: { start: { line: 3, character: 7 }, end: { line: 3, character: 18 } },
      children: [{
        name: "T_room",
        detail: "state",
        kind: 8,
        selectionRange: { start: { line: 4, character: 10 }, end: { line: 4, character: 16 } },
        children: []
      }]
    }]), "state").map((item) => item.name))`),
    '["T_room"]'
  );
}

function documentBreadcrumbsTrackNestedSymbolsAndFreshness() {
  run(`
    state.currentPath = "models/main.eng";
    state.source = "checked source";
    state.highlightSource = "checked source";
    state.check.documentSymbols = [{
      name: "RoomThermal",
      detail: "system",
      kind: 5,
      range: { start: { line: 1, character: 0 }, end: { line: 8, character: 1 } },
      selectionRange: { start: { line: 1, character: 7 }, end: { line: 1, character: 18 } },
      children: [{
        name: "balance",
        detail: "function",
        kind: 12,
        range: { start: { line: 3, character: 2 }, end: { line: 6, character: 3 } },
        selectionRange: { start: { line: 3, character: 5 }, end: { line: 3, character: 12 } },
        children: [{
          name: "source",
          detail: "parameter",
          kind: 26,
          range: { start: { line: 3, character: 2 }, end: { line: 6, character: 3 } },
          selectionRange: { start: { line: 3, character: 13 }, end: { line: 3, character: 19 } },
          children: []
        }, {
          name: "T_room",
          detail: "local",
          kind: 13,
          range: { start: { line: 4, character: 4 }, end: { line: 4, character: 16 } },
          selectionRange: { start: { line: 4, character: 4 }, end: { line: 4, character: 10 } },
          children: []
        }]
      }]
    }];
  `);

  assert.strictEqual(
    run("JSON.stringify(documentSymbolBreadcrumbPath(state.check.documentSymbols, { line: 4, column: 6 }).map((item) => item.name))"),
    '["RoomThermal","balance","T_room"]'
  );
  assert.strictEqual(
    run("JSON.stringify(documentSymbolBreadcrumbPath(state.check.documentSymbols, { line: 7, character: 0 }).map((item) => item.name))"),
    '["RoomThermal"]'
  );
  assert.strictEqual(
    run("JSON.stringify(documentSymbolBreadcrumbPath(state.check.documentSymbols, { line: 5, character: 4 }).map((item) => item.name))"),
    '["RoomThermal","balance"]'
  );
  assert.strictEqual(
    run("JSON.stringify(documentSymbolBreadcrumbPath(state.check.documentSymbols, { line: 3, character: 15 }).map((item) => item.name))"),
    '["RoomThermal","balance","source"]'
  );
  assert.strictEqual(
    run("JSON.stringify(documentSymbolBreadcrumbPath(state.check.documentSymbols, { line: 10, character: 0 }))"),
    "[]"
  );
  const markup = run("renderEditorBreadcrumbs({ line: 4, character: 6 })");
  assert.match(markup, /main\.eng/);
  assert.match(markup, /RoomThermal/);
  assert.match(markup, /balance/);
  assert.match(markup, /T_room/);
  assert.match(markup, /aria-current="location">T_room/);

  run('state.source = "changed source"');
  const staleMarkup = run("renderEditorBreadcrumbs({ line: 4, character: 6 })");
  assert.match(staleMarkup, /main\.eng/);
  assert.doesNotMatch(staleMarkup, /RoomThermal|balance|T_room/);
  run(`
    state.currentPath = "";
    state.source = "";
    state.highlightSource = null;
    state.check.documentSymbols = [];
  `);
}

function documentBreadcrumbNavigationUsesUtf16Coordinates() {
  run(`
    globalThis.realByIdForBreadcrumb = byId;
    globalThis.breadcrumbEditor = {
      value: "head\\n  \\u{1F600}alpha = 1\\nlast",
      selectionStart: 0,
      selectionEnd: 0,
      scrollTop: 40,
      focused: false,
      focus() {
        this.focused = true;
      }
    };
    byId = (id) => id === "editor" ? globalThis.breadcrumbEditor : null;
    globalThis.breadcrumbNavigated = navigateEditorBreadcrumb({
      dataset: {
        editorBreadcrumbLine: "1",
        editorBreadcrumbCharacter: "4",
        editorBreadcrumbEndLine: "1",
        editorBreadcrumbEndCharacter: "9",
        editorBreadcrumbName: "alpha"
      }
    });
  `);

  assert.strictEqual(run("globalThis.breadcrumbNavigated"), true);
  assert.strictEqual(run("globalThis.breadcrumbEditor.value.slice(globalThis.breadcrumbEditor.selectionStart, globalThis.breadcrumbEditor.selectionEnd)"), "alpha");
  assert.strictEqual(run("globalThis.breadcrumbEditor.focused"), true);
  assert.strictEqual(run("state.status"), "Breadcrumb: alpha");
  run("byId = globalThis.realByIdForBreadcrumb");
}

function editorViewStatePersistsAcrossRendersAndTabs() {
  run(`
    globalThis.realByIdForEditorView = byId;
    state.currentPath = "main.eng";
    state.source = "head\\n  \\u{1F600}alpha = 1\\nlast";
    state.tabs = [
      { path: "main.eng", source: state.source, dirty: false },
      {
        path: "short.eng",
        source: "short",
        dirty: false,
        selectionStart: 99,
        selectionEnd: 120,
        selectionDirection: "sideways",
        scrollTop: 33,
        scrollLeft: 17
      }
    ];
    globalThis.editorViewControl = {
      value: state.source,
      selectionStart: 9,
      selectionEnd: 14,
      selectionDirection: "backward",
      scrollTop: 240,
      scrollLeft: 31
    };
    byId = (id) => id === "editor" ? globalThis.editorViewControl : null;
    globalThis.editorViewRemembered = rememberCurrentEditorView();
  `);

  assert.strictEqual(run("globalThis.editorViewRemembered"), true);
  assert.deepStrictEqual(
    JSON.parse(run("JSON.stringify({ selectionStart: state.tabs[0].selectionStart, selectionEnd: state.tabs[0].selectionEnd, selectionDirection: state.tabs[0].selectionDirection, scrollTop: state.tabs[0].scrollTop, scrollLeft: state.tabs[0].scrollLeft })")),
    { selectionStart: 9, selectionEnd: 14, selectionDirection: "backward", scrollTop: 240, scrollLeft: 31 }
  );

  run(`
    globalThis.editorViewControl = {
      value: state.source,
      selectionStart: 0,
      selectionEnd: 0,
      selectionDirection: "none",
      scrollTop: 0,
      scrollLeft: 0,
      setSelectionRange(start, end, direction) {
        this.selectionStart = start;
        this.selectionEnd = end;
        this.selectionDirection = direction;
      }
    };
    globalThis.editorViewRestored = restoreCurrentEditorView();
  `);
  assert.strictEqual(run("globalThis.editorViewRestored"), true);
  assert.deepStrictEqual(
    Array.from(run("[globalThis.editorViewControl.selectionStart, globalThis.editorViewControl.selectionEnd, globalThis.editorViewControl.selectionDirection, globalThis.editorViewControl.scrollTop, globalThis.editorViewControl.scrollLeft]")),
    [9, 14, "backward", 240, 31]
  );

  run(`
    state.currentPath = "short.eng";
    state.source = "short";
    globalThis.editorViewControl.value = state.source;
    globalThis.editorViewClamped = restoreCurrentEditorView();
  `);
  assert.strictEqual(run("globalThis.editorViewClamped"), true);
  assert.deepStrictEqual(
    Array.from(run("[globalThis.editorViewControl.selectionStart, globalThis.editorViewControl.selectionEnd, globalThis.editorViewControl.selectionDirection, globalThis.editorViewControl.scrollTop, globalThis.editorViewControl.scrollLeft]")),
    [5, 5, "none", 33, 17]
  );
  run(`
    byId = globalThis.realByIdForEditorView;
    state.currentPath = "";
    state.source = "";
    state.tabs = [];
  `);
}

function editorLineNumbersTrackSourceAndScroll() {
  assert.strictEqual(run('renderEditorLineNumbers("")'), "1");
  assert.strictEqual(run('renderEditorLineNumbers("one\\ntwo\\n")'), "1\n2\n3");
  assert.strictEqual(run('editorGutterWidth("x\\n".repeat(98) + "x")'), 38);
  assert.strictEqual(run('editorGutterWidth("x\\n".repeat(99) + "x")'), 42);
  assert.strictEqual(run('editorGutterWidth("x\\n".repeat(9999) + "x")'), 58);

  run(`
    globalThis.realByIdForLineNumbers = byId;
    state.source = "one\\ntwo\\nthree";
    globalThis.lineNumberEditor = {
      scrollTop: 73,
      scrollLeft: 29
    };
    globalThis.lineNumberHighlight = {
      scrollTop: 0,
      scrollLeft: 0
    };
    globalThis.lineNumberGutterLines = {
      textContent: "",
      style: { top: "", left: "0px" }
    };
    globalThis.lineNumberShell = {
      style: {
        values: {},
        setProperty(name, value) {
          this.values[name] = value;
        }
      }
    };
    byId = (id) => ({
      editor: globalThis.lineNumberEditor,
      editorHighlight: globalThis.lineNumberHighlight,
      editorGutterLines: globalThis.lineNumberGutterLines,
      editorShell: globalThis.lineNumberShell
    })[id] || null;
    updateEditorLineNumbers();
    syncEditorHighlightScroll();
    globalThis.lineNumberCaret = caretOverlayPosition({
      value: "ab\\ncd",
      selectionStart: 5,
      clientWidth: 900,
      clientHeight: 500,
      scrollLeft: 0,
      scrollTop: 0
    });
  `);

  assert.strictEqual(run("globalThis.lineNumberGutterLines.textContent"), "1\n2\n3");
  assert.strictEqual(run('globalThis.lineNumberShell.style.values["--editor-gutter-width"]'), "38px");
  assert.strictEqual(run("globalThis.lineNumberGutterLines.style.top"), "-73px");
  assert.strictEqual(run("globalThis.lineNumberGutterLines.style.left"), "0px");
  assert.deepStrictEqual(
    Array.from(run("[globalThis.lineNumberHighlight.scrollTop, globalThis.lineNumberHighlight.scrollLeft]")),
    [73, 29]
  );
  assert.deepStrictEqual(
    JSON.parse(run("JSON.stringify(globalThis.lineNumberCaret)")),
    { left: 68.4, top: 52 }
  );

  run(`
    state.source = Array.from({ length: 100 }, (_, index) => "line " + (index + 1)).join("\\n");
    updateEditorLineNumbers();
  `);
  assert.strictEqual(run('globalThis.lineNumberShell.style.values["--editor-gutter-width"]'), "42px");
  assert.strictEqual(run('globalThis.lineNumberGutterLines.textContent.split("\\n").at(-1)'), "100");
  run(`
    byId = globalThis.realByIdForLineNumbers;
    state.source = "";
  `);
}

function editorLocationNavigationUsesValidatedUtf16Coordinates() {
  assert.deepStrictEqual(
    JSON.parse(run('JSON.stringify(parseEditorLocation("2:3", "alpha\\n\\u{1F600}beta\\n"))')),
    { line: 2, column: 3, lineIndex: 1, columnIndex: 2, offset: 8 }
  );
  assert.deepStrictEqual(
    JSON.parse(run('JSON.stringify(parseEditorLocation("3", "alpha\\n\\u{1F600}beta\\n"))')),
    { line: 3, column: 1, lineIndex: 2, columnIndex: 0, offset: 13 }
  );
  assert.strictEqual(
    run('parseEditorLocation("2:8", "alpha\\n\\u{1F600}beta\\n").error'),
    "Column must be between 1 and 7 on line 2."
  );
  assert.strictEqual(
    run('parseEditorLocation("4", "alpha\\n\\u{1F600}beta\\n").error'),
    "Line must be between 1 and 3."
  );
  assert.match(run('parseEditorLocation("line two", "alpha\\nbeta").error'), /optionally followed by :column/);

  run(`
    globalThis.realByIdForEditorLocation = byId;
    state.currentPath = "main.eng";
    state.source = "alpha\\n\\u{1F600}beta\\nlast";
    state.highlightSource = null;
    state.tabs = [{ path: "main.eng", source: state.source, dirty: false }];
    state.pendingGoToLine = { path: state.currentPath, source: state.source };
    globalThis.editorLocationInput = { value: "2:3" };
    globalThis.editorLocationError = { textContent: "", hidden: true };
    globalThis.editorLocationBackdrop = {
      removed: false,
      remove() {
        this.removed = true;
      }
    };
    globalThis.editorLocationApp = { inert: true };
    globalThis.editorLocationControl = {
      value: state.source,
      selectionStart: 0,
      selectionEnd: 0,
      selectionDirection: "none",
      scrollTop: 200,
      scrollLeft: 150,
      clientHeight: 100,
      clientWidth: 100,
      focused: false,
      focus() {
        this.focused = true;
      },
      setSelectionRange(start, end, direction) {
        this.selectionStart = start;
        this.selectionEnd = end;
        this.selectionDirection = direction;
      }
    };
    byId = (id) => ({
      app: globalThis.editorLocationApp,
      editor: globalThis.editorLocationControl,
      goToLineBackdrop: globalThis.editorLocationBackdrop,
      goToLineError: globalThis.editorLocationError,
      goToLineInput: globalThis.editorLocationInput
    })[id] || null;
    globalThis.editorLocationMarkup = renderCursorInsight();
    globalThis.editorLocationSubmitted = submitGoToLine();
  `);

  assert.match(run("globalThis.editorLocationMarkup"), /data-go-to-line/);
  assert.match(run("globalThis.editorLocationMarkup"), /L1:C1/);
  assert.strictEqual(run("globalThis.editorLocationSubmitted"), true);
  assert.strictEqual(run("state.pendingGoToLine"), null);
  assert.strictEqual(run("globalThis.editorLocationBackdrop.removed"), true);
  assert.strictEqual(run("globalThis.editorLocationApp.inert"), false);
  assert.strictEqual(run("globalThis.editorLocationControl.focused"), true);
  assert.deepStrictEqual(
    Array.from(run("[globalThis.editorLocationControl.selectionStart, globalThis.editorLocationControl.selectionEnd, globalThis.editorLocationControl.selectionDirection, globalThis.editorLocationControl.scrollTop, globalThis.editorLocationControl.scrollLeft]")),
    [8, 8, "none", 0, 0]
  );
  assert.strictEqual(run("state.status"), "Line 2, column 3");
  assert.deepStrictEqual(
    Array.from(run("[state.tabs[0].selectionStart, state.tabs[0].selectionEnd, state.tabs[0].scrollTop, state.tabs[0].scrollLeft]")),
    [8, 8, 0, 0]
  );

  run(`
    globalThis.editorLocationControl.clientHeight = 100;
    globalThis.editorLocationControl.clientWidth = 100;
    globalThis.editorLocationControl.scrollTop = 0;
    globalThis.editorLocationControl.scrollLeft = 0;
    revealEditorLocation(globalThis.editorLocationControl, { lineIndex: 20, columnIndex: 80 });
  `);
  assert.deepStrictEqual(
    Array.from(run("[globalThis.editorLocationControl.scrollTop, globalThis.editorLocationControl.scrollLeft]")),
    [347, 620]
  );

  run(`
    state.pendingGoToLine = { path: state.currentPath, source: state.source };
    globalThis.editorLocationInput.value = "2:8";
    globalThis.editorLocationError.hidden = true;
    globalThis.editorLocationError.textContent = "";
    globalThis.invalidEditorLocationSubmitted = submitGoToLine();
  `);
  assert.strictEqual(run("globalThis.invalidEditorLocationSubmitted"), false);
  assert.strictEqual(run("globalThis.editorLocationError.hidden"), false);
  assert.strictEqual(run("globalThis.editorLocationError.textContent"), "Column must be between 1 and 7 on line 2.");
  assert.notStrictEqual(run("state.pendingGoToLine"), null);
  run("cancelGoToLine()");

  run(`
    globalThis.editorLocationOriginalSource = state.source;
    state.pendingGoToLine = { path: state.currentPath, source: state.source };
    globalThis.editorLocationInput.value = "1";
    state.source += "\\nchanged";
    globalThis.editorLocationControl.value = state.source;
    globalThis.editorLocationError.hidden = true;
    globalThis.editorLocationError.textContent = "";
    globalThis.staleEditorLocationSubmitted = submitGoToLine();
  `);
  assert.strictEqual(run("globalThis.staleEditorLocationSubmitted"), false);
  assert.strictEqual(run("globalThis.editorLocationError.textContent"), "The editor changed. Close this dialog and try again.");
  run(`
    state.source = globalThis.editorLocationOriginalSource;
    globalThis.editorLocationControl.value = state.source;
    cancelGoToLine();
  `);

  run(`
    globalThis.realOpenGoToLine = openGoToLine;
    globalThis.openGoToLineCalls = 0;
    openGoToLine = () => {
      globalThis.openGoToLineCalls += 1;
      return true;
    };
    globalThis.goToLineShortcutEvent = {
      altKey: false,
      ctrlKey: true,
      key: "g",
      metaKey: false,
      prevented: false,
      shiftKey: false,
      preventDefault() {
        this.prevented = true;
      }
    };
    handleGlobalKeyDown(globalThis.goToLineShortcutEvent);
    openGoToLine = globalThis.realOpenGoToLine;
  `);
  assert.strictEqual(run("globalThis.goToLineShortcutEvent.prevented"), true);
  assert.strictEqual(run("globalThis.openGoToLineCalls"), 1);
  run(`
    byId = globalThis.realByIdForEditorLocation;
    state.currentPath = "";
    state.source = "";
    state.tabs = [];
  `);
}

function outlineSelectionUsesUtf16Coordinates() {
  run(`
    globalThis.outlineEditor = {
      value: "head\\n  😀alpha = 1\\nlast",
      selectionStart: 0,
      selectionEnd: 0,
      scrollTop: 50,
      focused: false,
      focus() {
        this.focused = true;
      }
    };
    globalThis.outlineSelection = selectEditorUtf16Range(globalThis.outlineEditor, {
      line: 1,
      character: 4,
      endLine: 1,
      endCharacter: 9
    });
  `);
  assert.strictEqual(run("globalThis.outlineEditor.value.slice(globalThis.outlineEditor.selectionStart, globalThis.outlineEditor.selectionEnd)"), "alpha");
  assert.strictEqual(run("globalThis.outlineEditor.focused"), true);
  assert.strictEqual(run("JSON.stringify(globalThis.outlineSelection)"), '{"start":9,"end":14}');
}

function outlineRefreshPreservesFilterFocus() {
  run(`
    globalThis.realByIdForOutlineRefresh = byId;
    globalThis.outlinePanel = { outerHTML: "" };
    globalThis.outlineInput = {
      value: "room",
      focused: false,
      selectionStart: 1,
      selectionEnd: 3,
      focus() {
        this.focused = true;
      },
      setSelectionRange(start, end) {
        this.selectionStart = start;
        this.selectionEnd = end;
      }
    };
    document.activeElement = globalThis.outlineInput;
    document.activeElement.id = "outlineQueryInput";
    byId = (id) => ({
      outlinePanel: globalThis.outlinePanel,
      outlineQueryInput: globalThis.outlineInput
    })[id] || null;
    state.outlineOpen = true;
    state.outlineQuery = "room";
    state.check.documentSymbols = [];
    refreshOutlinePanel();
  `);

  assert.strictEqual(run("globalThis.outlineInput.focused"), true);
  assert.strictEqual(run("JSON.stringify([globalThis.outlineInput.selectionStart, globalThis.outlineInput.selectionEnd])"), "[1,3]");
  run(`
    byId = globalThis.realByIdForOutlineRefresh;
    document.activeElement = null;
  `);
}

function outlineShortcutFocusesCurrentFileSymbols() {
  run(`
    state.pendingTabClose = null;
    state.pendingWindowClose = false;
    globalThis.outlineShortcutCalls = 0;
    globalThis.realFocusOutline = focusOutline;
    focusOutline = () => {
      globalThis.outlineShortcutCalls += 1;
    };
    globalThis.outlineShortcutEvent = {
      altKey: false,
      ctrlKey: true,
      key: "o",
      metaKey: false,
      prevented: false,
      shiftKey: true,
      preventDefault() {
        this.prevented = true;
      }
    };
    handleGlobalKeyDown(globalThis.outlineShortcutEvent);
    focusOutline = globalThis.realFocusOutline;
  `);

  assert.strictEqual(run("globalThis.outlineShortcutEvent.prevented"), true);
  assert.strictEqual(run("globalThis.outlineShortcutCalls"), 1);
}

function findRangesRespectCaseMode() {
  assert.strictEqual(
    run(`JSON.stringify(editorFindRanges("Alpha alpha ALPHA", "alpha", false))`),
    '[{"start":0,"end":5},{"start":6,"end":11},{"start":12,"end":17}]'
  );
  assert.strictEqual(
    run(`JSON.stringify(editorFindRanges("Alpha alpha ALPHA", "alpha", true))`),
    '[{"start":6,"end":11}]'
  );
  assert.strictEqual(
    run(`JSON.stringify(editorFindRanges("aaaa", "aa", true))`),
    '[{"start":0,"end":2},{"start":2,"end":4}]'
  );
}

function findShortcutOpensCurrentFileSearch() {
  run(`
    state.pendingTabClose = null;
    state.pendingWindowClose = false;
    globalThis.findShortcutCalls = 0;
    globalThis.realOpenEditorFind = openEditorFind;
    openEditorFind = () => {
      globalThis.findShortcutCalls += 1;
    };
    globalThis.findShortcutEvent = {
      altKey: false,
      ctrlKey: true,
      key: "f",
      metaKey: false,
      prevented: false,
      shiftKey: false,
      preventDefault() {
        this.prevented = true;
      }
    };
    handleGlobalKeyDown(globalThis.findShortcutEvent);
    openEditorFind = globalThis.realOpenEditorFind;
  `);

  assert.strictEqual(run("globalThis.findShortcutEvent.prevented"), true);
  assert.strictEqual(run("globalThis.findShortcutCalls"), 1);
}

function openingFindDismissesCompletions() {
  run(`
    globalThis.realByIdForOpenFind = byId;
    globalThis.openFindEditor = {
      value: "alpha",
      selectionStart: 0,
      selectionEnd: 0,
      focus() {}
    };
    globalThis.openFindBar = {
      classList: {
        remove() {}
      }
    };
    globalThis.openFindInput = {
      value: "",
      focus() {},
      select() {}
    };
    globalThis.openFindOverlay = {
      hidden: false,
      innerHTML: "completion",
      classList: {
        add(name) {
          if (name === "hidden") globalThis.openFindOverlay.hidden = true;
        }
      }
    };
    byId = (id) => ({
      editor: globalThis.openFindEditor,
      editorFindBar: globalThis.openFindBar,
      editorFindInput: globalThis.openFindInput,
      completionOverlay: globalThis.openFindOverlay
    })[id] || null;
    state.completionItems = [{ label: "alpha" }];
    state.editorFindOpen = false;
    state.editorFindQuery = "";
    openEditorFind();
  `);

  assert.strictEqual(run("state.completionItems.length"), 0);
  assert.strictEqual(run("globalThis.openFindOverlay.hidden"), true);
  assert.strictEqual(run("globalThis.openFindOverlay.innerHTML"), "");
  assert.strictEqual(run("state.editorFindOpen"), true);
  run("byId = globalThis.realByIdForOpenFind");
}

function findNavigationWrapsBothDirections() {
  run(`
    globalThis.realByIdForFind = byId;
    globalThis.findEditor = {
      value: "alpha beta alpha",
      selectionStart: 0,
      selectionEnd: 0,
      scrollTop: 0,
      scrollLeft: 0,
      clientHeight: 100,
      focus() {}
    };
    globalThis.findCount = { textContent: "" };
    globalThis.findHighlight = { scrollTop: 0, scrollLeft: 0 };
    byId = (id) => ({
      editor: globalThis.findEditor,
      editorFindCount: globalThis.findCount,
      editorHighlight: globalThis.findHighlight
    })[id] || null;
    state.editorFindQuery = "alpha";
    state.editorFindCaseSensitive = false;
    state.editorFindMatchIndex = -1;
  `);

  assert.strictEqual(run("findEditorMatch(1, true)"), true);
  assert.deepStrictEqual(
    Array.from(run("[state.editorFindMatchIndex, globalThis.findEditor.selectionStart, globalThis.findEditor.selectionEnd]")),
    [0, 0, 5]
  );
  assert.strictEqual(run("globalThis.findCount.textContent"), "1/2");

  run("findEditorMatch(1)");
  assert.deepStrictEqual(
    Array.from(run("[state.editorFindMatchIndex, globalThis.findEditor.selectionStart, globalThis.findEditor.selectionEnd]")),
    [1, 11, 16]
  );
  run("findEditorMatch(1)");
  assert.strictEqual(run("state.editorFindMatchIndex"), 0);
  run("findEditorMatch(-1)");
  assert.strictEqual(run("state.editorFindMatchIndex"), 1);
  run("byId = globalThis.realByIdForFind");
}

function dirtyWindowRequestsUnloadConfirmation() {
  run(`
    state.tabs = [{ path: "dirty.eng", source: "changed", dirty: true }];
    globalThis.beforeUnloadEvent = {
      prevented: false,
      returnValue: undefined,
      preventDefault() {
        this.prevented = true;
      }
    };
    globalThis.beforeUnloadResult = handleBeforeUnload(globalThis.beforeUnloadEvent);
  `);

  assert.strictEqual(run("globalThis.beforeUnloadEvent.prevented"), true);
  assert.strictEqual(run("globalThis.beforeUnloadEvent.returnValue"), "");
  assert.strictEqual(run("globalThis.beforeUnloadResult"), "");

  run(`
    state.tabs[0].dirty = false;
    globalThis.cleanUnloadEvent = {
      prevented: false,
      preventDefault() {
        this.prevented = true;
      }
    };
    globalThis.cleanUnloadResult = handleBeforeUnload(globalThis.cleanUnloadEvent);
  `);
  assert.strictEqual(run("globalThis.cleanUnloadEvent.prevented"), false);
  assert.strictEqual(run("globalThis.cleanUnloadResult"), undefined);
}

async function nativeWindowCloseRequiresDecision() {
  nativeWindowState.closeCallback = null;
  run(`
    nativeCloseListenerBound = false;
    nativeAppWindow = null;
    state.tabs = [{ path: "dirty.eng", source: "changed", dirty: true }];
    state.pendingWindowClose = false;
    globalThis.windowDialogOpenCount = 0;
    globalThis.realOpenUnsavedWindowDialog = openUnsavedWindowDialog;
    openUnsavedWindowDialog = () => {
      state.pendingWindowClose = true;
      globalThis.windowDialogOpenCount += 1;
    };
  `);

  await run("bindNativeWindowClose()");
  assert.strictEqual(typeof nativeWindowState.closeCallback, "function");
  const dirtyEvent = {
    prevented: false,
    preventDefault() {
      this.prevented = true;
    }
  };
  nativeWindowState.closeCallback(dirtyEvent);
  assert.strictEqual(dirtyEvent.prevented, true);
  assert.strictEqual(run("state.pendingWindowClose"), true);
  assert.strictEqual(run("globalThis.windowDialogOpenCount"), 1);

  run("state.tabs[0].dirty = false");
  const cleanEvent = {
    prevented: false,
    preventDefault() {
      this.prevented = true;
    }
  };
  nativeWindowState.closeCallback(cleanEvent);
  assert.strictEqual(cleanEvent.prevented, false);
  assert.strictEqual(run("globalThis.windowDialogOpenCount"), 1);
  run("openUnsavedWindowDialog = globalThis.realOpenUnsavedWindowDialog");
}

async function saveAllPersistsWithoutClosingWindow() {
  invokeCalls.length = 0;
  nativeWindowState.destroyCalls = 0;
  run(`
    state.tabs = [
      { path: "first.eng", source: "first changed", savedSource: "first disk", dirty: true },
      { path: "second.eng", source: "second changed", savedSource: "second disk", dirty: true }
    ];
    state.currentPath = "second.eng";
    state.source = "second changed";
    state.savedSource = "second disk";
    state.dirty = true;
    state.pendingWindowClose = false;
  `);

  assert.strictEqual(await run("saveAllDirtyTabs()"), true);
  assert.strictEqual(invokeCalls.length, 1);
  assert.strictEqual(invokeCalls[0].command, "ide_save_files");
  assert.deepStrictEqual(JSON.parse(JSON.stringify(invokeCalls[0].args.files)), [
    { path: "first.eng", source: "first changed", expectedSource: "first disk" },
    { path: "second.eng", source: "second changed", expectedSource: "second disk" }
  ]);
  assert.deepStrictEqual(Array.from(run("state.tabs.map((tab) => tab.dirty)")), [false, false]);
  assert.strictEqual(nativeWindowState.destroyCalls, 0);
}

async function saveAllDecisionPersistsThenDestroysWindow() {
  invokeCalls.length = 0;
  nativeWindowState.destroyCalls = 0;
  run(`
    state.tabs = [
      { path: "first.eng", source: "first changed", savedSource: "first disk", dirty: true },
      { path: "second.eng", source: "second changed", savedSource: "second disk", dirty: true }
    ];
    state.currentPath = "second.eng";
    state.source = "second changed";
    state.savedSource = "second disk";
    state.dirty = true;
    state.pendingWindowClose = true;
  `);

  await run("saveAllDirtyTabsAndClose()");
  assert.strictEqual(invokeCalls.length, 1);
  assert.strictEqual(invokeCalls[0].command, "ide_save_files");
  assert.deepStrictEqual(JSON.parse(JSON.stringify(invokeCalls[0].args.files)), [
    { path: "first.eng", source: "first changed", expectedSource: "first disk" },
    { path: "second.eng", source: "second changed", expectedSource: "second disk" }
  ]);
  assert.deepStrictEqual(Array.from(run("state.tabs.map((tab) => tab.dirty)")), [false, false]);
  assert.strictEqual(run("state.dirty"), false);
  assert.strictEqual(nativeWindowState.destroyCalls, 1);
}

async function discardAllDecisionDestroysWithoutSaving() {
  invokeCalls.length = 0;
  nativeWindowState.destroyCalls = 0;
  run(`
    state.tabs = [{ path: "dirty.eng", source: "changed", dirty: true }];
    state.pendingWindowClose = true;
  `);

  await run("discardAllDirtyTabsAndClose()");
  assert.strictEqual(invokeCalls.length, 0);
  assert.strictEqual(nativeWindowState.destroyCalls, 1);
}

async function saveAllFailureKeepsRemainingDirtyFilesOpen() {
  invokeCalls.length = 0;
  nativeWindowState.destroyCalls = 0;
  saveFailurePath = "second.eng";
  run(`
    state.tabs = [
      { path: "first.eng", source: "first changed", savedSource: "first disk", dirty: true },
      { path: "second.eng", source: "second changed", savedSource: "second disk", dirty: true }
    ];
    state.currentPath = "second.eng";
    state.source = "second changed";
    state.savedSource = "second disk";
    state.dirty = true;
    state.pendingWindowClose = true;
    globalThis.failureDialogOpenCount = 0;
    globalThis.realOpenUnsavedWindowDialog = openUnsavedWindowDialog;
    openUnsavedWindowDialog = () => {
      state.pendingWindowClose = true;
      globalThis.failureDialogOpenCount += 1;
    };
  `);

  await run("saveAllDirtyTabsAndClose()");
  saveFailurePath = null;
  assert.strictEqual(invokeCalls.length, 1);
  assert.strictEqual(invokeCalls[0].command, "ide_save_files");
  assert.deepStrictEqual(Array.from(run("state.tabs.map((tab) => tab.dirty)")), [true, true]);
  assert.strictEqual(run("state.dirty"), true);
  assert.strictEqual(run("globalThis.failureDialogOpenCount"), 1);
  assert.strictEqual(nativeWindowState.destroyCalls, 0);
  run("openUnsavedWindowDialog = globalThis.realOpenUnsavedWindowDialog");
}

function liveCheckTracksAllDirtyImportedBuffers() {
  run(`
    state.root = "C:/Repo";
    state.currentPath = "main.eng";
    state.source = "use \\"module.eng\\"\\nresult = SHARED_GAIN\\n";
    state.dirty = true;
    state.tabs = [
      { path: "main.eng", source: state.source, dirty: true },
      { path: "module.eng", source: "const SHARED_GAIN: Ratio = 0.9\\n", dirty: true },
      { path: "notes.csv", source: "ignored", dirty: true }
    ];
    globalThis.liveCheckRequest = beginCheckRequest();
  `);

  assert.deepStrictEqual(
    JSON.parse(run("JSON.stringify(globalThis.liveCheckRequest.documents)")),
    [{ path: "module.eng", source: "const SHARED_GAIN: Ratio = 0.9\n" }]
  );
  assert.strictEqual(run("checkRequestIsCurrent(globalThis.liveCheckRequest)"), true);
  run("state.tabs[1].source = 'const CHANGED_GAIN: Ratio = 0.7\\n'");
  assert.strictEqual(run("checkRequestIsCurrent(globalThis.liveCheckRequest)"), false);
}

async function main() {
  await dirtyTabRequiresDecision();
  await reopeningDirtyTabPreservesTheOpenBuffer();
  await saveDecisionPersistsThenCloses();
  await runSafelySavesBeforeExecuting();
  await terminalRunSafelySavesBeforeExecuting();
  await runConflictKeepsEveryBufferUnsaved();
  await runChangedImportDuringSaveCancelsExecution();
  saveShortcutUsesCurrentAction();
  definitionPathsNormalizeWorkspaceTargets();
  definitionRequestUsesUtf16Caret();
  await definitionNavigationPreservesDirtyOpenTab();
  await definitionNavigationUsesAndGuardsAllDirtyWorkspaceBuffers();
  definitionShortcutUsesCurrentAction();
  workspaceSymbolShortcutOpensCompilerSearch();
  await workspaceSymbolSearchUsesDirtyBuffersAndCompilerLocations();
  await closedWorkspaceSymbolSearchRejectsLateResults();
  await workspaceSymbolNavigationSelectsUtf16Range();
  documentHighlightShortcutUsesCurrentAction();
  quickFixShortcutUsesCurrentProblemAction();
  problemNavigationUsesFilteredUtf16RangesAndWraps();
  problemNavigationShortcutUsesBothDirections();
  renameShortcutUsesCurrentAction();
  busyRenameCanBeCancelledSafely();
  await renamePreparationAllowsOtherDirtyEngLangTabs();
  semanticTokenAndReferenceRangesUseUtf16Coordinates();
  workspaceReferencesTrackAllDirtyOpenBuffers();
  await workspaceRenameStagesVerifiedUtf16Buffers();
  await compilerQuickFixAppliesUnsavedUtf16Edits();
  documentSymbolsNormalizeAndFilter();
  documentBreadcrumbsTrackNestedSymbolsAndFreshness();
  documentBreadcrumbNavigationUsesUtf16Coordinates();
  editorViewStatePersistsAcrossRendersAndTabs();
  editorLineNumbersTrackSourceAndScroll();
  editorLocationNavigationUsesValidatedUtf16Coordinates();
  outlineSelectionUsesUtf16Coordinates();
  outlineRefreshPreservesFilterFocus();
  outlineShortcutFocusesCurrentFileSymbols();
  findRangesRespectCaseMode();
  findShortcutOpensCurrentFileSearch();
  openingFindDismissesCompletions();
  findNavigationWrapsBothDirections();
  dirtyWindowRequestsUnloadConfirmation();
  await nativeWindowCloseRequiresDecision();
  await saveAllPersistsWithoutClosingWindow();
  await saveAllDecisionPersistsThenDestroysWindow();
  await discardAllDecisionDestroysWithoutSaving();
  await saveAllFailureKeepsRemainingDirtyFilesOpen();
  liveCheckTracksAllDirtyImportedBuffers();
  process.stdout.write("Native IDE editor safety smoke passed.\n");
}

main().catch((error) => {
  process.stderr.write(String(error.stack || error.message) + "\n");
  process.exitCode = 1;
});
