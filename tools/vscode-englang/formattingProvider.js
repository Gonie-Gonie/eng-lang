const vscode = require("vscode");

class EngFormattingProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.isEngDocument = options.isEngDocument ?? (() => true);
    this.formatDocumentSource = options.formatDocumentSource;
  }

  async provideDocumentFormattingEdits(document, _options, cancellationToken) {
    if (!this.isEngDocument(document)) {
      return [];
    }
    const payload = await this.formatDocumentSource?.(
      document,
      this.context,
      cancellationToken
    );
    if (!payload?.changed || typeof payload.formatted !== "string") {
      return [];
    }
    return [vscode.TextEdit.replace(fullDocumentRange(document), payload.formatted)];
  }
}

function fullDocumentRange(document) {
  if (document.lineCount === 0) {
    return new vscode.Range(0, 0, 0, 0);
  }
  const lastLine = document.lineAt(document.lineCount - 1);
  return new vscode.Range(0, 0, lastLine.lineNumber, lastLine.text.length);
}

module.exports = {
  EngFormattingProvider
};
