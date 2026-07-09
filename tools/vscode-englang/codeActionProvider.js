const vscode = require("vscode");
const { localCodeActions } = require("./localCodeActions");
const { lspCodeActionsFromPayload } = require("./lspCodeActions");

class EngCodeActionProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.codeActionsForDocumentSource = options.codeActionsForDocumentSource;
    this.completionItems = Array.isArray(options.completionItems) ? options.completionItems : [];
  }

  async provideCodeActions(document, _range, context, cancellationToken) {
    if (!shouldProvideQuickFixes(context)) {
      return [];
    }

    const localActions = () => localCodeActions(document, context, {
      completionItems: this.completionItems
    });
    if (cancellationToken?.isCancellationRequested) {
      return [];
    }

    const documentVersion = document.version;
    let payload;
    try {
      payload = await this.codeActionsForDocumentSource?.(
        document,
        this.context,
        cancellationToken
      );
    } catch (_error) {
      if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
        return [];
      }
      return localActions();
    }
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return [];
    }

    const lspActions = lspCodeActionsFromPayload(document, payload, context.diagnostics);
    return mergeCodeActions(lspActions, localActions());
  }
}

function shouldProvideQuickFixes(context) {
  return codeActionKindIntersects(context?.only, vscode.CodeActionKind.QuickFix);
}

function codeActionKindIntersects(requestedKind, providedKind) {
  if (!requestedKind) {
    return true;
  }
  if (typeof requestedKind.intersects === "function") {
    return requestedKind.intersects(providedKind);
  }
  if (typeof requestedKind.contains === "function" && requestedKind.contains(providedKind)) {
    return true;
  }
  if (typeof providedKind.contains === "function" && providedKind.contains(requestedKind)) {
    return true;
  }

  const requestedValue = codeActionKindValue(requestedKind);
  const providedValue = codeActionKindValue(providedKind);
  return (
    requestedValue === providedValue ||
    requestedValue.startsWith(`${providedValue}.`) ||
    providedValue.startsWith(`${requestedValue}.`)
  );
}

function codeActionKindValue(kind) {
  return kind?.value ?? String(kind ?? "");
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
