const vscode = require("vscode");

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
    if (typeof code === "string" && code.startsWith("E-DIM-ADD-")) {
      actions.push(...missingUnitActions(document, diagnostic));
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
    if (code === "E-WITH-OPTION-001") {
      const action = withOptionAliasAction(document, diagnostic);
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
    if (code === "E-LOG-LEVEL-001") {
      const action = logLevelInfoAction(document, diagnostic);
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
    if (code === "E-CMD-AMBIG-001") {
      const action = commandTargetParenthesesAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-STDLIB-MODULE-UNKNOWN") {
      const action = stdlibModuleReplacementAction(document, diagnostic, options.completionSeed);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    const optionAction = optionValueReplacementAction(document, diagnostic, optionQuickFix(code));
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

function stdlibModuleReplacementAction(document, diagnostic, completionSeed) {
  const unknown = stdlibModuleNameFromDiagnostic(diagnostic.message);
  if (!unknown) {
    return undefined;
  }
  const replacement = closestStdlibModuleName(unknown, completionSeed);
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

function stdlibModuleNameFromDiagnostic(message) {
  const candidates = String(message ?? "").match(/`eng\.[A-Za-z0-9_.-]+`/g) ?? [];
  const last = candidates.at(-1);
  return last ? last.slice(1, -1) : undefined;
}

function closestStdlibModuleName(unknown, completionSeed) {
  const moduleNames = stdlibModuleNamesFromCompletionSeed(completionSeed)
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

function stdlibModuleNamesFromCompletionSeed(completionSeed) {
  return Array.from(
    new Set(
      (Array.isArray(completionSeed) ? completionSeed : [])
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

function missingUnitActions(document, diagnostic) {
  const line = document.lineAt(diagnostic.range.start.line);
  const unit = missingUnitHint(diagnostic.message, line.text);
  if (!unit) {
    return [];
  }

  return bareNumericRanges(line.text).map((range) => {
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

function missingUnitHint(message, lineText) {
  const fromHelp =
    /(?:such as|write)\s+`([^`]+)`/.exec(message)?.[1] ??
    /unit such as\s+`([^`]+)`/.exec(message)?.[1];
  if (isUnitHint(fromHelp)) {
    return fromHelp;
  }
  return firstUnitOnLine(lineText);
}

function firstUnitOnLine(lineText) {
  const unitLiteral = /\b\d+(?:\.\d+)?\s+([A-Za-z%][A-Za-z0-9/%]*)\b/.exec(lineText);
  if (isUnitHint(unitLiteral?.[1])) {
    return unitLiteral[1];
  }
  const bracketUnit = /\[([A-Za-z%][A-Za-z0-9/%]*)\]/.exec(lineText);
  if (isUnitHint(bracketUnit?.[1])) {
    return bracketUnit[1];
  }
  return undefined;
}

function isUnitHint(value) {
  return typeof value === "string" && /^[A-Za-z%][A-Za-z0-9/%]*$/.test(value);
}

function bareNumericRanges(lineText) {
  const ranges = [];
  const pattern = /(^|[=+\-*/(,]\s*)(\d+(?:\.\d+)?)(?!\s*[A-Za-z_%])/g;
  let match;
  while ((match = pattern.exec(lineText)) !== null) {
    const literalStart = match.index + match[1].length;
    const literalEnd = literalStart + match[2].length;
    ranges.push({ start: literalStart, end: literalEnd });
  }
  return ranges;
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
    default:
      return undefined;
  }
}

function optionValueReplacementAction(document, diagnostic, fix) {
  if (!fix) {
    return undefined;
  }
  const line = document.lineAt(diagnostic.range.start.line);
  const assignment = optionAssignmentRange(line.text, fix.optionNames);
  if (!assignment) {
    return undefined;
  }
  const optionLabel = fix.optionNames.length === 1 ? fix.optionNames[0] : assignment.optionName;
  const action = new vscode.CodeAction(
    `${fix.label}: ${optionLabel} = ${fix.value}`,
    vscode.CodeActionKind.QuickFix
  );
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(line.lineNumber, assignment.valueStart, line.lineNumber, assignment.valueEnd),
    fix.value
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

function withOptionAliasAction(document, diagnostic) {
  const fix = withOptionAliasFix(unknownWithOptionName(diagnostic.message));
  if (!fix) {
    return undefined;
  }
  const line = document.lineAt(diagnostic.range.start.line);
  const pattern = new RegExp(`^(\\s*)${escapeRegExp(fix.from)}(\\s*=)`);
  const match = pattern.exec(stripLineComment(line.text));
  if (!match) {
    return undefined;
  }
  const startCharacter = match[1].length;
  const action = new vscode.CodeAction(fix.title, vscode.CodeActionKind.QuickFix);
  action.diagnostics = [diagnostic];
  action.edit = new vscode.WorkspaceEdit();
  action.edit.replace(
    document.uri,
    new vscode.Range(line.lineNumber, startCharacter, line.lineNumber, startCharacter + fix.from.length),
    fix.to
  );
  return action;
}

function withOptionAliasFix(optionName) {
  switch (optionName) {
    case "unit":
    case "y_unit":
      return {
        from: optionName,
        to: "unit y",
        title: "Use plot y-axis option: unit y ="
      };
    case "x_unit":
      return {
        from: optionName,
        to: "unit x",
        title: "Use plot x-axis option: unit x ="
      };
    case "confidence":
      return {
        from: optionName,
        to: "confidence_band",
        title: "Use confidence band option: confidence_band ="
      };
    default:
      return undefined;
  }
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
      const placeholder = `${indent}${source} = normal(mean=5 kW, std=0.8 kW, samples=31)`;
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
      const placeholder = `${indent}${source} = normal(mean=5 kW, std=0.8 kW, samples=31)`;
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
  const match = new RegExp(`^(\\s*)(${options})(\\s*=\\s*)([^#]*?)(\\s*(?:#.*)?)$`).exec(lineText);
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
