const fs = require("fs");
const path = require("path");
const vscode = require("vscode");

const WORKSPACE_INDEX_IGNORED_DIRECTORIES = new Set([
  ".dev",
  ".git",
  ".vscode",
  "build",
  "dist",
  "node_modules",
  "target",
  "__pycache__"
]);

function workspaceRoot(document) {
  return vscode.workspace.getWorkspaceFolder(document.uri)?.uri.fsPath ?? path.dirname(document.uri.fsPath);
}

function workspaceRootKey(root) {
  if (!root) {
    return "";
  }
  const resolved = path.resolve(root);
  return process.platform === "win32" ? resolved.toLowerCase() : resolved;
}

function isWorkspaceEngSourceUri(uri) {
  if (!uri?.fsPath || uri.scheme !== "file" || path.extname(uri.fsPath).toLowerCase() !== ".eng") {
    return false;
  }
  const folderRoot = vscode.workspace.getWorkspaceFolder(uri)?.uri.fsPath;
  if (!folderRoot) {
    return false;
  }
  const relative = path.relative(path.resolve(folderRoot), path.resolve(uri.fsPath));
  if (
    relative === ""
    || path.isAbsolute(relative)
    || relative === ".."
    || relative.startsWith(`..${path.sep}`)
  ) {
    return false;
  }
  const directories = relative.split(path.sep).slice(0, -1);
  return !directories.some((directory) => {
    const key = process.platform === "win32" ? directory.toLowerCase() : directory;
    return WORKSPACE_INDEX_IGNORED_DIRECTORIES.has(key);
  });
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
  isWorkspaceEngSourceUri,
  workspaceRoot,
  workspaceRootKey
};
