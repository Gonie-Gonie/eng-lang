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
    if (code === "E-NET-HASH-MISMATCH") {
      const action = expectedSha256Action(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
    }
    if (code === "E-WITH-OPTION-001") {
      const action = plotUnitOptionAction(document, diagnostic);
      if (action) {
        action.isPreferred = true;
        actions.push(action);
      }
      const confidenceAction = confidenceBandOptionAction(document, diagnostic);
      if (confidenceAction) {
        confidenceAction.isPreferred = true;
        actions.push(confidenceAction);
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
    case "E-SAMPLING-SEED-INVALID":
      return { optionNames: ["seed"], value: "42", label: "Set sample seed" };
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

function expectedSha256FromDiagnostic(diagnostic) {
  const match = /(?:fixture SHA256 was|observed) `([0-9a-fA-F]{64})`/.exec(
    diagnostic.message ?? ""
  );
  return match ? match[1].toLowerCase() : undefined;
}

function plotUnitOptionAction(document, diagnostic) {
  return withOptionRenameAction(document, diagnostic, {
    from: "unit",
    to: "unit y",
    title: "Use plot y-axis option: unit y ="
  });
}

function confidenceBandOptionAction(document, diagnostic) {
  return withOptionRenameAction(document, diagnostic, {
    from: "confidence",
    to: "confidence_band",
    title: "Use confidence band option: confidence_band ="
  });
}

function withOptionRenameAction(document, diagnostic, fix) {
  if (unknownWithOptionName(diagnostic.message) !== fix.from) {
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

function unknownWithOptionName(message) {
  const match = /Unknown with option `([^`]+)`/.exec(String(message ?? ""));
  return match?.[1]?.trim();
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

module.exports = {
  localCodeActions,
  diagnosticCode
};
