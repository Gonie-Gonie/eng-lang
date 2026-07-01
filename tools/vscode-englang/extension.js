const cp = require("child_process");
const fs = require("fs");
const path = require("path");
const vscode = require("vscode");

const LANGUAGE_ID = "englang";
const reviewCache = new Map();
let output;

const QUANTITIES = [
  ["AbsoluteTemperature", "K", "Affine absolute thermodynamic temperature."],
  ["TemperatureDelta", "K", "Temperature interval or difference."],
  ["Length", "m", "Linear distance."],
  ["Area", "m2", "Area."],
  ["Volume", "m3", "Volume."],
  ["Conductance", "W/K", "Thermal conductance."],
  ["HeatCapacity", "J/K", "Lumped heat capacity."],
  ["SpecificHeat", "J/kg/K", "Specific heat capacity."],
  ["HeatRate", "W", "Thermal power or heat flow rate."],
  ["ElectricPower", "W", "Electrical power."],
  ["MechanicalPower", "W", "Mechanical shaft or fluid power."],
  ["Energy", "J", "Energy, heat, or work quantity."],
  ["Mass", "kg", "Mass."],
  ["MassFlowRate", "kg/s", "Mass flow rate."],
  ["Pressure", "Pa", "Pressure."],
  ["Irradiance", "W/m2", "Power per unit area."],
  ["PeopleDensity", "person/m2", "People per unit floor area."],
  ["DimensionlessNumber", "1", "Plain dimensionless scalar."],
  ["Ratio", "1", "Dimensionless ratio."],
  ["ReynoldsNumber", "1", "Dimensionless Reynolds number."]
];

const UNITS = [
  "K",
  "degC",
  "m",
  "cm",
  "mm",
  "m2",
  "m3",
  "W",
  "kW",
  "J",
  "kJ",
  "MJ",
  "Wh",
  "kWh",
  "W/K",
  "J/K",
  "kJ/K",
  "J/kg/K",
  "Pa",
  "kPa",
  "kg",
  "kg/s",
  "W/m2",
  "person/m2",
  "s",
  "min",
  "h",
  "1"
];

const TYPES = [
  ["Path", "Generic filesystem path."],
  ["FilePath", "Generic file path."],
  ["DirectoryPath", "Directory path."],
  ["CsvFile", "CSV file path."],
  ["TextFile", "UTF-8 text file path."],
  ["JsonFile", "JSON file path."],
  ["TomlFile", "TOML file path."],
  ["Url", "HTTP or HTTPS URL."],
  ["Secret[String]", "Redacted string value for credentials or tokens."],
  ["Optional[T]", "Optional value that may be missing or none."],
  ["Bool", "Boolean value."],
  ["Int", "Integer value."],
  ["Float", "Floating-point value."],
  ["Number", "Dimensionless numeric value."],
  ["String", "String value."],
  ["Date", "Calendar date."],
  ["DateTime", "Timestamp value."],
  ["Table[T]", "Typed table value."],
  ["TimeSeries[T]", "Typed time-indexed series value."],
  ["ProcessResult", "External command result metadata."],
  ["Report", "Report artifact request metadata."]
];

const KEYWORDS = [
  "schema",
  "args",
  "const",
  "use",
  "system",
  "domain",
  "across",
  "through",
  "conservation",
  "component",
  "port",
  "connect",
  "package",
  "version",
  "state",
  "parameter",
  "input",
  "equation",
  "promote",
  "from",
  "as",
  "read",
  "text",
  "json",
  "toml",
  "csv",
  "policy",
  "missing",
  "constraints",
  "between",
  "is",
  "none",
  "true",
  "false",
  "interpolate",
  "monotonic",
  "where",
  "with",
  "run",
  "command",
  "render",
  "template",
  "open",
  "sqlite",
  "to",
  "show",
  "over",
  "by",
  "using",
  "mode",
  "append",
  "insert",
  "upsert",
  "return",
  "print",
  "log",
  "test",
  "assert",
  "golden",
  "matches",
  "within",
  "export",
  "write",
  "copy",
  "move",
  "delete",
  "plot",
  "line",
  "bar",
  "histogram",
  "check",
  "coverage",
  "sample",
  "grid",
  "random",
  "lhs",
  "uniform",
  "select_first_row",
  "filter",
  "select",
  "derive",
  "sort",
  "require_one",
  "train_test_split",
  "regression",
  "mlp",
  "evaluate",
  "model_card",
  "leakage_lint",
  "predict",
  "der",
  "eq",
  "integrate",
  "mean",
  "max",
  "median",
  "std",
  "duration_above"
];

