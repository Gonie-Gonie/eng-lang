const invoke = window.__TAURI__?.core?.invoke;
const listen = window.__TAURI__?.event?.listen;
const RUN_HISTORY_LIMIT = 40;
const RUN_HISTORY_STORAGE_PREFIX = "englang.nativeIde.runHistory.v1:";

const state = {
  root: "",
  fileTree: [],
  tabs: [],
  completions: [],
  completionItems: [],
  completionIndex: 0,
  modules: [],
  openDirs: new Set(["examples", "examples/official", "stdlib"]),
  currentPath: "",
  runDir: "",
  source: "",
  dirty: false,
  check: { diagnostics: [], symbols: [], status: "", semanticTokens: { legend: {}, tokens: [] }, hovers: [] },
  highlightSource: "",
  variables: [],
  args: [],
  artifacts: [],
  plotSpec: null,
  reportTitle: "",
  inspectors: emptyInspectors(),
  profile: "normal",
  runHistory: [],
  terminalEntries: [{ kind: "info", text: "Ready." }],
  terminalCommands: [],
  terminalHistoryIndex: null,
  bottomTab: "terminal",
  problemSeverity: "all",
  problemCode: "all",
  problemQuery: "",
  moduleCategory: "all",
  moduleQuery: "",
  highlightTokenQuery: "",
  sideTab: "variables",
  selectedVariable: null,
  selectedWorkflowNodeId: null,
  status: "Starting"
};

let dragDropBound = false;

function byId(id) {
  return document.getElementById(id);
}

function emptyInspectors() {
  return {
    schemas: [],
    unitConversions: [],
    timeAxes: [],
    timeSeries: [],
    timeSeriesCoverage: [],
    metrics: [],
    validations: [],
    quality: null,
    timeAlignments: [],
    tableTransforms: [],
    structuredReads: [],
    configPromotions: [],
    systems: [],
    systemIr: [],
    kernelPlan: null,
    classObjects: [],
    assemblies: [],
    componentGraph: null,
    reviewDocument: null,
    artifactOutlines: [],
    effectRecords: null,
    networkCache: null,
    dbWrites: null,
    modelCards: null,
    caseManifests: null,
    outputManifest: null,
    runPlan: null,
    runLog: null,
    processResults: null,
    testResults: null
  };
}

async function call(cmd, args = {}) {
  if (!invoke) throw new Error("Tauri invoke API is not available");
  return await invoke(cmd, args);
}

function applyCheck(check, source = state.source) {
  state.check = normalizeCheck(check);
  state.highlightSource = String(source ?? "");
}

function normalizeCheck(check) {
  const semanticTokens = check?.semanticTokens ?? check?.semantic_tokens ?? { legend: {}, tokens: [] };
  return {
    diagnostics: Array.isArray(check?.diagnostics) ? check.diagnostics : [],
    symbols: Array.isArray(check?.symbols) ? check.symbols : [],
    status: check?.status || "",
    hovers: Array.isArray(check?.hovers) ? check.hovers : [],
    semanticTokens: {
      legend: semanticTokens.legend || {},
      tokens: Array.isArray(semanticTokens.tokens) ? semanticTokens.tokens : []
    }
  };
}

async function boot() {
  try {
    const data = await call("ide_bootstrap");
    state.root = data.root;
    state.fileTree = data.fileTree;
    state.completions = data.completions ?? [];
    state.modules = data.modules ?? [];
    state.runHistory = loadRunHistory(data.root);
    state.currentPath = data.current.path;
    state.runDir = data.currentDir || directoryOf(data.current.path);
    state.source = data.current.source;
    state.tabs = [{ path: state.currentPath, source: state.source, dirty: false }];
    applyCheck(data.check, state.source);
    state.terminalEntries = [
      { kind: "info", text: `Workspace ${data.root}` },
      { kind: "info", text: `Loaded ${state.currentPath}` }
    ];
    state.status = `Loaded ${state.currentPath}`;
    render();
  } catch (error) {
    document.body.innerHTML = `<pre class="error">${escapeHtml(String(error))}</pre>`;
  }
}

function render() {
  const app = byId("app");
  app.className = "shell";
  app.innerHTML = `
    ${renderToolbar()}
    ${renderWorkspaceBar()}
    ${renderExplorer()}
    <div class="splitter splitter-left" data-splitter="left"></div>
    <main class="editor-wrap">
      <div class="editor-tabs">${renderTabs()}</div>
      <div class="editor-meta">
        <span>${escapeHtml(state.currentPath)}</span>
        <span id="cursorInsight" class="cursor-insight">${renderCursorInsight()}</span>
        <span>${lineCount(state.source)} lines</span>
      </div>
      <div class="editor-shell">
        <pre id="editorHighlight" class="editor-highlight" aria-hidden="true">${renderHighlightedSource()}</pre>
        <textarea id="editor" class="editor" spellcheck="false" wrap="off">${escapeHtml(state.source)}</textarea>
        <div id="completionOverlay" class="completion-popover hidden"></div>
      </div>
    </main>
    <div class="splitter splitter-right" data-splitter="right"></div>
    ${renderSidePanel()}
    <div class="splitter splitter-bottom" data-splitter="bottom"></div>
    <section class="bottom">
      <div class="bottom-tabs">
        <button class="bottom-tab ${state.bottomTab === "problems" ? "active" : ""}" data-tab="problems">Problems</button>
        <button class="bottom-tab ${state.bottomTab === "terminal" ? "active" : ""}" data-tab="terminal">Terminal</button>
      </div>
      <div class="bottom-body">${state.bottomTab === "problems" ? renderProblems() : renderTerminal()}</div>
    </section>
    <footer class="statusbar">
      <span>${escapeHtml(state.check.status || "ready")}</span>
      <span>${escapeHtml(state.currentPath || "-")}</span>
      <span>Run Dir: ${escapeHtml(state.runDir || ".")}</span>
    </footer>
  `;
  bind();
  bindGlobalEvents();
  syncEditorHighlightScroll();
  updateCursorInsight();
  if (state.sideTab === "plot" && state.plotSpec) drawPlot("sidePlotCanvas");
}

function renderToolbar() {
  return `
    <div class="toolbar">
      <div class="title-mark">EngLang</div>
      ${toolButton("runBtn", "Run", "Run current file", "play", true)}
      ${toolButton("checkBtn", "Check", "Check diagnostics", "check")}
      ${toolButton("saveBtn", "Save", "Save current file", "save")}
      <span class="toolbar-separator"></span>
      ${toolButton("reportBtn", "Report", "Open last report", "file")}
      ${toolButton("outputBtn", "Output", "Open output folder", "folder")}
      ${toolButton("plotBtn", "Plot", "Show plot panel", "chart")}
      <select id="profileSelect" class="profile-select" title="Execution profile">
        ${["normal", "safe", "repro"].map((profile) => `<option value="${profile}" ${state.profile === profile ? "selected" : ""}>${profile}</option>`).join("")}
      </select>
      <span class="badge ${errorCount() ? "bad" : ""}">Errors ${errorCount()}</span>
      <span class="badge ${warningCount() ? "warn" : ""}">Warnings ${warningCount()}</span>
      <span class="status">${escapeHtml(state.status)}</span>
    </div>
  `;
}

function renderWorkspaceBar() {
  return `
    <div class="pathbar">
      <span class="path-label">Workspace</span>
      <span class="workspace-root" title="${escapeAttr(state.root)}">${escapeHtml(compactPath(state.root))}</span>
      <span class="path-label">File</span>
      <input id="pathInput" value="${escapeAttr(state.currentPath)}" />
      <button id="openPathBtn">Open</button>
      <span class="path-label">Run Dir</span>
      <input id="runDirInput" class="run-dir-input" value="${escapeAttr(state.runDir || ".")}" />
      <button id="applyRunDirBtn">Use</button>
    </div>
  `;
}

function renderExplorer() {
  return `
    <aside class="sidebar">
      <div class="panel-title explorer-title">
        <span>Explorer</span>
        <small>${escapeHtml(state.runDir || ".")}</small>
      </div>
      <div class="open-editors">
        <div class="mini-title">Open Editors</div>
        ${renderOpenEditors()}
      </div>
      <div class="tree-head">
        <span>Workspace</span>
        <button id="collapseExplorerBtn" title="Collapse folders">Collapse</button>
      </div>
      <div class="scroll tree">${renderTree(state.fileTree, 0)}</div>
    </aside>
  `;
}

function renderOpenEditors() {
  return state.tabs.map((tab) => `
    <button class="open-editor ${tab.path === state.currentPath ? "active" : ""}" data-tab-path="${escapeAttr(tab.path)}" title="${escapeAttr(tab.path)}">
      <span>${escapeHtml(fileName(tab.path))}${tab.dirty ? " *" : ""}</span>
    </button>
  `).join("");
}

function toolButton(id, label, title, iconName, primary = false) {
  return `
    <button class="tool ${primary ? "primary" : ""}" id="${id}" title="${escapeAttr(title)}">
      ${icon(iconName)}
      <span>${escapeHtml(label)}</span>
    </button>
  `;
}

function icon(name) {
  const paths = {
    play: '<path d="M7 5v14l11-7z"/>',
    check: '<path d="M5 12.5l4 4L19 6"/>',
    save: '<path d="M5 5h12l2 2v12H5z"/><path d="M8 5v5h8V5"/><path d="M8 19v-5h8v5"/>',
    file: '<path d="M7 3h7l5 5v13H7z"/><path d="M14 3v6h5"/>',
    folder: '<path d="M3 6h7l2 2h9v10H3z"/><path d="M3 8h18"/>',
    chart: '<path d="M4 19h16"/><path d="M7 16v-5"/><path d="M12 16V7"/><path d="M17 16v-8"/>'
  };
  return `<svg class="icon" viewBox="0 0 24 24" aria-hidden="true">${paths[name] || ""}</svg>`;
}

function bind() {
  const editor = byId("editor");
  editor.addEventListener("keydown", handleEditorKeyDown);
  editor.addEventListener("scroll", syncEditorHighlightScroll);
  editor.addEventListener("keyup", (event) => {
    updateCursorInsight();
    if (["ArrowDown", "ArrowUp", "Enter", "Tab", "Escape"].includes(event.key)) return;
    updateCompletionOverlay();
  });
  editor.addEventListener("click", () => {
    updateCursorInsight();
    updateCompletionOverlay();
  });
  editor.addEventListener("mouseup", updateCursorInsight);
  editor.addEventListener("select", updateCursorInsight);
  editor.addEventListener("input", (event) => {
    state.source = event.target.value;
    state.dirty = true;
    rememberCurrentTab();
    state.status = "Modified";
    renderTabLabels();
    updateEditorHighlight();
    updateCursorInsight();
    updateCompletionOverlay();
  });
  byId("checkBtn").onclick = checkCurrent;
  byId("saveBtn").onclick = saveCurrent;
  byId("runBtn").onclick = runCurrent;
  byId("reportBtn").onclick = () => openArtifact("report");
  byId("outputBtn").onclick = () => openArtifact("output_folder");
  byId("plotBtn").onclick = () => {
    state.sideTab = "plot";
    render();
  };
  byId("profileSelect").onchange = (event) => {
    state.profile = event.target.value;
    state.status = `Profile ${state.profile}`;
    render();
  };
  byId("openPathBtn").onclick = () => openFile(byId("pathInput").value);
  byId("pathInput").addEventListener("keydown", (event) => {
    if (event.key === "Enter") openFile(event.currentTarget.value);
  });
  byId("applyRunDirBtn").onclick = () => setRunDir(byId("runDirInput").value);
  byId("runDirInput").addEventListener("keydown", (event) => {
    if (event.key === "Enter") setRunDir(event.currentTarget.value);
  });
  const collapseExplorerBtn = byId("collapseExplorerBtn");
  if (collapseExplorerBtn) {
    collapseExplorerBtn.onclick = () => {
      state.openDirs = new Set(parentDirs(state.currentPath));
      render();
    };
  }
  document.querySelectorAll("[data-path]").forEach((node) => {
    node.onclick = () => {
      if (node.dataset.kind === "file") openFile(node.dataset.path);
      if (node.dataset.kind === "dir") toggleDir(node.dataset.path);
    };
  });
  document.querySelectorAll("[data-tab-path]").forEach((tab) => {
    tab.onclick = () => switchTab(tab.dataset.tabPath);
  });
  document.querySelectorAll("[data-close-path]").forEach((button) => {
    button.onclick = (event) => {
      event.stopPropagation();
      closeTab(button.dataset.closePath);
    };
  });
  document.querySelectorAll("[data-tab]").forEach((tab) => {
    tab.onclick = () => {
      state.bottomTab = tab.dataset.tab;
      render();
    };
  });
  document.querySelectorAll("[data-problem-severity]").forEach((button) => {
    button.onclick = () => {
      state.problemSeverity = button.dataset.problemSeverity;
      render();
    };
  });
  const problemCodeFilter = byId("problemCodeFilter");
  if (problemCodeFilter) {
    problemCodeFilter.onchange = (event) => {
      state.problemCode = event.target.value;
      render();
    };
  }
  const clearProblemFilters = byId("clearProblemFilters");
  if (clearProblemFilters) {
    clearProblemFilters.onclick = () => {
      state.problemSeverity = "all";
      state.problemCode = "all";
      state.problemQuery = "";
      render();
    };
  }
  const problemQueryInput = byId("problemQueryInput");
  if (problemQueryInput) {
    problemQueryInput.oninput = (event) => {
      const cursor = event.target.selectionStart ?? event.target.value.length;
      state.problemQuery = event.target.value;
      render();
      const nextInput = byId("problemQueryInput");
      if (nextInput) {
        nextInput.focus();
        nextInput.setSelectionRange(cursor, cursor);
      }
    };
  }
  document.querySelectorAll("[data-problem-line]").forEach((row) => {
    row.onclick = (event) => {
      if (event.target.closest("button")) return;
      selectSourceLine(Number(row.dataset.problemLine || 0));
    };
  });
  document.querySelectorAll("[data-module-category]").forEach((button) => {
    button.onclick = () => {
      state.moduleCategory = button.dataset.moduleCategory;
      render();
    };
  });
  const clearModuleFilters = byId("clearModuleFilters");
  if (clearModuleFilters) {
    clearModuleFilters.onclick = () => {
      state.moduleCategory = "all";
      state.moduleQuery = "";
      render();
    };
  }
  const moduleQueryInput = byId("moduleQueryInput");
  if (moduleQueryInput) {
    moduleQueryInput.oninput = (event) => {
      const cursor = event.target.selectionStart ?? event.target.value.length;
      state.moduleQuery = event.target.value;
      render();
      const nextInput = byId("moduleQueryInput");
      if (nextInput) {
        nextInput.focus();
        nextInput.setSelectionRange(cursor, cursor);
      }
    };
  }
  const clearHighlightTokenFilter = byId("clearHighlightTokenFilter");
  if (clearHighlightTokenFilter) {
    clearHighlightTokenFilter.onclick = () => {
      state.highlightTokenQuery = "";
      render();
    };
  }
  const highlightTokenQueryInput = byId("highlightTokenQueryInput");
  if (highlightTokenQueryInput) {
    highlightTokenQueryInput.oninput = (event) => {
      const cursor = event.target.selectionStart ?? event.target.value.length;
      state.highlightTokenQuery = event.target.value;
      render();
      const nextInput = byId("highlightTokenQueryInput");
      if (nextInput) {
        nextInput.focus();
        nextInput.setSelectionRange(cursor, cursor);
      }
    };
  }
  document.querySelectorAll("[data-side-tab]").forEach((tab) => {
    tab.onclick = () => {
      state.sideTab = tab.dataset.sideTab;
      render();
    };
  });
  document.querySelectorAll("[data-variable]").forEach((row) => {
    row.onclick = (event) => {
      if (event.target.closest("[data-source-line]")) return;
      state.selectedVariable = state.selectedVariable === row.dataset.variable ? null : row.dataset.variable;
      render();
    };
  });
  document.querySelectorAll("[data-workflow-node-id]").forEach((node) => {
    node.onclick = (event) => {
      if (event.target.closest("[data-source-line]")) return;
      state.selectedWorkflowNodeId = node.dataset.workflowNodeId;
      render();
    };
  });
  const openPlotArtifact = byId("openPlotArtifact");
  if (openPlotArtifact) openPlotArtifact.onclick = () => openArtifact("plot");
  document.querySelectorAll("[data-open-artifact-kind]").forEach((button) => {
    button.onclick = () => openArtifact(button.dataset.openArtifactKind);
  });
  document.querySelectorAll("[data-open-file-path]").forEach((button) => {
    button.onclick = () => openFile(button.dataset.openFilePath);
  });
  document.querySelectorAll("[data-open-path]").forEach((button) => {
    button.onclick = () => openWorkspacePath(button.dataset.openPath);
  });
  document.querySelectorAll("[data-source-line]").forEach((button) => {
    button.onclick = () => selectSourceLine(Number(button.dataset.sourceLine || 0));
  });
  document.querySelectorAll("[data-source-token-line]").forEach((button) => {
    button.onclick = () => selectSourceTokenRange(
      Number(button.dataset.sourceTokenLine || 0),
      Number(button.dataset.sourceTokenStart || 0),
      Number(button.dataset.sourceTokenLength || 0)
    );
  });
  document.querySelectorAll("[data-show-highlight-panel]").forEach((button) => {
    button.onclick = () => {
      state.sideTab = "highlight";
      render();
    };
  });
  const terminalInput = byId("terminalInput");
  if (terminalInput) {
    terminalInput.focus();
    terminalInput.addEventListener("keydown", (event) => {
      if (event.key === "Enter") sendTerminal();
      if (event.key === "ArrowUp") {
        event.preventDefault();
        recallTerminalCommand(-1);
      }
      if (event.key === "ArrowDown") {
        event.preventDefault();
        recallTerminalCommand(1);
      }
    });
    byId("terminalSend").onclick = sendTerminal;
    byId("terminalPlot").onclick = () => {
      state.sideTab = "plot";
      render();
    };
    byId("terminalReset").onclick = () => sendTerminalCommand("reset");
    byId("terminalClear").onclick = () => {
      clearTerminal();
      render();
    };
  }
  bindSplitters();
}

async function openFile(path) {
  try {
    rememberCurrentTab();
    const file = await call("ide_open_file", { path });
    state.currentPath = file.path;
    state.runDir = directoryOf(file.path);
    openParentDirs(file.path);
    state.source = file.source;
    state.dirty = false;
    const existing = tabFor(file.path);
    if (existing) {
      existing.source = file.source;
      existing.dirty = false;
    } else {
      state.tabs.push({ path: file.path, source: file.source, dirty: false });
    }
    state.variables = [];
    state.args = [];
    state.artifacts = [];
    state.inspectors = emptyInspectors();
    state.completionItems = [];
    state.plotSpec = null;
    state.reportTitle = "";
    state.selectedWorkflowNodeId = null;
    state.status = `Loaded ${file.path}`;
    const check = await call("ide_check", { path: state.currentPath, source: state.source });
    applyCheck(check, state.source);
    render();
  } catch (error) {
    state.status = String(error);
    appendTerminal("error", String(error));
    render();
  }
}

async function saveCurrent() {
  try {
    const file = await call("ide_save_file", { path: state.currentPath, source: state.source });
    state.currentPath = file.path;
    state.source = file.source;
    state.dirty = false;
    const tab = tabFor(file.path);
    if (tab) {
      tab.source = file.source;
      tab.dirty = false;
    }
    state.status = `Saved ${file.path}`;
    render();
  } catch (error) {
    state.status = String(error);
    appendTerminal("error", String(error));
    render();
  }
}

async function checkCurrent() {
  try {
    rememberCurrentTab();
    const check = await call("ide_check", { path: state.currentPath, source: state.source });
    applyCheck(check, state.source);
    state.status = `Checked: ${state.check.status}`;
    state.bottomTab = errorCount() ? "problems" : state.bottomTab;
    render();
  } catch (error) {
    state.status = String(error);
    appendTerminal("error", String(error));
    render();
  }
}

async function runCurrent() {
  try {
    rememberCurrentTab();
    appendTerminal("command", `${terminalPrompt()}run ${fileName(state.currentPath)}`);
    const result = await call("ide_run", { path: state.currentPath, source: state.source, profile: state.profile });
    applyRun(result, { mergeRuntime: false });
    appendRunResult(result, runHistoryContext("run"));
    state.status = result.ok ? "Run complete" : "Run blocked";
    state.bottomTab = "terminal";
    state.dirty = false;
    const tab = tabFor(state.currentPath);
    if (tab) tab.dirty = false;
    render();
  } catch (error) {
    appendTerminal("error", `Run failed: ${String(error)}`);
    state.status = "Run failed";
    state.bottomTab = "terminal";
    render();
  }
}

async function sendTerminal() {
  const input = byId("terminalInput");
  const command = input.value.trim();
  if (!command) return;
  input.value = "";
  rememberTerminalCommand(command);
  await sendTerminalCommand(command);
}

async function sendTerminalCommand(command) {
  const prompt = terminalPrompt();
  if (command.toLowerCase() === "clear") {
    clearTerminal();
    render();
    return;
  }
  if (command.toLowerCase().startsWith("cd ")) {
    appendTerminal("command", `${prompt}${command}`);
    setRunDir(command.slice(3).trim(), false);
    appendTerminal("info", `Run directory: ${state.runDir || "."}`);
    state.bottomTab = "terminal";
    render();
    return;
  }
  appendTerminal("command", `${prompt}${command}`);
  try {
    const result = await call("ide_terminal", {
      path: state.currentPath,
      source: state.source,
      command,
      runDir: state.runDir,
      profile: state.profile
    });
    applyRun(result, {
      mergeRuntime: command.toLowerCase() !== "run",
      checkSource: terminalCommandUsesCurrentFile(command) ? state.source : ""
    });
    appendRunResult(result, runHistoryContext(command));
    state.status = result.ok ? "Terminal command complete" : "Terminal diagnostics";
  } catch (error) {
    appendTerminal("error", String(error));
    state.status = "Terminal command failed";
  }
  state.bottomTab = "terminal";
  render();
}

async function openArtifact(kind) {
  try {
    const opened = await call("ide_open_artifact", { kind });
    appendTerminal("info", `Opened ${opened}`);
    state.status = `Opened ${kind}`;
    render();
  } catch (error) {
    state.status = String(error);
    appendTerminal("error", String(error));
    render();
  }
}

async function openWorkspacePath(path) {
  try {
    const opened = await call("ide_open_path", { path });
    appendTerminal("info", `Opened ${opened}`);
    state.status = `Opened ${opened}`;
    render();
  } catch (error) {
    state.status = String(error);
    appendTerminal("error", String(error));
    render();
  }
}

function applyRun(result, options = {}) {
  if (result.check) applyCheck(result.check, options.checkSource ?? state.source);
  if (result.runtimeUpdated) {
    state.variables = options.mergeRuntime
      ? mergeRuntimeRows(state.variables, result.variables ?? [])
      : result.variables ?? [];
    state.args = options.mergeRuntime
      ? mergeRuntimeRows(state.args, result.args ?? [])
      : result.args ?? [];
    state.artifacts = result.artifacts ?? state.artifacts;
    state.inspectors = result.inspectors ?? state.inspectors ?? emptyInspectors();
    state.plotSpec = hasPlotData(result.plotSpec) ? result.plotSpec : null;
    state.reportTitle = result.reportTitle ?? "";
    if (state.plotSpec) state.sideTab = "plot";
  }
}

function terminalCommandUsesCurrentFile(command) {
  const lower = String(command || "").trim().toLowerCase();
  return lower === "check" || lower === "run";
}

function appendRunResult(result, context = {}) {
  const text = (result.terminal || "").trim();
  if (text) appendTerminal(result.ok ? "stdout" : "error", text);
  if (!result.ok) state.bottomTab = "problems";
  recordRunHistory(result, context);
}

function runHistoryContext(command) {
  return {
    command,
    profile: state.profile,
    sourcePath: state.currentPath,
    runDir: state.runDir
  };
}

function recordRunHistory(result, context = {}) {
  const artifactRoot = artifactRootForRun(result) || context.runDir || state.runDir || ".";
  const artifactKinds = (result.artifacts ?? [])
    .map((artifact) => artifact.kind)
    .filter(Boolean)
    .slice(0, 8);
  const entry = {
    timestamp: new Date().toLocaleString(),
    command: context.command || "run",
    profile: context.profile || state.profile || "normal",
    status: runHistoryStatus(result),
    sourcePath: context.sourcePath || state.currentPath || "-",
    artifactRoot,
    reportTitle: result.reportTitle || state.reportTitle || "",
    artifactKinds
  };
  state.runHistory.unshift(entry);
  if (state.runHistory.length > RUN_HISTORY_LIMIT) {
    state.runHistory.splice(RUN_HISTORY_LIMIT);
  }
  saveRunHistory();
}

function runHistoryStatus(result) {
  if (!result.ok) return "blocked";
  if (result.runtimeUpdated) return "completed";
  return "checked";
}

function artifactRootForRun(result) {
  const artifacts = result.artifacts ?? [];
  const preferred = artifacts.find((artifact) => artifact.kind === "output_manifest")
    || artifacts.find((artifact) => artifact.kind === "report")
    || artifacts[0];
  return preferred?.path ? directoryOf(preferred.path) : "";
}

function loadRunHistory(root) {
  try {
    const raw = window.localStorage?.getItem(runHistoryStorageKey(root));
    const rows = raw ? JSON.parse(raw) : [];
    return Array.isArray(rows) ? rows.slice(0, RUN_HISTORY_LIMIT) : [];
  } catch {
    return [];
  }
}

