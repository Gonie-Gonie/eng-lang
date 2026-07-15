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
              return Promise.reject(new Error(`cannot save ${args.path}`));
            }
            return Promise.resolve({ path: args.path, source: args.source });
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

async function saveDecisionPersistsThenCloses() {
  invokeCalls.length = 0;
  run(`
    state.tabs = [
      { path: "current.eng", source: "current", dirty: false },
      { path: "dirty.eng", source: "changed", dirty: true }
    ];
    state.currentPath = "current.eng";
    state.source = "current";
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
  assert.deepStrictEqual(
    Array.from(run("state.tabs.map((tab) => tab.path)")),
    ["current.eng"]
  );
  assert.strictEqual(run("state.pendingTabClose"), null);
  assert.strictEqual(run("globalThis.renderCount"), 1);
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
      { path: "first.eng", source: "first changed", dirty: true },
      { path: "second.eng", source: "second changed", dirty: true }
    ];
    state.currentPath = "second.eng";
    state.source = "second changed";
    state.dirty = true;
    state.pendingWindowClose = false;
  `);

  assert.strictEqual(await run("saveAllDirtyTabs()"), true);
  assert.deepStrictEqual(
    invokeCalls.map((item) => [item.command, item.args.path]),
    [
      ["ide_save_file", "first.eng"],
      ["ide_save_file", "second.eng"]
    ]
  );
  assert.deepStrictEqual(Array.from(run("state.tabs.map((tab) => tab.dirty)")), [false, false]);
  assert.strictEqual(nativeWindowState.destroyCalls, 0);
}

async function saveAllDecisionPersistsThenDestroysWindow() {
  invokeCalls.length = 0;
  nativeWindowState.destroyCalls = 0;
  run(`
    state.tabs = [
      { path: "first.eng", source: "first changed", dirty: true },
      { path: "second.eng", source: "second changed", dirty: true }
    ];
    state.currentPath = "second.eng";
    state.source = "second changed";
    state.dirty = true;
    state.pendingWindowClose = true;
  `);

  await run("saveAllDirtyTabsAndClose()");
  assert.deepStrictEqual(
    invokeCalls.map((item) => [item.command, item.args.path, item.args.source]),
    [
      ["ide_save_file", "first.eng", "first changed"],
      ["ide_save_file", "second.eng", "second changed"]
    ]
  );
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
      { path: "first.eng", source: "first changed", dirty: true },
      { path: "second.eng", source: "second changed", dirty: true }
    ];
    state.currentPath = "second.eng";
    state.source = "second changed";
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
  assert.deepStrictEqual(Array.from(run("state.tabs.map((tab) => tab.dirty)")), [false, true]);
  assert.strictEqual(run("state.dirty"), true);
  assert.strictEqual(run("globalThis.failureDialogOpenCount"), 1);
  assert.strictEqual(nativeWindowState.destroyCalls, 0);
  run("openUnsavedWindowDialog = globalThis.realOpenUnsavedWindowDialog");
}

async function main() {
  await dirtyTabRequiresDecision();
  await saveDecisionPersistsThenCloses();
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
  renameShortcutUsesCurrentAction();
  busyRenameCanBeCancelledSafely();
  await renamePreparationAllowsOtherDirtyEngLangTabs();
  semanticTokenAndReferenceRangesUseUtf16Coordinates();
  workspaceReferencesTrackAllDirtyOpenBuffers();
  await workspaceRenameStagesVerifiedUtf16Buffers();
  await compilerQuickFixAppliesUnsavedUtf16Edits();
  documentSymbolsNormalizeAndFilter();
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
  process.stdout.write("Native IDE editor safety smoke passed.\n");
}

main().catch((error) => {
  process.stderr.write(String(error.stack || error.message) + "\n");
  process.exitCode = 1;
});