function activate(context) {
  output = vscode.window.createOutputChannel("EngLang");
  const diagnostics = vscode.languages.createDiagnosticCollection("englang");
  context.subscriptions.push(output, diagnostics);

  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((document) => maybeCheck(document, diagnostics, context)),
    vscode.workspace.onDidSaveTextDocument((document) => maybeCheck(document, diagnostics, context)),
    vscode.workspace.onDidCloseTextDocument((document) => diagnostics.delete(document.uri)),
    vscode.commands.registerCommand("englang.checkFile", () => checkActiveFile(diagnostics, context)),
    vscode.commands.registerCommand("englang.runFile", () => runActiveFile(context)),
    vscode.commands.registerCommand("englang.openReport", openLastReport),
    vscode.languages.registerHoverProvider(LANGUAGE_ID, new EngHoverProvider()),
    vscode.languages.registerCompletionItemProvider(LANGUAGE_ID, new EngCompletionProvider(), ":", " ", "[")
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
  checkDocument(document, diagnostics, context);
}

async function checkActiveFile(diagnostics, context) {
  const document = vscode.window.activeTextEditor?.document;
  if (!document || !isEngDocument(document)) {
    vscode.window.showWarningMessage("Open an EngLang .eng file first.");
    return;
  }
  if (document.isDirty) {
    await document.save();
  }
  await checkDocument(document, diagnostics, context);
}

