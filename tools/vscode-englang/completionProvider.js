const vscode = require("vscode");
const { completionKindFromLsp } = require("./lspKinds");

class EngCompletionProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.completionSeed = Array.isArray(options.completionSeed) ? options.completionSeed : [];
    this.httpResponseFields = Array.isArray(options.httpResponseFields) ? options.httpResponseFields : [];
    this.sampleTableFields = Array.isArray(options.sampleTableFields) ? options.sampleTableFields : [];
    this.caseTableFields = Array.isArray(options.caseTableFields) ? options.caseTableFields : [];
    this.caseOutputTableFields = Array.isArray(options.caseOutputTableFields) ? options.caseOutputTableFields : [];
    this.completionSnapshotForPosition = options.completionSnapshotForPosition;
    this.cachedSnapshotForDocument = options.cachedSnapshotForDocument ?? (() => undefined);
  }

  async provideCompletionItems(document, position, cancellationToken) {
    const completionPayload =
      (await this.completionSnapshotForPosition?.(document, position, this.context, cancellationToken)) ??
      this.cachedSnapshotForDocument(document);

    const localCompletions = localMemberCompletionsForContext(document, position, {
      httpResponseFields: this.httpResponseFields,
      sampleTableFields: this.sampleTableFields,
      caseTableFields: this.caseTableFields,
      caseOutputTableFields: this.caseOutputTableFields
    });
    return completionItemsFromPayload(completionPayload, this.completionSeed, { localCompletions });
  }
}

function completionItemsFromPayload(completionPayload, completionSeed, options = {}) {
  const items = [];
  const seen = new Set();
  const localCompletions = Array.isArray(options.localCompletions) ? options.localCompletions : [];
  const completions = Array.isArray(completionPayload?.completions)
    ? [...completionPayload.completions, ...localCompletions]
    : [
        ...localCompletions,
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
  return fieldCompletionsForMemberContext(memberContext, httpResponseFields, "HTTP response field");
}

function localMemberCompletionsForContext(document, position, catalogs) {
  const memberContext = memberAccessCompletionContext(document, position);
  if (!memberContext) {
    return [];
  }
  const groups = [
    {
      fields: catalogs?.httpResponseFields,
      detail: "HTTP response field",
      matchesReceiver: isResponseLikeReceiver
    },
    {
      fields: catalogs?.sampleTableFields,
      detail: "Sample table field",
      matchesReceiver: isSampleTableLikeReceiver
    },
    {
      fields: catalogs?.caseOutputTableFields,
      detail: "Case output table field",
      matchesReceiver: isCaseOutputTableLikeReceiver
    },
    {
      fields: catalogs?.caseTableFields,
      detail: "Case table field",
      matchesReceiver: isCaseTableLikeReceiver
    }
  ];
  const items = [];
  const seen = new Set();
  for (const group of groups) {
    if (!Array.isArray(group.fields) || !group.matchesReceiver(memberContext.receiver)) {
      continue;
    }
    for (const completion of fieldCompletionsForMemberContext(memberContext, group.fields, group.detail)) {
      if (!seen.has(completion.label)) {
        seen.add(completion.label);
        items.push(completion);
      }
    }
  }
  return items;
}

function fieldCompletionsForMemberContext(memberContext, fields, fallbackDetail) {
  const prefix = memberContext.prefix.toLowerCase();
  return fields
    .filter((field) => typeof field?.label === "string")
    .filter((field) => field.label.toLowerCase().startsWith(prefix))
    .map((field) => ({
      label: field.label,
      detail: field.detail ?? fallbackDetail,
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

function isSampleTableLikeReceiver(receiver) {
  const normalized = receiver.toLowerCase();
  return (
    normalized.includes("sample") ||
    normalized.includes("design") ||
    normalized.includes("lhs")
  );
}

function isCaseOutputTableLikeReceiver(receiver) {
  const normalized = receiver.toLowerCase();
  return (
    normalized.includes("case") &&
    (
      normalized.includes("input") ||
      normalized.includes("output") ||
      normalized.includes("planned") ||
      normalized.includes("manifest")
    )
  );
}

function isCaseTableLikeReceiver(receiver) {
  const normalized = receiver.toLowerCase();
  return (
    !isCaseOutputTableLikeReceiver(receiver) &&
    (
      normalized === "case" ||
      normalized === "cases" ||
      normalized.includes("case_table") ||
      normalized.endsWith("_case") ||
      normalized.endsWith("_cases")
    )
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
  localMemberCompletionsForContext,
  memberAccessCompletionContext
};
