const vscode = require("vscode");
const { completionKindFromLsp } = require("./lspKinds");

class EngCompletionProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.completionItems = Array.isArray(options.completionItems) ? options.completionItems : [];
    this.httpResponseFields = Array.isArray(options.httpResponseFields) ? options.httpResponseFields : [];
    this.sampleTableFields = Array.isArray(options.sampleTableFields) ? options.sampleTableFields : [];
    this.dbConnectionFields = Array.isArray(options.dbConnectionFields) ? options.dbConnectionFields : [];
    this.caseTableFields = Array.isArray(options.caseTableFields) ? options.caseTableFields : [];
    this.caseOutputTableFields = Array.isArray(options.caseOutputTableFields) ? options.caseOutputTableFields : [];
    this.caseResultCollectionTableFields = Array.isArray(options.caseResultCollectionTableFields)
      ? options.caseResultCollectionTableFields
      : [];
    this.completionSnapshotForPosition = options.completionSnapshotForPosition;
    this.cachedSnapshotForDocument = options.cachedSnapshotForDocument ?? (() => undefined);
  }

  async provideCompletionItems(document, position, cancellationToken) {
    const documentVersion = document.version;
    const liveCompletionPayload = await this.completionSnapshotForPosition?.(
      document,
      position,
      this.context,
      cancellationToken
    );
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return [];
    }
    const completionPayload = liveCompletionPayload ?? this.cachedSnapshotForDocument(document);

    const localCompletions = localMemberCompletionsForContext(document, position, {
      argsFields: argsFieldCompletionsFromDocument(document),
      schemaBindingFields: schemaBindingFieldCompletionsFromDocument(document),
      workflowBindingFields: workflowBindingFieldCompletionsFromDocument(document, {
        httpResponseFields: this.httpResponseFields,
        sampleTableFields: this.sampleTableFields,
        dbConnectionFields: this.dbConnectionFields,
        caseTableFields: this.caseTableFields,
        caseOutputTableFields: this.caseOutputTableFields,
        caseResultCollectionTableFields: this.caseResultCollectionTableFields
      }),
      httpResponseFields: this.httpResponseFields,
      sampleTableFields: this.sampleTableFields,
      dbConnectionFields: this.dbConnectionFields,
      caseTableFields: this.caseTableFields,
      caseOutputTableFields: this.caseOutputTableFields,
      caseResultCollectionTableFields: this.caseResultCollectionTableFields
    });
    return completionItemsFromPayload(completionPayload, this.completionItems, { localCompletions });
  }
}

