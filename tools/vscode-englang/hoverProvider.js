const vscode = require("vscode");
const {
  definitionNameCandidates,
  identifierPathRangeAt
} = require("./lspNavigation");

const HOVER_KIND_LABELS = Object.freeze({
  variable: "Variable",
  domain: "Domain",
  domain_variable: "Domain variable",
  domain_conservation: "Domain conservation",
  component: "Component",
  component_port: "Component port",
  connection: "Connection",
  component_assembly: "Component assembly",
  connection_set: "Connection set",
  assembly_equation: "Assembly equation",
  function: "Function",
  function_local: "Function local",
  where_local: "where local",
  class: "Class",
  class_field: "Class field",
  class_validation: "Class validation",
  class_method: "Class method",
  class_object: "Class object",
  object_field: "Object field",
  object_validation: "Object validation",
  http_response_field: "HTTP response field",
  sample_table_field: "Sample table field",
  db_connection_field: "DB connection field",
  case_table_field: "Case table field",
  case_output_table_field: "Case output field",
  case_result_collection_table_field: "Case result collection field",
  model_field: "Model field",
  prediction_table_field: "Prediction table field"
});

class EngHoverProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.isEngDocument = options.isEngDocument ?? (() => true);
    this.snapshotDocumentSource = options.snapshotDocumentSource;
    this.cachedSnapshotForDocument = options.cachedSnapshotForDocument ?? (() => undefined);
    this.cacheSnapshotForDocument = options.cacheSnapshotForDocument ?? (() => undefined);
  }

  async provideHover(document, position, cancellationToken) {
    if (!this.isEngDocument(document)) {
      return undefined;
    }
    const documentVersion = document.version;
    const liveSnapshot = await this.snapshotDocumentSource?.(document, this.context, cancellationToken);
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return undefined;
    }
    const snapshot = liveSnapshot ?? this.cachedSnapshotForDocument(document);
    if (!snapshot) {
      return undefined;
    }
    this.cacheSnapshotForDocument(document, snapshot);
    return hoverFromSnapshot(document, position, snapshot);
  }
}

function hoverFromSnapshot(document, position, snapshot) {
  const wordRange = hoverRangeAtPosition(document, position);
  const candidates = hoverCandidatesAtPosition(document, position, wordRange);
  const word = candidates[0] ?? "";
  if (!word) {
    return undefined;
  }
  const hover = findHoverForWord(snapshot, candidates, position.line + 1);
  if (!hover) {
    return undefined;
  }
  return hoverFromPayload(hover, word, wordRange);
}

function findHoverForWord(source, candidates, line) {
  const names = Array.isArray(candidates) ? candidates : [candidates];
  const hovers = [
    ...(source.hovers ?? []),
    ...(source.hover_hints ?? []),
    ...(source.type_info ?? [])
  ];
  return (
    hovers.find((hover) => hoverNameMatches(hover, names, line)) ??
    hovers.find((hover) => hoverNameMatches(hover, names, undefined))
  );
}

function hoverNameMatches(hover, candidates, line) {
  if (Number.isInteger(line) && Number(hover?.line) !== line) {
    return false;
  }
  const name = String(hover?.name ?? "");
  if (!name) {
    return false;
  }
  return candidates.some((candidate) => {
    const text = String(candidate ?? "");
    return text && (name === text || name.endsWith(`.${text}`) || text.endsWith(`.${name}`));
  });
}

function hoverRangeAtPosition(document, position) {
  const line = document.lineAt(position.line).text;
  const tokenRange = identifierPathRangeAt(line, position.character);
  if (tokenRange) {
    return new vscode.Range(position.line, tokenRange.start, position.line, tokenRange.end);
  }
  return document.getWordRangeAtPosition(position, /[A-Za-z_][A-Za-z0-9_]*/);
}

function hoverCandidatesAtPosition(document, position, wordRange) {
  const candidates = new Set(definitionNameCandidates(document, position));
  if (wordRange) {
    candidates.add(document.getText(wordRange));
  }
  return Array.from(candidates).filter((candidate) => candidate.length > 0);
}

function hoverFromPayload(hover, word, wordRange) {
  const markdown = hoverMarkdown(hover, word);
  return markdown ? new vscode.Hover(markdown, wordRange) : undefined;
}

function hoverMarkdown(hover, word) {
  if (hover.contents?.value) {
    const markdown = new vscode.MarkdownString(hover.contents.value);
    markdown.isTrusted = false;
    return markdown;
  }

  const markdown = new vscode.MarkdownString();
  markdown.isTrusted = false;
  markdown.appendMarkdown(`**${hover.name ?? word}**\n\n`);
  const kindLabel = hoverKindLabel(hover.kind);
  if (kindLabel) {
    markdown.appendMarkdown(`Kind: ${kindLabel}\n\n`);
  }
  markdown.appendMarkdown(`${hover.detail ?? "EngLang symbol"}\n\n`);
  if (hover.quantity_kind) {
    markdown.appendMarkdown(`Quantity: \`${hover.quantity_kind}\`\n\n`);
  }
  const displayUnit = hoverDisplayUnit(hover.display_unit);
  if (displayUnit) {
    markdown.appendMarkdown(`Display unit: \`${displayUnit}\`\n\n`);
  }
  const canonicalUnit = hoverDisplayUnit(hover.canonical_unit);
  if (canonicalUnit) {
    markdown.appendMarkdown(`Canonical unit: \`${canonicalUnit}\`\n\n`);
  }
  if (hover.dimension) {
    markdown.appendMarkdown(`Dimension: \`${hover.dimension}\`\n\n`);
  }
  const statusLabel = hoverStatusLabel(hover.status);
  if (statusLabel) {
    markdown.appendMarkdown(`Status: ${statusLabel}`);
  }
  return markdown;
}

function hoverDisplayUnit(value) {
  const text = String(value ?? "").trim();
  return text && text !== "-" ? text : "";
}

function hoverKindLabel(kind) {
  const text = String(kind ?? "").trim().toLowerCase();
  if (!text) {
    return "";
  }
  return HOVER_KIND_LABELS[text] ?? text
    .split(/[_-]+/)
    .filter((part) => part.length > 0)
    .map((part) => hoverKindWordLabel(part))
    .join(" ");
}

function hoverStatusLabel(status) {
  const text = String(status ?? "").trim().toLowerCase();
  if (!text) {
    return "";
  }
  return text
    .split(/[_-]+/)
    .filter((part) => part.length > 0)
    .map((part, index) => hoverStatusWordLabel(part, index))
    .join(" ");
}

function hoverStatusWordLabel(part, index) {
  if (["api", "db", "http", "jit", "lsp", "sha", "ttl"].includes(part)) {
    return part.toUpperCase();
  }
  return index === 0 ? hoverKindWordLabel(part) : part;
}

function hoverKindWordLabel(part) {
  if (part === "db") {
    return "DB";
  }
  if (part === "http") {
    return "HTTP";
  }
  return part.charAt(0).toUpperCase() + part.slice(1);
}

module.exports = {
  EngHoverProvider,
  findHoverForWord,
  hoverKindLabel,
  hoverStatusLabel,
  hoverFromSnapshot,
  hoverMarkdown
};
