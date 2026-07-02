const fs = require("fs");
const path = require("path");

function loadEditorMetadata(extensionRoot) {
  const metadataPath = path.join(
    extensionRoot,
    "generated",
    "editor",
    "englang-editor-metadata.json"
  );
  const metadata = JSON.parse(fs.readFileSync(metadataPath, "utf8"));
  const legend = metadata.semantic_token_legend ?? {};
  const semanticTokenTypes = legend.token_types;
  const semanticTokenModifiers = legend.token_modifiers;
  const completionSeed = metadata.completion_seed;
  if (
    !Array.isArray(semanticTokenTypes) ||
    !Array.isArray(semanticTokenModifiers) ||
    !Array.isArray(completionSeed)
  ) {
    throw new Error(`Invalid EngLang editor metadata at ${metadataPath}`);
  }
  return {
    semanticTokenTypes,
    semanticTokenModifiers,
    completionSeed
  };
}

module.exports = {
  loadEditorMetadata
};
