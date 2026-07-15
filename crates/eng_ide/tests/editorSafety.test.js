"use strict";

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const vm = require("vm");

const appPath = path.join(__dirname, "..", "ui", "app.js");
const source = fs.readFileSync(appPath, "utf8");
const invokeCalls = [];
let saveFailurePath = null;
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
  definitionShortcutUsesCurrentAction();
  documentHighlightShortcutUsesCurrentAction();
  semanticTokenAndReferenceRangesUseUtf16Coordinates();
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
  await saveAllDecisionPersistsThenDestroysWindow();
  await discardAllDecisionDestroysWithoutSaving();
  await saveAllFailureKeepsRemainingDirtyFilesOpen();
  process.stdout.write("Native IDE editor safety smoke passed.\n");
}

main().catch((error) => {
  process.stderr.write(String(error.stack || error.message) + "\n");
  process.exitCode = 1;
});
