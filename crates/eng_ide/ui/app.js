const invoke = window.__TAURI__?.core?.invoke;
const listen = window.__TAURI__?.event?.listen;
const RUN_HISTORY_LIMIT = 40;
const RUN_HISTORY_STORAGE_PREFIX = "englang.nativeIde.runHistory.v1:";
const LIVE_CHECK_DELAY_MS = 350;
const WORKSPACE_SYMBOL_DELAY_MS = 160;
const EDITOR_INDENT = "    ";
const EDITOR_PAIR_CLOSE = { "{": "}", "[": "]", "(": ")", "\"": "\"" };
const EDITOR_PAIR_OPEN = { "}": "{", "]": "[", ")": "(", "\"": "\"" };

const SIDE_TABS = [
  { key: "variables", label: "Variables" },
  { key: "units", label: "Units" },
  { key: "schema", label: "Schema" },
  { key: "time", label: "Time" },
  { key: "tables", label: "Tables" },
  { key: "reads", label: "Reads" },
  { key: "plot", label: "Plot" },
  { key: "review", label: "Review" },
  { key: "highlight", label: "Highlight" },
  { key: "quality", label: "Quality" },
  { key: "checks", label: "Checks" },
  { key: "effects", label: "Effects" },
  { key: "network", label: "Network" },
  { key: "artifacts", label: "Artifacts" },
  { key: "workflow", label: "Workflow" },
  { key: "case", label: "Case" },
  { key: "model", label: "Model" },
  { key: "db", label: "DB" },
  { key: "run", label: "Run" },
  { key: "modules", label: "Modules" },
  { key: "objects", label: "Objects" },
  { key: "assembly", label: "Assembly" },
  { key: "kernels", label: "Kernel" }
];

const LEXICAL_KEYWORD_GROUP_ORDER = [
  "deprecated", "import", "declaration", "function", "test", "block", "modifier",
  "side_effect", "external_boundary", "validation", "report", "solver", "workflow"
];
const LEXICAL_KEYWORD_GROUP_CLASSES = {
  deprecated: "hl-keyword hl-mod-deprecated",
  import: "hl-keyword hl-mod-imported",
  declaration: "hl-keyword hl-mod-declaration",
  function: "hl-keyword hl-function",
  test: "hl-keyword hl-mod-declaration",
  block: "hl-keyword hl-mod-local",
  modifier: "hl-keyword hl-modifier",
  side_effect: "hl-keyword hl-mod-sideEffect",
  external_boundary: "hl-keyword hl-mod-external",
  validation: "hl-keyword hl-mod-validation",
  report: "hl-keyword hl-mod-report",
  solver: "hl-keyword hl-mod-solver",
  workflow: "hl-keyword hl-mod-workflowStep"
};
const LEXICAL_WORKFLOW_BUILTIN_GROUP_ORDER = [
  "deprecated", "validation", "external", "path", "temporal", "model", "uncertain",
  "timeseries", "solver", "workflow_step"
];
const LEXICAL_WORKFLOW_BUILTIN_GROUP_CLASSES = {
  deprecated: "hl-function hl-mod-deprecated",
  validation: "hl-function hl-mod-validation",
  external: "hl-function hl-mod-external",
  path: "hl-function hl-mod-path",
  temporal: "hl-function hl-mod-temporal",
  model: "hl-function hl-mod-model",
  uncertain: "hl-function hl-mod-uncertain",
  timeseries: "hl-function hl-mod-timeseries",
  solver: "hl-function hl-mod-solver",
  workflow_step: "hl-function hl-mod-workflowStep"
};

const HOVER_KIND_LABELS = Object.freeze({
  variable: "Variable",
  domain: "Domain",
  domain_variable: "Domain variable",
  domain_conservation: "Domain conservation",
  component: "Component",
  component_port: "Component port",
  connection: "Connection",
  component_assembly: "Component assembly",
  connection_set: "Connection set",
  assembly_equation: "Assembly equation",
  function: "Function",
  function_local: "Function local",
  where_local: "where local",
  class: "Class",
  class_field: "Class field",
  class_validation: "Class validation",
  class_method: "Class method",
  class_object: "Class object",
  object_field: "Object field",
  object_validation: "Object validation",
  http_response_field: "HTTP response field",
  coverage_result_field: "Coverage result field",
  table_field: "Table field",
  sample_table_field: "Sample table field",
  db_connection_field: "DB connection field",
  case_table_field: "Case table field",
  case_output_table_field: "Case output field",
  case_result_collection_table_field: "Case result collection field",
  model_field: "Model field",
  prediction_table_field: "Prediction table field"
});

const state = {
  root: "",
  fileTree: [],
  tabs: [],
  completions: [],
  completionItems: [],
  completionIndex: 0,
  syntaxCatalog: emptySyntaxCatalog(),
  lexicalCatalog: buildLexicalCatalog(emptySyntaxCatalog()),
  modules: [],
  openDirs: new Set(["examples", "examples/official", "stdlib"]),
  currentPath: "",
  runDir: "",
  source: "",
  savedSource: "",
  dirty: false,
  check: { diagnostics: [], symbols: [], status: "", semanticTokens: { legend: {}, tokens: [] }, hovers: [], documentSymbols: [] },
  highlightSource: null,
  documentHighlights: { path: "", source: "", items: [] },
  workspaceReferences: { path: "", source: "", documents: [], label: "", items: [], notice: "" },
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
  outlineOpen: true,
  outlineQuery: "",
  highlightTokenQuery: "",
  editorFindOpen: false,
  editorFindQuery: "",
  editorFindCaseSensitive: false,
  editorFindMatchIndex: -1,
  pendingQuickFix: null,
  pendingRename: null,
  pendingWorkspaceSymbols: null,
  pendingTabClose: null,
  pendingWindowClose: false,
  sideTab: "variables",
  selectedVariable: null,
  selectedWorkflowNodeId: null,
  status: "Starting"
};

let dragDropBound = false;
let liveCheckTimer = null;
let liveCheckRevision = 0;
let navigationRevision = 0;
let definitionRequestRevision = 0;
let documentHighlightRequestRevision = 0;
let quickFixRequestRevision = 0;
let renameRequestRevision = 0;
let workspaceSymbolRequestRevision = 0;
let workspaceSymbolTimer = null;
let nativeAppWindow = null;
let nativeCloseListenerBound = false;

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

function emptySyntaxCatalog() {
  return {
    keywords: [],
    keywordGroups: {},
    workflowBuiltinGroups: {},
    constants: [],
    workflowStatusLiterals: [],
    operatorWords: [],
    workflowBuiltins: [],
    hyphenatedWorkflowBuiltins: [],
    legacyWorkflowBuiltinAliases: [],
    workflowOptions: [],
    legacyWorkflowOptionAliases: [],
    publicTypes: [],
    quantities: [],
    units: [],
    legacyUnitAliases: [],
    httpResponseFields: [],
    coverageResultFields: [],
    tableFields: [],
    sampleTableFields: [],
    dbConnectionFields: [],
    caseTableFields: [],
    caseOutputTableFields: [],
    caseRunResultTableFields: [],
    caseResultCollectionTableFields: [],
    modelFields: [],
    predictionTableFields: []
  };
}

function normalizeSyntaxCatalog(catalog) {
  const source = catalog && typeof catalog === "object" ? catalog : {};
  return {
    keywords: stringArray(source.keywords),
    keywordGroups: catalogKeywordGroups(source.keywordGroups ?? source.keyword_groups),
    workflowBuiltinGroups: catalogKeywordGroups(
      source.workflowBuiltinGroups ?? source.workflow_builtin_groups
    ),
    constants: stringArray(source.constants),
    workflowStatusLiterals: stringArray(source.workflowStatusLiterals ?? source.workflow_status_literals),
    operatorWords: stringArray(source.operatorWords ?? source.operator_words),
    workflowBuiltins: stringArray(source.workflowBuiltins ?? source.workflow_builtins),
    hyphenatedWorkflowBuiltins: stringArray(
      source.hyphenatedWorkflowBuiltins ?? source.hyphenated_workflow_builtins
    ),
    legacyWorkflowBuiltinAliases: stringArray(
      source.legacyWorkflowBuiltinAliases ?? source.legacy_workflow_builtin_aliases
    ),
    workflowOptions: catalogItemLabels(source.workflowOptions ?? source.workflow_options),
    legacyWorkflowOptionAliases: stringArray(
      source.legacyWorkflowOptionAliases ?? source.legacy_workflow_option_aliases
    ),
    publicTypes: catalogPublicTypeLabels(source.publicTypes ?? source.public_types),
    quantities: catalogItemLabels(source.quantities),
    units: catalogItemLabels(source.units),
    legacyUnitAliases: stringArray(source.legacyUnitAliases ?? source.legacy_unit_aliases),
    httpResponseFields: catalogFieldItems(source.httpResponseFields ?? source.http_response_fields),
    coverageResultFields: catalogFieldItems(source.coverageResultFields ?? source.coverage_result_fields),
    tableFields: catalogFieldItems(source.tableFields ?? source.table_fields),
    sampleTableFields: catalogFieldItems(source.sampleTableFields ?? source.sample_table_fields),
    dbConnectionFields: catalogFieldItems(source.dbConnectionFields ?? source.db_connection_fields),
    caseTableFields: catalogFieldItems(source.caseTableFields ?? source.case_table_fields),
    caseOutputTableFields: catalogFieldItems(source.caseOutputTableFields ?? source.case_output_table_fields),
    caseRunResultTableFields: catalogFieldItems(
      source.caseRunResultTableFields ?? source.case_run_result_table_fields
    ),
    caseResultCollectionTableFields: catalogFieldItems(
      source.caseResultCollectionTableFields ?? source.case_result_collection_table_fields
    ),
    modelFields: catalogFieldItems(source.modelFields ?? source.model_fields),
    predictionTableFields: catalogFieldItems(source.predictionTableFields ?? source.prediction_table_fields)
  };
}

function buildLexicalCatalog(catalog) {
  const normalized = normalizeSyntaxCatalog(catalog);
  const workflowBuiltinSet = new Set([
    ...normalized.workflowBuiltins,
    ...normalized.hyphenatedWorkflowBuiltins,
    ...normalized.legacyWorkflowBuiltinAliases
  ]);
  const keywordSet = new Set([
    ...normalized.keywords,
    ...workflowBuiltinSet
  ]);
  const unitLabels = uniqueStrings([
    ...normalized.units,
    ...normalized.legacyUnitAliases
  ]);
  return {
    keywords: keywordSet,
    keywordGroups: keywordGroupSets(normalized.keywordGroups),
    workflowBuiltinGroups: keywordGroupSets(normalized.workflowBuiltinGroups),
    workflowBuiltins: workflowBuiltinSet,
    workflowStatusLiterals: new Set(normalized.workflowStatusLiterals),
    operatorWords: new Set(normalized.operatorWords),
    constants: new Set(normalized.constants),
    workflowOptions: new Set([
      ...normalized.workflowOptions,
      ...normalized.legacyWorkflowOptionAliases
    ]),
    publicTypes: new Set(normalized.publicTypes),
    quantities: new Set(normalized.quantities),
    units: new Set(unitLabels),
    unitPattern: lexicalUnitPattern(unitLabels)
  };
}

function catalogKeywordGroups(value) {
  const groups = {};
  const source = value && typeof value === "object" ? value : {};
  for (const [group, items] of Object.entries(source)) {
    const words = stringArray(items);
    if (words.length) groups[group] = words;
  }
  return groups;
}

function keywordGroupSets(groups) {
  const result = {};
  for (const [group, words] of Object.entries(groups || {})) {
    result[group] = new Set(stringArray(words));
  }
  return result;
}

function stringArray(value) {
  return arrayOrEmpty(value).map((item) => String(item || "").trim()).filter(Boolean);
}

function catalogItemLabels(value) {
  return arrayOrEmpty(value)
    .map((item) => {
      if (typeof item === "string") return item;
      if (item && typeof item === "object") return item.label || item.base || "";
      return "";
    })
    .map((item) => String(item || "").trim())
    .filter(Boolean);
}

function catalogFieldItems(value) {
  const fields = [];
  const seen = new Set();
  for (const item of arrayOrEmpty(value)) {
    const label = typeof item === "string" ? item : item?.label || item?.base || "";
    const trimmedLabel = String(label || "").trim();
    if (!trimmedLabel || seen.has(trimmedLabel)) continue;
    seen.add(trimmedLabel);
    const detail = typeof item === "object" && item ? item.detail || item.documentation || "" : "";
    fields.push({
      label: trimmedLabel,
      detail: String(detail || "").trim(),
      kind: "property"
    });
  }
  return fields;
}

function catalogPublicTypeLabels(value) {
  return uniqueStrings(arrayOrEmpty(value).flatMap((item) => {
    if (typeof item === "string") return [item];
    if (!item || typeof item !== "object") return [];
    return [item.label, item.base];
  }));
}

function uniqueStrings(items) {
  return [...new Set(items.map((item) => String(item || "").trim()).filter(Boolean))];
}