function saveRunHistory() {
  if (!state.root) return;
  try {
    window.localStorage?.setItem(
      runHistoryStorageKey(state.root),
      JSON.stringify(state.runHistory.slice(0, RUN_HISTORY_LIMIT))
    );
  } catch {
    // History is a convenience feature; storage errors should not block editing.
  }
}

function runHistoryStorageKey(root) {
  return `${RUN_HISTORY_STORAGE_PREFIX}${normalizePath(root || "workspace")}`;
}

function appendTerminal(kind, text) {
  state.terminalEntries.push({ kind, text: String(text ?? "") });
  if (state.terminalEntries.length > 300) {
    state.terminalEntries.splice(0, state.terminalEntries.length - 300);
  }
}

function clearTerminal() {
  state.terminalEntries = [{ kind: "info", text: "Terminal cleared." }];
}

function hasPlotData(spec) {
  if (!spec || typeof spec !== "object") return false;
  if (Array.isArray(spec.points) && spec.points.length) return true;
  if (Array.isArray(spec.bins) && spec.bins.length) return true;
  return Array.isArray(spec.series) && spec.series.some((series) => (
    (Array.isArray(series.points) && series.points.length) ||
    (Array.isArray(series.bins) && series.bins.length)
  ));
}

function mergeRuntimeRows(existingRows, incomingRows) {
  if (!incomingRows?.length) return existingRows ?? [];
  const rows = [...(existingRows ?? [])];
  for (const incoming of incomingRows) {
    const index = rows.findIndex((row) => runtimeRowKey(row) === runtimeRowKey(incoming));
    if (index >= 0) rows[index] = { ...rows[index], ...incoming };
    else rows.push(incoming);
  }
  return rows;
}

function runtimeRowKey(row) {
  return `${row?.name ?? ""}:${row?.line ?? ""}`;
}

function selectSourceLine(line) {
  const editor = byId("editor");
  if (!editor || !Number.isFinite(line) || line <= 0) return;
  const lineRange = sourceLineRange(editor.value, line - 1);
  editor.focus();
  editor.selectionStart = lineRange.start;
  editor.selectionEnd = lineRange.end;
  editor.scrollTop = Math.max(0, (lineRange.lineIndex - 3) * 20);
  updateCursorInsight();
}

function selectSourceTokenRange(line, startByte, lengthBytes) {
  const editor = byId("editor");
  if (
    !editor ||
    !Number.isFinite(line) ||
    !Number.isFinite(startByte) ||
    !Number.isFinite(lengthBytes) ||
    line <= 0 ||
    startByte < 0 ||
    lengthBytes <= 0
  ) {
    return;
  }
  const lineRange = sourceLineRange(editor.value, line - 1);
  const startColumn = byteOffsetToCodeUnit(lineRange.text, startByte);
  const endColumn = byteOffsetToCodeUnit(lineRange.text, startByte + lengthBytes);
  editor.focus();
  editor.selectionStart = lineRange.start + startColumn;
  editor.selectionEnd = lineRange.start + Math.max(startColumn, endColumn);
  editor.scrollTop = Math.max(0, (lineRange.lineIndex - 3) * 20);
  updateCursorInsight();
}

function sourceLineRange(source, requestedLineIndex) {
  const safeSource = String(source || "");
  const targetLine = Math.max(0, Number(requestedLineIndex) || 0);
  const newlinePattern = /\r\n|\r|\n/g;
  let start = 0;
  let lineIndex = 0;
  let match;
  while ((match = newlinePattern.exec(safeSource)) !== null) {
    if (lineIndex === targetLine) {
      return {
        lineIndex,
        start,
        end: match.index,
        text: safeSource.slice(start, match.index)
      };
    }
    start = match.index + match[0].length;
    lineIndex += 1;
  }
  return {
    lineIndex,
    start,
    end: safeSource.length,
    text: safeSource.slice(start)
  };
}

function rememberTerminalCommand(command) {
  if (!command) return;
  if (state.terminalCommands[state.terminalCommands.length - 1] !== command) {
    state.terminalCommands.push(command);
  }
  if (state.terminalCommands.length > 80) {
    state.terminalCommands.splice(0, state.terminalCommands.length - 80);
  }
  state.terminalHistoryIndex = null;
}

function recallTerminalCommand(direction) {
  const input = byId("terminalInput");
  if (!input || !state.terminalCommands.length) return;
  if (state.terminalHistoryIndex === null) {
    state.terminalHistoryIndex = state.terminalCommands.length;
  }
  state.terminalHistoryIndex = Math.max(
    0,
    Math.min(state.terminalCommands.length, state.terminalHistoryIndex + direction)
  );
  input.value = state.terminalCommands[state.terminalHistoryIndex] || "";
  input.selectionStart = input.value.length;
  input.selectionEnd = input.value.length;
}

function rememberCurrentTab() {
  if (!state.currentPath) return;
  const tab = tabFor(state.currentPath);
  if (!tab) {
    state.tabs.push({ path: state.currentPath, source: state.source, dirty: state.dirty });
    return;
  }
  tab.source = state.source;
  tab.dirty = state.dirty;
}

async function switchTab(path) {
  if (path === state.currentPath) return;
  rememberCurrentTab();
  const tab = tabFor(path);
  if (!tab) return;
  state.currentPath = tab.path;
  state.runDir = directoryOf(tab.path);
  openParentDirs(tab.path);
  state.source = tab.source;
  state.dirty = tab.dirty;
  state.variables = [];
  state.args = [];
  state.artifacts = [];
  state.inspectors = emptyInspectors();
  state.completionItems = [];
  state.plotSpec = null;
  state.reportTitle = "";
  state.status = `Loaded ${tab.path}`;
  try {
    const check = await call("ide_check", { path: state.currentPath, source: state.source });
    applyCheck(check, state.source);
  } catch (error) {
    state.status = String(error);
  }
  render();
}

function closeTab(path) {
  if (state.tabs.length <= 1) return;
  rememberCurrentTab();
  const index = state.tabs.findIndex((tab) => tab.path === path);
  if (index < 0) return;
  const wasCurrent = state.currentPath === path;
  state.tabs.splice(index, 1);
  if (!wasCurrent) {
    render();
    return;
  }
  const next = state.tabs[Math.max(0, index - 1)];
  state.currentPath = next.path;
  state.runDir = directoryOf(next.path);
  openParentDirs(next.path);
  state.source = next.source;
  state.dirty = next.dirty;
  state.variables = [];
  state.args = [];
  state.artifacts = [];
  state.inspectors = emptyInspectors();
  state.completionItems = [];
  state.plotSpec = null;
  state.reportTitle = "";
  call("ide_check", { path: state.currentPath, source: state.source })
    .then((check) => {
      applyCheck(check, state.source);
      state.status = `Loaded ${state.currentPath}`;
      render();
    })
    .catch((error) => {
      state.status = String(error);
      render();
    });
}

function tabFor(path) {
  return state.tabs.find((tab) => tab.path === path);
}

function renderSidePanel() {
  return `
    <aside class="variables inspector">
      <div class="side-tabs">
        ${sideTabButton("variables", "Vars")}
        ${sideTabButton("schema", "Schema")}
        ${sideTabButton("time", "Time")}
        ${sideTabButton("tables", "Tables")}
        ${sideTabButton("reads", "Reads")}
        ${sideTabButton("plot", "Plot")}
        ${sideTabButton("checks", "Checks")}
        ${sideTabButton("highlight", "Highlight")}
        ${sideTabButton("quality", "Quality")}
        ${sideTabButton("kernels", "Kernel")}
        ${sideTabButton("objects", "Obj")}
        ${sideTabButton("modules", "Modules")}
        ${sideTabButton("workflow", "Flow")}
        ${sideTabButton("assembly", "Asm")}
        ${sideTabButton("review", "Review")}
        ${sideTabButton("artifacts", "Artifacts")}
        ${sideTabButton("effects", "Effects")}
        ${sideTabButton("network", "Net")}
        ${sideTabButton("case", "Case")}
        ${sideTabButton("model", "Model")}
        ${sideTabButton("db", "DB")}
        ${sideTabButton("run", "Run")}
      </div>
      <div class="side-body">${renderSideBody()}</div>
    </aside>
  `;
}

function sideTabButton(key, label) {
  return `<button class="side-tab ${state.sideTab === key ? "active" : ""}" data-side-tab="${key}">${label}</button>`;
}

function renderSideBody() {
  if (state.sideTab === "plot") return renderPlotPanel();
  if (state.sideTab === "schema") return renderSchemaPanel();
  if (state.sideTab === "time") return renderTimePanel();
  if (state.sideTab === "tables") return renderTablesPanel();
  if (state.sideTab === "reads") return renderReadsPanel();
  if (state.sideTab === "checks") return renderChecksPanel();
  if (state.sideTab === "highlight") return renderHighlightPanel();
  if (state.sideTab === "quality") return renderQualityPanel();
  if (state.sideTab === "kernels") return renderKernelPanel();
  if (state.sideTab === "objects") return renderObjectsPanel();
  if (state.sideTab === "modules") return renderModulesPanel();
  if (state.sideTab === "workflow") return renderWorkflowPanel();
  if (state.sideTab === "assembly") return renderAssemblyPanel();
  if (state.sideTab === "review") return renderReviewPanel();
  if (state.sideTab === "artifacts") return renderArtifactsPanel();
  if (state.sideTab === "effects") return renderEffectsPanel();
  if (state.sideTab === "network") return renderNetworkPanel();
  if (state.sideTab === "case") return renderCasePanel();
  if (state.sideTab === "model") return renderModelPanel();
  if (state.sideTab === "db") return renderDbPanel();
  if (state.sideTab === "run") return renderRunPanel();
  return `
    <div class="panel-title compact">Variables</div>
    <div class="badges">
      <span class="badge">Source ${state.check.symbols.length}</span>
      <span class="badge">Run ${state.variables.length}</span>
      <span class="badge">Args ${state.args.length}</span>
    </div>
    <div class="scroll">${renderVariables()}</div>
  `;
}

function renderPlotPanel() {
  if (!state.plotSpec) {
    return `
      <div class="panel-title compact">Plot</div>
      <div class="empty-state">Run a file that produces a plot.</div>
    `;
  }
  return `
    <div class="panel-title compact">${escapeHtml(state.plotSpec.title || "Plot")}</div>
    <div class="side-plot">
      <canvas id="sidePlotCanvas"></canvas>
      <div class="plot-meta">
        <span>${escapeHtml(axisLabel(state.plotSpec.x_axis) || "x")}</span>
        <span>${escapeHtml(axisLabel(state.plotSpec.y_axis) || "y")}</span>
      </div>
      <button id="openPlotArtifact">Open SVG artifact</button>
    </div>
  `;
}

function panelArtifactEmptyState(title, command, artifact) {
  return `
    <div class="empty-state panel-empty-state">
      <strong>${escapeHtml(title)}</strong>
      <span>${escapeHtml(command)}</span>
      <code>${escapeHtml(artifact)}</code>
    </div>
  `;
}

function renderRunPanel() {
  return `
    <div class="panel-title compact">Run Context</div>
    <div class="run-info">
      <div><span>Workspace</span><code title="${escapeAttr(state.root)}">${escapeHtml(compactPath(state.root))}</code></div>
      <div><span>Run Dir</span><code>${escapeHtml(state.runDir || ".")}</code></div>
      <div><span>File Dir</span><code>${escapeHtml(currentDirectory())}</code></div>
      <div><span>File</span><code>${escapeHtml(state.currentPath || "-")}</code></div>
      <div><span>Profile</span><code>${escapeHtml(state.profile)}</code></div>
      <div><span>Status</span><code>${escapeHtml(state.check.status || "-")}</code></div>
      <div><span>Report</span><code>${escapeHtml(state.reportTitle || "-")}</code></div>
    </div>
    <div class="run-actions">
      <button data-open-artifact-kind="report">Open Report</button>
      <button data-open-artifact-kind="output_folder">Open Output</button>
    </div>
    <div class="panel-title compact">Run History</div>
    <div class="scroll run-history">${renderRunHistory()}</div>
  `;
}

function renderRunHistory() {
  if (!state.runHistory.length) {
    return `<div class="empty-state">Run a file to build execution history.</div>`;
  }
  return `
    <table class="artifact-table run-history-table">
      <thead><tr><th>Timestamp</th><th>Profile</th><th>Status</th><th>Source</th><th>Output Root</th></tr></thead>
      <tbody>
        ${state.runHistory.map((entry) => `
          <tr>
            <td>${escapeHtml(entry.timestamp || "-")}<div class="muted">${escapeHtml(compactText(entry.command || "run", 42))}</div></td>
            <td><code>${escapeHtml(entry.profile || "-")}</code></td>
            <td><span class="status-pill ${escapeAttr(entry.status || "")}">${escapeHtml(entry.status || "-")}</span></td>
            <td>
              <button class="link-button" data-open-file-path="${escapeAttr(entry.sourcePath || "-")}">
                <code title="${escapeAttr(entry.sourcePath || "-")}">${escapeHtml(compactPath(entry.sourcePath || "-"))}</code>
              </button>
            </td>
            <td>
              <button class="link-button" data-open-path="${escapeAttr(entry.artifactRoot || "-")}">
                <code title="${escapeAttr(entry.artifactRoot || "-")}">${escapeHtml(compactPath(entry.artifactRoot || "-"))}</code>
              </button>
              ${entry.reportTitle ? `<div class="muted">${escapeHtml(compactText(entry.reportTitle, 56))}</div>` : ""}
            </td>
          </tr>
        `).join("")}
      </tbody>
    </table>
  `;
}

function renderSchemaPanel() {
  return `
    <div class="panel-title compact">Schema</div>
    <div class="badges">
      <span class="badge">Schemas ${inspectorRows("schemas").length}</span>
      <span class="badge">Conversions ${inspectorRows("unitConversions").length}</span>
    </div>
    <div class="scroll">
      ${renderSchemas()}
      <div class="panel-title compact">Unit Conversions</div>
      ${renderUnitConversions()}
    </div>
  `;
}

function renderTimePanel() {
  return `
    <div class="panel-title compact">TimeSeries</div>
    <div class="badges">
      <span class="badge">Series ${inspectorRows("timeSeries").length}</span>
      <span class="badge">Axes ${inspectorRows("timeAxes").length}</span>
      <span class="badge">Coverage ${inspectorRows("timeSeriesCoverage").length}</span>
      <span class="badge">Alignments ${inspectorRows("timeAlignments").length}</span>
      <span class="badge">Solver ${solverTrajectoryRows().length}</span>
    </div>
    <div class="scroll">
      ${renderTimeAxes()}
      <div class="panel-title compact">Series</div>
      ${renderTimeSeries()}
      <div class="panel-title compact">Coverage</div>
      ${renderTimeSeriesCoverage()}
      <div class="panel-title compact">Solver Results</div>
      ${renderSolverTrajectories()}
    </div>
  `;
}

function renderTablesPanel() {
  const transforms = inspectorRows("tableTransforms");
  const rowDiagnostics = transforms.reduce((sum, transform) => {
    return sum + Number(transform.row_diagnostic_count ?? transform.rowDiagnosticCount ?? 0);
  }, 0);
  return `
    <div class="panel-title compact">Tables</div>
    <div class="badges">
      <span class="badge">Transforms ${transforms.length}</span>
      <span class="badge">Rows ${rowDiagnostics}</span>
    </div>
    <div class="scroll">${renderTableTransforms(transforms)}</div>
  `;
}

function renderReadsPanel() {
  const reads = inspectorRows("structuredReads");
  const configs = inspectorRows("configPromotions");
  const parsed = reads.filter((read) => (read.parse_status || read.parseStatus) === "parsed").length;
  return `
    <div class="panel-title compact">Structured Reads</div>
    <div class="badges">
      <span class="badge">Reads ${reads.length}</span>
      <span class="badge">Parsed ${parsed}</span>
      <span class="badge">Configs ${configs.length}</span>
    </div>
    <div class="scroll">
      ${renderStructuredReads(reads)}
      <div class="panel-title compact">Config Promotions</div>
      ${renderConfigPromotions(configs)}
    </div>
  `;
}

function renderChecksPanel() {
  return `
    <div class="panel-title compact">Metrics</div>
    <div class="scroll">
      ${renderMetrics()}
      <div class="panel-title compact">Validations</div>
      ${renderValidations()}
      <div class="panel-title compact">Time Alignment</div>
      ${renderAlignments()}
      <div class="panel-title compact">Systems</div>
      ${renderSystems()}
      <div class="panel-title compact">State-Space Operators</div>
      ${renderLinearOperators()}
      <div class="panel-title compact">System Dependency Graph</div>
      ${renderSystemDependencyGraph()}
    </div>
  `;
}

function renderHighlightPanel() {
  const semantic = semanticTokenPayload();
  const legend = semantic.legend || {};
  const tokens = Array.isArray(semantic.tokens) ? semantic.tokens : [];
  const filteredTokens = filteredSemanticTokens(tokens);
  const typeCounts = countSemanticTokens(filteredTokens, (token) => token.type || "-");
  const modifierCounts = countSemanticTokens(filteredTokens.flatMap((token) => token.modifiers || []), (modifier) => modifier || "-");
  const tokenCurrent = state.source === state.highlightSource;
  return `
    <div class="panel-title compact">Highlight Tokens</div>
    <div class="badges">
      <span class="badge">Tokens ${tokens.length}</span>
      <span class="badge">Shown ${filteredTokens.length}</span>
      <span class="badge">Types ${arrayOrEmpty(legend.token_types || legend.tokenTypes).length}</span>
      <span class="badge">Modifiers ${arrayOrEmpty(legend.token_modifiers || legend.tokenModifiers).length}</span>
      <span class="badge ${tokenCurrent ? "" : "warn"}">${tokenCurrent ? "Current" : "Check needed"}</span>
    </div>
    <div class="scroll highlight-panel">
      <div class="module-toolbar">
        <input id="highlightTokenQueryInput" class="module-query" value="${escapeAttr(state.highlightTokenQuery)}" placeholder="Filter highlight tokens" title="Filter by token text, type, modifier, or source line" />
        <button id="clearHighlightTokenFilter">Clear</button>
        <span class="muted">${filteredTokens.length} of ${tokens.length}</span>
      </div>
      <div class="panel-title compact">Token Types</div>
      ${renderSemanticLegendTable(arrayOrEmpty(legend.token_types || legend.tokenTypes), typeCounts, "type")}
      <div class="panel-title compact">Modifiers</div>
      ${renderSemanticLegendTable(arrayOrEmpty(legend.token_modifiers || legend.tokenModifiers), modifierCounts, "modifier")}
      <div class="panel-title compact">Current File Tokens</div>
      ${renderSemanticTokenRows(filteredTokens, Boolean(state.highlightTokenQuery.trim()))}
      ${rawJsonToggle("Raw semantic token JSON", semantic)}
    </div>
  `;
}

function renderSemanticLegendTable(items, counts, kind) {
  const rows = items.map((item) => `
    <tr>
      <td><span class="token-chip token-${escapeAttr(kind)}">${escapeHtml(item)}</span></td>
      <td>${escapeHtml(String(counts.get(item) || 0))}</td>
    </tr>
  `).join("");
  return `
    <table class="var-table semantic-legend-table">
      <thead><tr><th>${escapeHtml(kind === "type" ? "Type" : "Modifier")}</th><th>Count</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="2" class="muted">No semantic legend entries.</td></tr>`}</tbody>
    </table>
  `;
}

function renderSemanticTokenRows(tokens, filtered = false) {
  const rows = tokens.slice(0, 120).map((token) => {
    const start = Number(token.start ?? 0);
    const length = Number(token.length ?? 0);
    const modifiers = arrayOrEmpty(token.modifiers);
    return `
      <tr>
        <td>${sourceTokenButton(token)}<div class="muted">${escapeHtml(String(start))}:${escapeHtml(String(length))}</div></td>
        <td><code>${escapeHtml(semanticTokenText(token))}</code></td>
        <td><span class="token-chip token-type">${escapeHtml(token.type || "-")}</span></td>
        <td>${modifiers.length ? modifiers.map((modifier) => `<span class="token-chip token-modifier">${escapeHtml(modifier)}</span>`).join(" ") : "-"}</td>
      </tr>
    `;
  }).join("");
  const hidden = tokens.length > 120 ? `<div class="empty-state">Showing first 120 of ${escapeHtml(String(tokens.length))} tokens.</div>` : "";
  const empty = filtered ? "No semantic tokens match the current filter." : "No semantic tokens for the current check.";
  return `
    <table class="var-table semantic-token-table">
      <thead><tr><th>Range</th><th>Text</th><th>Type</th><th>Modifiers</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="4" class="muted">${escapeHtml(empty)}</td></tr>`}</tbody>
    </table>
    ${hidden}
  `;
}

function renderQualityPanel() {
  const quality = inspectorObject("quality");
  const summary = quality.summary || {};
  const results = Array.isArray(quality.results) ? quality.results : [];
  const failureCount = Number(quality.failureCount ?? quality.failure_count ?? 0);
  if (!Object.keys(summary).length && !results.length) {
    return `
      <div class="panel-title compact">Quality</div>
      ${panelArtifactEmptyState(
        "No quality artifact data yet.",
        "Run a file with validations, schema constraints, or expectation suites.",
        "result.engres typed_payload.quality_results[]"
      )}
    `;
  }
  return `
    <div class="panel-title compact">Quality</div>
    <div class="badges">
      <span class="badge">Status ${escapeHtml(summary.status || "-")}</span>
      <span class="badge">Results ${results.length}</span>
      <span class="badge">Failures ${failureCount}</span>
    </div>
    <div class="scroll">
      ${renderQualityResults(results)}
      ${rawJsonToggle("Raw quality JSON", quality)}
    </div>
  `;
}

function renderKernelPanel() {
  const plan = inspectorObject("kernelPlan");
  const selection = plan.backend_selection || plan.backendSelection || {};
  const candidates = Array.isArray(plan.candidates) ? plan.candidates : [];
  if (!Object.keys(plan).length) {
    return `
      <div class="panel-title compact">Kernel Plan</div>
      ${panelArtifactEmptyState(
        "No kernel plan artifact data yet.",
        "Run a file with supported solver or state-space work.",
        "report_spec.json kernel_plan"
      )}
    `;
  }
  const rows = candidates.map((candidate) => {
    const estimate = candidate.estimate || {};
    const executor = candidate.executor || {};
    const estimatedRows = estimate.estimated_rows ?? estimate.estimatedRows ?? "-";
    const counts = [
      `rows ${estimatedRows}`,
      `inputs ${estimate.input_count ?? estimate.inputCount ?? "-"}`,
      `outputs ${estimate.output_count ?? estimate.outputCount ?? "-"}`,
      `ops ${estimate.operation_count ?? estimate.operationCount ?? "-"}`,
      `scans ${estimate.scan_count ?? estimate.scanCount ?? "-"}`
    ].join(", ");
    return `
      <tr>
        <td><strong>${escapeHtml(candidate.name || "-")}</strong><div class="muted">${sourceLineButton(candidate)}</div></td>
        <td>${escapeHtml(candidate.kind || "-")}<div class="muted">${escapeHtml(candidate.lowering_status || candidate.loweringStatus || "-")}</div></td>
        <td><code>${escapeHtml(compactText(candidate.source || "-", 72))}</code></td>
        <td>${escapeHtml(executor.status || "-")}<div class="muted">${escapeHtml(executor.backend || "-")}</div></td>
        <td>${escapeHtml(executor.fallback_reason || executor.fallbackReason || "-")}</td>
        <td>${escapeHtml(counts)}<div class="muted">${escapeHtml(Array.isArray(candidate.operations) ? candidate.operations.join(", ") : "-")}</div></td>
      </tr>
    `;
  }).join("");
  return `
    <div class="panel-title compact">Kernel Plan</div>
    <div class="badges">
      <span class="badge">Candidates ${candidates.length}</span>
      <span class="badge">Backend ${escapeHtml(plan.backend || "-")}</span>
      <span class="badge">Status ${escapeHtml(selection.status || "-")}</span>
    </div>
    <div class="scroll">
      <table class="var-table">
        <thead><tr><th>Requested</th><th>Selected</th><th>Reason</th></tr></thead>
        <tbody><tr><td>${escapeHtml(selection.requested || "-")}</td><td>${escapeHtml(selection.selected || "-")}</td><td>${escapeHtml(selection.reason || "-")}</td></tr></tbody>
      </table>
      <table class="var-table">
        <thead><tr><th>Candidate</th><th>Kind</th><th>Source</th><th>Executor</th><th>Fallback</th><th>Estimate</th></tr></thead>
        <tbody>${rows || `<tr><td colspan="6" class="muted">No kernel plan candidates.</td></tr>`}</tbody>
      </table>
      ${rawJsonToggle("Raw kernel plan JSON", plan)}
    </div>
  `;
}

function renderAssemblyPanel() {
  const graph = inspectorObject("componentGraph");
  const components = Array.isArray(graph.components) ? graph.components.length : 0;
  const connections = Array.isArray(graph.connections) ? graph.connections.length : 0;
  const behaviorNodes = Array.isArray(graph.behavior_nodes)
    ? graph.behavior_nodes.length
    : (Array.isArray(graph.behaviorNodes) ? graph.behaviorNodes.length : 0);
  return `
    <div class="panel-title compact">Assembly</div>
    <div class="badges">
      <span class="badge">Graphs ${inspectorRows("assemblies").length}</span>
      <span class="badge">Components ${components}</span>
      <span class="badge">Connections ${connections}</span>
      <span class="badge">Behavior ${behaviorNodes}</span>
    </div>
    <div class="scroll">
      ${renderAssemblies()}
      <div class="panel-title compact">Equations</div>
      ${renderAssemblyEquations()}
      <div class="panel-title compact">Residuals</div>
      ${renderAssemblyResiduals()}
      <div class="panel-title compact">Residual Graph</div>
      ${renderAssemblyResidualGraph()}
      <div class="panel-title compact">Component Graph</div>
      ${renderComponentGraph()}
    </div>
  `;
}

