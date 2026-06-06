use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::AstItem;
use crate::parser::ParseContext;
use crate::source::SourceSpan;
use crate::Diagnostic;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaColumn {
    pub name: String,
    pub type_name: String,
    pub unit: Option<String>,
    pub is_index: bool,
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
    pub resolved_path: String,
    pub source_hash: Option<String>,
    pub headers: Vec<String>,
    pub row_count: usize,
    pub missing_columns: Vec<String>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaAnalysis {
    pub schemas: Vec<SchemaInfo>,
    pub csv_promotions: Vec<CsvPromotion>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn analyze_schema(
    program: &crate::parser::ParsedProgram,
    source_base: Option<&Path>,
) -> SchemaAnalysis {
    let mut schemas: Vec<SchemaInfo> = Vec::new();
    let mut csv_promotions = Vec::new();
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
                    schemas[schema_index].columns.push(SchemaColumn {
                        name: declaration.name.clone(),
                        type_name: clean_schema_type(&declaration.type_name),
                        unit: declaration.unit.clone(),
                        is_index: declaration
                            .type_name
                            .split_whitespace()
                            .any(|part| part == "index"),
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
            if !schema
                .columns
                .iter()
                .any(|column| column.name == policy.column)
            {
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

        let resolved_path = resolve_csv_path(source_base, &source_literal);
        let csv_read = read_csv_header(&resolved_path);
        let mut headers = Vec::new();
        let mut row_count = 0usize;
        let mut source_hash = None;

        match csv_read {
            Ok(csv) => {
                headers = csv.headers;
                row_count = csv.row_count;
                source_hash = Some(csv.source_hash);
            }
            Err(error) => diagnostics.push(Diagnostic::error(
                "E-SCHEMA-CSV-001",
                binding.line,
                &format!("Cannot read CSV source `{source_literal}`: {error}."),
                Some("Check that the path is relative to the .eng source file."),
            )),
        }

        let missing_columns = schema
            .map(|schema| {
                schema
                    .columns
                    .iter()
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
            resolved_path: resolved_path.display().to_string(),
            source_hash,
            headers,
            row_count,
            missing_columns,
            line: binding.line,
        });
    }

    SchemaAnalysis {
        schemas,
        csv_promotions,
        diagnostics,
    }
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

fn resolve_csv_path(source_base: Option<&Path>, source_literal: &str) -> PathBuf {
    let path = PathBuf::from(source_literal);
    if path.is_absolute() {
        return path;
    }

    source_base.unwrap_or_else(|| Path::new(".")).join(path)
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
