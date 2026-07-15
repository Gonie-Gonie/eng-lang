const vscode = require("vscode");

function vscodeRangeFromLsp(range) {
  const startLine = range?.start?.line;
  const startCharacter = range?.start?.character;
  const endLine = range?.end?.line;
  const endCharacter = range?.end?.character;
  if (
    !Number.isInteger(startLine) ||
    !Number.isInteger(startCharacter) ||
    !Number.isInteger(endLine) ||
    !Number.isInteger(endCharacter) ||
    startLine < 0 ||
    startCharacter < 0 ||
    endLine < 0 ||
    endCharacter < 0 ||
    startLine > endLine ||
    (startLine === endLine && startCharacter > endCharacter)
  ) {
    return undefined;
  }
  return new vscode.Range(startLine, startCharacter, endLine, endCharacter);
}

module.exports = {
  vscodeRangeFromLsp
};