function renderObjectsPanel() {
  return `
    <div class="panel-title compact">Objects</div>
    <div class="badges">
      <span class="badge">Objects ${inspectorRows("classObjects").length}</span>
    </div>
    <div class="scroll">${renderClassObjects()}</div>
  `;
}

function renderModulesPanel() {
  const native = state.modules.filter((module) => moduleStatusCategory(module) === "native").length;
  const planned = state.modules.filter((module) => moduleStatusCategory(module) === "planned").length;
  const internal = state.modules.filter((module) => moduleStatusCategory(module) === "internal").length;
  const filtered = filteredModules();
  return `
    <div class="panel-title compact">Modules</div>
    <div class="badges">
      <span class="badge">Total ${state.modules.length}</span>
      <span class="badge">Native ${native}</span>
      <span class="badge">Planned ${planned}</span>
      <span class="badge">Internal ${internal}</span>
    </div>
    <div class="module-toolbar">
      <div class="segmented">
        ${["all", "native", "planned", "internal"].map((category) => `
          <button class="${state.moduleCategory === category ? "active" : ""}" data-module-category="${escapeAttr(category)}">
            ${escapeHtml(moduleCategoryLabel(category))}
          </button>
        `).join("")}
      </div>
      <input id="moduleQueryInput" class="module-query" value="${escapeAttr(state.moduleQuery)}" placeholder="Filter modules" title="Filter by name, status, purpose, symbols, artifacts, examples, or diagnostics" />
      <button id="clearModuleFilters">Clear</button>
      <span class="muted">${filtered.length} of ${state.modules.length}</span>
    </div>
    <div class="scroll">${renderModules(filtered)}</div>
  `;
}

function renderWorkflowPanel() {
  const plan = inspectorObject("runPlan");
  if (!Object.keys(plan).length) {
    return `
      <div class="panel-title compact">Workflow</div>
      ${panelArtifactEmptyState(
        "No workflow plan artifact data yet.",
        "Run the current file to generate the workflow graph.",
        "build/result/run_plan.json"
      )}
    `;
  }
  const graph = plan.graph && typeof plan.graph === "object" ? plan.graph : {};
  const nodes = Array.isArray(graph.nodes) ? graph.nodes : [];
  const edges = Array.isArray(graph.edges) ? graph.edges : [];
  const decision = plan.rerun_decision || plan.rerunDecision || {};
  const selectedNode = selectedWorkflowNode(nodes);
  return `
    <div class="panel-title compact">Workflow</div>
    <div class="badges">
      <span class="badge">Nodes ${nodes.length}</span>
      <span class="badge">Edges ${edges.length}</span>
      <span class="badge">Status ${escapeHtml(plan.status || "-")}</span>
      <span class="badge">Profile ${escapeHtml(plan.execution_profile || plan.executionProfile || "-")}</span>
    </div>
    <div class="run-actions">
      <button data-open-artifact-kind="run_plan">Open run_plan.json</button>
    </div>
    <div class="scroll">
      <div class="panel-title compact">DAG</div>
      ${renderWorkflowDag(nodes, edges, selectedNode?.id)}
      <div class="panel-title compact">Node Detail</div>
      ${renderWorkflowNodeDetail(selectedNode, edges)}
      <div class="panel-title compact">Nodes</div>
      ${renderWorkflowNodes(nodes)}
      <div class="panel-title compact">Edges</div>
      ${renderWorkflowEdges(edges)}
      <div class="panel-title compact">Rerun</div>
      <table class="var-table">
        <thead><tr><th>Decision</th><th>Reason</th><th>Result Hash</th><th>Review Hash</th></tr></thead>
        <tbody><tr>
          <td>${escapeHtml(decision.decision || "-")}</td>
          <td>${escapeHtml(decision.reason || "-")}</td>
          <td><code>${escapeHtml(plan.artifact_hashes?.result || plan.artifactHashes?.result || "-")}</code></td>
          <td><code>${escapeHtml(plan.artifact_hashes?.review || plan.artifactHashes?.review || "-")}</code></td>
        </tr></tbody>
      </table>
    </div>
  `;
}

function renderReviewPanel() {
  const doc = inspectorObject("reviewDocument");
  const contract = doc.root_contract || doc.rootContract || {};
  const symbols = reviewArray(doc, "symbols");
  const units = reviewArray(doc, "units_quantities", "unitsQuantities");
  const schemas = reviewArray(doc, "schemas");
  const timeAxes = reviewArray(doc, "time_axes", "timeAxes");
  const calculations = reviewArray(doc, "calculations");
  const outputs = reviewArray(doc, "report_outputs", "reportOutputs");
  const validations = reviewArray(doc, "validations");
  const sideEffects = reviewArray(doc, "side_effects", "sideEffects");
  const boundaries = reviewArray(doc, "external_boundaries", "externalBoundaries");
  const fallbacks = reviewArray(doc, "fallbacks");
  const risks = reviewArray(doc, "risks");
  const sectionHashes = doc.section_hashes || doc.sectionHashes || {};
  return `
    <div class="panel-title compact">Review</div>
    <div class="badges">
      <span class="badge">Status ${escapeHtml(doc.status || "-")}</span>
      <span class="badge">Inputs ${escapeHtml(contract.input_count ?? contract.inputCount ?? 0)}</span>
      <span class="badge">Calc ${calculations.length}</span>
      <span class="badge">Sections ${Object.keys(sectionHashes).length}</span>
      <span class="badge">Risk ${risks.length}</span>
    </div>
    <div class="scroll">
      <table class="var-table">
        <thead><tr><th>Area</th><th>Count</th></tr></thead>
        <tbody>
          <tr><td>Symbols</td><td>${symbols.length || escapeHtml(contract.symbol_count ?? contract.symbolCount ?? 0)}</td></tr>
          <tr><td>Units</td><td>${units.length || escapeHtml(contract.unit_quantity_count ?? contract.unitQuantityCount ?? 0)}</td></tr>
          <tr><td>Schemas</td><td>${schemas.length || escapeHtml(contract.schema_count ?? contract.schemaCount ?? 0)}</td></tr>
          <tr><td>Time Axes</td><td>${timeAxes.length || escapeHtml(contract.time_axis_count ?? contract.timeAxisCount ?? 0)}</td></tr>
          <tr><td>Outputs</td><td>${outputs.length || escapeHtml(contract.report_output_count ?? contract.reportOutputCount ?? 0)}</td></tr>
          <tr><td>Validations</td><td>${validations.length || escapeHtml(contract.validation_count ?? contract.validationCount ?? 0)}</td></tr>
          <tr><td>Side Effects</td><td>${sideEffects.length || escapeHtml(contract.side_effect_count ?? contract.sideEffectCount ?? 0)}</td></tr>
          <tr><td>External Boundaries</td><td>${boundaries.length}</td></tr>
          <tr><td>Fallbacks</td><td>${fallbacks.length}</td></tr>
        </tbody>
      </table>
      <div class="panel-title compact">Semantic Hash</div>
      <table class="artifact-table">
        <tbody>
          <tr><td><code>${escapeHtml(doc.semantic_hash || doc.semanticHash || "-")}</code></td></tr>
        </tbody>
      </table>
      <div class="panel-title compact">Variables</div>
      ${renderReviewSymbols(symbols)}
      <div class="panel-title compact">Units</div>
      ${renderReviewUnits(units)}
      <div class="panel-title compact">Schemas</div>
      ${renderReviewSchemas(schemas)}
      <div class="panel-title compact">Time Axes</div>
      ${renderReviewTimeAxes(timeAxes)}
      <div class="panel-title compact">Calculations</div>
      ${renderReviewCalculations(calculations)}
      <div class="panel-title compact">Report Outputs</div>
      ${renderReviewOutputs(outputs)}
      <div class="panel-title compact">Validations</div>
      ${renderReviewValidations(validations)}
      <div class="panel-title compact">Side Effects</div>
      ${renderReviewSideEffects(sideEffects)}
      <div class="panel-title compact">External Boundaries</div>
      ${renderReviewBoundaries(boundaries)}
      <div class="panel-title compact">Fallbacks</div>
      ${renderReviewFallbacks(fallbacks)}
      <div class="panel-title compact">Risks</div>
      ${renderReviewRisks(risks)}
      ${rawJsonToggle("Raw review document JSON", doc)}
    </div>
  `;
}

function reviewArray(object, snakeKey, camelKey = snakeKey) {
  const value = object?.[snakeKey] ?? object?.[camelKey];
  return Array.isArray(value) ? value : [];
}

function reviewValue(object, snakeKey, camelKey = snakeKey, fallback = "-") {
  if (!object || typeof object !== "object") return fallback;
  const value = object[snakeKey] ?? object[camelKey];
  return value === null || value === undefined || value === "" ? fallback : value;
}

function reviewList(value, limit = 90) {
  if (!Array.isArray(value) || !value.length) return "-";
  return compactText(value.map((item) => {
    if (item && typeof item === "object") return compactObjectSummary(item);
    return String(item);
  }).join("; "), limit);
}

function renderReviewSymbols(symbols) {
  const rows = symbols.map((symbol) => `
    <tr>
      <td>${sourceLineButton(symbol)}</td>
      <td><strong>${escapeHtml(reviewValue(symbol, "name"))}</strong></td>
      <td>${escapeHtml(reviewValue(symbol, "quantity_kind", "quantityKind"))}</td>
      <td>${escapeHtml(reviewValue(symbol, "display_unit", "displayUnit"))}</td>
      <td>${escapeHtml(reviewValue(symbol, "source"))}</td>
    </tr>
  `).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Line</th><th>Name</th><th>Quantity</th><th>Unit</th><th>Source</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">No variables.</td></tr>`}</tbody>
    </table>
  `;
}

function renderReviewUnits(units) {
  const rows = units.map((unit) => `
    <tr>
      <td>${sourceLineButton(unit)}</td>
      <td><strong>${escapeHtml(reviewValue(unit, "name"))}</strong><div class="muted">${escapeHtml(reviewValue(unit, "quantity_kind", "quantityKind"))}</div></td>
      <td>${escapeHtml(reviewValue(unit, "source_unit", "sourceUnit"))}</td>
      <td>${escapeHtml(reviewValue(unit, "canonical_unit", "canonicalUnit"))}</td>
      <td>${escapeHtml(reviewValue(unit, "display_unit", "displayUnit"))}</td>
      <td>${escapeHtml(reviewList(reviewArray(unit, "derivation_steps", "derivationSteps"), 120))}</td>
    </tr>
  `).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Line</th><th>Name</th><th>Source</th><th>Canonical</th><th>Display</th><th>Derivation</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No unit records.</td></tr>`}</tbody>
    </table>
  `;
}

function renderReviewSchemas(schemas) {
  const rows = schemas.map((schema) => `
    <tr>
      <td>${sourceLineButton(schema)}</td>
      <td><strong>${escapeHtml(reviewValue(schema, "name"))}</strong></td>
      <td>${escapeHtml(columnSummary(reviewArray(schema, "columns")))}</td>
      <td>${escapeHtml(reviewList(reviewArray(schema, "missing_policies", "missingPolicies"), 120))}</td>
      <td>${escapeHtml(reviewList(reviewArray(schema, "constraints"), 120))}</td>
    </tr>
  `).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Line</th><th>Name</th><th>Columns</th><th>Missing</th><th>Constraints</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">No schemas.</td></tr>`}</tbody>
    </table>
  `;
}

function renderModules(modules = filteredModules()) {
  const rows = modules.map((module) => `
    <tr>
      <td><strong>${escapeHtml(module.name || "-")}</strong></td>
      <td><strong>${escapeHtml(moduleStatusLabel(module))}</strong><div class="muted">${escapeHtml(moduleStatusDetail(module))}</div><div class="muted">${escapeHtml(moduleStatusDisplay(module))} / ${escapeHtml(moduleBackingLabel(module))}</div></td>
      <td>${escapeHtml(compactText(module.purpose || "-", 120))}</td>
      <td>${escapeHtml(Array.isArray(module.symbols) && module.symbols.length ? module.symbols.join("; ") : "-")}</td>
      <td>${escapeHtml(Array.isArray(module.artifacts) && module.artifacts.length ? module.artifacts.join("; ") : "-")}</td>
    </tr>
  `).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Module</th><th>Status</th><th>Purpose</th><th>Symbols</th><th>Artifacts</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">${state.modules.length ? "No modules match the active filters." : "No module registry entries."}</td></tr>`}</tbody>
    </table>
  `;
}

function filteredModules() {
  const query = state.moduleQuery.trim().toLowerCase();
  return state.modules.filter((module) => {
    const categoryMatches = state.moduleCategory === "all" || moduleStatusCategory(module) === state.moduleCategory;
    const queryMatches = !query || moduleSearchText(module).includes(query);
    return categoryMatches && queryMatches;
  });
}

function moduleSearchText(module) {
  return [
    module.name,
    module.status,
    module.backing,
    moduleStatusLabel(module),
    moduleStatusDetail(module),
    module.purpose,
    ...(Array.isArray(module.symbols) ? module.symbols : []),
    ...(Array.isArray(module.artifacts) ? module.artifacts : []),
    ...(Array.isArray(module.diagnostics) ? module.diagnostics : []),
    ...(Array.isArray(module.examples) ? module.examples : []),
    ...(Array.isArray(module.tests) ? module.tests : [])
  ].filter(Boolean).join(" ").toLowerCase();
}

function moduleCategoryLabel(category) {
  switch (category) {
    case "native":
      return "Native";
    case "planned":
      return "Planned";
    case "internal":
      return "Internal";
    default:
      return "All";
  }
}

function moduleStatusCategory(module) {
  const status = String(module.status || "");
  if (status === "native_preview" || status === "supported_seed") return "native";
  if (status.startsWith("supported")) return "native";
  if (status.includes("internal")) return "internal";
  if (status.includes("planned")) return "planned";
  return "unknown";
}

function moduleStatusLabel(module) {
  if (module.statusLabel) return module.statusLabel;
  switch (module.status) {
    case "supported":
      return "Supported";
    case "supported_narrow":
      return "Supported narrow";
    case "native_preview":
      return "Native workflow support";
    case "supported_seed":
      return "Native workflow support";
    case "planned":
      return "Planned";
    case "internal_planned":
      return "Internal planned";
    case "internal":
      return "Internal";
    default:
      return module.status || "-";
  }
}

function moduleStatusDetail(module) {
  if (module.statusDetail) return module.statusDetail;
  switch (module.status) {
    case "supported":
      return "Public built-in surface supported by compiler/runtime.";
    case "supported_narrow":
      return "Supported for the listed syntax forms and review artifacts.";
    case "native_preview":
      return "Native runtime path is implemented for the listed workflow commands and artifacts; unsupported combinations report diagnostics.";
    case "supported_seed":
      return "Native runtime path is implemented for the listed workflow commands and artifacts; unsupported combinations report diagnostics.";
    case "planned":
      return "Documented target surface; not executable as a public module yet.";
    case "internal_planned":
      return "Internal design target, not a public stdlib contract.";
    case "internal":
      return "Internal compiler/runtime vocabulary, not a public stdlib contract.";
    default:
      return "-";
  }
}

function moduleStatusDisplay(module) {
  return module.statusLabel || moduleStatusLabel(module) || "-";
}

function moduleBackingLabel(module) {
  switch (module.backing) {
    case "compiler_runtime_builtin":
      return "Compiler/runtime";
    case "none":
      return "No executable backing";
    case "internal":
      return "Internal";
    default:
      return module.backing ? String(module.backing).replaceAll("_", " ") : "-";
  }
}

function renderReviewTimeAxes(timeAxes) {
  const rows = timeAxes.map((axis) => `
    <tr>
      <td>${sourceLineButton(axis)}</td>
      <td><strong>${escapeHtml(reviewValue(axis, "axis"))}</strong></td>
      <td>${escapeHtml(reviewValue(axis, "binding"))}</td>
      <td>${escapeHtml(reviewValue(axis, "role"))}</td>
      <td>${escapeHtml(reviewValue(axis, "source"))}</td>
    </tr>
  `).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Line</th><th>Axis</th><th>Binding</th><th>Role</th><th>Source</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">No time axes.</td></tr>`}</tbody>
    </table>
  `;
}

function renderReviewCalculations(calculations) {
  const rows = calculations.map((calculation) => `
    <tr>
      <td>${sourceLineButton(calculation)}</td>
      <td><strong>${escapeHtml(reviewValue(calculation, "name"))}</strong><div class="muted">${escapeHtml(reviewValue(calculation, "kind"))}</div></td>
      <td>${escapeHtml(compactText(reviewValue(calculation, "expression"), 90))}</td>
      <td>${escapeHtml(reviewList(reviewArray(calculation, "input_symbols", "inputSymbols"), 80))}</td>
      <td>${escapeHtml(reviewValue(calculation, "output_quantity", "outputQuantity"))}</td>
      <td>${escapeHtml(reviewList(reviewArray(calculation, "unit_derivation", "unitDerivation"), 100))}</td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Line</th><th>Name</th><th>Expression</th><th>Inputs</th><th>Output</th><th>Unit</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No calculations.</td></tr>`}</tbody>
    </table>
  `;
}

function renderReviewOutputs(outputs) {
  const rows = outputs.map((output) => `
    <tr>
      <td>${sourceLineButton(output)}</td>
      <td><strong>${escapeHtml(reviewValue(output, "kind"))}</strong></td>
      <td>${escapeHtml(reviewValue(output, "source"))}</td>
      <td>${escapeHtml(reviewValue(output, "quantity_kind", "quantityKind"))}</td>
      <td>${escapeHtml(reviewList(reviewArray(output, "statistics"), 100))}</td>
      <td>${escapeHtml(reviewValue(output, "status"))}</td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Line</th><th>Kind</th><th>Source</th><th>Quantity</th><th>Stats</th><th>Status</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No report outputs.</td></tr>`}</tbody>
    </table>
  `;
}

function renderReviewValidations(validations) {
  const rows = validations.map((validation) => `
    <tr>
      <td>${sourceLineButton(validation)}</td>
      <td><strong>${escapeHtml(reviewValue(validation, "target", "name"))}</strong></td>
      <td>${escapeHtml(reviewValue(validation, "kind", "category"))}</td>
      <td>${escapeHtml(reviewValue(validation, "status"))}</td>
      <td>${escapeHtml(compactText(reviewValue(validation, "reason", "summary"), 110))}</td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Line</th><th>Target</th><th>Kind</th><th>Status</th><th>Reason</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">No validations.</td></tr>`}</tbody>
    </table>
  `;
}

function renderReviewSideEffects(sideEffects) {
  const rows = sideEffects.map((effect) => `
    <tr>
      <td>${sourceLineButton(effect)}</td>
      <td><strong>${escapeHtml(reviewValue(effect, "kind"))}</strong></td>
      <td><code>${escapeHtml(compactText(reviewValue(effect, "target", "path"), 80))}</code></td>
      <td>${escapeHtml(reviewValue(effect, "status"))}</td>
      <td>${escapeHtml(reviewValue(effect, "risk_level", "riskLevel"))}</td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Line</th><th>Kind</th><th>Target</th><th>Status</th><th>Risk</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">No side effects.</td></tr>`}</tbody>
    </table>
  `;
}

function renderReviewBoundaries(boundaries) {
  const rows = boundaries.map((boundary) => `
    <tr>
      <td>${sourceLineButton(boundary)}</td>
      <td><strong>${escapeHtml(reviewValue(boundary, "name", "kind"))}</strong><div class="muted">${escapeHtml(reviewValue(boundary, "kind"))}</div></td>
      <td><code>${escapeHtml(compactText(reviewValue(boundary, "target"), 70))}</code></td>
      <td>${escapeHtml(reviewList(reviewArray(boundary, "outputs"), 80))}</td>
      <td>${escapeHtml(reviewValue(boundary, "status"))}</td>
      <td>${escapeHtml(reviewValue(boundary, "risk_level", "riskLevel"))}</td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Line</th><th>Name</th><th>Target</th><th>Outputs</th><th>Status</th><th>Risk</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No external boundaries.</td></tr>`}</tbody>
    </table>
  `;
}

