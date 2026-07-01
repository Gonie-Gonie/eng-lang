const cp = require("child_process");
const crypto = require("crypto");
const fs = require("fs");
const path = require("path");
const vscode = require("vscode");

const LANGUAGE_ID = "englang";
const CHECK_DEBOUNCE_MS = 350;
const EXECUTION_PROFILES = [
  {
    id: "normal",
    description: "Default workflow execution",
    detail: "Runs declared effects and writes the standard review artifacts."
  },
  {
    id: "safe",
    description: "Reject side effects",
    detail: "Fails workflows with explicit write, export, process, file, or DB mutation effects."
  },
  {
    id: "repro",
    description: "Require reproducibility metadata",
    detail: "Records environment dependencies and rejects unseeded sampling or unpinned network/cache reads."
  }
];
const reviewCache = new Map();
const changeTimers = new Map();
let output;

const LAST_RUN_ARTIFACTS = [
  {
    id: "report",
    label: "Report HTML",
    description: "build/result/report.html",
    relativePath: ["build", "result", "report.html"],
    external: true
  },
  {
    id: "review",
    label: "Review JSON",
    description: "build/result/review.json",
    relativePath: ["build", "result", "review.json"]
  },
  {
    id: "outputManifest",
    label: "Output Manifest",
    description: "build/result/output_manifest.json",
    relativePath: ["build", "result", "output_manifest.json"]
  },
  {
    id: "runLog",
    label: "Run Log",
    description: "build/result/run_log.json",
    relativePath: ["build", "result", "run_log.json"]
  },
  {
    id: "runPlan",
    label: "Run Plan",
    description: "build/result/run_plan.json",
    relativePath: ["build", "result", "run_plan.json"]
  },
  {
    id: "runLock",
    label: "Run Lock",
    description: "build/result/run_lock.json",
    relativePath: ["build", "result", "run_lock.json"]
  },
  {
    id: "processResults",
    label: "Process Results",
    description: "build/result/process_results.json",
    relativePath: ["build", "result", "process_results.json"]
  },
  {
    id: "cacheManifest",
    label: "Cache Manifest",
    description: "build/result/cache_manifest.json",
    relativePath: ["build", "result", "cache_manifest.json"]
  },
  {
    id: "testResults",
    label: "Test Results",
    description: "build/result/test_results.json",
    relativePath: ["build", "result", "test_results.json"]
  }
];

const SEMANTIC_TOKEN_TYPES = [
  "namespace",
  "type",
  "class",
  "interface",
  "parameter",
  "variable",
  "property",
  "function",
  "method",
  "keyword",
  "modifier",
  "string",
  "number",
  "operator",
  "comment"
];

const SEMANTIC_TOKEN_MODIFIERS = [
  "declaration",
  "definition",
  "readonly",
  "static",
  "local",
  "imported",
  "defaultLibrary",
  "deprecated",
  "unit",
  "quantity",
  "axis",
  "timeseries",
  "uncertain",
  "sideEffect",
  "external",
  "validation",
  "report",
  "planned",
  "internal",
  "riskHigh",
  "riskMedium",
  "state",
  "input"
];

const semanticLegend = new vscode.SemanticTokensLegend(
  SEMANTIC_TOKEN_TYPES,
  SEMANTIC_TOKEN_MODIFIERS
);

function activate(context) {
  output = vscode.window.createOutputChannel("EngLang");
  const diagnostics = vscode.languages.createDiagnosticCollection("englang");
  context.subscriptions.push(output, diagnostics);

  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((document) => maybeCheck(document, diagnostics, context)),
    vscode.workspace.onDidChangeTextDocument((event) => scheduleChangedCheck(event.document, diagnostics, context)),
    vscode.workspace.onDidSaveTextDocument((document) => maybeCheck(document, diagnostics, context)),
    vscode.workspace.onDidCloseTextDocument((document) => {
      clearPendingCheck(document);
      diagnostics.delete(document.uri);
    }),
    vscode.commands.registerCommand("englang.checkFile", () => checkActiveFile(diagnostics, context)),
    vscode.commands.registerCommand("englang.runFile", () => runActiveFile(context)),
    vscode.commands.registerCommand("englang.runExample", () => runExample(context)),
    vscode.commands.registerCommand("englang.switchProfile", () => switchExecutionProfile()),
    vscode.commands.registerCommand("englang.reviewFile", () => reviewActiveFile(context)),
    vscode.commands.registerCommand("englang.openReviewPanel", () => openReviewPanel(context)),
    vscode.commands.registerCommand("englang.openReport", () => openLastRunArtifact("report")),
    vscode.commands.registerCommand("englang.openLastArtifact", openLastRunArtifactPicker),
    vscode.commands.registerCommand("englang.openReviewJson", () => openLastRunArtifact("review")),
    vscode.commands.registerCommand("englang.openOutputManifest", () => openLastRunArtifact("outputManifest")),
    vscode.commands.registerCommand("englang.openRunLog", () => openLastRunArtifact("runLog")),
    vscode.commands.registerCommand("englang.openRunPlan", () => openLastRunArtifact("runPlan")),
    vscode.commands.registerCommand("englang.openProcessResults", () => openLastRunArtifact("processResults")),
    vscode.commands.registerCommand("englang.openCacheManifest", () => openLastRunArtifact("cacheManifest")),
    vscode.commands.registerCommand("englang.showSemanticTokensDebug", () => showSemanticTokensDebug(context)),
    vscode.languages.registerHoverProvider(LANGUAGE_ID, new EngHoverProvider()),
    vscode.languages.registerCompletionItemProvider(
      LANGUAGE_ID,
      new EngCompletionProvider(context),
      ":",
      " ",
      "[",
      "."
    ),
    vscode.languages.registerDocumentSemanticTokensProvider(
      LANGUAGE_ID,
      new EngSemanticTokensProvider(context),
      semanticLegend
    ),
    vscode.languages.registerDocumentSymbolProvider(
      LANGUAGE_ID,
      new EngDocumentSymbolProvider(context)
    ),
    vscode.languages.registerFoldingRangeProvider(
      LANGUAGE_ID,
      new EngFoldingRangeProvider(context)
    ),
    vscode.languages.registerCodeActionsProvider(LANGUAGE_ID, new EngCodeActionProvider(), {
      providedCodeActionKinds: [vscode.CodeActionKind.QuickFix]
    })
  );

  for (const document of vscode.workspace.textDocuments) {
    maybeCheck(document, diagnostics, context);
  }
}

function deactivate() {}

function maybeCheck(document, diagnostics, context) {
  if (!isEngDocument(document)) {
    return;
  }
  const config = vscode.workspace.getConfiguration("englang", document.uri);
  if (!config.get("lintOnSave", true)) {
    return;
  }
  if (document.isDirty) {
    return;
  }
  clearPendingCheck(document);
  checkDocument(document, diagnostics, context);
}

function scheduleChangedCheck(document, diagnostics, context) {
  if (!isEngDocument(document)) {
    return;
  }
  const config = vscode.workspace.getConfiguration("englang", document.uri);
  if (!config.get("lintOnChange", true)) {
    return;
  }
  clearPendingCheck(document);
  const key = document.uri.toString();
  const timer = setTimeout(() => {
    changeTimers.delete(key);
    checkDocumentSource(document, diagnostics, context);
  }, CHECK_DEBOUNCE_MS);
  changeTimers.set(key, timer);
}

function clearPendingCheck(document) {
  const key = document.uri.toString();
  const timer = changeTimers.get(key);
  if (timer) {
    clearTimeout(timer);
    changeTimers.delete(key);
  }
}

async function checkActiveFile(diagnostics, context) {
  const document = vscode.window.activeTextEditor?.document;
  if (!document || !isEngDocument(document)) {
    vscode.window.showWarningMessage("Open an EngLang .eng file first.");
    return;
  }
  if (document.isDirty) {
    checkDocumentSource(document, diagnostics, context);
    return;
  }
  await checkDocument(document, diagnostics, context);
}

function checkDocument(document, diagnostics, context) {
  const backend = diagnosticsBackend(document);
  const runtime = backend === "lsp-snapshot" ? findLspRuntime(context, document) : findRuntime(context, document);
  const args = backend === "lsp-snapshot" ? ["--snapshot", document.uri.fsPath] : ["ide-check", document.uri.fsPath];
  const cwd = workspaceRoot(document);
  const documentVersion = document.version;
  output.appendLine(`${backend} check ${document.uri.fsPath}`);

  cp.execFile(
    runtime,
    args,
    { cwd, maxBuffer: 10 * 1024 * 1024 },
    (error, stdout, stderr) => {
      finishDocumentCheck(document, diagnostics, backend, documentVersion, error, stdout, stderr);
    }
  );
}

