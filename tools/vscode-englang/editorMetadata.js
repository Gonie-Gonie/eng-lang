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
  const syntaxCatalog = metadata.syntax_catalog ?? {};
  if (
    !Array.isArray(semanticTokenTypes) ||
    !Array.isArray(semanticTokenModifiers) ||
    !Array.isArray(completionSeed) ||
    !Array.isArray(syntaxCatalog.keywords) ||
    !Array.isArray(syntaxCatalog.workflow_builtins) ||
    !Array.isArray(syntaxCatalog.workflow_options) ||
    !Array.isArray(syntaxCatalog.http_response_fields) ||
    !Array.isArray(syntaxCatalog.sample_table_fields) ||
    !Array.isArray(syntaxCatalog.case_table_fields) ||
    !Array.isArray(syntaxCatalog.case_output_table_fields)
  ) {
    throw new Error(`Invalid EngLang editor metadata at ${metadataPath}`);
  }
  return {
    semanticTokenTypes,
    semanticTokenModifiers,
    completionSeed,
    syntaxCatalog
  };
}

module.exports = {
  loadEditorMetadata
};