function renderReviewFallbacks(fallbacks) {
  const rows = fallbacks.map((fallback) => `
    <tr>
      <td>${sourceLineButton(fallback)}</td>
      <td><strong>${escapeHtml(reviewValue(fallback, "kind"))}</strong></td>
      <td>${escapeHtml(reviewValue(fallback, "target"))}</td>
      <td>${escapeHtml(reviewValue(fallback, "method"))}</td>
      <td>${escapeHtml(compactText(reviewValue(fallback, "assumption", "reason"), 90))}</td>
      <td>${escapeHtml(reviewValue(fallback, "risk_level", "riskLevel"))}</td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Line</th><th>Kind</th><th>Target</th><th>Method</th><th>Assumption</th><th>Risk</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No fallbacks.</td></tr>`}</tbody>
    </table>
  `;
}

function renderReviewRisks(risks) {
  const rows = risks.map((risk) => `
    <tr>
      <td>${sourceLineButton(risk)}</td>
      <td><strong>${escapeHtml(reviewValue(risk, "category"))}</strong></td>
      <td>${escapeHtml(reviewValue(risk, "level"))}</td>
      <td>${escapeHtml(reviewValue(risk, "severity"))}</td>
      <td>${escapeHtml(compactText(reviewValue(risk, "summary"), 110))}</td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Line</th><th>Category</th><th>Level</th><th>Severity</th><th>Summary</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">No review risks.</td></tr>`}</tbody>
    </table>
  `;
}

function renderArtifactsPanel() {
  return `
    <div class="panel-title compact">Artifacts</div>
    <div class="scroll">
      ${renderArtifacts()}
      <div class="panel-title compact">Outlines</div>
      ${renderArtifactOutlines()}
    </div>
  `;
}

function renderArtifacts() {
  if (!state.artifacts.length) {
    return `<div class="empty-state">Run a file to inspect runtime objects.</div>`;
  }
  return `
    <table class="artifact-table">
      <thead><tr><th>Kind</th><th>Status</th><th>Path</th></tr></thead>
      <tbody>
        ${state.artifacts.map((artifact) => `
          <tr>
            <td>${escapeHtml(artifact.kind)}</td>
            <td>${escapeHtml(artifact.status)}</td>
            <td>${openPathButton(artifact.path, 90)}</td>
          </tr>
        `).join("")}
      </tbody>
    </table>
  `;
}

function renderStructuredReads(reads) {
  const rows = reads.map((read) => {
    const status = read.parse_status || read.parseStatus || "-";
    const rootType = read.root_type || read.rootType || "-";
    const fieldCount = read.field_count ?? read.fieldCount;
    const itemCount = read.item_count ?? read.itemCount;
    const shape = `${rootType}${fieldCount != null ? ` fields=${fieldCount}` : ""}${itemCount != null ? ` items=${itemCount}` : ""}`;
    const error = read.error ? `<div class="muted">${escapeHtml(compactText(read.error, 90))}</div>` : "";
    return `
      <tr>
        <td><strong>${escapeHtml(read.binding || "-")}</strong><div class="muted">${sourceLineButton(read)}</div></td>
        <td>${escapeHtml(read.kind || "-")}</td>
        <td>${openPathButton(read.path, 72)}</td>
        <td>${escapeHtml(status)}${error}</td>
        <td>${escapeHtml(shape)}</td>
        <td><code>${escapeHtml(compactText(read.source_hash || read.sourceHash || "-", 64))}</code></td>
      </tr>
    `;
  }).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Binding</th><th>Kind</th><th>Path</th><th>Status</th><th>Shape</th><th>Hash</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">Run a file with read text/json/toml to inspect structured inputs.</td></tr>`}</tbody>
    </table>
  `;
}

function renderConfigPromotions(configs) {
  const rows = configs.map((config) => {
    const missing = config.missing_fields || config.missingFields || [];
    const unknown = config.unknown_fields || config.unknownFields || [];
    const optionalMissing = config.optional_missing_fields || config.optionalMissingFields || [];
    const optionalNull = config.optional_null_fields || config.optionalNullFields || [];
    const fieldCount = config.field_count ?? config.fieldCount ?? "-";
    const policy = [
      missing.length ? `missing=${missing.length}` : "",
      unknown.length ? `unknown=${unknown.length}` : "",
      optionalMissing.length ? `optional missing=${optionalMissing.length}` : "",
      optionalNull.length ? `optional null=${optionalNull.length}` : ""
    ].filter(Boolean).join("; ") || "ok";
    return `
      <tr>
        <td><strong>${escapeHtml(config.binding || "-")}</strong><div class="muted">${sourceLineButton(config)}</div></td>
        <td>${escapeHtml(config.format || "-")}<div class="muted">${escapeHtml(config.schema_name || config.schemaName || "-")}</div></td>
        <td><code>${escapeHtml(compactText(config.source || config.source_literal || config.sourceLiteral || "-", 64))}</code><div class="muted">${openPathButton(config.resolved_path || config.resolvedPath, 72)}</div></td>
        <td>${escapeHtml(config.status || "-")}<div class="muted">fields ${escapeHtml(fieldCount)}</div></td>
        <td>${escapeHtml(compactText(policy, 80))}</td>
        <td><code>${escapeHtml(compactText(config.source_hash || config.sourceHash || "-", 64))}</code></td>
      </tr>
    `;
  }).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Binding</th><th>Format</th><th>Source</th><th>Status</th><th>Policy</th><th>Hash</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No typed config promotions.</td></tr>`}</tbody>
    </table>
  `;
}

function renderSchemas() {
  const rows = inspectorRows("schemas").map((schema) => `
    <tr>
      <td><strong>${escapeHtml(schema.name || "-")}</strong><div class="muted">${sourceLineButton(schema)}</div></td>
      <td>${escapeHtml(schema.row_count ?? schema.rowCount ?? "-")}</td>
      <td>${escapeHtml(schema.date_time_index || schema.dateTimeIndex || "-")}</td>
      <td>${escapeHtml(columnSummary(schema.columns))}</td>
      <td>${escapeHtml(schema.parse_failure_count ?? schema.parseFailureCount ?? 0)} / ${escapeHtml(schema.conversion_failure_count ?? schema.conversionFailureCount ?? 0)}</td>
      <td><code>${escapeHtml(schema.source_hash || schema.sourceHash || "-")}</code></td>
    </tr>
  `).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Name</th><th>Rows</th><th>Index</th><th>Columns</th><th>Parse/Conv</th><th>Hash</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">Run a CSV workflow.</td></tr>`}</tbody>
    </table>
  `;
}

function renderUnitConversions() {
  const rows = inspectorRows("unitConversions").map((item) => `
    <tr>
      <td><strong>${escapeHtml(item.name || "-")}</strong><div class="muted">${escapeHtml(item.quantity_kind || item.quantityKind || "-")}</div></td>
      <td>${escapeHtml(item.source_unit ?? item.sourceUnit ?? "-")}</td>
      <td>${escapeHtml(item.canonical_unit || item.canonicalUnit || "-")}</td>
      <td>${escapeHtml(item.display_unit || item.displayUnit || "-")}</td>
      <td>${escapeHtml(Array.isArray(item.steps) ? item.steps.join("; ") : "-")}</td>
    </tr>
  `).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Name</th><th>Source</th><th>Canonical</th><th>Display</th><th>Expression</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">No conversion records.</td></tr>`}</tbody>
    </table>
  `;
}

function renderTableTransforms(transforms = inspectorRows("tableTransforms")) {
  const rows = transforms.map((transform) => {
    const line = sourceLineButton(transform);
    const operation = transform.operation || "-";
    const source = transform.secondary_table || transform.secondaryTable
      ? `${transform.source_table || transform.sourceTable || "-"} + ${transform.secondary_table || transform.secondaryTable}`
      : (transform.source_table || transform.sourceTable || "-");
    return `
      <tr>
        <td><strong>${escapeHtml(transform.binding || "-")}</strong><div class="muted">${line}</div></td>
        <td>${escapeHtml(operation)}<div class="muted">${escapeHtml(source)}</div><div class="muted">${escapeHtml(transform.schema_name || transform.schemaName || "-")}</div></td>
        <td>${escapeHtml(tableTransformRowSummary(transform))}</td>
        <td>${escapeHtml(tablePredicateSummary(transform.predicates))}</td>
        <td>${escapeHtml(tableTransformShapeSummary(transform))}</td>
        <td>${escapeHtml(tableRowDiagnosticsSummary(transform))}</td>
        <td><strong>${escapeHtml(transform.status || "-")}</strong><div class="muted">${escapeHtml(transform.contract_status || transform.contractStatus || "-")}</div><div class="muted">${escapeHtml(compactText(transform.reason || "-", 80))}</div></td>
      </tr>
    `;
  }).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Binding</th><th>Operation</th><th>Rows</th><th>Predicates</th><th>Shape</th><th>Row Diagnostics</th><th>Status</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="7" class="muted">Run a table workflow.</td></tr>`}</tbody>
    </table>
  `;
}

function tableTransformRowSummary(transform) {
  const input = transform.input_row_count ?? transform.inputRowCount ?? "-";
  const output = transform.output_row_count ?? transform.outputRowCount ?? "-";
  const pairCount = transform.matched_pair_count ?? transform.matchedPairCount;
  const secondary = transform.secondary_input_row_count ?? transform.secondaryInputRowCount;
  const matched = Array.isArray(transform.matched_row_indices)
    ? transform.matched_row_indices
    : (Array.isArray(transform.matchedRowIndices) ? transform.matchedRowIndices : []);
  const parts = [`${input} -> ${output}`];
  if (secondary !== null && secondary !== undefined) parts.push(`secondary ${secondary}`);
  if (pairCount !== null && pairCount !== undefined) parts.push(`pairs ${pairCount}`);
  if (matched.length) parts.push(`matched ${matched.slice(0, 6).join(", ")}${matched.length > 6 ? " ..." : ""}`);
  return parts.join("; ");
}

function tablePredicateSummary(predicates) {
  if (!Array.isArray(predicates) || !predicates.length) return "-";
  return compactText(predicates.map((predicate) => {
    const expression = predicate.expression || "-";
    const resolved = predicate.resolved_value ?? predicate.resolvedValue ?? predicate.value;
    const suffix = resolved === null || resolved === undefined ? "" : ` => ${resolved}`;
    return `${expression}${suffix}`;
  }).join("; "), 130);
}

function tableTransformShapeSummary(transform) {
  const parts = [];
  const selected = transform.selected_columns || transform.selectedColumns || [];
  const derived = transform.derived_columns || transform.derivedColumns || [];
  const sortKeys = transform.sort_keys || transform.sortKeys || [];
  const joinKeys = transform.join_keys || transform.joinKeys || [];
  if (Array.isArray(selected) && selected.length) {
    parts.push(`select ${selected.map((column) => column.name || column).join(", ")}`);
  }
  if (Array.isArray(derived) && derived.length) {
    parts.push(`derive ${derived.map((column) => column.name || "-").join(", ")}`);
  }
  if (Array.isArray(sortKeys) && sortKeys.length) {
    parts.push(`sort ${sortKeys.map((key) => `${key.column || "-"} ${key.direction || ""}`.trim()).join(", ")}`);
  }
  if (Array.isArray(joinKeys) && joinKeys.length) {
    parts.push(`join ${joinKeys.map((key) => key.expression || `${key.left_table || key.leftTable || "left"}.${key.left_column || key.leftColumn || "key"} == ${key.right_table || key.rightTable || "right"}.${key.right_column || key.rightColumn || "key"}`).join(", ")}`);
  }
  return compactText(parts.join("; ") || "-", 130);
}

function tableRowDiagnosticsSummary(transform) {
  const summary = transform.row_diagnostic_summary || transform.rowDiagnosticSummary;
  if (Array.isArray(summary) && summary.length) {
    return summary.map((item) => `${item.status || "-"} ${item.count ?? 0}`).join("; ");
  }
  const preview = transform.row_diagnostics_preview || transform.rowDiagnosticsPreview;
  if (Array.isArray(preview) && preview.length) {
    const counts = new Map();
    preview.forEach((row) => {
      const status = row.status || "unknown";
      counts.set(status, (counts.get(status) || 0) + 1);
    });
    return [...counts.entries()].map(([status, count]) => `${status} ${count}`).join("; ");
  }
  return "-";
}

function renderTimeAxes() {
  const rows = inspectorRows("timeAxes").map((axis) => {
    const status = axis.irregular ? "irregular" : "regular";
    return `
    <tr>
      <td><strong>${escapeHtml(axis.name || "-")}</strong><div class="muted">${escapeHtml(axis.source_table || axis.sourceTable || "-")}.${escapeHtml(axis.source_column || axis.sourceColumn || "-")}</div></td>
      <td>${metricCell(axis.start)} - ${metricCell(axis.end)}<div class="muted">${escapeHtml(axis.unit || "-")}</div></td>
      <td>${escapeHtml(axis.count ?? "-")}</td>
      <td>${metricCell(axis.nominal_step ?? axis.nominalStep)}</td>
      <td>${escapeHtml(axis.missing_count ?? axis.missingCount ?? 0)}</td>
      <td><strong>${escapeHtml(status)}</strong></td>
    </tr>
  `;
  }).join("");
  return `
    <div class="panel-title compact">Time Axis</div>
    <table class="var-table">
      <thead><tr><th>Name</th><th>Range</th><th>Count</th><th>Step</th><th>Missing</th><th>Status</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No time-axis metadata.</td></tr>`}</tbody>
    </table>
  `;
}

function renderTimeSeries() {
  const rows = inspectorRows("timeSeries").map((series) => `
    <tr>
      <td><strong>${escapeHtml(series.name || "-")}</strong><div class="muted">${escapeHtml(series.axis || "-")}</div></td>
      <td>${escapeHtml(series.start_time || series.startTime || "-")}<div class="muted">${escapeHtml(series.end_time || series.endTime || "-")}</div></td>
      <td>${escapeHtml(series.timestep || "-")}</td>
      <td>${escapeHtml(series.row_count ?? series.rowCount ?? "-")}<div class="muted">missing ${escapeHtml(series.missing_count ?? series.missingCount ?? 0)}</div></td>
      <td>${escapeHtml(series.display_unit || series.displayUnit || "-")}</td>
      <td>${metricCell(series.mean)} / ${metricCell(series.max)}</td>
    </tr>
  `).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Name</th><th>Range</th><th>Step</th><th>Rows</th><th>Unit</th><th>Mean/Max</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">Run a TimeSeries workflow.</td></tr>`}</tbody>
    </table>
  `;
}

function renderTimeSeriesCoverage() {
  const rows = inspectorRows("timeSeriesCoverage").map((coverage) => {
    const expected = coverage.expected_count ?? coverage.expectedCount ?? "-";
    const actual = coverage.actual_count ?? coverage.actualCount ?? "-";
    const missing = coverage.missing_count ?? coverage.missingCount ?? 0;
    const maxGap = coverage.max_gap ?? coverage.maxGap;
    const step = coverage.expected_step ?? coverage.expectedStep;
    const intervals = coverage.missing_intervals || coverage.missingIntervals || [];
    const intervalText = Array.isArray(intervals) && intervals.length
      ? intervals.slice(0, 3).map((interval) => `${metricCell(interval.start)}-${metricCell(interval.end)} (${interval.missing_count ?? interval.missingCount ?? "?"})`).join("; ")
      : "-";
    return `
      <tr>
        <td><strong>${escapeHtml(coverage.binding || coverage.name || "-")}</strong><div class="muted">${sourceLineButton(coverage)}</div></td>
        <td>${escapeHtml(coverage.source_table || coverage.sourceTable || "-")}.${escapeHtml(coverage.source_column || coverage.sourceColumn || "-")}<div class="muted">${escapeHtml(coverage.source_start || coverage.sourceStart || "-")} - ${escapeHtml(coverage.source_end || coverage.sourceEnd || "-")}</div></td>
        <td>${escapeHtml(actual)} / ${escapeHtml(expected)}<div class="muted">missing ${escapeHtml(missing)}</div></td>
        <td>${metricCell(step)}<div class="muted">max gap ${metricCell(maxGap)}</div></td>
        <td>${escapeHtml(coverage.status || "-")}<div class="muted">${escapeHtml(coverage.coverage_year ?? coverage.coverageYear ?? "-")} ${escapeHtml(coverage.leap_year_policy || coverage.leapYearPolicy || "-")}</div></td>
        <td>${escapeHtml(compactText(intervalText, 110))}</td>
      </tr>
    `;
  }).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Coverage</th><th>Source</th><th>Actual/Expected</th><th>Step</th><th>Status</th><th>Missing Intervals</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No TimeSeries coverage records.</td></tr>`}</tbody>
    </table>
  `;
}

function renderSolverTrajectories() {
  const rows = solverTrajectoryRows().map((row) => {
    return `
      <tr>
        <td><strong>${escapeHtml(row.owner || "-")}</strong><div class="muted">${escapeHtml(row.binding || row.kind || "-")}</div></td>
        <td>${escapeHtml(row.states)}<div class="muted">${escapeHtml(row.stateDetail)}</div></td>
        <td>${escapeHtml(row.algebraic)}</td>
        <td>${escapeHtml(row.inputs)}</td>
        <td>${escapeHtml(row.outputs)}</td>
        <td>${escapeHtml(row.status)}<div class="muted">${escapeHtml(row.method)}</div><div class="muted">${escapeHtml(row.convergence)}</div></td>
        <td>${escapeHtml(row.pointCount)}<div class="muted">${escapeHtml(row.step)} / ${escapeHtml(row.duration)}</div></td>
        <td>${escapeHtml(row.finalValue)}<div class="muted">${escapeHtml(row.failure)}</div></td>
      </tr>
    `;
  }).join("");
  return `
    <table class="var-table">
      <thead><tr><th>System</th><th>State Traj</th><th>Algebraic Traj</th><th>Input Series</th><th>Output Series</th><th>Solver</th><th>Points</th><th>Final</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="8" class="muted">No solver trajectories.</td></tr>`}</tbody>
    </table>
  `;
}

function solverTrajectoryRows() {
  const systemRows = systemSolverResults().map(({ system, result }) => {
    const states = stringList(result, "states", "states");
    const state = result.state || "-";
    const unit = result.display_unit || result.displayUnit || "";
    const stepDiagnostics = systemStepDiagnostics(result);
    const outputStep = result.time_step_s ?? result.timeStepS ?? result.time_step ?? result.timeStep ?? "-";
    const substeps = systemStepSummary(stepDiagnostics);
    return {
      kind: "system",
      owner: system.name || "-",
      binding: result.binding || "-",
      states: joinOrDash(states.length ? states : [state]),
      stateDetail: `${state} ${unit}`.trim(),
      algebraic: joinOrDash(stringList(result, "algebraic_variables", "algebraicVariables")),
      inputs: joinOrDash(stringList(result, "inputs", "inputs")),
      outputs: joinOrDash(stringList(result, "outputs", "outputs")),
      status: result.status || "-",
      method: result.method || "-",
      convergence: result.convergence_status || result.convergenceStatus || "-",
      pointCount: Array.isArray(result.points) ? String(result.points.length) : "-",
      step: substeps === "-" ? outputStep : `${outputStep}; ${substeps}`,
      duration: result.duration_s ?? result.durationS ?? result.duration ?? "-",
      finalValue: metricCell(result.final_value ?? result.finalValue),
      failure: result.failure_reason ?? result.failureReason ?? "-"
    };
  });
  const componentRows = componentSolverResults().map(({ assembly, result }) => {
    const trajectories = Array.isArray(result.trajectories) ? result.trajectories : [];
    const stateTrajectories = trajectories.filter((trajectory) => (trajectory.role || "") === "state");
    const algebraicTrajectories = trajectories.filter((trajectory) => (trajectory.role || "") === "algebraic");
    const stepDiagnostics = Array.isArray(result.step_diagnostics)
      ? result.step_diagnostics
      : (Array.isArray(result.stepDiagnostics) ? result.stepDiagnostics : []);
    const lastStep = stepDiagnostics[stepDiagnostics.length - 1] || {};
    const firstTrajectory = trajectories[0] || {};
    const failure = result.failure_reason
      || result.failureReason
      || result.failure_artifact?.message
      || result.failureArtifact?.message
      || "-";
    return {
      kind: "assembly",
      owner: assembly.name || "-",
      binding: result.reason || "component solver",
      states: joinOrDash(stateTrajectories.map((trajectory) => trajectory.name || "state")),
      stateDetail: componentSolverTrajectorySummary(stateTrajectories),
      algebraic: joinOrDash(algebraicTrajectories.map((trajectory) => trajectory.name || "algebraic")),
      inputs: "-",
      outputs: "-",
      status: result.status || "-",
      method: result.method || "-",
      convergence: result.convergence_status || result.convergenceStatus || "-",
      pointCount: componentTrajectoryPointSummary(trajectories),
      step: componentStepSummary(stepDiagnostics),
      duration: lastStep.time_s ?? lastStep.timeS ?? "-",
      finalValue: componentSolverTrajectorySummary(trajectories.length ? [firstTrajectory] : []),
      failure
    };
  });
  return [...systemRows, ...componentRows];
}

function systemSourceEquationSummary(result) {
  const equations = Array.isArray(result?.source_equations)
    ? result.source_equations
    : (Array.isArray(result?.sourceEquations) ? result.sourceEquations : []);
  if (!equations.length) return "-";
  const values = equations.slice(0, 3).map((equation) => {
    const line = equation.source_line ?? equation.sourceLine ?? "?";
    return `${equation.kind || "equation"}:${equation.target || "-"} L${line}`;
  });
  if (equations.length > values.length) values.push(`+${equations.length - values.length} more`);
  return values.join("; ");
}
function systemStepDiagnostics(result) {
  const diagnostics = result?.step_diagnostics ?? result?.stepDiagnostics;
  return Array.isArray(diagnostics) ? diagnostics : [];
}

function systemStepSummary(stepDiagnostics) {
  if (!Array.isArray(stepDiagnostics) || !stepDiagnostics.length) return "-";
  const accepted = stepDiagnostics.filter((diagnostic) => {
    return (diagnostic.status || "") === "accepted";
  }).length;
  const rejected = stepDiagnostics.length - accepted;
  const maxError = stepDiagnostics.reduce((current, diagnostic) => {
    const error = Number(diagnostic.error_norm ?? diagnostic.errorNorm ?? 0);
    return Number.isFinite(error) ? Math.max(current, Math.abs(error)) : current;
  }, 0);
  return `substeps ${stepDiagnostics.length}, accepted ${accepted}, rejected ${rejected}, max error ${fmt(maxError)}`;
}

function systemSolverResults() {
  return inspectorRows("systems").flatMap((system) => {
    const results = Array.isArray(system.solver_results)
      ? system.solver_results
      : (Array.isArray(system.solverResults) ? system.solverResults : []);
    return results.map((result) => ({ system, result }));
  });
}

function componentSolverResults() {
  return inspectorRows("assemblies").flatMap((assembly) => {
    const result = assembly.solver_result || assembly.solverResult;
    if (!result || typeof result !== "object") return [];
    const trajectories = Array.isArray(result.trajectories) ? result.trajectories : [];
    if (!trajectories.length) return [];
    return [{ assembly, result }];
  });
}

function componentTrajectoryPointSummary(trajectories) {
  if (!Array.isArray(trajectories) || !trajectories.length) return "-";
  return trajectories.map((trajectory) => {
    const count = trajectory.point_count ?? trajectory.pointCount ?? (
      Array.isArray(trajectory.points) ? trajectory.points.length : "-"
    );
    return `${trajectory.role || "trajectory"}:${trajectory.name || "var"} ${count}`;
  }).join(", ");
}

function componentStepSummary(stepDiagnostics) {
  if (!Array.isArray(stepDiagnostics) || !stepDiagnostics.length) return "-";
  const failed = stepDiagnostics.find((diagnostic) => {
    return diagnostic.failure_artifact || diagnostic.failureArtifact
      || diagnostic.failure_code || diagnostic.failureCode;
  });
  if (failed) {
    const failure = failed.failure_artifact || failed.failureArtifact || {};
    const code = failure.code || failed.failure_code || failed.failureCode || "failure";
    const step = failed.step_index ?? failed.stepIndex ?? "-";
    return `failed@${step} ${code}`;
  }
  if (stepDiagnostics.length < 2) {
    return stepDiagnostics[0]?.convergence_status
      || stepDiagnostics[0]?.convergenceStatus
      || "-";
  }
  const first = stepDiagnostics[0]?.time_s ?? stepDiagnostics[0]?.timeS;
  const second = stepDiagnostics[1]?.time_s ?? stepDiagnostics[1]?.timeS;
  if (Number.isFinite(Number(first)) && Number.isFinite(Number(second))) {
    return `${Number(second) - Number(first)} s`;
  }
  return "-";
}

function stringList(item, snakeName, camelName) {
  const values = item?.[snakeName] ?? item?.[camelName];
  return Array.isArray(values) ? values : [];
}

function joinOrDash(values) {
  return values.length ? values.join(", ") : "-";
}

function renderMetrics() {
  const rows = inspectorRows("metrics").map((metric) => {
    const alignmentReference = metric.alignment_reference ?? metric.alignmentReference ?? "-";
    const alignmentStatus = metric.alignment_status ?? metric.alignmentStatus ?? "-";
    const alignmentStepStatus = metric.alignment_step_status ?? metric.alignmentStepStatus ?? "-";
    return `
    <tr>
      <td><strong>${escapeHtml(metric.binding || "-")}</strong><div class="muted">${escapeHtml(metric.kind || "-")}</div></td>
      <td>${escapeHtml(metric.left || "-")} vs ${escapeHtml(metric.right || "-")}</td>
      <td>${metricCell(metric.value)} ${escapeHtml(metric.unit || "")}</td>
      <td>${escapeHtml(alignmentReference)}<div class="muted">${escapeHtml(alignmentStatus)} / ${escapeHtml(alignmentStepStatus)}</div></td>
      <td>${escapeHtml(metric.status || "-")}</td>
    </tr>
  `;
  }).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Name</th><th>Inputs</th><th>Value</th><th>Alignment</th><th>Status</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">No metrics.</td></tr>`}</tbody>
    </table>
  `;
}

function renderValidations() {
  const rows = inspectorRows("validations").map((item) => `
    <tr>
      <td><strong>${escapeHtml(item.status || "-")}</strong><div class="muted">${sourceLineButton(item)}</div></td>
      <td>${escapeHtml(item.expression || "-")}</td>
      <td>${metricCell(item.left_value ?? item.leftValue)} ${escapeHtml(item.unit || "")}</td>
      <td>${metricCell(item.right_value ?? item.rightValue)} ${escapeHtml(item.unit || "")}</td>
    </tr>
  `).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Status</th><th>Expression</th><th>Value</th><th>Threshold</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="4" class="muted">No validations.</td></tr>`}</tbody>
    </table>
  `;
}

function renderQualityResults(results = []) {
  const rows = results.map((item) => {
    const score = item.score === null || item.score === undefined ? "-" : metricCell(item.score);
    const counts = [
      item.passed_count ?? item.passedCount ?? 0,
      item.warning_count ?? item.warningCount ?? 0,
      item.failed_count ?? item.failedCount ?? 0
    ].join("/");
    return `
    <tr>
      <td><strong>${escapeHtml(item.status || "-")}</strong><div class="muted">${sourceLineButton(item)}</div></td>
      <td>${escapeHtml(item.binding || "-")}<div class="muted">${escapeHtml(item.category || "-")}</div></td>
      <td>${escapeHtml(item.subject || item.target || "-")}</td>
      <td>${escapeHtml(score)}<div class="muted">${escapeHtml(counts)}</div></td>
      <td>${escapeHtml(item.reason || "-")}</td>
      <td>${renderQualityFailures(item.failures)}</td>
    </tr>
  `;
  }).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Status</th><th>Result</th><th>Subject</th><th>Score P/W/F</th><th>Reason</th><th>Failures</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No quality results.</td></tr>`}</tbody>
    </table>
  `;
}

function renderQualityFailures(failures = []) {
  if (!Array.isArray(failures) || !failures.length) return `<span class="muted">-</span>`;
  return failures.slice(0, 5).map((failure) => `
    <div><strong>row ${escapeHtml(failure.row ?? "-")}</strong> ${escapeHtml(failure.field || "-")}: ${escapeHtml(failure.value || "-")}</div>
    <div class="muted">${escapeHtml(failure.message || "-")}</div>
  `).join("") + (failures.length > 5 ? `<div class="muted">+${failures.length - 5} more</div>` : "");
}

function renderAlignments() {
  const rows = inspectorRows("timeAlignments").map((item) => {
    const leftCount = item.left_count ?? item.leftCount ?? "-";
    const rightCount = item.right_count ?? item.rightCount ?? "-";
    const leftStep = item.left_nominal_step ?? item.leftNominalStep;
    const rightStep = item.right_nominal_step ?? item.rightNominalStep;
    const stepStatus = item.step_status ?? item.stepStatus ?? "-";
    const alignmentPass = item.status === "matched" && (stepStatus === "matched" || stepStatus === "-");
    return `
    <tr>
      <td><strong>${alignmentPass ? "PASS" : "FAIL"}</strong><div class="muted">${escapeHtml(item.status || "-")} / ${escapeHtml(item.axis || "-")}</div></td>
      <td>${escapeHtml(item.left || "-")}<div class="muted">${escapeHtml(item.right || "-")}</div></td>
      <td>${escapeHtml(item.matched_count ?? item.matchedCount ?? "-")}<div class="muted">${escapeHtml(leftCount)} / ${escapeHtml(rightCount)}</div></td>
      <td><strong>${escapeHtml(stepStatus)}</strong><div class="muted">${metricCell(leftStep)} / ${metricCell(rightStep)}</div></td>
      <td>${escapeHtml(item.overlap_start ?? item.overlapStart ?? "-")} - ${escapeHtml(item.overlap_end ?? item.overlapEnd ?? "-")}</td>
    </tr>
  `;
  }).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Alignment</th><th>Series</th><th>Matched</th><th>Step</th><th>Overlap</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">No alignment metadata.</td></tr>`}</tbody>
    </table>
  `;
}

function renderSystems() {
  const rows = inspectorRows("systems").map((system) => {
    const solverResults = Array.isArray(system.solver_results)
      ? system.solver_results
      : (Array.isArray(system.solverResults) ? system.solverResults : []);
    const solver = solverResults[0] || system.solver_result || system.solverResult || {};
    const stateNames = stringList(solver, "states", "states");
    const inputNames = stringList(solver, "inputs", "inputs");
    const parameterNames = stringList(solver, "parameters", "parameters");
    const algebraicNames = stringList(solver, "algebraic_variables", "algebraicVariables");
    const outputNames = stringList(solver, "outputs", "outputs");
    const stateLabel = stateNames.length
      ? stateNames.join(", ")
      : (solverResults.length > 1
        ? solverResults.map((item) => item.state || "-").join(", ")
        : (solver.state || "-"));
    const steps = solverResults.length > 1
      ? solverResults.map((item) => item.step_count ?? item.stepCount ?? "-").join(", ")
      : (solver.step_count ?? solver.stepCount ?? "-");
    const timeStep = solver.time_step_s ?? solver.timeStepS ?? solver.time_step ?? solver.timeStep ?? "-";
    const tolerance = solver.tolerance ?? "-";
    const iterations = `${solver.iteration_count ?? solver.iterationCount ?? "-"} / ${solver.max_iterations ?? solver.maxIterations ?? "-"}`;
    const convergence = solver.convergence_status ?? solver.convergenceStatus ?? "-";
    const failure = solver.failure_reason ?? solver.failureReason ?? "-";
    const sourceEquations = systemSourceEquationSummary(solver);
    const substeps = systemStepSummary(systemStepDiagnostics(solver));
    return `
      <tr>
        <td><strong>${escapeHtml(system.name || "-")}</strong><div class="muted">${sourceLineButton(system)}</div></td>
        <td>${escapeHtml(stateLabel)}<div class="muted">alg ${escapeHtml(joinOrDash(algebraicNames))}</div></td>
        <td>${escapeHtml(joinOrDash(inputNames))}<div class="muted">params ${escapeHtml(joinOrDash(parameterNames))}</div><div class="muted">outputs ${escapeHtml(joinOrDash(outputNames))}</div></td>
        <td>${escapeHtml(sourceEquations)}</td>
        <td>${escapeHtml(solver.status || "-")}<div class="muted">${escapeHtml(solver.method || "-")}</div></td>
        <td>${escapeHtml(timeStep)}<div class="muted">steps ${escapeHtml(steps)}</div><div class="muted">${escapeHtml(substeps)}</div></td>
        <td>${escapeHtml(tolerance)}<div class="muted">iter ${escapeHtml(iterations)}</div></td>
        <td>${escapeHtml(convergence)}<div class="muted">${escapeHtml(failure)}</div></td>
      </tr>
    `;
  }).join("");
  return `
    <table class="var-table">
      <thead><tr><th>System</th><th>States</th><th>Inputs/Params</th><th>Source Equations</th><th>Solver</th><th>Timestep</th><th>Tol/Iter</th><th>Convergence</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="8" class="muted">No system metadata.</td></tr>`}</tbody>
    </table>
  `;
}

