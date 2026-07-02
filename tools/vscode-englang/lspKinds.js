const vscode = require("vscode");

function symbolKindFromLsp(kind) {
  if (typeof kind === "number" && kind >= 1 && kind <= 26) {
    return kind - 1;
  }
  switch (kind) {
    case "module":
      return vscode.SymbolKind.Module;
    case "class":
      return vscode.SymbolKind.Class;
    case "method":
      return vscode.SymbolKind.Method;
    case "property":
      return vscode.SymbolKind.Property;
    case "interface":
      return vscode.SymbolKind.Interface;
    case "function":
      return vscode.SymbolKind.Function;
    case "variable":
      return vscode.SymbolKind.Variable;
    case "constant":
      return vscode.SymbolKind.Constant;
    case "object":
      return vscode.SymbolKind.Object;
    case "key":
      return vscode.SymbolKind.Key;
    case "struct":
      return vscode.SymbolKind.Struct;
    case "operator":
      return vscode.SymbolKind.Operator;
    case "typeParameter":
      return vscode.SymbolKind.TypeParameter;
    default:
      return vscode.SymbolKind.Variable;
  }
}

function foldingRangeKindFromLsp(kind) {
  switch (kind) {
    case "comment":
      return vscode.FoldingRangeKind.Comment;
    case "imports":
      return vscode.FoldingRangeKind.Imports;
    case "region":
      return vscode.FoldingRangeKind.Region;
    default:
      return undefined;
  }
}

function completionKindFromLsp(kind) {
  if (typeof kind === "number" && kind >= 1 && kind <= 25) {
    return kind - 1;
  }
  switch (kind) {
    case "method":
      return vscode.CompletionItemKind.Method;
    case "function":
      return vscode.CompletionItemKind.Function;
    case "variable":
      return vscode.CompletionItemKind.Variable;
    case "property":
      return vscode.CompletionItemKind.Property;
    case "class":
      return vscode.CompletionItemKind.Class;
    case "stdlib":
      return vscode.CompletionItemKind.Module;
    case "unit":
      return vscode.CompletionItemKind.Unit;
    case "value":
      return vscode.CompletionItemKind.Value;
    case "keyword":
      return vscode.CompletionItemKind.Keyword;
    default:
      return vscode.CompletionItemKind.Text;
  }
}

module.exports = {
  completionKindFromLsp,
  foldingRangeKindFromLsp,
  symbolKindFromLsp
};
