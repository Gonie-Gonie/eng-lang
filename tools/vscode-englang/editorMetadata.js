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
  const completionItems = metadata.completion_items;
  const legacyCompletionItems = metadata.completion_seed;
  const syntaxCatalog = metadata.syntax_catalog ?? {};
  const keywordGroups = syntaxCatalog.keyword_groups ?? {};
  const requiredKeywordGroups = [
    "import",
    "deprecated",
    "declaration",
    "function",
    "test",
    "block",
    "modifier",
    "report",
    "validation",
    "side_effect",
    "external_boundary",
    "solver",
    "workflow"
  ];
  if (
    !Array.isArray(semanticTokenTypes) ||
    !Array.isArray(semanticTokenModifiers) ||
    !Array.isArray(completionItems) ||
    !Array.isArray(legacyCompletionItems) ||
    !Array.isArray(syntaxCatalog.keywords) ||
    !Array.isArray(syntaxCatalog.constants) ||
    !Array.isArray(syntaxCatalog.workflow_status_literals) ||
    !Array.isArray(syntaxCatalog.operator_words) ||
    !Array.isArray(syntaxCatalog.legacy_unit_aliases) ||
    typeof keywordGroups !== "object" ||
    requiredKeywordGroups.some((group) => !Array.isArray(keywordGroups[group])) ||
    !Array.isArray(syntaxCatalog.workflow_builtins) ||
    !Array.isArray(syntaxCatalog.hyphenated_workflow_builtins) ||
    !Array.isArray(syntaxCatalog.workflow_options) ||
    !Array.isArray(syntaxCatalog.public_types) ||
    !Array.isArray(syntaxCatalog.quantities) ||
    !Array.isArray(syntaxCatalog.units) ||
    !Array.isArray(syntaxCatalog.http_response_fields) ||
    !Array.isArray(syntaxCatalog.sample_table_fields) ||
    !Array.isArray(syntaxCatalog.case_table_fields) ||
    !Array.isArray(syntaxCatalog.case_output_table_fields) ||
    !Array.isArray(syntaxCatalog.case_result_collection_table_fields)
  ) {
    throw new Error(`Invalid EngLang editor metadata at ${metadataPath}`);
  }
  if (
    metadata.completion_items_count !== completionItems.length ||
    metadata.completion_seed_count !== legacyCompletionItems.length ||
    JSON.stringify(legacyCompletionItems) !== JSON.stringify(completionItems)
  ) {
    throw new Error(
      `Invalid EngLang editor metadata at ${metadataPath}: completion_seed must remain an exact legacy alias of completion_items`
    );
  }
  return {
    semanticTokenTypes,
    semanticTokenModifiers,
    completionItems,
    syntaxCatalog
  };
}

module.exports = {
  loadEditorMetadata
};
