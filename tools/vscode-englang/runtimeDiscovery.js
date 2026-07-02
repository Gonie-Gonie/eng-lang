const fs = require("fs");
const path = require("path");
const vscode = require("vscode");

function workspaceRoot(document) {
  return vscode.workspace.getWorkspaceFolder(document.uri)?.uri.fsPath ?? path.dirname(document.uri.fsPath);
}

function currentWorkspaceRoot() {
  const document = vscode.window.activeTextEditor?.document;
  if (document) {
    const folder = vscode.workspace.getWorkspaceFolder(document.uri);
    if (folder) {
      return folder.uri.fsPath;
    }
    if (isEngFileDocument(document)) {
      return path.dirname(document.uri.fsPath);
    }
  }
  return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
}

function engConfig(document) {
  const uri = document?.uri;
  return uri
    ? vscode.workspace.getConfiguration("englang", uri)
    : vscode.workspace.getConfiguration("englang");
}

function findRuntime(context, document) {
  const root = workspaceRoot(document);
  const configPath = engConfig(document).get("runtimePath", "");
  const candidates = [
    configPath,
    path.join(context.extensionPath, "bin", "eng.exe"),
    path.join(context.extensionPath, "..", "..", "eng.exe"),
    path.join(root, "eng.exe"),
    path.join(root, "target", "debug", "eng.exe"),
    path.join(root, "target", "release", "eng.exe")
  ].filter((candidate) => candidate && candidate.trim().length > 0);

  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  return "eng.exe";
}

function findLspRuntime(context, document) {
  return findLspRuntimeForRoot(context, workspaceRoot(document), document);
}

function findLspRuntimeForRoot(context, root, document) {
  const configPath = engConfig(document).get("lspPath", "");
  const rootCandidates = root
    ? [
        path.join(root, "eng-lsp.exe"),
        path.join(root, "target", "debug", "eng-lsp.exe"),
        path.join(root, "target", "release", "eng-lsp.exe")
      ]
    : [];
  const candidates = [
    configPath,
    path.join(context.extensionPath, "bin", "eng-lsp.exe"),
    path.join(context.extensionPath, "..", "..", "eng-lsp.exe"),
    ...rootCandidates
  ].filter((candidate) => candidate && candidate.trim().length > 0);

  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  return "eng-lsp.exe";
}

function isEngFileDocument(document) {
  return document.languageId === "englang" || document.fileName.endsWith(".eng");
}

module.exports = {
  currentWorkspaceRoot,
  engConfig,
  findLspRuntime,
  findLspRuntimeForRoot,
  findRuntime,
  workspaceRoot
};