function renderLinearOperators() {
  const rows = inspectorRows("linearOperators").map((operator) => {
    const rowMembers = Array.isArray(operator.row_members) ? operator.row_members : (operator.rowMembers || []);
    const columnMembers = Array.isArray(operator.column_members) ? operator.column_members : (operator.columnMembers || []);
    const rowUnits = Array.isArray(operator.row_units) ? operator.row_units : (operator.rowUnits || []);
    const columnUnits = Array.isArray(operator.column_units) ? operator.column_units : (operator.columnUnits || []);
    const canonicalMatrix = operator.canonical_matrix ?? operator.canonicalMatrix;
    const canonicalEntries = operator.canonical_entries ?? operator.canonicalEntries ?? [];
    return `
      <tr>
        <td><strong>${escapeHtml(operator.system || "-")}</strong><div class="muted">${sourceLineButton(operator)}</div></td>
        <td>${escapeHtml(operator.name || "-")}<div class="muted">${escapeHtml(operator.from || "-")} -> ${escapeHtml(operator.to || "-")}</div></td>
        <td>${escapeHtml(operator.row_count ?? operator.rowCount ?? 0)}x${escapeHtml(operator.column_count ?? operator.columnCount ?? 0)}</td>
        <td>${escapeHtml(joinOrDash(rowMembers))}<div class="muted">${escapeHtml(joinOrDash(rowUnits))}</div></td>
        <td>${escapeHtml(joinOrDash(columnMembers))}<div class="muted">${escapeHtml(joinOrDash(columnUnits))}</div></td>
        <td><code>${escapeHtml(compactText(operator.expression || "-", 60))}</code><div class="muted">${escapeHtml(matrixSummary(canonicalMatrix))}</div><div class="muted">${escapeHtml(entriesSummary(canonicalEntries))}</div></td>
        <td>${escapeHtml(operator.compatibility_status || operator.compatibilityStatus || "-")}<div class="muted">${escapeHtml(operator.status || "-")}</div></td>
      </tr>
    `;
  }).join("");
  return `
    <table class="var-table">
      <thead><tr><th>System</th><th>Operator</th><th>Shape</th><th>Rows</th><th>Columns</th><th>Matrix</th><th>Status</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="7" class="muted">No state-space operators.</td></tr>`}</tbody>
    </table>
  `;
}

function renderSystemDependencyGraph() {
  const rows = inspectorRows("systemIr").flatMap((system) => {
    const equations = Array.isArray(system.equations) ? system.equations : [];
    return equations.flatMap((equation) => {
      const dependencies = Array.isArray(equation.dependencies) ? equation.dependencies : [];
      if (!dependencies.length) {
        return [`
          <tr>
            <td><strong>${escapeHtml(system.name || "-")}</strong><div class="muted">${sourceLineButton(equation)}</div></td>
            <td>${escapeHtml(equation.residual || "-")}<div class="muted">${escapeHtml(equation.relation || "-")}</div></td>
            <td>-</td>
            <td>${escapeHtml(equation.normalized_residual || equation.normalizedResidual || "-")}</td>
            <td>${escapeHtml(Array.isArray(equation.derivative_states) ? equation.derivative_states.join(", ") : "-")}</td>
            <td>${escapeHtml(equation.status || "-")}</td>
          </tr>
        `];
      }
      return dependencies.map((dependency) => `
        <tr>
          <td><strong>${escapeHtml(system.name || "-")}</strong><div class="muted">${sourceLineButton(equation)}</div></td>
          <td>${escapeHtml(equation.residual || "-")}<div class="muted">${escapeHtml(equation.relation || "-")}</div></td>
          <td>${escapeHtml(dependency.name || "-")}<div class="muted">${escapeHtml(dependency.role || "-")}</div></td>
          <td>${escapeHtml(equation.normalized_residual || equation.normalizedResidual || "-")}</td>
          <td>${escapeHtml(Array.isArray(equation.derivative_states) ? equation.derivative_states.join(", ") : "-")}</td>
          <td>${escapeHtml(dependency.quantity_kind || dependency.quantityKind || equation.status || "-")}</td>
        </tr>
      `);
    });
  }).join("");
  return `
    <table class="var-table">
      <thead><tr><th>System</th><th>Residual</th><th>Variable</th><th>Normalized</th><th>Derivatives</th><th>Quantity/Status</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No system dependency graph.</td></tr>`}</tbody>
    </table>
  `;
}

function renderAssemblies() {
  const rows = inspectorRows("assemblies").map((assembly) => {
    const boundary = assembly.boundary || {};
    const residualGraph = assembly.residual_graph || assembly.residualGraph || {};
    const solverPreview = assembly.solver_preview || assembly.solverPreview || {};
    const setCount = Array.isArray(assembly.connection_sets)
      ? assembly.connection_sets.length
      : (assembly.connectionSets?.length ?? 0);
    const domainCount = assembly.domain_count ?? assembly.domainCount ?? 0;
    const limitations = Array.isArray(solverPreview.limitations)
      ? solverPreview.limitations.join(", ")
      : "-";
    const solverResult = assembly.solver_result || assembly.solverResult || {};
    const solverMethod = solverResult.method
      || solverPreview.method
      || residualGraph.solver_plan
      || residualGraph.solverPlan
      || "-";
    const solverStatus = solverResult.convergence_status
      || solverResult.convergenceStatus
      || solverPreview.status
      || boundary.balance_status
      || boundary.balanceStatus
      || "-";
    const failure = solverResult.failure_artifact || solverResult.failureArtifact || {};
    const failureReason = solverResult.failure_reason
      || solverResult.failureReason
      || failure.message
      || failure.reason
      || "-";
    const largestResiduals = solverResult.largest_residuals
      || solverResult.largestResiduals
      || solverResult.residuals;
    const tolerance = solverResult.tolerance ?? "-";
    const iterations = `${solverResult.iteration_count ?? solverResult.iterationCount ?? "-"} / ${solverResult.max_iterations ?? solverResult.maxIterations ?? "-"}`;
    return `
      <tr>
        <td><strong>${escapeHtml(assembly.name || "-")}</strong><div class="muted">${escapeHtml(assembly.status || "-")}</div></td>
        <td>${escapeHtml(assembly.component_count ?? assembly.componentCount ?? 0)} / ${escapeHtml(assembly.port_count ?? assembly.portCount ?? 0)}</td>
        <td>${escapeHtml(setCount)}<div class="muted">domains ${escapeHtml(domainCount)}</div></td>
        <td>${escapeHtml(Array.isArray(assembly.equations) ? assembly.equations.length : 0)}<div class="muted">component ${escapeHtml(assembly.component_equation_count ?? assembly.componentEquationCount ?? 0)}</div><div class="muted">unknowns ${escapeHtml(boundary.unknown_count ?? boundary.unknownCount ?? 0)}</div></td>
        <td>${escapeHtml(solverStatus)}<div class="muted">${escapeHtml(solverMethod)}</div><div class="muted">${escapeHtml(limitations)}</div></td>
        <td>${metricCell(solverResult.residual_norm ?? solverResult.residualNorm)}<div class="muted">tol ${escapeHtml(tolerance)}</div><div class="muted">iter ${escapeHtml(iterations)}</div></td>
        <td>${escapeHtml(componentSolverVariableSummary(solverResult.variables))}</td>
        <td>${escapeHtml(componentSolverTrajectorySummary(solverResult.trajectories))}</td>
        <td>${escapeHtml(componentSolverLargestResidual(largestResiduals))}<div class="muted">${escapeHtml(failure.code || "-")}</div><div class="muted">${escapeHtml(failureReason)}</div></td>
      </tr>
    `;
  }).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Graph</th><th>Comp/Ports</th><th>Sets</th><th>Eq</th><th>Solver</th><th>Residual</th><th>Variables</th><th>Trajectories</th><th>Largest</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="9" class="muted">Run a domain/component workflow.</td></tr>`}</tbody>
    </table>
  `;
}

function renderAssemblyEquations() {
  const rows = inspectorRows("assemblies").flatMap((assembly) => {
    const equations = Array.isArray(assembly.equations) ? assembly.equations : [];
    return equations.map((equation) => `
      <tr>
        <td><strong>${escapeHtml(assembly.name || "-")}</strong><div class="muted">${escapeHtml(equation.kind || "-")}</div></td>
        <td><code>${escapeHtml(equation.expression || "-")}</code><div class="muted">residual ${escapeHtml(equation.residual || "-")}</div></td>
        <td>${escapeHtml(Array.isArray(equation.dependencies) ? equation.dependencies.join(", ") : "-")}</td>
        <td>${escapeHtml(equation.reason || "-")}<div class="muted">${escapeHtml(equation.status || "-")}</div></td>
        <td>${sourceLineButton(equation)}</td>
      </tr>
    `);
  }).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Assembly</th><th>Generated Equation</th><th>Dependencies</th><th>Reason</th><th>Source</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">No generated equations.</td></tr>`}</tbody>
    </table>
  `;
}

function renderAssemblyResiduals() {
  const rows = inspectorRows("assemblies").flatMap((assembly) => {
    const solverResult = assembly.solver_result || assembly.solverResult || {};
    const residuals = Array.isArray(solverResult.residuals) ? solverResult.residuals : [];
    return residuals.map((residual) => {
      const normalized = residual.normalized_value ?? residual.normalizedValue;
      const scale = residual.scale ?? "-";
      const scalePolicy = residual.scale_policy ?? residual.scalePolicy ?? "-";
      return `
        <tr>
          <td><strong>${escapeHtml(assembly.name || "-")}</strong><div class="muted">${escapeHtml(residual.name || "-")}</div></td>
          <td><code>${escapeHtml(residual.expression || "-")}</code></td>
          <td>${metricCell(residual.value)} ${escapeHtml(residual.unit || "")}</td>
          <td>${metricCell(normalized)}<div class="muted">scale ${metricCell(scale)} ${escapeHtml(scalePolicy)}</div></td>
          <td>${escapeHtml(residual.status || "-")}</td>
        </tr>
      `;
    });
  }).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Assembly</th><th>Residual</th><th>Value</th><th>Normalized</th><th>Status</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">No evaluated residuals.</td></tr>`}</tbody>
    </table>
  `;
}

function renderAssemblyResidualGraph() {
  const rows = inspectorRows("assemblies").flatMap((assembly) => {
    const residualGraph = assembly.residual_graph || assembly.residualGraph || {};
    const dependencies = Array.isArray(residualGraph.dependencies) ? residualGraph.dependencies : [];
    if (!dependencies.length && Array.isArray(residualGraph.residuals) && residualGraph.residuals.length) {
      return residualGraph.residuals.map((residual) => `
        <tr>
          <td><strong>${escapeHtml(assembly.name || "-")}</strong><div class="muted">${escapeHtml(residualGraph.status || "-")}</div></td>
          <td>${escapeHtml(residual)}</td>
          <td>-</td>
          <td>${escapeHtml(residualGraph.solver_plan || residualGraph.solverPlan || "-")}</td>
        </tr>
      `);
    }
    return dependencies.map((dependency) => `
      <tr>
        <td><strong>${escapeHtml(assembly.name || "-")}</strong><div class="muted">${escapeHtml(residualGraph.status || "-")}</div></td>
        <td>${escapeHtml(dependency.residual || "-")}</td>
        <td>${escapeHtml(dependency.variable || "-")}</td>
        <td>${escapeHtml(residualGraph.solver_plan || residualGraph.solverPlan || "-")}</td>
      </tr>
    `);
  }).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Assembly</th><th>Residual</th><th>Variable</th><th>Solver Plan</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="4" class="muted">No residual dependency graph.</td></tr>`}</tbody>
    </table>
  `;
}

function componentSolverVariableSummary(variables) {
  if (!Array.isArray(variables) || !variables.length) return "-";
  const shown = variables.slice(0, 4).map((variable) => {
    const unit = variable.unit ? ` ${variable.unit}` : "";
    return `${variable.name || "var"}=${metricCell(variable.value)}${unit}`;
  });
  if (variables.length > shown.length) shown.push(`+${variables.length - shown.length} more`);
  return shown.join(", ");
}

function componentSolverTrajectorySummary(trajectories) {
  if (!Array.isArray(trajectories) || !trajectories.length) return "-";
  const shown = trajectories.slice(0, 3).map((trajectory) => {
    const role = trajectory.role || "trajectory";
    const name = trajectory.name || "var";
    const unit = trajectory.unit ? ` ${trajectory.unit}` : "";
    const initial = trajectory.initial_value ?? trajectory.initialValue;
    const final = trajectory.final_value ?? trajectory.finalValue;
    const count = trajectory.point_count ?? trajectory.pointCount ?? (
      Array.isArray(trajectory.points) ? trajectory.points.length : "-"
    );
    return `${role}:${name} ${metricCell(initial)}->${metricCell(final)}${unit} (${count} pts)`;
  });
  if (trajectories.length > shown.length) shown.push(`+${trajectories.length - shown.length} more`);
  return shown.join(", ");
}

function componentSolverLargestResidual(residuals) {
  if (!Array.isArray(residuals) || !residuals.length) return "-";
  const largest = residuals.reduce((best, residual) => {
    if (!best) return residual;
    const residualScore = Math.abs(Number(residual.normalized_value ?? residual.normalizedValue ?? residual.value ?? 0));
    const bestScore = Math.abs(Number(best.normalized_value ?? best.normalizedValue ?? best.value ?? 0));
    return residualScore > bestScore
      ? residual
      : best;
  }, null);
  if (!largest) return "-";
  const unit = largest.unit ? ` ${largest.unit}` : "";
  const normalized = largest.normalized_value ?? largest.normalizedValue;
  const normalizedText = normalized == null ? "" : `, norm=${metricCell(normalized)}`;
  return `${largest.name || "residual"}=${metricCell(largest.value)}${unit}${normalizedText} (${largest.status || "-"})`;
}

function renderComponentGraph() {
  const graph = inspectorObject("componentGraph");
  const components = Array.isArray(graph.components) ? graph.components : [];
  const ports = Array.isArray(graph.ports) ? graph.ports : [];
  const connections = Array.isArray(graph.connections) ? graph.connections : [];
  const behaviorNodes = Array.isArray(graph.behavior_nodes)
    ? graph.behavior_nodes
    : (Array.isArray(graph.behaviorNodes) ? graph.behaviorNodes : []);
  const componentRows = components.map((component) => `
    <tr>
      <td><strong>${escapeHtml(component.name || "-")}</strong><div class="muted">${escapeHtml(component.kind || "-")}</div></td>
      <td>${escapeHtml(component.port_count ?? component.portCount ?? 0)}</td>
      <td>${escapeHtml(Array.isArray(component.ports) ? component.ports.join(", ") : "-")}</td>
      <td>${sourceLineButton(component)}</td>
    </tr>
  `).join("");
  const connectionRows = connections.map((connection) => `
    <tr>
      <td><strong>${escapeHtml(connection.left || "-")}</strong><div class="muted">${escapeHtml(connection.right || "-")}</div></td>
      <td>${escapeHtml(connection.domain_label || connection.domainLabel || "-")}</td>
      <td>${escapeHtml(connection.medium_label || connection.mediumLabel || connection.frame_label || connection.frameLabel || connection.axis_label || connection.axisLabel || "-")}</td>
      <td>${escapeHtml(connection.status || "-")}</td>
      <td>${sourceLineButton(connection)}</td>
    </tr>
  `).join("");
  const portRows = ports.map((port) => `
    <tr>
      <td><strong>${escapeHtml(port.component || "-")}.${escapeHtml(port.name || "-")}</strong></td>
      <td>${escapeHtml(port.domain_label || port.domainLabel || "-")}</td>
      <td>${escapeHtml(port.medium_label || port.mediumLabel || port.frame_label || port.frameLabel || port.axis_label || port.axisLabel || "-")}</td>
      <td>${escapeHtml(port.status || "-")}</td>
      <td>${sourceLineButton(port)}</td>
    </tr>
  `).join("");
  const behaviorRows = behaviorNodes.map((node) => `
    <tr>
      <td><strong>${escapeHtml(node.component || "-")}.${escapeHtml(node.name || "-")}</strong><div class="muted">${escapeHtml(node.behavior_kind || node.behaviorKind || "-")}</div></td>
      <td><code>${escapeHtml(node.expression || "-")}</code></td>
      <td>${escapeHtml(node.status || "-")}</td>
      <td>${escapeHtml(behaviorNodeDetails(node))}</td>
      <td>${sourceLineButton(node)}</td>
    </tr>
  `).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Component</th><th>Ports</th><th>Port IDs</th><th>Source</th></tr></thead>
      <tbody>${componentRows || `<tr><td colspan="4" class="muted">No component graph nodes.</td></tr>`}</tbody>
    </table>
    <table class="var-table">
      <thead><tr><th>Connection</th><th>Domain</th><th>Label</th><th>Status</th><th>Source</th></tr></thead>
      <tbody>${connectionRows || `<tr><td colspan="5" class="muted">No component graph connections.</td></tr>`}</tbody>
    </table>
    <table class="var-table">
      <thead><tr><th>Port</th><th>Domain</th><th>Label</th><th>Status</th><th>Source</th></tr></thead>
      <tbody>${portRows || `<tr><td colspan="5" class="muted">No component graph ports.</td></tr>`}</tbody>
    </table>
    <table class="var-table">
      <thead><tr><th>Behavior</th><th>Expression</th><th>Status</th><th>Details</th><th>Source</th></tr></thead>
      <tbody>${behaviorRows || `<tr><td colspan="5" class="muted">No component behavior nodes.</td></tr>`}</tbody>
    </table>
  `;
}

function renderWorkflowDag(nodes, edges, selectedId = "") {
  if (!nodes.length) {
    return `<div class="empty-state">Run a workflow to populate run_plan.json.</div>`;
  }
  const incoming = new Map();
  const outgoing = new Map();
  edges.forEach((edge) => {
    incoming.set(edge.to, (incoming.get(edge.to) || 0) + 1);
    outgoing.set(edge.from, (outgoing.get(edge.from) || 0) + 1);
  });
  const nodeHtml = nodes.map((node) => {
    const risk = String(node.risk || "low").toLowerCase();
    const selected = node.id === selectedId ? " selected" : "";
    return `
      <div class="workflow-node risk-${escapeAttr(risk)}${selected}" data-workflow-node-id="${escapeAttr(node.id || "")}">
        <div class="workflow-node-head">
          <strong>${escapeHtml(node.label || node.id || "-")}</strong>
          <span>${escapeHtml(node.status || "-")}</span>
        </div>
        <div class="muted">${escapeHtml(node.kind || "-")} / ${escapeHtml(node.phase || "-")} / ${escapeHtml(node.risk || "-")}</div>
        <div class="workflow-node-meta">
          <span>in ${incoming.get(node.id) || 0}</span>
          <span>out ${outgoing.get(node.id) || 0}</span>
          <span>${sourceLineButton(node)}</span>
        </div>
      </div>
    `;
  }).join("");
  return `<div class="workflow-graph">${nodeHtml}</div>`;
}

function selectedWorkflowNode(nodes) {
  if (!nodes.length) return null;
  return nodes.find((node) => node.id === state.selectedWorkflowNodeId) || nodes[0];
}

function renderWorkflowNodeDetail(node, edges) {
  if (!node) {
    return `<div class="empty-state">Select a workflow node to inspect its rerun decision, outputs, and edges.</div>`;
  }
  const decision = node.rerun_decision || node.rerunDecision || {};
  const incoming = edges.filter((edge) => edge.to === node.id);
  const outgoing = edges.filter((edge) => edge.from === node.id);
  return `
    <div class="workflow-node-detail">
      <div class="workflow-detail-head">
        <strong>${escapeHtml(node.label || node.id || "-")}</strong>
        <span>${escapeHtml(node.status || "-")}</span>
      </div>
      <div class="badges">
        <span class="badge">Kind ${escapeHtml(node.kind || "-")}</span>
        <span class="badge">Phase ${escapeHtml(node.phase || "-")}</span>
        <span class="badge">Risk ${escapeHtml(node.risk || "-")}</span>
        <span class="badge">Source ${sourceLineButton(node)}</span>
      </div>
      <table class="var-table compact-table">
        <tbody>
          <tr><th>ID</th><td><code>${escapeHtml(node.id || "-")}</code></td></tr>
          <tr><th>Rerun</th><td>${escapeHtml(decision.decision || "-")}<div class="muted">${escapeHtml(decision.reason || "-")}</div></td></tr>
          <tr><th>Prior Hash</th><td><code>${escapeHtml(decision.prior_input_hash || decision.priorInputHash || "-")}</code></td></tr>
          <tr><th>Risk Category</th><td>${escapeHtml(node.risk_category || node.riskCategory || "-")}<div class="muted">${escapeHtml(node.risk_severity || node.riskSeverity || "-")}</div></td></tr>
          <tr><th>Edges</th><td>in ${incoming.length} / out ${outgoing.length}</td></tr>
        </tbody>
      </table>
      <div class="panel-title compact">Outputs</div>
      ${renderWorkflowNodeOutputs(node.outputs)}
      <div class="panel-title compact">Edges</div>
      ${renderWorkflowNodeEdges(incoming, outgoing)}
      <details class="raw-json-toggle">
        <summary>Raw node JSON</summary>
        <pre>${escapeHtml(JSON.stringify(node, null, 2))}</pre>
      </details>
    </div>
  `;
}

function renderWorkflowNodeOutputs(outputs) {
  if (!Array.isArray(outputs) || !outputs.length) {
    return `<div class="empty-state">This node has no recorded outputs.</div>`;
  }
  const rows = outputs.map((output) => {
    const details = workflowOutputDetail(output);
    return `
      <tr>
        <td>${escapeHtml(output.kind || "-")}</td>
        <td>${openPathButton(output.path, 82)}</td>
        <td><code>${escapeHtml(output.hash || "-")}</code></td>
        <td>${escapeHtml(details)}</td>
      </tr>
    `;
  }).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Kind</th><th>Path</th><th>Hash</th><th>Details</th></tr></thead>
      <tbody>${rows}</tbody>
    </table>
  `;
}

function workflowOutputDetail(output) {
  if (!output || typeof output !== "object") return "-";
  const parts = [];
  for (const [key, value] of Object.entries(output)) {
    if (["kind", "path", "hash"].includes(key)) continue;
    if (value === null || value === undefined || value === "") continue;
    const text = Array.isArray(value)
      ? value.join(", ")
      : (typeof value === "object" ? compactObjectSummary(value) : String(value));
    parts.push(`${key}=${text}`);
  }
  return compactText(parts.join("; ") || "-", 120);
}

function renderWorkflowNodeEdges(incoming, outgoing) {
  const rows = [
    ...incoming.map((edge) => ({ ...edge, direction: "in" })),
    ...outgoing.map((edge) => ({ ...edge, direction: "out" }))
  ].map((edge) => `
    <tr>
      <td>${escapeHtml(edge.direction)}</td>
      <td><code>${escapeHtml(edge.from || "-")}</code></td>
      <td><code>${escapeHtml(edge.to || "-")}</code></td>
      <td>${escapeHtml(edge.kind || "-")}</td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Dir</th><th>From</th><th>To</th><th>Kind</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="4" class="muted">No node edges.</td></tr>`}</tbody>
    </table>
  `;
}

