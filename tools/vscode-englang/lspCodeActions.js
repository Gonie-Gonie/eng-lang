const vscode = require("vscode");
const { vscodeRangeFromLsp } = require("./lspRanges");

function lspCodeActionsFromPayload(document, payload, contextDiagnostics) {
  const lspActions = Array.isArray(payload?.actions)
    ? payload.actions
    : (Array.isArray(payload) ? payload : []);
  return lspActions
    .map((action) => vscodeCodeActionFromLsp(document, action, contextDiagnostics))
    .filter((action) => action !== undefined);
}

function vscodeCodeActionFromLsp(document, lspAction, contextDiagnostics) {
  if (
    !lspAction ||
    typeof lspAction.title !== "string" ||
    lspAction.title.trim().length === 0 ||
    lspAction.kind !== "quickfix"
  ) {
    return undefined;
  }
  if (
    !Array.isArray(contextDiagnostics) ||
    contextDiagnostics.length === 0 ||
    !lspActionMatchesDiagnostics(lspAction, contextDiagnostics)
  ) {
    return undefined;
  }
  const edit = workspaceEditFromLspCodeAction(document, lspAction);
  if (!edit) {
    return undefined;
  }
  const action = new vscode.CodeAction(lspAction.title, vscode.CodeActionKind.QuickFix);
  action.isPreferred = lspAction.isPreferred === true;
  action.edit = edit;
  action.diagnostics = matchingDiagnosticsForLspAction(lspAction, contextDiagnostics);
  return action;
}

function workspaceEditFromLspCodeAction(document, lspAction) {
  const changes = lspAction.edit?.changes;
  if (!changes || typeof changes !== "object") {
    return undefined;
  }
  const entries = Object.entries(changes);
  const documentUris = new Set([
    document.uri.toString(),
    document.uri.toString(true)
  ]);
  if (entries.length !== 1) {
    return undefined;
  }
  const [uri, edits] = entries[0];
  if (!documentUris.has(uri) || !Array.isArray(edits) || edits.length === 0) {
    return undefined;
  }

  const convertedEdits = [];
  for (const edit of edits) {
    const range = vscodeRangeFromLsp(edit?.range);
    if (
      !range ||
      typeof edit?.newText !== "string" ||
      !rangeIsInsideDocument(document, range)
    ) {
      return undefined;
    }
    convertedEdits.push({ range, newText: edit.newText });
  }
  if (editRangesConflict(convertedEdits.map((edit) => edit.range))) {
    return undefined;
  }

  const workspaceEdit = new vscode.WorkspaceEdit();
  for (const edit of convertedEdits) {
    workspaceEdit.replace(document.uri, edit.range, edit.newText);
  }
  return workspaceEdit;
}

function rangeIsInsideDocument(document, range) {
  if (
    !Number.isInteger(document?.lineCount) ||
    document.lineCount <= 0 ||
    typeof document.lineAt !== "function" ||
    comparePositions(range.start, range.end) > 0
  ) {
    return false;
  }
  for (const position of [range.start, range.end]) {
    if (
      position.line < 0 ||
      position.line >= document.lineCount ||
      position.character < 0
    ) {
      return false;
    }
    let line;
    try {
      line = document.lineAt(position.line);
    } catch (_error) {
      return false;
    }
    if (typeof line?.text !== "string" || position.character > line.text.length) {
      return false;
    }
  }
  return true;
}

function editRangesConflict(ranges) {
  const ordered = [...ranges].sort((left, right) => {
    const startOrder = comparePositions(left.start, right.start);
    return startOrder !== 0 ? startOrder : comparePositions(left.end, right.end);
  });
  for (let index = 1; index < ordered.length; index += 1) {
    const previous = ordered[index - 1];
    const current = ordered[index];
    if (
      comparePositions(previous.start, current.start) === 0 ||
      comparePositions(previous.end, current.start) > 0
    ) {
      return true;
    }
  }
  return false;
}

function comparePositions(left, right) {
  return left.line === right.line
    ? left.character - right.character
    : left.line - right.line;
}

function lspActionMatchesDiagnostics(lspAction, contextDiagnostics) {
  const actionDiagnostics = Array.isArray(lspAction.diagnostics)
    ? lspAction.diagnostics
    : [];
  return actionDiagnostics.some((lspDiagnostic) =>
    contextDiagnostics.some((diagnostic) => lspDiagnosticMatchesVscode(lspDiagnostic, diagnostic))
  );
}

function matchingDiagnosticsForLspAction(lspAction, contextDiagnostics) {
  if (!Array.isArray(contextDiagnostics) || contextDiagnostics.length === 0) {
    return [];
  }
  const actionDiagnostics = Array.isArray(lspAction.diagnostics)
    ? lspAction.diagnostics
    : [];
  return contextDiagnostics.filter((diagnostic) =>
    actionDiagnostics.some((lspDiagnostic) => lspDiagnosticMatchesVscode(lspDiagnostic, diagnostic))
  );
}

function lspDiagnosticMatchesVscode(lspDiagnostic, diagnostic) {
  const lspCode = typeof lspDiagnostic?.code === "string"
    ? lspDiagnostic.code
    : lspDiagnostic?.code?.value;
  if (lspCode !== diagnosticCode(diagnostic)) {
    return false;
  }
  const lspRange = vscodeRangeFromLsp(lspDiagnostic.range);
  if (!lspRange) {
    return false;
  }
  return (
    lspRange.start.line === diagnostic.range.start.line &&
    lspRange.start.character === diagnostic.range.start.character &&
    lspRange.end.line === diagnostic.range.end.line &&
    lspRange.end.character === diagnostic.range.end.character
  );
}

function diagnosticCode(diagnostic) {
  if (typeof diagnostic?.code === "string") {
    return diagnostic.code;
  }
  return diagnostic?.code?.value;
}

module.exports = {
  lspCodeActionsFromPayload
};
