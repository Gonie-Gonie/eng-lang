const invoke = window.__TAURI__?.core?.invoke;

const state = {
  root: "",
  fileTree: [],
  tabs: [],
  completions: [],
  completionItems: [],
  completionIndex: 0,
  currentPath: "",
  source: "",
  dirty: false,
  check: { diagnostics: [], symbols: [], status: "" },
  variables: [],
  args: [],
  plotSpec: null,
  reportTitle: "",
  terminalEntries: [{ kind: "info", text: "Ready." }],
  bottomTab: "terminal",
  sideTab: "variables",
  selectedVariable: null,
  status: "Starting"
};

let dragDropBound = false;

function byId(id) {
  return document.getElementById(id);
}

async function call(cmd, args = {}) {
  if (!invoke) throw new Error("Tauri invoke API is not available");
  return await invoke(cmd, args);
}

async function boot() {
  try {
    const data = await call("ide_bootstrap");
    state.root = data.root;
    state.fileTree = data.fileTree;
    state.completions = data.completions ?? [];
    state.currentPath = data.current.path;
    state.source = data.current.source;
    state.tabs = [{ path: state.currentPath, source: state.source, dirty: false }];
    state.check = data.check;
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
    <div class="toolbar">
      <div class="title-mark">EngLang</div>
      <button class="tool primary" id="runBtn" title="Run current file">Run</button>
      <button class="tool" id="checkBtn" title="Check diagnostics">Check</button>
      <button class="tool" id="saveBtn" title="Save current file">Save</button>
      <span class="toolbar-separator"></span>
      <button class="tool" id="reportBtn" title="Open last report">Report</button>
      <button class="tool" id="plotBtn" title="Show plot panel">Plot</button>
      <span class="badge ${errorCount() ? "bad" : ""}">Errors ${errorCount()}</span>
      <span class="badge ${warningCount() ? "warn" : ""}">Warnings ${warningCount()}</span>
      <span class="status">${escapeHtml(state.status)}</span>
    </div>
    <div class="pathbar">
      <span class="path-label">Workspace</span>
      <span class="workspace-root" title="${escapeAttr(state.root)}">${escapeHtml(compactPath(state.root))}</span>
      <span class="path-label">File</span>
      <input id="pathInput" value="${escapeAttr(state.currentPath)}" />
      <button id="openPathBtn">Open</button>
    </div>
    <aside class="sidebar">
      <div class="panel-title">Explorer</div>
      <div class="scroll tree">${renderTree(state.fileTree, 0)}</div>
    </aside>
    <div class="splitter splitter-left" data-splitter="left"></div>
    <main class="editor-wrap">
      <div class="editor-tabs">${renderTabs()}</div>
      <div class="editor-meta">
        <span>${escapeHtml(state.currentPath)}</span>
        <span>${lineCount(state.source)} lines</span>
      </div>
      <div class="editor-shell">
        <textarea id="editor" class="editor" spellcheck="false">${escapeHtml(state.source)}</textarea>
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
  `;
  bind();
  bindGlobalEvents();
  if (state.sideTab === "plot" && state.plotSpec) drawPlot("sidePlotCanvas");
}

function bind() {
  const editor = byId("editor");
  editor.addEventListener("keydown", handleEditorKeyDown);
  editor.addEventListener("keyup", (event) => {
    if (["ArrowDown", "ArrowUp", "Enter", "Tab", "Escape"].includes(event.key)) return;
    updateCompletionOverlay();
  });
  editor.addEventListener("click", updateCompletionOverlay);
  editor.addEventListener("input", (event) => {
    state.source = event.target.value;
    state.dirty = true;
    rememberCurrentTab();
    state.status = "Modified";
    renderTabLabels();
    updateCompletionOverlay();
  });
  byId("checkBtn").onclick = checkCurrent;
  byId("saveBtn").onclick = saveCurrent;
  byId("runBtn").onclick = runCurrent;
  byId("reportBtn").onclick = () => openArtifact("report");
  byId("plotBtn").onclick = () => {
    state.sideTab = "plot";
    render();
  };
  byId("openPathBtn").onclick = () => openFile(byId("pathInput").value);
  byId("pathInput").addEventListener("keydown", (event) => {
    if (event.key === "Enter") openFile(event.currentTarget.value);
  });
  document.querySelectorAll("[data-path]").forEach((node) => {
    node.onclick = () => {
      if (node.dataset.kind === "file") openFile(node.dataset.path);
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
  document.querySelectorAll("[data-side-tab]").forEach((tab) => {
    tab.onclick = () => {
      state.sideTab = tab.dataset.sideTab;
      render();
    };
  });
  document.querySelectorAll("[data-variable]").forEach((row) => {
    row.onclick = () => {
      state.selectedVariable = state.selectedVariable === row.dataset.variable ? null : row.dataset.variable;
      render();
    };
  });
  const openPlotArtifact = byId("openPlotArtifact");
  if (openPlotArtifact) openPlotArtifact.onclick = () => openArtifact("plot");
  const terminalInput = byId("terminalInput");
  if (terminalInput) {
    terminalInput.focus();
    terminalInput.addEventListener("keydown", (event) => {
      if (event.key === "Enter") sendTerminal();
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
    state.completionItems = [];
    state.plotSpec = null;
    state.reportTitle = "";
    state.status = `Loaded ${file.path}`;
    const check = await call("ide_check", { path: state.currentPath, source: state.source });
    state.check = check;
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
    state.check = await call("ide_check", { path: state.currentPath, source: state.source });
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
    const result = await call("ide_run", { path: state.currentPath, source: state.source });
    applyRun(result);
    appendRunResult(result);
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
  await sendTerminalCommand(command);
}

async function sendTerminalCommand(command) {
  const prompt = terminalPrompt();
  if (command.toLowerCase() === "clear") {
    clearTerminal();
    render();
    return;
  }
  appendTerminal("command", `${prompt}${command}`);
  try {
    const result = await call("ide_terminal", {
      path: state.currentPath,
      source: state.source,
      command
    });
    applyRun(result);
    appendRunResult(result);
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

function applyRun(result) {
  state.check = result.check ?? state.check;
  state.variables = result.variables ?? state.variables;
  state.args = result.args ?? state.args;
  state.plotSpec = result.plotSpec && Object.keys(result.plotSpec).length ? result.plotSpec : state.plotSpec;
  state.reportTitle = result.reportTitle ?? state.reportTitle;
  if (result.plotSpec && Object.keys(result.plotSpec).length) state.sideTab = "plot";
}

function appendRunResult(result) {
  const text = (result.terminal || "").trim();
  if (text) appendTerminal(result.ok ? "stdout" : "error", text);
  if (!text && result.ok) appendTerminal("info", "Run complete.");
  if (!result.ok) state.bottomTab = "problems";
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
  state.source = tab.source;
  state.dirty = tab.dirty;
  state.variables = [];
  state.args = [];
  state.completionItems = [];
  state.plotSpec = null;
  state.reportTitle = "";
  state.status = `Loaded ${tab.path}`;
  try {
    state.check = await call("ide_check", { path: state.currentPath, source: state.source });
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
  state.source = next.source;
  state.dirty = next.dirty;
  state.variables = [];
  state.args = [];
  state.completionItems = [];
  state.plotSpec = null;
  state.reportTitle = "";
  call("ide_check", { path: state.currentPath, source: state.source })
    .then((check) => {
      state.check = check;
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
        <button class="side-tab ${state.sideTab === "variables" ? "active" : ""}" data-side-tab="variables">Variables</button>
        <button class="side-tab ${state.sideTab === "plot" ? "active" : ""}" data-side-tab="plot">Plot</button>
        <button class="side-tab ${state.sideTab === "run" ? "active" : ""}" data-side-tab="run">Run</button>
      </div>
      <div class="side-body">${renderSideBody()}</div>
    </aside>
  `;
}

function renderSideBody() {
  if (state.sideTab === "plot") return renderPlotPanel();
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

function renderRunPanel() {
  return `
    <div class="panel-title compact">Run Context</div>
    <div class="run-info">
      <div><span>Workspace</span><code title="${escapeAttr(state.root)}">${escapeHtml(compactPath(state.root))}</code></div>
      <div><span>Directory</span><code>${escapeHtml(currentDirectory())}</code></div>
      <div><span>File</span><code>${escapeHtml(state.currentPath || "-")}</code></div>
      <div><span>Status</span><code>${escapeHtml(state.check.status || "-")}</code></div>
      <div><span>Report</span><code>${escapeHtml(state.reportTitle || "-")}</code></div>
    </div>
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
  hideCompletions();
  editor.focus();
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

function renderTree(nodes, depth) {
  return nodes.map((node) => {
    const indent = depth * 13;
    const icon = node.kind === "dir" ? ">" : "";
    const active = node.path === state.currentPath ? " active" : "";
    const children = node.children?.length ? renderTree(node.children, depth + 1) : "";
    return `
      <div class="node ${node.kind}${active}" style="padding-left:${6 + indent}px" data-kind="${node.kind}" data-path="${escapeAttr(node.path)}">
        <span>${icon}</span><span>${escapeHtml(node.name)}</span>
      </div>
      ${children}
    `;
  }).join("");
}

function renderVariables() {
  const rows = state.variables.map((variable) => `
    <tr data-variable="${escapeAttr(variable.name)}">
      <td><strong>${escapeHtml(variable.name)}</strong></td>
      <td>${escapeHtml(variable.quantityKind || "-")}</td>
      <td>${escapeHtml(variable.displayUnit || "-")}</td>
      <td><code>${escapeHtml(variable.value || "-")}</code></td>
      <td>${escapeHtml(variable.source || "-")}</td>
    </tr>
    ${state.selectedVariable === variable.name ? `<tr><td colspan="5">${renderVariableDetail(variable)}</td></tr>` : ""}
  `).join("");
  const args = state.args.length ? `
    <div class="panel-title">Args</div>
    <table class="var-table">
      <thead><tr><th>Name</th><th>Type</th><th>Value</th><th>Source</th></tr></thead>
      <tbody>${state.args.map((arg) => `<tr><td>${escapeHtml(arg.name)}</td><td>${escapeHtml(arg.typeName)}</td><td><code>${escapeHtml(arg.value)}</code></td><td>${escapeHtml(arg.source)}</td></tr>`).join("")}</tbody>
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
  const rows = state.check.diagnostics.map((diag) => `
    <tr>
      <td class="${diag.severity === "error" ? "error" : "warning"}">${escapeHtml(diag.severity)}</td>
      <td>L${diag.line}</td>
      <td><code>${escapeHtml(diag.code)}</code></td>
      <td>${escapeHtml(diag.message)}${diag.help ? `<div class="muted">help: ${escapeHtml(diag.help)}</div>` : ""}</td>
    </tr>
  `).join("");
  return `
    <div class="scroll" style="height:100%">
      <table class="problems-table">
        <thead><tr><th>Severity</th><th>Line</th><th>Code</th><th>Message</th></tr></thead>
        <tbody>${rows || `<tr><td colspan="4" class="ok">No diagnostics</td></tr>`}</tbody>
      </table>
    </div>
  `;
}

function renderTerminal() {
  return `
    <div class="terminal">
      <div class="terminal-bar">
        <span>${escapeHtml(currentDirectory())}</span>
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
  return `EngLang ${currentDirectory()} >> `;
}

function currentDirectory() {
  const normalized = state.currentPath.replaceAll("\\", "/");
  return normalized.split("/").slice(0, -1).join("/") || ".";
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
