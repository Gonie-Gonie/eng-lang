use crate::ast::AstItem;
use crate::parser::ParsedProgram;
use crate::semantic::SemanticProgram;
use crate::{Diagnostic, SchemaInfo};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TablePredicateInfo {
    pub expression: String,
    pub column: Option<String>,
    pub operator: String,
    pub value: Option<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableTransformInfo {
    pub binding: String,
    pub operation: String,
    pub source_table: String,
    pub schema_name: Option<String>,
    pub predicates: Vec<TablePredicateInfo>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TableAnalysis {
    pub transforms: Vec<TableTransformInfo>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn analyze_table_transforms(
    parsed: &ParsedProgram,
    program: &SemanticProgram,
) -> TableAnalysis {
    let mut analysis = TableAnalysis::default();
    for item in &parsed.items {
        let AstItem::FastBinding(binding) = item else {
            continue;
        };
        if let Some(source_table) = parse_filter_expression(&binding.expression) {
            let predicates = predicates_for_owner(
                parsed,
                program,
                binding.line,
                &source_table,
                &mut analysis.diagnostics,
            );
            analysis.transforms.push(TableTransformInfo {
                binding: binding.name.clone(),
                operation: "filter".to_owned(),
                schema_name: schema_name_for_table(program, &source_table),
                source_table,
                predicates,
                status: "declared".to_owned(),
                line: binding.line,
            });
        } else if let Some(source_table) = parse_require_one_expression(&binding.expression) {
            let schema_name = schema_name_for_table(program, &source_table).or_else(|| {
                analysis
                    .transforms
                    .iter()
                    .find(|transform| transform.binding == source_table)
                    .and_then(|transform| transform.schema_name.clone())
            });
            analysis.transforms.push(TableTransformInfo {
                binding: binding.name.clone(),
                operation: "require_one".to_owned(),
                schema_name,
                source_table,
                predicates: Vec::new(),
                status: "declared".to_owned(),
                line: binding.line,
            });
        }
    }
    analysis
}

pub fn is_filter_expression(expression: &str) -> bool {
    parse_filter_expression(expression).is_some()
}

pub fn is_require_one_expression(expression: &str) -> bool {
    parse_require_one_expression(expression).is_some()
}

fn parse_filter_expression(expression: &str) -> Option<String> {
    let source = expression.trim().strip_prefix("filter ")?.trim();
    simple_identifier(source)
}

fn parse_require_one_expression(expression: &str) -> Option<String> {
    let source = expression.trim().strip_prefix("require_one ")?.trim();
    simple_identifier(source)
}

fn simple_identifier(source: &str) -> Option<String> {
    let value = source.split_whitespace().next()?.trim();
    if is_identifier(value) {
        Some(value.to_owned())
    } else {
        None
    }
}

fn predicates_for_owner(
    parsed: &ParsedProgram,
    program: &SemanticProgram,
    owner_line: usize,
    source_table: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<TablePredicateInfo> {
    parsed
        .items
        .iter()
        .filter_map(|item| match item {
            AstItem::WherePredicate(predicate) if predicate.owner_line == Some(owner_line) => {
                Some(predicate)
            }
            _ => None,
        })
        .map(|predicate| {
            let mut info = parse_predicate(&predicate.expression, predicate.line);
            for column in predicate_columns(&predicate.expression) {
                if !table_has_column(program, source_table, &column) {
                    diagnostics.push(Diagnostic::error(
                        "E-TABLE-UNKNOWN-COLUMN",
                        predicate.line,
                        &format!(
                            "Table `{source_table}` does not have column `{column}` used by filter predicate."
                        ),
                        Some("Use a column declared in the promoted table schema."),
                    ));
                    info.status = "unknown_column".to_owned();
                }
            }
            info
        })
        .collect()
}

fn parse_predicate(expression: &str, line: usize) -> TablePredicateInfo {
    if split_logical_expression(expression, "or").len() > 1 {
        return TablePredicateInfo {
            expression: expression.to_owned(),
            column: None,
            operator: "or".to_owned(),
            value: None,
            status: "accepted".to_owned(),
            line,
        };
    }
    if split_logical_expression(expression, "and").len() > 1 {
        return TablePredicateInfo {
            expression: expression.to_owned(),
            column: None,
            operator: "and".to_owned(),
            value: None,
            status: "accepted".to_owned(),
            line,
        };
    }
    for operator in ["==", "!=", "<=", ">=", "<", ">"] {
        if let Some((left, right)) = expression.split_once(operator) {
            return TablePredicateInfo {
                expression: expression.to_owned(),
                column: simple_column(left),
                operator: operator.to_owned(),
                value: Some(right.trim().to_owned()),
                status: "accepted".to_owned(),
                line,
            };
        }
    }
    let lowered = expression.to_ascii_lowercase();
    if let Some(index) = lowered.find(" is not none") {
        return TablePredicateInfo {
            expression: expression.to_owned(),
            column: simple_column(&expression[..index]),
            operator: "is_not_none".to_owned(),
            value: None,
            status: "accepted".to_owned(),
            line,
        };
    }
    if let Some(index) = lowered.find(" is none") {
        return TablePredicateInfo {
            expression: expression.to_owned(),
            column: simple_column(&expression[..index]),
            operator: "is_none".to_owned(),
            value: None,
            status: "accepted".to_owned(),
            line,
        };
    }
    TablePredicateInfo {
        expression: expression.to_owned(),
        column: None,
        operator: "expression".to_owned(),
        value: None,
        status: "metadata_only".to_owned(),
        line,
    }
}

fn predicate_columns(expression: &str) -> Vec<String> {
    let or_parts = split_logical_expression(expression, "or");
    if or_parts.len() > 1 {
        return or_parts
            .iter()
            .flat_map(|part| predicate_columns(part))
            .collect();
    }
    let and_parts = split_logical_expression(expression, "and");
    if and_parts.len() > 1 {
        return and_parts
            .iter()
            .flat_map(|part| predicate_columns(part))
            .collect();
    }
    parse_predicate(expression, 0)
        .column
        .map(|column| vec![column])
        .unwrap_or_default()
}

fn split_logical_expression<'a>(expression: &'a str, keyword: &str) -> Vec<&'a str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut previous = '\0';
    let mut start = 0usize;
    for (index, character) in expression.char_indices() {
        if character == '"' && previous != '\\' {
            in_string = !in_string;
        } else if !in_string {
            match character {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ if depth == 0 && logical_keyword_at(expression, index, keyword) => {
                    parts.push(expression[start..index].trim());
                    start = index + keyword.len();
                }
                _ => {}
            }
        }
        previous = character;
    }
    let tail = expression[start..].trim();
    if !tail.is_empty() {
        parts.push(tail);
    }
    parts
}

fn logical_keyword_at(expression: &str, index: usize, keyword: &str) -> bool {
    let Some(slice) = expression.get(index..index + keyword.len()) else {
        return false;
    };
    if !slice.eq_ignore_ascii_case(keyword) {
        return false;
    }
    let before = expression[..index].chars().next_back();
    let after = expression[index + keyword.len()..].chars().next();
    before.is_none_or(|character| !is_identifier_part(character))
        && after.is_none_or(|character| !is_identifier_part(character))
}

fn simple_column(value: &str) -> Option<String> {
    let trimmed = value.trim();
    is_identifier(trimmed).then(|| trimmed.to_owned())
}

fn table_has_column(program: &SemanticProgram, table: &str, column: &str) -> bool {
    let Some(schema_name) = schema_name_for_table(program, table) else {
        return true;
    };
    program
        .schemas
        .iter()
        .find(|schema| schema.name == schema_name)
        .is_none_or(|schema| schema_has_column(schema, column))
}

fn schema_name_for_table(program: &SemanticProgram, table: &str) -> Option<String> {
    program
        .csv_promotions
        .iter()
        .find(|promotion| promotion.binding == table)
        .map(|promotion| promotion.schema_name.clone())
}

fn schema_has_column(schema: &SchemaInfo, column: &str) -> bool {
    schema
        .columns
        .iter()
        .any(|candidate| candidate.name == column)
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_') && chars.all(is_identifier_part)
}

fn is_identifier_part(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}
