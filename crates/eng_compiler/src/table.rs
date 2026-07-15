use crate::ast::AstItem;
use crate::parser::ParsedProgram;
use crate::semantic::SemanticProgram;
use crate::{Diagnostic, SchemaColumn};

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
pub struct TableJoinKeyInfo {
    pub expression: String,
    pub left_table: String,
    pub left_column: String,
    pub right_table: String,
    pub right_column: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableColumnInfo {
    pub name: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableSortKeyInfo {
    pub column: String,
    pub direction: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableDerivedColumnInfo {
    pub name: String,
    pub expression: String,
    pub source_columns: Vec<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableTransformInfo {
    pub binding: String,
    pub operation: String,
    pub source_table: String,
    pub secondary_table: Option<String>,
    pub schema_name: Option<String>,
    pub selected_columns: Vec<TableColumnInfo>,
    pub sort_keys: Vec<TableSortKeyInfo>,
    pub derived_columns: Vec<TableDerivedColumnInfo>,
    pub predicates: Vec<TablePredicateInfo>,
    pub join_keys: Vec<TableJoinKeyInfo>,
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
                &analysis.transforms,
                binding.line,
                &source_table,
                &mut analysis.diagnostics,
            );
            analysis.transforms.push(TableTransformInfo {
                binding: binding.name.clone(),
                operation: "filter".to_owned(),
                schema_name: schema_name_for_table(program, &source_table),
                source_table,
                secondary_table: None,
                selected_columns: Vec::new(),
                sort_keys: Vec::new(),
                derived_columns: Vec::new(),
                predicates,
                join_keys: Vec::new(),
                status: "declared".to_owned(),
                line: binding.line,
            });
        } else if let Some((source_table, columns)) = parse_select_expression(&binding.expression) {
            let selected_columns = selected_columns_for_transform(
                program,
                &analysis.transforms,
                &source_table,
                columns,
                binding.line,
                &mut analysis.diagnostics,
            );
            let schema_name = schema_name_for_source(program, &analysis.transforms, &source_table);
            analysis.transforms.push(TableTransformInfo {
                binding: binding.name.clone(),
                operation: "select".to_owned(),
                schema_name,
                source_table,
                secondary_table: None,
                selected_columns,
                sort_keys: Vec::new(),
                derived_columns: Vec::new(),
                predicates: Vec::new(),
                join_keys: Vec::new(),
                status: "declared".to_owned(),
                line: binding.line,
            });
        } else if let Some((source_table, keys)) = parse_sort_expression(&binding.expression) {
            let sort_keys = sort_keys_for_transform(
                program,
                &analysis.transforms,
                &source_table,
                keys,
                binding.line,
                &mut analysis.diagnostics,
            );
            let schema_name = schema_name_for_source(program, &analysis.transforms, &source_table);
            analysis.transforms.push(TableTransformInfo {
                binding: binding.name.clone(),
                operation: "sort".to_owned(),
                schema_name,
                source_table,
                secondary_table: None,
                selected_columns: Vec::new(),
                sort_keys,
                derived_columns: Vec::new(),
                predicates: Vec::new(),
                join_keys: Vec::new(),
                status: "declared".to_owned(),
                line: binding.line,
            });
        } else if let Some((source_table, name, expression)) =
            parse_derive_expression(&binding.expression)
        {
            let derived_columns = derived_columns_for_transform(
                program,
                &analysis.transforms,
                &source_table,
                name,
                expression,
                binding.line,
                &mut analysis.diagnostics,
            );
            let schema_name = schema_name_for_source(program, &analysis.transforms, &source_table);
            analysis.transforms.push(TableTransformInfo {
                binding: binding.name.clone(),
                operation: "derive".to_owned(),
                schema_name,
                source_table,
                secondary_table: None,
                selected_columns: Vec::new(),
                sort_keys: Vec::new(),
                derived_columns,
                predicates: Vec::new(),
                join_keys: Vec::new(),
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
                secondary_table: None,
                selected_columns: Vec::new(),
                sort_keys: Vec::new(),
                derived_columns: Vec::new(),
                predicates: Vec::new(),
                join_keys: Vec::new(),
                status: "declared".to_owned(),
                line: binding.line,
            });
        } else if let Some((left_table, right_table)) = parse_join_expression(&binding.expression) {
            let join_keys = join_keys_for_owner(
                parsed,
                program,
                &analysis.transforms,
                binding.line,
                &left_table,
                &right_table,
                &mut analysis.diagnostics,
            );
            let schema_name = schema_name_for_source(program, &analysis.transforms, &left_table)
                .zip(schema_name_for_source(
                    program,
                    &analysis.transforms,
                    &right_table,
                ))
                .map(|(left, right)| format!("{left}+{right}"));
            analysis.transforms.push(TableTransformInfo {
                binding: binding.name.clone(),
                operation: "join".to_owned(),
                source_table: left_table,
                secondary_table: Some(right_table),
                schema_name,
                selected_columns: Vec::new(),
                sort_keys: Vec::new(),
                derived_columns: Vec::new(),
                predicates: Vec::new(),
                join_keys,
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

pub fn is_select_expression(expression: &str) -> bool {
    parse_select_expression(expression).is_some()
}

pub fn is_sort_expression(expression: &str) -> bool {
    parse_sort_expression(expression).is_some()
}

pub fn is_derive_expression(expression: &str) -> bool {
    parse_derive_expression(expression).is_some()
}

pub fn is_join_expression(expression: &str) -> bool {
    parse_join_expression(expression).is_some()
}

fn parse_filter_expression(expression: &str) -> Option<String> {
    let source = expression.trim().strip_prefix("filter ")?.trim();
    simple_identifier(source)
}

fn parse_require_one_expression(expression: &str) -> Option<String> {
    let source = expression.trim().strip_prefix("require_one ")?.trim();
    simple_identifier(source)
}

fn parse_select_expression(expression: &str) -> Option<(String, Vec<String>)> {
    let source = expression.trim().strip_prefix("select ")?.trim();
    let (source_table, columns) = source
        .split_once(" columns ")
        .or_else(|| source.split_once(" column "))?;
    Some((simple_identifier(source_table)?, parse_column_list(columns)))
}

fn parse_sort_expression(expression: &str) -> Option<(String, Vec<(String, String)>)> {
    let source = expression.trim().strip_prefix("sort ")?.trim();
    let (source_table, keys) = source.split_once(" by ")?;
    Some((simple_identifier(source_table)?, parse_sort_key_list(keys)))
}

fn parse_derive_expression(expression: &str) -> Option<(String, String, String)> {
    let source = expression.trim().strip_prefix("derive ")?.trim();
    let (source_table, rest) = source
        .split_once(" column ")
        .or_else(|| source.split_once(" columns "))?;
    let (name, expression) = rest.split_once('=')?;
    let name = simple_identifier(name.trim())?;
    let expression = expression.trim();
    if expression.is_empty() {
        return None;
    }
    Some((
        simple_identifier(source_table)?,
        name,
        expression.to_owned(),
    ))
}

fn parse_join_expression(expression: &str) -> Option<(String, String)> {
    let source = expression.trim().strip_prefix("join ")?.trim();
    let (left, right) = source.split_once(" with ")?;
    Some((simple_identifier(left)?, simple_identifier(right)?))
}

fn parse_column_list(columns: &str) -> Vec<String> {
    let trimmed = columns
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_start_matches('{')
        .trim_end_matches('}');
    trimmed
        .split(',')
        .map(str::trim)
        .filter(|column| is_identifier(column))
        .map(str::to_owned)
        .collect()
}

fn parse_sort_key_list(keys: &str) -> Vec<(String, String)> {
    keys.split(',')
        .filter_map(|key| {
            let mut parts = key.split_whitespace();
            let column = parts.next()?;
            if !is_identifier(column) {
                return None;
            }
            let direction = parts
                .next()
                .filter(|value| value.eq_ignore_ascii_case("desc"))
                .map(|_| "desc")
                .unwrap_or("asc");
            Some((column.to_owned(), direction.to_owned()))
        })
        .collect()
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
    transforms: &[TableTransformInfo],
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
                if !source_has_column(program, transforms, source_table, &column) {
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
            validate_predicate_type(
                program,
                transforms,
                source_table,
                &predicate.expression,
                predicate.line,
                &mut info.status,
                diagnostics,
            );
            info
        })
        .collect()
}

fn selected_columns_for_transform(
    program: &SemanticProgram,
    transforms: &[TableTransformInfo],
    source_table: &str,
    columns: Vec<String>,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<TableColumnInfo> {
    columns
        .into_iter()
        .map(|column| {
            let mut status = "accepted".to_owned();
            if !source_has_column(program, transforms, source_table, &column) {
                diagnostics.push(Diagnostic::error(
                    "E-TABLE-UNKNOWN-COLUMN",
                    line,
                    &format!("Table `{source_table}` does not have selected column `{column}`."),
                    Some("Use a column declared in the promoted table schema."),
                ));
                status = "unknown_column".to_owned();
            }
            TableColumnInfo {
                name: column,
                status,
                line,
            }
        })
        .collect()
}

fn sort_keys_for_transform(
    program: &SemanticProgram,
    transforms: &[TableTransformInfo],
    source_table: &str,
    keys: Vec<(String, String)>,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<TableSortKeyInfo> {
    keys.into_iter()
        .map(|(column, direction)| {
            let mut status = "accepted".to_owned();
            if !source_has_column(program, transforms, source_table, &column) {
                diagnostics.push(Diagnostic::error(
                    "E-TABLE-UNKNOWN-COLUMN",
                    line,
                    &format!("Table `{source_table}` does not have sort key column `{column}`."),
                    Some("Use a column declared in the promoted table schema."),
                ));
                status = "unknown_column".to_owned();
            }
            TableSortKeyInfo {
                column,
                direction,
                status,
                line,
            }
        })
        .collect()
}

fn derived_columns_for_transform(
    program: &SemanticProgram,
    transforms: &[TableTransformInfo],
    source_table: &str,
    name: String,
    expression: String,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<TableDerivedColumnInfo> {
    let source_columns = expression_columns(&expression);
    let mut status = "accepted".to_owned();
    for column in &source_columns {
        if !source_has_column(program, transforms, source_table, column) {
            diagnostics.push(Diagnostic::error(
                "E-TABLE-UNKNOWN-COLUMN",
                line,
                &format!(
                    "Table `{source_table}` does not have source column `{column}` used by derived column `{name}`."
                ),
                Some("Use columns declared in the promoted table schema or prior table transform."),
            ));
            status = "unknown_column".to_owned();
        }
    }
    vec![TableDerivedColumnInfo {
        name,
        expression,
        source_columns,
        status,
        line,
    }]
}

fn join_keys_for_owner(
    parsed: &ParsedProgram,
    program: &SemanticProgram,
    transforms: &[TableTransformInfo],
    owner_line: usize,
    left_table: &str,
    right_table: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<TableJoinKeyInfo> {
    let keys = parsed
        .items
        .iter()
        .filter_map(|item| match item {
            AstItem::OnPredicate(predicate) if predicate.owner_line == Some(owner_line) => {
                Some(predicate)
            }
            _ => None,
        })
        .map(|predicate| {
            parse_join_key(
                &predicate.expression,
                predicate.line,
                left_table,
                right_table,
                diagnostics,
            )
        })
        .collect::<Vec<_>>();

    if keys.is_empty() {
        diagnostics.push(Diagnostic::error(
            "E-TABLE-JOIN-KEY-MISMATCH",
            owner_line,
            &format!("Join `{left_table}` with `{right_table}` requires at least one `on` key."),
            Some("Attach an `on { left.column == right.column }` block to the join."),
        ));
    }

    for key in &keys {
        validate_join_key_columns(
            program,
            transforms,
            left_table,
            right_table,
            key,
            diagnostics,
        );
    }

    keys
}

fn parse_join_key(
    expression: &str,
    line: usize,
    left_table: &str,
    right_table: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> TableJoinKeyInfo {
    let invalid = || TableJoinKeyInfo {
        expression: expression.to_owned(),
        left_table: left_table.to_owned(),
        left_column: String::new(),
        right_table: right_table.to_owned(),
        right_column: String::new(),
        status: "invalid_key".to_owned(),
        line,
    };

    let Some((left, right)) = expression.split_once("==") else {
        diagnostics.push(join_key_mismatch_diagnostic(
            line,
            expression,
            left_table,
            right_table,
        ));
        return invalid();
    };
    let Some((first_table, first_column)) = parse_qualified_column(left.trim()) else {
        diagnostics.push(join_key_mismatch_diagnostic(
            line,
            expression,
            left_table,
            right_table,
        ));
        return invalid();
    };
    let Some((second_table, second_column)) = parse_qualified_column(right.trim()) else {
        diagnostics.push(join_key_mismatch_diagnostic(
            line,
            expression,
            left_table,
            right_table,
        ));
        return invalid();
    };

    if first_table == left_table && second_table == right_table {
        TableJoinKeyInfo {
            expression: expression.to_owned(),
            left_table: first_table,
            left_column: first_column,
            right_table: second_table,
            right_column: second_column,
            status: "accepted".to_owned(),
            line,
        }
    } else if first_table == right_table && second_table == left_table {
        TableJoinKeyInfo {
            expression: expression.to_owned(),
            left_table: second_table,
            left_column: second_column,
            right_table: first_table,
            right_column: first_column,
            status: "accepted".to_owned(),
            line,
        }
    } else {
        diagnostics.push(join_key_mismatch_diagnostic(
            line,
            expression,
            left_table,
            right_table,
        ));
        invalid()
    }
}

fn parse_qualified_column(value: &str) -> Option<(String, String)> {
    let (table, column) = value.split_once('.')?;
    let table = table.trim();
    let column = column.trim();
    if is_identifier(table) && is_identifier(column) {
        Some((table.to_owned(), column.to_owned()))
    } else {
        None
    }
}

fn validate_join_key_columns(
    program: &SemanticProgram,
    transforms: &[TableTransformInfo],
    left_table: &str,
    right_table: &str,
    key: &TableJoinKeyInfo,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if key.status != "accepted" {
        return;
    }
    let left_column = schema_column_for_source(program, transforms, left_table, &key.left_column);
    if left_column.is_none()
        && !source_has_column(program, transforms, left_table, &key.left_column)
        && schema_name_for_source(program, transforms, left_table).is_some()
    {
        diagnostics.push(Diagnostic::error(
            "E-TABLE-UNKNOWN-COLUMN",
            key.line,
            &format!(
                "Table `{left_table}` does not have join key column `{}`.",
                key.left_column
            ),
            Some("Use a column declared in the promoted table schema."),
        ));
    }
    let right_column =
        schema_column_for_source(program, transforms, right_table, &key.right_column);
    if right_column.is_none()
        && !source_has_column(program, transforms, right_table, &key.right_column)
        && schema_name_for_source(program, transforms, right_table).is_some()
    {
        diagnostics.push(Diagnostic::error(
            "E-TABLE-UNKNOWN-COLUMN",
            key.line,
            &format!(
                "Table `{right_table}` does not have join key column `{}`.",
                key.right_column
            ),
            Some("Use a column declared in the promoted table schema."),
        ));
    }
    if let (Some(left_column), Some(right_column)) = (left_column, right_column) {
        if left_column.type_name != right_column.type_name || left_column.unit != right_column.unit
        {
            diagnostics.push(Diagnostic::error(
                "E-TABLE-SCHEMA-MISMATCH",
                key.line,
                &format!(
                    "Join key `{left_table}.{}` and `{right_table}.{}` have incompatible schema types.",
                    key.left_column, key.right_column
                ),
                Some("Join columns should use the same schema type and unit."),
            ));
        }
    }
}

fn join_key_mismatch_diagnostic(
    line: usize,
    expression: &str,
    left_table: &str,
    right_table: &str,
) -> Diagnostic {
    Diagnostic::error(
        "E-TABLE-JOIN-KEY-MISMATCH",
        line,
        &format!(
            "Join key `{expression}` must compare `{left_table}.<column>` with `{right_table}.<column>`."
        ),
        Some("Use `on { left.column == right.column }` with the tables named in the join."),
    )
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

fn validate_predicate_type(
    program: &SemanticProgram,
    transforms: &[TableTransformInfo],
    source_table: &str,
    expression: &str,
    line: usize,
    status: &mut String,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let or_parts = split_logical_expression(expression, "or");
    if or_parts.len() > 1 {
        for part in or_parts {
            validate_predicate_type(
                program,
                transforms,
                source_table,
                part,
                line,
                status,
                diagnostics,
            );
        }
        return;
    }
    let and_parts = split_logical_expression(expression, "and");
    if and_parts.len() > 1 {
        for part in and_parts {
            validate_predicate_type(
                program,
                transforms,
                source_table,
                part,
                line,
                status,
                diagnostics,
            );
        }
        return;
    }

    let predicate = parse_predicate(expression, line);
    let Some(column_name) = predicate.column.as_deref() else {
        return;
    };
    let Some(column) = schema_column_for_source(program, transforms, source_table, column_name)
    else {
        return;
    };
    let Some(value) = predicate.value.as_deref() else {
        return;
    };
    if schema_type_is_temporal(&column.type_name) {
        if predicate_value_is_obviously_non_temporal(value) {
            diagnostics.push(table_predicate_type_diagnostic(
                line,
                expression,
                column_name,
                &column.type_name,
                "Date/DateTime literal such as date(year, month, day) or an ISO timestamp string",
            ));
            *status = "type_mismatch".to_owned();
        }
    } else if predicate_value_is_date_constructor(value) {
        diagnostics.push(table_predicate_type_diagnostic(
            line,
            expression,
            column_name,
            &column.type_name,
            "non-date value",
        ));
        *status = "type_mismatch".to_owned();
    }
}

fn schema_type_is_temporal(type_name: &str) -> bool {
    matches!(type_name, "Date" | "DateTime")
}

fn predicate_value_is_date_constructor(value: &str) -> bool {
    value.trim().starts_with("date(")
}

fn predicate_value_is_obviously_non_temporal(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.starts_with("args.") || is_identifier(trimmed) {
        return false;
    }
    if predicate_value_is_date_constructor(trimmed) {
        return false;
    }
    let unquoted = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(trimmed);
    if date_prefix(unquoted).is_some() || datetime_like(unquoted) {
        return false;
    }
    trimmed.parse::<f64>().is_ok()
        || matches!(
            unquoted.to_ascii_lowercase().as_str(),
            "true" | "false" | "none" | "null"
        )
        || (trimmed.starts_with('"') && trimmed.ends_with('"'))
}

fn datetime_like(value: &str) -> bool {
    let Some((date, time)) = value.split_once('T') else {
        return false;
    };
    date_prefix(date).is_some()
        && time.len() >= 9
        && time.as_bytes().get(2) == Some(&b':')
        && time.as_bytes().get(5) == Some(&b':')
}

fn date_prefix(value: &str) -> Option<&str> {
    let date = value.trim().get(..10)?;
    let bytes = date.as_bytes();
    if bytes.len() == 10
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[4] == b'-'
        && bytes[5].is_ascii_digit()
        && bytes[6].is_ascii_digit()
        && bytes[7] == b'-'
        && bytes[8].is_ascii_digit()
        && bytes[9].is_ascii_digit()
    {
        Some(date)
    } else {
        None
    }
}

fn table_predicate_type_diagnostic(
    line: usize,
    expression: &str,
    column: &str,
    type_name: &str,
    expected: &str,
) -> Diagnostic {
    Diagnostic::error(
        "E-TABLE-PREDICATE-TYPE",
        line,
        &format!(
            "Filter predicate `{expression}` compares column `{column}` of type `{type_name}` with an incompatible value."
        ),
        Some(&format!("Use a {expected} for this table predicate.")),
    )
}

pub(crate) fn expression_columns(expression: &str) -> Vec<String> {
    let mut columns = Vec::new();
    let chars = expression.char_indices().collect::<Vec<_>>();
    let mut index = 0usize;
    while index < chars.len() {
        let (byte_index, character) = chars[index];
        if character.is_ascii_alphabetic() || character == '_' {
            let start = byte_index;
            let mut end = byte_index + character.len_utf8();
            index += 1;
            while index < chars.len() {
                let (next_byte, next_character) = chars[index];
                if !is_identifier_part(next_character) {
                    break;
                }
                end = next_byte + next_character.len_utf8();
                index += 1;
            }
            let identifier = &expression[start..end];
            let next_non_space = expression[end..].chars().find(|next| !next.is_whitespace());
            if !table_expression_keyword(identifier)
                && next_non_space != Some('(')
                && !columns.iter().any(|column| column == identifier)
            {
                columns.push(identifier.to_owned());
            }
            continue;
        }
        index += 1;
    }
    columns
}

fn table_expression_keyword(identifier: &str) -> bool {
    matches!(
        identifier.to_ascii_lowercase().as_str(),
        "and" | "or" | "is" | "not" | "none" | "null" | "true" | "false"
    )
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

fn source_has_column(
    program: &SemanticProgram,
    transforms: &[TableTransformInfo],
    table: &str,
    column: &str,
) -> bool {
    if let Some(transform) = transforms
        .iter()
        .find(|transform| transform.binding == table)
    {
        if transform.operation == "select" && !transform.selected_columns.is_empty() {
            return transform
                .selected_columns
                .iter()
                .any(|selected| selected.status == "accepted" && selected.name == column);
        }
        if transform
            .derived_columns
            .iter()
            .any(|derived| derived.status == "accepted" && derived.name == column)
        {
            return true;
        }
    }
    schema_column_for_source(program, transforms, table, column).is_some()
        || schema_name_for_source(program, transforms, table).is_none()
}

fn schema_name_for_table(program: &SemanticProgram, table: &str) -> Option<String> {
    program
        .csv_promotions
        .iter()
        .find(|promotion| promotion.binding == table)
        .map(|promotion| promotion.schema_name.clone())
}

fn schema_name_for_source(
    program: &SemanticProgram,
    transforms: &[TableTransformInfo],
    table: &str,
) -> Option<String> {
    schema_name_for_table(program, table).or_else(|| {
        transforms
            .iter()
            .find(|transform| transform.binding == table)
            .and_then(|transform| transform.schema_name.clone())
    })
}

fn schema_column_for_source<'a>(
    program: &'a SemanticProgram,
    transforms: &[TableTransformInfo],
    table: &str,
    column: &str,
) -> Option<&'a SchemaColumn> {
    if let Some(transform) = transforms
        .iter()
        .find(|transform| transform.binding == table)
    {
        if transform.operation == "select"
            && !transform.selected_columns.is_empty()
            && !transform
                .selected_columns
                .iter()
                .any(|selected| selected.status == "accepted" && selected.name == column)
        {
            return None;
        }
    }
    let schema_name = schema_name_for_source(program, transforms, table)?;
    let column = program
        .schemas
        .iter()
        .find(|schema| schema.name == schema_name)?
        .columns
        .iter()
        .find(|candidate| candidate.name == column)?;
    if optional_column_missing_from_source(program, transforms, table, column) {
        return None;
    }
    Some(column)
}

fn optional_column_missing_from_source(
    program: &SemanticProgram,
    transforms: &[TableTransformInfo],
    table: &str,
    column: &SchemaColumn,
) -> bool {
    if !column.optional {
        return false;
    }
    let base_table = base_table_for_source(transforms, table);
    program
        .csv_promotions
        .iter()
        .find(|promotion| promotion.binding == base_table)
        .is_some_and(|promotion| {
            promotion.source_hash.is_some()
                && promotion
                    .optional_missing_columns
                    .iter()
                    .any(|missing| missing == &column.name)
        })
}

fn base_table_for_source<'a>(transforms: &'a [TableTransformInfo], table: &'a str) -> &'a str {
    let mut current = table;
    let mut seen = Vec::new();
    loop {
        if seen.iter().any(|item| item == &current) {
            return current;
        }
        seen.push(current);
        let Some(transform) = transforms
            .iter()
            .find(|candidate| candidate.binding == current)
        else {
            return current;
        };
        current = &transform.source_table;
    }
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
