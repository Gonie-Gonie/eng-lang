const vscode = require("vscode");
const { symbolKindFromLsp } = require("./lspKinds");
const { vscodeRangeFromLsp } = require("./lspRanges");

function definitionLocationFromLsp(payload, fallbackUri, reportError) {
  if (!payload || Array.isArray(payload)) {
    return undefined;
  }
  const range = vscodeRangeFromLsp(payload.range);
  if (!range) {
    return undefined;
  }
  try {
    const uri = payload.uri ? vscode.Uri.parse(payload.uri) : fallbackUri;
    return new vscode.Location(uri, range);
  } catch (error) {
    reportLspNavigationError(reportError, `Unable to parse EngLang definition URI: ${error.message}`);
    return undefined;
  }
}

function workspaceSymbolInformationFromLsp(symbol, reportError) {
  if (!symbol?.name) {
    return undefined;
  }
  const range = vscodeRangeFromLsp(symbol.location?.range);
  const uriText = symbol.location?.uri;
  if (!range || !uriText) {
    return undefined;
  }
  try {
    return new vscode.SymbolInformation(
      symbol.name,
      symbolKindFromLsp(symbol.kind),
      symbol.containerName ?? "",
      new vscode.Location(vscode.Uri.parse(uriText), range)
    );
  } catch (error) {
    reportLspNavigationError(reportError, `Unable to parse EngLang workspace symbol URI: ${error.message}`);
    return undefined;
  }
}

function documentSymbolsFromSnapshot(snapshot) {
  return (snapshot.document_symbols ?? [])
    .map(documentSymbolFromSnapshot)
    .filter((symbol) => symbol !== undefined);
}

function documentSymbolFromSnapshot(symbol) {
  if (!symbol?.name) {
    return undefined;
  }
  const range = vscodeRangeFromLsp(symbol.range);
  const selectionRange = vscodeRangeFromLsp(symbol.selectionRange) ?? range;
  if (!range || !selectionRange) {
    return undefined;
  }
  const documentSymbol = new vscode.DocumentSymbol(
    symbol.name,
    symbol.detail ?? "",
    symbolKindFromLsp(symbol.kind),
    range,
    selectionRange
  );
  for (const child of symbol.children ?? []) {
    const childSymbol = documentSymbolFromSnapshot(child);
    if (childSymbol) {
      documentSymbol.children.push(childSymbol);
    }
  }
  return documentSymbol;
}

function flattenSnapshotSymbols(symbols) {
  const flattened = [];
  for (const symbol of symbols ?? []) {
    if (symbol?.name) {
      flattened.push(symbol);
    }
    flattened.push(...flattenSnapshotSymbols(symbol?.children ?? []));
  }
  return flattened;
}

function definitionLocationFromSnapshotSymbols(symbols, candidates, uri) {
  for (const symbol of flattenSnapshotSymbols(symbols)) {
    if (!candidates.includes(symbol.name)) {
      continue;
    }
    const range = vscodeRangeFromLsp(symbol.selectionRange) ?? vscodeRangeFromLsp(symbol.range);
    if (range) {
      return new vscode.Location(uri, range);
    }
  }
  return undefined;
}

function definitionNameCandidates(document, position) {
  const line = document.lineAt(position.line).text;
  const tokenRange = identifierPathRangeAt(line, position.character);
  if (!tokenRange) {
    return [];
  }
  const token = line.slice(tokenRange.start, tokenRange.end);
  const parts = token.split(".").filter((part) => part.length > 0);
  const currentPart = partAtCharacter(token, tokenRange.start, position.character);
  return Array.from(
    new Set([
      token,
      currentPart,
      parts[parts.length - 1],
      parts[0]
    ].filter((part) => part && /^[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*$/.test(part)))
  );
}

function identifierPathRangeAt(line, character) {
  const isPathChar = (value) => /[A-Za-z0-9_.]/.test(value);
  let start = Math.min(character, line.length);
  while (start > 0 && isPathChar(line[start - 1])) {
    start -= 1;
  }
  let end = Math.min(character, line.length);
  while (end < line.length && isPathChar(line[end])) {
    end += 1;
  }
  const text = line.slice(start, end).replace(/^\.+|\.+$/g, "");
  if (!/^[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*$/.test(text)) {
    return undefined;
  }
  return { start, end };
}

function partAtCharacter(token, tokenStart, character) {
  const relative = Math.max(0, Math.min(token.length, character - tokenStart));
  let offset = 0;
  for (const part of token.split(".")) {
    const start = offset;
    const end = offset + part.length;
    if (relative >= start && relative <= end) {
      return part;
    }
    offset = end + 1;
  }
  return undefined;
}

function reportLspNavigationError(reportError, message) {
  if (typeof reportError === "function") {
    reportError(message);
  }
}

module.exports = {
  definitionLocationFromLsp,
  definitionLocationFromSnapshotSymbols,
  definitionNameCandidates,
  documentSymbolsFromSnapshot,
  flattenSnapshotSymbols,
  identifierPathRangeAt,
  workspaceSymbolInformationFromLsp
};
