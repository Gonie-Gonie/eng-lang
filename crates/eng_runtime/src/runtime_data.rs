use std::collections::HashMap;
use std::fs;

use eng_compiler::{
    all_quantity_completions, all_unit_infos, normalize_unit, CheckReport, SchemaColumn, SchemaInfo,
};
use eng_report::{
    PlotAxis, PlotBin, PlotPoint, PlotSeries, PlotSpec, ReportComputedIntegration,
    ReportComputedStatisticValue, ReportComputedStatistics, ReportMlCoefficient, ReportMlInfo,
    ReportPolicyResult, ReportPolicyViolation, ReportSpec, ReportUncertaintyInfo,
    ReportUncertaintyPropagationTerm,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RuntimeData {
    pub tables: Vec<RuntimeTable>,
    pub time_series: Vec<RuntimeTimeSeries>,
    pub statistics: Vec<RuntimeStatistics>,
    pub integrations: Vec<RuntimeIntegration>,
    pub uncertainties: Vec<RuntimeUncertainty>,
    pub ml_artifacts: Vec<RuntimeMlArtifact>,
    pub policy_results: Vec<RuntimePolicyResult>,
    pub system_solutions: Vec<RuntimeSystemSolution>,
    pub plot_options: PlotOptions,
}

impl RuntimeData {
    pub fn apply_plot_spec(&self, report: &CheckReport, spec: &mut PlotSpec) {
        if let Some(distribution_name) = self.plot_options.distribution.as_deref() {
            if let Some(uncertainty) = self
                .uncertainties
                .iter()
                .find(|uncertainty| uncertainty.binding == distribution_name)
            {
                self.apply_uncertainty_plot(uncertainty, spec);
                return;
            }
        }
        if let Some(model_plot) = &self.plot_options.model_plot {
            if let Some(artifact) = self
                .ml_artifacts
                .iter()
                .find(|artifact| artifact.binding == model_plot.source)
            {
                self.apply_model_plot(model_plot, artifact, spec);
                return;
            }
        }

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
            bins: Vec::new(),
            points,
        }];

        if spec.series.is_empty() && !report.semantic_program.typed_bindings.is_empty() {
            *spec = eng_report::plot_spec_from_report(report);
        }
    }

    fn apply_uncertainty_plot(&self, uncertainty: &RuntimeUncertainty, spec: &mut PlotSpec) {
        let title = self
            .plot_options
            .title
            .clone()
            .unwrap_or_else(|| format!("{} distribution", uncertainty.binding));
        spec.title = title;
        spec.plot_type = "histogram".to_owned();
        spec.x_axis = PlotAxis {
            name: uncertainty.binding.clone(),
            label: uncertainty.quantity_kind.clone(),
            unit: uncertainty.display_unit.clone(),
        };
        spec.y_axis = PlotAxis {
            name: "Frequency".to_owned(),
            label: "Frequency".to_owned(),
            unit: "count".to_owned(),
        };
        let bins = histogram_bins(&uncertainty.samples);
        let points = histogram_points_from_bins(&bins);
        spec.series = vec![PlotSeries {
            name: uncertainty.binding.clone(),
            quantity_kind: uncertainty.quantity_kind.clone(),
            display_unit: uncertainty.display_unit.clone(),
            bins,
            points,
        }];
    }

    fn apply_model_plot(
        &self,
        model_plot: &ModelPlotOptions,
        artifact: &RuntimeMlArtifact,
        spec: &mut PlotSpec,
    ) {
        let title = self.plot_options.title.clone().unwrap_or_else(|| {
            format!(
                "{} {}",
                artifact.binding,
                if model_plot.kind == "parity" {
                    "parity"
                } else {
                    "residuals"
                }
            )
        });
        spec.title = title;
        spec.plot_type = if model_plot.kind == "parity" {
            "scatter".to_owned()
        } else {
            "bar".to_owned()
        };
        spec.x_axis = PlotAxis {
            name: if model_plot.kind == "parity" {
                "Actual".to_owned()
            } else {
                "Sample".to_owned()
            },
            label: if model_plot.kind == "parity" {
                "Actual".to_owned()
            } else {
                "Sample".to_owned()
            },
            unit: artifact.display_unit.clone(),
        };
        spec.y_axis = PlotAxis {
            name: if model_plot.kind == "parity" {
                "Predicted".to_owned()
            } else {
                "Residual".to_owned()
            },
            label: if model_plot.kind == "parity" {
                "Predicted".to_owned()
            } else {
                "Residual".to_owned()
            },
            unit: artifact.display_unit.clone(),
        };
        let points = if model_plot.kind == "parity" {
            artifact.parity_points.clone()
        } else {
            artifact.residual_points.clone()
        }
        .into_iter()
        .map(|point| PlotPoint {
            x: point.x,
            y: point.y,
        })
        .collect();
        spec.series = vec![PlotSeries {
            name: artifact.binding.clone(),
            quantity_kind: artifact
                .target
                .clone()
                .unwrap_or_else(|| "Model".to_owned()),
            display_unit: artifact.display_unit.clone(),
            bins: Vec::new(),
            points,
        }];
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

    pub fn report_uncertainty(&self) -> Vec<ReportUncertaintyInfo> {
        self.uncertainties
            .iter()
            .map(|uncertainty| ReportUncertaintyInfo {
                binding: uncertainty.binding.clone(),
                kind: uncertainty.kind.clone(),
                quantity_kind: uncertainty.quantity_kind.clone(),
                display_unit: uncertainty.display_unit.clone(),
                expression: uncertainty.expression.clone(),
                source: uncertainty.source.clone(),
                distribution: uncertainty.distribution.clone(),
                method: uncertainty.method.clone(),
                scale: uncertainty.scale.map(format_number),
                offset: uncertainty.offset.map(format_number),
                mean: uncertainty.mean.map(format_number),
                stddev: uncertainty.stddev.map(format_number),
                lower: uncertainty.lower.map(format_number),
                upper: uncertainty.upper.map(format_number),
                p05: uncertainty.p05.map(format_number),
                p50: uncertainty.p50.map(format_number),
                p95: uncertainty.p95.map(format_number),
                sample_count: uncertainty.sample_count,
                propagation_count: uncertainty.propagation_count,
                propagation: uncertainty.propagation.clone(),
                line: uncertainty.line,
            })
            .collect()
    }

    pub fn report_ml(&self) -> Vec<ReportMlInfo> {
        self.ml_artifacts
            .iter()
            .map(|artifact| ReportMlInfo {
                binding: artifact.binding.clone(),
                kind: artifact.kind.clone(),
                source: artifact.source.clone(),
                target: artifact.target.clone(),
                features: artifact.features.clone(),
                algorithm: artifact.algorithm.clone(),
                test_fraction: artifact.test_fraction.clone(),
                seed: artifact.seed.clone(),
                hidden_layers: artifact.hidden_layers.clone(),
                epochs: artifact.epochs,
                status: artifact.status.clone(),
                train_count: artifact.train_count,
                test_count: artifact.test_count,
                rmse: artifact.rmse,
                mae: artifact.mae,
                r2: artifact.r2,
                leakage_status: artifact.leakage_status.clone(),
                leakage_findings: artifact.leakage_findings.clone(),
                coefficients: artifact
                    .coefficients
                    .iter()
                    .map(|coefficient| ReportMlCoefficient {
                        feature: coefficient.feature.clone(),
                        value: coefficient.value,
                    })
                    .collect(),
                intercept: artifact.intercept,
                loss_history: artifact.loss_history.clone(),
                model_card: artifact.model_card.clone(),
                expression: artifact.expression.clone(),
                line: artifact.line,
            })
            .collect()
    }

    pub fn apply_system_solutions(&self, spec: &mut ReportSpec) {
        for solution in &self.system_solutions {
            let Some(system_ir) = spec
                .system_ir
                .iter_mut()
                .find(|system| system.name == solution.system)
            else {
                continue;
            };
            system_ir.solver_boundary.status = solution.status.clone();
            system_ir.solver_boundary.reason = solution.reason.clone();
            system_ir.solver_plan.status = solution.status.clone();
            system_ir.solver_plan.method = solution.method.clone();
            system_ir.solver_plan.ode_runner.status = solution.status.clone();
            system_ir.solver_plan.ode_runner.reason = solution.reason.clone();
        }
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
pub struct RuntimeUncertainty {
    pub binding: String,
    pub kind: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub expression: String,
    pub source: Option<String>,
    pub distribution: Option<String>,
    pub method: Option<String>,
    pub scale: Option<f64>,
    pub offset: Option<f64>,
    pub mean: Option<f64>,
    pub stddev: Option<f64>,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    pub p05: Option<f64>,
    pub p50: Option<f64>,
    pub p95: Option<f64>,
    pub sample_count: usize,
    pub propagation_count: usize,
    pub propagation: Vec<ReportUncertaintyPropagationTerm>,
    pub samples: Vec<f64>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeMlArtifact {
    pub binding: String,
    pub kind: String,
    pub source: Option<String>,
    pub target: Option<String>,
    pub features: Vec<String>,
    pub algorithm: Option<String>,
    pub test_fraction: Option<String>,
    pub seed: Option<String>,
    pub hidden_layers: Vec<usize>,
    pub epochs: Option<usize>,
    pub status: String,
    pub train_count: Option<usize>,
    pub test_count: Option<usize>,
    pub rmse: Option<f64>,
    pub mae: Option<f64>,
    pub r2: Option<f64>,
    pub leakage_status: Option<String>,
    pub leakage_findings: Vec<String>,
    pub coefficients: Vec<RuntimeMlCoefficient>,
    pub intercept: Option<f64>,
    pub loss_history: Vec<f64>,
    pub model_card: Option<String>,
    pub expression: String,
    pub display_unit: String,
    pub parity_points: Vec<RuntimePoint>,
    pub residual_points: Vec<RuntimePoint>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeMlCoefficient {
    pub feature: String,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct MlDataset {
    feature_names: Vec<String>,
    target_name: String,
    display_unit: String,
    rows: Vec<MlRow>,
}

#[derive(Clone, Debug, PartialEq)]
struct MlRow {
    features: Vec<f64>,
    target: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct MlTrainingResult {
    status: String,
    actual: Vec<f64>,
    predicted: Vec<f64>,
    coefficients: Vec<RuntimeMlCoefficient>,
    intercept: f64,
    loss_history: Vec<f64>,
    rmse: f64,
    mae: f64,
    r2: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct Standardization {
    means: Vec<f64>,
    scales: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSystemSolution {
    pub system: String,
    pub status: String,
    pub method: String,
    pub reason: String,
    pub state: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub time_unit: String,
    pub duration_s: f64,
    pub time_step_s: f64,
    pub step_count: usize,
    pub initial_value: f64,
    pub final_value: f64,
    pub canonical_initial_value: f64,
    pub canonical_final_value: f64,
    pub points: Vec<RuntimePoint>,
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
    pub distribution: Option<String>,
    pub model_plot: Option<ModelPlotOptions>,
    pub plot_type: Option<String>,
    pub title: Option<String>,
    pub y_unit: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelPlotOptions {
    pub kind: String,
    pub source: String,
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
    data.uncertainties = materialize_uncertainties(report);
    data.ml_artifacts = materialize_ml_artifacts(report, &data.time_series, &data.tables);
    data.system_solutions = materialize_system_solutions(report);
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

fn materialize_uncertainties(report: &CheckReport) -> Vec<RuntimeUncertainty> {
    let mut uncertainties = Vec::new();
    for info in &report.semantic_program.uncertainty_infos {
        let uncertainty = materialize_uncertainty(info, &uncertainties);
        uncertainties.push(uncertainty);
    }
    uncertainties
}

fn materialize_uncertainty(
    info: &eng_compiler::UncertaintyInfo,
    prior: &[RuntimeUncertainty],
) -> RuntimeUncertainty {
    let declared_mean = info.mean.as_deref().and_then(first_numeric_value);
    let declared_stddev = info.stddev.as_deref().and_then(first_numeric_value);
    let declared_lower = info.lower.as_deref().and_then(first_numeric_value);
    let declared_upper = info.upper.as_deref().and_then(first_numeric_value);
    let requested_count = info.sample_count.clamp(1, 256);
    let distribution = info
        .distribution
        .clone()
        .unwrap_or_else(|| info.kind.to_ascii_lowercase());
    let method = info.method.clone();
    let source = info.source.as_deref().and_then(|source| {
        prior
            .iter()
            .find(|uncertainty| uncertainty.binding == source)
    });
    let is_propagation = info
        .expression
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("propagate(");
    let scale = info
        .scale
        .as_deref()
        .and_then(first_numeric_value)
        .unwrap_or(1.0);
    let offset = info
        .offset
        .as_deref()
        .and_then(first_numeric_value)
        .unwrap_or(0.0);
    let source_missing = (info.kind == "Ensemble" || is_propagation) && source.is_none();

    let mut samples = match info.kind.as_str() {
        "Measured" => match (declared_mean, declared_stddev) {
            (Some(mean), Some(stddev)) => normal_samples(mean, stddev, requested_count.max(3)),
            (Some(mean), None) => vec![mean],
            _ => Vec::new(),
        },
        "Interval" => interval_samples(declared_lower, declared_upper),
        "Ensemble" if source_missing => Vec::new(),
        "Ensemble" => source
            .map(|source| resample_deterministic(&source.samples, requested_count))
            .unwrap_or_default(),
        "Distribution" if is_propagation && source_missing => Vec::new(),
        "Distribution" if is_propagation => source
            .map(|source| {
                resample_deterministic(&source.samples, requested_count)
                    .into_iter()
                    .map(|value| value * scale + offset)
                    .collect()
            })
            .unwrap_or_default(),
        "Distribution" if distribution == "uniform" => {
            uniform_samples(declared_lower, declared_upper, requested_count)
        }
        "Distribution" => normal_samples(
            declared_mean.unwrap_or(0.0),
            declared_stddev.unwrap_or(1.0),
            requested_count,
        ),
        _ => Vec::new(),
    };
    if samples.is_empty() {
        samples.push(declared_mean.unwrap_or(0.0));
    }
    let summary = sample_summary(&samples);
    let p05 = quantile(&samples, 0.05);
    let p50 = quantile(&samples, 0.50);
    let p95 = quantile(&samples, 0.95);

    RuntimeUncertainty {
        binding: info.binding.clone(),
        kind: info.kind.clone(),
        quantity_kind: info.quantity_kind.clone(),
        display_unit: info.display_unit.clone(),
        expression: info.expression.clone(),
        source: info.source.clone(),
        distribution: Some(distribution),
        method,
        scale: info.scale.as_ref().map(|_| scale),
        offset: info.offset.as_ref().map(|_| offset),
        mean: declared_mean.or(summary.mean),
        stddev: declared_stddev.or(summary.stddev),
        lower: declared_lower.or(summary.lower),
        upper: declared_upper.or(summary.upper),
        p05,
        p50,
        p95,
        sample_count: samples.len(),
        propagation_count: info.propagation.len(),
        propagation: info
            .propagation
            .iter()
            .map(|term| ReportUncertaintyPropagationTerm {
                source: term.source.clone(),
                role: term.role.clone(),
                quantity_kind: term.quantity_kind.clone(),
            })
            .collect(),
        samples,
        status: if source_missing {
            "source_unresolved".to_owned()
        } else if is_propagation {
            "propagated_linear".to_owned()
        } else if info.kind == "Measured" {
            "measured_sampled".to_owned()
        } else {
            "sampled_distribution".to_owned()
        },
        line: info.line,
    }
}

fn materialize_ml_artifacts(
    report: &CheckReport,
    series: &[RuntimeTimeSeries],
    tables: &[RuntimeTable],
) -> Vec<RuntimeMlArtifact> {
    let mut artifacts = Vec::new();
    for info in &report.semantic_program.ml_infos {
        let artifact = materialize_ml_artifact(info, &artifacts, series, tables);
        artifacts.push(artifact);
    }
    artifacts
}

fn materialize_ml_artifact(
    info: &eng_compiler::MlInfo,
    prior: &[RuntimeMlArtifact],
    series: &[RuntimeTimeSeries],
    tables: &[RuntimeTable],
) -> RuntimeMlArtifact {
    match info.kind.as_str() {
        "TrainTestSplit" => materialize_split_artifact(info, series, tables),
        "RegressionModel" | "MlpModel" => materialize_model_artifact(info, prior, series, tables),
        "ModelMetrics" => materialize_metrics_artifact(info, prior),
        "LeakageLint" => materialize_leakage_artifact(info, prior),
        "ModelCard" => materialize_model_card_artifact(info, prior),
        _ => base_ml_artifact(info, "metadata"),
    }
}

fn materialize_split_artifact(
    info: &eng_compiler::MlInfo,
    series: &[RuntimeTimeSeries],
    tables: &[RuntimeTable],
) -> RuntimeMlArtifact {
    let source_series = info
        .source
        .as_deref()
        .and_then(|source| series.iter().find(|series| series.name == source));
    let len = source_series.map(|series| series.points.len()).unwrap_or(0);
    let test_fraction = parse_fraction(info.test_fraction.as_deref()).unwrap_or(0.25);
    let test_count = if len > 1 {
        ((len as f64 * test_fraction).round() as usize).clamp(1, len - 1)
    } else {
        0
    };
    let train_count = len.saturating_sub(test_count);
    let mut artifact = base_ml_artifact(info, "prepared");
    artifact.train_count = Some(train_count);
    artifact.test_count = Some(test_count);
    artifact.leakage_findings = leakage_findings(info, source_series, tables);
    artifact.leakage_status = Some(leakage_status_from_findings(&artifact.leakage_findings));
    artifact.display_unit = source_series
        .map(|series| series.display_unit.clone())
        .unwrap_or_else(|| "1".to_owned());
    artifact
}

fn materialize_model_artifact(
    info: &eng_compiler::MlInfo,
    prior: &[RuntimeMlArtifact],
    series: &[RuntimeTimeSeries],
    tables: &[RuntimeTable],
) -> RuntimeMlArtifact {
    let split = info
        .source
        .as_deref()
        .and_then(|source| prior.iter().find(|artifact| artifact.binding == source));
    let source_name = split
        .and_then(|split| split.source.as_deref())
        .or(info.source.as_deref());
    let target = info
        .target
        .clone()
        .or_else(|| split.and_then(|split| split.target.clone()))
        .or_else(|| source_name.map(str::to_owned));
    let features = if info.features.is_empty() {
        split
            .map(|split| split.features.clone())
            .unwrap_or_default()
    } else {
        info.features.clone()
    };
    let source_series =
        source_name.and_then(|source| series.iter().find(|series| series.name == source));
    let mut artifact = base_ml_artifact(info, "unavailable");
    artifact.target = target.clone();
    artifact.features = features.clone();
    artifact.display_unit = source_series
        .map(|series| series.display_unit.clone())
        .unwrap_or_else(|| "1".to_owned());
    artifact.leakage_status = split.and_then(|split| split.leakage_status.clone());
    artifact.leakage_findings = split
        .map(|split| split.leakage_findings.clone())
        .unwrap_or_default();

    let Some(source_name) = source_name else {
        artifact.leakage_findings.push("missing_source".to_owned());
        artifact.leakage_status = Some(leakage_status_from_findings(&artifact.leakage_findings));
        return artifact;
    };
    let dataset = match ml_dataset(source_name, target.as_deref(), &features, series, tables) {
        Ok(dataset) => dataset,
        Err(mut findings) => {
            artifact.leakage_findings.append(&mut findings);
            artifact.leakage_status =
                Some(leakage_status_from_findings(&artifact.leakage_findings));
            return artifact;
        }
    };
    let total_count = dataset.rows.len();
    let requested_train_count = split
        .and_then(|split| split.train_count)
        .unwrap_or_else(|| {
            let test_fraction = parse_fraction(info.test_fraction.as_deref()).unwrap_or(0.25);
            let test_count = ((total_count as f64 * test_fraction).round() as usize)
                .clamp(1, total_count.saturating_sub(1).max(1));
            total_count.saturating_sub(test_count)
        });
    let train_count = requested_train_count
        .min(total_count.saturating_sub(1))
        .max((total_count > 1) as usize);
    let requested_test_count = split
        .and_then(|split| split.test_count)
        .unwrap_or_else(|| total_count.saturating_sub(train_count));
    let test_count = requested_test_count
        .min(total_count.saturating_sub(train_count))
        .max((total_count > train_count) as usize);
    let training = if info.kind == "MlpModel" {
        train_mlp_model(info, &dataset, train_count, test_count)
    } else {
        train_linear_model(info, &dataset, train_count, test_count)
    };
    artifact.status = training.status;
    artifact.train_count = Some(train_count);
    artifact.test_count = Some(training.actual.len());
    artifact.rmse = Some(training.rmse);
    artifact.mae = Some(training.mae);
    artifact.r2 = Some(training.r2);
    artifact.coefficients = training.coefficients;
    artifact.intercept = Some(training.intercept);
    artifact.loss_history = training.loss_history;
    artifact.parity_points = training
        .actual
        .iter()
        .zip(&training.predicted)
        .map(|(actual, predicted)| RuntimePoint {
            x: *actual,
            y: *predicted,
        })
        .collect();
    artifact.residual_points = training
        .actual
        .iter()
        .zip(&training.predicted)
        .enumerate()
        .map(|(index, (actual, predicted))| RuntimePoint {
            x: index as f64,
            y: actual - predicted,
        })
        .collect();
    artifact.model_card = Some(model_card_text(info, &artifact, &dataset));
    artifact
}

fn materialize_metrics_artifact(
    info: &eng_compiler::MlInfo,
    prior: &[RuntimeMlArtifact],
) -> RuntimeMlArtifact {
    let source = info
        .source
        .as_deref()
        .and_then(|source| prior.iter().find(|artifact| artifact.binding == source));
    let mut artifact = base_ml_artifact(info, "evaluated");
    if let Some(source) = source {
        artifact.target = source.target.clone();
        artifact.features = source.features.clone();
        artifact.algorithm = source.algorithm.clone();
        artifact.train_count = source.train_count;
        artifact.test_count = source.test_count;
        artifact.rmse = source.rmse;
        artifact.mae = source.mae;
        artifact.r2 = source.r2;
        artifact.leakage_status = source.leakage_status.clone();
        artifact.leakage_findings = source.leakage_findings.clone();
        artifact.coefficients = source.coefficients.clone();
        artifact.intercept = source.intercept;
        artifact.loss_history = source.loss_history.clone();
        artifact.model_card = source.model_card.clone();
        artifact.display_unit = source.display_unit.clone();
        artifact.parity_points = source.parity_points.clone();
        artifact.residual_points = source.residual_points.clone();
    }
    artifact
}

fn materialize_leakage_artifact(
    info: &eng_compiler::MlInfo,
    prior: &[RuntimeMlArtifact],
) -> RuntimeMlArtifact {
    let source = info
        .source
        .as_deref()
        .and_then(|source| prior.iter().find(|artifact| artifact.binding == source));
    let mut artifact = base_ml_artifact(info, "executed");
    artifact.leakage_status = Some(
        source
            .and_then(|source| source.leakage_status.clone())
            .unwrap_or_else(|| leakage_status_from_findings(&leakage_findings(info, None, &[]))),
    );
    artifact.leakage_findings = source
        .map(|source| source.leakage_findings.clone())
        .unwrap_or_else(|| leakage_findings(info, None, &[]));
    artifact
}

fn materialize_model_card_artifact(
    info: &eng_compiler::MlInfo,
    prior: &[RuntimeMlArtifact],
) -> RuntimeMlArtifact {
    let source = info
        .source
        .as_deref()
        .and_then(|source| prior.iter().find(|artifact| artifact.binding == source));
    let mut artifact = base_ml_artifact(info, "documented");
    if let Some(source) = source {
        artifact.model_card = source.model_card.clone().or_else(|| {
            Some(format!(
                "{} model card: status={}, train={}, test={}",
                source.binding,
                source.status,
                source.train_count.unwrap_or(0),
                source.test_count.unwrap_or(0)
            ))
        });
        artifact.rmse = source.rmse;
        artifact.mae = source.mae;
        artifact.r2 = source.r2;
        artifact.leakage_status = source.leakage_status.clone();
        artifact.leakage_findings = source.leakage_findings.clone();
        artifact.coefficients = source.coefficients.clone();
        artifact.intercept = source.intercept;
        artifact.loss_history = source.loss_history.clone();
    }
    artifact
}

fn base_ml_artifact(info: &eng_compiler::MlInfo, status: &str) -> RuntimeMlArtifact {
    RuntimeMlArtifact {
        binding: info.binding.clone(),
        kind: info.kind.clone(),
        source: info.source.clone(),
        target: info.target.clone(),
        features: info.features.clone(),
        algorithm: info.algorithm.clone(),
        test_fraction: info.test_fraction.clone(),
        seed: info.seed.clone(),
        hidden_layers: info.hidden_layers.clone(),
        epochs: info.epochs,
        status: status.to_owned(),
        train_count: None,
        test_count: None,
        rmse: None,
        mae: None,
        r2: None,
        leakage_status: None,
        leakage_findings: Vec::new(),
        coefficients: Vec::new(),
        intercept: None,
        loss_history: Vec::new(),
        model_card: None,
        expression: info.expression.clone(),
        display_unit: "1".to_owned(),
        parity_points: Vec::new(),
        residual_points: Vec::new(),
        line: info.line,
    }
}

fn materialize_system_solutions(report: &CheckReport) -> Vec<RuntimeSystemSolution> {
    report
        .semantic_program
        .systems
        .iter()
        .filter_map(materialize_first_order_thermal_solution)
        .collect()
}

fn materialize_first_order_thermal_solution(
    system: &eng_compiler::SystemInfo,
) -> Option<RuntimeSystemSolution> {
    let equation = system.equations.first()?;
    let state = system.variables.iter().find(|variable| {
        variable.role == "state" && variable.quantity_kind == "AbsoluteTemperature"
    })?;
    let heat_capacity = system.variables.iter().find(|variable| {
        variable.role == "parameter" && variable.quantity_kind == "HeatCapacity"
    })?;
    let conductance = system
        .variables
        .iter()
        .find(|variable| variable.role == "parameter" && variable.quantity_kind == "Conductance")?;
    let outdoor_temperature = system.variables.iter().find(|variable| {
        variable.role == "input" && variable.quantity_kind == "AbsoluteTemperature"
    })?;
    let internal_heat = system
        .variables
        .iter()
        .find(|variable| variable.role == "input" && variable.quantity_kind == "HeatRate")?;

    if !equation.left.contains(&heat_capacity.name)
        || !equation.left.contains(&format!("der({})", state.name))
        || !equation.right.contains(&conductance.name)
        || !equation.right.contains(&outdoor_temperature.name)
        || !equation.right.contains(&state.name)
        || !equation.right.contains(&internal_heat.name)
    {
        return None;
    }

    let heat_capacity_j_per_k = canonical_variable_value(heat_capacity)?;
    let conductance_w_per_k = canonical_variable_value(conductance)?;
    let outdoor_temperature_k = canonical_variable_value(outdoor_temperature)?;
    let internal_heat_w = canonical_variable_value(internal_heat)?;
    let initial_temperature_k = canonical_variable_value(state)?;

    if heat_capacity_j_per_k <= 0.0 || conductance_w_per_k < 0.0 {
        return None;
    }

    let duration_s = 3600.0;
    let time_step_s = 300.0;
    let step_count = (duration_s / time_step_s) as usize;
    let mut temperature_k = initial_temperature_k;
    let mut points = vec![RuntimePoint {
        x: 0.0,
        y: display_variable_value(temperature_k, state),
    }];

    for step in 1..=step_count {
        let derivative_k_per_s = (conductance_w_per_k * (outdoor_temperature_k - temperature_k)
            + internal_heat_w)
            / heat_capacity_j_per_k;
        temperature_k += derivative_k_per_s * time_step_s;
        points.push(RuntimePoint {
            x: step as f64 * time_step_s,
            y: display_variable_value(temperature_k, state),
        });
    }

    Some(RuntimeSystemSolution {
        system: system.name.clone(),
        status: "computed".to_owned(),
        method: "explicit_euler_fixed_step".to_owned(),
        reason: "recognized first-order thermal ODE and executed fixed-step preview".to_owned(),
        state: state.name.clone(),
        quantity_kind: state.quantity_kind.clone(),
        display_unit: state.display_unit.clone(),
        canonical_unit: state.canonical_unit.clone(),
        time_unit: "s".to_owned(),
        duration_s,
        time_step_s,
        step_count,
        initial_value: display_variable_value(initial_temperature_k, state),
        final_value: display_variable_value(temperature_k, state),
        canonical_initial_value: initial_temperature_k,
        canonical_final_value: temperature_k,
        points,
    })
}

fn canonical_variable_value(variable: &eng_compiler::SystemVariableInfo) -> Option<f64> {
    let expression = variable.initial_value.as_deref()?;
    let (value, unit) = number_with_optional_unit(expression)?;
    convert_to_canonical_unit(
        value,
        unit.as_deref(),
        &variable.canonical_unit,
        &variable.quantity_kind,
    )
    .ok()
}

fn display_variable_value(value: f64, variable: &eng_compiler::SystemVariableInfo) -> f64 {
    convert_from_canonical_unit(
        value,
        &variable.canonical_unit,
        &variable.display_unit,
        &variable.quantity_kind,
    )
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
    if let Some(distribution) = parse_distribution_header(header) {
        options.distribution = Some(distribution);
        options.plot_type = Some("histogram".to_owned());
    } else if let Some(model_plot) = parse_model_plot_header(header) {
        options.plot_type = Some(if model_plot.kind == "parity" {
            "scatter".to_owned()
        } else {
            "bar".to_owned()
        });
        options.model_plot = Some(model_plot);
    } else if let Some((series, axis)) = header.split_once(" over ") {
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

fn parse_model_plot_header(header: &str) -> Option<ModelPlotOptions> {
    parse_call_header(header, "parity")
        .map(|source| ModelPlotOptions {
            kind: "parity".to_owned(),
            source,
        })
        .or_else(|| {
            parse_call_header(header, "residuals").map(|source| ModelPlotOptions {
                kind: "residuals".to_owned(),
                source,
            })
        })
}

fn parse_distribution_header(header: &str) -> Option<String> {
    parse_call_header(header, "distribution")
}

fn parse_call_header(header: &str, name: &str) -> Option<String> {
    let header = header.trim();
    let prefix = format!("{name}(");
    let inner = header
        .strip_prefix(&prefix)
        .and_then(|rest| rest.strip_suffix(')'))?;
    inner
        .split(',')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn supported_plot_type(value: &str) -> Option<String> {
    let plot_type = value.split_whitespace().next()?;
    matches!(plot_type, "line" | "bar" | "histogram" | "scatter").then(|| plot_type.to_owned())
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
    let from_unit = normalize_unit(from_unit);
    let to_unit = normalize_unit(to_unit);
    match (from_unit.as_str(), to_unit.as_str()) {
        ("w", "kw") => value / 1000.0,
        ("kw", "w") => value * 1000.0,
        ("degc", "degc") => value,
        ("k", "degc") => value - 273.15,
        ("degc", "k") => value + 273.15,
        _ => value,
    }
}

fn first_numeric_value(text: &str) -> Option<f64> {
    number_with_optional_unit(text).map(|(value, _)| value)
}

fn interval_samples(lower: Option<f64>, upper: Option<f64>) -> Vec<f64> {
    match (lower, upper) {
        (Some(lower), Some(upper)) if (upper - lower).abs() > f64::EPSILON => {
            vec![lower, (lower + upper) * 0.5, upper]
        }
        (Some(value), _) | (_, Some(value)) => vec![value],
        _ => Vec::new(),
    }
}

fn normal_samples(mean: f64, stddev: f64, count: usize) -> Vec<f64> {
    let count = count.clamp(1, 256);
    if count == 1 || stddev == 0.0 {
        return vec![mean; count];
    }
    (0..count)
        .map(|index| {
            let probability = (index as f64 + 0.5) / count as f64;
            mean + inverse_standard_normal(probability) * stddev
        })
        .collect()
}

fn uniform_samples(lower: Option<f64>, upper: Option<f64>, count: usize) -> Vec<f64> {
    let Some(lower) = lower else {
        return Vec::new();
    };
    let Some(upper) = upper else {
        return Vec::new();
    };
    let count = count.clamp(1, 256);
    if count == 1 || (upper - lower).abs() <= f64::EPSILON {
        return vec![(lower + upper) * 0.5; count];
    }
    (0..count)
        .map(|index| {
            let fraction = (index as f64 + 0.5) / count as f64;
            lower + (upper - lower) * fraction
        })
        .collect()
}

fn resample_deterministic(values: &[f64], count: usize) -> Vec<f64> {
    if values.is_empty() {
        return Vec::new();
    }
    let count = count.clamp(1, 256);
    (0..count)
        .map(|index| values[index % values.len()])
        .collect()
}

fn quantile(values: &[f64], probability: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    if sorted.len() == 1 {
        return sorted.first().copied();
    }
    let position = probability.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower_index = position.floor() as usize;
    let upper_index = position.ceil() as usize;
    let lower = sorted[lower_index];
    let upper = sorted[upper_index];
    Some(lower + (upper - lower) * (position - lower_index as f64))
}

fn inverse_standard_normal(probability: f64) -> f64 {
    let p = probability.clamp(1.0e-12, 1.0 - 1.0e-12);
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    let plow = 0.02425;
    let phigh = 1.0 - plow;
    if p < plow {
        let q = (-2.0 * p.ln()).sqrt();
        return (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0);
    }
    if p <= phigh {
        let q = p - 0.5;
        let r = q * q;
        return (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0);
    }
    let q = (-2.0 * (1.0 - p).ln()).sqrt();
    -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
        / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
}

#[derive(Clone, Copy, Debug)]
struct SampleSummary {
    mean: Option<f64>,
    stddev: Option<f64>,
    lower: Option<f64>,
    upper: Option<f64>,
}

fn sample_summary(values: &[f64]) -> SampleSummary {
    if values.is_empty() {
        return SampleSummary {
            mean: None,
            stddev: None,
            lower: None,
            upper: None,
        };
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
    let lower = values.iter().copied().fold(f64::INFINITY, f64::min);
    let upper = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    SampleSummary {
        mean: Some(mean),
        stddev: Some(variance.sqrt()),
        lower: Some(lower),
        upper: Some(upper),
    }
}

fn histogram_bins(values: &[f64]) -> Vec<PlotBin> {
    if values.is_empty() {
        return Vec::new();
    }
    let lower = values.iter().copied().fold(f64::INFINITY, f64::min);
    let upper = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if (upper - lower).abs() <= f64::EPSILON {
        return vec![PlotBin {
            lower,
            upper,
            center: lower,
            count: values.len(),
        }];
    }
    let bin_count = values.len().clamp(3, 12);
    let width = (upper - lower) / bin_count as f64;
    let mut bins = vec![0usize; bin_count];
    for value in values {
        let mut index = ((value - lower) / width).floor() as usize;
        if index >= bin_count {
            index = bin_count - 1;
        }
        bins[index] += 1;
    }
    bins.into_iter()
        .enumerate()
        .map(|(index, count)| {
            let bin_lower = lower + index as f64 * width;
            let bin_upper = if index + 1 == bin_count {
                upper
            } else {
                bin_lower + width
            };
            PlotBin {
                lower: bin_lower,
                upper: bin_upper,
                center: bin_lower + (bin_upper - bin_lower) * 0.5,
                count,
            }
        })
        .collect()
}

fn histogram_points_from_bins(bins: &[PlotBin]) -> Vec<PlotPoint> {
    bins.iter()
        .map(|bin| PlotPoint {
            x: bin.center,
            y: bin.count as f64,
        })
        .collect()
}

fn format_number(value: f64) -> String {
    let mut text = format!("{value:.6}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn parse_fraction(value: Option<&str>) -> Option<f64> {
    let value = value?.trim().trim_end_matches('%');
    let parsed = value.parse::<f64>().ok()?;
    if parsed > 1.0 {
        Some((parsed / 100.0).clamp(0.05, 0.95))
    } else {
        Some(parsed.clamp(0.05, 0.95))
    }
}

fn leakage_findings(
    info: &eng_compiler::MlInfo,
    source_series: Option<&RuntimeTimeSeries>,
    tables: &[RuntimeTable],
) -> Vec<String> {
    let mut findings = Vec::new();
    if let Some(target) = info.target.as_deref() {
        if info.features.iter().any(|feature| feature == target) {
            findings.push(format!("target_in_features:{target}"));
        }
    } else if info.kind == "TrainTestSplit" {
        findings.push("missing_target".to_owned());
    }
    if info.features.is_empty() && info.kind == "TrainTestSplit" {
        findings.push("missing_features".to_owned());
    }
    let Some(source_series) = source_series else {
        if info.kind == "TrainTestSplit" {
            findings.push("missing_source_series".to_owned());
        }
        findings.sort();
        findings.dedup();
        return findings;
    };
    let Some(table) = tables
        .iter()
        .find(|table| table.binding == source_series.source_table)
    else {
        findings.push(format!(
            "missing_source_table:{}",
            source_series.source_table
        ));
        findings.sort();
        findings.dedup();
        return findings;
    };
    for feature in &info.features {
        match table.column(feature) {
            Some(column) if !matches!(&column.values, RuntimeValues::Number(_)) => {
                findings.push(format!("non_numeric_feature:{feature}"));
            }
            Some(column) if column.is_index => {
                findings.push(format!("index_feature_requires_temporal_review:{feature}"));
            }
            Some(_) => {}
            None => findings.push(format!("missing_feature:{feature}")),
        }
    }
    findings.sort();
    findings.dedup();
    findings
}

fn leakage_status_from_findings(findings: &[String]) -> String {
    if findings.is_empty() {
        return "passed".to_owned();
    }
    if findings
        .iter()
        .any(|finding| finding.starts_with("target_in_features:"))
    {
        return "failed_target_in_features".to_owned();
    }
    format!("failed_{}_findings", findings.len())
}

fn ml_dataset(
    source_name: &str,
    target_name: Option<&str>,
    features: &[String],
    series: &[RuntimeTimeSeries],
    tables: &[RuntimeTable],
) -> Result<MlDataset, Vec<String>> {
    let source_series = series
        .iter()
        .find(|series| series.name == source_name)
        .ok_or_else(|| vec![format!("missing_source_series:{source_name}")])?;
    let target_name = target_name.unwrap_or(source_name);
    let target_series = series
        .iter()
        .find(|series| series.name == target_name)
        .unwrap_or(source_series);
    let table = tables
        .iter()
        .find(|table| table.binding == source_series.source_table)
        .ok_or_else(|| {
            vec![format!(
                "missing_source_table:{}",
                source_series.source_table
            )]
        })?;
    if features.is_empty() {
        return Err(vec!["missing_features".to_owned()]);
    }
    let mut feature_columns = Vec::new();
    let mut findings = Vec::new();
    for feature in features {
        match table.column(feature) {
            Some(column) if matches!(&column.values, RuntimeValues::Number(_)) => {
                feature_columns.push(column);
            }
            Some(_) => findings.push(format!("non_numeric_feature:{feature}")),
            None => findings.push(format!("missing_feature:{feature}")),
        }
    }
    if !findings.is_empty() {
        return Err(findings);
    }
    let row_count = table
        .row_count
        .min(source_series.points.len())
        .min(target_series.points.len());
    let mut rows = Vec::new();
    for index in 0..row_count {
        let mut feature_values = Vec::with_capacity(feature_columns.len());
        let mut complete = true;
        for column in &feature_columns {
            match numeric_column_value(column, index) {
                Some(value) => feature_values.push(value),
                None => {
                    complete = false;
                    break;
                }
            }
        }
        if complete {
            rows.push(MlRow {
                features: feature_values,
                target: target_series.points[index].y,
            });
        }
    }
    if rows.len() < 2 {
        return Err(vec![format!("insufficient_complete_rows:{}", rows.len())]);
    }
    Ok(MlDataset {
        feature_names: features.to_vec(),
        target_name: target_name.to_owned(),
        display_unit: target_series.display_unit.clone(),
        rows,
    })
}

fn numeric_column_value(column: &RuntimeColumn, index: usize) -> Option<f64> {
    column
        .canonical_values
        .get(index)
        .copied()
        .flatten()
        .or_else(|| optional_number_at(column, index))
}

fn train_linear_model(
    info: &eng_compiler::MlInfo,
    dataset: &MlDataset,
    train_count: usize,
    test_count: usize,
) -> MlTrainingResult {
    let (train_rows, test_rows) = split_ml_rows(dataset, train_count, test_count);
    let feature_count = dataset.feature_names.len();
    let standardization = standardization(train_rows, feature_count);
    let mut coefficients = vec![0.0; feature_count];
    let mut intercept = mean_target(train_rows);
    let epochs = info.epochs.unwrap_or(320).max(1);
    let learning_rate = 0.08 / (feature_count.max(1) as f64).sqrt();
    let checkpoint = (epochs / 5).max(1);
    let mut loss_history = Vec::new();

    for epoch in 0..epochs {
        let mut intercept_gradient = 0.0;
        let mut coefficient_gradients = vec![0.0; feature_count];
        for row in train_rows {
            let features = standardized_features(row, &standardization);
            let predicted = intercept + dot(&coefficients, &features);
            let error = predicted - row.target;
            intercept_gradient += error;
            for index in 0..feature_count {
                coefficient_gradients[index] += error * features[index];
            }
        }
        let scale = train_rows.len().max(1) as f64;
        intercept -= learning_rate * intercept_gradient / scale;
        for index in 0..feature_count {
            coefficients[index] -= learning_rate * coefficient_gradients[index] / scale;
        }
        if epoch == 0 || (epoch + 1) % checkpoint == 0 || epoch + 1 == epochs {
            loss_history.push(linear_rmse(
                train_rows,
                &standardization,
                &coefficients,
                intercept,
            ));
        }
    }

    let predicted = test_rows
        .iter()
        .map(|row| {
            let features = standardized_features(row, &standardization);
            intercept + dot(&coefficients, &features)
        })
        .collect::<Vec<_>>();
    let actual = test_rows.iter().map(|row| row.target).collect::<Vec<_>>();
    let (rmse, mae, r2) = regression_metrics(&actual, &predicted);
    let original_intercept = intercept
        - coefficients
            .iter()
            .enumerate()
            .map(|(index, coefficient)| {
                coefficient * standardization.means[index] / standardization.scales[index]
            })
            .sum::<f64>();
    let original_coefficients = coefficients
        .iter()
        .enumerate()
        .map(|(index, coefficient)| RuntimeMlCoefficient {
            feature: dataset.feature_names[index].clone(),
            value: coefficient / standardization.scales[index],
        })
        .collect();

    MlTrainingResult {
        status: "trained_linear".to_owned(),
        actual,
        predicted,
        coefficients: original_coefficients,
        intercept: original_intercept,
        loss_history,
        rmse,
        mae,
        r2,
    }
}

fn train_mlp_model(
    info: &eng_compiler::MlInfo,
    dataset: &MlDataset,
    train_count: usize,
    test_count: usize,
) -> MlTrainingResult {
    let (train_rows, test_rows) = split_ml_rows(dataset, train_count, test_count);
    let feature_count = dataset.feature_names.len();
    let hidden_count = info
        .hidden_layers
        .first()
        .copied()
        .unwrap_or(4)
        .clamp(1, 16);
    let standardization = standardization(train_rows, feature_count);
    let (target_mean, target_scale) = target_mean_scale(train_rows);
    let seed = info
        .seed
        .as_deref()
        .and_then(|seed| seed.parse::<u64>().ok())
        .unwrap_or(1);
    let mut hidden_weights = (0..hidden_count)
        .map(|hidden| {
            (0..feature_count)
                .map(|feature| deterministic_weight(seed, hidden, feature))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut hidden_bias = vec![0.0; hidden_count];
    let mut output_weights = (0..hidden_count)
        .map(|hidden| deterministic_weight(seed.wrapping_add(17), hidden, 0))
        .collect::<Vec<_>>();
    let mut output_bias = 0.0;
    let epochs = info.epochs.unwrap_or(240).max(1);
    let learning_rate = 0.03;
    let checkpoint = (epochs / 5).max(1);
    let mut loss_history = Vec::new();

    for epoch in 0..epochs {
        for row in train_rows {
            let features = standardized_features(row, &standardization);
            let hidden = mlp_hidden(&features, &hidden_weights, &hidden_bias);
            let predicted = output_bias + dot(&output_weights, &hidden);
            let expected = (row.target - target_mean) / target_scale;
            let error = predicted - expected;
            let previous_output_weights = output_weights.clone();
            output_bias -= learning_rate * error;
            for hidden_index in 0..hidden_count {
                output_weights[hidden_index] -= learning_rate * error * hidden[hidden_index];
            }
            for hidden_index in 0..hidden_count {
                let hidden_delta = error
                    * previous_output_weights[hidden_index]
                    * (1.0 - hidden[hidden_index] * hidden[hidden_index]);
                hidden_bias[hidden_index] -= learning_rate * hidden_delta;
                for feature_index in 0..feature_count {
                    hidden_weights[hidden_index][feature_index] -=
                        learning_rate * hidden_delta * features[feature_index];
                }
            }
        }
        if epoch == 0 || (epoch + 1) % checkpoint == 0 || epoch + 1 == epochs {
            loss_history.push(mlp_rmse(
                train_rows,
                &standardization,
                &hidden_weights,
                &hidden_bias,
                &output_weights,
                output_bias,
            ));
        }
    }

    let predicted = test_rows
        .iter()
        .map(|row| {
            let features = standardized_features(row, &standardization);
            let normalized = output_bias
                + dot(
                    &output_weights,
                    &mlp_hidden(&features, &hidden_weights, &hidden_bias),
                );
            target_mean + normalized * target_scale
        })
        .collect::<Vec<_>>();
    let actual = test_rows.iter().map(|row| row.target).collect::<Vec<_>>();
    let (rmse, mae, r2) = regression_metrics(&actual, &predicted);
    let coefficients = (0..feature_count)
        .map(|feature_index| RuntimeMlCoefficient {
            feature: dataset.feature_names[feature_index].clone(),
            value: output_weights
                .iter()
                .enumerate()
                .map(|(hidden_index, output_weight)| {
                    hidden_weights[hidden_index][feature_index] * output_weight * target_scale
                        / standardization.scales[feature_index]
                })
                .sum(),
        })
        .collect::<Vec<_>>();
    let intercept = target_mean + output_bias * target_scale;

    MlTrainingResult {
        status: "trained_mlp".to_owned(),
        actual,
        predicted,
        coefficients,
        intercept,
        loss_history,
        rmse,
        mae,
        r2,
    }
}

fn split_ml_rows(
    dataset: &MlDataset,
    train_count: usize,
    test_count: usize,
) -> (&[MlRow], &[MlRow]) {
    let train_end = train_count.min(dataset.rows.len().saturating_sub(1));
    let test_end = train_end.saturating_add(test_count).min(dataset.rows.len());
    let train_rows = &dataset.rows[..train_end.max(1)];
    let test_rows = if test_end > train_end {
        &dataset.rows[train_end..test_end]
    } else {
        train_rows
    };
    (train_rows, test_rows)
}

fn standardization(rows: &[MlRow], feature_count: usize) -> Standardization {
    let mut means = vec![0.0; feature_count];
    for row in rows {
        for (index, value) in row.features.iter().enumerate() {
            means[index] += value;
        }
    }
    let row_count = rows.len().max(1) as f64;
    for mean in &mut means {
        *mean /= row_count;
    }
    let mut scales = vec![0.0; feature_count];
    for row in rows {
        for (index, value) in row.features.iter().enumerate() {
            let delta = value - means[index];
            scales[index] += delta * delta;
        }
    }
    for scale in &mut scales {
        *scale = (*scale / row_count).sqrt();
        if *scale <= f64::EPSILON {
            *scale = 1.0;
        }
    }
    Standardization { means, scales }
}

fn standardized_features(row: &MlRow, standardization: &Standardization) -> Vec<f64> {
    row.features
        .iter()
        .enumerate()
        .map(|(index, value)| {
            (value - standardization.means[index]) / standardization.scales[index]
        })
        .collect()
}

fn mean_target(rows: &[MlRow]) -> f64 {
    rows.iter().map(|row| row.target).sum::<f64>() / rows.len().max(1) as f64
}

fn target_mean_scale(rows: &[MlRow]) -> (f64, f64) {
    let mean = mean_target(rows);
    let variance = rows
        .iter()
        .map(|row| {
            let delta = row.target - mean;
            delta * delta
        })
        .sum::<f64>()
        / rows.len().max(1) as f64;
    let scale = variance.sqrt();
    (mean, if scale <= f64::EPSILON { 1.0 } else { scale })
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn linear_rmse(
    rows: &[MlRow],
    standardization: &Standardization,
    coefficients: &[f64],
    intercept: f64,
) -> f64 {
    let actual = rows.iter().map(|row| row.target).collect::<Vec<_>>();
    let predicted = rows
        .iter()
        .map(|row| intercept + dot(coefficients, &standardized_features(row, standardization)))
        .collect::<Vec<_>>();
    regression_metrics(&actual, &predicted).0
}

fn mlp_hidden(features: &[f64], weights: &[Vec<f64>], bias: &[f64]) -> Vec<f64> {
    weights
        .iter()
        .enumerate()
        .map(|(index, row)| (bias[index] + dot(row, features)).tanh())
        .collect()
}

fn mlp_rmse(
    rows: &[MlRow],
    standardization: &Standardization,
    hidden_weights: &[Vec<f64>],
    hidden_bias: &[f64],
    output_weights: &[f64],
    output_bias: f64,
) -> f64 {
    if rows.is_empty() {
        return 0.0;
    }
    let actual = rows
        .iter()
        .map(|row| {
            let (mean, scale) = target_mean_scale(rows);
            (row.target - mean) / scale
        })
        .collect::<Vec<_>>();
    let predicted = rows
        .iter()
        .map(|row| {
            let features = standardized_features(row, standardization);
            output_bias
                + dot(
                    output_weights,
                    &mlp_hidden(&features, hidden_weights, hidden_bias),
                )
        })
        .collect::<Vec<_>>();
    regression_metrics(&actual, &predicted).0
}

fn deterministic_weight(seed: u64, row: usize, column: usize) -> f64 {
    let mut value = seed
        .wrapping_add((row as u64 + 1).wrapping_mul(1_103_515_245))
        .wrapping_add((column as u64 + 1).wrapping_mul(12_345));
    value ^= value >> 13;
    value = value.wrapping_mul(0xff51afd7ed558ccd);
    let unit = (value % 10_000) as f64 / 10_000.0;
    (unit - 0.5) * 0.4
}

fn regression_metrics(actual: &[f64], predicted: &[f64]) -> (f64, f64, f64) {
    if actual.is_empty() || predicted.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let count = actual.len().min(predicted.len());
    let actual = &actual[..count];
    let predicted = &predicted[..count];
    let mean_actual = actual.iter().sum::<f64>() / count as f64;
    let mut squared_error = 0.0;
    let mut absolute_error = 0.0;
    let mut total_sum_squares = 0.0;
    for (actual, predicted) in actual.iter().zip(predicted) {
        let error = actual - predicted;
        squared_error += error * error;
        absolute_error += error.abs();
        let centered = actual - mean_actual;
        total_sum_squares += centered * centered;
    }
    let rmse = (squared_error / count as f64).sqrt();
    let mae = absolute_error / count as f64;
    let r2 = if total_sum_squares <= f64::EPSILON {
        1.0
    } else {
        1.0 - squared_error / total_sum_squares
    };
    (rmse, mae, r2)
}

fn model_card_text(
    info: &eng_compiler::MlInfo,
    artifact: &RuntimeMlArtifact,
    dataset: &MlDataset,
) -> String {
    let coefficient_summary = artifact
        .coefficients
        .iter()
        .take(4)
        .map(|coefficient| {
            format!(
                "{}={}",
                coefficient.feature,
                format_number(coefficient.value)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let loss_summary = match (artifact.loss_history.first(), artifact.loss_history.last()) {
        (Some(first), Some(last)) => {
            format!("loss {} -> {}", format_number(*first), format_number(*last))
        }
        _ => "loss unavailable".to_owned(),
    };
    format!(
        "{} {}: target={}, features=[{}], rows={}, train={}, test={}, rmse={} {}, mae={} {}, r2={}, {}, coefficients=[{}]",
        info.binding,
        info.algorithm.as_deref().unwrap_or(info.kind.as_str()),
        dataset.target_name,
        dataset.feature_names.join(", "),
        dataset.rows.len(),
        artifact.train_count.unwrap_or(0),
        artifact.test_count.unwrap_or(0),
        format_number(artifact.rmse.unwrap_or(0.0)),
        dataset.display_unit,
        format_number(artifact.mae.unwrap_or(0.0)),
        dataset.display_unit,
        format_number(artifact.r2.unwrap_or(0.0)),
        loss_summary,
        coefficient_summary
    )
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

    let normalized_source_unit = normalize_unit(source_unit);
    let normalized_target_unit = normalize_unit(target_unit);

    if normalized_source_unit == normalized_target_unit {
        return Ok(value);
    }

    let Some(info) = all_unit_infos()
        .iter()
        .find(|info| normalize_unit(info.symbol) == normalized_source_unit)
    else {
        return Err(format!(
            "unsupported source unit `{source_unit}` for {quantity_kind}"
        ));
    };

    if normalize_unit(info.canonical_unit) != normalized_target_unit
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

fn convert_from_canonical_unit(
    value: f64,
    canonical_unit: &str,
    display_unit: &str,
    quantity_kind: &str,
) -> f64 {
    let normalized_canonical_unit = normalize_unit(canonical_unit);
    let normalized_display_unit = normalize_unit(display_unit);
    if normalized_canonical_unit == normalized_display_unit {
        return value;
    }

    match (
        normalized_canonical_unit.as_str(),
        normalized_display_unit.as_str(),
        quantity_kind,
    ) {
        ("k", "degc", "AbsoluteTemperature") => value - 273.15,
        ("w", "kw", "HeatRate" | "ElectricPower" | "MechanicalPower") => value / 1000.0,
        ("j/k", "kj/k", "HeatCapacity") => value / 1000.0,
        _ => value,
    }
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
    fn parses_distribution_plot_options() {
        let options = parse_plot_options(
            r#"
script main(args: Args) -> Report {
    return report {
        plot distribution(Q_coil_dist) {
            title = "Coil uncertainty"
        }
    }
}
"#,
        );

        assert_eq!(options.distribution.as_deref(), Some("Q_coil_dist"));
        assert_eq!(options.plot_type.as_deref(), Some("histogram"));
        assert_eq!(options.title.as_deref(), Some("Coil uncertainty"));
    }

    #[test]
    fn parses_model_plot_options() {
        let options = parse_plot_options(
            r#"
script main(args: Args) -> Report {
    return report {
        plot parity(reg_eval) {
            title = "Regression parity"
        }
    }
}
"#,
        );

        let model_plot = options.model_plot.as_ref().unwrap();
        assert_eq!(model_plot.kind, "parity");
        assert_eq!(model_plot.source, "reg_eval");
        assert_eq!(options.plot_type.as_deref(), Some("scatter"));
        assert_eq!(options.title.as_deref(), Some("Regression parity"));
    }

    #[test]
    fn celsius_symbol_alias_converts_like_degc() {
        assert_eq!(
            round2(
                convert_to_canonical_unit(24.0, Some("°C"), "K", "AbsoluteTemperature").unwrap()
            ),
            297.15
        );
        assert_eq!(
            round2(convert_from_canonical_unit(
                297.15,
                "K",
                "°C",
                "AbsoluteTemperature"
            )),
            24.0
        );
        assert_eq!(convert_display_value(24.0, "°C", "degC"), 24.0);
    }

    #[test]
    fn materializes_uncertainty_samples_and_histogram_plot() {
        let source = r#"
script main(args: Args) -> Report {
    Q_coil_dist = normal(mean=5 kW, std=0.8 kW, samples=31)
    Q_unc = propagate(Q_coil_dist, method=linear, scale=1.1, offset=0.2 kW)

    return report {
        plot distribution(Q_coil_dist) {
            title = "Coil uncertainty"
        }
    }
}
"#;
        let report = eng_compiler::check_source("ok.eng", source, &CheckOptions::default());
        let runtime = materialize_runtime_data(&report, source);
        let mut plot_spec = eng_report::plot_spec_from_report(&report);
        runtime.apply_plot_spec(&report, &mut plot_spec);

        assert_eq!(runtime.uncertainties.len(), 2);
        assert_eq!(runtime.uncertainties[0].sample_count, 31);
        assert_eq!(
            runtime.uncertainties[0].distribution.as_deref(),
            Some("normal")
        );
        assert_eq!(runtime.uncertainties[1].status, "propagated_linear");
        assert_eq!(runtime.uncertainties[1].method.as_deref(), Some("linear"));
        assert_eq!(runtime.uncertainties[1].scale, Some(1.1));
        assert_eq!(runtime.uncertainties[1].offset, Some(0.2));
        assert_eq!(runtime.uncertainties[1].propagation.len(), 1);
        assert_eq!(
            runtime.uncertainties[1].propagation[0].source,
            "Q_coil_dist"
        );
        assert!(runtime.uncertainties[0].p05.is_some());
        assert!(runtime.uncertainties[1].mean.unwrap() > runtime.uncertainties[0].mean.unwrap());
        assert_eq!(round2(runtime.uncertainties[0].mean.unwrap()), 5.0);
        assert_eq!(plot_spec.plot_type, "histogram");
        assert_eq!(plot_spec.title, "Coil uncertainty");
        assert!(!plot_spec.series[0].points.is_empty());
        assert_eq!(
            plot_spec.series[0]
                .bins
                .iter()
                .map(|bin| bin.count)
                .sum::<usize>(),
            runtime.uncertainties[0].sample_count
        );
        assert_eq!(
            plot_spec.series[0].points.len(),
            plot_spec.series[0].bins.len()
        );
        let plot_json = eng_report::plot_spec_json(&plot_spec);
        let plot_svg = eng_report::render_svg_from_spec(&plot_spec);
        assert!(plot_json.contains("\"bins\""));
        assert!(plot_json.contains("\"lower\""));
        assert!(plot_svg.contains("data-bin-lower"));
    }

    #[test]
    fn marks_unresolved_uncertainty_source_when_materialized() {
        let source = r#"
script main(args: Args) -> Report {
    Q_unc = propagate(Q_missing, method=linear, samples=8)
}
"#;
        let report = eng_compiler::check_source("bad.eng", source, &CheckOptions::default());
        let runtime = materialize_runtime_data(&report, source);

        assert!(report.has_errors());
        assert_eq!(runtime.uncertainties.len(), 1);
        assert_eq!(runtime.uncertainties[0].status, "source_unresolved");
        assert_eq!(runtime.uncertainties[0].sample_count, 1);
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

    #[test]
    fn materializes_ml_metrics_and_parity_plot() {
        let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/official/05_data_driven_modeling/main.eng");
        let source = std::fs::read_to_string(&source_path).unwrap();
        let report = check_file(&source_path, &CheckOptions::default()).unwrap();
        let runtime = materialize_runtime_data(&report, &source);
        let mut plot_spec = eng_report::plot_spec_from_report(&report);
        runtime.apply_plot_spec(&report, &mut plot_spec);

        let regression = runtime
            .ml_artifacts
            .iter()
            .find(|artifact| artifact.kind == "RegressionModel")
            .unwrap();
        let mlp = runtime
            .ml_artifacts
            .iter()
            .find(|artifact| artifact.kind == "MlpModel")
            .unwrap();
        assert_eq!(regression.status, "trained_linear");
        assert_eq!(mlp.status, "trained_mlp");
        assert_eq!(regression.coefficients.len(), 3);
        assert_eq!(mlp.coefficients.len(), 3);
        assert!(regression.intercept.is_some());
        assert!(mlp.intercept.is_some());
        assert!(regression.loss_history.len() >= 2);
        assert!(mlp.loss_history.len() >= 2);
        assert!(regression.rmse.unwrap() > 0.0);
        assert!(mlp.rmse.unwrap() > 0.0);
        assert!(regression
            .model_card
            .as_deref()
            .unwrap()
            .contains("coefficients=["));
        assert!(runtime
            .ml_artifacts
            .iter()
            .any(|artifact| artifact.leakage_status.as_deref() == Some("passed")));
        assert_eq!(plot_spec.plot_type, "scatter");
        assert_eq!(plot_spec.title, "Regression parity");
        assert!(!plot_spec.series[0].points.is_empty());
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
