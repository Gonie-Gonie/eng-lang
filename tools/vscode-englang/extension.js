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
  ["Conductance", "W/K", "Thermal conductance."],
  ["HeatCapacity", "J/K", "Lumped heat capacity."],
  ["SpecificHeat", "J/kg/K", "Specific heat capacity."],
  ["HeatRate", "W", "Thermal power or heat flow rate."],
  ["ElectricPower", "W", "Electrical power."],
  ["MechanicalPower", "W", "Mechanical shaft or fluid power."],
  ["Energy", "J", "Energy, heat, or work quantity."],
  ["MassFlowRate", "kg/s", "Mass flow rate."],
  ["Ratio", "1", "Dimensionless ratio."],
  ["ReynoldsNumber", "1", "Dimensionless Reynolds number."]
];

const UNITS = [
  "K",
  "degC",
  "m",
  "cm",
  "mm",
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
  "kg/s",
  "s",
  "min",
  "h",
  "1"
];

const KEYWORDS = [
  "schema",
  "script",
  "struct",
  "system",
  "state",
  "parameter",
  "input",
  "equation",
  "promote",
  "from",
  "policy",
  "missing",
  "where",
  "return",
  "plot",
  "line",
  "bar",
  "histogram",
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
  const runtime = findRuntime(context, document);
  const cwd = workspaceRoot(document);
  output.appendLine(`check ${document.uri.fsPath}`);

  cp.execFile(
    runtime,
    ["ide-check", document.uri.fsPath],
    { cwd, maxBuffer: 10 * 1024 * 1024 },
    (error, stdout, stderr) => {
      if (stderr && stderr.trim().length > 0) {
        output.appendLine(stderr.trim());
      }

      let review;
      try {
        review = JSON.parse(stdout);
      } catch (parseError) {
        output.appendLine(`Unable to parse eng ide-check output: ${parseError.message}`);
        if (error) {
          output.appendLine(error.message);
        }
        diagnostics.set(document.uri, [
          new vscode.Diagnostic(
            firstLineRange(document),
            "EngLang runtime did not return IDE JSON. Check englang.runtimePath.",
            vscode.DiagnosticSeverity.Error
          )
        ]);
        return;
      }

      reviewCache.set(document.uri.fsPath, review);
      diagnostics.set(document.uri, toDiagnostics(document, review));
      const errors = review.diagnostics?.filter((item) => item.severity === "error").length ?? 0;
      const warnings = review.diagnostics?.filter((item) => item.severity === "warning").length ?? 0;
      output.appendLine(`diagnostics: ${errors} error(s), ${warnings} warning(s)`);
    }
  );
}

function toDiagnostics(document, review) {
  return (review.diagnostics ?? []).map((item) => {
    const line = Math.max(0, (item.line ?? 1) - 1);
    const textLine = document.lineAt(Math.min(line, document.lineCount - 1));
    const range = new vscode.Range(line, 0, line, Math.max(1, textLine.text.length));
    const severity =
      item.severity === "error"
        ? vscode.DiagnosticSeverity.Error
        : item.severity === "warning"
          ? vscode.DiagnosticSeverity.Warning
          : vscode.DiagnosticSeverity.Information;
    const diagnostic = new vscode.Diagnostic(range, item.message, severity);
    diagnostic.code = item.code;
    diagnostic.source = "eng";
    if (item.help) {
      diagnostic.message = `${item.message}\n${item.help}`;
    }
    return diagnostic;
  });
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
  const entry = vscode.workspace.getConfiguration("englang", document.uri).get("runEntry", "main");
  output.show(true);
  output.appendLine(`run ${document.uri.fsPath} --entry ${entry}`);
  cp.execFile(
    runtime,
    ["run", document.uri.fsPath, "--entry", entry],
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
      (review.type_info ?? []).find((item) => item.name === word);
    if (!hover) {
      return undefined;
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
  provideCompletionItems() {
    const items = [];

    for (const keyword of KEYWORDS) {
      const item = new vscode.CompletionItem(keyword, vscode.CompletionItemKind.Keyword);
      item.detail = "EngLang keyword";
      items.push(item);
    }

    for (const [quantity, canonicalUnit, description] of QUANTITIES) {
      const item = new vscode.CompletionItem(quantity, vscode.CompletionItemKind.Class);
      item.detail = `quantity kind, canonical unit ${canonicalUnit}`;
      item.documentation = description;
      items.push(item);
    }

    for (const unit of UNITS) {
      const item = new vscode.CompletionItem(unit, vscode.CompletionItemKind.Unit);
      item.detail = "EngLang unit";
      items.push(item);
    }

    items.push(snippet("schema csv", "schema ${1:Sensor} {\n    ${2:time}: DateTime [iso8601]\n    ${3:heat}: HeatRate [kW]\n}", "Typed CSV schema"));
    items.push(snippet("script main", "script main() -> Report {\n    ${1:value} = ${2:1 kW}\n    return plot line ${1:value}\n}", "Main report script"));
    items.push(snippet("system thermal", "system ${1:Room} {\n    state ${2:T}: AbsoluteTemperature = ${3:20 degC}\n    parameter ${4:C}: HeatCapacity = ${5:1200 kJ/K}\n    parameter ${6:UA}: Conductance = ${7:250 W/K}\n    input ${8:T_out}: AbsoluteTemperature = ${9:10 degC}\n    input ${10:Q_internal}: HeatRate = ${11:500 W}\n    equation energy_balance:\n        ${4:C} * der(${2:T}) eq ${6:UA} * (${8:T_out} - ${2:T}) + ${10:Q_internal}\n}", "First-order thermal system"));

    return items;
  }
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

function firstLineRange(document) {
  const line = document.lineAt(0);
  return new vscode.Range(0, 0, 0, Math.max(1, line.text.length));
}

module.exports = {
  activate,
  deactivate
};
