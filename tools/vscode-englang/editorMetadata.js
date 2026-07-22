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
  const syntaxCatalog = metadata.syntax_catalog ?? {};
  const keywordGroups = syntaxCatalog.keyword_groups ?? {};
  const builtinFunctionSignatures = syntaxCatalog.builtin_function_signatures;
  const uncertaintyArgumentAliases = syntaxCatalog.uncertainty_argument_aliases;
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
    !Array.isArray(syntaxCatalog.keywords) ||
    !Array.isArray(syntaxCatalog.constants) ||
    !Array.isArray(syntaxCatalog.workflow_status_literals) ||
    !Array.isArray(syntaxCatalog.operator_words) ||
    !Array.isArray(syntaxCatalog.legacy_unit_aliases) ||
    typeof keywordGroups !== "object" ||
    requiredKeywordGroups.some((group) => !Array.isArray(keywordGroups[group])) ||
    !Array.isArray(syntaxCatalog.workflow_builtins) ||
    typeof syntaxCatalog.percentile_statistic_pattern !== "string" ||
    !syntaxCatalog.percentile_statistic_pattern.trim() ||
    !Array.isArray(syntaxCatalog.hyphenated_workflow_builtins) ||
    !Array.isArray(syntaxCatalog.legacy_workflow_builtin_aliases) ||
    !Array.isArray(syntaxCatalog.legacy_workflow_option_aliases) ||
    !Array.isArray(builtinFunctionSignatures) ||
    builtinFunctionSignatures.some((signature) => (
      !signature ||
      typeof signature.owner !== "string" ||
      !signature.owner.trim() ||
      typeof signature.status !== "string" ||
      !signature.status.trim() ||
      typeof signature.status_label !== "string" ||
      !signature.status_label.trim() ||
      typeof signature.documentation !== "string" ||
      !signature.documentation.trim() ||
      typeof signature.name !== "string" ||
      !signature.name.trim() ||
      typeof signature.label !== "string" ||
      !signature.label.trim() ||
      !Array.isArray(signature.parameters) ||
      signature.parameters.some((parameter) => (
        !parameter ||
        typeof parameter.name !== "string" ||
        !parameter.name.trim() ||
        typeof parameter.label !== "string" ||
        !parameter.label.trim() ||
        typeof parameter.type !== "string" ||
        !parameter.type.trim() ||
        typeof parameter.optional !== "boolean"
      )) ||
      typeof signature.return_type !== "string" ||
      !signature.return_type.trim() ||
      !(
        signature.return_display_unit === null ||
        (
          typeof signature.return_display_unit === "string" &&
          signature.return_display_unit.trim()
        )
      )
    )) ||
    !Array.isArray(uncertaintyArgumentAliases) ||
    uncertaintyArgumentAliases.some((item) => (
      !item ||
      typeof item.alias !== "string" ||
      !item.alias.trim() ||
      typeof item.canonical !== "string" ||
      !item.canonical.trim() ||
      !Array.isArray(item.calls) ||
      item.calls.length === 0 ||
      item.calls.some((call) => typeof call !== "string" || !call.trim())
    )) ||
    !Array.isArray(syntaxCatalog.workflow_options) ||
    !Array.isArray(syntaxCatalog.public_types) ||
    !Array.isArray(syntaxCatalog.quantities) ||
    !Array.isArray(syntaxCatalog.units) ||
    !Array.isArray(syntaxCatalog.http_response_fields) ||
    !Array.isArray(syntaxCatalog.coverage_result_fields) ||
    !Array.isArray(syntaxCatalog.time_alignment_result_fields) ||
    !Array.isArray(syntaxCatalog.table_fields) ||
    !Array.isArray(syntaxCatalog.sample_table_fields) ||
    !Array.isArray(syntaxCatalog.db_connection_fields) ||
    !Array.isArray(syntaxCatalog.case_table_fields) ||
    !Array.isArray(syntaxCatalog.case_output_table_fields) ||
    !Array.isArray(syntaxCatalog.case_run_result_table_fields) ||
    !Array.isArray(syntaxCatalog.case_result_collection_table_fields) ||
    !Array.isArray(syntaxCatalog.model_fields) ||
    !Array.isArray(syntaxCatalog.prediction_table_fields)
  ) {
    throw new Error(`Invalid EngLang editor metadata at ${metadataPath}`);
  }
  if (metadata.completion_items_count !== completionItems.length) {
    throw new Error(
      `Invalid EngLang editor metadata at ${metadataPath}: completion_items_count must match completion_items`
    );
  }
  if (syntaxCatalog.builtin_function_signatures_count !== builtinFunctionSignatures.length) {
    throw new Error(
      "Invalid EngLang editor metadata at " +
        metadataPath +
        ": builtin_function_signatures_count must match builtin_function_signatures"
    );
  }
  return {
    semanticTokenTypes,
    semanticTokenModifiers,
    completionItems,
    builtinFunctionSignatures,
    syntaxCatalog
  };
}

module.exports = {
  loadEditorMetadata
};
