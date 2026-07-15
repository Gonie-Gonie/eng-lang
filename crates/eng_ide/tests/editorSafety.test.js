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
