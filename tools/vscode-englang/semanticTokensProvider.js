const vscode = require("vscode");
const { semanticTokensFromSnapshot } = require("./lspSemanticTokens");

class EngSemanticTokensProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.isEngDocument = options.isEngDocument ?? (() => true);
    this.snapshotDocumentSource = options.snapshotDocumentSource;
    this.cacheSnapshotForDocument = options.cacheSnapshotForDocument ?? (() => undefined);
    this.updateSemanticSymbolDecorations =
      options.updateSemanticSymbolDecorations ?? (() => undefined);
    this.semanticLegend = options.semanticLegend;
    this.semanticTokenTypes = options.semanticTokenTypes ?? [];
    this.semanticTokenModifiers = options.semanticTokenModifiers ?? [];
  }

  async provideDocumentSemanticTokens(document, cancellationToken) {
    if (!this.isEngDocument(document)) {
      return emptySemanticTokens();
    }
    const config = vscode.workspace.getConfiguration("englang", document.uri);
    if (!config.get("semanticHighlighting.enabled", true)) {
      return emptySemanticTokens();
    }

    const snapshot = await this.snapshotDocumentSource?.(
      document,
      this.context,
      cancellationToken
    );
    if (!snapshot) {
      return emptySemanticTokens();
    }
    this.cacheSnapshotForDocument(document, snapshot);
    this.updateSemanticSymbolDecorations(document, snapshot);
    return semanticTokensFromSnapshot(
      snapshot,
      this.semanticLegend,
      this.semanticTokenTypes,
      this.semanticTokenModifiers
    );
  }
}

function emptySemanticTokens() {
  return new vscode.SemanticTokens(new Uint32Array());
}

module.exports = {
  EngSemanticTokensProvider
};