function checkDocumentSource(document, diagnostics, context) {
  const runtime = findLspRuntime(context, document);
  const cwd = workspaceRoot(document);
  const documentVersion = document.version;
  output.appendLine(`lsp-buffer check ${document.uri.fsPath}`);

  const child = cp.execFile(
    runtime,
    ["--snapshot-stdin", document.uri.fsPath],
    { cwd, maxBuffer: 10 * 1024 * 1024 },
    (error, stdout, stderr) => {
      finishDocumentCheck(document, diagnostics, "lsp-buffer", documentVersion, error, stdout, stderr);
    }
  );
  if (child.stdin) {
    child.stdin.end(document.getText());
  }
}

function finishDocumentCheck(document, diagnostics, backend, documentVersion, error, stdout, stderr) {
  if (document.version !== documentVersion) {
    return;
  }
  if (stderr && stderr.trim().length > 0) {
    output.appendLine(stderr.trim());
  }

  let review;
  try {
    review = JSON.parse(stdout);
  } catch (parseError) {
    output.appendLine(`Unable to parse EngLang ${backend} output: ${parseError.message}`);
    if (error) {
      output.appendLine(error.message);
    }
    diagnostics.set(document.uri, [
      new vscode.Diagnostic(
        firstLineRange(document),
        "EngLang runtime did not return editor JSON. Check englang.runtimePath or englang.lspPath.",
        vscode.DiagnosticSeverity.Error
      )
    ]);
    return;
  }

  reviewCache.set(document.uri.fsPath, review);
  diagnostics.set(document.uri, toDiagnostics(document, review));
  const errors = review.diagnostics?.filter((item) => severityName(item.severity) === "error").length ?? 0;
  const warnings = review.diagnostics?.filter((item) => severityName(item.severity) === "warning").length ?? 0;
  output.appendLine(`diagnostics: ${errors} error(s), ${warnings} warning(s)`);
}

function toDiagnostics(document, review) {
  return (review.diagnostics ?? []).map((item) => {
    const line = item.range?.start?.line ?? Math.max(0, (item.line ?? 1) - 1);
    const textLine = document.lineAt(Math.min(line, document.lineCount - 1));
    const startCharacter = item.range?.start?.character ?? 0;
    const endCharacter = item.range?.end?.character ?? Math.max(1, textLine.text.length);
    const range = new vscode.Range(line, startCharacter, line, Math.max(startCharacter + 1, endCharacter));
    const severity = toVscodeSeverity(item.severity);
    const diagnostic = new vscode.Diagnostic(range, item.message, severity);
    diagnostic.code = item.code;
    diagnostic.source = "eng";
    if (item.help) {
      diagnostic.message = `${item.message}\n${item.help}`;
    }
    return diagnostic;
  });
}

function severityName(severity) {
  if (severity === 1 || severity === "error") {
    return "error";
  }
  if (severity === 2 || severity === "warning") {
    return "warning";
  }
  return "info";
}

function toVscodeSeverity(severity) {
  const name = severityName(severity);
  if (name === "error") {
    return vscode.DiagnosticSeverity.Error;
  }
  if (name === "warning") {
    return vscode.DiagnosticSeverity.Warning;
  }
  return vscode.DiagnosticSeverity.Information;
}

async function runActiveFile(context) {
  const document = vscode.window.activeTextEditor?.document;
  if (!document || !isEngDocument(document)) {
    vscode.window.showWarningMessage("Open an EngLang .eng file first.");
    return;
  }
  await runDocumentFile(context, document);
}

async function runDocumentFile(context, document) {
  if (document.isDirty) {
    await document.save();
  }

  const runtime = findRuntime(context, document);
  const cwd = workspaceRoot(document);
  const profile = executionProfile(document);
  const args = ["run", document.uri.fsPath, "--profile", profile, "--save-artifacts"];
  output.show(true);
  output.appendLine(`run ${document.uri.fsPath} --profile ${profile}`);
  cp.execFile(
    runtime,
    args,
    { cwd, maxBuffer: 10 * 1024 * 1024 },
    (error, stdout, stderr) => {
      if (stdout) {
        output.appendLine(stdout.trim());
      }
      if (stderr) {
        output.appendLine(stderr.trim());
      }
      if (error) {
        vscode.window.showErrorMessage(`EngLang run failed in ${profile} profile. See the EngLang output panel.`);
      } else {
        vscode.window.showInformationMessage(`EngLang run completed (${profile}).`);
      }
    }
  );
}

async function runExample(context) {
  const root = currentWorkspaceRoot();
  if (!root) {
    vscode.window.showWarningMessage("Open an EngLang workspace first.");
    return;
  }

  const examples = findExampleFiles(root);
  if (examples.length === 0) {
    vscode.window.showWarningMessage("No EngLang examples found under examples/official or examples/workflows.");
    return;
  }

  const picked = await vscode.window.showQuickPick(
    examples.map((example) => ({
      label: example.label,
      description: example.kind,
      detail: example.relativePath,
      path: example.path
    })),
    { placeHolder: "Select an EngLang example to run" }
  );
  if (!picked) {
    return;
  }

  const document = await vscode.workspace.openTextDocument(vscode.Uri.file(picked.path));
  await vscode.window.showTextDocument(document, { preview: false });
  await runDocumentFile(context, document);
}

async function switchExecutionProfile() {
  const document = vscode.window.activeTextEditor?.document;
  const current = executionProfile(document);
  const picked = await vscode.window.showQuickPick(
    EXECUTION_PROFILES.map((profile) => ({
      label: profile.id,
      description: profile.description,
      detail: profile.detail,
      profile: profile.id
    })),
    { placeHolder: `Current EngLang execution profile: ${current}` }
  );
  if (!picked) {
    return;
  }

  const target = vscode.workspace.workspaceFolders?.length
    ? vscode.ConfigurationTarget.Workspace
    : vscode.ConfigurationTarget.Global;
  await engConfig(document).update("executionProfile", picked.profile, target);
  vscode.window.showInformationMessage(`EngLang execution profile set to ${picked.profile}.`);
}

async function reviewActiveFile(context) {
  const result = await runReviewForActiveDocument(context);
  if (!result) {
    return;
  }

  const reviewDocument = await vscode.workspace.openTextDocument({
    language: "json",
    content: JSON.stringify(result.review, null, 2)
  });
  await vscode.window.showTextDocument(reviewDocument, { preview: false });
  announceReviewResult(
    result,
    "EngLang review JSON opened.",
    "EngLang review JSON opened with diagnostics. See the EngLang output panel."
  );
}

async function openReviewPanel(context) {
  const result = await runReviewForActiveDocument(context);
  if (!result) {
    return;
  }

  const panel = vscode.window.createWebviewPanel(
    "englangReviewPanel",
    "EngLang Review",
    vscode.ViewColumn.Beside,
    {
      enableScripts: true,
      retainContextWhenHidden: true
    }
  );
  panel.webview.html = renderReviewSummaryHtml(
    result.review,
    result.document.uri.fsPath,
    reviewPanelNonce(),
    reviewPanelArtifacts(result.document)
  );
  panel.webview.onDidReceiveMessage((message) => {
    if (message?.type === "openSourceLine") {
      openSourceLine(result.document.uri, message.line).catch((error) => {
        output.appendLine(`Unable to open EngLang source line: ${error.message}`);
      });
    }
    if (message?.type === "openArtifact") {
      openLastRunArtifact(message.artifactId, result.document).catch((error) => {
        output.appendLine(`Unable to open EngLang artifact: ${error.message}`);
      });
    }
  });
  announceReviewResult(
    result,
    "EngLang review panel opened.",
    "EngLang review panel opened with diagnostics. See the EngLang output panel."
  );
}

async function runReviewForActiveDocument(context) {
  const document = vscode.window.activeTextEditor?.document;
  if (!document || !isEngDocument(document)) {
    vscode.window.showWarningMessage("Open an EngLang .eng file first.");
    return undefined;
  }
  if (document.isDirty) {
    await document.save();
  }

  return runReviewForDocument(context, document);
}

