const cp = require("child_process");
const fs = require("fs");
const path = require("path");
const vscode = require("vscode");

const LANGUAGE_ID = "englang";
const CHECK_DEBOUNCE_MS = 350;
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
  if (document.isDirty) {
    await document.save();
  }

  const runtime = findRuntime(context, document);
  const cwd = workspaceRoot(document);
  output.show(true);
  output.appendLine(`run ${document.uri.fsPath}`);
  cp.execFile(
    runtime,
    ["run", document.uri.fsPath, "--save-artifacts"],
    { cwd, maxBuffer: 10 * 1024 * 1024 },
    (error, stdout, stderr) => {
      if (stdout) {
        output.appendLine(stdout.trim());
      }
      if (stderr) {
        output.appendLine(stderr.trim());
      }
      if (error) {
        vscode.window.showErrorMessage("EngLang run failed. See the EngLang output panel.");
      } else {
        vscode.window.showInformationMessage("EngLang run completed.");
      }
    }
  );
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

async function openLastRunArtifact(artifactId) {
  const artifact = LAST_RUN_ARTIFACTS.find((item) => item.id === artifactId);
  if (!artifact) {
    vscode.window.showWarningMessage(`Unknown EngLang artifact: ${artifactId}`);
    return;
  }
  const root = currentWorkspaceRoot();
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

function diagnosticsBackend(document) {
  return vscode.workspace.getConfiguration("englang", document.uri).get("diagnosticsBackend", "eng-cli");
}

function findRuntime(context, document) {
  const configPath = vscode.workspace.getConfiguration("englang", document.uri).get("runtimePath", "");
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
  const configPath = vscode.workspace.getConfiguration("englang", document.uri).get("lspPath", "");
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
