const vscode = require("vscode");

class EngFormattingProvider {
  constructor(context, options = {}) {
    this.context = context;
    this.isEngDocument = options.isEngDocument ?? (() => true);
    this.formatDocumentSource = options.formatDocumentSource;
  }

  async provideDocumentFormattingEdits(document, _options, cancellationToken) {
    if (!this.isEngDocument(document)) {
      return [];
    }
    const documentVersion = document.version;
    const payload = await this.formatDocumentSource?.(
      document,
      this.context,
      cancellationToken
    );
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return [];
    }
    if (!payload?.changed || typeof payload.formatted !== "string") {
      return [];
    }
    return [vscode.TextEdit.replace(fullDocumentRange(document), payload.formatted)];
  }

  async provideDocumentRangeFormattingEdits(document, range, _options, cancellationToken) {
    if (!this.isEngDocument(document)) {
      return [];
    }
    const documentVersion = document.version;
    const payload = await this.formatDocumentSource?.(
      document,
      this.context,
      cancellationToken
    );
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return [];
    }
    if (!payload?.changed || typeof payload.formatted !== "string") {
      return [];
    }
    const edit = rangeFormattingEdit(document, range, payload.formatted);
    return edit ? [edit] : [];
  }

  async provideOnTypeFormattingEdits(document, position, ch, _options, cancellationToken) {
    if (
      ch !== "}"
      || cancellationToken?.isCancellationRequested
      || !this.isEngDocument(document)
      || !isStructuralClosingBrace(document, position)
    ) {
      return [];
    }
    const documentVersion = document.version;
    const payload = await this.formatDocumentSource?.(
      document,
      this.context,
      cancellationToken
    );
    if (document.version !== documentVersion || cancellationToken?.isCancellationRequested) {
      return [];
    }
    if (!payload?.changed || typeof payload.formatted !== "string") {
      return [];
    }
    const line = document.lineAt(position.line);
    const lineRange = new vscode.Range(position.line, 0, position.line, line.text.length);
    const edit = rangeFormattingEdit(document, lineRange, payload.formatted);
    return edit ? [edit] : [];
  }
}

function documentLineExists(document, line) {
  return Number.isInteger(line) && line >= 0 && line < document.lineCount;
}

function isStructuralClosingBrace(document, position) {
  if (!documentLineExists(document, position?.line)) {
    return false;
  }
  const line = document.lineAt(position.line).text;
  const closingIndex = position.character - 1;
  if (closingIndex < 0 || line[closingIndex] !== "}") {
    return false;
  }

  let inString = false;
  for (let index = 0; index < closingIndex; index += 1) {
    const character = line[index];
    if (inString) {
      if (character === "\\") {
        index += 1;
      } else if (character === '"') {
        inString = false;
      }
      continue;
    }
    if (character === '"') {
      inString = true;
    } else if (character === "#" || (character === "/" && line[index + 1] === "/")) {
      return false;
    }
  }
  return !inString;
}

function fullDocumentRange(document) {
  if (document.lineCount === 0) {
    return new vscode.Range(0, 0, 0, 0);
  }
  const lastLine = document.lineAt(document.lineCount - 1);
  return new vscode.Range(0, 0, lastLine.lineNumber, lastLine.text.length);
}

function rangeFormattingEdit(document, range, formatted) {
  const formattedLines = splitLogicalLines(formatted);
  if (formattedLines.length !== document.lineCount) {
    return undefined;
  }
  const selected = selectedLineRange(document, range);
  if (!selected) {
    return undefined;
  }
  const newline = documentNewline(document);
  const newText = formattedLines.slice(selected.startLine, selected.endLine + 1).join(newline);
  const oldText = documentLines(document, selected.startLine, selected.endLine).join(newline);
  if (newText === oldText) {
    return undefined;
  }
  const replaceRange = new vscode.Range(
    selected.startLine,
    0,
    selected.endLine,
    document.lineAt(selected.endLine).text.length
  );
  return vscode.TextEdit.replace(replaceRange, newText);
}

function selectedLineRange(document, range) {
  if (document.lineCount === 0 || range.start.line >= document.lineCount || range.end.line >= document.lineCount) {
    return undefined;
  }
  const endLine = range.end.character === 0 && range.end.line > range.start.line
    ? range.end.line - 1
    : range.end.line;
  if (range.start.line > endLine || endLine >= document.lineCount) {
    return undefined;
  }
  return { startLine: range.start.line, endLine };
}

function documentLines(document, startLine, endLine) {
  const lines = [];
  for (let line = startLine; line <= endLine; line += 1) {
    lines.push(document.lineAt(line).text);
  }
  return lines;
}

function splitLogicalLines(text) {
  return String(text ?? "").split(/\r?\n/);
}

function documentNewline(document) {
  return document.eol === vscode.EndOfLine.CRLF ? "\r\n" : "\n";
}

module.exports = {
  EngFormattingProvider
};
