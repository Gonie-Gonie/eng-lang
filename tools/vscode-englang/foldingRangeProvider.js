const vscode = require("vscode");
const { foldingRangeKindFromLsp } = require("./lspKinds");

class EngFoldingRangeProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.isEngDocument = options.isEngDocument ?? (() => true);
    this.foldingRangesForDocument = options.foldingRangesForDocument;
    this.snapshotDocumentSource = options.snapshotDocumentSource;
    this.cacheSnapshotForDocument = options.cacheSnapshotForDocument ?? (() => undefined);
  }

  async provideFoldingRanges(document, _context, cancellationToken) {
    if (!this.isEngDocument(document)) {
      return [];
    }
    const documentVersion = document.version;
    const protocolRanges = await this.foldingRangesForDocument?.(
      document,
      cancellationToken
    );
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return [];
    }
    if (protocolRanges !== undefined) {
      return foldingRangesFromLsp(protocolRanges);
    }

    const snapshot = await this.snapshotDocumentSource?.(
      document,
      this.context,
      cancellationToken
    );
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return [];
    }
    if (!snapshot) {
      return [];
    }
    this.cacheSnapshotForDocument(document, snapshot);
    return foldingRangesFromSnapshot(snapshot);
  }
}

function foldingRangesFromSnapshot(snapshot) {
  return foldingRangesFromLsp(snapshot.folding_ranges);
}

function foldingRangesFromLsp(payload) {
  return (Array.isArray(payload) ? payload : [])
    .map(foldingRangeFromSnapshot)
    .filter((range) => range !== undefined);
}

function foldingRangeFromSnapshot(range) {
  const startLine = range?.startLine;
  const endLine = range?.endLine;
  if (!Number.isInteger(startLine) || !Number.isInteger(endLine) || endLine <= startLine) {
    return undefined;
  }
  const kind = foldingRangeKindFromLsp(range.kind);
  if (kind) {
    return new vscode.FoldingRange(startLine, endLine, kind);
  }
  return new vscode.FoldingRange(startLine, endLine);
}

module.exports = {
  EngFoldingRangeProvider,
  foldingRangesFromLsp
};
