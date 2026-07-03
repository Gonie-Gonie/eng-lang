const vscode = require("vscode");
const { completionKindFromLsp } = require("./lspKinds");

class EngCompletionProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.completionSeed = Array.isArray(options.completionSeed) ? options.completionSeed : [];
    this.httpResponseFields = Array.isArray(options.httpResponseFields) ? options.httpResponseFields : [];
    this.completionSnapshotForPosition = options.completionSnapshotForPosition;
    this.cachedSnapshotForDocument = options.cachedSnapshotForDocument ?? (() => undefined);
  }

  async provideCompletionItems(document, position, cancellationToken) {
    const completionPayload =
      (await this.completionSnapshotForPosition?.(document, position, this.context, cancellationToken)) ??
      this.cachedSnapshotForDocument(document);

    const localCompletions = Array.isArray(completionPayload?.completions)
      ? []
      : httpResponseFieldCompletionsForContext(document, position, this.httpResponseFields);
    return completionItemsFromPayload(completionPayload, this.completionSeed, { localCompletions });
  }
}

function completionItemsFromPayload(completionPayload, completionSeed, options = {}) {
  const items = [];
  const seen = new Set();
  const completions = Array.isArray(completionPayload?.completions)
    ? completionPayload.completions
    : [
        ...(Array.isArray(options.localCompletions) ? options.localCompletions : []),
        ...(Array.isArray(completionSeed) ? completionSeed : [])
      ];
  for (const completion of completions) {
    if (!completion?.label) {
      continue;
    }
    const item = completionItemFromLsp(completion);
    addCompletion(items, seen, item);
  }
  return items;
}

function httpResponseFieldCompletionsForContext(document, position, httpResponseFields) {
  const memberContext = memberAccessCompletionContext(document, position);
  if (!memberContext || !Array.isArray(httpResponseFields)) {
    return [];
  }
  if (!isResponseLikeReceiver(memberContext.receiver)) {
    return [];
  }
  const prefix = memberContext.prefix.toLowerCase();
  return httpResponseFields
    .filter((field) => typeof field?.label === "string")
    .filter((field) => field.label.toLowerCase().startsWith(prefix))
    .map((field) => ({
      label: field.label,
      detail: field.detail ?? "HTTP response field",
      kind: "property",
      lsp_kind: "property"
    }));
}

function memberAccessCompletionContext(document, position) {
  const linePrefix = document.lineAt(position.line).text.slice(0, position.character);
  const match = /([A-Za-z_][A-Za-z0-9_]*)\.([A-Za-z_][A-Za-z0-9_]*)?$/.exec(linePrefix);
  if (!match) {
    return undefined;
  }
  return {
    receiver: match[1],
    prefix: match[2] ?? ""
  };
}

function isResponseLikeReceiver(receiver) {
  const normalized = receiver.toLowerCase();
  return (
    normalized.includes("response") ||
    normalized.includes("http") ||
    normalized.includes("api") ||
    normalized.includes("network")
  );
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
  completionItemsFromPayload,
  httpResponseFieldCompletionsForContext,
  memberAccessCompletionContext
};
