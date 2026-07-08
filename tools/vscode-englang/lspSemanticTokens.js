const vscode = require("vscode");

function createSemanticLegend(tokenTypes, tokenModifiers) {
  return new vscode.SemanticTokensLegend(tokenTypes, tokenModifiers);
}

function semanticTokensFromSnapshot(snapshot, semanticLegend, tokenTypes, tokenModifiers) {
  const builder = new vscode.SemanticTokensBuilder(semanticLegend);
  const tokens = snapshot.semantic_tokens?.tokens ?? [];
  for (const token of tokens) {
    const tokenType = tokenTypes.indexOf(token.type);
    if (tokenType < 0 || token.length <= 0) {
      continue;
    }
    builder.push(
      token.line,
      token.start,
      token.length,
      tokenType,
      semanticModifierBits(token.modifiers ?? [], tokenModifiers)
    );
  }
  return builder.build();
}

function semanticModifierBits(modifiers, tokenModifiers) {
  let bits = 0;
  for (const modifier of modifiers) {
    const index = tokenModifiers.indexOf(modifier);
    if (index >= 0) {
      bits |= 1 << index;
    }
  }
  return bits;
}

function semanticTokenRange(document, token) {
  const line = Number(token.line);
  const start = Number(token.start);
  const length = Number(token.length);
  if (
    !Number.isFinite(line) ||
    !Number.isFinite(start) ||
    !Number.isFinite(length) ||
    line < 0 ||
    line >= document.lineCount ||
    start < 0 ||
    length <= 0
  ) {
    return undefined;
  }
  const textLine = document.lineAt(line);
  if (start >= textLine.text.length) {
    return undefined;
  }
  const end = Math.min(textLine.text.length, start + length);
  return new vscode.Range(line, start, line, Math.max(start + 1, end));
}

function semanticTokenDebugSample(document, token, semanticScopeMap = {}) {
  const line = Number(token?.line ?? -1);
  const range = semanticTokenRange(document, token);
  const semantic_selectors = semanticTokenSelectors(token);
  return {
    text: range ? document.getText(range) : "",
    line: Number.isFinite(line) && line >= 0 ? line + 1 : null,
    start: token?.start ?? null,
    length: token?.length ?? null,
    type: token?.type || "-",
    modifiers: token?.modifiers ?? [],
    semantic_selectors,
    fallback_scopes: semanticTokenFallbackScopes(token, semanticScopeMap, semantic_selectors)
  };
}

function semanticTokenSelectors(token) {
  const type = token?.type || "";
  if (!type) {
    return [];
  }
  const selectors = [];
  for (const modifier of token?.modifiers ?? []) {
    if (modifier) {
      selectors.push(`${type}.${modifier}`);
    }
  }
  selectors.push(type);
  return [...new Set(selectors)];
}

function semanticTokenFallbackScopes(token, semanticScopeMap = {}, selectors = undefined) {
  const scopes = [];
  for (const selector of selectors ?? semanticTokenSelectors(token)) {
    const mappedScopes = semanticScopeMap[selector];
    const values = Array.isArray(mappedScopes)
      ? mappedScopes
      : typeof mappedScopes === "string"
        ? [mappedScopes]
        : [];
    for (const scope of values) {
      if (scope && !scopes.includes(scope)) {
        scopes.push(scope);
      }
    }
  }
  return scopes;
}

function addSemanticTokenDebugSample(samplesByKey, key, sample) {
  if (!key || !sample || !sample.text) {
    return;
  }
  const samples = samplesByKey[key] ?? [];
  if (samples.length < 8 && !samples.some((item) => item.text === sample.text && item.line === sample.line && item.start === sample.start)) {
    samples.push(sample);
  }
  samplesByKey[key] = samples;
}

module.exports = {
  addSemanticTokenDebugSample,
  createSemanticLegend,
  semanticTokenDebugSample,
  semanticTokenFallbackScopes,
  semanticTokenRange,
  semanticTokenSelectors,
  semanticTokensFromSnapshot
};
