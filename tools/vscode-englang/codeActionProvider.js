const vscode = require("vscode");
const { localCodeActions } = require("./localCodeActions");
const { lspCodeActionsFromPayload } = require("./lspCodeActions");

class EngCodeActionProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.codeActionsForDocumentSource = options.codeActionsForDocumentSource;
  }

  async provideCodeActions(document, _range, context, cancellationToken) {
    const payload = await this.codeActionsForDocumentSource?.(
      document,
      this.context,
      cancellationToken
    );
    const lspActions = lspCodeActionsFromPayload(document, payload, context.diagnostics);
    const localActions = localCodeActions(document, context);
    return mergeCodeActions(lspActions, localActions);
  }
}

function mergeCodeActions(primaryActions, fallbackActions) {
  const merged = [];
  const seen = new Set();
  for (const action of [...primaryActions, ...fallbackActions]) {
    if (!action) {
      continue;
    }
    const key = codeActionKey(action);
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    merged.push(action);
  }
  return merged;
}

function codeActionKey(action) {
  const kind = action?.kind?.value ?? String(action?.kind ?? "");
  return `${action?.title ?? ""}\n${kind}\n${codeActionEditKey(action?.edit)}`;
}

function codeActionEditKey(edit) {
  if (!edit || typeof edit.entries !== "function") {
    return "";
  }
  return edit.entries()
    .map(([uri, edits]) => {
      const editKeys = edits.map((textEdit) => {
        const range = textEdit.range;
        return [
          range.start.line,
          range.start.character,
          range.end.line,
          range.end.character,
          textEdit.newText
        ].join(":");
      });
      return `${uri.toString()}:${editKeys.join("|")}`;
    })
    .sort()
    .join("\n");
}

module.exports = {
  EngCodeActionProvider,
  mergeCodeActions
};
