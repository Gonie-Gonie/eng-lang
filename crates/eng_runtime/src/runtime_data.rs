use std::collections::{HashMap, HashSet};
use std::fs;

use eng_compiler::{
    all_quantity_completions, all_unit_infos, normalize_unit, CheckReport, SchemaColumn, SchemaInfo,
};
use eng_report::{
    PlotAxis, PlotBin, PlotPoint, PlotSeries, PlotSpec, ReportComponentSolverResidual,
    ReportComponentSolverResult, ReportComponentSolverStepDiagnostic,
    ReportComponentSolverTrajectory, ReportComponentSolverVariable, ReportComputedIntegration,
    ReportComputedMetric, ReportComputedStatisticValue, ReportComputedStatistics,
    ReportMlCoefficient, ReportMlInfo, ReportPolicyResult, ReportPolicyViolation,
    ReportSolverFailureArtifact, ReportSpec, ReportSystemSolution, ReportSystemSolutionPoint,
    ReportSystemSolverStepDiagnostic, ReportTimeAlignment, ReportTimeAxis, ReportUncertaintyInfo,
    ReportUncertaintyPropagationTerm, ReportValidationResult,
};

use crate::solver::expression::evaluate_source_arithmetic_expression;
use crate::solver::{
    assembly::{
        ComponentEquation, ComponentInstance, ConnectionEdge, ConnectionSet, EquationAssembly,
        GeneratedEquation, PortInstance, UnknownVariable,
    },
    initialize_algebraic_variables, solve_adaptive_heun_ode, solve_adaptive_state_space,
    solve_continuous_state_space, solve_discrete_state_space, solve_dynamic_component_assembly,
    solve_first_order_thermal, solve_fixed_point, solve_fixed_step_ode, solve_implicit_euler_dae,
    solve_linear_residual_graph, solve_newton, solve_newton_with_jacobian, AdaptiveOdeOptions,
    AdaptiveOdeStepReport, AlgebraicInitializationInput, BehaviorExecutionProfile,
    BehaviorGraphRhsAdapter, BehaviorRhsNode, BehaviorRhsSample, BehaviorSignalContract,
    BehaviorSignalSource, DaeInput, DaeOptions, DaeSample, DaeVariable, DelayBehaviorNode,
    DelayBuffer, DelayInitialHistoryPolicy, DelayInterpolationPolicy,
    DynamicComponentAssemblySolveInput, DynamicComponentOptions, DynamicComponentResult,
    ExternalBehaviorContract, ExternalBehaviorDeterminism, ExternalBehaviorKind,
    ExternalBehaviorProfilePolicy, FirstOrderThermalModel, FixedPointOptions, FixedStepMethod,
    InputLayout, LayoutEntry, NewtonOptions, OutputLayout, ParameterLayout, PredictorContract,
    PredictorDifferentiability, PredictorJacobianPolicy, PredictorSolverPolicy, ResidualEvaluator,
    ResidualGraph, ResidualInput, ResidualOutput, RhsEvaluator, RhsInput, RhsStateInfo,
    SimulationPlan, SolverDiagnostics, SolverFailure, SolverInput, SolverOptions, SolverPlan,
    SolverResult, SolverScalar, SourceRhsEquation, SourceRhsEvaluator, StateLayout,
    StateTrajectory, TimeGrid,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RuntimeData {
    pub tables: Vec<RuntimeTable>,
    pub time_axes: Vec<RuntimeTimeAxis>,
    pub time_series: Vec<RuntimeTimeSeries>,
    pub statistics: Vec<RuntimeStatistics>,
    pub integrations: Vec<RuntimeIntegration>,
    pub uncertainties: Vec<RuntimeUncertainty>,
    pub ml_artifacts: Vec<RuntimeMlArtifact>,
    pub policy_results: Vec<RuntimePolicyResult>,
    pub system_solutions: Vec<RuntimeSystemSolution>,
    pub component_solutions: Vec<RuntimeComponentSolution>,
    pub metrics: Vec<RuntimeMetric>,
    pub validations: Vec<RuntimeValidation>,
    pub time_alignments: Vec<RuntimeTimeAlignment>,
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
            .histogram
            .as_deref()
            .or(self.plot_options.series.as_deref())
            .or_else(|| spec.series.first().map(|series| series.name.as_str()));
        let series = requested_series
            .and_then(|name| self.time_series.iter().find(|series| series.name == name))
            .or_else(|| self.time_series.first());

        let Some(series) = series else {
            return;
        };

        let histogram_requested = self.plot_options.histogram.is_some()
            || self.plot_options.plot_type.as_deref() == Some("histogram");
        if histogram_requested {
            self.apply_time_series_histogram(series, spec);
            return;
        }

        let selected_series = if self.plot_options.series_list.len() > 1 {
            self.plot_options
                .series_list
                .iter()
                .filter_map(|name| self.time_series.iter().find(|series| series.name == *name))
                .collect::<Vec<_>>()
        } else {
            vec![series]
        };
        if selected_series.is_empty() {
            return;
        }

        let display_unit = self
            .plot_options
            .y_unit
            .clone()
            .unwrap_or_else(|| series.display_unit.clone());

        let title = self.plot_options.title.clone().unwrap_or_else(|| {
            format!(
                "{} over {}",
                selected_series
                    .iter()
                    .map(|series| series.name.as_str())
                    .collect::<Vec<_>>()
                    .join(" and "),
                series.axis
            )
        });

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
        spec.series = selected_series
            .into_iter()
            .map(|series| {
                let unit = self
                    .plot_options
                    .y_unit
                    .clone()
                    .unwrap_or_else(|| series.display_unit.clone());
                let points = series
                    .points
                    .iter()
                    .map(|point| PlotPoint {
                        x: point.x,
                        y: convert_display_value(point.y, &series.display_unit, &unit),
                    })
                    .collect();
                PlotSeries {
                    name: series.name.clone(),
                    quantity_kind: series.quantity_kind.clone(),
                    display_unit: unit,
                    bins: Vec::new(),
                    points,
                }
            })
            .collect();

        if spec.series.is_empty() && !report.semantic_program.typed_bindings.is_empty() {
            *spec = eng_report::plot_spec_from_report(report);
        }
    }

    fn apply_time_series_histogram(&self, series: &RuntimeTimeSeries, spec: &mut PlotSpec) {
        let display_unit = self
            .plot_options
            .x_unit
            .clone()
            .or_else(|| self.plot_options.y_unit.clone())
            .unwrap_or_else(|| series.display_unit.clone());
        let values = series
            .points
            .iter()
            .map(|point| convert_display_value(point.y, &series.display_unit, &display_unit))
            .collect::<Vec<_>>();
        let bins = histogram_bins(&values);
        let points = histogram_points_from_bins(&bins);
        let title = self
            .plot_options
            .title
            .clone()
            .unwrap_or_else(|| format!("{} histogram", series.name));

        spec.title = title;
        spec.plot_type = "histogram".to_owned();
        spec.x_axis = PlotAxis {
            name: series.name.clone(),
            label: series.quantity_kind.clone(),
            unit: display_unit.clone(),
        };
        spec.y_axis = PlotAxis {
            name: "Frequency".to_owned(),
            label: "Frequency".to_owned(),
            unit: "count".to_owned(),
        };
        spec.series = vec![PlotSeries {
            name: series.name.clone(),
            quantity_kind: series.quantity_kind.clone(),
            display_unit,
            bins,
            points,
        }];
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

    pub fn report_computed_metrics(&self) -> Vec<ReportComputedMetric> {
        self.metrics
            .iter()
            .map(|metric| ReportComputedMetric {
                binding: metric.binding.clone(),
                kind: metric.kind.clone(),
                left: metric.left.clone(),
                right: metric.right.clone(),
                quantity_kind: metric.quantity_kind.clone(),
                unit: metric.unit.clone(),
                value: metric.value,
                sample_count: metric.sample_count,
                alignment_reference: metric.alignment_reference.clone(),
                alignment_status: metric.alignment_status.clone(),
                alignment_step_status: metric.alignment_step_status.clone(),
                status: metric.status.clone(),
                line: metric.line,
            })
            .collect()
    }

    pub fn report_validations(&self) -> Vec<ReportValidationResult> {
        self.validations
            .iter()
            .map(|validation| ReportValidationResult {
                expression: validation.expression.clone(),
                left: validation.left.clone(),
                operator: validation.operator.clone(),
                right: validation.right.clone(),
                left_value: validation.left_value,
                right_value: validation.right_value,
                unit: validation.unit.clone(),
                status: validation.status.clone(),
                line: validation.line,
            })
            .collect()
    }

    pub fn report_time_axes(&self) -> Vec<ReportTimeAxis> {
        self.time_axes
            .iter()
            .map(|axis| ReportTimeAxis {
                name: axis.name.clone(),
                source_table: axis.source_table.clone(),
                source_column: axis.source_column.clone(),
                axis: axis.axis.clone(),
                unit: axis.unit.clone(),
                start: axis.start,
                end: axis.end,
                count: axis.count,
                nominal_step: axis.nominal_step,
                irregular: axis.irregular,
                missing_count: axis.missing_count,
            })
            .collect()
    }

    pub fn report_time_alignments(&self) -> Vec<ReportTimeAlignment> {
        self.time_alignments
            .iter()
            .map(|alignment| ReportTimeAlignment {
                left: alignment.left.clone(),
                right: alignment.right.clone(),
                axis: alignment.axis.clone(),
                left_count: alignment.left_count,
                right_count: alignment.right_count,
                matched_count: alignment.matched_count,
                left_nominal_step: alignment.left_nominal_step,
                right_nominal_step: alignment.right_nominal_step,
                left_irregular: alignment.left_irregular,
                right_irregular: alignment.right_irregular,
                step_status: alignment.step_status.clone(),
                overlap_start: alignment.overlap_start,
                overlap_end: alignment.overlap_end,
                status: alignment.status.clone(),
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
        for system_ir in &mut spec.system_ir {
            let solver_results = self
                .system_solutions
                .iter()
                .filter(|solution| solution.system == system_ir.name)
                .map(RuntimeSystemSolution::to_report_solution)
                .collect::<Vec<_>>();
            if let Some(solution) = solver_results.first() {
                system_ir.solver_boundary.status = solution.status.clone();
                system_ir.solver_boundary.reason = solution.reason.clone();
                system_ir.solver_plan.status = solution.status.clone();
                system_ir.solver_plan.method = solution.method.clone();
                system_ir.solver_plan.ode_runner.status = solution.status.clone();
                system_ir.solver_plan.ode_runner.reason = solution.reason.clone();
            }
            system_ir.solver_results = solver_results;
        }
    }

    pub fn apply_component_solutions(&self, spec: &mut ReportSpec) {
        let behavior_graph_attempted = self
            .component_solutions
            .iter()
            .any(|solution| solution.method.starts_with("behavior_graph_"));
        for solution in &self.component_solutions {
            let Some(assembly) = spec
                .assemblies
                .iter_mut()
                .find(|assembly| assembly.name == solution.assembly)
            else {
                continue;
            };
            assembly.status = solution.status.clone();
            assembly.residual_graph.status = solution.convergence_status.clone();
            assembly.residual_graph.solver_plan = solution.method.clone();
            assembly.boundary.equation_count = solution.equation_count;
            assembly.boundary.unknown_count = solution.unknown_count;
            if let Some(failure) = &solution.failure_artifact {
                assembly.boundary.diagnostic_code = Some(failure.code.clone());
            }
            assembly.solver_result = Some(solution.to_report_solver_result());
            if solution.method.starts_with("behavior_graph_") {
                if assembly.solver_preview.delay_history
                    == "delay_call_runtime_buffer_seed_not_integrated"
                {
                    assembly.solver_preview.delay_history =
                        "delay_call_runtime_buffer_integrated".to_owned();
                }
                if assembly.solver_preview.predictor
                    == "predictor_call_contract_seed_not_integrated"
                {
                    assembly.solver_preview.predictor =
                        "predictor_call_contract_integrated".to_owned();
                }
                if assembly.solver_preview.external_adapter
                    == "external_behavior_wrapper_seed_not_integrated"
                {
                    assembly.solver_preview.external_adapter =
                        "external_behavior_wrapper_integrated".to_owned();
                }
            }
        }
        if behavior_graph_attempted {
            mark_behavior_graph_report_integrated(spec);
        }
    }
}

fn mark_behavior_graph_report_integrated(spec: &mut ReportSpec) {
    for node in &mut spec.component_graph.behavior_nodes {
        match node.behavior_kind.as_str() {
            "delay" => {
                node.status = "delay_call_runtime_buffer_integrated".to_owned();
                node.relationship_status = Some("delay_relationship_runtime_evaluated".to_owned());
                node.runtime_warning_status =
                    Some("evaluated_in_language_behavior_graph".to_owned());
            }
            "predictor" => {
                node.status = "predictor_call_contract_integrated".to_owned();
                node.jacobian_policy = Some("finite_difference_allowed".to_owned());
                node.runtime_warning_status =
                    Some("evaluated_in_language_behavior_graph".to_owned());
                fill_behavior_output_contract_from_input(
                    node,
                    "predictor_output_typed_identity_seed",
                );
            }
            "external" => {
                node.status = "external_behavior_wrapper_integrated".to_owned();
                node.runtime_warning_status =
                    Some("evaluated_in_language_behavior_graph".to_owned());
                fill_behavior_output_contract_from_input(
                    node,
                    "external_output_typed_identity_seed",
                );
            }
            _ => {}
        }
    }
}

fn fill_behavior_output_contract_from_input(
    node: &mut eng_report::ReportComponentGraphBehaviorNode,
    status: &str,
) {
    let Some(input) = node.contract_inputs.first().cloned() else {
        return;
    };
    for output in &mut node.contract_outputs {
        if output.quantity_kind == "unspecified_by_seed" {
            output.quantity_kind = input.quantity_kind.clone();
            output.display_unit = input.display_unit.clone();
            output.canonical_unit = input.canonical_unit.clone();
        }
        output.status = status.to_owned();
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
pub struct RuntimeTimeAxis {
    pub name: String,
    pub source_table: String,
    pub source_column: String,
    pub axis: String,
    pub unit: String,
    pub start: Option<f64>,
    pub end: Option<f64>,
    pub count: usize,
    pub nominal_step: Option<f64>,
    pub irregular: bool,
    pub missing_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RuntimePoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSystemStepDiagnostic {
    pub output_index: usize,
    pub start_time_s: f64,
    pub end_time_s: f64,
    pub dt_s: f64,
    pub error_norm: f64,
    pub status: String,
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
pub struct RuntimeMetric {
    pub binding: String,
    pub kind: String,
    pub left: String,
    pub right: String,
    pub quantity_kind: String,
    pub unit: String,
    pub value: f64,
    pub sample_count: usize,
    pub alignment_reference: Option<String>,
    pub alignment_status: Option<String>,
    pub alignment_step_status: Option<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeValidation {
    pub expression: String,
    pub left: String,
    pub operator: String,
    pub right: String,
    pub left_value: Option<f64>,
    pub right_value: Option<f64>,
    pub unit: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeTimeAlignment {
    pub left: String,
    pub right: String,
    pub axis: String,
    pub left_count: usize,
    pub right_count: usize,
    pub matched_count: usize,
    pub left_nominal_step: Option<f64>,
    pub right_nominal_step: Option<f64>,
    pub left_irregular: bool,
    pub right_irregular: bool,
    pub step_status: String,
    pub overlap_start: Option<f64>,
    pub overlap_end: Option<f64>,
    pub status: String,
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
    pub binding: Option<String>,
    pub status: String,
    pub method: String,
    pub reason: String,
    pub states: Vec<String>,
    pub algebraic_variables: Vec<String>,
    pub inputs: Vec<String>,
    pub parameters: Vec<String>,
    pub outputs: Vec<String>,
    pub state: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub time_unit: String,
    pub duration_s: f64,
    pub time_step_s: f64,
    pub step_count: usize,
    pub tolerance: f64,
    pub max_iterations: usize,
    pub iteration_count: usize,
    pub convergence_status: String,
    pub failure_code: Option<String>,
    pub failure_reason: Option<String>,
    pub initial_value: f64,
    pub final_value: f64,
    pub canonical_initial_value: f64,
    pub canonical_final_value: f64,
    pub step_diagnostics: Vec<RuntimeSystemStepDiagnostic>,
    pub points: Vec<RuntimePoint>,
}

impl RuntimeSystemSolution {
    pub fn from_solver_result(
        system: &eng_compiler::SystemInfo,
        binding: Option<&str>,
        state: &eng_compiler::SystemVariableInfo,
        solver_result: &SolverResult,
        reason: &str,
    ) -> Option<Self> {
        let trajectory = solver_result.single_state()?;
        Self::from_solver_trajectory(system, binding, state, solver_result, trajectory, reason)
    }

    pub fn from_solver_trajectory(
        system: &eng_compiler::SystemInfo,
        binding: Option<&str>,
        state: &eng_compiler::SystemVariableInfo,
        solver_result: &SolverResult,
        trajectory: &StateTrajectory,
        reason: &str,
    ) -> Option<Self> {
        let canonical_initial_value = trajectory.initial_value()?;
        let canonical_final_value = trajectory.final_value()?;
        let points = trajectory
            .time_value_points(&solver_result.time_grid)
            .into_iter()
            .map(|(x, value)| RuntimePoint {
                x,
                y: display_variable_value(value, state),
            })
            .collect::<Vec<_>>();

        Some(Self {
            system: system.name.clone(),
            binding: binding.map(str::to_owned),
            status: solver_result.diagnostics.status.clone(),
            method: solver_result.plan.options.method.clone(),
            reason: reason.to_owned(),
            states: solver_result.plan.simulation.states.clone(),
            algebraic_variables: system_variable_names_by_role(system, "algebraic"),
            inputs: solver_result.plan.simulation.inputs.clone(),
            parameters: solver_result.plan.simulation.parameters.clone(),
            outputs: solver_result.plan.simulation.outputs.clone(),
            state: trajectory.name.clone(),
            quantity_kind: trajectory.quantity_kind.clone(),
            display_unit: state.display_unit.clone(),
            canonical_unit: trajectory.canonical_unit.clone(),
            time_unit: solver_result.time_grid.unit.clone(),
            duration_s: solver_result.time_grid.duration_s,
            time_step_s: solver_result.time_grid.timestep_s,
            step_count: solver_result.time_grid.step_count,
            tolerance: solver_result.diagnostics.tolerance,
            max_iterations: solver_result.diagnostics.max_iterations,
            iteration_count: solver_result.diagnostics.iteration_count,
            convergence_status: solver_result.diagnostics.convergence_status.clone(),
            failure_code: solver_result
                .diagnostics
                .failure
                .as_ref()
                .map(|failure| failure.code.clone()),
            failure_reason: solver_result
                .diagnostics
                .failure
                .as_ref()
                .map(|failure| failure.message.clone()),
            initial_value: display_variable_value(canonical_initial_value, state),
            final_value: display_variable_value(canonical_final_value, state),
            canonical_initial_value,
            canonical_final_value,
            step_diagnostics: Vec::new(),
            points,
        })
    }

    pub fn failed_from_solver_failure(
        system: &eng_compiler::SystemInfo,
        binding: Option<&str>,
        state: &eng_compiler::SystemVariableInfo,
        solver_input: &SolverInput,
        failure: &SolverFailure,
        reason: &str,
    ) -> Option<Self> {
        let canonical_initial_value = solver_input
            .state_layout
            .index_of(&state.name)
            .and_then(|index| solver_input.initial_state.get(index).copied())
            .or_else(|| canonical_variable_value(state))
            .unwrap_or(0.0);
        let initial_value = display_variable_value(canonical_initial_value, state);

        Some(Self {
            system: system.name.clone(),
            binding: binding.map(str::to_owned),
            status: "failed".to_owned(),
            method: solver_input.plan.options.method.clone(),
            reason: reason.to_owned(),
            states: solver_input.plan.simulation.states.clone(),
            algebraic_variables: system_variable_names_by_role(system, "algebraic"),
            inputs: solver_input.plan.simulation.inputs.clone(),
            parameters: solver_input.plan.simulation.parameters.clone(),
            outputs: solver_input.plan.simulation.outputs.clone(),
            state: state.name.clone(),
            quantity_kind: state.quantity_kind.clone(),
            display_unit: state.display_unit.clone(),
            canonical_unit: state.canonical_unit.clone(),
            time_unit: solver_input.time_grid.unit.clone(),
            duration_s: solver_input.time_grid.duration_s,
            time_step_s: solver_input.time_grid.timestep_s,
            step_count: solver_input.time_grid.step_count,
            tolerance: solver_input.plan.options.tolerance,
            max_iterations: solver_input.plan.options.max_iterations,
            iteration_count: 0,
            convergence_status: "failed".to_owned(),
            failure_code: Some(failure.code.clone()),
            failure_reason: Some(failure.message.clone()),
            initial_value,
            final_value: initial_value,
            canonical_initial_value,
            canonical_final_value: canonical_initial_value,
            step_diagnostics: Vec::new(),
            points: vec![RuntimePoint {
                x: solver_input.time_grid.start_s,
                y: initial_value,
            }],
        })
    }

    pub fn to_report_solution(&self) -> ReportSystemSolution {
        ReportSystemSolution {
            binding: self.binding.clone(),
            status: self.status.clone(),
            method: self.method.clone(),
            reason: self.reason.clone(),
            states: self.states.clone(),
            algebraic_variables: self.algebraic_variables.clone(),
            inputs: self.inputs.clone(),
            parameters: self.parameters.clone(),
            outputs: self.outputs.clone(),
            state: self.state.clone(),
            quantity_kind: self.quantity_kind.clone(),
            display_unit: self.display_unit.clone(),
            canonical_unit: self.canonical_unit.clone(),
            time_unit: self.time_unit.clone(),
            duration_s: self.duration_s,
            time_step_s: self.time_step_s,
            step_count: self.step_count,
            tolerance: self.tolerance,
            max_iterations: self.max_iterations,
            iteration_count: self.iteration_count,
            convergence_status: self.convergence_status.clone(),
            failure_code: self.failure_code.clone(),
            failure_reason: self.failure_reason.clone(),
            initial_value: self.initial_value,
            final_value: self.final_value,
            canonical_initial_value: self.canonical_initial_value,
            canonical_final_value: self.canonical_final_value,
            step_diagnostics: self
                .step_diagnostics
                .iter()
                .map(|diagnostic| ReportSystemSolverStepDiagnostic {
                    output_index: diagnostic.output_index,
                    start_time_s: diagnostic.start_time_s,
                    end_time_s: diagnostic.end_time_s,
                    dt_s: diagnostic.dt_s,
                    error_norm: diagnostic.error_norm,
                    status: diagnostic.status.clone(),
                })
                .collect(),
            points: self
                .points
                .iter()
                .map(|point| ReportSystemSolutionPoint {
                    x: point.x,
                    y: point.y,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeComponentSolution {
    pub assembly: String,
    pub status: String,
    pub method: String,
    pub reason: String,
    pub equation_count: usize,
    pub unknown_count: usize,
    pub residual_norm: f64,
    pub tolerance: f64,
    pub max_iterations: usize,
    pub iteration_count: usize,
    pub convergence_status: String,
    pub variables: Vec<RuntimeComponentVariableSolution>,
    pub trajectories: Vec<RuntimeComponentTrajectory>,
    pub step_diagnostics: Vec<RuntimeComponentStepDiagnostic>,
    pub residuals: Vec<RuntimeComponentResidualEvaluation>,
    pub largest_residuals: Vec<RuntimeComponentResidualEvaluation>,
    pub failure_artifact: Option<RuntimeSolverFailureArtifact>,
}

#[allow(dead_code)]
impl RuntimeComponentSolution {
    pub fn from_solver_assembly(assembly_name: &str, solver_assembly: &EquationAssembly) -> Self {
        let residual_graph = ResidualGraph::from_assembly(solver_assembly);
        let equation_count = solver_assembly.equation_count();
        let unknown_count = solver_assembly.unknown_count();
        let mut variable_values = vec![0.0; unknown_count];
        let mut variable_status = "homogeneous_zero_seed".to_owned();
        let mut method = "linear_residual_graph_shape_check".to_owned();
        let mut iteration_count = usize::from(equation_count > 0 && unknown_count > 0);

        let (mut status, mut reason, mut failure_artifact, mut convergence_status) =
            if equation_count == 0 {
                (
                    "not_solved_no_equations".to_owned(),
                    "assembly graph has no generated equations".to_owned(),
                    Some(RuntimeSolverFailureArtifact {
                        code: "E-ASSEMBLY-SOLVE-001".to_owned(),
                        message: "component assembly has no generated equations to solve"
                            .to_owned(),
                    }),
                    "linear_residual_not_attempted".to_owned(),
                )
            } else if unknown_count == 0 {
                (
                    "not_solved_no_unknowns".to_owned(),
                    "assembly graph has no classified unknown variables".to_owned(),
                    Some(RuntimeSolverFailureArtifact {
                        code: "E-ASSEMBLY-SOLVE-002".to_owned(),
                        message: "component assembly has equations but no classified unknowns"
                            .to_owned(),
                    }),
                    "linear_residual_not_attempted".to_owned(),
                )
            } else if equation_count < unknown_count {
                (
                "constraint_satisfied_nonunique".to_owned(),
                "homogeneous connection constraints evaluate to zero, but boundary/component equations are missing so the physical solution is non-unique".to_owned(),
                Some(RuntimeSolverFailureArtifact {
                    code: "E-ASSEMBLY-UNDERDETERMINED".to_owned(),
                    message: "assembly has fewer equations than unknowns; add component behavior or boundary conditions before treating this as a physical solve".to_owned(),
                }),
                "linear_residual_satisfied_nonunique".to_owned(),
            )
            } else if equation_count > unknown_count {
                (
                "not_solved_overdetermined".to_owned(),
                "assembly graph has more equations than unknowns, so the dense linear residual solve was not attempted".to_owned(),
                Some(RuntimeSolverFailureArtifact {
                    code: "E-ASSEMBLY-OVERDETERMINED".to_owned(),
                    message: "assembly has more equations than unknowns; review generated constraints before numeric solving".to_owned(),
                }),
                "linear_residual_not_attempted_overdetermined".to_owned(),
            )
            } else {
                method = "dense_linear_residual_graph".to_owned();
                iteration_count = 1;
                match solve_linear_residual_graph(
                    &residual_graph,
                    COMPONENT_LINEAR_SOLVER_TOLERANCE,
                ) {
                    Ok(linear_solution) => {
                        variable_values = linear_solution
                            .variables
                            .iter()
                            .map(|variable| variable.value)
                            .collect();
                        variable_status = "solved_linear".to_owned();
                        iteration_count = linear_solution.iteration_count;
                        let converged = linear_solution.status == "converged";
                        (
                        if converged {
                            "solved_linear".to_owned()
                        } else {
                            "linear_residual_above_tolerance".to_owned()
                        },
                        "dense linear residual graph solve completed for the square algebraic assembly"
                            .to_owned(),
                        None,
                        if converged {
                            "linear_converged".to_owned()
                        } else {
                            "linear_residual_above_tolerance".to_owned()
                        },
                    )
                    }
                    Err(failure) => (
                        "linear_solve_failed".to_owned(),
                        failure.message.clone(),
                        Some(RuntimeSolverFailureArtifact {
                            code: failure.code,
                            message: failure.message,
                        }),
                        "linear_failed".to_owned(),
                    ),
                }
            };

        let variables = solver_assembly
            .unknowns
            .iter()
            .zip(variable_values.iter())
            .map(|(variable, value)| RuntimeComponentVariableSolution {
                name: variable.name.clone(),
                role: variable.role.clone(),
                value: *value,
                unit: variable.unit.clone(),
                status: variable_status.clone(),
            })
            .collect::<Vec<_>>();
        let residual_output = match residual_graph.evaluate(
            &ResidualInput::new(&variable_values).with_tolerance(COMPONENT_LINEAR_SOLVER_TOLERANCE),
        ) {
            Ok(output) => output,
            Err(failure) => {
                if failure_artifact.is_none() {
                    status = "residual_evaluation_failed".to_owned();
                    convergence_status = "residual_evaluation_failed".to_owned();
                    reason = failure.message.clone();
                    failure_artifact = Some(RuntimeSolverFailureArtifact {
                        code: failure.code,
                        message: failure.message,
                    });
                }
                ResidualOutput {
                    values: Vec::new(),
                    residual_norm: f64::INFINITY,
                    tolerance: COMPONENT_LINEAR_SOLVER_TOLERANCE,
                }
            }
        };
        let residuals = residual_graph
            .residuals
            .iter()
            .zip(residual_output.values.iter())
            .map(|(residual, value)| RuntimeComponentResidualEvaluation {
                name: residual.name.clone(),
                expression: residual.expression.text.clone(),
                value: value.value,
                unit: residual.unit.unit.clone(),
                normalized_value: value.normalized_value,
                scale: residual.scale.value,
                scale_policy: residual.scale.policy.clone(),
                status: value.status.clone(),
            })
            .collect::<Vec<_>>();
        let largest_residuals = largest_component_residuals(&residuals);

        Self {
            assembly: assembly_name.to_owned(),
            status,
            method,
            reason,
            equation_count,
            unknown_count,
            residual_norm: residual_output.residual_norm,
            tolerance: COMPONENT_LINEAR_SOLVER_TOLERANCE,
            max_iterations: 1,
            iteration_count,
            convergence_status,
            variables,
            trajectories: Vec::new(),
            step_diagnostics: Vec::new(),
            residuals,
            largest_residuals,
            failure_artifact,
        }
    }

    pub fn from_fixed_point_solver_assembly(
        assembly_name: &str,
        solver_assembly: &EquationAssembly,
        options: FixedPointOptions,
        initial_value: f64,
    ) -> Self {
        let residual_graph = ResidualGraph::from_assembly(solver_assembly);
        let equation_count = solver_assembly.equation_count();
        let unknown_count = solver_assembly.unknown_count();
        let initial_values = vec![initial_value; unknown_count];
        let mut variable_values = initial_values.clone();
        let mut variable_status = "fixed_point_not_attempted".to_owned();
        let mut status: String;
        let mut reason = "fixed-point residual graph solve requested from source".to_owned();
        let mut failure_artifact = None;
        let mut convergence_status: String;
        let mut iteration_count = 0;

        match fixed_point_update_plan(&residual_graph) {
            Ok(update_plan) => {
                match solve_fixed_point(&initial_values, &options, |values| {
                    fixed_point_update_values(&residual_graph, &update_plan, values)
                }) {
                    Ok(fixed_point) => {
                        variable_values = fixed_point.values;
                        variable_status = if fixed_point.failure.is_none() {
                            "solved_fixed_point".to_owned()
                        } else {
                            "fixed_point_not_converged".to_owned()
                        };
                        iteration_count = fixed_point.iteration_count;
                        convergence_status = fixed_point.convergence_status;
                        if let Some(failure) = fixed_point.failure {
                            status = "fixed_point_not_converged".to_owned();
                            reason = failure.message.clone();
                            failure_artifact = Some(RuntimeSolverFailureArtifact {
                                code: failure.code,
                                message: failure.message,
                            });
                        } else {
                            status = "solved_fixed_point".to_owned();
                        }
                    }
                    Err(failure) => {
                        status = "fixed_point_failed".to_owned();
                        convergence_status = "fixed_point_failed".to_owned();
                        reason = failure.message.clone();
                        failure_artifact = Some(RuntimeSolverFailureArtifact {
                            code: failure.code,
                            message: failure.message,
                        });
                    }
                }
            }
            Err(failure) => {
                status = "fixed_point_not_capable".to_owned();
                convergence_status = "fixed_point_not_capable".to_owned();
                reason = failure.message.clone();
                failure_artifact = Some(RuntimeSolverFailureArtifact {
                    code: failure.code,
                    message: failure.message,
                });
            }
        }

        let variables = solver_assembly
            .unknowns
            .iter()
            .zip(variable_values.iter())
            .map(|(variable, value)| RuntimeComponentVariableSolution {
                name: variable.name.clone(),
                role: variable.role.clone(),
                value: *value,
                unit: variable.unit.clone(),
                status: variable_status.clone(),
            })
            .collect::<Vec<_>>();
        let residual_output = match residual_graph
            .evaluate(&ResidualInput::new(&variable_values).with_tolerance(options.tolerance))
        {
            Ok(output) => output,
            Err(failure) => {
                if failure_artifact.is_none() {
                    status = "residual_evaluation_failed".to_owned();
                    convergence_status = "residual_evaluation_failed".to_owned();
                    reason = failure.message.clone();
                    failure_artifact = Some(RuntimeSolverFailureArtifact {
                        code: failure.code,
                        message: failure.message,
                    });
                }
                ResidualOutput {
                    values: Vec::new(),
                    residual_norm: f64::INFINITY,
                    tolerance: options.tolerance,
                }
            }
        };
        if failure_artifact.is_none() && residual_output.residual_norm > options.tolerance {
            status = "fixed_point_residual_above_tolerance".to_owned();
            convergence_status = "fixed_point_residual_above_tolerance".to_owned();
            reason = format!(
                "fixed-point residual norm {} exceeded tolerance {}",
                residual_output.residual_norm, options.tolerance
            );
            failure_artifact = Some(RuntimeSolverFailureArtifact {
                code: "E-FIXED-POINT-RESIDUAL".to_owned(),
                message: reason.clone(),
            });
        }
        let residuals = residual_graph
            .residuals
            .iter()
            .zip(residual_output.values.iter())
            .map(|(residual, value)| RuntimeComponentResidualEvaluation {
                name: residual.name.clone(),
                expression: residual.expression.text.clone(),
                value: value.value,
                unit: residual.unit.unit.clone(),
                normalized_value: value.normalized_value,
                scale: residual.scale.value,
                scale_policy: residual.scale.policy.clone(),
                status: value.status.clone(),
            })
            .collect::<Vec<_>>();
        let largest_residuals = largest_component_residuals(&residuals);

        Self {
            assembly: assembly_name.to_owned(),
            status,
            method: "fixed_point_residual_graph".to_owned(),
            reason,
            equation_count,
            unknown_count,
            residual_norm: residual_output.residual_norm,
            tolerance: options.tolerance,
            max_iterations: options.max_iterations,
            iteration_count,
            convergence_status,
            variables,
            trajectories: Vec::new(),
            step_diagnostics: Vec::new(),
            residuals,
            largest_residuals,
            failure_artifact,
        }
    }

    pub fn from_dynamic_component_result(
        assembly_name: &str,
        dynamic_result: &DynamicComponentResult,
        reason: &str,
    ) -> Self {
        let mut solution =
            Self::from_dynamic_solver_result(assembly_name, &dynamic_result.solver_result, reason);
        solution.step_diagnostics = dynamic_result
            .step_diagnostics
            .iter()
            .map(|diagnostic| RuntimeComponentStepDiagnostic {
                step_index: diagnostic.step_index,
                time_s: diagnostic.time_s,
                algebraic_iteration_count: diagnostic.algebraic_iteration_count,
                residual_norm: diagnostic.residual_norm,
                convergence_status: diagnostic.convergence_status.clone(),
                failure_artifact: diagnostic.failure.as_ref().map(|failure| {
                    RuntimeSolverFailureArtifact {
                        code: failure.code.clone(),
                        message: failure.message.clone(),
                    }
                }),
            })
            .collect();
        solution
    }

    pub fn from_dynamic_component_assembly_result(
        assembly: &EquationAssembly,
        dynamic_result: &DynamicComponentResult,
        reason: &str,
    ) -> Self {
        let mut solution =
            Self::from_dynamic_component_result(&assembly.name, dynamic_result, reason);
        solution.equation_count = assembly.equation_count();
        solution.unknown_count = assembly.unknown_count();
        solution
    }

    pub fn from_dynamic_solver_result(
        assembly_name: &str,
        solver_result: &SolverResult,
        reason: &str,
    ) -> Self {
        let state_trajectories = solver_result
            .output
            .state_trajectories
            .iter()
            .map(|trajectory| {
                component_trajectory_from_solver_trajectory(
                    trajectory,
                    "state",
                    &solver_result.time_grid,
                )
            });
        let algebraic_trajectories =
            solver_result
                .output
                .algebraic_trajectories
                .iter()
                .map(|trajectory| {
                    component_trajectory_from_solver_trajectory(
                        trajectory,
                        "algebraic",
                        &solver_result.time_grid,
                    )
                });
        let trajectories = state_trajectories
            .chain(algebraic_trajectories)
            .collect::<Vec<_>>();
        let variable_status = if solver_result.diagnostics.status == "computed" {
            "trajectory_computed"
        } else {
            "trajectory_failed"
        };
        let variables = trajectories
            .iter()
            .map(|trajectory| RuntimeComponentVariableSolution {
                name: trajectory.name.clone(),
                role: trajectory.role.clone(),
                value: trajectory.final_value,
                unit: trajectory.unit.clone(),
                status: variable_status.to_owned(),
            })
            .collect::<Vec<_>>();

        Self {
            assembly: assembly_name.to_owned(),
            status: solver_result.diagnostics.status.clone(),
            method: solver_result.plan.options.method.clone(),
            reason: reason.to_owned(),
            equation_count: 0,
            unknown_count: variables.len(),
            residual_norm: 0.0,
            tolerance: solver_result.diagnostics.tolerance,
            max_iterations: solver_result.diagnostics.max_iterations,
            iteration_count: solver_result.diagnostics.iteration_count,
            convergence_status: solver_result.diagnostics.convergence_status.clone(),
            variables,
            trajectories,
            step_diagnostics: Vec::new(),
            residuals: Vec::new(),
            largest_residuals: Vec::new(),
            failure_artifact: solver_result.diagnostics.failure.as_ref().map(|failure| {
                RuntimeSolverFailureArtifact {
                    code: failure.code.clone(),
                    message: failure.message.clone(),
                }
            }),
        }
    }

    pub fn to_report_solver_result(&self) -> ReportComponentSolverResult {
        ReportComponentSolverResult {
            status: self.status.clone(),
            method: self.method.clone(),
            reason: self.reason.clone(),
            residual_norm: self.residual_norm,
            tolerance: self.tolerance,
            max_iterations: self.max_iterations,
            iteration_count: self.iteration_count,
            convergence_status: self.convergence_status.clone(),
            variables: self
                .variables
                .iter()
                .map(|variable| ReportComponentSolverVariable {
                    name: variable.name.clone(),
                    role: variable.role.clone(),
                    value: variable.value,
                    unit: variable.unit.clone(),
                    status: variable.status.clone(),
                })
                .collect(),
            trajectories: self
                .trajectories
                .iter()
                .map(|trajectory| ReportComponentSolverTrajectory {
                    name: trajectory.name.clone(),
                    role: trajectory.role.clone(),
                    quantity_kind: trajectory.quantity_kind.clone(),
                    unit: trajectory.unit.clone(),
                    initial_value: trajectory.initial_value,
                    final_value: trajectory.final_value,
                    point_count: trajectory.point_count,
                    points: trajectory
                        .points
                        .iter()
                        .map(|point| ReportSystemSolutionPoint {
                            x: point.x,
                            y: point.y,
                        })
                        .collect(),
                })
                .collect(),
            step_diagnostics: self
                .step_diagnostics
                .iter()
                .map(|diagnostic| ReportComponentSolverStepDiagnostic {
                    step_index: diagnostic.step_index,
                    time_s: diagnostic.time_s,
                    algebraic_iteration_count: diagnostic.algebraic_iteration_count,
                    residual_norm: diagnostic.residual_norm,
                    convergence_status: diagnostic.convergence_status.clone(),
                    failure_artifact: diagnostic.failure_artifact.as_ref().map(|failure| {
                        ReportSolverFailureArtifact {
                            code: failure.code.clone(),
                            message: failure.message.clone(),
                        }
                    }),
                })
                .collect(),
            residuals: self
                .residuals
                .iter()
                .map(|residual| ReportComponentSolverResidual {
                    name: residual.name.clone(),
                    expression: residual.expression.clone(),
                    value: residual.value,
                    unit: residual.unit.clone(),
                    normalized_value: residual.normalized_value,
                    scale: residual.scale,
                    scale_policy: residual.scale_policy.clone(),
                    status: residual.status.clone(),
                })
                .collect(),
            largest_residuals: self
                .largest_residuals
                .iter()
                .map(|residual| ReportComponentSolverResidual {
                    name: residual.name.clone(),
                    expression: residual.expression.clone(),
                    value: residual.value,
                    unit: residual.unit.clone(),
                    normalized_value: residual.normalized_value,
                    scale: residual.scale,
                    scale_policy: residual.scale_policy.clone(),
                    status: residual.status.clone(),
                })
                .collect(),
            failure_artifact: self.failure_artifact.as_ref().map(|failure| {
                ReportSolverFailureArtifact {
                    code: failure.code.clone(),
                    message: failure.message.clone(),
                }
            }),
        }
    }
}

fn largest_component_residuals(
    residuals: &[RuntimeComponentResidualEvaluation],
) -> Vec<RuntimeComponentResidualEvaluation> {
    let mut largest = residuals.to_vec();
    largest.sort_by(|left, right| {
        right
            .normalized_value
            .abs()
            .total_cmp(&left.normalized_value.abs())
            .then_with(|| left.name.cmp(&right.name))
    });
    largest.truncate(3);
    largest
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeComponentVariableSolution {
    pub name: String,
    pub role: String,
    pub value: f64,
    pub unit: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeComponentTrajectory {
    pub name: String,
    pub role: String,
    pub quantity_kind: String,
    pub unit: String,
    pub initial_value: f64,
    pub final_value: f64,
    pub point_count: usize,
    pub points: Vec<RuntimePoint>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeComponentStepDiagnostic {
    pub step_index: usize,
    pub time_s: f64,
    pub algebraic_iteration_count: usize,
    pub residual_norm: f64,
    pub convergence_status: String,
    pub failure_artifact: Option<RuntimeSolverFailureArtifact>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeComponentResidualEvaluation {
    pub name: String,
    pub expression: String,
    pub value: f64,
    pub unit: String,
    pub normalized_value: f64,
    pub scale: f64,
    pub scale_policy: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSolverFailureArtifact {
    pub code: String,
    pub message: String,
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
    pub series_list: Vec<String>,
    pub axis: Option<String>,
    pub histogram: Option<String>,
    pub distribution: Option<String>,
    pub model_plot: Option<ModelPlotOptions>,
    pub plot_type: Option<String>,
    pub title: Option<String>,
    pub x_unit: Option<String>,
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
    data.time_axes = materialize_time_axes(&data.tables);
    data.time_series = materialize_time_series(report, &data.tables);
    data.system_solutions = materialize_system_solutions(report, &data.time_series);
    data.component_solutions = materialize_component_solutions(report);
    data.time_series
        .extend(materialize_system_solution_series(&data.system_solutions));
    data.time_series
        .extend(materialize_component_solution_series(
            &data.component_solutions,
        ));
    data.time_alignments = materialize_time_alignments(&data.time_series);
    data.statistics = materialize_statistics(report, &data.time_series);
    data.integrations = materialize_integrations(report, &data.time_series);
    data.uncertainties = materialize_uncertainties(report);
    data.ml_artifacts = materialize_ml_artifacts(report, &data.time_series, &data.tables);
    data.metrics = materialize_metrics(report, &data.time_series, &data.time_alignments);
    data.validations = materialize_validations(report, &data.metrics, &data.integrations);
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

fn materialize_time_axes(tables: &[RuntimeTable]) -> Vec<RuntimeTimeAxis> {
    tables
        .iter()
        .map(|table| {
            let (source_column, unit, values, missing_count) =
                if let Some(column) = table.time_index_column() {
                    let parse_failure_count = table
                        .parse_failures
                        .iter()
                        .filter(|failure| failure.column == column.name)
                        .count();
                    (
                        column.name.clone(),
                        "s".to_owned(),
                        table
                            .normalized_time_axis_values()
                            .unwrap_or_else(|| sample_axis_values(table.row_count)),
                        column.missing_count + parse_failure_count,
                    )
                } else {
                    (
                        "row_index".to_owned(),
                        "sample".to_owned(),
                        sample_axis_values(table.row_count),
                        0,
                    )
                };
            let nominal_step = nominal_step_from_values(&values);
            let irregular = missing_count > 0 || axis_values_irregular(&values, nominal_step);
            RuntimeTimeAxis {
                name: format!("{}.Time", table.binding),
                source_table: table.binding.clone(),
                source_column,
                axis: "Time".to_owned(),
                unit,
                start: values.first().copied(),
                end: values.last().copied(),
                count: table.row_count,
                nominal_step,
                irregular,
                missing_count,
            }
        })
        .collect()
}

fn sample_axis_values(row_count: usize) -> Vec<f64> {
    (0..row_count).map(|index| index as f64).collect()
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
    for block in &report.semantic_program.where_blocks {
        for binding in &block.bindings {
            let Some((axis, quantity_kind)) = time_series_quantity(&binding.quantity_kind) else {
                continue;
            };
            if quantity_kind != "HeatRate" {
                continue;
            }
            if let Some(runtime_series) = heat_rate_series(
                &binding.name,
                &axis,
                &quantity_kind,
                &binding.display_unit,
                &binding.expression,
                report,
                tables,
            ) {
                series.push(runtime_series);
            }
        }
    }
    for table in tables {
        series.extend(materialize_table_column_series(table));
    }
    series
}

fn materialize_table_column_series(table: &RuntimeTable) -> Vec<RuntimeTimeSeries> {
    let (x_values, x_unit) = table.axis_values();
    table
        .columns
        .iter()
        .filter(|column| !column.is_index)
        .filter_map(|column| {
            let RuntimeValues::Number(values) = &column.values else {
                return None;
            };
            let display_unit = column
                .unit
                .clone()
                .or_else(|| column.canonical_unit.clone())
                .unwrap_or_else(|| "1".to_owned());
            let mut points = Vec::new();
            for (index, value) in values.iter().enumerate() {
                let Some(value) = value else {
                    continue;
                };
                points.push(RuntimePoint {
                    x: x_values.get(index).copied().unwrap_or(index as f64),
                    y: *value,
                });
            }
            Some(RuntimeTimeSeries {
                name: format!("{}.{}", table.binding, column.name),
                axis: "Time".to_owned(),
                x_unit: x_unit.clone(),
                quantity_kind: column.type_name.clone(),
                display_unit,
                source_table: table.binding.clone(),
                source_expression: format!("{}.{}", table.binding, column.name),
                points,
            })
        })
        .collect()
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

fn materialize_metrics(
    report: &CheckReport,
    series: &[RuntimeTimeSeries],
    alignments: &[RuntimeTimeAlignment],
) -> Vec<RuntimeMetric> {
    report
        .inferred_declarations
        .iter()
        .filter_map(|declaration| {
            let (left, right) = parse_rmse_expression(&declaration.expression)?;
            let left_series = series.iter().find(|series| series.name == left)?;
            let right_series = series.iter().find(|series| series.name == right)?;
            let (alignment_reference, alignment_status, alignment_step_status) =
                metric_alignment_reference(&left, &right, alignments);
            let mut actual = Vec::new();
            let mut predicted = Vec::new();
            for point in &left_series.points {
                let Some(right_value) = interpolate_series_value(right_series, point.x) else {
                    continue;
                };
                let left_value = convert_display_value(
                    point.y,
                    &left_series.display_unit,
                    &right_series.display_unit,
                );
                actual.push(left_value);
                predicted.push(right_value);
            }
            if actual.is_empty() {
                return Some(RuntimeMetric {
                    binding: declaration.name.clone(),
                    kind: "rmse".to_owned(),
                    left,
                    right,
                    quantity_kind: "unknown".to_owned(),
                    unit: right_series.display_unit.clone(),
                    value: 0.0,
                    sample_count: 0,
                    alignment_reference,
                    alignment_status,
                    alignment_step_status,
                    status: "unavailable".to_owned(),
                    line: declaration.line,
                });
            }
            let value = regression_metrics(&actual, &predicted).0;
            let quantity_kind = if left_series.quantity_kind == "AbsoluteTemperature"
                && right_series.quantity_kind == "AbsoluteTemperature"
            {
                "TemperatureDelta".to_owned()
            } else {
                left_series.quantity_kind.clone()
            };
            let unit = if quantity_kind == "TemperatureDelta" {
                "K".to_owned()
            } else {
                right_series.display_unit.clone()
            };
            Some(RuntimeMetric {
                binding: declaration.name.clone(),
                kind: "rmse".to_owned(),
                left,
                right,
                quantity_kind,
                unit,
                value,
                sample_count: actual.len(),
                alignment_reference,
                alignment_status,
                alignment_step_status,
                status: "computed".to_owned(),
                line: declaration.line,
            })
        })
        .collect()
}

fn metric_alignment_reference(
    left: &str,
    right: &str,
    alignments: &[RuntimeTimeAlignment],
) -> (Option<String>, Option<String>, Option<String>) {
    alignments
        .iter()
        .find(|alignment| {
            (alignment.left == left && alignment.right == right)
                || (alignment.left == right && alignment.right == left)
        })
        .map(|alignment| {
            (
                Some(format!("{} vs {}", alignment.left, alignment.right)),
                Some(alignment.status.clone()),
                Some(alignment.step_status.clone()),
            )
        })
        .unwrap_or((None, None, None))
}

fn parse_rmse_expression(expression: &str) -> Option<(String, String)> {
    let rest = expression.trim().strip_prefix("rmse ")?;
    let (left, right) = rest.split_once(" vs ")?;
    Some((left.trim().to_owned(), right.trim().to_owned()))
}

fn materialize_validations(
    report: &CheckReport,
    metrics: &[RuntimeMetric],
    integrations: &[RuntimeIntegration],
) -> Vec<RuntimeValidation> {
    report
        .semantic_program
        .command_styles
        .iter()
        .filter(|command| command.verb == "validate")
        .map(|command| {
            let expression = command.target.clone();
            let Some((left, operator, right)) = parse_validation_expression(&expression) else {
                return RuntimeValidation {
                    expression,
                    left: String::new(),
                    operator: String::new(),
                    right: String::new(),
                    left_value: None,
                    right_value: None,
                    unit: String::new(),
                    status: "unavailable".to_owned(),
                    line: command.line,
                };
            };
            let left_metric = metrics.iter().find(|metric| metric.binding == left);
            let left_integration = integrations
                .iter()
                .find(|integration| integration.binding == left);
            let left_value = left_metric
                .map(|metric| metric.value)
                .or_else(|| left_integration.map(|integration| integration.value));
            let unit = left_metric
                .map(|metric| metric.unit.clone())
                .or_else(|| left_integration.map(|integration| integration.unit.clone()))
                .unwrap_or_default();
            let right_value = number_with_optional_unit(&right).map(|(value, right_unit)| {
                right_unit
                    .as_deref()
                    .map(|right_unit| convert_display_value(value, right_unit, &unit))
                    .unwrap_or(value)
            });
            let status = match (left_value, right_value) {
                (Some(left_value), Some(right_value)) => {
                    if compare_values(left_value, right_value, &operator) {
                        "passed"
                    } else {
                        "failed"
                    }
                }
                _ => "unavailable",
            }
            .to_owned();
            RuntimeValidation {
                expression,
                left,
                operator,
                right,
                left_value,
                right_value,
                unit,
                status,
                line: command.line,
            }
        })
        .collect()
}

fn parse_validation_expression(expression: &str) -> Option<(String, String, String)> {
    for operator in ["<=", ">=", "==", "!=", "<", ">"] {
        if let Some((left, right)) = expression.split_once(operator) {
            return Some((
                left.trim().to_owned(),
                operator.to_owned(),
                right.trim().to_owned(),
            ));
        }
    }
    None
}

fn compare_values(left: f64, right: f64, operator: &str) -> bool {
    match operator {
        "<" => left < right,
        "<=" => left <= right,
        ">" => left > right,
        ">=" => left >= right,
        "==" => (left - right).abs() <= f64::EPSILON,
        "!=" => (left - right).abs() > f64::EPSILON,
        _ => false,
    }
}

fn nominal_time_step(points: &[RuntimePoint]) -> Option<f64> {
    let values = points.iter().map(|point| point.x).collect::<Vec<_>>();
    nominal_step_from_values(&values)
}

fn nominal_step_from_values(values: &[f64]) -> Option<f64> {
    let mut steps = values
        .windows(2)
        .filter_map(|window| {
            let step = window[1] - window[0];
            (step.is_finite() && step > 0.0).then_some(step)
        })
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return None;
    }
    steps.sort_by(|left, right| left.total_cmp(right));
    Some(steps[(steps.len() - 1) / 2])
}

fn time_step_tolerance(step: f64) -> f64 {
    1e-6_f64.max(step.abs() * 1e-6)
}

fn time_axis_irregular(points: &[RuntimePoint], nominal_step: Option<f64>) -> bool {
    let values = points.iter().map(|point| point.x).collect::<Vec<_>>();
    axis_values_irregular(&values, nominal_step)
}

fn axis_values_irregular(values: &[f64], nominal_step: Option<f64>) -> bool {
    let Some(nominal_step) = nominal_step else {
        return false;
    };
    if values.len() < 3 || nominal_step <= 0.0 || !nominal_step.is_finite() {
        return false;
    }
    let tolerance = time_step_tolerance(nominal_step);
    values.windows(2).any(|window| {
        let step = window[1] - window[0];
        !step.is_finite() || step <= 0.0 || (step - nominal_step).abs() > tolerance
    })
}

fn time_step_status(
    left_step: Option<f64>,
    right_step: Option<f64>,
    left_irregular: bool,
    right_irregular: bool,
) -> &'static str {
    let (Some(left_step), Some(right_step)) = (left_step, right_step) else {
        return "unavailable";
    };
    if left_irregular || right_irregular {
        return "mismatch";
    }
    let tolerance = time_step_tolerance(left_step).max(time_step_tolerance(right_step));
    if (left_step - right_step).abs() <= tolerance {
        "matched"
    } else {
        "mismatch"
    }
}

fn materialize_time_alignments(series: &[RuntimeTimeSeries]) -> Vec<RuntimeTimeAlignment> {
    let mut alignments = Vec::new();
    let table_series = series
        .iter()
        .filter(|series| !series.source_table.is_empty())
        .collect::<Vec<_>>();
    for left_index in 0..table_series.len() {
        for right_index in (left_index + 1)..table_series.len() {
            let left = table_series[left_index];
            let right = table_series[right_index];
            if left.source_table == right.source_table || left.axis != right.axis {
                continue;
            }
            let left_start = left.points.first().map(|point| point.x);
            let left_end = left.points.last().map(|point| point.x);
            let right_start = right.points.first().map(|point| point.x);
            let right_end = right.points.last().map(|point| point.x);
            let overlap_start = match (left_start, right_start) {
                (Some(left), Some(right)) => Some(left.max(right)),
                _ => None,
            };
            let overlap_end = match (left_end, right_end) {
                (Some(left), Some(right)) => Some(left.min(right)),
                _ => None,
            };
            let matched_count = left
                .points
                .iter()
                .filter(|left_point| {
                    right
                        .points
                        .iter()
                        .any(|right_point| (left_point.x - right_point.x).abs() <= 1e-6)
                })
                .count();
            let status = if matched_count == left.points.len().min(right.points.len())
                && left.points.len() == right.points.len()
            {
                "matched"
            } else if overlap_start
                .zip(overlap_end)
                .is_some_and(|(start, end)| end >= start)
            {
                "overlap"
            } else {
                "mismatch"
            };
            let left_nominal_step = nominal_time_step(&left.points);
            let right_nominal_step = nominal_time_step(&right.points);
            let left_irregular = time_axis_irregular(&left.points, left_nominal_step);
            let right_irregular = time_axis_irregular(&right.points, right_nominal_step);
            let step_status = time_step_status(
                left_nominal_step,
                right_nominal_step,
                left_irregular,
                right_irregular,
            );
            alignments.push(RuntimeTimeAlignment {
                left: left.name.clone(),
                right: right.name.clone(),
                axis: left.axis.clone(),
                left_count: left.points.len(),
                right_count: right.points.len(),
                matched_count,
                left_nominal_step,
                right_nominal_step,
                left_irregular,
                right_irregular,
                step_status: step_status.to_owned(),
                overlap_start,
                overlap_end,
                status: status.to_owned(),
            });
        }
    }
    alignments
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

fn materialize_system_solutions(
    report: &CheckReport,
    series: &[RuntimeTimeSeries],
) -> Vec<RuntimeSystemSolution> {
    let mut solutions = Vec::new();
    for system in &report.semantic_program.systems {
        let requests = simulate_requests(report, &system.name);
        if requests.is_empty() {
            if let Some(mut state_space_solutions) =
                materialize_state_space_solutions(report, system, None, &[], series)
            {
                solutions.append(&mut state_space_solutions);
            } else if let Some(solution) =
                materialize_first_order_thermal_solution(system, None, &[], series)
            {
                solutions.push(solution);
            } else if let Some(mut source_ode_solutions) =
                materialize_source_ode_solutions(system, None, &[], series)
            {
                solutions.append(&mut source_ode_solutions);
            }
        } else {
            for request in requests {
                if let Some(mut state_space_solutions) = materialize_state_space_solutions(
                    report,
                    system,
                    Some(request.binding.as_str()),
                    &request.options,
                    series,
                ) {
                    solutions.append(&mut state_space_solutions);
                } else if let Some(solution) = materialize_first_order_thermal_solution(
                    system,
                    Some(request.binding.as_str()),
                    &request.options,
                    series,
                ) {
                    solutions.push(solution);
                } else if let Some(mut source_ode_solutions) = materialize_source_ode_solutions(
                    system,
                    Some(request.binding.as_str()),
                    &request.options,
                    series,
                ) {
                    solutions.append(&mut source_ode_solutions);
                } else {
                    solutions.push(skipped_system_solution(
                        system,
                        Some(request.binding.as_str()),
                        &request.options,
                    ));
                }
            }
        }
    }
    solutions
}

fn materialize_component_solutions(report: &CheckReport) -> Vec<RuntimeComponentSolution> {
    let mut solutions = Vec::new();
    for assembly in &report.semantic_program.component_assemblies {
        let solver_assembly = solver_equation_assembly_from_component_info(report, assembly);
        let requests = component_solve_requests(report, &assembly.name);
        if requests.is_empty() {
            let mut solution =
                RuntimeComponentSolution::from_solver_assembly(&assembly.name, &solver_assembly);
            annotate_component_behavior_solution(&mut solution, assembly);
            solutions.push(solution);
            continue;
        }
        for request in requests {
            let solver = option_value(&request.options, "solver").unwrap_or("dense_linear");
            let mut solution = match solver.trim() {
                "fixed_point" => {
                    let options = fixed_point_options_from_solve_request(&request.options);
                    RuntimeComponentSolution::from_fixed_point_solver_assembly(
                        &assembly.name,
                        &solver_assembly,
                        options,
                        fixed_point_initial_value_from_solve_request(&request.options),
                    )
                }
                "newton" | "nonlinear_newton" => {
                    nonlinear_component_solution_from_solve_request(&solver_assembly, &request)
                }
                "implicit_euler_dae" | "dae_implicit_euler" => {
                    dae_component_solution_from_solve_request(&solver_assembly, &request)
                }
                "dynamic_component_explicit_euler" | "dynamic_component_semi_implicit_euler" => {
                    dynamic_component_solution_from_solve_request(
                        report,
                        assembly,
                        &solver_assembly,
                        &request,
                    )
                }
                _ => {
                    RuntimeComponentSolution::from_solver_assembly(&assembly.name, &solver_assembly)
                }
            };
            append_component_solution_reason(
                &mut solution,
                &format!("source solve binding `{}`", request.binding),
            );
            annotate_component_behavior_solution(&mut solution, assembly);
            solutions.push(solution);
        }
    }
    solutions
}

const COMPONENT_LINEAR_SOLVER_TOLERANCE: f64 = 1e-9;
const COMPONENT_BEHAVIOR_NOT_INTEGRATED_CODE: &str = "E-BEHAVIOR-NOT-INTEGRATED";
const COMPONENT_BEHAVIOR_NOT_INTEGRATED_NOTE: &str =
    "behavior graph nodes are present but not yet integrated into numeric residual evaluation";

#[derive(Clone, Debug, PartialEq)]
struct ComponentSolveRequest {
    binding: String,
    options: Vec<eng_compiler::WithOptionInfo>,
}

fn component_solve_requests(
    report: &CheckReport,
    assembly_name: &str,
) -> Vec<ComponentSolveRequest> {
    report
        .inferred_declarations
        .iter()
        .filter_map(|declaration| {
            let requested_assembly = declaration.expression.trim().strip_prefix("solve ")?.trim();
            if requested_assembly != assembly_name {
                return None;
            }
            let options = report
                .semantic_program
                .with_blocks
                .iter()
                .find(|block| block.owner_line == Some(declaration.line))
                .map(|block| block.options.clone())
                .unwrap_or_default();
            Some(ComponentSolveRequest {
                binding: declaration.name.clone(),
                options,
            })
        })
        .collect()
}

fn nonlinear_component_solution_from_solve_request(
    solver_assembly: &EquationAssembly,
    request: &ComponentSolveRequest,
) -> RuntimeComponentSolution {
    let options = newton_options_from_solve_request(&request.options);
    let tolerance = options.tolerance;

    if !solver_assembly.states.is_empty() {
        return failed_source_component_solution(
            solver_assembly,
            "newton_source_residual_graph",
            "Newton source solve requires an algebraic-only component residual graph; use implicit_euler_dae for state derivatives",
            &SolverFailure::new(
                "E-NEWTON-SOURCE-SHAPE",
                "source Newton solve cannot solve residual graphs with state derivatives",
            ),
            tolerance,
            options.max_iterations,
            "newton_source_failed",
        );
    }
    if solver_assembly.generated_equations.is_empty()
        || solver_assembly.unknowns.is_empty()
        || solver_assembly.generated_equations.len() != solver_assembly.unknowns.len()
    {
        return failed_source_component_solution(
            solver_assembly,
            "newton_source_residual_graph",
            "Newton source solve requires a non-empty square algebraic residual graph",
            &SolverFailure::new(
                "E-NEWTON-SOURCE-SHAPE",
                format!(
                    "source Newton solve expected a square graph, got {} residual(s) and {} unknown(s)",
                    solver_assembly.generated_equations.len(),
                    solver_assembly.unknowns.len()
                ),
            ),
            tolerance,
            options.max_iterations,
            "newton_source_failed",
        );
    }

    let residual_graph = ResidualGraph::from_assembly(solver_assembly);
    let initial_values = component_initial_values_from_options(
        &request.options,
        "initial",
        &solver_assembly.unknowns,
        1.0,
    );
    let initial_values = match initial_values {
        Ok(values) => values,
        Err(failure) => {
            return failed_source_component_solution(
                solver_assembly,
                "newton_source_residual_graph",
                "Newton source solve initial values could not be materialized",
                &failure,
                tolerance,
                options.max_iterations,
                "newton_source_failed",
            );
        }
    };

    let jacobian_policy = option_value(&request.options, "jacobian")
        .map(str::trim)
        .unwrap_or("finite_difference");
    let solve_result = if jacobian_policy == "source_linear_terms" {
        let scaled_jacobian = match source_linear_terms_jacobian(&residual_graph) {
            Ok(jacobian) => jacobian,
            Err(failure) => {
                return failed_source_component_solution(
                    solver_assembly,
                    "newton_source_residual_graph_with_provided_jacobian",
                    "Newton source solve could not materialize the requested source-linear Jacobian hook",
                    &failure,
                    tolerance,
                    options.max_iterations,
                    "newton_source_failed",
                );
            }
        };
        solve_newton_with_jacobian(
            &initial_values,
            &options,
            |values| source_algebraic_residual_values(solver_assembly, &residual_graph, values),
            |_values, _baseline| Ok(scaled_jacobian.clone()),
        )
    } else {
        solve_newton(&initial_values, &options, |values| {
            source_algebraic_residual_values(solver_assembly, &residual_graph, values)
        })
    };

    let newton = match solve_result {
        Ok(result) => result,
        Err(failure) => {
            return failed_source_component_solution(
                solver_assembly,
                "newton_source_residual_graph",
                "Newton source solve failed before iteration",
                &failure,
                tolerance,
                options.max_iterations,
                "newton_source_failed",
            );
        }
    };
    let method_name = if jacobian_policy == "source_linear_terms" {
        "newton_source_residual_graph_with_provided_jacobian"
    } else {
        "newton_source_residual_graph"
    };
    let evaluation = match source_residual_evaluation_for_unknowns(
        solver_assembly,
        &residual_graph,
        &newton.values,
        tolerance,
    ) {
        Ok(evaluation) => evaluation,
        Err(failure) => {
            return failed_source_component_solution(
                solver_assembly,
                method_name,
                "Newton source solve residual evaluation failed after iteration",
                &failure,
                tolerance,
                options.max_iterations,
                "newton_source_failed",
            );
        }
    };
    let failed = newton.failure.clone();
    let status = if failed.is_some() {
        "newton_not_converged"
    } else {
        "solved_nonlinear"
    };
    let variable_status = if failed.is_some() {
        "newton_not_converged"
    } else {
        "solved_newton"
    };
    let variables = solver_assembly
        .unknowns
        .iter()
        .zip(newton.values.iter())
        .map(|(variable, value)| RuntimeComponentVariableSolution {
            name: variable.name.clone(),
            role: variable.role.clone(),
            value: *value,
            unit: variable.unit.clone(),
            status: variable_status.to_owned(),
        })
        .collect::<Vec<_>>();
    let step_diagnostics = newton_residual_history_diagnostics(&newton, failed.as_ref());

    RuntimeComponentSolution {
        assembly: solver_assembly.name.clone(),
        status: status.to_owned(),
        method: method_name.to_owned(),
        reason: if failed.is_some() {
            "Newton source solve returned a failure artifact".to_owned()
        } else {
            "Newton source solve executed source-level residual graph".to_owned()
        },
        equation_count: solver_assembly.equation_count(),
        unknown_count: solver_assembly.unknown_count(),
        residual_norm: evaluation.residual_norm,
        tolerance,
        max_iterations: options.max_iterations,
        iteration_count: newton.iteration_count,
        convergence_status: newton.convergence_status,
        variables,
        trajectories: Vec::new(),
        step_diagnostics,
        largest_residuals: largest_component_residuals(&evaluation.residuals),
        residuals: evaluation.residuals,
        failure_artifact: failed.map(|failure| RuntimeSolverFailureArtifact {
            code: failure.code,
            message: failure.message,
        }),
    }
}

fn dae_component_solution_from_solve_request(
    solver_assembly: &EquationAssembly,
    request: &ComponentSolveRequest,
) -> RuntimeComponentSolution {
    let method = "implicit_euler_dae_source_residual_graph";
    let newton_options = newton_options_from_solve_request(&request.options);
    let tolerance = newton_options.tolerance;
    let timestep_s =
        match option_value(&request.options, "timestep").and_then(parse_duration_seconds) {
            Some(value) => value,
            None => {
                return failed_source_component_solution(
                    solver_assembly,
                    method,
                    "DAE source solve requires a positive timestep duration",
                    &SolverFailure::new(
                        "E-DAE-SOURCE-TIMESTEP",
                        "DAE source solve requires a positive timestep duration",
                    ),
                    tolerance,
                    newton_options.max_iterations,
                    "dae_source_failed",
                );
            }
        };
    let duration_s =
        match option_value(&request.options, "duration").and_then(parse_duration_seconds) {
            Some(value) => value,
            None => {
                return failed_source_component_solution(
                    solver_assembly,
                    method,
                    "DAE source solve requires a positive duration",
                    &SolverFailure::new(
                        "E-DAE-SOURCE-DURATION",
                        "DAE source solve requires a positive duration",
                    ),
                    tolerance,
                    newton_options.max_iterations,
                    "dae_source_failed",
                );
            }
        };
    let time_grid = match TimeGrid::fixed_step(duration_s, timestep_s) {
        Ok(grid) => grid,
        Err(failure) => {
            return failed_source_component_solution(
                solver_assembly,
                method,
                "DAE source solve time grid could not be materialized",
                &failure,
                tolerance,
                newton_options.max_iterations,
                "dae_source_failed",
            );
        }
    };
    let split = match solver_assembly.dynamic_component_split() {
        Ok(split) => split,
        Err(failure) => {
            return failed_source_component_solution(
                solver_assembly,
                method,
                "DAE source solve could not split state and algebraic variables from assembly",
                &failure,
                tolerance,
                newton_options.max_iterations,
                "dae_source_failed",
            );
        }
    };
    let residual_graph = ResidualGraph::from_assembly(solver_assembly);
    let initial_state = match component_initial_values_from_options(
        &request.options,
        "initial",
        &solver_assembly.states,
        0.0,
    ) {
        Ok(values) => values,
        Err(failure) => {
            return failed_source_component_solution(
                solver_assembly,
                method,
                "DAE source solve initial state could not be materialized",
                &failure,
                tolerance,
                newton_options.max_iterations,
                "dae_source_failed",
            );
        }
    };
    let derivative_variables = derivative_variables_for_states(&solver_assembly.states);
    let initial_state_derivatives = match component_initial_values_from_options(
        &request.options,
        "initial_derivative",
        &derivative_variables,
        0.0,
    ) {
        Ok(values) => values,
        Err(failure) => {
            return failed_source_component_solution(
                solver_assembly,
                method,
                "DAE source solve initial state derivatives could not be materialized",
                &failure,
                tolerance,
                newton_options.max_iterations,
                "dae_source_failed",
            );
        }
    };
    let mut initial_algebraic = match component_initial_values_from_options(
        &request.options,
        "initial_algebraic",
        &solver_assembly.algebraic_variables,
        0.0,
    ) {
        Ok(values) => values,
        Err(failure) => {
            return failed_source_component_solution(
                solver_assembly,
                method,
                "DAE source solve initial algebraic values could not be materialized",
                &failure,
                tolerance,
                newton_options.max_iterations,
                "dae_source_failed",
            );
        }
    };
    let algebraic_initialization = option_value(&request.options, "algebraic_initialization")
        .map(str::trim)
        .unwrap_or("newton");
    if algebraic_initialization == "newton" && !initial_algebraic.is_empty() {
        let initialized = initialize_algebraic_variables(
            AlgebraicInitializationInput {
                state: &initial_state,
                state_derivative: &initial_state_derivatives,
                algebraic_guess: &initial_algebraic,
                inputs: &[],
                parameters: &[],
                time_s: 0.0,
            },
            &newton_options,
            |sample| {
                source_dae_residual_values(
                    solver_assembly,
                    &residual_graph,
                    sample,
                    SourceResidualSubset::AlgebraicOnly,
                )
            },
        );
        match initialized {
            Ok(result) if result.failure.is_none() => {
                initial_algebraic = result.values;
            }
            Ok(result) => {
                let failure = result.failure.unwrap_or_else(|| {
                    SolverFailure::new(
                        "E-DAE-ALGEBRAIC-INITIALIZATION",
                        "DAE algebraic initialization did not converge",
                    )
                });
                return failed_source_component_solution(
                    solver_assembly,
                    method,
                    "DAE source solve algebraic initialization failed",
                    &failure,
                    tolerance,
                    newton_options.max_iterations,
                    "dae_source_failed",
                );
            }
            Err(failure) => {
                return failed_source_component_solution(
                    solver_assembly,
                    method,
                    "DAE source solve algebraic initialization could not be evaluated",
                    &failure,
                    tolerance,
                    newton_options.max_iterations,
                    "dae_source_failed",
                );
            }
        }
    }

    let consistency_tolerance = option_value(&request.options, "consistency_tolerance")
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(tolerance);
    let dae_input = DaeInput {
        states: solver_assembly
            .states
            .iter()
            .zip(initial_state.iter().copied())
            .map(|(state, initial)| DaeVariable::new(state.name.clone(), initial))
            .collect(),
        initial_state_derivatives,
        algebraic: solver_assembly
            .algebraic_variables
            .iter()
            .zip(initial_algebraic.iter().copied())
            .map(|(variable, initial)| DaeVariable::new(variable.name.clone(), initial))
            .collect(),
        inputs: Vec::new(),
        parameters: Vec::new(),
    };
    let dae_options = DaeOptions {
        timestep_s,
        step_count: time_grid.step_count,
        consistency_tolerance,
        newton: newton_options.clone(),
        mass_matrix: None,
        ..DaeOptions::default()
    };
    let dae = solve_implicit_euler_dae(&dae_input, &dae_options, |sample| {
        source_dae_residual_values(
            solver_assembly,
            &residual_graph,
            sample,
            SourceResidualSubset::All,
        )
    });
    let dae = match dae {
        Ok(result) => result,
        Err(failure) => {
            return failed_source_component_solution(
                solver_assembly,
                method,
                "DAE source solve failed during initial consistency checks",
                &failure,
                tolerance,
                newton_options.max_iterations,
                "dae_source_failed",
            );
        }
    };

    let state_trajectories = dae
        .state_trajectories
        .iter()
        .filter_map(|trajectory| {
            solver_assembly
                .states
                .iter()
                .find(|state| state.name == trajectory.name)
                .map(|state| StateTrajectory {
                    name: trajectory.name.clone(),
                    quantity_kind: state.quantity_kind.clone(),
                    canonical_unit: state.unit.clone(),
                    values: trajectory.values.clone(),
                })
        })
        .collect::<Vec<_>>();
    let algebraic_trajectories = dae
        .algebraic_trajectories
        .iter()
        .filter_map(|trajectory| {
            solver_assembly
                .algebraic_variables
                .iter()
                .find(|variable| variable.name == trajectory.name)
                .map(|variable| StateTrajectory {
                    name: trajectory.name.clone(),
                    quantity_kind: variable.quantity_kind.clone(),
                    canonical_unit: variable.unit.clone(),
                    values: trajectory.values.clone(),
                })
        })
        .collect::<Vec<_>>();
    let diagnostics = SolverDiagnostics {
        status: if dae.failure.is_some() {
            "failed".to_owned()
        } else {
            "computed".to_owned()
        },
        convergence_status: dae.convergence_status.clone(),
        failure: dae.failure.clone(),
        iteration_count: dae
            .step_reports
            .iter()
            .map(|report| report.newton.iteration_count)
            .sum(),
        tolerance,
        max_iterations: newton_options.max_iterations,
    };
    let solver_result = SolverResult {
        plan: SolverPlan::new(
            solver_assembly.name.clone(),
            SimulationPlan {
                inputs: Vec::new(),
                outputs: layout_names_from_unknowns(&solver_assembly.states),
                states: layout_names_from_unknowns(&solver_assembly.states),
                parameters: Vec::new(),
            },
            SolverOptions {
                method: method.to_owned(),
                timestep_s,
                tolerance,
                max_iterations: newton_options.max_iterations,
            },
        ),
        time_grid: time_grid.clone(),
        state_layout: split.state_layout.clone(),
        output_layout: OutputLayout {
            entries: split.state_layout.entries.clone(),
        },
        output: crate::solver::SolverOutput {
            state_trajectories,
            algebraic_trajectories,
        },
        diagnostics,
    };
    let mut solution = RuntimeComponentSolution::from_dynamic_solver_result(
        &solver_assembly.name,
        &solver_result,
        "DAE source solve executed source-level residual graph with identity mass-matrix fallback",
    );
    solution.equation_count = solver_assembly.equation_count();
    solution.unknown_count = solver_assembly.unknown_count();
    solution.step_diagnostics = dae
        .step_reports
        .iter()
        .map(|report| RuntimeComponentStepDiagnostic {
            step_index: report.step_index,
            time_s: report.time_s,
            algebraic_iteration_count: report.newton.iteration_count,
            residual_norm: report
                .newton
                .residual_history
                .last()
                .copied()
                .unwrap_or(f64::INFINITY),
            convergence_status: report.newton.convergence_status.clone(),
            failure_artifact: report.newton.failure.as_ref().map(|failure| {
                RuntimeSolverFailureArtifact {
                    code: failure.code.clone(),
                    message: failure.message.clone(),
                }
            }),
        })
        .collect();
    if let Ok(evaluation) = source_residual_evaluation_for_dae_final(
        solver_assembly,
        &residual_graph,
        &dae_input,
        &solver_result,
        tolerance,
    ) {
        solution.residual_norm = evaluation.residual_norm;
        solution.residuals = evaluation.residuals;
        solution.largest_residuals = largest_component_residuals(&solution.residuals);
    }
    solution
}

fn dynamic_component_solution_from_solve_request(
    report: &CheckReport,
    component_info: &eng_compiler::ComponentAssemblyInfo,
    solver_assembly: &EquationAssembly,
    request: &ComponentSolveRequest,
) -> RuntimeComponentSolution {
    let solver = option_value(&request.options, "solver")
        .map(str::trim)
        .unwrap_or("dynamic_component_semi_implicit_euler");
    let options = DynamicComponentOptions {
        algebraic: fixed_point_options_from_solve_request(&request.options),
    };
    if solver == "dynamic_component_explicit_euler"
        && component_assembly_has_behavior_seed(component_info)
    {
        return behavior_dynamic_component_solution_from_solve_request(
            report,
            solver_assembly,
            request,
            &options,
        );
    }
    let explicit_assembly = if solver == "dynamic_component_explicit_euler" {
        match explicit_dynamic_component_source_assembly(solver_assembly) {
            Ok(assembly) => Some(assembly),
            Err(failure) => {
                return failed_dynamic_component_source_solution(
                    solver_assembly,
                    solver,
                    &options,
                    &failure,
                    "dynamic component explicit source solve could not isolate derivative residuals",
                );
            }
        }
    } else {
        None
    };
    let dynamic_assembly = explicit_assembly.as_ref().unwrap_or(solver_assembly);
    let split = match dynamic_assembly.dynamic_component_split() {
        Ok(split) => split,
        Err(failure) => {
            return failed_dynamic_component_source_solution(
                dynamic_assembly,
                solver,
                &options,
                &failure,
                "dynamic component source solve could not split the component assembly",
            );
        }
    };
    let solve_input = match dynamic_component_solve_input_from_request(
        dynamic_assembly,
        &split,
        &request.options,
    ) {
        Ok(input) => input,
        Err(failure) => {
            return failed_dynamic_component_source_solution(
                dynamic_assembly,
                solver,
                &options,
                &failure,
                "dynamic component source solve options could not be materialized",
            );
        }
    };

    match solve_dynamic_component_assembly(dynamic_assembly, solve_input, options.clone()) {
        Ok(dynamic_result) => RuntimeComponentSolution::from_dynamic_component_assembly_result(
            dynamic_assembly,
            &dynamic_result,
            "dynamic component source solve executed assembled residual graph",
        ),
        Err(failure) => failed_dynamic_component_source_solution(
            dynamic_assembly,
            solver,
            &options,
            &failure,
            "dynamic component source solve failed before timestep execution",
        ),
    }
}

fn behavior_dynamic_component_solution_from_solve_request(
    report: &CheckReport,
    solver_assembly: &EquationAssembly,
    request: &ComponentSolveRequest,
    options: &DynamicComponentOptions,
) -> RuntimeComponentSolution {
    let method = "behavior_graph_explicit_euler_source";
    let dynamic_assembly = match explicit_dynamic_component_source_assembly(solver_assembly) {
        Ok(assembly) => assembly,
        Err(failure) => {
            return failed_dynamic_component_source_solution(
                solver_assembly,
                method,
                options,
                &failure,
                "behavior graph source solve could not isolate derivative residuals",
            );
        }
    };
    let split = match dynamic_assembly.dynamic_component_split() {
        Ok(split) => split,
        Err(failure) => {
            return failed_dynamic_component_source_solution(
                &dynamic_assembly,
                method,
                options,
                &failure,
                "behavior graph source solve could not split the component assembly",
            );
        }
    };
    let solve_input = match dynamic_component_solve_input_from_request(
        &dynamic_assembly,
        &split,
        &request.options,
    ) {
        Ok(input) => input,
        Err(failure) => {
            return failed_dynamic_component_source_solution(
                &dynamic_assembly,
                method,
                options,
                &failure,
                "behavior graph source solve options could not be materialized",
            );
        }
    };
    let (mut behavior_graph, behavior_output_symbols) =
        match source_behavior_graph_from_report(report, &dynamic_assembly) {
            Ok(graph) => graph,
            Err(failure) => {
                return failed_dynamic_component_source_solution(
                    &dynamic_assembly,
                    method,
                    options,
                    &failure,
                    "behavior graph source solve could not materialize behavior nodes",
                );
            }
        };
    let time_grid = match TimeGrid::fixed_step(solve_input.duration_s, solve_input.timestep_s) {
        Ok(grid) => grid,
        Err(failure) => {
            return failed_dynamic_component_source_solution(
                &dynamic_assembly,
                method,
                options,
                &failure,
                "behavior graph source solve time grid could not be materialized",
            );
        }
    };
    let solver_input = SolverInput {
        plan: SolverPlan::new(
            dynamic_assembly.name.clone(),
            SimulationPlan {
                inputs: layout_names_from_unknowns(&dynamic_assembly.inputs),
                outputs: layout_names_from_unknowns(&dynamic_assembly.states),
                states: layout_names_from_unknowns(&dynamic_assembly.states),
                parameters: layout_names_from_unknowns(&dynamic_assembly.parameters),
            },
            SolverOptions {
                method: method.to_owned(),
                timestep_s: solve_input.timestep_s,
                tolerance: options.algebraic.tolerance,
                max_iterations: 1,
            },
        ),
        time_grid,
        state_layout: split.state_layout.clone(),
        input_layout: split.input_layout.clone(),
        parameter_layout: split.parameter_layout.clone(),
        output_layout: OutputLayout {
            entries: split.state_layout.entries.clone(),
        },
        initial_state: solve_input.initial_state.clone(),
        inputs: solve_input.inputs.clone(),
        parameters: solve_input.parameters.clone(),
    };
    let mut step_diagnostics = Vec::new();
    let solver_result =
        solve_fixed_step_ode(FixedStepMethod::ExplicitEuler, &solver_input, |sample| {
            let behavior_evaluation = behavior_graph.evaluate_rhs(
                &BehaviorRhsSample::new(
                    sample.time_s,
                    sample.state,
                    sample.inputs,
                    sample.parameters,
                ),
                |behavior| {
                    step_diagnostics.push(RuntimeComponentStepDiagnostic {
                        step_index: step_diagnostics.len() + 1,
                        time_s: sample.time_s,
                        algebraic_iteration_count: 0,
                        residual_norm: 0.0,
                        convergence_status: format!("behavior_graph_{}", behavior.status),
                        failure_artifact: None,
                    });
                    let symbols = source_behavior_symbols(
                        &dynamic_assembly,
                        sample.time_s,
                        sample.state,
                        sample.inputs,
                        sample.parameters,
                        behavior,
                        &behavior_output_symbols,
                    )?;
                    source_behavior_derivatives(&dynamic_assembly, &symbols)
                },
            )?;
            Ok(behavior_evaluation.derivatives)
        });
    match solver_result {
        Ok(solver_result) => {
            let mut solution = RuntimeComponentSolution::from_dynamic_solver_result(
                &dynamic_assembly.name,
                &solver_result,
                "behavior graph source solve executed typed behavior nodes during explicit-Euler RHS evaluation",
            );
            solution.equation_count = dynamic_assembly.equation_count();
            solution.unknown_count = dynamic_assembly.unknown_count();
            solution.step_diagnostics = step_diagnostics;
            solution.convergence_status = if solution
                .step_diagnostics
                .iter()
                .any(|diagnostic| diagnostic.convergence_status.contains("range_warning"))
            {
                "behavior_graph_range_warning".to_owned()
            } else {
                "behavior_graph_integrated".to_owned()
            };
            solution
        }
        Err(failure) => failed_dynamic_component_source_solution(
            &dynamic_assembly,
            method,
            options,
            &failure,
            "behavior graph source solve failed during RHS evaluation",
        ),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourceBehaviorKind {
    Delay,
    Predictor,
    External,
}

struct SourceBehaviorCall {
    kind: SourceBehaviorKind,
    signal: String,
    delay_s: Option<f64>,
}

fn source_behavior_graph_from_report(
    report: &CheckReport,
    assembly: &EquationAssembly,
) -> Result<(BehaviorGraphRhsAdapter, Vec<String>), SolverFailure> {
    let assembly_components = assembly
        .components
        .iter()
        .map(|component| component.name.as_str())
        .collect::<HashSet<_>>();
    let mut nodes = Vec::new();
    let mut output_symbols = Vec::new();
    let mut output_indices = HashMap::new();
    for component in &report.semantic_program.components {
        if !assembly_components.contains(component.name.as_str()) {
            continue;
        }
        for local in &component.local_expressions {
            let Some(call) = source_behavior_call(&local.expression)? else {
                continue;
            };
            if output_indices.contains_key(&local.name) {
                return Err(SolverFailure::new(
                    "E-BEHAVIOR-SOURCE-DUPLICATE",
                    format!(
                        "behavior graph source solve found duplicate behavior output `{}`",
                        local.name
                    ),
                ));
            }
            let (source, signal_name, quantity_kind, canonical_unit) =
                source_behavior_signal_source(
                    assembly,
                    component,
                    local.line,
                    &call.signal,
                    &output_indices,
                )?;
            let node = match call.kind {
                SourceBehaviorKind::Delay => {
                    let delay_s = call.delay_s.ok_or_else(|| {
                        SolverFailure::new(
                            "E-BEHAVIOR-SOURCE-DELAY",
                            "source delay behavior node requires a finite delay duration",
                        )
                    })?;
                    let buffer = DelayBuffer::new(
                        signal_name,
                        quantity_kind,
                        canonical_unit,
                        delay_s,
                        DelayInterpolationPolicy::Linear,
                        DelayInitialHistoryPolicy::HoldInitial,
                    )?;
                    BehaviorRhsNode::delay(
                        local.name.clone(),
                        source,
                        DelayBehaviorNode::new(buffer),
                    )
                }
                SourceBehaviorKind::Predictor => {
                    let contract = PredictorContract::new(
                        format!("source_predictor_{}", local.name),
                        vec![BehaviorSignalContract::new(
                            signal_name.clone(),
                            quantity_kind.clone(),
                            canonical_unit.clone(),
                        )],
                        vec![BehaviorSignalContract::new(
                            local.name.clone(),
                            quantity_kind,
                            canonical_unit,
                        )],
                        format!("sha256:source-identity-predictor-{}", local.name),
                        PredictorDifferentiability::Differentiable,
                        PredictorSolverPolicy {
                            explicit_call_only: true,
                            finite_difference_allowed: true,
                            jacobian_policy: PredictorJacobianPolicy::FiniteDifferenceAllowed,
                        },
                    )?;
                    BehaviorRhsNode::predictor(
                        local.name.clone(),
                        vec![source],
                        contract,
                        |inputs| Ok(vec![inputs[0]]),
                    )
                }
                SourceBehaviorKind::External => {
                    let contract = ExternalBehaviorContract::new(
                        format!("source_external_{}", local.name),
                        ExternalBehaviorKind::Function,
                        vec![BehaviorSignalContract::new(
                            signal_name.clone(),
                            quantity_kind.clone(),
                            canonical_unit.clone(),
                        )],
                        vec![BehaviorSignalContract::new(
                            local.name.clone(),
                            quantity_kind,
                            canonical_unit,
                        )],
                        format!("sha256:source-identity-external-{}", local.name),
                        ExternalBehaviorDeterminism::Deterministic,
                        ExternalBehaviorProfilePolicy {
                            safe_allowed: true,
                            repro_allowed: true,
                        },
                    )?;
                    BehaviorRhsNode::external(
                        local.name.clone(),
                        BehaviorExecutionProfile::Repro,
                        vec![source],
                        contract,
                        |inputs| Ok(vec![inputs[0]]),
                    )
                }
            };
            output_indices.insert(local.name.clone(), output_symbols.len());
            output_symbols.push(local.name.clone());
            nodes.push(node);
        }
    }
    if nodes.is_empty() {
        return Err(SolverFailure::new(
            "E-BEHAVIOR-SOURCE-SHAPE",
            "behavior graph source solve requires at least one behavior node",
        ));
    }
    Ok((BehaviorGraphRhsAdapter::new(nodes), output_symbols))
}

fn source_behavior_call(expression: &str) -> Result<Option<SourceBehaviorCall>, SolverFailure> {
    let trimmed = expression.trim();
    if let Some(arguments) = behavior_call_arguments_expression(trimmed, "delay") {
        let parts = split_behavior_arguments(&arguments);
        if parts.len() != 2 {
            return Err(SolverFailure::new(
                "E-BEHAVIOR-SOURCE-DELAY",
                "source delay behavior expression must use delay(signal, duration)",
            ));
        }
        let delay_s = parse_duration_seconds(parts[1].trim()).ok_or_else(|| {
            SolverFailure::new(
                "E-BEHAVIOR-SOURCE-DELAY",
                "source delay behavior expression requires a positive duration",
            )
        })?;
        return Ok(Some(SourceBehaviorCall {
            kind: SourceBehaviorKind::Delay,
            signal: parts[0].trim().to_owned(),
            delay_s: Some(delay_s),
        }));
    }
    for (name, kind) in [
        ("predictor", SourceBehaviorKind::Predictor),
        ("predict", SourceBehaviorKind::Predictor),
        ("external", SourceBehaviorKind::External),
        ("adapter", SourceBehaviorKind::External),
    ] {
        if let Some(arguments) = behavior_call_arguments_expression(trimmed, name) {
            let parts = split_behavior_arguments(&arguments);
            if parts.len() != 1 {
                return Err(SolverFailure::new(
                    "E-BEHAVIOR-SOURCE-CALL",
                    format!("source behavior expression `{name}` requires one signal argument"),
                ));
            }
            return Ok(Some(SourceBehaviorCall {
                kind,
                signal: parts[0].trim().to_owned(),
                delay_s: None,
            }));
        }
    }
    Ok(None)
}

fn behavior_call_arguments_expression(expression: &str, call_name: &str) -> Option<String> {
    let trimmed = expression.trim();
    let prefix = format!("{call_name}(");
    if !trimmed
        .to_ascii_lowercase()
        .starts_with(&prefix.to_ascii_lowercase())
        || !trimmed.ends_with(')')
    {
        return None;
    }
    let mut depth = 0usize;
    for (index, character) in trimmed.char_indices() {
        if character == '(' {
            depth += 1;
        } else if character == ')' {
            depth = depth.checked_sub(1)?;
            if depth == 0 && index + character.len_utf8() != trimmed.len() {
                return None;
            }
        }
    }
    (depth == 0).then(|| trimmed[prefix.len()..trimmed.len() - 1].to_owned())
}

fn split_behavior_arguments(arguments: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    for character in arguments.chars() {
        match character {
            '(' => {
                depth += 1;
                current.push(character);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                current.push(character);
            }
            ',' if depth == 0 => {
                parts.push(current.trim().to_owned());
                current.clear();
            }
            _ => current.push(character),
        }
    }
    if !current.trim().is_empty() || !arguments.trim().is_empty() {
        parts.push(current.trim().to_owned());
    }
    parts
}

fn source_behavior_signal_source(
    assembly: &EquationAssembly,
    component: &eng_compiler::ComponentInfo,
    current_line: usize,
    signal: &str,
    output_indices: &HashMap<String, usize>,
) -> Result<(BehaviorSignalSource, String, String, String), SolverFailure> {
    let signal = signal.trim();
    if let Some(index) = output_indices.get(signal) {
        return Ok((
            BehaviorSignalSource::BehaviorOutput {
                node_index: *index,
                output_index: 0,
            },
            signal.to_owned(),
            "DimensionlessNumber".to_owned(),
            "1".to_owned(),
        ));
    }
    if let Some(source) = source_behavior_variable_source(assembly, &component.name, signal) {
        return Ok(source);
    }
    if let Some(prior) = component
        .local_expressions
        .iter()
        .find(|local| local.name == signal && local.line < current_line)
    {
        return source_behavior_signal_source(
            assembly,
            component,
            current_line,
            &prior.expression,
            output_indices,
        );
    }
    Err(SolverFailure::new(
        "E-BEHAVIOR-SOURCE-SIGNAL",
        format!(
            "source behavior signal `{signal}` is not available in the dynamic component layout"
        ),
    ))
}

fn source_behavior_variable_source(
    assembly: &EquationAssembly,
    component_name: &str,
    signal: &str,
) -> Option<(BehaviorSignalSource, String, String, String)> {
    let candidates = if signal.contains('.') {
        vec![signal.to_owned(), format!("{component_name}.{signal}")]
    } else {
        vec![signal.to_owned()]
    };
    for candidate in candidates {
        if let Some((index, variable)) = assembly
            .states
            .iter()
            .enumerate()
            .find(|(_, variable)| variable.name == candidate)
        {
            return Some((
                BehaviorSignalSource::State(index),
                variable.name.clone(),
                variable.quantity_kind.clone(),
                variable.unit.clone(),
            ));
        }
        if let Some((index, variable)) = assembly
            .inputs
            .iter()
            .enumerate()
            .find(|(_, variable)| variable.name == candidate)
        {
            return Some((
                BehaviorSignalSource::Input(index),
                variable.name.clone(),
                variable.quantity_kind.clone(),
                variable.unit.clone(),
            ));
        }
        if let Some((index, variable)) = assembly
            .parameters
            .iter()
            .enumerate()
            .find(|(_, variable)| variable.name == candidate)
        {
            return Some((
                BehaviorSignalSource::Parameter(index),
                variable.name.clone(),
                variable.quantity_kind.clone(),
                variable.unit.clone(),
            ));
        }
    }
    None
}

fn source_behavior_symbols(
    assembly: &EquationAssembly,
    time_s: f64,
    state: &[f64],
    inputs: &[SolverScalar],
    parameters: &[SolverScalar],
    behavior: &crate::solver::BehaviorGraphEvaluation,
    behavior_output_symbols: &[String],
) -> Result<HashMap<String, f64>, SolverFailure> {
    let mut symbols = HashMap::new();
    symbols.insert("t".to_owned(), time_s);
    symbols.insert("time".to_owned(), time_s);
    for (variable, value) in assembly.states.iter().zip(state.iter().copied()) {
        symbols.insert(variable.name.clone(), value);
    }
    for input in inputs {
        symbols.insert(input.name.clone(), input.value);
    }
    for parameter in parameters {
        symbols.insert(parameter.name.clone(), parameter.value);
    }
    for (index, symbol) in behavior_output_symbols.iter().enumerate() {
        let value = behavior.output(index, 0).ok_or_else(|| {
            SolverFailure::new(
                "E-BEHAVIOR-SOURCE-OUTPUT",
                format!("source behavior node `{symbol}` did not produce output 0"),
            )
        })?;
        symbols.insert(symbol.clone(), value);
    }
    Ok(symbols)
}

fn source_behavior_derivatives(
    assembly: &EquationAssembly,
    symbols: &HashMap<String, f64>,
) -> Result<Vec<f64>, SolverFailure> {
    let mut derivatives = Vec::with_capacity(assembly.states.len());
    let derivative_names = assembly
        .states
        .iter()
        .map(|state| format!("der({})", state.name))
        .collect::<Vec<_>>();
    for (state_index, state) in assembly.states.iter().enumerate() {
        let derivative_name = &derivative_names[state_index];
        let equation = assembly
            .generated_equations
            .iter()
            .find(|equation| {
                equation
                    .dependencies
                    .iter()
                    .any(|dependency| dependency == derivative_name)
            })
            .ok_or_else(|| {
                SolverFailure::new(
                    "E-BEHAVIOR-SOURCE-RHS-SHAPE",
                    format!(
                        "behavior graph source solve could not find derivative residual for `{}`",
                        state.name
                    ),
                )
            })?;
        let mut base_symbols = symbols.clone();
        for name in &derivative_names {
            base_symbols.insert(name.clone(), 0.0);
        }
        let base = evaluate_source_arithmetic_expression(&equation.residual, &base_symbols)?;
        let mut coefficient_symbols = base_symbols.clone();
        coefficient_symbols.insert(derivative_name.clone(), 1.0);
        let coefficient =
            evaluate_source_arithmetic_expression(&equation.residual, &coefficient_symbols)? - base;
        if !coefficient.is_finite() || coefficient.abs() <= f64::EPSILON {
            return Err(SolverFailure::new(
                "E-BEHAVIOR-SOURCE-RHS-SHAPE",
                format!(
                    "behavior graph source residual `{}` does not contain a solvable derivative coefficient",
                    equation.name
                ),
            ));
        }
        let target = equation.rhs_value.unwrap_or(0.0);
        let derivative = (target - base) / coefficient;
        if !derivative.is_finite() {
            return Err(SolverFailure::new(
                "E-BEHAVIOR-SOURCE-RHS-FINITE",
                format!(
                    "behavior graph source residual `{}` produced a non-finite derivative",
                    equation.name
                ),
            ));
        }
        derivatives.push(derivative);
    }
    Ok(derivatives)
}

fn explicit_dynamic_component_source_assembly(
    assembly: &EquationAssembly,
) -> Result<EquationAssembly, SolverFailure> {
    let generated_equations = assembly
        .generated_equations
        .iter()
        .filter(|equation| {
            equation
                .dependencies
                .iter()
                .any(|dependency| dependency.trim().starts_with("der("))
        })
        .cloned()
        .collect::<Vec<_>>();
    if generated_equations.is_empty() {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-EXPLICIT-SHAPE",
            "explicit dynamic component source solve requires at least one derivative residual",
        ));
    }
    if assembly.states.is_empty() {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-EXPLICIT-SHAPE",
            "explicit dynamic component source solve requires at least one state variable",
        ));
    }
    let mut explicit = assembly.clone();
    explicit.generated_equations = generated_equations;
    explicit.algebraic_variables.clear();
    explicit.unknowns = explicit.states.clone();
    Ok(explicit)
}

fn dynamic_component_solve_input_from_request(
    assembly: &EquationAssembly,
    split: &crate::solver::assembly::DynamicComponentAssemblySplit,
    options: &[eng_compiler::WithOptionInfo],
) -> Result<DynamicComponentAssemblySolveInput, SolverFailure> {
    let timestep_s = option_value(options, "timestep")
        .and_then(parse_duration_seconds)
        .ok_or_else(|| {
            SolverFailure::new(
                "E-DYNAMIC-COMPONENT-SOURCE-TIMESTEP",
                "dynamic component source solve requires a positive timestep duration",
            )
        })?;
    let duration_s = option_value(options, "duration")
        .and_then(parse_duration_seconds)
        .ok_or_else(|| {
            SolverFailure::new(
                "E-DYNAMIC-COMPONENT-SOURCE-DURATION",
                "dynamic component source solve requires a positive duration",
            )
        })?;
    let initial_state =
        component_initial_values_from_options(options, "initial", &assembly.states, 0.0)?;
    let initial_algebraic = component_initial_values_from_options(
        options,
        "initial_algebraic",
        &assembly.algebraic_variables,
        0.0,
    )?;
    if initial_algebraic.len() != split.algebraic_layout.len() {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-SOURCE-INITIAL-LAYOUT",
            format!(
                "dynamic component source option `initial_algebraic` materialized {} value(s) for {} algebraic variable(s)",
                initial_algebraic.len(),
                split.algebraic_layout.len()
            ),
        ));
    }
    let inputs = assembly
        .inputs
        .iter()
        .map(|input| {
            Ok(SolverScalar::new(
                input.name.clone(),
                input.quantity_kind.clone(),
                input.unit.clone(),
                0.0,
            ))
        })
        .collect::<Result<Vec<_>, SolverFailure>>()?;
    let parameters = assembly
        .parameters
        .iter()
        .map(|parameter| {
            Ok(SolverScalar::new(
                parameter.name.clone(),
                parameter.quantity_kind.clone(),
                parameter.unit.clone(),
                parameter.value.unwrap_or(0.0),
            ))
        })
        .collect::<Result<Vec<_>, SolverFailure>>()?;

    Ok(DynamicComponentAssemblySolveInput {
        duration_s,
        timestep_s,
        initial_state,
        initial_algebraic,
        inputs,
        parameters,
    })
}

fn derivative_variables_for_states(states: &[UnknownVariable]) -> Vec<UnknownVariable> {
    states
        .iter()
        .map(|state| UnknownVariable {
            name: format!("der({})", state.name),
            role: "state_derivative".to_owned(),
            quantity_kind: state.quantity_kind.clone(),
            unit: derivative_unit_for_state_unit(&state.unit),
            source: state.source.clone(),
            status: state.status.clone(),
            value: None,
        })
        .collect()
}

fn derivative_unit_for_state_unit(unit: &str) -> String {
    let unit = unit.trim();
    if unit.is_empty() || unit == "1" {
        "1/s".to_owned()
    } else {
        format!("{unit}/s")
    }
}
fn component_initial_values_from_options(
    options: &[eng_compiler::WithOptionInfo],
    key: &str,
    variables: &[UnknownVariable],
    default_value: f64,
) -> Result<Vec<f64>, SolverFailure> {
    let Some(value) = option_value(options, key) else {
        return Ok(vec![default_value; variables.len()]);
    };
    let Some((is_vector, values)) = parse_component_initial_option(value) else {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-SOURCE-INITIAL",
            format!(
                "dynamic component source option `{key}` is not a finite numeric literal or bracketed list"
            ),
        ));
    };
    if is_vector && values.len() != variables.len() {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-SOURCE-INITIAL-LAYOUT",
            format!(
                "dynamic component source option `{key}` provided {} initial value(s) for {} variable(s)",
                values.len(),
                variables.len()
            ),
        ));
    }
    variables
        .iter()
        .enumerate()
        .map(|(index, variable)| {
            let (raw_value, unit) = if is_vector {
                values[index].clone()
            } else {
                values[0].clone()
            };
            if !raw_value.is_finite() {
                return Err(SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-SOURCE-INITIAL",
                    format!("dynamic component source option `{key}` is not finite"),
                ));
            }
            let source_unit = unit.as_deref().unwrap_or(variable.unit.as_str());
            Ok(convert_display_value(
                raw_value,
                source_unit,
                &variable.unit,
            ))
        })
        .collect()
}

fn parse_component_initial_option(value: &str) -> Option<(bool, Vec<(f64, Option<String>)>)> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(inner) = trimmed
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    {
        let values = split_component_initial_items(inner)
            .into_iter()
            .map(|item| parse_numeric_value_with_optional_unit(&item))
            .collect::<Option<Vec<_>>>()?;
        if values.is_empty() {
            return None;
        }
        return Some((true, values));
    }
    parse_numeric_value_with_optional_unit(trimmed).map(|value| (false, vec![value]))
}

fn split_component_initial_items(text: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;
    for (index, character) in text.char_indices() {
        match character {
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' => depth -= 1,
            ',' if depth == 0 => {
                let item = text[start..index].trim();
                if !item.is_empty() {
                    items.push(item.to_owned());
                }
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    let item = text[start..].trim();
    if !item.is_empty() {
        items.push(item.to_owned());
    }
    items
}

fn failed_dynamic_component_source_solution(
    assembly: &EquationAssembly,
    method: &str,
    options: &DynamicComponentOptions,
    failure: &SolverFailure,
    reason: &str,
) -> RuntimeComponentSolution {
    let variables = assembly
        .states
        .iter()
        .chain(assembly.algebraic_variables.iter())
        .map(|variable| RuntimeComponentVariableSolution {
            name: variable.name.clone(),
            role: variable.role.clone(),
            value: 0.0,
            unit: variable.unit.clone(),
            status: "trajectory_failed".to_owned(),
        })
        .collect::<Vec<_>>();
    RuntimeComponentSolution {
        assembly: assembly.name.clone(),
        status: "failed".to_owned(),
        method: method.to_owned(),
        reason: reason.to_owned(),
        equation_count: assembly.equation_count(),
        unknown_count: assembly.unknown_count(),
        residual_norm: f64::INFINITY,
        tolerance: options.algebraic.tolerance,
        max_iterations: options.algebraic.max_iterations,
        iteration_count: 0,
        convergence_status: "dynamic_component_source_failed".to_owned(),
        variables,
        trajectories: Vec::new(),
        step_diagnostics: Vec::new(),
        residuals: Vec::new(),
        largest_residuals: Vec::new(),
        failure_artifact: Some(RuntimeSolverFailureArtifact {
            code: failure.code.clone(),
            message: failure.message.clone(),
        }),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourceResidualSubset {
    All,
    AlgebraicOnly,
}

#[derive(Clone, Debug, PartialEq)]
struct SourceResidualEvaluation {
    residuals: Vec<RuntimeComponentResidualEvaluation>,
    normalized_values: Vec<f64>,
    residual_norm: f64,
}

fn source_algebraic_residual_values(
    assembly: &EquationAssembly,
    graph: &ResidualGraph,
    values: &[f64],
) -> Result<Vec<f64>, SolverFailure> {
    source_residual_evaluation_for_unknowns(assembly, graph, values, DEFAULT_NEWTON_TOLERANCE)
        .map(|evaluation| evaluation.normalized_values)
}

const DEFAULT_NEWTON_TOLERANCE: f64 = 1e-9;

fn source_residual_evaluation_for_unknowns(
    assembly: &EquationAssembly,
    graph: &ResidualGraph,
    values: &[f64],
    tolerance: f64,
) -> Result<SourceResidualEvaluation, SolverFailure> {
    if values.len() != assembly.unknowns.len() {
        return Err(SolverFailure::new(
            "E-SOURCE-RESIDUAL-LAYOUT",
            "source residual value vector length does not match assembly unknown count",
        ));
    }
    let mut symbols = assembly
        .unknowns
        .iter()
        .zip(values.iter().copied())
        .map(|(variable, value)| (variable.name.clone(), value))
        .collect::<HashMap<_, _>>();
    insert_assembly_parameter_symbols(assembly, &mut symbols, None)?;
    source_residual_evaluation_with_symbols(
        assembly,
        graph,
        &symbols,
        tolerance,
        SourceResidualSubset::All,
    )
}

fn source_dae_residual_values(
    assembly: &EquationAssembly,
    graph: &ResidualGraph,
    sample: DaeSample<'_>,
    subset: SourceResidualSubset,
) -> Result<Vec<f64>, SolverFailure> {
    source_residual_evaluation_for_dae_sample(
        assembly,
        graph,
        sample,
        DEFAULT_NEWTON_TOLERANCE,
        subset,
    )
    .map(|evaluation| evaluation.normalized_values)
}

fn source_residual_evaluation_for_dae_sample(
    assembly: &EquationAssembly,
    graph: &ResidualGraph,
    sample: DaeSample<'_>,
    tolerance: f64,
    subset: SourceResidualSubset,
) -> Result<SourceResidualEvaluation, SolverFailure> {
    if sample.state.len() != assembly.states.len()
        || sample.state_derivative.len() != assembly.states.len()
        || sample.algebraic.len() != assembly.algebraic_variables.len()
        || (!sample.parameters.is_empty() && sample.parameters.len() != assembly.parameters.len())
    {
        return Err(SolverFailure::new(
            "E-SOURCE-RESIDUAL-LAYOUT",
            "source DAE residual sample layout does not match assembly state/algebraic split",
        ));
    }
    let mut symbols = HashMap::new();
    symbols.insert("t".to_owned(), sample.time_s);
    symbols.insert("time".to_owned(), sample.time_s);
    for (state, value) in assembly.states.iter().zip(sample.state.iter().copied()) {
        symbols.insert(state.name.clone(), value);
    }
    for (state, value) in assembly
        .states
        .iter()
        .zip(sample.state_derivative.iter().copied())
    {
        symbols.insert(format!("der({})", state.name), value);
    }
    for (variable, value) in assembly
        .algebraic_variables
        .iter()
        .zip(sample.algebraic.iter().copied())
    {
        symbols.insert(variable.name.clone(), value);
    }
    insert_assembly_parameter_symbols(assembly, &mut symbols, Some(sample.parameters))?;
    source_residual_evaluation_with_symbols(assembly, graph, &symbols, tolerance, subset)
}

fn insert_assembly_parameter_symbols(
    assembly: &EquationAssembly,
    symbols: &mut HashMap<String, f64>,
    sample_values: Option<&[f64]>,
) -> Result<(), SolverFailure> {
    for (index, parameter) in assembly.parameters.iter().enumerate() {
        let value = sample_values
            .and_then(|values| values.get(index).copied())
            .or(parameter.value)
            .ok_or_else(|| {
                SolverFailure::new(
                    "E-SOURCE-RESIDUAL-PARAMETER",
                    format!(
                        "source residual parameter `{}` has no materialized value",
                        parameter.name
                    ),
                )
            })?;
        symbols.insert(parameter.name.clone(), value);
    }
    Ok(())
}

fn source_residual_evaluation_for_dae_final(
    assembly: &EquationAssembly,
    graph: &ResidualGraph,
    input: &DaeInput,
    solver_result: &SolverResult,
    tolerance: f64,
) -> Result<SourceResidualEvaluation, SolverFailure> {
    let state = solver_result
        .output
        .state_trajectories
        .iter()
        .map(|trajectory| trajectory.values.last().copied().unwrap_or(0.0))
        .collect::<Vec<_>>();
    let state_derivative = solver_result
        .output
        .state_trajectories
        .iter()
        .enumerate()
        .map(|(index, trajectory)| {
            if trajectory.values.len() >= 2 {
                let last = trajectory.values[trajectory.values.len() - 1];
                let previous = trajectory.values[trajectory.values.len() - 2];
                (last - previous) / solver_result.time_grid.timestep_s
            } else {
                input
                    .initial_state_derivatives
                    .get(index)
                    .copied()
                    .unwrap_or(0.0)
            }
        })
        .collect::<Vec<_>>();
    let algebraic = solver_result
        .output
        .algebraic_trajectories
        .iter()
        .map(|trajectory| trajectory.values.last().copied().unwrap_or(0.0))
        .collect::<Vec<_>>();
    source_residual_evaluation_for_dae_sample(
        assembly,
        graph,
        DaeSample {
            time_s: solver_result.time_grid.duration_s,
            state: &state,
            state_derivative: &state_derivative,
            mass_state_derivative: None,
            algebraic: &algebraic,
            inputs: &[],
            parameters: &[],
        },
        tolerance,
        SourceResidualSubset::All,
    )
}

fn source_residual_evaluation_with_symbols(
    assembly: &EquationAssembly,
    graph: &ResidualGraph,
    symbols: &HashMap<String, f64>,
    tolerance: f64,
    subset: SourceResidualSubset,
) -> Result<SourceResidualEvaluation, SolverFailure> {
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(SolverFailure::new(
            "E-SOURCE-RESIDUAL-TOLERANCE",
            "source residual tolerance must be a positive finite number",
        ));
    }
    let metadata = graph
        .residuals
        .iter()
        .map(|residual| (residual.name.as_str(), residual))
        .collect::<HashMap<_, _>>();
    let mut residuals = Vec::new();
    let mut normalized_values = Vec::new();
    for equation in &assembly.generated_equations {
        if subset == SourceResidualSubset::AlgebraicOnly
            && equation
                .dependencies
                .iter()
                .any(|dependency| dependency.trim().starts_with("der("))
        {
            continue;
        }
        let raw_expression_value =
            evaluate_source_arithmetic_expression(&equation.residual, symbols).map_err(
                |failure| {
                    SolverFailure::new(
                        failure.code,
                        format!("{} in residual `{}`", failure.message, equation.name),
                    )
                },
            )?;
        let value = raw_expression_value - equation.rhs_value.unwrap_or(0.0);
        if !value.is_finite() {
            return Err(SolverFailure::new(
                "E-SOURCE-RESIDUAL-FINITE",
                format!(
                    "source residual `{}` evaluated to a non-finite value",
                    equation.name
                ),
            ));
        }
        let Some(residual_metadata) = metadata.get(equation.name.as_str()) else {
            return Err(SolverFailure::new(
                "E-SOURCE-RESIDUAL-METADATA",
                format!(
                    "source residual `{}` has no residual graph metadata",
                    equation.name
                ),
            ));
        };
        let scale = residual_metadata.scale.value;
        if !scale.is_finite() || scale <= 0.0 {
            return Err(SolverFailure::new(
                "E-SOURCE-RESIDUAL-SCALE",
                format!("source residual `{}` has an invalid scale", equation.name),
            ));
        }
        let normalized_value = value / scale.max(f64::EPSILON);
        if !normalized_value.is_finite() {
            return Err(SolverFailure::new(
                "E-SOURCE-RESIDUAL-FINITE",
                format!(
                    "source residual `{}` normalized value is non-finite",
                    equation.name
                ),
            ));
        }
        normalized_values.push(normalized_value);
        residuals.push(RuntimeComponentResidualEvaluation {
            name: equation.name.clone(),
            expression: equation.residual.clone(),
            value,
            unit: residual_metadata.unit.unit.clone(),
            normalized_value,
            scale,
            scale_policy: residual_metadata.scale.policy.clone(),
            status: if normalized_value.abs() <= tolerance {
                "satisfied".to_owned()
            } else {
                "unsatisfied".to_owned()
            },
        });
    }
    if residuals.is_empty() {
        return Err(SolverFailure::new(
            "E-SOURCE-RESIDUAL-SHAPE",
            "source residual evaluation selected no residual equations",
        ));
    }
    let residual_norm = crate::solver::euclidean_norm(&normalized_values);
    if !residual_norm.is_finite() {
        return Err(SolverFailure::new(
            "E-SOURCE-RESIDUAL-FINITE",
            "source residual norm is non-finite",
        ));
    }
    Ok(SourceResidualEvaluation {
        residuals,
        normalized_values,
        residual_norm,
    })
}

fn source_linear_terms_jacobian(graph: &ResidualGraph) -> Result<Vec<Vec<f64>>, SolverFailure> {
    let system = graph.assemble_linear_system()?;
    Ok(system
        .matrix
        .iter()
        .zip(graph.residuals.iter())
        .map(|(row, residual)| {
            row.iter()
                .map(|value| value / residual.scale.value.max(f64::EPSILON))
                .collect::<Vec<_>>()
        })
        .collect())
}

fn newton_options_from_solve_request(options: &[eng_compiler::WithOptionInfo]) -> NewtonOptions {
    let mut newton = NewtonOptions::default();
    if let Some(tolerance) = option_value(options, "tolerance")
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
    {
        newton.tolerance = tolerance;
    }
    if let Some(max_iterations) = option_value(options, "max_iter")
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
    {
        newton.max_iterations = max_iterations;
    }
    if let Some(step) = option_value(options, "finite_difference_step")
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
    {
        newton.finite_difference_step = step;
    }
    if let Some(damping) = option_value(options, "damping")
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0 && *value <= 1.0)
    {
        newton.damping = damping;
    }
    if let Some(steps) = option_value(options, "line_search_steps")
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
    {
        newton.line_search_steps = steps;
    }
    newton
}

fn newton_residual_history_diagnostics(
    newton: &crate::solver::NewtonResult,
    failure: Option<&SolverFailure>,
) -> Vec<RuntimeComponentStepDiagnostic> {
    newton
        .residual_history
        .iter()
        .enumerate()
        .map(|(index, residual_norm)| RuntimeComponentStepDiagnostic {
            step_index: index,
            time_s: 0.0,
            algebraic_iteration_count: index,
            residual_norm: *residual_norm,
            convergence_status: if index + 1 == newton.residual_history.len() {
                newton.convergence_status.clone()
            } else {
                "newton_iteration".to_owned()
            },
            failure_artifact: (index + 1 == newton.residual_history.len())
                .then(|| failure)
                .flatten()
                .map(|failure| RuntimeSolverFailureArtifact {
                    code: failure.code.clone(),
                    message: failure.message.clone(),
                }),
        })
        .collect()
}

fn failed_source_component_solution(
    assembly: &EquationAssembly,
    method: &str,
    reason: &str,
    failure: &SolverFailure,
    tolerance: f64,
    max_iterations: usize,
    convergence_status: &str,
) -> RuntimeComponentSolution {
    let variables = assembly
        .unknowns
        .iter()
        .map(|variable| RuntimeComponentVariableSolution {
            name: variable.name.clone(),
            role: variable.role.clone(),
            value: 0.0,
            unit: variable.unit.clone(),
            status: "source_solve_failed".to_owned(),
        })
        .collect::<Vec<_>>();
    RuntimeComponentSolution {
        assembly: assembly.name.clone(),
        status: "failed".to_owned(),
        method: method.to_owned(),
        reason: reason.to_owned(),
        equation_count: assembly.equation_count(),
        unknown_count: assembly.unknown_count(),
        residual_norm: f64::INFINITY,
        tolerance,
        max_iterations,
        iteration_count: 0,
        convergence_status: convergence_status.to_owned(),
        variables,
        trajectories: Vec::new(),
        step_diagnostics: Vec::new(),
        residuals: Vec::new(),
        largest_residuals: Vec::new(),
        failure_artifact: Some(RuntimeSolverFailureArtifact {
            code: failure.code.clone(),
            message: failure.message.clone(),
        }),
    }
}

fn layout_names_from_unknowns(variables: &[UnknownVariable]) -> Vec<String> {
    variables
        .iter()
        .map(|variable| variable.name.clone())
        .collect()
}

fn fixed_point_options_from_solve_request(
    options: &[eng_compiler::WithOptionInfo],
) -> FixedPointOptions {
    let mut fixed_point = FixedPointOptions::default();
    if let Some(tolerance) = option_value(options, "tolerance")
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
    {
        fixed_point.tolerance = tolerance;
    }
    if let Some(max_iterations) = option_value(options, "max_iter")
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
    {
        fixed_point.max_iterations = max_iterations;
    }
    if let Some(relaxation) = option_value(options, "relaxation")
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0 && *value <= 1.0)
    {
        fixed_point.relaxation = relaxation;
    }
    fixed_point
}

fn fixed_point_initial_value_from_solve_request(options: &[eng_compiler::WithOptionInfo]) -> f64 {
    option_value(options, "initial")
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(0.0)
}

#[derive(Clone, Debug, PartialEq)]
struct FixedPointPivot {
    variable_index: usize,
    residual_index: usize,
    coefficient: f64,
}

fn fixed_point_update_plan(graph: &ResidualGraph) -> Result<Vec<FixedPointPivot>, SolverFailure> {
    let variable_count = graph.variables.len();
    let residual_count = graph.residuals.len();
    if variable_count == 0 || residual_count == 0 || variable_count != residual_count {
        return Err(SolverFailure::new(
            "E-FIXED-POINT-GRAPH-SHAPE",
            format!(
                "fixed-point residual graph requires a non-empty square system, got {residual_count} residual(s) and {variable_count} variable(s)"
            ),
        ));
    }

    let mut residual_order = (0..residual_count).collect::<Vec<_>>();
    residual_order.sort_by_key(|index| {
        let residual = &graph.residuals[*index];
        let source_priority = residual
            .source
            .generated_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("component-local equation"))
            .then_some(1)
            .unwrap_or(2);
        if residual.terms.len() == 1 {
            (0, residual.terms.len())
        } else {
            (source_priority, residual.terms.len())
        }
    });

    let mut used_variables = HashSet::new();
    let mut used_residuals = HashSet::new();
    let mut pivots = Vec::with_capacity(variable_count);
    for residual_index in residual_order {
        let residual = &graph.residuals[residual_index];
        let Some(term) = residual.terms.iter().find(|term| {
            term.variable_index < variable_count
                && !used_variables.contains(&term.variable_index)
                && term.coefficient.is_finite()
                && term.coefficient.abs() > 1e-12
        }) else {
            continue;
        };
        used_variables.insert(term.variable_index);
        used_residuals.insert(residual_index);
        pivots.push(FixedPointPivot {
            variable_index: term.variable_index,
            residual_index,
            coefficient: term.coefficient,
        });
    }

    if pivots.len() != variable_count {
        let missing = graph
            .variables
            .iter()
            .filter(|variable| !used_variables.contains(&variable.index))
            .map(|variable| variable.name.clone())
            .collect::<Vec<_>>();
        return Err(SolverFailure::new(
            "E-FIXED-POINT-GRAPH-PIVOT",
            format!(
                "fixed-point residual graph could not assign one update equation per variable; missing pivot(s): {}",
                missing.join(", ")
            ),
        ));
    }
    if used_residuals.len() != residual_count {
        return Err(SolverFailure::new(
            "E-FIXED-POINT-GRAPH-PIVOT",
            "fixed-point residual graph could not assign each residual to one variable update",
        ));
    }
    pivots.sort_by_key(|pivot| pivot.variable_index);
    Ok(pivots)
}

fn fixed_point_update_values(
    graph: &ResidualGraph,
    update_plan: &[FixedPointPivot],
    values: &[f64],
) -> Result<Vec<f64>, SolverFailure> {
    if values.len() != graph.variables.len() {
        return Err(SolverFailure::new(
            "E-FIXED-POINT-LAYOUT",
            "fixed-point update input length does not match residual graph variables",
        ));
    }
    let mut next = values.to_vec();
    for pivot in update_plan {
        let Some(residual) = graph.residuals.get(pivot.residual_index) else {
            return Err(SolverFailure::new(
                "E-FIXED-POINT-GRAPH-PIVOT",
                "fixed-point update references an unknown residual",
            ));
        };
        if !residual.rhs_value.is_finite() {
            return Err(SolverFailure::new(
                "E-FIXED-POINT-VALUE",
                format!("residual `{}` has a non-finite RHS value", residual.name),
            ));
        }
        let mut rhs = residual.rhs_value;
        for term in &residual.terms {
            if term.variable_index == pivot.variable_index {
                continue;
            }
            let Some(value) = values.get(term.variable_index) else {
                return Err(SolverFailure::new(
                    "E-FIXED-POINT-LAYOUT",
                    format!(
                        "fixed-point residual `{}` references variable index {} outside the update vector",
                        residual.name, term.variable_index
                    ),
                ));
            };
            if !term.coefficient.is_finite() || !value.is_finite() {
                return Err(SolverFailure::new(
                    "E-FIXED-POINT-VALUE",
                    format!(
                        "fixed-point residual `{}` contains a non-finite update term",
                        residual.name
                    ),
                ));
            }
            rhs -= term.coefficient * value;
        }
        next[pivot.variable_index] = rhs / pivot.coefficient;
    }
    Ok(next)
}

fn annotate_component_behavior_solution(
    solution: &mut RuntimeComponentSolution,
    assembly: &eng_compiler::ComponentAssemblyInfo,
) {
    if !component_assembly_has_behavior_seed(assembly) {
        return;
    }
    if solution.method.starts_with("behavior_graph_")
        || solution.convergence_status.starts_with("behavior_graph_")
    {
        return;
    }
    append_component_solution_reason(solution, COMPONENT_BEHAVIOR_NOT_INTEGRATED_NOTE);
    if solution.failure_artifact.is_none() {
        solution.status = "not_solved_behavior_not_integrated".to_owned();
        solution.convergence_status = "behavior_graph_not_integrated".to_owned();
        for variable in &mut solution.variables {
            if variable.status == "solved_linear" {
                variable.status = "behavior_not_integrated".to_owned();
            }
        }
        solution.failure_artifact = Some(RuntimeSolverFailureArtifact {
            code: COMPONENT_BEHAVIOR_NOT_INTEGRATED_CODE.to_owned(),
            message: COMPONENT_BEHAVIOR_NOT_INTEGRATED_NOTE.to_owned(),
        });
    } else if let Some(failure) = &mut solution.failure_artifact {
        if !failure
            .message
            .contains(COMPONENT_BEHAVIOR_NOT_INTEGRATED_NOTE)
        {
            failure.message = format!(
                "{}; {}",
                failure.message, COMPONENT_BEHAVIOR_NOT_INTEGRATED_NOTE
            );
        }
    }
}

fn component_assembly_has_behavior_seed(assembly: &eng_compiler::ComponentAssemblyInfo) -> bool {
    assembly.solver_preview.delay_history != "deferred_no_delay_calls"
        || assembly.solver_preview.predictor != "deferred_no_predictor_calls"
        || assembly.solver_preview.external_adapter != "deferred_no_external_behavior_adapter"
}

fn append_component_solution_reason(solution: &mut RuntimeComponentSolution, note: &str) {
    if !solution.reason.contains(note) {
        solution.reason = format!("{}; {}", solution.reason, note);
    }
}

#[allow(dead_code)]
fn component_trajectory_from_solver_trajectory(
    trajectory: &StateTrajectory,
    role: &str,
    time_grid: &TimeGrid,
) -> RuntimeComponentTrajectory {
    let points = trajectory
        .time_value_points(time_grid)
        .into_iter()
        .map(|(x, y)| RuntimePoint { x, y })
        .collect::<Vec<_>>();
    RuntimeComponentTrajectory {
        name: trajectory.name.clone(),
        role: role.to_owned(),
        quantity_kind: trajectory.quantity_kind.clone(),
        unit: trajectory.canonical_unit.clone(),
        initial_value: trajectory.initial_value().unwrap_or(0.0),
        final_value: trajectory.final_value().unwrap_or(0.0),
        point_count: points.len(),
        points,
    }
}

fn solver_equation_assembly_from_component_info(
    report: &CheckReport,
    assembly: &eng_compiler::ComponentAssemblyInfo,
) -> EquationAssembly {
    let components = report
        .semantic_program
        .components
        .iter()
        .map(|component| ComponentInstance {
            name: component.name.clone(),
            component_type: "component".to_owned(),
            ports: component
                .ports
                .iter()
                .map(|port| PortInstance {
                    name: port.name.clone(),
                    component: component.name.clone(),
                    domain: port.domain.clone(),
                    medium: port.type_arguments.first().cloned(),
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    let ports = components
        .iter()
        .flat_map(|component| component.ports.iter().cloned())
        .collect::<Vec<_>>();
    let connections = report
        .semantic_program
        .connections
        .iter()
        .map(|connection| ConnectionEdge {
            from: connection.left.clone(),
            to: connection.right.clone(),
            source_line: connection.line,
        })
        .collect::<Vec<_>>();
    let connection_sets = assembly
        .connection_sets
        .iter()
        .map(|connection_set| ConnectionSet {
            name: connection_set.name.clone(),
            domain: connection_set.domain.clone(),
            ports: connection_set.ports.clone(),
        })
        .collect::<Vec<_>>();
    let generated_equations = assembly
        .equations
        .iter()
        .map(|equation| GeneratedEquation {
            name: equation.name.clone(),
            kind: equation.kind.clone(),
            domain: equation.domain.clone(),
            expression: equation.expression.clone(),
            residual: equation.residual.clone(),
            rhs_value: equation
                .rhs
                .as_ref()
                .and_then(|rhs| component_equation_rhs_value(report, assembly, equation, rhs)),
            dependencies: equation.dependencies.clone(),
            source: if equation.kind == "component_boundary" {
                "component_local_expression".to_owned()
            } else if equation.kind == "component_equation" {
                "component_local_equation".to_owned()
            } else {
                "component_connection".to_owned()
            },
            reason: equation.reason.clone(),
            source_line: Some(equation.line),
            status: equation.status.clone(),
        })
        .collect::<Vec<_>>();
    let variables = assembly
        .variables
        .iter()
        .map(|variable| {
            let (quantity_kind, unit) = assembly_variable_quantity_unit(report, variable);
            let value = assembly_variable_value(report, variable, &unit);
            UnknownVariable {
                name: variable.name.clone(),
                role: variable.role.clone(),
                quantity_kind,
                unit,
                source: variable.source.clone(),
                status: variable.status.clone(),
                value,
            }
        })
        .collect::<Vec<_>>();
    let unknowns = variables
        .iter()
        .filter(|variable| variable.role == "state" || variable.role == "algebraic")
        .cloned()
        .collect::<Vec<_>>();
    let states = variables
        .iter()
        .filter(|variable| variable.role == "state")
        .cloned()
        .collect::<Vec<_>>();
    let algebraic_variables = variables
        .iter()
        .filter(|variable| variable.role == "algebraic")
        .cloned()
        .collect::<Vec<_>>();
    let inputs = variables
        .iter()
        .filter(|variable| variable.role == "input")
        .cloned()
        .collect::<Vec<_>>();
    let parameters = variables
        .iter()
        .filter(|variable| variable.role == "parameter")
        .cloned()
        .collect::<Vec<_>>();

    EquationAssembly {
        name: assembly.name.clone(),
        components,
        ports,
        connections,
        connection_sets,
        generated_equations,
        component_equations: Vec::<ComponentEquation>::new(),
        unknowns,
        states,
        algebraic_variables,
        inputs,
        parameters,
    }
}

fn component_equation_rhs_value(
    report: &CheckReport,
    assembly: &eng_compiler::ComponentAssemblyInfo,
    equation: &eng_compiler::ComponentAssemblyEquationInfo,
    rhs: &str,
) -> Option<f64> {
    let dependency = equation.dependencies.first()?;
    let variable = assembly
        .variables
        .iter()
        .find(|variable| variable.name == *dependency)?;
    let (_quantity_kind, display_unit) = assembly_variable_quantity_unit(report, variable);
    let (value, unit) = parse_numeric_value_with_optional_unit(rhs)?;
    let source_unit = unit.as_deref().unwrap_or(display_unit.as_str());
    Some(convert_display_value(value, source_unit, &display_unit))
}

fn parse_numeric_value_with_optional_unit(value: &str) -> Option<(f64, Option<String>)> {
    let mut parts = value.split_whitespace();
    let number = parts.next()?.parse::<f64>().ok()?;
    let unit = parts.next().map(str::to_owned);
    Some((number, unit))
}

fn assembly_variable_value(
    report: &CheckReport,
    variable: &eng_compiler::ComponentAssemblyVariableInfo,
    display_unit: &str,
) -> Option<f64> {
    if variable.role != "parameter" {
        return None;
    }
    let (component_name, parameter_name) = variable.name.split_once('.')?;
    let parameter = report
        .semantic_program
        .components
        .iter()
        .find(|component| component.name == component_name)
        .and_then(|component| {
            component
                .parameters
                .iter()
                .find(|parameter| parameter.name == parameter_name)
        })?;
    let (value, unit) = parse_numeric_value_with_optional_unit(parameter.value.as_deref()?)?;
    let source_unit = unit.as_deref().unwrap_or(display_unit);
    Some(convert_display_value(value, source_unit, display_unit))
}

fn assembly_variable_quantity_unit(
    report: &CheckReport,
    variable: &eng_compiler::ComponentAssemblyVariableInfo,
) -> (String, String) {
    if variable.role == "parameter" {
        if let Some((component_name, parameter_name)) = variable.name.split_once('.') {
            if let Some(parameter) = report
                .semantic_program
                .components
                .iter()
                .find(|component| component.name == component_name)
                .and_then(|component| {
                    component
                        .parameters
                        .iter()
                        .find(|parameter| parameter.name == parameter_name)
                })
            {
                return (
                    parameter.quantity_kind.clone(),
                    parameter.display_unit.clone(),
                );
            }
        }
    }
    let Some((domain_name, variable_name)) = variable.source.split_once('.') else {
        return ("unknown".to_owned(), "1".to_owned());
    };
    report
        .semantic_program
        .domains
        .iter()
        .find(|domain| domain.name == domain_name)
        .and_then(|domain| {
            domain
                .variables
                .iter()
                .find(|domain_variable| domain_variable.name == variable_name)
        })
        .map(|domain_variable| {
            (
                domain_variable.quantity_kind.clone(),
                domain_variable.display_unit.clone(),
            )
        })
        .unwrap_or_else(|| ("unknown".to_owned(), "1".to_owned()))
}

#[derive(Clone, Debug, PartialEq)]
struct SimulateRequest {
    binding: String,
    options: Vec<eng_compiler::WithOptionInfo>,
}

fn simulate_requests(report: &CheckReport, system_name: &str) -> Vec<SimulateRequest> {
    report
        .inferred_declarations
        .iter()
        .filter_map(|declaration| {
            let expression = declaration.expression.trim();
            let requested_system = expression.strip_prefix("simulate ")?.trim();
            if requested_system != system_name {
                return None;
            }
            let options = report
                .semantic_program
                .with_blocks
                .iter()
                .find(|block| block.owner_line == Some(declaration.line))
                .map(|block| block.options.clone())
                .unwrap_or_default();
            Some(SimulateRequest {
                binding: declaration.name.clone(),
                options,
            })
        })
        .collect()
}

fn materialize_state_space_solutions(
    report: &CheckReport,
    system: &eng_compiler::SystemInfo,
    binding: Option<&str>,
    options: &[eng_compiler::WithOptionInfo],
    series: &[RuntimeTimeSeries],
) -> Option<Vec<RuntimeSystemSolution>> {
    let state_vector = report
        .semantic_program
        .state_space_vectors
        .iter()
        .find(|vector| vector.system == system.name && vector.role == "states")?;
    let input_vector = report
        .semantic_program
        .state_space_vectors
        .iter()
        .find(|vector| vector.system == system.name && vector.role == "inputs")?;
    let output_vector = report
        .semantic_program
        .state_space_vectors
        .iter()
        .find(|vector| vector.system == system.name && vector.role == "outputs");
    let states = state_vector
        .members
        .iter()
        .map(|name| {
            system
                .variables
                .iter()
                .find(|variable| variable.name == *name && variable.role == "state")
        })
        .collect::<Option<Vec<_>>>()?;
    let inputs = input_vector
        .members
        .iter()
        .map(|name| {
            system
                .variables
                .iter()
                .find(|variable| variable.name == *name && variable.role == "input")
        })
        .collect::<Option<Vec<_>>>()?;
    let operator_a = report
        .semantic_program
        .linear_operators
        .iter()
        .find(|operator| {
            operator.system == system.name
                && operator.from == "StateVector"
                && operator.to == "Derivative[StateVector]"
                && operator.status == "shape_checked"
        })?;
    let operator_b = report
        .semantic_program
        .linear_operators
        .iter()
        .find(|operator| {
            operator.system == system.name
                && operator.from == "InputVector"
                && operator.to == "Derivative[StateVector]"
                && operator.status == "shape_checked"
        })?;
    let matrix_a = operator_a.canonical_matrix.clone()?;
    let matrix_b = operator_b.canonical_matrix.clone()?;
    if matrix_a.len() != states.len()
        || matrix_a.iter().any(|row| row.len() != states.len())
        || matrix_b.len() != states.len()
        || matrix_b.iter().any(|row| row.len() != inputs.len())
    {
        return None;
    }

    let input_series = inputs
        .iter()
        .map(|input| {
            option_value(options, &input.name)
                .map(str::trim)
                .and_then(|name| series.iter().find(|series| series.name == name))
        })
        .collect::<Vec<_>>();
    let initial_state = states
        .iter()
        .map(|state| canonical_variable_value(state))
        .collect::<Option<Vec<_>>>()?;
    let time_step_s = option_value(options, "timestep")
        .and_then(parse_duration_seconds)
        .unwrap_or(300.0);
    let series_duration_s = input_series
        .iter()
        .filter_map(|series| {
            series
                .and_then(|series| series.points.last())
                .map(|point| point.x)
                .filter(|duration| *duration > 0.0)
        })
        .reduce(f64::min);
    let duration_s = option_value(options, "duration")
        .and_then(parse_duration_seconds)
        .or(series_duration_s)
        .unwrap_or(3600.0);
    let time_grid = TimeGrid::fixed_step(duration_s, time_step_s).ok()?;
    let solver_name = option_value(options, "solver")
        .map(str::trim)
        .unwrap_or("fixed_step");
    let fixed_step_method = FixedStepMethod::from_solver_name(Some(solver_name));
    let is_discrete_state_space = system
        .equations
        .iter()
        .any(|equation| equation.left.trim().starts_with("next("));
    if is_discrete_state_space && solver_name == "adaptive_heun" {
        return None;
    }
    let use_adaptive_heun = solver_name == "adaptive_heun";
    let adaptive_options =
        use_adaptive_heun.then(|| adaptive_heun_options_from_simulation(options, time_step_s));
    let solver_method = if is_discrete_state_space {
        "state_space_discrete_fixed_step".to_owned()
    } else if use_adaptive_heun {
        "adaptive_heun".to_owned()
    } else {
        fixed_step_method.method_name("state_space")
    };
    let solver_options = adaptive_options
        .as_ref()
        .map(|options| SolverOptions {
            method: solver_method.clone(),
            timestep_s: time_step_s,
            tolerance: options.tolerance,
            max_iterations: options.max_steps,
        })
        .unwrap_or_else(|| SolverOptions::fixed_step(solver_method, time_step_s));
    let output_members = output_vector
        .map(|vector| vector.members.clone())
        .unwrap_or_else(|| state_vector.members.clone());
    let solver_plan = SolverPlan::new(
        system.name.clone(),
        SimulationPlan {
            inputs: input_vector.members.clone(),
            outputs: output_members.clone(),
            states: state_vector.members.clone(),
            parameters: Vec::new(),
        },
        solver_options,
    );
    let solver_input = SolverInput {
        plan: solver_plan,
        time_grid,
        state_layout: StateLayout::new(
            states
                .iter()
                .enumerate()
                .map(|(index, state)| {
                    LayoutEntry::new(
                        index,
                        state.name.clone(),
                        state.quantity_kind.clone(),
                        state.canonical_unit.clone(),
                        state.display_unit.clone(),
                    )
                })
                .collect(),
        ),
        input_layout: InputLayout {
            entries: inputs
                .iter()
                .enumerate()
                .map(|(index, input)| {
                    LayoutEntry::new(
                        index,
                        input.name.clone(),
                        system_variable_value_quantity(input),
                        input.canonical_unit.clone(),
                        input.display_unit.clone(),
                    )
                })
                .collect(),
        },
        parameter_layout: ParameterLayout::default(),
        output_layout: OutputLayout {
            entries: output_members
                .iter()
                .enumerate()
                .filter_map(|(index, member)| {
                    states
                        .iter()
                        .find(|state| state.name == *member)
                        .map(|state| {
                            LayoutEntry::new(
                                index,
                                state.name.clone(),
                                state.quantity_kind.clone(),
                                state.canonical_unit.clone(),
                                state.display_unit.clone(),
                            )
                        })
                })
                .collect(),
        },
        initial_state,
        inputs: inputs
            .iter()
            .zip(input_series.iter())
            .map(|(input, series)| {
                dynamic_input_value(input, *series, 0.0).map(|value| {
                    SolverScalar::new(
                        input.name.clone(),
                        system_variable_value_quantity(input),
                        input.canonical_unit.clone(),
                        value,
                    )
                })
            })
            .collect::<Option<Vec<_>>>()?,
        parameters: Vec::new(),
    };
    if is_discrete_state_space {
        let solver_result = match solve_discrete_state_space(
            &solver_input,
            &matrix_a,
            &matrix_b,
            |sample_time_s| {
                inputs
                    .iter()
                    .zip(input_series.iter())
                    .map(|(input, series)| dynamic_input_value(input, *series, sample_time_s))
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| {
                        SolverFailure::new(
                            "E-SIM-MISSING-INPUT",
                            "discrete state-space solver could not materialize one or more input values",
                        )
                    })
            },
        ) {
            Ok(result) => result,
            Err(failure) => {
                let reason = "recognized discrete-time state-space A/B operators, but solver evaluation failed";
                return failed_system_runtime_solutions(
                    system,
                    binding,
                    &states,
                    &solver_input,
                    &failure,
                    reason,
                );
            }
        };
        let reason = if input_series.iter().any(Option::is_some) {
            "recognized discrete-time state-space A/B operators and executed state update with TimeSeries input materialization"
        } else {
            "recognized discrete-time state-space A/B operators and executed state update"
        };
        return system_runtime_solutions(system, binding, &states, &solver_result, reason);
    }
    if let Some(adaptive_options) = adaptive_options {
        let adaptive_result = match solve_adaptive_state_space(
            &solver_input,
            &matrix_a,
            &matrix_b,
            &adaptive_options,
            |sample_time_s| {
                inputs
                    .iter()
                    .zip(input_series.iter())
                    .map(|(input, series)| dynamic_input_value(input, *series, sample_time_s))
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| {
                        SolverFailure::new(
                            "E-SIM-MISSING-INPUT",
                            "adaptive state-space solver could not materialize one or more input values",
                        )
                    })
            },
        ) {
            Ok(result) => result,
            Err(failure) => {
                let reason = "recognized continuous state-space A/B operators, but adaptive Heun solver evaluation failed";
                return failed_system_runtime_solutions(
                    system,
                    binding,
                    &states,
                    &solver_input,
                    &failure,
                    reason,
                );
            }
        };
        let reason = if input_series.iter().any(Option::is_some) {
            "recognized continuous state-space A/B operators and executed adaptive Heun trajectories with TimeSeries input materialization"
        } else {
            "recognized continuous state-space A/B operators and executed adaptive Heun trajectories"
        };
        let step_diagnostics = runtime_system_step_diagnostics(&adaptive_result.step_reports);
        let mut solutions = system_runtime_solutions(
            system,
            binding,
            &states,
            &adaptive_result.solver_result,
            reason,
        )?;
        attach_system_step_diagnostics(&mut solutions, &step_diagnostics);
        return Some(solutions);
    }
    let solver_result = match solve_continuous_state_space(
        fixed_step_method,
        &solver_input,
        &matrix_a,
        &matrix_b,
        |sample_time_s| {
            inputs
                .iter()
                .zip(input_series.iter())
                .map(|(input, series)| dynamic_input_value(input, *series, sample_time_s))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    SolverFailure::new(
                        "E-SIM-MISSING-INPUT",
                        "state-space solver could not materialize one or more input values",
                    )
                })
        },
    ) {
        Ok(result) => result,
        Err(failure) => {
            let reason = "recognized continuous state-space A/B operators, but fixed-step solver evaluation failed";
            return failed_system_runtime_solutions(
                system,
                binding,
                &states,
                &solver_input,
                &failure,
                reason,
            );
        }
    };

    let reason = if input_series.iter().any(Option::is_some) {
        "recognized multi-state state-space A/B operators and executed fixed-step trajectories with TimeSeries input materialization"
    } else {
        "recognized multi-state state-space A/B operators and executed fixed-step trajectories"
    };
    system_runtime_solutions(system, binding, &states, &solver_result, reason)
}

fn system_runtime_solutions(
    system: &eng_compiler::SystemInfo,
    binding: Option<&str>,
    states: &[&eng_compiler::SystemVariableInfo],
    solver_result: &SolverResult,
    reason: &str,
) -> Option<Vec<RuntimeSystemSolution>> {
    let output_names = solver_result
        .output_layout
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect::<Vec<_>>();
    let include_all_states = output_names.is_empty();
    let solutions = solver_result
        .output
        .state_trajectories
        .iter()
        .filter(|trajectory| {
            include_all_states || output_names.iter().any(|name| *name == trajectory.name)
        })
        .filter_map(|trajectory| {
            states
                .iter()
                .find(|state| state.name == trajectory.name)
                .and_then(|state| {
                    RuntimeSystemSolution::from_solver_trajectory(
                        system,
                        binding,
                        state,
                        solver_result,
                        trajectory,
                        reason,
                    )
                })
        })
        .collect::<Vec<_>>();

    (!solutions.is_empty()).then_some(solutions)
}

fn failed_system_runtime_solutions(
    system: &eng_compiler::SystemInfo,
    binding: Option<&str>,
    states: &[&eng_compiler::SystemVariableInfo],
    solver_input: &SolverInput,
    failure: &SolverFailure,
    reason: &str,
) -> Option<Vec<RuntimeSystemSolution>> {
    let output_names = solver_input
        .output_layout
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect::<Vec<_>>();
    let include_all_states = output_names.is_empty();
    let solutions = states
        .iter()
        .filter(|state| include_all_states || output_names.iter().any(|name| *name == state.name))
        .filter_map(|state| {
            RuntimeSystemSolution::failed_from_solver_failure(
                system,
                binding,
                state,
                solver_input,
                failure,
                reason,
            )
        })
        .collect::<Vec<_>>();

    (!solutions.is_empty()).then_some(solutions)
}

fn runtime_system_step_diagnostics(
    reports: &[AdaptiveOdeStepReport],
) -> Vec<RuntimeSystemStepDiagnostic> {
    reports
        .iter()
        .map(|report| RuntimeSystemStepDiagnostic {
            output_index: report.output_index,
            start_time_s: report.start_time_s,
            end_time_s: report.end_time_s,
            dt_s: report.dt_s,
            error_norm: report.error_norm,
            status: report.status.clone(),
        })
        .collect()
}

fn attach_system_step_diagnostics(
    solutions: &mut [RuntimeSystemSolution],
    diagnostics: &[RuntimeSystemStepDiagnostic],
) {
    for solution in solutions {
        solution.step_diagnostics = diagnostics.to_vec();
    }
}

fn dynamic_input_value(
    input: &eng_compiler::SystemVariableInfo,
    series: Option<&RuntimeTimeSeries>,
    time_s: f64,
) -> Option<f64> {
    let quantity_kind = system_variable_value_quantity(input);
    if let Some(series) = series {
        let value = interpolate_series_value(series, time_s)?;
        return convert_to_canonical_unit(
            value,
            Some(&series.display_unit),
            &input.canonical_unit,
            &quantity_kind,
        )
        .ok();
    }
    canonical_variable_value(input)
}

fn materialize_source_ode_solutions(
    system: &eng_compiler::SystemInfo,
    binding: Option<&str>,
    options: &[eng_compiler::WithOptionInfo],
    series: &[RuntimeTimeSeries],
) -> Option<Vec<RuntimeSystemSolution>> {
    let states = system
        .variables
        .iter()
        .filter(|variable| variable.role == "state")
        .collect::<Vec<_>>();
    if states.is_empty() {
        return None;
    }
    let equations = source_ode_equations_for_states(system, &states)?;
    let solver_name = option_value(options, "solver")
        .map(str::trim)
        .unwrap_or("fixed_step");
    if solver_name == "adaptive_heun" {
        return None;
    }
    let fixed_step_method = FixedStepMethod::from_solver_name(Some(solver_name));
    let time_step_s = option_value(options, "timestep")
        .and_then(parse_duration_seconds)
        .unwrap_or(300.0);
    let inputs = system
        .variables
        .iter()
        .filter(|variable| variable.role == "input")
        .collect::<Vec<_>>();
    let input_series = inputs
        .iter()
        .map(|input| {
            option_value(options, &input.name)
                .and_then(|name| series.iter().find(|series| series.name == name))
        })
        .collect::<Vec<_>>();
    let series_duration_s = input_series
        .iter()
        .flatten()
        .filter_map(|series| series.points.last().map(|point| point.x))
        .filter(|duration| *duration > 0.0)
        .reduce(f64::max);
    let duration_s = option_value(options, "duration")
        .and_then(parse_duration_seconds)
        .or(series_duration_s)
        .unwrap_or(3600.0);
    let time_grid = TimeGrid::fixed_step(duration_s, time_step_s).ok()?;

    let parameters = system
        .variables
        .iter()
        .filter(|variable| variable.role == "parameter")
        .collect::<Vec<_>>();
    let initial_state = states
        .iter()
        .map(|state| canonical_variable_value(state))
        .collect::<Option<Vec<_>>>()?;
    let solver_inputs = inputs
        .iter()
        .zip(input_series.iter().copied())
        .map(|(input, source)| {
            dynamic_input_value(input, source, 0.0).map(|value| {
                SolverScalar::new(
                    input.name.clone(),
                    system_variable_value_quantity(input),
                    input.canonical_unit.clone(),
                    value,
                )
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let solver_parameters = parameters
        .iter()
        .map(|parameter| {
            canonical_variable_value(parameter).map(|value| {
                SolverScalar::new(
                    parameter.name.clone(),
                    parameter.quantity_kind.clone(),
                    parameter.canonical_unit.clone(),
                    value,
                )
            })
        })
        .collect::<Option<Vec<_>>>()?;

    let solver_options = SolverOptions::fixed_step(fixed_step_method.method_name(""), time_step_s);
    let solver_plan = SolverPlan::new(
        system.name.clone(),
        SimulationPlan {
            inputs: inputs.iter().map(|input| input.name.clone()).collect(),
            outputs: states.iter().map(|state| state.name.clone()).collect(),
            states: states.iter().map(|state| state.name.clone()).collect(),
            parameters: parameters
                .iter()
                .map(|parameter| parameter.name.clone())
                .collect(),
        },
        solver_options,
    );
    let solver_input = SolverInput {
        plan: solver_plan,
        time_grid,
        state_layout: StateLayout::new(
            states
                .iter()
                .enumerate()
                .map(|(index, state)| {
                    LayoutEntry::new(
                        index,
                        state.name.clone(),
                        state.quantity_kind.clone(),
                        state.canonical_unit.clone(),
                        state.display_unit.clone(),
                    )
                })
                .collect(),
        ),
        input_layout: InputLayout::new(
            inputs
                .iter()
                .enumerate()
                .map(|(index, input)| {
                    LayoutEntry::new(
                        index,
                        input.name.clone(),
                        system_variable_value_quantity(input),
                        input.canonical_unit.clone(),
                        input.display_unit.clone(),
                    )
                })
                .collect(),
        ),
        parameter_layout: ParameterLayout::new(
            parameters
                .iter()
                .enumerate()
                .map(|(index, parameter)| {
                    LayoutEntry::new(
                        index,
                        parameter.name.clone(),
                        parameter.quantity_kind.clone(),
                        parameter.canonical_unit.clone(),
                        parameter.display_unit.clone(),
                    )
                })
                .collect(),
        ),
        output_layout: OutputLayout::new(
            states
                .iter()
                .enumerate()
                .map(|(index, state)| {
                    LayoutEntry::new(
                        index,
                        state.name.clone(),
                        state.quantity_kind.clone(),
                        state.canonical_unit.clone(),
                        state.display_unit.clone(),
                    )
                })
                .collect(),
        ),
        initial_state,
        inputs: solver_inputs,
        parameters: solver_parameters,
    };

    let evaluator = match SourceRhsEvaluator::new(
        states
            .iter()
            .map(|state| {
                RhsStateInfo::new(
                    state.name.clone(),
                    state.quantity_kind.clone(),
                    state.canonical_unit.clone(),
                )
            })
            .collect(),
        inputs.iter().map(|input| input.name.clone()).collect(),
        parameters
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect(),
        equations,
    ) {
        Ok(evaluator) => evaluator,
        Err(failure) => {
            return failed_system_runtime_solutions(
                system,
                binding,
                &states,
                &solver_input,
                &failure,
                "recognized source derivative equations, but RHS evaluator creation failed",
            );
        }
    };
    let parameter_values = solver_input
        .parameters
        .iter()
        .map(|parameter| parameter.value)
        .collect::<Vec<_>>();
    let solver_result = match solve_fixed_step_ode(fixed_step_method, &solver_input, |sample| {
        let input_values = inputs
            .iter()
            .zip(input_series.iter().copied())
            .map(|(input, source)| {
                dynamic_input_value(input, source, sample.time_s).ok_or_else(|| {
                    SolverFailure::new(
                        "E-RHS-INPUT-VALUE",
                        format!(
                            "source RHS input `{}` cannot be evaluated at t={} s",
                            input.name, sample.time_s
                        ),
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let output = evaluator.evaluate(&RhsInput {
            t: sample.time_s,
            x: sample.state.to_vec(),
            u: input_values,
            p: parameter_values.clone(),
        })?;
        Ok(output.derivatives)
    }) {
        Ok(result) => result,
        Err(failure) => {
            return failed_system_runtime_solutions(
                system,
                binding,
                &states,
                &solver_input,
                &failure,
                "recognized source derivative equations, but fixed-step solver evaluation failed",
            );
        }
    };

    let reason = if input_series.iter().any(Option::is_some) {
        "recognized source derivative equations and executed fixed-step RHS with TimeSeries input materialization"
    } else {
        "recognized source derivative equations and executed fixed-step RHS"
    };
    system_runtime_solutions(system, binding, &states, &solver_result, reason)
}

fn source_ode_equations_for_states(
    system: &eng_compiler::SystemInfo,
    states: &[&eng_compiler::SystemVariableInfo],
) -> Option<Vec<SourceRhsEquation>> {
    let mut equations = Vec::new();
    for state in states {
        let derivative = format!("der({})", state.name);
        let matches = system
            .equations
            .iter()
            .filter(|equation| equation.left.contains(&derivative))
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return None;
        }
        equations.push(SourceRhsEquation::new(
            state.name.clone(),
            matches[0].left.clone(),
            matches[0].right.clone(),
        ));
    }
    Some(equations)
}

fn materialize_first_order_thermal_solution(
    system: &eng_compiler::SystemInfo,
    binding: Option<&str>,
    options: &[eng_compiler::WithOptionInfo],
    series: &[RuntimeTimeSeries],
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
        variable.role == "input"
            && system_variable_matches_quantity(variable, "AbsoluteTemperature")
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
    let outdoor_series = option_value(options, &outdoor_temperature.name)
        .and_then(|name| series.iter().find(|series| series.name == name));
    let outdoor_quantity_kind = system_variable_value_quantity(outdoor_temperature);
    let outdoor_temperature_k = canonical_variable_value(outdoor_temperature).or_else(|| {
        outdoor_series.and_then(|series| {
            series.points.first().and_then(|point| {
                convert_to_canonical_unit(
                    point.y,
                    Some(&series.display_unit),
                    &outdoor_temperature.canonical_unit,
                    &outdoor_quantity_kind,
                )
                .ok()
            })
        })
    })?;
    let internal_heat_w = canonical_variable_value(internal_heat)?;
    let initial_temperature_k = canonical_variable_value(state)?;

    let time_step_s = option_value(options, "timestep")
        .and_then(parse_duration_seconds)
        .unwrap_or(300.0);
    let series_duration_s = outdoor_series
        .and_then(|series| series.points.last().map(|point| point.x))
        .filter(|duration| *duration > 0.0);
    let duration_s = option_value(options, "duration")
        .and_then(parse_duration_seconds)
        .or(series_duration_s)
        .unwrap_or(3600.0);
    let time_grid = TimeGrid::fixed_step(duration_s, time_step_s).ok()?;
    let solver_name = option_value(options, "solver")
        .map(str::trim)
        .unwrap_or("fixed_step");
    let use_adaptive_heun = solver_name == "adaptive_heun";
    let fixed_step_method = FixedStepMethod::from_solver_name(Some(solver_name));
    let adaptive_options =
        use_adaptive_heun.then(|| adaptive_heun_options_from_simulation(options, time_step_s));
    let solver_options = adaptive_options
        .as_ref()
        .map(|options| SolverOptions {
            method: "adaptive_heun".to_owned(),
            timestep_s: time_step_s,
            tolerance: options.tolerance,
            max_iterations: options.max_steps,
        })
        .unwrap_or_else(|| {
            SolverOptions::fixed_step(fixed_step_method.method_name(""), time_step_s)
        });
    let solver_plan = SolverPlan::new(
        system.name.clone(),
        SimulationPlan {
            inputs: vec![outdoor_temperature.name.clone(), internal_heat.name.clone()],
            outputs: vec![state.name.clone()],
            states: vec![state.name.clone()],
            parameters: vec![heat_capacity.name.clone(), conductance.name.clone()],
        },
        solver_options,
    );
    let solver_input = SolverInput {
        plan: solver_plan,
        time_grid,
        state_layout: StateLayout::new(vec![LayoutEntry::new(
            0,
            state.name.clone(),
            state.quantity_kind.clone(),
            state.canonical_unit.clone(),
            state.display_unit.clone(),
        )]),
        input_layout: InputLayout {
            entries: vec![
                LayoutEntry::new(
                    0,
                    outdoor_temperature.name.clone(),
                    outdoor_quantity_kind.clone(),
                    outdoor_temperature.canonical_unit.clone(),
                    outdoor_temperature.display_unit.clone(),
                ),
                LayoutEntry::new(
                    1,
                    internal_heat.name.clone(),
                    internal_heat.quantity_kind.clone(),
                    internal_heat.canonical_unit.clone(),
                    internal_heat.display_unit.clone(),
                ),
            ],
        },
        parameter_layout: ParameterLayout {
            entries: vec![
                LayoutEntry::new(
                    0,
                    heat_capacity.name.clone(),
                    heat_capacity.quantity_kind.clone(),
                    heat_capacity.canonical_unit.clone(),
                    heat_capacity.display_unit.clone(),
                ),
                LayoutEntry::new(
                    1,
                    conductance.name.clone(),
                    conductance.quantity_kind.clone(),
                    conductance.canonical_unit.clone(),
                    conductance.display_unit.clone(),
                ),
            ],
        },
        output_layout: OutputLayout {
            entries: vec![LayoutEntry::new(
                0,
                state.name.clone(),
                state.quantity_kind.clone(),
                state.canonical_unit.clone(),
                state.display_unit.clone(),
            )],
        },
        initial_state: vec![initial_temperature_k],
        inputs: vec![
            SolverScalar::new(
                outdoor_temperature.name.clone(),
                outdoor_quantity_kind.clone(),
                outdoor_temperature.canonical_unit.clone(),
                outdoor_temperature_k,
            ),
            SolverScalar::new(
                internal_heat.name.clone(),
                internal_heat.quantity_kind.clone(),
                internal_heat.canonical_unit.clone(),
                internal_heat_w,
            ),
        ],
        parameters: vec![
            SolverScalar::new(
                heat_capacity.name.clone(),
                heat_capacity.quantity_kind.clone(),
                heat_capacity.canonical_unit.clone(),
                heat_capacity_j_per_k,
            ),
            SolverScalar::new(
                conductance.name.clone(),
                conductance.quantity_kind.clone(),
                conductance.canonical_unit.clone(),
                conductance_w_per_k,
            ),
        ],
    };

    let thermal_model = match FirstOrderThermalModel::new(
        heat_capacity_j_per_k,
        conductance_w_per_k,
        internal_heat_w,
    ) {
        Ok(model) => model,
        Err(failure) => {
            return RuntimeSystemSolution::failed_from_solver_failure(
                system,
                binding,
                state,
                &solver_input,
                &failure,
                "recognized first-order thermal ODE, but thermal model validation failed",
            );
        }
    };

    let (solver_result, step_diagnostics) = if let Some(adaptive_options) = adaptive_options {
        let adaptive_result = match solve_adaptive_heun_ode(
            &solver_input,
            &adaptive_options,
            |sample| {
                let temperature_k = sample.state[0];
                let outdoor_k = outdoor_series
                    .and_then(|series| interpolate_series_value(series, sample.time_s))
                    .map(|value| {
                        convert_to_canonical_unit(
                            value,
                            outdoor_series.map(|series| series.display_unit.as_str()),
                            &outdoor_temperature.canonical_unit,
                            &outdoor_quantity_kind,
                        )
                        .unwrap_or(outdoor_temperature_k)
                    })
                    .unwrap_or(outdoor_temperature_k);
                if !outdoor_k.is_finite() {
                    return Err(SolverFailure::new(
                        "E-SOLVER-THERMAL-INPUT-INVALID",
                        "first-order thermal solver requires finite outdoor temperature input",
                    ));
                }
                let derivative_k_per_s = (thermal_model.conductance_w_per_k
                    * (outdoor_k - temperature_k)
                    + thermal_model.internal_heat_w)
                    / thermal_model.heat_capacity_j_per_k;
                Ok(vec![derivative_k_per_s])
            },
        ) {
            Ok(result) => result,
            Err(failure) => {
                return RuntimeSystemSolution::failed_from_solver_failure(
                    system,
                    binding,
                    state,
                    &solver_input,
                    &failure,
                    "recognized first-order thermal ODE, but adaptive Heun solver evaluation failed",
                );
            }
        };
        (
            adaptive_result.solver_result,
            runtime_system_step_diagnostics(&adaptive_result.step_reports),
        )
    } else {
        (
            match solve_first_order_thermal(
                fixed_step_method,
                &solver_input,
                thermal_model,
                |time_s| {
                    let outdoor_k = outdoor_series
                        .and_then(|series| interpolate_series_value(series, time_s))
                        .map(|value| {
                            convert_to_canonical_unit(
                                value,
                                outdoor_series.map(|series| series.display_unit.as_str()),
                                &outdoor_temperature.canonical_unit,
                                &outdoor_quantity_kind,
                            )
                            .unwrap_or(outdoor_temperature_k)
                        })
                        .unwrap_or(outdoor_temperature_k);
                    Ok(outdoor_k)
                },
            ) {
                Ok(result) => result,
                Err(failure) => {
                    return RuntimeSystemSolution::failed_from_solver_failure(
                        system,
                        binding,
                        state,
                        &solver_input,
                        &failure,
                        "recognized first-order thermal ODE, but fixed-step solver evaluation failed",
                    );
                }
            },
            Vec::new(),
        )
    };
    let reason = if use_adaptive_heun {
        "recognized first-order thermal ODE and executed through SolverResult adaptive Heun one-state path"
    } else {
        "recognized first-order thermal ODE and executed through SolverResult fixed-step one-state path"
    };

    let mut solution =
        RuntimeSystemSolution::from_solver_result(system, binding, state, &solver_result, reason)?;
    solution.step_diagnostics = step_diagnostics;
    Some(solution)
}

fn system_variable_matches_quantity(
    variable: &eng_compiler::SystemVariableInfo,
    quantity_kind: &str,
) -> bool {
    if variable.quantity_kind == quantity_kind {
        return true;
    }
    time_series_quantity(&variable.quantity_kind)
        .is_some_and(|(_, quantity)| quantity == quantity_kind)
}

fn system_variable_value_quantity(variable: &eng_compiler::SystemVariableInfo) -> String {
    time_series_quantity(&variable.quantity_kind)
        .map(|(_, quantity)| quantity)
        .unwrap_or_else(|| variable.quantity_kind.clone())
}

fn materialize_system_solution_series(
    solutions: &[RuntimeSystemSolution],
) -> Vec<RuntimeTimeSeries> {
    solutions
        .iter()
        .filter(|solution| solution.status == "computed")
        .map(|solution| RuntimeTimeSeries {
            name: match &solution.binding {
                Some(binding) => format!("{binding}.{}", solution.state),
                None => format!("{}.{}", solution.system, solution.state),
            },
            axis: "Time".to_owned(),
            x_unit: solution.time_unit.clone(),
            quantity_kind: solution.quantity_kind.clone(),
            display_unit: solution.display_unit.clone(),
            source_table: solution.system.clone(),
            source_expression: format!("simulate {}", solution.system),
            points: solution.points.clone(),
        })
        .collect()
}

fn materialize_component_solution_series(
    solutions: &[RuntimeComponentSolution],
) -> Vec<RuntimeTimeSeries> {
    solutions
        .iter()
        .filter(|solution| solution.status == "computed")
        .flat_map(|solution| {
            solution
                .trajectories
                .iter()
                .map(|trajectory| RuntimeTimeSeries {
                    name: format!("{}.{}", solution.assembly, trajectory.name),
                    axis: "Time".to_owned(),
                    x_unit: "s".to_owned(),
                    quantity_kind: trajectory.quantity_kind.clone(),
                    display_unit: trajectory.unit.clone(),
                    source_table: solution.assembly.clone(),
                    source_expression: format!("component solver {}", solution.assembly),
                    points: trajectory.points.clone(),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn skipped_system_solution(
    system: &eng_compiler::SystemInfo,
    binding: Option<&str>,
    options: &[eng_compiler::WithOptionInfo],
) -> RuntimeSystemSolution {
    let state = system
        .variables
        .iter()
        .find(|variable| variable.role == "state");
    let canonical_initial_value = state.and_then(canonical_variable_value).unwrap_or(0.0);
    let initial_value = state
        .map(|state| display_variable_value(canonical_initial_value, state))
        .unwrap_or(0.0);
    RuntimeSystemSolution {
        system: system.name.clone(),
        binding: binding.map(str::to_owned),
        status: "skipped_unsupported_shape".to_owned(),
        method: "explicit_euler_fixed_step".to_owned(),
        reason:
            "system shape is outside the supported state-space/source ODE/first-order thermal runners"
                .to_owned(),
        states: system_variable_names_by_role(system, "state"),
        algebraic_variables: system_variable_names_by_role(system, "algebraic"),
        inputs: system_variable_names_by_role(system, "input"),
        parameters: system_variable_names_by_role(system, "parameter"),
        outputs: Vec::new(),
        state: state.map(|state| state.name.clone()).unwrap_or_default(),
        quantity_kind: state
            .map(|state| state.quantity_kind.clone())
            .unwrap_or_default(),
        display_unit: state
            .map(|state| state.display_unit.clone())
            .unwrap_or_default(),
        canonical_unit: state
            .map(|state| state.canonical_unit.clone())
            .unwrap_or_default(),
        time_unit: "s".to_owned(),
        duration_s: 0.0,
        time_step_s: option_value(options, "timestep")
            .and_then(parse_duration_seconds)
            .unwrap_or(0.0),
        step_count: 0,
        tolerance: SolverOptions::fixed_step("explicit_euler_fixed_step", 0.0).tolerance,
        max_iterations: SolverOptions::fixed_step("explicit_euler_fixed_step", 0.0).max_iterations,
        iteration_count: 0,
        convergence_status: "skipped_unsupported_shape".to_owned(),
        failure_code: Some("E-SIM-SYSTEM-SHAPE-UNSUPPORTED".to_owned()),
        failure_reason: Some(
            "system shape is outside the supported state-space/source ODE/first-order thermal runners"
                .to_owned(),
        ),
        initial_value,
        final_value: initial_value,
        canonical_initial_value,
        canonical_final_value: canonical_initial_value,
        step_diagnostics: Vec::new(),
        points: Vec::new(),
    }
}

fn system_variable_names_by_role(system: &eng_compiler::SystemInfo, role: &str) -> Vec<String> {
    system
        .variables
        .iter()
        .filter(|variable| variable.role == role)
        .map(|variable| variable.name.clone())
        .collect()
}

fn option_value<'a>(options: &'a [eng_compiler::WithOptionInfo], key: &str) -> Option<&'a str> {
    options
        .iter()
        .find(|option| option.key == key)
        .map(|option| option.value.as_str())
}

fn adaptive_heun_options_from_simulation(
    options: &[eng_compiler::WithOptionInfo],
    output_timestep_s: f64,
) -> AdaptiveOdeOptions {
    let mut adaptive = AdaptiveOdeOptions::default();
    if let Some(tolerance) = option_value(options, "tolerance")
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
    {
        adaptive.tolerance = tolerance;
    }
    adaptive.initial_step_s = adaptive
        .initial_step_s
        .min(output_timestep_s)
        .max(adaptive.min_step_s);
    adaptive.max_step_s = adaptive
        .max_step_s
        .min(output_timestep_s)
        .max(adaptive.min_step_s);
    adaptive
}

fn parse_duration_seconds(value: &str) -> Option<f64> {
    let (amount, unit) = number_with_optional_unit(value)?;
    let unit = unit.as_deref().map(normalize_unit);
    Some(match unit.as_deref() {
        Some("min") => amount * 60.0,
        Some("h") => amount * 3600.0,
        Some("s") | None => amount,
        _ => return None,
    })
}

fn interpolate_series_value(series: &RuntimeTimeSeries, x: f64) -> Option<f64> {
    let first = series.points.first()?;
    if x <= first.x {
        return Some(first.y);
    }
    let last = series.points.last()?;
    if x >= last.x {
        return Some(last.y);
    }
    for window in series.points.windows(2) {
        let a = window[0];
        let b = window[1];
        if x >= a.x && x <= b.x {
            let span = b.x - a.x;
            if span.abs() <= f64::EPSILON {
                return Some(a.y);
            }
            let t = (x - a.x) / span;
            return Some(a.y + (b.y - a.y) * t);
        }
    }
    None
}

fn canonical_variable_value(variable: &eng_compiler::SystemVariableInfo) -> Option<f64> {
    let expression = variable.initial_value.as_deref()?;
    let (value, unit) = number_with_optional_unit(expression)?;
    let quantity_kind = system_variable_value_quantity(variable);
    convert_to_canonical_unit(
        value,
        unit.as_deref(),
        &variable.canonical_unit,
        &quantity_kind,
    )
    .ok()
}

fn display_variable_value(value: f64, variable: &eng_compiler::SystemVariableInfo) -> f64 {
    let quantity_kind = system_variable_value_quantity(variable);
    convert_from_canonical_unit(
        value,
        &variable.canonical_unit,
        &variable.display_unit,
        &quantity_kind,
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

    fn time_index_column(&self) -> Option<&RuntimeColumn> {
        self.columns
            .iter()
            .find(|column| column.is_index && column.type_name == "DateTime")
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
        if let Some(values) = self.normalized_time_axis_values() {
            return (values, "s".to_owned());
        }
        (sample_axis_values(self.row_count), "sample".to_owned())
    }

    fn normalized_time_axis_values(&self) -> Option<Vec<f64>> {
        let column = self.time_index_column()?;
        let RuntimeValues::Text(values) = &column.values else {
            return None;
        };
        let timestamps = values
            .iter()
            .map(|value| parse_utc_timestamp_seconds(value))
            .collect::<Option<Vec<_>>>();
        let timestamps = timestamps?;
        let Some(first) = timestamps.first().copied() else {
            return Some(Vec::new());
        };
        Some(
            timestamps
                .iter()
                .map(|timestamp| (*timestamp - first) as f64)
                .collect(),
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
    let header_end = [after_plot.find('{'), after_plot.find('\n')]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(after_plot.len());
    let header = after_plot[..header_end].trim();
    if let Some(histogram) = parse_histogram_header(header) {
        options.histogram = Some(histogram);
        options.plot_type = Some("histogram".to_owned());
    } else if let Some(distribution) = parse_distribution_header(header) {
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
        options.series_list = series
            .split(" and ")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect();
        options.series = options.series_list.first().cloned();
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
        if let Some(rest) = line.strip_prefix("unit x =") {
            options.x_unit = rest.split_whitespace().next().map(str::to_owned);
        } else if let Some(rest) = line.strip_prefix("unit y =") {
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

fn parse_histogram_header(header: &str) -> Option<String> {
    parse_call_header(header, "histogram")
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
        ("pa", "kpa") => value / 1000.0,
        ("kpa", "pa") => value * 1000.0,
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
        ("pa", "kpa", "Pressure") => value / 1000.0,
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
    use eng_compiler::{check_file, check_source, CheckOptions, CheckReport};

    #[test]
    fn parses_plot_options() {
        let options = parse_plot_options(
            r#"
report {
    plot Q_coil over Time {
        unit y = kW
        type = histogram
        title = "Coil heat rate"
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
report {
    plot distribution(Q_coil_dist) {
        title = "Coil uncertainty"
    }
}
"#,
        );

        assert_eq!(options.distribution.as_deref(), Some("Q_coil_dist"));
        assert_eq!(options.plot_type.as_deref(), Some("histogram"));
        assert_eq!(options.title.as_deref(), Some("Coil uncertainty"));
    }

    #[test]
    fn parses_histogram_plot_options() {
        let options = parse_plot_options(
            r#"
report {
    plot histogram(Q_coil) {
        unit x = kW
        title = "Coil heat-rate distribution"
    }
}
"#,
        );

        assert_eq!(options.histogram.as_deref(), Some("Q_coil"));
        assert_eq!(options.plot_type.as_deref(), Some("histogram"));
        assert_eq!(options.x_unit.as_deref(), Some("kW"));
        assert_eq!(
            options.title.as_deref(),
            Some("Coil heat-rate distribution")
        );
    }

    #[test]
    fn parses_with_plot_options_for_special_headers() {
        let histogram_options = parse_plot_options(
            r#"
report {
    plot histogram(Q_coil)
    with {
        unit x = kW
        title = "Coil heat-rate distribution"
    }
}
"#,
        );

        assert_eq!(histogram_options.histogram.as_deref(), Some("Q_coil"));
        assert_eq!(histogram_options.plot_type.as_deref(), Some("histogram"));
        assert_eq!(histogram_options.x_unit.as_deref(), Some("kW"));

        let distribution_options = parse_plot_options(
            r#"
report {
    plot distribution(Q_coil_dist)
    with {
        title = "Coil uncertainty"
    }
}
"#,
        );

        assert_eq!(
            distribution_options.distribution.as_deref(),
            Some("Q_coil_dist")
        );
        assert_eq!(distribution_options.plot_type.as_deref(), Some("histogram"));

        let model_options = parse_plot_options(
            r#"
report {
    plot residuals(reg_eval)
    with {
        title = "Regression residuals"
    }
}
"#,
        );

        let model_plot = model_options.model_plot.as_ref().unwrap();
        assert_eq!(model_plot.kind, "residuals");
        assert_eq!(model_plot.source, "reg_eval");
        assert_eq!(model_options.plot_type.as_deref(), Some("bar"));
    }

    #[test]
    fn parses_model_plot_options() {
        let options = parse_plot_options(
            r#"
report {
    plot parity(reg_eval) {
        title = "Regression parity"
    }
}
"#,
        );

        let model_plot = options.model_plot.as_ref().unwrap();
        assert_eq!(model_plot.kind, "parity");
        assert_eq!(model_plot.source, "reg_eval");
        assert_eq!(options.plot_type.as_deref(), Some("scatter"));
        assert_eq!(options.title.as_deref(), Some("Regression parity"));

        let residual_options = parse_plot_options(
            r#"
report {
    plot residuals(reg_eval) {
        title = "Regression residuals"
    }
}
"#,
        );
        let residual_plot = residual_options.model_plot.as_ref().unwrap();
        assert_eq!(residual_plot.kind, "residuals");
        assert_eq!(residual_plot.source, "reg_eval");
        assert_eq!(residual_options.plot_type.as_deref(), Some("bar"));
        assert_eq!(
            residual_options.title.as_deref(),
            Some("Regression residuals")
        );
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
Q_coil_dist = normal(mean=5 kW, std=0.8 kW, samples=31)
Q_unc = propagate(Q_coil_dist, method=linear, scale=1.1, offset=0.2 kW)

report {
    plot distribution(Q_coil_dist) {
        title = "Coil uncertainty"
    }
}
"#;
        let report = eng_compiler::check_source("ok.eng", source, &CheckOptions::default());
        let runtime = materialize_runtime_data(&report, source);
        let mut plot_spec = eng_report::plot_spec_from_report(&report);
        runtime.apply_plot_spec(&report, &mut plot_spec);

        assert_eq!(runtime.uncertainties.len(), 2);
        assert_eq!(runtime.uncertainties[0].sample_count, 31);
        assert_eq!(runtime.uncertainties[0].display_unit, "kW");
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
        assert_eq!(plot_spec.x_axis.unit, "kW");
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
Q_unc = propagate(Q_missing, method=linear, samples=8)
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
    fn materializes_component_assembly_constraint_check() {
        let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/internal/06_domain_port/main.eng");
        let source = std::fs::read_to_string(&source_path).unwrap();
        let report = check_file(&source_path, &CheckOptions::default()).unwrap();
        let runtime = materialize_runtime_data(&report, &source);
        let solver_assembly = solver_equation_assembly_from_component_info(
            &report,
            &report.semantic_program.component_assemblies[0],
        );

        assert_eq!(solver_assembly.name, "component_graph");
        assert_eq!(solver_assembly.equation_count(), 6);
        assert_eq!(solver_assembly.unknown_count(), 12);
        assert!(solver_assembly
            .generated_equations
            .iter()
            .any(|equation| equation.reason.contains("through variable conservation")));
        let residual_graph = ResidualGraph::from_assembly(&solver_assembly);
        assert_eq!(residual_graph.name, "component_graph.residual_graph");
        assert_eq!(
            residual_graph.residuals.len(),
            solver_assembly.equation_count()
        );
        assert!(residual_graph.residuals.iter().any(|residual| {
            residual.name == "connection_set_1.through_Q_conservation"
                && residual.variable_indices.len() == 2
                && residual.terms.iter().all(|term| term.coefficient == 1.0)
                && residual
                    .source
                    .generated_reason
                    .as_deref()
                    .is_some_and(|reason| reason.contains("through variable conservation"))
        }));
        let zero_values = vec![0.0; residual_graph.variables.len()];
        let zero_output = residual_graph
            .evaluate(&ResidualInput::new(&zero_values))
            .unwrap();
        assert_eq!(zero_output.residual_norm, 0.0);
        let mut perturbed_values = zero_values;
        perturbed_values[0] = 1.0;
        let perturbed_output = residual_graph
            .evaluate(&ResidualInput::new(&perturbed_values))
            .unwrap();
        assert!(perturbed_output.residual_norm > 0.0);

        assert_eq!(runtime.component_solutions.len(), 1);
        let solution = &runtime.component_solutions[0];
        assert_eq!(solution.assembly, "component_graph");
        assert_eq!(solution.status, "constraint_satisfied_nonunique");
        assert_eq!(solution.method, "linear_residual_graph_shape_check");
        assert_eq!(solution.residual_norm, 0.0);
        assert_eq!(
            solution.convergence_status,
            "linear_residual_satisfied_nonunique"
        );
        assert_eq!(
            solution
                .failure_artifact
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("E-ASSEMBLY-UNDERDETERMINED")
        );
        assert!(solution.residuals.iter().any(|residual| residual.name
            == "connection_set_1.through_Q_conservation"
            && residual.normalized_value == 0.0
            && residual.scale == 1.0
            && residual.scale_policy == "unit_default:HeatRate[kW]"));
        assert_eq!(solution.largest_residuals.len(), 3);
        assert!(solution
            .largest_residuals
            .iter()
            .all(|residual| residual.normalized_value == 0.0));
        assert!(solution.residuals.len() > solution.largest_residuals.len());

        let mut spec =
            eng_report::report_spec_from_report(&report, "plots/plot_manifest.json", "abc123");
        runtime.apply_component_solutions(&mut spec);
        assert_eq!(spec.assemblies[0].status, "constraint_satisfied_nonunique");
        assert_eq!(
            spec.assemblies[0].residual_graph.status,
            "linear_residual_satisfied_nonunique"
        );
        let solver_result = spec.assemblies[0].solver_result.as_ref().unwrap();
        assert_eq!(solver_result.status, "constraint_satisfied_nonunique");
        assert_eq!(solver_result.method, "linear_residual_graph_shape_check");
        assert_eq!(solver_result.residual_norm, 0.0);
        assert_eq!(solver_result.tolerance, COMPONENT_LINEAR_SOLVER_TOLERANCE);
        assert_eq!(solver_result.max_iterations, 1);
        assert_eq!(solver_result.variables.len(), 12);
        assert!(solver_result.residuals.iter().any(|residual| residual.name
            == "connection_set_1.through_Q_conservation"
            && residual.normalized_value == 0.0
            && residual.scale_policy == "unit_default:HeatRate[kW]"));
        assert_eq!(solver_result.largest_residuals.len(), 3);
        assert!(solver_result
            .largest_residuals
            .iter()
            .all(|residual| residual.normalized_value == 0.0));
        assert_eq!(
            solver_result
                .failure_artifact
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("E-ASSEMBLY-UNDERDETERMINED")
        );
        let json = eng_report::report_spec_json(&spec);
        assert!(json.contains("\"failure_code\": \"E-ASSEMBLY-UNDERDETERMINED\""));
        assert!(json.contains("\"failure_reason\": \"assembly has fewer equations than unknowns"));
    }

    #[test]
    fn solves_square_component_residual_graph_with_dense_linear_solver() {
        let assembly = square_linear_test_assembly("component_graph");

        let solution = RuntimeComponentSolution::from_solver_assembly("component_graph", &assembly);

        assert_eq!(solution.status, "solved_linear");
        assert_eq!(solution.method, "dense_linear_residual_graph");
        assert_eq!(solution.convergence_status, "linear_converged");
        assert_eq!(solution.iteration_count, 1);
        assert_eq!(solution.tolerance, COMPONENT_LINEAR_SOLVER_TOLERANCE);
        assert_eq!(solution.max_iterations, 1);
        assert_eq!(solution.residual_norm, 0.0);
        assert!(solution.failure_artifact.is_none());
        assert!(solution
            .variables
            .iter()
            .all(|variable| variable.status == "solved_linear" && variable.value == 0.0));
        assert!(solution
            .residuals
            .iter()
            .all(|residual| residual.status == "satisfied"));
    }

    #[test]
    fn materializes_fixed_point_source_solve_request() {
        let source = r#"
domain PowerScalar {
    across q: HeatRate [kW]
    through balance: HeatRate [kW]
    conservation sum(balance) = 0
}

component RelaxingLoop {
    port source: PowerScalar
    port target: PowerScalar
    source.q eq 0.5 * target.q + 1 kW
    source.balance eq 0 kW
}

system FixedPointLoop {
    relax = RelaxingLoop()
    connect relax.source to relax.target
}

fixed_point_result = solve component_graph
with {
    solver = fixed_point
    tolerance = 0.000001
    max_iter = 80
    relaxation = 1
    initial = 0
}
"#;
        let report = check_source("fixed_point.eng", source, &CheckOptions::default());
        assert!(!report.has_errors(), "{:?}", report.diagnostics);

        let runtime = materialize_runtime_data(&report, source);

        assert_eq!(runtime.component_solutions.len(), 1);
        let solution = &runtime.component_solutions[0];
        assert_eq!(solution.status, "solved_fixed_point");
        assert_eq!(solution.method, "fixed_point_residual_graph");
        assert_eq!(solution.convergence_status, "fixed_point_converged");
        assert_eq!(solution.tolerance, 0.000001);
        assert_eq!(solution.max_iterations, 80);
        assert!(solution.iteration_count > 1);
        assert!(solution.failure_artifact.is_none());
        assert!(solution.residual_norm <= solution.tolerance);
        assert!(solution
            .reason
            .contains("source solve binding `fixed_point_result`"));
        assert!(solution.variables.iter().any(|variable| {
            variable.name == "relax.source.q" && variable.status == "solved_fixed_point"
        }));

        let mut spec =
            eng_report::report_spec_from_report(&report, "plots/plot_manifest.json", "abc123");
        runtime.apply_component_solutions(&mut spec);
        let json = eng_report::report_spec_json(&spec);
        assert!(json.contains("\"method\": \"fixed_point_residual_graph\""));
        assert!(json.contains("\"convergence_status\": \"fixed_point_converged\""));
        assert!(json.contains("relax.source.q - 0.5 * relax.target.q + 1 kW"));
    }

    #[test]
    fn materializes_dynamic_component_explicit_source_solve_request() {
        let source = r#"
domain ScalarState {
    across x: DimensionlessNumber [1]
    through balance: DimensionlessNumber [1]
    conservation sum(balance) = 0
}

component DecayNode {
    port node: ScalarState
    der(node.x) eq -0.5 * node.x
}

system DynamicExplicit {
    node = DecayNode()
    connect node.node to node.node
}

explicit_result = solve component_graph
with {
    solver = dynamic_component_explicit_euler
    timestep = 1 s
    duration = 3 s
    initial = 4
}
"#;
        let report = check_source("dynamic_explicit.eng", source, &CheckOptions::default());
        assert!(!report.has_errors(), "{:?}", report.diagnostics);

        let runtime = materialize_runtime_data(&report, source);

        assert_eq!(runtime.component_solutions.len(), 1);
        let solution = &runtime.component_solutions[0];
        assert_eq!(solution.status, "computed");
        assert_eq!(solution.method, "dynamic_component_assembly_explicit_euler");
        assert_eq!(
            solution.convergence_status,
            "dynamic_component_fixed_step_completed"
        );
        assert_eq!(solution.step_diagnostics.len(), 4);
        assert!(solution
            .step_diagnostics
            .iter()
            .all(|diagnostic| diagnostic.convergence_status == "algebraic_not_required"));
        assert_eq!(solution.trajectories.len(), 1);
        assert_eq!(solution.trajectories[0].name, "node.node.x");
        assert_eq!(solution.trajectories[0].point_count, 4);
        assert_eq!(solution.trajectories[0].points[0].y, 4.0);
        assert_eq!(solution.trajectories[0].points[3].y, 0.5);
        assert!(solution.failure_artifact.is_none());
        assert!(solution
            .reason
            .contains("source solve binding `explicit_result`"));
    }

    #[test]
    fn materializes_dynamic_component_semi_implicit_source_solve_request() {
        let source = r#"
domain Thermal {
    across T: AbsoluteTemperature [degC]
    through Q: HeatRate [kW]
    conservation sum(Q) = 0
}

component ZoneNode {
    port heat: Thermal
    1000 * der(heat.T) eq heat.Q
}

component HeatBoundary {
    port heat: Thermal
    heat.Q eq 1 kW
}

system DynamicSemiImplicit {
    zone = ZoneNode()
    boundary = HeatBoundary()
    connect zone.heat to boundary.heat
}

semi_result = solve component_graph
with {
    solver = dynamic_component_semi_implicit_euler
    timestep = 1 s
    duration = 3 s
    initial = 20 degC
    tolerance = 0.000001
    max_iter = 20
}
"#;
        let report = check_source("dynamic_semi.eng", source, &CheckOptions::default());
        assert!(!report.has_errors(), "{:?}", report.diagnostics);

        let runtime = materialize_runtime_data(&report, source);

        assert_eq!(runtime.component_solutions.len(), 1);
        let solution = &runtime.component_solutions[0];
        assert_eq!(solution.status, "computed");
        assert_eq!(
            solution.method,
            "dynamic_component_assembly_semi_implicit_euler"
        );
        assert_eq!(solution.tolerance, 0.000001);
        assert_eq!(solution.max_iterations, 20);
        assert_eq!(solution.step_diagnostics.len(), 4);
        assert!(solution
            .step_diagnostics
            .iter()
            .all(|diagnostic| { diagnostic.convergence_status == "linear_algebraic_converged" }));
        assert!(solution
            .trajectories
            .iter()
            .any(|trajectory| trajectory.name == "zone.heat.T"
                && trajectory.role == "state"
                && trajectory.point_count == 4
                && (trajectory.final_value - 19.997).abs() < 1e-9));
        assert!(solution
            .trajectories
            .iter()
            .any(|trajectory| trajectory.name == "zone.heat.Q"
                && trajectory.role == "algebraic"
                && trajectory.point_count == 4));
        assert!(solution.failure_artifact.is_none());
        assert!(solution
            .reason
            .contains("source solve binding `semi_result`"));
    }

    #[test]
    fn materializes_dynamic_component_source_failure_artifact() {
        let source = r#"
domain SingularState {
    across x: DimensionlessNumber [1]
    across z: DimensionlessNumber [1]
    through balance: DimensionlessNumber [1]
    conservation sum(balance) = 0
}

component SingularNode {
    port node: SingularState
    der(node.x) eq node.z
    node.balance eq 0
}

system DynamicSingular {
    node = SingularNode()
    connect node.node to node.node
}

singular_result = solve component_graph
with {
    solver = dynamic_component_semi_implicit_euler
    timestep = 1 s
    duration = 2 s
    initial = 1
    tolerance = 0.000001
    max_iter = 3
}
"#;
        let report = check_source("dynamic_failure.eng", source, &CheckOptions::default());
        assert!(!report.has_errors(), "{:?}", report.diagnostics);

        let runtime = materialize_runtime_data(&report, source);

        assert_eq!(runtime.component_solutions.len(), 1);
        let solution = &runtime.component_solutions[0];
        assert_eq!(solution.status, "failed");
        assert_eq!(
            solution.method,
            "dynamic_component_assembly_semi_implicit_euler"
        );
        assert_eq!(solution.convergence_status, "algebraic_solve_failed");
        assert_eq!(
            solution
                .failure_artifact
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("E-LINEAR-SINGULAR")
        );
        assert_eq!(solution.step_diagnostics.len(), 1);
        assert_eq!(
            solution.step_diagnostics[0].convergence_status,
            "linear_algebraic_solve_failed"
        );
        assert_eq!(
            solution.step_diagnostics[0]
                .failure_artifact
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("E-LINEAR-SINGULAR")
        );
        assert!(solution
            .reason
            .contains("source solve binding `singular_result`"));
    }

    #[test]
    fn materializes_fixed_point_nonconvergence_failure_from_source() {
        let source = r#"
domain Scalar {
    across x: DimensionlessNumber [1]
    through balance: DimensionlessNumber [1]
    conservation sum(balance) = 0
}

component DivergingLoop {
    port source: Scalar
    port target: Scalar
    source.x eq 1.5 * target.x
    source.balance eq 0
}

system FixedPointLoop {
    loop_node = DivergingLoop()
    connect loop_node.source to loop_node.target
}

fixed_point_result = solve component_graph
with {
    solver = fixed_point
    tolerance = 0.000001
    max_iter = 3
    relaxation = 1
    initial = 1
}
"#;
        let report = check_source(
            "fixed_point_nonconvergence.eng",
            source,
            &CheckOptions::default(),
        );
        assert!(!report.has_errors(), "{:?}", report.diagnostics);

        let runtime = materialize_runtime_data(&report, source);

        assert_eq!(runtime.component_solutions.len(), 1);
        let solution = &runtime.component_solutions[0];
        assert_eq!(solution.status, "fixed_point_not_converged");
        assert_eq!(solution.method, "fixed_point_residual_graph");
        assert_eq!(solution.convergence_status, "fixed_point_not_converged");
        assert_eq!(solution.iteration_count, 3);
        assert_eq!(
            solution
                .failure_artifact
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("E-FIXED-POINT-NONCONVERGENCE")
        );
        assert!(solution
            .reason
            .contains("source solve binding `fixed_point_result`"));
    }

    #[test]
    fn behavior_seed_prevents_claiming_linear_component_solve() {
        let behavior_report = check_source(
            "behavior.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent Source {\n    port out: Thermal\n    temperature_signal = out.T\n    delayed_temperature = delay(temperature_signal, 5 s)\n}\n\ncomponent Sink {\n    port inlet: Thermal\n}\n\nconnect Source.out -> Sink.inlet\n",
            &CheckOptions::default(),
        );
        let behavior_assembly = &behavior_report.semantic_program.component_assemblies[0];
        let square_assembly = square_linear_test_assembly("component_graph");
        let mut solution =
            RuntimeComponentSolution::from_solver_assembly("component_graph", &square_assembly);

        assert_eq!(solution.status, "solved_linear");
        assert!(solution.failure_artifact.is_none());

        annotate_component_behavior_solution(&mut solution, behavior_assembly);

        assert_eq!(solution.status, "not_solved_behavior_not_integrated");
        assert_eq!(solution.convergence_status, "behavior_graph_not_integrated");
        assert!(solution
            .variables
            .iter()
            .all(|variable| variable.status == "behavior_not_integrated"));
        assert_eq!(
            solution
                .failure_artifact
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some(COMPONENT_BEHAVIOR_NOT_INTEGRATED_CODE)
        );
        assert!(solution
            .reason
            .contains(COMPONENT_BEHAVIOR_NOT_INTEGRATED_NOTE));

        let mut spec = eng_report::report_spec_from_report(
            &behavior_report,
            "plots/plot_manifest.json",
            "abc123",
        );
        RuntimeData {
            component_solutions: vec![solution],
            ..RuntimeData::default()
        }
        .apply_component_solutions(&mut spec);
        let json = eng_report::report_spec_json(&spec);
        assert!(json.contains("\"status\": \"not_solved_behavior_not_integrated\""));
        assert!(json.contains("\"failure_code\": \"E-BEHAVIOR-NOT-INTEGRATED\""));
    }

    #[test]
    fn adapts_dynamic_component_solver_result_with_algebraic_trajectories() {
        use crate::solver::algorithms::dynamic_component::{
            solve_explicit_euler_with_algebraic, DynamicComponentOptions,
        };

        let input = SolverInput {
            plan: SolverPlan::new(
                "component_graph",
                SimulationPlan {
                    states: vec!["x".to_owned()],
                    outputs: vec!["x".to_owned(), "z".to_owned()],
                    ..SimulationPlan::default()
                },
                SolverOptions::fixed_step("dynamic_component_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(2.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![0.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };
        let algebraic_layout =
            StateLayout::new(vec![LayoutEntry::new(0, "z", "Dimensionless", "1", "1")]);
        let dynamic = solve_explicit_euler_with_algebraic(
            &input,
            algebraic_layout,
            vec![0.0],
            DynamicComponentOptions::default(),
            |sample| Ok(vec![0.5 * sample.state[0] + 1.0]),
            |sample| Ok(vec![sample.algebraic[0]]),
        )
        .unwrap();

        let solution = RuntimeComponentSolution::from_dynamic_component_result(
            "component_graph",
            &dynamic,
            "dynamic component SolverResult artifact adapter test",
        );

        assert_eq!(solution.status, "computed");
        assert_eq!(solution.method, "dynamic_component_explicit_euler");
        assert_eq!(solution.tolerance, 1e-9);
        assert_eq!(solution.max_iterations, 50);
        assert_eq!(solution.variables.len(), 2);
        assert_eq!(solution.trajectories.len(), 2);
        assert!(solution
            .variables
            .iter()
            .any(|variable| variable.name == "x"
                && variable.role == "state"
                && variable.value == 2.5));
        assert!(solution
            .variables
            .iter()
            .any(|variable| variable.name == "z"
                && variable.role == "algebraic"
                && variable.value == 2.25));
        assert!(solution
            .trajectories
            .iter()
            .any(|trajectory| trajectory.name == "z"
                && trajectory.role == "algebraic"
                && trajectory.point_count == 3
                && trajectory.points[2].y == 2.25));
        assert_eq!(solution.step_diagnostics.len(), 3);
        assert!(solution
            .step_diagnostics
            .iter()
            .all(
                |diagnostic| diagnostic.convergence_status == "fixed_point_converged"
                    && diagnostic.failure_artifact.is_none()
            ));
        let component_series =
            materialize_component_solution_series(std::slice::from_ref(&solution));
        assert!(component_series
            .iter()
            .any(|series| series.name == "component_graph.x"
                && series.axis == "Time"
                && series.points[2].y == 2.5));
        assert!(component_series
            .iter()
            .any(|series| series.name == "component_graph.z"
                && series.axis == "Time"
                && series.points[2].y == 2.25));

        let report = check_source_with_runtime_component_graph();
        let mut spec =
            eng_report::report_spec_from_report(&report, "plots/plot_manifest.json", "abc123");
        let runtime = RuntimeData {
            component_solutions: vec![solution],
            ..RuntimeData::default()
        };
        runtime.apply_component_solutions(&mut spec);
        let solver_result = spec.assemblies[0].solver_result.as_ref().unwrap();
        assert_eq!(solver_result.tolerance, 1e-9);
        assert_eq!(solver_result.max_iterations, 50);
        assert_eq!(solver_result.trajectories.len(), 2);
        assert!(solver_result
            .trajectories
            .iter()
            .any(|trajectory| trajectory.name == "z"
                && trajectory.role == "algebraic"
                && trajectory.final_value == 2.25));
        assert_eq!(solver_result.step_diagnostics.len(), 3);

        let json = eng_report::report_spec_json(&spec);
        let html = eng_report::render_html_with_spec(&report, "plots/timeseries.svg", &spec);
        assert!(json.contains("\"tolerance\": 0.000000001"));
        assert!(json.contains("\"max_iterations\": 50"));
        assert!(json.contains("\"trajectories\""));
        assert!(json.contains("\"step_diagnostics\""));
        assert!(json.contains("\"algebraic_iteration_count\""));
        assert!(json.contains("\"role\": \"algebraic\""));
        assert!(html.contains("Trajectories"));
        assert!(html.contains("Step Diagnostics"));
        assert!(html.contains("algebraic:z"));
    }

    #[test]
    fn adapts_dynamic_component_assembly_result_with_counts() {
        let assembly = dynamic_component_test_assembly("component_graph");
        let dynamic = crate::solver::solve_dynamic_component_assembly(
            &assembly,
            crate::solver::DynamicComponentAssemblySolveInput {
                duration_s: 1.0,
                timestep_s: 1.0,
                initial_state: vec![1.0],
                initial_algebraic: vec![0.0],
                inputs: vec![SolverScalar::new("u", "Dimensionless", "1", 5.0)],
                parameters: vec![SolverScalar::new("k", "Dimensionless", "1", 2.0)],
            },
            crate::solver::DynamicComponentOptions::default(),
        )
        .unwrap();

        let solution = RuntimeComponentSolution::from_dynamic_component_assembly_result(
            &assembly,
            &dynamic,
            "dynamic component assembly bridge artifact adapter test",
        );

        assert_eq!(solution.status, "computed");
        assert_eq!(
            solution.method,
            "dynamic_component_assembly_semi_implicit_euler"
        );
        assert_eq!(solution.equation_count, 2);
        assert_eq!(solution.unknown_count, 2);
        assert_eq!(solution.trajectories.len(), 2);
        assert!(solution
            .variables
            .iter()
            .any(|variable| variable.name == "x"
                && variable.role == "state"
                && variable.value == 3.0));
        assert!(solution
            .variables
            .iter()
            .any(|variable| variable.name == "z"
                && variable.role == "algebraic"
                && variable.value == 0.0));
        assert_eq!(solution.step_diagnostics.len(), 2);
        assert!(solution.failure_artifact.is_none());
    }

    #[test]
    fn adapts_dynamic_component_nonconvergence_failure_artifact() {
        use crate::solver::algorithms::dynamic_component::{
            solve_explicit_euler_with_algebraic, DynamicComponentOptions,
        };
        use crate::solver::algorithms::fixed_point::FixedPointOptions;

        let input = SolverInput {
            plan: SolverPlan::new(
                "component_graph",
                SimulationPlan {
                    states: vec!["x".to_owned()],
                    outputs: vec!["x".to_owned(), "z".to_owned()],
                    ..SimulationPlan::default()
                },
                SolverOptions::fixed_step("dynamic_component_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(2.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![0.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };
        let algebraic_layout =
            StateLayout::new(vec![LayoutEntry::new(0, "z", "Dimensionless", "1", "1")]);
        let dynamic = solve_explicit_euler_with_algebraic(
            &input,
            algebraic_layout,
            vec![0.0],
            DynamicComponentOptions {
                algebraic: FixedPointOptions {
                    tolerance: 1e-12,
                    max_iterations: 3,
                    relaxation: 1.0,
                },
            },
            |sample| Ok(vec![sample.algebraic[0] + 1.0]),
            |_sample| Ok(vec![0.0]),
        )
        .unwrap();

        let solution = RuntimeComponentSolution::from_dynamic_component_result(
            "component_graph",
            &dynamic,
            "dynamic component nonconvergence adapter test",
        );

        assert_eq!(solution.status, "failed");
        assert_eq!(solution.convergence_status, "algebraic_solve_failed");
        assert_eq!(solution.tolerance, 1e-12);
        assert_eq!(solution.max_iterations, 3);
        assert_eq!(solution.iteration_count, 3);
        assert_eq!(
            solution
                .failure_artifact
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("E-FIXED-POINT-NONCONVERGENCE")
        );
        assert_eq!(solution.step_diagnostics.len(), 1);
        assert_eq!(
            solution.step_diagnostics[0].convergence_status,
            "fixed_point_not_converged"
        );
        assert_eq!(
            solution.step_diagnostics[0]
                .failure_artifact
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("E-FIXED-POINT-NONCONVERGENCE")
        );

        let report = check_source_with_runtime_component_graph();
        let mut spec =
            eng_report::report_spec_from_report(&report, "plots/plot_manifest.json", "abc123");
        let runtime = RuntimeData {
            component_solutions: vec![solution],
            ..RuntimeData::default()
        };
        runtime.apply_component_solutions(&mut spec);
        let solver_result = spec.assemblies[0].solver_result.as_ref().unwrap();
        assert_eq!(solver_result.status, "failed");
        assert_eq!(solver_result.convergence_status, "algebraic_solve_failed");
        assert_eq!(
            solver_result
                .failure_artifact
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("E-FIXED-POINT-NONCONVERGENCE")
        );
        assert_eq!(
            solver_result.step_diagnostics[0]
                .failure_artifact
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("E-FIXED-POINT-NONCONVERGENCE")
        );

        let json = eng_report::report_spec_json(&spec);
        let html = eng_report::render_html_with_spec(&report, "plots/timeseries.svg", &spec);
        assert!(json.contains("\"convergence_status\": \"algebraic_solve_failed\""));
        assert!(json.contains("\"failure_code\": \"E-FIXED-POINT-NONCONVERGENCE\""));
        assert!(json.contains("\"failure_reason\""));
        assert!(json.contains("\"convergence_status\": \"fixed_point_not_converged\""));
        assert!(html.contains("failed@0 E-FIXED-POINT-NONCONVERGENCE"));
        assert!(html.contains("E-FIXED-POINT-NONCONVERGENCE"));
    }

    #[test]
    fn reports_singular_square_component_residual_graph_failure() {
        let mut assembly = square_linear_test_assembly("component_graph");
        assembly.generated_equations[0].kind = "through_conservation".to_owned();
        assembly.generated_equations[0].residual = "x + y".to_owned();

        let solution = RuntimeComponentSolution::from_solver_assembly("component_graph", &assembly);

        assert_eq!(solution.status, "linear_solve_failed");
        assert_eq!(solution.method, "dense_linear_residual_graph");
        assert_eq!(solution.convergence_status, "linear_failed");
        assert_eq!(
            solution
                .failure_artifact
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("E-LINEAR-SINGULAR")
        );
    }

    #[test]
    fn reports_overdetermined_component_residual_graph_limitation() {
        let mut assembly = square_linear_test_assembly("component_graph");
        assembly.generated_equations.push(GeneratedEquation {
            name: "r3".to_owned(),
            kind: "component_boundary".to_owned(),
            domain: "Test".to_owned(),
            expression: "x eq 1".to_owned(),
            residual: "x - 1".to_owned(),
            rhs_value: Some(1.0),
            dependencies: vec!["x".to_owned()],
            source: "test".to_owned(),
            reason: "test overdetermined boundary".to_owned(),
            source_line: Some(3),
            status: "generated".to_owned(),
        });

        let solution = RuntimeComponentSolution::from_solver_assembly("component_graph", &assembly);

        assert_eq!(solution.status, "not_solved_overdetermined");
        assert_eq!(solution.method, "linear_residual_graph_shape_check");
        assert_eq!(
            solution.convergence_status,
            "linear_residual_not_attempted_overdetermined"
        );
        assert_eq!(
            solution
                .failure_artifact
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("E-ASSEMBLY-OVERDETERMINED")
        );
    }

    #[test]
    fn uses_compiler_canonical_state_space_matrix_coefficients() {
        let report = check_source(
            "ok.eng",
            "system CanonicalStateSpace {\n    state T1: AbsoluteTemperature = 22 degC\n    state T2: AbsoluteTemperature = 21 degC\n    state T3: AbsoluteTemperature = 20 degC\n\n    states x = [T1, T2, T3]\n    A: LinearOperator[StateVector -> Derivative[StateVector]] = [[60 1/min, 2 1/h, -0.5]; [0, 0, 0]; [0, 0, 0]]\n\n    equation {\n        der(x) eq A * x\n    }\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        let matrix = report.semantic_program.linear_operators[0]
            .canonical_matrix
            .as_ref()
            .unwrap();

        assert_eq!(matrix.len(), 3);
        assert!((matrix[0][0] - 1.0).abs() < 1e-12);
        assert!((matrix[0][1] - (2.0 / 3600.0)).abs() < 1e-12);
        assert_eq!(matrix[0][2], -0.5);
    }

    #[test]
    fn materializes_one_state_state_space_solution() {
        let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/internal/18_state_space_metadata/main.eng");
        let source = std::fs::read_to_string(&source_path).unwrap();
        let report = check_file(&source_path, &CheckOptions::default()).unwrap();
        let runtime = materialize_runtime_data(&report, &source);

        assert_eq!(runtime.system_solutions.len(), 1);
        let solution = &runtime.system_solutions[0];
        assert_eq!(solution.system, "ThermalStateSpaceMetadata");
        assert_eq!(solution.binding.as_deref(), Some("sim"));
        assert_eq!(solution.status, "computed");
        assert_eq!(solution.method, "state_space_explicit_euler_fixed_step");
        assert!(solution.reason.contains("TimeSeries input materialization"));
        assert_eq!(solution.state, "T_zone");
        assert_eq!(solution.states, vec!["T_zone".to_owned()]);
        assert!(solution.algebraic_variables.is_empty());
        assert_eq!(
            solution.inputs,
            vec!["T_out".to_owned(), "Q_internal".to_owned()]
        );
        assert!(solution.parameters.is_empty());
        assert_eq!(solution.outputs, vec!["T_zone".to_owned()]);
        assert_eq!(solution.time_step_s, 600.0);
        assert_eq!(solution.duration_s, 3600.0);
        assert_eq!(solution.step_count, 6);
        assert_eq!(solution.tolerance, 1e-9);
        assert_eq!(solution.max_iterations, 1);
        assert_eq!(solution.iteration_count, 6);
        assert_eq!(solution.convergence_status, "fixed_step_completed");
        assert!(solution.failure_reason.is_none());
        assert_eq!(solution.points.len(), 7);
        assert!(solution.final_value.is_finite());
        assert!(runtime
            .time_series
            .iter()
            .any(|series| series.name == "sim.T_zone"));
    }

    #[test]
    fn materializes_adaptive_state_space_solution() {
        let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/internal/28_adaptive_state_space/main.eng");
        let source = std::fs::read_to_string(&source_path).unwrap();
        let report = check_file(&source_path, &CheckOptions::default()).unwrap();
        let runtime = materialize_runtime_data(&report, &source);

        assert_eq!(runtime.system_solutions.len(), 1);
        let solution = &runtime.system_solutions[0];
        assert_eq!(solution.system, "AdaptiveStateSpace");
        assert_eq!(solution.binding.as_deref(), Some("sim"));
        assert_eq!(solution.status, "computed");
        assert_eq!(solution.method, "adaptive_heun");
        assert_eq!(solution.convergence_status, "adaptive_heun_completed");
        assert_eq!(solution.tolerance, 0.0001);
        assert!(solution.iteration_count >= solution.step_count);
        assert_eq!(solution.step_count, 3);
        assert_eq!(solution.duration_s, 1800.0);
        assert!(solution
            .reason
            .contains("continuous state-space A/B operators"));
        assert!(solution.reason.contains("adaptive Heun"));
        assert_eq!(solution.points.len(), solution.step_count + 1);
        assert!(!solution.step_diagnostics.is_empty());
        assert!(solution
            .step_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.status == "accepted"));
        assert!(solution
            .step_diagnostics
            .iter()
            .all(|diagnostic| diagnostic.output_index <= solution.step_count));
        assert_eq!(
            solution.to_report_solution().step_diagnostics.len(),
            solution.step_diagnostics.len()
        );
        assert!(solution.final_value.is_finite());
        assert!(runtime
            .time_series
            .iter()
            .any(|series| series.name == "sim.T_zone"));
    }

    #[test]
    fn materializes_multi_state_state_space_solution() {
        let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/internal/20_multi_state_thermal/main.eng");
        let source = std::fs::read_to_string(&source_path).unwrap();
        let report = check_file(&source_path, &CheckOptions::default()).unwrap();
        let runtime = materialize_runtime_data(&report, &source);

        let sim_solutions = runtime
            .system_solutions
            .iter()
            .filter(|solution| solution.binding.as_deref() == Some("sim"))
            .collect::<Vec<_>>();
        assert_eq!(sim_solutions.len(), 2);
        assert!(sim_solutions
            .iter()
            .all(|solution| solution.status == "computed"));
        assert!(sim_solutions
            .iter()
            .all(|solution| solution.method == "state_space_rk4_fixed_step"));
        assert!(sim_solutions
            .iter()
            .all(|solution| solution.reason.contains("multi-state")));
        assert!(sim_solutions
            .iter()
            .all(|solution| solution.states == vec!["T_air".to_owned(), "T_wall".to_owned()]));
        assert!(sim_solutions
            .iter()
            .all(|solution| solution.outputs == vec!["T_air".to_owned(), "T_wall".to_owned()]));
        assert!(sim_solutions
            .iter()
            .all(|solution| solution.convergence_status == "fixed_step_completed"));
        assert!(sim_solutions
            .iter()
            .any(|solution| solution.state == "T_air"));
        assert!(sim_solutions
            .iter()
            .any(|solution| solution.state == "T_wall"));
        assert!(runtime
            .time_series
            .iter()
            .any(|series| series.name == "sim.T_air"));
        assert!(runtime
            .time_series
            .iter()
            .any(|series| series.name == "sim.T_wall"));
    }

    #[test]
    fn materializes_typed_block_state_space_solution() {
        let source = r#"
states ZoneState {
    T_air: AbsoluteTemperature [degC]
    T_wall: AbsoluteTemperature [degC]
}

inputs ZoneInput {
    Q_hvac: HeatRate [W]
}

system ZoneSS {
    state x: StateVector[ZoneState] = [20 degC, 19 degC]
    input u: InputVector[ZoneInput] = [1000 W]

    operator A: LinearOperator[ZoneState -> Derivative[ZoneState]] = [[-0.01 1/min, 0.01 1/min]; [0.02 1/min, -0.02 1/min]]
    operator B: LinearOperator[ZoneInput -> Derivative[ZoneState]] = [[0.000001]; [0.0]]

    equation {
        der(x) eq A * x + B * u
    }
}

sim = simulate ZoneSS
with {
    timestep = 10 min
    duration = 30 min
    solver = rk4
}
"#;
        let report = check_source("typed_state_space.eng", source, &CheckOptions::default());
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let runtime = materialize_runtime_data(&report, source);

        let sim_solutions = runtime
            .system_solutions
            .iter()
            .filter(|solution| solution.binding.as_deref() == Some("sim"))
            .collect::<Vec<_>>();
        assert_eq!(sim_solutions.len(), 2);
        assert!(sim_solutions
            .iter()
            .all(|solution| solution.method == "state_space_rk4_fixed_step"));
        assert!(sim_solutions
            .iter()
            .all(|solution| solution.status == "computed"));
        assert!(sim_solutions
            .iter()
            .all(|solution| solution.states == vec!["T_air".to_owned(), "T_wall".to_owned()]));
        assert!(sim_solutions
            .iter()
            .any(|solution| solution.state == "T_air" && solution.final_value.is_finite()));
        assert!(sim_solutions
            .iter()
            .any(|solution| solution.state == "T_wall" && solution.final_value.is_finite()));
        assert!(runtime
            .time_series
            .iter()
            .any(|series| series.name == "sim.T_air"));
        assert!(runtime
            .time_series
            .iter()
            .any(|series| series.name == "sim.T_wall"));
    }

    #[test]
    fn state_space_runtime_adapter_respects_output_layout() {
        let source = r#"
system OutputSubsetStateSpace {
    state T_air: AbsoluteTemperature = 20 degC
    state T_wall: AbsoluteTemperature = 20 degC
    input Q_hvac: HeatRate = 1000 W

    states x = [T_air, T_wall]
    inputs u = [Q_hvac]
    outputs y = [T_air]

    A: LinearOperator[StateVector -> Derivative[StateVector]] = [[1.0, 0.0]; [0.0, 1.0]]
    B: LinearOperator[InputVector -> Derivative[StateVector]] = [[0.001]; [0.002]]

    equation {
        next(x) eq A * x + B * u
    }
}

sim = simulate OutputSubsetStateSpace
with {
    timestep = 10 min
    solver = fixed_step
}
"#;
        let report = check_source("output_subset.eng", source, &CheckOptions::default());
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let runtime = materialize_runtime_data(&report, source);

        let sim_solutions = runtime
            .system_solutions
            .iter()
            .filter(|solution| solution.binding.as_deref() == Some("sim"))
            .collect::<Vec<_>>();
        assert_eq!(sim_solutions.len(), 1);
        assert_eq!(sim_solutions[0].state, "T_air");
        assert_eq!(sim_solutions[0].outputs, vec!["T_air".to_owned()]);
        assert!(runtime
            .time_series
            .iter()
            .any(|series| series.name == "sim.T_air"));
        assert!(!runtime
            .time_series
            .iter()
            .any(|series| series.name == "sim.T_wall"));
    }

    #[test]
    fn state_space_solver_failure_materializes_failed_solution() {
        let overflowing_coefficient = format!("1{}", "0".repeat(306));
        let source = format!(
            r#"
system FailingStateSpace {{
    state T_air: AbsoluteTemperature = 20 degC
    state T_wall: AbsoluteTemperature = 20 degC
    input Q_hvac: HeatRate = 1000 W

    states x = [T_air, T_wall]
    inputs u = [Q_hvac]
    outputs y = [T_air]

    A: LinearOperator[StateVector -> Derivative[StateVector]] = [[{overflowing_coefficient}, 0.0]; [0.0, 1.0]]
    B: LinearOperator[InputVector -> Derivative[StateVector]] = [[0.0]; [0.0]]

    equation {{
        next(x) eq A * x + B * u
    }}
}}

sim = simulate FailingStateSpace
with {{
    timestep = 10 min
    solver = fixed_step
}}
"#
        );
        let report = check_source("failing_state_space.eng", &source, &CheckOptions::default());
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let runtime = materialize_runtime_data(&report, &source);

        let sim_solutions = runtime
            .system_solutions
            .iter()
            .filter(|solution| solution.binding.as_deref() == Some("sim"))
            .collect::<Vec<_>>();
        assert_eq!(sim_solutions.len(), 1);
        let solution = sim_solutions[0];
        assert_eq!(solution.status, "failed");
        assert_eq!(solution.state, "T_air");
        assert_eq!(solution.outputs, vec!["T_air".to_owned()]);
        assert_eq!(solution.failure_code.as_deref(), Some("E-RHS-STATE-FINITE"));
        assert!(solution
            .failure_reason
            .as_deref()
            .unwrap()
            .contains("discrete state-space state"));
        assert!(solution.reason.contains("solver evaluation failed"));
        assert!(!runtime
            .time_series
            .iter()
            .any(|series| series.name == "sim.T_air"));
    }

    #[test]
    fn materializes_discrete_state_space_solution() {
        let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/internal/26_state_space_discrete/main.eng");
        let source = std::fs::read_to_string(&source_path).unwrap();
        let report = check_file(&source_path, &CheckOptions::default()).unwrap();
        let runtime = materialize_runtime_data(&report, &source);

        let sim_solutions = runtime
            .system_solutions
            .iter()
            .filter(|solution| solution.binding.as_deref() == Some("sim"))
            .collect::<Vec<_>>();
        assert_eq!(sim_solutions.len(), 2);
        let air = sim_solutions
            .iter()
            .find(|solution| solution.state == "T_air")
            .unwrap();
        let wall = sim_solutions
            .iter()
            .find(|solution| solution.state == "T_wall")
            .unwrap();
        assert_eq!(air.method, "state_space_discrete_fixed_step");
        assert_eq!(air.step_count, 6);
        assert_eq!(round2(air.final_value), 26.0);
        assert_eq!(round2(wall.final_value), 32.0);
    }

    #[test]
    fn materializes_ml_metrics_and_parity_plot() {
        let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/internal/05_data_driven_modeling/main.eng");
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
        assert!(!mlp.loss_history.is_empty());
        assert!(mlp
            .loss_history
            .iter()
            .all(|loss| loss.is_finite() && *loss >= 0.0));
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

        let residual_source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/internal/05_data_driven_modeling/residuals.eng");
        let residual_source = std::fs::read_to_string(&residual_source_path).unwrap();
        let residual_report = check_file(&residual_source_path, &CheckOptions::default()).unwrap();
        let residual_runtime = materialize_runtime_data(&residual_report, &residual_source);
        let mut residual_plot_spec = eng_report::plot_spec_from_report(&residual_report);
        residual_runtime.apply_plot_spec(&residual_report, &mut residual_plot_spec);

        assert_eq!(residual_plot_spec.plot_type, "bar");
        assert_eq!(residual_plot_spec.title, "Regression residuals");
        assert_eq!(residual_plot_spec.y_axis.name, "Residual");
        assert!(!residual_plot_spec.series[0].points.is_empty());
    }

    #[test]
    fn materializes_timeseries_histogram_plot() {
        let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/official/01_csv_plot/histogram.eng");
        let source = std::fs::read_to_string(&source_path).unwrap();
        let report = check_file(&source_path, &CheckOptions::default()).unwrap();
        let runtime = materialize_runtime_data(&report, &source);
        let mut plot_spec = eng_report::plot_spec_from_report(&report);
        runtime.apply_plot_spec(&report, &mut plot_spec);

        assert_eq!(plot_spec.plot_type, "histogram");
        assert_eq!(plot_spec.title, "Coil heat-rate distribution");
        assert_eq!(plot_spec.x_axis.unit, "kW");
        assert_eq!(plot_spec.y_axis.unit, "count");
        assert!(!plot_spec.series[0].bins.is_empty());
        assert_eq!(
            plot_spec.series[0].points.len(),
            plot_spec.series[0].bins.len()
        );
    }

    #[test]
    fn materializes_time_axes_from_table_indexes() {
        let tables = vec![
            time_axis_table(
                "weather",
                &[
                    "2026-01-01T00:00:00Z",
                    "2026-01-01T00:05:00Z",
                    "2026-01-01T00:10:00Z",
                ],
            ),
            time_axis_table(
                "measured",
                &[
                    "2026-01-01T00:00:00Z",
                    "2026-01-01T00:05:00Z",
                    "2026-01-01T00:20:00Z",
                ],
            ),
        ];

        let axes = materialize_time_axes(&tables);

        assert_eq!(axes.len(), 2);
        assert_eq!(axes[0].name, "weather.Time");
        assert_eq!(axes[0].source_column, "timestamp");
        assert_eq!(axes[0].start, Some(0.0));
        assert_eq!(axes[0].end, Some(600.0));
        assert_eq!(axes[0].count, 3);
        assert_eq!(axes[0].nominal_step, Some(300.0));
        assert!(!axes[0].irregular);
        assert_eq!(axes[0].missing_count, 0);

        assert_eq!(axes[1].nominal_step, Some(300.0));
        assert!(axes[1].irregular);
    }

    #[test]
    fn materializes_time_alignment_step_metadata() {
        let series = vec![
            time_series_for_alignment("left", "table_a", &[0.0, 60.0, 120.0, 180.0]),
            time_series_for_alignment("right", "table_b", &[0.0, 120.0, 240.0, 360.0]),
            time_series_for_alignment("irregular", "table_c", &[0.0, 60.0, 150.0, 210.0]),
        ];

        let alignments = materialize_time_alignments(&series);

        assert_eq!(alignments.len(), 3);
        let step_mismatch = alignments
            .iter()
            .find(|alignment| alignment.left == "left" && alignment.right == "right")
            .unwrap();
        assert_eq!(step_mismatch.left_nominal_step, Some(60.0));
        assert_eq!(step_mismatch.right_nominal_step, Some(120.0));
        assert!(!step_mismatch.left_irregular);
        assert!(!step_mismatch.right_irregular);
        assert_eq!(step_mismatch.step_status, "mismatch");

        let irregular = alignments
            .iter()
            .find(|alignment| alignment.left == "left" && alignment.right == "irregular")
            .unwrap();
        assert_eq!(irregular.right_nominal_step, Some(60.0));
        assert!(irregular.right_irregular);
        assert_eq!(irregular.step_status, "mismatch");
    }

    #[test]
    fn materializes_rmse_metric_alignment_reference() {
        let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/internal/17_measured_vs_simulated/main.eng");
        let source = std::fs::read_to_string(&source_path).unwrap();
        let report = check_file(&source_path, &CheckOptions::default()).unwrap();
        let runtime = materialize_runtime_data(&report, &source);

        let room = report
            .semantic_program
            .systems
            .iter()
            .find(|system| system.name == "RoomThermal")
            .unwrap();
        let outdoor_input = room
            .variables
            .iter()
            .find(|variable| variable.name == "T_out")
            .unwrap();
        assert_eq!(
            outdoor_input.quantity_kind,
            "TimeSeries[Time] of AbsoluteTemperature"
        );
        let solution = runtime
            .system_solutions
            .iter()
            .find(|solution| solution.binding.as_deref() == Some("sim"))
            .unwrap();
        assert_eq!(solution.status, "computed");
        assert_eq!(solution.method, "explicit_euler_fixed_step");
        assert!(solution.reason.contains("SolverResult"));
        assert_eq!(solution.states, vec!["T_zone".to_owned()]);
        assert_eq!(
            solution.inputs,
            vec!["T_out".to_owned(), "Q_internal".to_owned()]
        );
        assert_eq!(solution.parameters, vec!["C".to_owned(), "UA".to_owned()]);
        assert_eq!(solution.outputs, vec!["T_zone".to_owned()]);
        assert_eq!(solution.tolerance, 1e-9);
        assert_eq!(solution.iteration_count, solution.step_count);
        assert_eq!(solution.convergence_status, "fixed_step_completed");
        assert!(solution.failure_reason.is_none());
        assert_eq!(solution.points.len(), solution.step_count + 1);
        assert!(runtime
            .time_series
            .iter()
            .any(|series| series.name == "sim.T_zone"));

        let metric = runtime
            .metrics
            .iter()
            .find(|metric| metric.binding == "rmse_T")
            .unwrap();

        assert_eq!(
            metric.alignment_reference.as_deref(),
            Some("measured_data.T_zone vs sim.T_zone")
        );
        assert!(metric.alignment_status.is_some());
        assert_eq!(metric.alignment_step_status.as_deref(), Some("matched"));
    }

    #[test]
    fn materializes_one_state_adaptive_heun_solution() {
        let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/internal/27_adaptive_heun_thermal/main.eng");
        let source = std::fs::read_to_string(&source_path).unwrap();
        let report = check_file(&source_path, &CheckOptions::default()).unwrap();
        assert!(!report.has_errors());

        let runtime = materialize_runtime_data(&report, &source);
        let solution = runtime
            .system_solutions
            .iter()
            .find(|solution| solution.binding.as_deref() == Some("sim"))
            .unwrap();

        assert_eq!(solution.status, "computed");
        assert_eq!(solution.method, "adaptive_heun");
        assert_eq!(solution.convergence_status, "adaptive_heun_completed");
        assert_eq!(solution.tolerance, 0.0001);
        assert!(solution.iteration_count > solution.step_count);
        assert!(solution.reason.contains("adaptive Heun"));
        assert_eq!(solution.step_count, 3);
        assert_eq!(solution.states, vec!["T_zone".to_owned()]);
        assert_eq!(solution.outputs, vec!["T_zone".to_owned()]);
        assert_eq!(solution.points.len(), solution.step_count + 1);
        assert!(!solution.step_diagnostics.is_empty());
        assert!(solution
            .step_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.status == "accepted"));
        assert!(solution
            .step_diagnostics
            .iter()
            .all(|diagnostic| diagnostic.output_index <= solution.step_count));
        assert_eq!(
            solution.to_report_solution().step_diagnostics.len(),
            solution.step_diagnostics.len()
        );
    }

    #[test]
    fn materializes_two_state_source_ode_fixed_step_solutions() {
        for (solver_name, method_name) in [
            ("explicit_euler", "explicit_euler_fixed_step"),
            ("rk4", "rk4_fixed_step"),
        ] {
            let source = format!(
                r#"
system TwoStateSourceThermal {{
    parameter C_air: HeatCapacity = 100 kJ/K
    parameter C_wall: HeatCapacity = 200 kJ/K
    parameter UA_aw: Conductance = 50 W/K
    parameter UA_ao: Conductance = 100 W/K
    parameter UA_wo: Conductance = 20 W/K
    parameter T_out: AbsoluteTemperature = 12 degC
    parameter Q_hvac: HeatRate = 1000 W

    state T_air: AbsoluteTemperature = 22 degC
    state T_wall: AbsoluteTemperature = 20 degC

    equation {{
        C_air * der(T_air) eq UA_aw * (T_wall - T_air) + UA_ao * (T_out - T_air) + Q_hvac
        C_wall * der(T_wall) eq UA_aw * (T_air - T_wall) + UA_wo * (T_out - T_wall)
    }}
}}

sim = simulate TwoStateSourceThermal
with {{
    timestep = 10 min
    duration = 20 min
    solver = {solver_name}
}}
"#
            );
            let report = eng_compiler::check_source(
                "two_state_source.eng",
                &source,
                &CheckOptions::default(),
            );
            assert!(!report.has_errors(), "{:?}", report.diagnostics);

            let runtime = materialize_runtime_data(&report, &source);
            let solutions = runtime
                .system_solutions
                .iter()
                .filter(|solution| solution.binding.as_deref() == Some("sim"))
                .collect::<Vec<_>>();

            assert_eq!(solutions.len(), 2);
            assert!(solutions
                .iter()
                .all(|solution| solution.status == "computed"));
            assert!(solutions
                .iter()
                .all(|solution| solution.method == method_name));
            assert!(solutions
                .iter()
                .all(|solution| solution.reason.contains("source derivative equations")));
            assert!(solutions.iter().all(|solution| {
                solution.states == vec!["T_air".to_owned(), "T_wall".to_owned()]
                    && solution.outputs == vec!["T_air".to_owned(), "T_wall".to_owned()]
                    && solution.points.len() == 3
            }));
            let air = solutions
                .iter()
                .find(|solution| solution.state == "T_air")
                .unwrap();
            assert!(air.final_value < air.initial_value);
            assert!(runtime
                .time_series
                .iter()
                .any(|series| series.name == "sim.T_air"));
            assert!(runtime
                .time_series
                .iter()
                .any(|series| series.name == "sim.T_wall"));
        }
    }

    #[test]
    fn first_order_thermal_model_failure_materializes_failed_solution() {
        let source = r#"
system BadRoomThermal {
    parameter C: HeatCapacity = 0 J/K
    parameter UA: Conductance = 150 W/K

    state T: AbsoluteTemperature = 24 degC

    input T_out: AbsoluteTemperature = 10 degC
    input Q_internal: HeatRate = 500 W

    equation {
        C * der(T) eq UA * (T_out - T) + Q_internal
    }
}
"#;
        let report = eng_compiler::check_source("bad_room.eng", source, &CheckOptions::default());
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let runtime = materialize_runtime_data(&report, source);

        assert_eq!(runtime.system_solutions.len(), 1);
        let solution = &runtime.system_solutions[0];
        assert_eq!(solution.status, "failed");
        assert_eq!(solution.binding.as_deref(), None);
        assert_eq!(solution.method, "explicit_euler_fixed_step");
        assert_eq!(solution.state, "T");
        assert_eq!(
            solution.failure_code.as_deref(),
            Some("E-SOLVER-THERMAL-CAPACITY-INVALID")
        );
        assert!(solution
            .failure_reason
            .as_deref()
            .unwrap()
            .contains("positive finite heat capacity"));
        assert!(solution.reason.contains("thermal model validation failed"));
        assert_eq!(solution.points.len(), 1);
        assert!(runtime
            .time_series
            .iter()
            .all(|series| series.name != "BadRoomThermal.T"));
    }

    #[test]
    fn records_skipped_system_solution_for_unsupported_simulate_shape() {
        let source = r#"
system UnsupportedThermal {
    state T: AbsoluteTemperature = 24 degC
}

sim = simulate UnsupportedThermal
with {
    timestep = 10 min
    solver = fixed_step
}
"#;
        let report = eng_compiler::check_source("ok.eng", source, &CheckOptions::default());
        assert!(!report.has_errors());

        let runtime = materialize_runtime_data(&report, source);

        assert_eq!(runtime.system_solutions.len(), 1);
        let solution = &runtime.system_solutions[0];
        assert_eq!(solution.status, "skipped_unsupported_shape");
        assert_eq!(solution.binding.as_deref(), Some("sim"));
        assert_eq!(solution.method, "explicit_euler_fixed_step");
        assert_eq!(solution.time_step_s, 600.0);
        assert_eq!(solution.step_count, 0);
        assert_eq!(
            solution.failure_code.as_deref(),
            Some("E-SIM-SYSTEM-SHAPE-UNSUPPORTED")
        );
        assert!(solution
            .failure_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("supported state-space/source ODE")));
        assert_eq!(
            solution.to_report_solution().failure_code.as_deref(),
            Some("E-SIM-SYSTEM-SHAPE-UNSUPPORTED")
        );
        assert!(solution.points.is_empty());
        assert!(runtime
            .time_series
            .iter()
            .all(|series| series.name != "sim.T"));
    }

    fn time_axis_table(binding: &str, timestamps: &[&str]) -> RuntimeTable {
        RuntimeTable {
            binding: binding.to_owned(),
            schema_name: format!("{binding}Schema"),
            source: format!("{binding}.csv"),
            source_hash: Some(format!("{binding}-hash")),
            row_count: timestamps.len(),
            columns: vec![RuntimeColumn {
                name: "timestamp".to_owned(),
                type_name: "DateTime".to_owned(),
                unit: None,
                canonical_unit: None,
                is_index: true,
                values: RuntimeValues::Text(
                    timestamps.iter().map(|value| value.to_string()).collect(),
                ),
                canonical_values: Vec::new(),
                missing_count: 0,
                conversion_failures: Vec::new(),
            }],
            parse_failures: Vec::new(),
        }
    }

    fn time_series_for_alignment(name: &str, table: &str, xs: &[f64]) -> RuntimeTimeSeries {
        RuntimeTimeSeries {
            name: name.to_owned(),
            axis: "Time".to_owned(),
            x_unit: "s".to_owned(),
            quantity_kind: "Temperature".to_owned(),
            display_unit: "K".to_owned(),
            source_table: table.to_owned(),
            source_expression: String::new(),
            points: xs
                .iter()
                .enumerate()
                .map(|(index, x)| RuntimePoint {
                    x: *x,
                    y: index as f64,
                })
                .collect(),
        }
    }

    #[test]
    fn official_thermal_fluid_loop_solves_with_parameter_rhs() {
        let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/official/32_small_thermal_fluid_loop/main.eng");
        let report = check_file(&source_path, &CheckOptions::default()).unwrap();

        let runtime = materialize_runtime_data(&report, "");
        let solution = runtime
            .component_solutions
            .iter()
            .find(|solution| solution.assembly == "component_graph")
            .unwrap();

        assert_eq!(solution.status, "solved_linear");
        assert!(solution
            .variables
            .iter()
            .any(|variable| variable.name == "pump.supply.p"
                && (variable.value - 220000.0).abs() <= 1e-9));
        assert!(solution
            .residuals
            .iter()
            .any(|residual| residual.name == "pump.boundary_supply_pressure"
                && residual.value.abs() <= 1e-9));
    }
    #[test]
    fn source_component_parameters_materialize_solver_values() {
        let source = r#"
domain Fluid[Medium M] {
    across p: Pressure [Pa]
    through m_dot: MassFlowRate [kg/s]
    conservation sum(m_dot) = 0
}

component PumpBoundary {
    port supply: Fluid[Water]
    parameter p_supply: Pressure [Pa] = 200000 Pa
    supply_pressure = supply.p = p_supply
}

component PipeRun {
    port inlet: Fluid[Water]
    port outlet: Fluid[Water]
    parameter dp: Pressure [Pa] = 20000 Pa
    outlet.p + dp eq inlet.p
}

component ReturnBoundary {
    port inlet: Fluid[Water]
}

const P_SUPPLY: Pressure [Pa] = 220000 Pa

system FluidLoop {
    pump = PumpBoundary(P_SUPPLY)
    pipe = PipeRun()
    return_node = ReturnBoundary()
    connect pump.supply to pipe.inlet
    connect pipe.outlet to return_node.inlet
}
"#;
        let report = check_source(
            "parameterized_fluid_loop.eng",
            source,
            &CheckOptions::default(),
        );
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let pump = report
            .semantic_program
            .components
            .iter()
            .find(|component| component.name == "pump")
            .unwrap();
        assert_eq!(pump.constructor_arguments[0].name, "p_supply");
        assert_eq!(pump.constructor_arguments[0].value, "P_SUPPLY");
        let assembly = report
            .semantic_program
            .component_assemblies
            .iter()
            .find(|assembly| assembly.name == "component_graph")
            .unwrap();
        let solver_assembly = solver_equation_assembly_from_component_info(&report, assembly);

        assert!(solver_assembly
            .parameters
            .iter()
            .any(
                |parameter| parameter.name == "pump.p_supply" && parameter.value == Some(220000.0)
            ));
        assert!(solver_assembly
            .parameters
            .iter()
            .any(|parameter| parameter.name == "pipe.dp" && parameter.value == Some(20000.0)));

        let residual_graph = ResidualGraph::from_assembly(&solver_assembly);
        let supply = residual_graph
            .residuals
            .iter()
            .find(|residual| residual.name == "pump.boundary_supply_pressure")
            .unwrap();
        assert_eq!(supply.rhs_value, 220000.0);
        let pressure_drop = residual_graph
            .residuals
            .iter()
            .find(|residual| residual.name == "pipe.equation_1")
            .unwrap();
        assert_eq!(pressure_drop.rhs_value, -20000.0);
    }
    #[test]
    fn source_residual_evaluation_uses_component_parameter_values() {
        let x = UnknownVariable {
            name: "x".to_owned(),
            role: "algebraic".to_owned(),
            quantity_kind: "Dimensionless".to_owned(),
            unit: "1".to_owned(),
            source: "Test.x".to_owned(),
            status: "unknown".to_owned(),
            value: None,
        };
        let k = UnknownVariable {
            name: "k".to_owned(),
            role: "parameter".to_owned(),
            quantity_kind: "Dimensionless".to_owned(),
            unit: "1".to_owned(),
            source: "component_parameter.Dimensionless".to_owned(),
            status: "defaulted".to_owned(),
            value: Some(2.0),
        };
        let assembly = EquationAssembly {
            name: "parametric_source".to_owned(),
            generated_equations: vec![GeneratedEquation {
                name: "r1".to_owned(),
                kind: "component_equation".to_owned(),
                domain: "Test".to_owned(),
                expression: "x eq k".to_owned(),
                residual: "x - k".to_owned(),
                rhs_value: None,
                dependencies: vec!["x".to_owned(), "k".to_owned()],
                source: "test".to_owned(),
                reason: "test parameterized source residual".to_owned(),
                source_line: Some(1),
                status: "generated".to_owned(),
            }],
            unknowns: vec![x.clone()],
            algebraic_variables: vec![x],
            parameters: vec![k],
            ..EquationAssembly::default()
        };
        let graph = ResidualGraph::from_assembly(&assembly);

        let evaluation =
            source_residual_evaluation_for_unknowns(&assembly, &graph, &[2.0], 1e-9).unwrap();

        assert_eq!(evaluation.residuals.len(), 1);
        assert_eq!(evaluation.residuals[0].value, 0.0);
        assert_eq!(evaluation.normalized_values[0], 0.0);
    }
    fn square_linear_test_assembly(name: &str) -> EquationAssembly {
        let x = UnknownVariable {
            name: "x".to_owned(),
            role: "algebraic".to_owned(),
            quantity_kind: "Dimensionless".to_owned(),
            unit: "1".to_owned(),
            source: "Test.x".to_owned(),
            status: "unknown".to_owned(),
            value: None,
        };
        let y = UnknownVariable {
            name: "y".to_owned(),
            role: "algebraic".to_owned(),
            quantity_kind: "Dimensionless".to_owned(),
            unit: "1".to_owned(),
            source: "Test.y".to_owned(),
            status: "unknown".to_owned(),
            value: None,
        };
        EquationAssembly {
            name: name.to_owned(),
            generated_equations: vec![
                GeneratedEquation {
                    name: "r1".to_owned(),
                    kind: "across_equality".to_owned(),
                    domain: "Test".to_owned(),
                    expression: "x eq y".to_owned(),
                    residual: "x - y".to_owned(),
                    rhs_value: None,
                    dependencies: vec!["x".to_owned(), "y".to_owned()],
                    source: "test".to_owned(),
                    reason: "test linear equality".to_owned(),
                    source_line: Some(1),
                    status: "generated".to_owned(),
                },
                GeneratedEquation {
                    name: "r2".to_owned(),
                    kind: "through_conservation".to_owned(),
                    domain: "Test".to_owned(),
                    expression: "sum(x, y) eq 0".to_owned(),
                    residual: "x + y".to_owned(),
                    rhs_value: None,
                    dependencies: vec!["x".to_owned(), "y".to_owned()],
                    source: "test".to_owned(),
                    reason: "test linear conservation".to_owned(),
                    source_line: Some(2),
                    status: "generated".to_owned(),
                },
            ],
            unknowns: vec![x.clone(), y.clone()],
            algebraic_variables: vec![x, y],
            ..EquationAssembly::default()
        }
    }

    fn dynamic_component_test_assembly(name: &str) -> EquationAssembly {
        let x = component_test_variable("x", "state");
        let z = component_test_variable("z", "algebraic");
        let u = component_test_variable("u", "input");
        let k = component_test_variable("k", "parameter");
        EquationAssembly {
            name: name.to_owned(),
            generated_equations: vec![
                GeneratedEquation {
                    name: "x_rhs".to_owned(),
                    kind: "dynamic_rhs".to_owned(),
                    domain: "Test".to_owned(),
                    expression: "der(x) eq z".to_owned(),
                    residual: "der_x - z".to_owned(),
                    rhs_value: None,
                    dependencies: vec!["der_x".to_owned(), "z".to_owned()],
                    source: "test".to_owned(),
                    reason: "test dynamic component derivative residual".to_owned(),
                    source_line: Some(1),
                    status: "generated".to_owned(),
                },
                GeneratedEquation {
                    name: "z_balance".to_owned(),
                    kind: "dynamic_algebraic".to_owned(),
                    domain: "Test".to_owned(),
                    expression: "z + x + k eq u".to_owned(),
                    residual: "z + x + k - u".to_owned(),
                    rhs_value: None,
                    dependencies: vec![
                        "z".to_owned(),
                        "x".to_owned(),
                        "k".to_owned(),
                        "u".to_owned(),
                    ],
                    source: "test".to_owned(),
                    reason: "test dynamic component algebraic residual".to_owned(),
                    source_line: Some(2),
                    status: "generated".to_owned(),
                },
            ],
            unknowns: vec![x.clone(), z.clone()],
            states: vec![x],
            algebraic_variables: vec![z],
            inputs: vec![u],
            parameters: vec![k],
            ..EquationAssembly::default()
        }
    }

    fn component_test_variable(name: &str, role: &str) -> UnknownVariable {
        UnknownVariable {
            name: name.to_owned(),
            role: role.to_owned(),
            quantity_kind: "Dimensionless".to_owned(),
            unit: "1".to_owned(),
            source: format!("Test.{name}"),
            status: "classified".to_owned(),
            value: None,
        }
    }

    fn check_source_with_runtime_component_graph() -> CheckReport {
        check_source(
            "ok.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent Source {\n    port out: Thermal\n}\n\ncomponent Sink {\n    port inlet: Thermal\n}\n\nconnect Source.out -> Sink.inlet\n",
            &CheckOptions::default(),
        )
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