function runReviewForDocument(context, document) {
  const runtime = findRuntime(context, document);
  const cwd = workspaceRoot(document);
  output.show(true);
  output.appendLine(`review ${document.uri.fsPath}`);
  return new Promise((resolve) => {
    cp.execFile(
      runtime,
      ["review", document.uri.fsPath, "--json"],
      { cwd, maxBuffer: 20 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (stderr && stderr.trim().length > 0) {
          output.appendLine(stderr.trim());
        }

        let review;
        try {
          review = JSON.parse(stdout);
        } catch (parseError) {
          output.appendLine(`Unable to parse EngLang review output: ${parseError.message}`);
          if (error) {
            output.appendLine(error.message);
          }
          vscode.window.showErrorMessage("EngLang review failed. See the EngLang output panel.");
          resolve(undefined);
          return;
        }

        reviewCache.set(document.uri.fsPath, review);
        resolve({ document, review, error });
      }
    );
  });
}

function announceReviewResult(result, successMessage, warningMessage) {
  if (result.error) {
    output.appendLine(result.error.message);
    vscode.window.showWarningMessage(warningMessage);
    return;
  }
  vscode.window.showInformationMessage(successMessage);
}

async function openSourceLine(uri, line) {
  const lineNumber = Number(line);
  if (!Number.isFinite(lineNumber) || lineNumber < 1) {
    return;
  }
  const document = await vscode.workspace.openTextDocument(uri);
  const editor = await vscode.window.showTextDocument(document, {
    preview: false,
    viewColumn: vscode.ViewColumn.One
  });
  const targetLine = Math.min(Math.max(0, Math.trunc(lineNumber) - 1), document.lineCount - 1);
  const textLine = document.lineAt(targetLine);
  const position = new vscode.Position(targetLine, 0);
  const range = new vscode.Range(
    targetLine,
    0,
    targetLine,
    Math.max(1, textLine.text.length)
  );
  editor.selection = new vscode.Selection(position, position);
  editor.revealRange(range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
}

function renderReviewSummaryHtml(review, sourcePath, nonce, artifactLinks = []) {
  const doc = normalizedReviewDocument(review);
  const contract = doc.root_contract || doc.rootContract || {};
  const diagnostics = firstReviewArray(doc, review, "diagnostics");
  const inputs = reviewArray(doc, "inputs");
  const calculations = reviewArray(doc, "calculations");
  const symbols = reviewArray(doc, "symbols");
  const units = reviewArray(doc, "units_quantities", "unitsQuantities");
  const schemas = reviewArray(doc, "schemas");
  const timeAxes = reviewArray(doc, "time_axes", "timeAxes");
  const derivedValues = reviewArray(doc, "derived_values", "derivedValues");
  const tableTransforms = reviewArray(doc, "table_transforms", "tableTransforms");
  const outputs = reviewArray(doc, "report_outputs", "reportOutputs");
  const validations = reviewArray(doc, "validations");
  const sideEffects = reviewArray(doc, "side_effects", "sideEffects");
  const boundaries = reviewArray(doc, "external_boundaries", "externalBoundaries");
  const fallbacks = reviewArray(doc, "fallbacks");
  const risks = reviewArray(doc, "risks");
  const caches = reviewArray(doc, "caches");
  const modules = reviewArray(doc, "workflow_modules", "workflowModules");
  const sectionHashes = doc.section_hashes || doc.sectionHashes || {};
  const nativeModuleCount = modules.filter((module) => moduleStatusCategory(module) === "native").length;
  const plannedModuleCount = modules.filter((module) => moduleStatusCategory(module) === "planned").length;
  const internalModuleCount = modules.filter((module) => moduleStatusCategory(module) === "internal").length;

  const counts = [
    ["Inputs", countOrContract(inputs, contract, "input_count", "inputCount")],
    ["Symbols", countOrContract(symbols, contract, "symbol_count", "symbolCount")],
    ["Units", countOrContract(units, contract, "unit_quantity_count", "unitQuantityCount")],
    ["Schemas", countOrContract(schemas, contract, "schema_count", "schemaCount")],
    ["Time axes", countOrContract(timeAxes, contract, "time_axis_count", "timeAxisCount")],
    ["Derived values", derivedValues.length],
    ["Calculations", calculations.length],
    ["Caches", caches.length],
    ["Artifacts", artifactLinks.filter((artifact) => artifact.exists).length],
    ["Table transforms", tableTransforms.length],
    ["Outputs", countOrContract(outputs, contract, "report_output_count", "reportOutputCount")],
    ["Validations", countOrContract(validations, contract, "validation_count", "validationCount")],
    ["Side effects", countOrContract(sideEffects, contract, "side_effect_count", "sideEffectCount")],
    ["External boundaries", boundaries.length],
    ["Fallbacks", fallbacks.length],
    ["Risks", risks.length],
    ["Workflow modules", modules.length],
    ["Section hashes", Object.keys(sectionHashes).length]
  ];

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-${escapeAttr(nonce)}';">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>EngLang Review</title>
  <style>
    :root {
      color-scheme: light dark;
    }
    body {
      margin: 0;
      padding: 0;
      color: var(--vscode-editor-foreground);
      background: var(--vscode-editor-background);
      font-family: var(--vscode-font-family);
      font-size: var(--vscode-font-size);
      line-height: 1.45;
    }
    header {
      padding: 18px 22px 14px;
      border-bottom: 1px solid var(--vscode-panel-border);
      background: var(--vscode-sideBar-background);
    }
    main {
      padding: 18px 22px 28px;
    }
    h1, h2 {
      margin: 0;
      font-weight: 600;
      letter-spacing: 0;
    }
    h1 {
      font-size: 20px;
    }
    h2 {
      margin-top: 22px;
      margin-bottom: 8px;
      font-size: 14px;
    }
    code {
      color: var(--vscode-textPreformat-foreground);
      font-family: var(--vscode-editor-font-family);
      font-size: 0.95em;
      white-space: pre-wrap;
      word-break: break-word;
    }
    table {
      width: 100%;
      border-collapse: collapse;
      table-layout: fixed;
    }
    th, td {
      padding: 7px 8px;
      border-bottom: 1px solid var(--vscode-panel-border);
      text-align: left;
      vertical-align: top;
      word-break: break-word;
    }
    th {
      color: var(--vscode-descriptionForeground);
      background: var(--vscode-editorGroupHeader-tabsBackground);
      font-size: 12px;
      font-weight: 600;
    }
    .path {
      margin-top: 4px;
      color: var(--vscode-descriptionForeground);
      word-break: break-all;
    }
    .badges {
      display: flex;
      flex-wrap: wrap;
      gap: 6px;
      margin-top: 12px;
    }
    .badge, .pill {
      display: inline-flex;
      align-items: center;
      min-height: 20px;
      padding: 1px 7px;
      border: 1px solid var(--vscode-panel-border);
      border-radius: 4px;
      background: var(--vscode-button-secondaryBackground);
      color: var(--vscode-button-secondaryForeground);
      font-size: 12px;
      white-space: nowrap;
    }
    .pill.good {
      border-color: var(--vscode-testing-iconPassed);
      color: var(--vscode-testing-iconPassed);
      background: transparent;
    }
    .pill.warn {
      border-color: var(--vscode-editorWarning-foreground);
      color: var(--vscode-editorWarning-foreground);
      background: transparent;
    }
    .pill.bad {
      border-color: var(--vscode-editorError-foreground);
      color: var(--vscode-editorError-foreground);
      background: transparent;
    }
    .grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
      gap: 8px;
      margin-bottom: 8px;
    }
    .metric {
      padding: 9px 10px;
      border: 1px solid var(--vscode-panel-border);
      border-radius: 4px;
      background: var(--vscode-editorWidget-background);
    }
    .metric strong {
      display: block;
      margin-bottom: 2px;
      font-size: 15px;
    }
    .metric span, .muted {
      color: var(--vscode-descriptionForeground);
      font-size: 12px;
    }
    .table-wrap {
      overflow-x: auto;
      border: 1px solid var(--vscode-panel-border);
      border-radius: 4px;
    }
    .section-note {
      margin: -3px 0 8px;
      color: var(--vscode-descriptionForeground);
      font-size: 12px;
    }
    .line-button {
      display: inline-flex;
      align-items: center;
      min-height: 20px;
      padding: 0 6px;
      border: 1px solid var(--vscode-button-border, transparent);
      border-radius: 4px;
      color: var(--vscode-textLink-foreground);
      background: transparent;
      font: inherit;
      cursor: pointer;
    }
    .line-button:hover,
    .line-button:focus {
      color: var(--vscode-textLink-activeForeground);
      background: var(--vscode-list-hoverBackground);
      outline: 1px solid var(--vscode-focusBorder);
      outline-offset: 1px;
    }
  </style>
</head>
<body>
  <header>
    <h1>Review</h1>
    <div class="path">${escapeHtml(sourcePath)}</div>
    <div class="badges">
      ${badge("Status", doc.status || "-")}
      ${badge("Diagnostics", diagnostics.length)}
      ${badge("Native modules", nativeModuleCount)}
      ${badge("Planned", plannedModuleCount)}
      ${badge("Internal", internalModuleCount)}
      ${badge("Format", doc.format || "-")}
    </div>
  </header>
  <main>
    <div class="grid">
      ${counts.map(([label, value]) => `<div class="metric"><strong>${escapeHtml(value)}</strong><span>${escapeHtml(label)}</span></div>`).join("")}
    </div>

    <h2>Semantic Hash</h2>
    <div class="table-wrap">
      <table><tbody><tr><td><code>${escapeHtml(doc.semantic_hash || doc.semanticHash || "-")}</code></td></tr></tbody></table>
    </div>

    <h2>Last Run Artifacts</h2>
    ${renderReviewTable(
      ["Artifact", "Path", "Status", "Action"],
      artifactLinks,
      "No artifact links are configured.",
      (artifact) => `<tr>
        <td><strong>${escapeHtml(artifact.label)}</strong></td>
        <td><code>${escapeHtml(artifact.description)}</code></td>
        <td>${statusPill(artifact.exists ? "available" : "missing")}</td>
        <td>${artifact.exists ? `<button class="line-button" type="button" data-artifact-id="${escapeAttr(artifact.id)}" title="Open ${escapeAttr(artifact.label)}">Open</button>` : `<span class="muted">Run current file first</span>`}</td>
      </tr>`
    )}

    <h2>Inputs</h2>
    ${renderReviewTable(
      ["Line", "Name", "Kind", "Type", "Default", "Required"],
      inputs,
      "No inputs.",
      (input) => `<tr>
        <td>${sourceLineCell(input)}</td>
        <td><strong>${escapeHtml(reviewValue(input, "name"))}</strong></td>
        <td>${escapeHtml(reviewValue(input, "kind"))}</td>
        <td>${escapeHtml(reviewValue(input, "type"))}</td>
        <td><code>${escapeHtml(compactText(reviewValue(input, "default"), 110))}</code></td>
        <td>${escapeHtml(String(input.required ?? false))}${input.redacted ? `<div class="muted">redacted</div>` : ""}</td>
      </tr>`
    )}

    <h2>Symbols</h2>
    ${renderReviewTable(
      ["Line", "Name", "Quantity", "Unit", "Source"],
      symbols,
      "No symbols.",
      (symbol) => `<tr>
        <td>${sourceLineCell(symbol)}</td>
        <td><strong>${escapeHtml(reviewValue(symbol, "name"))}</strong></td>
        <td>${escapeHtml(reviewValue(symbol, "quantity_kind", "quantityKind"))}</td>
        <td>${escapeHtml(reviewValue(symbol, "display_unit", "displayUnit"))}</td>
        <td>${escapeHtml(reviewValue(symbol, "source"))}</td>
      </tr>`
    )}

    <h2>Schemas</h2>
    ${renderReviewTable(
      ["Line", "Schema", "Columns", "Constraints", "Missing Policy"],
      schemas,
      "No schemas.",
      (schema) => `<tr>
        <td>${sourceLineCell(schema)}</td>
        <td><strong>${escapeHtml(reviewValue(schema, "name"))}</strong></td>
        <td>${escapeHtml(columnSummary(reviewArray(schema, "columns"), 170))}</td>
        <td>${escapeHtml(schemaRuleSummary(reviewArray(schema, "constraints"), "text", 130))}</td>
        <td>${escapeHtml(schemaRuleSummary(reviewArray(schema, "missing_policies", "missingPolicies"), "policy", 130))}</td>
      </tr>`
    )}

    <h2>Units And Quantities</h2>
    ${renderReviewTable(
      ["Line", "Name", "Quantity", "Source", "Display", "Derivation"],
      units,
      "No unit or quantity records.",
      (unit) => `<tr>
        <td>${sourceLineCell(unit)}</td>
        <td><strong>${escapeHtml(reviewValue(unit, "name"))}</strong><div class="muted">${escapeHtml(reviewValue(unit, "status"))}</div></td>
        <td>${escapeHtml(reviewValue(unit, "quantity_kind", "quantityKind"))}</td>
        <td>${escapeHtml(reviewValue(unit, "source_unit", "sourceUnit"))}</td>
        <td>${escapeHtml(reviewValue(unit, "display_unit", "displayUnit"))}<div class="muted">${escapeHtml(reviewValue(unit, "canonical_unit", "canonicalUnit"))}</div></td>
        <td>${escapeHtml(reviewList(reviewArray(unit, "derivation_steps", "derivationSteps"), 140))}</td>
      </tr>`
    )}

    <h2>Time Axes</h2>
    ${renderReviewTable(
      ["Line", "Axis", "Binding", "Role", "Source"],
      timeAxes,
      "No time axes.",
      (axis) => `<tr>
        <td>${sourceLineCell(axis)}</td>
        <td><strong>${escapeHtml(reviewValue(axis, "axis"))}</strong></td>
        <td>${escapeHtml(reviewValue(axis, "binding"))}</td>
        <td>${escapeHtml(reviewValue(axis, "role"))}</td>
        <td>${escapeHtml(reviewValue(axis, "source"))}</td>
      </tr>`
    )}

    <h2>Derived Values</h2>
    ${renderReviewTable(
      ["Line", "Name", "Expression", "Quantity", "Unit"],
      derivedValues,
      "No derived values.",
      (derived) => `<tr>
        <td>${sourceLineCell(derived)}</td>
        <td><strong>${escapeHtml(reviewValue(derived, "name"))}</strong></td>
        <td><code>${escapeHtml(compactText(reviewValue(derived, "expression"), 150))}</code></td>
        <td>${escapeHtml(reviewValue(derived, "quantity_kind", "quantityKind"))}</td>
        <td>${escapeHtml(reviewValue(derived, "display_unit", "displayUnit"))}</td>
      </tr>`
    )}

    <h2>Caches</h2>
    ${renderReviewTable(
      ["Line", "Owner", "Status", "Key", "Hash"],
      caches,
      "No cache records.",
      (cache) => `<tr>
        <td>${sourceLineCell(cache)}</td>
        <td><strong>${escapeHtml(reviewValue(cache, "owner_name", "ownerName"))}</strong><div class="muted">${escapeHtml(reviewValue(cache, "owner_kind", "ownerKind"))}</div></td>
        <td>${statusPill(reviewValue(cache, "status"))}<div class="muted">${escapeHtml(reviewValue(cache, "policy"))}</div></td>
        <td><code>${escapeHtml(compactText(reviewValue(cache, "cache_key", "cacheKey"), 130))}</code></td>
        <td><code>${escapeHtml(compactText(reviewValue(cache, "observed_hash", "observedHash"), 80))}</code></td>
      </tr>`
    )}

    <h2>Diagnostics</h2>
    ${renderReviewTable(
      ["Line", "Severity", "Code", "Message"],
      diagnostics,
      "No diagnostics.",
      (diagnostic) => `<tr>
        <td>${sourceLineCell(diagnostic)}</td>
        <td>${statusPill(severityName(diagnostic.severity))}</td>
        <td><code>${escapeHtml(reviewValue(diagnostic, "code"))}</code></td>
        <td>${escapeHtml(compactText(reviewValue(diagnostic, "message"), 180))}${diagnostic.help ? `<div class="muted">${escapeHtml(compactText(diagnostic.help, 180))}</div>` : ""}</td>
      </tr>`
    )}

    <h2>External Boundaries</h2>
    ${renderReviewTable(
      ["Line", "Name", "Target", "Status", "Risk", "Effects"],
      boundaries,
      "No external boundaries.",
      (boundary) => `<tr>
        <td>${sourceLineCell(boundary)}</td>
        <td><strong>${escapeHtml(reviewValue(boundary, "name", "kind"))}</strong><div class="muted">${escapeHtml(reviewValue(boundary, "kind"))}</div></td>
        <td><code>${escapeHtml(compactText(reviewValue(boundary, "target"), 120))}</code></td>
        <td>${statusPill(reviewValue(boundary, "status"))}<div class="muted">${escapeHtml(boundary.status_class || boundary.statusClass || "")} ${escapeHtml(boundary.status_code ?? boundary.statusCode ?? "")}</div></td>
        <td>${statusPill(reviewValue(boundary, "risk_level", "riskLevel"))}</td>
        <td>${escapeHtml(reviewList(reviewArray(boundary, "side_effects", "sideEffects"), 120))}</td>
      </tr>`
    )}

    <h2>Side Effects</h2>
    ${renderReviewTable(
      ["Line", "Kind", "Target", "Status", "Risk"],
      sideEffects,
      "No side effects.",
      (effect) => `<tr>
        <td>${sourceLineCell(effect)}</td>
        <td><strong>${escapeHtml(reviewValue(effect, "kind"))}</strong></td>
        <td><code>${escapeHtml(compactText(reviewValue(effect, "target", "path"), 120))}</code></td>
        <td>${statusPill(reviewValue(effect, "status"))}</td>
        <td>${statusPill(reviewValue(effect, "risk_level", "riskLevel"))}</td>
      </tr>`
    )}

    <h2>Table Transforms</h2>
    ${renderReviewTable(
      ["Line", "Binding", "Operation", "Source", "Predicates", "Status"],
      tableTransforms,
      "No table transforms.",
      (transform) => `<tr>
        <td>${sourceLineCell(transform)}</td>
        <td><strong>${escapeHtml(reviewValue(transform, "binding"))}</strong><div class="muted">${escapeHtml(reviewValue(transform, "schema_name", "schemaName"))}</div></td>
        <td>${escapeHtml(reviewValue(transform, "operation"))}</td>
        <td>${escapeHtml(reviewValue(transform, "source_table", "sourceTable"))}</td>
        <td>${escapeHtml(predicateSummary(reviewArray(transform, "predicates"), 160))}</td>
        <td>${statusPill(reviewValue(transform, "status"))}</td>
      </tr>`
    )}

    <h2>Calculations</h2>
    ${renderReviewTable(
      ["Line", "Name", "Expression", "Inputs", "Output"],
      calculations,
      "No calculations.",
      (calculation) => `<tr>
        <td>${sourceLineCell(calculation)}</td>
        <td><strong>${escapeHtml(reviewValue(calculation, "name"))}</strong><div class="muted">${escapeHtml(reviewValue(calculation, "kind"))}</div></td>
        <td><code>${escapeHtml(compactText(reviewValue(calculation, "expression"), 130))}</code></td>
        <td>${escapeHtml(reviewList(reviewArray(calculation, "input_symbols", "inputSymbols"), 100))}</td>
        <td>${escapeHtml(reviewValue(calculation, "output_quantity", "outputQuantity"))}</td>
      </tr>`
    )}

    <h2>Report Outputs</h2>
    ${renderReviewTable(
      ["Line", "Kind", "Source", "Quantity", "Status"],
      outputs,
      "No report outputs.",
      (outputItem) => `<tr>
        <td>${sourceLineCell(outputItem)}</td>
        <td><strong>${escapeHtml(reviewValue(outputItem, "kind"))}</strong></td>
        <td>${escapeHtml(reviewValue(outputItem, "source"))}</td>
        <td>${escapeHtml(reviewValue(outputItem, "quantity_kind", "quantityKind"))}</td>
        <td>${statusPill(reviewValue(outputItem, "status"))}</td>
      </tr>`
    )}

    <h2>Validations</h2>
    ${renderReviewTable(
      ["Line", "Target", "Kind", "Status", "Reason"],
      validations,
      "No validations.",
      (validation) => `<tr>
        <td>${sourceLineCell(validation)}</td>
        <td><strong>${escapeHtml(reviewValue(validation, "target", "name"))}</strong></td>
        <td>${escapeHtml(reviewValue(validation, "kind", "category"))}</td>
        <td>${statusPill(reviewValue(validation, "status"))}</td>
        <td>${escapeHtml(compactText(reviewValue(validation, "reason", "summary"), 140))}</td>
      </tr>`
    )}

    <h2>Fallbacks</h2>
    ${renderReviewTable(
      ["Line", "Kind", "Target", "Method", "Risk", "Assumption"],
      fallbacks,
      "No fallbacks.",
      (fallback) => `<tr>
        <td>${sourceLineCell(fallback)}</td>
        <td><strong>${escapeHtml(reviewValue(fallback, "kind"))}</strong></td>
        <td>${escapeHtml(reviewValue(fallback, "target"))}</td>
        <td>${escapeHtml(reviewValue(fallback, "method"))}</td>
        <td>${statusPill(reviewValue(fallback, "risk_level", "riskLevel"))}</td>
        <td>${escapeHtml(compactText(reviewValue(fallback, "assumption", "reason"), 140))}</td>
      </tr>`
    )}

    <h2>Risks</h2>
    ${renderReviewTable(
      ["Line", "Category", "Level", "Severity", "Summary"],
      risks,
      "No review risks.",
      (risk) => `<tr>
        <td>${sourceLineCell(risk)}</td>
        <td><strong>${escapeHtml(reviewValue(risk, "category"))}</strong></td>
        <td>${statusPill(reviewValue(risk, "level"))}</td>
        <td>${escapeHtml(reviewValue(risk, "severity"))}</td>
        <td>${escapeHtml(compactText(reviewValue(risk, "summary"), 150))}</td>
      </tr>`
    )}

    <h2>Workflow Modules</h2>
    <div class="section-note">Native means compiler/runtime-backed for the current public surface.</div>
    ${renderReviewTable(
      ["Module", "Status", "Backing", "Purpose", "Artifacts", "Tests"],
      modules,
      "No workflow modules.",
      (module) => `<tr>
        <td><strong>${escapeHtml(reviewValue(module, "name"))}</strong></td>
        <td>${statusPill(module.status_label || module.statusLabel || module.status || "-")}<div class="muted">${escapeHtml(compactText(module.status_detail || module.statusDetail || module.status || "-", 100))}</div></td>
        <td>${escapeHtml(reviewValue(module, "backing"))}</td>
        <td>${escapeHtml(compactText(reviewValue(module, "purpose"), 160))}</td>
        <td>${escapeHtml(module.artifact_count ?? module.artifactCount ?? reviewArray(module, "artifacts").length)}</td>
        <td>${escapeHtml(module.test_count ?? module.testCount ?? reviewArray(module, "tests").length)}</td>
      </tr>`
    )}
  </main>
  <script nonce="${escapeAttr(nonce)}">
    (() => {
      const vscode = acquireVsCodeApi();
      document.addEventListener("click", (event) => {
        const artifactTarget = event.target.closest("[data-artifact-id]");
        if (artifactTarget) {
          vscode.postMessage({
            type: "openArtifact",
            artifactId: artifactTarget.getAttribute("data-artifact-id")
          });
          return;
        }
        const target = event.target.closest("[data-source-line]");
        if (!target) {
          return;
        }
        const line = Number(target.getAttribute("data-source-line"));
        if (Number.isFinite(line) && line > 0) {
          vscode.postMessage({ type: "openSourceLine", line });
        }
      });
    })();
  </script>
</body>
</html>`;
}

function reviewPanelNonce() {
  return crypto.randomBytes(16).toString("base64");
}

function reviewPanelArtifacts(document) {
  const root = workspaceRoot(document);
  return LAST_RUN_ARTIFACTS.map((artifact) => {
    const artifactPath = path.join(root, ...artifact.relativePath);
    return {
      id: artifact.id,
      label: artifact.label,
      description: artifact.description,
      exists: fs.existsSync(artifactPath)
    };
  });
}

function normalizedReviewDocument(review) {
  if (review && typeof review === "object") {
    return review.review_document || review.reviewDocument || review;
  }
  return {};
}

function firstReviewArray(primary, fallback, snakeKey, camelKey = snakeKey) {
  const primaryValue = reviewArray(primary, snakeKey, camelKey);
  if (primaryValue.length > 0) {
    return primaryValue;
  }
  return reviewArray(fallback, snakeKey, camelKey);
}

function reviewArray(object, snakeKey, camelKey = snakeKey) {
  const value = object?.[snakeKey] ?? object?.[camelKey];
  return Array.isArray(value) ? value : [];
}

function reviewValue(object, snakeKey, camelKey = snakeKey, fallback = "-") {
  if (!object || typeof object !== "object") {
    return fallback;
  }
  const value = object[snakeKey] ?? object[camelKey];
  return value === null || value === undefined || value === "" ? fallback : value;
}

function countOrContract(items, contract, snakeKey, camelKey) {
  if (items.length > 0) {
    return items.length;
  }
  return contract?.[snakeKey] ?? contract?.[camelKey] ?? 0;
}

function lineValue(item) {
  return item?.line ?? item?.source_line ?? item?.sourceLine ?? "-";
}

function sourceLineCell(item) {
  const line = lineValue(item);
  const lineNumber = Number(line);
  if (!Number.isFinite(lineNumber) || lineNumber < 1) {
    return escapeHtml(line);
  }
  const safeLine = Math.trunc(lineNumber);
  return `<button class="line-button" type="button" data-source-line="${escapeAttr(safeLine)}" title="Open source line ${escapeAttr(safeLine)}">L${escapeHtml(safeLine)}</button>`;
}

function reviewList(value, limit = 120) {
  if (!Array.isArray(value) || value.length === 0) {
    return "-";
  }
  return compactText(
    value.map((item) => {
      if (item && typeof item === "object") {
        return JSON.stringify(item);
      }
      return String(item);
    }).join("; "),
    limit
  );
}

function columnSummary(columns, limit = 140) {
  if (!Array.isArray(columns) || columns.length === 0) {
    return "-";
  }
  return compactText(
    columns.map((column) => {
      const name = column.name || "-";
      const type = column.type || "-";
      const unit = column.unit ? ` [${column.unit}]` : "";
      const flags = [
        column.is_index || column.isIndex ? "index" : "",
        column.optional ? "optional" : ""
      ].filter(Boolean);
      return `${name}: ${type}${unit}${flags.length ? ` (${flags.join(", ")})` : ""}`;
    }).join("; "),
    limit
  );
}

function schemaRuleSummary(items, valueKey, limit = 120) {
  if (!Array.isArray(items) || items.length === 0) {
    return "-";
  }
  return compactText(
    items.map((item) => {
      if (!item || typeof item !== "object") {
        return String(item);
      }
      const column = item.column ? `${item.column}: ` : "";
      return `${column}${item[valueKey] || item.text || item.policy || JSON.stringify(item)}`;
    }).join("; "),
    limit
  );
}

function predicateSummary(predicates, limit = 140) {
  if (!Array.isArray(predicates) || predicates.length === 0) {
    return "-";
  }
  const text = predicates.map((predicate) => {
    const expression = predicate.expression || [
      predicate.column,
      predicate.operator,
      predicate.value
    ].filter((part) => part !== null && part !== undefined && part !== "").join(" ");
    return `${expression || "-"} (${predicate.status || "-"})`;
  }).join("; ");
  return compactText(text, limit);
}

function compactText(value, limit = 120) {
  if (value === null || value === undefined || value === "") {
    return "-";
  }
  const text = typeof value === "string" ? value : String(value);
  if (text.length <= limit) {
    return text;
  }
  return `${text.slice(0, Math.max(0, limit - 3))}...`;
}

function renderReviewTable(headers, rows, emptyLabel, renderRow) {
  const headerHtml = headers.map((header) => `<th>${escapeHtml(header)}</th>`).join("");
  const bodyHtml = rows.length > 0
    ? rows.map(renderRow).join("")
    : `<tr><td colspan="${headers.length}" class="muted">${escapeHtml(emptyLabel)}</td></tr>`;
  return `<div class="table-wrap"><table><thead><tr>${headerHtml}</tr></thead><tbody>${bodyHtml}</tbody></table></div>`;
}

function badge(label, value) {
  return `<span class="badge">${escapeHtml(label)} ${escapeHtml(value)}</span>`;
}

function statusPill(value) {
  return `<span class="pill ${statusClass(value)}">${escapeHtml(value)}</span>`;
}

function statusClass(value) {
  const text = String(value ?? "").toLowerCase();
  if (!text || text === "-") {
    return "";
  }
  if (
    text.includes("error") ||
    text.includes("fail") ||
    text.includes("high") ||
    text.includes("blocked") ||
    text.includes("invalid")
  ) {
    return "bad";
  }
  if (
    text.includes("warn") ||
    text.includes("medium") ||
    text.includes("stale") ||
    text.includes("missing") ||
    text.includes("planned")
  ) {
    return "warn";
  }
  if (
    text.includes("success") ||
    text.includes("supported") ||
    text.includes("native") ||
    text.includes("accepted") ||
    text.includes("declared") ||
    text.includes("fixture") ||
    text.includes("passed") ||
    text.includes("ok")
  ) {
    return "good";
  }
  return "";
}

function moduleStatusCategory(module) {
  const status = String(module?.status || "").toLowerCase();
  if (status.startsWith("supported") || status.includes("native")) {
    return "native";
  }
  if (status.includes("internal")) {
    return "internal";
  }
  if (status.includes("planned")) {
    return "planned";
  }
  return "other";
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function escapeAttr(value) {
  return escapeHtml(value);
}

async function openLastRunArtifactPicker() {
  const picked = await vscode.window.showQuickPick(
    LAST_RUN_ARTIFACTS.map((artifact) => ({
      label: artifact.label,
      description: artifact.description,
      artifact
    })),
    { placeHolder: "Open an artifact from build/result" }
  );
  if (picked) {
    await openLastRunArtifact(picked.artifact.id);
  }
}

async function openLastRunArtifact(artifactId, sourceDocument = undefined) {
  const artifact = LAST_RUN_ARTIFACTS.find((item) => item.id === artifactId);
  if (!artifact) {
    vscode.window.showWarningMessage(`Unknown EngLang artifact: ${artifactId}`);
    return;
  }
  const root = sourceDocument ? workspaceRoot(sourceDocument) : currentWorkspaceRoot();
  if (!root) {
    vscode.window.showWarningMessage("Open an EngLang workspace folder first.");
    return;
  }
  const artifactPath = path.join(root, ...artifact.relativePath);
  if (!fs.existsSync(artifactPath)) {
    vscode.window.showWarningMessage(`No ${artifact.description} found yet. Run the current file first.`);
    return;
  }
  const uri = vscode.Uri.file(artifactPath);
  if (artifact.external) {
    await vscode.env.openExternal(uri);
    return;
  }
  const document = await vscode.workspace.openTextDocument(uri);
  await vscode.window.showTextDocument(document, { preview: false });
}

async function showSemanticTokensDebug(context) {
  const document = vscode.window.activeTextEditor?.document;
  if (!document || !isEngDocument(document)) {
    vscode.window.showWarningMessage("Open an EngLang .eng file first.");
    return;
  }
  const snapshot = await snapshotDocumentSource(document, context);
  if (!snapshot) {
    vscode.window.showWarningMessage("No semantic token snapshot is available. See the EngLang output panel.");
    return;
  }
  reviewCache.set(document.uri.fsPath, snapshot);
  const semanticTokens = snapshot.semantic_tokens ?? { legend: {}, tokens: [] };
  const tokenCounts = {};
  for (const token of semanticTokens.tokens ?? []) {
    tokenCounts[token.type] = (tokenCounts[token.type] ?? 0) + 1;
  }
  const payload = {
    source: document.uri.fsPath,
    token_count: semanticTokens.tokens?.length ?? 0,
    token_counts_by_type: tokenCounts,
    semantic_tokens: semanticTokens
  };
  const debugDocument = await vscode.workspace.openTextDocument({
    language: "json",
    content: JSON.stringify(payload, null, 2)
  });
  await vscode.window.showTextDocument(debugDocument, { preview: false });
}

class EngSemanticTokensProvider {
  constructor(context) {
    this.context = context;
  }

  async provideDocumentSemanticTokens(document, cancellationToken) {
    if (!isEngDocument(document)) {
      return new vscode.SemanticTokens(new Uint32Array());
    }
    const config = vscode.workspace.getConfiguration("englang", document.uri);
    if (!config.get("semanticHighlighting.enabled", true)) {
      return new vscode.SemanticTokens(new Uint32Array());
    }

    const snapshot = await snapshotDocumentSource(document, this.context, cancellationToken);
    if (!snapshot) {
      return new vscode.SemanticTokens(new Uint32Array());
    }
    reviewCache.set(document.uri.fsPath, snapshot);
    return semanticTokensFromSnapshot(snapshot);
  }
}

function snapshotDocumentSource(document, context, cancellationToken) {
  return new Promise((resolve) => {
    const runtime = findLspRuntime(context, document);
    const cwd = workspaceRoot(document);
    let settled = false;
    const finish = (value) => {
      if (settled) {
        return;
      }
      settled = true;
      resolve(value);
    };

    const child = cp.execFile(
      runtime,
      ["--snapshot-stdin", document.uri.fsPath],
      { cwd, maxBuffer: 10 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (stderr && stderr.trim().length > 0) {
          output.appendLine(stderr.trim());
        }
        if (error) {
          output.appendLine(`LSP snapshot failed: ${error.message}`);
          finish(undefined);
          return;
        }
        try {
          finish(JSON.parse(stdout));
        } catch (parseError) {
          output.appendLine(`Unable to parse EngLang LSP snapshot: ${parseError.message}`);
          finish(undefined);
        }
      }
    );

    cancellationToken?.onCancellationRequested(() => {
      child.kill();
      finish(undefined);
    });

    if (child.stdin) {
      child.stdin.end(document.getText());
    }
  });
}

function completionSnapshotForPosition(document, position, context, cancellationToken) {
  return new Promise((resolve) => {
    if (!isEngDocument(document)) {
      resolve(undefined);
      return;
    }

    const runtime = findLspRuntime(context, document);
    const cwd = workspaceRoot(document);
    let settled = false;
    const finish = (value) => {
      if (settled) {
        return;
      }
      settled = true;
      resolve(value);
    };

    const child = cp.execFile(
      runtime,
      [
        "--completion-stdin",
        document.uri.fsPath,
        String(position.line),
        String(position.character)
      ],
      { cwd, maxBuffer: 10 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (stderr && stderr.trim().length > 0) {
          output.appendLine(stderr.trim());
        }
        if (error) {
          output.appendLine(`completion snapshot failed: ${error.message}`);
          finish(undefined);
          return;
        }
        try {
          const payload = JSON.parse(stdout);
          if (Array.isArray(payload)) {
            finish({ completions: payload });
            return;
          }
          finish(payload);
        } catch (parseError) {
          output.appendLine(`Unable to parse EngLang completion snapshot: ${parseError.message}`);
          finish(undefined);
        }
      }
    );

    cancellationToken?.onCancellationRequested(() => {
      child.kill();
      finish(undefined);
    });

    if (child.stdin) {
      child.stdin.end(document.getText());
    }
  });
}

function semanticTokensFromSnapshot(snapshot) {
  const builder = new vscode.SemanticTokensBuilder(semanticLegend);
  const tokens = snapshot.semantic_tokens?.tokens ?? [];
  for (const token of tokens) {
    const tokenType = SEMANTIC_TOKEN_TYPES.indexOf(token.type);
    if (tokenType < 0 || token.length <= 0) {
      continue;
    }
    builder.push(
      token.line,
      token.start,
      token.length,
      tokenType,
      semanticModifierBits(token.modifiers ?? [])
    );
  }
  return builder.build();
}

class EngDocumentSymbolProvider {
  constructor(context) {
    this.context = context;
  }

  async provideDocumentSymbols(document, cancellationToken) {
    if (!isEngDocument(document)) {
      return [];
    }
    const snapshot = await snapshotDocumentSource(document, this.context, cancellationToken);
    if (!snapshot) {
      return [];
    }
    reviewCache.set(document.uri.fsPath, snapshot);
    return documentSymbolsFromSnapshot(snapshot);
  }
}

class EngFoldingRangeProvider {
  constructor(context) {
    this.context = context;
  }

  async provideFoldingRanges(document, _context, cancellationToken) {
    if (!isEngDocument(document)) {
      return [];
    }
    const snapshot = await snapshotDocumentSource(document, this.context, cancellationToken);
    if (!snapshot) {
      return [];
    }
    reviewCache.set(document.uri.fsPath, snapshot);
    return foldingRangesFromSnapshot(snapshot);
  }
}

function documentSymbolsFromSnapshot(snapshot) {
  return (snapshot.document_symbols ?? [])
    .map(documentSymbolFromSnapshot)
    .filter((symbol) => symbol !== undefined);
}

function documentSymbolFromSnapshot(symbol) {
  if (!symbol?.name) {
    return undefined;
  }
  const range = vscodeRangeFromLsp(symbol.range);
  const selectionRange = vscodeRangeFromLsp(symbol.selectionRange) ?? range;
  if (!range || !selectionRange) {
    return undefined;
  }
  const documentSymbol = new vscode.DocumentSymbol(
    symbol.name,
    symbol.detail ?? "",
    symbolKindFromLsp(symbol.kind),
    range,
    selectionRange
  );
  for (const child of symbol.children ?? []) {
    const childSymbol = documentSymbolFromSnapshot(child);
    if (childSymbol) {
      documentSymbol.children.push(childSymbol);
    }
  }
  return documentSymbol;
}

function foldingRangesFromSnapshot(snapshot) {
  return (snapshot.folding_ranges ?? [])
    .map(foldingRangeFromSnapshot)
    .filter((range) => range !== undefined);
}

function foldingRangeFromSnapshot(range) {
  const startLine = range?.startLine;
  const endLine = range?.endLine;
  if (!Number.isInteger(startLine) || !Number.isInteger(endLine) || endLine <= startLine) {
    return undefined;
  }
  const kind = foldingRangeKindFromLsp(range.kind);
  if (kind) {
    return new vscode.FoldingRange(startLine, endLine, kind);
  }
  return new vscode.FoldingRange(startLine, endLine);
}

function vscodeRangeFromLsp(range) {
  const startLine = range?.start?.line;
  const startCharacter = range?.start?.character;
  const endLine = range?.end?.line;
  const endCharacter = range?.end?.character;
  if (
    !Number.isInteger(startLine) ||
    !Number.isInteger(startCharacter) ||
    !Number.isInteger(endLine) ||
    !Number.isInteger(endCharacter)
  ) {
    return undefined;
  }
  return new vscode.Range(startLine, startCharacter, endLine, endCharacter);
}

function semanticModifierBits(modifiers) {
  let bits = 0;
  for (const modifier of modifiers) {
    const index = SEMANTIC_TOKEN_MODIFIERS.indexOf(modifier);
    if (index >= 0) {
      bits |= 1 << index;
    }
  }
  return bits;
}

function symbolKindFromLsp(kind) {
  if (typeof kind === "number" && kind >= 1 && kind <= 26) {
    return kind - 1;
  }
  switch (kind) {
    case "module":
      return vscode.SymbolKind.Module;
    case "class":
      return vscode.SymbolKind.Class;
    case "method":
      return vscode.SymbolKind.Method;
    case "property":
      return vscode.SymbolKind.Property;
    case "interface":
      return vscode.SymbolKind.Interface;
    case "function":
      return vscode.SymbolKind.Function;
    case "variable":
      return vscode.SymbolKind.Variable;
    case "constant":
      return vscode.SymbolKind.Constant;
    case "object":
      return vscode.SymbolKind.Object;
    case "key":
      return vscode.SymbolKind.Key;
    case "struct":
      return vscode.SymbolKind.Struct;
    case "operator":
      return vscode.SymbolKind.Operator;
    case "typeParameter":
      return vscode.SymbolKind.TypeParameter;
    default:
      return vscode.SymbolKind.Variable;
  }
}

function foldingRangeKindFromLsp(kind) {
  switch (kind) {
    case "comment":
      return vscode.FoldingRangeKind.Comment;
    case "imports":
      return vscode.FoldingRangeKind.Imports;
    case "region":
      return vscode.FoldingRangeKind.Region;
    default:
      return undefined;
  }
}

function completionKindFromLsp(kind) {
  if (typeof kind === "number" && kind >= 1 && kind <= 25) {
    return kind - 1;
  }
  switch (kind) {
    case "method":
      return vscode.CompletionItemKind.Method;
    case "function":
      return vscode.CompletionItemKind.Function;
    case "variable":
      return vscode.CompletionItemKind.Variable;
    case "property":
      return vscode.CompletionItemKind.Property;
    case "class":
      return vscode.CompletionItemKind.Class;
    case "stdlib":
      return vscode.CompletionItemKind.Module;
    case "unit":
      return vscode.CompletionItemKind.Unit;
    case "value":
      return vscode.CompletionItemKind.Value;
    case "keyword":
      return vscode.CompletionItemKind.Keyword;
    default:
      return vscode.CompletionItemKind.Text;
  }
}

class EngHoverProvider {
  provideHover(document, position) {
    const review = reviewCache.get(document.uri.fsPath);
    if (!review) {
      return undefined;
    }
    const wordRange = document.getWordRangeAtPosition(position, /[A-Za-z_][A-Za-z0-9_]*/);
    const word = wordRange ? document.getText(wordRange) : "";
    const line = position.line + 1;
    const hover =
      (review.hover_hints ?? []).find((item) => item.line === line && item.name === word) ??
      (review.hovers ?? []).find((item) => item.line === line && item.name === word) ??
      (review.type_info ?? []).find((item) => item.name === word);
    if (!hover) {
      return undefined;
    }

    if (hover.contents?.value) {
      const markdown = new vscode.MarkdownString(hover.contents.value);
      markdown.isTrusted = false;
      return new vscode.Hover(markdown, wordRange);
    }

    const markdown = new vscode.MarkdownString();
    markdown.isTrusted = false;
    markdown.appendMarkdown(`**${hover.name ?? word}**\n\n`);
    markdown.appendMarkdown(`${hover.detail ?? "EngLang symbol"}\n\n`);
    if (hover.quantity_kind) {
      markdown.appendMarkdown(`Quantity: \`${hover.quantity_kind}\`\n\n`);
    }
    if (hover.display_unit) {
      markdown.appendMarkdown(`Display unit: \`${hover.display_unit}\`\n\n`);
    }
    if (hover.canonical_unit) {
      markdown.appendMarkdown(`Canonical unit: \`${hover.canonical_unit}\`\n\n`);
    }
    if (hover.dimension) {
      markdown.appendMarkdown(`Dimension: \`${hover.dimension}\``);
    }
    return new vscode.Hover(markdown, wordRange);
  }
}