function checkDocument(document, diagnostics, context) {
  const backend = diagnosticsBackend(document);
  const runtime = backend === "lsp-snapshot" ? findLspRuntime(context, document) : findRuntime(context, document);
  const args = backend === "lsp-snapshot" ? ["--snapshot", document.uri.fsPath] : ["ide-check", document.uri.fsPath];
  const cwd = workspaceRoot(document);
  output.appendLine(`${backend} check ${document.uri.fsPath}`);

  cp.execFile(
    runtime,
    args,
    { cwd, maxBuffer: 10 * 1024 * 1024 },
    (error, stdout, stderr) => {
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
  );
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
    ["run", document.uri.fsPath],
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

async function openLastReport() {
  const folder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (!folder) {
    vscode.window.showWarningMessage("Open an EngLang workspace folder first.");
    return;
  }
  const reportPath = path.join(folder, "build", "result", "report.html");
  if (!fs.existsSync(reportPath)) {
    vscode.window.showWarningMessage("No build/result/report.html found yet.");
    return;
  }
  await vscode.env.openExternal(vscode.Uri.file(reportPath));
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
  provideCompletionItems(document) {
    const items = [];
    const seen = new Set();
    const review = reviewCache.get(document.uri.fsPath);

    for (const completion of review?.completions ?? []) {
      const item = new vscode.CompletionItem(completion.label, completion.kind ?? vscode.CompletionItemKind.Text);
      item.detail = completion.detail;
      addCompletion(items, seen, item);
    }

    for (const keyword of KEYWORDS) {
      const item = new vscode.CompletionItem(keyword, vscode.CompletionItemKind.Keyword);
      item.detail = "EngLang keyword";
      addCompletion(items, seen, item);
    }

    for (const [quantity, canonicalUnit, description] of QUANTITIES) {
      const item = new vscode.CompletionItem(quantity, vscode.CompletionItemKind.Class);
      item.detail = `quantity kind, canonical unit ${canonicalUnit}`;
      item.documentation = description;
      addCompletion(items, seen, item);
    }

    for (const [typeName, description] of TYPES) {
      const item = new vscode.CompletionItem(typeName, vscode.CompletionItemKind.Class);
      item.detail = "EngLang type";
      item.documentation = description;
      addCompletion(items, seen, item);
    }

    for (const unit of UNITS) {
      const item = new vscode.CompletionItem(unit, vscode.CompletionItemKind.Unit);
      item.detail = "EngLang unit";
      addCompletion(items, seen, item);
    }

    addCompletion(items, seen, snippet("schema csv", "schema ${1:Sensor} {\n    ${2:time}: DateTime [iso8601]\n    ${3:heat}: HeatRate [kW]\n}", "Typed CSV schema"));
    addCompletion(items, seen, snippet("args block", "args {\n    ${1:input}: CsvFile = file(\"${2:data/sensor.csv}\")\n}", "Root CLI argument block"));
    addCompletion(items, seen, snippet("print log", "print \"${1:case ready}\"\nlog info \"${2:Q = {Q: .2 kW}}\"", "Direct output plus structured log message"));
    addCompletion(items, seen, snippet("run command", "${1:process_result} = run command \"${2:cmd}\"\nwith {\n    args = [\"${3:/C}\", \"${4:echo}\", \"${5:ok}\"]\n}", "Run an external command and capture a ProcessResult"));
    addCompletion(items, seen, snippet("test block", "test \"${1:summary values}\" {\n    assert ${2:mean_Q} > ${3:0 kW}\n    assert ${4:E_coil} == ${5:1.26 kWh} within ${6:0.02 kWh}\n    golden \"${7:summary.csv}\" matches file(\"${8:golden/summary.csv}\")\n}", "Run unit-aware assertions and golden artifact checks"));
    addCompletion(items, seen, snippet("system thermal", "system ${1:Room} {\n    state ${2:T}: AbsoluteTemperature = ${3:20 degC}\n    parameter ${4:C}: HeatCapacity = ${5:1200 kJ/K}\n    parameter ${6:UA}: Conductance = ${7:250 W/K}\n    input ${8:T_out}: AbsoluteTemperature = ${9:10 degC}\n    input ${10:Q_internal}: HeatRate = ${11:500 W}\n    equation energy_balance:\n        ${4:C} * der(${2:T}) eq ${6:UA} * (${8:T_out} - ${2:T}) + ${10:Q_internal}\n}", "First-order thermal system"));
    addCompletion(items, seen, snippet("domain ports", "domain ${1:Thermal} package \"${2:eng.std.domains.thermal}\" version \"${3:0.1.0}\" {\n    across ${4:T}: AbsoluteTemperature [degC]\n    through ${5:Q}: HeatRate [kW]\n    conservation sum(${5:Q}) = 0\n}\n\ndomain ${6:Fluid}[${7:Medium M}] package \"${8:eng.std.domains.fluid}\" version \"${9:0.1.0}\" {\n    across ${10:height}: Length [m]\n    through ${11:m_dot}: MassFlowRate [kg/s]\n    conservation sum(${11:m_dot}) = 0\n}\n\ndomain ${12:MechanicalNode}[${13:Frame F}, ${14:Axis DOF}] package \"${15:eng.std.domains.mechanical}\" version \"${16:0.1.0}\" {\n    across ${17:x}: Length [m]\n    through ${18:P}: MechanicalPower [W]\n    conservation sum(${18:P}) = 0\n}\n\ncomponent ${19:RoomBoundary} {\n    port ${20:heat}: ${1:Thermal}\n}\n\ncomponent ${21:SupplyPipe} {\n    port ${22:inlet}: ${6:Fluid}[${23:Water}]\n    port ${24:outlet}: ${6:Fluid}[${23:Water}]\n}\n\ncomponent ${25:ShaftA} {\n    port ${26:shaft}: ${12:MechanicalNode}[${27:World}, ${28:X}]\n}\n\ncomponent ${29:ShaftB} {\n    port ${30:shaft}: ${12:MechanicalNode}[${27:World}, ${28:X}]\n}\n\nconnect ${21:SupplyPipe}.${22:inlet} -> ${21:SupplyPipe}.${24:outlet}\nconnect ${25:ShaftA}.${26:shaft} -> ${29:ShaftB}.${30:shaft}", "Domain package/version, generic ports, and connection"));

    return items;
  }
}

function addCompletion(items, seen, item) {
  const label = typeof item.label === "string" ? item.label : item.label?.label;
  if (!label || seen.has(label)) {
    return;
  }
  seen.add(label);
  items.push(item);
}

function snippet(label, body, detail) {
  const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Snippet);
  item.insertText = new vscode.SnippetString(body);
  item.detail = detail;
  return item;
}

function isEngDocument(document) {
  return document.languageId === LANGUAGE_ID || document.fileName.endsWith(".eng");
}

function workspaceRoot(document) {
  return vscode.workspace.getWorkspaceFolder(document.uri)?.uri.fsPath ?? path.dirname(document.uri.fsPath);
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