function lexicalUnitPattern(units) {
  const escaped = uniqueStrings(units)
    .sort((left, right) => right.length - left.length || left.localeCompare(right))
    .map(escapeRegExp);
  if (!escaped.length) return null;
  return new RegExp(`^(?:${escaped.join("|")})(?![A-Za-z0-9_/^])`);
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

async function call(cmd, args = {}) {
  if (!invoke) throw new Error("Tauri invoke API is not available");
  return await invoke(cmd, args);
}

function applyCheck(check, source = state.source) {
  state.check = normalizeCheck(check);
  state.highlightSource = String(source ?? "");
}

function markCheckPending() {
  if (state.highlightSource === null && state.check.status === "checking") return false;
  state.check = normalizeCheck({ status: "checking" });
  state.highlightSource = null;
  return true;
}

function markCheckFailed() {
  state.check = normalizeCheck({ status: "check failed" });
  state.highlightSource = null;
}

function invalidateLiveCheck() {
  if (liveCheckTimer !== null) {
    clearTimeout(liveCheckTimer);
    liveCheckTimer = null;
  }
  liveCheckRevision += 1;
  return liveCheckRevision;
}

function beginCheckRequest() {
  return {
    revision: invalidateLiveCheck(),
    path: state.currentPath,
    source: state.source,
    documents: dirtyWorkspaceDocuments(state.currentPath)
  };
}

function checkRequestIsCurrent(request) {
  return request.revision === liveCheckRevision
    && bufferRequestIsCurrent(request)
    && workspaceDocumentsAreCurrent(request.documents, request.path);
}

function bufferRequestIsCurrent(request) {
  return request.path === state.currentPath && request.source === state.source;
}

function beginNavigation() {
  navigationRevision += 1;
  invalidateLiveCheck();
  return navigationRevision;
}

function navigationIsCurrent(revision) {
  return revision === navigationRevision;
}

function scheduleLiveCheck() {
  if (!state.currentPath) return;
  const request = beginCheckRequest();
  liveCheckTimer = setTimeout(() => {
    liveCheckTimer = null;
    void runLiveCheck(request);
  }, LIVE_CHECK_DELAY_MS);
}

async function runLiveCheck(request) {
  try {
    const check = await call("ide_check", {
      path: request.path,
      source: request.source,
      documents: request.documents
    });
    if (!checkRequestIsCurrent(request)) return;
    applyCheck(check, request.source);
    state.status = `Analyzed: ${state.check.status}`;
    refreshLiveCheckUi();
  } catch (error) {
    if (!checkRequestIsCurrent(request)) return;
    markCheckFailed();
    state.status = `Live check failed: ${compactText(String(error), 90)}`;
    refreshLiveCheckUi();
  }
}

function normalizeCheck(check) {
  const semanticTokens = check?.semanticTokens ?? check?.semantic_tokens ?? { legend: {}, tokens: [] };
  return {
    diagnostics: Array.isArray(check?.diagnostics) ? check.diagnostics : [],
    symbols: Array.isArray(check?.symbols) ? check.symbols : [],
    status: check?.status || "",
    hovers: Array.isArray(check?.hovers) ? check.hovers : [],
    documentSymbols: Array.isArray(check?.documentSymbols)
      ? check.documentSymbols
      : (Array.isArray(check?.document_symbols) ? check.document_symbols : []),
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
    state.syntaxCatalog = normalizeSyntaxCatalog(data.syntaxCatalog ?? data.syntax_catalog);
    state.lexicalCatalog = buildLexicalCatalog(state.syntaxCatalog);
    state.modules = data.modules ?? [];
    state.runHistory = loadRunHistory(data.root);
    state.currentPath = data.current.path;
    state.runDir = data.currentDir || directoryOf(data.current.path);
    state.source = data.current.source;
    state.savedSource = data.current.source;
    state.tabs = [{
      path: state.currentPath,
      source: state.source,
      savedSource: state.savedSource,
      dirty: false
    }];
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
  rememberCurrentEditorView();
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
        <nav id="editorBreadcrumbs" class="editor-breadcrumbs" aria-label="Current file and symbol path">${renderEditorBreadcrumbs()}</nav>
        <span id="cursorInsight" class="cursor-insight">${renderCursorInsight()}</span>
        <span id="editorLineCount">${lineCount(state.source)} lines</span>
      </div>
      <div class="editor-shell">
        ${renderEditorFindBar()}
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
      <div id="bottomBody" class="bottom-body">${state.bottomTab === "problems" ? renderProblems() : renderTerminal()}</div>
    </section>
    <footer class="statusbar">
      <span id="checkStatus">${escapeHtml(state.check.status || "ready")}</span>
      <span>${escapeHtml(state.currentPath || "-")}</span>
      <span>Run Dir: ${escapeHtml(state.runDir || ".")}</span>
    </footer>
  `;
  bind();
  bindGlobalEvents();
  restoreCurrentEditorView();
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
      ${toolButton("formatBtn", "Format", "Format current buffer", "format")}
      ${toolButton("findBtn", "Find", "Find in current file", "search")}
      ${toolButton("saveBtn", "Save", "Save current file", "save")}
      ${toolButton("saveAllBtn", "Save All", "Save all modified files", "save")}
      <span class="toolbar-separator"></span>
      ${toolButton("reportBtn", "Report", "Open last report", "file")}
      ${toolButton("outputBtn", "Output", "Open output folder", "folder")}
      ${toolButton("plotBtn", "Plot", "Show plot panel", "chart")}
      <select id="profileSelect" class="profile-select" title="Execution profile">
        ${["normal", "safe", "repro"].map((profile) => `<option value="${profile}" ${state.profile === profile ? "selected" : ""}>${profile}</option>`).join("")}
      </select>
      <span id="errorBadge" class="badge ${errorCount() ? "bad" : ""}">${escapeHtml(diagnosticCountLabel("Errors", errorCount()))}</span>
      <span id="warningBadge" class="badge ${warningCount() ? "warn" : ""}">${escapeHtml(diagnosticCountLabel("Warnings", warningCount()))}</span>
      <span id="ideStatus" class="status">${escapeHtml(state.status)}</span>
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
      ${renderOutlinePanel()}
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

function renderOutlinePanel() {
  const allItems = flattenDocumentSymbols(state.check.documentSymbols);
  const items = filteredOutlineItems(allItems, state.outlineQuery);
  const count = state.outlineQuery.trim() ? `${items.length}/${allItems.length}` : String(allItems.length);
  return `
    <section id="outlinePanel" class="outline-panel">
      <div class="tree-head outline-head">
        <span>Outline <small>${escapeHtml(count)}</small></span>
        <button id="toggleOutlineBtn" class="outline-toggle" title="${state.outlineOpen ? "Collapse" : "Expand"} current file outline" aria-label="${state.outlineOpen ? "Collapse" : "Expand"} current file outline" aria-expanded="${state.outlineOpen}">${state.outlineOpen ? "v" : ">"}</button>
      </div>
      ${state.outlineOpen ? `
        <div class="outline-body">
          <input id="outlineQueryInput" class="outline-query" value="${escapeAttr(state.outlineQuery)}" placeholder="Filter symbols" aria-label="Filter current file symbols" autocomplete="off" spellcheck="false" />
          <div id="outlineList" class="outline-list">${renderOutlineItems(items)}</div>
        </div>
      ` : ""}
    </section>
  `;
}

function flattenDocumentSymbols(symbols, depth = 0, items = []) {
  for (const symbol of arrayOrEmpty(symbols)) {
    if (!symbol || typeof symbol !== "object") continue;
    const selection = symbol.selectionRange ?? symbol.selection_range ?? symbol.range ?? {};
    const start = selection.start ?? {};
    const end = selection.end ?? start;
    const name = String(symbol.name || "").trim();
    if (name) {
      const kind = documentSymbolCoordinate(symbol.kind, 0);
      items.push({
        name,
        detail: String(symbol.detail || "").trim(),
        kind,
        depth,
        line: documentSymbolCoordinate(start.line, 0),
        character: documentSymbolCoordinate(start.character, 0),
        endLine: documentSymbolCoordinate(end.line, documentSymbolCoordinate(start.line, 0)),
        endCharacter: documentSymbolCoordinate(end.character, documentSymbolCoordinate(start.character, 0) + name.length)
      });
    }
    flattenDocumentSymbols(symbol.children, depth + 1, items);
  }
  return items;
}

function documentSymbolCoordinate(value, fallback = 0) {
  const numeric = Number(value);
  return Number.isFinite(numeric) ? Math.max(0, Math.trunc(numeric)) : fallback;
}

function documentSymbolPosition(value) {
  const line = Number(value?.line);
  const character = Number(value?.character ?? value?.column);
  if (!Number.isInteger(line) || !Number.isInteger(character) || line < 0 || character < 0) return null;
  return { line, character };
}

function compareDocumentPositions(left, right) {
  return left.line - right.line || left.character - right.character;
}

function documentSymbolRange(symbol, selectionOnly = false) {
  const selection = symbol?.selectionRange ?? symbol?.selection_range;
  const raw = selectionOnly ? (selection ?? symbol?.range) : (symbol?.range ?? selection);
  const start = documentSymbolPosition(raw?.start);
  const end = documentSymbolPosition(raw?.end);
  if (!start || !end || compareDocumentPositions(start, end) > 0) return null;
  return { start, end };
}

function documentSymbolRangeContains(range, position) {
  return compareDocumentPositions(position, range.start) >= 0
    && compareDocumentPositions(position, range.end) <= 0;
}

function documentSymbolOwnsScope(symbol) {
  const kind = documentSymbolCoordinate(symbol?.kind, 0);
  return arrayOrEmpty(symbol?.children).length > 0
    || [2, 3, 4, 5, 6, 9, 10, 11, 12, 19, 23].includes(kind);
}

function documentSymbolBreadcrumbPath(symbols, position) {
  const caret = documentSymbolPosition(position);
  if (!caret) return [];
  let best = [];
  for (const symbol of arrayOrEmpty(symbols)) {
    if (!symbol || typeof symbol !== "object") continue;
    const name = String(symbol.name || "").trim();
    const fullRange = documentSymbolRange(symbol);
    const selectionRange = documentSymbolRange(symbol, true) ?? fullRange;
    const scopeRange = documentSymbolOwnsScope(symbol) ? fullRange : selectionRange;
    if (!name || !scopeRange || !selectionRange || !documentSymbolRangeContains(scopeRange, caret)) continue;
    const item = {
      name,
      detail: String(symbol.detail || "").trim(),
      kind: documentSymbolCoordinate(symbol.kind, 0),
      line: selectionRange.start.line,
      character: selectionRange.start.character,
      endLine: selectionRange.end.line,
      endCharacter: selectionRange.end.character,
      scopeRange
    };
    const candidate = [item, ...documentSymbolBreadcrumbPath(symbol.children, caret)];
    if (
      candidate.length > best.length
      || (candidate.length === best.length && documentSymbolPathIsNarrower(candidate, best))
    ) {
      best = candidate;
    }
  }
  return best;
}

function documentSymbolPathIsNarrower(candidate, current) {
  if (!current.length) return true;
  const next = candidate[0].scopeRange;
  const previous = current[0].scopeRange;
  return compareDocumentPositions(next.start, previous.start) >= 0
    && compareDocumentPositions(next.end, previous.end) <= 0;
}

function currentEditorDocumentPosition() {
  const editor = byId("editor");
  if (!editor || String(editor.value ?? "") !== String(state.source ?? "")) {
    return { line: 0, character: 0 };
  }
  const position = editorCursorPosition(editor.value, editor.selectionStart ?? 0);
  return { line: position.line, character: position.column };
}

function renderEditorBreadcrumbs(position = currentEditorDocumentPosition()) {
  const file = fileName(state.currentPath) || state.currentPath || "Untitled";
  const current = state.source === state.highlightSource;
  const symbols = current ? documentSymbolBreadcrumbPath(state.check.documentSymbols, position) : [];
  const fileCurrent = symbols.length ? "" : ' aria-current="location"';
  const fileButton = `<button class="editor-breadcrumb-file" data-editor-breadcrumb-line="0" data-editor-breadcrumb-character="0" data-editor-breadcrumb-end-line="0" data-editor-breadcrumb-end-character="0" data-editor-breadcrumb-name="${escapeAttr(file)}" title="Go to the start of ${escapeAttr(state.currentPath || file)}"${fileCurrent}>${escapeHtml(file)}</button>`;
  return [fileButton, ...symbols.map((symbol, index) => renderEditorSymbolBreadcrumb(symbol, index === symbols.length - 1))]
    .join('<span class="editor-breadcrumb-separator" aria-hidden="true">&gt;</span>');
}

function renderEditorSymbolBreadcrumb(symbol, current) {
  const kind = outlineKindMeta(symbol.kind);
  const title = `${kind.label}: ${symbol.name}${symbol.detail ? ` - ${symbol.detail}` : ""} - line ${symbol.line + 1}`;
  return `<button class="editor-breadcrumb-symbol" data-editor-breadcrumb-line="${symbol.line}" data-editor-breadcrumb-character="${symbol.character}" data-editor-breadcrumb-end-line="${symbol.endLine}" data-editor-breadcrumb-end-character="${symbol.endCharacter}" data-editor-breadcrumb-name="${escapeAttr(symbol.name)}" title="${escapeAttr(title)}"${current ? ' aria-current="location"' : ""}>${escapeHtml(symbol.name)}</button>`;
}

function bindEditorBreadcrumbs(root = byId("editorBreadcrumbs")) {
  if (!root) return;
  root.querySelectorAll("[data-editor-breadcrumb-line]").forEach((button) => {
    button.onclick = () => navigateEditorBreadcrumb(button);
  });
}

function navigateEditorBreadcrumb(button) {
  const editor = byId("editor");
  if (!editor || !button?.dataset) return false;
  const selected = selectEditorUtf16Range(editor, {
    line: Number(button.dataset.editorBreadcrumbLine),
    character: Number(button.dataset.editorBreadcrumbCharacter),
    endLine: Number(button.dataset.editorBreadcrumbEndLine),
    endCharacter: Number(button.dataset.editorBreadcrumbEndCharacter)
  });
  if (!selected) return false;
  syncEditorHighlightScroll();
  updateEditorFindStatus();
  updateCursorInsight();
  setStatus(`Breadcrumb: ${button.dataset.editorBreadcrumbName || fileName(state.currentPath)}`);
  return true;
}

function filteredOutlineItems(items, query = state.outlineQuery) {
  const needle = String(query || "").trim().toLowerCase();
  if (!needle) return items;
  return items.filter((item) => [item.name, item.detail, outlineKindMeta(item.kind).label]
    .some((value) => String(value || "").toLowerCase().includes(needle)));
}

function renderOutlineItems(items) {
  if (!items.length) {
    const message = state.check.status === "checking"
      ? "Analyzing current buffer..."
      : (state.outlineQuery.trim() ? "No matching symbols" : "No symbols");
    return `<div class="outline-empty">${escapeHtml(message)}</div>`;
  }
  const flattenDepth = Boolean(state.outlineQuery.trim());
  return items.map((item) => {
    const kind = outlineKindMeta(item.kind);
    const depth = flattenDepth ? 0 : Math.min(item.depth, 8);
    const title = `${kind.label}: ${item.name}${item.detail ? ` - ${item.detail}` : ""} - line ${item.line + 1}`;
    return `
      <button class="outline-item" style="--outline-depth: ${depth}" data-outline-line="${item.line}" data-outline-character="${item.character}" data-outline-end-line="${item.endLine}" data-outline-end-character="${item.endCharacter}" data-outline-name="${escapeAttr(item.name)}" title="${escapeAttr(title)}">
        <span class="outline-kind ${kind.className}" title="${escapeAttr(kind.label)}">${escapeHtml(kind.short)}</span>
        <span class="outline-name">${escapeHtml(item.name)}</span>
        <small class="outline-detail">${escapeHtml(item.detail || kind.label)}</small>
      </button>
    `;
  }).join("");
}

function outlineKindMeta(kind) {
  if ([5, 10, 11, 23, 26].includes(kind)) return { label: "Type", short: "T", className: "type" };
  if ([6, 9, 12].includes(kind)) return { label: "Function", short: "F", className: "function" };
  if ([7, 8, 20, 22].includes(kind)) return { label: "Property", short: "P", className: "property" };
  if (kind === 14) return { label: "Constant", short: "C", className: "constant" };
  if (kind === 13) return { label: "Variable", short: "V", className: "variable" };
  if ([2, 3, 4].includes(kind)) return { label: "Module", short: "M", className: "module" };
  return { label: "Symbol", short: "S", className: "symbol" };
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
    format: '<path d="M5 6h14"/><path d="M5 12h10"/><path d="M5 18h14"/>',
    search: '<circle cx="11" cy="11" r="6"/><path d="m16 16 4 4"/>',
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
    updateEditorFindStatus();
    if (["ArrowDown", "ArrowUp", "Enter", "Tab", "Escape"].includes(event.key)) return;
    updateCompletionOverlay();
  });
  editor.addEventListener("click", (event) => {
    updateCursorInsight();
    updateEditorFindStatus();
    if ((event.ctrlKey || event.metaKey) && !event.altKey && event.button === 0) {
      hideCompletions();
      void goToDefinitionAtCaret();
      return;
    }
    updateCompletionOverlay();
  });
  editor.addEventListener("mouseup", () => {
    updateCursorInsight();
    updateEditorFindStatus();
  });
  editor.addEventListener("select", () => {
    updateCursorInsight();
    updateEditorFindStatus();
  });
  editor.addEventListener("input", (event) => {
    state.source = event.target.value;
    state.dirty = state.source !== state.savedSource;
    clearReferenceResults();
    rememberCurrentTab();
    state.status = "Modified";
    const checkChanged = markCheckPending();
    renderTabLabels();
    updateEditorLineCount();
    updateCheckSummaryUi();
    updateEditorHighlight();
    updateCursorInsight();
    updateEditorFindStatus();
    if (checkChanged) refreshCheckPanels();
    updateCompletionOverlay();
    scheduleLiveCheck();
  });
  bindEditorBreadcrumbs();
  byId("checkBtn").onclick = checkCurrent;
  byId("formatBtn").onclick = formatCurrent;
  byId("findBtn").onclick = openEditorFind;
  byId("saveBtn").onclick = saveCurrent;
  byId("saveAllBtn").onclick = () => void saveAllDirtyTabs();
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
  bindEditorFindControls();
  bindOutlineControls(document);
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
  bindProblemActions(document);
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
  bindHighlightPanelActions(document);
  document.querySelectorAll("[data-side-tab]").forEach((tab) => {
    tab.onclick = () => {
      state.sideTab = tab.dataset.sideTab;
      render();
    };
  });
  bindVariableActions(document);
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
  bindSourceLineButtons(document);
  bindSourceTokenRangeButtons(document);
  bindSourceTokenCopyButtons(document);
  bindHighlightTokenFilterButtons(document);
  bindWorkspaceReferenceButtons(document);
  bindRenameActions(document);
  bindInspectorTabButtons(document);
  bindShowHighlightPanelButtons(document);
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

function bindOutlineControls(root) {
  const toggle = root.querySelector("#toggleOutlineBtn");
  if (toggle) {
    toggle.onclick = () => {
      state.outlineOpen = !state.outlineOpen;
      refreshOutlinePanel();
    };
  }
  const input = root.querySelector("#outlineQueryInput");
  if (input) {
    input.oninput = (event) => {
      const cursor = event.target.selectionStart ?? event.target.value.length;
      state.outlineQuery = event.target.value;
      refreshOutlinePanel({ start: cursor, end: cursor });
    };
    input.onkeydown = (event) => {
      if (event.key === "Enter") {
        const first = byId("outlineList")?.querySelector("[data-outline-line]");
        if (first) {
          event.preventDefault();
          selectOutlineSymbol(first);
        }
      } else if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        if (state.outlineQuery) {
          state.outlineQuery = "";
          refreshOutlinePanel({ start: 0, end: 0 });
        } else {
          byId("editor")?.focus();
        }
      }
    };
  }
  root.querySelectorAll("[data-outline-line]").forEach((button) => {
    button.onclick = () => selectOutlineSymbol(button);
  });
}

function refreshOutlinePanel(focus = null) {
  const panel = byId("outlinePanel");
  if (!panel) return;
  const active = document.activeElement;
  const nextFocus = focus || (active?.id === "outlineQueryInput" ? {
    start: active.selectionStart ?? active.value.length,
    end: active.selectionEnd ?? active.value.length
  } : null);
  panel.outerHTML = renderOutlinePanel();
  bindOutlineControls(document);
  if (!nextFocus) return;
  const input = byId("outlineQueryInput");
  if (!input) return;
  input.focus();
  input.setSelectionRange(nextFocus.start, nextFocus.end);
}

function focusOutline() {
  state.editorFindOpen = false;
  byId("editorFindBar")?.classList.add("hidden");
  hideCompletions();
  if (!state.outlineOpen) {
    state.outlineOpen = true;
    refreshOutlinePanel();
  }
  const input = byId("outlineQueryInput");
  if (!input) return;
  input.focus();
  input.select();
}

function selectOutlineSymbol(button) {
  const editor = byId("editor");
  if (!editor) return false;
  const selected = selectEditorUtf16Range(editor, {
    line: Number(button.dataset.outlineLine || 0),
    character: Number(button.dataset.outlineCharacter || 0),
    endLine: Number(button.dataset.outlineEndLine || button.dataset.outlineLine || 0),
    endCharacter: Number(button.dataset.outlineEndCharacter || button.dataset.outlineCharacter || 0)
  });
  if (!selected) return false;
  hideCompletions();
  syncEditorHighlightScroll();
  updateEditorFindStatus();
  updateCursorInsight();
  setStatus(`Outline: ${button.dataset.outlineName || "symbol"}`);
  return true;
}

function selectEditorUtf16Range(editor, range) {
  const line = documentSymbolCoordinate(range?.line, 0);
  const endLine = documentSymbolCoordinate(range?.endLine, line);
  const startRange = sourceLineRange(editor.value, line);
  const endRange = sourceLineRange(editor.value, endLine);
  const character = Math.min(startRange.text.length, documentSymbolCoordinate(range?.character, 0));
  const endCharacter = Math.min(endRange.text.length, documentSymbolCoordinate(range?.endCharacter, character + 1));
  const start = startRange.start + character;
  const end = Math.max(start + 1, endRange.start + endCharacter);
  editor.focus();
  editor.selectionStart = Math.min(start, editor.value.length);
  editor.selectionEnd = Math.min(end, editor.value.length);
  editor.scrollTop = Math.max(0, (line - 3) * 20);
  return { start: editor.selectionStart, end: editor.selectionEnd };
}

function openWorkspaceSymbolSearch() {
  if (
    state.pendingWorkspaceSymbols
    || state.pendingQuickFix
    || state.pendingRename
    || state.pendingTabClose
    || state.pendingWindowClose
  ) {
    return false;
  }
  rememberCurrentTab();
  const pending = {
    busy: true,
    error: "",
    items: [],
    query: "",
    revision: 0,
    selectedIndex: 0
  };
  state.pendingWorkspaceSymbols = pending;
  const backdrop = document.createElement("div");
  backdrop.id = "workspaceSymbolsBackdrop";
  backdrop.className = "dialog-backdrop workspace-symbol-backdrop";
  backdrop.innerHTML = `
    <div class="workspace-symbol-dialog" role="dialog" aria-modal="true" aria-labelledby="workspaceSymbolTitle">
      <div class="workspace-symbol-heading">
        <h2 id="workspaceSymbolTitle">Go to Symbol</h2>
        <button id="workspaceSymbolCloseBtn" class="workspace-symbol-close" title="Close" aria-label="Close workspace symbol search">&#215;</button>
      </div>
      <input id="workspaceSymbolInput" class="workspace-symbol-input" placeholder="Search workspace symbols" aria-label="Search workspace symbols" aria-controls="workspaceSymbolResults" aria-autocomplete="list" aria-expanded="true" role="combobox" autocomplete="off" spellcheck="false" />
      <div id="workspaceSymbolStatus" class="workspace-symbol-status" role="status" aria-live="polite"></div>
      <div id="workspaceSymbolResults" class="workspace-symbol-results" role="listbox" aria-label="Workspace symbols"></div>
    </div>
  `;
  document.body.appendChild(backdrop);
  syncDialogInert();
  const input = backdrop.querySelector("#workspaceSymbolInput");
  const closeButton = backdrop.querySelector("#workspaceSymbolCloseBtn");
  closeButton.onclick = cancelWorkspaceSymbolSearch;
  backdrop.onclick = (event) => {
    if (event.target === backdrop) cancelWorkspaceSymbolSearch();
  };
  input.oninput = () => {
    if (state.pendingWorkspaceSymbols !== pending) return;
    pending.query = input.value;
    pending.items = [];
    pending.selectedIndex = 0;
    pending.error = "";
    pending.busy = true;
    pending.revision = ++workspaceSymbolRequestRevision;
    refreshWorkspaceSymbolDialog();
    scheduleWorkspaceSymbolRequest(pending);
  };
  input.onkeydown = (event) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      event.stopPropagation();
      moveWorkspaceSymbolSelection(1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      event.stopPropagation();
      moveWorkspaceSymbolSelection(-1);
    } else if (event.key === "Enter") {
      event.preventDefault();
      event.stopPropagation();
      void openWorkspaceSymbolItem(pending.selectedIndex);
    } else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      cancelWorkspaceSymbolSearch();
    }
  };
  refreshWorkspaceSymbolDialog();
  input.focus();
  setStatus("Finding workspace symbols...");
  void requestWorkspaceSymbols(pending);
  return true;
}

function scheduleWorkspaceSymbolRequest(pending) {
  if (workspaceSymbolTimer) clearTimeout(workspaceSymbolTimer);
  workspaceSymbolTimer = setTimeout(() => {
    workspaceSymbolTimer = null;
    void requestWorkspaceSymbols(pending);
  }, WORKSPACE_SYMBOL_DELAY_MS);
}

async function requestWorkspaceSymbols(pending = state.pendingWorkspaceSymbols) {
  if (!pending || state.pendingWorkspaceSymbols !== pending) return false;
  if (workspaceSymbolTimer) {
    clearTimeout(workspaceSymbolTimer);
    workspaceSymbolTimer = null;
  }
  const revision = ++workspaceSymbolRequestRevision;
  pending.revision = revision;
  pending.busy = true;
  pending.error = "";
  refreshWorkspaceSymbolDialog();
  const query = pending.query;
  const documents = dirtyWorkspaceSymbolDocuments();
  try {
    const payload = await call("ide_workspace_symbols", { query, documents });
    if (state.pendingWorkspaceSymbols !== pending || pending.revision !== revision) return false;
    const items = workspaceSymbolItemsFromPayload(payload, query);
    pending.items = items;
    pending.selectedIndex = items.length ? 0 : -1;
    pending.busy = false;
    refreshWorkspaceSymbolDialog();
    setStatus(items.length
      ? `Workspace symbols: ${items.length}`
      : `No workspace symbols${query.trim() ? ` matching ${query.trim()}` : ""}`);
    return items.length > 0;
  } catch (error) {
    if (state.pendingWorkspaceSymbols !== pending || pending.revision !== revision) return false;
    const message = String(error);
    pending.items = [];
    pending.selectedIndex = -1;
    pending.busy = false;
    pending.error = message;
    refreshWorkspaceSymbolDialog();
    setStatus(message);
    appendTerminal("error", message);
    return false;
  }
}

function dirtyWorkspaceSymbolDocuments() {
  return dirtyWorkspaceDocuments();
}

function dirtyWorkspaceDocuments(originPath = "") {
  rememberCurrentTab();
  const documents = [];
  const seen = new Set();
  for (const tab of state.tabs) {
    if (!tab.dirty || !/\.eng$/i.test(String(tab.path || ""))) continue;
    if (originPath && sameDefinitionPath(tab.path, originPath)) continue;
    const path = workspaceSymbolRelativePath(tab.path);
    if (!path) continue;
    const key = definitionPathKey(path);
    if (seen.has(key)) continue;
    seen.add(key);
    documents.push({ path, source: tab.source });
  }
  return documents;
}

function workspaceDocumentsAreCurrent(documents, originPath = "") {
  const expected = Array.isArray(documents) ? documents : [];
  const current = dirtyWorkspaceDocuments(originPath);
  if (current.length !== expected.length) return false;
  const currentByPath = new Map(current.map((document) => [definitionPathKey(document.path), document]));
  return expected.every((document) => {
    const candidate = currentByPath.get(definitionPathKey(document.path));
    return candidate?.source === document.source;
  });
}

function workspaceSymbolItemsFromPayload(payload, query = "") {
  if (payload?.format !== "eng-lsp-snapshot-v1" || !Array.isArray(payload?.symbols)) {
    throw new Error("Workspace symbol lookup returned an invalid compiler snapshot.");
  }
  if (payload.symbols.length > 200) {
    throw new Error("Workspace symbol lookup exceeded the 200-symbol safety limit.");
  }
  const seen = new Set();
  const items = payload.symbols.map((symbol) => {
    const name = String(symbol?.name || "").trim();
    const kind = Number(symbol?.kind);
    const detail = typeof symbol?.containerName === "string" ? symbol.containerName.trim() : null;
    const uri = String(symbol?.location?.uri || "");
    const range = workspaceReferenceRange(symbol?.location);
    const absolutePath = definitionPathFromUri(uri);
    const path = workspaceSymbolRelativePath(absolutePath);
    if (
      !name
      || !Number.isInteger(kind)
      || kind < 1
      || kind > 26
      || detail === null
      || !range
      || !absolutePath
      || !path
      || !/\.eng$/i.test(path)
    ) {
      throw new Error("Workspace symbol lookup returned an incomplete symbol location.");
    }
    const key = `${name}\n${definitionPathKey(absolutePath)}\n${range.start.line}\n${range.start.character}`;
    if (seen.has(key)) {
      throw new Error("Workspace symbol lookup returned a duplicate symbol.");
    }
    seen.add(key);
    return { absolutePath, detail, kind, name, path, range, uri };
  });
  return items.sort((left, right) =>
    workspaceSymbolMatchRank(left, query) - workspaceSymbolMatchRank(right, query)
    || left.name.localeCompare(right.name)
    || left.path.localeCompare(right.path)
    || left.range.start.line - right.range.start.line
  );
}

function workspaceSymbolMatchRank(item, query) {
  const needle = String(query || "").trim().toLowerCase();
  if (!needle) return 0;
  const name = item.name.toLowerCase();
  if (name === needle) return 0;
  if (name.startsWith(needle)) return 1;
  if (name.includes(needle)) return 2;
  return item.detail.toLowerCase().includes(needle) ? 3 : 4;
}

function workspaceSymbolRelativePath(path) {
  const normalized = normalizePath(path).replace(/^\.\//, "");
  if (!normalized) return "";
  const absolute = /^[A-Za-z]:\//.test(normalized) || normalized.startsWith("/");
  if (!absolute) {
    if (normalized === ".." || normalized.startsWith("../")) return "";
    return normalized;
  }
  const root = normalizePath(state.root);
  const pathKey = definitionPathKey(normalized);
  const rootKey = definitionPathKey(root);
  if (!rootKey || !pathKey.startsWith(`${rootKey}/`)) return "";
  return normalized.slice(root.length + 1);
}

function refreshWorkspaceSymbolDialog() {
  const pending = state.pendingWorkspaceSymbols;
  const status = byId("workspaceSymbolStatus");
  const results = byId("workspaceSymbolResults");
  if (!pending || !status || !results) return;
  status.textContent = pending.busy
    ? "Searching..."
    : pending.error
      ? "Search unavailable"
      : `${pending.items.length} symbol${pending.items.length === 1 ? "" : "s"}`;
  results.innerHTML = renderWorkspaceSymbolResults(pending);
  results.querySelectorAll("[data-workspace-symbol-index]").forEach((button) => {
    const index = Number(button.dataset.workspaceSymbolIndex);
    button.onclick = () => void openWorkspaceSymbolItem(index);
    button.onmousemove = () => {
      if (pending.selectedIndex !== index) setWorkspaceSymbolSelection(index, false);
    };
  });
  results.querySelector(".workspace-symbol-option.selected")?.scrollIntoView({ block: "nearest" });
}

function renderWorkspaceSymbolResults(pending) {
  if (pending.error) {
    return `<div class="workspace-symbol-empty error">${escapeHtml(pending.error)}</div>`;
  }
  if (!pending.items.length) {
    return `<div class="workspace-symbol-empty">${pending.busy ? "Searching workspace..." : "No matching symbols"}</div>`;
  }
  return pending.items.map((item, index) => {
    const kind = outlineKindMeta(item.kind);
    const selected = index === pending.selectedIndex;
    const title = `${kind.label}: ${item.name}${item.detail ? ` - ${item.detail}` : ""} - ${item.path}:${item.range.start.line + 1}`;
    return `
      <button class="workspace-symbol-option ${selected ? "selected" : ""}" data-workspace-symbol-index="${index}" role="option" aria-selected="${selected}" title="${escapeAttr(title)}">
        <span class="outline-kind ${kind.className}" title="${escapeAttr(kind.label)}">${escapeHtml(kind.short)}</span>
        <span class="workspace-symbol-copy">
          <strong>${escapeHtml(item.name)}</strong>
          <small>${escapeHtml(item.detail || kind.label)}</small>
        </span>
        <span class="workspace-symbol-location">${escapeHtml(item.path)}:${item.range.start.line + 1}</span>
      </button>
    `;
  }).join("");
}

function moveWorkspaceSymbolSelection(direction) {
  const pending = state.pendingWorkspaceSymbols;
  if (!pending?.items.length) return false;
  const index = pending.selectedIndex < 0 ? 0 : pending.selectedIndex;
  const next = (index + direction + pending.items.length) % pending.items.length;
  return setWorkspaceSymbolSelection(next);
}

function setWorkspaceSymbolSelection(index, scroll = true) {
  const pending = state.pendingWorkspaceSymbols;
  if (!pending?.items.length) return false;
  pending.selectedIndex = Math.max(0, Math.min(pending.items.length - 1, Number(index) || 0));
  const results = byId("workspaceSymbolResults");
  results?.querySelectorAll("[data-workspace-symbol-index]").forEach((button) => {
    const selected = Number(button.dataset.workspaceSymbolIndex) === pending.selectedIndex;
    button.classList.toggle("selected", selected);
    button.setAttribute("aria-selected", String(selected));
    if (selected && scroll) button.scrollIntoView({ block: "nearest" });
  });
  return true;
}

async function openWorkspaceSymbolItem(index) {
  const pending = state.pendingWorkspaceSymbols;
  const item = pending?.items?.[index];
  if (!pending || !item) return false;
  closeWorkspaceSymbolSearch();
  try {
    if (!await openDefinitionTarget(item.absolutePath)) {
      throw new Error(`Could not open workspace symbol file: ${item.absolutePath}`);
    }
    const editor = byId("editor");
    if (!editor) return false;
    selectEditorUtf16Range(editor, {
      line: item.range.start.line,
      character: item.range.start.character,
      endLine: item.range.end.line,
      endCharacter: item.range.end.character
    });
    syncEditorHighlightScroll();
    updateEditorFindStatus();
    updateCursorInsight();
    setStatus(`Symbol: ${item.name} - ${item.path}:${item.range.start.line + 1}`);
    return true;
  } catch (error) {
    const message = String(error);
    setStatus(message);
    appendTerminal("error", message);
    return false;
  }
}

function closeWorkspaceSymbolSearch() {
  if (workspaceSymbolTimer) {
    clearTimeout(workspaceSymbolTimer);
    workspaceSymbolTimer = null;
  }
  workspaceSymbolRequestRevision += 1;
  byId("workspaceSymbolsBackdrop")?.remove();
  state.pendingWorkspaceSymbols = null;
  syncDialogInert();
}

function cancelWorkspaceSymbolSearch() {
  if (!state.pendingWorkspaceSymbols) return;
  closeWorkspaceSymbolSearch();
  setStatus("Workspace symbol search cancelled");
  byId("editor")?.focus();
}

function bindProblemActions(root) {
  root.querySelectorAll("[data-problem-severity]").forEach((button) => {
    button.onclick = () => {
      state.problemSeverity = button.dataset.problemSeverity;
      render();
    };
  });
  const problemCodeFilter = root.querySelector("#problemCodeFilter");
  if (problemCodeFilter) {
    problemCodeFilter.onchange = (event) => {
      state.problemCode = event.target.value;
      render();
    };
  }
  const clearProblemFilters = root.querySelector("#clearProblemFilters");
  if (clearProblemFilters) {
    clearProblemFilters.onclick = () => {
      state.problemSeverity = "all";
      state.problemCode = "all";
      state.problemQuery = "";
      render();
    };
  }
  const previousProblemBtn = root.querySelector("#previousProblemBtn");
  if (previousProblemBtn) previousProblemBtn.onclick = () => navigateProblem(-1);
  const nextProblemBtn = root.querySelector("#nextProblemBtn");
  if (nextProblemBtn) nextProblemBtn.onclick = () => navigateProblem(1);
  const problemQueryInput = root.querySelector("#problemQueryInput");
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
  const copyVisibleProblemsBtn = root.querySelector("#copyVisibleProblemsBtn");
  if (copyVisibleProblemsBtn) copyVisibleProblemsBtn.onclick = copyVisibleProblems;
  const copyCursorProblemBtn = root.querySelector("#copyCursorProblemBtn");
  if (copyCursorProblemBtn) copyCursorProblemBtn.onclick = copyCursorProblem;
  const quickFixCursorProblemBtn = root.querySelector("#quickFixCursorProblemBtn");
  if (quickFixCursorProblemBtn) {
    quickFixCursorProblemBtn.onclick = () => void requestCursorProblemQuickFix();
  }
  root.querySelectorAll("[data-problem-line]").forEach((row) => {
    row.onclick = (event) => {
      if (event.target.closest("button")) return;
      if (selectProblemRange(row)) {
        activateProblemRow(Number(row.dataset.problemIndex ?? -1), root);
      }
    };
  });
  root.querySelectorAll("[data-copy-problem-index]").forEach((button) => {
    button.onclick = (event) => {
      event.stopPropagation();
      copyProblemDiagnostic(Number(button.dataset.copyProblemIndex || -1));
    };
  });
  root.querySelectorAll("[data-quick-fix-problem-index]").forEach((button) => {
    button.onclick = (event) => {
      event.stopPropagation();
      void requestProblemQuickFixByIndex(Number(button.dataset.quickFixProblemIndex || -1));
    };
  });
}

function bindHighlightPanelActions(root) {
  const clearHighlightTokenFilter = root.querySelector("#clearHighlightTokenFilter");
  if (clearHighlightTokenFilter) {
    clearHighlightTokenFilter.onclick = () => {
      state.highlightTokenQuery = "";
      render();
    };
  }
  const copyVisibleHighlightsBtn = root.querySelector("#copyVisibleHighlightsBtn");
  if (copyVisibleHighlightsBtn) copyVisibleHighlightsBtn.onclick = copyVisibleHighlights;
  const copyHighlightSummaryBtn = root.querySelector("#copyHighlightSummaryBtn");
  if (copyHighlightSummaryBtn) copyHighlightSummaryBtn.onclick = copyHighlightSummary;
  const highlightTokenQueryInput = root.querySelector("#highlightTokenQueryInput");
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
}

function bindVariableActions(root) {
  root.querySelectorAll("[data-variable]").forEach((row) => {
    row.onclick = (event) => {
      if (event.target.closest("[data-source-line]")) return;
      state.selectedVariable = state.selectedVariable === row.dataset.variable ? null : row.dataset.variable;
      render();
    };
  });
}

function bindSourceLineButtons(root) {
  root.querySelectorAll("[data-source-line]").forEach((button) => {
    button.onclick = () => selectSourceLine(
      Number(button.dataset.sourceLine || 0),
      Number(button.dataset.sourceColumn || 1)
    );
  });
}

function bindShowHighlightPanelButtons(root) {
  root.querySelectorAll("[data-show-highlight-panel]").forEach((button) => {
    button.onclick = () => {
      state.sideTab = "highlight";
      render();
    };
  });
}

function updateEditorLineCount() {
  const target = byId("editorLineCount");
  if (target) target.textContent = `${lineCount(state.source)} lines`;
}

function updateCheckSummaryUi() {
  const errors = errorCount();
  const warnings = warningCount();
  const errorBadge = byId("errorBadge");
  if (errorBadge) {
    errorBadge.className = `badge ${errors ? "bad" : ""}`;
    errorBadge.textContent = diagnosticCountLabel("Errors", errors);
  }
  const warningBadge = byId("warningBadge");
  if (warningBadge) {
    warningBadge.className = `badge ${warnings ? "warn" : ""}`;
    warningBadge.textContent = diagnosticCountLabel("Warnings", warnings);
  }
  const checkStatus = byId("checkStatus");
  if (checkStatus) checkStatus.textContent = state.check.status || "ready";
  const ideStatus = byId("ideStatus");
  if (ideStatus) ideStatus.textContent = state.status;
}

function captureCheckPanelInputFocus() {
  const active = document.activeElement;
  if (!active || !["problemQueryInput", "highlightTokenQueryInput"].includes(active.id)) return null;
  return {
    id: active.id,
    start: active.selectionStart ?? active.value.length,
    end: active.selectionEnd ?? active.value.length
  };
}

function restoreCheckPanelInputFocus(focus) {
  if (!focus) return;
  const target = byId(focus.id);
  if (!target) return;
  target.focus();
  target.setSelectionRange(focus.start, focus.end);
}

function refreshCheckPanels() {
  const focus = captureCheckPanelInputFocus();
  const bottomBody = byId("bottomBody");
  if (bottomBody && state.bottomTab === "problems") {
    bottomBody.innerHTML = renderProblems();
    bindProblemActions(bottomBody);
  }
  const sideBody = byId("sideBody");
  if (sideBody && ["variables", "highlight"].includes(state.sideTab)) {
    sideBody.innerHTML = renderSideBody();
    bindVariableActions(sideBody);
    bindHighlightPanelActions(sideBody);
    bindSourceLineButtons(sideBody);
    bindSourceTokenRangeButtons(sideBody);
    bindSourceTokenCopyButtons(sideBody);
    bindHighlightTokenFilterButtons(sideBody);
    bindWorkspaceReferenceButtons(sideBody);
    bindRenameActions(sideBody);
    bindInspectorTabButtons(sideBody);
    bindShowHighlightPanelButtons(sideBody);
  }
  refreshOutlinePanel();
  restoreCheckPanelInputFocus(focus);
}

function refreshLiveCheckUi() {
  updateCheckSummaryUi();
  updateEditorLineCount();
  updateEditorHighlight();
  refreshCheckPanels();
  updateCursorInsight();
}

async function openFile(path) {
  rememberCurrentTab();
  const existing = state.tabs.find((tab) => sameWorkspaceFilePath(tab.path, path));
  if (existing) {
    await switchTab(existing.path);
    return;
  }
  const navigation = beginNavigation();
  let request = null;
  try {
    const file = await call("ide_open_file", { path });
    if (!navigationIsCurrent(navigation)) return;
    state.currentPath = file.path;
    state.runDir = directoryOf(file.path);
    openParentDirs(file.path);
    state.source = file.source;
    state.savedSource = file.source;
    state.dirty = false;
    state.tabs.push({
      path: file.path,
      source: file.source,
      savedSource: file.source,
      dirty: false
    });
    state.variables = [];
    state.args = [];
    state.artifacts = [];
    state.inspectors = emptyInspectors();
    state.completionItems = [];
    state.plotSpec = null;
    state.reportTitle = "";
    state.selectedWorkflowNodeId = null;
    state.status = `Loaded ${file.path}`;
    request = beginCheckRequest();
    markCheckPending();
    render();
    const check = await call("ide_check", {
      path: request.path,
      source: request.source,
      documents: request.documents
    });
    if (!checkRequestIsCurrent(request)) return;
    applyCheck(check, request.source);
    state.status = `Loaded ${request.path}`;
    refreshLiveCheckUi();
  } catch (error) {
    if (!navigationIsCurrent(navigation)) return;
    if (request && !checkRequestIsCurrent(request)) return;
    if (request) markCheckFailed();
    state.status = String(error);
    appendTerminal("error", String(error));
    render();
  }
}

async function saveCurrent() {
  rememberCurrentTab();
  const tab = tabFor(state.currentPath);
  if (!tab) return;
  const request = { revision: liveCheckRevision, ...saveRequestForTab(tab) };
  try {
    const file = await persistSaveRequest(request);
    const currentTab = state.tabs.find((candidate) => sameWorkspaceFilePath(candidate.path, file.path));
    state.status = currentTab?.dirty
      ? `Saved previous revision of ${file.path}; current buffer remains modified`
      : `Saved ${file.path}`;
    render();
  } catch (error) {
    state.status = String(error);
    appendTerminal("error", String(error));
    if (sameWorkspaceFilePath(state.currentPath, request.path) && state.source !== state.highlightSource) {
      scheduleLiveCheck();
    }
    render();
  }
}

function tabSavedSource(tab) {
  return typeof tab?.savedSource === "string" ? tab.savedSource : String(tab?.source ?? "");
}

function saveRequestForTab(tab) {
  return {
    path: tab.path,
    source: tab.source,
    expectedSource: tabSavedSource(tab)
  };
}

function sameWorkspaceFilePath(left, right) {
  return sameDefinitionPath(definitionWorkspacePath(left), definitionWorkspacePath(right));
}

function validateSavedFile(request, file) {
  if (
    !file
    || typeof file.source !== "string"
    || !sameWorkspaceFilePath(file.path, request.path)
    || file.source !== request.source
  ) {
    throw new Error(`Save returned an invalid result for ${fileName(request.path)}`);
  }
}

function applySavedFile(request, file) {
  validateSavedFile(request, file);
  const tab = state.tabs.find((candidate) => sameWorkspaceFilePath(candidate.path, request.path));
  if (!tab) throw new Error(`Saved tab is no longer open: ${fileName(request.path)}`);
  tab.savedSource = file.source;
  tab.dirty = tab.source !== tab.savedSource;
  if (sameWorkspaceFilePath(state.currentPath, request.path)) {
    state.savedSource = file.source;
    state.dirty = state.source !== state.savedSource;
  }
  return !tab.dirty;
}

async function persistSaveRequest(request) {
  const file = await call("ide_save_file", {
    path: request.path,
    source: request.source,
    expectedSource: request.expectedSource
  });
  applySavedFile(request, file);
  return file;
}

async function saveAllDirtyTabs() {
  rememberCurrentTab();
  const requests = dirtyTabs().map(saveRequestForTab);
  if (!requests.length) {
    setStatus("No modified files to save");
    return true;
  }
  try {
    await persistTabSaveRequests(requests);
    state.status = `Saved ${requests.length} ${requests.length === 1 ? "file" : "files"}`;
    render();
    return true;
  } catch (error) {
    const message = `Save failed: ${compactText(String(error), 90)}`;
    appendTerminal("error", message);
    state.status = message;
    render();
    return false;
  }
}

async function persistTabSaveRequests(requests) {
  setStatus(`Saving ${requests.length} ${requests.length === 1 ? "file" : "files"}`);
  const files = await call("ide_save_files", { files: requests });
  if (!Array.isArray(files) || files.length !== requests.length) {
    throw new Error("Save batch returned an incomplete result");
  }
  files.forEach((file, index) => validateSavedFile(requests[index], file));
  const changed = [];
  files.forEach((file, index) => {
    if (!applySavedFile(requests[index], file)) changed.push(fileName(requests[index].path));
  });
  if (changed.length) {
    throw new Error(`Buffer changed while saving: ${changed.join(", ")}`);
  }
}

function workspaceRunSaveRequests() {
  rememberCurrentTab();
  const requests = [];
  const seen = new Set();
  for (const tab of state.tabs) {
    const path = workspaceSymbolRelativePath(tab.path);
    if (!path) continue;
    const key = definitionPathKey(path);
    if (seen.has(key)) throw new Error(`Run found the same open workspace file twice: ${path}`);
    seen.add(key);
    requests.push({ ...saveRequestForTab(tab), path });
  }
  if (!requests.some((request) => sameWorkspaceFilePath(request.path, state.currentPath))) {
    throw new Error("Run requires the current file to be inside the EngLang workspace");
  }
  return requests;
}

function workspaceRunBuffersAreSaved(request, saveRequests) {
  if (!workspaceRunBufferSourcesAreCurrent(request, saveRequests)) return false;
  const current = new Map();
  for (const tab of state.tabs) {
    const path = workspaceSymbolRelativePath(tab.path);
    if (path) current.set(definitionPathKey(path), tab);
  }
  return saveRequests.every((saved) => {
    const tab = current.get(definitionPathKey(saved.path));
    return tab && tabSavedSource(tab) === saved.source && !tab.dirty;
  });
}

function workspaceRunBufferSourcesAreCurrent(request, saveRequests) {
  if (
    !request
    || request.revision !== liveCheckRevision
    || !bufferRequestIsCurrent(request)
    || !Array.isArray(saveRequests)
    || !saveRequests.length
  ) {
    return false;
  }
  const current = new Map();
  for (const tab of state.tabs) {
    const path = workspaceSymbolRelativePath(tab.path);
    if (!path) continue;
    const key = definitionPathKey(path);
    if (current.has(key)) return false;
    current.set(key, { tab, path });
  }
  if (current.size !== saveRequests.length) return false;
  return saveRequests.every((saved) => {
    const entry = current.get(definitionPathKey(saved.path));
    return entry && entry.tab.source === saved.source;
  });
}

async function formatCurrent() {
  const request = beginCheckRequest();
  try {
    rememberCurrentTab();
    const result = await call("ide_format", { path: request.path, source: request.source });
    if (!bufferRequestIsCurrent(request)) return;
    if (!result.changed) {
      state.status = "Already formatter-clean";
      if (state.source !== state.highlightSource) scheduleLiveCheck();
      render();
      return;
    }
    state.source = result.source;
    state.dirty = state.source !== state.savedSource;
    const tab = tabFor(state.currentPath);
    if (tab) {
      tab.source = state.source;
      tab.dirty = state.dirty;
    }
    state.status = "Formatted current buffer";
    markCheckPending();
    scheduleLiveCheck();
    render();
  } catch (error) {
    if (!bufferRequestIsCurrent(request)) return;
    state.status = String(error);
    appendTerminal("error", String(error));
    if (state.source !== state.highlightSource) scheduleLiveCheck();
    render();
  }
}

async function checkCurrent() {
  const request = beginCheckRequest();
  try {
    rememberCurrentTab();
    markCheckPending();
    state.status = "Checking";
    refreshLiveCheckUi();
    const check = await call("ide_check", {
      path: request.path,
      source: request.source,
      documents: request.documents
    });
    if (!checkRequestIsCurrent(request)) return;
    applyCheck(check, request.source);
    state.status = `Checked: ${state.check.status}`;
    state.bottomTab = errorCount() ? "problems" : state.bottomTab;
    render();
  } catch (error) {
    if (!checkRequestIsCurrent(request)) return;
    markCheckFailed();
    state.status = String(error);
    appendTerminal("error", String(error));
    render();
  }
}

async function runCurrent() {
  rememberCurrentTab();
  const request = beginCheckRequest();
  let saveRequests = [];
  try {
    appendTerminal("command", `${terminalPrompt()}run ${fileName(request.path)}`);
    saveRequests = workspaceRunSaveRequests();
    setStatus(`Synchronizing ${saveRequests.length} open workspace ${saveRequests.length === 1 ? "file" : "files"} before run`);
    await persistTabSaveRequests(saveRequests);
    if (!workspaceRunBuffersAreSaved(request, saveRequests)) {
      state.status = "Run cancelled; an open workspace buffer changed while saving";
      render();
      return;
    }
    const result = await call("ide_run", { path: request.path, source: request.source, profile: state.profile });
    const requestCurrent = workspaceRunBuffersAreSaved(request, saveRequests);
    applyRun(result, { mergeRuntime: false, applyCheck: requestCurrent, checkSource: request.source });
    appendRunResult(result, { ...runHistoryContext("run"), sourcePath: request.path });
    state.status = requestCurrent
      ? (result.ok ? "Run complete" : "Run blocked")
      : (result.ok ? "Run complete; buffer changed" : "Run blocked; buffer changed");
    state.bottomTab = "terminal";
    render();
  } catch (error) {
    const requestCurrent = workspaceRunBufferSourcesAreCurrent(request, saveRequests);
    const message = `Run failed: ${compactText(String(error), 90)}`;
    appendTerminal("error", message);
    state.status = requestCurrent ? message : "Run failed; buffer changed";
    if (requestCurrent && state.source !== state.highlightSource) scheduleLiveCheck();
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
  const usesCurrentFile = terminalCommandUsesCurrentFile(command);
  if (usesCurrentFile) rememberCurrentTab();
  const request = usesCurrentFile ? beginCheckRequest() : null;
  const runContext = runHistoryContext(command);
  let saveRequests = [];
  try {
    if (String(command || "").trim().toLowerCase() === "run") {
      saveRequests = workspaceRunSaveRequests();
      setStatus(`Synchronizing ${saveRequests.length} open workspace ${saveRequests.length === 1 ? "file" : "files"} before run`);
      await persistTabSaveRequests(saveRequests);
      if (!workspaceRunBuffersAreSaved(request, saveRequests)) {
        state.status = "Terminal run cancelled; an open workspace buffer changed while saving";
        state.bottomTab = "terminal";
        render();
        return;
      }
    }
    const result = await call("ide_terminal", {
      path: request?.path ?? state.currentPath,
      source: request?.source ?? state.source,
      command,
      runDir: state.runDir,
      profile: state.profile
    });
    const requestCurrent = !request || (saveRequests.length
      ? workspaceRunBuffersAreSaved(request, saveRequests)
      : checkRequestIsCurrent(request));
    applyRun(result, {
      mergeRuntime: command.toLowerCase() !== "run",
      applyCheck: requestCurrent,
      checkSource: request?.source ?? ""
    });
    appendRunResult(result, runContext);
    state.status = requestCurrent
      ? (result.ok ? "Terminal command complete" : "Terminal diagnostics")
      : (result.ok ? "Terminal command complete; buffer changed" : "Terminal diagnostics; buffer changed");
  } catch (error) {
    const message = compactText(String(error), 90);
    appendTerminal("error", message);
    const requestCurrent = !request || (saveRequests.length
      ? workspaceRunBufferSourcesAreCurrent(request, saveRequests)
      : checkRequestIsCurrent(request));
    state.status = request && !requestCurrent
      ? "Terminal command failed; buffer changed"
      : `Terminal command failed: ${message}`;
    if (request && requestCurrent && state.source !== state.highlightSource) scheduleLiveCheck();
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
  if (result.check && options.applyCheck !== false) {
    applyCheck(result.check, options.checkSource ?? state.source);
  }
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

function selectSourceLine(line, column = 1) {
  const editor = byId("editor");
  if (!editor || !Number.isFinite(line) || line <= 0) return;
  const lineRange = sourceLineRange(editor.value, line - 1);
  const columnStart = sourceColumnStart(lineRange.text, column);
  editor.focus();
  editor.selectionStart = lineRange.start + (columnStart ?? 0);
  editor.selectionEnd = lineRange.end;
  editor.scrollTop = Math.max(0, (lineRange.lineIndex - 3) * 20);
  updateCursorInsight();
}

function selectProblemRange(row) {
  return selectProblemDiagnostic({
    line: Number(row?.dataset?.problemLine || 0),
    column: Number(row?.dataset?.problemColumn || 1),
    startCharacter: Number(row?.dataset?.problemStartCharacter ?? -1),
    endCharacter: Number(row?.dataset?.problemEndCharacter ?? -1)
  });
}

function problemSourceSelection(diag, source = state.source) {
  const line = Number(sourceLineValue(diag));
  if (!Number.isInteger(line) || line < 1) return null;
  const lineIndex = line - 1;
  const startCharacter = Number(diag?.startCharacter ?? diag?.start_character);
  const endCharacter = Number(diag?.endCharacter ?? diag?.end_character);
  if (
    Number.isInteger(startCharacter)
    && Number.isInteger(endCharacter)
    && startCharacter >= 0
    && endCharacter > startCharacter
  ) {
    const start = sourceUtf16Offset(source, { line: lineIndex, character: startCharacter });
    const end = sourceUtf16Offset(source, { line: lineIndex, character: endCharacter });
    if (start !== null && end !== null && end > start) {
      return { start, end, line: lineIndex, character: startCharacter };
    }
  }
  const lineStart = sourceUtf16Offset(source, { line: lineIndex, character: 0 });
  if (lineStart === null) return null;
  const lineRange = sourceLineRange(source, lineIndex);
  const character = Math.min(
    lineRange.text.length,
    Math.max(0, sourceColumnStart(lineRange.text, sourceColumnValue(diag)) ?? 0)
  );
  return {
    start: lineStart + character,
    end: lineRange.end,
    line: lineIndex,
    character
  };
}

function selectProblemDiagnostic(diag, editor = byId("editor")) {
  if (!editor || String(editor.value ?? "") !== String(state.source ?? "")) return false;
  const selection = problemSourceSelection(diag, editor.value);
  if (!selection) return false;
  editor.focus();
  editor.selectionStart = selection.start;
  editor.selectionEnd = selection.end;
  editor.scrollTop = Math.max(0, (selection.line - 3) * 20);
  syncEditorHighlightScroll();
  updateEditorFindStatus();
  updateCursorInsight();
  return true;
}

function selectSourceCharacterRange(line, startCharacter, endCharacter) {
  const editor = byId("editor");
  if (
    !editor ||
    !Number.isFinite(line) ||
    !Number.isFinite(startCharacter) ||
    !Number.isFinite(endCharacter) ||
    line <= 0 ||
    startCharacter < 0 ||
    endCharacter <= startCharacter
  ) {
    return;
  }
  const lineRange = sourceLineRange(editor.value, line - 1);
  const lineLength = lineRange.text.length;
  const startColumn = Math.min(lineLength, Math.max(0, Math.trunc(startCharacter)));
  const endColumn = Math.min(lineLength, Math.max(startColumn + 1, Math.trunc(endCharacter)));
  editor.focus();
  editor.selectionStart = lineRange.start + startColumn;
  editor.selectionEnd = lineRange.start + endColumn;
  editor.scrollTop = Math.max(0, (lineRange.lineIndex - 3) * 20);
  updateCursorInsight();
}

function sourceColumnStart(lineText, column) {
  const columnNumber = Number(column);
  if (!Number.isFinite(columnNumber) || columnNumber <= 1) return null;
  const targetByte = Math.max(0, Math.trunc(columnNumber) - 1);
  return byteOffsetToCodeUnit(String(lineText || ""), targetByte);
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
  const startColumn = Math.min(lineRange.text.length, Math.max(0, Math.trunc(startByte)));
  const endColumn = Math.min(
    lineRange.text.length,
    Math.max(startColumn, Math.trunc(startByte + lengthBytes))
  );
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

function normalizedTabEditorView(tab) {
  const sourceLength = String(tab?.source ?? "").length;
  const startValue = Number(tab?.selectionStart);
  const endValue = Number(tab?.selectionEnd);
  const start = Math.max(0, Math.min(sourceLength, Number.isFinite(startValue) ? Math.trunc(startValue) : 0));
  const end = Math.max(start, Math.min(sourceLength, Number.isFinite(endValue) ? Math.trunc(endValue) : start));
  const direction = ["forward", "backward", "none"].includes(tab?.selectionDirection)
    ? tab.selectionDirection
    : "none";
  const scrollTop = Number(tab?.scrollTop);
  const scrollLeft = Number(tab?.scrollLeft);
  return {
    selectionStart: start,
    selectionEnd: end,
    selectionDirection: direction,
    scrollTop: Number.isFinite(scrollTop) ? Math.max(0, scrollTop) : 0,
    scrollLeft: Number.isFinite(scrollLeft) ? Math.max(0, scrollLeft) : 0
  };
}

function currentEditorViewSnapshot(editor = byId("editor")) {
  if (!editor || String(editor.value ?? "") !== String(state.source ?? "")) return null;
  return normalizedTabEditorView({
    source: editor.value,
    selectionStart: editor.selectionStart,
    selectionEnd: editor.selectionEnd,
    selectionDirection: editor.selectionDirection,
    scrollTop: editor.scrollTop,
    scrollLeft: editor.scrollLeft
  });
}

function rememberCurrentEditorView(editor = byId("editor")) {
  const tab = tabFor(state.currentPath);
  const view = currentEditorViewSnapshot(editor);
  if (!tab || !view) return false;
  Object.assign(tab, view);
  return true;
}

function restoreCurrentEditorView(editor = byId("editor")) {
  const tab = tabFor(state.currentPath);
  if (!tab || !editor || String(editor.value ?? "") !== String(state.source ?? "")) return false;
  const view = normalizedTabEditorView(tab);
  if (typeof editor.setSelectionRange === "function") {
    editor.setSelectionRange(view.selectionStart, view.selectionEnd, view.selectionDirection);
  } else {
    editor.selectionStart = view.selectionStart;
    editor.selectionEnd = view.selectionEnd;
    editor.selectionDirection = view.selectionDirection;
  }
  editor.scrollTop = view.scrollTop;
  editor.scrollLeft = view.scrollLeft;
  return true;
}

function rememberCurrentTab() {
  if (!state.currentPath) return;
  const view = currentEditorViewSnapshot();
  const tab = tabFor(state.currentPath);
  if (!tab) {
    state.tabs.push({
      path: state.currentPath,
      source: state.source,
      savedSource: state.savedSource,
      dirty: state.dirty,
      ...(view || {})
    });
    return;
  }
  if (typeof tab.savedSource !== "string") {
    tab.savedSource = typeof state.savedSource === "string" ? state.savedSource : tab.source;
  }
  tab.source = state.source;
  tab.dirty = state.dirty;
  if (view) Object.assign(tab, view);
}

async function switchTab(path) {
  if (path === state.currentPath) return;
  rememberCurrentTab();
  const tab = tabFor(path);
  if (!tab) return;
  const navigation = beginNavigation();
  state.currentPath = tab.path;
  state.runDir = directoryOf(tab.path);
  openParentDirs(tab.path);
  state.source = tab.source;
  state.savedSource = tabSavedSource(tab);
  state.dirty = tab.dirty;
  state.variables = [];
  state.args = [];
  state.artifacts = [];
  state.inspectors = emptyInspectors();
  state.completionItems = [];
  state.plotSpec = null;
  state.reportTitle = "";
  state.status = `Loaded ${tab.path}`;
  const request = beginCheckRequest();
  markCheckPending();
  render();
  try {
    const check = await call("ide_check", {
      path: request.path,
      source: request.source,
      documents: request.documents
    });
    if (!navigationIsCurrent(navigation) || !checkRequestIsCurrent(request)) return;
    applyCheck(check, request.source);
    state.status = `Loaded ${request.path}`;
    refreshLiveCheckUi();
  } catch (error) {
    if (!navigationIsCurrent(navigation) || !checkRequestIsCurrent(request)) return;
    markCheckFailed();
    state.status = String(error);
    refreshLiveCheckUi();
  }
}

function syncDialogInert() {
  const app = byId("app");
  if (app) app.inert = Boolean(
    state.pendingQuickFix
      || state.pendingRename
      || state.pendingWorkspaceSymbols
      || state.pendingTabClose
      || state.pendingWindowClose
  );
}

function openUnsavedChangesDialog(path) {
  const tab = tabFor(path);
  if (!tab?.dirty) {
    void closeTab(path, true);
    return;
  }
  closeUnsavedChangesDialog();
  state.pendingTabClose = path;
  const backdrop = document.createElement("div");
  backdrop.id = "unsavedChangesBackdrop";
  backdrop.className = "dialog-backdrop";
  backdrop.innerHTML = `
    <div class="unsaved-dialog" role="dialog" aria-modal="true" aria-labelledby="unsavedChangesTitle" aria-describedby="unsavedChangesDescription">
      <h2 id="unsavedChangesTitle">Save changes?</h2>
      <p id="unsavedChangesDescription"><strong>${escapeHtml(fileName(path))}</strong> has unsaved changes.</p>
      <div class="unsaved-dialog-path" title="${escapeAttr(path)}">${escapeHtml(path)}</div>
      <div class="unsaved-dialog-actions">
        <button id="unsavedCancelBtn">Cancel</button>
        <button id="unsavedDiscardBtn" class="danger">Discard</button>
        <button id="unsavedSaveBtn" class="primary">Save</button>
      </div>
    </div>
  `;
  document.body.appendChild(backdrop);
  syncDialogInert();
  byId("unsavedCancelBtn").onclick = cancelPendingTabClose;
  byId("unsavedDiscardBtn").onclick = () => void discardPendingTabClose();
  byId("unsavedSaveBtn").onclick = () => void savePendingTabAndClose();
  backdrop.onclick = (event) => {
    if (event.target === backdrop) cancelPendingTabClose();
  };
  byId("unsavedSaveBtn").focus();
  setStatus(`Unsaved changes in ${fileName(path)}`);
}

function closeUnsavedChangesDialog() {
  byId("unsavedChangesBackdrop")?.remove();
  state.pendingTabClose = null;
  syncDialogInert();
}

function cancelPendingTabClose() {
  const path = state.pendingTabClose;
  if (!path) return;
  closeUnsavedChangesDialog();
  setStatus(`Kept ${fileName(path)} open`);
  byId("editor")?.focus();
}

async function discardPendingTabClose() {
  const path = state.pendingTabClose;
  if (!path) return;
  closeUnsavedChangesDialog();
  await closeTab(path, true);
}

function setUnsavedChangesDialogBusy(busy) {
  for (const id of ["unsavedCancelBtn", "unsavedDiscardBtn", "unsavedSaveBtn"]) {
    const button = byId(id);
    if (button) button.disabled = busy;
  }
  const saveButton = byId("unsavedSaveBtn");
  if (saveButton) saveButton.textContent = busy ? "Saving..." : "Save";
}

async function savePendingTabAndClose() {
  const path = state.pendingTabClose;
  const tab = tabFor(path);
  if (!path || !tab) {
    closeUnsavedChangesDialog();
    return;
  }
  const request = saveRequestForTab(tab);
  setUnsavedChangesDialogBusy(true);
  setStatus(`Saving ${fileName(request.path)}`);
  try {
    const file = await persistSaveRequest(request);
    const currentTab = state.tabs.find((candidate) => sameWorkspaceFilePath(candidate.path, request.path));
    if (!currentTab || currentTab.dirty) {
      setStatus("Buffer changed while saving; close cancelled");
      setUnsavedChangesDialogBusy(false);
      return;
    }
    state.status = `Saved ${file.path}`;
    closeUnsavedChangesDialog();
    await closeTab(request.path, true);
  } catch (error) {
    setStatus(`Save failed: ${compactText(String(error), 90)}`);
    setUnsavedChangesDialogBusy(false);
  }
}

function dirtyTabs() {
  return state.tabs.filter((tab) => tab.dirty);
}

function openUnsavedWindowDialog() {
  const tabs = dirtyTabs();
  if (tabs.length === 0) {
    void destroyNativeWindow();
    return;
  }
  closeUnsavedChangesDialog();
  closeUnsavedWindowDialog();
  state.pendingWindowClose = true;
  const backdrop = document.createElement("div");
  backdrop.id = "unsavedWindowBackdrop";
  backdrop.className = "dialog-backdrop";
  const fileItems = tabs
    .map((tab) => `<li title="${escapeAttr(tab.path)}">${escapeHtml(tab.path)}</li>`)
    .join("");
  const fileLabel = tabs.length === 1 ? "file has" : "files have";
  backdrop.innerHTML = `
    <div class="unsaved-dialog" role="dialog" aria-modal="true" aria-labelledby="unsavedWindowTitle" aria-describedby="unsavedWindowDescription">
      <h2 id="unsavedWindowTitle">Save changes before closing?</h2>
      <p id="unsavedWindowDescription"><strong>${tabs.length}</strong> ${fileLabel} unsaved changes.</p>
      <ul class="unsaved-file-list">${fileItems}</ul>
      <div class="unsaved-dialog-actions">
        <button id="unsavedWindowCancelBtn">Cancel</button>
        <button id="unsavedWindowDiscardBtn" class="danger">Discard All</button>
        <button id="unsavedWindowSaveBtn" class="primary">Save All</button>
      </div>
    </div>
  `;
  document.body.appendChild(backdrop);
  syncDialogInert();
  byId("unsavedWindowCancelBtn").onclick = cancelPendingWindowClose;
  byId("unsavedWindowDiscardBtn").onclick = () => void discardAllDirtyTabsAndClose();
  byId("unsavedWindowSaveBtn").onclick = () => void saveAllDirtyTabsAndClose();
  backdrop.onclick = (event) => {
    if (event.target === backdrop) cancelPendingWindowClose();
  };
  byId("unsavedWindowSaveBtn").focus();
  setStatus(`${tabs.length} unsaved ${tabs.length === 1 ? "file" : "files"}`);
}

function closeUnsavedWindowDialog() {
  byId("unsavedWindowBackdrop")?.remove();
  state.pendingWindowClose = false;
  syncDialogInert();
}

function cancelPendingWindowClose() {
  if (!state.pendingWindowClose) return;
  closeUnsavedWindowDialog();
  setStatus("Window close cancelled");
  byId("editor")?.focus();
}

function setUnsavedWindowDialogBusy(busy, activeAction = "save") {
  for (const id of ["unsavedWindowCancelBtn", "unsavedWindowDiscardBtn", "unsavedWindowSaveBtn"]) {
    const button = byId(id);
    if (button) button.disabled = busy;
  }
  const saveButton = byId("unsavedWindowSaveBtn");
  if (saveButton) saveButton.textContent = busy && activeAction === "save" ? "Saving..." : "Save All";
  const discardButton = byId("unsavedWindowDiscardBtn");
  if (discardButton) discardButton.textContent = busy && activeAction === "discard" ? "Closing..." : "Discard All";
}

async function saveAllDirtyTabsAndClose() {
  const requests = dirtyTabs().map(saveRequestForTab);
  if (requests.length === 0) {
    closeUnsavedWindowDialog();
    await destroyNativeWindow();
    return;
  }
  setUnsavedWindowDialogBusy(true);
  try {
    await persistTabSaveRequests(requests);
  } catch (error) {
    const message = `Save failed: ${compactText(String(error), 90)}`;
    renderTabLabels();
    closeUnsavedWindowDialog();
    if (hasDirtyTabs()) openUnsavedWindowDialog();
    setStatus(message);
    return;
  }
  state.status = `Saved ${requests.length} ${requests.length === 1 ? "file" : "files"}`;
  renderTabLabels();
  try {
    await destroyNativeWindow();
  } catch (error) {
    closeUnsavedWindowDialog();
    setStatus(`Close failed after saving: ${compactText(String(error), 90)}`);
  }
}

async function discardAllDirtyTabsAndClose() {
  setUnsavedWindowDialogBusy(true, "discard");
  setStatus("Discarding unsaved changes and closing");
  try {
    await destroyNativeWindow();
  } catch (error) {
    setStatus(`Close failed: ${compactText(String(error), 90)}`);
    setUnsavedWindowDialogBusy(false);
  }
}

async function destroyNativeWindow() {
  const getCurrentWindow = window.__TAURI__?.window?.getCurrentWindow;
  const appWindow = nativeAppWindow || (typeof getCurrentWindow === "function" ? getCurrentWindow() : null);
  if (!appWindow || typeof appWindow.destroy !== "function") {
    throw new Error("Native window control is unavailable");
  }
  await appWindow.destroy();
}

async function closeTab(path, force = false) {
  if (state.tabs.length <= 1) return;
  rememberCurrentTab();
  const index = state.tabs.findIndex((tab) => tab.path === path);
  if (index < 0) return;
  const tab = state.tabs[index];
  if (tab.dirty && !force) {
    openUnsavedChangesDialog(path);
    return;
  }
  if (state.pendingTabClose === path) closeUnsavedChangesDialog();
  const wasCurrent = state.currentPath === path;
  state.tabs.splice(index, 1);
  if (!wasCurrent) {
    render();
    return;
  }
  const navigation = beginNavigation();
  const next = state.tabs[Math.max(0, index - 1)];
  state.currentPath = next.path;
  state.runDir = directoryOf(next.path);
  openParentDirs(next.path);
  state.source = next.source;
  state.savedSource = tabSavedSource(next);
  state.dirty = next.dirty;
  state.variables = [];
  state.args = [];
  state.artifacts = [];
  state.inspectors = emptyInspectors();
  state.completionItems = [];
  state.plotSpec = null;
  state.reportTitle = "";
  state.status = `Loaded ${next.path}`;
  const request = beginCheckRequest();
  markCheckPending();
  render();
  try {
    const check = await call("ide_check", {
      path: request.path,
      source: request.source,
      documents: request.documents
    });
    if (!navigationIsCurrent(navigation) || !checkRequestIsCurrent(request)) return;
    applyCheck(check, request.source);
    state.status = `Loaded ${request.path}`;
    refreshLiveCheckUi();
  } catch (error) {
    if (!navigationIsCurrent(navigation) || !checkRequestIsCurrent(request)) return;
    markCheckFailed();
    state.status = String(error);
    refreshLiveCheckUi();
  }
}

function tabFor(path) {
  return state.tabs.find((tab) => tab.path === path);
}

function renderSidePanel() {
  return `
    <aside class="variables inspector">
      <div class="side-tabs">
        ${SIDE_TABS.map((tab) => sideTabButton(tab.key, tab.label)).join("")}
      </div>
      <div id="sideBody" class="side-body">${renderSideBody()}</div>
    </aside>
  `;
}

function sideTabButton(key, label) {
  return `<button class="side-tab ${state.sideTab === key ? "active" : ""}" data-side-tab="${key}">${label}</button>`;
}

function renderSideBody() {
  if (state.sideTab === "units") return renderUnitsPanel();
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

function renderUnitsPanel() {
  const units = reviewArray(inspectorObject("reviewDocument"), "units_quantities", "unitsQuantities");
  const conversions = inspectorRows("unitConversions");
  return `
    <div class="panel-title compact">Units</div>
    <div class="badges">
      <span class="badge">Review ${units.length}</span>
      <span class="badge">Conversions ${conversions.length}</span>
    </div>
    <div class="scroll">
      <div class="panel-title compact">Review Units</div>
      ${renderReviewUnits(units)}
      <div class="panel-title compact">Unit Conversions</div>
      ${renderUnitConversions()}
    </div>
  `;
}

function renderSchemaPanel() {
  return `
    <div class="panel-title compact">Schema</div>
    <div class="badges">
      <span class="badge">Schemas ${inspectorRows("schemas").length}</span>
    </div>
    <div class="scroll">
      ${renderSchemas()}
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

function renderEditorFindBar() {
  const matches = editorFindRanges(
    state.source,
    state.editorFindQuery,
    state.editorFindCaseSensitive
  );
  const activeIndex = state.editorFindMatchIndex >= 0
    && state.editorFindMatchIndex < matches.length
    ? state.editorFindMatchIndex
    : -1;
  const count = matches.length ? `${activeIndex + 1}/${matches.length}` : "0/0";
  return `
    <div id="editorFindBar" class="editor-find ${state.editorFindOpen ? "" : "hidden"}" role="search">
      <input id="editorFindInput" value="${escapeAttr(state.editorFindQuery)}" placeholder="Find" aria-label="Find in current file" autocomplete="off" spellcheck="false" />
      <span id="editorFindCount" class="editor-find-count" aria-live="polite">${count}</span>
      <button id="editorFindPrevBtn" class="editor-find-action" title="Previous match" aria-label="Previous match">&#8593;</button>
      <button id="editorFindNextBtn" class="editor-find-action" title="Next match" aria-label="Next match">&#8595;</button>
      <button id="editorFindCaseBtn" class="editor-find-action case-toggle ${state.editorFindCaseSensitive ? "active" : ""}" title="Match case" aria-label="Match case" aria-pressed="${state.editorFindCaseSensitive}">Aa</button>
      <button id="editorFindCloseBtn" class="editor-find-action" title="Close find" aria-label="Close find">&times;</button>
    </div>
  `;
}

function checkFreshnessLabel() {
  if (state.source === state.highlightSource) return "Current";
  return state.check.status === "checking" ? "Analyzing" : "Unavailable";
}

function renderHighlightPanel() {
  const semantic = semanticTokenPayload();
  const legend = semantic.legend || {};
  const tokens = Array.isArray(semantic.tokens) ? semantic.tokens : [];
  const filteredTokens = filteredSemanticTokens(tokens);
  const typeCounts = countSemanticTokens(filteredTokens, (token) => token.type || "-");
  const modifierCounts = countSemanticTokens(filteredTokens.flatMap((token) => token.modifiers || []), (modifier) => modifier || "-");
  const selectorCounts = countSemanticTokens(filteredTokens.flatMap((token) => semanticTokenSelectors(token)), (selector) => selector || "-");
  const coverageRows = highlightCoverageRows(tokens);
  const overlaps = semanticTokenOverlaps(tokens);
  const tokenCurrent = state.source === state.highlightSource;
  const caretToken = currentCaretSemanticToken();
  return `
    <div class="panel-title compact">Highlights</div>
    <div class="badges">
      <span class="badge">Highlights ${tokens.length}</span>
      <span class="badge">Shown ${filteredTokens.length}</span>
      <span class="badge">Categories ${arrayOrEmpty(legend.token_types || legend.tokenTypes).length}</span>
      <span class="badge">Details ${arrayOrEmpty(legend.token_modifiers || legend.tokenModifiers).length}</span>
      <span class="badge ${overlaps.length ? "warn" : ""}">Overlaps ${overlaps.length}</span>
      <span class="badge ${tokenCurrent ? "" : "warn"}">${escapeHtml(checkFreshnessLabel())}</span>
    </div>
    ${renderHighlightPanelStatus(tokens, filteredTokens, tokenCurrent)}
    <div class="scroll highlight-panel">
      <div class="module-toolbar">
        <input id="highlightTokenQueryInput" class="module-query" value="${escapeAttr(state.highlightTokenQuery)}" placeholder="Filter highlights" title="Filter by text, category, detail, selector, or source line" />
        <button id="clearHighlightTokenFilter">Clear</button>
        <button id="copyVisibleHighlightsBtn" title="Copy filtered highlights" ${filteredTokens.length ? "" : "disabled"}>Copy visible</button>
        <button id="copyHighlightSummaryBtn" title="Copy highlight coverage summary">Copy summary</button>
        <span class="muted">${filteredTokens.length} of ${tokens.length}</span>
      </div>
      <div class="panel-title compact">Coverage Summary</div>
      ${renderHighlightCoverageTable(coverageRows)}
      <div class="panel-title compact">Caret Highlight</div>
      <div id="caretHighlightSummary">${renderCaretHighlightSummary(caretToken, tokenCurrent)}</div>
      <div class="panel-title compact">Semantic References</div>
      ${renderDocumentHighlightReferences()}
      <div class="panel-title compact">Categories</div>
      ${renderSemanticLegendTable(arrayOrEmpty(legend.token_types || legend.tokenTypes), typeCounts, "type")}
      <div class="panel-title compact">Details</div>
      ${renderSemanticLegendTable(arrayOrEmpty(legend.token_modifiers || legend.tokenModifiers), modifierCounts, "modifier")}
      <div class="panel-title compact">Selectors</div>
      ${renderSemanticSelectorTable(selectorCounts)}
      <div class="panel-title compact">Range Overlaps</div>
      ${renderSemanticOverlapSummary(overlaps)}
      <div class="panel-title compact">Current File Highlights</div>
      ${renderSemanticTokenRows(filteredTokens, Boolean(state.highlightTokenQuery.trim()))}
      ${advancedDataToggle("Advanced highlight data", semantic)}
    </div>
  `;
}

function renderHighlightPanelStatus(tokens, filteredTokens, tokenCurrent) {
  if (!tokenCurrent) {
    const message = state.check.status === "checking"
      ? "Analyzing current buffer..."
      : "Current buffer analysis is unavailable. Use Check to retry.";
    return `<div class="empty-state">${escapeHtml(message)}</div>`;
  }
  if (!tokens.length) {
    return `<div class="empty-state">No role-aware highlights were returned for the current check.</div>`;
  }
  if (state.highlightTokenQuery.trim() && !filteredTokens.length) {
    return `<div class="empty-state">Filter hides all current highlights. Clear the filter to inspect the checked ranges.</div>`;
  }
  return `<div class="empty-state">Highlight data is current. Showing ${escapeHtml(String(filteredTokens.length))} of ${escapeHtml(String(tokens.length))} checked ranges.</div>`;
}

function highlightCoverageRows(tokens) {
  const tokenCounts = semanticTokenTextCounts(tokens);
  return highlightCoverageCatalog().map((domain) => {
    const catalogWords = uniqueStrings(domain.words);
    const sourceWords = sourceCatalogWords(catalogWords, { allowNumericPrefix: domain.key === "unit" });
    const highlightedWords = sourceWords.filter((word) => (tokenCounts.get(normalizedCatalogWord(word)) || 0) > 0);
    const missingWords = sourceWords.filter((word) => (tokenCounts.get(normalizedCatalogWord(word)) || 0) === 0);
    const highlightCount = catalogWords.reduce((total, word) => total + (tokenCounts.get(normalizedCatalogWord(word)) || 0), 0);
    const status = missingWords.length ? "unmatched" : sourceWords.length ? "covered" : "not used";
    return {
      key: domain.key,
      label: domain.label,
      filter: domain.filter,
      status,
      catalogCount: catalogWords.length,
      sourceWords,
      highlightedWords,
      missingWords,
      highlightCount
    };
  });
}

function highlightCoverageCatalog() {
  const catalog = state.syntaxCatalog || emptySyntaxCatalog();
  const keywordGroupWords = Object.values(catalog.keywordGroups || {}).flatMap((items) => arrayOrEmpty(items));
  const publicFieldWords = [
    ...catalogItemLabels(catalog.tableFields),
    ...catalogItemLabels(catalog.sampleTableFields),
    ...catalogItemLabels(catalog.httpResponseFields),
    ...catalogItemLabels(catalog.coverageResultFields),
    ...catalogItemLabels(catalog.dbConnectionFields),
    ...catalogItemLabels(catalog.caseTableFields),
    ...catalogItemLabels(catalog.caseOutputTableFields),
    ...catalogItemLabels(catalog.caseRunResultTableFields),
    ...catalogItemLabels(catalog.caseResultCollectionTableFields),
    ...catalogItemLabels(catalog.modelFields),
    ...catalogItemLabels(catalog.predictionTableFields)
  ];
  return [
    {
      key: "keyword",
      label: "Keywords",
      filter: "keyword",
      words: [...catalog.keywords, ...keywordGroupWords]
    },
    {
      key: "type",
      label: "Types",
      filter: "type",
      words: catalog.publicTypes
    },
    {
      key: "quantity",
      label: "Quantities",
      filter: "quantity",
      words: catalog.quantities
    },
    {
      key: "workflow",
      label: "Workflow",
      filter: "workflow",
      words: [
        ...catalog.workflowBuiltins,
        ...catalog.hyphenatedWorkflowBuiltins,
        ...catalog.legacyWorkflowBuiltinAliases,
        ...catalog.workflowStatusLiterals
      ]
    },
    {
      key: "option",
      label: "Options",
      filter: "option",
      words: [...catalog.workflowOptions, ...catalog.legacyWorkflowOptionAliases]
    },
    {
      key: "unit",
      label: "Units",
      filter: "unit",
      words: [...catalog.units, ...catalog.legacyUnitAliases]
    },
    {
      key: "field",
      label: "Public fields",
      filter: "property",
      words: publicFieldWords
    },
    {
      key: "constant",
      label: "Constants",
      filter: "constant",
      words: [...catalog.constants, ...catalog.workflowStatusLiterals]
    },
    {
      key: "operator",
      label: "Operators",
      filter: "operator",
      words: catalog.operatorWords
    }
  ];
}

function renderHighlightCoverageTable(rows) {
  const body = rows.map((row) => {
    const statusClass = row.status === "unmatched" ? "blocked" : row.status === "covered" ? "completed" : "checked";
    return `
      <tr>
        <td>${highlightFilterChip(row.filter, row.label, row.key, `Filter ${row.label}`)}<div class="muted">catalog ${escapeHtml(String(row.catalogCount))}</div></td>
        <td><span class="status-pill ${statusClass}">${escapeHtml(row.status)}</span></td>
        <td>${escapeHtml(String(row.sourceWords.length))}</td>
        <td>${highlightFilterButton(row.filter, String(row.highlightCount))}</td>
        <td>${renderCoverageWordChips(row.highlightedWords, row.key, "No highlighted source words")}</td>
        <td>${renderCoverageWordChips(row.missingWords, "missing", "None")}</td>
      </tr>
    `;
  }).join("");
  return `
    <table class="var-table highlight-coverage-table">
      <thead><tr><th>Domain</th><th>Status</th><th>Source Words</th><th>Highlighted Ranges</th><th>Examples</th><th>Unmatched Source Words</th></tr></thead>
      <tbody>${body || `<tr><td colspan="6" class="muted">No highlight coverage summary for the current check.</td></tr>`}</tbody>
    </table>
  `;
}

function renderCoverageWordChips(words, kind, emptyText) {
  const items = arrayOrEmpty(words);
  if (!items.length) return `<span class="muted">${escapeHtml(emptyText)}</span>`;
  const visible = items.slice(0, 8);
  const chips = visible.map((word) => highlightFilterChip(word, word, kind, `Filter ${word}`)).join(" ");
  const hidden = items.length > visible.length
    ? ` <span class="muted">+${escapeHtml(String(items.length - visible.length))}</span>`
    : "";
  return `${chips}${hidden}`;
}
function renderCaretHighlightSummary(caret, tokenCurrent) {
  if (!tokenCurrent) {
    return `<div class="empty-state">${escapeHtml(state.check.status === "checking" ? "Analyzing current buffer..." : "Highlight data unavailable.")}</div>`;
  }
  const lineOverlapNotice = renderCaretLineOverlapNotice(caret?.lineOverlaps);
  if (!caret?.token) {
    const nearestTokens = arrayOrEmpty(caret?.nearestTokens);
    if (!nearestTokens.length) {
      return `<div class="empty-state">No exact highlight at the caret.</div>${lineOverlapNotice}`;
    }
    return `
      <div class="empty-state">No exact highlight at the caret.</div>
      ${lineOverlapNotice}
      <div class="panel-title compact">Nearby Highlights</div>
      ${renderNearbySemanticTokenRows(nearestTokens)}
    `;
  }
  const token = caret.token;
  const text = semanticTokenText(token);
  const modifiers = arrayOrEmpty(token.modifiers);
  const line = Number(token.line ?? -1) + 1;
  const start = Number(token.start ?? 0);
  const length = Number(token.length ?? 0);
  const hover = caret.hover ? hoverTitle(caret.hover) : "-";
  const selectorButtons = semanticTokenSelectors(token)
    .map((selector) => highlightFilterButton(selector, `Selector ${selector}`));
  const filterButtons = [
    text && text !== "-" ? highlightFilterButton(text, "Text") : "",
    token.type ? highlightFilterButton(token.type, "Category") : "",
    ...modifiers.map((modifier) => highlightFilterButton(modifier, `Detail ${modifier}`)),
    ...selectorButtons
  ].filter(Boolean).join(" ");
  const actionButtons = [
    sourceTokenCopyButton(token, "text", "Copy Text"),
    sourceTokenCopyButton(token, "range", "Copy Range"),
    '<button class="link-button token-range-button" data-show-document-highlights title="Highlight semantic references in this file (Shift+F12)">References</button>',
    '<button class="link-button token-range-button" data-rename-symbol title="Rename semantic symbol (F2)">Rename</button>',
    renderInspectorTabButtons(inspectorTabsForSemanticToken(token, caret.hover))
  ].filter(Boolean).join(" ");
  const modifierCells = semanticTokenModifierChips(modifiers);
  return `
    <table class="var-table caret-highlight-table">
      <tbody>
        <tr><th>Range</th><td>${sourceTokenButton(token, `L${line}`)} <span class="muted">${escapeHtml(String(start))}:${escapeHtml(String(length))}</span></td></tr>
        <tr><th>Actions</th><td>${actionButtons || "-"}</td></tr>
        <tr><th>Text</th><td><code>${escapeHtml(text)}</code></td></tr>
        <tr><th>Category</th><td>${semanticTokenTypeChip(token.type)}</td></tr>
        <tr><th>Details</th><td>${modifierCells}</td></tr>
        <tr><th>Selectors</th><td>${semanticTokenSelectorCells(token)}</td></tr>
        <tr><th>Line Overlaps</th><td>${renderCaretLineOverlapCell(caret?.lineOverlaps)}</td></tr>
        <tr><th>Hover</th><td>${escapeHtml(hover)}</td></tr>
        <tr><th>Filter</th><td>${filterButtons || "-"}</td></tr>
      </tbody>
    </table>
  `;
}

function renderCaretLineOverlapCell(overlaps) {
  const count = arrayOrEmpty(overlaps).length;
  if (!count) return "None";
  return `<span class="badge warn">Overlaps ${escapeHtml(String(count))}</span> <button class="link-button token-range-button" data-show-highlight-panel title="Open Highlight panel range overlaps">Highlight</button>`;
}

function renderCaretLineOverlapNotice(overlaps) {
  const count = arrayOrEmpty(overlaps).length;
  if (!count) return "";
  return `<div class="empty-state"><span class="badge warn">Line Overlaps ${escapeHtml(String(count))}</span> <button class="link-button token-range-button" data-show-highlight-panel title="Open Highlight panel range overlaps">Highlight</button></div>`;
}

function renderNearbySemanticTokenRows(tokens) {
  const rows = tokens.map((token) => {
    const modifiers = arrayOrEmpty(token.modifiers);
    return `
      <tr>
        <td>${sourceTokenButton(token)}<div class="muted">${sourceTokenCopyButton(token, "text", "Copy")}</div></td>
        <td><code>${escapeHtml(semanticTokenText(token))}</code></td>
        <td>${semanticTokenTypeChip(token.type)}</td>
        <td>${semanticTokenModifierChips(modifiers)}</td>
        <td>${semanticTokenSelectorCells(token)}</td>
      </tr>
    `;
  }).join("");
  return `
    <table class="var-table semantic-token-table">
      <thead><tr><th>Range</th><th>Text</th><th>Category</th><th>Details</th><th>Selectors</th></tr></thead>
      <tbody>${rows}</tbody>
    </table>
  `;
}

function highlightFilterButton(query, label) {
  return `<button class="link-button token-range-button" data-highlight-token-filter="${escapeAttr(query)}">${escapeHtml(label)}</button>`;
}

function highlightFilterChip(query, label, kind, title = "") {
  const safeKind = String(kind || "selector").replace(/[^A-Za-z0-9_-]/g, "") || "selector";
  const roleClass = highlightChipRoleClass(kind, query || label);
  const className = ["token-chip", `token-${safeKind}`, roleClass, "token-filter-chip"].filter(Boolean).join(" ");
  const chipTitle = title || `Filter ${label}`;
  return `<button class="${escapeAttr(className)}" data-highlight-token-filter="${escapeAttr(query)}" title="${escapeAttr(chipTitle)}">${escapeHtml(label)}</button>`;
}


function highlightChipRoleClass(kind, query) {
  const value = String(query || "").trim();
  if (!value || value === "-") return "";
  const normalizedKind = String(kind || "").trim();
  if (normalizedKind === "type") return semanticTokenTypeClass(value);
  if (normalizedKind === "modifier") return semanticTokenModifierClass(value);
  if (normalizedKind === "selector") return semanticTokenSelectorClass(value);
  switch (normalizedKind) {
    case "keyword": return "hl-keyword";
    case "option": return "hl-property";
    case "unit": return "hl-mod-unit";
    case "quantity": return "hl-mod-quantity";
    case "workflow": return "hl-mod-workflowStep";
    case "constant": return "hl-constant";
    case "operator": return "hl-operator";
    case "axis": return "hl-mod-axis";
    case "timeseries": return "hl-mod-timeseries";
    case "uncertain": return "hl-mod-uncertain";
    case "validation": return "hl-mod-validation";
    case "report": return "hl-mod-report";
    case "solver": return "hl-mod-solver";
    case "sideEffect": return "hl-mod-sideEffect";
    case "external": return "hl-mod-external";
    case "model": return "hl-mod-model";
    case "db": return "hl-mod-db";
    case "cache": return "hl-mod-cache";
    default: return "";
  }
}

function semanticTokenTypeClass(type) {
  switch (String(type || "").trim()) {
    case "namespace": return "hl-namespace";
    case "type": return "hl-type";
    case "class": return "hl-class";
    case "interface": return "hl-interface";
    case "parameter": return "hl-parameter";
    case "variable": return "hl-variable";
    case "property": return "hl-property";
    case "function": return "hl-function";
    case "method": return "hl-method";
    case "keyword": return "hl-keyword";
    case "modifier": return "hl-modifier";
    case "string": return "hl-string";
    case "number": return "hl-number";
    case "operator": return "hl-operator";
    case "comment": return "hl-comment";
    default: return "";
  }
}

function semanticTokenModifierClass(modifier) {
  const value = String(modifier || "").trim();
  return value ? `hl-mod-${safeCssToken(value)}` : "";
}

function semanticTokenSelectorClass(selector) {
  const parts = String(selector || "").split(".").map((part) => part.trim()).filter(Boolean);
  if (!parts.length) return "";
  return [semanticTokenTypeClass(parts[0]), ...parts.slice(1).map(semanticTokenModifierClass)]
    .filter(Boolean)
    .join(" ");
}

function semanticTokenTypeChip(type) {
  const value = String(type || "-");
  if (value === "-") return `<span class="token-chip token-type">-</span>`;
  return highlightFilterChip(value, value, "type", `Filter category ${value}`);
}

function semanticTokenModifierChips(modifiers) {
  const items = arrayOrEmpty(modifiers).filter(Boolean);
  return items.length
    ? items.map((modifier) => highlightFilterChip(modifier, modifier, "modifier", `Filter detail ${modifier}`)).join(" ")
    : "-";
}

function semanticTokenLegendChip(item, kind) {
  const role = kind === "type" ? "category" : "detail";
  return highlightFilterChip(item, item, kind, `Filter ${role} ${item}`);
}

function renderSemanticLegendTable(items, counts, kind) {
  const rows = items.map((item) => `
    <tr>
      <td>${semanticTokenLegendChip(item, kind)}</td>
      <td>${escapeHtml(String(counts.get(item) || 0))}</td>
    </tr>
  `).join("");
  return `
    <table class="var-table semantic-legend-table">
      <thead><tr><th>${escapeHtml(kind === "type" ? "Category" : "Detail")}</th><th>Count</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="2" class="muted">No highlight categories for the current check.</td></tr>`}</tbody>
    </table>
  `;
}

function renderSemanticSelectorTable(counts) {
  const rows = [...counts.entries()]
    .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
    .map(([selector, count]) => `
      <tr>
        <td><code>${escapeHtml(selector)}</code></td>
        <td>${highlightFilterButton(selector, String(count))}</td>
      </tr>
    `).join("");
  return `
    <table class="var-table semantic-legend-table">
      <thead><tr><th>Selector</th><th>Count</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="2" class="muted">No highlight selectors for the current check.</td></tr>`}</tbody>
    </table>
  `;
}

function renderSemanticOverlapSummary(overlaps) {
  const rows = overlaps.slice(0, 40).map((item) => {
    const left = item.left || {};
    const right = item.right || {};
    const selectors = [...new Set([
      ...semanticTokenSelectors(left),
      ...semanticTokenSelectors(right)
    ])];
    return `
      <tr>
        <td>${sourceTokenButton(left, `L${String(item.line)}`)}<div class="muted">${escapeHtml(String(item.start))}:${escapeHtml(String(item.end - item.start))}</div></td>
        <td><code>${escapeHtml(item.text || "-")}</code></td>
        <td>${semanticTokenTypeChip(left.type)} <span class="muted">vs</span> ${semanticTokenTypeChip(right.type)}</td>
        <td>${semanticTokenModifierChips(arrayOrEmpty(left.modifiers))}<div class="muted">${semanticTokenModifierChips(arrayOrEmpty(right.modifiers))}</div></td>
        <td>${selectors.length ? selectors.map((selector) => highlightFilterChip(selector, selector, "selector", `Filter selector ${selector}`)).join(" ") : "-"}</td>
        <td>${sourceTokenActions(left)}<div class="muted">${sourceTokenActions(right)}</div></td>
      </tr>
    `;
  }).join("");
  const hidden = overlaps.length > 40 ? `<div class="empty-state">Showing first 40 of ${escapeHtml(String(overlaps.length))} overlapping ranges.</div>` : "";
  return `
    <table class="var-table semantic-token-table">
      <thead><tr><th>Range</th><th>Text</th><th>Categories</th><th>Details</th><th>Selectors</th><th>Actions</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">No overlapping semantic highlight ranges for the current check.</td></tr>`}</tbody>
    </table>
    ${hidden}
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
        <td>${semanticTokenTypeChip(token.type)}</td>
        <td>${semanticTokenModifierChips(modifiers)}</td>
        <td>${semanticTokenSelectorCells(token)}</td>
        <td>${sourceTokenActions(token)}</td>
      </tr>
    `;
  }).join("");
  const hidden = tokens.length > 120 ? `<div class="empty-state">Showing first 120 of ${escapeHtml(String(tokens.length))} highlights.</div>` : "";
  const empty = filtered ? "No highlights match the current filter." : "No highlights for the current check.";
  return `
    <table class="var-table semantic-token-table">
      <thead><tr><th>Range</th><th>Text</th><th>Category</th><th>Details</th><th>Selectors</th><th>Actions</th></tr></thead>
      <tbody>${rows || `<tr><td colspan="6" class="muted">${escapeHtml(empty)}</td></tr>`}</tbody>
    </table>
    ${hidden}
  `;
}

function sourceTokenActions(token) {
  const actions = [
    sourceTokenCopyButton(token, "text", "Copy Text"),
    sourceTokenCopyButton(token, "range", "Copy Range"),
    sourceTokenCopyButton(token, "selector", "Copy Selector"),
    renderInspectorTabButtons(inspectorTabsForSemanticToken(token))
  ].filter(Boolean);
  return actions.length ? actions.join(" ") : "-";
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
        "No quality results yet.",
        "Run a file with validations, schema constraints, or expectation suites.",
        "Quality results are saved with the run result data."
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
      ${advancedDataToggle("Advanced quality data", quality)}
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
        "No kernel plan yet.",
        "Run a file with supported solver or state-space work.",
        "Kernel plan details are saved with the report data."
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
      ${advancedDataToggle("Advanced kernel plan data", plan)}
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
  const processResults = inspectorObject("processResults");
  const processes = Array.isArray(processResults.processes) ? processResults.processes : [];
  const processEvidence = workflowProcessEvidence(processResults, processes);
  if (!Object.keys(plan).length) {
    return `
      <div class="panel-title compact">Workflow</div>
      ${panelArtifactEmptyState(
        "No workflow graph yet.",
        "Run the current file to generate the workflow graph.",
        "Workflow graph details are saved after a successful run."
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
      <span class="badge">Processes ${escapeHtml(processEvidence.countLabel)}</span>
    </div>
    <div class="run-actions">
      <button data-open-artifact-kind="run_plan">Open run_plan.json</button>
      <button data-open-artifact-kind="process_results">Open process_results.json</button>
    </div>
    <div class="scroll">
      <div class="panel-title compact">DAG</div>
      ${renderWorkflowDag(nodes, edges, selectedNode?.id)}
      <div class="panel-title compact">Native Evidence</div>
      ${renderWorkflowNativeEvidence(plan, processEvidence, nodes, edges)}
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

function workflowProcessEvidence(processResults, processes) {
  const hasArtifact = hasAdvancedData(processResults);
  const count = hasArtifact ? processResultCount(processResults, processes) : null;
  const processListCount = hasArtifact ? processes.length : null;
  const zeroExternal = hasArtifact && count === 0 && processListCount === 0;
  return {
    hasArtifact,
    count,
    countLabel: hasArtifact ? String(count) : "missing",
    processListCount,
    zeroExternal,
    status: !hasArtifact
      ? "missing"
      : zeroExternal
        ? "zero external processes"
        : "external processes present",
    profile: processResults.execution_profile || processResults.executionProfile || "-",
    format: processResults.format || "-"
  };
}

function renderWorkflowNativeEvidence(plan, processEvidence, nodes, edges) {
  const graphStatus = Object.keys(plan).length ? "present" : "missing";
  const hashes = plan.artifact_hashes || plan.artifactHashes || {};
  const runPlanHash = hashes.run_plan || hashes.runPlan || "-";
  const staticRunPlanHash = hashes.static_run_plan || hashes.staticRunPlan || "-";
  const processDetail = processEvidence.hasArtifact
    ? `process_count=${processEvidence.count}; processes=${processEvidence.processListCount}; profile=${processEvidence.profile}; format=${processEvidence.format}`
    : "process_results.json missing";
  return `
    <table class="var-table compact-table">
      <thead><tr><th>Evidence</th><th>Status</th><th>Detail</th></tr></thead>
      <tbody>
        <tr>
          <td>Run graph</td>
          <td>${escapeHtml(graphStatus)}</td>
          <td>nodes=${escapeHtml(nodes.length)}; edges=${escapeHtml(edges.length)}; format=${escapeHtml(plan.format || "-")}</td>
        </tr>
        <tr>
          <td>Process results</td>
          <td>${escapeHtml(processEvidence.status)}</td>
          <td>${escapeHtml(processDetail)}</td>
        </tr>
        <tr>
          <td>Graph hashes</td>
          <td>${escapeHtml(runPlanHash !== "-" || staticRunPlanHash !== "-" ? "recorded" : "missing")}</td>
          <td><code>${escapeHtml(compactText(`run_plan=${runPlanHash}; static_run_plan=${staticRunPlanHash}`, 150))}</code></td>
        </tr>
      </tbody>
    </table>
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
      <div class="panel-title compact">Review Fingerprint</div>
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
      ${advancedDataToggle("Advanced review data", doc)}
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
  if (status === "native_preview") return "native";
  if (status.startsWith("supported")) return "native";
  if (status.includes("internal")) return "internal";
  if (status.includes("planned")) return "planned";
  return "unknown";
}

function moduleStatusLabel(module) {
  if (module.statusLabel) return module.statusLabel;
  if (module.status_label) return module.status_label;
  switch (module.status) {
    case "supported":
      return "Supported";
    case "supported_narrow":
      return "Supported narrow";
    case "native_preview":
      return "Native";
    case "planned":
      return "Planned";
    case "internal_planned":
      return "Internal target";
    case "internal":
      return "Internal";
    default:
      return module.status || "-";
  }
}

function moduleStatusDetail(module) {
  if (module.statusDetail) return module.statusDetail;
  if (module.status_detail) return module.status_detail;
  switch (module.status) {
    case "supported":
      return "Public built-in surface supported by compiler/runtime.";
    case "supported_narrow":
      return "Supported for the listed syntax forms and review artifacts.";
    case "native_preview":
      return "Native runtime path is implemented for the listed workflow commands and artifacts; unsupported combinations report diagnostics.";
    case "planned":
      return "Documented target module; not yet executable as a public stdlib API.";
    case "internal_planned":
      return "Internal target, not a public stdlib API.";
    case "internal":
      return "Internal compiler/runtime vocabulary outside the public stdlib API.";
    default:
      return "-";
  }
}

function moduleStatusDisplay(module) {
  return module.statusLabel || module.status_label || moduleStatusLabel(module) || "-";
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
    const limitations = statusListLabel(solverPreview.limitations);
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
        <td><strong>${escapeHtml(assembly.name || "-")}</strong><div class="muted">${escapeHtml(statusLabel(assembly.status || "-"))}</div></td>
        <td>${escapeHtml(assembly.component_count ?? assembly.componentCount ?? 0)} / ${escapeHtml(assembly.port_count ?? assembly.portCount ?? 0)}</td>
        <td>${escapeHtml(setCount)}<div class="muted">domains ${escapeHtml(domainCount)}</div></td>
        <td>${escapeHtml(Array.isArray(assembly.equations) ? assembly.equations.length : 0)}<div class="muted">component ${escapeHtml(assembly.component_equation_count ?? assembly.componentEquationCount ?? 0)}</div><div class="muted">unknowns ${escapeHtml(boundary.unknown_count ?? boundary.unknownCount ?? 0)}</div></td>
        <td>${escapeHtml(statusLabel(solverStatus))}<div class="muted">${escapeHtml(statusLabel(solverMethod))}</div><div class="muted">${escapeHtml(limitations)}</div></td>
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
      <td>${escapeHtml(statusLabel(node.status || "-"))}</td>
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
      <details class="advanced-data-toggle">
        <summary>Advanced node data</summary>
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
  if (relationship) parts.push(`relationship=${statusLabel(relationship)}`);
  if (contract) parts.push(`contract=${statusLabel(contract)}`);
  if (jacobian) parts.push(`jacobian=${statusLabel(jacobian)}`);
  if (profile) parts.push(`profile=${statusLabel(profile)}`);
  if (Array.isArray(contractInputs) && contractInputs.length) {
    parts.push(`inputs=${behaviorContractDetails(contractInputs)}`);
  }
  if (Array.isArray(contractOutputs) && contractOutputs.length) {
    parts.push(`outputs=${behaviorContractDetails(contractOutputs)}`);
  }
  if (Array.isArray(diagnostics) && diagnostics.length) {
    parts.push(`diagnostics=${diagnostics.join("|")}`);
  }
  if (runtimeWarnings) parts.push(`runtime_warnings=${statusLabel(runtimeWarnings)}`);
  return parts.length ? parts.join(", ") : "-";
}

function behaviorContractDetails(contracts) {
  return contracts.map((contract) => {
    const role = contract.role || "-";
    const name = contract.name || "-";
    const quantity = contract.quantity_kind || contract.quantityKind || "-";
    const unit = contract.display_unit || contract.displayUnit || "-";
    const status = statusLabel(contract.status || "-");
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
  const processCount = processResultCount(processResults, processes);
  if (!artifacts.length && !boundaries.length && !processes.length) {
    return `
      <div class="panel-title compact">Effects</div>
      ${panelArtifactEmptyState(
        "No side-effect records yet.",
        "Run a file with write/render/run/test/database operations.",
        "Side-effect records are saved after a run with explicit effects."
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
      <div class="panel-title compact">${escapeHtml(processResultsPanelTitle(processCount))}</div>
      ${renderProcessResults(processes, processCount)}
      ${advancedDataToggle("Advanced effects data", { effects, processResults })}
    </div>
  `;
}

function processResultCount(processResults, processes) {
  const count = Number(processResults.process_count ?? processResults.processCount);
  if (Number.isFinite(count) && count >= 0) {
    return count;
  }
  return processes.length;
}

function processResultsPanelTitle(processCount) {
  if (processCount === 0) {
    return "Process Results (0 external processes)";
  }
  return `External Process Results (${processCount})`;
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
        "No network or cache records yet.",
        "Run a file with http/download/cache boundaries.",
        "Network and cache records are saved after a run with HTTP, download, or cache boundaries."
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
      ${advancedDataToggle("Advanced network/cache data", network)}
    </div>
  `;
}

function renderNetworkBoundaries(boundaries) {
  const rows = boundaries.map((boundary) => {
    const query = Array.isArray(boundary.query) ? boundary.query : [];
    const bodyLimit = boundary.body_size_limit_bytes ?? boundary.bodySizeLimitBytes;
    const responseSource = boundary.response_source || boundary.responseSource || boundary.status || "-";
    const httpStatus = [boundary.status_class || boundary.statusClass || "", boundary.status_code ?? boundary.statusCode ?? ""]
      .filter((part) => part !== "")
      .join(" ") || "-";
    const policy = [
      boundary.retry !== undefined && boundary.retry !== null ? `retry ${boundary.retry}` : "",
      boundary.timeout ? `timeout ${boundary.timeout}` : "",
      bodyLimit !== undefined && bodyLimit !== null ? `limit ${bodyLimit} B` : "",
      query.length ? `query ${query.length}` : "",
    ].filter(Boolean).join("; ") || "-";
    return `
      <tr>
        <td><strong>${escapeHtml(boundary.kind || "-")}</strong><div class="muted">${escapeHtml(boundary.binding || boundary.target || "-")}</div></td>
        <td>${escapeHtml(responseSource)}<div class="muted">HTTP ${escapeHtml(httpStatus)}</div></td>
        <td><code>${escapeHtml(compactText(boundary.url || boundary.target || "-", 90))}</code></td>
        <td>${escapeHtml(policy)}</td>
        <td><code>${escapeHtml(compactText(boundary.response_hash || boundary.responseHash || "-", 68))}</code><div class="muted"><code>${escapeHtml(compactText(boundary.expected_sha256 || boundary.expectedSha256 || "-", 68))}</code></div></td>
        <td>${sourceLineButton(boundary)}</td>
      </tr>
    `;
  }).join("");
  return `
    <table class="artifact-table">
      <thead><tr><th>Boundary</th><th>Response Source</th><th>URL / Target</th><th>Policy</th><th>Observed / Expected</th><th>Source</th></tr></thead>
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
      ${advancedDataToggle("Advanced DB data", db)}
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
      ${advancedDataToggle("Advanced model data", model)}
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
      ${advancedDataToggle("Advanced case data", caseData)}
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

function renderProcessResults(processes, processCount = processes.length) {
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
      <tbody>${rows || `<tr><td colspan="5" class="muted">${escapeHtml(processResultsEmptyText(processCount))}</td></tr>`}</tbody>
    </table>
  `;
}

function processResultsEmptyText(processCount) {
  if (processCount === 0) {
    return "No external process executions recorded.";
  }
  return "No process result rows.";
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

function renderDocumentHighlightReferences() {
  const references = currentWorkspaceReferences();
  const payload = state.workspaceReferences || {};
  if (references.length) {
    const files = new Set(references.map((reference) => reference?.uri).filter(Boolean));
    const rows = references.slice(0, 200).map((reference) => {
      const range = workspaceReferenceRange(reference);
      if (!range) return "";
      const absolutePath = definitionPathFromUri(reference.uri);
      const workspacePath = absolutePath ? definitionWorkspacePath(absolutePath) : String(reference.uri || "");
      const folder = directoryOf(workspacePath);
      const highlight = documentHighlightForWorkspaceReference(reference);
      const kind = Number(highlight?.kind) === 3
        ? "Write"
        : Number(highlight?.kind) === 2
          ? "Read"
          : "Reference";
      const locationLabel = `${fileName(workspacePath)}:${range.start.line + 1}`;
      return `
        <tr>
          <td>
            ${absolutePath ? `<button class="link-button token-range-button" data-workspace-reference-uri="${escapeAttr(reference.uri)}" data-workspace-reference-line="${range.start.line}" data-workspace-reference-character="${range.start.character}" data-workspace-reference-end-line="${range.end.line}" data-workspace-reference-end-character="${range.end.character}" title="Open ${escapeAttr(workspacePath)}">${escapeHtml(locationLabel)}</button>` : escapeHtml(locationLabel)}
            <div class="muted">${escapeHtml(folder)}</div>
          </td>
          <td><span class="status-pill">${escapeHtml(kind)}</span></td>
          <td><code>${escapeHtml(payload.label || "symbol")}</code></td>
        </tr>
      `;
    }).filter(Boolean).join("");
    return `
      <div class="badges">
        <span class="badge">References ${references.length}</span>
        <span class="badge">Files ${files.size}</span>
      </div>
      ${payload.notice ? `<div class="empty-state compact">${escapeHtml(payload.notice)}</div>` : ""}
      <table class="var-table semantic-reference-table">
        <thead><tr><th>Location</th><th>Access</th><th>Symbol</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>
    `;
  }
  const highlights = currentDocumentHighlights();
  if (!highlights.length) {
    const notice = String(payload.notice || "");
    return `<div class="empty-state">${escapeHtml(notice || "Place the caret on a symbol and use References or Shift+F12.")}</div>`;
  }
  const rows = highlights.slice(0, 100).map((highlight) => {
    const token = documentHighlightToken(highlight);
    if (!token) return "";
    const kind = Number(highlight.kind) === 3 ? "Write" : Number(highlight.kind) === 2 ? "Read" : "Text";
    return `
      <tr>
        <td>${sourceTokenButton(token, `L${token.line + 1}`)}</td>
        <td><span class="status-pill">${escapeHtml(kind)}</span></td>
        <td><code>${escapeHtml(semanticTokenText(token))}</code></td>
      </tr>
    `;
  }).filter(Boolean).join("");
  return `
    <div class="badges"><span class="badge">References ${highlights.length}</span></div>
    ${payload.notice ? `<div class="empty-state compact">${escapeHtml(payload.notice)}</div>` : ""}
    <table class="var-table semantic-reference-table">
      <thead><tr><th>Range</th><th>Access</th><th>Symbol</th></tr></thead>
      <tbody>${rows}</tbody>
    </table>
  `;
}

function documentHighlightToken(highlight) {
  const start = highlight?.range?.start;
  const end = highlight?.range?.end;
  const line = Number(start?.line);
  const startCharacter = Number(start?.character);
  const endLine = Number(end?.line);
  const endCharacter = Number(end?.character);
  if (
    !Number.isInteger(line)
    || !Number.isInteger(startCharacter)
    || endLine !== line
    || !Number.isInteger(endCharacter)
    || endCharacter <= startCharacter
  ) {
    return null;
  }
  return { line, start: startCharacter, length: endCharacter - startCharacter };
}

function workspaceReferenceRange(reference) {
  const start = reference?.range?.start;
  const end = reference?.range?.end;
  const range = {
    start: { line: Number(start?.line), character: Number(start?.character) },
    end: { line: Number(end?.line), character: Number(end?.character) }
  };
  if (
    !Number.isInteger(range.start.line)
    || !Number.isInteger(range.start.character)
    || !Number.isInteger(range.end.line)
    || !Number.isInteger(range.end.character)
    || range.start.line < 0
    || range.start.character < 0
    || range.end.line < range.start.line
    || (range.end.line === range.start.line && range.end.character <= range.start.character)
  ) {
    return null;
  }
  return range;
}

function documentHighlightForWorkspaceReference(reference) {
  const referencePath = definitionPathFromUri(reference?.uri);
  const payload = state.documentHighlights || {};
  const workspacePayload = state.workspaceReferences || {};
  if (
    !referencePath
    || payload.path !== workspacePayload.path
    || payload.source !== workspacePayload.source
    || !sameDefinitionPath(definitionWorkspacePath(referencePath), payload.path)
  ) {
    return null;
  }
  const range = workspaceReferenceRange(reference);
  if (!range || range.start.line !== range.end.line) return null;
  return arrayOrEmpty(payload.items).find((item) => {
    const token = documentHighlightToken(item);
    return token
      && token.line === range.start.line
      && token.start === range.start.character
      && token.length === range.end.character - range.start.character;
  }) || null;
}

function bindEditorFindControls() {
  const input = byId("editorFindInput");
  if (!input) return;
  input.oninput = (event) => {
    state.editorFindQuery = event.target.value;
    state.editorFindMatchIndex = -1;
    findEditorMatch(1, true);
  };
  input.onkeydown = (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      findEditorMatch(event.shiftKey ? -1 : 1);
    } else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      closeEditorFind();
    }
  };
  byId("editorFindPrevBtn").onclick = () => {
    findEditorMatch(-1);
    input.focus();
  };
  byId("editorFindNextBtn").onclick = () => {
    findEditorMatch(1);
    input.focus();
  };
  byId("editorFindCaseBtn").onclick = () => {
    state.editorFindCaseSensitive = !state.editorFindCaseSensitive;
    state.editorFindMatchIndex = -1;
    const button = byId("editorFindCaseBtn");
    button.classList.toggle("active", state.editorFindCaseSensitive);
    button.setAttribute("aria-pressed", String(state.editorFindCaseSensitive));
    findEditorMatch(1, true);
    input.focus();
  };
  byId("editorFindCloseBtn").onclick = closeEditorFind;
  updateEditorFindStatus();
}

function openEditorFind() {
  hideCompletions();
  const editor = byId("editor");
  const selectedQuery = selectedEditorFindQuery(editor);
  if (selectedQuery) {
    state.editorFindQuery = selectedQuery;
    state.editorFindMatchIndex = -1;
  }
  state.editorFindOpen = true;
  const bar = byId("editorFindBar");
  const input = byId("editorFindInput");
  if (!bar || !input) return;
  bar.classList.remove("hidden");
  input.value = state.editorFindQuery;
  if (state.editorFindQuery) findEditorMatch(1, true);
  else updateEditorFindStatus();
  input.focus();
  input.select();
}

function closeEditorFind() {
  state.editorFindOpen = false;
  byId("editorFindBar")?.classList.add("hidden");
  byId("editor")?.focus();
}

function selectedEditorFindQuery(editor) {
  if (!editor || editor.selectionStart === editor.selectionEnd) return "";
  const selected = editor.value.slice(editor.selectionStart, editor.selectionEnd);
  if (!selected.trim() || selected.length > 200 || /[\r\n]/.test(selected)) return "";
  return selected;
}

function editorFindRanges(source, query, caseSensitive = false) {
  const needle = String(query || "");
  if (!needle) return [];
  const original = String(source || "");
  const haystack = caseSensitive ? original : original.toLowerCase();
  const comparableNeedle = caseSensitive ? needle : needle.toLowerCase();
  const ranges = [];
  let offset = 0;
  while (offset <= haystack.length - comparableNeedle.length) {
    const start = haystack.indexOf(comparableNeedle, offset);
    if (start < 0) break;
    ranges.push({ start, end: start + needle.length });
    offset = start + Math.max(1, comparableNeedle.length);
  }
  return ranges;
}

function findEditorMatch(direction = 1, fromSelection = false) {
  const editor = byId("editor");
  if (!editor) return false;
  const ranges = editorFindRanges(
    editor.value,
    state.editorFindQuery,
    state.editorFindCaseSensitive
  );
  if (!ranges.length) {
    state.editorFindMatchIndex = -1;
    updateEditorFindStatus();
    return false;
  }

  let index = state.editorFindMatchIndex;
  if (fromSelection || index < 0 || index >= ranges.length) {
    const cursor = Math.min(editor.selectionStart, editor.selectionEnd);
    if (direction < 0) {
      index = ranges.length - 1;
      for (let candidate = ranges.length - 1; candidate >= 0; candidate -= 1) {
        if (ranges[candidate].start < cursor) {
          index = candidate;
          break;
        }
      }
    } else {
      index = ranges.findIndex((range) => range.start >= cursor);
      if (index < 0) index = 0;
    }
  } else {
    index = (index + (direction < 0 ? -1 : 1) + ranges.length) % ranges.length;
  }

  const match = ranges[index];
  state.editorFindMatchIndex = index;
  editor.selectionStart = match.start;
  editor.selectionEnd = match.end;
  revealEditorFindMatch(editor, match.start);
  updateEditorFindStatus();
  return true;
}

function revealEditorFindMatch(editor, offset) {
  const line = editor.value.slice(0, offset).split("\n").length - 1;
  const lineHeight = 19;
  const lineTop = line * lineHeight;
  const viewportHeight = Number(editor.clientHeight || 200);
  const viewportBottom = Number(editor.scrollTop || 0) + viewportHeight - lineHeight;
  if (lineTop < editor.scrollTop || lineTop > viewportBottom) {
    editor.scrollTop = Math.max(0, lineTop - Math.floor(viewportHeight / 2));
    syncEditorHighlightScroll();
  }
}

function updateEditorFindStatus() {
  const editor = byId("editor");
  const ranges = editorFindRanges(
    editor?.value ?? state.source,
    state.editorFindQuery,
    state.editorFindCaseSensitive
  );
  if (editor) {
    state.editorFindMatchIndex = ranges.findIndex((range) =>
      range.start === editor.selectionStart && range.end === editor.selectionEnd
    );
  } else if (state.editorFindMatchIndex >= ranges.length) {
    state.editorFindMatchIndex = -1;
  }
  const count = byId("editorFindCount");
  if (count) {
    count.textContent = ranges.length
      ? `${state.editorFindMatchIndex + 1}/${ranges.length}`
      : "0/0";
  }
}

function handleEditorKeyDown(event) {
  const editor = event.currentTarget;
  const overlayVisible = state.completionItems.length > 0;
  if ((event.ctrlKey || event.metaKey) && event.key === "/") {
    event.preventDefault();
    toggleEditorLineComment(editor);
    return;
  }
  if ((event.ctrlKey || event.metaKey) && event.key === " ") {
    event.preventDefault();
    updateCompletionOverlay(true);
    return;
  }
  if (overlayVisible && (event.key === "Tab" || event.key === "Enter")) {
    event.preventDefault();
    insertCompletion(state.completionItems[state.completionIndex]);
    return;
  }
  if (overlayVisible && event.key === "Escape") {
    event.preventDefault();
    hideCompletions();
    return;
  }
  if (overlayVisible && event.key === "ArrowDown") {
    event.preventDefault();
    state.completionIndex = (state.completionIndex + 1) % state.completionItems.length;
    drawCompletionOverlay();
    return;
  }
  if (overlayVisible && event.key === "ArrowUp") {
    event.preventDefault();
    state.completionIndex = (state.completionIndex + state.completionItems.length - 1) % state.completionItems.length;
    drawCompletionOverlay();
    return;
  }
  if (handleEditorPairKey(event, editor)) {
    return;
  }
  if (event.key === "Tab") {
    event.preventDefault();
    if (event.shiftKey) outdentEditorSelection(editor);
    else indentEditorSelection(editor);
    return;
  }
  if (event.key === "Enter" && !event.ctrlKey && !event.metaKey && !event.altKey) {
    event.preventDefault();
    insertEditorNewlineWithIndent(editor);
  }
}

function toggleEditorLineComment(editor) {
  const source = editor.value;
  const range = selectedLineEditRange(source, editor.selectionStart, editor.selectionEnd);
  const block = source.slice(range.start, range.end);
  const lines = splitTextLines(block);
  const contentLines = lines.filter((line) => line.text.trim().length > 0);
  const shouldUncomment = contentLines.length > 0
    && contentLines.every((line) => isLineCommented(line.text));
  const changed = lines.map((line) => ({
    ...line,
    text: shouldUncomment ? uncommentLine(line.text) : commentLine(line.text)
  }));
  applyLineBlockEdit(editor, range, lines, changed);
}

function indentEditorSelection(editor) {
  const source = editor.value;
  const range = selectedLineEditRange(source, editor.selectionStart, editor.selectionEnd);
  const lines = splitTextLines(source.slice(range.start, range.end));
  const changed = lines.map((line) => ({ ...line, text: `${EDITOR_INDENT}${line.text}` }));
  applyLineBlockEdit(editor, range, lines, changed);
}

function outdentEditorSelection(editor) {
  const source = editor.value;
  const range = selectedLineEditRange(source, editor.selectionStart, editor.selectionEnd);
  const lines = splitTextLines(source.slice(range.start, range.end));
  const changed = lines.map((line) => ({ ...line, text: outdentLine(line.text) }));
  applyLineBlockEdit(editor, range, lines, changed);
}

function handleEditorPairKey(event, editor) {
  if (event.ctrlKey || event.metaKey || event.altKey) return false;
  if (event.key === "Backspace") {
    const handled = deleteEmptyEditorPair(editor);
    if (handled) event.preventDefault();
    return handled;
  }
  if (event.key === "}" && insertClosingBraceWithIndent(editor)) {
    event.preventDefault();
    return true;
  }
  if (EDITOR_PAIR_OPEN[event.key] && skipEditorClosingPair(editor, event.key)) {
    event.preventDefault();
    return true;
  }
  if (EDITOR_PAIR_CLOSE[event.key]) {
    event.preventDefault();
    insertEditorPair(editor, event.key, EDITOR_PAIR_CLOSE[event.key]);
    return true;
  }
  return false;
}

function insertEditorPair(editor, open, close) {
  const start = Math.min(editor.selectionStart, editor.selectionEnd);
  const end = Math.max(editor.selectionStart, editor.selectionEnd);
  const selectedText = editor.value.slice(start, end);
  replaceEditorRange(
    editor,
    start,
    end,
    `${open}${selectedText}${close}`,
    start + open.length,
    start + open.length + selectedText.length
  );
}

function skipEditorClosingPair(editor, close) {
  if (editor.selectionStart !== editor.selectionEnd) return false;
  const cursor = editor.selectionStart;
  if (editor.value[cursor] !== close) return false;
  editor.selectionStart = cursor + close.length;
  editor.selectionEnd = cursor + close.length;
  hideCompletions();
  updateCursorInsight();
  return true;
}

function deleteEmptyEditorPair(editor) {
  if (editor.selectionStart !== editor.selectionEnd) return false;
  const cursor = editor.selectionStart;
  if (cursor <= 0) return false;
  const open = editor.value[cursor - 1];
  const close = editor.value[cursor];
  if (EDITOR_PAIR_CLOSE[open] !== close) return false;
  replaceEditorRange(editor, cursor - 1, cursor + close.length, "", cursor - 1, cursor - 1);
  return true;
}

function insertClosingBraceWithIndent(editor) {
  if (editor.selectionStart !== editor.selectionEnd) return false;
  const source = editor.value;
  const cursor = editor.selectionStart;
  if (source[cursor] === "}") return false;
  const lineStart = lineStartOffset(source, cursor);
  const beforeLine = source.slice(lineStart, cursor);
  if (!/^\s*$/.test(beforeLine)) return false;
  const nextBeforeLine = outdentLine(beforeLine);
  const insertText = `${nextBeforeLine}}`;
  const nextCursor = lineStart + insertText.length;
  replaceEditorRange(editor, lineStart, cursor, insertText, nextCursor, nextCursor);
  return true;
}

function insertEditorNewlineWithIndent(editor) {
  const source = editor.value;
  const start = Math.min(editor.selectionStart, editor.selectionEnd);
  const end = Math.max(editor.selectionStart, editor.selectionEnd);
  const lineStart = lineStartOffset(source, start);
  const lineEnd = lineEndOffset(source, end);
  const beforeLine = source.slice(lineStart, start);
  const afterLine = source.slice(end, lineEnd);
  const indent = (beforeLine.match(/^\s*/) || [""])[0];
  const trimmedBefore = beforeLine.trimEnd();
  const lineEnding = preferredLineEnding(source);
  let nextIndent = indent;
  const docComment = /^(\s*)\/\/\/ ?/.exec(beforeLine);
  if (docComment) {
    nextIndent = `${docComment[1]}/// `;
  } else if (trimmedBefore.endsWith("{")) {
    nextIndent = `${indent}${EDITOR_INDENT}`;
  }

  const shouldSplitClosingBrace = trimmedBefore.endsWith("{") && /^\s*\}/.test(afterLine);
  const insertText = shouldSplitClosingBrace
    ? `${lineEnding}${nextIndent}${lineEnding}${indent}`
    : `${lineEnding}${nextIndent}`;
  const cursor = start + lineEnding.length + nextIndent.length;
  replaceEditorRange(editor, start, end, insertText, cursor, cursor);
}

function commentLine(line) {
  if (!line.trim()) return `${line}# `;
  return line.replace(/^(\s*)/, "$1# ");
}

function uncommentLine(line) {
  return line.replace(/^(\s*)(?:#|\/\/(?!\/)) ?/, "$1");
}

function isLineCommented(line) {
  return /^\s*(?:#|\/\/(?!\/)) ?/.test(line);
}

function outdentLine(line) {
  if (line.startsWith(EDITOR_INDENT)) return line.slice(EDITOR_INDENT.length);
  if (line.startsWith("\t")) return line.slice(1);
  if (line.startsWith(" ")) return line.slice(1);
  return line;
}

function applyLineBlockEdit(editor, range, originalLines, changedLines) {
  const nextBlock = joinTextLines(changedLines);
  const selected = editor.selectionStart !== editor.selectionEnd;
  const source = editor.value;
  const oldBlock = source.slice(range.start, range.end);
  const before = source.slice(0, range.start);
  const after = source.slice(range.end);
  editor.value = `${before}${nextBlock}${after}`;
  if (selected) {
    editor.selectionStart = range.start;
    editor.selectionEnd = range.start + nextBlock.length;
  } else {
    const column = editor.selectionStart - range.start;
    const firstOriginal = originalLines[0]?.text ?? oldBlock;
    const firstChanged = changedLines[0]?.text ?? nextBlock;
    const delta = lineEditDeltaBeforeColumn(firstOriginal, firstChanged, column);
    const cursor = Math.max(range.start, range.start + column + delta);
    editor.selectionStart = cursor;
    editor.selectionEnd = cursor;
  }
  syncEditorManualEdit(editor);
}

function lineEditDeltaBeforeColumn(original, changed, column) {
  let prefix = 0;
  const limit = Math.min(original.length, changed.length);
  while (prefix < limit && original[prefix] === changed[prefix]) prefix += 1;
  return prefix < column ? changed.length - original.length : 0;
}

function replaceEditorRange(editor, start, end, text, selectionStart, selectionEnd) {
  const source = editor.value;
  editor.value = `${source.slice(0, start)}${text}${source.slice(end)}`;
  editor.selectionStart = selectionStart;
  editor.selectionEnd = selectionEnd;
  syncEditorManualEdit(editor);
}

function syncEditorManualEdit(editor) {
  state.source = editor.value;
  state.dirty = state.source !== state.savedSource;
  rememberCurrentTab();
  state.status = "Modified";
  const checkChanged = markCheckPending();
  renderTabLabels();
  hideCompletions();
  updateEditorLineCount();
  updateCheckSummaryUi();
  updateEditorHighlight();
  updateCursorInsight();
  updateEditorFindStatus();
  if (checkChanged) refreshCheckPanels();
  scheduleLiveCheck();
  editor.focus();
}

function selectedLineEditRange(source, selectionStart, selectionEnd) {
  const start = Math.min(selectionStart, selectionEnd);
  const rawEnd = Math.max(selectionStart, selectionEnd);
  const end = trimTrailingSelectedLineBreak(source, start, rawEnd);
  return {
    start: lineStartOffset(source, start),
    end: lineEndOffset(source, end)
  };
}

function trimTrailingSelectedLineBreak(source, selectionStart, selectionEnd) {
  if (selectionEnd <= selectionStart) return selectionEnd;
  let end = selectionEnd;
  if (end > 0 && source[end - 1] === "\n") end -= 1;
  if (end > 0 && source[end - 1] === "\r") end -= 1;
  return end;
}

function lineStartOffset(source, offset) {
  const before = source.slice(0, Math.max(0, offset));
  return Math.max(before.lastIndexOf("\n"), before.lastIndexOf("\r")) + 1;
}

function lineEndOffset(source, offset) {
  const safeOffset = Math.max(0, offset);
  const lf = source.indexOf("\n", safeOffset);
  const cr = source.indexOf("\r", safeOffset);
  if (lf === -1 && cr === -1) return source.length;
  if (lf === -1) return cr;
  if (cr === -1) return lf;
  return Math.min(lf, cr);
}

function splitTextLines(text) {
  const parts = String(text ?? "").split(/(\r\n|\r|\n)/);
  const lines = [];
  for (let index = 0; index < parts.length; index += 2) {
    if (index === parts.length - 1 && parts[index] === "") continue;
    lines.push({ text: parts[index] || "", ending: parts[index + 1] || "" });
  }
  return lines.length ? lines : [{ text: "", ending: "" }];
}

function joinTextLines(lines) {
  return lines.map((line) => `${line.text}${line.ending || ""}`).join("");
}

function preferredLineEnding(source) {
  const match = String(source ?? "").match(/\r\n|\r|\n/);
  return match ? match[0] : "\n";
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
  const memberItems = localMemberCompletionCandidates(prefix);
  if (memberItems.length) {
    return memberItems.slice(0, 9);
  }

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

function localMemberCompletionCandidates(prefix) {
  const context = memberCompletionContextFromPrefix(prefix);
  if (!context) return [];
  const workflowCatalog = state.syntaxCatalog || emptySyntaxCatalog();
  const groups = [
    {
      fields: argsFieldCompletionsFromSource(state.source),
      detail: "args field",
      matchesReceiver: (receiver) => receiver === "args"
    },
    {
      fields: schemaFieldsForBinding(schemaBindingFieldCompletionsFromSource(state.source), context.receiverCandidates),
      detail: "schema field",
      matchesReceiver: () => true
    },
    {
      fields: workflowFieldsForBinding(
        workflowBindingFieldCompletionsFromSource(state.source, workflowCatalog),
        context.receiverCandidates
      ),
      detail: "workflow field",
      matchesReceiver: () => true
    },
    {
      fields: workflowCatalog.httpResponseFields,
      detail: "HTTP response field",
      matchesReceiver: isResponseLikeReceiver
    },
    {
      fields: workflowCatalog.coverageResultFields,
      detail: "Coverage result field",
      matchesReceiver: isCoverageResultLikeReceiver
    },
    {
      fields: workflowCatalog.tableFields,
      detail: "Table field",
      matchesReceiver: isTableLikeReceiver
    },
    {
      fields: workflowCatalog.sampleTableFields,
      detail: "Sample table field",
      matchesReceiver: isSampleTableLikeReceiver
    },
    {
      fields: workflowCatalog.dbConnectionFields,
      detail: "DB connection field",
      matchesReceiver: isDbConnectionLikeReceiver
    },
    {
      fields: workflowCatalog.caseOutputTableFields,
      detail: "Case output table field",
      matchesReceiver: isCaseOutputTableLikeReceiver
    },
    {
      fields: workflowCatalog.caseRunResultTableFields,
      detail: "Native case run result field",
      matchesReceiver: isCaseRunResultTableLikeReceiver
    },
    {
      fields: workflowCatalog.caseResultCollectionTableFields,
      detail: "Case result collection field",
      matchesReceiver: isCaseResultCollectionLikeReceiver
    },
    {
      fields: workflowCatalog.caseTableFields,
      detail: "Case table field",
      matchesReceiver: isCaseTableLikeReceiver
    },
    {
      fields: workflowCatalog.modelFields,
      detail: "Model field",
      matchesReceiver: isModelLikeReceiver
    },
    {
      fields: workflowCatalog.predictionTableFields,
      detail: "Prediction table field",
      matchesReceiver: isPredictionTableLikeReceiver
    }
  ];
  const items = [];
  const seen = new Set();
  for (const group of groups) {
    if (!Array.isArray(group.fields) || !receiverMatchesContext(context, group.matchesReceiver)) continue;
    for (const item of memberCompletionItemsForFields(context, group.fields, group.detail)) {
      const key = `${item.kind}:${item.insert}`;
      if (seen.has(key)) continue;
      seen.add(key);
      items.push(item);
    }
  }
  return items;
}

function memberCompletionContextFromPrefix(prefix) {
  const match = /^((?:[A-Za-z_][A-Za-z0-9_]*\.)*[A-Za-z_][A-Za-z0-9_]*)\.([A-Za-z_][A-Za-z0-9_]*)?$/.exec(prefix || "");
  if (!match) return null;
  const receiver = match[1];
  return {
    receiver,
    receiverCandidates: receiverLookupCandidates(receiver),
    prefix: match[2] || ""
  };
}

function receiverLookupCandidates(receiver) {
  const normalized = String(receiver || "").trim();
  if (!normalized) return [];
  const candidates = [normalized];
  const lastSegment = normalized.split(".").filter(Boolean).pop();
  if (lastSegment && lastSegment !== normalized) {
    candidates.push(lastSegment);
  }
  return candidates;
}

function receiverMatchesContext(context, predicate) {
  return (context.receiverCandidates || [context.receiver]).some((receiver) => predicate(receiver));
}

function memberCompletionItemsForFields(context, fields, fallbackDetail) {
  const lower = context.prefix.toLowerCase();
  return fields
    .filter((field) => typeof field?.label === "string")
    .filter((field) => field.label.toLowerCase().startsWith(lower))
    .map((field) => ({
      label: field.label,
      insert: `${context.receiver}.${field.label}`,
      detail: field.detail || fallbackDetail,
      kind: "property"
    }));
}

function argsFieldCompletionsFromSource(source) {
  const body = firstBlockBodyFromSource(source, /\bargs\s*\{/g);
  if (!body) return [];
  return schemaFieldCompletionsFromBody(body).map((field) => ({
    ...field,
    detail: field.detail ? `args field: ${field.detail}` : "args field"
  }));
}

function schemaBindingFieldCompletionsFromSource(source) {
  const schemas = schemaFieldsFromSource(source);
  const bindings = promotedSchemaBindingsFromSource(source);
  const result = {};
  for (const [binding, schemaName] of Object.entries(bindings)) {
    const fields = schemas[schemaName];
    if (!Array.isArray(fields)) continue;
    result[binding] = fields.map((field) => ({
      ...field,
      detail: field.detail ? `${schemaName} field: ${field.detail}` : `${schemaName} field`
    }));
  }
  return result;
}

function schemaFieldsForBinding(schemaBindingFields, receiverCandidates) {
  return firstMappedFieldsForReceiver(schemaBindingFields, receiverCandidates);
}

function workflowFieldsForBinding(workflowBindingFields, receiverCandidates) {
  return firstMappedFieldsForReceiver(workflowBindingFields, receiverCandidates);
}

function firstMappedFieldsForReceiver(fieldMap, receiverCandidates) {
  if (!fieldMap || typeof fieldMap !== "object") return [];
  for (const receiver of receiverCandidates || []) {
    const fields = fieldMap[receiver];
    if (Array.isArray(fields)) return fields;
  }
  return [];
}

function workflowBindingFieldCompletionsFromSource(source, catalog) {
  const text = String(source ?? "");
  const normalizedCatalog = normalizeSyntaxCatalog(catalog);
  const result = {};
  const groups = [
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*http\s+(?:get|post|put|patch|head|request|fetch)\b/gm,
      fields: normalizedCatalog.httpResponseFields,
      detail: "HTTP response field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*promote\s+(?:csv|toml|json(?:\s+records)?)\b/gm,
      fields: normalizedCatalog.tableFields,
      detail: "Table field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?:filter|derive|sort|join|select)\b/gm,
      fields: normalizedCatalog.tableFields,
      detail: "Table field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*check\s+coverage\b/gm,
      fields: normalizedCatalog.coverageResultFields,
      detail: "Coverage result field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*sample\s+(?:lhs|latin[_-]hypercube|grid|random|uniform)\b/gm,
      fields: normalizedCatalog.sampleTableFields,
      detail: "Sample table field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*open\s+sqlite\b/gm,
      fields: normalizedCatalog.dbConnectionFields,
      detail: "DB connection field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*materialize\s+cases\b/gm,
      fields: normalizedCatalog.caseTableFields,
      detail: "Case table field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*apply\s+[A-Za-z_][A-Za-z0-9_.-]*\s+over\s+[A-Za-z_][A-Za-z0-9_.-]*\b/gm,
      fields: normalizedCatalog.caseOutputTableFields,
      detail: "Case output table field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*apply\s*\(\s*[A-Za-z_][A-Za-z0-9_.-]*\s*,\s*over\s*=\s*[A-Za-z_][A-Za-z0-9_.-]*\s*\)/gm,
      fields: normalizedCatalog.caseOutputTableFields,
      detail: "Case output table field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*apply\s+run_case\s+over\s+[A-Za-z_][A-Za-z0-9_.-]*\b/gm,
      fields: normalizedCatalog.caseRunResultTableFields,
      detail: "Native case run result field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*apply\s*\(\s*run_case\s*,\s*over\s*=\s*[A-Za-z_][A-Za-z0-9_.-]*\s*\)/gm,
      fields: normalizedCatalog.caseRunResultTableFields,
      detail: "Native case run result field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*collect\s+results\s+[A-Za-z_][A-Za-z0-9_.]*\b/gm,
      fields: normalizedCatalog.caseResultCollectionTableFields,
      detail: "Case result collection field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?:train\s+regression|regression_table|train_test_split|evaluate|model_card)\b/gm,
      fields: normalizedCatalog.modelFields,
      detail: "Model field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*predict\b/gm,
      fields: normalizedCatalog.predictionTableFields,
      detail: "Prediction table field"
    }
  ];
  for (const group of groups) {
    if (!Array.isArray(group.fields) || !group.fields.length) continue;
    group.pattern.lastIndex = 0;
    let match;
    while ((match = group.pattern.exec(text)) !== null) {
      result[match[1]] = workflowMemberCompletionFields(group.fields, group.detail);
    }
  }
  return result;
}

function workflowMemberCompletionFields(fields, fallbackDetail) {
  return fields
    .filter((field) => typeof field?.label === "string")
    .map((field) => ({
      ...field,
      detail: field.detail ? `${fallbackDetail}: ${field.detail}` : fallbackDetail,
      kind: "property"
    }));
}

function isResponseLikeReceiver(receiver) {
  const normalized = String(receiver || "").toLowerCase();
  return (
    normalized.includes("response") ||
    normalized.includes("http") ||
    normalized.includes("api") ||
    normalized.includes("network")
  );
}

function isTableLikeReceiver(receiver) {
  const normalized = String(receiver || "").toLowerCase();
  return normalized.includes("table") || normalized.includes("rows") || normalized.includes("records");
}

function isCoverageResultLikeReceiver(receiver) {
  const normalized = String(receiver || "").toLowerCase();
  return normalized === "coverage" || normalized.includes("coverage");
}

function isSampleTableLikeReceiver(receiver) {
  const normalized = String(receiver || "").toLowerCase();
  return (
    normalized.includes("sample") ||
    normalized.includes("design") ||
    normalized.includes("lhs")
  );
}

function isCaseOutputTableLikeReceiver(receiver) {
  const normalized = String(receiver || "").toLowerCase();
  return (
    !isCaseRunResultTableLikeReceiver(receiver) &&
    normalized.includes("case") &&
    (
      normalized.includes("input") ||
      normalized.includes("output") ||
      normalized.includes("planned") ||
      normalized.includes("rendered") ||
      normalized.includes("blocked") ||
      normalized.includes("manifest")
    )
  );
}

function isCaseRunResultTableLikeReceiver(receiver) {
  const normalized = String(receiver || "").toLowerCase();
  return (
    !normalized.includes("collection") &&
    (
      normalized === "case_runs" ||
      normalized.endsWith("_case_runs") ||
      normalized.includes("case_run_result")
    )
  );
}

function isCaseResultCollectionLikeReceiver(receiver) {
  const normalized = String(receiver || "").toLowerCase();
  return (
    !isCaseRunResultTableLikeReceiver(receiver) &&
    (
      normalized.includes("collection") ||
      (normalized.includes("case") && normalized.includes("result"))
    )
  );
}

function isCaseTableLikeReceiver(receiver) {
  const normalized = String(receiver || "").toLowerCase();
  return (
    !isCaseOutputTableLikeReceiver(receiver) &&
    !isCaseRunResultTableLikeReceiver(receiver) &&
    !isCaseResultCollectionLikeReceiver(receiver) &&
    (
      normalized === "case" ||
      normalized === "cases" ||
      normalized.includes("case_table") ||
      normalized.endsWith("_case") ||
      normalized.endsWith("_cases")
    )
  );
}

function isDbConnectionLikeReceiver(receiver) {
  const normalized = String(receiver || "").toLowerCase();
  return normalized.includes("db") || normalized.includes("database") || normalized.includes("sqlite");
}

function isModelLikeReceiver(receiver) {
  const normalized = String(receiver || "").toLowerCase();
  return normalized.includes("model") || normalized.includes("regression") || normalized.includes("training");
}

function isPredictionTableLikeReceiver(receiver) {
  const normalized = String(receiver || "").toLowerCase();
  return normalized.includes("prediction") || normalized.includes("predictions") || normalized.includes("forecast");
}

function schemaFieldsFromSource(source) {
  const text = String(source ?? "");
  const schemas = {};
  const pattern = /\bschema\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{/g;
  let match;
  while ((match = pattern.exec(text)) !== null) {
    const openIndex = text.indexOf("{", match.index);
    const closeIndex = blockCloseIndex(text, openIndex);
    if (openIndex < 0 || closeIndex < 0) continue;
    schemas[match[1]] = schemaFieldCompletionsFromBody(text.slice(openIndex + 1, closeIndex));
    pattern.lastIndex = closeIndex + 1;
  }
  return schemas;
}

function promotedSchemaBindingsFromSource(source) {
  const text = String(source ?? "");
  const bindings = {};
  const pattern = /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*promote\s+(?:csv|toml|json(?:\s+records)?)\b[^\n]*\bas\s+([A-Za-z_][A-Za-z0-9_]*)\b/gm;
  let match;
  while ((match = pattern.exec(text)) !== null) {
    bindings[match[1]] = match[2];
  }
  return bindings;
}

function schemaFieldCompletionsFromBody(body) {
  const fields = [];
  const seen = new Set();
  for (const line of String(body ?? "").split(/\r?\n/)) {
    const withoutComment = line.replace(/#.*/, "").replace(/\/\/.*/, "");
    const match = /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([^=]*)/.exec(withoutComment);
    if (!match || seen.has(match[1])) continue;
    seen.add(match[1]);
    fields.push({
      label: match[1],
      detail: match[2].trim(),
      kind: "property"
    });
  }
  return fields;
}

function firstBlockBodyFromSource(source, openerRegex) {
  const text = String(source ?? "");
  openerRegex.lastIndex = 0;
  const match = openerRegex.exec(text);
  if (!match) return "";
  const openIndex = text.indexOf("{", match.index);
  const closeIndex = blockCloseIndex(text, openIndex);
  if (openIndex < 0 || closeIndex < 0) return "";
  return text.slice(openIndex + 1, closeIndex);
}

function blockCloseIndex(text, openIndex) {
  if (openIndex < 0) return -1;
  let depth = 0;
  let inString = false;
  let escaped = false;
  for (let index = openIndex; index < text.length; index += 1) {
    const char = text[index];
    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (char === "\\") {
        escaped = true;
      } else if (char === "\"") {
        inString = false;
      }
      continue;
    }
    if (char === "\"") {
      inString = true;
      continue;
    }
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) return index;
    }
  }
  return -1;
}

function currentPrefix(editor) {
  const before = editor.value.slice(0, editor.selectionStart);
  const match = before.match(/[A-Za-z_][A-Za-z0-9_./-]*$/);
  return match ? match[0] : "";
}

function completionInsertEdit(item) {
  const fallback = typeof item.insert === "string" ? item.insert : String(item.label || "");
  if (typeof item.insertSnippet !== "string" || !item.insertSnippet) {
    return {
      text: fallback,
      selectionStart: fallback.length,
      selectionEnd: fallback.length
    };
  }
  const edit = snippetInsertEdit(item.insertSnippet);
  if (!edit.text) {
    return {
      text: fallback,
      selectionStart: fallback.length,
      selectionEnd: fallback.length
    };
  }
  return edit;
}

function snippetInsertEdit(snippet) {
  let text = "";
  let firstPlaceholder = null;
  for (let index = 0; index < snippet.length;) {
    const char = snippet[index];
    if (char === "\\" && index + 1 < snippet.length && ["$", "}", "\\"].includes(snippet[index + 1])) {
      text += snippet[index + 1];
      index += 2;
      continue;
    }
    if (char !== "$" || index + 1 >= snippet.length) {
      text += char;
      index += 1;
      continue;
    }
    const next = snippet[index + 1];
    if (next === "{") {
      const close = snippet.indexOf("}", index + 2);
      if (close === -1) {
        text += char;
        index += 1;
        continue;
      }
      const body = snippet.slice(index + 2, close);
      const placeholderMatch = /^(\d+)(?::([\s\S]*))?$/.exec(body);
      if (!placeholderMatch) {
        text += snippet.slice(index, close + 1);
        index = close + 1;
        continue;
      }
      const placeholderText = placeholderMatch[2] || "";
      if (!firstPlaceholder && placeholderMatch[1] !== "0") {
        firstPlaceholder = { start: text.length, end: text.length + placeholderText.length };
      }
      text += placeholderText;
      index = close + 1;
      continue;
    }
    if (/\d/.test(next)) {
      let digitEnd = index + 2;
      while (digitEnd < snippet.length && /\d/.test(snippet[digitEnd])) digitEnd += 1;
      if (!firstPlaceholder && snippet.slice(index + 1, digitEnd) !== "0") {
        firstPlaceholder = { start: text.length, end: text.length };
      }
      index = digitEnd;
      continue;
    }
    text += char;
    index += 1;
  }
  return {
    text,
    selectionStart: firstPlaceholder ? firstPlaceholder.start : text.length,
    selectionEnd: firstPlaceholder ? firstPlaceholder.end : text.length
  };
}

function insertCompletion(item) {
  const editor = byId("editor");
  if (!editor || !item) return;
  const edit = completionInsertEdit(item);
  const prefix = currentPrefix(editor);
  const start = editor.selectionStart - prefix.length;
  const end = editor.selectionEnd;
  const before = editor.value.slice(0, start);
  const after = editor.value.slice(end);
  editor.value = `${before}${edit.text}${after}`;
  editor.selectionStart = before.length + edit.selectionStart;
  editor.selectionEnd = before.length + edit.selectionEnd;
  state.source = editor.value;
  state.dirty = state.source !== state.savedSource;
  rememberCurrentTab();
  state.status = "Modified";
  const checkChanged = markCheckPending();
  renderTabLabels();
  updateEditorLineCount();
  updateCheckSummaryUi();
  updateEditorHighlight();
  updateCursorInsight();
  hideCompletions();
  if (checkChanged) refreshCheckPanels();
  scheduleLiveCheck();
  editor.focus();
}
function updateCursorInsight() {
  updateEditorBreadcrumbs();
  const target = byId("cursorInsight");
  if (!target) return;
  target.outerHTML = `<span id="cursorInsight" class="cursor-insight">${renderCursorInsight()}</span>`;
  bindCursorInsightActions();
  updateCaretHighlightSummary();
}

function updateEditorBreadcrumbs() {
  const target = byId("editorBreadcrumbs");
  if (!target) return;
  target.outerHTML = `<nav id="editorBreadcrumbs" class="editor-breadcrumbs" aria-label="Current file and symbol path">${renderEditorBreadcrumbs()}</nav>`;
  bindEditorBreadcrumbs();
}

function bindCursorInsightActions() {
  const target = byId("cursorInsight");
  if (!target) return;
  bindSourceTokenRangeButtons(target);
  bindSourceTokenCopyButtons(target);
  target.querySelectorAll("[data-show-highlight-panel]").forEach((button) => {
    button.onclick = () => {
      state.sideTab = "highlight";
      render();
    };
  });
  target.querySelectorAll("[data-go-to-definition]").forEach((button) => {
    button.onclick = () => void goToDefinitionAtCaret();
  });
  bindDocumentHighlightActions(target);
  bindInspectorTabButtons(target);
}

function bindDocumentHighlightActions(root) {
  root.querySelectorAll("[data-show-document-highlights]").forEach((button) => {
    button.onclick = () => void showDocumentHighlightsAtCaret();
  });
}

function bindWorkspaceReferenceButtons(root) {
  root.querySelectorAll("[data-workspace-reference-uri]").forEach((button) => {
    button.onclick = () => void openWorkspaceReferenceLocation({
      uri: button.dataset.workspaceReferenceUri,
      range: {
        start: {
          line: Number(button.dataset.workspaceReferenceLine),
          character: Number(button.dataset.workspaceReferenceCharacter)
        },
        end: {
          line: Number(button.dataset.workspaceReferenceEndLine),
          character: Number(button.dataset.workspaceReferenceEndCharacter)
        }
      }
    });
  });
}

function bindRenameActions(root) {
  root.querySelectorAll("[data-rename-symbol]").forEach((button) => {
    button.onclick = () => void startSemanticRename();
  });
}

function bindInspectorTabButtons(root) {
  root.querySelectorAll("[data-open-inspector-tab]").forEach((button) => {
    button.onclick = () => {
      const tab = button.dataset.openInspectorTab;
      if (!SIDE_TABS.some((item) => item.key === tab)) return;
      state.sideTab = tab;
      render();
    };
  });
}

function updateCaretHighlightSummary() {
  const target = byId("caretHighlightSummary");
  if (!target) return;
  const tokenCurrent = state.source === state.highlightSource;
  target.outerHTML = `<div id="caretHighlightSummary">${renderCaretHighlightSummary(currentCaretSemanticToken(), tokenCurrent)}</div>`;
  const nextTarget = byId("caretHighlightSummary");
  if (!nextTarget) return;
  bindSourceTokenRangeButtons(nextTarget);
  bindSourceTokenCopyButtons(nextTarget);
  bindHighlightTokenFilterButtons(nextTarget);
  bindDocumentHighlightActions(nextTarget);
  bindInspectorTabButtons(nextTarget);
  nextTarget.querySelectorAll("[data-show-highlight-panel]").forEach((button) => {
    button.onclick = () => {
      state.sideTab = "highlight";
      render();
    };
  });
}

function currentCaretSemanticToken() {
  const editor = byId("editor");
  if (!editor || state.source !== state.highlightSource) return null;
  if (String(editor.value ?? "") !== String(state.source ?? "")) return null;
  const position = editorCursorPosition(editor.value, editor.selectionStart ?? 0);
  const token = semanticTokenAtCaret(editor, position);
  const nearestTokens = token ? [] : semanticTokensNearCaret(editor, position, 3);
  return {
    position,
    token,
    nearestTokens,
    lineOverlaps: semanticTokenLineOverlaps(position.line),
    hover: token ? hoverForSemanticToken(token, position.line) : null
  };
}

function renderCursorInsight() {
  const editor = byId("editor");
  const source = editor?.value ?? state.source ?? "";
  const position = editorCursorPosition(source, editor?.selectionStart ?? 0);
  const token = editor ? semanticTokenAtCaret(editor, position) : null;
  const nearestToken = !token && editor && state.source === state.highlightSource
    ? semanticTokensNearCaret(editor, position, 1)[0]
    : null;
  const hover = token ? hoverForSemanticToken(token, position.line) : null;
  const lineOverlaps = editor && state.source === state.highlightSource ? semanticTokenLineOverlaps(position.line) : [];
  const bracket = editor ? editorBracketMatch(source, editor.selectionStart) : null;
  const parts = [`L${position.line + 1}:C${position.column + 1}`];
  if (state.source !== state.highlightSource) {
    parts.push(checkFreshnessLabel());
  } else if (token) {
    parts.push(tokenLabel(token));
    if (hover?.quantity_kind || hover?.quantityKind) {
      const quantity = hover.quantity_kind || hover.quantityKind;
      const unit = hover.display_unit || hover.displayUnit || "-";
      parts.push(`${quantity} [${unit}]`);
    }
  } else if (nearestToken) {
    parts.push(`near ${tokenLabel(nearestToken)}`);
  } else {
    parts.push("plain");
  }
  if (lineOverlaps.length) {
    parts.push(`overlaps ${lineOverlaps.length}`);
  }
  if (bracket?.matched) {
    parts.push(`${bracket.open}${bracket.close} match L${bracket.line + 1}:C${bracket.column + 1}`);
  } else if (bracket) {
    parts.push(`unmatched ${bracket.char}`);
  }
  const title = hover ? hoverTitle(hover) : parts.join(" / ");
  return `
    <span title="${escapeAttr(title)}">${escapeHtml(parts.join(" / "))}</span>
    ${token ? renderCursorInsightActions(token, "Select", hover, true) : nearestToken ? renderCursorInsightActions(nearestToken, "Select Nearby") : ""}
  `;
}

function renderCursorInsightActions(token, selectLabel = "Select", hover = null, showDefinition = false) {
  return `
    ${sourceTokenButton(token, selectLabel)}
    ${sourceTokenCopyButton(token, "text", "Copy")}
    ${showDefinition ? '<button class="link-button token-range-button" data-go-to-definition title="Go to definition (F12)">Definition</button>' : ""}
    ${showDefinition ? '<button class="link-button token-range-button" data-show-document-highlights title="Highlight semantic references in this file (Shift+F12)">References</button>' : ""}
    ${showDefinition ? '<button class="link-button token-range-button" data-rename-symbol title="Rename semantic symbol (F2)">Rename</button>' : ""}
    <button class="link-button token-range-button" data-show-highlight-panel title="Open Highlight panel">Highlight</button>
    ${renderInspectorTabButtons(inspectorTabsForSemanticToken(token, hover))}
  `;
}

function renderInspectorTabButtons(tabs) {
  return arrayOrEmpty(tabs).map((tab) => {
    const item = SIDE_TABS.find((candidate) => candidate.key === tab);
    if (!item) return "";
    return `<button class="link-button token-range-button" data-open-inspector-tab="${escapeAttr(item.key)}" title="Open ${escapeAttr(item.label)} panel">${escapeHtml(item.label)}</button>`;
  }).filter(Boolean).join(" ");
}

function inspectorTabsForSemanticToken(token, hover = null) {
  const modifiers = arrayOrEmpty(token?.modifiers).map((modifier) => String(modifier));
  const modifierText = modifiers.join(" ").toLowerCase();
  const kind = String(hover?.kind || "").toLowerCase();
  const quantity = String(hover?.quantity_kind || hover?.quantityKind || "").toLowerCase();
  const tokenText = semanticTokenText(token).toLowerCase();
  const detailText = [
    kind,
    quantity,
    hover?.name,
    hover?.detail,
    token?.type,
    tokenText,
    modifierText,
    semanticTokenSelectors(token).join(" ")
  ].map((value) => String(value || "").toLowerCase()).join(" ");
  const tabs = [];
  const add = (tab) => {
    if (!tabs.includes(tab)) tabs.push(tab);
  };

  if (detailText.includes("schema") || kind === "schema_field") add("schema");
  if (modifiers.includes("timeseries") || modifiers.includes("axis") || detailText.includes("timeseries") || detailText.includes("time axis")) add("time");
  if (modifiers.includes("validation") || kind.includes("validation")) add("checks");
  if (modifiers.includes("workflowStep")) add("workflow");
  if (modifiers.includes("workflowStep") && /case|materialize|collect|apply/.test(detailText)) add("case");
  if (modifiers.includes("sideEffect")) add("effects");
  if (modifiers.includes("external")) {
    add("effects");
    if (/http|network|cache|response|download|url/.test(detailText)) add("network");
  }
  if (modifiers.includes("cache") || /cache|cache_key|cachekey|offline_response/.test(detailText)) add("network");
  if (String(token?.type || "") === "namespace" || modifiers.includes("defaultLibrary") || modifiers.includes("imported") || modifiers.includes("internal") || modifiers.includes("planned") || /\beng\./.test(detailText)) add("modules");
  if (modifiers.includes("db") || quantity.includes("dbconnection") || /sqlite|database|db_/.test(detailText)) add("db");
  if (modifiers.includes("model") || kind.includes("model") || kind.includes("prediction")) add("model");
  if (modifiers.includes("report") || /report|plot|artifact/.test(detailText)) add("review");
  if (modifiers.includes("unit") || kind === "unit") add("units");
  if (modifiers.includes("quantity") || (quantity && quantity !== "-" && quantity !== "local")) add("units");
  if (["variable", "parameter", "property"].includes(String(token?.type || ""))) add("variables");

  return tabs.slice(0, 4);
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

function editorDefinitionRequest(editor) {
  if (!editor || !state.currentPath) return null;
  const position = editorCursorPosition(editor.value, editor.selectionStart ?? 0);
  return {
    path: state.currentPath,
    source: editor.value,
    line: position.line,
    character: position.column
  };
}

function currentDocumentHighlights() {
  const payload = state.documentHighlights || {};
  if (payload.path !== state.currentPath || payload.source !== state.source) return [];
  return arrayOrEmpty(payload.items);
}

function currentWorkspaceReferences() {
  const payload = state.workspaceReferences || {};
  if (!payload.path || !payload.source) return [];
  const origin = sameDefinitionPath(payload.path, state.currentPath)
    ? { source: state.source }
    : state.tabs.find((tab) => sameDefinitionPath(tab.path, payload.path));
  if (
    !origin
    || origin.source !== payload.source
    || !workspaceDocumentsAreCurrent(payload.documents, payload.path)
  ) return [];
  return arrayOrEmpty(payload.items);
}

function clearReferenceResults() {
  state.documentHighlights = { path: "", source: "", items: [] };
  state.workspaceReferences = {
    path: "",
    source: "",
    documents: [],
    label: "",
    items: [],
    notice: ""
  };
}

async function showDocumentHighlightsAtCaret() {
  const editor = byId("editor");
  const request = editorDefinitionRequest(editor);
  if (!request) return false;
  const selectedToken = semanticTokenAtCaret(editor, {
    line: request.line,
    column: request.character
  });
  const label = selectedToken ? semanticTokenText(selectedToken) : "";
  const documents = dirtyWorkspaceDocuments(request.path);
  const revision = ++documentHighlightRequestRevision;
  hideCompletions();
  setStatus("Finding references...");
  try {
    let workspaceError = "";
    const referencesPromise = call("ide_references", { ...request, documents }).catch((error) => {
      workspaceError = String(error);
      return [];
    });
    const [highlights, references] = await Promise.all([
      call("ide_document_highlights", request),
      referencesPromise
    ]);
    if (revision !== documentHighlightRequestRevision) return false;
    if (!bufferRequestIsCurrent(request)) {
      setStatus("References cancelled; buffer changed");
      return false;
    }
    if (!workspaceDocumentsAreCurrent(documents, request.path)) {
      setStatus("References cancelled; another modified buffer changed");
      return false;
    }
    const items = Array.isArray(highlights) ? highlights : [];
    const workspaceItems = Array.isArray(references) ? references : [];
    const notice = workspaceError
      ? "Workspace reference search was unavailable. Current-file references are shown."
      : "";
    state.documentHighlights = {
      path: request.path,
      source: request.source,
      items
    };
    state.workspaceReferences = {
      path: request.path,
      source: request.source,
      documents,
      label,
      items: workspaceItems,
      notice
    };
    state.sideTab = "highlight";
    const fileCount = new Set(workspaceItems.map((item) => item?.uri).filter(Boolean)).size;
    state.status = workspaceItems.length
      ? `References: ${workspaceItems.length} across ${fileCount} file${fileCount === 1 ? "" : "s"}`
      : items.length
        ? `References: ${items.length} in current file`
        : "No semantic references found";
    if (workspaceError) appendTerminal("error", workspaceError);
    render();
    return workspaceItems.length > 0 || items.length > 0;
  } catch (error) {
    if (revision !== documentHighlightRequestRevision) return false;
    const message = String(error);
    setStatus(message);
    appendTerminal("error", message);
    return false;
  }
}

async function openWorkspaceReferenceLocation(reference) {
  const range = workspaceReferenceRange(reference);
  const absolutePath = definitionPathFromUri(reference?.uri);
  if (!range || !absolutePath) {
    setStatus("Reference location is not a valid local source range");
    return false;
  }
  try {
    if (!await openDefinitionTarget(absolutePath)) {
      throw new Error(`Could not open reference file: ${absolutePath}`);
    }
    const editor = byId("editor");
    if (!editor) return false;
    selectEditorUtf16Range(editor, {
      line: range.start.line,
      character: range.start.character,
      endLine: range.end.line,
      endCharacter: range.end.character
    });
    syncEditorHighlightScroll();
    updateEditorFindStatus();
    updateCursorInsight();
    setStatus(`Reference: ${fileName(state.currentPath)}:${range.start.line + 1}`);
    return true;
  } catch (error) {
    const message = String(error);
    setStatus(message);
    appendTerminal("error", message);
    return false;
  }
}

async function startSemanticRename() {
  if (state.pendingQuickFix || state.pendingRename || state.pendingWorkspaceSymbols) return false;
  const editor = byId("editor");
  const request = editorDefinitionRequest(editor);
  if (!request) return false;
  rememberCurrentTab();
  const documents = dirtyWorkspaceDocuments(request.path);
  const revision = ++renameRequestRevision;
  hideCompletions();
  setStatus("Preparing rename...");
  try {
    const preparation = await call("ide_prepare_rename", { ...request, documents });
    if (revision !== renameRequestRevision) return false;
    if (!bufferRequestIsCurrent(request)) {
      setStatus("Rename cancelled; buffer changed");
      return false;
    }
    if (!workspaceDocumentsAreCurrent(documents, request.path)) {
      setStatus("Rename cancelled; another modified buffer changed");
      return false;
    }
    const range = workspaceReferenceRange(preparation);
    const placeholder = String(preparation?.placeholder || "");
    if (!range || !validRenameIdentifier(placeholder)) {
      throw new Error("Rename preparation did not return a valid EngLang symbol.");
    }
    selectEditorUtf16Range(editor, {
      line: range.start.line,
      character: range.start.character,
      endLine: range.end.line,
      endCharacter: range.end.character
    });
    openSemanticRenameDialog({ request, documents, range, placeholder, revision, busy: false });
    return true;
  } catch (error) {
    if (revision !== renameRequestRevision) return false;
    const message = String(error);
    setStatus(message);
    appendTerminal("error", message);
    return false;
  }
}

function openSemanticRenameDialog(pending) {
  byId("renameBackdrop")?.remove();
  state.pendingRename = pending;
  const backdrop = document.createElement("div");
  backdrop.id = "renameBackdrop";
  backdrop.className = "dialog-backdrop";
  backdrop.innerHTML = `
    <div class="unsaved-dialog rename-dialog" role="dialog" aria-modal="true" aria-labelledby="renameTitle">
      <h2 id="renameTitle">Rename Symbol</h2>
      <div class="unsaved-dialog-path" title="${escapeAttr(pending.request.path)}">${escapeHtml(pending.request.path)}</div>
      <label class="rename-dialog-label" for="renameNameInput">New name</label>
      <input id="renameNameInput" class="rename-dialog-input" value="${escapeAttr(pending.placeholder)}" autocomplete="off" spellcheck="false" />
      <div id="renameDialogError" class="rename-dialog-error" role="alert" hidden></div>
      <div class="unsaved-dialog-actions">
        <button id="renameCancelBtn">Cancel</button>
        <button id="renameApplyBtn" class="primary">Rename</button>
      </div>
    </div>
  `;
  document.body.appendChild(backdrop);
  syncDialogInert();
  const input = byId("renameNameInput");
  byId("renameCancelBtn").onclick = cancelSemanticRename;
  byId("renameApplyBtn").onclick = () => void submitSemanticRename();
  input.onkeydown = (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      event.stopPropagation();
      void submitSemanticRename();
    } else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      cancelSemanticRename();
    }
  };
  input.oninput = () => setRenameDialogError("");
  backdrop.onclick = (event) => {
    if (event.target === backdrop) cancelSemanticRename();
  };
  input.focus();
  input.setSelectionRange(0, input.value.length);
  setStatus(`Rename ${pending.placeholder}`);
}

function cancelSemanticRename() {
  const pending = state.pendingRename;
  if (!pending) return;
  renameRequestRevision += 1;
  byId("renameBackdrop")?.remove();
  state.pendingRename = null;
  syncDialogInert();
  setStatus("Rename cancelled");
  byId("editor")?.focus();
}

function closeSemanticRenameDialog(pending) {
  if (state.pendingRename !== pending) return;
  byId("renameBackdrop")?.remove();
  state.pendingRename = null;
  syncDialogInert();
}

function setRenameDialogBusy(busy) {
  const pending = state.pendingRename;
  if (!pending) return;
  pending.busy = busy;
  const input = byId("renameNameInput");
  const cancel = byId("renameCancelBtn");
  const apply = byId("renameApplyBtn");
  if (input) input.disabled = busy;
  if (cancel) cancel.disabled = false;
  if (apply) {
    apply.disabled = busy;
    apply.textContent = busy ? "Renaming..." : "Rename";
  }
}

function setRenameDialogError(message) {
  const target = byId("renameDialogError");
  if (!target) return;
  target.textContent = String(message || "");
  target.hidden = !message;
}

async function submitSemanticRename() {
  const pending = state.pendingRename;
  const input = byId("renameNameInput");
  if (!pending || pending.busy || !input) return false;
  const newName = String(input.value || "").trim();
  if (!validRenameIdentifier(newName)) {
    setRenameDialogError("Enter a valid EngLang identifier.");
    return false;
  }
  if (newName === pending.placeholder) {
    cancelSemanticRename();
    return false;
  }
  const documents = arrayOrEmpty(pending.documents);
  if (!workspaceDocumentsAreCurrent(documents, pending.request.path)) {
    setRenameDialogError("Rename preparation is stale because another modified buffer changed.");
    return false;
  }
  setRenameDialogBusy(true);
  setRenameDialogError("");
  setStatus(`Renaming ${pending.placeholder}...`);
  try {
    const payload = await call("ide_rename", { ...pending.request, newName, documents });
    if (state.pendingRename !== pending || pending.revision !== renameRequestRevision) return false;
    if (!bufferRequestIsCurrent(pending.request)) {
      throw new Error("Rename cancelled because the current buffer changed.");
    }
    if (!workspaceDocumentsAreCurrent(documents, pending.request.path)) {
      throw new Error("Rename cancelled because another modified buffer changed.");
    }
    const staged = await stageWorkspaceRename(pending, payload, newName, documents);
    if (state.pendingRename !== pending || pending.revision !== renameRequestRevision) return false;
    if (!bufferRequestIsCurrent(pending.request)) {
      throw new Error("Rename cancelled because the current buffer changed.");
    }
    if (!workspaceDocumentsAreCurrent(documents, pending.request.path)) {
      throw new Error("Rename cancelled because another modified buffer changed.");
    }
    commitWorkspaceRename(pending, staged, documents);
    closeSemanticRenameDialog(pending);
    clearReferenceResults();
    markCheckPending();
    state.status = `Renamed ${pending.placeholder} to ${newName}: ${staged.editCount} edit${staged.editCount === 1 ? "" : "s"} across ${staged.updates.length} file${staged.updates.length === 1 ? "" : "s"}`;
    render();
    const nextEditor = byId("editor");
    if (nextEditor && staged.focus) {
      nextEditor.focus();
      nextEditor.selectionStart = staged.focus.start;
      nextEditor.selectionEnd = staged.focus.end;
      syncEditorHighlightScroll();
      updateEditorFindStatus();
      updateCursorInsight();
    }
    scheduleLiveCheck();
    return true;
  } catch (error) {
    if (state.pendingRename !== pending) return false;
    const message = String(error);
    setRenameDialogBusy(false);
    setRenameDialogError(message);
    setStatus(message);
    appendTerminal("error", message);
    return false;
  }
}

function validRenameIdentifier(value) {
  return /^[A-Za-z_][A-Za-z0-9_]*$/.test(String(value || ""));
}

function workspaceRenamePlan(payload, newName, originPath = state.currentPath) {
  if (payload?.error) throw new Error(String(payload.error));
  const changes = payload?.changes;
  if (!changes || typeof changes !== "object" || Array.isArray(changes)) {
    throw new Error("Rename did not return a workspace edit.");
  }
  const targets = [];
  const pathKeys = new Set();
  let editCount = 0;
  let originIncluded = false;
  for (const [uri, rawEdits] of Object.entries(changes)) {
    const absolutePath = definitionPathFromUri(uri);
    const path = definitionWorkspacePath(absolutePath);
    const isOrigin = sameDefinitionPath(path, originPath);
    if (
      !absolutePath
      || (!definitionPathInsideWorkspace(absolutePath) && !isOrigin)
      || !/\.eng$/i.test(absolutePath)
    ) {
      throw new Error("Rename returned an edit outside the EngLang workspace.");
    }
    const pathKey = definitionPathKey(path);
    if (pathKeys.has(pathKey)) {
      throw new Error("Rename returned duplicate workspace file edits.");
    }
    if (isOrigin) originIncluded = true;
    pathKeys.add(pathKey);
    if (!Array.isArray(rawEdits) || !rawEdits.length) {
      throw new Error(`Rename returned no edits for ${path}.`);
    }
    const edits = rawEdits.map((edit) => {
      const range = workspaceReferenceRange(edit);
      if (!range || typeof edit?.newText !== "string" || edit.newText !== newName) {
        throw new Error(`Rename returned an invalid replacement for ${path}.`);
      }
      editCount += 1;
      if (editCount > 1000) {
        throw new Error("Rename exceeded the 1000-edit native IDE safety limit.");
      }
      return { range, newText: edit.newText };
    });
    targets.push({ uri, absolutePath, path, edits });
  }
  if (!targets.length || !editCount) throw new Error("Rename returned no workspace edits.");
  if (!originIncluded) throw new Error("Rename did not edit the selected EngLang file.");
  if (targets.length > 1 && targets.some((target) => !definitionPathInsideWorkspace(target.absolutePath))) {
    throw new Error("Rename cannot mix an external file with workspace edits.");
  }
  targets.sort((left, right) => left.path.localeCompare(right.path));
  return { targets, editCount };
}

function definitionPathInsideWorkspace(path) {
  const pathKey = definitionPathKey(path);
  const rootKey = definitionPathKey(state.root);
  return Boolean(rootKey && (pathKey === rootKey || pathKey.startsWith(`${rootKey}/`)));
}

async function stageWorkspaceRename(pending, payload, newName, documents = []) {
  const plan = workspaceRenamePlan(payload, newName, pending.request.path);
  const openDocuments = new Map(
    documents.map((document) => [definitionPathKey(document.path), document.source])
  );
  const updates = [];
  for (const target of plan.targets) {
    if (
      pending.revision !== undefined
      && (pending.revision !== renameRequestRevision || state.pendingRename !== pending)
    ) {
      throw new Error("Rename cancelled before all files were verified.");
    }
    let source;
    let savedSource;
    const openTab = state.tabs.find((tab) => sameWorkspaceFilePath(tab.path, target.path));
    if (sameDefinitionPath(target.path, pending.request.path)) {
      source = pending.request.source;
      savedSource = openTab ? tabSavedSource(openTab) : source;
    } else if (openDocuments.has(definitionPathKey(target.path))) {
      source = openDocuments.get(definitionPathKey(target.path));
      savedSource = openTab ? tabSavedSource(openTab) : source;
    } else if (openTab) {
      source = openTab.source;
      savedSource = tabSavedSource(openTab);
    } else {
      const file = await call("ide_open_file", { path: target.path });
      if (!file || !sameDefinitionPath(file.path, target.path) || typeof file.source !== "string") {
        throw new Error(`Could not verify rename source ${target.path}.`);
      }
      source = file.source;
      savedSource = file.source;
    }
    const applied = applyWorkspaceTextEdits(source, target.edits, pending.placeholder);
    updates.push({
      ...target,
      source: applied.source,
      savedSource,
      mappedEdits: applied.mappedEdits
    });
  }
  const origin = updates.find((update) => sameDefinitionPath(update.path, pending.request.path));
  const focusEdit = origin?.mappedEdits.find((edit) => sameWorkspaceRange(edit.range, pending.range));
  if (!origin || !focusEdit) {
    throw new Error("Rename did not include the selected symbol edit.");
  }
  return {
    updates,
    editCount: plan.editCount,
    focus: { start: focusEdit.newStart, end: focusEdit.newEnd }
  };
}

function applyWorkspaceTextEdits(source, edits, expectedText) {
  const normalized = edits.map((edit) => {
    const start = sourceUtf16Offset(source, edit.range.start);
    const end = sourceUtf16Offset(source, edit.range.end);
    if (start === null || end === null || end <= start) {
      throw new Error("Rename returned a source range outside the current file.");
    }
    if (source.slice(start, end) !== expectedText) {
      throw new Error("Rename source changed before all edits could be verified.");
    }
    return { ...edit, start, end };
  }).sort((left, right) => left.start - right.start || left.end - right.end);
  for (let index = 1; index < normalized.length; index += 1) {
    if (normalized[index].start < normalized[index - 1].end) {
      throw new Error("Rename returned overlapping source edits.");
    }
  }
  let cursor = 0;
  let changed = "";
  const mappedEdits = [];
  for (const edit of normalized) {
    changed += source.slice(cursor, edit.start);
    const newStart = changed.length;
    changed += edit.newText;
    mappedEdits.push({
      range: edit.range,
      newStart,
      newEnd: changed.length
    });
    cursor = edit.end;
  }
  changed += source.slice(cursor);
  return { source: changed, mappedEdits };
}

function sourceUtf16Offset(source, position) {
  const line = Number(position?.line);
  const character = Number(position?.character);
  if (!Number.isInteger(line) || !Number.isInteger(character) || line < 0 || character < 0) {
    return null;
  }
  const text = String(source || "");
  const newlinePattern = /\r\n|\r|\n/g;
  let start = 0;
  for (let lineIndex = 0; lineIndex < line; lineIndex += 1) {
    const match = newlinePattern.exec(text);
    if (!match) return null;
    start = match.index + match[0].length;
  }
  const nextNewline = newlinePattern.exec(text);
  const end = nextNewline ? nextNewline.index : text.length;
  if (character > end - start) return null;
  return start + character;
}

function sameWorkspaceRange(left, right) {
  return left?.start?.line === right?.start?.line
    && left?.start?.character === right?.start?.character
    && left?.end?.line === right?.end?.line
    && left?.end?.character === right?.end?.character;
}

function commitWorkspaceRename(pending, staged, documents = []) {
  rememberCurrentTab();
  if (!bufferRequestIsCurrent(pending.request)) {
    throw new Error("Rename cancelled because the current buffer changed.");
  }
  if (!workspaceDocumentsAreCurrent(documents, pending.request.path)) {
    throw new Error("Rename cancelled because another modified buffer changed.");
  }
  for (const update of staged.updates) {
    const tab = state.tabs.find((candidate) => sameWorkspaceFilePath(candidate.path, update.path));
    if (tab) {
      if (typeof tab.savedSource !== "string") tab.savedSource = update.savedSource;
      tab.source = update.source;
      tab.dirty = tab.source !== tab.savedSource;
    } else {
      state.tabs.push({
        path: update.path,
        source: update.source,
        savedSource: update.savedSource,
        dirty: update.source !== update.savedSource
      });
    }
  }
  const current = staged.updates.find((update) =>
    sameDefinitionPath(update.path, pending.request.path)
  );
  const currentTab = state.tabs.find((tab) => sameWorkspaceFilePath(tab.path, pending.request.path));
  state.source = current.source;
  state.savedSource = tabSavedSource(currentTab);
  state.dirty = state.source !== state.savedSource;
}

async function goToDefinitionAtCaret() {
  const editor = byId("editor");
  const request = editorDefinitionRequest(editor);
  if (!request) return false;
  rememberCurrentTab();
  const documents = dirtyWorkspaceDocuments(request.path);
  const revision = ++definitionRequestRevision;
  hideCompletions();
  setStatus("Finding definition...");
  try {
    const target = await call("ide_definition", { ...request, documents });
    if (revision !== definitionRequestRevision) return false;
    if (!bufferRequestIsCurrent(request)) {
      setStatus("Definition cancelled; buffer changed");
      return false;
    }
    if (!workspaceDocumentsAreCurrent(documents, request.path)) {
      setStatus("Definition cancelled; another modified buffer changed");
      return false;
    }
    if (!target?.uri || !target?.range) {
      setStatus("No definition found");
      return false;
    }
    const absolutePath = definitionPathFromUri(target.uri);
    if (!absolutePath) throw new Error("Definition location was not a local file.");
    if (!await openDefinitionTarget(absolutePath)) {
      throw new Error(`Could not open definition file: ${absolutePath}`);
    }
    if (revision !== definitionRequestRevision) return false;
    const nextEditor = byId("editor");
    if (!nextEditor) return false;
    const start = target.range.start || {};
    const end = target.range.end || start;
    selectEditorUtf16Range(nextEditor, {
      line: start.line,
      character: start.character,
      endLine: end.line,
      endCharacter: end.character
    });
    syncEditorHighlightScroll();
    updateEditorFindStatus();
    updateCursorInsight();
    setStatus(`Definition: ${fileName(state.currentPath)}:${Number(start.line || 0) + 1}`);
    return true;
  } catch (error) {
    if (revision !== definitionRequestRevision) return false;
    const message = String(error);
    setStatus(message);
    appendTerminal("error", message);
    return false;
  }
}

async function openDefinitionTarget(absolutePath) {
  const targetPath = definitionWorkspacePath(absolutePath);
  if (sameDefinitionPath(targetPath, state.currentPath)) return true;
  const existing = state.tabs.find((tab) => sameDefinitionPath(tab.path, targetPath));
  if (existing) await switchTab(existing.path);
  else await openFile(absolutePath);
  return sameDefinitionPath(state.currentPath, targetPath);
}

function definitionPathFromUri(uri) {
  try {
    const parsed = new URL(String(uri || ""));
    if (parsed.protocol !== "file:") return "";
    let path = decodeURIComponent(parsed.pathname || "");
    if (/^\/[A-Za-z]:\//.test(path)) path = path.slice(1);
    return normalizePath(path);
  } catch {
    return "";
  }
}

function definitionWorkspacePath(path) {
  const normalized = normalizePath(path);
  const root = normalizePath(state.root);
  const pathKey = definitionPathKey(normalized);
  const rootKey = definitionPathKey(root);
  if (!rootKey || pathKey === rootKey) return pathKey === rootKey ? "." : normalized;
  if (pathKey.startsWith(`${rootKey}/`)) return normalized.slice(root.length + 1);
  return normalized;
}

function definitionPathKey(path) {
  const normalized = normalizePath(path);
  const root = normalizePath(state.root);
  return /^[A-Za-z]:\//.test(normalized) || /^[A-Za-z]:\//.test(root)
    ? normalized.toLowerCase()
    : normalized;
}

function sameDefinitionPath(left, right) {
  return definitionPathKey(left) === definitionPathKey(right);
}

function editorBracketMatch(source, offset) {
  const bracket = editorBracketAtCaret(source, offset);
  if (!bracket) return null;
  const matchOffset = matchingBracketOffset(source, bracket.offset, bracket.char);
  if (matchOffset < 0) {
    return { ...bracket, matched: false };
  }
  const position = editorCursorPosition(source, matchOffset);
  return {
    ...bracket,
    matched: true,
    line: position.line,
    column: position.column
  };
}

function editorBracketAtCaret(source, offset) {
  const safeOffset = Math.max(0, Math.min(Number(offset) || 0, source.length));
  for (const candidateOffset of [safeOffset, safeOffset - 1]) {
    if (candidateOffset < 0 || candidateOffset >= source.length) continue;
    const char = source[candidateOffset];
    if (!isEditorBracket(char)) continue;
    return {
      char,
      offset: candidateOffset,
      open: EDITOR_PAIR_OPEN[char] || char,
      close: EDITOR_PAIR_CLOSE[char] || char
    };
  }
  return null;
}

function isEditorBracket(char) {
  return char === "{" || char === "}" || char === "[" || char === "]" || char === "(" || char === ")";
}

function matchingBracketOffset(source, offset, char) {
  if (EDITOR_PAIR_CLOSE[char] && char !== "\"") {
    return scanBracketForward(source, offset, char, EDITOR_PAIR_CLOSE[char]);
  }
  if (EDITOR_PAIR_OPEN[char] && char !== "\"") {
    return scanBracketBackward(source, offset, char, EDITOR_PAIR_OPEN[char]);
  }
  return -1;
}

function scanBracketForward(source, offset, open, close) {
  let depth = 0;
  for (let index = offset; index < source.length; index += 1) {
    const char = source[index];
    if (char === open) depth += 1;
    if (char === close) depth -= 1;
    if (depth === 0) return index;
  }
  return -1;
}

function scanBracketBackward(source, offset, close, open) {
  let depth = 0;
  for (let index = offset; index >= 0; index -= 1) {
    const char = source[index];
    if (char === close) depth += 1;
    if (char === open) depth -= 1;
    if (depth === 0) return index;
  }
  return -1;
}

function semanticTokenLineOverlaps(lineIndex) {
  if (state.source !== state.highlightSource) return [];
  const tokens = semanticTokensByLine(semanticTokenPayload().tokens || []).get(lineIndex) || [];
  return semanticTokenOverlaps(tokens);
}

function semanticTokenAtCaret(editor, position) {
  if (state.source !== state.highlightSource) return null;
  const line = editor.value.split(/\r\n|\r|\n/)[position.line] || "";
  const columnByte = codeUnitToByteOffset(line, position.column);
  const tokens = semanticTokensByLine(semanticTokenPayload().tokens || []).get(position.line) || [];
  return tokens.find((token) => {
    const start = Number(token.start ?? 0);
    const end = start + Number(token.length ?? 0);
    return columnByte >= start && columnByte < end;
  }) || null;
}

function semanticTokensNearCaret(editor, position, limit = 3) {
  if (state.source !== state.highlightSource) return [];
  const line = editor.value.split(/\r\n|\r|\n/)[position.line] || "";
  const columnByte = codeUnitToByteOffset(line, position.column);
  const tokens = semanticTokensByLine(semanticTokenPayload().tokens || []).get(position.line) || [];
  return tokens
    .map((token) => ({
      ...token,
      caretDistance: semanticTokenCaretDistance(token, columnByte)
    }))
    .sort((left, right) =>
      left.caretDistance - right.caretDistance || Number(left.start ?? 0) - Number(right.start ?? 0)
    )
    .slice(0, limit);
}

function semanticTokenCaretDistance(token, columnByte) {
  const start = Number(token.start ?? 0);
  const length = Number(token.length ?? 0);
  if (!Number.isFinite(start) || !Number.isFinite(length) || length <= 0) {
    return Number.MAX_SAFE_INTEGER;
  }
  const end = start + length;
  if (columnByte >= start && columnByte < end) {
    return 0;
  }
  return columnByte < start ? start - columnByte : columnByte - end;
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

function hoverKindLabel(kind) {
  const text = String(kind ?? "").trim().toLowerCase();
  if (!text) {
    return "";
  }
  return HOVER_KIND_LABELS[text] ?? text
    .split(/[_-]+/)
    .filter((part) => part.length > 0)
    .map((part) => hoverKindWordLabel(part))
    .join(" ");
}

function hoverStatusLabel(status) {
  const text = String(status ?? "").trim().toLowerCase();
  if (!text) {
    return "";
  }
  return text
    .split(/[_-]+/)
    .filter((part) => part.length > 0)
    .map((part, index) => hoverStatusWordLabel(part, index))
    .join(" ");
}

function hoverStatusWordLabel(part, index) {
  if (["api", "db", "http", "jit", "lsp", "sha", "ttl"].includes(part)) {
    return part.toUpperCase();
  }
  return index === 0 ? hoverKindWordLabel(part) : part;
}

function hoverKindWordLabel(part) {
  if (part === "db") {
    return "DB";
  }
  if (part === "http") {
    return "HTTP";
  }
  return part.charAt(0).toUpperCase() + part.slice(1);
}

function hoverTitle(hover) {
  return [
    hover.name,
    hoverKindLabel(hover.kind),
    hover.detail,
    hover.quantity_kind || hover.quantityKind,
    hover.display_unit || hover.displayUnit,
    hoverStatusLabel(hover.status)
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
  const lines = String(state.source ?? "").split(/\r\n|\r|\n/);
  if (state.source !== state.highlightSource) {
    return lines.map(renderLexicalHighlightedLine).join("\n") || "\n";
  }
  const tokensByLine = semanticTokensByLine(semanticTokenPayload().tokens || []);
  return lines.map((line, index) => renderHighlightedLine(line, tokensByLine.get(index) || [], index)).join("\n") || "\n";
}

function renderHighlightedLine(line, tokens, lineIndex) {
  if (!tokens.length) return renderLexicalHighlightedLine(line);
  const ranges = tokens
    .map((token) => semanticTokenRange(line, token))
    .filter(Boolean)
    .sort((left, right) => left.start - right.start || right.end - left.end);
  let cursor = 0;
  let html = "";
  for (const range of ranges) {
    if (range.start < cursor || range.end <= range.start) continue;
    html += renderLexicalHighlightedLine(line.slice(cursor, range.start));
    const referenceKind = documentHighlightKindForToken(range.token, lineIndex);
    const referenceClass = referenceKind === 3
      ? " hl-reference hl-reference-write"
      : referenceKind === 2
        ? " hl-reference hl-reference-read"
        : referenceKind === 1
          ? " hl-reference"
          : "";
    html += `<span class="${escapeAttr(`${semanticTokenClass(range.token)}${referenceClass}`)}">${escapeHtml(line.slice(range.start, range.end))}</span>`;
    cursor = range.end;
  }
  html += renderLexicalHighlightedLine(line.slice(cursor));
  return html;
}

function documentHighlightKindForToken(token, lineIndex) {
  const line = Number(token?.line ?? lineIndex);
  const start = Number(token?.start);
  const length = Number(token?.length);
  const highlight = currentDocumentHighlights().find((item) => {
    const candidate = documentHighlightToken(item);
    return candidate
      && candidate.line === line
      && candidate.start === start
      && candidate.length === length;
  });
  return Number(highlight?.kind || 0);
}

function renderLexicalHighlightedLine(line) {
  let index = 0;
  let html = "";
  while (index < line.length) {
    const rest = line.slice(index);
    if (rest.startsWith("///")) {
      html += lexicalSpan("hl-doc-comment", rest);
      break;
    }
    if (rest.startsWith("//") || rest.startsWith("#")) {
      html += lexicalSpan("hl-comment", rest);
      break;
    }
    if (rest[0] === "\"") {
      const end = scanStringEnd(line, index);
      html += renderLexicalString(line.slice(index, end));
      index = end;
      continue;
    }
    const moduleMatch = /^eng(?:\.[A-Za-z_][A-Za-z0-9_]*)+/.exec(rest);
    if (moduleMatch) {
      html += lexicalSpan("hl-namespace", moduleMatch[0]);
      index += moduleMatch[0].length;
      continue;
    }
    const numberMatch = /^[0-9]+(?:\.[0-9]+)?/.exec(rest);
    if (numberMatch) {
      html += lexicalSpan("hl-number", numberMatch[0]);
      index += numberMatch[0].length;
      const unitRest = line.slice(index);
      const unitMatch = /^(\s+)(.+)$/.exec(unitRest);
      if (unitMatch) {
        const unit = state.lexicalCatalog.unitPattern?.exec(unitMatch[2]);
        if (unit) {
          html += escapeHtml(unitMatch[1]);
          html += lexicalSpan("hl-mod-unit", unit[0]);
          index += unitMatch[1].length + unit[0].length;
        }
      }
      continue;
    }
    const unitMatch = state.lexicalCatalog.unitPattern?.exec(rest);
    if (unitMatch) {
      html += lexicalSpan("hl-mod-unit", unitMatch[0]);
      index += unitMatch[0].length;
      continue;
    }
    const wordMatch = /^[A-Za-z_][A-Za-z0-9_]*(?:-[A-Za-z0-9_]+)?/.exec(rest);
    if (wordMatch) {
      const word = wordMatch[0];
      const cssClass = lexicalClassForWord(word, line, index);
      html += cssClass ? lexicalSpan(cssClass, word) : escapeHtml(word);
      index += word.length;
      continue;
    }
    const symbolMatch = /^(?:->|==|!=|>=|<=|=|\+|-|\*|\/|>|<)/.exec(rest);
    if (symbolMatch) {
      html += lexicalSpan("hl-operator", symbolMatch[0]);
      index += symbolMatch[0].length;
      continue;
    }
    const punctuationMatch = /^[{}[\](),:.]/.exec(rest);
    if (punctuationMatch) {
      html += lexicalSpan("hl-punctuation", punctuationMatch[0]);
      index += punctuationMatch[0].length;
      continue;
    }
    html += escapeHtml(rest[0]);
    index += 1;
  }
  return html;
}

function scanStringEnd(line, start) {
  for (let index = start + 1; index < line.length; index += 1) {
    if (line[index] === "\\") {
      index += 1;
      continue;
    }
    if (line[index] === "\"") {
      return index + 1;
    }
  }
  return line.length;
}

function renderLexicalString(text) {
  let index = 0;
  let html = "";
  while (index < text.length) {
    if (text[index] === "\\") {
      const next = Math.min(index + 2, text.length);
      html += lexicalSpan("hl-string", text.slice(index, next));
      index = next;
      continue;
    }
    if (text[index] === "{") {
      const end = scanInterpolationEnd(text, index);
      if (end > index) {
        html += lexicalSpan("hl-interpolation", text[index]);
        html += renderLexicalInterpolation(text.slice(index + 1, end));
        html += lexicalSpan("hl-interpolation", text[end]);
        index = end + 1;
        continue;
      }
    }
    const next = nextStringSpecial(text, index);
    html += lexicalSpan("hl-string", text.slice(index, next));
    index = next;
  }
  return html;
}

function scanInterpolationEnd(text, start) {
  for (let index = start + 1; index < text.length; index += 1) {
    if (text[index] === "\\") {
      index += 1;
      continue;
    }
    if (text[index] === "}") {
      return index;
    }
    if (text[index] === "\"") {
      index = scanStringEnd(text, index) - 1;
    }
  }
  return -1;
}

function nextStringSpecial(text, start) {
  for (let index = start + 1; index < text.length; index += 1) {
    if (text[index] === "\\" || text[index] === "{") {
      return index;
    }
  }
  return text.length;
}

function renderLexicalInterpolation(text) {
  return renderLexicalHighlightedLine(text);
}

function lexicalClassForWord(word, line, index) {
  if (line[index - 1] === ".") return "hl-property";
  const lexical = state.lexicalCatalog || buildLexicalCatalog(emptySyntaxCatalog());
  if (lexical.workflowStatusLiterals?.has(word) && isWorkflowStatusLiteralContext(line, index)) {
    return "hl-keyword hl-mod-workflowStep";
  }
  const workflowBuiltinClass = lexicalWorkflowBuiltinClass(word, line, index, lexical);
  if (workflowBuiltinClass) return workflowBuiltinClass;
  if (lexical.constants.has(word)) return "hl-constant";
  if (lexical.operatorWords.has(word)) return "hl-operator";
  const keywordClass = lexicalKeywordGroupClass(word, lexical);
  if (keywordClass) return keywordClass;
  if (lexical.workflowBuiltins?.has(word)) return "hl-keyword hl-mod-workflowStep";
  if (lexical.keywords.has(word)) return "hl-keyword";
  if (lexical.workflowOptions.has(word)) return "hl-property";
  if (lexical.publicTypes.has(word)) return "hl-type";
  if (lexical.quantities.has(word)) return "hl-mod-quantity";
  return lexicalCompletionClass(word);
}

function isWorkflowStatusLiteralContext(line, index) {
  return /\bstatus\s*(?:=|==|!=)\s*$/.test(String(line || "").slice(0, index));
}

function lexicalKeywordGroupClass(word, lexical) {
  const groups = lexical?.keywordGroups || {};
  for (const group of LEXICAL_KEYWORD_GROUP_ORDER) {
    if (groups[group]?.has(word)) {
      return LEXICAL_KEYWORD_GROUP_CLASSES[group] || "hl-keyword";
    }
  }
  return "";
}

function lexicalWorkflowBuiltinClass(word, line, index, lexical) {
  const suffix = String(line || "").slice(index + word.length);
  const isCall = /^\s*\(/.test(suffix);
  const isCanonicalSecretEnv = word === "secret" && /^\s+env\s*\(/.test(suffix);
  const isWorkflowStepValue = word === "run_case";
  if (!isCall && !isCanonicalSecretEnv && !isWorkflowStepValue) return "";

  const groups = lexical?.workflowBuiltinGroups || {};
  for (const group of LEXICAL_WORKFLOW_BUILTIN_GROUP_ORDER) {
    if (groups[group]?.has(word)) {
      return LEXICAL_WORKFLOW_BUILTIN_GROUP_CLASSES[group] || "hl-function";
    }
  }
  return isCall && lexical?.workflowBuiltins?.has(word) ? "hl-function" : "";
}

function lexicalCompletionClass(word) {
  for (const item of state.completions || []) {
    const label = String(item.label || "").replace(/\(\.\.\.\)$/, "");
    if (label !== word) continue;
    switch (item.kind) {
      case "class":
        return "hl-type";
      case "function":
        return "hl-function";
      case "method":
        return "hl-method";
      case "property":
      case "field":
        return "hl-property";
      case "variable":
        return "hl-variable";
      case "constant":
      case "value":
        return "hl-constant";
      case "operator":
        return "hl-operator";
      case "keyword":
      case "snippet":
        return "hl-keyword";
      case "unit":
        return "hl-mod-unit";
      case "stdlib":
      case "module":
        return "hl-namespace";
      default:
        return "";
    }
  }
  return "";
}

function lexicalSpan(cssClass, text) {
  return `<span class="hl-token ${escapeAttr(cssClass)}">${escapeHtml(text)}</span>`;
}

function semanticTokenOverlaps(tokens) {
  const lines = String(state.highlightSource || "").split(/\r\n|\r|\n/);
  const overlaps = [];
  for (const [lineIndex, lineTokens] of semanticTokensByLine(tokens).entries()) {
    const line = lines[lineIndex] || "";
    const ranges = lineTokens
      .map((token) => semanticTokenRange(line, token))
      .filter((range) => range && range.end > range.start)
      .sort((left, right) => left.start - right.start || left.end - right.end);
    let previous = null;
    for (const range of ranges) {
      if (previous && range.start < previous.end) {
        const start = Math.max(previous.start, range.start);
        const end = Math.min(previous.end, range.end);
        overlaps.push({
          line: lineIndex + 1,
          start,
          end,
          text: line.slice(start, end),
          left: previous.token,
          right: range.token
        });
        if (range.end > previous.end) {
          previous = range;
        }
        continue;
      }
      previous = range;
    }
  }
  return overlaps;
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
  const start = Number(token.start ?? 0);
  const length = Number(token.length ?? 0);
  if (!Number.isFinite(start) || !Number.isFinite(length) || length <= 0) {
    return null;
  }
  const startColumn = Math.min(line.length, Math.max(0, Math.trunc(start)));
  const endColumn = Math.min(line.length, Math.max(startColumn, Math.trunc(start + length)));
  return { start: startColumn, end: endColumn, token };
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

function semanticTokenSelectors(token) {
  const type = String(token?.type || "").trim();
  if (!type) return [];
  const selectors = [];
  for (const modifier of arrayOrEmpty(token?.modifiers)) {
    const detail = String(modifier || "").trim();
    if (detail) {
      selectors.push(`${type}.${detail}`);
    }
  }
  selectors.push(type);
  return [...new Set(selectors)];
}

function semanticTokenSelectorCells(token) {
  const selectors = semanticTokenSelectors(token);
  return selectors.length
    ? selectors.map((selector) => highlightFilterChip(selector, selector, "selector", `Filter selector ${selector}`)).join(" ")
    : "-";
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
    ...semanticTokenSelectors(token),
    Number.isFinite(line) && line > 0 ? `L${line}` : "",
    Number.isFinite(line) && line > 0 ? `line:${line}` : "",
    token?.start,
    token?.length
  ].map((part) => String(part ?? "").toLowerCase()).join(" ");
}

function semanticTokenTextCounts(tokens) {
  const counts = new Map();
  for (const token of arrayOrEmpty(tokens)) {
    const key = normalizedCatalogWord(semanticTokenText(token));
    if (!key || key === "-") continue;
    counts.set(key, (counts.get(key) || 0) + 1);
  }
  addSemanticTokenPhraseCounts(counts, tokens);
  return counts;
}

function addSemanticTokenPhraseCounts(counts, tokens) {
  const lines = String(state.highlightSource || "").split(/\r\n|\r|\n/);
  const tokensByLine = new Map();
  for (const token of arrayOrEmpty(tokens)) {
    const lineIndex = Number(token?.line);
    if (!Number.isFinite(lineIndex) || lineIndex < 0) continue;
    const sourceLine = lines[lineIndex] || "";
    const range = semanticTokenRange(sourceLine, token);
    if (!range) continue;
    const key = normalizedCatalogWord(sourceLine.slice(range.start, range.end));
    if (!key || key === "-" || /\s/.test(key)) continue;
    if (!tokensByLine.has(lineIndex)) tokensByLine.set(lineIndex, []);
    tokensByLine.get(lineIndex).push({ key, start: range.start, end: range.end });
  }
  for (const [lineIndex, ranges] of tokensByLine.entries()) {
    const sourceLine = lines[lineIndex] || "";
    const ordered = ranges.sort((left, right) => left.start - right.start || left.end - right.end);
    for (let index = 0; index < ordered.length; index += 1) {
      let phrase = ordered[index].key;
      let currentEnd = ordered[index].end;
      for (let nextIndex = index + 1; nextIndex < Math.min(index + 4, ordered.length); nextIndex += 1) {
        const next = ordered[nextIndex];
        const gap = sourceLine.slice(currentEnd, next.start);
        if (!/^\s+$/.test(gap)) break;
        phrase = `${phrase} ${next.key}`;
        counts.set(phrase, (counts.get(phrase) || 0) + 1);
        currentEnd = next.end;
      }
    }
  }
}

function sourceCatalogWords(words, options = {}) {
  const source = String(state.highlightSource || "");
  if (!source) return [];
  return uniqueStrings(words)
    .filter((word) => sourceContainsCatalogWord(source, word, options))
    .sort((left, right) => left.localeCompare(right));
}

function sourceContainsCatalogWord(source, word, options = {}) {
  const value = String(word || "");
  if (!value) return false;
  let index = source.indexOf(value);
  while (index >= 0) {
    if (catalogWordBoundaryOk(source, index, value.length, options)) return true;
    index = source.indexOf(value, index + value.length);
  }
  return false;
}

function catalogWordBoundaryOk(source, index, length, options = {}) {
  const left = index > 0 ? source[index - 1] : "";
  const right = index + length < source.length ? source[index + length] : "";
  const leftOk = !isCatalogWordChar(left) || (options.allowNumericPrefix && /[0-9.]/.test(left));
  const rightOk = !isCatalogWordChar(right);
  return leftOk && rightOk;
}

function isCatalogWordChar(char) {
  return /[A-Za-z0-9_-]/.test(String(char || ""));
}

function normalizedCatalogWord(value) {
  return String(value || "").trim().toLowerCase();
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

function statusLabel(status) {
  switch (String(status ?? "-")) {
    case "algebraic_only_preview":
      return "algebraic-only preview";
    case "algebraic_split_preview":
      return "algebraic split preview";
    case "component_local_signal_resolved":
      return "component-local signal resolved";
    case "behavior_graph_not_integrated":
      return "behavior graph not connected to this language-level solve";
    case "behavior_not_integrated":
      return "behavior variable not connected to this language-level solve";
    case "dae_split_deferred":
      return "DAE split deferred";
    case "delay_call_runtime_buffer_pending_integration":
      return "delay runtime buffer not connected to this language-level solve";
    case "delay_relationship_metadata_only":
      return "delay relationship metadata";
    case "external_behavior_contract_metadata":
      return "external behavior contract metadata";
    case "external_behavior_wrapper_pending_integration":
      return "external behavior adapter not connected to this language-level solve";
    case "external_output_typed_identity_contract":
      return "external output typed from input";
    case "mixed_state_algebraic_preview":
      return "mixed state/algebraic preview";
    case "no_jit_speed_claim":
      return "no JIT speed claim";
    case "not_adaptive":
      return "not adaptive";
    case "not_full_dae":
      return "not a full DAE solve";
    case "not_general_nonlinear":
      return "not a general nonlinear solve";
    case "not_solved_behavior_not_integrated":
      return "not solved because behavior graph is not connected";
    case "not_production_multi_domain":
      return "not production multi-domain";
    case "predictor_call_contract_pending_integration":
      return "Predictor contract not connected to this language-level solve";
    case "predictor_contract_metadata":
      return "Predictor contract metadata";
    case "predictor_output_typed_identity_contract":
      return "Predictor output typed from input";
    case "safe_repro_profile_policy_metadata":
      return "safe/repro profile policy metadata";
    case "solver_policy_not_integrated":
      return "solver policy not connected";
    case "symbolic_residual_preview_no_nonlinear_iteration":
      return "symbolic residual preview, no nonlinear iteration";
    default:
      return String(status ?? "-");
  }
}

function statusListLabel(values) {
  return Array.isArray(values) && values.length
    ? values.map(statusLabel).join(", ")
    : "-";
}

function sourceLineButton(item) {
  const line = sourceLineValue(item);
  const lineNumber = Number(line);
  if (!Number.isFinite(lineNumber) || lineNumber < 1) {
    return line ? escapeHtml(line) : "-";
  }
  const safeLine = Math.trunc(lineNumber);
  const column = sourceColumnValue(item);
  const columnNumber = Number(column);
  const hasColumn = Number.isFinite(columnNumber) && columnNumber > 1;
  const safeColumn = hasColumn ? Math.trunc(columnNumber) : null;
  const columnAttr = hasColumn ? ` data-source-column="${escapeAttr(safeColumn)}"` : "";
  const label = hasColumn ? `L${safeLine}:C${safeColumn}` : `L${safeLine}`;
  return `<button class="link-button" data-source-line="${escapeAttr(safeLine)}"${columnAttr}>${escapeHtml(label)}</button>`;
}

function sourceLineValue(item) {
  return item?.source_span?.line
    ?? item?.sourceSpan?.line
    ?? item?.source_line
    ?? item?.sourceLine
    ?? item?.line;
}

function sourceColumnValue(item) {
  return item?.source_span?.column
    ?? item?.sourceSpan?.column
    ?? item?.source_column
    ?? item?.sourceColumn
    ?? item?.column;
}

function sourceTokenButton(token, label = null) {
  const line = Number(token?.line ?? -1) + 1;
  const start = Number(token?.start ?? -1);
  const length = Number(token?.length ?? 0);
  if (!validSourceTokenRange(line, start, length)) {
    return "-";
  }
  const buttonLabel = label || `L${line}`;
  return `<button class="link-button token-range-button" data-source-token-line="${escapeAttr(line)}" data-source-token-start="${escapeAttr(start)}" data-source-token-length="${escapeAttr(length)}" title="Select token range">${escapeHtml(buttonLabel)}</button>`;
}

function sourceTokenCopyButton(token, mode, label) {
  const line = Number(token?.line ?? -1) + 1;
  const start = Number(token?.start ?? -1);
  const length = Number(token?.length ?? 0);
  if (!validSourceTokenRange(line, start, length)) {
    return "";
  }
  const title = mode === "range"
    ? "Copy token source range"
    : mode === "selector"
      ? "Copy token selector"
      : "Copy token text";
  return `<button class="link-button token-range-button" data-copy-source-token="${escapeAttr(mode)}" data-source-token-line="${escapeAttr(line)}" data-source-token-start="${escapeAttr(start)}" data-source-token-length="${escapeAttr(length)}" title="${escapeAttr(title)}">${escapeHtml(label)}</button>`;
}

function validSourceTokenRange(line, start, length) {
  return Number.isFinite(line)
    && Number.isFinite(start)
    && Number.isFinite(length)
    && line > 0
    && start >= 0
    && length > 0;
}

function bindSourceTokenRangeButtons(root) {
  root.querySelectorAll("[data-source-token-line]").forEach((button) => {
    button.onclick = () => selectSourceTokenRange(
      Number(button.dataset.sourceTokenLine || 0),
      Number(button.dataset.sourceTokenStart || 0),
      Number(button.dataset.sourceTokenLength || 0)
    );
  });
}

function bindSourceTokenCopyButtons(root) {
  root.querySelectorAll("[data-copy-source-token]").forEach((button) => {
    button.onclick = () => copySourceTokenRange(
      Number(button.dataset.sourceTokenLine || 0),
      Number(button.dataset.sourceTokenStart || 0),
      Number(button.dataset.sourceTokenLength || 0),
      button.dataset.copySourceToken || "text"
    );
  });
}

async function copyHighlightSummary() {
  const tokens = Array.isArray(semanticTokenPayload().tokens) ? semanticTokenPayload().tokens : [];
  const copied = await copyTextToClipboard(highlightSummaryCopyText(highlightCoverageRows(tokens)));
  setStatus(copied ? "Copied highlight summary" : "Copy failed");
}

function highlightSummaryCopyText(rows) {
  const current = checkFreshnessLabel().toLowerCase();
  const lines = [
    `file: ${state.currentPath || "-"}`,
    `highlight_data: ${current}`
  ];
  for (const row of rows) {
    lines.push(`${row.label}: ${row.status}; source_words=${row.sourceWords.length}; highlighted_ranges=${row.highlightCount}; matched=${row.highlightedWords.join(", ") || "-"}; unmatched=${row.missingWords.join(", ") || "-"}`);
  }
  return lines.join("\n");
}
async function copyVisibleHighlights() {
  const tokens = Array.isArray(semanticTokenPayload().tokens) ? semanticTokenPayload().tokens : [];
  const visible = filteredSemanticTokens(tokens);
  if (!visible.length) {
    setStatus("No visible highlights to copy");
    return;
  }
  const copied = await copyTextToClipboard(highlightTokenCopyText(visible));
  const noun = visible.length === 1 ? "highlight" : "highlights";
  setStatus(copied ? `Copied ${visible.length} visible ${noun}` : "Copy failed");
}

function highlightTokenCopyText(tokens) {
  const lines = [`file: ${state.currentPath || "-"}`];
  for (const token of tokens) {
    const line = Number(token.line ?? -1) + 1;
    const start = Number(token.start ?? 0);
    const length = Number(token.length ?? 0);
    const modifiers = arrayOrEmpty(token.modifiers).join(", ") || "-";
    const selectors = semanticTokenSelectors(token).join(", ") || "-";
    lines.push(`L${line}:${start}:${length} | ${semanticTokenText(token)} | ${token.type || "-"} | ${modifiers} | selectors=${selectors}`);
  }
  return lines.join("\n");
}

async function copySourceTokenRange(line, startByte, lengthBytes, mode = "text") {
  const editor = byId("editor");
  if (!editor || !validSourceTokenRange(line, startByte, lengthBytes)) return;
  const lineRange = sourceLineRange(editor.value, line - 1);
  const startColumn = Math.min(lineRange.text.length, Math.max(0, Math.trunc(startByte)));
  const endColumn = Math.min(
    lineRange.text.length,
    Math.max(startColumn, Math.trunc(startByte + lengthBytes))
  );
  const tokenText = lineRange.text.slice(startColumn, Math.max(startColumn, endColumn));
  const rangeText = `L${line}:${startByte}:${lengthBytes}`;
  const selectorText = mode === "selector" ? semanticTokenPrimarySelector(line, startByte, lengthBytes) : "";
  const copyText = mode === "range" ? rangeText : mode === "selector" ? selectorText : tokenText;
  const copied = await copyTextToClipboard(copyText);
  const copiedKind = mode === "range" ? "range" : mode === "selector" ? "selector" : "text";
  setStatus(copied
    ? `Copied token ${copiedKind} ${rangeText}`
    : "Copy failed");
}

function semanticTokenPrimarySelector(line, startByte, lengthBytes) {
  const token = semanticTokenForRange(line, startByte, lengthBytes);
  return semanticTokenSelectors(token)[0] || "";
}

function semanticTokenForRange(line, startByte, lengthBytes) {
  const lineIndex = Number(line) - 1;
  const tokens = Array.isArray(semanticTokenPayload().tokens) ? semanticTokenPayload().tokens : [];
  return tokens.find((token) =>
    Number(token?.line) === lineIndex
    && Number(token?.start) === Number(startByte)
    && Number(token?.length) === Number(lengthBytes)
  ) || null;
}

async function copyTextToClipboard(text) {
  const value = String(text ?? "");
  if (!value) return false;
  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(value);
      return true;
    } catch (_) {
      // Fall back to the hidden textarea path below.
    }
  }
  const helper = document.createElement("textarea");
  helper.value = value;
  helper.style.position = "fixed";
  helper.style.opacity = "0";
  document.body.appendChild(helper);
  helper.focus();
  helper.select();
  const copied = document.execCommand?.("copy") === true;
  helper.remove();
  return copied;
}

function setStatus(message) {
  state.status = String(message || "");
  const target = byId("ideStatus");
  if (target) target.textContent = state.status;
}

function bindHighlightTokenFilterButtons(root) {
  root.querySelectorAll("[data-highlight-token-filter]").forEach((button) => {
    button.onclick = () => applyHighlightTokenFilter(button.dataset.highlightTokenFilter || "");
  });
}

function applyHighlightTokenFilter(query) {
  state.highlightTokenQuery = String(query || "");
  state.sideTab = "highlight";
  render();
  const input = byId("highlightTokenQueryInput");
  if (input) {
    input.focus();
    input.setSelectionRange(input.value.length, input.value.length);
  }
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

function advancedDataToggle(title, data) {
  if (!hasAdvancedData(data)) return "";
  return `
    <details class="advanced-data-toggle">
      <summary>${escapeHtml(title)}</summary>
      <pre>${escapeHtml(JSON.stringify(data, null, 2))}</pre>
    </details>
  `;
}

function hasAdvancedData(data) {
  if (Array.isArray(data)) return data.length > 0;
  return Boolean(data && typeof data === "object" && Object.keys(data).length);
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
  if (state.highlightSource === null) {
    const message = state.check.status === "checking"
      ? "Analyzing current buffer..."
      : "Diagnostics unavailable. Use Check to retry.";
    return `<div class="problem-panel"><div class="empty-state">${escapeHtml(message)}</div></div>`;
  }
  const diagnostics = state.check.diagnostics || [];
  const codes = problemCodeOptions(diagnostics);
  const activeCode = activeProblemCode(diagnostics);
  const filtered = filteredProblems(activeCode);
  const rows = filtered.map((diag, index) => `
    <tr class="problem-row" data-problem-index="${escapeAttr(index)}" data-problem-line="${escapeAttr(diag.line || 0)}" data-problem-column="${escapeAttr(diag.column || 1)}" data-problem-start-character="${escapeAttr(diag.startCharacter ?? diag.start_character ?? -1)}" data-problem-end-character="${escapeAttr(diag.endCharacter ?? diag.end_character ?? -1)}" title="Select ${escapeAttr(diag.rangeText || diag.range_text || `line ${diag.line || "-"}`)}">
      <td class="${diag.severity === "error" ? "error" : "warning"}">${escapeHtml(diag.severity)}</td>
      <td>${problemRangeCell(diag)}</td>
      <td><code>${escapeHtml(diag.code)}</code></td>
      <td>
        <div class="problem-message">${escapeHtml(diag.message)}${diag.help ? `<div class="muted">help: ${escapeHtml(diag.help)}</div>` : ""}</div>
        <div class="problem-actions">${problemQuickFixButton(index)} ${problemCopyButton(index)}</div>
      </td>
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
        <input id="problemQueryInput" class="problem-query" value="${escapeAttr(state.problemQuery)}" placeholder="Filter diagnostics" title="Filter by code, message, help, line, or column" />
        <button id="clearProblemFilters">Clear</button>
        <div class="problem-navigation" role="group" aria-label="Problem navigation">
          <button id="previousProblemBtn" title="Previous problem (Shift+F8)" aria-label="Previous problem" ${filtered.length ? "" : "disabled"}>&uarr;</button>
          <button id="nextProblemBtn" title="Next problem (F8)" aria-label="Next problem" ${filtered.length ? "" : "disabled"}>&darr;</button>
        </div>
        <button id="quickFixCursorProblemBtn" title="Apply a compiler quick fix to the current or nearest same-line diagnostic" ${diagnostics.length ? "" : "disabled"}>Quick Fix at cursor</button>
        <button id="copyCursorProblemBtn" title="Copy current or nearest same-line diagnostic" ${diagnostics.length ? "" : "disabled"}>Copy at cursor</button>
        <button id="copyVisibleProblemsBtn" title="Copy filtered diagnostics" ${filtered.length ? "" : "disabled"}>Copy visible</button>
        <span class="muted">${filtered.length} of ${diagnostics.length}</span>
      </div>
      <div class="scroll problem-scroll">
      <table class="problems-table">
        <thead><tr><th>Severity</th><th>Range</th><th>Code</th><th>Message</th></tr></thead>
        <tbody>${rows || `<tr><td colspan="4" class="ok">${diagnostics.length ? "No diagnostics match the active filters" : "No diagnostics"}</td></tr>`}</tbody>
      </table>
      </div>
    </div>
  `;
}

function problemRangeCell(diag) {
  const rangeText = diag?.rangeText || diag?.range_text || "";
  const lineButton = sourceLineButton(diag);
  if (!rangeText) return lineButton;
  return `${lineButton}<div class="muted">${escapeHtml(rangeText)}</div>`;
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
      diag.rangeText,
      diag.range_text,
      `line ${diag.line}`,
      `l${diag.line}`,
      `column ${diag.column}`,
      `c${diag.column}`
    ].some((value) => String(value || "").toLowerCase().includes(query));
    return severityMatches && codeMatches && queryMatches;
  });
}

function activeProblemCode(diagnostics = state.check.diagnostics || []) {
  const codes = problemCodeOptions(diagnostics);
  return codes.includes(state.problemCode) ? state.problemCode : "all";
}

function problemCodeOptions(diagnostics) {
  return [...new Set(diagnostics.map((diag) => diag.code).filter(Boolean))].sort();
}

function problemSeverityLabel(severity, diagnostics) {
  if (severity === "all") return `All ${diagnostics.length}`;
  const count = diagnostics.filter((diag) => diag.severity === severity).length;
  return `${severity === "error" ? "Errors" : "Warnings"} ${count}`;
}

function orderedNavigableProblems() {
  const diagnostics = state.check.diagnostics || [];
  return filteredProblems(activeProblemCode(diagnostics))
    .map((diag, index) => ({
      diag,
      index,
      selection: problemSourceSelection(diag)
    }))
    .filter((item) => item.selection)
    .sort((left, right) => (
      left.selection.start - right.selection.start
      || left.selection.end - right.selection.end
      || problemSeverityRank(left.diag) - problemSeverityRank(right.diag)
      || String(left.diag.code || "").localeCompare(String(right.diag.code || ""))
      || left.index - right.index
    ));
}

function problemNavigationIndex(items, editor, direction) {
  if (!items.length) return -1;
  const step = direction < 0 ? -1 : 1;
  const selectionStart = Number(editor?.selectionStart) || 0;
  const selectionEnd = Number(editor?.selectionEnd) || selectionStart;
  const activeProblemIndex = Number(document.querySelector(".problem-row.active")?.dataset?.problemIndex);
  const active = Number.isInteger(activeProblemIndex)
    ? items.findIndex((item) => (
      item.index === activeProblemIndex
      && item.selection.start === selectionStart
      && item.selection.end === selectionEnd
    ))
    : -1;
  if (active >= 0) return (active + step + items.length) % items.length;
  const selected = items.findIndex((item) => (
    item.selection.start === selectionStart && item.selection.end === selectionEnd
  ));
  if (selected >= 0) return (selected + step + items.length) % items.length;
  if (step > 0) {
    const next = items.findIndex((item) => item.selection.start >= selectionStart);
    return next >= 0 ? next : 0;
  }
  for (let index = items.length - 1; index >= 0; index -= 1) {
    if (items[index].selection.start < selectionStart) return index;
  }
  return items.length - 1;
}

function navigateProblem(direction = 1) {
  if (state.highlightSource !== state.source) {
    setStatus(state.check.status === "checking"
      ? "Problems are still being analyzed"
      : "Analyze the current buffer before navigating problems");
    if (state.check.status !== "checking") scheduleLiveCheck();
    return false;
  }
  const editor = byId("editor");
  if (!editor || String(editor.value ?? "") !== String(state.source ?? "")) {
    setStatus("Current editor is unavailable");
    return false;
  }
  const items = orderedNavigableProblems();
  if (!items.length) {
    const diagnostics = state.check.diagnostics || [];
    setStatus(diagnostics.length
      ? "No problems match the active filters"
      : "No problems in the current buffer");
    return false;
  }
  const navigationIndex = problemNavigationIndex(items, editor, direction);
  const item = items[navigationIndex];
  state.bottomTab = "problems";
  render();
  if (!selectProblemDiagnostic(item.diag)) {
    setStatus("Problem range is no longer available");
    return false;
  }
  activateProblemRow(item.index);
  const code = item.diag.code || "diagnostic";
  const line = sourceLineValue(item.diag) || "-";
  setStatus(`Problem ${navigationIndex + 1} of ${items.length}: ${code} at L${line}`);
  return true;
}

function activateProblemRow(index, root = document) {
  root.querySelectorAll(".problem-row.active").forEach((row) => row.classList.remove("active"));
  if (!Number.isInteger(index) || index < 0) return false;
  const row = root.querySelector(`[data-problem-index="${index}"]`);
  if (!row) return false;
  row.classList.add("active");
  row.scrollIntoView?.({ block: "nearest" });
  return true;
}

function problemCopyButton(index) {
  return `<button class="link-button problem-copy-button" data-copy-problem-index="${escapeAttr(index)}" title="Copy diagnostic details">Copy</button>`;
}

function problemQuickFixButton(index) {
  return `<button class="link-button problem-quick-fix-button" data-quick-fix-problem-index="${escapeAttr(index)}" title="Find compiler quick fixes for this diagnostic">Quick Fix...</button>`;
}

async function requestProblemQuickFixByIndex(index) {
  const diagnostics = state.check.diagnostics || [];
  const diag = filteredProblems(activeProblemCode(diagnostics))[index];
  if (!diag) {
    setStatus("Problem no longer available");
    return false;
  }
  return await requestProblemQuickFix(diag);
}

async function requestCursorProblemQuickFix() {
  const match = problemAtCursor();
  if (!match) {
    setStatus("No same-line problem at the cursor");
    return false;
  }
  return await requestProblemQuickFix(match.diag);
}

async function requestProblemQuickFix(diag) {
  if (state.pendingQuickFix || state.pendingRename || state.pendingWorkspaceSymbols) return false;
  if (state.highlightSource !== state.source) {
    setStatus("Analyze the current buffer before requesting a quick fix");
    scheduleLiveCheck();
    return false;
  }
  if (!problemDiagnosticLspRange(diag)) {
    setStatus("Problem does not have a complete source range");
    return false;
  }
  rememberCurrentTab();
  const request = { path: state.currentPath, source: state.source };
  const revision = ++quickFixRequestRevision;
  setStatus(`Finding quick fixes for ${diag.code || "diagnostic"}...`);
  try {
    const payload = await call("ide_code_actions", request);
    if (revision !== quickFixRequestRevision) return false;
    if (!bufferRequestIsCurrent(request)) {
      setStatus("Quick fix cancelled; buffer changed");
      return false;
    }
    const plans = codeActionPlansForProblem(payload, diag, request);
    if (!plans.length) {
      setStatus(`No compiler quick fix for ${diag.code || "this diagnostic"}`);
      return false;
    }
    if (plans.length === 1) return applyProblemQuickFix(plans[0], request);
    openProblemQuickFixDialog({ request, diag, plans, revision });
    return true;
  } catch (error) {
    if (revision !== quickFixRequestRevision) return false;
    const message = String(error);
    setStatus(message);
    appendTerminal("error", message);
    return false;
  }
}

function codeActionPlansForProblem(payload, diag, request) {
  const payloadPath = definitionWorkspacePath(definitionPathFromUri(payload?.uri));
  if (!payloadPath || !sameDefinitionPath(payloadPath, request.path)) {
    throw new Error("Quick fix response did not match the current file.");
  }
  const actions = payload?.actions;
  if (!Array.isArray(actions)) throw new Error("Quick fix response did not contain an action list.");
  if (actions.length > 256) throw new Error("Quick fix response exceeded the 256-action safety limit.");
  return actions
    .filter((action) => arrayOrEmpty(action?.diagnostics).some((candidate) =>
      codeActionDiagnosticMatches(candidate, diag)
    ))
    .map((action) => codeActionPlan(action, request))
    .sort((left, right) => Number(right.isPreferred) - Number(left.isPreferred) || left.title.localeCompare(right.title));
}

function codeActionPlan(action, request) {
  const title = String(action?.title || "").trim();
  if (!title || action?.kind !== "quickfix") {
    throw new Error("Quick fix response contained an incomplete action.");
  }
  const changes = action?.edit?.changes;
  const entries = changes && typeof changes === "object" && !Array.isArray(changes)
    ? Object.entries(changes)
    : [];
  if (entries.length !== 1) {
    throw new Error("Native IDE quick fixes must edit only the current file.");
  }
  const [uri, rawEdits] = entries[0];
  const targetPath = definitionWorkspacePath(definitionPathFromUri(uri));
  if (!targetPath || !sameDefinitionPath(targetPath, request.path)) {
    throw new Error("Quick fix attempted to edit a different file.");
  }
  if (!Array.isArray(rawEdits) || !rawEdits.length || rawEdits.length > 1000) {
    throw new Error("Quick fix returned an invalid number of text edits.");
  }
  const edits = rawEdits.map((edit) => {
    const range = codeActionTextEditRange(edit);
    if (!range || typeof edit?.newText !== "string") {
      throw new Error("Quick fix returned an incomplete text edit.");
    }
    return { range, newText: edit.newText };
  });
  return { title, isPreferred: action.isPreferred === true, edits };
}

function codeActionTextEditRange(edit) {
  const start = edit?.range?.start;
  const end = edit?.range?.end;
  const range = {
    start: { line: Number(start?.line), character: Number(start?.character) },
    end: { line: Number(end?.line), character: Number(end?.character) }
  };
  if (
    !Number.isInteger(range.start.line)
    || !Number.isInteger(range.start.character)
    || !Number.isInteger(range.end.line)
    || !Number.isInteger(range.end.character)
    || range.start.line < 0
    || range.start.character < 0
    || range.end.line < range.start.line
    || (range.end.line === range.start.line && range.end.character < range.start.character)
  ) {
    return null;
  }
  return range;
}

function problemDiagnosticLspRange(diag) {
  const line = Number(diag?.line) - 1;
  const startCharacter = Number(diag?.startCharacter ?? diag?.start_character);
  const endCharacter = Number(diag?.endCharacter ?? diag?.end_character);
  if (
    !Number.isInteger(line)
    || line < 0
    || !Number.isInteger(startCharacter)
    || startCharacter < 0
    || !Number.isInteger(endCharacter)
    || endCharacter <= startCharacter
  ) {
    return null;
  }
  return {
    start: { line, character: startCharacter },
    end: { line, character: endCharacter }
  };
}

function codeActionDiagnosticMatches(candidate, diag) {
  const code = typeof candidate?.code === "string" ? candidate.code : candidate?.code?.value;
  const range = codeActionTextEditRange(candidate);
  const expected = problemDiagnosticLspRange(diag);
  return code === diag?.code && range && expected && sameWorkspaceRange(range, expected);
}

function openProblemQuickFixDialog(pending) {
  byId("quickFixBackdrop")?.remove();
  state.pendingQuickFix = pending;
  const backdrop = document.createElement("div");
  backdrop.id = "quickFixBackdrop";
  backdrop.className = "dialog-backdrop";
  backdrop.innerHTML = `
    <div class="unsaved-dialog quick-fix-dialog" role="dialog" aria-modal="true" aria-labelledby="quickFixTitle">
      <h2 id="quickFixTitle">Quick Fix</h2>
      <div class="unsaved-dialog-path" title="${escapeAttr(pending.request.path)}">${escapeHtml(pending.request.path)}</div>
      <div class="quick-fix-diagnostic"><code>${escapeHtml(pending.diag.code || "diagnostic")}</code> ${escapeHtml(pending.diag.message || "")}</div>
      <div class="quick-fix-options" role="list">
        ${pending.plans.map((plan, index) => `
          <button class="quick-fix-option ${plan.isPreferred ? "preferred" : ""}" data-quick-fix-plan-index="${escapeAttr(index)}" role="listitem">
            ${escapeHtml(plan.title)}${plan.isPreferred ? `<span>Preferred</span>` : ""}
          </button>
        `).join("")}
      </div>
      <div class="unsaved-dialog-actions">
        <button id="quickFixCancelBtn">Cancel</button>
      </div>
    </div>
  `;
  document.body.appendChild(backdrop);
  syncDialogInert();
  backdrop.querySelectorAll("[data-quick-fix-plan-index]").forEach((button) => {
    button.onclick = () => applyPendingProblemQuickFix(Number(button.dataset.quickFixPlanIndex || 0));
  });
  byId("quickFixCancelBtn").onclick = cancelProblemQuickFix;
  backdrop.onclick = (event) => {
    if (event.target === backdrop) cancelProblemQuickFix();
  };
  backdrop.querySelector("[data-quick-fix-plan-index]")?.focus();
  setStatus(`${pending.plans.length} quick fixes for ${pending.diag.code || "diagnostic"}`);
}

function cancelProblemQuickFix() {
  if (!state.pendingQuickFix) return;
  quickFixRequestRevision += 1;
  byId("quickFixBackdrop")?.remove();
  state.pendingQuickFix = null;
  syncDialogInert();
  setStatus("Quick fix cancelled");
  byId("editor")?.focus();
}

function closeProblemQuickFixDialog(pending) {
  if (state.pendingQuickFix !== pending) return;
  byId("quickFixBackdrop")?.remove();
  state.pendingQuickFix = null;
  syncDialogInert();
}

function applyPendingProblemQuickFix(index) {
  const pending = state.pendingQuickFix;
  const plan = pending?.plans?.[index];
  if (!pending || !plan || pending.revision !== quickFixRequestRevision) return false;
  closeProblemQuickFixDialog(pending);
  try {
    return applyProblemQuickFix(plan, pending.request);
  } catch (error) {
    const message = String(error);
    setStatus(message);
    appendTerminal("error", message);
    return false;
  }
}

function applyProblemQuickFix(plan, request) {
  if (!bufferRequestIsCurrent(request)) {
    throw new Error("Quick fix cancelled because the current buffer changed.");
  }
  const applied = applyCodeActionTextEdits(request.source, plan.edits);
  if (applied.source === request.source) throw new Error("Quick fix did not change the current buffer.");
  const tab = tabFor(request.path);
  if (!tab || tab.source !== request.source) {
    throw new Error("Quick fix cancelled because the current tab changed.");
  }
  state.source = applied.source;
  state.dirty = state.source !== state.savedSource;
  tab.source = applied.source;
  tab.dirty = state.dirty;
  quickFixRequestRevision += 1;
  clearReferenceResults();
  markCheckPending();
  state.status = `Applied quick fix: ${plan.title}`;
  render();
  const editor = byId("editor");
  const focus = applied.mappedEdits[0];
  if (editor && focus) {
    editor.focus();
    editor.selectionStart = focus.newStart;
    editor.selectionEnd = focus.newEnd;
    syncEditorHighlightScroll();
    updateEditorFindStatus();
    updateCursorInsight();
  }
  scheduleLiveCheck();
  return true;
}

function applyCodeActionTextEdits(source, edits) {
  const normalized = edits.map((edit) => {
    const start = sourceUtf16Offset(source, edit.range.start);
    const end = sourceUtf16Offset(source, edit.range.end);
    if (start === null || end === null || end < start || typeof edit.newText !== "string") {
      throw new Error("Quick fix returned a source range outside the current file.");
    }
    return { ...edit, start, end };
  }).sort((left, right) => left.start - right.start || left.end - right.end);
  for (let index = 1; index < normalized.length; index += 1) {
    const previous = normalized[index - 1];
    const current = normalized[index];
    if (current.start < previous.end || current.start === previous.start) {
      throw new Error("Quick fix returned overlapping source edits.");
    }
  }
  let cursor = 0;
  let changed = "";
  const mappedEdits = [];
  for (const edit of normalized) {
    changed += source.slice(cursor, edit.start);
    const newStart = changed.length;
    changed += edit.newText;
    mappedEdits.push({ range: edit.range, newStart, newEnd: changed.length });
    cursor = edit.end;
  }
  changed += source.slice(cursor);
  return { source: changed, mappedEdits };
}

async function copyProblemDiagnostic(index) {
  const diagnostics = state.check.diagnostics || [];
  const diag = filteredProblems(activeProblemCode(diagnostics))[index];
  if (!diag) {
    setStatus("Problem no longer available");
    return;
  }
  const copied = await copyTextToClipboard(problemCopyText(diag));
  const label = `${diag.code || "diagnostic"} L${sourceLineValue(diag) || "-"}`;
  setStatus(copied ? `Copied problem ${label}` : "Copy failed");
}

async function copyCursorProblem() {
  const match = problemAtCursor();
  if (!match) {
    setStatus("No same-line problem at the cursor");
    return;
  }
  const copied = await copyTextToClipboard(problemCopyText(match.diag));
  const label = `${match.diag.code || "diagnostic"} L${sourceLineValue(match.diag) || "-"}`;
  const qualifier = match.distance === 0 ? "problem" : "nearest same-line problem";
  setStatus(copied ? `Copied ${qualifier} ${label}` : "Copy failed");
}

function problemAtCursor() {
  const diagnostics = state.check.diagnostics || [];
  if (!diagnostics.length) return null;
  const editor = byId("editor");
  const source = editor?.value ?? state.source ?? "";
  const position = editorCursorPosition(source, editor?.selectionStart ?? 0);
  const line = position.line + 1;
  return diagnostics
    .map((diag, index) => ({
      diag,
      index,
      distance: problemCaretDistance(diag, line, position.column)
    }))
    .filter((item) => Number.isFinite(item.distance))
    .sort((left, right) => (
      left.distance - right.distance
      || problemSeverityRank(left.diag) - problemSeverityRank(right.diag)
      || left.index - right.index
    ))[0] || null;
}

function problemCaretDistance(diag, line, caretColumn) {
  const diagnosticLine = Number(sourceLineValue(diag));
  if (!Number.isFinite(diagnosticLine) || Math.trunc(diagnosticLine) !== line) return null;
  const startCharacter = Number(diag?.startCharacter ?? diag?.start_character);
  const endCharacter = Number(diag?.endCharacter ?? diag?.end_character);
  if (Number.isFinite(startCharacter) && Number.isFinite(endCharacter) && endCharacter > startCharacter) {
    const start = Math.max(0, Math.trunc(startCharacter));
    const end = Math.max(start + 1, Math.trunc(endCharacter));
    if (caretColumn >= start && caretColumn <= end) return 0;
    return Math.min(Math.abs(caretColumn - start), Math.abs(caretColumn - end));
  }
  const column = Number(sourceColumnValue(diag));
  if (Number.isFinite(column) && column > 0) {
    return Math.abs(caretColumn - Math.max(0, Math.trunc(column) - 1));
  }
  return 0;
}

function problemSeverityRank(diag) {
  if (diag?.severity === "error") return 0;
  if (diag?.severity === "warning") return 1;
  return 2;
}

async function copyVisibleProblems() {
  const diagnostics = state.check.diagnostics || [];
  const visible = filteredProblems(activeProblemCode(diagnostics));
  if (!visible.length) {
    setStatus("No visible problems to copy");
    return;
  }
  const copied = await copyTextToClipboard(visible.map(problemCopyText).join("\n\n"));
  const noun = visible.length === 1 ? "problem" : "problems";
  setStatus(copied ? `Copied ${visible.length} visible ${noun}` : "Copy failed");
}

function problemCopyText(diag) {
  const lines = [
    `file: ${state.currentPath || "-"}`,
    `line: ${sourceLineValue(diag) || "-"}`,
    `column: ${sourceColumnValue(diag) || "-"}`,
    `range: ${diag?.rangeText || diag?.range_text || "-"}`,
    `severity: ${diag?.severity || "-"}`,
    `code: ${diag?.code || "-"}`,
    `message: ${diag?.message || "-"}`
  ];
  const sourceLine = problemSourceLineText(diag);
  if (sourceLine) lines.push(`source: ${sourceLine}`);
  if (diag?.help) lines.push(`help: ${diag.help}`);
  return lines.join("\n");
}

function problemSourceLineText(diag) {
  const lineNumber = Number(sourceLineValue(diag));
  if (!Number.isFinite(lineNumber) || lineNumber < 1) return "";
  return sourceLineRange(state.source || "", Math.trunc(lineNumber) - 1).text.trimEnd();
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
        <input id="terminalInput" placeholder="check, run, cd <dir>, or EngLang statement" title="Supports check, run, reset, clear, cd <dir>, and one-line EngLang statements." />
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

function handleGlobalKeyDown(event) {
  const editorModalOpen = Boolean(
    state.pendingQuickFix
      || state.pendingRename
      || state.pendingWorkspaceSymbols
      || state.pendingWindowClose
      || state.pendingTabClose
  );
  const quickFixShortcut = (event.ctrlKey || event.metaKey)
    && !event.altKey
    && !event.shiftKey
    && String(event.key || "") === ".";
  if (quickFixShortcut) {
    event.preventDefault();
    if (!editorModalOpen) void requestCursorProblemQuickFix();
    return;
  }
  const outlineShortcut = (event.ctrlKey || event.metaKey)
    && event.shiftKey
    && !event.altKey
    && String(event.key || "").toLowerCase() === "o";
  if (outlineShortcut) {
    event.preventDefault();
    if (!editorModalOpen) focusOutline();
    return;
  }
  const workspaceSymbolShortcut = (event.ctrlKey || event.metaKey)
    && !event.altKey
    && !event.shiftKey
    && String(event.key || "").toLowerCase() === "t";
  if (workspaceSymbolShortcut) {
    event.preventDefault();
    if (state.pendingWorkspaceSymbols) byId("workspaceSymbolInput")?.focus();
    else if (!editorModalOpen) openWorkspaceSymbolSearch();
    return;
  }
  const findShortcut = (event.ctrlKey || event.metaKey)
    && !event.altKey
    && String(event.key || "").toLowerCase() === "f";
  if (findShortcut) {
    event.preventDefault();
    if (!editorModalOpen) openEditorFind();
    return;
  }
  const saveShortcut = (event.ctrlKey || event.metaKey)
    && !event.altKey
    && !event.shiftKey
    && String(event.key || "").toLowerCase() === "s";
  if (saveShortcut) {
    event.preventDefault();
    if (state.pendingQuickFix || state.pendingRename || state.pendingWorkspaceSymbols) return;
    if (state.pendingWindowClose) void saveAllDirtyTabsAndClose();
    else if (state.pendingTabClose) void savePendingTabAndClose();
    else void saveCurrent();
    return;
  }
  if (event.key === "F2" && !event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey) {
    event.preventDefault();
    if (!editorModalOpen) {
      void startSemanticRename();
    }
    return;
  }
  if (event.key === "F8" && !event.ctrlKey && !event.metaKey && !event.altKey) {
    event.preventDefault();
    if (!editorModalOpen) navigateProblem(event.shiftKey ? -1 : 1);
    return;
  }
  if (event.key === "F12" && event.shiftKey && !event.ctrlKey && !event.metaKey && !event.altKey) {
    event.preventDefault();
    if (!editorModalOpen) {
      void showDocumentHighlightsAtCaret();
    }
    return;
  }
  if (event.key === "F12" && !event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey) {
    event.preventDefault();
    if (!editorModalOpen) {
      void goToDefinitionAtCaret();
    }
    return;
  }
  if (event.key === "F3" && !event.ctrlKey && !event.metaKey && !event.altKey) {
    event.preventDefault();
    if (!editorModalOpen) {
      if (state.editorFindQuery) findEditorMatch(event.shiftKey ? -1 : 1);
      else openEditorFind();
    }
    return;
  }
  if (event.key === "Escape" && state.pendingWorkspaceSymbols) {
    event.preventDefault();
    cancelWorkspaceSymbolSearch();
  } else if (event.key === "Escape" && state.pendingQuickFix) {
    event.preventDefault();
    cancelProblemQuickFix();
  } else if (event.key === "Escape" && state.pendingRename) {
    event.preventDefault();
    cancelSemanticRename();
  } else if (event.key === "Escape" && state.pendingWindowClose) {
    event.preventDefault();
    cancelPendingWindowClose();
  } else if (event.key === "Escape" && state.pendingTabClose) {
    event.preventDefault();
    cancelPendingTabClose();
  } else if (event.key === "Escape" && state.editorFindOpen) {
    event.preventDefault();
    closeEditorFind();
  }
}

function hasDirtyTabs() {
  return dirtyTabs().length > 0;
}

function handleBeforeUnload(event) {
  if (!hasDirtyTabs()) return undefined;
  event.preventDefault();
  event.returnValue = "";
  return "";
}

function handleNativeWindowClose(event) {
  if (state.pendingQuickFix || state.pendingRename) {
    event.preventDefault();
    return;
  }
  if (!hasDirtyTabs()) return;
  event.preventDefault();
  if (state.pendingWorkspaceSymbols) closeWorkspaceSymbolSearch();
  openUnsavedWindowDialog();
}

async function bindNativeWindowClose() {
  if (nativeCloseListenerBound) return;
  const getCurrentWindow = window.__TAURI__?.window?.getCurrentWindow;
  if (typeof getCurrentWindow !== "function") return;
  const appWindow = getCurrentWindow();
  if (!appWindow || typeof appWindow.onCloseRequested !== "function") return;
  nativeAppWindow = appWindow;
  nativeCloseListenerBound = true;
  try {
    await appWindow.onCloseRequested(handleNativeWindowClose);
  } catch (error) {
    nativeCloseListenerBound = false;
    nativeAppWindow = null;
    setStatus(`Window close protection unavailable: ${compactText(String(error), 70)}`);
  }
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
  window.addEventListener("keydown", handleGlobalKeyDown);
  window.addEventListener("beforeunload", handleBeforeUnload);
  void bindNativeWindowClose();
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

function diagnosticCountLabel(label, count) {
  return state.highlightSource === null ? `${label} -` : `${label} ${count}`;
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
