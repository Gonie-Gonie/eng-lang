const vscode = require("vscode");
const { lspCodeActionsFromPayload } = require("./lspCodeActions");

class EngCodeActionProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.codeActionsForDocumentRange = options.codeActionsForDocumentRange;
    this.codeActionsForDocumentSource = options.codeActionsForDocumentSource;
    this.appendOutputLine = options.appendOutputLine ?? (() => undefined);
  }

  async provideCodeActions(document, range, context, cancellationToken) {
    if (!shouldProvideQuickFixes(context)) {
      return [];
    }

    if (cancellationToken?.isCancellationRequested) {
      return [];
    }

    const documentVersion = document.version;
    let payload;
    try {
      payload = await this.codeActionsForDocumentRange?.(
        document,
        range,
        context.diagnostics,
        cancellationToken
      );
      if (payload === undefined) {
        payload = await this.codeActionsForDocumentSource?.(
          document,
          this.context,
          cancellationToken
        );
      }
    } catch (error) {
      if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
        return [];
      }
      this.appendOutputLine(`Code action lookup failed: ${error.message}`);
      return [];
    }
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return [];
    }

    return lspCodeActionsFromPayload(document, payload, context.diagnostics);
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

module.exports = {
  EngCodeActionProvider
};