function renderWorkflowNodes(nodes) {
  const rows = nodes.map((node) => {
    const decision = node.rerun_decision || node.rerunDecision || {};
    return `
      <tr>
        <td><strong>${escapeHtml(node.label || "-")}</strong><div class="muted"><code>${escapeHtml(node.id || "-")}</code></div></td>
        <td>${escapeHtml(node.kind || "-")}</td>
        <td>${escapeHtml(node.phase || "-")}<div class="muted">${escapeHtml(node.risk || "-")}</div></td>
        <td>${escapeHtml(node.status || "-")}</td>
        <td>${escapeHtml(decision.decision || "-")}<div class="muted">${escapeHtml(decision.reason || "-")}</div></td>
        <td>${escapeHtml(workflowOutputSummary(node.outputs))}</td>
        <td>${sourceLineButton(node)}</td>
      </tr>
    `;
  }).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Node</th><th>Kind</th><th>Phase</th><th>Status</th><th>Rerun</th><th>Outputs</th><th>Source</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="7" class="muted">No workflow nodes.</td></tr>`}</tbody>
    </table>
  `;
}

function renderWorkflowEdges(edges) {
  const rows = edges.map((edge) => `
    <tr>
      <td><code>${escapeHtml(edge.from || "-")}</code></td>
      <td><code>${escapeHtml(edge.to || "-")}</code></td>
      <td>${escapeHtml(edge.kind || "-")}</td>
    </tr>
  `).join("");
  return `
    <table class="var-table">
      <thead><tr><th>From</th><th>To</th><th>Kind</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="3" class="muted">No workflow edges.</td></tr>`}</tbody>
    </table>
  `;
}

function workflowOutputSummary(outputs) {
  if (!Array.isArray(outputs) || !outputs.length) return "-";
  const labels = outputs.slice(0, 3).map((output) => {
    if (!output || typeof output !== "object") return String(output ?? "-");
    return output.kind || output.path || output.hash || output.status || "output";
  });
  const suffix = outputs.length > labels.length ? ` +${outputs.length - labels.length}` : "";
  return `${labels.join(", ")}${suffix}`;
}

function behaviorNodeDetails(node) {
  const parts = [];
  const signal = node.signal;
  const delayS = node.delay_s ?? node.delayS;
  const relationship = node.relationship_status ?? node.relationshipStatus;
  const contract = node.contract_status ?? node.contractStatus;
  const jacobian = node.jacobian_policy ?? node.jacobianPolicy;
  const profile = node.profile_policy ?? node.profilePolicy;
  const contractInputs = node.contract_inputs ?? node.contractInputs ?? [];
  const contractOutputs = node.contract_outputs ?? node.contractOutputs ?? [];
  const diagnostics = node.diagnostic_channels ?? node.diagnosticChannels ?? [];
  const runtimeWarnings = node.runtime_warning_status ?? node.runtimeWarningStatus;
  if (signal) parts.push(`signal=${signal}`);
  if (delayS !== null && delayS !== undefined) parts.push(`delay_s=${delayS}`);
  if (relationship) parts.push(`relationship=${relationship}`);
  if (contract) parts.push(`contract=${contract}`);
  if (jacobian) parts.push(`jacobian=${jacobian}`);
  if (profile) parts.push(`profile=${profile}`);
  if (Array.isArray(contractInputs) && contractInputs.length) {
    parts.push(`inputs=${behaviorContractDetails(contractInputs)}`);
  }
  if (Array.isArray(contractOutputs) && contractOutputs.length) {
    parts.push(`outputs=${behaviorContractDetails(contractOutputs)}`);
  }
  if (Array.isArray(diagnostics) && diagnostics.length) {
    parts.push(`diagnostics=${diagnostics.join("|")}`);
  }
  if (runtimeWarnings) parts.push(`runtime_warnings=${runtimeWarnings}`);
  return parts.length ? parts.join(", ") : "-";
}

function behaviorContractDetails(contracts) {
  return contracts.map((contract) => {
    const role = contract.role || "-";
    const name = contract.name || "-";
    const quantity = contract.quantity_kind || contract.quantityKind || "-";
    const unit = contract.display_unit || contract.displayUnit || "-";
    const status = contract.status || "-";
    return `${role}:${name}:${quantity}[${unit}]/${status}`;
  }).join("|");
}

function renderClassObjects() {
  const rows = inspectorRows("classObjects").map((object) => {
    const fields = Array.isArray(object.fields) ? object.fields : [];
    const validations = Array.isArray(object.validations) ? object.validations : [];
    return `
      <tr>
        <td><strong>${escapeHtml(object.name || "-")}</strong><div class="muted">${sourceLineButton(object)}</div></td>
        <td>${escapeHtml(object.class_name || object.className || "-")}<div class="muted">${escapeHtml(object.construction || "-")}</div></td>
        <td>${escapeHtml(object.source_object || object.sourceObject || "-")}</td>
        <td>${escapeHtml(fieldSummary(fields))}</td>
        <td>${escapeHtml(validationSummary(validations))}</td>
        <td>${escapeHtml(object.status || "-")}</td>
      </tr>
    `;
  }).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Object</th><th>Class</th><th>Source</th><th>Fields</th><th>Validation</th><th>Status</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">Run a class object workflow.</td></tr>`}</tbody>
    </table>
  `;
}

function renderArtifactOutlines() {
  const rows = inspectorRows("artifactOutlines").map((artifact) => `
    <tr>
      <td><button class="link-button" data-open-artifact-kind="${escapeAttr(artifact.kind)}"><strong>${escapeHtml(artifact.kind || "-")}</strong></button><div class="muted">${escapeHtml(artifact.status || "-")}</div></td>
      <td>${openPathButton(artifact.path, 90)}</td>
      <td>${escapeHtml((artifact.sections || []).map((section) => `${section.name}: ${section.summary}`).join("; "))}</td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Kind</th><th>Path</th><th>Sections</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="3" class="muted">Run a file to outline artifacts.</td></tr>`}</tbody>
    </table>
  `;
}

function renderEffectsPanel() {
  const effects = inspectorObject("effectRecords");
  const processResults = inspectorObject("processResults");
  const artifacts = Array.isArray(effects.artifactRecords) ? effects.artifactRecords : [];
  const boundaries = Array.isArray(effects.externalBoundaryRecords) ? effects.externalBoundaryRecords : [];
  const processes = Array.isArray(processResults.processes) ? processResults.processes : [];
  if (!artifacts.length && !boundaries.length && !processes.length) {
    return `
      <div class="panel-title compact">Effects</div>
      ${panelArtifactEmptyState(
        "No side-effect artifact data yet.",
        "Run a file with write/render/run/test/database operations.",
        "output_manifest.json, process_results.json, test_results.json"
      )}
    `;
  }
  return `
    <div class="panel-title compact">Effects</div>
    <div class="badges">
      <span class="badge">Artifacts ${artifacts.length}</span>
      <span class="badge">Boundaries ${boundaries.length}</span>
      <span class="badge">Processes ${processes.length}</span>
    </div>
    <div class="scroll">
      <div class="panel-title compact">Artifact Records</div>
      ${renderArtifactRecords(artifacts)}
      <div class="panel-title compact">External Boundary Records</div>
      ${renderExternalBoundaryRecords(boundaries)}
      <div class="panel-title compact">External Process Results</div>
      ${renderProcessResults(processes)}
      ${rawJsonToggle("Raw effects JSON", { effects, processResults })}
    </div>
  `;
}

function renderNetworkPanel() {
  const network = inspectorObject("networkCache");
  const boundaries = Array.isArray(network.networkBoundaries) ? network.networkBoundaries : [];
  const requests = Array.isArray(network.networkRequests) ? network.networkRequests : [];
  const events = Array.isArray(network.networkEvents) ? network.networkEvents : [];
  const caches = Array.isArray(network.manifestCaches) ? network.manifestCaches : [];
  const cacheEvents = Array.isArray(network.cacheEvents) ? network.cacheEvents : [];
  if (!boundaries.length && !requests.length && !events.length && !caches.length && !cacheEvents.length) {
    return `
      <div class="panel-title compact">Network / Cache</div>
      ${panelArtifactEmptyState(
        "No network/cache artifact data yet.",
        "Run a file with http/download/cache boundaries.",
        "result.engres typed_payload.network_cache and run_log.json cache events"
      )}
    `;
  }
  return `
    <div class="panel-title compact">Network / Cache</div>
    <div class="badges">
      <span class="badge">Boundaries ${boundaries.length}</span>
      <span class="badge">Requests ${requests.length || events.length}</span>
      <span class="badge">Cache ${caches.length || cacheEvents.length}</span>
    </div>
    ${sourceBreadcrumbs("Source spans", [...boundaries, ...requests, ...events, ...cacheEvents])}
    <div class="scroll">
      <div class="panel-title compact">Network Boundaries</div>
      ${renderNetworkBoundaries(boundaries)}
      <div class="panel-title compact">Network Events</div>
      ${renderNetworkEvents(events, requests)}
      <div class="panel-title compact">Cache Events</div>
      ${renderCacheEvents(cacheEvents, caches)}
      ${rawJsonToggle("Raw network/cache JSON", network)}
    </div>
  `;
}

function renderNetworkBoundaries(boundaries) {
  const rows = boundaries.map((boundary) => {
    const query = Array.isArray(boundary.query) ? boundary.query : [];
    const bodyLimit = boundary.body_size_limit_bytes ?? boundary.bodySizeLimitBytes;
    const policy = [
      boundary.retry !== undefined && boundary.retry !== null ? `retry ${boundary.retry}` : "",
      boundary.timeout ? `timeout ${boundary.timeout}` : "",
      bodyLimit !== undefined && bodyLimit !== null ? `limit ${bodyLimit} B` : "",
      query.length ? `query ${query.length}` : "",
    ].filter(Boolean).join("; ") || "-";
    return `
      <tr>
        <td><strong>${escapeHtml(boundary.kind || "-")}</strong><div class="muted">${escapeHtml(boundary.binding || boundary.target || "-")}</div></td>
        <td>${escapeHtml(boundary.status || "-")}<div class="muted">${escapeHtml(boundary.status_class || boundary.statusClass || "-")} ${escapeHtml(boundary.status_code ?? boundary.statusCode ?? "")}</div></td>
        <td><code>${escapeHtml(compactText(boundary.url || boundary.target || "-", 90))}</code></td>
        <td>${escapeHtml(policy)}</td>
        <td><code>${escapeHtml(compactText(boundary.response_hash || boundary.responseHash || "-", 68))}</code><div class="muted"><code>${escapeHtml(compactText(boundary.expected_sha256 || boundary.expectedSha256 || "-", 68))}</code></div></td>
        <td>${sourceLineButton(boundary)}</td>
      </tr>
    `;
  }).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Boundary</th><th>Status</th><th>URL / Target</th><th>Policy</th><th>Observed / Expected</th><th>Source</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No network boundaries.</td></tr>`}</tbody>
    </table>
  `;
}

function renderDbPanel() {
  const db = inspectorObject("dbWrites");
  const manifests = Array.isArray(db.manifests) ? db.manifests : [];
  const registry = Array.isArray(db.registryWrites) ? db.registryWrites : [];
  const tableCount = manifests.reduce((sum, manifest) => {
    const tables = Array.isArray(manifest.tables) ? manifest.tables : [];
    return sum + tables.length;
  }, 0);
  if (!manifests.length && !registry.length) {
    return `
      <div class="panel-title compact">DB Writes</div>
      ${panelArtifactEmptyState(
        "No DB write records yet.",
        "Run a file with open sqlite and write <table> to db.table(...).",
        "Saved result data contains DB write records and table write details."
      )}
    `;
  }
  return `
    <div class="panel-title compact">DB Writes</div>
    <div class="badges">
      <span class="badge">Write Records ${manifests.length}</span>
      <span class="badge">Tables ${tableCount}</span>
      <span class="badge">Registry ${registry.length}</span>
    </div>
    ${sourceBreadcrumbs("Source spans", [...manifests, ...registry])}
    <div class="scroll">
      <div class="panel-title compact">Write Records</div>
      ${renderDbManifests(manifests)}
      <div class="panel-title compact">Registry</div>
      ${renderDbRegistry(registry)}
      ${rawJsonToggle("Raw DB JSON", db)}
    </div>
  `;
}

function renderModelPanel() {
  const model = inspectorObject("modelCards");
  const cards = Array.isArray(model.cards) ? model.cards : [];
  const artifacts = Array.isArray(model.artifacts) ? model.artifacts : [];
  const specs = Array.isArray(model.specs) ? model.specs : [];
  const predictionManifests = Array.isArray(model.predictionManifests) ? model.predictionManifests : [];
  const diagnostics = Array.isArray(model.diagnostics) ? model.diagnostics : [];
  const residualPoints = artifacts.reduce((sum, artifact) => {
    const points = Array.isArray(artifact.residual_points) ? artifact.residual_points : (Array.isArray(artifact.residualPoints) ? artifact.residualPoints : []);
    return sum + points.length;
  }, 0);
  if (!cards.length && !artifacts.length && !specs.length && !predictionManifests.length && !diagnostics.length) {
    return `
      <div class="panel-title compact">Model Review</div>
      ${panelArtifactEmptyState(
        "No model review data yet.",
        "Run a file with regression_table, model_card, evaluate, or predict <model> using <table>.",
        "Saved result data contains model cards, training plans, prediction runs, and diagnostics."
      )}
    `;
  }
  return `
    <div class="panel-title compact">Model Review</div>
    <div class="badges">
      <span class="badge">Training Plans ${specs.length}</span>
      <span class="badge">Cards ${cards.length}</span>
      <span class="badge">Prediction Runs ${predictionManifests.length}</span>
      <span class="badge">Diagnostics ${diagnostics.length}</span>
      <span class="badge">Residuals ${residualPoints}</span>
    </div>
    ${sourceBreadcrumbs("Source spans", [...specs, ...cards, ...artifacts, ...predictionManifests, ...diagnostics])}
    <div class="scroll">
      <div class="panel-title compact">Training Plans</div>
      ${renderModelSpecs(specs)}
      <div class="panel-title compact">Model Cards</div>
      ${renderModelCards(cards)}
      <div class="panel-title compact">Training Results</div>
      ${renderModelArtifacts(artifacts)}
      <div class="panel-title compact">Prediction Runs</div>
      ${renderPredictionManifests(predictionManifests)}
      <div class="panel-title compact">Model Diagnostics</div>
      ${renderModelDiagnostics(diagnostics)}
      ${rawJsonToggle("Raw model JSON", model)}
    </div>
  `;
}

function renderCasePanel() {
  const caseData = inspectorObject("caseManifests");
  const manifests = Array.isArray(caseData.manifests) ? caseData.manifests : [];
  const caseTables = Array.isArray(caseData.caseTables) ? caseData.caseTables : [];
  const diagnostics = Array.isArray(caseData.diagnostics) ? caseData.diagnostics : [];
  const failed = Array.isArray(caseData.failedCases) ? caseData.failedCases : [];
  if (!manifests.length && !caseTables.length && !diagnostics.length && !failed.length) {
    return `
      <div class="panel-title compact">Cases</div>
      ${panelArtifactEmptyState(
        "No case run data yet.",
        "Run a file that materializes case tables or case input artifacts.",
        "Saved result data contains case tables and case run records."
      )}
    `;
  }
  return `
    <div class="panel-title compact">Cases</div>
    <div class="badges">
      <span class="badge">Tables ${caseTables.length}</span>
      <span class="badge">Cases ${manifests.length}</span>
      <span class="badge">Diagnostics ${diagnostics.length}</span>
      <span class="badge">Failed ${failed.length}</span>
    </div>
    ${sourceBreadcrumbs("Source spans", [...caseTables, ...manifests, ...diagnostics, ...failed])}
    <div class="scroll">
      ${renderCaseTables(caseTables)}
      <div class="panel-title compact">Case Runs</div>
      ${renderCaseManifests(manifests)}
      <div class="panel-title compact">Case Diagnostics</div>
      ${renderCaseDiagnostics(diagnostics)}
      <div class="panel-title compact">Failed Cases</div>
      ${renderFailedCases(failed)}
      ${rawJsonToggle("Raw case JSON", caseData)}
    </div>
  `;
}

function renderCaseTables(tables) {
  const rows = tables.map((table) => `
    <tr>
      <td><strong>${escapeHtml(table.sample_table || table.sampleTable || "-")}</strong><div class="muted">${escapeHtml(table.status || "-")}</div></td>
      <td>${escapeHtml(table.case_count ?? table.caseCount ?? "-")}</td>
      <td>${escapeHtml(table.pending_count ?? table.pendingCount ?? 0)} / ${escapeHtml(table.succeeded_count ?? table.succeededCount ?? 0)} / ${escapeHtml(table.failed_count ?? table.failedCount ?? 0)} / ${escapeHtml(table.skipped_count ?? table.skippedCount ?? 0)}</td>
      <td>${escapeHtml(table.runner || "-")}<div class="muted">${escapeHtml(table.scheduler || "-")}</div></td>
      <td>${escapeHtml(table.cache_hit_count ?? table.cacheHitCount ?? 0)} / ${escapeHtml(table.cache_miss_count ?? table.cacheMissCount ?? 0)}</td>
      <td>${escapeHtml((table.duplicate_case_ids || table.duplicateCaseIds || []).join(", ") || "-")}</td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Case Table</th><th>Count</th><th>Pending / Succeeded / Failed / Skipped</th><th>Runner</th><th>Cache H/M</th><th>Duplicates</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No case tables.</td></tr>`}</tbody>
    </table>
  `;
}

function renderCaseDiagnostics(diagnostics) {
  const rows = diagnostics.map((diagnostic) => `
    <tr>
      <td><strong>${escapeHtml(diagnostic.code || "-")}</strong><div class="muted">${escapeHtml(diagnostic.severity || "-")}</div></td>
      <td>${escapeHtml(diagnostic.case_id || diagnostic.caseId || "-")}</td>
      <td>${escapeHtml(diagnostic.sample_table || diagnostic.sampleTable || "-")}</td>
      <td>${escapeHtml(compactText(diagnostic.message || "-", 120))}</td>
      <td>${sourceLineButton(diagnostic)}</td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Code</th><th>Case</th><th>Table</th><th>Message</th><th>Source</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">No case diagnostics.</td></tr>`}</tbody>
    </table>
  `;
}

function renderCaseManifests(manifests) {
  const rows = manifests.map((manifest) => `
    <tr>
      <td><strong>${escapeHtml(manifest.case_id || manifest.caseId || "-")}</strong><div class="muted">${escapeHtml(manifest.status || "-")}</div></td>
      <td>${escapeHtml(manifest.sample_table || manifest.sampleTable || "-")}<div class="muted">row ${escapeHtml(manifest.sample_row_number ?? manifest.sampleRowNumber ?? "-")}</div></td>
      <td>${openPathStack([manifest.case_dir || manifest.caseDir, manifest.generated_input_file || manifest.generatedInputFile], 80)}</td>
      <td>${escapeHtml(caseProcessSummary(manifest))}</td>
      <td>${escapeHtml(caseOutputSummary(manifest))}</td>
      <td>${escapeHtml(compactText(manifest.failure_reason || manifest.failureReason || "-", 90))}</td>
      <td>${sourceLineButton(manifest)}</td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Case</th><th>Sample</th><th>Files</th><th>Process</th><th>Outputs</th><th>Failure</th><th>Source</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="7" class="muted">No case runs.</td></tr>`}</tbody>
    </table>
  `;
}

function renderFailedCases(cases) {
  const rows = cases.map((manifest) => `
    <tr>
      <td><strong>${escapeHtml(manifest.case_id || manifest.caseId || "-")}</strong></td>
      <td>${escapeHtml(manifest.status || "-")}</td>
      <td>${escapeHtml(caseProcessSummary(manifest))}</td>
      <td>${escapeHtml(compactText(manifest.failure_reason || manifest.failureReason || "-", 110))}</td>
      <td>${sourceLineButton(manifest)}</td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Case</th><th>Status</th><th>Process</th><th>Reason</th><th>Source</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">No failed cases.</td></tr>`}</tbody>
    </table>
  `;
}

function caseProcessSummary(manifest) {
  const bindings = manifest.process_bindings || manifest.processBindings || [];
  const statuses = manifest.process_statuses || manifest.processStatuses || [];
  const bindingText = Array.isArray(bindings) && bindings.length ? bindings.join(", ") : "-";
  const statusText = Array.isArray(statuses) && statuses.length
    ? statuses.map((status) => `${status.name || status.binding || "-"}:${status.status || "-"}`).join(", ")
    : "-";
  return compactText(`${bindingText}; ${statusText}`, 100);
}

function caseOutputSummary(manifest) {
  const artifacts = manifest.output_artifacts || manifest.outputArtifacts || [];
  const results = manifest.result_files || manifest.resultFiles || [];
  const metrics = manifest.metrics || [];
  const parts = [];
  if (Array.isArray(artifacts) && artifacts.length) parts.push(`artifacts ${artifacts.length}`);
  if (Array.isArray(results) && results.length) parts.push(`results ${results.length}`);
  if (Array.isArray(metrics) && metrics.length) parts.push(`metrics ${metrics.length}`);
  return parts.length ? parts.join(", ") : "-";
}

function renderModelSpecs(specs) {
  const rows = specs.map((spec) => `
    <tr>
      <td><strong>${escapeHtml(spec.binding || "-")}</strong><div class="muted">${sourceLineButton(spec)}</div></td>
      <td>${escapeHtml(spec.model_kind || spec.modelKind || "-")}<div class="muted">${escapeHtml(spec.status || "-")}</div></td>
      <td>${escapeHtml(modelFeatureSpecSummary(spec.features))}</td>
      <td>${escapeHtml(modelTargetSpecSummary(spec.target))}</td>
      <td>${escapeHtml(spec.train_count ?? spec.trainCount ?? "-")} / ${escapeHtml(spec.test_count ?? spec.testCount ?? "-")}<div class="muted">split ${escapeHtml(spec.test_fraction ?? spec.testFraction ?? "-")} seed ${escapeHtml(spec.seed ?? "-")}</div></td>
      <td><code>${escapeHtml(compactText(spec.model_artifact_hash || spec.modelArtifactHash || "-", 70))}</code></td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Binding</th><th>Model</th><th>Features</th><th>Target</th><th>Train/Test</th><th>Hash</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No model training plans.</td></tr>`}</tbody>
    </table>
  `;
}

function modelFeatureSpecSummary(features) {
  if (!Array.isArray(features) || !features.length) return "-";
  return compactText(features.map((feature) => {
    if (typeof feature === "string") return feature;
    const name = feature.name || "-";
    const quantity = feature.quantity || "";
    const unit = feature.unit || "";
    const suffix = [quantity, unit].filter(Boolean).join(" ");
    return suffix ? `${name} (${suffix})` : name;
  }).join(", "), 120);
}

function modelTargetSpecSummary(target) {
  if (!target || typeof target !== "object") return "-";
  const name = target.name || "-";
  const suffix = [target.quantity, target.unit].filter(Boolean).join(" ");
  return suffix ? `${name} (${suffix})` : name;
}

function renderModelCards(cards) {
  const rows = cards.map((card) => {
    const metrics = card.metrics || {};
    return `
      <tr>
        <td><strong>${escapeHtml(card.binding || "-")}</strong><div class="muted">${sourceLineButton(card)}</div></td>
        <td>${escapeHtml(card.model_kind || card.modelKind || "-")}<div class="muted">${escapeHtml(card.status || "-")}</div></td>
        <td>${escapeHtml(card.target || "-")}<div class="muted">${escapeHtml(card.target_quantity || card.targetQuantity || "-")} ${escapeHtml(card.target_unit || card.targetUnit || "")}</div></td>
        <td>${metricCell(metrics.rmse)} / ${metricCell(metrics.mae)} / ${metricCell(metrics.r2)}</td>
        <td>${openPathButton(card.residual_plot || card.residualPlot, 82)}<div class="muted">points ${escapeHtml(card.residual_point_count ?? card.residualPointCount ?? 0)}</div></td>
        <td><code>${escapeHtml(compactText(card.model_artifact_hash || card.modelArtifactHash || "-", 70))}</code></td>
      </tr>
    `;
  }).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Binding</th><th>Model</th><th>Target</th><th>RMSE/MAE/R2</th><th>Residual Plot</th><th>Hash</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No model cards.</td></tr>`}</tbody>
    </table>
  `;
}

function renderModelArtifacts(artifacts) {
  const rows = artifacts.map((artifact) => {
    const residuals = artifact.residual_points || artifact.residualPoints || [];
    const parity = artifact.parity_points || artifact.parityPoints || [];
    return `
      <tr>
        <td><strong>${escapeHtml(artifact.binding || "-")}</strong><div class="muted">${escapeHtml(artifact.kind || "-")}</div></td>
        <td>${escapeHtml(artifact.algorithm || "-")}<div class="muted">${escapeHtml(artifact.status || "-")}</div></td>
        <td>${escapeHtml(Array.isArray(artifact.features) ? artifact.features.join(", ") : "-")}</td>
        <td>${metricCell(artifact.rmse)} / ${metricCell(artifact.mae)} / ${metricCell(artifact.r2)}</td>
        <td>${escapeHtml(Array.isArray(residuals) ? residuals.length : 0)}<div class="muted">parity ${escapeHtml(Array.isArray(parity) ? parity.length : 0)}</div></td>
        <td><code>${escapeHtml(compactText(artifact.training_data_hash || artifact.trainingDataHash || "-", 70))}</code></td>
        <td>${sourceLineButton(artifact)}</td>
      </tr>
    `;
  }).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Training Run</th><th>Algorithm</th><th>Features</th><th>RMSE/MAE/R2</th><th>Points</th><th>Training Hash</th><th>Source</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="7" class="muted">No training results.</td></tr>`}</tbody>
    </table>
  `;
}

