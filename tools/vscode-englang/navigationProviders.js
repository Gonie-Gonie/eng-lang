const {
  definitionLocationFromLsp,
  definitionLocationFromSnapshotSymbols,
  definitionNameCandidates,
  documentHighlightsFromLsp,
  documentSymbolsFromLsp,
  documentSymbolsFromSnapshot,
  prepareRenameFromLsp,
  referenceLocationsFromLsp,
  workspaceEditFromLsp,
  workspaceSymbolInformationFromLsp
} = require("./lspNavigation");

class EngDocumentSymbolProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.isEngDocument = options.isEngDocument ?? (() => true);
    this.documentSymbolsForDocument = options.documentSymbolsForDocument;
    this.snapshotDocumentSource = options.snapshotDocumentSource;
    this.cacheSnapshotForDocument = options.cacheSnapshotForDocument ?? (() => undefined);
  }

  async provideDocumentSymbols(document, cancellationToken) {
    if (!this.isEngDocument(document)) {
      return [];
    }
    const documentVersion = document.version;
    const protocolSymbols = await this.documentSymbolsForDocument?.(
      document,
      cancellationToken
    );
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return [];
    }
    if (protocolSymbols !== undefined) {
      return documentSymbolsFromLsp(protocolSymbols);
    }

    const snapshot = await this.snapshotDocumentSource?.(document, this.context, cancellationToken);
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return [];
    }
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
    const documentVersion = document.version;
    const liveDefinitionPayload = await this.definitionSnapshotForPosition?.(
      document,
      position,
      this.context,
      cancellationToken
    );
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return undefined;
    }
    const liveDefinition = definitionLocationFromLsp(
      liveDefinitionPayload,
      document.uri,
      this.appendOutputLine
    );
    if (liveDefinition || liveDefinitionPayload !== undefined) {
      return liveDefinition;
    }

    const liveSnapshot = await this.snapshotDocumentSource?.(document, this.context, cancellationToken);
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return undefined;
    }
    const snapshot = liveSnapshot ?? this.cachedSnapshotForDocument(document);
    if (!snapshot) {
      return undefined;
    }
    this.cacheSnapshotForDocument(document, snapshot);

    const candidates = definitionNameCandidates(document, position);
    return definitionLocationFromSnapshotSymbols(snapshot.document_symbols ?? [], candidates, document.uri);
  }
}

class EngDocumentHighlightProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.isEngDocument = options.isEngDocument ?? (() => true);
    this.documentHighlightsForPosition = options.documentHighlightsForPosition;
  }

  async provideDocumentHighlights(document, position, cancellationToken) {
    if (!this.isEngDocument(document)) {
      return [];
    }
    const documentVersion = document.version;
    const payload = await this.documentHighlightsForPosition?.(
      document,
      position,
      this.context,
      cancellationToken
    );
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return [];
    }
    return documentHighlightsFromLsp(payload);
  }
}

class EngReferenceProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.isEngDocument = options.isEngDocument ?? (() => true);
    this.referencesForPosition = options.referencesForPosition;
    this.appendOutputLine = options.appendOutputLine ?? (() => undefined);
  }

  async provideReferences(document, position, referenceContext, cancellationToken) {
    if (!this.isEngDocument(document)) {
      return [];
    }
    const documentVersion = document.version;
    const payload = await this.referencesForPosition?.(
      document,
      position,
      referenceContext?.includeDeclaration !== false,
      this.context,
      cancellationToken
    );
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return [];
    }
    return referenceLocationsFromLsp(payload, document.uri, this.appendOutputLine);
  }
}

class EngRenameProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.isEngDocument = options.isEngDocument ?? (() => true);
    this.prepareRenameForPosition = options.prepareRenameForPosition;
    this.renameForPosition = options.renameForPosition;
    this.appendOutputLine = options.appendOutputLine ?? (() => undefined);
  }

  async prepareRename(document, position, cancellationToken) {
    if (!this.isEngDocument(document)) {
      return undefined;
    }
    const documentVersion = document.version;
    const payload = await this.prepareRenameForPosition?.(
      document,
      position,
      this.context,
      cancellationToken
    );
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return undefined;
    }
    if (payload?.error) {
      throw new Error(String(payload.error));
    }
    return prepareRenameFromLsp(payload);
  }

  async provideRenameEdits(document, position, newName, cancellationToken) {
    if (!this.isEngDocument(document)) {
      return undefined;
    }
    const documentVersion = document.version;
    const payload = await this.renameForPosition?.(
      document,
      position,
      newName,
      this.context,
      cancellationToken
    );
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return undefined;
    }
    if (payload?.error) {
      throw new Error(String(payload.error));
    }
    return workspaceEditFromLsp(payload, this.appendOutputLine);
  }
}

module.exports = {
  EngDefinitionProvider,
  EngDocumentHighlightProvider,
  EngDocumentSymbolProvider,
  EngReferenceProvider,
  EngRenameProvider,
  EngWorkspaceSymbolProvider
};
