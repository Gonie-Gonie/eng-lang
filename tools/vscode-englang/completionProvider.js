const vscode = require("vscode");
const { completionKindFromLsp } = require("./lspKinds");

class EngCompletionProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.completionSeed = Array.isArray(options.completionSeed) ? options.completionSeed : [];
    this.completionSnapshotForPosition = options.completionSnapshotForPosition;
    this.cachedSnapshotForDocument = options.cachedSnapshotForDocument ?? (() => undefined);
  }

  async provideCompletionItems(document, position, cancellationToken) {
    const completionPayload =
      (await this.completionSnapshotForPosition?.(document, position, this.context, cancellationToken)) ??
      this.cachedSnapshotForDocument(document);

    return completionItemsFromPayload(completionPayload, this.completionSeed);
  }
}

function completionItemsFromPayload(completionPayload, completionSeed) {
  const items = [];
  const seen = new Set();
  const completions = Array.isArray(completionPayload?.completions)
    ? completionPayload.completions
    : (Array.isArray(completionSeed) ? completionSeed : []);
  for (const completion of completions) {
    if (!completion?.label) {
      continue;
    }
    const item = completionItemFromLsp(completion);
    addCompletion(items, seen, item);
  }
  return items;
}

function completionItemFromLsp(completion) {
  const item = new vscode.CompletionItem(
    completion.label,
    completionKindFromLsp(completion.lsp_kind ?? completion.kind)
  );
  item.detail = completion.detail;
  if (completion.documentation) {
    item.documentation = completion.documentation;
  }
  return item;
}

function addCompletion(items, seen, item) {
  const label = typeof item.label === "string" ? item.label : item.label?.label;
  if (!label || seen.has(label)) {
    return;
  }
  seen.add(label);
  items.push(item);
}

module.exports = {
  EngCompletionProvider,
  completionItemsFromPayload
};
