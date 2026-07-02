const vscode = require("vscode");
const { diagnosticCode } = require("./localCodeActions");

function lspCodeActionsFromPayload(document, payload, contextDiagnostics) {
  const lspActions = Array.isArray(payload?.actions)
    ? payload.actions
    : (Array.isArray(payload) ? payload : []);
  return lspActions
    .map((action) => vscodeCodeActionFromLsp(document, action, contextDiagnostics))
    .filter((action) => action !== undefined);
}

function vscodeCodeActionFromLsp(document, lspAction, contextDiagnostics) {
  if (!lspAction || typeof lspAction.title !== "string") {
    return undefined;
  }
  if (
    Array.isArray(contextDiagnostics) &&
    contextDiagnostics.length > 0 &&
    !lspActionMatchesDiagnostics(lspAction, contextDiagnostics)
  ) {
    return undefined;
  }
  const edit = workspaceEditFromLspCodeAction(document, lspAction);
  if (!edit) {
    return undefined;
  }
  const action = new vscode.CodeAction(lspAction.title, codeActionKindFromLsp(lspAction.kind));
  action.isPreferred = lspAction.isPreferred === true;
  action.edit = edit;
  action.diagnostics = matchingDiagnosticsForLspAction(lspAction, contextDiagnostics);
  return action;
}

function codeActionKindFromLsp(kind) {
  return kind === "quickfix" ? vscode.CodeActionKind.QuickFix : vscode.CodeActionKind.Empty;
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
  const entry =
    entries.find(([uri]) => documentUris.has(uri)) ??
    (entries.length === 1 ? entries[0] : undefined);
  if (!entry || !Array.isArray(entry[1])) {
    return undefined;
  }

  const workspaceEdit = new vscode.WorkspaceEdit();
  let hasEdit = false;
  for (const edit of entry[1]) {
    const range = vscodeRangeFromLsp(edit.range);
    if (!range || typeof edit.newText !== "string") {
      continue;
    }
    workspaceEdit.replace(document.uri, range, edit.newText);
    hasEdit = true;
  }
  return hasEdit ? workspaceEdit : undefined;
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

function vscodeRangeFromLsp(range) {
  if (!range?.start || !range?.end) {
    return undefined;
  }
  return new vscode.Range(
    Number(range.start.line ?? 0),
    Number(range.start.character ?? 0),
    Number(range.end.line ?? range.start.line ?? 0),
    Number(range.end.character ?? range.start.character ?? 0)
  );
}

module.exports = {
  lspCodeActionsFromPayload
};