function renderPredictionManifests(manifests) {
  const rows = manifests.map((manifest) => `
    <tr>
      <td><strong>${escapeHtml(manifest.binding || "-")}</strong><div class="muted">${sourceLineButton(manifest)}</div></td>
      <td>${escapeHtml(manifest.model || "-")}<div class="muted">${escapeHtml(manifest.status || "-")}</div></td>
      <td>${openPathStack([manifest.manifest_path || manifest.manifestPath, manifestFilePath(manifest.output_file || manifest.outputFile)], 80)}</td>
      <td>${escapeHtml(manifest.row_count ?? manifest.rowCount ?? "-")}<div class="muted">cases ${escapeHtml(Array.isArray(manifest.case_ids || manifest.caseIds) ? (manifest.case_ids || manifest.caseIds).length : 0)}</div></td>
      <td>${escapeHtml(predictionOutputSummary(manifest.outputs))}</td>
      <td>${escapeHtml(manifest.confidence_column || manifest.confidenceColumn || "-")}</td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Binding</th><th>Model</th><th>Files</th><th>Rows</th><th>Outputs</th><th>Confidence</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No prediction runs.</td></tr>`}</tbody>
    </table>
  `;
}

function manifestFilePath(file) {
  if (!file || typeof file !== "object") return "-";
  return file.path || "-";
}

function predictionOutputSummary(outputs) {
  if (!Array.isArray(outputs) || !outputs.length) return "-";
  return compactText(outputs.map((output) => {
    const column = output.column || "-";
    const suffix = [output.quantity, output.unit].filter(Boolean).join(" ");
    return suffix ? `${column} (${suffix})` : column;
  }).join(", "), 120);
}

function renderModelDiagnostics(diagnostics) {
  const rows = diagnostics.map((diagnostic) => `
    <tr>
      <td><strong>${escapeHtml(diagnostic.code || "-")}</strong><div class="muted">${escapeHtml(diagnostic.severity || "-")}</div></td>
      <td>${escapeHtml(diagnostic.binding || "-")}</td>
      <td>${escapeHtml(compactText(diagnostic.message || "-", 140))}</td>
      <td>${sourceLineButton(diagnostic)}</td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Code</th><th>Binding</th><th>Message</th><th>Source</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="4" class="muted">No model diagnostics.</td></tr>`}</tbody>
    </table>
  `;
}

function renderDbManifests(manifests) {
  const rows = manifests.flatMap((manifest) => {
    const tables = Array.isArray(manifest.tables) ? manifest.tables : [];
    if (!tables.length) {
      return [`
        <tr>
          <td><strong>${escapeHtml(manifest.binding || "-")}</strong><div class="muted">${escapeHtml(manifest.status || "-")}</div></td>
          <td>${openPathButton(manifest.database || manifest.manifest_path || manifest.manifestPath, 80)}</td>
          <td>${escapeHtml(manifest.transaction_status || manifest.transactionStatus || "-")}<div class="muted">${escapeHtml(manifest.schema_status || manifest.schemaStatus || "-")}</div></td>
          <td>-</td>
          <td>-</td>
          <td>-</td>
          <td>${sourceLineButton(manifest)}</td>
        </tr>
      `];
    }
    return tables.map((table) => `
      <tr>
        <td><strong>${escapeHtml(manifest.binding || "-")}</strong><div class="muted">${escapeHtml(manifest.status || "-")}</div></td>
        <td>${openPathButton(manifest.database || manifest.manifest_path || manifest.manifestPath, 80)}</td>
        <td>${escapeHtml(manifest.transaction_status || manifest.transactionStatus || "-")}<div class="muted">${escapeHtml(manifest.schema_status || manifest.schemaStatus || "-")}</div></td>
        <td><strong>${escapeHtml(table.name || "-")}</strong><div class="muted">${escapeHtml(table.mode || "-")}</div></td>
        <td>${escapeHtml(table.row_count ?? table.rowCount ?? "-")}</td>
        <td>${escapeHtml(dbTableShape(table))}</td>
        <td>${sourceLineButton(manifest)}</td>
      </tr>
    `);
  }).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Binding</th><th>Database</th><th>Transaction</th><th>Table</th><th>Rows</th><th>Shape</th><th>Source</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="7" class="muted">No DB write records.</td></tr>`}</tbody>
    </table>
  `;
}

function dbTableShape(table) {
  const key = Array.isArray(table.key) && table.key.length ? `key ${table.key.join(", ")}` : "key -";
  const schema = Array.isArray(table.schema) && table.schema.length ? `schema ${table.schema.join(", ")}` : "schema -";
  return compactText(`${key}; ${schema}`, 100);
}

function renderDbRegistry(records) {
  const rows = records.map((record) => `
    <tr>
      <td><strong>${escapeHtml(record.binding || "-")}</strong><div class="muted">${escapeHtml(record.status || "-")}</div></td>
      <td>${openPathButton(record.database || record.manifest_path || record.manifestPath, 90)}</td>
      <td>${escapeHtml(record.transaction_status || record.transactionStatus || "-")}</td>
      <td>${escapeHtml(record.table_count ?? record.tableCount ?? "-")}</td>
      <td><code>${escapeHtml(compactText(record.hash || "-", 70))}</code></td>
      <td>${sourceLineButton(record)}</td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Binding</th><th>Target</th><th>Transaction</th><th>Tables</th><th>Hash</th><th>Source</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No DB registry records.</td></tr>`}</tbody>
    </table>
  `;
}

function renderNetworkEvents(events, requests) {
  const source = events.length ? events : requests;
  const rows = source.map((event) => `
    <tr>
      <td><strong>${escapeHtml(event.kind || event.method || "-")}</strong><div class="muted">${sourceLineButton(event)}</div></td>
      <td>${escapeHtml(event.status || "-")}<div class="muted">${escapeHtml(event.status_class || event.statusClass || "-")} ${escapeHtml(event.status_code ?? event.statusCode ?? "")}</div></td>
      <td><code>${escapeHtml(compactText(event.target || event.url || "-", 90))}</code></td>
      <td><code>${escapeHtml(compactText(event.response_hash || event.responseHash || event.hash || "-", 70))}</code></td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Kind</th><th>Status</th><th>Target</th><th>Hash</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="4" class="muted">No network events.</td></tr>`}</tbody>
    </table>
  `;
}

function renderCacheEvents(events, caches) {
  const source = events.length ? events : caches;
  const rows = source.map((event) => `
    <tr>
      <td><strong>${escapeHtml(event.owner_kind || event.ownerKind || "-")}</strong><div class="muted">${escapeHtml(event.owner_name || event.ownerName || "-")}</div></td>
      <td>${escapeHtml(event.status || "-")}</td>
      <td><code>${escapeHtml(compactText(event.cache_key || event.cacheKey || event.cache_key_hash || event.cacheKeyHash || "-", 80))}</code></td>
      <td>${openPathButton(event.cache_path || event.cachePath || event.cache_dir || event.cacheDir, 90)}</td>
      <td>${sourceLineButton(event)}</td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Owner</th><th>Status</th><th>Key</th><th>Path</th><th>Source</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">No cache events.</td></tr>`}</tbody>
    </table>
  `;
}

function renderArtifactRecords(artifacts) {
  const rows = artifacts.map((artifact) => `
    <tr>
      <td><strong>${escapeHtml(artifact.kind || "-")}</strong></td>
      <td>${escapeHtml(artifact.class || "-")}</td>
      <td>${openPathButton(artifact.path, 90)}</td>
      <td>${escapeHtml(artifact.status || "-")}</td>
      <td><code>${escapeHtml(artifact.hash || "-")}</code></td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Kind</th><th>Class</th><th>Path</th><th>Status</th><th>Hash</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">No artifact records.</td></tr>`}</tbody>
    </table>
  `;
}

function renderExternalBoundaryRecords(boundaries) {
  const rows = boundaries.map((boundary) => `
    <tr>
      <td><strong>${escapeHtml(boundary.kind || "-")}</strong><div class="muted">${sourceLineButton(boundary)}</div></td>
      <td>${escapeHtml(boundary.binding || "-")}</td>
      <td><code>${escapeHtml(compactText(boundary.target || boundary.command || "-", 80))}</code></td>
      <td>${escapeHtml(boundary.status || "-")}<div class="muted">${boundary.success === true ? "success" : boundary.success === false ? "failed" : "-"}</div></td>
      <td><code>${escapeHtml(compactText(boundary.response_hash || boundary.stdout_hash || boundary.expected_sha256 || "-", 70))}</code></td>
    </tr>
  `).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Kind</th><th>Binding</th><th>Target</th><th>Status</th><th>Hash</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">No external boundary records.</td></tr>`}</tbody>
    </table>
  `;
}

function renderRunLog(messages) {
  const rows = messages.map((message) => `
    <tr>
      <td><strong>${escapeHtml(message.level || "-")}</strong><div class="muted">${sourceLineButton(message)}</div></td>
      <td>${escapeHtml(message.message || "-")}</td>
    </tr>
  `).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Level</th><th>Message</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="2" class="muted">No run log messages.</td></tr>`}</tbody>
    </table>
  `;
}

function renderProcessResults(processes) {
  const rows = processes.map((process) => {
    const outputs = Array.isArray(process.expected_outputs)
      ? process.expected_outputs
      : (Array.isArray(process.expectedOutputs) ? process.expectedOutputs : []);
    return `
      <tr>
        <td><strong>${escapeHtml(process.binding || "-")}</strong><div class="muted">${sourceLineButton(process)}</div></td>
        <td><code>${escapeHtml([process.command, ...(process.args || [])].filter(Boolean).join(" "))}</code><div class="muted">${escapeHtml(process.cwd || "-")}</div></td>
        <td>${escapeHtml(process.status || "-")}<div class="muted">exit ${escapeHtml(process.exit_code ?? process.exitCode ?? "-")}</div><div class="muted">${escapeHtml(process.expected_output_status || process.expectedOutputStatus || "-")}</div></td>
        <td><code>${escapeHtml(compactText(process.stdout_hash || process.stdoutHash || "-", 70))}</code><div class="muted"><code>${escapeHtml(compactText(process.stderr_hash || process.stderrHash || "-", 70))}</code></div></td>
        <td>${renderOutputPathList(outputs, 70)}</td>
      </tr>
    `;
  }).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Binding</th><th>Command</th><th>Status</th><th>Stdout/Stderr Hash</th><th>Expected Outputs</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">No process results.</td></tr>`}</tbody>
    </table>
  `;
}

function renderTestResults(tests) {
  const rows = tests.map((test) => {
    const assertions = Array.isArray(test.assertions) ? test.assertions : [];
    const goldens = Array.isArray(test.goldens) ? test.goldens : [];
    return `
      <tr>
        <td><strong>${escapeHtml(test.name || "-")}</strong><div class="muted">${sourceLineButton(test)}</div></td>
        <td>${escapeHtml(test.status || "-")}</td>
        <td>${escapeHtml(assertions.length)}</td>
        <td>${escapeHtml(goldens.length)}</td>
      </tr>
    `;
  }).join("");
  return `
    <table class="var-table">
      <thead><tr><th>Name</th><th>Status</th><th>Assertions</th><th>Goldens</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="4" class="muted">No test results.</td></tr>`}</tbody>
    </table>
  `;
}

function renderTabs() {
  return state.tabs.map((tab) => {
    const active = tab.path === state.currentPath ? " active" : "";
    const dirty = tab.dirty ? " *" : "";
    const close = state.tabs.length > 1
      ? `<button class="tab-close" data-close-path="${escapeAttr(tab.path)}" title="Close">x</button>`
      : "";
    return `
      <div class="tab${active}" data-tab-path="${escapeAttr(tab.path)}" title="${escapeAttr(tab.path)}">
        <span>${escapeHtml(fileName(tab.path))}${dirty}</span>
        ${close}
      </div>
    `;
  }).join("");
}

function renderTabLabels() {
  const tabs = document.querySelector(".editor-tabs");
  if (!tabs) return;
  tabs.innerHTML = renderTabs();
  document.querySelectorAll("[data-tab-path]").forEach((tab) => {
    tab.onclick = () => switchTab(tab.dataset.tabPath);
  });
  document.querySelectorAll("[data-close-path]").forEach((button) => {
    button.onclick = (event) => {
      event.stopPropagation();
      closeTab(button.dataset.closePath);
    };
  });
}

function handleEditorKeyDown(event) {
  const overlayVisible = state.completionItems.length > 0;
  if ((event.ctrlKey || event.metaKey) && event.key === " ") {
    event.preventDefault();
    updateCompletionOverlay(true);
    return;
  }
  if (!overlayVisible) return;
  if (event.key === "Tab" || event.key === "Enter") {
    event.preventDefault();
    insertCompletion(state.completionItems[state.completionIndex]);
  } else if (event.key === "Escape") {
    event.preventDefault();
    hideCompletions();
  } else if (event.key === "ArrowDown") {
    event.preventDefault();
    state.completionIndex = (state.completionIndex + 1) % state.completionItems.length;
    drawCompletionOverlay();
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    state.completionIndex = (state.completionIndex + state.completionItems.length - 1) % state.completionItems.length;
    drawCompletionOverlay();
  }
}

function updateCompletionOverlay(force = false) {
  const editor = byId("editor");
  if (!editor) return;
  const prefix = currentPrefix(editor);
  if (!force && prefix.length < 2) {
    hideCompletions();
    return;
  }
  state.completionItems = completionCandidates(prefix);
  state.completionIndex = 0;
  if (!state.completionItems.length) {
    hideCompletions();
    return;
  }
  drawCompletionOverlay();
}

function drawCompletionOverlay() {
  const overlay = byId("completionOverlay");
  const editor = byId("editor");
  if (!overlay || !editor || !state.completionItems.length) return;
  const position = caretOverlayPosition(editor);
  overlay.style.left = `${position.left}px`;
  overlay.style.top = `${position.top}px`;
  overlay.innerHTML = state.completionItems.map((item, index) => `
    <button class="completion-item ${index === state.completionIndex ? "active" : ""}" data-completion-index="${index}">
      <span>${escapeHtml(item.label)}</span>
      <small>${escapeHtml(item.detail || item.kind || "")}</small>
    </button>
  `).join("");
  overlay.classList.remove("hidden");
  document.querySelectorAll("[data-completion-index]").forEach((button) => {
    button.onclick = () => insertCompletion(state.completionItems[Number(button.dataset.completionIndex)]);
  });
}

function hideCompletions() {
  state.completionItems = [];
  const overlay = byId("completionOverlay");
  if (overlay) {
    overlay.classList.add("hidden");
    overlay.innerHTML = "";
  }
}

