use std::collections::HashMap;
use std::fs;

use eng_compiler::{
    all_quantity_completions, all_unit_infos, CheckReport, SchemaColumn, SchemaInfo,
};
use eng_report::{
    PlotAxis, PlotPoint, PlotSeries, PlotSpec, ReportComputedIntegration,
    ReportComputedStatisticValue, ReportComputedStatistics, ReportPolicyResult,
    ReportPolicyViolation,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RuntimeData {
    pub tables: Vec<RuntimeTable>,
    pub time_series: Vec<RuntimeTimeSeries>,
    pub statistics: Vec<RuntimeStatistics>,
    pub integrations: Vec<RuntimeIntegration>,
    pub policy_results: Vec<RuntimePolicyResult>,
    pub plot_options: PlotOptions,
}

impl RuntimeData {
    pub fn apply_plot_spec(&self, report: &CheckReport, spec: &mut PlotSpec) {
        let requested_series = self
            .plot_options
            .series
            .as_deref()
            .or_else(|| spec.series.first().map(|series| series.name.as_str()));
        let series = requested_series
            .and_then(|name| self.time_series.iter().find(|series| series.name == name))
            .or_else(|| self.time_series.first());

        let Some(series) = series else {
            return;
        };

        let display_unit = self
            .plot_options
            .y_unit
            .clone()
            .unwrap_or_else(|| series.display_unit.clone());
        let points = series
            .points
            .iter()
            .map(|point| PlotPoint {
                x: point.x,
                y: convert_display_value(point.y, &series.display_unit, &display_unit),
            })
            .collect();

        let title = self
            .plot_options
            .title
            .clone()
            .unwrap_or_else(|| format!("{} over {}", series.name, series.axis));

        spec.title = title;
        if let Some(plot_type) = &self.plot_options.plot_type {
            spec.plot_type.clone_from(plot_type);
        }
        spec.x_axis = PlotAxis {
            name: series.axis.clone(),
            label: series.axis.clone(),
            unit: series.x_unit.clone(),
        };
        spec.y_axis = PlotAxis {
            name: series.quantity_kind.clone(),
            label: series.quantity_kind.clone(),
            unit: display_unit.clone(),
        };
        spec.series = vec![PlotSeries {
            name: series.name.clone(),
            quantity_kind: series.quantity_kind.clone(),
            display_unit,
            points,
        }];

        if spec.series.is_empty() && !report.semantic_program.typed_bindings.is_empty() {
            *spec = eng_report::plot_spec_from_report(report);
        }
    }

    pub fn report_computed_statistics(&self) -> Vec<ReportComputedStatistics> {
        self.statistics
            .iter()
            .map(|summary| ReportComputedStatistics {
                source: summary.source.clone(),
                quantity_kind: summary.quantity_kind.clone(),
                axis: summary.axis.clone(),
                status: summary.status.clone(),
                values: summary
                    .values
                    .iter()
                    .map(|value| ReportComputedStatisticValue {
                        name: value.name.clone(),
                        value: value.value,
                        unit: value.unit.clone(),
                    })
                    .collect(),
            })
            .collect()
    }

    pub fn report_computed_integrations(&self) -> Vec<ReportComputedIntegration> {
        self.integrations
            .iter()
            .map(|integration| ReportComputedIntegration {
                binding: integration.binding.clone(),
                source: integration.source.clone(),
                input_quantity: integration.input_quantity.clone(),
                over_axis: integration.over_axis.clone(),
                result_quantity: integration.result_quantity.clone(),
                value: integration.value,
                unit: integration.unit.clone(),
                method: integration.method.clone(),
                status: integration.status.clone(),
            })
            .collect()
    }

    pub fn report_policy_results(&self) -> Vec<ReportPolicyResult> {
        self.policy_results
            .iter()
            .map(|policy| ReportPolicyResult {
                schema: policy.schema.clone(),
                binding: policy.binding.clone(),
                kind: policy.kind.clone(),
                target: policy.target.clone(),
                policy: policy.policy.clone(),
                status: policy.status.clone(),
                checked_rows: policy.checked_rows,
                violation_count: policy.violations.len(),
                violations: policy
                    .violations
                    .iter()
                    .map(|violation| ReportPolicyViolation {
                        row: violation.row,
                        column: violation.column.clone(),
                        value: violation.value.clone(),
                        message: violation.message.clone(),
                    })
                    .collect(),
                line: policy.line,
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeTable {
    pub binding: String,
    pub schema_name: String,
    pub source: String,
    pub source_hash: Option<String>,
    pub row_count: usize,
    pub columns: Vec<RuntimeColumn>,
    pub parse_failures: Vec<RuntimeParseFailure>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeColumn {
    pub name: String,
    pub type_name: String,
    pub unit: Option<String>,
    pub canonical_unit: Option<String>,
    pub is_index: bool,
    pub values: RuntimeValues,
    pub canonical_values: Vec<Option<f64>>,
    pub missing_count: usize,
    pub conversion_failures: Vec<RuntimeConversionFailure>,
}

impl RuntimeColumn {
    pub fn len(&self) -> usize {
        match &self.values {
            RuntimeValues::Text(values) => values.len(),
            RuntimeValues::Number(values) => values.len(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeValues {
    Text(Vec<String>),
    Number(Vec<Option<f64>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeParseFailure {
    pub row: usize,
    pub column: String,
    pub value: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeConversionFailure {
    pub row: usize,
    pub column: String,
    pub value: String,
    pub source_unit: String,
    pub target_unit: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeTimeSeries {
    pub name: String,
    pub axis: String,
    pub x_unit: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub source_table: String,
    pub source_expression: String,
    pub points: Vec<RuntimePoint>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimePoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeStatistics {
    pub source: String,
    pub quantity_kind: String,
    pub axis: String,
    pub cache_key: String,
    pub status: String,
    pub values: Vec<RuntimeStatisticValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeStatisticValue {
    pub name: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeIntegration {
    pub binding: String,
    pub source: String,
    pub input_quantity: String,
    pub over_axis: String,
    pub result_quantity: String,
    pub value: f64,
    pub unit: String,
    pub method: String,
    pub status: String,
    pub interval_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimePolicyResult {
    pub schema: String,
    pub binding: String,
    pub kind: String,
    pub target: String,
    pub policy: String,
    pub status: String,
    pub checked_rows: usize,
    pub violations: Vec<RuntimePolicyViolation>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimePolicyViolation {
    pub row: usize,
    pub column: String,
    pub value: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlotOptions {
    pub series: Option<String>,
    pub axis: Option<String>,
    pub plot_type: Option<String>,
    pub title: Option<String>,
    pub y_unit: Option<String>,
}

pub fn materialize_runtime_data(report: &CheckReport, source: &str) -> RuntimeData {
    let mut data = RuntimeData {
        plot_options: parse_plot_options(source),
        ..RuntimeData::default()
    };

    for promotion in &report.semantic_program.csv_promotions {
        let Some(schema) = report
            .semantic_program
            .schemas
            .iter()
            .find(|schema| schema.name == promotion.schema_name)
        else {
            continue;
        };
        if let Some(table) = materialize_table(schema, promotion) {
            data.tables.push(table);
        }
    }

    data.policy_results = materialize_policy_results(report, &mut data.tables);
    data.time_series = materialize_time_series(report, &data.tables);
    data.statistics = materialize_statistics(report, &data.time_series);
    data.integrations = materialize_integrations(report, &data.time_series);
    data
}

fn materialize_table(
    schema: &SchemaInfo,
    promotion: &eng_compiler::CsvPromotion,
) -> Option<RuntimeTable> {
    let source = fs::read_to_string(&promotion.resolved_path).ok()?;
    let rows = parse_csv(&source);
    let headers = rows.first()?.clone();
    let header_index = headers
        .iter()
        .enumerate()
        .map(|(index, header)| (header.trim().to_owned(), index))
        .collect::<HashMap<_, _>>();
    let data_rows = rows.into_iter().skip(1).collect::<Vec<_>>();
    let mut parse_failures = Vec::new();
    let mut columns = Vec::new();

    for column in &schema.columns {
        let Some(index) = header_index.get(&column.name).copied() else {
            continue;
        };
        columns.push(materialize_column(
            column,
            index,
            &data_rows,
            &mut parse_failures,
        ));
    }

    Some(RuntimeTable {
        binding: promotion.binding.clone(),
        schema_name: promotion.schema_name.clone(),
        source: promotion.source_literal.clone(),
        source_hash: promotion.source_hash.clone(),
        row_count: data_rows.len(),
        columns,
        parse_failures,
    })
}

fn materialize_column(
    column: &SchemaColumn,
    index: usize,
    rows: &[Vec<String>],
    parse_failures: &mut Vec<RuntimeParseFailure>,
) -> RuntimeColumn {
    let mut missing_count = 0usize;
    if column.type_name == "DateTime" {
        let mut values = Vec::new();
        for (row_index, row) in rows.iter().enumerate() {
            let value = row.get(index).map(String::as_str).unwrap_or("").trim();
            if value.is_empty() {
                missing_count += 1;
            } else if parse_utc_timestamp_seconds(value).is_none() {
                parse_failures.push(RuntimeParseFailure {
                    row: row_index + 2,
                    column: column.name.clone(),
                    value: value.to_owned(),
                    message: "expected UTC DateTime like 2026-01-01T00:00:00Z".to_owned(),
                });
            }
            values.push(value.to_owned());
        }
        return RuntimeColumn {
            name: column.name.clone(),
            type_name: column.type_name.clone(),
            unit: column.unit.clone(),
            canonical_unit: None,
            is_index: column.is_index,
            values: RuntimeValues::Text(values),
            canonical_values: Vec::new(),
            missing_count,
            conversion_failures: Vec::new(),
        };
    }

    if is_numeric_schema_type(&column.type_name) {
        let mut values = Vec::new();
        let canonical_unit = canonical_unit_for_quantity(&column.type_name);
        let mut canonical_values = Vec::new();
        let mut conversion_failures = Vec::new();
        for (row_index, row) in rows.iter().enumerate() {
            let value = row.get(index).map(String::as_str).unwrap_or("").trim();
            if value.is_empty() {
                missing_count += 1;
                values.push(None);
                if canonical_unit.is_some() {
                    canonical_values.push(None);
                }
                continue;
            }
            match value.parse::<f64>() {
                Ok(number) if number.is_finite() => {
                    values.push(Some(number));
                    if let Some(target_unit) = canonical_unit.as_deref() {
                        match convert_to_canonical_unit(
                            number,
                            column.unit.as_deref(),
                            target_unit,
                            &column.type_name,
                        ) {
                            Ok(converted) => canonical_values.push(Some(converted)),
                            Err(message) => {
                                canonical_values.push(None);
                                conversion_failures.push(RuntimeConversionFailure {
                                    row: row_index + 2,
                                    column: column.name.clone(),
                                    value: value.to_owned(),
                                    source_unit: column.unit.clone().unwrap_or_default(),
                                    target_unit: target_unit.to_owned(),
                                    message,
                                });
                            }
                        }
                    }
                }
                _ => {
                    parse_failures.push(RuntimeParseFailure {
                        row: row_index + 2,
                        column: column.name.clone(),
                        value: value.to_owned(),
                        message: "expected finite numeric cell".to_owned(),
                    });
                    values.push(None);
                    if canonical_unit.is_some() {
                        canonical_values.push(None);
                    }
                }
            }
        }
        return RuntimeColumn {
            name: column.name.clone(),
            type_name: column.type_name.clone(),
            unit: column.unit.clone(),
            canonical_unit,
            is_index: column.is_index,
            values: RuntimeValues::Number(values),
            canonical_values,
            missing_count,
            conversion_failures,
        };
    }

    let values = rows
        .iter()
        .map(|row| row.get(index).cloned().unwrap_or_default())
        .inspect(|value| {
            if value.trim().is_empty() {
                missing_count += 1;
            }
        })
        .collect();
    RuntimeColumn {
        name: column.name.clone(),
        type_name: column.type_name.clone(),
        unit: column.unit.clone(),
        canonical_unit: None,
        is_index: column.is_index,
        values: RuntimeValues::Text(values),
        canonical_values: Vec::new(),
        missing_count,
        conversion_failures: Vec::new(),
    }
}

fn materialize_time_series(
    report: &CheckReport,
    tables: &[RuntimeTable],
) -> Vec<RuntimeTimeSeries> {
    let mut series = Vec::new();
    for binding in &report.semantic_program.typed_bindings {
        let Some((axis, quantity_kind)) =
            time_series_quantity(&binding.semantic_type.quantity_kind)
        else {
            continue;
        };
        let Some(inferred) = report
            .inferred_declarations
            .iter()
            .find(|declaration| declaration.name == binding.name)
        else {
            continue;
        };
        if quantity_kind != "HeatRate" {
            continue;
        }
        if let Some(runtime_series) = heat_rate_series(
            &binding.name,
            &axis,
            &quantity_kind,
            &binding.semantic_type.display_unit,
            &inferred.expression,
            report,
            tables,
        ) {
            series.push(runtime_series);
        }
    }
    series
}

fn heat_rate_series(
    name: &str,
    axis: &str,
    quantity_kind: &str,
    display_unit: &str,
    expression: &str,
    report: &CheckReport,
    tables: &[RuntimeTable],
) -> Option<RuntimeTimeSeries> {
    let table = tables
        .iter()
        .find(|table| expression.contains(&format!("{}.", table.binding)))?;
    let mass_flow = table.numeric_column_by_type("MassFlowRate")?;
    let supply = table.temperature_column("supply")?;
    let return_temp = table.temperature_column("return")?;
    let cp = specific_heat_value(report, expression)?;
    let (x_values, x_unit) = table.axis_values();

    let mut points = Vec::new();
    for index in 0..table.row_count {
        let (Some(m_dot), Some(supply), Some(return_temp)) = (
            optional_number_at(mass_flow, index),
            optional_number_at(supply, index),
            optional_number_at(return_temp, index),
        ) else {
            continue;
        };
        let x = x_values.get(index).copied().unwrap_or(index as f64);
        points.push(RuntimePoint {
            x,
            y: m_dot * cp * (return_temp - supply),
        });
    }

    Some(RuntimeTimeSeries {
        name: name.to_owned(),
        axis: axis.to_owned(),
        x_unit,
        quantity_kind: quantity_kind.to_owned(),
        display_unit: display_unit.to_owned(),
        source_table: table.binding.clone(),
        source_expression: expression.to_owned(),
        points,
    })
}

fn materialize_statistics(
    report: &CheckReport,
    series: &[RuntimeTimeSeries],
) -> Vec<RuntimeStatistics> {
    report
        .semantic_program
        .stats_infos
        .iter()
        .map(|stats| {
            let values = series
                .iter()
                .find(|series| series.name == stats.source)
                .map(|series| {
                    stats
                        .statistics
                        .iter()
                        .filter_map(|name| statistic_value(name, series))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            RuntimeStatistics {
                source: stats.source.clone(),
                quantity_kind: stats.quantity_kind.clone(),
                axis: stats.axis.clone(),
                cache_key: stats.cache_key.clone(),
                status: if values.is_empty() {
                    "unavailable".to_owned()
                } else {
                    "computed".to_owned()
                },
                values,
            }
        })
        .collect()
}

fn materialize_integrations(
    report: &CheckReport,
    series: &[RuntimeTimeSeries],
) -> Vec<RuntimeIntegration> {
    report
        .semantic_program
        .integrations
        .iter()
        .map(|integration| {
            let integrated = series
                .iter()
                .find(|series| series.name == integration.source)
                .and_then(trapezoidal_integral);
            RuntimeIntegration {
                binding: integration.binding.clone(),
                source: integration.source.clone(),
                input_quantity: integration.input_quantity.clone(),
                over_axis: integration.over_axis.clone(),
                result_quantity: integration.result_quantity.clone(),
                value: integrated.unwrap_or(0.0),
                unit: "J".to_owned(),
                method: "trapezoidal".to_owned(),
                status: if integrated.is_some() {
                    "computed".to_owned()
                } else {
                    "unavailable".to_owned()
                },
                interval_count: series
                    .iter()
                    .find(|series| series.name == integration.source)
                    .map(|series| series.points.len().saturating_sub(1))
                    .unwrap_or(0),
            }
        })
        .collect()
}

fn materialize_policy_results(
    report: &CheckReport,
    tables: &mut [RuntimeTable],
) -> Vec<RuntimePolicyResult> {
    let mut results = Vec::new();
    for promotion in &report.semantic_program.csv_promotions {
        let Some(schema) = report
            .semantic_program
            .schemas
            .iter()
            .find(|schema| schema.name == promotion.schema_name)
        else {
            continue;
        };
        let Some(table_index) = tables
            .iter()
            .position(|table| table.binding == promotion.binding)
        else {
            continue;
        };

        for policy in &schema.missing_policies {
            results.push(execute_missing_policy(
                &mut tables[table_index],
                schema,
                policy,
            ));
        }
        for constraint in &schema.constraints {
            results.push(execute_constraint_policy(
                &tables[table_index],
                schema,
                constraint,
            ));
        }
    }
    results
}

fn execute_constraint_policy(
    table: &RuntimeTable,
    schema: &SchemaInfo,
    constraint: &eng_compiler::SchemaConstraint,
) -> RuntimePolicyResult {
    let text = constraint.text.trim();
    if let Some(column) = text.strip_suffix(" is monotonic").map(str::trim) {
        return execute_monotonic_constraint(table, schema, constraint, column);
    }
    if let Some((column, min, max)) = parse_between_constraint(text) {
        return execute_between_constraint(table, schema, constraint, &column, min, max);
    }
    if let Some(bound) = parse_bound_constraint(text) {
        return execute_bound_constraint(table, schema, constraint, &bound);
    }
    policy_result(
        table,
        schema,
        PolicyResultDraft {
            kind: "constraint",
            target: text,
            policy: text,
            status: "recorded",
            checked_rows: 0,
            violations: Vec::new(),
            line: constraint.line,
        },
    )
}

fn execute_monotonic_constraint(
    table: &RuntimeTable,
    schema: &SchemaInfo,
    constraint: &eng_compiler::SchemaConstraint,
    column_name: &str,
) -> RuntimePolicyResult {
    let Some(column) = table.column(column_name) else {
        return policy_result(
            table,
            schema,
            PolicyResultDraft {
                kind: "constraint",
                target: column_name,
                policy: &constraint.text,
                status: "recorded",
                checked_rows: 0,
                violations: Vec::new(),
                line: constraint.line,
            },
        );
    };
    let RuntimeValues::Text(values) = &column.values else {
        return policy_result(
            table,
            schema,
            PolicyResultDraft {
                kind: "constraint",
                target: column_name,
                policy: &constraint.text,
                status: "recorded",
                checked_rows: 0,
                violations: Vec::new(),
                line: constraint.line,
            },
        );
    };
    let mut previous = None;
    let mut violations = Vec::new();
    for (index, value) in values.iter().enumerate() {
        let Some(timestamp) = parse_utc_timestamp_seconds(value) else {
            violations.push(RuntimePolicyViolation {
                row: index + 2,
                column: column_name.to_owned(),
                value: value.clone(),
                message: "cannot evaluate monotonic policy for invalid DateTime".to_owned(),
            });
            continue;
        };
        if previous.is_some_and(|previous| timestamp < previous) {
            violations.push(RuntimePolicyViolation {
                row: index + 2,
                column: column_name.to_owned(),
                value: value.clone(),
                message: "DateTime value is earlier than the previous row".to_owned(),
            });
        }
        previous = Some(timestamp);
    }
    policy_result(
        table,
        schema,
        PolicyResultDraft {
            kind: "constraint",
            target: column_name,
            policy: &constraint.text,
            status: "executed",
            checked_rows: values.len(),
            violations,
            line: constraint.line,
        },
    )
}

fn execute_between_constraint(
    table: &RuntimeTable,
    schema: &SchemaInfo,
    constraint: &eng_compiler::SchemaConstraint,
    column_name: &str,
    min: f64,
    max: f64,
) -> RuntimePolicyResult {
    let Some(column) = table.column(column_name) else {
        return policy_result(
            table,
            schema,
            PolicyResultDraft {
                kind: "constraint",
                target: column_name,
                policy: &constraint.text,
                status: "recorded",
                checked_rows: 0,
                violations: Vec::new(),
                line: constraint.line,
            },
        );
    };
    let values = numeric_values(column);
    let violations = values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            let value = (*value)?;
            if value < min || value > max {
                Some(RuntimePolicyViolation {
                    row: index + 2,
                    column: column_name.to_owned(),
                    value: value.to_string(),
                    message: format!("value is outside [{min}, {max}]"),
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    policy_result(
        table,
        schema,
        PolicyResultDraft {
            kind: "constraint",
            target: column_name,
            policy: &constraint.text,
            status: "executed",
            checked_rows: values.len(),
            violations,
            line: constraint.line,
        },
    )
}

fn execute_bound_constraint(
    table: &RuntimeTable,
    schema: &SchemaInfo,
    constraint: &eng_compiler::SchemaConstraint,
    bound: &BoundConstraint,
) -> RuntimePolicyResult {
    let Some(column) = table.column(&bound.column) else {
        return policy_result(
            table,
            schema,
            PolicyResultDraft {
                kind: "constraint",
                target: &bound.column,
                policy: &constraint.text,
                status: "recorded",
                checked_rows: 0,
                violations: Vec::new(),
                line: constraint.line,
            },
        );
    };
    let values = numeric_values(column);
    let violations = values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            let value = (*value)?;
            if !bound.accepts(value) {
                Some(RuntimePolicyViolation {
                    row: index + 2,
                    column: bound.column.clone(),
                    value: value.to_string(),
                    message: bound.violation_message(),
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    policy_result(
        table,
        schema,
        PolicyResultDraft {
            kind: "constraint",
            target: &bound.column,
            policy: &constraint.text,
            status: "executed",
            checked_rows: values.len(),
            violations,
            line: constraint.line,
        },
    )
}

fn execute_missing_policy(
    table: &mut RuntimeTable,
    schema: &SchemaInfo,
    policy: &eng_compiler::MissingPolicy,
) -> RuntimePolicyResult {
    let Some(column_index) = table
        .columns
        .iter()
        .position(|column| column.name == policy.column)
    else {
        return policy_result(
            table,
            schema,
            PolicyResultDraft {
                kind: "missing",
                target: &policy.column,
                policy: &policy.policy,
                status: "recorded",
                checked_rows: 0,
                violations: Vec::new(),
                line: policy.line,
            },
        );
    };
    let missing_rows = missing_rows(&table.columns[column_index]);
    if policy.policy.trim() == "error" {
        let violations = missing_rows
            .iter()
            .map(|row| RuntimePolicyViolation {
                row: *row,
                column: policy.column.clone(),
                value: String::new(),
                message: "missing value violates `error` policy".to_owned(),
            })
            .collect::<Vec<_>>();
        return policy_result(
            table,
            schema,
            PolicyResultDraft {
                kind: "missing",
                target: &policy.column,
                policy: &policy.policy,
                status: "executed",
                checked_rows: table.columns[column_index].len(),
                violations,
                line: policy.line,
            },
        );
    }

    if policy.policy.trim_start().starts_with("interpolate") {
        let violations =
            interpolate_missing_values(&mut table.columns[column_index], &missing_rows);
        return policy_result(
            table,
            schema,
            PolicyResultDraft {
                kind: "missing",
                target: &policy.column,
                policy: &policy.policy,
                status: "executed",
                checked_rows: table.columns[column_index].len(),
                violations,
                line: policy.line,
            },
        );
    }

    policy_result(
        table,
        schema,
        PolicyResultDraft {
            kind: "missing",
            target: &policy.column,
            policy: &policy.policy,
            status: "recorded",
            checked_rows: table.columns[column_index].len(),
            violations: Vec::new(),
            line: policy.line,
        },
    )
}

fn interpolate_missing_values(
    column: &mut RuntimeColumn,
    missing_rows: &[usize],
) -> Vec<RuntimePolicyViolation> {
    let RuntimeValues::Number(values) = &mut column.values else {
        return missing_rows
            .iter()
            .map(|row| RuntimePolicyViolation {
                row: *row,
                column: column.name.clone(),
                value: String::new(),
                message: "interpolation requires a numeric column".to_owned(),
            })
            .collect();
    };

    let mut violations = Vec::new();
    for row in missing_rows {
        let Some(index) = row.checked_sub(2) else {
            continue;
        };
        if values.get(index).and_then(|value| *value).is_some() {
            continue;
        }
        let previous = (0..index)
            .rev()
            .find_map(|candidate| values[candidate].map(|value| (candidate, value)));
        let next = ((index + 1)..values.len())
            .find_map(|candidate| values[candidate].map(|value| (candidate, value)));
        let (Some((previous_index, previous_value)), Some((next_index, next_value))) =
            (previous, next)
        else {
            violations.push(RuntimePolicyViolation {
                row: *row,
                column: column.name.clone(),
                value: String::new(),
                message: "cannot interpolate without surrounding numeric values".to_owned(),
            });
            continue;
        };
        let fraction = (index - previous_index) as f64 / (next_index - previous_index) as f64;
        values[index] = Some(previous_value + (next_value - previous_value) * fraction);
    }
    column.missing_count = values.iter().filter(|value| value.is_none()).count();
    violations
}

struct PolicyResultDraft<'a> {
    kind: &'a str,
    target: &'a str,
    policy: &'a str,
    status: &'a str,
    checked_rows: usize,
    violations: Vec<RuntimePolicyViolation>,
    line: usize,
}

fn policy_result(
    table: &RuntimeTable,
    schema: &SchemaInfo,
    draft: PolicyResultDraft<'_>,
) -> RuntimePolicyResult {
    RuntimePolicyResult {
        schema: schema.name.clone(),
        binding: table.binding.clone(),
        kind: draft.kind.to_owned(),
        target: draft.target.to_owned(),
        policy: draft.policy.to_owned(),
        status: draft.status.to_owned(),
        checked_rows: draft.checked_rows,
        violations: draft.violations,
        line: draft.line,
    }
}

impl RuntimeTable {
    fn column(&self, name: &str) -> Option<&RuntimeColumn> {
        self.columns.iter().find(|column| column.name == name)
    }

    fn numeric_column_by_type(&self, type_name: &str) -> Option<&RuntimeColumn> {
        self.columns.iter().find(|column| {
            column.type_name == type_name && matches!(&column.values, RuntimeValues::Number(_))
        })
    }

    fn temperature_column(&self, name_hint: &str) -> Option<&RuntimeColumn> {
        self.columns.iter().find(|column| {
            column.type_name == "AbsoluteTemperature"
                && column.name.to_ascii_lowercase().contains(name_hint)
                && matches!(&column.values, RuntimeValues::Number(_))
        })
    }

    fn axis_values(&self) -> (Vec<f64>, String) {
        let Some(column) = self
            .columns
            .iter()
            .find(|column| column.is_index && column.type_name == "DateTime")
        else {
            return (
                (0..self.row_count).map(|index| index as f64).collect(),
                "sample".to_owned(),
            );
        };
        let RuntimeValues::Text(values) = &column.values else {
            return (
                (0..self.row_count).map(|index| index as f64).collect(),
                "sample".to_owned(),
            );
        };
        let timestamps = values
            .iter()
            .map(|value| parse_utc_timestamp_seconds(value))
            .collect::<Option<Vec<_>>>();
        let Some(timestamps) = timestamps else {
            return (
                (0..self.row_count).map(|index| index as f64).collect(),
                "sample".to_owned(),
            );
        };
        let Some(first) = timestamps.first().copied() else {
            return (Vec::new(), "s".to_owned());
        };
        (
            timestamps
                .iter()
                .map(|timestamp| (*timestamp - first) as f64)
                .collect(),
            "s".to_owned(),
        )
    }
}

fn numeric_values(column: &RuntimeColumn) -> &[Option<f64>] {
    let RuntimeValues::Number(values) = &column.values else {
        return &[];
    };
    values
}

fn missing_rows(column: &RuntimeColumn) -> Vec<usize> {
    match &column.values {
        RuntimeValues::Text(values) => values
            .iter()
            .enumerate()
            .filter(|(_, value)| value.trim().is_empty())
            .map(|(index, _)| index + 2)
            .collect(),
        RuntimeValues::Number(values) => values
            .iter()
            .enumerate()
            .filter(|(_, value)| value.is_none())
            .map(|(index, _)| index + 2)
            .collect(),
    }
}

fn parse_between_constraint(text: &str) -> Option<(String, f64, f64)> {
    let (column, rest) = text.split_once(" between ")?;
    let (min_part, max_part) = rest.split_once(" and ")?;
    Some((
        column.trim().to_owned(),
        first_number(min_part)?,
        first_number(max_part)?,
    ))
}

#[derive(Clone, Debug, PartialEq)]
struct BoundConstraint {
    column: String,
    operator: String,
    threshold: f64,
}

impl BoundConstraint {
    fn accepts(&self, value: f64) -> bool {
        match self.operator.as_str() {
            ">=" => value >= self.threshold,
            ">" => value > self.threshold,
            "<=" => value <= self.threshold,
            "<" => value < self.threshold,
            _ => true,
        }
    }

    fn violation_message(&self) -> String {
        match self.operator.as_str() {
            ">=" => format!("value is below lower bound {}", self.threshold),
            ">" => format!("value is not greater than {}", self.threshold),
            "<=" => format!("value is above upper bound {}", self.threshold),
            "<" => format!("value is not less than {}", self.threshold),
            _ => "value violates bound constraint".to_owned(),
        }
    }
}

fn parse_bound_constraint(text: &str) -> Option<BoundConstraint> {
    for operator in [">=", "<=", ">", "<"] {
        if let Some((column, rest)) = text.split_once(operator) {
            return Some(BoundConstraint {
                column: column.trim().to_owned(),
                operator: operator.to_owned(),
                threshold: first_number(rest)?,
            });
        }
    }
    None
}

fn first_number(text: &str) -> Option<f64> {
    text.split_whitespace()
        .find_map(|part| part.parse::<f64>().ok())
}

fn parse_plot_options(source: &str) -> PlotOptions {
    let mut options = PlotOptions::default();
    let Some(plot_index) = source.find("plot ") else {
        return options;
    };
    let after_plot = &source[plot_index + "plot ".len()..];
    let header_end = after_plot.find('{').unwrap_or(after_plot.len());
    let header = after_plot[..header_end].trim();
    if let Some((series, axis)) = header.split_once(" over ") {
        options.series = series.split_whitespace().next().map(str::to_owned);
        options.axis = axis.split_whitespace().next().map(str::to_owned);
    }

    let Some(block_start) = after_plot.find('{') else {
        return options;
    };
    let Some(block_end) = after_plot[block_start + 1..].find('}') else {
        return options;
    };
    let block = &after_plot[block_start + 1..block_start + 1 + block_end];
    for line in block.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("unit y =") {
            options.y_unit = rest.split_whitespace().next().map(str::to_owned);
        } else if let Some(rest) = line.strip_prefix("type =") {
            options.plot_type = supported_plot_type(rest.trim());
        } else if let Some(rest) = line.strip_prefix("title =") {
            options.title = quoted_value(rest.trim());
        }
    }
    options
}

fn supported_plot_type(value: &str) -> Option<String> {
    let plot_type = value.split_whitespace().next()?;
    matches!(plot_type, "line" | "bar" | "histogram").then(|| plot_type.to_owned())
}

fn quoted_value(value: &str) -> Option<String> {
    let rest = value.strip_prefix('"')?;
    let (quoted, _) = rest.split_once('"')?;
    Some(quoted.to_owned())
}

fn specific_heat_value(report: &CheckReport, expression: &str) -> Option<f64> {
    if let Some(value) = number_before_unit(expression, "J/kg/K") {
        return Some(value);
    }

    report
        .inferred_declarations
        .iter()
        .filter(|declaration| expression_contains_identifier(expression, &declaration.name))
        .find_map(|declaration| number_before_unit(&declaration.expression, "J/kg/K"))
}

fn expression_contains_identifier(expression: &str, identifier: &str) -> bool {
    expression
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .any(|token| token == identifier)
}

fn number_before_unit(expression: &str, unit: &str) -> Option<f64> {
    let unit_index = expression.find(unit)?;
    expression[..unit_index]
        .split_whitespace()
        .rev()
        .find_map(|part| part.parse::<f64>().ok())
}

fn number_with_optional_unit(text: &str) -> Option<(f64, Option<String>)> {
    let parts = text.split_whitespace().collect::<Vec<_>>();
    for (index, part) in parts.iter().enumerate() {
        let number_part = part.trim_matches(|character| matches!(character, '(' | ')' | ','));
        if let Ok(value) = number_part.parse::<f64>() {
            let unit = parts
                .get(index + 1)
                .map(|unit| unit.trim_matches(|character| matches!(character, '(' | ')' | ',')))
                .filter(|unit| !unit.is_empty())
                .map(str::to_owned);
            return Some((value, unit));
        }
    }
    None
}

fn statistic_value(name: &str, series: &RuntimeTimeSeries) -> Option<RuntimeStatisticValue> {
    let values = series
        .points
        .iter()
        .map(|point| point.y)
        .collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    let (value, unit) = match name {
        "mean" => (
            values.iter().sum::<f64>() / values.len() as f64,
            series.display_unit.clone(),
        ),
        "time_weighted_mean" => (time_weighted_mean(series)?, series.display_unit.clone()),
        "max" => (
            values.iter().copied().reduce(f64::max)?,
            series.display_unit.clone(),
        ),
        "min" => (
            values.iter().copied().reduce(f64::min)?,
            series.display_unit.clone(),
        ),
        "median" => (median(&values)?, series.display_unit.clone()),
        "std" => (population_std(&values)?, series.display_unit.clone()),
        percentile if percentile_fraction(percentile).is_some() => (
            nearest_rank_percentile(&values, percentile_fraction(percentile)?)?,
            series.display_unit.clone(),
        ),
        _ => {
            let threshold = duration_above_threshold(name, &series.display_unit)?;
            (duration_above(series, threshold)?, "s".to_owned())
        }
    };
    Some(RuntimeStatisticValue {
        name: name.to_owned(),
        value,
        unit,
    })
}

fn nearest_rank_percentile(values: &[f64], percentile: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = (percentile * sorted.len() as f64).ceil() as usize;
    sorted.get(rank.saturating_sub(1)).copied()
}

fn percentile_fraction(name: &str) -> Option<f64> {
    let percentile = name.strip_prefix('p')?.parse::<u32>().ok()?;
    (1..=100)
        .contains(&percentile)
        .then_some(percentile as f64 / 100.0)
}

fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let midpoint = sorted.len() / 2;
    if sorted.len() & 1 == 0 {
        Some((sorted[midpoint - 1] + sorted[midpoint]) * 0.5)
    } else {
        sorted.get(midpoint).copied()
    }
}

fn population_std(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>()
        / values.len() as f64;
    Some(variance.sqrt())
}

fn duration_above_threshold(name: &str, display_unit: &str) -> Option<f64> {
    let inside = name
        .trim()
        .strip_prefix("duration_above(")?
        .strip_suffix(')')?;
    let (value, unit) = number_with_optional_unit(inside)?;
    Some(
        unit.as_deref()
            .map(|unit| convert_display_value(value, unit, display_unit))
            .unwrap_or(value),
    )
}

fn duration_above(series: &RuntimeTimeSeries, threshold: f64) -> Option<f64> {
    if series.x_unit != "s" || series.points.len() < 2 {
        return None;
    }
    let mut duration = 0.0;
    for window in series.points.windows(2) {
        let dt = window[1].x - window[0].x;
        if dt <= 0.0 {
            return None;
        }
        let y0 = window[0].y;
        let y1 = window[1].y;
        let y0_above = y0 > threshold;
        let y1_above = y1 > threshold;
        if y0_above && y1_above {
            duration += dt;
        } else if y0_above != y1_above {
            let dy = y1 - y0;
            if dy.abs() <= f64::EPSILON {
                continue;
            }
            let fraction = ((threshold - y0) / dy).clamp(0.0, 1.0);
            duration += if y0_above {
                fraction * dt
            } else {
                (1.0 - fraction) * dt
            };
        }
    }
    Some(duration)
}

fn time_weighted_mean(series: &RuntimeTimeSeries) -> Option<f64> {
    let total_duration = series.points.last()?.x - series.points.first()?.x;
    if series.x_unit != "s" || total_duration <= 0.0 {
        return None;
    }
    Some(trapezoidal_integral(series)? / total_duration)
}

fn trapezoidal_integral(series: &RuntimeTimeSeries) -> Option<f64> {
    if series.x_unit != "s" || series.points.len() < 2 {
        return None;
    }
    let mut integral = 0.0;
    for window in series.points.windows(2) {
        let dt = window[1].x - window[0].x;
        if dt <= 0.0 {
            return None;
        }
        integral += (window[0].y + window[1].y) * 0.5 * dt;
    }
    Some(integral)
}

fn optional_number_at(column: &RuntimeColumn, index: usize) -> Option<f64> {
    let RuntimeValues::Number(values) = &column.values else {
        return None;
    };
    values.get(index).copied().flatten()
}

fn convert_display_value(value: f64, from_unit: &str, to_unit: &str) -> f64 {
    match (from_unit, to_unit) {
        ("W", "kW") => value / 1000.0,
        ("kW", "W") => value * 1000.0,
        _ => value,
    }
}

fn canonical_unit_for_quantity(quantity_kind: &str) -> Option<String> {
    all_quantity_completions()
        .iter()
        .find(|completion| completion.quantity_kind == quantity_kind)
        .map(|completion| completion.canonical_unit.to_owned())
}

fn convert_to_canonical_unit(
    value: f64,
    source_unit: Option<&str>,
    target_unit: &str,
    quantity_kind: &str,
) -> Result<f64, String> {
    let source_unit = source_unit
        .map(str::trim)
        .filter(|unit| !unit.is_empty())
        .unwrap_or(target_unit);

    if source_unit.eq_ignore_ascii_case(target_unit) {
        return Ok(value);
    }

    let Some(info) = all_unit_infos()
        .iter()
        .find(|info| info.symbol.eq_ignore_ascii_case(source_unit))
    else {
        return Err(format!(
            "unsupported source unit `{source_unit}` for {quantity_kind}"
        ));
    };

    if !info.canonical_unit.eq_ignore_ascii_case(target_unit)
        || !unit_seed_matches_quantity(info.quantity_hint, quantity_kind)
    {
        return Err(format!(
            "cannot convert `{source_unit}` to canonical `{target_unit}` for {quantity_kind}"
        ));
    }

    let scale = info.scale_to_canonical.parse::<f64>().map_err(|_| {
        format!(
            "invalid conversion scale `{}` for unit `{source_unit}`",
            info.scale_to_canonical
        )
    })?;
    let offset = info
        .affine_offset
        .map(|offset| {
            offset
                .parse::<f64>()
                .map_err(|_| format!("invalid affine offset `{offset}` for unit `{source_unit}`"))
        })
        .transpose()?
        .unwrap_or(0.0);

    Ok(value * scale + offset)
}

fn unit_seed_matches_quantity(seed_quantity: &str, quantity_kind: &str) -> bool {
    seed_quantity == quantity_kind
        || seed_quantity == "Power"
            && matches!(
                quantity_kind,
                "HeatRate" | "ElectricPower" | "MechanicalPower"
            )
        || seed_quantity == "TemperatureDelta" && quantity_kind == "AbsoluteTemperature"
}

fn is_numeric_schema_type(type_name: &str) -> bool {
    !matches!(type_name, "DateTime" | "String")
}

fn time_series_quantity(quantity_kind: &str) -> Option<(String, String)> {
    let rest = quantity_kind.strip_prefix("TimeSeries[")?;
    let (axis, after_axis) = rest.split_once(']')?;
    let quantity = after_axis.trim().strip_prefix("of ")?;
    Some((axis.trim().to_owned(), quantity.trim().to_owned()))
}

fn parse_csv(source: &str) -> Vec<Vec<String>> {
    source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_csv_line)
        .collect()
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                fields.push(field.trim().to_owned());
                field.clear();
            }
            _ => field.push(character),
        }
    }
    fields.push(field.trim().to_owned());
    fields
}

fn parse_utc_timestamp_seconds(value: &str) -> Option<i64> {
    let value = value.strip_suffix('Z')?;
    let (date, time) = value.split_once('T')?;
    let mut date_parts = date.split('-');
    let year = date_parts.next()?.parse::<i32>().ok()?;
    let month = date_parts.next()?.parse::<u32>().ok()?;
    let day = date_parts.next()?.parse::<u32>().ok()?;
    if date_parts.next().is_some() {
        return None;
    }
    let mut time_parts = time.split(':');
    let hour = time_parts.next()?.parse::<u32>().ok()?;
    let minute = time_parts.next()?.parse::<u32>().ok()?;
    let second = time_parts.next()?.parse::<u32>().ok()?;
    if time_parts.next().is_some()
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + i64::from(hour * 3600 + minute * 60 + second))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = (year - era * 400) as u32;
    let month = month as i32;
    let day_of_year =
        ((153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i32 - 1) as u32;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    i64::from(era) * 146_097 + i64::from(day_of_era) - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use eng_compiler::{check_file, CheckOptions};

    #[test]
    fn parses_plot_options() {
        let options = parse_plot_options(
            r#"
script main(args: Args) -> Report {
    return report {
        plot Q_coil over Time {
            unit y = kW
            type = histogram
            title = "Coil heat rate"
        }
    }
}
"#,
        );

        assert_eq!(options.series.as_deref(), Some("Q_coil"));
        assert_eq!(options.axis.as_deref(), Some("Time"));
        assert_eq!(options.y_unit.as_deref(), Some("kW"));
        assert_eq!(options.plot_type.as_deref(), Some("histogram"));
        assert_eq!(options.title.as_deref(), Some("Coil heat rate"));
    }

    #[test]
    fn computes_heat_rate_statistics_and_integral() {
        let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/official/01_csv_plot/main.eng");
        let source = std::fs::read_to_string(&source_path).unwrap();
        let report = check_file(&source_path, &CheckOptions::default()).unwrap();
        let runtime = materialize_runtime_data(&report, &source);

        assert_eq!(runtime.tables[0].row_count, 4);
        assert_eq!(runtime.time_series[0].points.len(), 4);
        assert_eq!(runtime.time_series[0].points[1].x, 300.0);
        assert_eq!(round2(runtime.time_series[0].points[0].y), 4873.88);
        assert_eq!(
            round2(stat_value(&runtime.statistics[0].values, "mean").unwrap()),
            5072.43
        );
        assert_eq!(
            round2(stat_value(&runtime.statistics[0].values, "time_weighted_mean").unwrap()),
            5048.05
        );
        assert_eq!(
            round2(stat_value(&runtime.statistics[0].values, "max").unwrap()),
            5417.28
        );
        assert_eq!(
            round2(stat_value(&runtime.statistics[0].values, "median").unwrap()),
            4999.28
        );
        assert_eq!(
            round2(stat_value(&runtime.statistics[0].values, "std").unwrap()),
            205.58
        );
        assert_eq!(
            round2(stat_value(&runtime.statistics[0].values, "p90").unwrap()),
            5417.28
        );
        assert_eq!(
            round2(stat_value(&runtime.statistics[0].values, "duration_above(5 kW)").unwrap()),
            299.48
        );
        assert_eq!(
            stat_unit(&runtime.statistics[0].values, "duration_above(5 kW)").as_deref(),
            Some("s")
        );
        assert_eq!(round2(runtime.integrations[0].value), 4543242.0);
        assert_eq!(runtime.policy_results.len(), 7);
        assert_eq!(
            runtime
                .policy_results
                .iter()
                .filter(|policy| policy.status == "executed")
                .count(),
            7
        );
        assert_eq!(
            runtime
                .policy_results
                .iter()
                .filter(|policy| policy.status == "validated")
                .count(),
            0
        );
        assert!(runtime
            .policy_results
            .iter()
            .all(|policy| policy.violations.is_empty()));
    }

    fn round2(value: f64) -> f64 {
        (value * 100.0).round() / 100.0
    }

    fn stat_value(values: &[RuntimeStatisticValue], name: &str) -> Option<f64> {
        values
            .iter()
            .find(|value| value.name == name)
            .map(|value| value.value)
    }

    fn stat_unit(values: &[RuntimeStatisticValue], name: &str) -> Option<String> {
        values
            .iter()
            .find(|value| value.name == name)
            .map(|value| value.unit.clone())
    }
}
