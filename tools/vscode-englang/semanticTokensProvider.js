const vscode = require("vscode");
const { semanticTokensFromSnapshot } = require("./lspSemanticTokens");

const SEMANTIC_REFRESH_DEBOUNCE_MS = 350;

class EngSemanticTokensProvider {
  constructor(context, options = {}) {
    this.context = context;
    this._onDidChangeSemanticTokens = new vscode.EventEmitter();
    this.onDidChangeSemanticTokens = this._onDidChangeSemanticTokens.event;
    this.isEngDocument = options.isEngDocument ?? (() => true);
    this.snapshotDocumentSource = options.snapshotDocumentSource;
    this.cacheSnapshotForDocument = options.cacheSnapshotForDocument ?? (() => undefined);
    this.updateReviewValidationDecorations =
      options.updateReviewValidationDecorations ?? (() => undefined);
    this.updateSemanticSymbolDecorations =
      options.updateSemanticSymbolDecorations ?? (() => undefined);
    this.semanticLegend = options.semanticLegend;
    this.semanticTokenTypes = options.semanticTokenTypes ?? [];
    this.semanticTokenModifiers = options.semanticTokenModifiers ?? [];
    this.refreshTimer = undefined;
  }

  async provideDocumentSemanticTokens(document, cancellationToken) {
    if (!this.isEngDocument(document)) {
      return emptySemanticTokens();
    }
    const config = vscode.workspace.getConfiguration("englang", document.uri);
    if (!config.get("semanticHighlighting.enabled", true)) {
      return emptySemanticTokens();
    }

    const documentVersion = document.version;
    const snapshot = await this.snapshotDocumentSource?.(
      document,
      this.context,
      cancellationToken
    );
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return emptySemanticTokens();
    }
    if (!snapshot) {
      return emptySemanticTokens();
    }
    this.cacheSnapshotForDocument(document, snapshot);
    this.updateReviewValidationDecorations(document, snapshot);
    this.updateSemanticSymbolDecorations(document, snapshot);
    return semanticTokensFromSnapshot(
      snapshot,
      this.semanticLegend,
      this.semanticTokenTypes,
      this.semanticTokenModifiers
    );
  }

  refresh() {
    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer);
      this.refreshTimer = undefined;
    }
    this._onDidChangeSemanticTokens.fire();
  }

  scheduleRefresh(delayMs = SEMANTIC_REFRESH_DEBOUNCE_MS) {
    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer);
    }
    this.refreshTimer = setTimeout(() => {
      this.refreshTimer = undefined;
      this.refresh();
    }, delayMs);
  }

  dispose() {
    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer);
      this.refreshTimer = undefined;
    }
    this._onDidChangeSemanticTokens.dispose();
  }
}

function emptySemanticTokens() {
  return new vscode.SemanticTokens(new Uint32Array());
}

module.exports = {
  EngSemanticTokensProvider
};
