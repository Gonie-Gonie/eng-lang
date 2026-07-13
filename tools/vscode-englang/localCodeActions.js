const vscode = require("vscode");

const STATEMENT_ONLY_BINDING_CODES = new Set([
  "E-REPORT-BINDING-001",
  "E-VALIDATE-BINDING-001",
  "E-SIDE-EFFECT-BINDING-001",
  "E-BLOCK-BINDING-001",
  "E-STATEMENT-BINDING-001",
  "E-OPTION-BINDING-001"
]);

function localCodeActions(document, context, options = {}) {
  const actions = [];
  for (const diagnostic of context.diagnostics) {
    const code = diagnosticCode(diagnostic);
    if (code === "E-SYNTAX-DECL-001") {
      const action = replacementAction(document, diagnostic, ":=", "=", "Replace := with =");
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
    if (code === "E-EQ-BOOL-001") {
      const action = replacementAction(document, diagnostic, "==", "eq", "Replace == with eq");
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
    if (code === "W-QTY-AMBIG-001") {
      actions.push(...quantityAnnotationActions(document, diagnostic));
    }
    if (code === "W-STATS-SUM-001") {
      const action = heatRateSumAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (typeof code === "string" && code.startsWith("E-DIM-ADD-")) {
      actions.push(...missingUnitActions(document, diagnostic, options.unitLabels));
    }
    if (code === "E-PUBLIC-ANNOTATION-001") {
      const action = schemaAnnotationAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-FS-CONFIRM-001") {
      const action = fileMutationConfirmAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-FS-DELETE-001") {
      const action = recursiveDeleteAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-SAMPLING-SEED-MISSING") {
      const action = sampleSeedMissingAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "W-WITH-UNCERTAINTY-SEED-001") {
      const action = uncertaintySeedMissingAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-NET-INVALID-URL") {
      const action = absoluteHttpUrlAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-NET-BODY-METHOD") {
      const action = httpBodyMethodAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-NET-HASH-MISMATCH") {
      const action = expectedSha256Action(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "W-NET-FIXTURE-ALIAS") {
      const action = optionKeyReplacementAction(
        document,
        diagnostic,
        "fixture",
        "offline_response",
        "Rename fixture to offline_response"
      );
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "W-NET-RESPONSE-HASH-ALIAS") {
      const action = diagnosticRangeReplacementAction(
        document,
        diagnostic,
        "response_hash",
        "Rename hash to response_hash"
      );
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "W-NET-RESPONSE-STATUS-ALIAS") {
      const action = diagnosticRangeReplacementAction(
        document,
        diagnostic,
        "response_source",
        "Rename status to response_source"
      );
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "W-TABLE-LEGACY-SELECT-FIRST-ROW") {
      const action = selectFirstRowMigrationAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-IO-JSON-FIELD-ACCESS-001") {
      const action = jsonReadPromotionAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-WITH-OPTION-001") {
      const action = withOptionAliasAction(document, diagnostic, options.workflowOptionLabels);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-WITH-UNIT-001") {
      const action = removeIncompatibleDisplayUnitAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-PRINT-FMT-001" || code === "E-WRITE-FMT-001") {
      const action = closeUnterminatedInterpolationAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-PRINT-FMT-002" || code === "E-WRITE-FMT-002") {
      const action = removeEmptyInterpolationAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-PRINT-FMT-003" || code === "E-WRITE-FMT-003") {
      const action = removeInterpolationDisplayUnitAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-PRINT-FMT-004" || code === "E-WRITE-FMT-004") {
      const action = convertUnresolvedInterpolationAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-LOG-LEVEL-001") {
      const action = logLevelInfoAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (STATEMENT_ONLY_BINDING_CODES.has(code)) {
      const action = statementOnlyUnbindAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-PROCESS-BINDING-001") {
      const action = bindProcessResultAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-PROCESS-BINDING-002") {
      const action = uniqueProcessBindingAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-PROCESS-CMD-001") {
      const action = processCommandAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-ASSERT-001") {
      const action = wrapAssertionAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-GOLDEN-002") {
      const action = goldenExpectedFileAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-WHERE-FWD-001") {
      const action = reorderWhereLocalDefinitionAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-NAME-LOCAL-001") {
      const action = promoteWhereLocalAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (typeof code === "string" && code.startsWith("E-UNC-ARGS-")) {
      actions.push(...uncertaintyArgumentActions(document, diagnostic));
    }
    if (code === "E-UNC-SOURCE-001" || code === "E-UNC-SOURCE-002") {
      actions.push(...uncertaintySourceActions(document, diagnostic));
    }
    if (code === "E-UNC-DIRECT-COMPARE") {
      const action = uncertaintyDirectCompareAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-CMD-AMBIG-001") {
      const action = commandTargetParenthesesAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-STDLIB-MODULE-UNKNOWN") {
      const action = stdlibModuleReplacementAction(document, diagnostic, options.completionItems);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "W-STDLIB-MODULE-PLANNED" || code === "W-STDLIB-MODULE-INTERNAL") {
      const status = code === "W-STDLIB-MODULE-PLANNED" ? "planned" : "internal";
      const action = removeStdlibModuleImportAction(document, diagnostic, status);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    const modelOptionAction = modelOptionValueAction(document, diagnostic, code, options.workflowOptionLabels);
    if (modelOptionAction) {
      modelOptionAction.isPreferred = true;
      actions.push(modelOptionAction);
    }
    const optionAction = optionValueReplacementAction(
      document,
      diagnostic,
      optionQuickFix(code),
      options.workflowOptionLabels
    );
    if (optionAction) {
      optionAction.isPreferred = true;
      actions.push(optionAction);
    }
  }
  return actions;
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

function heatRateSumAction(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  const range = sumFunctionNameRange(line.text);
  if (!range) {
    return undefined;
  }
  const action = new vscode.CodeAction("Replace sum with integrate", vscode.CodeActionKind.QuickFix);
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(line.lineNumber, range.start, line.lineNumber, range.end),
    "integrate"
  );
  return action;
}

function sumFunctionNameRange(lineText) {
  const code = stripLineComment(lineText);
  let searchStart = 0;
  while (searchStart < code.length) {
    const start = code.indexOf("sum", searchStart);
    if (start < 0) {
      break;
    }
    const afterName = start + "sum".length;
    if (identifierBoundary(code, start, afterName)) {
      let open = afterName;
      while (open < code.length && /\s/.test(code[open])) {
        open += 1;
      }
      if (code[open] === "(") {
        return { start, end: afterName };
      }
    }
    searchStart = afterName;
  }
  return undefined;
}

function stdlibModuleReplacementAction(document, diagnostic, completionItems) {
  const unknown = stdlibModuleNameFromDiagnostic(diagnostic.message);
  if (!unknown) {
    return undefined;
  }
  const replacement = closestStdlibModuleName(unknown, completionItems);
  if (!replacement) {
    return undefined;
  }
  const line = document.lineAt(diagnostic.range.start.line);
  const index = line.text.indexOf(unknown);
  if (index < 0) {
    return undefined;
  }
  const action = new vscode.CodeAction(
    `Replace ${unknown} with ${replacement}`,
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(line.lineNumber, index, line.lineNumber, index + unknown.length),
    replacement
  );
  return action;
}

function removeStdlibModuleImportAction(document, diagnostic, status) {
  const moduleName = stdlibModuleNameFromDiagnostic(diagnostic.message);
  if (!moduleName) {
    return undefined;
  }
  const lineNumber = diagnostic.range.start.line;
  if (lineNumber < 0 || lineNumber >= document.lineCount) {
    return undefined;
  }
  const line = document.lineAt(lineNumber);
  if (!stripLineComment(line.text).includes(moduleName)) {
    return undefined;
  }
  const action = new vscode.CodeAction(
    `Remove ${status} stdlib module import`,
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.delete(document.uri, fullLineRange(document, lineNumber));
  return action;
}

function stdlibModuleNameFromDiagnostic(message) {
  const candidates = String(message ?? "").match(/`eng\.[A-Za-z0-9_.-]+`/g) ?? [];
  const last = candidates.at(-1);
  return last ? last.slice(1, -1) : undefined;
}

function closestStdlibModuleName(unknown, completionItems) {
  const moduleNames = stdlibModuleNamesFromCompletionItems(completionItems)
    .filter((name) => name !== unknown);
  let best;
  for (const name of moduleNames) {
    const distance = editDistance(unknown, name);
    if (!best || distance < best.distance || (distance === best.distance && name < best.name)) {
      best = { distance, name };
    }
  }
  if (!best) {
    return undefined;
  }
  return best.distance <= 2 || (best.distance <= 3 && unknown.length >= 8) ? best.name : undefined;
}

function stdlibModuleNamesFromCompletionItems(completionItems) {
  return Array.from(
    new Set(
      (Array.isArray(completionItems) ? completionItems : [])
        .map((completion) => completion?.label)
        .filter((label) => /^eng\.[A-Za-z0-9_.-]+$/.test(label ?? ""))
    )
  ).sort();
}

function editDistance(left, right) {
  const previous = Array.from({ length: right.length + 1 }, (_value, index) => index);
  const current = new Array(right.length + 1).fill(0);
  for (let leftIndex = 0; leftIndex < left.length; leftIndex += 1) {
    current[0] = leftIndex + 1;
    for (let rightIndex = 0; rightIndex < right.length; rightIndex += 1) {
      const substitution = previous[rightIndex] + (left[leftIndex] === right[rightIndex] ? 0 : 1);
      current[rightIndex + 1] = Math.min(
        previous[rightIndex + 1] + 1,
        current[rightIndex] + 1,
        substitution
      );
    }
    for (let index = 0; index < current.length; index += 1) {
      previous[index] = current[index];
    }
  }
  return previous[right.length];
}

function quantityAnnotationActions(document, diagnostic) {
  const details = ambiguousQuantityDetails(diagnostic.message);
  if (!details) {
    return [];
  }

  const line = document.lineAt(diagnostic.range.start.line);
  const assignment = new RegExp(`^(\\s*)(${escapeRegExp(details.name)})(\\s*=)`).exec(line.text);
  if (!assignment) {
    return [];
  }

  const startCharacter = assignment[1].length;
  const endCharacter = startCharacter + assignment[2].length + assignment[3].length;
  return details.candidates.map((candidate) => {
    const action = new vscode.CodeAction(
      `Annotate ${details.name} as ${candidate} [${details.unit}]`,
      vscode.CodeActionKind.QuickFix
    );
    action.diagnostics = [diagnostic];
    action.edit = new vscode.WorkspaceEdit();
    action.edit.replace(
      document.uri,
      new vscode.Range(line.lineNumber, startCharacter, line.lineNumber, endCharacter),
      `${details.name}: ${candidate} [${details.unit}] =`
    );
    return action;
  });
}

function ambiguousQuantityDetails(message) {
  const header = /`([^`]+)` has unit ([^,\s]+), but quantity kind is ambiguous\./.exec(message);
  const candidates = /Candidate quantity kinds:\s*([^.]+)\./.exec(message);
  if (!header || !candidates) {
    return undefined;
  }
  const candidateList = candidates[1]
    .split(",")
    .map((candidate) => candidate.trim())
    .filter((candidate) => /^[A-Za-z_][A-Za-z0-9_]*$/.test(candidate));
  if (candidateList.length === 0) {
    return undefined;
  }
  return {
    name: header[1],
    unit: header[2],
    candidates: candidateList
  };
}

function missingUnitActions(document, diagnostic, unitLabels) {
  const line = document.lineAt(diagnostic.range.start.line);
  const unit = missingUnitHint(diagnostic.message, line.text, unitLabelSet(unitLabels));
  if (!unit) {
    return [];
  }

  return bareNumericRanges(line.text, unitLabelSet(unitLabels)).map((range) => {
    const literal = line.text.slice(range.start, range.end);
    const action = new vscode.CodeAction(
      `Add unit ${unit} to ${literal}`,
      vscode.CodeActionKind.QuickFix
    );
    action.diagnostics = [diagnostic];
    action.edit = new vscode.WorkspaceEdit();
    action.edit.insert(document.uri, new vscode.Position(line.lineNumber, range.end), ` ${unit}`);
    return action;
  });
}

function missingUnitHint(message, lineText, unitLabels) {
  const fromHelp =
    /(?:such as|write)\s+`([^`]+)`/.exec(message)?.[1] ??
    /unit such as\s+`([^`]+)`/.exec(message)?.[1];
  if (isUnitHint(fromHelp, unitLabels)) {
    return fromHelp;
  }
  return firstUnitOnLine(lineText, unitLabels);
}

function firstUnitOnLine(lineText, unitLabels) {
  const unitLiteral = /\b\d+(?:\.\d+)?\s+([^\s,;)\]}]+)/.exec(lineText);
  if (isUnitHint(unitLiteral?.[1], unitLabels)) {
    return unitLiteral[1];
  }
  const bracketUnit = /\[([^\]\s]+)\]/.exec(lineText);
  if (isUnitHint(bracketUnit?.[1], unitLabels)) {
    return bracketUnit[1];
  }
  return undefined;
}

function isUnitHint(value, unitLabels) {
  if (typeof value !== "string" || value.length === 0) {
    return false;
  }
  const knownUnits = unitLabelSet(unitLabels);
  if (knownUnits.size > 0) {
    return knownUnits.has(value);
  }
  return /^[A-Za-z%][A-Za-z0-9/%_^()*]*$/.test(value);
}

function unitLabelSet(unitLabels) {
  if (unitLabels instanceof Set) {
    return unitLabels;
  }
  return new Set(
    (Array.isArray(unitLabels) ? unitLabels : []).filter(
      (label) => typeof label === "string" && label.length > 0
    )
  );
}

function bareNumericRanges(lineText, unitLabels) {
  const ranges = [];
  const pattern = /(^|[=+\-*/(,]\s*)(\d+(?:\.\d+)?)(?!\s*[A-Za-z_%])/g;
  let match;
  while ((match = pattern.exec(lineText)) !== null) {
    const literalStart = match.index + match[1].length;
    const literalEnd = literalStart + match[2].length;
    if (hasKnownUnitAfter(lineText, literalEnd, unitLabels)) {
      continue;
    }
    ranges.push({ start: literalStart, end: literalEnd });
  }
  return ranges;
}

function hasKnownUnitAfter(lineText, index, unitLabels) {
  const knownUnits = unitLabelSet(unitLabels);
  if (knownUnits.size === 0) {
    return false;
  }
  const match = /^[ \t]+([^\s,;)\]}]+)/.exec(lineText.slice(index));
  return Boolean(match && knownUnits.has(match[1]));
}

function schemaAnnotationAction(document, diagnostic) {
  const details = schemaAnnotationDetails(diagnostic.message);
  if (!details) {
    return undefined;
  }

  const line = document.lineAt(diagnostic.range.start.line);
  const assignment = new RegExp(`^(\\s*)(${escapeRegExp(details.name)})\\s*=.*$`).exec(line.text);
  if (!assignment) {
    return undefined;
  }
  if (details.unit && !sourceLineContainsUnit(line.text, details.unit)) {
    return undefined;
  }

  const action = new vscode.CodeAction(
    `Convert ${details.name} to schema column annotation`,
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(line.lineNumber, 0, line.lineNumber, line.text.length),
    `${assignment[1]}${details.annotation}`
  );
  return action;
}

function schemaAnnotationDetails(message) {
  const match = /Write `([A-Za-z_][A-Za-z0-9_]*:\s*[A-Za-z_][A-Za-z0-9_]*(?:\s*\[[^\]\r\n`]+\])?)` instead of assigning a value\./.exec(message);
  if (!match) {
    return undefined;
  }
  const annotation = match[1].replace(/\s+/g, " ").trim();
  const name = /^([A-Za-z_][A-Za-z0-9_]*)\s*:/.exec(annotation)?.[1];
  if (!name) {
    return undefined;
  }
  const unit = /\[([^\]\r\n`]+)\]/.exec(annotation)?.[1]?.trim();
  return { annotation, name, unit };
}

function sourceLineContainsUnit(lineText, unit) {
  return new RegExp(`\\b${escapeRegExp(unit)}\\b`).test(lineText);
}

function fileMutationConfirmAction(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  if (!/^\s*(move|delete)\b/.test(line.text)) {
    return undefined;
  }
  return booleanWithOptionsAction(document, diagnostic, ["confirm"]);
}

function recursiveDeleteAction(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  if (!/^\s*delete\s+dir\(/.test(line.text)) {
    return undefined;
  }
  return booleanWithOptionsAction(document, diagnostic, ["recursive", "confirm"]);
}

function optionQuickFix(code) {
  switch (code) {
    case "E-NET-RETRY-POLICY":
    case "E-PROCESS-RETRY-POLICY":
      return { optionNames: ["retry"], value: "0", label: "Disable retries" };
    case "E-NET-TIMEOUT":
      return { optionNames: ["timeout"], value: "30 s", label: "Set timeout to 30 s" };
    case "E-PROCESS-TIMEOUT":
      return { optionNames: ["timeout"], value: "10 s", label: "Set timeout to 10 s" };
    case "E-NET-BODY-SIZE-LIMIT":
      return {
        optionNames: ["body_size_limit", "response_body_limit"],
        value: "10 MB",
        label: "Set response body limit to 10 MB"
      };
    case "E-NET-BODY-POLICY":
      return {
        optionNames: ["body"],
        value: "\"{}\"",
        label: "Replace request body with string literal"
      };
    case "E-PROCESS-ALLOW-FAILURE":
      return {
        optionNames: ["allow_failure"],
        value: "true",
        label: "Allow process failure"
      };
    case "E-PROCESS-CWD-001":
      return { optionNames: ["cwd"], value: "dir(\".\")", label: "Set process cwd" };
    case "E-PROCESS-ENV-001":
      return { optionNames: ["env"], value: "{ NAME = \"value\" }", label: "Set process env" };
    case "E-SAMPLING-COUNT-INVALID":
      return { optionNames: ["count"], value: "1", label: "Set sample count" };
    case "E-SAMPLING-SEED-INVALID":
      return { optionNames: ["seed"], value: "42", label: "Set sample seed" };
    case "E-WITH-UNCERTAINTY-POLICY-001":
      return { optionNames: ["uncertainty"], value: "linear", label: "Set uncertainty policy" };
    case "E-WITH-UNCERTAINTY-SAMPLES-001":
      return { optionNames: ["samples"], value: "64", label: "Set uncertainty samples" };
    case "E-WITH-UNCERTAINTY-SEED-001":
      return { optionNames: ["seed"], value: "7", label: "Set uncertainty seed" };
    case "E-CACHE-KEY-NONDETERMINISTIC":
      return { optionNames: ["cache_key"], value: "[\"stable\", \"v1\"]", label: "Set deterministic cache key" };
    case "E-CACHE-DIR":
      return { optionNames: ["cache_dir"], value: "dir(\"cache\")", label: "Set cache directory" };
    case "E-CACHE-TTL":
      return { optionNames: ["cache_ttl"], value: "1 h", label: "Set cache TTL to 1 h" };
    case "E-SIM-TIMESTEP-INVALID":
      return { optionNames: ["timestep"], value: "10 min", label: "Set simulation timestep" };
    case "E-SOLVE-TIMESTEP-INVALID":
      return { optionNames: ["timestep"], value: "1 s", label: "Set solver timestep" };
    case "E-SIM-DURATION-INVALID":
      return { optionNames: ["duration"], value: "30 min", label: "Set simulation duration" };
    case "E-SOLVE-DURATION-INVALID":
      return { optionNames: ["duration"], value: "10 s", label: "Set solver duration" };
    case "E-SIM-TOLERANCE-INVALID":
      return { optionNames: ["tolerance"], value: "0.0001", label: "Set simulation tolerance" };
    case "E-SOLVE-TOLERANCE-INVALID":
      return { optionNames: ["tolerance"], value: "0.0001", label: "Set solver tolerance" };
    case "E-SIM-SOLVER-UNSUPPORTED":
      return { optionNames: ["solver"], value: "fixed_step", label: "Set simulation solver" };
    case "E-SOLVE-SOLVER-UNSUPPORTED":
      return { optionNames: ["solver"], value: "fixed_point", label: "Set solve solver" };
    case "E-SOLVE-RELAXATION-INVALID":
      return { optionNames: ["relaxation"], value: "0.5", label: "Set solver relaxation" };
    case "E-SOLVE-FD-STEP-INVALID":
      return { optionNames: ["finite_difference_step"], value: "0.000001", label: "Set finite-difference step" };
    case "E-SOLVE-DAMPING-INVALID":
      return { optionNames: ["damping"], value: "1", label: "Set solver damping" };
    case "E-SOLVE-CONSISTENCY-TOLERANCE-INVALID":
      return { optionNames: ["consistency_tolerance"], value: "0.000001", label: "Set consistency tolerance" };
    case "E-SOLVE-MAX-ITER-INVALID":
      return { optionNames: ["max_iter"], value: "50", label: "Set solver max iterations" };
    case "E-SOLVE-LINE-SEARCH-STEPS-INVALID":
      return { optionNames: ["line_search_steps"], value: "8", label: "Set line-search steps" };
    case "E-SOLVE-INITIAL-INVALID":
      return {
        optionNames: ["initial", "initial_derivative", "initial_algebraic"],
        value: "1",
        label: "Set solver initial value"
      };
    case "E-SOLVE-VARIABLE-SCALE-INVALID":
      return {
        optionNames: ["variable_scale", "variable_scales"],
        value: "1",
        label: "Set solver variable scale"
      };
    case "E-SOLVE-MASS-MATRIX-INVALID":
      return { optionNames: ["mass_matrix"], value: "identity", label: "Set mass matrix" };
    case "E-SOLVE-JACOBIAN-UNSUPPORTED":
      return {
        optionNames: ["jacobian"],
        value: "finite_difference",
        label: "Set solver Jacobian policy"
      };
    case "E-SOLVE-ALGEBRAIC-INITIALIZATION-UNSUPPORTED":
      return {
        optionNames: ["algebraic_initialization"],
        value: "newton",
        label: "Set algebraic initialization"
      };
    case "E-ML-ARGS-003":
      return { optionNames: ["algorithm"], value: "linear", label: "Set regression algorithm" };
    default:
      return undefined;
  }
}

function optionValueReplacementAction(document, diagnostic, fix, workflowOptionLabels) {
  if (!fix) {
    return undefined;
  }
  const optionNames = knownWorkflowOptionNames(fix.optionNames, workflowOptionLabels);
  if (optionNames.length === 0) {
    return undefined;
  }
  const line = document.lineAt(diagnostic.range.start.line);
  const assignment = optionAssignmentRange(line.text, optionNames);
  if (!assignment) {
    return undefined;
  }
  const optionLabel = optionNames.length === 1 ? optionNames[0] : assignment.optionName;
  return optionValueAction(document, diagnostic, line, assignment, fix.label, fix.value, optionLabel);
}

function modelOptionValueAction(document, diagnostic, code, workflowOptionLabels) {
  const fixes = modelOptionFixes(code).filter((fix) =>
    knownWorkflowOptionLabel(fix.optionName, workflowOptionLabels)
  );
  if (fixes.length === 0) {
    return undefined;
  }
  const line = document.lineAt(diagnostic.range.start.line);
  const assignment = optionAssignmentRange(
    line.text,
    fixes.map((fix) => fix.optionName)
  );
  if (!assignment) {
    return undefined;
  }
  const fix = fixes.find((candidate) => candidate.optionName === assignment.optionName);
  if (!fix) {
    return undefined;
  }
  return optionValueAction(document, diagnostic, line, assignment, fix.label, fix.value, assignment.optionName);
}

function modelOptionFixes(code) {
  const sharedFixes = [
    { optionName: "test", value: "0.25", label: "Set model test split" },
    { optionName: "hidden", value: "[8]", label: "Set model hidden layers" },
    { optionName: "epochs", value: "20", label: "Set model epochs" }
  ];
  if (code === "E-ML-ARGS-001") {
    return sharedFixes;
  }
  if (code === "E-ML-ARGS-002") {
    return [
      ...sharedFixes,
      { optionName: "seed", value: "7", label: "Set model seed" }
    ];
  }
  return [];
}

function optionValueAction(document, diagnostic, line, assignment, label, value, optionLabel) {
  const action = new vscode.CodeAction(
    `${label}: ${optionLabel} = ${value}`,
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(line.lineNumber, assignment.valueStart, line.lineNumber, assignment.valueEnd),
    value
  );
  return action;
}

function absoluteHttpUrlAction(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  const range = netUrlLiteralRange(line.text);
  if (!range) {
    return undefined;
  }
  const action = new vscode.CodeAction(
    "Replace URL with https://example.org",
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(line.lineNumber, range.start, line.lineNumber, range.end),
    "\"https://example.org\""
  );
  return action;
}

function httpBodyMethodAction(document, diagnostic) {
  const ownerLineNumber = ownerLineForEnclosingWithBlock(document, diagnostic.range.start.line);
  if (ownerLineNumber === undefined) {
    return undefined;
  }
  const ownerLine = document.lineAt(ownerLineNumber);
  const range = httpMethodTokenRange(ownerLine.text);
  if (!range) {
    return undefined;
  }
  const action = new vscode.CodeAction("Change HTTP method to post", vscode.CodeActionKind.QuickFix);
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(ownerLine.lineNumber, range.start, ownerLine.lineNumber, range.end),
    "post"
  );
  return action;
}

function expectedSha256Action(document, diagnostic) {
  const hash = expectedSha256FromDiagnostic(diagnostic);
  if (!hash) {
    return undefined;
  }
  const line = document.lineAt(diagnostic.range.start.line);
  const assignment = optionAssignmentRange(line.text, ["expected_sha256"]);
  if (!assignment) {
    return undefined;
  }
  const action = new vscode.CodeAction(
    "Update expected_sha256 to pinned response SHA-256",
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(line.lineNumber, assignment.valueStart, line.lineNumber, assignment.valueEnd),
    `"${hash}"`
  );
  return action;
}

function netUrlLiteralRange(lineText) {
  return callStringArgumentRange(lineText, "url") ?? firstStringLiteralRange(lineText);
}

function callStringArgumentRange(lineText, functionName) {
  let searchStart = 0;
  while (searchStart < lineText.length) {
    const start = lineText.indexOf(functionName, searchStart);
    if (start < 0) {
      break;
    }
    const afterName = start + functionName.length;
    if (identifierBoundary(lineText, start, afterName)) {
      let cursor = afterName;
      while (cursor < lineText.length && /\s/.test(lineText[cursor])) {
        cursor += 1;
      }
      if (lineText[cursor] === "(") {
        cursor += 1;
        while (cursor < lineText.length && /\s/.test(lineText[cursor])) {
          cursor += 1;
        }
        const range = stringLiteralRangeAt(lineText, cursor);
        if (range) {
          return range;
        }
      }
    }
    searchStart = afterName;
  }
  return undefined;
}

function firstStringLiteralRange(lineText) {
  const quote = lineText.indexOf("\"");
  return quote >= 0 ? stringLiteralRangeAt(lineText, quote) : undefined;
}

function stringLiteralRangeAt(lineText, quoteStart) {
  if (lineText[quoteStart] !== "\"") {
    return undefined;
  }
  let escaped = false;
  for (let index = quoteStart + 1; index < lineText.length; index += 1) {
    const character = lineText[index];
    if (escaped) {
      escaped = false;
      continue;
    }
    if (character === "\\") {
      escaped = true;
      continue;
    }
    if (character === "\"") {
      return { start: quoteStart, end: index + 1 };
    }
  }
  return undefined;
}

function ownerLineForEnclosingWithBlock(document, lineNumber) {
  for (let cursor = lineNumber - 1; cursor >= 0; cursor -= 1) {
    if (stripLineComment(document.lineAt(cursor).text).trim() !== "with {") {
      continue;
    }
    for (let owner = cursor - 1; owner >= 0; owner -= 1) {
      if (stripLineComment(document.lineAt(owner).text).trim() !== "") {
        return owner;
      }
    }
    return undefined;
  }
  return undefined;
}

function httpMethodTokenRange(lineText) {
  const code = stripLineComment(lineText);
  let searchStart = 0;
  while (searchStart < code.length) {
    const httpStart = code.indexOf("http", searchStart);
    if (httpStart < 0) {
      break;
    }
    const afterHttp = httpStart + "http".length;
    if (identifierBoundary(code, httpStart, afterHttp)) {
      let cursor = afterHttp;
      while (cursor < code.length && /\s/.test(code[cursor])) {
        cursor += 1;
      }
      const methodStart = cursor;
      while (cursor < code.length && /[A-Za-z]/.test(code[cursor])) {
        cursor += 1;
      }
      const method = code.slice(methodStart, cursor).toLowerCase();
      if (["get", "head", "request", "fetch"].includes(method)) {
        return { start: methodStart, end: cursor };
      }
    }
    searchStart = afterHttp;
  }
  return undefined;
}

function expectedSha256FromDiagnostic(diagnostic) {
  const match = /(?:fixture SHA256 was|observed) `([0-9a-fA-F]{64})`/.exec(
    diagnostic.message ?? ""
  );
  return match ? match[1].toLowerCase() : undefined;
}

function jsonReadPromotionAction(document, diagnostic) {
  const access = jsonReadFieldAccessFromDiagnostic(diagnostic.message);
  if (!access) {
    return undefined;
  }
  const accessLineNumber = diagnostic.range.start.line;
  if (accessLineNumber < 0 || accessLineNumber >= document.lineCount) {
    return undefined;
  }
  const accessLine = document.lineAt(accessLineNumber);
  const accessRange = jsonFieldAccessRange(accessLine.text, access.binding, access.field);
  if (!accessRange) {
    return undefined;
  }

  const schemaName = uniqueSchemaName(document, schemaNameFromBinding(access.binding));
  const typedBinding = availableBindingName(document, `${access.binding}_typed`);
  const newline = documentNewline(document);
  const readLineNumber = readJsonBindingLine(document, access.binding);
  const insertLineNumber = readLineNumber ?? accessLineNumber;
  const insertLine = document.lineAt(insertLineNumber);
  const indent = lineIndent(insertLine.text);
  const schemaText = `${indent}schema ${schemaName} {${newline}${indent}    ${access.field}: String${newline}${indent}}${newline}${newline}`;

  const action = new vscode.CodeAction(
    `Promote ${access.binding} before field access`,
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  if (readLineNumber !== undefined) {
    action.edit.insert(document.uri, new vscode.Position(readLineNumber, 0), schemaText);
    action.edit.insert(
      document.uri,
      document.lineAt(readLineNumber).range.end,
      `${newline}${indent}${typedBinding} = promote json ${access.binding} as ${schemaName}`
    );
  } else {
    action.edit.insert(
      document.uri,
      new vscode.Position(accessLineNumber, 0),
      `${schemaText}${indent}${typedBinding} = promote json ${access.binding} as ${schemaName}${newline}`
    );
  }
  action.edit.replace(
    document.uri,
    new vscode.Range(
      accessLine.lineNumber,
      accessRange.start,
      accessLine.lineNumber,
      accessRange.end
    ),
    `${typedBinding}.${access.field}`
  );
  return action;
}

function jsonReadFieldAccessFromDiagnostic(message) {
  for (const payload of backtickPayloads(message)) {
    const [binding, field, extra] = payload.split(".");
    if (extra === undefined && isIdentifier(binding) && isIdentifier(field)) {
      return { binding, field };
    }
  }
  return undefined;
}

function backtickPayloads(message) {
  return Array.from(String(message ?? "").matchAll(/`([^`]+)`/g), (match) => match[1].trim());
}

function readJsonBindingLine(document, binding) {
  for (let lineNumber = 0; lineNumber < document.lineCount; lineNumber += 1) {
    const code = stripLineComment(document.lineAt(lineNumber).text);
    const indent = lineIndent(code);
    const rest = code.slice(indent.length);
    if (!rest.startsWith(binding)) {
      continue;
    }
    const afterBinding = rest.slice(binding.length);
    if (isIdentifierCharacter(afterBinding[0])) {
      continue;
    }
    const equalsIndex = afterBinding.indexOf("=");
    if (equalsIndex < 0 || afterBinding.slice(0, equalsIndex).trim()) {
      continue;
    }
    if (afterBinding.slice(equalsIndex + 1).trimStart().startsWith("read json ")) {
      return lineNumber;
    }
  }
  return undefined;
}

function jsonFieldAccessRange(lineText, binding, field) {
  const code = stripLineComment(lineText);
  const access = `${binding}.${field}`;
  let searchStart = 0;
  while (searchStart < code.length) {
    const start = code.indexOf(access, searchStart);
    if (start < 0) {
      break;
    }
    const end = start + access.length;
    if (identifierBoundary(code, start, end)) {
      return { start, end };
    }
    searchStart = end;
  }
  return undefined;
}

function availableBindingName(document, base) {
  if (!bindingNameExists(document, base)) {
    return base;
  }
  return uniqueBindingName(document, base);
}

function uniqueSchemaName(document, base) {
  if (!schemaNameExists(document, base)) {
    return base;
  }
  for (let index = 2; ; index += 1) {
    const candidate = `${base}${index}`;
    if (!schemaNameExists(document, candidate)) {
      return candidate;
    }
  }
}

function schemaNameExists(document, name) {
  for (let lineNumber = 0; lineNumber < document.lineCount; lineNumber += 1) {
    const code = stripLineComment(document.lineAt(lineNumber).text).trimStart();
    if (!code.startsWith("schema")) {
      continue;
    }
    const rest = code.slice("schema".length);
    if (!/\s/.test(rest[0] ?? "")) {
      continue;
    }
    const candidate = rest.trimStart();
    if (candidate.startsWith(name) && !isIdentifierCharacter(candidate[name.length])) {
      return true;
    }
  }
  return false;
}

function schemaNameFromBinding(binding) {
  const base = binding
    .split(/[^A-Za-z0-9]+/)
    .filter(Boolean)
    .map((segment) => segment[0].toUpperCase() + segment.slice(1).toLowerCase())
    .join("");
  return `${base || "JsonPayload"}Schema`;
}

function withOptionAliasAction(document, diagnostic, workflowOptionLabels) {
  const fix = withOptionAliasFix(unknownWithOptionName(diagnostic.message), workflowOptionLabels);
  if (!fix) {
    return undefined;
  }
  return optionKeyReplacementAction(document, diagnostic, fix.from, fix.to, fix.title);
}

function diagnosticRangeReplacementAction(document, diagnostic, replacement, title) {
  const action = new vscode.CodeAction(title, vscode.CodeActionKind.QuickFix);
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(document.uri, diagnostic.range, replacement);
  return action;
}

function optionKeyReplacementAction(document, diagnostic, from, to, title) {
  const line = document.lineAt(diagnostic.range.start.line);
  const pattern = new RegExp(`^(\\s*)${escapeRegExp(from)}(\\s*=)`);
  const match = pattern.exec(stripLineComment(line.text));
  if (!match) {
    return undefined;
  }
  const startCharacter = match[1].length;
  const action = new vscode.CodeAction(title, vscode.CodeActionKind.QuickFix);
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(line.lineNumber, startCharacter, line.lineNumber, startCharacter + from.length),
    to
  );
  return action;
}

function withOptionAliasFix(optionName, workflowOptionLabels) {
  let fix;
  switch (optionName) {
    case "unit":
    case "y_unit":
      fix = {
        from: optionName,
        to: "unit y",
        title: "Use plot y-axis option: unit y ="
      };
      break;
    case "x_unit":
      fix = {
        from: optionName,
        to: "unit x",
        title: "Use plot x-axis option: unit x ="
      };
      break;
    case "confidence":
      fix = {
        from: optionName,
        to: "confidence_band",
        title: "Use confidence band option: confidence_band ="
      };
      break;
    default:
      return undefined;
  }
  return knownWorkflowOptionLabel(fix.to, workflowOptionLabels) ? fix : undefined;
}

function knownWorkflowOptionLabel(label, workflowOptionLabels) {
  const labels = workflowOptionLabelSet(workflowOptionLabels);
  return labels.size === 0 || labels.has(label);
}

function knownWorkflowOptionNames(optionNames, workflowOptionLabels) {
  const names = Array.isArray(optionNames) ? optionNames : [];
  const labels = workflowOptionLabelSet(workflowOptionLabels);
  if (labels.size === 0) {
    return names;
  }
  return names.filter((name) => labels.has(name));
}

function workflowOptionLabelSet(workflowOptionLabels) {
  if (workflowOptionLabels instanceof Set) {
    return workflowOptionLabels;
  }
  return new Set(
    (Array.isArray(workflowOptionLabels) ? workflowOptionLabels : []).filter(
      (label) => typeof label === "string" && label.length > 0
    )
  );
}

function unknownWithOptionName(message) {
  const match = /Unknown with option `([^`]+)`/.exec(String(message ?? ""));
  return match?.[1]?.trim();
}

function removeIncompatibleDisplayUnitAction(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  const assignment = optionAssignmentRange(line.text, ["unit y", "unit x", "display_unit", "unit"]);
  if (!assignment) {
    return undefined;
  }
  const action = new vscode.CodeAction(
    "Remove incompatible display unit option",
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.delete(document.uri, fullLineRange(document, line.lineNumber));
  return action;
}

function closeUnterminatedInterpolationAction(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  const insertCharacter = unterminatedInterpolationClosePosition(line.text, diagnostic.range);
  if (insertCharacter === undefined) {
    return undefined;
  }
  const action = new vscode.CodeAction("Close interpolation with }", vscode.CodeActionKind.QuickFix);
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.insert(document.uri, new vscode.Position(line.lineNumber, insertCharacter), "}");
  return action;
}

function unterminatedInterpolationClosePosition(lineText, diagnosticRange) {
  const open = interpolationOpenIndex(lineText, diagnosticRange);
  if (open === undefined) {
    return undefined;
  }
  const quoteEnd = unescapedQuoteIndexAfter(lineText, open + 1);
  if (quoteEnd === undefined || lineText.slice(open + 1, quoteEnd).includes("}")) {
    return undefined;
  }
  return quoteEnd;
}

function removeEmptyInterpolationAction(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  const removal = emptyInterpolationRange(line.text, diagnostic.range);
  if (!removal) {
    return undefined;
  }
  const action = new vscode.CodeAction("Remove empty interpolation", vscode.CodeActionKind.QuickFix);
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.delete(
    document.uri,
    new vscode.Range(line.lineNumber, removal.start, line.lineNumber, removal.end)
  );
  return action;
}

function emptyInterpolationRange(lineText, diagnosticRange) {
  const ranges = emptyInterpolationRanges(stripLineComment(lineText));
  if (!ranges.length) {
    return undefined;
  }
  const diagnosticStart = diagnosticRange?.start?.character ?? -1;
  return ranges.find((range) => diagnosticStart >= range.start && diagnosticStart <= range.end)
    ?? ranges[0];
}

function emptyInterpolationRanges(code) {
  const ranges = [];
  let cursor = 0;
  while (cursor < code.length) {
    const open = code.indexOf("{", cursor);
    if (open < 0) break;
    const close = code.indexOf("}", open + 1);
    if (close < 0) break;
    if (code.slice(open + 1, close).trim() === "") {
      ranges.push({ start: open, end: close + 1 });
    }
    cursor = close + 1;
  }
  return ranges;
}

function convertUnresolvedInterpolationAction(document, diagnostic) {
  const expression = firstBacktickPayload(diagnostic.message);
  if (!expression) {
    return undefined;
  }
  const line = document.lineAt(diagnostic.range.start.line);
  const edit = unresolvedInterpolationLiteralEdit(line.text, expression, diagnostic.range);
  if (!edit) {
    return undefined;
  }
  const action = new vscode.CodeAction(
    "Convert unresolved interpolation to literal text",
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(line.lineNumber, edit.start, line.lineNumber, edit.end),
    edit.newText
  );
  return action;
}

function unresolvedInterpolationLiteralEdit(lineText, expression, diagnosticRange) {
  const ranges = interpolationLiteralRanges(stripLineComment(lineText), expression);
  if (!ranges.length) {
    return undefined;
  }
  const diagnosticStart = diagnosticRange?.start?.character ?? -1;
  return ranges.find((range) => diagnosticStart >= range.start && diagnosticStart <= range.end)
    ?? ranges[0];
}

function interpolationLiteralRanges(code, expression) {
  const ranges = [];
  let cursor = 0;
  while (cursor < code.length) {
    const open = code.indexOf("{", cursor);
    if (open < 0) break;
    const close = code.indexOf("}", open + 1);
    if (close < 0) break;
    const inside = code.slice(open + 1, close).trim();
    const expressionPart = inside.split(":", 1)[0].trim();
    if (inside && expressionPart === expression.trim()) {
      ranges.push({ start: open, end: close + 1, newText: inside });
    }
    cursor = close + 1;
  }
  return ranges;
}

function interpolationOpenIndex(lineText, diagnosticRange) {
  const diagnosticStart = diagnosticRange?.start?.character;
  if (Number.isInteger(diagnosticStart) && lineText[diagnosticStart] === "{") {
    return diagnosticStart;
  }
  const searchStart = Number.isInteger(diagnosticStart) ? Math.max(0, diagnosticStart - 1) : 0;
  const before = lineText.lastIndexOf("{", searchStart);
  if (before >= 0) {
    return before;
  }
  const after = lineText.indexOf("{", searchStart);
  return after >= 0 ? after : undefined;
}

function unescapedQuoteIndexAfter(lineText, start) {
  let escaped = false;
  for (let index = start; index < lineText.length; index += 1) {
    const char = lineText[index];
    if (escaped) {
      escaped = false;
      continue;
    }
    if (char === "\\") {
      escaped = true;
      continue;
    }
    if (char === "\"") {
      return index;
    }
  }
  return undefined;
}

function removeInterpolationDisplayUnitAction(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  const unit = lastBacktickPayload(diagnostic.message);
  if (!unit) {
    return undefined;
  }
  const removal = interpolationUnitRemovalRange(line.text, unit, diagnostic.range);
  if (!removal) {
    return undefined;
  }
  const action = new vscode.CodeAction(
    "Remove incompatible interpolation unit",
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.delete(
    document.uri,
    new vscode.Range(line.lineNumber, removal.start, line.lineNumber, removal.end)
  );
  return action;
}

function interpolationUnitRemovalRange(lineText, unit, diagnosticRange) {
  const ranges = interpolationUnitRemovalRanges(stripLineComment(lineText), unit);
  if (!ranges.length) {
    return undefined;
  }
  const diagnosticStart = diagnosticRange?.start?.character ?? -1;
  return ranges.find((range) => diagnosticStart >= range.start && diagnosticStart <= range.end)
    ?? ranges[0];
}

function interpolationUnitRemovalRanges(code, unit) {
  const ranges = [];
  let cursor = 0;
  while (cursor < code.length) {
    const open = code.indexOf("{", cursor);
    if (open < 0) {
      break;
    }
    const close = code.indexOf("}", open + 1);
    if (close < 0) {
      break;
    }
    const inside = code.slice(open + 1, close);
    const colon = inside.indexOf(":");
    if (colon >= 0) {
      const colonIndex = open + 1 + colon;
      const specStart = colonIndex + 1;
      const spec = code.slice(specStart, close);
      if (spec.trim() === unit) {
        ranges.push({ start: colonIndex, end: close });
      } else {
        const match = new RegExp(`${escapeRegExp(unit)}\\s*$`).exec(spec);
        if (match && formatSpecPrefixCanStandWithoutUnit(spec.slice(0, match.index))) {
          ranges.push({ start: specStart + match.index, end: specStart + match.index + match[0].length });
        }
      }
    }
    cursor = close + 1;
  }
  return ranges;
}

function formatSpecPrefixCanStandWithoutUnit(prefix) {
  const trimmed = String(prefix ?? "").trim();
  return /^\.\d+$/.test(trimmed);
}

function lastBacktickPayload(message) {
  const matches = [...String(message ?? "").matchAll(/`([^`]+)`/g)];
  return matches.length ? matches[matches.length - 1][1].trim() : undefined;
}

function logLevelInfoAction(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  const edit = logLevelInfoEdit(line.text);
  if (!edit) {
    return undefined;
  }
  const action = new vscode.CodeAction("Set log level to info", vscode.CodeActionKind.QuickFix);
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(line.lineNumber, edit.start, line.lineNumber, edit.end),
    edit.newText
  );
  return action;
}

function logLevelInfoEdit(lineText) {
  const code = stripLineComment(lineText);
  const match = /^(\s*)log(\s*)/.exec(code);
  if (!match) {
    return undefined;
  }
  const tokenStart = match[1].length + "log".length + match[2].length;
  const first = code[tokenStart];
  if (first === '"') {
    return { start: tokenStart, end: tokenStart, newText: "info " };
  }
  const levelMatch = /^[^\s]+/.exec(code.slice(tokenStart));
  if (!levelMatch) {
    return undefined;
  }
  const level = levelMatch[0];
  if (["debug", "info", "warn", "error"].includes(level)) {
    return undefined;
  }
  return { start: tokenStart, end: tokenStart + level.length, newText: "info" };
}

function statementOnlyUnbindAction(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  const range = statementBindingPrefixRange(line.text);
  if (!range) {
    return undefined;
  }
  const action = new vscode.CodeAction("Remove invalid binding prefix", vscode.CodeActionKind.QuickFix);
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.delete(
    document.uri,
    new vscode.Range(line.lineNumber, range.start, line.lineNumber, range.end)
  );
  return action;
}

function statementBindingPrefixRange(lineText) {
  const code = stripLineComment(lineText);
  const match = /^(\s*)[A-Za-z_][A-Za-z0-9_]*(?:\s*:\s*[^=]+)?\s*=\s*/.exec(code);
  if (!match || code.slice(match[0].length).trim() === "") {
    return undefined;
  }
  return { start: match[1].length, end: match[0].length };
}

function bindProcessResultAction(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  const code = stripLineComment(line.text);
  const indent = lineIndent(code);
  if (!code.slice(indent.length).startsWith("run command")) {
    return undefined;
  }
  const action = new vscode.CodeAction("Bind process result", vscode.CodeActionKind.QuickFix);
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.insert(
    document.uri,
    new vscode.Position(line.lineNumber, indent.length),
    "result = "
  );
  return action;
}

function uniqueProcessBindingAction(document, diagnostic) {
  const name = firstBacktickPayload(diagnostic.message);
  if (!name || !isIdentifier(name)) {
    return undefined;
  }
  const line = document.lineAt(diagnostic.range.start.line);
  const code = stripLineComment(line.text);
  const indent = lineIndent(code);
  const rest = code.slice(indent.length);
  if (!rest.startsWith(name)) {
    return undefined;
  }
  const afterName = rest.slice(name.length);
  if (isIdentifierCharacter(afterName[0]) || !afterName.trimStart().startsWith("=")) {
    return undefined;
  }
  const replacement = uniqueBindingName(document, name);
  const action = new vscode.CodeAction(
    `Rename process result to ${replacement}`,
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(line.lineNumber, indent.length, line.lineNumber, indent.length + name.length),
    replacement
  );
  return action;
}

function uniqueBindingName(document, base) {
  for (let index = 2; ; index += 1) {
    const candidate = `${base}_${index}`;
    if (!bindingNameExists(document, candidate)) {
      return candidate;
    }
  }
}

function bindingNameExists(document, name) {
  for (let lineNumber = 0; lineNumber < document.lineCount; lineNumber += 1) {
    const code = stripLineComment(document.lineAt(lineNumber).text);
    const trimmed = code.trimStart();
    if (!trimmed.startsWith(name)) {
      continue;
    }
    const rest = trimmed.slice(name.length);
    if (!isIdentifierCharacter(rest[0]) && rest.trimStart().startsWith("=")) {
      return true;
    }
  }
  return false;
}

function processCommandAction(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  const edit = processCommandEdit(line.text);
  if (!edit) {
    return undefined;
  }
  const action = new vscode.CodeAction("Add process command string", vscode.CodeActionKind.QuickFix);
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(line.lineNumber, edit.start, line.lineNumber, edit.end),
    edit.newText
  );
  return action;
}

function processCommandEdit(lineText) {
  const code = stripLineComment(lineText);
  const commandStart = code.indexOf("run command");
  if (commandStart < 0) {
    return undefined;
  }
  const afterCommand = commandStart + "run command".length;
  const whitespace = /^\s*/.exec(code.slice(afterCommand))?.[0] ?? "";
  const argumentStart = afterCommand + whitespace.length;
  const argument = code.slice(argumentStart);
  if (argument.startsWith("\"\"")) {
    return { start: argumentStart, end: argumentStart + 2, newText: "\"tool\"" };
  }
  if (argument.trim() === "") {
    const insertAt = code.trimEnd().length;
    return { start: insertAt, end: insertAt, newText: " \"tool\"" };
  }
  return undefined;
}

function wrapAssertionAction(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  const code = stripLineComment(line.text);
  const indent = lineIndent(code);
  const assertion = code.slice(indent.length).trimEnd();
  if (!assertion.startsWith("assert ")) {
    return undefined;
  }
  const newline = documentNewline(document);
  const replacement =
    `${indent}test "assertion" {${newline}${indent}    ${assertion}${newline}${indent}}${newline}`;
  const action = new vscode.CodeAction(
    "Wrap assertion in test block",
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(document.uri, fullLineRange(document, line.lineNumber), replacement);
  return action;
}

function goldenExpectedFileAction(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  const code = stripLineComment(line.text);
  const range = goldenBareExpectedStringRange(code);
  if (!range) {
    return undefined;
  }
  const expected = code.slice(range.start, range.end);
  const action = new vscode.CodeAction(
    "Wrap golden expected path with file(...)",
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(line.lineNumber, range.start, line.lineNumber, range.end),
    `file(${expected})`
  );
  return action;
}

function goldenBareExpectedStringRange(lineText) {
  const indent = lineIndent(lineText).length;
  if (!lineText.slice(indent).startsWith("golden ")) {
    return undefined;
  }
  const matchesIndex = lineText.indexOf(" matches ");
  if (matchesIndex < 0) {
    return undefined;
  }
  let cursor = matchesIndex + " matches ".length;
  while (cursor < lineText.length && /\s/.test(lineText[cursor])) {
    cursor += 1;
  }
  if (lineText.slice(cursor).startsWith("file(")) {
    return undefined;
  }
  const range = stringLiteralRangeAt(lineText, cursor);
  if (!range || lineText.slice(range.end).trim() !== "") {
    return undefined;
  }
  return range;
}
function uncertaintyArgumentActions(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  const message = String(diagnostic.message ?? "");
  const actions = [];

  const example = uncertaintyCallExampleFromDiagnostic(message);
  const callRange = uncertaintyCallRangeOnLine(line.text);
  if (example && callRange) {
    const action = new vscode.CodeAction(
      `Replace uncertainty call with ${example}`,
      vscode.CodeActionKind.QuickFix
    );
    action.isPreferred = true;
    action.diagnostics = [diagnostic];
    action.edit = new vscode.WorkspaceEdit();
    action.edit.replace(
      document.uri,
      new vscode.Range(line.lineNumber, callRange.start, line.lineNumber, callRange.end),
      example
    );
    actions.push(action);
  }

  if (message.includes("method=linear")) {
    const range = namedArgumentValueRange(line.text, ["method"]);
    if (range) {
      const action = new vscode.CodeAction(
        "Set uncertainty method to linear",
        vscode.CodeActionKind.QuickFix
      );
      action.isPreferred = true;
      action.diagnostics = [diagnostic];
      action.edit = new vscode.WorkspaceEdit();
      action.edit.replace(
        document.uri,
        new vscode.Range(line.lineNumber, range.valueStart, line.lineNumber, range.valueEnd),
        "linear"
      );
      actions.push(action);
    }
  }

  if (
    message.includes("supports `normal` and `uniform`") &&
    stripLineComment(line.text).includes("distribution(")
  ) {
    const range = namedArgumentValueRange(line.text, ["kind"]);
    if (range) {
      const action = new vscode.CodeAction(
        "Set distribution kind to normal",
        vscode.CodeActionKind.QuickFix
      );
      action.isPreferred = true;
      action.diagnostics = [diagnostic];
      action.edit = new vscode.WorkspaceEdit();
      action.edit.replace(
        document.uri,
        new vscode.Range(line.lineNumber, range.valueStart, line.lineNumber, range.valueEnd),
        "normal"
      );
      actions.push(action);
    }
  }

  if (message.includes("between 1 and 256")) {
    const range = namedArgumentValueRange(line.text, ["samples", "n"]);
    if (range) {
      const action = new vscode.CodeAction(
        "Set uncertainty samples to 31",
        vscode.CodeActionKind.QuickFix
      );
      action.isPreferred = true;
      action.diagnostics = [diagnostic];
      action.edit = new vscode.WorkspaceEdit();
      action.edit.replace(
        document.uri,
        new vscode.Range(line.lineNumber, range.valueStart, line.lineNumber, range.valueEnd),
        "31"
      );
      actions.push(action);
    }
  }

  return actions;
}

function uncertaintySourceActions(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  const message = String(diagnostic.message ?? "");
  const actions = [];

  if (message.includes("Unknown uncertainty source")) {
    const source = uncertaintySourceNameFromDiagnostic(message);
    if (source) {
      const indent = lineIndent(line.text);
      const placeholder = `${indent}${uncertaintySourceDefinition(source, line.text)}`;
      const action = new vscode.CodeAction(
        `Define uncertainty source ${source}`,
        vscode.CodeActionKind.QuickFix
      );
      action.isPreferred = true;
      action.diagnostics = [diagnostic];
      action.edit = new vscode.WorkspaceEdit();
      action.edit.insert(
        document.uri,
        new vscode.Position(line.lineNumber, 0),
        `${placeholder}${documentNewline(document)}`
      );
      actions.push(action);
    }
  }

  if (message.includes("requires a prior uncertainty binding as its first argument")) {
    const openParen = uncertaintySourceCallOpenParen(line.text);
    if (openParen !== undefined) {
      const source = "Q_source_unc";
      const indent = lineIndent(line.text);
      const placeholder = `${indent}${uncertaintySourceDefinition(source, line.text)}`;
      const action = new vscode.CodeAction(
        "Add uncertainty source Q_source_unc",
        vscode.CodeActionKind.QuickFix
      );
      action.isPreferred = true;
      action.diagnostics = [diagnostic];
      action.edit = new vscode.WorkspaceEdit();
      action.edit.insert(
        document.uri,
        new vscode.Position(line.lineNumber, 0),
        `${placeholder}${documentNewline(document)}`
      );
      action.edit.insert(
        document.uri,
        new vscode.Position(line.lineNumber, openParen + 1),
        `${source}, `
      );
      actions.push(action);
    }
  }

  if (message.includes("not an uncertainty source")) {
    const source = uncertaintySourceNameFromDiagnostic(message);
    const bindingRange = source
      ? bindingExpressionRangeForName(document, source, line.lineNumber)
      : undefined;
    if (
      bindingRange &&
      expressionStartsNumeric(bindingRange.expression) &&
      !isUncertaintyCallExpression(bindingRange.expression)
    ) {
      const unit = firstUnitOnLine(bindingRange.expression);
      if (unit) {
        const stdUnit = unit === "degC" ? "K" : unit;
        const replacement = `measured(${bindingRange.expression.trim()}, std=0.8 ${stdUnit})`;
        const action = new vscode.CodeAction(
          `Convert ${source} to measured uncertainty source`,
          vscode.CodeActionKind.QuickFix
        );
        action.isPreferred = true;
        action.diagnostics = [diagnostic];
        action.edit = new vscode.WorkspaceEdit();
        action.edit.replace(
          document.uri,
          new vscode.Range(
            bindingRange.lineNumber,
            bindingRange.expressionStart,
            bindingRange.lineNumber,
            bindingRange.expressionEnd
          ),
          replacement
        );
        actions.push(action);
      }
    }
  }

  return actions;
}

function selectFirstRowMigrationAction(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  const migration = selectFirstRowMigrationFromLine(line.text);
  if (!migration) {
    return undefined;
  }
  const replacement = selectFirstRowMigrationReplacement(
    migration,
    lineIndent(line.text),
    documentNewline(document)
  );
  const action = new vscode.CodeAction(
    "Replace select_first_row with filter + require_one",
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(document.uri, fullLineRange(document, line.lineNumber), replacement);
  return action;
}

function selectFirstRowMigrationFromLine(lineText) {
  const code = stripLineComment(lineText);
  const callStart = code.indexOf("select_first_row(");
  if (callStart < 0) {
    return undefined;
  }
  const beforeCall = code.slice(0, callStart);
  const equals = beforeCall.lastIndexOf("=");
  if (equals < 0) {
    return undefined;
  }
  const lhs = beforeCall.slice(lineIndent(beforeCall).length, equals).trim();
  const binding = lhs.split(":", 1)[0].trim();
  if (!isIdentifier(binding)) {
    return undefined;
  }
  const open = callStart + "select_first_row".length;
  if (code[open] !== "(") {
    return undefined;
  }
  const close = matchingCloseParenIndex(code, open);
  if (close === undefined || code.slice(close + 1).trim() !== "") {
    return undefined;
  }
  const parts = splitTopLevelCommas(code.slice(open + 1, close));
  const table = parts[0]?.trim();
  if (!isSimplePathExpression(table)) {
    return undefined;
  }

  let returnColumn;
  const filters = [];
  for (const part of parts.slice(1)) {
    const assignment = splitTopLevelAssignment(part);
    if (!assignment) {
      return undefined;
    }
    const name = assignment.name.trim();
    const value = assignment.value.trim();
    if (name === "return_column") {
      returnColumn = selectFirstRowReturnColumn(value);
      if (!returnColumn) {
        return undefined;
      }
      continue;
    }
    if (
      !isIdentifier(name) ||
      value === "" ||
      value.includes("{") ||
      value.includes("}") ||
      value.includes("\n") ||
      value.includes("\r")
    ) {
      return undefined;
    }
    filters.push({ name, value });
  }
  if (!returnColumn || filters.length === 0) {
    return undefined;
  }
  return { lhs, binding, table, returnColumn, filters };
}

function selectFirstRowReturnColumn(value) {
  const candidate = unquotedSimpleString(value) ?? String(value ?? "").trim();
  return isIdentifier(candidate) ? candidate : undefined;
}

function unquotedSimpleString(value) {
  const text = String(value ?? "").trim();
  if (!text.startsWith('"') || !text.endsWith('"')) {
    return undefined;
  }
  const inner = text.slice(1, -1);
  return inner.includes("\\") || inner.includes('"') ? undefined : inner;
}

function selectFirstRowMigrationReplacement(migration, indent, newline) {
  const rowsBinding = `${migration.binding}_rows`;
  const rowBinding = `${migration.binding}_row`;
  const lines = [
    `${indent}${rowsBinding} = filter ${migration.table}`,
    `${indent}where {`,
    ...migration.filters.map((filter) => `${indent}    ${filter.name} == ${filter.value}`),
    `${indent}}`,
    `${indent}${rowBinding} = require_one ${rowsBinding}`,
    `${indent}${migration.lhs} = ${rowBinding}.${migration.returnColumn}`
  ];
  return `${lines.join(newline)}${newline}`;
}

function splitTopLevelCommas(text) {
  const parts = [];
  let start = 0;
  let depth = 0;
  let inString = false;
  let escaped = false;
  for (let index = 0; index < text.length; index += 1) {
    const character = text[index];
    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (character === "\\") {
        escaped = true;
      } else if (character === '"') {
        inString = false;
      }
      continue;
    }
    if (character === '"') {
      inString = true;
    } else if (character === "(" || character === "[" || character === "{") {
      depth += 1;
    } else if (character === ")" || character === "]" || character === "}") {
      depth = Math.max(0, depth - 1);
    } else if (character === "," && depth === 0) {
      parts.push(text.slice(start, index).trim());
      start = index + 1;
    }
  }
  parts.push(text.slice(start).trim());
  return parts;
}

function splitTopLevelAssignment(text) {
  let depth = 0;
  let inString = false;
  let escaped = false;
  for (let index = 0; index < text.length; index += 1) {
    const character = text[index];
    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (character === "\\") {
        escaped = true;
      } else if (character === '"') {
        inString = false;
      }
      continue;
    }
    if (character === '"') {
      inString = true;
    } else if (character === "(" || character === "[" || character === "{") {
      depth += 1;
    } else if (character === ")" || character === "]" || character === "}") {
      depth = Math.max(0, depth - 1);
    } else if (character === "=" && depth === 0) {
      return { name: text.slice(0, index), value: text.slice(index + 1) };
    }
  }
  return undefined;
}

function isSimplePathExpression(value) {
  const text = String(value ?? "").trim();
  return text !== "" && text.split(".").every((part) => isIdentifier(part));
}

function uncertaintyDirectCompareAction(document, diagnostic) {
  const expression = directUncertaintyExpressionFromDiagnostic(diagnostic.message);
  if (!expression) {
    return undefined;
  }
  const line = document.lineAt(diagnostic.range.start.line);
  const range = directUncertaintyExpressionRange(line.text, expression);
  if (!range) {
    return undefined;
  }
  const action = new vscode.CodeAction(
    `Compare mean(${expression}) instead`,
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(line.lineNumber, range.start, line.lineNumber, range.end),
    `mean(${expression})`
  );
  return action;
}

function directUncertaintyExpressionFromDiagnostic(message) {
  const expression = firstBacktickPayload(message);
  if (!expression || expression.includes("\n") || expression.includes("\r")) {
    return undefined;
  }
  return expression.startsWith("mean(") ? undefined : expression;
}

function directUncertaintyExpressionRange(lineText, expression) {
  const code = stripLineComment(lineText);
  let searchStart = 0;
  while (searchStart <= code.length) {
    const start = code.indexOf(expression, searchStart);
    if (start < 0) {
      return undefined;
    }
    const end = start + expression.length;
    if (expressionBoundary(code, start, end)) {
      return { start, end };
    }
    searchStart = end;
  }
  return undefined;
}

function expressionBoundary(text, start, end) {
  const before = start > 0 ? text[start - 1] : undefined;
  const after = end < text.length ? text[end] : undefined;
  return !isExpressionEdgeCharacter(before) && !isExpressionEdgeCharacter(after);
}

function isExpressionEdgeCharacter(value) {
  return isIdentifierCharacter(value) || value === ".";
}

function uncertaintySourceDefinition(source, lineText) {
  const { mean, std } = uncertaintyNormalLiterals(firstUnitOnLine(lineText) || "kW");
  return `${source} = normal(mean=${mean}, std=${std}, samples=31)`;
}

function uncertaintyNormalLiterals(unit) {
  if (unit === "degC") {
    return { mean: "20 degC", std: "0.8 K" };
  }
  if (unit === "%") {
    return { mean: "50 %", std: "5 %" };
  }
  return { mean: `5 ${unit}`, std: `0.8 ${unit}` };
}

function uncertaintySourceNameFromDiagnostic(message) {
  if (String(message ?? "").includes("Unknown uncertainty source")) {
    const match = /Unknown uncertainty source `([^`]+)`/.exec(String(message ?? ""));
    const source = match?.[1]?.trim();
    return isIdentifier(source) ? source : undefined;
  }
  const source = /`([^`]+)`/.exec(String(message ?? ""))?.[1]?.trim();
  return isIdentifier(source) ? source : undefined;
}

function uncertaintySourceCallOpenParen(lineText) {
  const code = stripLineComment(lineText);
  for (const call of ["propagate", "ensemble", "probability"]) {
    let searchStart = 0;
    while (searchStart < code.length) {
      const start = code.indexOf(call, searchStart);
      if (start < 0) {
        break;
      }
      const afterName = start + call.length;
      if (identifierBoundary(code, start, afterName)) {
        let open = afterName;
        while (open < code.length && /\s/.test(code[open])) {
          open += 1;
        }
        if (code[open] === "(") {
          return open;
        }
      }
      searchStart = afterName;
    }
  }
  return undefined;
}

function bindingExpressionRangeForName(document, name, lineLimit) {
  for (let lineNumber = 0; lineNumber < lineLimit; lineNumber += 1) {
    const line = document.lineAt(lineNumber);
    const code = stripLineComment(line.text);
    const indent = lineIndent(code).length;
    const rest = code.slice(indent);
    if (!rest.startsWith(name)) {
      continue;
    }
    const afterName = rest.slice(name.length);
    if (isIdentifierCharacter(afterName[0])) {
      continue;
    }
    const equalsOffset = afterName.indexOf("=");
    if (equalsOffset < 0 || afterName.slice(0, equalsOffset).trim() !== "") {
      continue;
    }
    const rawStart = indent + name.length + equalsOffset + 1;
    const rawEnd = code.trimEnd().length;
    const rawExpression = code.slice(rawStart, rawEnd);
    const leading = rawExpression.length - rawExpression.trimStart().length;
    const trailing = rawExpression.length - rawExpression.trimEnd().length;
    const expressionStart = rawStart + leading;
    const expressionEnd = rawEnd - trailing;
    const expression = code.slice(expressionStart, expressionEnd);
    if (expression.trim() === "") {
      continue;
    }
    return { lineNumber, expression, expressionStart, expressionEnd };
  }
  return undefined;
}

function expressionStartsNumeric(expression) {
  return /^\s*\d/.test(String(expression ?? ""));
}

function isUncertaintyCallExpression(expression) {
  return /^\s*(measured|interval|normal|uniform|distribution|ensemble|propagate|probability)\s*\(/.test(
    String(expression ?? "")
  );
}

function uncertaintyCallExampleFromDiagnostic(message) {
  const pattern = /`([^`]+)`/g;
  let match;
  while ((match = pattern.exec(String(message ?? ""))) !== null) {
    const candidate = match[1].trim();
    if (
      /^(measured|interval|normal|uniform|distribution|propagate|ensemble|probability)\s*\(.*\)$/.test(
        candidate
      )
    ) {
      return candidate;
    }
  }
  return undefined;
}

function uncertaintyCallRangeOnLine(lineText) {
  const code = stripLineComment(lineText);
  for (const call of [
    "measured",
    "interval",
    "normal",
    "uniform",
    "distribution",
    "propagate",
    "ensemble",
    "probability"
  ]) {
    let searchStart = 0;
    while (searchStart < code.length) {
      const start = code.indexOf(call, searchStart);
      if (start < 0) {
        break;
      }
      const afterName = start + call.length;
      if (identifierBoundary(code, start, afterName)) {
        let open = afterName;
        while (open < code.length && /\s/.test(code[open])) {
          open += 1;
        }
        if (code[open] === "(") {
          const close = matchingCloseParenIndex(code, open);
          if (close !== undefined) {
            return { start, end: close + 1 };
          }
        }
      }
      searchStart = afterName;
    }
  }
  return undefined;
}

function namedArgumentValueRange(lineText, names) {
  const code = stripLineComment(lineText);
  for (const name of names) {
    const pattern = new RegExp(`(^|[^A-Za-z0-9_])(${escapeRegExp(name)})(\\s*=\\s*)`, "g");
    let match;
    while ((match = pattern.exec(code)) !== null) {
      const nameStart = match.index + match[1].length;
      const valueStart = nameStart + name.length + match[3].length;
      let valueEnd = valueStart;
      while (valueEnd < code.length && code[valueEnd] !== "," && code[valueEnd] !== ")") {
        valueEnd += 1;
      }
      while (valueEnd > valueStart && /\s/.test(code[valueEnd - 1])) {
        valueEnd -= 1;
      }
      if (valueEnd > valueStart) {
        return { optionName: name, valueStart, valueEnd };
      }
    }
  }
  return undefined;
}

function matchingCloseParenIndex(text, openIndex) {
  let depth = 0;
  for (let index = openIndex; index < text.length; index += 1) {
    if (text[index] === "(") {
      depth += 1;
    } else if (text[index] === ")") {
      depth -= 1;
      if (depth === 0) {
        return index;
      }
    }
  }
  return undefined;
}

function identifierBoundary(text, start, end) {
  const before = start > 0 ? text[start - 1] : undefined;
  const after = end < text.length ? text[end] : undefined;
  return !isIdentifierCharacter(before) && !isIdentifierCharacter(after);
}

function isIdentifier(value) {
  return typeof value === "string" && /^[A-Za-z_][A-Za-z0-9_]*$/.test(value);
}

function isIdentifierCharacter(value) {
  return typeof value === "string" && /^[A-Za-z0-9_]$/.test(value);
}

function reorderWhereLocalDefinitionAction(document, diagnostic) {
  const name = firstBacktickPayload(diagnostic.message);
  if (!isIdentifier(name)) {
    return undefined;
  }
  const useLine = diagnostic.range.start.line;
  const block = whereBlockRange(document, useLine);
  if (!block) {
    return undefined;
  }
  const definitionLine = whereLocalDefinitionLine(document, name, useLine + 1, block.endLine);
  if (definitionLine === undefined) {
    return undefined;
  }
  const definitionText = document.lineAt(definitionLine).text;
  const definitionCode = stripLineComment(definitionText);
  if (definitionCode.includes("{") || definitionCode.includes("}")) {
    return undefined;
  }

  const action = new vscode.CodeAction(
    `Move ${name} definition before first use`,
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.insert(
    document.uri,
    new vscode.Position(useLine, 0),
    `${definitionText}${documentNewline(document)}`
  );
  action.edit.delete(document.uri, fullLineRange(document, definitionLine));
  return action;
}

function promoteWhereLocalAction(document, diagnostic) {
  const name = firstBacktickPayload(diagnostic.message);
  if (!isIdentifier(name)) {
    return undefined;
  }
  const escapeLine = diagnostic.range.start.line;
  const match = whereBlockDefiningBefore(document, name, escapeLine);
  if (!match) {
    return undefined;
  }
  const definitionText = document.lineAt(match.definitionLine).text;
  const definitionCode = stripLineComment(definitionText);
  if (definitionCode.includes("{") || definitionCode.includes("}")) {
    return undefined;
  }
  const ownerLine = match.block.startLine - 1;
  if (ownerLine < 0) {
    return undefined;
  }
  const promotedDefinition = definitionText.trimStart();
  if (!promotedDefinition) {
    return undefined;
  }
  const removalRange =
    whereBlockMeaningfulLineCount(document, match.block.startLine, match.block.endLine) === 1
      ? fullLineBlockRange(document, match.block.startLine, match.block.endLine)
      : fullLineRange(document, match.definitionLine);

  const action = new vscode.CodeAction(
    `Promote ${name} to top-level binding`,
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.insert(
    document.uri,
    new vscode.Position(ownerLine, 0),
    `${promotedDefinition}${documentNewline(document)}`
  );
  action.edit.delete(document.uri, removalRange);
  return action;
}

function whereBlockDefiningBefore(document, name, lineLimit) {
  let selected;
  for (let startLine = 0; startLine < lineLimit; startLine += 1) {
    if (!isWhereBlockStart(document.lineAt(startLine).text)) {
      continue;
    }
    const endLine = findMatchingBlockEnd(document, startLine);
    if (endLine === undefined || endLine >= lineLimit) {
      continue;
    }
    const definitionLine = whereLocalDefinitionLine(document, name, startLine + 1, endLine);
    if (definitionLine !== undefined) {
      selected = { block: { startLine, endLine }, definitionLine };
    }
  }
  return selected;
}

function whereBlockMeaningfulLineCount(document, startLine, endLine) {
  let count = 0;
  for (let lineNumber = startLine + 1; lineNumber < endLine; lineNumber += 1) {
    if (stripLineComment(document.lineAt(lineNumber).text).trim()) {
      count += 1;
    }
  }
  return count;
}

function firstBacktickPayload(message) {
  return /`([^`]+)`/.exec(String(message ?? ""))?.[1]?.trim();
}

function whereBlockRange(document, lineNumber) {
  for (let startLine = lineNumber; startLine >= 0; startLine -= 1) {
    if (!isWhereBlockStart(document.lineAt(startLine).text)) {
      continue;
    }
    const endLine = findMatchingBlockEnd(document, startLine);
    if (endLine !== undefined && endLine > lineNumber) {
      return { startLine, endLine };
    }
  }
  return undefined;
}

function isWhereBlockStart(text) {
  const trimmed = stripLineComment(text).trim();
  if (!trimmed.startsWith("where")) {
    return false;
  }
  return trimmed.slice("where".length).trim() === "{";
}

function whereLocalDefinitionLine(document, name, startLine, endLine) {
  for (let lineNumber = startLine; lineNumber < endLine; lineNumber += 1) {
    const code = stripLineComment(document.lineAt(lineNumber).text);
    if (whereLocalDefinitionMatches(code, name)) {
      return lineNumber;
    }
  }
  return undefined;
}

function whereLocalDefinitionMatches(text, name) {
  const trimmed = text.trimStart();
  if (!trimmed.startsWith(name)) {
    return false;
  }
  const rest = trimmed.slice(name.length);
  if (isIdentifierCharacter(rest[0])) {
    return false;
  }
  return rest.trimStart().startsWith("=");
}

function commandTargetParenthesesAction(document, diagnostic) {
  const message = String(diagnostic.message ?? "");
  if (!message.includes("ambiguous without parentheses")) {
    return undefined;
  }
  const target = commandTargetFromDiagnostic(message);
  if (!target || target.startsWith("(")) {
    return undefined;
  }
  const line = document.lineAt(diagnostic.range.start.line);
  const code = stripLineComment(line.text);
  const startCharacter = code.indexOf(target);
  if (startCharacter < 0) {
    return undefined;
  }
  const endCharacter = startCharacter + target.length;
  const before = firstNonWhitespaceFromRight(code.slice(0, startCharacter));
  const after = firstNonWhitespace(code.slice(endCharacter));
  if (before === "(" && after === ")") {
    return undefined;
  }

  const action = new vscode.CodeAction("Parenthesize command target", vscode.CodeActionKind.QuickFix);
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(line.lineNumber, startCharacter, line.lineNumber, endCharacter),
    `(${target})`
  );
  return action;
}

function commandTargetFromDiagnostic(message) {
  return /Command target `([^`]+)` is ambiguous without parentheses\./
    .exec(String(message ?? ""))?.[1]?.trim();
}

function firstNonWhitespace(text) {
  return String(text ?? "").match(/\S/)?.[0];
}

function firstNonWhitespaceFromRight(text) {
  const chars = Array.from(String(text ?? ""));
  for (let index = chars.length - 1; index >= 0; index -= 1) {
    if (/\S/.test(chars[index])) {
      return chars[index];
    }
  }
  return undefined;
}

function optionAssignmentRange(lineText, optionNames) {
  const options = optionNames.map(escapeRegExp).join("|");
  const code = stripLineComment(lineText);
  const match = new RegExp(`^(\\s*)(${options})(\\s*=\\s*)(.*?)(\\s*)$`).exec(code);
  if (!match) {
    return undefined;
  }
  const valueStart = match[1].length + match[2].length + match[3].length;
  const valueEnd = valueStart + match[4].trimEnd().length;
  return { optionName: match[2], valueStart, valueEnd };
}

function booleanWithOptionsAction(document, diagnostic, optionNames) {
  const lineNumber = diagnostic.range.start.line;
  const attachedBlock = attachedWithBlock(document, lineNumber);
  const missingOptions = optionNames.filter(
    (optionName) => !attachedBlock || !withBlockContainsOption(document, attachedBlock, optionName)
  );
  if (missingOptions.length === 0) {
    return undefined;
  }

  const title =
    missingOptions.length === 1
      ? `Add ${missingOptions[0]} = true`
      : `Add ${missingOptions.join(" = true and ")} = true`;
  const action = new vscode.CodeAction(title, vscode.CodeActionKind.QuickFix);
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  if (attachedBlock) {
    const insertion = missingOptions
      .map((optionName) => `${attachedBlock.indent}    ${optionName} = true`)
      .join(documentNewline(document));
    action.edit.insert(
      document.uri,
      new vscode.Position(attachedBlock.endLine, 0),
      `${insertion}${documentNewline(document)}`
    );
  } else {
    const indent = lineIndent(document.lineAt(lineNumber).text);
    const optionLines = missingOptions
      .map((optionName) => `${indent}    ${optionName} = true`)
      .join(documentNewline(document));
    action.edit.insert(
      document.uri,
      document.lineAt(lineNumber).range.end,
      `${documentNewline(document)}${indent}with {${documentNewline(document)}${optionLines}${documentNewline(document)}${indent}}`
    );
  }
  return action;
}

function sampleSeedMissingAction(document, diagnostic) {
  const lineNumber = diagnostic.range.start.line;
  if (lineNumber < 0 || lineNumber >= document.lineCount) {
    return undefined;
  }
  const ownerLine = document.lineAt(lineNumber);
  if (!isSampleGenerationOwnerLine(ownerLine.text)) {
    return undefined;
  }
  const attachedBlock = attachedWithBlock(document, lineNumber);
  if (attachedBlock && withBlockContainsOption(document, attachedBlock, "seed")) {
    return undefined;
  }

  const action = new vscode.CodeAction("Add sample seed: seed = 42", vscode.CodeActionKind.QuickFix);
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  if (attachedBlock) {
    action.edit.insert(
      document.uri,
      new vscode.Position(attachedBlock.endLine, 0),
      `${attachedBlock.indent}    seed = 42${documentNewline(document)}`
    );
  } else {
    const indent = lineIndent(ownerLine.text);
    action.edit.insert(
      document.uri,
      ownerLine.range.end,
      `${documentNewline(document)}${indent}with {${documentNewline(document)}${indent}    seed = 42${documentNewline(document)}${indent}}`
    );
  }
  return action;
}

function isSampleGenerationOwnerLine(text) {
  const code = stripLineComment(text).trim();
  const equalsIndex = code.indexOf("=");
  if (equalsIndex < 0) {
    return false;
  }
  const expression = code.slice(equalsIndex + 1).trim().toLowerCase();
  return /^sample\s+(grid|random|uniform|lhs|latin_hypercube|latin-hypercube)$/.test(expression);
}

function uncertaintySeedMissingAction(document, diagnostic) {
  const lineNumber = diagnostic.range.start.line;
  if (lineNumber < 0 || lineNumber >= document.lineCount) {
    return undefined;
  }
  const block = withBlockContainingLine(document, lineNumber);
  if (!block || withBlockContainsOption(document, block, "seed")) {
    return undefined;
  }
  const action = new vscode.CodeAction("Add uncertainty seed: seed = 7", vscode.CodeActionKind.QuickFix);
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.insert(
    document.uri,
    new vscode.Position(block.endLine, 0),
    `${block.indent}    seed = 7${documentNewline(document)}`
  );
  return action;
}

function withBlockContainingLine(document, lineNumber) {
  for (let cursor = lineNumber; cursor >= 0; cursor -= 1) {
    const line = document.lineAt(cursor);
    if (stripLineComment(line.text).trim() !== "with {") {
      continue;
    }
    const endLine = findMatchingBlockEnd(document, cursor);
    if (endLine !== undefined && endLine > lineNumber) {
      return { startLine: cursor, endLine, indent: lineIndent(line.text) };
    }
  }
  return undefined;
}

function attachedWithBlock(document, ownerLineNumber) {
  let lineNumber = ownerLineNumber + 1;
  while (lineNumber < document.lineCount && document.lineAt(lineNumber).text.trim() === "") {
    lineNumber += 1;
  }
  if (lineNumber >= document.lineCount) {
    return undefined;
  }
  const line = document.lineAt(lineNumber);
  if (!/^\s*with\s*\{\s*$/.test(line.text)) {
    return undefined;
  }
  const endLine = findMatchingBlockEnd(document, lineNumber);
  if (endLine === undefined || endLine <= lineNumber) {
    return undefined;
  }
  return { startLine: lineNumber, endLine, indent: lineIndent(line.text) };
}

function withBlockContainsOption(document, block, optionName) {
  const pattern = new RegExp(`^\\s*${escapeRegExp(optionName)}\\s*=`);
  for (let lineNumber = block.startLine + 1; lineNumber < block.endLine; lineNumber += 1) {
    if (pattern.test(stripLineComment(document.lineAt(lineNumber).text))) {
      return true;
    }
  }
  return false;
}

function documentNewline(document) {
  return document.eol === vscode.EndOfLine.CRLF ? "\r\n" : "\n";
}

function lineIndent(text) {
  return /^(\s*)/.exec(text)?.[1] ?? "";
}

function escapeRegExp(text) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
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
  const index = lineCommentStart(text);
  return index >= 0 ? text.slice(0, index) : text;
}

function lineCommentStart(text) {
  let inString = false;
  for (let index = 0; index < text.length; index += 1) {
    const character = text[index];
    if (character === "\\" && inString) {
      index += 1;
      continue;
    }
    if (character === '"') {
      inString = !inString;
      continue;
    }
    if (!inString && character === "#") {
      return index;
    }
    if (!inString && character === "/" && text[index + 1] === "/") {
      return index;
    }
  }
  return -1;
}

function fullLineRange(document, lineNumber) {
  const line = document.lineAt(lineNumber);
  if (lineNumber + 1 < document.lineCount) {
    return new vscode.Range(lineNumber, 0, lineNumber + 1, 0);
  }
  return new vscode.Range(lineNumber, 0, lineNumber, line.text.length);
}

function fullLineBlockRange(document, startLine, endLine) {
  const end = document.lineAt(endLine);
  if (endLine + 1 < document.lineCount) {
    return new vscode.Range(startLine, 0, endLine + 1, 0);
  }
  return new vscode.Range(startLine, 0, endLine, end.text.length);
}

module.exports = {
  localCodeActions,
  diagnosticCode
};