class EngCompletionProvider {
  constructor(context) {
    this.context = context;
  }

  async provideCompletionItems(document, position, cancellationToken) {
    const items = [];
    const seen = new Set();
    const completionPayload =
      (await completionSnapshotForPosition(document, position, this.context, cancellationToken)) ??
      reviewCache.get(document.uri.fsPath);

    for (const completion of completionPayload?.completions ?? []) {
      const item = new vscode.CompletionItem(
        completion.label,
        completionKindFromLsp(completion.kind)
      );
      item.detail = completion.detail;
      if (completion.documentation) {
        item.documentation = completion.documentation;
      }
      addCompletion(items, seen, item);
    }

    return items;
  }
}

class EngCodeActionProvider {
  provideCodeActions(document, _range, context) {
    const actions = [];
    for (const diagnostic of context.diagnostics) {
      const code = diagnosticCode(diagnostic);
      if (code === "E-SYNTAX-DECL-001") {
        const action = replacementAction(
          document,
          diagnostic,
          ":=",
          "=",
          "Replace := with ="
        );
        if (action) {
          action.isPreferred = true;
          actions.push(action);
        }
      }
      if (code === "E-STRUCT-ARGS-001") {
        const action = replacementAction(
          document,
          diagnostic,
          "struct Args",
          "args",
          "Replace struct Args with args"
        );
        if (action) {
          action.isPreferred = true;
          actions.push(action);
        }
      }
      if (code === "E-SCRIPT-001") {
        const action = removeScriptWrapperAction(document, diagnostic);
        if (action) {
          action.isPreferred = true;
          actions.push(action);
        }
      }
    }
    return actions;
  }
}

