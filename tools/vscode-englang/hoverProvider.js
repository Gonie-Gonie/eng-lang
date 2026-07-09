const vscode = require("vscode");
const {
  definitionNameCandidates,
  identifierPathRangeAt
} = require("./lspNavigation");

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
  if (hover.kind) {
    markdown.appendMarkdown(`Kind: \`${hover.kind}\`\n\n`);
  }
  markdown.appendMarkdown(`${hover.detail ?? "EngLang symbol"}\n\n`);
  if (hover.quantity_kind) {
    markdown.appendMarkdown(`Quantity: \`${hover.quantity_kind}\`\n\n`);
  }
  if (hover.display_unit) {
    markdown.appendMarkdown(`Display unit: \`${hover.display_unit}\`\n\n`);
  }
  if (hover.canonical_unit) {
    markdown.appendMarkdown(`Canonical unit: \`${hover.canonical_unit}\`\n\n`);
  }
  if (hover.dimension) {
    markdown.appendMarkdown(`Dimension: \`${hover.dimension}\`\n\n`);
  }
  if (hover.status) {
    markdown.appendMarkdown(`Status: \`${hover.status}\``);
  }
  return markdown;
}

module.exports = {
  EngHoverProvider,
  findHoverForWord,
  hoverFromSnapshot,
  hoverMarkdown
};
