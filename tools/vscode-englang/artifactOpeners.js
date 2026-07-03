const fs = require("fs");
const path = require("path");
const vscode = require("vscode");
const {
  LAST_RUN_ARTIFACTS,
  lastRunArtifactDisplay
} = require("./artifactRegistry");

function createArtifactOpeners({ currentWorkspaceRoot, workspaceRoot }) {
  async function openLastRunArtifactPicker() {
    const root = currentWorkspaceRoot();
    if (!root) {
      vscode.window.showWarningMessage("Open an EngLang workspace folder first.");
      return;
    }
    const picked = await vscode.window.showQuickPick(
      lastRunArtifactQuickPickItems(root),
      { placeHolder: "Open a generated artifact from the latest run" }
    );
    if (picked) {
      await openLastRunArtifact(picked.artifact.id);
    }
  }

  async function openGeneratedOutputArtifactPicker() {
    const root = currentWorkspaceRoot();
    if (!root) {
      vscode.window.showWarningMessage("Open an EngLang workspace folder first.");
      return;
    }
    const manifestPath = path.join(root, "build", "result", "output_manifest.json");
    if (!fs.existsSync(manifestPath)) {
      vscode.window.showWarningMessage("No generated output list found yet. Run the current file first.");
      return;
    }

    let manifest;
    try {
      manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
    } catch (error) {
      vscode.window.showWarningMessage(`Could not read the generated output list: ${error.message}`);
      return;
    }

    const artifacts = outputManifestArtifactItems(manifest, root);
    if (artifacts.length === 0) {
      vscode.window.showWarningMessage("The latest generated output list does not point to any existing files.");
      return;
    }
    const picked = await vscode.window.showQuickPick(artifacts, {
      placeHolder: "Open a generated output from the latest run"
    });
    if (!picked) {
      return;
    }
    const uri = vscode.Uri.file(picked.filePath);
    if (picked.external) {
      await vscode.env.openExternal(uri);
      return;
    }
    const document = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(document, { preview: false });
  }

  async function openLastRunArtifact(artifactId, sourceDocument = undefined) {
    const artifact = LAST_RUN_ARTIFACTS.find((item) => item.id === artifactId);
    if (!artifact) {
      vscode.window.showWarningMessage(`Unknown EngLang artifact: ${artifactId}`);
      return;
    }
    const root = sourceDocument ? workspaceRoot(sourceDocument) : currentWorkspaceRoot();
    if (!root) {
      vscode.window.showWarningMessage("Open an EngLang workspace folder first.");
      return;
    }
    const artifactPath = path.join(root, ...artifact.relativePath);
    if (!fs.existsSync(artifactPath)) {
      vscode.window.showWarningMessage(`No ${artifact.description} found yet. Run the current file first.`);
      return;
    }
    const uri = vscode.Uri.file(artifactPath);
    if (artifact.external) {
      await vscode.env.openExternal(uri);
      return;
    }
    const document = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(document, { preview: false });
  }

  return {
    openGeneratedOutputArtifactPicker,
    openLastRunArtifact,
    openLastRunArtifactPicker
  };
}

function lastRunArtifactQuickPickItems(root) {
  return LAST_RUN_ARTIFACTS.map((artifact) => {
    const display = lastRunArtifactDisplay(artifact, root);
    return {
      label: display.label,
      description: artifact.description,
      detail: display.detail,
      artifact
    };
  });
}

function outputManifestArtifactItems(manifest, root) {
  const outputDir = outputManifestOutputDir(manifest, root);
  const artifacts = Array.isArray(manifest?.artifacts) ? manifest.artifacts : [];
  const seen = new Set();
  const items = [];
  for (const artifact of artifacts) {
    if (!artifact || typeof artifact !== "object") {
      continue;
    }
    const manifestPath = String(artifact.path ?? "").trim();
    if (!manifestPath) {
      continue;
    }
    const filePath = resolveOutputManifestPath(manifestPath, outputDir, root);
    if (seen.has(filePath) || !fs.existsSync(filePath)) {
      continue;
    }
    seen.add(filePath);
    const kind = String(artifact.kind ?? "artifact");
    const artifactClass = String(artifact.class ?? "").trim();
    const status = String(artifact.status ?? "").trim();
    items.push({
      label: outputManifestArtifactLabel(kind),
      description: relativeDisplayPath(root, filePath),
      detail: [artifactClass, status].filter(Boolean).join(" | "),
      filePath,
      external: shouldOpenArtifactExternally(filePath)
    });
  }
  return items.sort((left, right) => {
    const pathOrder = left.description.localeCompare(right.description);
    return pathOrder !== 0 ? pathOrder : left.label.localeCompare(right.label);
  });
}

function outputManifestOutputDir(manifest, root) {
  const outputDir = String(manifest?.output_dir ?? "").trim();
  if (!outputDir) {
    return path.join(root, "build", "result");
  }
  if (path.isAbsolute(outputDir)) {
    return outputDir;
  }
  return path.resolve(root, outputDir);
}

function resolveOutputManifestPath(manifestPath, outputDir, root) {
  if (path.isAbsolute(manifestPath)) {
    return manifestPath;
  }
  const normalized = manifestPath.replaceAll("\\", "/");
  if (normalized.startsWith("build/result/")) {
    return path.resolve(root, normalized);
  }
  return path.resolve(outputDir, manifestPath);
}

function outputManifestArtifactLabel(kind) {
  return String(kind)
    .split(/[_\s-]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ") || "Artifact";
}

function relativeDisplayPath(root, filePath) {
  const relative = path.relative(root, filePath);
  return relative && !relative.startsWith("..") ? relative.replace(/[\\/]/g, "/") : filePath;
}

function shouldOpenArtifactExternally(filePath) {
  const extension = path.extname(filePath).toLowerCase();
  return [".html", ".htm", ".svg", ".png", ".jpg", ".jpeg", ".gif", ".webp", ".pdf"].includes(extension);
}

module.exports = {
  createArtifactOpeners,
  lastRunArtifactDisplay,
  lastRunArtifactQuickPickItems,
  outputManifestArtifactItems,
  outputManifestArtifactLabel,
  resolveOutputManifestPath
};