function diagnosticCode(diagnostic) {
  if (typeof diagnostic.code === "string") {
    return diagnostic.code;
  }
  return diagnostic.code?.value;
}

function replacementAction(document, diagnostic, search, replacement, title) {
  const line = document.lineAt(diagnostic.range.start.line);
  const index = line.text.indexOf(search);
  if (index < 0) {
    return undefined;
  }
  const action = new vscode.CodeAction(title, vscode.CodeActionKind.QuickFix);
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(line.lineNumber, index, line.lineNumber, index + search.length),
    replacement
  );
  return action;
}

function removeScriptWrapperAction(document, diagnostic) {
  const startLineNumber = diagnostic.range.start.line;
  if (startLineNumber < 0 || startLineNumber >= document.lineCount) {
    return undefined;
  }
  const startLine = document.lineAt(startLineNumber);
  if (!/^\s*script(?:\s+[A-Za-z_][A-Za-z0-9_]*)?\s*\{\s*$/.test(startLine.text)) {
    return undefined;
  }
  const endLineNumber = findMatchingBlockEnd(document, startLineNumber);
  if (endLineNumber === undefined || endLineNumber <= startLineNumber) {
    return undefined;
  }
  const endLine = document.lineAt(endLineNumber);
  if (endLine.text.trim() !== "}") {
    return undefined;
  }

  const action = new vscode.CodeAction(
    "Promote script body to top-level workflow",
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.delete(document.uri, fullLineRange(document, endLineNumber));
  action.edit.delete(document.uri, fullLineRange(document, startLineNumber));
  return action;
}

function findMatchingBlockEnd(document, startLineNumber) {
  let depth = 0;
  for (let lineNumber = startLineNumber; lineNumber < document.lineCount; lineNumber += 1) {
    const text = stripLineComment(document.lineAt(lineNumber).text);
    for (const char of text) {
      if (char === "{") {
        depth += 1;
      } else if (char === "}") {
        depth -= 1;
        if (depth === 0) {
          return lineNumber;
        }
      }
    }
  }
  return undefined;
}

function stripLineComment(text) {
  const index = text.indexOf("#");
  return index >= 0 ? text.slice(0, index) : text;
}

function fullLineRange(document, lineNumber) {
  const line = document.lineAt(lineNumber);
  if (lineNumber + 1 < document.lineCount) {
    return new vscode.Range(lineNumber, 0, lineNumber + 1, 0);
  }
  return new vscode.Range(lineNumber, 0, lineNumber, line.text.length);
}

function addCompletion(items, seen, item) {
  const label = typeof item.label === "string" ? item.label : item.label?.label;
  if (!label || seen.has(label)) {
    return;
  }
  seen.add(label);
  items.push(item);
}

function isEngDocument(document) {
  return document.languageId === LANGUAGE_ID || document.fileName.endsWith(".eng");
}

function workspaceRoot(document) {
  return vscode.workspace.getWorkspaceFolder(document.uri)?.uri.fsPath ?? path.dirname(document.uri.fsPath);
}

function currentWorkspaceRoot() {
  const document = vscode.window.activeTextEditor?.document;
  if (document) {
    const folder = vscode.workspace.getWorkspaceFolder(document.uri);
    if (folder) {
      return folder.uri.fsPath;
    }
    if (isEngDocument(document)) {
      return path.dirname(document.uri.fsPath);
    }
  }
  return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
}

function findExampleFiles(root) {
  const groups = [
    { kind: "official", dir: path.join(root, "examples", "official") },
    { kind: "workflow", dir: path.join(root, "examples", "workflows") }
  ];
  const examples = [];
  for (const group of groups) {
    collectExampleMainFiles(group.dir, root, group.kind, examples);
  }
  return examples.sort((left, right) => left.relativePath.localeCompare(right.relativePath));
}

function collectExampleMainFiles(dir, root, kind, examples) {
  let entries;
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }

  for (const entry of entries) {
    const entryPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (!entry.name.startsWith(".") && entry.name !== "build" && entry.name !== "target") {
        collectExampleMainFiles(entryPath, root, kind, examples);
      }
      continue;
    }
    if (!entry.isFile() || entry.name !== "main.eng") {
      continue;
    }
    const relativePath = path.relative(root, entryPath).replace(/[\\/]/g, "/");
    examples.push({
      kind,
      path: entryPath,
      relativePath,
      label: relativePath.replace(/^examples\//, "").replace(/\/main\.eng$/, "")
    });
  }
}

function engConfig(document) {
  const uri = document?.uri;
  return uri
    ? vscode.workspace.getConfiguration("englang", uri)
    : vscode.workspace.getConfiguration("englang");
}

function executionProfile(document) {
  const configured = engConfig(document).get("executionProfile", "normal");
  return EXECUTION_PROFILES.some((profile) => profile.id === configured)
    ? configured
    : "normal";
}

function diagnosticsBackend(document) {
  return engConfig(document).get("diagnosticsBackend", "eng-cli");
}

function findRuntime(context, document) {
  const configPath = engConfig(document).get("runtimePath", "");
  const candidates = [
    configPath,
    path.join(context.extensionPath, "bin", "eng.exe"),
    path.join(context.extensionPath, "..", "..", "eng.exe"),
    path.join(workspaceRoot(document), "eng.exe"),
    path.join(workspaceRoot(document), "target", "debug", "eng.exe"),
    path.join(workspaceRoot(document), "target", "release", "eng.exe")
  ].filter((candidate) => candidate && candidate.trim().length > 0);

  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  return "eng.exe";
}

function findLspRuntime(context, document) {
  const configPath = engConfig(document).get("lspPath", "");
  const candidates = [
    configPath,
    path.join(context.extensionPath, "bin", "eng-lsp.exe"),
    path.join(context.extensionPath, "..", "..", "eng-lsp.exe"),
    path.join(workspaceRoot(document), "eng-lsp.exe"),
    path.join(workspaceRoot(document), "target", "debug", "eng-lsp.exe"),
    path.join(workspaceRoot(document), "target", "release", "eng-lsp.exe")
  ].filter((candidate) => candidate && candidate.trim().length > 0);

  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  return "eng-lsp.exe";
}

function firstLineRange(document) {
  const line = document.lineAt(0);
  return new vscode.Range(0, 0, 0, Math.max(1, line.text.length));
}

module.exports = {
  activate,
  deactivate
};