function completionItemsFromPayload(completionPayload, fallbackCompletionItems, options = {}) {
  const items = [];
  const seen = new Set();
  const localCompletions = Array.isArray(options.localCompletions) ? options.localCompletions : [];
  const completions = Array.isArray(completionPayload?.completions)
    ? [...completionPayload.completions, ...localCompletions]
    : [
        ...localCompletions,
        ...(Array.isArray(fallbackCompletionItems) ? fallbackCompletionItems : [])
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

function argsFieldCompletionsFromDocument(document) {
  const text = document?.getText?.();
  if (typeof text !== "string") {
    return [];
  }
  const body = firstBlockBody(text, /\bargs\s*\{/g);
  if (!body) {
    return [];
  }
  const fields = [];
  const seen = new Set();
  for (const line of body.split(/\r?\n/)) {
    const withoutComment = line.replace(/#.*/, "").replace(/\/\/.*/, "");
    const match = /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([^=]*)/.exec(withoutComment);
    if (!match || seen.has(match[1])) {
      continue;
    }
    seen.add(match[1]);
    const typeLabel = match[2].trim();
    fields.push({
      label: match[1],
      detail: typeLabel ? `args field: ${typeLabel}` : "args field",
      kind: "property",
      lsp_kind: "property"
    });
  }
  return fields;
}

function schemaBindingFieldCompletionsFromDocument(document) {
  const text = document?.getText?.();
  if (typeof text !== "string") {
    return {};
  }
  const schemas = schemaFieldsFromDocument(text);
  const bindings = promotedSchemaBindingsFromDocument(text);
  const result = {};
  for (const [binding, schemaName] of Object.entries(bindings)) {
    const fields = schemas[schemaName];
    if (!Array.isArray(fields)) {
      continue;
    }
    result[binding] = fields.map((field) => ({
      ...field,
      detail: field.detail ? `${schemaName} field: ${field.detail}` : `${schemaName} field`
    }));
  }
  return result;
}

function workflowBindingFieldCompletionsFromDocument(document, catalogs) {
  const text = document?.getText?.();
  if (typeof text !== "string") {
    return {};
  }
  return workflowBindingFieldCompletionsFromSource(text, catalogs);
}

function workflowBindingFieldCompletionsFromSource(source, catalogs) {
  const text = String(source ?? "");
  const result = {};
  const groups = [
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*http\s+(?:get|post|put|patch|head|request|fetch)\b/gm,
      fields: catalogs?.httpResponseFields,
      detail: "HTTP response field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*sample\s+(?:lhs|latin[_-]hypercube|grid|random|uniform)\b/gm,
      fields: catalogs?.sampleTableFields,
      detail: "Sample table field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*open\s+sqlite\b/gm,
      fields: catalogs?.dbConnectionFields,
      detail: "SQLite connection field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*materialize\s+cases\b/gm,
      fields: catalogs?.caseTableFields,
      detail: "Case table field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*apply\s+[A-Za-z_][A-Za-z0-9_.-]*\s+over\s+[A-Za-z_][A-Za-z0-9_.-]*\b/gm,
      fields: catalogs?.caseOutputTableFields,
      detail: "Case output table field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*apply\s*\(\s*[A-Za-z_][A-Za-z0-9_.-]*\s*,\s*over\s*=\s*[A-Za-z_][A-Za-z0-9_.-]*\s*\)/gm,
      fields: catalogs?.caseOutputTableFields,
      detail: "Case output table field"
    },
    {
      pattern: /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*collect\s+results\s+[A-Za-z_][A-Za-z0-9_.]*\b/gm,
      fields: catalogs?.caseResultCollectionTableFields,
      detail: "Case result collection field"
    }
  ];
  for (const group of groups) {
    if (!Array.isArray(group.fields) || !group.fields.length) {
      continue;
    }
    group.pattern.lastIndex = 0;
    let match;
    while ((match = group.pattern.exec(text)) !== null) {
      result[match[1]] = group.fields.map((field) => ({
        ...field,
        detail: field.detail ? `${group.detail}: ${field.detail}` : group.detail,
        kind: "property",
        lsp_kind: "property"
      }));
    }
  }
  return result;
}

function schemaFieldsFromDocument(text) {
  const schemas = {};
  const schemaPattern = /\bschema\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{/g;
  let match;
  while ((match = schemaPattern.exec(text)) !== null) {
    const openIndex = text.indexOf("{", match.index);
    const closeIndex = blockCloseIndex(text, openIndex);
    if (openIndex < 0 || closeIndex < 0) {
      continue;
    }
    const schemaName = match[1];
    schemas[schemaName] = schemaFieldCompletionsFromBody(text.slice(openIndex + 1, closeIndex));
    schemaPattern.lastIndex = closeIndex + 1;
  }
  return schemas;
}

function promotedSchemaBindingsFromDocument(text) {
  const bindings = {};
  const promotePattern = /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*promote\s+(?:csv|toml|json(?:\s+records)?)\b[^\n]*\bas\s+([A-Za-z_][A-Za-z0-9_]*)\b/gm;
  let match;
  while ((match = promotePattern.exec(text)) !== null) {
    bindings[match[1]] = match[2];
  }
  return bindings;
}

function schemaFieldCompletionsFromBody(body) {
  const fields = [];
  const seen = new Set();
  for (const line of body.split(/\r?\n/)) {
    const withoutComment = line.replace(/#.*/, "").replace(/\/\/.*/, "");
    const match = /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([^=]*)/.exec(withoutComment);
    if (!match || seen.has(match[1])) {
      continue;
    }
    seen.add(match[1]);
    const typeLabel = match[2].trim();
    fields.push({
      label: match[1],
      detail: typeLabel,
      kind: "property",
      lsp_kind: "property"
    });
  }
  return fields;
}

function blockCloseIndex(text, openIndex) {
  if (openIndex < 0) {
    return -1;
  }
  let depth = 0;
  let inString = false;
  let escaped = false;
  for (let index = openIndex; index < text.length; index += 1) {
    const char = text[index];
    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (char === "\\") {
        escaped = true;
      } else if (char === "\"") {
        inString = false;
      }
      continue;
    }
    if (char === "\"") {
      inString = true;
      continue;
    }
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return index;
      }
    }
  }
  return -1;
}

function firstBlockBody(text, openerRegex) {
  openerRegex.lastIndex = 0;
  const match = openerRegex.exec(text);
  if (!match) {
    return "";
  }
  const openIndex = text.indexOf("{", match.index);
  const closeIndex = blockCloseIndex(text, openIndex);
  if (openIndex < 0 || closeIndex < 0) {
    return "";
  }
  return text.slice(openIndex + 1, closeIndex);
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
      fields: catalogs?.argsFields,
      detail: "args field",
      matchesReceiver: isArgsReceiver
    },
    {
      fields: fieldsForSchemaBinding(catalogs?.schemaBindingFields, memberContext.receiver),
      detail: "schema field",
      matchesReceiver: () => true
    },
    {
      fields: fieldsForWorkflowBinding(catalogs?.workflowBindingFields, memberContext.receiver),
      detail: "workflow field",
      matchesReceiver: () => true
    },
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
      fields: catalogs?.dbConnectionFields,
      detail: "SQLite connection field",
      matchesReceiver: isDbConnectionLikeReceiver
    },
    {
      fields: catalogs?.caseOutputTableFields,
      detail: "Case output table field",
      matchesReceiver: isCaseOutputTableLikeReceiver
    },
    {
      fields: catalogs?.caseResultCollectionTableFields,
      detail: "Case result collection field",
      matchesReceiver: isCaseResultCollectionLikeReceiver
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

function fieldsForSchemaBinding(schemaBindingFields, receiver) {
  if (!schemaBindingFields || typeof schemaBindingFields !== "object") {
    return [];
  }
  return firstMappedFieldsForReceiver(schemaBindingFields, receiver);
}

function fieldsForWorkflowBinding(workflowBindingFields, receiver) {
  if (!workflowBindingFields || typeof workflowBindingFields !== "object") {
    return [];
  }
  return firstMappedFieldsForReceiver(workflowBindingFields, receiver);
}

function firstMappedFieldsForReceiver(fieldMap, receiver) {
  for (const candidate of receiverLookupCandidates(receiver)) {
    const fields = fieldMap[candidate];
    if (Array.isArray(fields)) {
      return fields;
    }
  }
  return [];
}

function receiverLookupCandidates(receiver) {
  if (typeof receiver !== "string" || receiver.length === 0) {
    return [];
  }
  const lastSegment = receiver.split(".").filter(Boolean).pop();
  return lastSegment && lastSegment !== receiver ? [receiver, lastSegment] : [receiver];
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
  const match = /((?:[A-Za-z_][A-Za-z0-9_]*\.)*[A-Za-z_][A-Za-z0-9_]*)\.([A-Za-z_][A-Za-z0-9_]*)?$/.exec(linePrefix);
  if (!match) {
    return undefined;
  }
  return {
    receiver: match[1],
    prefix: match[2] ?? ""
  };
}

function isArgsReceiver(receiver) {
  return receiver === "args";
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

function isDbConnectionLikeReceiver(receiver) {
  const normalized = receiver.toLowerCase();
  return (
    normalized === "db" ||
    normalized.includes("sqlite") ||
    normalized.includes("database") ||
    normalized.endsWith("_db")
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
      normalized.includes("rendered") ||
      normalized.includes("blocked") ||
      normalized.includes("manifest")
    )
  );
}

function isCaseResultCollectionLikeReceiver(receiver) {
  const normalized = receiver.toLowerCase();
  return (
    normalized.includes("collection") ||
    (normalized.includes("case") && normalized.includes("result"))
  );
}

function isCaseTableLikeReceiver(receiver) {
  const normalized = receiver.toLowerCase();
  return (
    !isCaseOutputTableLikeReceiver(receiver) &&
    !isCaseResultCollectionLikeReceiver(receiver) &&
    (
      normalized === "case" ||
      normalized === "cases" ||
      normalized.includes("case_table") ||
      normalized.endsWith("_case") ||
      normalized.endsWith("_cases")
    )
  );
}

function completionInsertTextFromCompletion(completion) {
  if (typeof completion?.insert_snippet === "string" && completion.insert_snippet.length > 0) {
    return new vscode.SnippetString(completion.insert_snippet);
  }
  if (typeof completion?.insert === "string" && completion.insert.length > 0) {
    return completion.insert;
  }
  return undefined;
}

function completionItemFromLsp(completion) {
  const item = new vscode.CompletionItem(
    completion.label,
    completionKindFromLsp(completion.lsp_kind ?? completion.kind)
  );
  item.detail = completion.detail;
  const insertText = completionInsertTextFromCompletion(completion);
  if (insertText) {
    item.insertText = insertText;
  }
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
  completionInsertTextFromCompletion,
  completionItemsFromPayload,
  argsFieldCompletionsFromDocument,
  schemaBindingFieldCompletionsFromDocument,
  workflowBindingFieldCompletionsFromDocument,
  workflowBindingFieldCompletionsFromSource,
  httpResponseFieldCompletionsForContext,
  localMemberCompletionsForContext,
  memberAccessCompletionContext,
  receiverLookupCandidates
};
