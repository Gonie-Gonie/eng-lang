const vscode = require("vscode");

class EngSignatureHelpProvider {
  constructor(options = {}) {
    this.isEngDocument = options.isEngDocument ?? (() => true);
    this.signatureHelpForPosition = options.signatureHelpForPosition;
  }

  async provideSignatureHelp(document, position, cancellationToken) {
    if (!this.isEngDocument(document)) {
      return undefined;
    }
    const documentVersion = document.version;
    const payload = await this.signatureHelpForPosition?.(
      document,
      position,
      cancellationToken
    );
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return undefined;
    }
    return signatureHelpFromLsp(payload);
  }
}

function signatureHelpFromLsp(payload) {
  const signatures = Array.isArray(payload?.signatures)
    ? payload.signatures.map(signatureInformationFromLsp).filter(Boolean)
    : [];
  if (signatures.length === 0) {
    return undefined;
  }
  const result = new vscode.SignatureHelp();
  result.signatures = signatures;
  result.activeSignature = boundedIndex(payload.activeSignature, signatures.length);
  const activeParameters = signatures[result.activeSignature]?.parameters ?? [];
  result.activeParameter = activeParameters.length === 0
    ? 0
    : boundedIndex(payload.activeParameter, activeParameters.length);
  return result;
}

function signatureInformationFromLsp(payload) {
  const label = typeof payload?.label === "string" ? payload.label : "";
  if (!label) {
    return undefined;
  }
  const signature = new vscode.SignatureInformation(
    label,
    documentationFromLsp(payload.documentation)
  );
  signature.parameters = Array.isArray(payload.parameters)
    ? payload.parameters.map(parameterInformationFromLsp).filter(Boolean)
    : [];
  return signature;
}

function parameterInformationFromLsp(payload) {
  const label = parameterLabelFromLsp(payload?.label);
  if (label === undefined) {
    return undefined;
  }
  return new vscode.ParameterInformation(label, documentationFromLsp(payload.documentation));
}

function parameterLabelFromLsp(label) {
  if (typeof label === "string" && label.length > 0) {
    return label;
  }
  if (
    Array.isArray(label)
    && label.length === 2
    && label.every((offset) => Number.isInteger(offset) && offset >= 0)
  ) {
    return [label[0], label[1]];
  }
  return undefined;
}

function documentationFromLsp(documentation) {
  const value = typeof documentation === "string"
    ? documentation
    : documentation?.value;
  if (typeof value !== "string" || value.length === 0) {
    return undefined;
  }
  const markdown = new vscode.MarkdownString(value);
  markdown.isTrusted = false;
  return markdown;
}

function boundedIndex(value, length) {
  const index = Number.isInteger(value) ? value : 0;
  return Math.max(0, Math.min(index, Math.max(0, length - 1)));
}

module.exports = {
  EngSignatureHelpProvider,
  documentationFromLsp,
  parameterLabelFromLsp,
  signatureHelpFromLsp,
  signatureInformationFromLsp
};
