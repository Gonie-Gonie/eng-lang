use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::AstItem;
use crate::parser::ParseContext;
use crate::semantic::ArgValueInfo;
use crate::source::SourceSpan;
use crate::Diagnostic;
use serde_json::Value as JsonValue;
use toml::Value as TomlValue;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaColumn {
    pub name: String,
    pub type_name: String,
    pub unit: Option<String>,
    pub is_index: bool,
    pub optional: bool,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaConstraint {
    pub text: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissingPolicy {
    pub column: String,
    pub policy: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaInfo {
    pub name: String,
    pub columns: Vec<SchemaColumn>,
    pub constraints: Vec<SchemaConstraint>,
    pub missing_policies: Vec<MissingPolicy>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CsvPromotion {
    pub binding: String,
    pub schema_name: String,
    pub source_literal: String,
    pub source_value: String,
    pub resolved_path: String,
    pub source_hash: Option<String>,
    pub headers: Vec<String>,
    pub row_count: usize,
    pub missing_columns: Vec<String>,
    pub optional_missing_columns: Vec<String>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigTypeMismatch {
    pub field: String,
    pub expected: String,
    pub actual: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigPromotion {
    pub binding: String,
    pub format: String,
    pub schema_name: String,
    pub source_literal: String,
    pub source_value: String,
    pub resolved_path: String,
    pub source_hash: Option<String>,
    pub field_count: usize,
    pub missing_fields: Vec<String>,
    pub unknown_fields: Vec<String>,
    pub null_fields: Vec<String>,
    pub optional_fields: Vec<String>,
    pub optional_missing_fields: Vec<String>,
    pub optional_null_fields: Vec<String>,
    pub type_mismatches: Vec<ConfigTypeMismatch>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaAnalysis {
    pub schemas: Vec<SchemaInfo>,
    pub csv_promotions: Vec<CsvPromotion>,
    pub config_promotions: Vec<ConfigPromotion>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn analyze_schema(
    program: &crate::parser::ParsedProgram,
    source_base: Option<&Path>,
    arg_values: &[ArgValueInfo],
) -> SchemaAnalysis {
    let mut schemas: Vec<SchemaInfo> = Vec::new();
    let mut csv_promotions = Vec::new();
    let mut config_promotions = Vec::new();
    let mut diagnostics = Vec::new();
    let mut current_schema_index: Option<usize> = None;

    for item in &program.items {
        match item {
            AstItem::Schema(schema) => {
                schemas.push(SchemaInfo {
                    name: schema.name.clone(),
                    columns: Vec::new(),
                    constraints: Vec::new(),
                    missing_policies: Vec::new(),
                    line: schema.span.line,
                });
                current_schema_index = Some(schemas.len() - 1);
            }
            AstItem::Script(_) => current_schema_index = None,
            AstItem::ExplicitDecl(declaration) if declaration.context == ParseContext::Schema => {
                if let Some(schema_index) = current_schema_index {
                    let (type_name, optional) = schema_column_type(&declaration.type_name);
                    schemas[schema_index].columns.push(SchemaColumn {
                        name: declaration.name.clone(),
                        type_name,
                        unit: declaration.unit.clone(),
                        is_index: declaration
                            .type_name
                            .split_whitespace()
                            .any(|part| part == "index"),
                        optional,
                        line: declaration.line,
                        span: declaration.span,
                    });
                }
            }
            AstItem::Constraint(constraint) => {
                if let Some(schema_index) = current_schema_index {
                    schemas[schema_index].constraints.push(SchemaConstraint {
                        text: constraint.text.clone(),
                        line: constraint.line,
                    });
                }
            }
            AstItem::MissingPolicy(policy) => {
                if let Some(schema_index) = current_schema_index {
                    schemas[schema_index].missing_policies.push(MissingPolicy {
                        column: policy.column.clone(),
                        policy: policy.policy.clone(),
                        line: policy.line,
                    });
                }
            }
            _ => {}
        }
    }

    for schema in &schemas {
        for policy in &schema.missing_policies {
            let column_exists = schema
                .columns
                .iter()
                .any(|column| column.name == policy.column);
            if !column_exists {
                diagnostics.push(Diagnostic::error(
                    "E-SCHEMA-MISSING-001",
                    policy.line,
                    &format!(
                        "Missing policy references unknown schema column `{}`.",
                        policy.column
                    ),
                    Some("Add the column to the schema or remove the missing policy."),
                ));
            }
        }
    }

    for item in &program.items {
        let AstItem::FastBinding(binding) = item else {
            continue;
        };
        let Some((source_literal, schema_name)) = parse_promote_csv(&binding.expression) else {
            continue;
        };
        let schema = schemas
            .iter()
            .find(|candidate| candidate.name == schema_name);
        if schema.is_none() {
            diagnostics.push(Diagnostic::error(
                "E-SCHEMA-PROMOTE-001",
                binding.line,
                &format!("CSV promotion references unknown schema `{schema_name}`."),
                Some("Define the schema before the `promote csv` expression."),
            ));
        }
        let mut headers = Vec::new();
        let mut row_count = 0usize;
        let mut source_hash = None;

        let source_value = match resolve_source_value(&source_literal, arg_values) {
            Ok(value) => value,
            Err(arg_name) => {
                diagnostics.push(Diagnostic::error(
                    "E-ARGS-CSV-001",
                    binding.line,
                    &format!(
                        "CSV promotion path references `args.{arg_name}`, but no value is available."
                    ),
                    Some("Provide the field with `--<name> <value>` or add a default in `args { ... }`."),
                ));
                csv_promotions.push(CsvPromotion {
                    binding: binding.name.clone(),
                    schema_name,
                    source_literal,
                    source_value: String::new(),
                    resolved_path: String::new(),
                    source_hash: None,
                    headers,
                    row_count,
                    missing_columns: Vec::new(),
                    optional_missing_columns: Vec::new(),
                    line: binding.line,
                });
                continue;
            }
        };
        let resolved_path = resolve_csv_path(source_base, &source_value);
        let csv_read = read_csv_header(&resolved_path);

        match csv_read {
            Ok(csv) => {
                headers = csv.headers;
                row_count = csv.row_count;
                source_hash = Some(csv.source_hash);
            }
            Err(error) => diagnostics.push(Diagnostic::error(
                "E-SCHEMA-CSV-001",
                binding.line,
                &format!("Cannot read CSV source `{source_value}`: {error}."),
                Some("Check that the path is relative to the .eng source file."),
            )),
        }

        let missing_columns = schema
            .map(|schema| {
                schema
                    .columns
                    .iter()
                    .filter(|column| !column.optional)
                    .filter(|column| !headers.iter().any(|header| header == &column.name))
                    .map(|column| column.name.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let optional_missing_columns = schema
            .map(|schema| {
                schema
                    .columns
                    .iter()
                    .filter(|column| column.optional)
                    .filter(|column| !headers.iter().any(|header| header == &column.name))
                    .map(|column| column.name.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !missing_columns.is_empty() {
            diagnostics.push(Diagnostic::error(
                "E-SCHEMA-CSV-002",
                binding.line,
                &format!(
                    "CSV source `{source_literal}` is missing required column(s): {}.",
                    missing_columns.join(", ")
                ),
                Some("Add the missing CSV headers or update the schema."),
            ));
        }

        csv_promotions.push(CsvPromotion {
            binding: binding.name.clone(),
            schema_name,
            source_literal,
            source_value,
            resolved_path: resolved_path.display().to_string(),
            source_hash,
            headers,
            row_count,
            missing_columns,
            optional_missing_columns,
            line: binding.line,
        });
    }

    for item in &program.items {
        let AstItem::FastBinding(binding) = item else {
            continue;
        };
        let Some((format, source_literal, schema_name)) = parse_promote_config(&binding.expression)
        else {
            continue;
        };
        let schema = schemas
            .iter()
            .find(|candidate| candidate.name == schema_name);
        if schema.is_none() {
            diagnostics.push(Diagnostic::error(
                "E-SCHEMA-PROMOTE-001",
                binding.line,
                &format!("Config promotion references unknown schema `{schema_name}`."),
                Some("Define the schema before the `promote json/toml` expression."),
            ));
        }

        let source_value = match resolve_source_value(&source_literal, arg_values) {
            Ok(value) => value,
            Err(arg_name) => {
                diagnostics.push(Diagnostic::error(
                    "E-ARGS-CONFIG-001",
                    binding.line,
                    &format!(
                        "Config promotion path references `args.{arg_name}`, but no value is available."
                    ),
                    Some("Provide the field with `--<name> <value>` or add a default in `args { ... }`."),
                ));
                config_promotions.push(ConfigPromotion {
                    binding: binding.name.clone(),
                    format,
                    schema_name,
                    source_literal,
                    source_value: String::new(),
                    resolved_path: String::new(),
                    source_hash: None,
                    field_count: 0,
                    missing_fields: Vec::new(),
                    unknown_fields: Vec::new(),
                    null_fields: Vec::new(),
                    optional_fields: Vec::new(),
                    optional_missing_fields: Vec::new(),
                    optional_null_fields: Vec::new(),
                    type_mismatches: Vec::new(),
                    status: "missing_arg".to_owned(),
                    line: binding.line,
                });
                continue;
            }
        };
        let resolved_path = resolve_source_path(source_base, &source_value);
        let mut source_hash = None;
        let mut fields = Vec::new();
        let mut status = "validated".to_owned();

        match read_config_fields(&format, &resolved_path) {
            Ok(config) => {
                source_hash = Some(config.source_hash);
                fields = config.fields;
            }
            Err(error) => {
                status = "source_error".to_owned();
                diagnostics.push(Diagnostic::error(
                    "E-CONFIG-SOURCE-001",
                    binding.line,
                    &format!("Cannot read config source `{source_value}`: {error}."),
                    Some("Check that the path is relative to the .eng source file and is valid UTF-8."),
                ));
            }
        }

        let field_names = fields
            .iter()
            .map(|field| field.name.clone())
            .collect::<Vec<_>>();
        let missing_fields = schema
            .map(|schema| {
                schema
                    .columns
                    .iter()
                    .filter(|column| !column.optional)
                    .filter(|column| !field_names.iter().any(|field| field == &column.name))
                    .map(|column| column.name.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let optional_fields = schema
            .map(|schema| {
                schema
                    .columns
                    .iter()
                    .filter(|column| column.optional)
                    .map(|column| column.name.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let optional_missing_fields = schema
            .map(|schema| {
                schema
                    .columns
                    .iter()
                    .filter(|column| column.optional)
                    .filter(|column| !field_names.iter().any(|field| field == &column.name))
                    .map(|column| column.name.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let unknown_fields = schema
            .map(|schema| {
                field_names
                    .iter()
                    .filter(|field| {
                        !schema
                            .columns
                            .iter()
                            .any(|column| column.name.as_str() == field.as_str())
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let null_fields = schema
            .map(|schema| {
                fields
                    .iter()
                    .filter(|field| {
                        field.kind == ConfigValueKind::Null
                            && schema
                                .columns
                                .iter()
                                .any(|column| column.name == field.name && !column.optional)
                    })
                    .map(|field| field.name.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let optional_null_fields = schema
            .map(|schema| {
                fields
                    .iter()
                    .filter(|field| {
                        field.kind == ConfigValueKind::Null
                            && schema
                                .columns
                                .iter()
                                .any(|column| column.name == field.name && column.optional)
                    })
                    .map(|field| field.name.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let type_mismatches = schema
            .map(|schema| config_type_mismatches(schema, &fields))
            .unwrap_or_default();

        if !missing_fields.is_empty() {
            status = "invalid".to_owned();
            diagnostics.push(Diagnostic::error(
                "E-CONFIG-MISSING-FIELD",
                binding.line,
                &format!(
                    "Config source `{source_literal}` is missing required field(s): {}.",
                    missing_fields.join(", ")
                ),
                Some("Add the missing config fields or update the schema."),
            ));
        }
        if !unknown_fields.is_empty() {
            status = "invalid".to_owned();
            diagnostics.push(Diagnostic::error(
                "E-CONFIG-UNKNOWN-FIELD",
                binding.line,
                &format!(
                    "Config source `{source_literal}` has unknown field(s): {}.",
                    unknown_fields.join(", ")
                ),
                Some("Remove unknown config fields or declare them in the schema."),
            ));
        }
        if !null_fields.is_empty() {
            status = "invalid".to_owned();
            diagnostics.push(Diagnostic::error(
                "E-CONFIG-NULL-NOT-OPTIONAL",
                binding.line,
                &format!(
                    "Config source `{source_literal}` sets non-optional field(s) to null: {}.",
                    null_fields.join(", ")
                ),
                Some("Use a concrete value, or add optional-field support before using null."),
            ));
        }
        for mismatch in &type_mismatches {
            status = "invalid".to_owned();
            diagnostics.push(Diagnostic::error(
                "E-CONFIG-TYPE-MISMATCH",
                binding.line,
                &format!(
                    "Config field `{}` expected `{}` but found `{}`.",
                    mismatch.field, mismatch.expected, mismatch.actual
                ),
                Some("Update the config value type or the schema field type."),
            ));
        }

        config_promotions.push(ConfigPromotion {
            binding: binding.name.clone(),
            format,
            schema_name,
            source_literal,
            source_value,
            resolved_path: resolved_path.display().to_string(),
            source_hash,
            field_count: field_names.len(),
            missing_fields,
            unknown_fields,
            null_fields,
            optional_fields,
            optional_missing_fields,
            optional_null_fields,
            type_mismatches,
            status,
            line: binding.line,
        });
    }

    SchemaAnalysis {
        schemas,
        csv_promotions,
        config_promotions,
        diagnostics,
    }
}

fn schema_column_type(type_name: &str) -> (String, bool) {
    let cleaned = clean_schema_type(type_name);
    optional_schema_type(&cleaned).unwrap_or((cleaned, false))
}

fn optional_schema_type(type_name: &str) -> Option<(String, bool)> {
    let trimmed = type_name.trim();
    if let Some(inner) = trimmed
        .strip_prefix("Optional[")
        .and_then(|value| value.strip_suffix(']'))
    {
        return Some((inner.trim().to_owned(), true));
    }
    if let Some(inner) = trimmed.strip_suffix('?') {
        return Some((inner.trim().to_owned(), true));
    }
    None
}

fn clean_schema_type(type_name: &str) -> String {
    type_name
        .split_whitespace()
        .filter(|part| *part != "index")
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_promote_csv(expression: &str) -> Option<(String, String)> {
    let trimmed = expression.trim();
    if !trimmed.starts_with("promote csv ") {
        return None;
    }

    let after_prefix = trimmed.trim_start_matches("promote csv ").trim();
    let source_literal = if let Some(rest) = after_prefix.strip_prefix('"') {
        let (path, _) = rest.split_once('"')?;
        path.to_owned()
    } else {
        after_prefix.split_whitespace().next()?.to_owned()
    };

    let schema_name = trimmed.rsplit_once(" as ")?.1.trim();
    let schema_name = schema_name
        .split_whitespace()
        .next()
        .unwrap_or(schema_name)
        .trim_matches('{')
        .to_owned();

    Some((source_literal, schema_name))
}

fn parse_promote_config(expression: &str) -> Option<(String, String, String)> {
    let trimmed = expression.trim();
    let (format, after_prefix) = if let Some(rest) = trimmed.strip_prefix("promote json ") {
        ("json", rest.trim())
    } else if let Some(rest) = trimmed.strip_prefix("promote toml ") {
        ("toml", rest.trim())
    } else {
        return None;
    };
    let (source_literal, schema_name) = after_prefix.rsplit_once(" as ")?;
    let schema_name = schema_name
        .split_whitespace()
        .next()
        .unwrap_or(schema_name)
        .trim_matches('{')
        .to_owned();
    Some((
        format.to_owned(),
        source_literal.trim().to_owned(),
        schema_name,
    ))
}

fn resolve_source_value(
    source_literal: &str,
    arg_values: &[ArgValueInfo],
) -> Result<String, String> {
    if let Some(arg_name) = source_literal.strip_prefix("args.") {
        return arg_values
            .iter()
            .find(|arg| arg.name == arg_name)
            .map(|arg| arg.value.clone())
            .ok_or_else(|| arg_name.to_owned());
    }
    if let Some(value) = strip_call_string_arg(source_literal, "file") {
        return Ok(value);
    }
    if let Some(value) = strip_call_string_arg(source_literal, "dir") {
        return Ok(value);
    }
    Ok(strip_string_literal(source_literal))
}

fn strip_call_string_arg(expression: &str, function_name: &str) -> Option<String> {
    let trimmed = expression.trim();
    let prefix = format!("{function_name}(");
    let inner = trimmed.strip_prefix(&prefix)?.strip_suffix(')')?.trim();
    Some(strip_string_literal(inner))
}

fn strip_string_literal(value: &str) -> String {
    let trimmed = value.trim();
    if let Some(inner) = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        inner.to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn resolve_csv_path(source_base: Option<&Path>, source_literal: &str) -> PathBuf {
    resolve_source_path(source_base, source_literal)
}

fn resolve_source_path(source_base: Option<&Path>, source_literal: &str) -> PathBuf {
    let path = PathBuf::from(source_literal);
    if path.is_absolute() {
        return path;
    }

    source_base.unwrap_or_else(|| Path::new(".")).join(path)
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ConfigValueKind {
    Null,
    Bool,
    Integer,
    Float,
    String,
    Array,
    Object,
    DateTime,
}

impl ConfigValueKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool => "bool",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::String => "string",
            Self::Array => "array",
            Self::Object => "object",
            Self::DateTime => "datetime",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ConfigFieldValue {
    name: String,
    kind: ConfigValueKind,
}

struct ConfigRead {
    fields: Vec<ConfigFieldValue>,
    source_hash: String,
}

fn read_config_fields(format: &str, path: &Path) -> std::io::Result<ConfigRead> {
    let text = fs::read_to_string(path)?;
    let fields = match format {
        "json" => json_config_fields(&text).map_err(invalid_config_data)?,
        "toml" => toml_config_fields(&text).map_err(invalid_config_data)?,
        _ => Vec::new(),
    };
    Ok(ConfigRead {
        fields,
        source_hash: hash_text(&text),
    })
}

fn invalid_config_data(message: String) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}

fn json_config_fields(source: &str) -> Result<Vec<ConfigFieldValue>, String> {
    let value = serde_json::from_str::<JsonValue>(source).map_err(|error| error.to_string())?;
    let Some(object) = value.as_object() else {
        return Err("root value must be an object".to_owned());
    };
    Ok(object
        .iter()
        .map(|(name, value)| ConfigFieldValue {
            name: name.clone(),
            kind: json_value_kind(value),
        })
        .collect())
}

fn toml_config_fields(source: &str) -> Result<Vec<ConfigFieldValue>, String> {
    let value = source
        .parse::<TomlValue>()
        .map_err(|error| error.to_string())?;
    let Some(table) = value.as_table() else {
        return Err("root value must be a table".to_owned());
    };
    Ok(table
        .iter()
        .map(|(name, value)| ConfigFieldValue {
            name: name.clone(),
            kind: toml_value_kind(value),
        })
        .collect())
}

fn json_value_kind(value: &JsonValue) -> ConfigValueKind {
    match value {
        JsonValue::Null => ConfigValueKind::Null,
        JsonValue::Bool(_) => ConfigValueKind::Bool,
        JsonValue::Number(number) if number.is_i64() || number.is_u64() => ConfigValueKind::Integer,
        JsonValue::Number(_) => ConfigValueKind::Float,
        JsonValue::String(_) => ConfigValueKind::String,
        JsonValue::Array(_) => ConfigValueKind::Array,
        JsonValue::Object(_) => ConfigValueKind::Object,
    }
}

fn toml_value_kind(value: &TomlValue) -> ConfigValueKind {
    match value {
        TomlValue::String(_) => ConfigValueKind::String,
        TomlValue::Integer(_) => ConfigValueKind::Integer,
        TomlValue::Float(_) => ConfigValueKind::Float,
        TomlValue::Boolean(_) => ConfigValueKind::Bool,
        TomlValue::Datetime(_) => ConfigValueKind::DateTime,
        TomlValue::Array(_) => ConfigValueKind::Array,
        TomlValue::Table(_) => ConfigValueKind::Object,
    }
}

fn config_type_mismatches(
    schema: &SchemaInfo,
    fields: &[ConfigFieldValue],
) -> Vec<ConfigTypeMismatch> {
    let mut mismatches = Vec::new();
    for column in &schema.columns {
        let Some(field) = fields.iter().find(|field| field.name == column.name) else {
            continue;
        };
        if field.kind == ConfigValueKind::Null {
            continue;
        }
        if config_value_matches_schema_type(&column.type_name, &field.kind) {
            continue;
        }
        mismatches.push(ConfigTypeMismatch {
            field: column.name.clone(),
            expected: column.type_name.clone(),
            actual: field.kind.as_str().to_owned(),
        });
    }
    mismatches
}

fn config_value_matches_schema_type(type_name: &str, kind: &ConfigValueKind) -> bool {
    match type_name.to_ascii_lowercase().as_str() {
        "string" | "path" | "filepath" | "csvfile" | "jsonfile" | "tomlfile" | "textfile"
        | "reportfile" | "plotfile" | "directorypath" => *kind == ConfigValueKind::String,
        "bool" | "boolean" => *kind == ConfigValueKind::Bool,
        "int" | "integer" | "count" => *kind == ConfigValueKind::Integer,
        "float" | "number" | "dimensionlessnumber" | "ratio" => {
            matches!(kind, ConfigValueKind::Integer | ConfigValueKind::Float)
        }
        "datetime" => matches!(kind, ConfigValueKind::String | ConfigValueKind::DateTime),
        _ => true,
    }
}

struct CsvRead {
    headers: Vec<String>,
    row_count: usize,
    source_hash: String,
}

fn read_csv_header(path: &Path) -> std::io::Result<CsvRead> {
    let text = fs::read_to_string(path)?;
    let mut lines = text.lines();
    let headers = lines
        .next()
        .unwrap_or("")
        .split(',')
        .map(|header| header.trim().to_owned())
        .filter(|header| !header.is_empty())
        .collect::<Vec<_>>();
    let row_count = lines.filter(|line| !line.trim().is_empty()).count();

    Ok(CsvRead {
        headers,
        row_count,
        source_hash: hash_text(&text),
    })
}

fn hash_text(source: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in source.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