function completionCandidates(prefix) {
  const lower = prefix.toLowerCase();
  const symbolItems = state.check.symbols.map((symbol) => ({
    label: symbol.name,
    insert: symbol.name,
    detail: `${symbol.quantityKind || "symbol"} ${symbol.displayUnit || ""}`.trim(),
    kind: "symbol"
  }));
  const seen = new Set();
  return [...symbolItems, ...state.completions]
    .filter((item) => item.label && item.label.toLowerCase().startsWith(lower))
    .filter((item) => {
      const key = `${item.kind}:${item.label}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    })
    .slice(0, 9);
}

function currentPrefix(editor) {
  const before = editor.value.slice(0, editor.selectionStart);
  const match = before.match(/[A-Za-z_][A-Za-z0-9_./-]*$/);
  return match ? match[0] : "";
}

function insertCompletion(item) {
  const editor = byId("editor");
  if (!editor || !item) return;
  const prefix = currentPrefix(editor);
  const start = editor.selectionStart - prefix.length;
  const end = editor.selectionEnd;
  const before = editor.value.slice(0, start);
  const after = editor.value.slice(end);
  editor.value = `${before}${item.insert}${after}`;
  const cursor = before.length + item.insert.length;
  editor.selectionStart = cursor;
  editor.selectionEnd = cursor;
  state.source = editor.value;
  state.dirty = true;
  rememberCurrentTab();
  renderTabLabels();
  updateEditorHighlight();
  updateCursorInsight();
  hideCompletions();
  editor.focus();
}

function updateCursorInsight() {
  const target = byId("cursorInsight");
  if (!target) return;
  target.outerHTML = `<span id="cursorInsight" class="cursor-insight">${renderCursorInsight()}</span>`;
  bindCursorInsightActions();
}

function bindCursorInsightActions() {
  const target = byId("cursorInsight");
  if (!target) return;
  target.querySelectorAll("[data-source-token-line]").forEach((button) => {
    button.onclick = () => selectSourceTokenRange(
      Number(button.dataset.sourceTokenLine || 0),
      Number(button.dataset.sourceTokenStart || 0),
      Number(button.dataset.sourceTokenLength || 0)
    );
  });
  target.querySelectorAll("[data-show-highlight-panel]").forEach((button) => {
    button.onclick = () => {
      state.sideTab = "highlight";
      render();
    };
  });
}

function renderCursorInsight() {
  const editor = byId("editor");
  const source = editor?.value ?? state.source ?? "";
  const position = editorCursorPosition(source, editor?.selectionStart ?? 0);
  const token = editor ? semanticTokenAtCaret(editor, position) : null;
  const hover = token ? hoverForSemanticToken(token, position.line) : null;
  const parts = [`L${position.line + 1}:C${position.column + 1}`];
  if (state.source !== state.highlightSource) {
    parts.push("Check needed");
  } else if (token) {
    parts.push(tokenLabel(token));
    if (hover?.quantity_kind || hover?.quantityKind) {
      const quantity = hover.quantity_kind || hover.quantityKind;
      const unit = hover.display_unit || hover.displayUnit || "-";
      parts.push(`${quantity} [${unit}]`);
    }
  } else {
    parts.push("plain");
  }
  const title = hover ? hoverTitle(hover) : parts.join(" / ");
  return `
    <span title="${escapeAttr(title)}">${escapeHtml(parts.join(" / "))}</span>
    ${token ? renderCursorInsightActions(token) : ""}
  `;
}

function renderCursorInsightActions(token) {
  return `
    ${sourceTokenButton(token, "Select")}
    <button class="link-button token-range-button" data-show-highlight-panel title="Open Highlight panel">Highlight</button>
  `;
}

function editorCursorPosition(source, offset) {
  const safeOffset = Math.max(0, Math.min(Number(offset) || 0, source.length));
  const before = source.slice(0, safeOffset);
  const lines = before.split(/\r\n|\r|\n/);
  return {
    line: lines.length - 1,
    column: lines[lines.length - 1].length,
    offset: safeOffset
  };
}

function semanticTokenAtCaret(editor, position) {
  if (state.source !== state.highlightSource) return null;
  const line = editor.value.split(/\r\n|\r|\n/)[position.line] || "";
  const columnByte = codeUnitToByteOffset(line, position.column);
  const tokens = semanticTokensByLine(semanticTokenPayload().tokens || []).get(position.line) || [];
  return tokens.find((token) => {
    const start = Number(token.start ?? 0);
    const end = start + Number(token.length ?? 0);
    return columnByte >= start && columnByte <= end;
  }) || null;
}

function hoverForSemanticToken(token, lineIndex) {
  const line = lineIndex + 1;
  const hovers = Array.isArray(state.check?.hovers) ? state.check.hovers : [];
  const tokenText = tokenTextForSemanticToken(token, lineIndex);
  return hovers.find((hover) => {
    if (Number(hover.line || 0) !== line) return false;
    const name = String(hover.name || "");
    return name === tokenText || name.endsWith(`.${tokenText}`) || tokenText.endsWith(`.${name}`);
  }) || hovers.find((hover) => Number(hover.line || 0) === line) || null;
}

function tokenTextForSemanticToken(token, lineIndex) {
  const line = String(state.highlightSource || "").split(/\r\n|\r|\n/)[lineIndex] || "";
  const range = semanticTokenRange(line, token);
  return range ? line.slice(range.start, range.end) : "";
}

function tokenLabel(token) {
  const modifiers = arrayOrEmpty(token.modifiers);
  return [token.type || "token", ...modifiers].join("/");
}

function hoverTitle(hover) {
  return [
    hover.name,
    hover.kind,
    hover.detail,
    hover.quantity_kind || hover.quantityKind,
    hover.display_unit || hover.displayUnit,
    hover.status
  ].filter(Boolean).join(" / ");
}

function caretOverlayPosition(editor) {
  const before = editor.value.slice(0, editor.selectionStart);
  const lines = before.split(/\r\n|\r|\n/);
  const line = lines.length - 1;
  const column = lines[lines.length - 1].length;
  const lineHeight = 20;
  const charWidth = 8.2;
  const left = Math.max(8, Math.min(editor.clientWidth - 280, 12 + column * charWidth - editor.scrollLeft));
  const top = Math.max(8, Math.min(editor.clientHeight - 210, 12 + (line + 1) * lineHeight - editor.scrollTop));
  return { left, top };
}

function updateEditorHighlight() {
  const highlight = byId("editorHighlight");
  if (!highlight) return;
  highlight.innerHTML = renderHighlightedSource();
  syncEditorHighlightScroll();
}

function syncEditorHighlightScroll() {
  const editor = byId("editor");
  const highlight = byId("editorHighlight");
  if (!editor || !highlight) return;
  highlight.scrollTop = editor.scrollTop;
  highlight.scrollLeft = editor.scrollLeft;
}

function renderHighlightedSource() {
  if (state.source !== state.highlightSource) {
    return escapeHtml(state.source || "\n");
  }
  const tokensByLine = semanticTokensByLine(semanticTokenPayload().tokens || []);
  const lines = String(state.source ?? "").split(/\r\n|\r|\n/);
  return lines.map((line, index) => renderHighlightedLine(line, tokensByLine.get(index) || [])).join("\n") || "\n";
}

function renderHighlightedLine(line, tokens) {
  if (!tokens.length) return escapeHtml(line);
  const ranges = tokens
    .map((token) => semanticTokenRange(line, token))
    .filter(Boolean)
    .sort((left, right) => left.start - right.start || right.end - left.end);
  let cursor = 0;
  let html = "";
  for (const range of ranges) {
    if (range.start < cursor || range.end <= range.start) continue;
    html += escapeHtml(line.slice(cursor, range.start));
    html += `<span class="${escapeAttr(semanticTokenClass(range.token))}">${escapeHtml(line.slice(range.start, range.end))}</span>`;
    cursor = range.end;
  }
  html += escapeHtml(line.slice(cursor));
  return html;
}

function semanticTokensByLine(tokens) {
  const map = new Map();
  for (const token of Array.isArray(tokens) ? tokens : []) {
    const line = Number(token.line);
    if (!Number.isFinite(line) || line < 0) continue;
    if (!map.has(line)) map.set(line, []);
    map.get(line).push(token);
  }
  return map;
}

function semanticTokenRange(line, token) {
  const startByte = Number(token.start ?? 0);
  const lengthBytes = Number(token.length ?? 0);
  if (!Number.isFinite(startByte) || !Number.isFinite(lengthBytes) || lengthBytes <= 0) {
    return null;
  }
  const start = byteOffsetToCodeUnit(line, startByte);
  const end = byteOffsetToCodeUnit(line, startByte + lengthBytes);
  return { start, end, token };
}

function semanticTokenText(token) {
  const lineIndex = Number(token?.line ?? -1);
  if (!Number.isFinite(lineIndex) || lineIndex < 0) return "-";
  const line = String(state.highlightSource || "").split(/\r\n|\r|\n/)[lineIndex] || "";
  const range = semanticTokenRange(line, token);
  if (!range) return "-";
  return line.slice(range.start, range.end) || "-";
}

function byteOffsetToCodeUnit(text, byteOffset) {
  let bytes = 0;
  for (let index = 0; index < text.length; index += 1) {
    const codePoint = text.codePointAt(index);
    const char = String.fromCodePoint(codePoint);
    const charBytes = utf8ByteLength(char);
    if (bytes + charBytes > byteOffset) return index;
    bytes += charBytes;
    if (codePoint > 0xffff) index += 1;
  }
  return text.length;
}

function codeUnitToByteOffset(text, codeUnitOffset) {
  let bytes = 0;
  const limit = Math.max(0, Math.min(Number(codeUnitOffset) || 0, text.length));
  for (let index = 0; index < limit; index += 1) {
    const codePoint = text.codePointAt(index);
    bytes += utf8ByteLength(String.fromCodePoint(codePoint));
    if (codePoint > 0xffff) index += 1;
  }
  return bytes;
}

function utf8ByteLength(value) {
  const codePoint = value.codePointAt(0) || 0;
  if (codePoint <= 0x7f) return 1;
  if (codePoint <= 0x7ff) return 2;
  if (codePoint <= 0xffff) return 3;
  return 4;
}

function semanticTokenClass(token) {
  const classes = [`hl-token`, `hl-${safeCssToken(token.type || "plain")}`];
  for (const modifier of arrayOrEmpty(token.modifiers)) {
    classes.push(`hl-mod-${safeCssToken(modifier)}`);
  }
  return classes.join(" ");
}

function safeCssToken(value) {
  return String(value || "plain").replace(/[^a-zA-Z0-9_-]/g, "-");
}

function semanticTokenPayload() {
  return state.check?.semanticTokens ?? state.check?.semantic_tokens ?? { legend: {}, tokens: [] };
}

function filteredSemanticTokens(tokens) {
  const query = state.highlightTokenQuery.trim().toLowerCase();
  if (!query) return tokens;
  return tokens.filter((token) => semanticTokenSearchText(token).includes(query));
}

function semanticTokenSearchText(token) {
  const line = Number(token?.line ?? -1) + 1;
  return [
    semanticTokenText(token),
    token?.type,
    ...arrayOrEmpty(token?.modifiers),
    Number.isFinite(line) && line > 0 ? `L${line}` : "",
    Number.isFinite(line) && line > 0 ? `line:${line}` : "",
    token?.start,
    token?.length
  ].map((part) => String(part ?? "").toLowerCase()).join(" ");
}

function arrayOrEmpty(value) {
  return Array.isArray(value) ? value : [];
}

function countSemanticTokens(items, keyFn) {
  const counts = new Map();
  for (const item of items || []) {
    const key = keyFn(item);
    counts.set(key, (counts.get(key) || 0) + 1);
  }
  return counts;
}

function renderTree(nodes, depth) {
  return nodes.map((node) => {
    const indent = depth * 13;
    const isDir = node.kind === "dir";
    const isOpen = isDir && state.openDirs.has(node.path);
    const icon = isDir ? (isOpen ? "v" : ">") : "";
    const active = node.path === state.currentPath ? " active" : "";
    const run = isDir && normalizePath(node.path) === normalizePath(state.runDir) ? " run-dir" : "";
    const children = isOpen && node.children?.length ? renderTree(node.children, depth + 1) : "";
    return `
      <div class="node ${node.kind}${active}${run}" style="padding-left:${6 + indent}px" data-kind="${node.kind}" data-path="${escapeAttr(node.path)}">
        <span>${icon}</span><span>${escapeHtml(node.name)}</span>
      </div>
      ${children}
    `;
  }).join("");
}

function inspectorRows(key) {
  const value = state.inspectors?.[key];
  return Array.isArray(value) ? value : [];
}

function inspectorObject(key) {
  const value = state.inspectors?.[key];
  if (!value || Array.isArray(value) || typeof value !== "object") return {};
  return value;
}

function compactText(value, limit = 80) {
  const text = String(value ?? "").replace(/\s+/g, " ").trim();
  if (text.length <= limit) return text;
  return `${text.slice(0, Math.max(0, limit - 1))}...`;
}

function matrixSummary(matrix) {
  if (!Array.isArray(matrix) || !matrix.length) return "canonical -";
  return `canonical ${matrix.map((row) => {
    if (!Array.isArray(row)) return "[]";
    return `[${row.map((value) => metricCell(value)).join(", ")}]`;
  }).join("; ")}`;
}

function entriesSummary(entries) {
  if (!Array.isArray(entries) || !entries.length) return "entries -";
  return `entries ${entries.slice(0, 6).map((entry) => {
    const row = entry.row_member ?? entry.rowMember ?? `r${entry.row_index ?? entry.rowIndex ?? "-"}`;
    const column = entry.column_member ?? entry.columnMember ?? `c${entry.column_index ?? entry.columnIndex ?? "-"}`;
    const coefficient = entry.coefficient ?? "-";
    return `${row}<-${column}: ${metricCell(coefficient)}`;
  }).join("; ")}${entries.length > 6 ? " ..." : ""}`;
}

function columnSummary(columns) {
  if (!Array.isArray(columns) || !columns.length) return "-";
  return columns.map((column) => {
    const unit = column.unit || column.canonical_unit || column.canonicalUnit || "";
    const suffix = unit ? ` [${unit}]` : "";
    return `${column.name || "column"}: ${column.type || "-"}${suffix}`;
  }).join("; ");
}

function fieldSummary(fields) {
  if (!Array.isArray(fields) || !fields.length) return "-";
  return fields.map((field) => {
    const unit = field.display_unit || field.displayUnit || "";
    const suffix = unit ? ` [${unit}]` : "";
    const type = field.quantity_kind || field.quantityKind || "-";
    return `${field.name || "field"}: ${type}${suffix}`;
  }).join("; ");
}

function validationSummary(validations) {
  if (!Array.isArray(validations) || !validations.length) return "-";
  const passed = validations.filter((validation) => validation.status === "passed").length;
  const failed = validations.filter((validation) => validation.status === "failed").length;
  return `${passed} passed / ${failed} failed`;
}

function sourceLineButton(item) {
  const line = sourceLineValue(item);
  const lineNumber = Number(line);
  if (!Number.isFinite(lineNumber) || lineNumber < 1) {
    return line ? escapeHtml(line) : "-";
  }
  const safeLine = Math.trunc(lineNumber);
  return `<button class="link-button" data-source-line="${escapeAttr(safeLine)}">L${escapeHtml(safeLine)}</button>`;
}

function sourceLineValue(item) {
  return item?.source_span?.line
    ?? item?.sourceSpan?.line
    ?? item?.source_line
    ?? item?.sourceLine
    ?? item?.line;
}

function sourceTokenButton(token, label = null) {
  const line = Number(token?.line ?? -1) + 1;
  const start = Number(token?.start ?? -1);
  const length = Number(token?.length ?? 0);
  if (!Number.isFinite(line) || !Number.isFinite(start) || !Number.isFinite(length) || line <= 0 || start < 0 || length <= 0) {
    return "-";
  }
  const buttonLabel = label || `L${line}`;
  return `<button class="link-button token-range-button" data-source-token-line="${escapeAttr(line)}" data-source-token-start="${escapeAttr(start)}" data-source-token-length="${escapeAttr(length)}" title="Select token range">${escapeHtml(buttonLabel)}</button>`;
}

function sourceBreadcrumbs(label, items) {
  const lines = [...new Set((items || [])
    .map(sourceLineValue)
    .filter((line) => Number.isFinite(Number(line)) && Number(line) > 0)
    .map((line) => Number(line)))]
    .sort((left, right) => left - right);
  if (!lines.length) return "";
  const visible = lines.slice(0, 10);
  const hidden = lines.length - visible.length;
  return `
    <div class="source-breadcrumbs">
      <span>${escapeHtml(label)}</span>
      ${visible.map((line) => `<button class="link-button" data-source-line="${escapeAttr(line)}">L${escapeHtml(line)}</button>`).join("")}
      ${hidden > 0 ? `<span class="muted">+${hidden}</span>` : ""}
    </div>
  `;
}

function rawJsonToggle(title, payload) {
  if (!hasPayloadData(payload)) return "";
  return `
    <details class="raw-json-toggle">
      <summary>${escapeHtml(title)}</summary>
      <pre>${escapeHtml(JSON.stringify(payload, null, 2))}</pre>
    </details>
  `;
}

function hasPayloadData(payload) {
  if (Array.isArray(payload)) return payload.length > 0;
  return Boolean(payload && typeof payload === "object" && Object.keys(payload).length);
}

function compactObjectSummary(value) {
  if (!value || typeof value !== "object") return String(value ?? "-");
  return Object.entries(value)
    .slice(0, 4)
    .map(([key, item]) => {
      const text = Array.isArray(item)
        ? `[${item.length}]`
        : (item && typeof item === "object" ? "{...}" : String(item ?? "-"));
      return `${key}=${text}`;
    })
    .join(", ") || "-";
}

function openPathButton(path, limit = 80) {
  const text = String(path ?? "").trim();
  if (!text || text === "-") return `<code>-</code>`;
  return `
    <button class="link-button" data-open-path="${escapeAttr(text)}">
      <code title="${escapeAttr(text)}">${escapeHtml(compactText(text, limit))}</code>
    </button>
  `;
}

function openPathStack(paths, limit = 80) {
  const rows = paths
    .map((path) => String(path ?? "").trim())
    .filter((path) => path && path !== "-");
  if (!rows.length) return `<code>-</code>`;
  return rows.map((path, index) => {
    const content = openPathButton(path, limit);
    return index === 0 ? content : `<div class="muted">${content}</div>`;
  }).join("");
}

function renderOutputPathList(outputs, limit = 80) {
  if (!Array.isArray(outputs) || !outputs.length) return "-";
  return outputs.map((output) => {
    const path = typeof output === "string" ? output : output?.path;
    const status = typeof output === "object" ? (output?.status || "-") : "";
    const statusText = status ? `<span class="muted"> ${escapeHtml(status)}</span>` : "";
    return `<div>${openPathButton(path, limit)}${statusText}</div>`;
  }).join("");
}

function metricCell(value) {
  if (value === null || value === undefined || value === "") return "-";
  if (typeof value === "number") return fmt(value);
  return escapeHtml(value);
}

function renderVariables() {
  const rows = state.variables.map((variable) => `
    <tr data-variable="${escapeAttr(variable.name)}">
      <td><strong>${escapeHtml(variable.name)}</strong></td>
      <td>${escapeHtml(variable.quantityKind || "-")}</td>
      <td>${escapeHtml(variable.displayUnit || "-")}</td>
      <td><code>${escapeHtml(variable.value || "-")}</code></td>
      <td>${variableSourceCell(variable)}</td>
    </tr>
    ${state.selectedVariable === variable.name ? `<tr><td colspan="5">${renderVariableDetail(variable)}</td></tr>` : ""}
  `).join("");
  const args = state.args.length ? `
    <div class="panel-title">Args</div>
    <table class="var-table">
      <thead><tr><th>Name</th><th>Type</th><th>Value</th><th>Source</th></tr></thead>
      <tbody>${state.args.map((arg) => `<tr><td>${escapeHtml(arg.name)}</td><td>${escapeHtml(arg.typeName)}</td><td><code>${escapeHtml(arg.value)}</code></td><td>${variableSourceCell(arg)}</td></tr>`).join("")}</tbody>
    </table>
  ` : "";
  return `
    <table class="var-table">
      <thead><tr><th>Name</th><th>Kind</th><th>Unit</th><th>Value</th><th>Source</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="5" class="muted">Run a file to populate runtime variables.</td></tr>`}</tbody>
    </table>
    ${args}
  `;
}

function variableSourceCell(item) {
  const source = escapeHtml(item?.source || "-");
  const lineButton = sourceLineButton(item);
  if (lineButton === "-") return source;
  return `${source}<div class="muted variable-source-line">${lineButton}</div>`;
}

function renderVariableDetail(variable) {
  return `
    <div class="var-detail">
      <div>line: ${escapeHtml(String(variable.line || "-"))}</div>
      <div>canonical: ${escapeHtml(variable.canonicalUnit || "-")}</div>
      <div>dimension: ${escapeHtml(variable.dimension || "-")}</div>
      <div>role: ${escapeHtml(variable.role || "-")}</div>
    </div>
  `;
}

function renderProblems() {
  const diagnostics = state.check.diagnostics || [];
  const codes = problemCodeOptions(diagnostics);
  const activeCode = codes.includes(state.problemCode) ? state.problemCode : "all";
  const filtered = filteredProblems(activeCode);
  const rows = filtered.map((diag) => `
    <tr class="problem-row" data-problem-line="${escapeAttr(diag.line || 0)}" title="Jump to source line ${escapeAttr(diag.line || "-")}">
      <td class="${diag.severity === "error" ? "error" : "warning"}">${escapeHtml(diag.severity)}</td>
      <td>${sourceLineButton(diag)}</td>
      <td><code>${escapeHtml(diag.code)}</code></td>
      <td>${escapeHtml(diag.message)}${diag.help ? `<div class="muted">help: ${escapeHtml(diag.help)}</div>` : ""}</td>
    </tr>
  `).join("");
  return `
    <div class="problem-panel">
      <div class="problem-toolbar">
        <div class="segmented" aria-label="Problem severity">
          ${["all", "error", "warning"].map((severity) => `
            <button class="${state.problemSeverity === severity ? "active" : ""}" data-problem-severity="${escapeAttr(severity)}">
              ${escapeHtml(problemSeverityLabel(severity, diagnostics))}
            </button>
          `).join("")}
        </div>
        <select id="problemCodeFilter" title="Diagnostic code">
          <option value="all">All codes</option>
          ${codes.map((code) => `<option value="${escapeAttr(code)}" ${activeCode === code ? "selected" : ""}>${escapeHtml(code)}</option>`).join("")}
        </select>
        <input id="problemQueryInput" class="problem-query" value="${escapeAttr(state.problemQuery)}" placeholder="Filter text" title="Filter by code, message, help, or line" />
        <button id="clearProblemFilters">Clear</button>
        <span class="muted">${filtered.length} of ${diagnostics.length}</span>
      </div>
      <div class="scroll problem-scroll">
      <table class="problems-table">
        <thead><tr><th>Severity</th><th>Line</th><th>Code</th><th>Message</th></tr></thead>
        <tbody>${rows || `<tr><td colspan="4" class="ok">${diagnostics.length ? "No diagnostics match the active filters" : "No diagnostics"}</td></tr>`}</tbody>
      </table>
      </div>
    </div>
  `;
}

function filteredProblems(activeCode = state.problemCode) {
  const query = state.problemQuery.trim().toLowerCase();
  return (state.check.diagnostics || []).filter((diag) => {
    const severityMatches = state.problemSeverity === "all" || diag.severity === state.problemSeverity;
    const codeMatches = activeCode === "all" || diag.code === activeCode;
    const queryMatches = !query || [
      diag.severity,
      diag.code,
      diag.message,
      diag.help,
      `line ${diag.line}`,
      `l${diag.line}`
    ].some((value) => String(value || "").toLowerCase().includes(query));
    return severityMatches && codeMatches && queryMatches;
  });
}

function problemCodeOptions(diagnostics) {
  return [...new Set(diagnostics.map((diag) => diag.code).filter(Boolean))].sort();
}

function problemSeverityLabel(severity, diagnostics) {
  if (severity === "all") return `All ${diagnostics.length}`;
  const count = diagnostics.filter((diag) => diag.severity === severity).length;
  return `${severity === "error" ? "Errors" : "Warnings"} ${count}`;
}

function renderTerminal() {
  return `
    <div class="terminal">
      <div class="terminal-bar">
        <span>${escapeHtml(state.runDir || ".")}</span>
        <div>
          <button id="terminalPlot">Plot</button>
          <button id="terminalReset">Reset</button>
          <button id="terminalClear">Clear</button>
        </div>
      </div>
      <div class="terminal-log">${renderTerminalEntries()}</div>
      <div class="terminal-input">
        <span class="prompt">${escapeHtml(terminalPrompt())}</span>
        <input id="terminalInput" placeholder="type EngLang command, run, check, reset, clear" />
        <button class="primary" id="terminalSend">Enter</button>
      </div>
    </div>
  `;
}

function renderTerminalEntries() {
  return state.terminalEntries.map((entry) => `
    <div class="terminal-entry ${escapeAttr(entry.kind)}">${escapeHtml(entry.text)}</div>
  `).join("");
}

function drawPlot(canvasId) {
  const canvas = byId(canvasId);
  if (!canvas || !state.plotSpec) return;
  const ctx = canvas.getContext("2d");
  const dpr = window.devicePixelRatio || 1;
  const rect = canvas.getBoundingClientRect();
  canvas.width = Math.max(1, Math.floor(rect.width * dpr));
  canvas.height = Math.max(1, Math.floor(rect.height * dpr));
  ctx.scale(dpr, dpr);
  ctx.clearRect(0, 0, rect.width, rect.height);
  ctx.fillStyle = "#f8fafc";
  ctx.fillRect(0, 0, rect.width, rect.height);

  const seriesList = state.plotSpec.series ?? [];
  const series = seriesList[0] ?? {};
  const points = series.points ?? [];
  const bins = series.bins ?? [];
  const isHistogram = state.plotSpec.plot_type === "histogram" && bins.length;
  const left = 68;
  const right = rect.width - 28;
  const top = 26;
  const bottom = rect.height - 50;
  const bounds = isHistogram ? boundsFromBins(bins) : boundsFromSeries(seriesList);
  const xTicks = ticks(bounds.minX, bounds.maxX, 5);
  const yTicks = ticks(bounds.minY, bounds.maxY, 5);

  ctx.strokeStyle = "#dce4ef";
  ctx.lineWidth = 1;
  ctx.fillStyle = "#66758a";
  ctx.font = "11px Segoe UI";
  for (const tick of xTicks) {
    const x = sx(tick, bounds, left, right);
    line(ctx, x, top, x, bottom);
    ctx.textAlign = "center";
    ctx.fillText(fmt(tick), x, bottom + 18);
  }
  for (const tick of yTicks) {
    const y = sy(tick, bounds, top, bottom);
    line(ctx, left, y, right, y);
    ctx.textAlign = "right";
    ctx.fillText(fmt(tick), left - 8, y + 4);
  }

  ctx.strokeStyle = "#4b5563";
  ctx.lineWidth = 1.5;
  line(ctx, left, bottom, right, bottom);
  line(ctx, left, top, left, bottom);

  const colors = ["#1f6fc8", "#26805a", "#a86d14", "#7c3aed", "#b82c2c"];
  ctx.fillStyle = colors[0];
  ctx.strokeStyle = colors[0];
  ctx.lineWidth = 2;
  if (isHistogram) {
    const baseline = sy(0, bounds, top, bottom);
    for (const bin of bins) {
      const x1 = sx(Math.min(bin.lower, bin.upper), bounds, left, right);
      const x2 = sx(Math.max(bin.lower, bin.upper), bounds, left, right);
      const y = sy(bin.count, bounds, top, bottom);
      ctx.fillRect(x1, y, Math.max(2, x2 - x1), Math.max(1, baseline - y));
    }
  } else if (state.plotSpec.plot_type === "bar") {
    const width = ((right - left) / Math.max(1, points.length)) * 0.68;
    const baseline = sy(Math.max(0, bounds.minY), bounds, top, bottom);
    for (const point of points) {
      const x = sx(point[0], bounds, left, right);
      const y = sy(point[1], bounds, top, bottom);
      ctx.fillRect(x - width / 2, Math.min(y, baseline), width, Math.abs(baseline - y));
    }
  } else {
    seriesList.forEach((item, seriesIndex) => {
      const itemPoints = item.points ?? [];
      ctx.strokeStyle = colors[seriesIndex % colors.length];
      ctx.beginPath();
      itemPoints.forEach((point, index) => {
        const x = sx(point[0], bounds, left, right);
        const y = sy(point[1], bounds, top, bottom);
        if (index === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      });
      ctx.stroke();
    });
    drawLegend(ctx, seriesList, colors, left, top);
  }

  ctx.fillStyle = "#1f2937";
  ctx.font = "13px Segoe UI";
  ctx.textAlign = "center";
  ctx.fillText(axisLabel(state.plotSpec.x_axis), (left + right) / 2, rect.height - 14);
  ctx.save();
  ctx.translate(18, (top + bottom) / 2);
  ctx.rotate(-Math.PI / 2);
  ctx.fillText(axisLabel(state.plotSpec.y_axis), 0, 0);
  ctx.restore();
}

function axisLabel(axis) {
  if (!axis) return "";
  return axis.unit ? `${axis.label || axis.name || ""} [${axis.unit}]` : (axis.label || axis.name || "");
}

function boundsFromPoints(points) {
  if (!points.length) return { minX: 0, maxX: 1, minY: 0, maxY: 1 };
  let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
  for (const point of points) {
    minX = Math.min(minX, point[0]);
    maxX = Math.max(maxX, point[0]);
    minY = Math.min(minY, point[1]);
    maxY = Math.max(maxY, point[1]);
  }
  return padBounds({ minX, maxX, minY, maxY });
}

function boundsFromSeries(seriesList) {
  const points = seriesList.flatMap((series) => series.points ?? []);
  return boundsFromPoints(points);
}

function boundsFromBins(bins) {
  let minX = Infinity, maxX = -Infinity, maxY = 1;
  for (const bin of bins) {
    minX = Math.min(minX, bin.lower, bin.upper);
    maxX = Math.max(maxX, bin.lower, bin.upper);
    maxY = Math.max(maxY, bin.count);
  }
  return padBounds({ minX, maxX, minY: 0, maxY });
}

function drawLegend(ctx, seriesList, colors, left, top) {
  const labelled = seriesList.filter((series) => series.label || series.name).slice(0, 5);
  if (labelled.length < 2) return;
  ctx.font = "11px Segoe UI";
  ctx.textAlign = "left";
  labelled.forEach((series, index) => {
    const y = top + 12 + index * 16;
    ctx.strokeStyle = colors[index % colors.length];
    line(ctx, left + 6, y - 4, left + 24, y - 4);
    ctx.fillStyle = "#344054";
    ctx.fillText(series.label || series.name, left + 30, y);
  });
}

function padBounds(bounds) {
  if (bounds.minX === bounds.maxX) {
    bounds.minX -= 0.5;
    bounds.maxX += 0.5;
  }
  if (bounds.minY === bounds.maxY) {
    bounds.minY -= 0.5;
    bounds.maxY += 0.5;
  }
  return bounds;
}

function sx(value, bounds, left, right) {
  return left + ((value - bounds.minX) / (bounds.maxX - bounds.minX)) * (right - left);
}

function sy(value, bounds, top, bottom) {
  return bottom - ((value - bounds.minY) / (bounds.maxY - bounds.minY)) * (bottom - top);
}

function ticks(min, max, count) {
  if (min === max) return [min];
  return Array.from({ length: count }, (_, i) => min + ((max - min) * i) / (count - 1));
}

function fmt(value) {
  const abs = Math.abs(value);
  if (abs >= 1000) return value.toFixed(0);
  if (abs >= 10) return value.toFixed(1);
  if (abs >= 1) return value.toFixed(2);
  return value.toFixed(3);
}

function line(ctx, x1, y1, x2, y2) {
  ctx.beginPath();
  ctx.moveTo(x1, y1);
  ctx.lineTo(x2, y2);
  ctx.stroke();
}

function bindSplitters() {
  document.querySelectorAll("[data-splitter]").forEach((splitter) => {
    splitter.onpointerdown = (event) => {
      event.preventDefault();
      splitter.setPointerCapture(event.pointerId);
      splitter.classList.add("dragging");
      const kind = splitter.dataset.splitter;
      const move = (moveEvent) => {
        if (kind === "left") {
          document.documentElement.style.setProperty("--left", `${Math.max(190, Math.min(560, moveEvent.clientX))}px`);
        } else if (kind === "right") {
          document.documentElement.style.setProperty("--right", `${Math.max(260, Math.min(680, window.innerWidth - moveEvent.clientX))}px`);
        } else {
          document.documentElement.style.setProperty("--bottom", `${Math.max(130, Math.min(560, window.innerHeight - moveEvent.clientY))}px`);
        }
      };
      const up = () => {
        splitter.classList.remove("dragging");
        window.removeEventListener("pointermove", move);
        window.removeEventListener("pointerup", up);
      };
      window.addEventListener("pointermove", move);
      window.addEventListener("pointerup", up);
    };
  });
}

function bindGlobalEvents() {
  if (dragDropBound) return;
  dragDropBound = true;
  if (listen) {
    listen("tauri://drag-drop", (event) => {
      const payload = event?.payload;
      const path = payload?.paths?.[0] || payload?.[0];
      if (path) openFile(path);
    }).catch(() => {});
  }
  window.addEventListener("dragover", (event) => {
    event.preventDefault();
    document.body.classList.add("dragging-file");
  });
  window.addEventListener("dragleave", () => {
    document.body.classList.remove("dragging-file");
  });
  window.addEventListener("drop", (event) => {
    event.preventDefault();
    document.body.classList.remove("dragging-file");
    const file = Array.from(event.dataTransfer?.files || [])[0];
    const path = file?.path || file?.name;
    if (path) openFile(path);
  });
}

function terminalPrompt() {
  return `EngLang ${state.runDir || currentDirectory()} >> `;
}

function currentDirectory() {
  return directoryOf(state.currentPath);
}

function directoryOf(path) {
  const normalized = normalizePath(path);
  return normalized.split("/").slice(0, -1).join("/") || ".";
}

function normalizePath(path) {
  return String(path || "").replaceAll("\\", "/").replace(/\/+/g, "/").replace(/\/$/, "");
}

function setRunDir(path, rerender = true) {
  const normalized = resolveRunDirInput(path || ".");
  state.runDir = normalized || ".";
  openParentDirs(`${state.runDir}/__dir__.eng`);
  state.status = `Run directory ${state.runDir}`;
  if (rerender) render();
}

function resolveRunDirInput(path) {
  const text = normalizePath(path);
  if (!text || text === ".") return state.runDir || currentDirectory();
  if (text === "..") return parentPath(state.runDir || currentDirectory());
  if (isAbsolutePath(text)) return text;
  const base = state.runDir || currentDirectory();
  return normalizePath(`${base}/${text}`);
}

function parentPath(path) {
  const normalized = normalizePath(path);
  const parts = normalized.split("/");
  if (parts.length <= 1) return ".";
  return parts.slice(0, -1).join("/") || ".";
}

function isAbsolutePath(path) {
  return /^[A-Za-z]:\//.test(path) || path.startsWith("/");
}

function toggleDir(path) {
  if (state.openDirs.has(path)) state.openDirs.delete(path);
  else state.openDirs.add(path);
  render();
}

function openParentDirs(path) {
  for (const dir of parentDirs(path)) {
    state.openDirs.add(dir);
  }
}

function parentDirs(path) {
  const normalized = normalizePath(path);
  const parts = normalized.split("/");
  const dirs = [];
  for (let index = 1; index < parts.length; index += 1) {
    dirs.push(parts.slice(0, index).join("/"));
  }
  return dirs;
}

function compactPath(path) {
  const text = String(path || "");
  if (text.length <= 56) return text;
  const normalized = text.replaceAll("\\", "/");
  const parts = normalized.split("/");
  if (parts.length <= 3) return `...${text.slice(-52)}`;
  return `${parts[0]}/.../${parts.slice(-2).join("/")}`;
}

function errorCount() {
  return state.check.diagnostics.filter((d) => d.severity === "error").length;
}

function warningCount() {
  return state.check.diagnostics.filter((d) => d.severity === "warning").length;
}

function lineCount(text) {
  return Math.max(1, text.split(/\r\n|\r|\n/).length);
}

function fileName(path) {
  return path.split(/[\\/]/).pop() || path;
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function escapeAttr(value) {
  return escapeHtml(value).replaceAll("'", "&#39;");
}

boot();
