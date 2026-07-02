const {
  definitionLocationFromLsp,
  definitionLocationFromSnapshotSymbols,
  definitionNameCandidates,
  documentSymbolsFromSnapshot,
  workspaceSymbolInformationFromLsp
} = require("./lspNavigation");

class EngDocumentSymbolProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.isEngDocument = options.isEngDocument ?? (() => true);
    this.snapshotDocumentSource = options.snapshotDocumentSource;
    this.cacheSnapshotForDocument = options.cacheSnapshotForDocument ?? (() => undefined);
  }

  async provideDocumentSymbols(document, cancellationToken) {
    if (!this.isEngDocument(document)) {
      return [];
    }
    const snapshot = await this.snapshotDocumentSource?.(document, this.context, cancellationToken);
    if (!snapshot) {
      return [];
    }
    this.cacheSnapshotForDocument(document, snapshot);
    return documentSymbolsFromSnapshot(snapshot);
  }
}

class EngWorkspaceSymbolProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.workspaceSymbolsForQuery = options.workspaceSymbolsForQuery ?? (() => []);
    this.appendOutputLine = options.appendOutputLine ?? (() => undefined);
  }

  async provideWorkspaceSymbols(query, cancellationToken) {
    const symbols = await this.workspaceSymbolsForQuery(query, this.context, cancellationToken);
    return symbols
      .map((symbol) => workspaceSymbolInformationFromLsp(symbol, this.appendOutputLine))
      .filter((symbol) => symbol !== undefined);
  }
}

class EngDefinitionProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.isEngDocument = options.isEngDocument ?? (() => true);
    this.definitionSnapshotForPosition = options.definitionSnapshotForPosition;
    this.snapshotDocumentSource = options.snapshotDocumentSource;
    this.cachedSnapshotForDocument = options.cachedSnapshotForDocument ?? (() => undefined);
    this.cacheSnapshotForDocument = options.cacheSnapshotForDocument ?? (() => undefined);
    this.appendOutputLine = options.appendOutputLine ?? (() => undefined);
  }

  async provideDefinition(document, position, cancellationToken) {
    if (!this.isEngDocument(document)) {
      return undefined;
    }
    const liveDefinition = definitionLocationFromLsp(
      await this.definitionSnapshotForPosition?.(document, position, this.context, cancellationToken),
      document.uri,
      this.appendOutputLine
    );
    if (liveDefinition) {
      return liveDefinition;
    }

    const snapshot =
      (await this.snapshotDocumentSource?.(document, this.context, cancellationToken)) ??
      this.cachedSnapshotForDocument(document);
    if (!snapshot) {
      return undefined;
    }
    this.cacheSnapshotForDocument(document, snapshot);

    const candidates = definitionNameCandidates(document, position);
    return definitionLocationFromSnapshotSymbols(snapshot.document_symbols ?? [], candidates, document.uri);
  }
}

module.exports = {
  EngDefinitionProvider,
  EngDocumentSymbolProvider,
  EngWorkspaceSymbolProvider
};
