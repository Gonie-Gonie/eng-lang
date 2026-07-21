use eng_compiler::{
    extract_review_document, CheckReport, DomainTypeParameterInfo, ReviewDocumentError, Severity,
};
use eng_jit::{candidate_executor_status, plan_for_report};
use serde_json::Value;

pub const REPORT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REPORT_SPEC_VERSION: u32 = 1;
pub const PLOT_SPEC_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq)]
pub struct PlotSpec {
    pub title: String,
    pub plot_type: String,
    pub x_axis: PlotAxis,
    pub y_axis: PlotAxis,
    pub series: Vec<PlotSeries>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlotAxis {
    pub name: String,
    pub label: String,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlotSeries {
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub bins: Vec<PlotBin>,
    pub points: Vec<PlotPoint>,
    pub confidence_band: Option<PlotConfidenceBand>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlotConfidenceBand {
    pub method: String,
    pub source: String,
    pub level: f64,
    pub lower: Vec<PlotPoint>,
    pub upper: Vec<PlotPoint>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlotBin {
    pub lower: f64,
    pub upper: f64,
    pub center: f64,
    pub count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlotPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportSpec {
    pub source_path: String,
    pub source_hash: String,
    pub compiler_version: String,
    pub report_version: String,
    pub variables: Vec<ReportVariable>,
    pub inferred_declarations: Vec<ReportInferredDeclaration>,
    pub unit_conversions: Vec<ReportUnitConversion>,
    pub schemas: Vec<ReportSchemaSummary>,
    pub args: Vec<ReportArgsBlock>,
    pub arg_values: Vec<ReportArgValue>,
    pub computed_statistics: Vec<ReportComputedStatistics>,
    pub computed_integrations: Vec<ReportComputedIntegration>,
    pub computed_metrics: Vec<ReportComputedMetric>,
    pub validations: Vec<ReportValidationResult>,
    pub quality_report: ReportQualityReport,
    pub time_axes: Vec<ReportTimeAxis>,
    pub time_alignments: Vec<ReportTimeAlignment>,
    pub uncertainty: Vec<ReportUncertaintyInfo>,
    pub ml: Vec<ReportMlInfo>,
    pub policy_results: Vec<ReportPolicyResult>,
    pub domains: Vec<ReportDomainSummary>,
    pub components: Vec<ReportComponentSummary>,
    pub connections: Vec<ReportConnectionSummary>,
    pub assemblies: Vec<ReportAssemblySummary>,
    pub component_graph: ReportComponentGraph,
    pub classes: Vec<ReportClassSummary>,
    pub class_objects: Vec<ReportClassObjectSummary>,
    pub systems: Vec<ReportSystemSummary>,
    pub state_space_vectors: Vec<ReportStateSpaceVector>,
    pub linear_operators: Vec<ReportLinearOperator>,
    pub system_ir: Vec<ReportSystemIr>,
    pub kernel_plan: ReportKernelPlan,
    pub plot_manifest: ReportPlotManifest,
    pub warnings: Vec<ReportWarning>,
    pub provenance: ReportProvenance,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportQualityReport {
    pub status: String,
    pub total_count: usize,
    pub passed_count: usize,
    pub warning_count: usize,
    pub failed_count: usize,
    pub unavailable_count: usize,
    pub results: Vec<ReportQualityResult>,
}

impl Default for ReportQualityReport {
    fn default() -> Self {
        Self {
            status: "unavailable".to_owned(),
            total_count: 0,
            passed_count: 0,
            warning_count: 0,
            failed_count: 0,
            unavailable_count: 0,
            results: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportQualityResult {
    pub binding: String,
    pub kind: String,
    pub category: String,
    pub target: String,
    pub subject: String,
    pub score: Option<f64>,
    pub passed_count: usize,
    pub warning_count: usize,
    pub failed_count: usize,
    pub status: String,
    pub reason: String,
    pub failures: Vec<ReportQualityFailure>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportQualityFailure {
    pub row: usize,
    pub field: String,
    pub value: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportKernelPlan {
    pub format: String,
    pub backend: String,
    pub backend_selection: ReportKernelBackendSelection,
    pub candidate_count: usize,
    pub candidates: Vec<ReportKernelCandidate>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportKernelBackendSelection {
    pub requested: String,
    pub selected: String,
    pub status: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportKernelCandidate {
    pub name: String,
    pub kind: String,
    pub line: usize,
    pub source: String,
    pub reason: String,
    pub lowering_status: String,
    pub operations: Vec<String>,
    pub estimated_rows: Option<usize>,
    pub input_count: usize,
    pub output_count: usize,
    pub operation_count: usize,
    pub scan_count: usize,
    pub complexity: String,
    pub executor_backend: String,
    pub executor_status: String,
    pub fallback_reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportVariable {
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub dimension: String,
    pub source: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportInferredDeclaration {
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub expression: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportUnitConversion {
    pub name: String,
    pub quantity_kind: String,
    pub source_unit: Option<String>,
    pub display_unit: String,
    pub canonical_unit: String,
    pub steps: Vec<String>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportSchemaSummary {
    pub name: String,
    pub columns: Vec<String>,
    pub column_count: usize,
    pub constraint_count: usize,
    pub missing_policy_count: usize,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportArgsBlock {
    pub name: String,
    pub fields: Vec<ReportArgsField>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportArgsField {
    pub name: String,
    pub type_name: String,
    pub default_value: Option<String>,
    pub required: bool,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportArgValue {
    pub name: String,
    pub type_name: String,
    pub value: String,
    pub source: String,
    pub required: bool,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComputedStatistics {
    pub source: String,
    pub quantity_kind: String,
    pub axis: String,
    pub status: String,
    pub values: Vec<ReportComputedStatisticValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComputedStatisticValue {
    pub name: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComputedIntegration {
    pub binding: String,
    pub source: String,
    pub input_quantity: String,
    pub over_axis: String,
    pub result_quantity: String,
    pub value: f64,
    pub unit: String,
    pub method: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComputedMetric {
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
pub struct ReportValidationResult {
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
pub struct ReportTimeAxis {
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

#[derive(Clone, Debug, PartialEq)]
pub struct ReportTimeAlignment {
    pub binding: String,
    pub left: String,
    pub right: String,
    pub axis: String,
    pub strategy: String,
    pub method: String,
    pub resample_step: Option<f64>,
    pub tolerance: Option<f64>,
    pub left_count: usize,
    pub right_count: usize,
    pub matched_count: usize,
    pub target_count: usize,
    pub output_count: usize,
    pub materialization_status: String,
    pub materialization_reason: String,
    pub left_nominal_step: Option<f64>,
    pub right_nominal_step: Option<f64>,
    pub left_irregular: bool,
    pub right_irregular: bool,
    pub step_status: String,
    pub overlap_start: Option<f64>,
    pub overlap_end: Option<f64>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportUncertaintyInfo {
    pub binding: String,
    pub kind: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub expression: String,
    pub source: Option<String>,
    pub distribution: Option<String>,
    pub method: Option<String>,
    pub scale: Option<String>,
    pub offset: Option<String>,
    pub mean: Option<String>,
    pub stddev: Option<String>,
    pub error: Option<String>,
    pub lower: Option<String>,
    pub upper: Option<String>,
    pub p05: Option<String>,
    pub p50: Option<String>,
    pub p95: Option<String>,
    pub sample_count: usize,
    pub propagation_count: usize,
    pub propagation: Vec<ReportUncertaintyPropagationTerm>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportUncertaintyPropagationTerm {
    pub source: String,
    pub role: String,
    pub quantity_kind: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportMlCoefficient {
    pub feature: String,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportMlInfo {
    pub binding: String,
    pub kind: String,
    pub source: Option<String>,
    pub target: Option<String>,
    pub target_quantity: Option<String>,
    pub target_unit: String,
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
    pub coefficients: Vec<ReportMlCoefficient>,
    pub intercept: Option<f64>,
    pub loss_history: Vec<f64>,
    pub training_data_hash: Option<String>,
    pub model_artifact_hash: Option<String>,
    pub model_card: Option<String>,
    pub expression: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportPolicyResult {
    pub schema: String,
    pub binding: String,
    pub kind: String,
    pub target: String,
    pub policy: String,
    pub status: String,
    pub checked_rows: usize,
    pub violation_count: usize,
    pub violations: Vec<ReportPolicyViolation>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportPolicyViolation {
    pub row: usize,
    pub column: String,
    pub value: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportDomainSummary {
    pub name: String,
    pub type_parameters: Vec<ReportDomainTypeParameter>,
    pub package: Option<String>,
    pub version: Option<String>,
    pub variables: Vec<ReportDomainVariable>,
    pub conservations: Vec<ReportDomainConservation>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportDomainTypeParameter {
    pub kind: String,
    pub name: String,
    pub display: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportDomainVariable {
    pub role: String,
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub dimension: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportDomainConservation {
    pub text: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComponentSummary {
    pub name: String,
    pub template_name: Option<String>,
    pub constructor_arguments: Vec<ReportComponentConstructorArgument>,
    pub parameters: Vec<ReportComponentParameter>,
    pub ports: Vec<ReportPort>,
    pub local_expressions: Vec<ReportComponentLocalExpression>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComponentConstructorArgument {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComponentParameter {
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub default_value: Option<String>,
    pub value: Option<String>,
    pub source: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComponentLocalExpression {
    pub name: String,
    pub expression: String,
    pub status: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub type_status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportPort {
    pub name: String,
    pub domain: String,
    pub domain_name: String,
    pub type_arguments: Vec<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportConnectionSummary {
    pub left: String,
    pub right: String,
    pub domain: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComponentGraph {
    pub format: String,
    pub status: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub components: Vec<ReportComponentGraphComponent>,
    pub ports: Vec<ReportComponentGraphPort>,
    pub connections: Vec<ReportComponentGraphConnection>,
    pub connection_sets: Vec<ReportComponentGraphConnectionSet>,
    pub behavior_nodes: Vec<ReportComponentGraphBehaviorNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComponentGraphComponent {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub port_count: usize,
    pub ports: Vec<String>,
    pub line: usize,
    pub source_span: ReportSourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComponentGraphPort {
    pub id: String,
    pub kind: String,
    pub component: String,
    pub name: String,
    pub domain_label: String,
    pub domain_name: String,
    pub type_arguments: Vec<String>,
    pub medium_label: Option<String>,
    pub frame_label: Option<String>,
    pub axis_label: Option<String>,
    pub status: String,
    pub line: usize,
    pub source_span: ReportSourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComponentGraphConnection {
    pub id: String,
    pub kind: String,
    pub left: String,
    pub right: String,
    pub left_component: String,
    pub left_port: String,
    pub right_component: String,
    pub right_port: String,
    pub domain_label: String,
    pub medium_label: Option<String>,
    pub frame_label: Option<String>,
    pub axis_label: Option<String>,
    pub status: String,
    pub line: usize,
    pub source_span: ReportSourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComponentGraphConnectionSet {
    pub assembly: String,
    pub name: String,
    pub domain_label: String,
    pub status: String,
    pub connection_count: usize,
    pub ports: Vec<String>,
    pub line: usize,
    pub source_span: ReportSourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComponentGraphBehaviorNode {
    pub id: String,
    pub kind: String,
    pub behavior_kind: String,
    pub component: String,
    pub name: String,
    pub expression: String,
    pub status: String,
    pub signal: Option<String>,
    pub delay_s: Option<f64>,
    pub relationship_status: Option<String>,
    pub contract_status: Option<String>,
    pub jacobian_policy: Option<String>,
    pub profile_policy: Option<String>,
    pub contract_inputs: Vec<ReportBehaviorSignalContract>,
    pub contract_outputs: Vec<ReportBehaviorSignalContract>,
    pub diagnostic_channels: Vec<String>,
    pub runtime_warning_status: Option<String>,
    pub line: usize,
    pub source_span: ReportSourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportBehaviorSignalContract {
    pub role: String,
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportSourceSpan {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportAssemblySummary {
    pub name: String,
    pub status: String,
    pub source_span: Option<ReportSourceSpan>,
    pub component_count: usize,
    pub port_count: usize,
    pub connection_count: usize,
    pub component_equation_count: usize,
    pub local_expression_count: usize,
    pub operator_call_count: usize,
    pub predictor_call_count: usize,
    pub domain_count: usize,
    pub domain_plans: Vec<ReportDomainPlan>,
    pub solver_preview: ReportComponentSolverPreview,
    pub connection_sets: Vec<ReportConnectionSet>,
    pub equations: Vec<ReportAssemblyEquation>,
    pub variables: Vec<ReportAssemblyVariable>,
    pub boundary: ReportAssemblyBoundary,
    pub residual_graph: ReportResidualGraph,
    pub solver_result: Option<ReportComponentSolverResult>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComponentSolverResult {
    pub status: String,
    pub method: String,
    pub reason: String,
    pub residual_norm: f64,
    pub linear_condition_estimate: Option<f64>,
    pub linear_minimum_pivot_abs: Option<f64>,
    pub linear_maximum_pivot_abs: Option<f64>,
    pub variable_scale_policy: String,
    pub variable_scale_min: Option<f64>,
    pub variable_scale_max: Option<f64>,
    pub tolerance: f64,
    pub max_iterations: usize,
    pub iteration_count: usize,
    pub convergence_status: String,
    pub variables: Vec<ReportComponentSolverVariable>,
    pub trajectories: Vec<ReportComponentSolverTrajectory>,
    pub step_diagnostics: Vec<ReportComponentSolverStepDiagnostic>,
    pub residuals: Vec<ReportComponentSolverResidual>,
    pub largest_residuals: Vec<ReportComponentSolverResidual>,
    pub failure_artifact: Option<ReportSolverFailureArtifact>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComponentSolverVariable {
    pub name: String,
    pub role: String,
    pub value: f64,
    pub unit: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComponentSolverTrajectory {
    pub name: String,
    pub role: String,
    pub quantity_kind: String,
    pub unit: String,
    pub initial_value: f64,
    pub final_value: f64,
    pub point_count: usize,
    pub points: Vec<ReportSystemSolutionPoint>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComponentSolverStepDiagnostic {
    pub step_index: usize,
    pub time_s: f64,
    pub algebraic_iteration_count: usize,
    pub residual_norm: f64,
    pub residual_values: Vec<f64>,
    pub normalized_residual_values: Vec<f64>,
    pub line_search_scale: Option<f64>,
    pub line_search_trial_count: Option<usize>,
    pub jacobian_policy: Option<String>,
    pub variable_scale_policy: Option<String>,
    pub linear_condition_estimate: Option<f64>,
    pub linear_minimum_pivot_abs: Option<f64>,
    pub linear_maximum_pivot_abs: Option<f64>,
    pub largest_residual_index: Option<usize>,
    pub largest_residual_name: Option<String>,
    pub largest_residual_source_expression: Option<String>,
    pub largest_residual_source_line: Option<usize>,
    pub largest_residual_source_reason: Option<String>,
    pub largest_residual_value: Option<f64>,
    pub largest_residual_abs_value: Option<f64>,
    pub convergence_status: String,
    pub failure_artifact: Option<ReportSolverFailureArtifact>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComponentSolverResidual {
    pub name: String,
    pub expression: String,
    pub source_expression: String,
    pub source_line: Option<usize>,
    pub source_reason: Option<String>,
    pub dependencies: Vec<String>,
    pub value: f64,
    pub unit: String,
    pub expression_unit: String,
    pub expression_quantity_kind: String,
    pub normalized_value: f64,
    pub scale: f64,
    pub scale_policy: String,
    pub lowering_status: String,
    pub lowering_failure_code: Option<String>,
    pub lowering_failure_reason: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportSolverFailureArtifact {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportDomainPlan {
    pub domain: String,
    pub connection_set_count: usize,
    pub equation_count: usize,
    pub variable_count: usize,
    pub conservation_status: String,
    pub solver_role: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComponentSolverPreview {
    pub status: String,
    pub method: String,
    pub mixed_algebraic_dynamic: String,
    pub nonlinear_residual: String,
    pub dae_split: String,
    pub delay_history: String,
    pub predictor: String,
    pub external_adapter: String,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportConnectionSet {
    pub name: String,
    pub domain: String,
    pub ports: Vec<String>,
    pub connection_count: usize,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportAssemblyEquation {
    pub name: String,
    pub kind: String,
    pub domain: String,
    pub expression: String,
    pub residual: String,
    pub rhs: Option<String>,
    pub reason: String,
    pub dependencies: Vec<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportAssemblyVariable {
    pub name: String,
    pub role: String,
    pub domain: String,
    pub source: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportAssemblyBoundary {
    pub state_count: usize,
    pub algebraic_count: usize,
    pub input_count: usize,
    pub output_count: usize,
    pub parameter_count: usize,
    pub equation_count: usize,
    pub unknown_count: usize,
    pub balance_status: String,
    pub diagnostic_code: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportResidualGraph {
    pub name: String,
    pub status: String,
    pub residuals: Vec<String>,
    pub residual_metadata: Vec<ReportResidualGraphResidual>,
    pub dependencies: Vec<ReportResidualDependency>,
    pub algebraic_loops: Vec<Vec<String>>,
    pub jacobian_sparsity: Vec<ReportAssemblyJacobianSeed>,
    pub solver_plan: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportResidualGraphResidual {
    pub name: String,
    pub kind: String,
    pub domain: String,
    pub source_expression: String,
    pub residual_expression: String,
    pub rhs: Option<String>,
    pub dependencies: Vec<String>,
    pub unit: String,
    pub expression_unit: String,
    pub expression_quantity_kind: String,
    pub scale_policy: String,
    pub lowering_status: String,
    pub lowering_failure_code: Option<String>,
    pub lowering_failure_reason: Option<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportResidualDependency {
    pub residual: String,
    pub variable: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportAssemblyJacobianSeed {
    pub residual: String,
    pub with_respect_to: Vec<String>,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportClassSummary {
    pub name: String,
    pub fields: Vec<ReportClassField>,
    pub validations: Vec<ReportClassValidation>,
    pub methods: Vec<ReportClassMethod>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportClassField {
    pub name: String,
    pub type_name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub dimension: String,
    pub default_value: Option<String>,
    pub required: bool,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportClassValidation {
    pub expression: String,
    pub left: String,
    pub operator: String,
    pub right: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportClassMethod {
    pub name: String,
    pub return_type: String,
    pub return_quantity_kind: String,
    pub return_display_unit: String,
    pub return_canonical_unit: String,
    pub expression: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportClassObjectSummary {
    pub name: String,
    pub class_name: String,
    pub source_object: Option<String>,
    pub construction: String,
    pub fields: Vec<ReportClassObjectField>,
    pub validations: Vec<ReportClassObjectValidation>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportClassObjectField {
    pub name: String,
    pub expression: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportClassObjectValidation {
    pub expression: String,
    pub left: String,
    pub operator: String,
    pub right: String,
    pub left_value: Option<String>,
    pub right_value: Option<String>,
    pub unit: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportSystemSummary {
    pub name: String,
    pub variables: Vec<ReportSystemVariable>,
    pub equations: Vec<ReportEquation>,
    pub residuals: Vec<ReportResidual>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportSystemVariable {
    pub role: String,
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub dimension: String,
    pub initial_value: Option<String>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportStateSpaceVector {
    pub system: String,
    pub role: String,
    pub name: String,
    pub vector_type: String,
    pub members: Vec<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportLinearOperator {
    pub system: String,
    pub name: String,
    pub from: String,
    pub to: String,
    pub expression: Option<String>,
    pub canonical_matrix: Option<Vec<Vec<f64>>>,
    pub canonical_entries: Vec<ReportLinearOperatorEntry>,
    pub row_count: usize,
    pub column_count: usize,
    pub row_members: Vec<String>,
    pub column_members: Vec<String>,
    pub row_quantity_kinds: Vec<String>,
    pub column_quantity_kinds: Vec<String>,
    pub row_units: Vec<String>,
    pub column_units: Vec<String>,
    pub compatibility_status: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportLinearOperatorEntry {
    pub row_index: usize,
    pub column_index: usize,
    pub row_member: String,
    pub column_member: String,
    pub coefficient: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportEquation {
    pub left: String,
    pub relation: String,
    pub right: String,
    pub left_dimension: String,
    pub right_dimension: String,
    pub residual: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportResidual {
    pub name: String,
    pub expression: String,
    pub dimension: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportSystemIr {
    pub name: String,
    pub solver_boundary: ReportSolverBoundary,
    pub solver_plan: ReportSolverPlan,
    pub solver_results: Vec<ReportSystemSolution>,
    pub equations: Vec<ReportEquationIr>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportSolverBoundary {
    pub status: String,
    pub reason: String,
    pub parameter_count: usize,
    pub state_count: usize,
    pub input_count: usize,
    pub equation_count: usize,
    pub residual_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportSolverPlan {
    pub status: String,
    pub method: String,
    pub solve_order: Vec<String>,
    pub ode_runner: ReportOdeRunner,
    pub jacobian_sparsity: Vec<ReportJacobianSparsity>,
    pub jacobian_seed: Vec<ReportJacobianSeed>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportOdeRunner {
    pub status: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportSystemEquationMetadata {
    pub kind: String,
    pub target: String,
    pub left: String,
    pub right: String,
    pub residual_expression: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub source_line: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportSystemSolution {
    pub binding: Option<String>,
    pub status: String,
    pub method: String,
    pub reason: String,
    pub states: Vec<String>,
    pub algebraic_variables: Vec<String>,
    pub inputs: Vec<String>,
    pub parameters: Vec<String>,
    pub outputs: Vec<String>,
    pub source_equations: Vec<ReportSystemEquationMetadata>,
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
    pub step_diagnostics: Vec<ReportSystemSolverStepDiagnostic>,
    pub points: Vec<ReportSystemSolutionPoint>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportSystemSolverStepDiagnostic {
    pub output_index: usize,
    pub start_time_s: f64,
    pub end_time_s: f64,
    pub dt_s: f64,
    pub error_norm: f64,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportSystemSolutionPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportJacobianSparsity {
    pub residual: String,
    pub with_respect_to: Vec<String>,
    pub derivative_states: Vec<String>,
    pub status: String,
}

pub type ReportJacobianSeed = ReportJacobianSparsity;

#[derive(Clone, Debug, PartialEq)]
pub struct ReportEquationIr {
    pub residual: String,
    pub relation: String,
    pub normalized_residual: String,
    pub dependencies: Vec<ReportEquationDependency>,
    pub derivative_states: Vec<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportEquationDependency {
    pub name: String,
    pub role: String,
    pub quantity_kind: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportPlotManifest {
    pub path: String,
    pub hash: String,
    pub format: String,
    pub plot_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportWarning {
    pub code: String,
    pub message: String,
    pub help: Option<String>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportEnvironmentDependency {
    pub name: String,
    pub kind: String,
    pub expression: String,
    pub resolved_value: String,
    pub source_hash: Option<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportProvenance {
    pub syntax_items: usize,
    pub schema_count: usize,
    pub csv_promotion_count: usize,
    pub domain_count: usize,
    pub component_count: usize,
    pub connection_count: usize,
    pub assembly_count: usize,
    pub class_count: usize,
    pub object_count: usize,
    pub system_count: usize,
    pub equation_count: usize,
    pub residual_count: usize,
    pub environment_dependencies: Vec<ReportEnvironmentDependency>,
    pub plot_spec_version: u32,
}

pub fn report_spec_from_report(
    report: &CheckReport,
    plot_manifest_relative_path: &str,
    plot_manifest_hash: &str,
) -> ReportSpec {
    let variables = report
        .semantic_program
        .typed_bindings
        .iter()
        .map(|binding| {
            let type_info = report
                .semantic_program
                .type_infos
                .iter()
                .find(|info| info.name == binding.name && info.line == binding.line);
            ReportVariable {
                name: binding.name.clone(),
                quantity_kind: binding.semantic_type.quantity_kind.clone(),
                display_unit: binding.semantic_type.display_unit.clone(),
                canonical_unit: type_info
                    .map(|info| info.canonical_unit.clone())
                    .unwrap_or_else(|| "unknown".to_owned()),
                dimension: type_info
                    .map(|info| info.dimension.clone())
                    .unwrap_or_else(|| "unknown".to_owned()),
                source: type_info
                    .map(|info| info.source.as_str().to_owned())
                    .unwrap_or_else(|| "runtime".to_owned()),
                line: binding.line,
            }
        })
        .collect();

    let inferred_declarations = report
        .inferred_declarations
        .iter()
        .map(|declaration| ReportInferredDeclaration {
            name: declaration.name.clone(),
            quantity_kind: declaration.quantity_kind.clone(),
            display_unit: declaration.display_unit.clone(),
            expression: declaration.expression.clone(),
            line: declaration.line,
        })
        .collect();

    let unit_conversions = report
        .semantic_program
        .unit_derivations
        .iter()
        .map(|derivation| ReportUnitConversion {
            name: derivation.name.clone(),
            quantity_kind: derivation.quantity_kind.clone(),
            source_unit: derivation.source_unit.clone(),
            display_unit: derivation.display_unit.clone(),
            canonical_unit: derivation.canonical_unit.clone(),
            steps: derivation.steps.clone(),
            line: derivation.line,
        })
        .collect();

    let schemas = report
        .semantic_program
        .schemas
        .iter()
        .map(|schema| ReportSchemaSummary {
            name: schema.name.clone(),
            columns: schema
                .columns
                .iter()
                .map(|column| column.name.clone())
                .collect(),
            column_count: schema.columns.len(),
            constraint_count: schema.constraints.len(),
            missing_policy_count: schema.missing_policies.len(),
            line: schema.line,
        })
        .collect();

    let args = report
        .semantic_program
        .args_blocks
        .iter()
        .map(|args_block| ReportArgsBlock {
            name: args_block.name.clone(),
            fields: args_block
                .fields
                .iter()
                .map(|field| ReportArgsField {
                    name: field.name.clone(),
                    type_name: field.type_name.clone(),
                    default_value: field.default_value.clone(),
                    required: field.required,
                    line: field.line,
                })
                .collect(),
            line: args_block.line,
        })
        .collect();
    let arg_values = report
        .semantic_program
        .arg_values
        .iter()
        .map(|arg| ReportArgValue {
            name: arg.name.clone(),
            type_name: arg.type_name.clone(),
            value: arg.value.clone(),
            source: arg.source.clone(),
            required: arg.required,
            line: arg.line,
        })
        .collect();
    let environment_dependencies = report
        .semantic_program
        .environment_dependencies
        .iter()
        .map(|dependency| ReportEnvironmentDependency {
            name: dependency.name.clone(),
            kind: dependency.kind.clone(),
            expression: dependency.expression.clone(),
            resolved_value: dependency.resolved_value.clone(),
            source_hash: dependency.source_hash.clone(),
            status: dependency.status.clone(),
            line: dependency.line,
        })
        .collect();

    let warnings = report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .map(|diagnostic| ReportWarning {
            code: diagnostic.code.clone(),
            message: diagnostic.message.clone(),
            help: diagnostic.help.clone(),
            line: diagnostic.line,
        })
        .collect();
    let uncertainty = report
        .semantic_program
        .uncertainty_infos
        .iter()
        .map(|info| ReportUncertaintyInfo {
            binding: info.binding.clone(),
            kind: info.kind.clone(),
            quantity_kind: info.quantity_kind.clone(),
            display_unit: info.display_unit.clone(),
            expression: info.expression.clone(),
            source: info.source.clone(),
            distribution: info.distribution.clone(),
            method: info.method.clone(),
            scale: info.scale.clone(),
            offset: info.offset.clone(),
            mean: info.mean.clone(),
            stddev: info.stddev.clone(),
            error: info.error.clone(),
            lower: info.lower.clone(),
            upper: info.upper.clone(),
            p05: None,
            p50: None,
            p95: None,
            sample_count: info.sample_count,
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
            line: info.line,
        })
        .collect();
    let ml = report
        .semantic_program
        .ml_infos
        .iter()
        .map(|info| ReportMlInfo {
            binding: info.binding.clone(),
            kind: info.kind.clone(),
            source: info.source.clone(),
            target: info.target.clone(),
            target_quantity: None,
            target_unit: "1".to_owned(),
            features: info.features.clone(),
            algorithm: info.algorithm.clone(),
            test_fraction: info.test_fraction.clone(),
            seed: info.seed.clone(),
            hidden_layers: info.hidden_layers.clone(),
            epochs: info.epochs,
            status: "metadata".to_owned(),
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
            training_data_hash: None,
            model_artifact_hash: None,
            model_card: None,
            expression: info.expression.clone(),
            line: info.line,
        })
        .collect();

    let domains = report
        .semantic_program
        .domains
        .iter()
        .map(|domain| ReportDomainSummary {
            name: domain.name.clone(),
            type_parameters: domain
                .type_parameters
                .iter()
                .map(|parameter| ReportDomainTypeParameter {
                    kind: parameter.kind.clone(),
                    name: parameter.name.clone(),
                    display: parameter.display.clone(),
                })
                .collect(),
            package: domain.package.clone(),
            version: domain.version.clone(),
            variables: domain
                .variables
                .iter()
                .map(|variable| ReportDomainVariable {
                    role: variable.role.clone(),
                    name: variable.name.clone(),
                    quantity_kind: variable.quantity_kind.clone(),
                    display_unit: variable.display_unit.clone(),
                    canonical_unit: variable.canonical_unit.clone(),
                    dimension: variable.dimension.clone(),
                    line: variable.line,
                })
                .collect(),
            conservations: domain
                .conservations
                .iter()
                .map(|conservation| ReportDomainConservation {
                    text: conservation.text.clone(),
                    status: conservation.status.clone(),
                    line: conservation.line,
                })
                .collect(),
            line: domain.line,
        })
        .collect::<Vec<_>>();
    let components = report
        .semantic_program
        .assembly_components()
        .iter()
        .map(|component| ReportComponentSummary {
            name: component.name.clone(),
            template_name: component.template_name.clone(),
            constructor_arguments: component
                .constructor_arguments
                .iter()
                .map(|argument| ReportComponentConstructorArgument {
                    name: argument.name.clone(),
                    value: argument.value.clone(),
                })
                .collect(),
            parameters: component
                .parameters
                .iter()
                .map(|parameter| ReportComponentParameter {
                    name: parameter.name.clone(),
                    quantity_kind: parameter.quantity_kind.clone(),
                    display_unit: parameter.display_unit.clone(),
                    canonical_unit: parameter.canonical_unit.clone(),
                    default_value: parameter.default_value.clone(),
                    value: parameter.value.clone(),
                    source: parameter.source.clone(),
                    status: parameter.status.clone(),
                    line: parameter.line,
                })
                .collect(),
            ports: component
                .ports
                .iter()
                .map(|port| ReportPort {
                    name: port.name.clone(),
                    domain: port.domain.clone(),
                    domain_name: port.domain_name.clone(),
                    type_arguments: port.type_arguments.clone(),
                    status: port.status.clone(),
                    line: port.line,
                })
                .collect(),
            local_expressions: component
                .local_expressions
                .iter()
                .map(|local| ReportComponentLocalExpression {
                    name: local.name.clone(),
                    expression: local.expression.clone(),
                    status: local.status.clone(),
                    quantity_kind: local.quantity_kind.clone(),
                    display_unit: local.display_unit.clone(),
                    canonical_unit: local.canonical_unit.clone(),
                    type_status: local.type_status.clone(),
                    line: local.line,
                })
                .collect(),
            line: component.line,
        })
        .collect::<Vec<_>>();
    let connections = report
        .semantic_program
        .connections
        .iter()
        .map(|connection| ReportConnectionSummary {
            left: connection.left.clone(),
            right: connection.right.clone(),
            domain: connection.domain.clone(),
            status: connection.status.clone(),
            line: connection.line,
        })
        .collect::<Vec<_>>();
    let assemblies = report
        .semantic_program
        .component_assemblies
        .iter()
        .map(|assembly| ReportAssemblySummary {
            name: assembly.name.clone(),
            status: assembly.status.clone(),
            source_span: Some(ReportSourceSpan {
                line: assembly.span.line,
                column: assembly.span.column,
            }),
            component_count: assembly.component_count,
            port_count: assembly.port_count,
            connection_count: assembly.connection_count,
            component_equation_count: assembly.component_equation_count,
            local_expression_count: assembly.local_expression_count,
            operator_call_count: assembly.operator_call_count,
            predictor_call_count: assembly.predictor_call_count,
            domain_count: assembly.domain_count,
            domain_plans: assembly
                .domain_plans
                .iter()
                .map(|plan| ReportDomainPlan {
                    domain: plan.domain.clone(),
                    connection_set_count: plan.connection_set_count,
                    equation_count: plan.equation_count,
                    variable_count: plan.variable_count,
                    conservation_status: plan.conservation_status.clone(),
                    solver_role: plan.solver_role.clone(),
                })
                .collect(),
            solver_preview: ReportComponentSolverPreview {
                status: assembly.solver_preview.status.clone(),
                method: assembly.solver_preview.method.clone(),
                mixed_algebraic_dynamic: assembly.solver_preview.mixed_algebraic_dynamic.clone(),
                nonlinear_residual: assembly.solver_preview.nonlinear_residual.clone(),
                dae_split: assembly.solver_preview.dae_split.clone(),
                delay_history: assembly.solver_preview.delay_history.clone(),
                predictor: assembly.solver_preview.predictor.clone(),
                external_adapter: assembly.solver_preview.external_adapter.clone(),
                limitations: assembly.solver_preview.limitations.clone(),
            },
            connection_sets: assembly
                .connection_sets
                .iter()
                .map(|connection_set| ReportConnectionSet {
                    name: connection_set.name.clone(),
                    domain: connection_set.domain.clone(),
                    ports: connection_set.ports.clone(),
                    connection_count: connection_set.connection_count,
                    status: connection_set.status.clone(),
                    line: connection_set.line,
                })
                .collect(),
            equations: assembly
                .equations
                .iter()
                .map(|equation| ReportAssemblyEquation {
                    name: equation.name.clone(),
                    kind: equation.kind.clone(),
                    domain: equation.domain.clone(),
                    expression: equation.expression.clone(),
                    residual: equation.residual.clone(),
                    rhs: equation.rhs.clone(),
                    reason: equation.reason.clone(),
                    dependencies: equation.dependencies.clone(),
                    status: equation.status.clone(),
                    line: equation.line,
                })
                .collect(),
            variables: assembly
                .variables
                .iter()
                .map(|variable| ReportAssemblyVariable {
                    name: variable.name.clone(),
                    role: variable.role.clone(),
                    domain: variable.domain.clone(),
                    source: variable.source.clone(),
                    status: variable.status.clone(),
                })
                .collect(),
            boundary: ReportAssemblyBoundary {
                state_count: assembly.boundary.state_count,
                algebraic_count: assembly.boundary.algebraic_count,
                input_count: assembly.boundary.input_count,
                output_count: assembly.boundary.output_count,
                parameter_count: assembly.boundary.parameter_count,
                equation_count: assembly.boundary.equation_count,
                unknown_count: assembly.boundary.unknown_count,
                balance_status: assembly.boundary.balance_status.clone(),
                diagnostic_code: assembly.boundary.diagnostic_code.clone(),
            },
            residual_graph: ReportResidualGraph {
                name: assembly.residual_graph.name.clone(),
                status: assembly.residual_graph.status.clone(),
                residuals: assembly.residual_graph.residuals.clone(),
                residual_metadata: assembly
                    .residual_graph
                    .residual_metadata
                    .iter()
                    .map(|metadata| ReportResidualGraphResidual {
                        name: metadata.name.clone(),
                        kind: metadata.kind.clone(),
                        domain: metadata.domain.clone(),
                        source_expression: metadata.source_expression.clone(),
                        residual_expression: metadata.residual_expression.clone(),
                        rhs: metadata.rhs.clone(),
                        dependencies: metadata.dependencies.clone(),
                        unit: String::new(),
                        expression_unit: String::new(),
                        expression_quantity_kind: String::new(),
                        scale_policy: String::new(),
                        lowering_status: String::new(),
                        lowering_failure_code: None,
                        lowering_failure_reason: None,
                        status: metadata.status.clone(),
                        line: metadata.line,
                    })
                    .collect(),
                dependencies: assembly
                    .residual_graph
                    .dependencies
                    .iter()
                    .map(|dependency| ReportResidualDependency {
                        residual: dependency.residual.clone(),
                        variable: dependency.variable.clone(),
                    })
                    .collect(),
                algebraic_loops: assembly.residual_graph.algebraic_loops.clone(),
                jacobian_sparsity: assembly
                    .residual_graph
                    .jacobian_sparsity
                    .iter()
                    .map(|seed| ReportAssemblyJacobianSeed {
                        residual: seed.residual.clone(),
                        with_respect_to: seed.with_respect_to.clone(),
                        status: seed.status.clone(),
                    })
                    .collect(),
                solver_plan: assembly.residual_graph.solver_plan.clone(),
            },
            solver_result: None,
            line: assembly.line,
        })
        .collect::<Vec<_>>();
    let component_graph = report_component_graph(report);
    let classes = report
        .semantic_program
        .classes
        .iter()
        .map(|class_info| ReportClassSummary {
            name: class_info.name.clone(),
            fields: class_info
                .fields
                .iter()
                .map(|field| ReportClassField {
                    name: field.name.clone(),
                    type_name: field.type_name.clone(),
                    quantity_kind: field.quantity_kind.clone(),
                    display_unit: field.display_unit.clone(),
                    canonical_unit: field.canonical_unit.clone(),
                    dimension: field.dimension.clone(),
                    default_value: field.default_value.clone(),
                    required: field.required,
                    status: field.status.clone(),
                    line: field.line,
                })
                .collect(),
            validations: class_info
                .validations
                .iter()
                .map(|validation| ReportClassValidation {
                    expression: validation.expression.clone(),
                    left: validation.left.clone(),
                    operator: validation.operator.clone(),
                    right: validation.right.clone(),
                    status: validation.status.clone(),
                    line: validation.line,
                })
                .collect(),
            methods: class_info
                .methods
                .iter()
                .map(|method| ReportClassMethod {
                    name: method.name.clone(),
                    return_type: method.return_type.clone(),
                    return_quantity_kind: method.return_quantity_kind.clone(),
                    return_display_unit: method.return_display_unit.clone(),
                    return_canonical_unit: method.return_canonical_unit.clone(),
                    expression: method.expression.clone(),
                    status: method.status.clone(),
                    line: method.line,
                })
                .collect(),
            status: class_info.status.clone(),
            line: class_info.line,
        })
        .collect::<Vec<_>>();
    let class_objects = report
        .semantic_program
        .class_objects
        .iter()
        .map(|object| ReportClassObjectSummary {
            name: object.name.clone(),
            class_name: object.class_name.clone(),
            source_object: object.source_object.clone(),
            construction: object.construction.clone(),
            fields: object
                .fields
                .iter()
                .map(|field| ReportClassObjectField {
                    name: field.name.clone(),
                    expression: field.expression.clone(),
                    quantity_kind: field.quantity_kind.clone(),
                    display_unit: field.display_unit.clone(),
                    status: field.status.clone(),
                    line: field.line,
                })
                .collect(),
            validations: object
                .validations
                .iter()
                .map(|validation| ReportClassObjectValidation {
                    expression: validation.expression.clone(),
                    left: validation.left.clone(),
                    operator: validation.operator.clone(),
                    right: validation.right.clone(),
                    left_value: validation.left_value.clone(),
                    right_value: validation.right_value.clone(),
                    unit: validation.unit.clone(),
                    status: validation.status.clone(),
                    line: validation.line,
                })
                .collect(),
            status: object.status.clone(),
            line: object.line,
        })
        .collect::<Vec<_>>();

    let systems = report
        .semantic_program
        .systems
        .iter()
        .map(|system| ReportSystemSummary {
            name: system.name.clone(),
            variables: system
                .variables
                .iter()
                .map(|variable| ReportSystemVariable {
                    role: variable.role.clone(),
                    name: variable.name.clone(),
                    quantity_kind: variable.quantity_kind.clone(),
                    display_unit: variable.display_unit.clone(),
                    dimension: variable.dimension.clone(),
                    initial_value: variable.initial_value.clone(),
                    line: variable.line,
                })
                .collect(),
            equations: system
                .equations
                .iter()
                .map(|equation| ReportEquation {
                    left: equation.left.clone(),
                    relation: equation.relation.clone(),
                    right: equation.right.clone(),
                    left_dimension: equation.left_dimension.clone(),
                    right_dimension: equation.right_dimension.clone(),
                    residual: equation.residual.clone(),
                    status: equation.status.clone(),
                    line: equation.line,
                })
                .collect(),
            residuals: system
                .residuals
                .iter()
                .map(|residual| ReportResidual {
                    name: residual.name.clone(),
                    expression: residual.expression.clone(),
                    dimension: residual.dimension.clone(),
                    line: residual.line,
                })
                .collect(),
            line: system.line,
        })
        .collect::<Vec<_>>();
    let equation_count = systems
        .iter()
        .map(|system| system.equations.len())
        .sum::<usize>();
    let residual_count = systems
        .iter()
        .map(|system| system.residuals.len())
        .sum::<usize>();
    let system_ir = report
        .semantic_program
        .systems
        .iter()
        .map(|system| ReportSystemIr {
            name: system.name.clone(),
            solver_boundary: ReportSolverBoundary {
                status: "unsolved".to_owned(),
                reason: "numeric solver deferred until the solver milestone".to_owned(),
                parameter_count: system
                    .variables
                    .iter()
                    .filter(|variable| variable.role == "parameter")
                    .count(),
                state_count: system
                    .variables
                    .iter()
                    .filter(|variable| variable.role == "state")
                    .count(),
                input_count: system
                    .variables
                    .iter()
                    .filter(|variable| variable.role == "input")
                    .count(),
                equation_count: system.equations.len(),
                residual_count: system.residuals.len(),
            },
            solver_plan: ReportSolverPlan {
                status: system.solver_plan.status.clone(),
                method: system.solver_plan.method.clone(),
                solve_order: system.solver_plan.solve_order.clone(),
                ode_runner: ReportOdeRunner {
                    status: system.solver_plan.ode_runner.status.clone(),
                    reason: system.solver_plan.ode_runner.reason.clone(),
                },
                jacobian_sparsity: system
                    .solver_plan
                    .jacobian_sparsity
                    .iter()
                    .map(|entry| ReportJacobianSparsity {
                        residual: entry.residual.clone(),
                        with_respect_to: entry.with_respect_to.clone(),
                        derivative_states: entry.derivative_states.clone(),
                        status: entry.status.clone(),
                    })
                    .collect(),
                jacobian_seed: system
                    .solver_plan
                    .jacobian_seed
                    .iter()
                    .map(|seed| ReportJacobianSeed {
                        residual: seed.residual.clone(),
                        with_respect_to: seed.with_respect_to.clone(),
                        derivative_states: seed.derivative_states.clone(),
                        status: seed.status.clone(),
                    })
                    .collect(),
            },
            solver_results: Vec::new(),
            equations: system
                .equation_ir
                .iter()
                .map(|equation| ReportEquationIr {
                    residual: equation.residual.clone(),
                    relation: equation.relation.clone(),
                    normalized_residual: equation.normalized_residual.clone(),
                    dependencies: equation
                        .dependencies
                        .iter()
                        .map(|dependency| ReportEquationDependency {
                            name: dependency.name.clone(),
                            role: dependency.role.clone(),
                            quantity_kind: dependency.quantity_kind.clone(),
                        })
                        .collect(),
                    derivative_states: equation.derivative_states.clone(),
                    status: equation.status.clone(),
                    line: equation.line,
                })
                .collect(),
            line: system.line,
        })
        .collect();
    let state_space_vectors = report
        .semantic_program
        .state_space_vectors
        .iter()
        .map(|vector| ReportStateSpaceVector {
            system: vector.system.clone(),
            role: vector.role.clone(),
            name: vector.name.clone(),
            vector_type: vector.vector_type.clone(),
            members: vector.members.clone(),
            status: vector.status.clone(),
            line: vector.line,
        })
        .collect();
    let linear_operators = report
        .semantic_program
        .linear_operators
        .iter()
        .map(|operator| ReportLinearOperator {
            system: operator.system.clone(),
            name: operator.name.clone(),
            from: operator.from.clone(),
            to: operator.to.clone(),
            expression: operator.expression.clone(),
            canonical_matrix: operator.canonical_matrix.clone(),
            canonical_entries: operator
                .canonical_entries
                .iter()
                .map(|entry| ReportLinearOperatorEntry {
                    row_index: entry.row_index,
                    column_index: entry.column_index,
                    row_member: entry.row_member.clone(),
                    column_member: entry.column_member.clone(),
                    coefficient: entry.coefficient,
                })
                .collect(),
            row_count: operator.row_count,
            column_count: operator.column_count,
            row_members: operator.row_members.clone(),
            column_members: operator.column_members.clone(),
            row_quantity_kinds: operator.row_quantity_kinds.clone(),
            column_quantity_kinds: operator.column_quantity_kinds.clone(),
            row_units: operator.row_units.clone(),
            column_units: operator.column_units.clone(),
            compatibility_status: operator.compatibility_status.clone(),
            status: operator.status.clone(),
            line: operator.line,
        })
        .collect();
    let kernel_plan = report_kernel_plan(report);

    ReportSpec {
        source_path: report.source_path.display().to_string(),
        source_hash: report.source_hash.clone(),
        compiler_version: eng_compiler::COMPILER_VERSION.to_owned(),
        report_version: REPORT_VERSION.to_owned(),
        variables,
        inferred_declarations,
        unit_conversions,
        schemas,
        args,
        arg_values,
        computed_statistics: Vec::new(),
        computed_integrations: Vec::new(),
        computed_metrics: Vec::new(),
        validations: Vec::new(),
        quality_report: ReportQualityReport::default(),
        time_axes: Vec::new(),
        time_alignments: Vec::new(),
        uncertainty,
        ml,
        policy_results: Vec::new(),
        domains,
        components,
        connections,
        assemblies,
        component_graph,
        classes,
        class_objects,
        systems,
        state_space_vectors,
        linear_operators,
        system_ir,
        kernel_plan,
        plot_manifest: ReportPlotManifest {
            path: plot_manifest_relative_path.to_owned(),
            hash: plot_manifest_hash.to_owned(),
            format: "eng-plot-manifest-v1".to_owned(),
            plot_count: 1,
        },
        warnings,
        provenance: ReportProvenance {
            syntax_items: report.syntax_summary.ast_items,
            schema_count: report.semantic_program.schemas.len(),
            csv_promotion_count: report.semantic_program.csv_promotions.len(),
            domain_count: report.semantic_program.domains.len(),
            component_count: report.semantic_program.assembly_components().len(),
            connection_count: report.semantic_program.connections.len(),
            assembly_count: report.semantic_program.component_assemblies.len(),
            class_count: report.semantic_program.classes.len(),
            object_count: report.semantic_program.class_objects.len(),
            system_count: report.semantic_program.systems.len(),
            equation_count,
            residual_count,
            environment_dependencies,
            plot_spec_version: PLOT_SPEC_VERSION,
        },
    }
}

fn report_kernel_plan(report: &CheckReport) -> ReportKernelPlan {
    let plan = plan_for_report(report);
    let candidates = plan
        .candidates
        .iter()
        .map(|candidate| {
            let (executor_status, fallback_reason) = candidate_executor_status(candidate);
            ReportKernelCandidate {
                name: candidate.name.clone(),
                kind: candidate.kind.clone(),
                line: candidate.line,
                source: candidate.source.clone(),
                reason: candidate.reason.clone(),
                lowering_status: candidate.lowering_status.clone(),
                operations: candidate.operations.clone(),
                estimated_rows: candidate.estimate.estimated_rows,
                input_count: candidate.estimate.input_count,
                output_count: candidate.estimate.output_count,
                operation_count: candidate.estimate.operation_count,
                scan_count: candidate.estimate.scan_count,
                complexity: candidate.estimate.complexity.clone(),
                executor_backend: eng_jit::INTERPRETER_FALLBACK_BACKEND.to_owned(),
                executor_status: executor_status.to_owned(),
                fallback_reason: fallback_reason.to_owned(),
            }
        })
        .collect::<Vec<_>>();
    ReportKernelPlan {
        format: plan.format,
        backend: plan.backend,
        backend_selection: ReportKernelBackendSelection {
            requested: plan.backend_selection.requested,
            selected: plan.backend_selection.selected,
            status: plan.backend_selection.status,
            reason: plan.backend_selection.reason,
        },
        candidate_count: candidates.len(),
        candidates,
    }
}

fn report_component_graph(report: &CheckReport) -> ReportComponentGraph {
    let program = &report.semantic_program;
    let assembly_components = program.assembly_components();
    let port_count = assembly_components
        .iter()
        .map(|component| component.ports.len())
        .sum::<usize>();
    let status = if assembly_components.is_empty() {
        "empty"
    } else if program
        .connections
        .iter()
        .any(|connection| connection.status != "domain_compatible")
    {
        "diagnostics_present"
    } else {
        "metadata_ready"
    };
    let components = assembly_components
        .iter()
        .map(|component| ReportComponentGraphComponent {
            id: component.name.clone(),
            kind: "component".to_owned(),
            name: component.name.clone(),
            port_count: component.ports.len(),
            ports: component
                .ports
                .iter()
                .map(|port| format!("{}.{}", component.name, port.name))
                .collect(),
            line: component.line,
            source_span: report_source_span(component.line),
        })
        .collect::<Vec<_>>();
    let ports = assembly_components
        .iter()
        .flat_map(|component| {
            component.ports.iter().map(move |port| {
                let (medium_label, frame_label, axis_label) = report_domain_argument_labels(
                    &program.domains,
                    &port.domain_name,
                    &port.type_arguments,
                );
                ReportComponentGraphPort {
                    id: format!("{}.{}", component.name, port.name),
                    kind: "port".to_owned(),
                    component: component.name.clone(),
                    name: port.name.clone(),
                    domain_label: port.domain.clone(),
                    domain_name: port.domain_name.clone(),
                    type_arguments: port.type_arguments.clone(),
                    medium_label,
                    frame_label,
                    axis_label,
                    status: port.status.clone(),
                    line: port.line,
                    source_span: report_source_span(port.line),
                }
            })
        })
        .collect::<Vec<_>>();
    let mut port_lookup = std::collections::HashMap::new();
    for component in assembly_components {
        for port in &component.ports {
            port_lookup.insert(format!("{}.{}", component.name, port.name), port);
        }
    }
    let connections = program
        .connections
        .iter()
        .map(|connection| {
            let (medium_label, frame_label, axis_label) = port_lookup
                .get(&connection.left)
                .or_else(|| port_lookup.get(&connection.right))
                .map(|port| {
                    report_domain_argument_labels(
                        &program.domains,
                        &port.domain_name,
                        &port.type_arguments,
                    )
                })
                .unwrap_or((None, None, None));
            ReportComponentGraphConnection {
                id: format!("{} -> {}", connection.left, connection.right),
                kind: "connection".to_owned(),
                left: connection.left.clone(),
                right: connection.right.clone(),
                left_component: connection.left_component.clone(),
                left_port: connection.left_port.clone(),
                right_component: connection.right_component.clone(),
                right_port: connection.right_port.clone(),
                domain_label: connection.domain.clone(),
                medium_label,
                frame_label,
                axis_label,
                status: connection.status.clone(),
                line: connection.line,
                source_span: report_source_span(connection.line),
            }
        })
        .collect::<Vec<_>>();
    let connection_sets = program
        .component_assemblies
        .iter()
        .flat_map(|assembly| {
            assembly.connection_sets.iter().map(move |connection_set| {
                ReportComponentGraphConnectionSet {
                    assembly: assembly.name.clone(),
                    name: connection_set.name.clone(),
                    domain_label: connection_set.domain.clone(),
                    status: connection_set.status.clone(),
                    connection_count: connection_set.connection_count,
                    ports: connection_set.ports.clone(),
                    line: connection_set.line,
                    source_span: report_source_span(connection_set.line),
                }
            })
        })
        .collect::<Vec<_>>();
    let behavior_nodes = report_component_behavior_nodes(report);

    ReportComponentGraph {
        format: "eng-component-graph-v1".to_owned(),
        status: status.to_owned(),
        node_count: assembly_components.len() + port_count + behavior_nodes.len(),
        edge_count: program.connections.len(),
        components,
        ports,
        connections,
        connection_sets,
        behavior_nodes,
    }
}

fn report_component_behavior_nodes(report: &CheckReport) -> Vec<ReportComponentGraphBehaviorNode> {
    report
        .semantic_program
        .assembly_components()
        .iter()
        .flat_map(|component| {
            component.local_expressions.iter().flat_map(move |local| {
                report_behavior_node_seeds(report, component, local)
                    .into_iter()
                    .map(move |seed| ReportComponentGraphBehaviorNode {
                        id: format!("{}.{}:{}", component.name, local.name, seed.behavior_kind),
                        kind: "behavior".to_owned(),
                        behavior_kind: seed.behavior_kind,
                        component: component.name.clone(),
                        name: local.name.clone(),
                        expression: local.expression.clone(),
                        status: seed.status,
                        signal: seed.signal,
                        delay_s: seed.delay_s,
                        relationship_status: seed.relationship_status,
                        contract_status: seed.contract_status,
                        jacobian_policy: seed.jacobian_policy,
                        profile_policy: seed.profile_policy,
                        contract_inputs: seed.contract_inputs,
                        contract_outputs: seed.contract_outputs,
                        diagnostic_channels: seed.diagnostic_channels,
                        runtime_warning_status: seed.runtime_warning_status,
                        line: local.line,
                        source_span: report_source_span(local.line),
                    })
            })
        })
        .collect()
}

struct ReportBehaviorSeed {
    behavior_kind: String,
    status: String,
    signal: Option<String>,
    delay_s: Option<f64>,
    relationship_status: Option<String>,
    contract_status: Option<String>,
    jacobian_policy: Option<String>,
    profile_policy: Option<String>,
    contract_inputs: Vec<ReportBehaviorSignalContract>,
    contract_outputs: Vec<ReportBehaviorSignalContract>,
    diagnostic_channels: Vec<String>,
    runtime_warning_status: Option<String>,
}

fn report_behavior_node_seeds(
    report: &CheckReport,
    component: &eng_compiler::ComponentInfo,
    local: &eng_compiler::ComponentLocalExpressionInfo,
) -> Vec<ReportBehaviorSeed> {
    let expression = &local.expression;
    let normalized = expression.to_ascii_lowercase();
    let mut nodes = Vec::new();
    if normalized.contains("delay(") {
        let arguments =
            first_report_behavior_call_arguments(expression, "delay").unwrap_or_default();
        let signal = arguments.first().cloned();
        let mut contract_inputs = Vec::new();
        if let Some(signal) = signal.as_deref() {
            contract_inputs.push(report_behavior_signal_contract(
                report, component, local, "input", signal,
            ));
        }
        contract_inputs.push(ReportBehaviorSignalContract {
            role: "input".to_owned(),
            name: "tau".to_owned(),
            quantity_kind: "Duration".to_owned(),
            display_unit: "s".to_owned(),
            canonical_unit: "s".to_owned(),
            status: "literal_duration_resolved".to_owned(),
        });
        let contract_outputs = contract_inputs
            .first()
            .map(|input| ReportBehaviorSignalContract {
                role: "output".to_owned(),
                name: local.name.clone(),
                quantity_kind: input.quantity_kind.clone(),
                display_unit: input.display_unit.clone(),
                canonical_unit: input.canonical_unit.clone(),
                status: "same_quantity_as_delayed_signal".to_owned(),
            })
            .into_iter()
            .collect();
        nodes.push(ReportBehaviorSeed {
            behavior_kind: "delay".to_owned(),
            status: "delay_call_runtime_buffer_pending_integration".to_owned(),
            signal,
            delay_s: arguments
                .get(1)
                .and_then(|duration| report_duration_seconds(duration.trim())),
            relationship_status: Some("delay_relationship_metadata_only".to_owned()),
            contract_status: None,
            jacobian_policy: None,
            profile_policy: None,
            contract_inputs,
            contract_outputs,
            diagnostic_channels: vec![
                "delay_history_underflow_failure".to_owned(),
                "delay_out_of_order_sample_failure".to_owned(),
            ],
            runtime_warning_status: Some("not_evaluated_in_language_behavior_graph".to_owned()),
        });
    }
    if normalized.contains("predict(") || normalized.contains("predictor(") {
        let signal = first_report_behavior_call_arguments(expression, "predictor")
            .or_else(|| first_report_behavior_call_arguments(expression, "predict"))
            .and_then(|arguments| arguments.first().cloned());
        let contract_inputs: Vec<ReportBehaviorSignalContract> = signal
            .as_deref()
            .map(|signal| {
                report_behavior_signal_contract(report, component, local, "input", signal)
            })
            .into_iter()
            .collect();
        nodes.push(ReportBehaviorSeed {
            behavior_kind: "predictor".to_owned(),
            status: "predictor_call_contract_pending_integration".to_owned(),
            signal,
            delay_s: None,
            relationship_status: None,
            contract_status: Some("predictor_contract_metadata".to_owned()),
            jacobian_policy: Some("solver_policy_not_integrated".to_owned()),
            profile_policy: None,
            contract_outputs: report_behavior_identity_output_contract(
                &contract_inputs,
                &local.name,
                "predictor_output_typed_identity_contract",
                "predictor_output_contract_unresolved",
            ),
            contract_inputs,
            diagnostic_channels: vec![
                "predictor_valid_range_warning".to_owned(),
                "predictor_output_layout_failure".to_owned(),
            ],
            runtime_warning_status: Some(
                "solver_api_only_until_behavior_graph_integration".to_owned(),
            ),
        });
    }
    if normalized.contains("external(") || normalized.contains("adapter(") {
        let signal = first_report_behavior_call_arguments(expression, "external")
            .or_else(|| first_report_behavior_call_arguments(expression, "adapter"))
            .and_then(|arguments| arguments.first().cloned());
        let contract_inputs: Vec<ReportBehaviorSignalContract> = signal
            .as_deref()
            .map(|signal| {
                report_behavior_signal_contract(report, component, local, "input", signal)
            })
            .into_iter()
            .collect();
        nodes.push(ReportBehaviorSeed {
            behavior_kind: "external".to_owned(),
            status: "external_behavior_wrapper_pending_integration".to_owned(),
            signal,
            delay_s: None,
            relationship_status: None,
            contract_status: Some("external_behavior_contract_metadata".to_owned()),
            jacobian_policy: None,
            profile_policy: Some("safe_repro_profile_policy_metadata".to_owned()),
            contract_outputs: report_behavior_identity_output_contract(
                &contract_inputs,
                &local.name,
                "external_output_typed_identity_contract",
                "external_output_contract_unresolved",
            ),
            contract_inputs,
            diagnostic_channels: vec![
                "external_profile_policy_failure".to_owned(),
                "external_adapter_failure".to_owned(),
            ],
            runtime_warning_status: Some(
                "solver_api_only_until_behavior_graph_integration".to_owned(),
            ),
        });
    }
    nodes
}

fn report_behavior_identity_output_contract(
    contract_inputs: &[ReportBehaviorSignalContract],
    output_name: &str,
    resolved_status: &str,
    unresolved_status: &str,
) -> Vec<ReportBehaviorSignalContract> {
    let Some(input) = contract_inputs.first() else {
        return vec![ReportBehaviorSignalContract {
            role: "output".to_owned(),
            name: output_name.to_owned(),
            quantity_kind: "unknown".to_owned(),
            display_unit: "unknown".to_owned(),
            canonical_unit: "unknown".to_owned(),
            status: unresolved_status.to_owned(),
        }];
    };
    vec![ReportBehaviorSignalContract {
        role: "output".to_owned(),
        name: output_name.to_owned(),
        quantity_kind: input.quantity_kind.clone(),
        display_unit: input.display_unit.clone(),
        canonical_unit: input.canonical_unit.clone(),
        status: resolved_status.to_owned(),
    }]
}

fn report_behavior_signal_contract(
    report: &CheckReport,
    component: &eng_compiler::ComponentInfo,
    local: &eng_compiler::ComponentLocalExpressionInfo,
    role: &str,
    signal: &str,
) -> ReportBehaviorSignalContract {
    report_behavior_expression_contract(report, component, local, role, signal, 0).unwrap_or_else(
        || ReportBehaviorSignalContract {
            role: role.to_owned(),
            name: signal.to_owned(),
            quantity_kind: "unknown".to_owned(),
            display_unit: "unknown".to_owned(),
            canonical_unit: "unknown".to_owned(),
            status: "signal_contract_unresolved".to_owned(),
        },
    )
}

fn report_behavior_expression_contract(
    report: &CheckReport,
    component: &eng_compiler::ComponentInfo,
    local: &eng_compiler::ComponentLocalExpressionInfo,
    role: &str,
    expression: &str,
    depth: usize,
) -> Option<ReportBehaviorSignalContract> {
    if depth > 8 {
        return None;
    }
    let trimmed_expression = report_strip_outer_parens(expression.trim());
    if let Some(contract) =
        report_named_signal_contract(report, component, local, role, trimmed_expression)
    {
        return Some(contract);
    }
    if let Some(arguments) = report_behavior_call_arguments_expression(trimmed_expression, "delay")
    {
        let parts = report_split_behavior_arguments(&arguments);
        if parts.len() != 2 || report_duration_seconds(parts[1].trim()).is_none() {
            return None;
        }
        let signal_contract = report_behavior_expression_contract(
            report,
            component,
            local,
            role,
            parts[0].trim(),
            depth + 1,
        )?;
        return Some(ReportBehaviorSignalContract {
            role: role.to_owned(),
            name: expression.trim().to_owned(),
            quantity_kind: signal_contract.quantity_kind,
            display_unit: signal_contract.display_unit,
            canonical_unit: signal_contract.canonical_unit,
            status: "delay_expression_signal_resolved".to_owned(),
        });
    }
    None
}

fn report_named_signal_contract(
    report: &CheckReport,
    component: &eng_compiler::ComponentInfo,
    local: &eng_compiler::ComponentLocalExpressionInfo,
    role: &str,
    signal: &str,
) -> Option<ReportBehaviorSignalContract> {
    let Some((port_name, variable_name)) = signal.split_once('.') else {
        return component
            .local_expressions
            .iter()
            .find(|candidate| candidate.name == signal.trim() && candidate.line < local.line)
            .filter(|candidate| {
                candidate.quantity_kind != "unknown"
                    && candidate.type_status != "signal_contract_unresolved"
            })
            .map(|local_signal| ReportBehaviorSignalContract {
                role: role.to_owned(),
                name: signal.to_owned(),
                quantity_kind: local_signal.quantity_kind.clone(),
                display_unit: local_signal.display_unit.clone(),
                canonical_unit: local_signal.canonical_unit.clone(),
                status: "component_local_signal_resolved".to_owned(),
            });
    };
    let port = component
        .ports
        .iter()
        .find(|port| port.name == port_name.trim())?;
    let domain = report
        .semantic_program
        .domains
        .iter()
        .find(|domain| domain.name == port.domain_name)?;
    let variable = domain
        .variables
        .iter()
        .find(|variable| variable.name == variable_name.trim())?;
    Some(ReportBehaviorSignalContract {
        role: role.to_owned(),
        name: signal.to_owned(),
        quantity_kind: variable.quantity_kind.clone(),
        display_unit: variable.display_unit.clone(),
        canonical_unit: variable.canonical_unit.clone(),
        status: "domain_signal_resolved".to_owned(),
    })
}

fn first_report_behavior_call_arguments(expression: &str, call_name: &str) -> Option<Vec<String>> {
    let lowered = expression.to_ascii_lowercase();
    let needle = format!("{call_name}(");
    let start = lowered.find(&needle)?;
    let open_index = start + call_name.len();
    let mut depth = 0i32;
    let mut close_index = None;
    for (index, character) in expression[open_index..].char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close_index = Some(open_index + index);
                    break;
                }
            }
            _ => {}
        }
    }
    let close_index = close_index?;
    Some(
        report_split_behavior_arguments(&expression[open_index + 1..close_index])
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect(),
    )
}

fn report_behavior_call_arguments_expression(expression: &str, call_name: &str) -> Option<String> {
    let trimmed = report_strip_outer_parens(expression.trim());
    let lowered = trimmed.to_ascii_lowercase();
    let prefix = format!("{call_name}(");
    if !lowered.starts_with(&prefix) || !trimmed.ends_with(')') {
        return None;
    }
    let inner = &trimmed[call_name.len() + 1..trimmed.len() - 1];
    report_is_balanced(inner).then(|| inner.to_owned())
}

fn report_split_behavior_arguments(arguments: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (index, character) in arguments.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(arguments[start..index].trim().to_owned());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(arguments[start..].trim().to_owned());
    parts
}

fn report_strip_outer_parens(mut expression: &str) -> &str {
    loop {
        let trimmed = expression.trim();
        if !(trimmed.starts_with('(') && trimmed.ends_with(')')) {
            return trimmed;
        }
        let inner = &trimmed[1..trimmed.len() - 1];
        if !report_is_balanced(inner) {
            return trimmed;
        }
        expression = inner;
    }
}

fn report_is_balanced(expression: &str) -> bool {
    let mut depth = 0i32;
    for character in expression.chars() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

fn report_duration_seconds(value: &str) -> Option<f64> {
    let mut parts = value.split_whitespace();
    let number = parts.next()?.parse::<f64>().ok()?;
    if number <= 0.0 {
        return None;
    }
    let unit = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    match unit {
        "s" | "sec" | "second" | "seconds" => Some(number),
        "min" | "minute" | "minutes" => Some(number * 60.0),
        "h" | "hr" | "hour" | "hours" => Some(number * 3600.0),
        _ => None,
    }
}

fn report_behavior_node_detail(node: &ReportComponentGraphBehaviorNode) -> String {
    let mut parts = Vec::new();
    if let Some(signal) = &node.signal {
        parts.push(format!("signal={signal}"));
    }
    if let Some(delay_s) = node.delay_s {
        parts.push(format!("delay_s={delay_s}"));
    }
    if let Some(status) = &node.relationship_status {
        parts.push(format!("relationship={}", report_status_label(status)));
    }
    if let Some(status) = &node.contract_status {
        parts.push(format!("contract={}", report_status_label(status)));
    }
    if let Some(policy) = &node.jacobian_policy {
        parts.push(format!("jacobian={}", report_status_label(policy)));
    }
    if let Some(policy) = &node.profile_policy {
        parts.push(format!("profile={}", report_status_label(policy)));
    }
    if !node.contract_inputs.is_empty() {
        parts.push(format!(
            "inputs={}",
            report_behavior_signal_contracts_detail(&node.contract_inputs)
        ));
    }
    if !node.contract_outputs.is_empty() {
        parts.push(format!(
            "outputs={}",
            report_behavior_signal_contracts_detail(&node.contract_outputs)
        ));
    }
    if !node.diagnostic_channels.is_empty() {
        parts.push(format!(
            "diagnostics={}",
            node.diagnostic_channels.join("|")
        ));
    }
    if let Some(status) = &node.runtime_warning_status {
        parts.push(format!("runtime_warnings={}", report_status_label(status)));
    }
    if parts.is_empty() {
        "-".to_owned()
    } else {
        parts.join(", ")
    }
}

fn report_behavior_signal_contracts_detail(contracts: &[ReportBehaviorSignalContract]) -> String {
    contracts
        .iter()
        .map(|contract| {
            format!(
                "{}:{}:{}[{}]/{}",
                contract.role,
                contract.name,
                contract.quantity_kind,
                contract.display_unit,
                report_status_label(&contract.status)
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn report_status_label(status: &str) -> &str {
    match status {
        "algebraic_only_preview" => "algebraic-only preview",
        "algebraic_split_preview" => "algebraic split preview",
        "component_local_signal_resolved" => "component-local signal resolved",
        "dae_split_deferred" => "DAE split deferred",
        "delay_call_runtime_buffer_pending_integration" => {
            "delay runtime buffer not connected to this language-level solve"
        }
        "delay_relationship_metadata_only" => "delay relationship metadata",
        "external_behavior_contract_metadata" => "external behavior contract metadata",
        "external_behavior_wrapper_pending_integration" => {
            "external behavior adapter not connected to this language-level solve"
        }
        "external_output_typed_identity_contract" => "external output typed from input",
        "mixed_state_algebraic_preview" => "mixed state/algebraic preview",
        "no_jit_speed_claim" => "no JIT speed claim",
        "not_adaptive" => "not adaptive",
        "not_full_dae" => "not a full DAE solve",
        "not_general_nonlinear" => "not a general nonlinear solve",
        "not_production_multi_domain" => "not production multi-domain",
        "predictor_call_contract_pending_integration" => {
            "Predictor contract not connected to this language-level solve"
        }
        "predictor_contract_metadata" => "Predictor contract metadata",
        "predictor_output_typed_identity_contract" => "Predictor output typed from input",
        "safe_repro_profile_policy_metadata" => "safe/repro profile policy metadata",
        "solver_policy_not_integrated" => "solver policy not connected",
        "symbolic_residual_preview_no_nonlinear_iteration" => {
            "symbolic residual preview, no nonlinear iteration"
        }
        other => other,
    }
}

fn report_status_list_label(values: &[String]) -> String {
    values
        .iter()
        .map(|value| report_status_label(value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn report_domain_argument_labels(
    domains: &[eng_compiler::DomainInfo],
    domain_name: &str,
    type_arguments: &[String],
) -> (Option<String>, Option<String>, Option<String>) {
    let mut medium_label = None;
    let mut frame_label = None;
    let mut axis_label = None;
    if let Some(domain) = domains.iter().find(|domain| domain.name == domain_name) {
        for (index, parameter) in domain.type_parameters.iter().enumerate() {
            let Some(argument) = type_arguments.get(index) else {
                continue;
            };
            match parameter.kind.as_str() {
                "Medium" => medium_label = Some(argument.clone()),
                "Frame" => frame_label = Some(argument.clone()),
                "Axis" => axis_label = Some(argument.clone()),
                _ => {}
            }
        }
    }
    (medium_label, frame_label, axis_label)
}

fn report_source_span(line: usize) -> ReportSourceSpan {
    ReportSourceSpan { line, column: 1 }
}

pub fn report_spec_json(spec: &ReportSpec) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"format\": \"eng-report-spec-v1\",\n");
    json.push_str(&format!(
        "  \"report_schema_version\": {REPORT_SPEC_VERSION},\n"
    ));
    json.push_str(&format!(
        "  \"compiler_version\": \"{}\",\n",
        json_escape(&spec.compiler_version)
    ));
    json.push_str(&format!(
        "  \"report_version\": \"{}\",\n",
        json_escape(&spec.report_version)
    ));
    json.push_str(&format!(
        "  \"source_path\": \"{}\",\n",
        json_escape(&spec.source_path)
    ));
    json.push_str(&format!(
        "  \"source_hash\": \"{}\",\n",
        json_escape(&spec.source_hash)
    ));
    json.push_str("  \"provenance\": {\n");
    json.push_str(&format!(
        "    \"syntax_items\": {},\n",
        spec.provenance.syntax_items
    ));
    json.push_str(&format!(
        "    \"schema_count\": {},\n",
        spec.provenance.schema_count
    ));
    json.push_str(&format!(
        "    \"csv_promotion_count\": {},\n",
        spec.provenance.csv_promotion_count
    ));
    json.push_str(&format!(
        "    \"domain_count\": {},\n",
        spec.provenance.domain_count
    ));
    json.push_str(&format!(
        "    \"component_count\": {},\n",
        spec.provenance.component_count
    ));
    json.push_str(&format!(
        "    \"connection_count\": {},\n",
        spec.provenance.connection_count
    ));
    json.push_str(&format!(
        "    \"assembly_count\": {},\n",
        spec.provenance.assembly_count
    ));
    json.push_str(&format!(
        "    \"class_count\": {},\n",
        spec.provenance.class_count
    ));
    json.push_str(&format!(
        "    \"object_count\": {},\n",
        spec.provenance.object_count
    ));
    json.push_str(&format!(
        "    \"system_count\": {},\n",
        spec.provenance.system_count
    ));
    json.push_str(&format!(
        "    \"equation_count\": {},\n",
        spec.provenance.equation_count
    ));
    json.push_str(&format!(
        "    \"residual_count\": {},\n",
        spec.provenance.residual_count
    ));
    json.push_str("    \"environment_dependencies\": [\n");
    for (index, dependency) in spec.provenance.environment_dependencies.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&dependency.name)
        ));
        json.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&dependency.kind)
        ));
        json.push_str(&format!(
            "        \"expression\": \"{}\",\n",
            json_escape(&dependency.expression)
        ));
        json.push_str(&format!(
            "        \"resolved_value\": \"{}\",\n",
            json_escape(&dependency.resolved_value)
        ));
        match &dependency.source_hash {
            Some(source_hash) => json.push_str(&format!(
                "        \"source_hash\": \"{}\",\n",
                json_escape(source_hash)
            )),
            None => json.push_str("        \"source_hash\": null,\n"),
        }
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&dependency.status)
        ));
        json.push_str(&format!("        \"line\": {}\n", dependency.line));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");
    json.push_str(&format!(
        "    \"plot_spec_version\": {}\n",
        spec.provenance.plot_spec_version
    ));
    json.push_str("  },\n");

    json.push_str("  \"variable_table\": [\n");
    for (index, variable) in spec.variables.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&variable.name)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&variable.quantity_kind)
        ));
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&variable.display_unit)
        ));
        json.push_str(&format!(
            "      \"canonical_unit\": \"{}\",\n",
            json_escape(&variable.canonical_unit)
        ));
        json.push_str(&format!(
            "      \"dimension\": \"{}\",\n",
            json_escape(&variable.dimension)
        ));
        json.push_str(&format!(
            "      \"source\": \"{}\",\n",
            json_escape(&variable.source)
        ));
        json.push_str(&format!("      \"line\": {}\n", variable.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"inferred_declaration_table\": [\n");
    for (index, declaration) in spec.inferred_declarations.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&declaration.name)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&declaration.quantity_kind)
        ));
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&declaration.display_unit)
        ));
        json.push_str(&format!(
            "      \"expression\": \"{}\",\n",
            json_escape(&declaration.expression)
        ));
        json.push_str(&format!("      \"line\": {}\n", declaration.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"unit_conversion_table\": [\n");
    for (index, conversion) in spec.unit_conversions.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&conversion.name)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&conversion.quantity_kind)
        ));
        if let Some(source_unit) = &conversion.source_unit {
            json.push_str(&format!(
                "      \"source_unit\": \"{}\",\n",
                json_escape(source_unit)
            ));
        } else {
            json.push_str("      \"source_unit\": null,\n");
        }
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&conversion.display_unit)
        ));
        json.push_str(&format!(
            "      \"canonical_unit\": \"{}\",\n",
            json_escape(&conversion.canonical_unit)
        ));
        json.push_str(&format!("      \"line\": {},\n", conversion.line));
        json.push_str("      \"steps\": [");
        push_json_string_array(&mut json, &conversion.steps);
        json.push_str("]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"schema_summary\": [\n");
    for (index, schema) in spec.schemas.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&schema.name)
        ));
        json.push_str(&format!("      \"line\": {},\n", schema.line));
        json.push_str("      \"columns\": [");
        push_json_string_array(&mut json, &schema.columns);
        json.push_str("],\n");
        json.push_str(&format!(
            "      \"column_count\": {},\n",
            schema.column_count
        ));
        json.push_str(&format!(
            "      \"constraint_count\": {},\n",
            schema.constraint_count
        ));
        json.push_str(&format!(
            "      \"missing_policy_count\": {}\n",
            schema.missing_policy_count
        ));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"args_summary\": [\n");
    for (index, args_block) in spec.args.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&args_block.name)
        ));
        json.push_str(&format!("      \"line\": {},\n", args_block.line));
        json.push_str(&format!(
            "      \"field_count\": {},\n",
            args_block.fields.len()
        ));
        json.push_str("      \"fields\": [\n");
        for (field_index, field) in args_block.fields.iter().enumerate() {
            if field_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&field.name)
            ));
            json.push_str(&format!(
                "          \"type\": \"{}\",\n",
                json_escape(&field.type_name)
            ));
            if let Some(default_value) = &field.default_value {
                json.push_str(&format!(
                    "          \"default\": \"{}\",\n",
                    json_escape(default_value)
                ));
            } else {
                json.push_str("          \"default\": null,\n");
            }
            json.push_str(&format!("          \"required\": {},\n", field.required));
            json.push_str(&format!("          \"line\": {}\n", field.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"arg_values\": [\n");
    for (index, arg) in spec.arg_values.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&arg.name)
        ));
        json.push_str(&format!(
            "      \"type\": \"{}\",\n",
            json_escape(&arg.type_name)
        ));
        json.push_str(&format!(
            "      \"value\": \"{}\",\n",
            json_escape(&arg.value)
        ));
        json.push_str(&format!(
            "      \"source\": \"{}\",\n",
            json_escape(&arg.source)
        ));
        json.push_str(&format!("      \"required\": {},\n", arg.required));
        json.push_str(&format!("      \"line\": {}\n", arg.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"computed_statistics\": [\n");
    for (index, summary) in spec.computed_statistics.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"source\": \"{}\",\n",
            json_escape(&summary.source)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&summary.quantity_kind)
        ));
        json.push_str(&format!(
            "      \"axis\": \"{}\",\n",
            json_escape(&summary.axis)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&summary.status)
        ));
        json.push_str("      \"values\": [\n");
        for (value_index, value) in summary.values.iter().enumerate() {
            if value_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&value.name)
            ));
            json.push_str(&format!("          \"value\": {},\n", value.value));
            json.push_str(&format!(
                "          \"unit\": \"{}\"\n",
                json_escape(&value.unit)
            ));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"computed_integrations\": [\n");
    for (index, integration) in spec.computed_integrations.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&integration.binding)
        ));
        json.push_str(&format!(
            "      \"source\": \"{}\",\n",
            json_escape(&integration.source)
        ));
        json.push_str(&format!(
            "      \"input_quantity\": \"{}\",\n",
            json_escape(&integration.input_quantity)
        ));
        json.push_str(&format!(
            "      \"over_axis\": \"{}\",\n",
            json_escape(&integration.over_axis)
        ));
        json.push_str(&format!(
            "      \"result_quantity\": \"{}\",\n",
            json_escape(&integration.result_quantity)
        ));
        json.push_str(&format!("      \"value\": {},\n", integration.value));
        json.push_str(&format!(
            "      \"unit\": \"{}\",\n",
            json_escape(&integration.unit)
        ));
        json.push_str(&format!(
            "      \"method\": \"{}\",\n",
            json_escape(&integration.method)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\"\n",
            json_escape(&integration.status)
        ));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"computed_metrics\": [\n");
    for (index, metric) in spec.computed_metrics.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&metric.binding)
        ));
        json.push_str(&format!(
            "      \"kind\": \"{}\",\n",
            json_escape(&metric.kind)
        ));
        json.push_str(&format!(
            "      \"left\": \"{}\",\n",
            json_escape(&metric.left)
        ));
        json.push_str(&format!(
            "      \"right\": \"{}\",\n",
            json_escape(&metric.right)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&metric.quantity_kind)
        ));
        json.push_str(&format!(
            "      \"unit\": \"{}\",\n",
            json_escape(&metric.unit)
        ));
        json.push_str(&format!("      \"value\": {},\n", metric.value));
        json.push_str(&format!(
            "      \"sample_count\": {},\n",
            metric.sample_count
        ));
        push_optional_json_string(
            &mut json,
            "alignment_reference",
            metric.alignment_reference.as_deref(),
            6,
        );
        push_optional_json_string(
            &mut json,
            "alignment_status",
            metric.alignment_status.as_deref(),
            6,
        );
        push_optional_json_string(
            &mut json,
            "alignment_step_status",
            metric.alignment_step_status.as_deref(),
            6,
        );
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&metric.status)
        ));
        json.push_str(&format!("      \"line\": {}\n", metric.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"validations\": [\n");
    for (index, validation) in spec.validations.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"expression\": \"{}\",\n",
            json_escape(&validation.expression)
        ));
        json.push_str(&format!(
            "      \"left\": \"{}\",\n",
            json_escape(&validation.left)
        ));
        json.push_str(&format!(
            "      \"operator\": \"{}\",\n",
            json_escape(&validation.operator)
        ));
        json.push_str(&format!(
            "      \"right\": \"{}\",\n",
            json_escape(&validation.right)
        ));
        push_optional_json_f64(&mut json, "left_value", validation.left_value, 6);
        push_optional_json_f64(&mut json, "right_value", validation.right_value, 6);
        json.push_str(&format!(
            "      \"unit\": \"{}\",\n",
            json_escape(&validation.unit)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&validation.status)
        ));
        json.push_str(&format!("      \"line\": {}\n", validation.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"quality_report\": {\n");
    json.push_str(&format!(
        "    \"status\": \"{}\",\n",
        json_escape(&spec.quality_report.status)
    ));
    json.push_str(&format!(
        "    \"total_count\": {},\n",
        spec.quality_report.total_count
    ));
    json.push_str(&format!(
        "    \"passed_count\": {},\n",
        spec.quality_report.passed_count
    ));
    json.push_str(&format!(
        "    \"warning_count\": {},\n",
        spec.quality_report.warning_count
    ));
    json.push_str(&format!(
        "    \"failed_count\": {},\n",
        spec.quality_report.failed_count
    ));
    json.push_str(&format!(
        "    \"unavailable_count\": {},\n",
        spec.quality_report.unavailable_count
    ));
    json.push_str("    \"results\": [\n");
    for (index, result) in spec.quality_report.results.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&result.binding)
        ));
        json.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&result.kind)
        ));
        json.push_str(&format!(
            "        \"category\": \"{}\",\n",
            json_escape(&result.category)
        ));
        json.push_str(&format!(
            "        \"target\": \"{}\",\n",
            json_escape(&result.target)
        ));
        json.push_str(&format!(
            "        \"subject\": \"{}\",\n",
            json_escape(&result.subject)
        ));
        push_optional_json_f64(&mut json, "score", result.score, 8);
        json.push_str(&format!(
            "        \"passed_count\": {},\n",
            result.passed_count
        ));
        json.push_str(&format!(
            "        \"warning_count\": {},\n",
            result.warning_count
        ));
        json.push_str(&format!(
            "        \"failed_count\": {},\n",
            result.failed_count
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&result.status)
        ));
        json.push_str(&format!(
            "        \"reason\": \"{}\",\n",
            json_escape(&result.reason)
        ));
        json.push_str("        \"failures\": [\n");
        for (failure_index, failure) in result.failures.iter().enumerate() {
            if failure_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!("            \"row\": {},\n", failure.row));
            json.push_str(&format!(
                "            \"field\": \"{}\",\n",
                json_escape(&failure.field)
            ));
            json.push_str(&format!(
                "            \"value\": \"{}\",\n",
                json_escape(&failure.value)
            ));
            json.push_str(&format!(
                "            \"message\": \"{}\"\n",
                json_escape(&failure.message)
            ));
            json.push_str("          }");
        }
        json.push_str("\n        ],\n");
        json.push_str(&format!("        \"line\": {}\n", result.line));
        json.push_str("      }");
    }
    json.push_str("\n    ]\n");
    json.push_str("  },\n");

    json.push_str("  \"time_axes\": [\n");
    for (index, axis) in spec.time_axes.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&axis.name)
        ));
        json.push_str(&format!(
            "      \"source_table\": \"{}\",\n",
            json_escape(&axis.source_table)
        ));
        json.push_str(&format!(
            "      \"source_column\": \"{}\",\n",
            json_escape(&axis.source_column)
        ));
        json.push_str(&format!(
            "      \"axis\": \"{}\",\n",
            json_escape(&axis.axis)
        ));
        json.push_str(&format!(
            "      \"unit\": \"{}\",\n",
            json_escape(&axis.unit)
        ));
        push_optional_json_f64(&mut json, "start", axis.start, 6);
        push_optional_json_f64(&mut json, "end", axis.end, 6);
        json.push_str(&format!("      \"count\": {},\n", axis.count));
        push_optional_json_f64(&mut json, "nominal_step", axis.nominal_step, 6);
        json.push_str(&format!("      \"irregular\": {},\n", axis.irregular));
        json.push_str(&format!(
            "      \"missing_count\": {}\n",
            axis.missing_count
        ));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"time_alignments\": [\n");
    for (index, alignment) in spec.time_alignments.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&alignment.binding)
        ));
        json.push_str(&format!(
            "      \"left\": \"{}\",\n",
            json_escape(&alignment.left)
        ));
        json.push_str(&format!(
            "      \"right\": \"{}\",\n",
            json_escape(&alignment.right)
        ));
        json.push_str(&format!(
            "      \"axis\": \"{}\",\n",
            json_escape(&alignment.axis)
        ));
        json.push_str(&format!(
            "      \"strategy\": \"{}\",\n",
            json_escape(&alignment.strategy)
        ));
        json.push_str(&format!(
            "      \"method\": \"{}\",\n",
            json_escape(&alignment.method)
        ));
        push_optional_json_f64(&mut json, "resample_step", alignment.resample_step, 6);
        push_optional_json_f64(&mut json, "tolerance", alignment.tolerance, 6);
        json.push_str(&format!(
            "      \"left_count\": {},\n",
            alignment.left_count
        ));
        json.push_str(&format!(
            "      \"right_count\": {},\n",
            alignment.right_count
        ));
        json.push_str(&format!(
            "      \"matched_count\": {},\n",
            alignment.matched_count
        ));
        json.push_str(&format!(
            "      \"target_count\": {},\n",
            alignment.target_count
        ));
        json.push_str(&format!(
            "      \"output_count\": {},\n",
            alignment.output_count
        ));
        json.push_str(&format!(
            "      \"materialization_status\": \"{}\",\n",
            json_escape(&alignment.materialization_status)
        ));
        json.push_str(&format!(
            "      \"materialization_reason\": \"{}\",\n",
            json_escape(&alignment.materialization_reason)
        ));
        push_optional_json_f64(
            &mut json,
            "left_nominal_step",
            alignment.left_nominal_step,
            6,
        );
        push_optional_json_f64(
            &mut json,
            "right_nominal_step",
            alignment.right_nominal_step,
            6,
        );
        json.push_str(&format!(
            "      \"left_irregular\": {},\n",
            alignment.left_irregular
        ));
        json.push_str(&format!(
            "      \"right_irregular\": {},\n",
            alignment.right_irregular
        ));
        json.push_str(&format!(
            "      \"step_status\": \"{}\",\n",
            json_escape(&alignment.step_status)
        ));
        push_optional_json_f64(&mut json, "overlap_start", alignment.overlap_start, 6);
        push_optional_json_f64(&mut json, "overlap_end", alignment.overlap_end, 6);
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&alignment.status)
        ));
        json.push_str(&format!("      \"line\": {}\n", alignment.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"uncertainty\": [\n");
    for (index, uncertainty) in spec.uncertainty.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&uncertainty.binding)
        ));
        json.push_str(&format!(
            "      \"kind\": \"{}\",\n",
            json_escape(&uncertainty.kind)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&uncertainty.quantity_kind)
        ));
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&uncertainty.display_unit)
        ));
        json.push_str(&format!(
            "      \"expression\": \"{}\",\n",
            json_escape(&uncertainty.expression)
        ));
        push_optional_json_string(&mut json, "source", uncertainty.source.as_deref(), 6);
        push_optional_json_string(
            &mut json,
            "distribution",
            uncertainty.distribution.as_deref(),
            6,
        );
        push_optional_json_string(&mut json, "method", uncertainty.method.as_deref(), 6);
        push_optional_json_string(&mut json, "scale", uncertainty.scale.as_deref(), 6);
        push_optional_json_string(&mut json, "offset", uncertainty.offset.as_deref(), 6);
        push_optional_json_string(&mut json, "mean", uncertainty.mean.as_deref(), 6);
        push_optional_json_string(&mut json, "stddev", uncertainty.stddev.as_deref(), 6);
        push_optional_json_string(&mut json, "error", uncertainty.error.as_deref(), 6);
        push_optional_json_string(&mut json, "lower", uncertainty.lower.as_deref(), 6);
        push_optional_json_string(&mut json, "upper", uncertainty.upper.as_deref(), 6);
        push_optional_json_string(&mut json, "p05", uncertainty.p05.as_deref(), 6);
        push_optional_json_string(&mut json, "p50", uncertainty.p50.as_deref(), 6);
        push_optional_json_string(&mut json, "p95", uncertainty.p95.as_deref(), 6);
        json.push_str(&format!(
            "      \"sample_count\": {},\n",
            uncertainty.sample_count
        ));
        json.push_str(&format!(
            "      \"propagation_count\": {},\n",
            uncertainty.propagation_count
        ));
        json.push_str("      \"propagation\": [");
        push_uncertainty_propagation_terms(&mut json, &uncertainty.propagation);
        json.push_str("],\n");
        json.push_str(&format!("      \"line\": {}\n", uncertainty.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"ml\": [\n");
    for (index, ml) in spec.ml.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&ml.binding)
        ));
        json.push_str(&format!("      \"kind\": \"{}\",\n", json_escape(&ml.kind)));
        push_optional_json_string(&mut json, "source", ml.source.as_deref(), 6);
        push_optional_json_string(&mut json, "target", ml.target.as_deref(), 6);
        push_optional_json_string(
            &mut json,
            "target_quantity",
            ml.target_quantity.as_deref(),
            6,
        );
        json.push_str(&format!(
            "      \"target_unit\": \"{}\",\n",
            json_escape(&ml.target_unit)
        ));
        json.push_str("      \"features\": [");
        push_json_string_array(&mut json, &ml.features);
        json.push_str("],\n");
        push_optional_json_string(&mut json, "algorithm", ml.algorithm.as_deref(), 6);
        push_optional_json_string(&mut json, "test_fraction", ml.test_fraction.as_deref(), 6);
        push_optional_json_string(&mut json, "seed", ml.seed.as_deref(), 6);
        json.push_str("      \"hidden_layers\": [");
        for (layer_index, layer) in ml.hidden_layers.iter().enumerate() {
            if layer_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&layer.to_string());
        }
        json.push_str("],\n");
        push_optional_json_usize(&mut json, "epochs", ml.epochs, 6);
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&ml.status)
        ));
        push_optional_json_usize(&mut json, "train_count", ml.train_count, 6);
        push_optional_json_usize(&mut json, "test_count", ml.test_count, 6);
        push_optional_json_f64(&mut json, "rmse", ml.rmse, 6);
        push_optional_json_f64(&mut json, "mae", ml.mae, 6);
        push_optional_json_f64(&mut json, "r2", ml.r2, 6);
        push_optional_json_string(&mut json, "leakage_status", ml.leakage_status.as_deref(), 6);
        json.push_str("      \"leakage_findings\": [");
        push_json_string_array(&mut json, &ml.leakage_findings);
        json.push_str("],\n");
        json.push_str("      \"coefficients\": [");
        for (coefficient_index, coefficient) in ml.coefficients.iter().enumerate() {
            if coefficient_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!(
                "{{\"feature\":\"{}\",\"value\":{}}}",
                json_escape(&coefficient.feature),
                coefficient.value
            ));
        }
        json.push_str("],\n");
        push_optional_json_f64(&mut json, "intercept", ml.intercept, 6);
        json.push_str("      \"loss_history\": [");
        for (loss_index, loss) in ml.loss_history.iter().enumerate() {
            if loss_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&loss.to_string());
        }
        json.push_str("],\n");
        push_optional_json_string(
            &mut json,
            "training_data_hash",
            ml.training_data_hash.as_deref(),
            6,
        );
        push_optional_json_string(
            &mut json,
            "model_artifact_hash",
            ml.model_artifact_hash.as_deref(),
            6,
        );
        push_optional_json_string(&mut json, "model_card", ml.model_card.as_deref(), 6);
        json.push_str(&format!(
            "      \"expression\": \"{}\",\n",
            json_escape(&ml.expression)
        ));
        json.push_str(&format!("      \"line\": {}\n", ml.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"policy_results\": [\n");
    for (index, policy) in spec.policy_results.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"schema\": \"{}\",\n",
            json_escape(&policy.schema)
        ));
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&policy.binding)
        ));
        json.push_str(&format!(
            "      \"kind\": \"{}\",\n",
            json_escape(&policy.kind)
        ));
        json.push_str(&format!(
            "      \"target\": \"{}\",\n",
            json_escape(&policy.target)
        ));
        json.push_str(&format!(
            "      \"policy\": \"{}\",\n",
            json_escape(&policy.policy)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&policy.status)
        ));
        json.push_str(&format!(
            "      \"checked_rows\": {},\n",
            policy.checked_rows
        ));
        json.push_str(&format!(
            "      \"violation_count\": {},\n",
            policy.violation_count
        ));
        json.push_str("      \"violations\": [\n");
        for (violation_index, violation) in policy.violations.iter().enumerate() {
            if violation_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!("          \"row\": {},\n", violation.row));
            json.push_str(&format!(
                "          \"column\": \"{}\",\n",
                json_escape(&violation.column)
            ));
            json.push_str(&format!(
                "          \"value\": \"{}\",\n",
                json_escape(&violation.value)
            ));
            json.push_str(&format!(
                "          \"message\": \"{}\"\n",
                json_escape(&violation.message)
            ));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str(&format!("      \"line\": {}\n", policy.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"domain_summary\": [\n");
    for (index, domain) in spec.domains.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&domain.name)
        ));
        json.push_str("      \"type_parameters\": [");
        for (parameter_index, parameter) in domain.type_parameters.iter().enumerate() {
            if parameter_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!(
                "{{\"kind\": \"{}\", \"name\": \"{}\", \"display\": \"{}\"}}",
                json_escape(&parameter.kind),
                json_escape(&parameter.name),
                json_escape(&parameter.display)
            ));
        }
        json.push_str("],\n");
        match &domain.package {
            Some(package) => json.push_str(&format!(
                "      \"package\": \"{}\",\n",
                json_escape(package)
            )),
            None => json.push_str("      \"package\": null,\n"),
        }
        match &domain.version {
            Some(version) => json.push_str(&format!(
                "      \"version\": \"{}\",\n",
                json_escape(version)
            )),
            None => json.push_str("      \"version\": null,\n"),
        }
        json.push_str(&format!("      \"line\": {},\n", domain.line));
        json.push_str(&format!(
            "      \"variable_count\": {},\n",
            domain.variables.len()
        ));
        json.push_str(&format!(
            "      \"conservation_count\": {},\n",
            domain.conservations.len()
        ));
        json.push_str("      \"variables\": [\n");
        for (variable_index, variable) in domain.variables.iter().enumerate() {
            if variable_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"role\": \"{}\",\n",
                json_escape(&variable.role)
            ));
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&variable.name)
            ));
            json.push_str(&format!(
                "          \"quantity_kind\": \"{}\",\n",
                json_escape(&variable.quantity_kind)
            ));
            json.push_str(&format!(
                "          \"display_unit\": \"{}\",\n",
                json_escape(&variable.display_unit)
            ));
            json.push_str(&format!(
                "          \"canonical_unit\": \"{}\",\n",
                json_escape(&variable.canonical_unit)
            ));
            json.push_str(&format!(
                "          \"dimension\": \"{}\",\n",
                json_escape(&variable.dimension)
            ));
            json.push_str(&format!("          \"line\": {}\n", variable.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"conservations\": [\n");
        for (conservation_index, conservation) in domain.conservations.iter().enumerate() {
            if conservation_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"text\": \"{}\",\n",
                json_escape(&conservation.text)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&conservation.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", conservation.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"component_summary\": [\n");
    for (index, component) in spec.components.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&component.name)
        ));
        push_optional_json_string(
            &mut json,
            "template_name",
            component.template_name.as_deref(),
            6,
        );
        json.push_str("      \"constructor_arguments\": [\n");
        for (argument_index, argument) in component.constructor_arguments.iter().enumerate() {
            if argument_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&argument.name)
            ));
            json.push_str(&format!(
                "          \"value\": \"{}\"\n",
                json_escape(&argument.value)
            ));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"parameters\": [\n");
        for (parameter_index, parameter) in component.parameters.iter().enumerate() {
            if parameter_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&parameter.name)
            ));
            json.push_str(&format!(
                "          \"quantity_kind\": \"{}\",\n",
                json_escape(&parameter.quantity_kind)
            ));
            json.push_str(&format!(
                "          \"display_unit\": \"{}\",\n",
                json_escape(&parameter.display_unit)
            ));
            json.push_str(&format!(
                "          \"canonical_unit\": \"{}\",\n",
                json_escape(&parameter.canonical_unit)
            ));
            push_optional_json_string(
                &mut json,
                "default_value",
                parameter.default_value.as_deref(),
                10,
            );
            push_optional_json_string(&mut json, "value", parameter.value.as_deref(), 10);
            json.push_str(&format!(
                "          \"source\": \"{}\",\n",
                json_escape(&parameter.source)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&parameter.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", parameter.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str(&format!("      \"line\": {},\n", component.line));
        json.push_str(&format!(
            "      \"parameter_count\": {},\n",
            component.parameters.len()
        ));
        json.push_str(&format!(
            "      \"port_count\": {},\n",
            component.ports.len()
        ));
        json.push_str(&format!(
            "      \"local_expression_count\": {},\n",
            component.local_expressions.len()
        ));
        json.push_str("      \"ports\": [\n");
        for (port_index, port) in component.ports.iter().enumerate() {
            if port_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&port.name)
            ));
            json.push_str(&format!(
                "          \"domain\": \"{}\",\n",
                json_escape(&port.domain)
            ));
            json.push_str(&format!(
                "          \"domain_name\": \"{}\",\n",
                json_escape(&port.domain_name)
            ));
            json.push_str("          \"type_arguments\": [");
            for (argument_index, argument) in port.type_arguments.iter().enumerate() {
                if argument_index > 0 {
                    json.push_str(", ");
                }
                json.push_str(&format!("\"{}\"", json_escape(argument)));
            }
            json.push_str("],\n");
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&port.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", port.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"local_expressions\": [\n");
        for (local_index, local) in component.local_expressions.iter().enumerate() {
            if local_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&local.name)
            ));
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&local.expression)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&local.status)
            ));
            json.push_str(&format!(
                "          \"quantity_kind\": \"{}\",\n",
                json_escape(&local.quantity_kind)
            ));
            json.push_str(&format!(
                "          \"display_unit\": \"{}\",\n",
                json_escape(&local.display_unit)
            ));
            json.push_str(&format!(
                "          \"canonical_unit\": \"{}\",\n",
                json_escape(&local.canonical_unit)
            ));
            json.push_str(&format!(
                "          \"type_status\": \"{}\",\n",
                json_escape(&local.type_status)
            ));
            json.push_str(&format!("          \"line\": {}\n", local.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"connection_summary\": [\n");
    for (index, connection) in spec.connections.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"left\": \"{}\",\n",
            json_escape(&connection.left)
        ));
        json.push_str(&format!(
            "      \"right\": \"{}\",\n",
            json_escape(&connection.right)
        ));
        json.push_str(&format!(
            "      \"domain\": \"{}\",\n",
            json_escape(&connection.domain)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&connection.status)
        ));
        json.push_str(&format!("      \"line\": {}\n", connection.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"assembly_summary\": [\n");
    for (index, assembly) in spec.assemblies.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&assembly.name)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&assembly.status)
        ));
        json.push_str(&format!("      \"line\": {},\n", assembly.line));
        if let Some(source_span) = &assembly.source_span {
            json.push_str(&format!(
                "      \"source_span\": {{ \"line\": {}, \"column\": {} }},\n",
                source_span.line, source_span.column
            ));
        } else {
            json.push_str("      \"source_span\": null,\n");
        }
        json.push_str(&format!(
            "      \"component_count\": {},\n",
            assembly.component_count
        ));
        json.push_str(&format!("      \"port_count\": {},\n", assembly.port_count));
        json.push_str(&format!(
            "      \"connection_count\": {},\n",
            assembly.connection_count
        ));
        json.push_str(&format!(
            "      \"component_equation_count\": {},\n",
            assembly.component_equation_count
        ));
        json.push_str(&format!(
            "      \"local_expression_count\": {},\n",
            assembly.local_expression_count
        ));
        json.push_str(&format!(
            "      \"operator_call_count\": {},\n",
            assembly.operator_call_count
        ));
        json.push_str(&format!(
            "      \"predictor_call_count\": {},\n",
            assembly.predictor_call_count
        ));
        json.push_str(&format!(
            "      \"domain_count\": {},\n",
            assembly.domain_count
        ));
        json.push_str("      \"domain_plans\": [\n");
        for (plan_index, plan) in assembly.domain_plans.iter().enumerate() {
            if plan_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"domain\": \"{}\",\n",
                json_escape(&plan.domain)
            ));
            json.push_str(&format!(
                "          \"connection_set_count\": {},\n",
                plan.connection_set_count
            ));
            json.push_str(&format!(
                "          \"equation_count\": {},\n",
                plan.equation_count
            ));
            json.push_str(&format!(
                "          \"variable_count\": {},\n",
                plan.variable_count
            ));
            json.push_str(&format!(
                "          \"conservation_status\": \"{}\",\n",
                json_escape(&plan.conservation_status)
            ));
            json.push_str(&format!(
                "          \"solver_role\": \"{}\"\n",
                json_escape(&plan.solver_role)
            ));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"solver_preview\": {\n");
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&assembly.solver_preview.status)
        ));
        json.push_str(&format!(
            "        \"method\": \"{}\",\n",
            json_escape(&assembly.solver_preview.method)
        ));
        json.push_str(&format!(
            "        \"mixed_algebraic_dynamic\": \"{}\",\n",
            json_escape(&assembly.solver_preview.mixed_algebraic_dynamic)
        ));
        json.push_str(&format!(
            "        \"nonlinear_residual\": \"{}\",\n",
            json_escape(&assembly.solver_preview.nonlinear_residual)
        ));
        json.push_str(&format!(
            "        \"dae_split\": \"{}\",\n",
            json_escape(&assembly.solver_preview.dae_split)
        ));
        json.push_str(&format!(
            "        \"delay_history\": \"{}\",\n",
            json_escape(&assembly.solver_preview.delay_history)
        ));
        json.push_str(&format!(
            "        \"predictor\": \"{}\",\n",
            json_escape(&assembly.solver_preview.predictor)
        ));
        json.push_str(&format!(
            "        \"external_adapter\": \"{}\",\n",
            json_escape(&assembly.solver_preview.external_adapter)
        ));
        json.push_str("        \"limitations\": [");
        for (limitation_index, limitation) in assembly.solver_preview.limitations.iter().enumerate()
        {
            if limitation_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(limitation)));
        }
        json.push_str("]\n");
        json.push_str("      },\n");
        json.push_str("      \"connection_sets\": [\n");
        for (set_index, connection_set) in assembly.connection_sets.iter().enumerate() {
            if set_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&connection_set.name)
            ));
            json.push_str(&format!(
                "          \"domain\": \"{}\",\n",
                json_escape(&connection_set.domain)
            ));
            json.push_str(&format!(
                "          \"connection_count\": {},\n",
                connection_set.connection_count
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&connection_set.status)
            ));
            json.push_str(&format!("          \"line\": {},\n", connection_set.line));
            json.push_str("          \"ports\": [");
            for (port_index, port) in connection_set.ports.iter().enumerate() {
                if port_index > 0 {
                    json.push_str(", ");
                }
                json.push_str(&format!("\"{}\"", json_escape(port)));
            }
            json.push_str("]\n");
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"equations\": [\n");
        for (equation_index, equation) in assembly.equations.iter().enumerate() {
            if equation_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&equation.name)
            ));
            json.push_str(&format!(
                "          \"kind\": \"{}\",\n",
                json_escape(&equation.kind)
            ));
            json.push_str(&format!(
                "          \"domain\": \"{}\",\n",
                json_escape(&equation.domain)
            ));
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&equation.expression)
            ));
            json.push_str(&format!(
                "          \"residual\": \"{}\",\n",
                json_escape(&equation.residual)
            ));
            match &equation.rhs {
                Some(rhs) => {
                    json.push_str(&format!("          \"rhs\": \"{}\",\n", json_escape(rhs)))
                }
                None => json.push_str("          \"rhs\": null,\n"),
            }
            json.push_str(&format!(
                "          \"reason\": \"{}\",\n",
                json_escape(&equation.reason)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&equation.status)
            ));
            json.push_str(&format!("          \"line\": {},\n", equation.line));
            json.push_str("          \"dependencies\": [");
            for (dependency_index, dependency) in equation.dependencies.iter().enumerate() {
                if dependency_index > 0 {
                    json.push_str(", ");
                }
                json.push_str(&format!("\"{}\"", json_escape(dependency)));
            }
            json.push_str("]\n");
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"variables\": [\n");
        for (variable_index, variable) in assembly.variables.iter().enumerate() {
            if variable_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&variable.name)
            ));
            json.push_str(&format!(
                "          \"role\": \"{}\",\n",
                json_escape(&variable.role)
            ));
            json.push_str(&format!(
                "          \"domain\": \"{}\",\n",
                json_escape(&variable.domain)
            ));
            json.push_str(&format!(
                "          \"source\": \"{}\",\n",
                json_escape(&variable.source)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\"\n",
                json_escape(&variable.status)
            ));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"boundary\": {\n");
        json.push_str(&format!(
            "        \"state_count\": {},\n",
            assembly.boundary.state_count
        ));
        json.push_str(&format!(
            "        \"algebraic_count\": {},\n",
            assembly.boundary.algebraic_count
        ));
        json.push_str(&format!(
            "        \"input_count\": {},\n",
            assembly.boundary.input_count
        ));
        json.push_str(&format!(
            "        \"output_count\": {},\n",
            assembly.boundary.output_count
        ));
        json.push_str(&format!(
            "        \"parameter_count\": {},\n",
            assembly.boundary.parameter_count
        ));
        json.push_str(&format!(
            "        \"equation_count\": {},\n",
            assembly.boundary.equation_count
        ));
        json.push_str(&format!(
            "        \"unknown_count\": {},\n",
            assembly.boundary.unknown_count
        ));
        json.push_str(&format!(
            "        \"balance_status\": \"{}\",\n",
            json_escape(&assembly.boundary.balance_status)
        ));
        match &assembly.boundary.diagnostic_code {
            Some(code) => json.push_str(&format!(
                "        \"diagnostic_code\": \"{}\"\n",
                json_escape(code)
            )),
            None => json.push_str("        \"diagnostic_code\": null\n"),
        }
        json.push_str("      },\n");
        json.push_str("      \"residual_graph\": {\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&assembly.residual_graph.name)
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&assembly.residual_graph.status)
        ));
        json.push_str(&format!(
            "        \"solver_plan\": \"{}\",\n",
            json_escape(&assembly.residual_graph.solver_plan)
        ));
        json.push_str("        \"residuals\": [");
        for (residual_index, residual) in assembly.residual_graph.residuals.iter().enumerate() {
            if residual_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(residual)));
        }
        json.push_str("],\n");
        json.push_str("        \"residual_metadata\": [\n");
        for (metadata_index, metadata) in
            assembly.residual_graph.residual_metadata.iter().enumerate()
        {
            if metadata_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"name\": \"{}\",\n",
                json_escape(&metadata.name)
            ));
            json.push_str(&format!(
                "            \"kind\": \"{}\",\n",
                json_escape(&metadata.kind)
            ));
            json.push_str(&format!(
                "            \"domain\": \"{}\",\n",
                json_escape(&metadata.domain)
            ));
            json.push_str(&format!(
                "            \"source_expression\": \"{}\",\n",
                json_escape(&metadata.source_expression)
            ));
            json.push_str(&format!(
                "            \"residual_expression\": \"{}\",\n",
                json_escape(&metadata.residual_expression)
            ));
            match &metadata.rhs {
                Some(rhs) => {
                    json.push_str(&format!("            \"rhs\": \"{}\",\n", json_escape(rhs)))
                }
                None => json.push_str("            \"rhs\": null,\n"),
            }
            json.push_str("            \"dependencies\": [");
            for (dependency_index, dependency) in metadata.dependencies.iter().enumerate() {
                if dependency_index > 0 {
                    json.push_str(", ");
                }
                json.push_str(&format!("\"{}\"", json_escape(dependency)));
            }
            json.push_str("],\n");
            json.push_str(&format!(
                "            \"unit\": \"{}\",\n",
                json_escape(&metadata.unit)
            ));
            json.push_str(&format!(
                "            \"expression_unit\": \"{}\",\n",
                json_escape(&metadata.expression_unit)
            ));
            json.push_str(&format!(
                "            \"expression_quantity_kind\": \"{}\",\n",
                json_escape(&metadata.expression_quantity_kind)
            ));
            json.push_str(&format!(
                "            \"scale_policy\": \"{}\",\n",
                json_escape(&metadata.scale_policy)
            ));
            json.push_str(&format!(
                "            \"lowering_status\": \"{}\",\n",
                json_escape(&metadata.lowering_status)
            ));
            push_optional_json_string(
                &mut json,
                "lowering_failure_code",
                metadata.lowering_failure_code.as_deref(),
                12,
            );
            push_optional_json_string(
                &mut json,
                "lowering_failure_reason",
                metadata.lowering_failure_reason.as_deref(),
                12,
            );
            json.push_str(&format!(
                "            \"status\": \"{}\",\n",
                json_escape(&metadata.status)
            ));
            json.push_str(&format!("            \"line\": {}\n", metadata.line));
            json.push_str("          }");
        }
        json.push_str("\n        ],\n");
        json.push_str("        \"dependencies\": [\n");
        for (dependency_index, dependency) in
            assembly.residual_graph.dependencies.iter().enumerate()
        {
            if dependency_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"residual\": \"{}\",\n",
                json_escape(&dependency.residual)
            ));
            json.push_str(&format!(
                "            \"variable\": \"{}\"\n",
                json_escape(&dependency.variable)
            ));
            json.push_str("          }");
        }
        json.push_str("\n        ],\n");
        json.push_str("        \"algebraic_loops\": [\n");
        for (loop_index, algebraic_loop) in
            assembly.residual_graph.algebraic_loops.iter().enumerate()
        {
            if loop_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          [");
            for (variable_index, variable) in algebraic_loop.iter().enumerate() {
                if variable_index > 0 {
                    json.push_str(", ");
                }
                json.push_str(&format!("\"{}\"", json_escape(variable)));
            }
            json.push(']');
        }
        json.push_str("\n        ],\n");
        json.push_str("        \"jacobian_sparsity\": [\n");
        for (seed_index, seed) in assembly.residual_graph.jacobian_sparsity.iter().enumerate() {
            if seed_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"residual\": \"{}\",\n",
                json_escape(&seed.residual)
            ));
            json.push_str(&format!(
                "            \"status\": \"{}\",\n",
                json_escape(&seed.status)
            ));
            json.push_str("            \"with_respect_to\": [");
            for (variable_index, variable) in seed.with_respect_to.iter().enumerate() {
                if variable_index > 0 {
                    json.push_str(", ");
                }
                json.push_str(&format!("\"{}\"", json_escape(variable)));
            }
            json.push_str("]\n");
            json.push_str("          }");
        }
        json.push_str("\n        ]\n");
        json.push_str("      },\n");
        push_report_component_solver_result_json(&mut json, &assembly.solver_result, "      ");
        json.push('\n');
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    write_report_component_graph_json(&mut json, &spec.component_graph);
    json.push_str(",\n");
    json.push_str("  \"class_summary\": [\n");
    for (index, class_info) in spec.classes.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&class_info.name)
        ));
        json.push_str(&format!("      \"line\": {},\n", class_info.line));
        json.push_str(&format!(
            "      \"field_count\": {},\n",
            class_info.fields.len()
        ));
        json.push_str(&format!(
            "      \"validation_count\": {},\n",
            class_info.validations.len()
        ));
        json.push_str(&format!(
            "      \"method_count\": {},\n",
            class_info.methods.len()
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&class_info.status)
        ));
        json.push_str("      \"fields\": [\n");
        for (field_index, field) in class_info.fields.iter().enumerate() {
            if field_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&field.name)
            ));
            json.push_str(&format!(
                "          \"type_name\": \"{}\",\n",
                json_escape(&field.type_name)
            ));
            json.push_str(&format!(
                "          \"quantity_kind\": \"{}\",\n",
                json_escape(&field.quantity_kind)
            ));
            json.push_str(&format!(
                "          \"display_unit\": \"{}\",\n",
                json_escape(&field.display_unit)
            ));
            json.push_str(&format!(
                "          \"canonical_unit\": \"{}\",\n",
                json_escape(&field.canonical_unit)
            ));
            json.push_str(&format!(
                "          \"dimension\": \"{}\",\n",
                json_escape(&field.dimension)
            ));
            match &field.default_value {
                Some(default_value) => json.push_str(&format!(
                    "          \"default\": \"{}\",\n",
                    json_escape(default_value)
                )),
                None => json.push_str("          \"default\": null,\n"),
            }
            json.push_str(&format!("          \"required\": {},\n", field.required));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&field.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", field.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"validations\": [\n");
        for (validation_index, validation) in class_info.validations.iter().enumerate() {
            if validation_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&validation.expression)
            ));
            json.push_str(&format!(
                "          \"left\": \"{}\",\n",
                json_escape(&validation.left)
            ));
            json.push_str(&format!(
                "          \"operator\": \"{}\",\n",
                json_escape(&validation.operator)
            ));
            json.push_str(&format!(
                "          \"right\": \"{}\",\n",
                json_escape(&validation.right)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&validation.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", validation.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"methods\": [\n");
        for (method_index, method) in class_info.methods.iter().enumerate() {
            if method_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&method.name)
            ));
            json.push_str(&format!(
                "          \"return_type\": \"{}\",\n",
                json_escape(&method.return_type)
            ));
            json.push_str(&format!(
                "          \"return_quantity_kind\": \"{}\",\n",
                json_escape(&method.return_quantity_kind)
            ));
            json.push_str(&format!(
                "          \"return_display_unit\": \"{}\",\n",
                json_escape(&method.return_display_unit)
            ));
            json.push_str(&format!(
                "          \"return_canonical_unit\": \"{}\",\n",
                json_escape(&method.return_canonical_unit)
            ));
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&method.expression)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&method.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", method.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"object_summary\": [\n");
    for (index, object) in spec.class_objects.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&object.name)
        ));
        json.push_str(&format!(
            "      \"class_name\": \"{}\",\n",
            json_escape(&object.class_name)
        ));
        match &object.source_object {
            Some(source_object) => json.push_str(&format!(
                "      \"source_object\": \"{}\",\n",
                json_escape(source_object)
            )),
            None => json.push_str("      \"source_object\": null,\n"),
        }
        json.push_str(&format!(
            "      \"construction\": \"{}\",\n",
            json_escape(&object.construction)
        ));
        json.push_str(&format!("      \"line\": {},\n", object.line));
        json.push_str(&format!(
            "      \"field_count\": {},\n",
            object.fields.len()
        ));
        json.push_str(&format!(
            "      \"validation_count\": {},\n",
            object.validations.len()
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&object.status)
        ));
        json.push_str("      \"fields\": [\n");
        for (field_index, field) in object.fields.iter().enumerate() {
            if field_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&field.name)
            ));
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&field.expression)
            ));
            json.push_str(&format!(
                "          \"quantity_kind\": \"{}\",\n",
                json_escape(&field.quantity_kind)
            ));
            json.push_str(&format!(
                "          \"display_unit\": \"{}\",\n",
                json_escape(&field.display_unit)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&field.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", field.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"validations\": [\n");
        for (validation_index, validation) in object.validations.iter().enumerate() {
            if validation_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&validation.expression)
            ));
            json.push_str(&format!(
                "          \"left\": \"{}\",\n",
                json_escape(&validation.left)
            ));
            json.push_str(&format!(
                "          \"operator\": \"{}\",\n",
                json_escape(&validation.operator)
            ));
            json.push_str(&format!(
                "          \"right\": \"{}\",\n",
                json_escape(&validation.right)
            ));
            push_optional_json_string(
                &mut json,
                "left_value",
                validation.left_value.as_deref(),
                10,
            );
            push_optional_json_string(
                &mut json,
                "right_value",
                validation.right_value.as_deref(),
                10,
            );
            json.push_str(&format!(
                "          \"unit\": \"{}\",\n",
                json_escape(&validation.unit)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&validation.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", validation.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"system_summary\": [\n");
    for (index, system) in spec.systems.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&system.name)
        ));
        json.push_str(&format!("      \"line\": {},\n", system.line));
        json.push_str("      \"variables\": [\n");
        for (variable_index, variable) in system.variables.iter().enumerate() {
            if variable_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"role\": \"{}\",\n",
                json_escape(&variable.role)
            ));
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&variable.name)
            ));
            json.push_str(&format!(
                "          \"quantity_kind\": \"{}\",\n",
                json_escape(&variable.quantity_kind)
            ));
            json.push_str(&format!(
                "          \"display_unit\": \"{}\",\n",
                json_escape(&variable.display_unit)
            ));
            json.push_str(&format!(
                "          \"dimension\": \"{}\",\n",
                json_escape(&variable.dimension)
            ));
            if let Some(initial_value) = &variable.initial_value {
                json.push_str(&format!(
                    "          \"initial_value\": \"{}\",\n",
                    json_escape(initial_value)
                ));
            } else {
                json.push_str("          \"initial_value\": null,\n");
            }
            json.push_str(&format!("          \"line\": {}\n", variable.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"equations\": [\n");
        for (equation_index, equation) in system.equations.iter().enumerate() {
            if equation_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"left\": \"{}\",\n",
                json_escape(&equation.left)
            ));
            json.push_str(&format!(
                "          \"relation\": \"{}\",\n",
                json_escape(&equation.relation)
            ));
            json.push_str(&format!(
                "          \"right\": \"{}\",\n",
                json_escape(&equation.right)
            ));
            json.push_str(&format!(
                "          \"left_dimension\": \"{}\",\n",
                json_escape(&equation.left_dimension)
            ));
            json.push_str(&format!(
                "          \"right_dimension\": \"{}\",\n",
                json_escape(&equation.right_dimension)
            ));
            json.push_str(&format!(
                "          \"residual\": \"{}\",\n",
                json_escape(&equation.residual)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&equation.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", equation.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"residuals\": [\n");
        for (residual_index, residual) in system.residuals.iter().enumerate() {
            if residual_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&residual.name)
            ));
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&residual.expression)
            ));
            json.push_str(&format!(
                "          \"dimension\": \"{}\",\n",
                json_escape(&residual.dimension)
            ));
            json.push_str(&format!("          \"line\": {}\n", residual.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"state_space_vectors\": [\n");
    for (index, vector) in spec.state_space_vectors.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"system\": \"{}\",\n",
            json_escape(&vector.system)
        ));
        json.push_str(&format!(
            "      \"role\": \"{}\",\n",
            json_escape(&vector.role)
        ));
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&vector.name)
        ));
        json.push_str(&format!(
            "      \"vector_type\": \"{}\",\n",
            json_escape(&vector.vector_type)
        ));
        json.push_str("      \"members\": [");
        for (member_index, member) in vector.members.iter().enumerate() {
            if member_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(member)));
        }
        json.push_str("],\n");
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&vector.status)
        ));
        json.push_str(&format!("      \"line\": {}\n", vector.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"linear_operators\": [\n");
    for (index, operator) in spec.linear_operators.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"system\": \"{}\",\n",
            json_escape(&operator.system)
        ));
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&operator.name)
        ));
        json.push_str(&format!(
            "      \"from\": \"{}\",\n",
            json_escape(&operator.from)
        ));
        json.push_str(&format!(
            "      \"to\": \"{}\",\n",
            json_escape(&operator.to)
        ));
        if let Some(expression) = &operator.expression {
            json.push_str(&format!(
                "      \"expression\": \"{}\",\n",
                json_escape(expression)
            ));
        } else {
            json.push_str("      \"expression\": null,\n");
        }
        json.push_str("      \"canonical_matrix\": ");
        push_optional_json_matrix(&mut json, operator.canonical_matrix.as_deref());
        json.push_str(",\n");
        push_linear_operator_entries_json(&mut json, &operator.canonical_entries, 6);
        json.push_str(&format!("      \"row_count\": {},\n", operator.row_count));
        json.push_str(&format!(
            "      \"column_count\": {},\n",
            operator.column_count
        ));
        json.push_str("      \"row_members\": [");
        push_json_string_array(&mut json, &operator.row_members);
        json.push_str("],\n");
        json.push_str("      \"column_members\": [");
        push_json_string_array(&mut json, &operator.column_members);
        json.push_str("],\n");
        json.push_str("      \"row_quantity_kinds\": [");
        push_json_string_array(&mut json, &operator.row_quantity_kinds);
        json.push_str("],\n");
        json.push_str("      \"column_quantity_kinds\": [");
        push_json_string_array(&mut json, &operator.column_quantity_kinds);
        json.push_str("],\n");
        json.push_str("      \"row_units\": [");
        push_json_string_array(&mut json, &operator.row_units);
        json.push_str("],\n");
        json.push_str("      \"column_units\": [");
        push_json_string_array(&mut json, &operator.column_units);
        json.push_str("],\n");
        json.push_str(&format!(
            "      \"compatibility_status\": \"{}\",\n",
            json_escape(&operator.compatibility_status)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&operator.status)
        ));
        json.push_str(&format!("      \"line\": {}\n", operator.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"system_ir\": [\n");
    for (index, system) in spec.system_ir.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&system.name)
        ));
        json.push_str("      \"solver_boundary\": {\n");
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&system.solver_boundary.status)
        ));
        json.push_str(&format!(
            "        \"reason\": \"{}\",\n",
            json_escape(&system.solver_boundary.reason)
        ));
        json.push_str(&format!(
            "        \"parameter_count\": {},\n",
            system.solver_boundary.parameter_count
        ));
        json.push_str(&format!(
            "        \"state_count\": {},\n",
            system.solver_boundary.state_count
        ));
        json.push_str(&format!(
            "        \"input_count\": {},\n",
            system.solver_boundary.input_count
        ));
        json.push_str(&format!(
            "        \"equation_count\": {},\n",
            system.solver_boundary.equation_count
        ));
        json.push_str(&format!(
            "        \"residual_count\": {}\n",
            system.solver_boundary.residual_count
        ));
        json.push_str("      },\n");
        push_report_solver_plan_json(&mut json, &system.solver_plan, "      ");
        json.push_str(",\n");
        push_report_system_solutions_json(&mut json, &system.solver_results, "      ");
        json.push_str(",\n");
        json.push_str("      \"equations\": [\n");
        for (equation_index, equation) in system.equations.iter().enumerate() {
            if equation_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"residual\": \"{}\",\n",
                json_escape(&equation.residual)
            ));
            json.push_str(&format!(
                "          \"relation\": \"{}\",\n",
                json_escape(&equation.relation)
            ));
            json.push_str(&format!(
                "          \"normalized_residual\": \"{}\",\n",
                json_escape(&equation.normalized_residual)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&equation.status)
            ));
            json.push_str("          \"dependencies\": [\n");
            for (dependency_index, dependency) in equation.dependencies.iter().enumerate() {
                if dependency_index > 0 {
                    json.push_str(",\n");
                }
                json.push_str("            {\n");
                json.push_str(&format!(
                    "              \"name\": \"{}\",\n",
                    json_escape(&dependency.name)
                ));
                json.push_str(&format!(
                    "              \"role\": \"{}\",\n",
                    json_escape(&dependency.role)
                ));
                json.push_str(&format!(
                    "              \"quantity_kind\": \"{}\"\n",
                    json_escape(&dependency.quantity_kind)
                ));
                json.push_str("            }");
            }
            json.push_str("\n          ],\n");
            json.push_str("          \"derivative_states\": [");
            for (state_index, state) in equation.derivative_states.iter().enumerate() {
                if state_index > 0 {
                    json.push_str(", ");
                }
                json.push_str(&format!("\"{}\"", json_escape(state)));
            }
            json.push_str("],\n");
            json.push_str(&format!("          \"line\": {}\n", equation.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str(&format!("      \"line\": {}\n", system.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    push_report_kernel_plan_json(&mut json, &spec.kernel_plan);
    json.push_str(",\n");

    json.push_str("  \"plot_manifest\": {\n");
    json.push_str(&format!(
        "    \"path\": \"{}\",\n",
        json_escape(&spec.plot_manifest.path)
    ));
    json.push_str(&format!(
        "    \"hash\": \"{}\",\n",
        json_escape(&spec.plot_manifest.hash)
    ));
    json.push_str(&format!(
        "    \"format\": \"{}\",\n",
        json_escape(&spec.plot_manifest.format)
    ));
    json.push_str(&format!(
        "    \"plot_count\": {}\n",
        spec.plot_manifest.plot_count
    ));
    json.push_str("  },\n");

    json.push_str("  \"warning_list\": [\n");
    for (index, warning) in spec.warnings.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"code\": \"{}\",\n",
            json_escape(&warning.code)
        ));
        json.push_str(&format!(
            "      \"message\": \"{}\",\n",
            json_escape(&warning.message)
        ));
        if let Some(help) = &warning.help {
            json.push_str(&format!("      \"help\": \"{}\",\n", json_escape(help)));
        } else {
            json.push_str("      \"help\": null,\n");
        }
        json.push_str(&format!("      \"line\": {}\n", warning.line));
        json.push_str("    }");
    }
    json.push_str("\n  ]\n");
    json.push_str("}\n");
    json
}

pub fn plot_spec_from_report(report: &CheckReport) -> PlotSpec {
    let series_binding = report
        .semantic_program
        .typed_bindings
        .iter()
        .find_map(|binding| {
            time_series_quantity(&binding.semantic_type.quantity_kind).map(|(axis, quantity)| {
                (
                    binding.name.clone(),
                    axis,
                    quantity,
                    binding.semantic_type.display_unit.clone(),
                )
            })
        });

    let (name, axis, quantity, unit) = series_binding.unwrap_or_else(|| {
        (
            "sample".to_owned(),
            "Time".to_owned(),
            "Value".to_owned(),
            "unit".to_owned(),
        )
    });

    PlotSpec {
        title: if name == "sample" {
            "EngLang sample plot".to_owned()
        } else {
            format!("{name} over {axis}")
        },
        plot_type: "line".to_owned(),
        x_axis: PlotAxis {
            name: axis.clone(),
            label: axis,
            unit: "sample".to_owned(),
        },
        y_axis: PlotAxis {
            name: quantity.clone(),
            label: quantity,
            unit: unit.clone(),
        },
        series: vec![PlotSeries {
            name,
            quantity_kind: "TimeSeries".to_owned(),
            display_unit: unit,
            bins: Vec::new(),
            points: sample_points(),
            confidence_band: None,
        }],
    }
}

pub fn render_svg(title: &str) -> String {
    render_svg_from_spec(&default_plot_spec(title))
}

pub fn render_svg_from_spec(spec: &PlotSpec) -> String {
    let title = xml_escape(&spec.title);
    let x_label = xml_escape(&axis_label(&spec.x_axis));
    let y_label = xml_escape(&axis_label(&spec.y_axis));
    let series = spec.series.first();
    let series_points = series
        .map(|series| series.points.as_slice())
        .unwrap_or_default();
    let plot_body = match spec.plot_type.as_str() {
        "bar" => svg_rect_plot(series_points, "#0b6bcb", 0.68),
        "histogram" => svg_histogram_plot(series),
        "scatter" => svg_scatter_plot(series_points, "#0b6bcb"),
        _ => svg_line_series(&spec.series),
    };
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="720" height="320" viewBox="0 0 720 320" role="img" aria-label="{title}">
  <rect width="720" height="320" fill="#f7f8fb"/>
  <line x1="72" y1="250" x2="660" y2="250" stroke="#222" stroke-width="2"/>
  <line x1="72" y1="40" x2="72" y2="250" stroke="#222" stroke-width="2"/>
  {plot_body}
  <text x="72" y="26" font-family="Segoe UI, Arial, sans-serif" font-size="20" fill="#111">{title}</text>
  <text x="328" y="294" font-family="Segoe UI, Arial, sans-serif" font-size="14" fill="#333">{x_label}</text>
  <text x="18" y="156" transform="rotate(-90 18 156)" font-family="Segoe UI, Arial, sans-serif" font-size="14" fill="#333">{y_label}</text>
</svg>
"##
    )
}

pub fn plot_spec_json(spec: &PlotSpec) -> String {
    let mut series_json = String::new();
    for (index, series) in spec.series.iter().enumerate() {
        if index > 0 {
            series_json.push_str(",\n");
        }
        series_json.push_str("    {\n");
        series_json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&series.name)
        ));
        series_json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&series.quantity_kind)
        ));
        series_json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&series.display_unit)
        ));
        series_json.push_str(&format!(
            "      \"points\": [{}],\n",
            plot_points_json(&series.points)
        ));
        series_json.push_str(&format!(
            "      \"bins\": [{}]",
            plot_bins_json(&series.bins)
        ));
        if let Some(confidence_band) = &series.confidence_band {
            series_json.push_str(",\n");
            series_json.push_str(&format!(
                "      \"confidence_band\": {}\n",
                plot_confidence_band_json(confidence_band)
            ));
        } else {
            series_json.push('\n');
        }
        series_json.push_str("    }");
    }
    format!(
        "{{\n  \"format\": \"eng-plotspec-v1\",\n  \"plot_spec_version\": {PLOT_SPEC_VERSION},\n  \"plot_type\": \"{}\",\n  \"title\": \"{}\",\n  \"x_axis\": {{ \"name\": \"{}\", \"label\": \"{}\", \"unit\": \"{}\" }},\n  \"y_axis\": {{ \"name\": \"{}\", \"label\": \"{}\", \"unit\": \"{}\" }},\n  \"series\": [\n{}\n  ]\n}}\n",
        json_escape(&spec.plot_type),
        json_escape(&spec.title),
        json_escape(&spec.x_axis.name),
        json_escape(&spec.x_axis.label),
        json_escape(&spec.x_axis.unit),
        json_escape(&spec.y_axis.name),
        json_escape(&spec.y_axis.label),
        json_escape(&spec.y_axis.unit),
        series_json
    )
}

fn plot_points_json(points: &[PlotPoint]) -> String {
    let mut json = String::new();
    for (index, point) in points.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!("[{}, {}]", point.x, point.y));
    }
    json
}

fn plot_bins_json(bins: &[PlotBin]) -> String {
    let mut json = String::new();
    for (index, bin) in bins.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!(
            "{{\"lower\": {}, \"upper\": {}, \"center\": {}, \"count\": {}}}",
            bin.lower, bin.upper, bin.center, bin.count
        ));
    }
    json
}

fn plot_confidence_band_json(band: &PlotConfidenceBand) -> String {
    format!(
        "{{\"method\": \"{}\", \"source\": \"{}\", \"level\": {}, \"lower\": [{}], \"upper\": [{}]}}",
        json_escape(&band.method),
        json_escape(&band.source),
        band.level,
        plot_points_json(&band.lower),
        plot_points_json(&band.upper)
    )
}

pub fn plot_manifest_json(
    spec: &PlotSpec,
    svg_relative_path: &str,
    plot_spec_hash: &str,
    svg_hash: &str,
) -> String {
    let series_names = spec
        .series
        .iter()
        .map(|series| series.name.clone())
        .collect::<Vec<_>>();
    let mut series_json = String::new();
    push_json_string_array(&mut series_json, &series_names);
    format!(
        "{{\n  \"format\": \"eng-plot-manifest-v1\",\n  \"plot_spec_version\": {PLOT_SPEC_VERSION},\n  \"plots\": [\n    {{\n      \"title\": \"{}\",\n      \"plot_type\": \"{}\",\n      \"plot_spec\": \"plot_spec.json\",\n      \"plot_spec_hash\": \"{}\",\n      \"svg\": \"{}\",\n      \"svg_hash\": \"{}\",\n      \"x_axis_label\": \"{}\",\n      \"y_axis_label\": \"{}\",\n      \"series\": [{}]\n    }}\n  ]\n}}\n",
        json_escape(&spec.title),
        json_escape(&spec.plot_type),
        json_escape(plot_spec_hash),
        json_escape(svg_relative_path),
        json_escape(svg_hash),
        json_escape(&axis_label(&spec.x_axis)),
        json_escape(&axis_label(&spec.y_axis)),
        series_json
    )
}

fn push_report_solver_plan_json(json: &mut String, plan: &ReportSolverPlan, indent: &str) {
    json.push_str(&format!("{indent}\"solver_plan\": {{\n"));
    json.push_str(&format!(
        "{indent}  \"status\": \"{}\",\n",
        json_escape(&plan.status)
    ));
    json.push_str(&format!(
        "{indent}  \"method\": \"{}\",\n",
        json_escape(&plan.method)
    ));
    json.push_str(&format!("{indent}  \"solve_order\": ["));
    for (index, residual) in plan.solve_order.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!("\"{}\"", json_escape(residual)));
    }
    json.push_str("],\n");
    json.push_str(&format!("{indent}  \"ode_runner\": {{\n"));
    json.push_str(&format!(
        "{indent}    \"status\": \"{}\",\n",
        json_escape(&plan.ode_runner.status)
    ));
    json.push_str(&format!(
        "{indent}    \"reason\": \"{}\"\n",
        json_escape(&plan.ode_runner.reason)
    ));
    json.push_str(&format!("{indent}  }},\n"));
    push_solver_plan_jacobian_entries_json(
        json,
        "jacobian_sparsity",
        &plan.jacobian_sparsity,
        indent,
        true,
    );
    push_solver_plan_jacobian_entries_json(
        json,
        "jacobian_seed",
        &plan.jacobian_seed,
        indent,
        false,
    );
    json.push_str(&format!("{indent}}}"));
}

fn push_solver_plan_jacobian_entries_json(
    json: &mut String,
    field_name: &str,
    entries: &[ReportJacobianSeed],
    indent: &str,
    trailing_comma: bool,
) {
    json.push_str(&format!("{indent}  \"{field_name}\": [\n"));
    for (entry_index, entry) in entries.iter().enumerate() {
        if entry_index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}    {{\n"));
        json.push_str(&format!(
            "{indent}      \"residual\": \"{}\",\n",
            json_escape(&entry.residual)
        ));
        json.push_str(&format!("{indent}      \"with_respect_to\": ["));
        for (variable_index, variable) in entry.with_respect_to.iter().enumerate() {
            if variable_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(variable)));
        }
        json.push_str("],\n");
        json.push_str(&format!("{indent}      \"derivative_states\": ["));
        for (state_index, state) in entry.derivative_states.iter().enumerate() {
            if state_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(state)));
        }
        json.push_str("],\n");
        json.push_str(&format!(
            "{indent}      \"status\": \"{}\"\n",
            json_escape(&entry.status)
        ));
        json.push_str(&format!("{indent}    }}"));
    }
    json.push_str(&format!("\n{indent}  ]"));
    if trailing_comma {
        json.push(',');
    }
    json.push('\n');
}

fn push_report_component_solver_result_json(
    json: &mut String,
    result: &Option<ReportComponentSolverResult>,
    indent: &str,
) {
    json.push_str(&format!("{indent}\"solver_result\": "));
    let Some(result) = result else {
        json.push_str("null");
        return;
    };

    json.push_str("{\n");
    json.push_str(&format!(
        "{indent}  \"status\": \"{}\",\n",
        json_escape(&result.status)
    ));
    json.push_str(&format!(
        "{indent}  \"method\": \"{}\",\n",
        json_escape(&result.method)
    ));
    json.push_str(&format!(
        "{indent}  \"reason\": \"{}\",\n",
        json_escape(&result.reason)
    ));
    json.push_str(&format!(
        "{indent}  \"residual_norm\": {},\n",
        result.residual_norm
    ));
    push_optional_json_f64(
        json,
        "linear_condition_estimate",
        result.linear_condition_estimate,
        indent.len() + 2,
    );
    push_optional_json_f64(
        json,
        "linear_minimum_pivot_abs",
        result.linear_minimum_pivot_abs,
        indent.len() + 2,
    );
    push_optional_json_f64(
        json,
        "linear_maximum_pivot_abs",
        result.linear_maximum_pivot_abs,
        indent.len() + 2,
    );
    json.push_str(&format!(
        "{indent}  \"variable_scale_policy\": \"{}\",\n",
        json_escape(&result.variable_scale_policy)
    ));
    push_optional_json_f64(
        json,
        "variable_scale_min",
        result.variable_scale_min,
        indent.len() + 2,
    );
    push_optional_json_f64(
        json,
        "variable_scale_max",
        result.variable_scale_max,
        indent.len() + 2,
    );
    json.push_str(&format!("{indent}  \"tolerance\": {},\n", result.tolerance));
    json.push_str(&format!(
        "{indent}  \"max_iterations\": {},\n",
        result.max_iterations
    ));
    json.push_str(&format!(
        "{indent}  \"iteration_count\": {},\n",
        result.iteration_count
    ));
    json.push_str(&format!(
        "{indent}  \"convergence_status\": \"{}\",\n",
        json_escape(&result.convergence_status)
    ));
    push_optional_json_string(
        json,
        "failure_code",
        result
            .failure_artifact
            .as_ref()
            .map(|failure| failure.code.as_str()),
        indent.len() + 2,
    );
    push_optional_json_string(
        json,
        "failure_reason",
        result
            .failure_artifact
            .as_ref()
            .map(|failure| failure.message.as_str()),
        indent.len() + 2,
    );
    json.push_str(&format!("{indent}  \"variables\": [\n"));
    for (index, variable) in result.variables.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}    {{\n"));
        json.push_str(&format!(
            "{indent}      \"name\": \"{}\",\n",
            json_escape(&variable.name)
        ));
        json.push_str(&format!(
            "{indent}      \"role\": \"{}\",\n",
            json_escape(&variable.role)
        ));
        json.push_str(&format!("{indent}      \"value\": {},\n", variable.value));
        json.push_str(&format!(
            "{indent}      \"unit\": \"{}\",\n",
            json_escape(&variable.unit)
        ));
        json.push_str(&format!(
            "{indent}      \"status\": \"{}\"\n",
            json_escape(&variable.status)
        ));
        json.push_str(&format!("{indent}    }}"));
    }
    json.push_str(&format!("\n{indent}  ],\n"));
    json.push_str(&format!("{indent}  \"trajectories\": [\n"));
    for (index, trajectory) in result.trajectories.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}    {{\n"));
        json.push_str(&format!(
            "{indent}      \"name\": \"{}\",\n",
            json_escape(&trajectory.name)
        ));
        json.push_str(&format!(
            "{indent}      \"role\": \"{}\",\n",
            json_escape(&trajectory.role)
        ));
        json.push_str(&format!(
            "{indent}      \"quantity_kind\": \"{}\",\n",
            json_escape(&trajectory.quantity_kind)
        ));
        json.push_str(&format!(
            "{indent}      \"unit\": \"{}\",\n",
            json_escape(&trajectory.unit)
        ));
        json.push_str(&format!(
            "{indent}      \"initial_value\": {},\n",
            trajectory.initial_value
        ));
        json.push_str(&format!(
            "{indent}      \"final_value\": {},\n",
            trajectory.final_value
        ));
        json.push_str(&format!(
            "{indent}      \"point_count\": {},\n",
            trajectory.point_count
        ));
        json.push_str(&format!("{indent}      \"points\": [\n"));
        for (point_index, point) in trajectory.points.iter().enumerate() {
            if point_index > 0 {
                json.push_str(",\n");
            }
            json.push_str(&format!("{indent}        {{\n"));
            json.push_str(&format!("{indent}          \"x\": {},\n", point.x));
            json.push_str(&format!("{indent}          \"y\": {}\n", point.y));
            json.push_str(&format!("{indent}        }}"));
        }
        json.push_str(&format!("\n{indent}      ]\n"));
        json.push_str(&format!("{indent}    }}"));
    }
    json.push_str(&format!("\n{indent}  ],\n"));
    json.push_str(&format!("{indent}  \"step_diagnostics\": [\n"));
    for (index, diagnostic) in result.step_diagnostics.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}    {{\n"));
        json.push_str(&format!(
            "{indent}      \"step_index\": {},\n",
            diagnostic.step_index
        ));
        json.push_str(&format!(
            "{indent}      \"time_s\": {},\n",
            diagnostic.time_s
        ));
        json.push_str(&format!(
            "{indent}      \"algebraic_iteration_count\": {},\n",
            diagnostic.algebraic_iteration_count
        ));
        json.push_str(&format!(
            "{indent}      \"residual_norm\": {},\n",
            diagnostic.residual_norm
        ));
        json.push_str(&format!("{indent}      \"residual_values\": ["));
        push_json_f64_array(json, &diagnostic.residual_values);
        json.push_str("],\n");
        json.push_str(&format!("{indent}      \"normalized_residual_values\": ["));
        push_json_f64_array(json, &diagnostic.normalized_residual_values);
        json.push_str("],\n");
        match diagnostic.line_search_scale {
            Some(scale) => json.push_str(&format!(
                "{indent}      \"line_search_scale\": {},\n",
                scale
            )),
            None => json.push_str(&format!("{indent}      \"line_search_scale\": null,\n")),
        }
        match diagnostic.line_search_trial_count {
            Some(count) => json.push_str(&format!(
                "{indent}      \"line_search_trial_count\": {},\n",
                count
            )),
            None => json.push_str(&format!(
                "{indent}      \"line_search_trial_count\": null,\n"
            )),
        }
        push_optional_json_string(
            json,
            "jacobian_policy",
            diagnostic.jacobian_policy.as_deref(),
            indent.len() + 6,
        );
        push_optional_json_string(
            json,
            "variable_scale_policy",
            diagnostic.variable_scale_policy.as_deref(),
            indent.len() + 6,
        );
        push_optional_json_f64(
            json,
            "linear_condition_estimate",
            diagnostic.linear_condition_estimate,
            indent.len() + 6,
        );
        push_optional_json_f64(
            json,
            "linear_minimum_pivot_abs",
            diagnostic.linear_minimum_pivot_abs,
            indent.len() + 6,
        );
        push_optional_json_f64(
            json,
            "linear_maximum_pivot_abs",
            diagnostic.linear_maximum_pivot_abs,
            indent.len() + 6,
        );
        push_optional_json_usize(
            json,
            "largest_residual_index",
            diagnostic.largest_residual_index,
            indent.len() + 6,
        );
        push_optional_json_string(
            json,
            "largest_residual_name",
            diagnostic.largest_residual_name.as_deref(),
            indent.len() + 6,
        );
        push_optional_json_string(
            json,
            "largest_residual_source_expression",
            diagnostic.largest_residual_source_expression.as_deref(),
            indent.len() + 6,
        );
        push_optional_json_usize(
            json,
            "largest_residual_source_line",
            diagnostic.largest_residual_source_line,
            indent.len() + 6,
        );
        push_optional_json_string(
            json,
            "largest_residual_source_reason",
            diagnostic.largest_residual_source_reason.as_deref(),
            indent.len() + 6,
        );
        push_optional_json_f64(
            json,
            "largest_residual_value",
            diagnostic.largest_residual_value,
            indent.len() + 6,
        );
        push_optional_json_f64(
            json,
            "largest_residual_abs_value",
            diagnostic.largest_residual_abs_value,
            indent.len() + 6,
        );
        json.push_str(&format!(
            "{indent}      \"convergence_status\": \"{}\",\n",
            json_escape(&diagnostic.convergence_status)
        ));
        push_optional_json_string(
            json,
            "failure_code",
            diagnostic
                .failure_artifact
                .as_ref()
                .map(|failure| failure.code.as_str()),
            indent.len() + 6,
        );
        push_optional_json_string(
            json,
            "failure_reason",
            diagnostic
                .failure_artifact
                .as_ref()
                .map(|failure| failure.message.as_str()),
            indent.len() + 6,
        );
        match &diagnostic.failure_artifact {
            Some(failure) => {
                json.push_str(&format!("{indent}      \"failure_artifact\": {{\n"));
                json.push_str(&format!(
                    "{indent}        \"code\": \"{}\",\n",
                    json_escape(&failure.code)
                ));
                json.push_str(&format!(
                    "{indent}        \"message\": \"{}\"\n",
                    json_escape(&failure.message)
                ));
                json.push_str(&format!("{indent}      }}\n"));
            }
            None => json.push_str(&format!("{indent}      \"failure_artifact\": null\n")),
        }
        json.push_str(&format!("{indent}    }}"));
    }
    json.push_str(&format!("\n{indent}  ],\n"));
    json.push_str(&format!("{indent}  \"residuals\": [\n"));
    for (index, residual) in result.residuals.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}    {{\n"));
        json.push_str(&format!(
            "{indent}      \"name\": \"{}\",\n",
            json_escape(&residual.name)
        ));
        json.push_str(&format!(
            "{indent}      \"expression\": \"{}\",\n",
            json_escape(&residual.expression)
        ));
        json.push_str(&format!(
            "{indent}      \"source_expression\": \"{}\",\n",
            json_escape(&residual.source_expression)
        ));
        push_optional_json_usize(json, "source_line", residual.source_line, indent.len() + 6);
        push_optional_json_string(
            json,
            "source_reason",
            residual.source_reason.as_deref(),
            indent.len() + 6,
        );
        json.push_str(&format!("{indent}      \"dependencies\": ["));
        for (dependency_index, dependency) in residual.dependencies.iter().enumerate() {
            if dependency_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(dependency)));
        }
        json.push_str("],\n");
        json.push_str(&format!("{indent}      \"value\": {},\n", residual.value));
        json.push_str(&format!(
            "{indent}      \"unit\": \"{}\",\n",
            json_escape(&residual.unit)
        ));
        json.push_str(&format!(
            "{indent}      \"expression_unit\": \"{}\",\n",
            json_escape(&residual.expression_unit)
        ));
        json.push_str(&format!(
            "{indent}      \"expression_quantity_kind\": \"{}\",\n",
            json_escape(&residual.expression_quantity_kind)
        ));
        json.push_str(&format!(
            "{indent}      \"normalized_value\": {},\n",
            residual.normalized_value
        ));
        json.push_str(&format!("{indent}      \"scale\": {},\n", residual.scale));
        json.push_str(&format!(
            "{indent}      \"scale_policy\": \"{}\",\n",
            json_escape(&residual.scale_policy)
        ));
        json.push_str(&format!(
            "{indent}      \"lowering_status\": \"{}\",\n",
            json_escape(&residual.lowering_status)
        ));
        push_optional_json_string(
            json,
            "lowering_failure_code",
            residual.lowering_failure_code.as_deref(),
            indent.len() + 6,
        );
        push_optional_json_string(
            json,
            "lowering_failure_reason",
            residual.lowering_failure_reason.as_deref(),
            indent.len() + 6,
        );
        json.push_str(&format!(
            "{indent}      \"status\": \"{}\"\n",
            json_escape(&residual.status)
        ));
        json.push_str(&format!("{indent}    }}"));
    }
    json.push_str(&format!("\n{indent}  ],\n"));
    json.push_str(&format!("{indent}  \"largest_residuals\": [\n"));
    for (index, residual) in result.largest_residuals.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}    {{\n"));
        json.push_str(&format!(
            "{indent}      \"name\": \"{}\",\n",
            json_escape(&residual.name)
        ));
        json.push_str(&format!(
            "{indent}      \"expression\": \"{}\",\n",
            json_escape(&residual.expression)
        ));
        json.push_str(&format!(
            "{indent}      \"source_expression\": \"{}\",\n",
            json_escape(&residual.source_expression)
        ));
        push_optional_json_usize(json, "source_line", residual.source_line, indent.len() + 6);
        push_optional_json_string(
            json,
            "source_reason",
            residual.source_reason.as_deref(),
            indent.len() + 6,
        );
        json.push_str(&format!("{indent}      \"dependencies\": ["));
        for (dependency_index, dependency) in residual.dependencies.iter().enumerate() {
            if dependency_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(dependency)));
        }
        json.push_str("],\n");
        json.push_str(&format!("{indent}      \"value\": {},\n", residual.value));
        json.push_str(&format!(
            "{indent}      \"unit\": \"{}\",\n",
            json_escape(&residual.unit)
        ));
        json.push_str(&format!(
            "{indent}      \"expression_unit\": \"{}\",\n",
            json_escape(&residual.expression_unit)
        ));
        json.push_str(&format!(
            "{indent}      \"expression_quantity_kind\": \"{}\",\n",
            json_escape(&residual.expression_quantity_kind)
        ));
        json.push_str(&format!(
            "{indent}      \"normalized_value\": {},\n",
            residual.normalized_value
        ));
        json.push_str(&format!("{indent}      \"scale\": {},\n", residual.scale));
        json.push_str(&format!(
            "{indent}      \"scale_policy\": \"{}\",\n",
            json_escape(&residual.scale_policy)
        ));
        json.push_str(&format!(
            "{indent}      \"lowering_status\": \"{}\",\n",
            json_escape(&residual.lowering_status)
        ));
        push_optional_json_string(
            json,
            "lowering_failure_code",
            residual.lowering_failure_code.as_deref(),
            indent.len() + 6,
        );
        push_optional_json_string(
            json,
            "lowering_failure_reason",
            residual.lowering_failure_reason.as_deref(),
            indent.len() + 6,
        );
        json.push_str(&format!(
            "{indent}      \"status\": \"{}\"\n",
            json_escape(&residual.status)
        ));
        json.push_str(&format!("{indent}    }}"));
    }
    json.push_str(&format!("\n{indent}  ],\n"));
    match &result.failure_artifact {
        Some(failure) => {
            json.push_str(&format!("{indent}  \"failure_artifact\": {{\n"));
            json.push_str(&format!(
                "{indent}    \"code\": \"{}\",\n",
                json_escape(&failure.code)
            ));
            json.push_str(&format!(
                "{indent}    \"message\": \"{}\"\n",
                json_escape(&failure.message)
            ));
            json.push_str(&format!("{indent}  }}\n"));
        }
        None => json.push_str(&format!("{indent}  \"failure_artifact\": null\n")),
    }
    json.push_str(&format!("{indent}}}"));
}

fn push_report_system_solutions_json(
    json: &mut String,
    solutions: &[ReportSystemSolution],
    indent: &str,
) {
    json.push_str(&format!("{indent}\"solver_results\": [\n"));
    for (solution_index, solution) in solutions.iter().enumerate() {
        if solution_index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}  {{\n"));
        if let Some(binding) = &solution.binding {
            json.push_str(&format!(
                "{indent}    \"binding\": \"{}\",\n",
                json_escape(binding)
            ));
        } else {
            json.push_str(&format!("{indent}    \"binding\": null,\n"));
        }
        json.push_str(&format!(
            "{indent}    \"status\": \"{}\",\n",
            json_escape(&solution.status)
        ));
        json.push_str(&format!(
            "{indent}    \"method\": \"{}\",\n",
            json_escape(&solution.method)
        ));
        json.push_str(&format!(
            "{indent}    \"reason\": \"{}\",\n",
            json_escape(&solution.reason)
        ));
        json.push_str(&format!("{indent}    \"states\": ["));
        push_json_string_array(json, &solution.states);
        json.push_str("],\n");
        json.push_str(&format!("{indent}    \"algebraic_variables\": ["));
        push_json_string_array(json, &solution.algebraic_variables);
        json.push_str("],\n");
        json.push_str(&format!("{indent}    \"inputs\": ["));
        push_json_string_array(json, &solution.inputs);
        json.push_str("],\n");
        json.push_str(&format!("{indent}    \"parameters\": ["));
        push_json_string_array(json, &solution.parameters);
        json.push_str("],\n");
        json.push_str(&format!("{indent}    \"outputs\": ["));
        push_json_string_array(json, &solution.outputs);
        json.push_str("],\n");
        push_report_system_source_equations_json(json, &solution.source_equations, indent);
        json.push_str(&format!(
            "{indent}    \"state\": \"{}\",\n",
            json_escape(&solution.state)
        ));
        json.push_str(&format!(
            "{indent}    \"quantity_kind\": \"{}\",\n",
            json_escape(&solution.quantity_kind)
        ));
        json.push_str(&format!(
            "{indent}    \"display_unit\": \"{}\",\n",
            json_escape(&solution.display_unit)
        ));
        json.push_str(&format!(
            "{indent}    \"canonical_unit\": \"{}\",\n",
            json_escape(&solution.canonical_unit)
        ));
        json.push_str(&format!(
            "{indent}    \"time_unit\": \"{}\",\n",
            json_escape(&solution.time_unit)
        ));
        json.push_str(&format!(
            "{indent}    \"duration_s\": {},\n",
            solution.duration_s
        ));
        json.push_str(&format!(
            "{indent}    \"time_step_s\": {},\n",
            solution.time_step_s
        ));
        json.push_str(&format!(
            "{indent}    \"step_count\": {},\n",
            solution.step_count
        ));
        json.push_str(&format!(
            "{indent}    \"tolerance\": {},\n",
            solution.tolerance
        ));
        json.push_str(&format!(
            "{indent}    \"max_iterations\": {},\n",
            solution.max_iterations
        ));
        json.push_str(&format!(
            "{indent}    \"iteration_count\": {},\n",
            solution.iteration_count
        ));
        json.push_str(&format!(
            "{indent}    \"convergence_status\": \"{}\",\n",
            json_escape(&solution.convergence_status)
        ));
        push_optional_json_string(
            json,
            "failure_code",
            solution.failure_code.as_deref(),
            indent.len() + 4,
        );
        push_optional_json_string(
            json,
            "failure_reason",
            solution.failure_reason.as_deref(),
            indent.len() + 4,
        );
        json.push_str(&format!(
            "{indent}    \"initial_value\": {},\n",
            solution.initial_value
        ));
        json.push_str(&format!(
            "{indent}    \"final_value\": {},\n",
            solution.final_value
        ));
        json.push_str(&format!(
            "{indent}    \"canonical_initial_value\": {},\n",
            solution.canonical_initial_value
        ));
        json.push_str(&format!(
            "{indent}    \"canonical_final_value\": {},\n",
            solution.canonical_final_value
        ));
        push_report_system_step_diagnostics_json(json, &solution.step_diagnostics, indent);
        json.push_str(&format!("{indent}    \"points\": [\n"));
        for (point_index, point) in solution.points.iter().enumerate() {
            if point_index > 0 {
                json.push_str(",\n");
            }
            json.push_str(&format!(
                "{indent}      {{ \"x\": {}, \"y\": {} }}",
                point.x, point.y
            ));
        }
        json.push_str(&format!("\n{indent}    ]\n"));
        json.push_str(&format!("{indent}  }}"));
    }
    json.push_str(&format!("\n{indent}]"));
}

fn push_report_system_source_equations_json(
    json: &mut String,
    equations: &[ReportSystemEquationMetadata],
    indent: &str,
) {
    json.push_str(&format!("{indent}    \"source_equations\": [\n"));
    for (equation_index, equation) in equations.iter().enumerate() {
        if equation_index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}      {{\n"));
        json.push_str(&format!(
            "{indent}        \"kind\": \"{}\",\n",
            json_escape(&equation.kind)
        ));
        json.push_str(&format!(
            "{indent}        \"target\": \"{}\",\n",
            json_escape(&equation.target)
        ));
        json.push_str(&format!(
            "{indent}        \"left\": \"{}\",\n",
            json_escape(&equation.left)
        ));
        json.push_str(&format!(
            "{indent}        \"right\": \"{}\",\n",
            json_escape(&equation.right)
        ));
        json.push_str(&format!(
            "{indent}        \"residual_expression\": \"{}\",\n",
            json_escape(&equation.residual_expression)
        ));
        json.push_str(&format!(
            "{indent}        \"quantity_kind\": \"{}\",\n",
            json_escape(&equation.quantity_kind)
        ));
        json.push_str(&format!(
            "{indent}        \"display_unit\": \"{}\",\n",
            json_escape(&equation.display_unit)
        ));
        json.push_str(&format!(
            "{indent}        \"canonical_unit\": \"{}\",\n",
            json_escape(&equation.canonical_unit)
        ));
        match equation.source_line {
            Some(line) => json.push_str(&format!("{indent}        \"source_line\": {}\n", line)),
            None => json.push_str(&format!("{indent}        \"source_line\": null\n")),
        }
        json.push_str(&format!("{indent}      }}"));
    }
    json.push_str(&format!("\n{indent}    ],\n"));
}

fn push_report_system_step_diagnostics_json(
    json: &mut String,
    diagnostics: &[ReportSystemSolverStepDiagnostic],
    indent: &str,
) {
    json.push_str(&format!("{indent}    \"step_diagnostics\": [\n"));
    for (diagnostic_index, diagnostic) in diagnostics.iter().enumerate() {
        if diagnostic_index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}      {{\n"));
        json.push_str(&format!(
            "{indent}        \"output_index\": {},\n",
            diagnostic.output_index
        ));
        json.push_str(&format!(
            "{indent}        \"start_time_s\": {},\n",
            diagnostic.start_time_s
        ));
        json.push_str(&format!(
            "{indent}        \"end_time_s\": {},\n",
            diagnostic.end_time_s
        ));
        json.push_str(&format!("{indent}        \"dt_s\": {},\n", diagnostic.dt_s));
        json.push_str(&format!(
            "{indent}        \"error_norm\": {},\n",
            diagnostic.error_norm
        ));
        json.push_str(&format!(
            "{indent}        \"status\": \"{}\"\n",
            json_escape(&diagnostic.status)
        ));
        json.push_str(&format!("{indent}      }}"));
    }
    json.push_str(&format!("\n{indent}    ],\n"));
}

fn push_report_kernel_plan_json(json: &mut String, plan: &ReportKernelPlan) {
    json.push_str("  \"kernel_plan\": {\n");
    json.push_str(&format!(
        "    \"format\": \"{}\",\n",
        json_escape(&plan.format)
    ));
    json.push_str(&format!(
        "    \"backend\": \"{}\",\n",
        json_escape(&plan.backend)
    ));
    json.push_str("    \"backend_selection\": {\n");
    json.push_str(&format!(
        "      \"requested\": \"{}\",\n",
        json_escape(&plan.backend_selection.requested)
    ));
    json.push_str(&format!(
        "      \"selected\": \"{}\",\n",
        json_escape(&plan.backend_selection.selected)
    ));
    json.push_str(&format!(
        "      \"status\": \"{}\",\n",
        json_escape(&plan.backend_selection.status)
    ));
    json.push_str(&format!(
        "      \"reason\": \"{}\"\n",
        json_escape(&plan.backend_selection.reason)
    ));
    json.push_str("    },\n");
    json.push_str(&format!(
        "    \"candidate_count\": {},\n",
        plan.candidate_count
    ));
    json.push_str("    \"candidates\": [\n");
    for (index, candidate) in plan.candidates.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&candidate.name)
        ));
        json.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&candidate.kind)
        ));
        json.push_str(&format!("        \"line\": {},\n", candidate.line));
        json.push_str(&format!(
            "        \"source\": \"{}\",\n",
            json_escape(&candidate.source)
        ));
        json.push_str(&format!(
            "        \"reason\": \"{}\",\n",
            json_escape(&candidate.reason)
        ));
        json.push_str(&format!(
            "        \"lowering_status\": \"{}\",\n",
            json_escape(&candidate.lowering_status)
        ));
        json.push_str("        \"operations\": [");
        push_json_string_array(json, &candidate.operations);
        json.push_str("],\n");
        json.push_str("        \"estimate\": {\n");
        if let Some(estimated_rows) = candidate.estimated_rows {
            json.push_str(&format!(
                "          \"estimated_rows\": {},\n",
                estimated_rows
            ));
        } else {
            json.push_str("          \"estimated_rows\": null,\n");
        }
        json.push_str(&format!(
            "          \"input_count\": {},\n",
            candidate.input_count
        ));
        json.push_str(&format!(
            "          \"output_count\": {},\n",
            candidate.output_count
        ));
        json.push_str(&format!(
            "          \"operation_count\": {},\n",
            candidate.operation_count
        ));
        json.push_str(&format!(
            "          \"scan_count\": {},\n",
            candidate.scan_count
        ));
        json.push_str(&format!(
            "          \"complexity\": \"{}\"\n",
            json_escape(&candidate.complexity)
        ));
        json.push_str("        },\n");
        json.push_str("        \"executor\": {\n");
        json.push_str(&format!(
            "          \"backend\": \"{}\",\n",
            json_escape(&candidate.executor_backend)
        ));
        json.push_str(&format!(
            "          \"status\": \"{}\",\n",
            json_escape(&candidate.executor_status)
        ));
        json.push_str(&format!(
            "          \"fallback_reason\": \"{}\"\n",
            json_escape(&candidate.fallback_reason)
        ));
        json.push_str("        }\n");
        json.push_str("      }");
    }
    json.push_str("\n    ]\n");
    json.push_str("  }");
}

pub fn render_html(report: &CheckReport, plot_relative_path: &str) -> String {
    render_html_inner(report, plot_relative_path, None, None)
}

pub fn render_html_with_spec(
    report: &CheckReport,
    plot_relative_path: &str,
    spec: &ReportSpec,
) -> String {
    render_html_inner(report, plot_relative_path, Some(spec), None)
}

pub fn render_html_with_spec_and_review_document(
    report: &CheckReport,
    plot_relative_path: &str,
    spec: &ReportSpec,
    review_input: &Value,
) -> Result<String, ReviewDocumentError> {
    let review_document = extract_review_document(review_input)?;
    Ok(render_html_inner(
        report,
        plot_relative_path,
        Some(spec),
        Some(review_document),
    ))
}

fn render_html_inner(
    report: &CheckReport,
    plot_relative_path: &str,
    spec: Option<&ReportSpec>,
    review_document: Option<&Value>,
) -> String {
    let title = html_escape(&format!(
        "EngLang Review - {}",
        report
            .source_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("source.eng")
    ));
    let mut diagnostics = String::new();
    for diagnostic in &report.diagnostics {
        diagnostics.push_str("<tr>");
        diagnostics.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            diagnostic.line,
            html_escape(diagnostic.severity.as_str()),
            html_escape(&diagnostic.code),
            html_escape(&diagnostic.message)
        ));
        diagnostics.push_str("</tr>");
    }
    if diagnostics.is_empty() {
        diagnostics.push_str("<tr><td colspan=\"4\">No diagnostics.</td></tr>");
    }

    let mut inferred = String::new();
    for declaration in &report.inferred_declarations {
        inferred.push_str("<tr>");
        inferred.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td>",
            declaration.line,
            html_escape(&declaration.name),
            html_escape(&declaration.quantity_kind),
            html_escape(&declaration.display_unit),
            html_escape(&declaration.expression)
        ));
        inferred.push_str("</tr>");
    }
    if inferred.is_empty() {
        inferred.push_str("<tr><td colspan=\"5\">No inferred local declarations.</td></tr>");
    }

    let mut hover_hints = String::new();
    for hover in &report.semantic_program.hover_hints {
        hover_hints.push_str("<tr>");
        hover_hints.push_str(&format!(
            "<td>{}:{}</td><td>{}</td><td>{}</td><td>{}</td>",
            hover.line,
            hover.column,
            html_escape(&hover.name),
            html_escape(&hover.quantity_kind),
            html_escape(&hover.detail)
        ));
        hover_hints.push_str("</tr>");
    }
    if hover_hints.is_empty() {
        hover_hints.push_str("<tr><td colspan=\"4\">No hover hints.</td></tr>");
    }

    let mut type_info = String::new();
    for info in &report.semantic_program.type_infos {
        type_info.push_str("<tr>");
        type_info.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            info.line,
            html_escape(&info.name),
            html_escape(&info.quantity_kind),
            html_escape(&info.display_unit),
            html_escape(&info.canonical_unit),
            html_escape(&info.dimension)
        ));
        type_info.push_str("</tr>");
    }
    if type_info.is_empty() {
        type_info.push_str("<tr><td colspan=\"6\">No type info.</td></tr>");
    }

    let mut unit_derivations = String::new();
    for derivation in &report.semantic_program.unit_derivations {
        unit_derivations.push_str("<tr>");
        unit_derivations.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            derivation.line,
            html_escape(&derivation.name),
            html_escape(derivation.source_unit.as_deref().unwrap_or("not detected")),
            html_escape(&derivation.display_unit),
            html_escape(&derivation.canonical_unit)
        ));
        unit_derivations.push_str("</tr>");
    }
    if unit_derivations.is_empty() {
        unit_derivations.push_str("<tr><td colspan=\"5\">No unit derivations.</td></tr>");
    }

    let mut axis_info = String::new();
    for axis in &report.semantic_program.axis_infos {
        axis_info.push_str("<tr>");
        axis_info.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            axis.line,
            html_escape(&axis.binding),
            html_escape(&axis.axis),
            html_escape(&axis.role),
            html_escape(&axis.source)
        ));
        axis_info.push_str("</tr>");
    }
    if axis_info.is_empty() {
        axis_info.push_str("<tr><td colspan=\"5\">No axis metadata.</td></tr>");
    }

    let mut stats_info = String::new();
    for stats in &report.semantic_program.stats_infos {
        stats_info.push_str("<tr>");
        stats_info.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            stats.line,
            html_escape(&stats.source),
            html_escape(&stats.quantity_kind),
            html_escape(&stats.axis),
            html_escape(&stats.statistics.join(", ")),
            html_escape(&stats.cache_key)
        ));
        stats_info.push_str("</tr>");
    }
    if stats_info.is_empty() {
        stats_info.push_str("<tr><td colspan=\"6\">No statistics summaries.</td></tr>");
    }

    let mut integrations = String::new();
    for integration in &report.semantic_program.integrations {
        integrations.push_str("<tr>");
        integrations.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            integration.line,
            html_escape(&integration.binding),
            html_escape(&integration.source),
            html_escape(&integration.input_quantity),
            html_escape(&integration.over_axis),
            html_escape(&integration.result_quantity)
        ));
        integrations.push_str("</tr>");
    }
    if integrations.is_empty() {
        integrations.push_str("<tr><td colspan=\"6\">No integrations.</td></tr>");
    }

    let mut uncertainty = String::new();
    for info in &report.semantic_program.uncertainty_infos {
        let transform = uncertainty_transform_label(
            info.scale.as_deref(),
            info.offset.as_deref(),
            info.error.as_deref(),
        );
        let propagation = uncertainty_propagation_label(&info.propagation);
        uncertainty.push_str("<tr>");
        uncertainty.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td>",
            info.line,
            html_escape(&info.binding),
            html_escape(&info.kind),
            html_escape(info.distribution.as_deref().unwrap_or("")),
            html_escape(info.method.as_deref().unwrap_or("")),
            html_escape(&transform),
            html_escape(&propagation),
            html_escape(&info.quantity_kind),
            html_escape(&info.display_unit),
            info.sample_count,
            html_escape(&info.expression)
        ));
        uncertainty.push_str("</tr>");
    }
    if uncertainty.is_empty() {
        uncertainty.push_str("<tr><td colspan=\"11\">No uncertainty metadata.</td></tr>");
    }

    let mut ml_info = String::new();
    for info in &report.semantic_program.ml_infos {
        ml_info.push_str("<tr>");
        ml_info.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td>",
            info.line,
            html_escape(&info.binding),
            html_escape(&info.kind),
            html_escape(info.source.as_deref().unwrap_or("")),
            html_escape(info.target.as_deref().unwrap_or("")),
            html_escape(&info.features.join(", ")),
            html_escape(&info.expression)
        ));
        ml_info.push_str("</tr>");
    }
    if ml_info.is_empty() {
        ml_info.push_str("<tr><td colspan=\"7\">No ML metadata.</td></tr>");
    }

    let mut domain_summary = String::new();
    for domain in &report.semantic_program.domains {
        let domain_signature = format_domain_signature(&domain.name, &domain.type_parameters);
        let package = domain.package.as_deref().unwrap_or("-");
        let version = domain.version.as_deref().unwrap_or("-");
        domain_summary.push_str("<tr>");
        domain_summary.push_str(&format!(
            "<td>{}</td><td>{}</td><td>metadata</td><td colspan=\"2\">package {}</td><td>version {}</td><td>{}</td><td>metadata</td>",
            domain.line,
            html_escape(&domain_signature),
            html_escape(package),
            html_escape(version),
            html_escape(&format_domain_parameter_list(&domain.type_parameters))
        ));
        domain_summary.push_str("</tr>");
        for variable in &domain.variables {
            domain_summary.push_str("<tr>");
            domain_summary.push_str(&format!(
                "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
                variable.line,
                html_escape(&domain_signature),
                html_escape(&variable.role),
                html_escape(&variable.name),
                html_escape(&variable.quantity_kind),
                html_escape(&variable.display_unit),
                html_escape(&variable.canonical_unit),
                html_escape(&variable.dimension)
            ));
            domain_summary.push_str("</tr>");
        }
        for conservation in &domain.conservations {
            domain_summary.push_str("<tr>");
            domain_summary.push_str(&format!(
                "<td>{}</td><td>{}</td><td>conservation</td><td colspan=\"4\"><code>{}</code></td><td>{}</td>",
                conservation.line,
                html_escape(&domain_signature),
                html_escape(&conservation.text),
                html_escape(&conservation.status)
            ));
            domain_summary.push_str("</tr>");
        }
    }
    if domain_summary.is_empty() {
        domain_summary.push_str("<tr><td colspan=\"8\">No domain metadata.</td></tr>");
    }

    let mut component_summary = String::new();
    for component in report.semantic_program.assembly_components() {
        for port in &component.ports {
            component_summary.push_str("<tr>");
            component_summary.push_str(&format!(
                "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
                port.line,
                html_escape(&component.name),
                html_escape(&port.name),
                html_escape(&port.domain),
                html_escape(&port.status)
            ));
            component_summary.push_str("</tr>");
        }
        for local in &component.local_expressions {
            component_summary.push_str("<tr>");
            component_summary.push_str(&format!(
                "<td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td>",
                local.line,
                html_escape(&component.name),
                html_escape(&local.name),
                html_escape(&local.expression),
                html_escape(&local.status)
            ));
            component_summary.push_str("</tr>");
        }
    }
    if component_summary.is_empty() {
        component_summary.push_str("<tr><td colspan=\"5\">No component ports.</td></tr>");
    }

    let behavior_nodes = spec
        .map(|spec| spec.component_graph.behavior_nodes.clone())
        .unwrap_or_else(|| report_component_behavior_nodes(report));
    let mut component_behavior = String::new();
    for node in behavior_nodes {
        let detail = report_behavior_node_detail(&node);
        component_behavior.push_str("<tr>");
        component_behavior.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td><td>{}</td>",
            node.line,
            html_escape(&node.component),
            html_escape(&node.name),
            html_escape(&node.behavior_kind),
            html_escape(&node.expression),
            html_escape(report_status_label(&node.status)),
            html_escape(&detail)
        ));
        component_behavior.push_str("</tr>");
    }
    if component_behavior.is_empty() {
        component_behavior.push_str("<tr><td colspan=\"7\">No component behavior nodes.</td></tr>");
    }

    let mut connection_summary = String::new();
    for connection in &report.semantic_program.connections {
        connection_summary.push_str("<tr>");
        connection_summary.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            connection.line,
            html_escape(&connection.left),
            html_escape(&connection.right),
            html_escape(&connection.domain),
            html_escape(&connection.status)
        ));
        connection_summary.push_str("</tr>");
    }
    if connection_summary.is_empty() {
        connection_summary.push_str("<tr><td colspan=\"5\">No component connections.</td></tr>");
    }

    let mut assembly_summary = String::new();
    for assembly in &report.semantic_program.component_assemblies {
        let spec_assembly = spec.and_then(|spec| {
            spec.assemblies
                .iter()
                .find(|candidate| candidate.name == assembly.name)
        });
        let assembly_status = spec_assembly
            .map(|assembly| assembly.status.as_str())
            .unwrap_or(&assembly.status);
        let (
            preview_status,
            preview_method,
            preview_mixed,
            preview_nonlinear,
            preview_dae,
            preview_delay,
            preview_predictor,
            preview_external,
            preview_limitations,
        ) = if let Some(spec_assembly) = spec_assembly {
            (
                spec_assembly.solver_preview.status.as_str(),
                spec_assembly.solver_preview.method.as_str(),
                spec_assembly
                    .solver_preview
                    .mixed_algebraic_dynamic
                    .as_str(),
                spec_assembly.solver_preview.nonlinear_residual.as_str(),
                spec_assembly.solver_preview.dae_split.as_str(),
                spec_assembly.solver_preview.delay_history.as_str(),
                spec_assembly.solver_preview.predictor.as_str(),
                spec_assembly.solver_preview.external_adapter.as_str(),
                report_status_list_label(&spec_assembly.solver_preview.limitations),
            )
        } else {
            (
                assembly.solver_preview.status.as_str(),
                assembly.solver_preview.method.as_str(),
                assembly.solver_preview.mixed_algebraic_dynamic.as_str(),
                assembly.solver_preview.nonlinear_residual.as_str(),
                assembly.solver_preview.dae_split.as_str(),
                assembly.solver_preview.delay_history.as_str(),
                assembly.solver_preview.predictor.as_str(),
                assembly.solver_preview.external_adapter.as_str(),
                report_status_list_label(&assembly.solver_preview.limitations),
            )
        };
        assembly_summary.push_str("<tr>");
        assembly_summary.push_str(&format!(
            "<td>{}</td><td>graph</td><td>{}</td><td>components={}</td><td>ports={}, connections={}, domains={}, component equations={}, local expressions={}, operators={}, predictors={}</td><td>{}</td>",
            assembly.line,
            html_escape(&assembly.name),
            assembly.component_count,
            assembly.port_count,
            assembly.connection_count,
            assembly.domain_count,
            assembly.component_equation_count,
            assembly.local_expression_count,
            assembly.operator_call_count,
            assembly.predictor_call_count,
            html_escape(assembly_status)
        ));
        assembly_summary.push_str("</tr>");
        assembly_summary.push_str("<tr>");
        assembly_summary.push_str(&format!(
            "<td>{}</td><td>boundary</td><td>{}</td><td>unknowns={}</td><td>states={}, algebraic={}, inputs={}, outputs={}, parameters={}, equations={}</td><td>{}</td>",
            assembly.line,
            html_escape(&assembly.name),
            assembly.boundary.unknown_count,
            assembly.boundary.state_count,
            assembly.boundary.algebraic_count,
            assembly.boundary.input_count,
            assembly.boundary.output_count,
            assembly.boundary.parameter_count,
            assembly.boundary.equation_count,
            html_escape(
                assembly
                    .boundary
                    .diagnostic_code
                    .as_deref()
                    .unwrap_or(&assembly.boundary.balance_status)
            )
        ));
        assembly_summary.push_str("</tr>");
        for domain_plan in &assembly.domain_plans {
            assembly_summary.push_str("<tr>");
            assembly_summary.push_str(&format!(
                "<td>{}</td><td>domain plan</td><td>{}</td><td>sets={}</td><td>equations={}, variables={}, conservation={}</td><td>{}</td>",
                assembly.line,
                html_escape(&domain_plan.domain),
                domain_plan.connection_set_count,
                domain_plan.equation_count,
                domain_plan.variable_count,
                html_escape(&domain_plan.conservation_status),
                html_escape(&domain_plan.solver_role)
            ));
            assembly_summary.push_str("</tr>");
        }
        assembly_summary.push_str("<tr>");
        assembly_summary.push_str(&format!(
            "<td>{}</td><td>constraint check</td><td>{}</td><td>{}</td><td>dynamic={}, nonlinear={}, dae={}, delay={}, predictor={}, adapter={}</td><td>{}</td>",
            assembly.line,
            html_escape(preview_status),
            html_escape(preview_method),
            html_escape(report_status_label(preview_mixed)),
            html_escape(report_status_label(preview_nonlinear)),
            html_escape(report_status_label(preview_dae)),
            html_escape(report_status_label(preview_delay)),
            html_escape(report_status_label(preview_predictor)),
            html_escape(report_status_label(preview_external)),
            html_escape(&preview_limitations)
        ));
        assembly_summary.push_str("</tr>");
        for connection_set in &assembly.connection_sets {
            assembly_summary.push_str("<tr>");
            assembly_summary.push_str(&format!(
                "<td>{}</td><td>connection set</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
                connection_set.line,
                html_escape(&connection_set.name),
                html_escape(&connection_set.domain),
                html_escape(&connection_set.ports.join(", ")),
                html_escape(&connection_set.status)
            ));
            assembly_summary.push_str("</tr>");
        }
        for equation in &assembly.equations {
            assembly_summary.push_str("<tr>");
            assembly_summary.push_str(&format!(
                "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code><br><small>residual: {}</small><br><small>reason: {}</small></td><td>{}</td>",
                equation.line,
                html_escape(&equation.kind),
                html_escape(&equation.name),
                html_escape(&equation.domain),
                html_escape(&equation.expression),
                html_escape(&equation.residual),
                html_escape(&equation.reason),
                html_escape(&equation.status)
            ));
            assembly_summary.push_str("</tr>");
        }
        assembly_summary.push_str("<tr>");
        assembly_summary.push_str(&format!(
            "<td>{}</td><td>residual graph</td><td>{}</td><td>residuals={}</td><td>dependencies={}, algebraic loop seeds={}, jacobian sparsity entries={}, solver plan={}</td><td>{}</td>",
            assembly.line,
            html_escape(&assembly.residual_graph.name),
            assembly.residual_graph.residuals.len(),
            assembly.residual_graph.dependencies.len(),
            assembly.residual_graph.algebraic_loops.len(),
            assembly.residual_graph.jacobian_sparsity.len(),
            html_escape(&assembly.residual_graph.solver_plan),
            html_escape(&assembly.residual_graph.status)
        ));
        assembly_summary.push_str("</tr>");
    }
    if assembly_summary.is_empty() {
        assembly_summary
            .push_str("<tr><td colspan=\"6\">No component assembly metadata.</td></tr>");
    }

    let mut class_summary = String::new();
    for class_info in &report.semantic_program.classes {
        if class_info.fields.is_empty()
            && class_info.validations.is_empty()
            && class_info.methods.is_empty()
        {
            class_summary.push_str("<tr>");
            class_summary.push_str(&format!(
                "<td>{}</td><td>{}</td><td colspan=\"6\">No fields.</td>",
                class_info.line,
                html_escape(&class_info.name)
            ));
            class_summary.push_str("</tr>");
            continue;
        }
        for field in &class_info.fields {
            class_summary.push_str("<tr>");
            class_summary.push_str(&format!(
                "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
                field.line,
                html_escape(&class_info.name),
                html_escape(&field.name),
                html_escape(&field.type_name),
                html_escape(&field.display_unit),
                html_escape(field.default_value.as_deref().unwrap_or("")),
                if field.required { "yes" } else { "no" },
                html_escape(&field.status)
            ));
            class_summary.push_str("</tr>");
        }
        for validation in &class_info.validations {
            class_summary.push_str("<tr>");
            class_summary.push_str(&format!(
                "<td>{}</td><td>{}</td><td>validate</td><td colspan=\"4\"><code>{}</code></td><td>{}</td>",
                validation.line,
                html_escape(&class_info.name),
                html_escape(&validation.expression),
                html_escape(&validation.status)
            ));
            class_summary.push_str("</tr>");
        }
        for method in &class_info.methods {
            class_summary.push_str("<tr>");
            class_summary.push_str(&format!(
                "<td>{}</td><td>{}</td><td>{}()</td><td>{}</td><td>{}</td><td><code>{}</code></td><td>method</td><td>{}</td>",
                method.line,
                html_escape(&class_info.name),
                html_escape(&method.name),
                html_escape(&method.return_type),
                html_escape(&method.return_display_unit),
                html_escape(&method.expression),
                html_escape(&method.status)
            ));
            class_summary.push_str("</tr>");
        }
    }
    if class_summary.is_empty() {
        class_summary.push_str("<tr><td colspan=\"8\">No class metadata.</td></tr>");
    }

    let mut object_summary = String::new();
    for object in &report.semantic_program.class_objects {
        if let Some(source_object) = &object.source_object {
            object_summary.push_str("<tr>");
            object_summary.push_str(&format!(
                "<td>{}</td><td>{}</td><td>{}</td><td>copy-with</td><td>{}</td><td>{}</td><td>object</td><td>{}</td>",
                object.line,
                html_escape(&object.name),
                html_escape(&object.class_name),
                html_escape(source_object),
                html_escape(&object.construction),
                html_escape(&object.status)
            ));
            object_summary.push_str("</tr>");
        }
        if object.fields.is_empty() && object.validations.is_empty() {
            object_summary.push_str("<tr>");
            object_summary.push_str(&format!(
                "<td>{}</td><td>{}</td><td>{}</td><td colspan=\"5\">No explicit fields.</td>",
                object.line,
                html_escape(&object.name),
                html_escape(&object.class_name)
            ));
            object_summary.push_str("</tr>");
            continue;
        }
        for field in &object.fields {
            object_summary.push_str("<tr>");
            object_summary.push_str(&format!(
                "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
                field.line,
                html_escape(&object.name),
                html_escape(&object.class_name),
                html_escape(&field.name),
                html_escape(&field.expression),
                html_escape(&field.quantity_kind),
                html_escape(&field.display_unit),
                html_escape(&field.status)
            ));
            object_summary.push_str("</tr>");
        }
        for validation in &object.validations {
            object_summary.push_str("<tr>");
            object_summary.push_str(&format!(
                "<td>{}</td><td>{}</td><td>{}</td><td>validate</td><td><code>{}</code></td><td>{} {} {}</td><td>{}</td><td>{}</td>",
                validation.line,
                html_escape(&object.name),
                html_escape(&object.class_name),
                html_escape(&validation.expression),
                html_escape(validation.left_value.as_deref().unwrap_or("")),
                html_escape(&validation.operator),
                html_escape(validation.right_value.as_deref().unwrap_or("")),
                html_escape(&validation.unit),
                html_escape(&validation.status)
            ));
            object_summary.push_str("</tr>");
        }
    }
    if object_summary.is_empty() {
        object_summary.push_str("<tr><td colspan=\"8\">No class objects.</td></tr>");
    }

    let mut system_equations = String::new();
    for system in &report.semantic_program.systems {
        for equation in &system.equations {
            system_equations.push_str("<tr>");
            system_equations.push_str(&format!(
                "<td>{}</td><td>{}</td><td><code>{} {} {}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
                equation.line,
                html_escape(&system.name),
                html_escape(&equation.left),
                html_escape(&equation.relation),
                html_escape(&equation.right),
                html_escape(&equation.left_dimension),
                html_escape(&equation.right_dimension),
                html_escape(&equation.residual),
                html_escape(&equation.status)
            ));
            system_equations.push_str("</tr>");
        }
    }
    if system_equations.is_empty() {
        system_equations.push_str("<tr><td colspan=\"7\">No system equations.</td></tr>");
    }

    let mut schemas = String::new();
    for schema in &report.semantic_program.schemas {
        schemas.push_str("<tr>");
        schemas.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            schema.line,
            html_escape(&schema.name),
            schema.columns.len(),
            schema.constraints.len(),
            schema.missing_policies.len()
        ));
        schemas.push_str("</tr>");
    }
    if schemas.is_empty() {
        schemas.push_str("<tr><td colspan=\"5\">No schemas.</td></tr>");
    }

    let mut csv_promotions = String::new();
    for promotion in &report.semantic_program.csv_promotions {
        csv_promotions.push_str("<tr>");
        csv_promotions.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            promotion.line,
            html_escape(&promotion.binding),
            html_escape(&promotion.schema_name),
            html_escape(&promotion.source_literal),
            promotion.row_count,
            html_escape(promotion.source_hash.as_deref().unwrap_or("not available"))
        ));
        csv_promotions.push_str("</tr>");
    }
    if csv_promotions.is_empty() {
        csv_promotions.push_str("<tr><td colspan=\"6\">No CSV promotions.</td></tr>");
    }

    let mut args_metadata = String::new();
    for args_block in &report.semantic_program.args_blocks {
        if args_block.fields.is_empty() {
            args_metadata.push_str("<tr>");
            args_metadata.push_str(&format!(
                "<td>{}</td><td>{}</td><td colspan=\"4\">No fields.</td>",
                args_block.line,
                html_escape(&args_block.name)
            ));
            args_metadata.push_str("</tr>");
            continue;
        }
        for field in &args_block.fields {
            args_metadata.push_str("<tr>");
            args_metadata.push_str(&format!(
                "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
                field.line,
                html_escape(&args_block.name),
                html_escape(&field.name),
                html_escape(&field.type_name),
                html_escape(field.default_value.as_deref().unwrap_or("")),
                if field.required { "yes" } else { "no" }
            ));
            args_metadata.push_str("</tr>");
        }
    }
    if args_metadata.is_empty() {
        args_metadata.push_str("<tr><td colspan=\"6\">No Args metadata.</td></tr>");
    }

    let error_count = report.diagnostic_count(Severity::Error);
    let warning_count = report.diagnostic_count(Severity::Warning);
    let syntax_items = report.syntax_summary.ast_items;
    let typed_bindings = report.semantic_program.typed_bindings.len();
    let expected_types = report.semantic_program.expected_types.len();
    let hover_count = report.semantic_program.hover_hints.len();
    let quantity_completion_count = report.quantity_completion_count;
    let unit_info_count = report.unit_info_count;
    let type_info_count = report.semantic_program.type_infos.len();
    let unit_derivation_count = report.semantic_program.unit_derivations.len();
    let axis_info_count = report.semantic_program.axis_infos.len();
    let stats_info_count = report.semantic_program.stats_infos.len();
    let integration_count = report.semantic_program.integrations.len();
    let uncertainty_count = report.semantic_program.uncertainty_infos.len();
    let ml_info_count = report.semantic_program.ml_infos.len();
    let domain_count = report.semantic_program.domains.len();
    let component_count = report.semantic_program.assembly_components().len();
    let connection_count = report.semantic_program.connections.len();
    let assembly_count = report.semantic_program.component_assemblies.len();
    let class_count = report.semantic_program.classes.len();
    let object_count = report.semantic_program.class_objects.len();
    let system_count = report.semantic_program.systems.len();
    let equation_count = report
        .semantic_program
        .systems
        .iter()
        .map(|system| system.equations.len())
        .sum::<usize>();
    let residual_count = report
        .semantic_program
        .systems
        .iter()
        .map(|system| system.residuals.len())
        .sum::<usize>();
    let schema_count = report.semantic_program.schemas.len();
    let csv_promotion_count = report.semantic_program.csv_promotions.len();
    let workflow = html_escape(&report.semantic_program.workflow.signature());
    let plot_relative_path = html_escape(plot_relative_path);
    let computed_metrics_section = spec
        .map(render_computed_metrics_section)
        .unwrap_or_default();
    let review_document_section = review_document
        .map(render_review_document_section)
        .unwrap_or_default();
    let validations_section = review_document.map_or_else(
        || spec.map(render_validations_section).unwrap_or_default(),
        render_review_validations_section,
    );
    let quality_report_section = spec.map(render_quality_report_section).unwrap_or_default();
    let time_axes_section = spec.map(render_time_axes_section).unwrap_or_default();
    let time_alignments_section = spec.map(render_time_alignments_section).unwrap_or_default();
    let component_solver_section = spec
        .map(render_component_solver_section)
        .unwrap_or_default();
    let state_space_section = spec.map(render_state_space_section).unwrap_or_default();
    let system_solver_section = spec.map(render_system_solver_section).unwrap_or_default();
    let kernel_plan_section = spec.map(render_kernel_plan_section).unwrap_or_default();

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <style>
    :root {{
      color-scheme: light;
      font-family: "Segoe UI", Arial, sans-serif;
      background: #f5f6f8;
      color: #20242a;
    }}
    body {{
      margin: 0;
      padding: 32px;
    }}
    main {{
      max-width: 1040px;
      margin: 0 auto;
    }}
    h1, h2 {{
      letter-spacing: 0;
    }}
    h1 {{
      margin: 0 0 8px;
      font-size: 28px;
    }}
    h2 {{
      margin-top: 28px;
      font-size: 20px;
    }}
    .summary {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: 12px;
      margin: 24px 0;
    }}
    .metric {{
      border: 1px solid #d9dee7;
      border-radius: 8px;
      padding: 14px;
      background: #fff;
    }}
    .metric strong {{
      display: block;
      font-size: 24px;
    }}
    table {{
      width: 100%;
      border-collapse: collapse;
      background: #fff;
      border: 1px solid #d9dee7;
    }}
    th, td {{
      text-align: left;
      border-bottom: 1px solid #e7ebf0;
      padding: 10px 12px;
      vertical-align: top;
      overflow-wrap: anywhere;
    }}
    th {{
      background: #eef2f7;
      font-weight: 600;
    }}
    code {{
      font-family: Consolas, "SFMono-Regular", monospace;
    }}
    .review-fingerprint {{
      overflow-wrap: anywhere;
    }}
    .plot {{
      width: 100%;
      min-height: 320px;
      border: 1px solid #d9dee7;
      border-radius: 8px;
      background: #fff;
    }}
  </style>
</head>
<body>
  <main>
    <h1>{title}</h1>
    <p>Reviewable EngLang artifact with source hash <code>{source_hash}</code>.</p>
    <section class="summary" aria-label="Run summary">
      <div class="metric"><span>Errors</span><strong>{error_count}</strong></div>
      <div class="metric"><span>Warnings</span><strong>{warning_count}</strong></div>
      <div class="metric"><span>AST Items</span><strong>{syntax_items}</strong></div>
      <div class="metric"><span>Typed Bindings</span><strong>{typed_bindings}</strong></div>
      <div class="metric"><span>Expected Types</span><strong>{expected_types}</strong></div>
      <div class="metric"><span>Hover Hints</span><strong>{hover_count}</strong></div>
      <div class="metric"><span>Quantity Completions</span><strong>{quantity_completion_count}</strong></div>
      <div class="metric"><span>Unit Infos</span><strong>{unit_info_count}</strong></div>
      <div class="metric"><span>Type Info</span><strong>{type_info_count}</strong></div>
      <div class="metric"><span>Unit Derivations</span><strong>{unit_derivation_count}</strong></div>
      <div class="metric"><span>Axis Info</span><strong>{axis_info_count}</strong></div>
      <div class="metric"><span>Stats Info</span><strong>{stats_info_count}</strong></div>
      <div class="metric"><span>Integrations</span><strong>{integration_count}</strong></div>
      <div class="metric"><span>Uncertainty</span><strong>{uncertainty_count}</strong></div>
      <div class="metric"><span>ML Info</span><strong>{ml_info_count}</strong></div>
      <div class="metric"><span>Domains</span><strong>{domain_count}</strong></div>
      <div class="metric"><span>Components</span><strong>{component_count}</strong></div>
      <div class="metric"><span>Connections</span><strong>{connection_count}</strong></div>
      <div class="metric"><span>Assemblies</span><strong>{assembly_count}</strong></div>
      <div class="metric"><span>Classes</span><strong>{class_count}</strong></div>
      <div class="metric"><span>Objects</span><strong>{object_count}</strong></div>
      <div class="metric"><span>Systems</span><strong>{system_count}</strong></div>
      <div class="metric"><span>Equations</span><strong>{equation_count}</strong></div>
      <div class="metric"><span>Residuals</span><strong>{residual_count}</strong></div>
      <div class="metric"><span>Schemas</span><strong>{schema_count}</strong></div>
      <div class="metric"><span>CSV Promotions</span><strong>{csv_promotion_count}</strong></div>
      <div class="metric"><span>Workflow</span><strong>{workflow}</strong></div>
      <div class="metric"><span>Compiler</span><strong>{compiler_version}</strong></div>
      <div class="metric"><span>Report</span><strong>{report_version}</strong></div>
    </section>
    {review_document_section}
    <h2>Args Metadata</h2>
    <table>
      <thead><tr><th>Line</th><th>Struct</th><th>Field</th><th>Type</th><th>Default</th><th>Required</th></tr></thead>
      <tbody>{args_metadata}</tbody>
    </table>
    <h2>Inferred Declarations</h2>
    <table>
      <thead><tr><th>Line</th><th>Name</th><th>Quantity</th><th>Display Unit</th><th>Expression</th></tr></thead>
      <tbody>{inferred}</tbody>
    </table>
    <h2>Hover Hints</h2>
    <table>
      <thead><tr><th>Position</th><th>Name</th><th>Quantity</th><th>Detail</th></tr></thead>
      <tbody>{hover_hints}</tbody>
    </table>
    <h2>Type Info</h2>
    <table>
      <thead><tr><th>Line</th><th>Name</th><th>Quantity</th><th>Display Unit</th><th>Canonical Unit</th><th>Dimension</th></tr></thead>
      <tbody>{type_info}</tbody>
    </table>
    <h2>Unit Derivations</h2>
    <table>
      <thead><tr><th>Line</th><th>Name</th><th>Source Unit</th><th>Display Unit</th><th>Canonical Unit</th></tr></thead>
      <tbody>{unit_derivations}</tbody>
    </table>
    <h2>Axis Info</h2>
    <table>
      <thead><tr><th>Line</th><th>Binding</th><th>Axis</th><th>Role</th><th>Source</th></tr></thead>
      <tbody>{axis_info}</tbody>
    </table>
    {time_axes_section}
    <h2>Statistics</h2>
    <table>
      <thead><tr><th>Line</th><th>Source</th><th>Quantity</th><th>Axis</th><th>Statistics</th><th>Cache Key</th></tr></thead>
      <tbody>{stats_info}</tbody>
    </table>
    <h2>Integrations</h2>
    <table>
      <thead><tr><th>Line</th><th>Binding</th><th>Source</th><th>Input</th><th>Axis</th><th>Result</th></tr></thead>
      <tbody>{integrations}</tbody>
    </table>
    {computed_metrics_section}
    {validations_section}
    {quality_report_section}
    {time_alignments_section}
    <h2>Uncertainty</h2>
    <table>
      <thead><tr><th>Line</th><th>Binding</th><th>Kind</th><th>Distribution</th><th>Method</th><th>Transform</th><th>Propagation</th><th>Quantity</th><th>Unit</th><th>Samples</th><th>Expression</th></tr></thead>
      <tbody>{uncertainty}</tbody>
    </table>
    <h2>ML Models</h2>
    <table>
      <thead><tr><th>Line</th><th>Binding</th><th>Kind</th><th>Source</th><th>Target</th><th>Features</th><th>Expression</th></tr></thead>
      <tbody>{ml_info}</tbody>
    </table>
    <h2>Domains</h2>
    <table>
      <thead><tr><th>Line</th><th>Domain</th><th>Role</th><th>Name</th><th>Quantity</th><th>Display Unit</th><th>Canonical Unit</th><th>Dimension/Status</th></tr></thead>
      <tbody>{domain_summary}</tbody>
    </table>
    <h2>Component Ports</h2>
    <table>
      <thead><tr><th>Line</th><th>Component</th><th>Port</th><th>Domain</th><th>Status</th></tr></thead>
      <tbody>{component_summary}</tbody>
    </table>
    <h2>Component Behavior</h2>
    <table>
      <thead><tr><th>Line</th><th>Component</th><th>Name</th><th>Kind</th><th>Expression</th><th>Status</th><th>Details</th></tr></thead>
      <tbody>{component_behavior}</tbody>
    </table>
    <h2>Connections</h2>
    <table>
      <thead><tr><th>Line</th><th>Left</th><th>Right</th><th>Domain</th><th>Status</th></tr></thead>
      <tbody>{connection_summary}</tbody>
    </table>
    <h2>Component Assembly</h2>
    <table>
      <thead><tr><th>Line</th><th>Kind</th><th>Name</th><th>Domain</th><th>Detail</th><th>Status</th></tr></thead>
      <tbody>{assembly_summary}</tbody>
    </table>
    {component_solver_section}
    <h2>Classes</h2>
    <table>
      <thead><tr><th>Line</th><th>Class</th><th>Field</th><th>Type</th><th>Unit</th><th>Default</th><th>Required</th><th>Status</th></tr></thead>
      <tbody>{class_summary}</tbody>
    </table>
    <h2>Objects</h2>
    <table>
      <thead><tr><th>Line</th><th>Object</th><th>Class</th><th>Field</th><th>Expression</th><th>Quantity</th><th>Unit</th><th>Status</th></tr></thead>
      <tbody>{object_summary}</tbody>
    </table>
    {state_space_section}
    {system_solver_section}
    {kernel_plan_section}
    <h2>System Equations</h2>
    <table>
      <thead><tr><th>Line</th><th>System</th><th>Equation</th><th>Left Dimension</th><th>Right Dimension</th><th>Residual</th><th>Status</th></tr></thead>
      <tbody>{system_equations}</tbody>
    </table>
    <h2>Schemas</h2>
    <table>
      <thead><tr><th>Line</th><th>Name</th><th>Columns</th><th>Constraints</th><th>Missing Policies</th></tr></thead>
      <tbody>{schemas}</tbody>
    </table>
    <h2>CSV Promotions</h2>
    <table>
      <thead><tr><th>Line</th><th>Binding</th><th>Schema</th><th>Source</th><th>Rows</th><th>Source Hash</th></tr></thead>
      <tbody>{csv_promotions}</tbody>
    </table>
    <h2>Diagnostics</h2>
    <table>
      <thead><tr><th>Line</th><th>Severity</th><th>Code</th><th>Message</th></tr></thead>
      <tbody>{diagnostics}</tbody>
    </table>
    <h2>Plot</h2>
    <iframe class="plot" src="{plot_relative_path}" title="Generated plot"></iframe>
  </main>
</body>
</html>
"#,
        source_hash = html_escape(&report.source_hash),
        compiler_version = html_escape(eng_compiler::COMPILER_VERSION),
        report_version = html_escape(REPORT_VERSION)
    )
}

fn review_document_array<'a>(document: &'a Value, key: &str) -> &'a [Value] {
    document
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn review_text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn review_scalar_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => value.to_string(),
        Some(Value::Bool(value)) => value.to_string(),
        _ => "-".to_owned(),
    }
}

fn review_runtime_result(item: &Value) -> Option<&Value> {
    item.get("runtime_result").filter(|value| value.is_object())
}

fn review_runtime_status(item: &Value) -> &str {
    review_runtime_result(item)
        .and_then(|result| review_text(result, "status"))
        .or_else(|| review_text(item, "status"))
        .unwrap_or("-")
}

fn review_runtime_evidence(item: &Value) -> String {
    let Some(result) = review_runtime_result(item) else {
        return "-".to_owned();
    };
    let mut evidence = Vec::new();
    for (key, label) in [
        ("provenance", ""),
        ("source", "source="),
        ("model", "model="),
        ("model_kind", "model_kind="),
        ("input_table", "input_table="),
        ("target", "target="),
        ("target_quantity", "target_quantity="),
        ("target_unit", "target_unit="),
        ("output_quantity", "output_quantity="),
        ("output_unit", "output_unit="),
        ("path", "path="),
        ("artifact_path", "path="),
        ("hash", "hash="),
        ("training_data_hash", "training_hash="),
        ("model_artifact_hash", "model_hash="),
        ("prediction_hash", "prediction_hash="),
        ("confidence_column", "confidence="),
        ("database", "database="),
        ("manifest_path", "manifest="),
        ("manifest_hash", "manifest_hash="),
        ("database_hash_before", "database_hash_before="),
        ("database_hash_after", "database_hash_after="),
        ("transaction_status", "transaction="),
        ("schema_status", "schema_status="),
        ("source_table", "table="),
        ("source_column", "column="),
        ("axis", "axis="),
        ("x_unit", "x_unit="),
        ("source_hash", "hash="),
        ("schema", "schema="),
        ("representation", "representation="),
        ("materialization", "materialization="),
    ] {
        if let Some(value) = review_text(result, key).filter(|value| !value.is_empty()) {
            evidence.push(format!("{label}{value}"));
        }
    }
    for (key, label) in [
        ("features", "features="),
        ("schema", "schema="),
        ("case_ids", "case_ids="),
    ] {
        if let Some(values) = result.get(key).and_then(Value::as_array) {
            let values = values.iter().filter_map(Value::as_str).collect::<Vec<_>>();
            if !values.is_empty() {
                evidence.push(format!("{label}{}", values.join(", ")));
            }
        }
    }
    if let Some(tables) = result.get("tables").and_then(Value::as_array) {
        for table in tables {
            let binding = review_text(table, "binding").unwrap_or("table");
            let source = review_text(table, "source").unwrap_or("-");
            let hash = review_text(table, "source_hash").unwrap_or("-");
            evidence.push(format!("{binding}: source={source}, hash={hash}"));
        }
    }
    if evidence.is_empty() {
        "-".to_owned()
    } else {
        evidence.join("; ")
    }
}

fn review_model_metrics_summary(result: &Value) -> Option<String> {
    let metrics = result.get("metrics").and_then(Value::as_object)?;
    let unit = review_text(result, "target_unit")
        .or_else(|| review_text(result, "output_unit"))
        .or_else(|| review_text(result, "unit"))
        .unwrap_or("");
    let mut values = Vec::new();
    for (key, label, with_unit) in [
        ("rmse", "RMSE", true),
        ("mae", "MAE", true),
        ("r2", "R2", false),
    ] {
        let Some(value) = metrics.get(key).filter(|value| !value.is_null()) else {
            continue;
        };
        let value = review_scalar_text(Some(value));
        if with_unit && !unit.is_empty() {
            values.push(format!("{label}={value} {unit}"));
        } else {
            values.push(format!("{label}={value}"));
        }
    }
    (!values.is_empty()).then(|| values.join("; "))
}

fn review_runtime_summary(item: &Value) -> String {
    let Some(result) = review_runtime_result(item) else {
        return "-".to_owned();
    };
    match review_text(result, "provenance").unwrap_or_default() {
        "runtime_model" => {
            let kind = review_text(result, "model_kind").unwrap_or("model");
            let train = review_scalar_text(result.get("train_count"));
            let test = review_scalar_text(result.get("test_count"));
            let mut summary = format!("{kind} model; train={train}, test={test}");
            if let Some(metrics) = review_model_metrics_summary(result) {
                summary.push_str("; ");
                summary.push_str(&metrics);
            }
            return summary;
        }
        "runtime_model_card" => {
            let kind = review_text(result, "model_kind").unwrap_or("model");
            let train = review_scalar_text(result.get("train_count"));
            let test = review_scalar_text(result.get("test_count"));
            return format!("{kind} model card; train={train}, test={test}");
        }
        "runtime_model_metrics" => {
            if let Some(metrics) = review_model_metrics_summary(result) {
                return metrics;
            }
        }
        "runtime_prediction" => {
            let rows = result.get("row_count").and_then(Value::as_u64).unwrap_or(0);
            let outputs = result
                .get("outputs")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            return format!("{rows} predictions; {outputs} outputs");
        }
        _ => {}
    }
    let unit = review_text(result, "unit").unwrap_or("");
    let with_unit = |value: String| {
        if unit.is_empty() {
            value
        } else {
            format!("{value} {unit}")
        }
    };

    if let Some(left_value) = result.get("left_value").filter(|value| !value.is_null()) {
        let left = review_scalar_text(Some(left_value));
        let operator = review_text(result, "operator").unwrap_or("");
        if let Some(right_value) = result.get("right_value").filter(|value| !value.is_null()) {
            return with_unit(
                [
                    left,
                    operator.to_owned(),
                    review_scalar_text(Some(right_value)),
                ]
                .into_iter()
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
            );
        }
        let right = review_text(result, "right").unwrap_or("");
        return [with_unit(left), operator.to_owned(), right.to_owned()]
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
    }
    if let Some(value) = result.get("value").filter(|value| !value.is_null()) {
        return with_unit(review_scalar_text(Some(value)));
    }
    if let Some(values) = result.get("values").and_then(Value::as_array) {
        if !values.is_empty() {
            return values
                .iter()
                .map(|value| {
                    let name = review_text(value, "name").unwrap_or("value");
                    let unit = review_text(value, "unit").unwrap_or("");
                    let number = review_scalar_text(value.get("value"));
                    if unit.is_empty() {
                        format!("{name}={number}")
                    } else {
                        format!("{name}={number} {unit}")
                    }
                })
                .collect::<Vec<_>>()
                .join("; ");
        }
    }
    if let Some(tables) = result.get("tables").and_then(Value::as_array) {
        if !tables.is_empty() {
            let rows = tables
                .iter()
                .filter_map(|table| table.get("row_count").and_then(Value::as_u64))
                .sum::<u64>();
            let noun = if tables.len() == 1 { "table" } else { "tables" };
            return format!("{} {noun}, {rows} rows", tables.len());
        }
    }
    if let Some(path) = review_text(result, "artifact_path") {
        let kind = review_text(result, "artifact_kind").unwrap_or("artifact");
        return format!("{kind}: {path}");
    }
    if let Some(actual_count) = result.get("actual_count").and_then(Value::as_u64) {
        let missing_count = result
            .get("missing_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if let Some(expected_count) = result.get("expected_count").and_then(Value::as_u64) {
            return format!("{actual_count}/{expected_count} samples; missing {missing_count}");
        }
        return format!("{actual_count} samples; missing {missing_count}");
    }
    if let Some(output_rows) = result.get("output_row_count").and_then(Value::as_u64) {
        if let Some(input_rows) = result.get("input_row_count").and_then(Value::as_u64) {
            return format!("{input_rows} -> {output_rows} rows");
        }
        return format!("{output_rows} rows");
    }
    if let Some(rows) = result.get("row_count").and_then(Value::as_u64) {
        return format!("{rows} rows");
    }
    if let Some(points) = result.get("point_count").and_then(Value::as_u64) {
        return format!("{points} points");
    }
    if let Some(count) = result.get("count").and_then(Value::as_u64) {
        let start = result.get("start").filter(|value| !value.is_null());
        let end = result.get("end").filter(|value| !value.is_null());
        if start.is_some() || end.is_some() {
            return format!(
                "{count} samples; {} -> {}{}",
                review_scalar_text(start),
                review_scalar_text(end),
                if unit.is_empty() {
                    String::new()
                } else {
                    format!(" {unit}")
                }
            );
        }
        return format!("{count} samples");
    }
    review_runtime_status(item).to_owned()
}

fn review_row_name(row: &Value) -> &str {
    ["name", "binding", "target", "source"]
        .iter()
        .find_map(|key| review_text(row, key))
        .unwrap_or("-")
}

fn render_review_document_section(document: &Value) -> String {
    let evidence = document
        .get("runtime_evidence")
        .filter(|value| value.is_object())
        .unwrap_or(&Value::Null);
    let mut rows = String::new();
    for (section, label) in [
        ("inputs", "Input"),
        ("schemas", "Schema"),
        ("symbols", "Symbol"),
        ("time_axes", "Time axis"),
        ("calculations", "Calculation"),
        ("table_transforms", "Table transform"),
        ("report_outputs", "Report output"),
        ("side_effects", "Side effect"),
    ] {
        for row in review_document_array(document, section) {
            if review_runtime_result(row).is_none() {
                continue;
            }
            rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                html_escape(label),
                row.get("line")
                    .and_then(Value::as_u64)
                    .map(|line| line.to_string())
                    .unwrap_or_else(|| "-".to_owned()),
                html_escape(review_row_name(row)),
                html_escape(&review_runtime_summary(row)),
                html_escape(&review_runtime_evidence(row)),
                html_escape(review_runtime_status(row))
            ));
        }
    }
    if rows.is_empty() {
        rows.push_str("<tr><td colspan=6>No runtime ReviewDocument values.</td></tr>");
    }

    format!(
        r#"<h2>Runtime Review</h2>
    <section class="summary" aria-label="Runtime review summary">
      <div class="metric"><span>Status</span><strong>{}</strong></div>
      <div class="metric"><span>Scope</span><strong>{}</strong></div>
      <div class="metric"><span>Values</span><strong>{}</strong></div>
      <div class="metric"><span>Tables</span><strong>{}</strong></div>
      <div class="metric"><span>Time series</span><strong>{}</strong></div>
      <div class="metric"><span>Coverage</span><strong>{}</strong></div>
      <div class="metric"><span>Models</span><strong>{}</strong></div>
      <div class="metric"><span>Predictions</span><strong>{}</strong></div>
      <div class="metric"><span>Side effects</span><strong>{}</strong></div>
      <div class="metric"><span>Validations</span><strong>{}</strong></div>
    </section>
    <p class="review-fingerprint">Review fingerprint <code>{}</code></p>
    <table>
      <thead><tr><th>Section</th><th>Line</th><th>Name</th><th>Runtime Result</th><th>Evidence</th><th>Status</th></tr></thead>
      <tbody>{rows}</tbody>
    </table>"#,
        html_escape(review_text(document, "status").unwrap_or("-")),
        html_escape(review_text(document, "semantic_hash_scope").unwrap_or("static")),
        evidence
            .get("numeric_value_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        evidence
            .get("table_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        evidence
            .get("timeseries_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        evidence
            .get("timeseries_coverage_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        evidence
            .get("model_result_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        evidence
            .get("prediction_result_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        evidence
            .get("side_effect_result_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        evidence
            .get("validation_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        html_escape(review_text(document, "semantic_hash").unwrap_or("-"))
    )
}

fn render_review_validations_section(document: &Value) -> String {
    let validations = review_document_array(document, "validations");
    if validations.is_empty() {
        return String::new();
    }
    let rows = validations
        .iter()
        .map(|validation| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                validation
                    .get("line")
                    .and_then(Value::as_u64)
                    .map(|line| line.to_string())
                    .unwrap_or_else(|| "-".to_owned()),
                html_escape(review_text(validation, "expression").unwrap_or("-")),
                html_escape(&review_runtime_summary(validation)),
                html_escape(review_runtime_status(validation))
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<h2>Validations</h2>
    <table>
      <thead><tr><th>Line</th><th>Expression</th><th>Runtime Result</th><th>Status</th></tr></thead>
      <tbody>{rows}</tbody>
    </table>"#
    )
}

fn render_computed_metrics_section(spec: &ReportSpec) -> String {
    if spec.computed_metrics.is_empty() {
        return String::new();
    }
    let rows = spec
        .computed_metrics
        .iter()
        .map(|metric| {
            let alignment = metric
                .alignment_reference
                .as_deref()
                .map(|reference| {
                    format!(
                        "{} ({}/{})",
                        reference,
                        metric.alignment_status.as_deref().unwrap_or("unknown"),
                        metric.alignment_step_status.as_deref().unwrap_or("unknown")
                    )
                })
                .unwrap_or_else(|| "n/a".to_owned());
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.6}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                metric.line,
                html_escape(&metric.binding),
                html_escape(&metric.kind),
                html_escape(&format!("{} vs {}", metric.left, metric.right)),
                metric.value,
                html_escape(&metric.unit),
                html_escape(&alignment),
                html_escape(&metric.status)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<h2>Computed Metrics</h2>
    <table>
      <thead><tr><th>Line</th><th>Binding</th><th>Kind</th><th>Comparison</th><th>Value</th><th>Unit</th><th>Alignment</th><th>Status</th></tr></thead>
      <tbody>{rows}</tbody>
    </table>"#
    )
}

fn render_validations_section(spec: &ReportSpec) -> String {
    if spec.validations.is_empty() {
        return String::new();
    }
    let rows = spec
        .validations
        .iter()
        .map(|validation| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                validation.line,
                html_escape(&validation.expression),
                validation
                    .left_value
                    .map(|value| format!("{value:.6}"))
                    .unwrap_or_else(|| "-".to_owned()),
                validation
                    .right_value
                    .map(|value| format!("{value:.6}"))
                    .unwrap_or_else(|| "-".to_owned()),
                html_escape(&validation.status)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<h2>Validations</h2>
    <table>
      <thead><tr><th>Line</th><th>Expression</th><th>Left</th><th>Right</th><th>Status</th></tr></thead>
      <tbody>{rows}</tbody>
    </table>"#
    )
}

fn render_quality_report_section(spec: &ReportSpec) -> String {
    if spec.quality_report.results.is_empty() {
        return String::new();
    }
    let summary = format!(
        "status={} total={} passed={} warning={} failed={} unavailable={}",
        spec.quality_report.status,
        spec.quality_report.total_count,
        spec.quality_report.passed_count,
        spec.quality_report.warning_count,
        spec.quality_report.failed_count,
        spec.quality_report.unavailable_count
    );
    let rows = spec
        .quality_report
        .results
        .iter()
        .map(|result| {
            let score = result
                .score
                .map(|value| format!("{value:.3}"))
                .unwrap_or_else(|| "-".to_owned());
            let failures = format_quality_failures_html(result);
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}/{}/{}</td><td>{}</td><td>{}</td></tr>",
                result.line,
                html_escape(&result.binding),
                html_escape(&result.category),
                html_escape(&result.subject),
                html_escape(&score),
                html_escape(&result.status),
                result.passed_count,
                result.warning_count,
                result.failed_count,
                html_escape(&result.reason),
                failures
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<h2>Quality Report</h2>
    <p><code>{}</code></p>
    <table>
      <thead><tr><th>Line</th><th>Binding</th><th>Category</th><th>Subject</th><th>Score</th><th>Status</th><th>Pass/Warn/Fail</th><th>Reason</th><th>Failures</th></tr></thead>
      <tbody>{rows}</tbody>
    </table>"#,
        html_escape(&summary)
    )
}

fn format_quality_failures_html(result: &ReportQualityResult) -> String {
    if result.failures.is_empty() {
        return "-".to_owned();
    }
    result
        .failures
        .iter()
        .take(5)
        .map(|failure| {
            format!(
                "row {} field {} value {}: {}",
                failure.row, failure.field, failure.value, failure.message
            )
        })
        .chain(
            (result.failures.len() > 5)
                .then(|| format!("+{} more", result.failures.len().saturating_sub(5))),
        )
        .map(|text| html_escape(&text))
        .collect::<Vec<_>>()
        .join("<br>")
}

fn render_time_alignments_section(spec: &ReportSpec) -> String {
    if spec.time_alignments.is_empty() {
        return String::new();
    }
    let rows = spec
        .time_alignments
        .iter()
        .map(|alignment| {
            let left_step = format_alignment_step(alignment.left_nominal_step, alignment.left_irregular);
            let right_step =
                format_alignment_step(alignment.right_nominal_step, alignment.right_irregular);
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}/{}</td><td>{}/{} ({})<br>{}</td><td>{} / {}</td><td>{}</td><td>{}</td></tr>",
                html_escape(&alignment.binding),
                html_escape(&alignment.left),
                html_escape(&alignment.right),
                html_escape(&alignment.axis),
                html_escape(&format!("{} / {}", alignment.strategy, alignment.method)),
                alignment.matched_count,
                alignment.left_count.min(alignment.right_count),
                alignment.output_count,
                alignment.target_count,
                html_escape(&alignment.materialization_status),
                html_escape(&alignment.materialization_reason),
                html_escape(&left_step),
                html_escape(&right_step),
                html_escape(&alignment.step_status),
                html_escape(&alignment.status)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<h2>Time Alignments</h2>
    <table>
      <thead><tr><th>Binding</th><th>Left</th><th>Right</th><th>Axis</th><th>Strategy</th><th>Matched</th><th>Output</th><th>Nominal Step</th><th>Step</th><th>Alignment</th></tr></thead>
      <tbody>{rows}</tbody>
    </table>"#
    )
}

fn format_alignment_step(step: Option<f64>, irregular: bool) -> String {
    let mut label = step
        .map(format_alignment_number)
        .unwrap_or_else(|| "n/a".to_owned());
    if irregular {
        label.push_str(" (irregular)");
    }
    label
}

fn format_optional_alignment_number(value: Option<f64>) -> String {
    value
        .map(format_alignment_number)
        .unwrap_or_else(|| "n/a".to_owned())
}

fn format_alignment_number(value: f64) -> String {
    let mut text = format!("{value:.6}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn render_state_space_section(spec: &ReportSpec) -> String {
    if spec.state_space_vectors.is_empty() && spec.linear_operators.is_empty() {
        return String::new();
    }
    let vector_rows = spec
        .state_space_vectors
        .iter()
        .map(|vector| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                vector.line,
                html_escape(&vector.system),
                html_escape(&vector.name),
                html_escape(&vector.vector_type),
                html_escape(&vector.members.join(", ")),
                html_escape(&vector.status)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let operator_rows = spec
        .linear_operators
        .iter()
        .map(|operator| {
            let canonical_matrix = operator
                .canonical_matrix
                .as_deref()
                .map(format_canonical_matrix)
                .unwrap_or_else(|| "-".to_owned());
            let canonical_entries = format_canonical_entries(&operator.canonical_entries);
            let compatibility = format!(
                "rows: {} [{}]; cols: {} [{}]; {}",
                operator.row_quantity_kinds.join(", "),
                operator.row_units.join(", "),
                operator.column_quantity_kinds.join(", "),
                operator.column_units.join(", "),
                operator.compatibility_status
            );
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{} -> {}</td><td>{}x{}</td><td><code>{}</code></td><td><code>{}</code></td><td><code>{}</code></td><td>{}</td><td>{}</td></tr>",
                operator.line,
                html_escape(&operator.system),
                html_escape(&operator.name),
                html_escape(&operator.from),
                html_escape(&operator.to),
                operator.row_count,
                operator.column_count,
                html_escape(operator.expression.as_deref().unwrap_or("-")),
                html_escape(&canonical_matrix),
                html_escape(&canonical_entries),
                html_escape(&compatibility),
                html_escape(&operator.status)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<h2>State-Space Metadata</h2>
    <table>
      <thead><tr><th>Line</th><th>System</th><th>Vector</th><th>Type</th><th>Members</th><th>Status</th></tr></thead>
      <tbody>{}</tbody>
    </table>
    <table>
      <thead><tr><th>Line</th><th>System</th><th>Operator</th><th>Mapping</th><th>Shape</th><th>Expression</th><th>Canonical Matrix</th><th>Canonical Entries</th><th>Compatibility</th><th>Status</th></tr></thead>
      <tbody>{}</tbody>
    </table>"#,
        if vector_rows.is_empty() {
            "<tr><td colspan=\"6\">No state-space vectors.</td></tr>".to_owned()
        } else {
            vector_rows
        },
        if operator_rows.is_empty() {
            "<tr><td colspan=\"10\">No linear operators.</td></tr>".to_owned()
        } else {
            operator_rows
        }
    )
}

fn format_canonical_entries(entries: &[ReportLinearOperatorEntry]) -> String {
    if entries.is_empty() {
        return "-".to_owned();
    }
    entries
        .iter()
        .map(|entry| {
            format!(
                "{}<-{}: {}",
                entry.row_member, entry.column_member, entry.coefficient
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn format_canonical_matrix(matrix: &[Vec<f64>]) -> String {
    matrix
        .iter()
        .map(|row| {
            format!(
                "[{}]",
                row.iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn render_system_solver_section(spec: &ReportSpec) -> String {
    let rows = spec
        .system_ir
        .iter()
        .flat_map(|system| {
            system.solver_results.iter().map(move |solution| {
                let binding = solution.binding.as_deref().unwrap_or("-");
                let variables = format!(
                    "states={} algebraic={} inputs={} parameters={} outputs={}",
                    html_escape(&join_or_dash(&solution.states)),
                    html_escape(&join_or_dash(&solution.algebraic_variables)),
                    html_escape(&join_or_dash(&solution.inputs)),
                    html_escape(&join_or_dash(&solution.parameters)),
                    html_escape(&join_or_dash(&solution.outputs))
                );
                let source_equations = format_system_solver_source_equations_summary(solution);
                let step_diagnostics = format_system_solver_step_diagnostics_summary(solution);
                let diagnostics = format!(
                    "tol={} iter={}/{} convergence={} substeps={} failure_code={} failure={}",
                    format_solver_tolerance(solution.tolerance),
                    solution.iteration_count,
                    solution.max_iterations,
                    html_escape(&solution.convergence_status),
                    html_escape(&step_diagnostics),
                    html_escape(solution.failure_code.as_deref().unwrap_or("-")),
                    html_escape(solution.failure_reason.as_deref().unwrap_or("-"))
                );
                format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{} / {}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.6} {}</td></tr>",
                    html_escape(&system.name),
                    html_escape(binding),
                    html_escape(&solution.state),
                    variables,
                    html_escape(&source_equations),
                    html_escape(&solution.status),
                    html_escape(&solution.method),
                    diagnostics,
                    solution.step_count,
                    format_alignment_number(solution.final_value),
                    solution.time_step_s,
                    html_escape(&solution.time_unit)
                )
            })
        })
        .collect::<Vec<_>>()
        .join("");
    if rows.is_empty() {
        return String::new();
    }
    format!(
        r#"<h2>System Solver Results</h2>
    <table>
      <thead><tr><th>System</th><th>Binding</th><th>Trajectory</th><th>Variables</th><th>Source Equations</th><th>Status/Method</th><th>Diagnostics</th><th>Steps</th><th>Final</th><th>Step</th></tr></thead>
      <tbody>{rows}</tbody>
    </table>"#
    )
}

fn format_system_solver_source_equations_summary(solution: &ReportSystemSolution) -> String {
    if solution.source_equations.is_empty() {
        return "-".to_owned();
    }
    let mut values = solution
        .source_equations
        .iter()
        .take(3)
        .map(|equation| {
            let source_line = equation
                .source_line
                .map(|line| format!("L{line}"))
                .unwrap_or_else(|| "L?".to_owned());
            format!(
                "{}:{} {} {}",
                equation.kind, equation.target, source_line, equation.residual_expression
            )
        })
        .collect::<Vec<_>>();
    if solution.source_equations.len() > values.len() {
        values.push(format!(
            "+{} more",
            solution.source_equations.len() - values.len()
        ));
    }
    values.join("; ")
}
fn format_system_solver_step_diagnostics_summary(solution: &ReportSystemSolution) -> String {
    if solution.step_diagnostics.is_empty() {
        return "-".to_owned();
    }
    let rejected = solution
        .step_diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.status != "accepted")
        .count();
    let max_error = solution
        .step_diagnostics
        .iter()
        .map(|diagnostic| diagnostic.error_norm.abs())
        .fold(0.0, f64::max);
    format!(
        "{} total, {} rejected, max_error={}",
        solution.step_diagnostics.len(),
        rejected,
        format_alignment_number(max_error)
    )
}

fn render_kernel_plan_section(spec: &ReportSpec) -> String {
    let plan = &spec.kernel_plan;
    let rows = plan
        .candidates
        .iter()
        .map(|candidate| {
            let rows = candidate
                .estimated_rows
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_owned());
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                candidate.line,
                html_escape(&candidate.name),
                html_escape(&candidate.kind),
                html_escape(&candidate.source),
                html_escape(&candidate.lowering_status),
                html_escape(&candidate.executor_status),
                html_escape(&candidate.fallback_reason),
                html_escape(&format!(
                    "rows={}, inputs={}, outputs={}, ops={}, scans={}",
                    rows,
                    candidate.input_count,
                    candidate.output_count,
                    candidate.operation_count,
                    candidate.scan_count
                )),
                html_escape(&candidate.operations.join(", "))
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let rows = if rows.is_empty() {
        "<tr><td colspan=\"9\">No kernel candidates.</td></tr>".to_owned()
    } else {
        rows
    };
    format!(
        r#"<h2>Runtime Optimization Kernel Plan</h2>
    <table>
      <thead><tr><th>Backend</th><th>Requested</th><th>Selected</th><th>Status</th><th>Reason</th><th>Candidates</th></tr></thead>
      <tbody><tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr></tbody>
    </table>
    <table>
      <thead><tr><th>Line</th><th>Name</th><th>Kind</th><th>Source</th><th>Lowering</th><th>Executor</th><th>Fallback Reason</th><th>Estimate</th><th>Operations</th></tr></thead>
      <tbody>{rows}</tbody>
    </table>"#,
        html_escape(&plan.backend),
        html_escape(&plan.backend_selection.requested),
        html_escape(&plan.backend_selection.selected),
        html_escape(&plan.backend_selection.status),
        html_escape(&plan.backend_selection.reason),
        plan.candidate_count
    )
}

fn join_or_dash(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_owned()
    } else {
        values.join(", ")
    }
}

fn format_solver_tolerance(value: f64) -> String {
    if value.is_finite() && value.abs() > 0.0 && value.abs() < 0.000001 {
        format!("{value:.3e}")
    } else {
        format_alignment_number(value)
    }
}

fn render_component_solver_section(spec: &ReportSpec) -> String {
    if spec.assemblies.is_empty() {
        return String::new();
    }
    let rows = spec
        .assemblies
        .iter()
        .map(|assembly| {
            let solver_result = assembly.solver_result.as_ref();
            let residual_norm = solver_result
                .map(|result| format_alignment_number(result.residual_norm))
                .unwrap_or_else(|| "-".to_owned());
            let iteration_count = solver_result
                .map(|result| {
                    format!(
                        "{} / {}",
                        result.iteration_count, result.max_iterations
                    )
                })
                .unwrap_or_else(|| "-".to_owned());
            let tolerance = solver_result
                .map(|result| format_solver_tolerance(result.tolerance))
                .unwrap_or_else(|| "-".to_owned());
            let conditioning = solver_result
                .map(format_component_solver_conditioning_summary)
                .unwrap_or_else(|| "-".to_owned());
            let variables = solver_result
                .map(format_component_solver_variables_summary)
                .unwrap_or_else(|| "-".to_owned());
            let trajectories = solver_result
                .map(format_component_solver_trajectory_summary)
                .unwrap_or_else(|| "-".to_owned());
            let step_diagnostics = solver_result
                .map(format_component_solver_step_diagnostics_summary)
                .unwrap_or_else(|| "-".to_owned());
            let largest_residual = solver_result
                .and_then(format_component_largest_residual_summary)
                .unwrap_or_else(|| "-".to_owned());
            let failure = solver_result
                .and_then(|result| result.failure_artifact.as_ref())
                .map(|failure| failure.code.clone())
                .unwrap_or_else(|| "-".to_owned());
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}/{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                assembly.line,
                html_escape(&assembly.name),
                html_escape(&assembly.status),
                assembly.boundary.equation_count,
                assembly.boundary.unknown_count,
                html_escape(&assembly.residual_graph.status),
                html_escape(&assembly.residual_graph.solver_plan),
                html_escape(&residual_norm),
                html_escape(&conditioning),
                html_escape(&tolerance),
                html_escape(&iteration_count),
                html_escape(&variables),
                html_escape(&trajectories),
                html_escape(&step_diagnostics),
                html_escape(&largest_residual),
                html_escape(&failure)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<h2>Connection Constraint Check</h2>
    <table>
      <thead><tr><th>Line</th><th>Assembly</th><th>Status</th><th>Eq/Unknowns</th><th>Convergence</th><th>Method</th><th>Residual Norm</th><th>Linear Conditioning</th><th>Tolerance</th><th>Iterations</th><th>Variables</th><th>Trajectories</th><th>Step Diagnostics</th><th>Largest Residual</th><th>Failure</th></tr></thead>
      <tbody>{rows}</tbody>
    </table>"#
    )
}

fn format_component_solver_conditioning_summary(result: &ReportComponentSolverResult) -> String {
    let Some(condition_estimate) = result.linear_condition_estimate else {
        return "-".to_owned();
    };
    let min_pivot = result
        .linear_minimum_pivot_abs
        .map(format_alignment_number)
        .unwrap_or_else(|| "-".to_owned());
    let max_pivot = result
        .linear_maximum_pivot_abs
        .map(format_alignment_number)
        .unwrap_or_else(|| "-".to_owned());
    format!(
        "pivot_ratio={} min={} max={}",
        format_alignment_number(condition_estimate),
        min_pivot,
        max_pivot
    )
}

fn format_component_solver_variables_summary(result: &ReportComponentSolverResult) -> String {
    if result.variables.is_empty() {
        return "-".to_owned();
    }
    let mut values = result
        .variables
        .iter()
        .take(4)
        .map(|variable| {
            format!(
                "{}={} {}",
                variable.name,
                format_alignment_number(variable.value),
                variable.unit
            )
        })
        .collect::<Vec<_>>();
    if result.variables.len() > values.len() {
        values.push(format!("+{} more", result.variables.len() - values.len()));
    }
    values.join(", ")
}

fn format_component_solver_trajectory_summary(result: &ReportComponentSolverResult) -> String {
    if result.trajectories.is_empty() {
        return "-".to_owned();
    }
    let mut values = result
        .trajectories
        .iter()
        .take(3)
        .map(|trajectory| {
            format!(
                "{}:{} {}->{} {} ({} pts)",
                trajectory.role,
                trajectory.name,
                format_alignment_number(trajectory.initial_value),
                format_alignment_number(trajectory.final_value),
                trajectory.unit,
                trajectory.point_count
            )
        })
        .collect::<Vec<_>>();
    if result.trajectories.len() > values.len() {
        values.push(format!(
            "+{} more",
            result.trajectories.len() - values.len()
        ));
    }
    values.join(", ")
}

fn format_component_solver_step_diagnostics_summary(
    result: &ReportComponentSolverResult,
) -> String {
    if result.step_diagnostics.is_empty() {
        return "-".to_owned();
    }
    let jacobian_policy = result
        .step_diagnostics
        .iter()
        .find_map(|diagnostic| diagnostic.jacobian_policy.as_deref());
    let jacobian_summary = jacobian_policy
        .map(|policy| format!(" jacobian={policy}"))
        .unwrap_or_default();
    let failed = result
        .step_diagnostics
        .iter()
        .find(|diagnostic| diagnostic.failure_artifact.is_some());
    if let Some(diagnostic) = failed {
        return format!(
            "steps={} failed@{} {}{}",
            result.step_diagnostics.len(),
            diagnostic.step_index,
            diagnostic
                .failure_artifact
                .as_ref()
                .map(|failure| failure.code.as_str())
                .unwrap_or("-"),
            jacobian_summary
        );
    }
    let max_residual = result
        .step_diagnostics
        .iter()
        .map(|diagnostic| diagnostic.residual_norm.abs())
        .fold(0.0, f64::max);
    let largest_step_residual = result
        .step_diagnostics
        .iter()
        .filter_map(|diagnostic| {
            Some((
                diagnostic.step_index,
                diagnostic.largest_residual_name.as_deref()?,
                diagnostic.largest_residual_value?,
                diagnostic.largest_residual_abs_value?,
            ))
        })
        .max_by(|left, right| left.3.total_cmp(&right.3));
    if let Some((step_index, name, value, _abs_value)) = largest_step_residual {
        return format!(
            "steps={} max_residual={} largest_step_residual={}@{}={}{}",
            result.step_diagnostics.len(),
            format_alignment_number(max_residual),
            name,
            step_index,
            format_alignment_number(value),
            jacobian_summary
        );
    }
    format!(
        "steps={} max_residual={}{}",
        result.step_diagnostics.len(),
        format_alignment_number(max_residual),
        jacobian_summary
    )
}

fn format_component_largest_residual_summary(
    result: &ReportComponentSolverResult,
) -> Option<String> {
    let residual = result.largest_residuals.first().or_else(|| {
        result.residuals.iter().max_by(|left, right| {
            left.normalized_value
                .abs()
                .total_cmp(&right.normalized_value.abs())
        })
    })?;
    let source_line = residual
        .source_line
        .map(|line| line.to_string())
        .unwrap_or_else(|| "-".to_owned());
    let dependencies = if residual.dependencies.is_empty() {
        "-".to_owned()
    } else {
        residual
            .dependencies
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    };
    Some(format!(
        "{}={} {}, normalized={} ({}), source_line={}, deps=[{}]",
        residual.name,
        format_alignment_number(residual.value),
        residual.unit,
        format_alignment_number(residual.normalized_value),
        residual.status,
        source_line,
        dependencies
    ))
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_time_axes_section(spec: &ReportSpec) -> String {
    if spec.time_axes.is_empty() {
        return String::new();
    }
    let rows = spec
        .time_axes
        .iter()
        .map(|axis| {
            let start = format_optional_alignment_number(axis.start);
            let end = format_optional_alignment_number(axis.end);
            let step = format_alignment_step(axis.nominal_step, axis.irregular);
            let status = if axis.irregular { "irregular" } else { "regular" };
            format!(
                "<tr><td>{}</td><td>{}.{}</td><td>{} - {}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                html_escape(&axis.name),
                html_escape(&axis.source_table),
                html_escape(&axis.source_column),
                html_escape(&start),
                html_escape(&end),
                axis.count,
                html_escape(&step),
                axis.missing_count,
                status
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<h2>Time Axes</h2>
    <table>
      <thead><tr><th>Name</th><th>Source</th><th>Range</th><th>Count</th><th>Nominal Step</th><th>Missing</th><th>Status</th></tr></thead>
      <tbody>{rows}</tbody>
    </table>"#
    )
}

fn format_domain_signature(name: &str, parameters: &[DomainTypeParameterInfo]) -> String {
    if parameters.is_empty() {
        name.to_owned()
    } else {
        format!("{name}[{}]", format_domain_parameter_list(parameters))
    }
}

fn format_domain_parameter_list(parameters: &[DomainTypeParameterInfo]) -> String {
    if parameters.is_empty() {
        "-".to_owned()
    } else {
        parameters
            .iter()
            .map(|parameter| parameter.display.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn uncertainty_transform_label(
    scale: Option<&str>,
    offset: Option<&str>,
    error: Option<&str>,
) -> String {
    let mut labels = Vec::new();
    if let Some(scale) = scale {
        labels.push(format!("scale={scale}"));
    }
    if let Some(offset) = offset {
        labels.push(format!("offset={offset}"));
    }
    if let Some(error) = error {
        labels.push(format!("error={error}"));
    }
    labels.join(", ")
}

fn uncertainty_propagation_label(terms: &[eng_compiler::UncertaintyPropagationTerm]) -> String {
    terms
        .iter()
        .map(|term| format!("{}:{}[{}]", term.source, term.role, term.quantity_kind))
        .collect::<Vec<_>>()
        .join(", ")
}

fn xml_escape(value: &str) -> String {
    html_escape(value)
}

fn write_report_component_graph_json(json: &mut String, graph: &ReportComponentGraph) {
    json.push_str("  \"component_graph\": {\n");
    json.push_str(&format!(
        "    \"format\": \"{}\",\n",
        json_escape(&graph.format)
    ));
    json.push_str(&format!(
        "    \"status\": \"{}\",\n",
        json_escape(&graph.status)
    ));
    json.push_str(&format!("    \"node_count\": {},\n", graph.node_count));
    json.push_str(&format!("    \"edge_count\": {},\n", graph.edge_count));
    json.push_str("    \"components\": [\n");
    for (component_index, component) in graph.components.iter().enumerate() {
        if component_index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"id\": \"{}\",\n",
            json_escape(&component.id)
        ));
        json.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&component.kind)
        ));
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&component.name)
        ));
        json.push_str(&format!(
            "        \"port_count\": {},\n",
            component.port_count
        ));
        json.push_str("        \"ports\": [");
        push_json_string_array(json, &component.ports);
        json.push_str("],\n");
        json.push_str(&format!("        \"line\": {},\n", component.line));
        write_report_source_span_json(json, "        ", &component.source_span, false);
        json.push_str("\n      }");
    }
    json.push_str("\n    ],\n");

    json.push_str("    \"ports\": [\n");
    for (port_index, port) in graph.ports.iter().enumerate() {
        if port_index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!("        \"id\": \"{}\",\n", json_escape(&port.id)));
        json.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&port.kind)
        ));
        json.push_str(&format!(
            "        \"component\": \"{}\",\n",
            json_escape(&port.component)
        ));
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&port.name)
        ));
        json.push_str(&format!(
            "        \"domain_label\": \"{}\",\n",
            json_escape(&port.domain_label)
        ));
        json.push_str(&format!(
            "        \"domain_name\": \"{}\",\n",
            json_escape(&port.domain_name)
        ));
        json.push_str("        \"type_arguments\": [");
        push_json_string_array(json, &port.type_arguments);
        json.push_str("],\n");
        push_optional_json_string(json, "medium_label", port.medium_label.as_deref(), 8);
        push_optional_json_string(json, "frame_label", port.frame_label.as_deref(), 8);
        push_optional_json_string(json, "axis_label", port.axis_label.as_deref(), 8);
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&port.status)
        ));
        json.push_str(&format!("        \"line\": {},\n", port.line));
        write_report_source_span_json(json, "        ", &port.source_span, false);
        json.push_str("\n      }");
    }
    json.push_str("\n    ],\n");

    json.push_str("    \"connections\": [\n");
    for (connection_index, connection) in graph.connections.iter().enumerate() {
        if connection_index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"id\": \"{}\",\n",
            json_escape(&connection.id)
        ));
        json.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&connection.kind)
        ));
        json.push_str(&format!(
            "        \"left\": \"{}\",\n",
            json_escape(&connection.left)
        ));
        json.push_str(&format!(
            "        \"right\": \"{}\",\n",
            json_escape(&connection.right)
        ));
        json.push_str(&format!(
            "        \"left_component\": \"{}\",\n",
            json_escape(&connection.left_component)
        ));
        json.push_str(&format!(
            "        \"left_port\": \"{}\",\n",
            json_escape(&connection.left_port)
        ));
        json.push_str(&format!(
            "        \"right_component\": \"{}\",\n",
            json_escape(&connection.right_component)
        ));
        json.push_str(&format!(
            "        \"right_port\": \"{}\",\n",
            json_escape(&connection.right_port)
        ));
        json.push_str(&format!(
            "        \"domain_label\": \"{}\",\n",
            json_escape(&connection.domain_label)
        ));
        push_optional_json_string(json, "medium_label", connection.medium_label.as_deref(), 8);
        push_optional_json_string(json, "frame_label", connection.frame_label.as_deref(), 8);
        push_optional_json_string(json, "axis_label", connection.axis_label.as_deref(), 8);
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&connection.status)
        ));
        json.push_str(&format!("        \"line\": {},\n", connection.line));
        write_report_source_span_json(json, "        ", &connection.source_span, false);
        json.push_str("\n      }");
    }
    json.push_str("\n    ],\n");

    json.push_str("    \"connection_sets\": [\n");
    for (set_index, connection_set) in graph.connection_sets.iter().enumerate() {
        if set_index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"assembly\": \"{}\",\n",
            json_escape(&connection_set.assembly)
        ));
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&connection_set.name)
        ));
        json.push_str(&format!(
            "        \"domain_label\": \"{}\",\n",
            json_escape(&connection_set.domain_label)
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&connection_set.status)
        ));
        json.push_str(&format!(
            "        \"connection_count\": {},\n",
            connection_set.connection_count
        ));
        json.push_str("        \"ports\": [");
        push_json_string_array(json, &connection_set.ports);
        json.push_str("],\n");
        json.push_str(&format!("        \"line\": {},\n", connection_set.line));
        write_report_source_span_json(json, "        ", &connection_set.source_span, false);
        json.push_str("\n      }");
    }
    json.push_str("\n    ],\n");

    json.push_str("    \"behavior_nodes\": [\n");
    for (node_index, node) in graph.behavior_nodes.iter().enumerate() {
        if node_index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!("        \"id\": \"{}\",\n", json_escape(&node.id)));
        json.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&node.kind)
        ));
        json.push_str(&format!(
            "        \"behavior_kind\": \"{}\",\n",
            json_escape(&node.behavior_kind)
        ));
        json.push_str(&format!(
            "        \"component\": \"{}\",\n",
            json_escape(&node.component)
        ));
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&node.name)
        ));
        json.push_str(&format!(
            "        \"expression\": \"{}\",\n",
            json_escape(&node.expression)
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&node.status)
        ));
        push_optional_json_string(json, "signal", node.signal.as_deref(), 8);
        push_optional_json_f64(json, "delay_s", node.delay_s, 8);
        push_optional_json_string(
            json,
            "relationship_status",
            node.relationship_status.as_deref(),
            8,
        );
        push_optional_json_string(json, "contract_status", node.contract_status.as_deref(), 8);
        push_optional_json_string(json, "jacobian_policy", node.jacobian_policy.as_deref(), 8);
        push_optional_json_string(json, "profile_policy", node.profile_policy.as_deref(), 8);
        write_behavior_signal_contracts_json(json, "contract_inputs", &node.contract_inputs, 8);
        write_behavior_signal_contracts_json(json, "contract_outputs", &node.contract_outputs, 8);
        json.push_str("        \"diagnostic_channels\": [");
        push_json_string_array(json, &node.diagnostic_channels);
        json.push_str("],\n");
        push_optional_json_string(
            json,
            "runtime_warning_status",
            node.runtime_warning_status.as_deref(),
            8,
        );
        json.push_str(&format!("        \"line\": {},\n", node.line));
        write_report_source_span_json(json, "        ", &node.source_span, false);
        json.push_str("\n      }");
    }
    json.push_str("\n    ]\n");
    json.push_str("  }");
}

fn write_behavior_signal_contracts_json(
    json: &mut String,
    key: &str,
    contracts: &[ReportBehaviorSignalContract],
    indent: usize,
) {
    let spaces = " ".repeat(indent);
    json.push_str(&format!("{spaces}\"{key}\": [\n"));
    for (index, contract) in contracts.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{spaces}  {{\n"));
        json.push_str(&format!(
            "{spaces}    \"role\": \"{}\",\n",
            json_escape(&contract.role)
        ));
        json.push_str(&format!(
            "{spaces}    \"name\": \"{}\",\n",
            json_escape(&contract.name)
        ));
        json.push_str(&format!(
            "{spaces}    \"quantity_kind\": \"{}\",\n",
            json_escape(&contract.quantity_kind)
        ));
        json.push_str(&format!(
            "{spaces}    \"display_unit\": \"{}\",\n",
            json_escape(&contract.display_unit)
        ));
        json.push_str(&format!(
            "{spaces}    \"canonical_unit\": \"{}\",\n",
            json_escape(&contract.canonical_unit)
        ));
        json.push_str(&format!(
            "{spaces}    \"status\": \"{}\"\n",
            json_escape(&contract.status)
        ));
        json.push_str(&format!("{spaces}  }}"));
    }
    json.push_str(&format!("\n{spaces}],\n"));
}

fn write_report_source_span_json(
    json: &mut String,
    indent: &str,
    span: &ReportSourceSpan,
    trailing_comma: bool,
) {
    json.push_str(&format!(
        "{}\"source_span\": {{ \"line\": {}, \"column\": {} }}{}",
        indent,
        span.line,
        span.column,
        if trailing_comma { "," } else { "" }
    ));
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn push_optional_json_string(json: &mut String, key: &str, value: Option<&str>, indent: usize) {
    let spaces = " ".repeat(indent);
    match value {
        Some(value) => json.push_str(&format!("{spaces}\"{key}\": \"{}\",\n", json_escape(value))),
        None => json.push_str(&format!("{spaces}\"{key}\": null,\n")),
    }
}

fn push_optional_json_usize(json: &mut String, key: &str, value: Option<usize>, indent: usize) {
    let spaces = " ".repeat(indent);
    match value {
        Some(value) => json.push_str(&format!("{spaces}\"{key}\": {value},\n")),
        None => json.push_str(&format!("{spaces}\"{key}\": null,\n")),
    }
}

fn push_optional_json_f64(json: &mut String, key: &str, value: Option<f64>, indent: usize) {
    let spaces = " ".repeat(indent);
    match value {
        Some(value) => json.push_str(&format!("{spaces}\"{key}\": {value},\n")),
        None => json.push_str(&format!("{spaces}\"{key}\": null,\n")),
    }
}

fn push_optional_json_matrix(json: &mut String, matrix: Option<&[Vec<f64>]>) {
    let Some(matrix) = matrix else {
        json.push_str("null");
        return;
    };
    json.push('[');
    for (row_index, row) in matrix.iter().enumerate() {
        if row_index > 0 {
            json.push_str(", ");
        }
        json.push('[');
        for (column_index, value) in row.iter().enumerate() {
            if column_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format_json_number(*value));
        }
        json.push(']');
    }
    json.push(']');
}

fn format_json_number(value: f64) -> String {
    if value.is_finite() {
        value.to_string()
    } else {
        "null".to_owned()
    }
}

fn push_linear_operator_entries_json(
    json: &mut String,
    entries: &[ReportLinearOperatorEntry],
    indent: usize,
) {
    let spaces = " ".repeat(indent);
    json.push_str(&format!("{spaces}\"canonical_entries\": [\n"));
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{spaces}  {{\n"));
        json.push_str(&format!(
            "{spaces}    \"row_index\": {},\n",
            entry.row_index
        ));
        json.push_str(&format!(
            "{spaces}    \"column_index\": {},\n",
            entry.column_index
        ));
        json.push_str(&format!(
            "{spaces}    \"row_member\": \"{}\",\n",
            json_escape(&entry.row_member)
        ));
        json.push_str(&format!(
            "{spaces}    \"column_member\": \"{}\",\n",
            json_escape(&entry.column_member)
        ));
        json.push_str(&format!(
            "{spaces}    \"coefficient\": {}\n",
            format_json_number(entry.coefficient)
        ));
        json.push_str(&format!("{spaces}  }}"));
    }
    json.push_str(&format!("\n{spaces}],\n"));
}

fn push_json_f64_array(json: &mut String, values: &[f64]) {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&value.to_string());
    }
}

fn push_json_string_array(json: &mut String, values: &[String]) {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!("\"{}\"", json_escape(value)));
    }
}

fn push_uncertainty_propagation_terms(
    json: &mut String,
    terms: &[ReportUncertaintyPropagationTerm],
) {
    for (index, term) in terms.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!(
            "{{\"source\": \"{}\", \"role\": \"{}\", \"quantity_kind\": \"{}\"}}",
            json_escape(&term.source),
            json_escape(&term.role),
            json_escape(&term.quantity_kind)
        ));
    }
}

fn default_plot_spec(title: &str) -> PlotSpec {
    PlotSpec {
        title: title.to_owned(),
        plot_type: "line".to_owned(),
        x_axis: PlotAxis {
            name: "Time".to_owned(),
            label: "Time".to_owned(),
            unit: "sample".to_owned(),
        },
        y_axis: PlotAxis {
            name: "Value".to_owned(),
            label: "unit-aware value".to_owned(),
            unit: "sample".to_owned(),
        },
        series: vec![PlotSeries {
            name: "sample".to_owned(),
            quantity_kind: "Value".to_owned(),
            display_unit: "sample".to_owned(),
            bins: Vec::new(),
            points: sample_points(),
            confidence_band: None,
        }],
    }
}

fn sample_points() -> Vec<PlotPoint> {
    vec![
        PlotPoint { x: 0.0, y: 20.0 },
        PlotPoint { x: 1.0, y: 32.0 },
        PlotPoint { x: 2.0, y: 36.0 },
        PlotPoint { x: 3.0, y: 54.0 },
        PlotPoint { x: 4.0, y: 61.0 },
        PlotPoint { x: 5.0, y: 78.0 },
        PlotPoint { x: 6.0, y: 74.0 },
        PlotPoint { x: 7.0, y: 96.0 },
    ]
}

fn axis_label(axis: &PlotAxis) -> String {
    if axis.unit.is_empty() {
        axis.label.clone()
    } else {
        format!("{} ({})", axis.label, axis.unit)
    }
}

fn svg_line_series(series: &[PlotSeries]) -> String {
    let all_points = series
        .iter()
        .flat_map(|series| {
            series.points.iter().chain(
                series
                    .confidence_band
                    .as_ref()
                    .into_iter()
                    .flat_map(|band| band.lower.iter().chain(band.upper.iter())),
            )
        })
        .copied()
        .collect::<Vec<_>>();
    if all_points.is_empty() {
        return String::new();
    }
    let min_x = all_points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let max_x = all_points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = all_points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let max_y = all_points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let x_span = (max_x - min_x).max(1.0);
    let y_span = (max_y - min_y).max(1.0);
    let colors = ["#0b6bcb", "#c2410c", "#15803d", "#7c3aed", "#b45309"];
    let mut svg = String::new();
    for (index, series) in series.iter().enumerate() {
        let color = colors[index % colors.len()];
        if let Some(band) = &series.confidence_band {
            let lower = band.lower.iter().map(|point| {
                let x = 72.0 + ((point.x - min_x) / x_span) * 588.0;
                let y = 250.0 - ((point.y - min_y) / y_span) * 210.0;
                format!("{x:.1},{y:.1}")
            });
            let upper = band.upper.iter().rev().map(|point| {
                let x = 72.0 + ((point.x - min_x) / x_span) * 588.0;
                let y = 250.0 - ((point.y - min_y) / y_span) * 210.0;
                format!("{x:.1},{y:.1}")
            });
            let polygon = lower.chain(upper).collect::<Vec<_>>().join(" ");
            if !polygon.is_empty() {
                svg.push_str(&format!(
                    r##"<polygon points="{polygon}" fill="{color}" fill-opacity="0.18" stroke="none" data-confidence-band="{}" data-confidence-level="{}"/>
  "##,
                    xml_escape(&band.source),
                    band.level
                ));
            }
        }
        let points = series
            .points
            .iter()
            .map(|point| {
                let x = 72.0 + ((point.x - min_x) / x_span) * 588.0;
                let y = 250.0 - ((point.y - min_y) / y_span) * 210.0;
                format!("{x:.1},{y:.1}")
            })
            .collect::<Vec<_>>()
            .join(" ");
        let label_y = 52 + index as i32 * 18;
        svg.push_str(&format!(
            r##"<polyline points="{points}" fill="none" stroke="{color}" stroke-width="4"/>
  <circle cx="548" cy="{label_y}" r="5" fill="{color}"/>
  <text x="560" y="{}" font-family="Segoe UI, Arial, sans-serif" font-size="12" fill="#111">{}</text>
  "##,
            label_y + 4,
            xml_escape(&series.name)
        ));
    }
    svg
}

fn svg_rect_plot(points: &[PlotPoint], fill: &str, width_fraction: f64) -> String {
    if points.is_empty() {
        return String::new();
    }

    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min)
        .min(0.0);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max)
        .max(0.0);
    let x_span = (max_x - min_x).max(1.0);
    let y_span = (max_y - min_y).max(1.0);
    let slot_width = 588.0 / points.len().max(1) as f64;
    let bar_width = (slot_width * width_fraction).clamp(4.0, 72.0);
    let baseline_y = 250.0 - ((0.0 - min_y) / y_span) * 210.0;

    points
        .iter()
        .map(|point| {
            let center_x = 72.0 + ((point.x - min_x) / x_span) * 588.0;
            let value_y = 250.0 - ((point.y - min_y) / y_span) * 210.0;
            let x = center_x - bar_width * 0.5;
            let y = value_y.min(baseline_y);
            let height = (baseline_y - value_y).abs().max(1.0);
            format!(
                r#"<rect x="{x:.0}" y="{y:.0}" width="{bar_width:.0}" height="{height:.0}" fill="{fill}"/>"#
            )
        })
        .collect::<Vec<_>>()
        .join("\n  ")
}

fn svg_histogram_plot(series: Option<&PlotSeries>) -> String {
    let Some(series) = series else {
        return String::new();
    };
    if series.bins.is_empty() {
        return svg_rect_plot(&series.points, "#4b7f52", 0.92);
    }

    let min_x = series
        .bins
        .iter()
        .map(|bin| bin.lower.min(bin.upper))
        .fold(f64::INFINITY, f64::min);
    let max_x = series
        .bins
        .iter()
        .map(|bin| bin.lower.max(bin.upper))
        .fold(f64::NEG_INFINITY, f64::max);
    if (max_x - min_x).abs() <= f64::EPSILON {
        return svg_rect_plot(&series.points, "#4b7f52", 0.92);
    }
    let y_span = series
        .bins
        .iter()
        .map(|bin| bin.count)
        .max()
        .unwrap_or(1)
        .max(1) as f64;

    series
        .bins
        .iter()
        .map(|bin| {
            let x1 = 72.0 + ((bin.lower - min_x) / (max_x - min_x)) * 588.0;
            let x2 = 72.0 + ((bin.upper - min_x) / (max_x - min_x)) * 588.0;
            let x = x1.min(x2);
            let width = (x2 - x1).abs().max(2.0);
            let value_y = 250.0 - (bin.count as f64 / y_span) * 210.0;
            let height = (250.0 - value_y).max(1.0);
            format!(
                r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="#4b7f52" opacity="0.88" data-bin-lower="{:.6}" data-bin-upper="{:.6}" data-bin-count="{}"/>"##,
                x,
                value_y,
                width,
                height,
                bin.lower,
                bin.upper,
                bin.count
            )
        })
        .collect::<Vec<_>>()
        .join("\n  ")
}

fn svg_scatter_plot(points: &[PlotPoint], fill: &str) -> String {
    if points.is_empty() {
        return String::new();
    }

    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let x_span = (max_x - min_x).max(1.0);
    let y_span = (max_y - min_y).max(1.0);

    points
        .iter()
        .map(|point| {
            let x = 72.0 + ((point.x - min_x) / x_span) * 588.0;
            let y = 250.0 - ((point.y - min_y) / y_span) * 210.0;
            format!(r#"<circle cx="{x:.0}" cy="{y:.0}" r="5" fill="{fill}"/>"#)
        })
        .collect::<Vec<_>>()
        .join("\n  ")
}

fn time_series_quantity(quantity_kind: &str) -> Option<(String, String)> {
    let rest = quantity_kind.strip_prefix("TimeSeries[")?;
    let (axis, after_axis) = rest.split_once(']')?;
    let quantity = after_axis.trim().strip_prefix("of ")?;
    Some((axis.trim().to_owned(), quantity.trim().to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use eng_compiler::{check_source, CheckOptions};
    use serde_json::json;

    #[test]
    fn plotspec_uses_timeseries_axis_unit_labels() {
        let report = check_source(
            "ok.eng",
            "sensor = promote csv \"data/sensor.csv\" as SensorData\n    cp = 4180 J/kg/K\n    Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\n}\n",
            &CheckOptions::default(),
        );

        let spec = plot_spec_from_report(&report);
        let json = plot_spec_json(&spec);
        let svg = render_svg_from_spec(&spec);

        assert_eq!(spec.plot_type, "line");
        assert_eq!(spec.x_axis.label, "Time");
        assert_eq!(spec.y_axis.unit, "W");
        assert!(json.contains("\"format\": \"eng-plotspec-v1\""));
        assert!(svg.contains("HeatRate (W)"));
    }

    #[test]
    fn plotspec_renders_bar_and_histogram_seeds() {
        let mut spec = sample_plot_spec("bar");
        let bar_json = plot_spec_json(&spec);
        let bar_svg = render_svg_from_spec(&spec);
        assert!(bar_json.contains("\"plot_type\": \"bar\""));
        assert!(bar_svg.contains("<rect x="));
        assert!(!bar_svg.contains("<polyline"));

        "histogram".clone_into(&mut spec.plot_type);
        let histogram_json = plot_spec_json(&spec);
        let histogram_svg = render_svg_from_spec(&spec);
        assert!(histogram_json.contains("\"plot_type\": \"histogram\""));
        assert!(histogram_svg.contains("<rect x="));
        assert!(!histogram_svg.contains("<polyline"));

        spec.series[0].bins = vec![
            PlotBin {
                lower: 0.0,
                upper: 1.0,
                center: 0.5,
                count: 2,
            },
            PlotBin {
                lower: 1.0,
                upper: 2.0,
                center: 1.5,
                count: 1,
            },
        ];
        let binned_json = plot_spec_json(&spec);
        let binned_svg = render_svg_from_spec(&spec);
        assert!(binned_json.contains("\"bins\""));
        assert!(binned_json.contains("\"count\": 2"));
        assert!(binned_svg.contains("data-bin-lower"));
        assert!(binned_svg.contains("data-bin-count=\"2\""));

        spec.series[0].bins.clear();
        "scatter".clone_into(&mut spec.plot_type);
        let scatter_json = plot_spec_json(&spec);
        let scatter_svg = render_svg_from_spec(&spec);
        assert!(scatter_json.contains("\"plot_type\": \"scatter\""));
        assert!(scatter_svg.contains("<circle cx="));
        assert!(!scatter_svg.contains("<polyline"));
    }

    #[test]
    fn plotspec_renders_multi_series_line_legend_and_manifest_series() {
        let mut spec = sample_plot_spec("line");
        spec.series.push(PlotSeries {
            name: "baseline".to_owned(),
            quantity_kind: "HeatRate".to_owned(),
            display_unit: "kW".to_owned(),
            bins: Vec::new(),
            points: vec![
                PlotPoint { x: 0.0, y: 0.8 },
                PlotPoint { x: 1.0, y: 1.8 },
                PlotPoint { x: 2.0, y: 1.2 },
            ],
            confidence_band: None,
        });

        let svg = render_svg_from_spec(&spec);
        let manifest = plot_manifest_json(&spec, "timeseries.svg", "plot-hash", "svg-hash");

        assert_eq!(svg.matches("<polyline").count(), 2);
        assert!(svg.contains(">value<"));
        assert!(svg.contains(">baseline<"));
        assert!(manifest.contains("\"series\": [\"value\", \"baseline\"]"));
    }

    #[test]
    fn report_spec_collects_v07_review_tables() {
        let report = check_source(
            "ok.eng",
            "schema SensorData {\n    time: DateTime index\n    T_supply: AbsoluteTemperature [degC]\n}\n\npower = 10 kW\n    L = 1 m + 20 cm\n}\n",
            &CheckOptions::default(),
        );

        let spec = report_spec_from_report(&report, "plots/plot_manifest.json", "abc123");
        let json = report_spec_json(&spec);

        assert_eq!(spec.plot_manifest.path, "plots/plot_manifest.json");
        assert_eq!(spec.plot_manifest.hash, "abc123");
        assert!(spec.variables.iter().any(|variable| variable.name == "L"));
        assert_eq!(spec.schemas[0].name, "SensorData");
        assert!(spec
            .warnings
            .iter()
            .any(|warning| warning.code == "W-QTY-AMBIG-001"));
        assert!(json.contains("\"format\": \"eng-report-spec-v1\""));
        assert!(json.contains("\"variable_table\""));
        assert!(json.contains("\"inferred_declaration_table\""));
        assert!(json.contains("\"unit_conversion_table\""));
        assert!(json.contains("\"schema_summary\""));
        assert!(json.contains("\"plot_manifest\""));
        assert!(json.contains("\"warning_list\""));
    }

    #[test]
    fn report_spec_and_html_include_uncertainty_metadata() {
        let report = check_source(
            "ok.eng",
            "Q_dist = normal(mean=5 kW, std=0.8 kW, samples=31)\n    Q_unc = propagate(Q_dist, method=linear, scale=1.1, offset=0.2 kW)\n}\n",
            &CheckOptions::default(),
        );

        let spec = report_spec_from_report(&report, "plots/plot_manifest.json", "abc123");
        let json = report_spec_json(&spec);
        let html = render_html(&report, "plots/timeseries.svg");

        assert_eq!(spec.uncertainty.len(), 2);
        assert_eq!(spec.uncertainty[0].kind, "Distribution");
        assert_eq!(spec.uncertainty[0].display_unit, "kW");
        assert_eq!(spec.uncertainty[0].sample_count, 31);
        assert_eq!(spec.uncertainty[1].scale.as_deref(), Some("1.1"));
        assert_eq!(spec.uncertainty[1].offset.as_deref(), Some("0.2 kW"));
        assert_eq!(spec.uncertainty[1].propagation.len(), 1);
        assert_eq!(spec.uncertainty[1].propagation[0].source, "Q_dist");
        assert!(json.contains("\"uncertainty\""));
        assert!(json.contains("\"scale\": \"1.1\""));
        assert!(json.contains("\"offset\": \"0.2 kW\""));
        assert!(json.contains("\"propagation\""));
        assert!(json.contains("\"source\": \"Q_dist\""));
        assert!(json.contains("\"Q_unc\""));
        assert!(html.contains("Uncertainty"));
        assert!(html.contains("scale=1.1, offset=0.2 kW"));
        assert!(html.contains("Q_dist:Distribution[HeatRate]"));
        assert!(html.contains("Q_dist"));
    }

    #[test]
    fn report_spec_and_html_include_ml_metadata() {
        let report = check_source(
            "ok.eng",
            "split = train_test_split(Q_coil, target=Q_coil, features=[T_supply, T_return], test=0.5, seed=7)\n    reg_model = regression(split, algorithm=linear)\n    reg_eval = evaluate(reg_model, split=split)\n}\n",
            &CheckOptions::default(),
        );

        let spec = report_spec_from_report(&report, "plots/plot_manifest.json", "abc123");
        let json = report_spec_json(&spec);
        let html = render_html(&report, "plots/timeseries.svg");

        assert_eq!(spec.ml.len(), 3);
        assert_eq!(spec.ml[0].kind, "TrainTestSplit");
        assert_eq!(spec.ml[1].kind, "RegressionModel");
        assert!(json.contains("\"ml\""));
        assert!(json.contains("\"ModelMetrics\""));
        assert!(html.contains("ML Models"));
        assert!(html.contains("reg_model"));
    }

    #[test]
    fn report_spec_and_html_include_kernel_plan_metadata() {
        let source = include_str!("../../../examples/official/01_csv_plot/main.eng");
        let report = check_source(
            "examples/official/01_csv_plot/main.eng",
            source,
            &CheckOptions::default(),
        );

        let spec = report_spec_from_report(&report, "plots/plot_manifest.json", "abc123");
        let json = report_spec_json(&spec);
        let html = render_html_with_spec(&report, "plots/timeseries.svg", &spec);

        assert_eq!(spec.kernel_plan.format, "eng-kernel-plan-v1");
        assert_eq!(spec.kernel_plan.backend, "interpreter-fallback");
        assert!(spec.kernel_plan.candidate_count >= 3);
        assert!(spec.kernel_plan.candidates.iter().any(|candidate| {
            candidate.kind == "timeseries_integrate"
                && candidate.executor_status == "interpreter_supported"
                && candidate.fallback_reason.contains("interpreter kernel IR")
        }));
        assert!(json.contains("\"kernel_plan\""));
        assert!(json.contains("\"kind\": \"timeseries_integrate\""));
        assert!(json.contains("\"executor\""));
        assert!(json.contains("candidate can execute through the interpreter kernel IR"));
        assert!(html.contains("Runtime Optimization Kernel Plan"));
        assert!(html.contains("interpreter_supported"));
    }

    #[test]
    fn report_spec_and_html_include_system_equation_summary() {
        let report = check_source(
            "ok.eng",
            "system RoomThermal {\n    parameter C: HeatCapacity = 500 kJ/K\n    parameter UA: Conductance = 150 W/K\n    state T: AbsoluteTemperature = 24 degC\n    input T_out: AbsoluteTemperature\n    input Q_internal: HeatRate\n    equation {\n        C * der(T) eq UA * (T_out - T) + Q_internal\n    }\n}\n",
            &CheckOptions::default(),
        );

        let spec = report_spec_from_report(&report, "plots/plot_manifest.json", "abc123");
        let json = report_spec_json(&spec);
        let html = render_html(&report, "plots/timeseries.svg");

        assert_eq!(spec.provenance.system_count, 1);
        assert_eq!(spec.provenance.equation_count, 1);
        assert_eq!(spec.systems[0].equations[0].status, "unit_consistent");
        assert_eq!(spec.system_ir[0].solver_boundary.status, "unsolved");
        assert_eq!(spec.system_ir[0].solver_plan.status, "metadata_only");
        assert_eq!(
            spec.system_ir[0].solver_plan.solve_order,
            vec!["RoomThermal.residual_1".to_owned()]
        );
        assert_eq!(
            spec.system_ir[0].solver_plan.jacobian_sparsity[0].with_respect_to,
            vec!["T".to_owned()]
        );
        assert_eq!(
            spec.system_ir[0].solver_plan.jacobian_sparsity[0].status,
            "sparsity_metadata"
        );
        assert_eq!(
            spec.system_ir[0].solver_plan.jacobian_seed[0].with_respect_to,
            vec!["T".to_owned()]
        );
        assert_eq!(
            spec.system_ir[0].solver_plan.jacobian_seed[0].status,
            "symbolic_seed"
        );
        assert_eq!(spec.system_ir[0].equations[0].dependencies.len(), 5);
        assert_eq!(
            spec.system_ir[0].equations[0].derivative_states,
            vec!["T".to_owned()]
        );
        assert!(json.contains("\"system_summary\""));
        assert!(json.contains("\"system_ir\""));
        assert!(json.contains("\"solver_boundary\""));
        assert!(json.contains("\"solver_plan\""));
        assert!(json.contains("\"jacobian_sparsity\""));
        assert!(json.contains("\"status\": \"sparsity_metadata\""));
        assert!(json.contains("\"jacobian_seed\""));
        assert!(json.contains("\"RoomThermal.residual_1\""));
        assert!(html.contains("System Equations"));
        assert!(html.contains("unit_consistent"));
    }

    #[test]
    fn report_spec_and_html_include_state_space_metadata() {
        let report = check_source(
            "ok.eng",
            "system ThermalStateSpaceMetadata {\n    state T_zone: AbsoluteTemperature = 22 degC\n    input T_out: AbsoluteTemperature = 8 degC\n    input Q_internal: HeatRate = 500 W\n    states x = [T_zone]\n    inputs u = [T_out, Q_internal]\n    outputs y = [T_zone]\n    A: LinearOperator[StateVector -> Derivative[StateVector]] = [[-0.012 1/min]]\n    B: LinearOperator[InputVector -> Derivative[StateVector]] = [[0.012 1/min, 0.001]]\n    equation {\n        der(x) eq A * x + B * u\n    }\n}\n",
            &CheckOptions::default(),
        );

        let spec = report_spec_from_report(&report, "plots/plot_manifest.json", "abc123");
        let json = report_spec_json(&spec);
        let html = render_html_with_spec(&report, "plots/timeseries.svg", &spec);

        assert_eq!(spec.state_space_vectors.len(), 3);
        assert_eq!(spec.linear_operators.len(), 2);
        assert_eq!(spec.state_space_vectors[0].vector_type, "StateVector");
        assert_eq!(spec.linear_operators[1].from, "InputVector");
        assert_eq!(spec.linear_operators[1].to, "Derivative[StateVector]");
        assert_eq!(
            spec.linear_operators[0].canonical_matrix.as_ref().unwrap()[0][0],
            -0.0002
        );
        assert_eq!(spec.linear_operators[1].canonical_entries.len(), 2);
        assert!(json.contains("\"state_space_vectors\""));
        assert!(json.contains("\"linear_operators\""));
        assert!(json.contains("\"canonical_matrix\": [[-0.0002]]"));
        assert!(json.contains("\"canonical_entries\""));
        assert!(json.contains("\"column_member\": \"Q_internal\""));
        assert!(json.contains("\"vector_type\": \"StateVector\""));
        assert!(json.contains("\"column_count\": 2"));
        assert!(html.contains("State-Space Metadata"));
        assert!(html.contains("Canonical Matrix"));
        assert!(html.contains("Canonical Entries"));
        assert!(html.contains("StateVector"));
        assert!(html.contains("InputVector"));
        assert!(html.contains("Derivative[StateVector]"));
    }

    #[test]
    fn report_spec_includes_component_constructor_provenance() {
        let report = check_source(
            "ok.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent RoomBoundary {\n    port heat: Thermal\n    boundary_T = heat.T = T_room\n    boundary_Q = heat.Q = Q_room\n}\n\ncomponent AmbientBoundary {\n    port heat: Thermal\n}\n\nsystem Envelope {\n    room = RoomBoundary(T_room=22 degC, Q_room=1 kW)\n    ambient = AmbientBoundary()\n    connect room.heat to ambient.heat\n}\n",
            &CheckOptions::default(),
        );

        let spec = report_spec_from_report(&report, "plots/plot_manifest.json", "abc123");
        let json = report_spec_json(&spec);
        let room = spec
            .components
            .iter()
            .find(|component| component.name == "room")
            .expect("room component");

        assert_eq!(room.template_name.as_deref(), Some("RoomBoundary"));
        assert_eq!(room.constructor_arguments.len(), 2);
        assert_eq!(room.constructor_arguments[0].name, "T_room");
        assert_eq!(room.constructor_arguments[0].value, "22 degC");
        assert!(json.contains("\"template_name\": \"RoomBoundary\""));
        assert!(json.contains("\"constructor_arguments\""));
        assert!(json.contains("\"value\": \"1 kW\""));
    }

    #[test]
    fn report_spec_and_html_include_domain_component_sections() {
        let report = check_source(
            "ok.eng",
            "domain Fluid[Medium M] package \"eng.std.domains.fluid\" version \"0.1.0\" {\n    across height: Length [m]\n    through m_dot: MassFlowRate [kg/s]\n    conservation sum(m_dot) = 0\n}\n\ncomponent Supply {\n    port outlet: Fluid[Water]\n    pressure_seed = delay(outlet.m_dot, 5 s)\n}\n\ncomponent Return {\n    port inlet: Fluid[Water]\n}\n\nconnect Supply.outlet -> Return.inlet\n",
            &CheckOptions::default(),
        );

        let spec = report_spec_from_report(&report, "plots/plot_manifest.json", "abc123");
        let json = report_spec_json(&spec);
        let html = render_html(&report, "plots/timeseries.svg");

        assert_eq!(spec.provenance.domain_count, 1);
        assert_eq!(spec.provenance.component_count, 2);
        assert_eq!(spec.provenance.connection_count, 1);
        assert_eq!(spec.provenance.assembly_count, 1);
        assert_eq!(
            spec.assemblies[0].residual_graph.residual_metadata.len(),
            spec.assemblies[0].equations.len()
        );
        assert!(spec.assemblies[0]
            .residual_graph
            .residual_metadata
            .iter()
            .any(
                |metadata| metadata.name == "connection_set_1.through_m_dot_conservation"
                    && metadata.kind == "through_conservation"
                    && metadata.source_expression.contains("sum(")
                    && metadata.dependencies.len() == 2
                    && metadata.line > 0
            ));
        assert_eq!(spec.domains[0].name, "Fluid");
        assert_eq!(spec.domains[0].type_parameters[0].kind, "Medium");
        assert_eq!(spec.domains[0].type_parameters[0].name, "M");
        assert_eq!(spec.domains[0].type_parameters[0].display, "Medium M");
        assert_eq!(
            spec.domains[0].package.as_deref(),
            Some("eng.std.domains.fluid")
        );
        assert_eq!(spec.domains[0].version.as_deref(), Some("0.1.0"));
        assert_eq!(spec.domains[0].variables[0].role, "across");
        assert_eq!(spec.components[0].ports[0].status, "domain_resolved");
        assert_eq!(spec.components[0].ports[0].domain, "Fluid[Water]");
        assert_eq!(
            spec.components[0].ports[0].type_arguments,
            vec!["Water".to_owned()]
        );
        assert_eq!(
            spec.components[0].local_expressions[0].name,
            "pressure_seed"
        );
        assert_eq!(spec.connections[0].status, "domain_compatible");
        assert_eq!(spec.component_graph.format, "eng-component-graph-v1");
        assert_eq!(spec.component_graph.node_count, 5);
        assert_eq!(spec.component_graph.edge_count, 1);
        assert_eq!(spec.component_graph.behavior_nodes.len(), 1);
        assert_eq!(
            spec.component_graph.behavior_nodes[0].behavior_kind,
            "delay"
        );
        assert_eq!(
            spec.component_graph.behavior_nodes[0].signal.as_deref(),
            Some("outlet.m_dot")
        );
        assert_eq!(spec.component_graph.behavior_nodes[0].delay_s, Some(5.0));
        assert_eq!(
            spec.component_graph.behavior_nodes[0]
                .relationship_status
                .as_deref(),
            Some("delay_relationship_metadata_only")
        );
        assert_eq!(
            spec.component_graph.behavior_nodes[0].contract_inputs[0].name,
            "outlet.m_dot"
        );
        assert_eq!(
            spec.component_graph.behavior_nodes[0].contract_inputs[0].quantity_kind,
            "MassFlowRate"
        );
        assert_eq!(
            spec.component_graph.behavior_nodes[0].contract_outputs[0].name,
            "pressure_seed"
        );
        assert_eq!(
            spec.component_graph.behavior_nodes[0].contract_outputs[0].quantity_kind,
            "MassFlowRate"
        );
        assert!(json.contains("\"residual_metadata\""));
        assert!(json.contains("\"source_expression\""));
        assert!(json.contains("\"connection_set_1.through_m_dot_conservation\""));
        assert!(spec.component_graph.behavior_nodes[0]
            .diagnostic_channels
            .contains(&"delay_history_underflow_failure".to_owned()));
        assert_eq!(
            spec.component_graph.ports[0].medium_label.as_deref(),
            Some("Water")
        );
        assert_eq!(spec.component_graph.connections[0].source_span.column, 1);
        assert_eq!(spec.assemblies[0].connection_sets.len(), 1);
        let assembly_span = spec.assemblies[0]
            .source_span
            .as_ref()
            .expect("component assembly source span");
        assert_eq!(assembly_span.line, 7);
        assert_eq!(assembly_span.column, 11);
        assert!(json.contains(
            "\"line\": 7,\n      \"source_span\": { \"line\": 7, \"column\": 11 },\n      \"component_count\""
        ));
        assert_eq!(spec.assemblies[0].local_expression_count, 1);
        assert_eq!(spec.assemblies[0].equations.len(), 2);
        assert!(spec.assemblies[0]
            .equations
            .iter()
            .any(|equation| equation.reason
                == "generated from through variable conservation within a connection set"));
        assert_eq!(spec.assemblies[0].boundary.unknown_count, 4);
        assert_eq!(spec.assemblies[0].domain_count, 1);
        assert_eq!(spec.assemblies[0].domain_plans[0].domain, "Fluid[Water]");
        assert!(spec.assemblies[0].solver_result.is_none());
        assert_eq!(
            spec.assemblies[0].solver_preview.status,
            "single_domain_preview"
        );
        assert_eq!(
            spec.assemblies[0].residual_graph.solver_plan,
            "metadata_only_no_numeric_solve"
        );
        assert!(json.contains("\"domain_summary\""));
        assert!(json.contains("\"component_summary\""));
        assert!(json.contains("\"local_expression_count\": 1"));
        assert!(json.contains("\"pressure_seed\""));
        assert!(json.contains("\"delay_call_runtime_buffer_pending_integration\""));
        assert!(json.contains("\"connection_summary\""));
        assert!(json.contains("\"assembly_summary\""));
        assert!(json.contains("\"solver_result\": null"));
        assert!(json.contains("\"component_graph\""));
        assert!(json.contains("\"behavior_nodes\""));
        assert!(json.contains("\"behavior_kind\": \"delay\""));
        assert!(json.contains("\"signal\": \"outlet.m_dot\""));
        assert!(json.contains("\"delay_s\": 5"));
        assert!(json.contains("\"relationship_status\": \"delay_relationship_metadata_only\""));
        assert!(json.contains("\"contract_inputs\""));
        assert!(json.contains("\"quantity_kind\": \"MassFlowRate\""));
        assert!(json.contains("\"diagnostic_channels\""));
        assert!(json.contains("\"delay_history_underflow_failure\""));
        assert!(json.contains("\"medium_label\": \"Water\""));
        assert!(json.contains("\"source_span\""));
        assert!(json.contains("\"assembly_count\": 1"));
        assert!(json.contains("\"through_conservation\""));
        assert!(
            json.contains("generated from through variable conservation within a connection set")
        );
        assert!(json.contains("\"jacobian_sparsity\""));
        assert!(json.contains("\"package\": \"eng.std.domains.fluid\""));
        assert!(json.contains("\"kind\": \"Medium\""));
        assert!(json.contains("\"display\": \"Medium M\""));
        assert!(json.contains("\"type_arguments\": [\"Water\"]"));
        assert!(json.contains("\"domain_count\": 1"));
        assert!(json.contains("\"single_domain_preview\""));
        assert!(json.contains("\"homogeneous_connection_constraints\""));
        assert!(html.contains("Domains"));
        assert!(html.contains("Fluid[Medium M]"));
        assert!(html.contains("eng.std.domains.fluid"));
        assert!(html.contains("Component Ports"));
        assert!(html.contains("pressure_seed"));
        assert!(html.contains("signal=outlet.m_dot"));
        assert!(html.contains("delay_s=5"));
        assert!(html.contains("relationship=delay relationship metadata"));
        assert!(html.contains("inputs=input:outlet.m_dot:MassFlowRate"));
        assert!(html.contains("outputs=output:pressure_seed:MassFlowRate"));
        assert!(html.contains("diagnostics=delay_history_underflow_failure"));
        assert!(!html.contains("delay_call_runtime_buffer_pending_integration"));
        assert!(html.contains("delay runtime buffer not connected to this language-level solve"));
        assert!(html.contains("Connections"));
        assert!(html.contains("Component Assembly"));
        assert!(html.contains("constraint check"));
        assert!(html.contains("domain plan"));
        assert!(html.contains("generated from through variable conservation"));
        assert!(html.contains("component_residual_graph"));
        assert!(html.contains("domain_compatible"));
    }

    #[test]
    fn report_behavior_nodes_resolve_prior_component_local_signals() {
        let report = check_source(
            "ok.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent Source {\n    port out: Thermal\n    temperature_signal = out.T\n    delayed_temperature = delay(temperature_signal, 5 s)\n    nested_delayed_temperature = delay(delay(out.T, 1 s), 5 s)\n    predicted_temperature = predictor(delay(out.T, 1 s))\n}\n",
            &CheckOptions::default(),
        );

        let spec = report_spec_from_report(&report, "plots/plot_manifest.json", "abc123");
        let json = report_spec_json(&spec);
        let html = render_html(&report, "plots/timeseries.svg");

        assert_eq!(
            spec.components[0].local_expressions[0].name,
            "temperature_signal"
        );
        assert_eq!(
            spec.components[0].local_expressions[0].type_status,
            "domain_signal_resolved"
        );
        assert_eq!(
            spec.components[0].local_expressions[1].type_status,
            "delay_output_matches_signal"
        );
        assert_eq!(
            spec.components[0].local_expressions[2].type_status,
            "delay_output_matches_signal"
        );
        let delay_node = spec
            .component_graph
            .behavior_nodes
            .iter()
            .find(|node| node.behavior_kind == "delay")
            .expect("delay behavior node");
        assert_eq!(delay_node.signal.as_deref(), Some("temperature_signal"));
        assert_eq!(
            delay_node.contract_inputs[0].status,
            "component_local_signal_resolved"
        );
        assert_eq!(
            delay_node.contract_inputs[0].quantity_kind,
            "AbsoluteTemperature"
        );
        assert_eq!(delay_node.contract_outputs[0].name, "delayed_temperature");
        assert_eq!(
            delay_node.contract_outputs[0].quantity_kind,
            "AbsoluteTemperature"
        );
        let nested_delay_node = spec
            .component_graph
            .behavior_nodes
            .iter()
            .find(|node| node.name == "nested_delayed_temperature")
            .expect("nested delay behavior node");
        assert_eq!(
            nested_delay_node.signal.as_deref(),
            Some("delay(out.T, 1 s)")
        );
        assert_eq!(
            nested_delay_node.contract_inputs[0].status,
            "delay_expression_signal_resolved"
        );
        assert_eq!(
            nested_delay_node.contract_inputs[0].quantity_kind,
            "AbsoluteTemperature"
        );
        let predictor_node = spec
            .component_graph
            .behavior_nodes
            .iter()
            .find(|node| node.behavior_kind == "predictor")
            .expect("predictor behavior node");
        assert_eq!(
            predictor_node.contract_inputs[0].status,
            "delay_expression_signal_resolved"
        );
        assert_eq!(
            predictor_node.contract_outputs[0].quantity_kind,
            "AbsoluteTemperature"
        );
        assert_eq!(
            predictor_node.contract_outputs[0].status,
            "predictor_output_typed_identity_contract"
        );
        assert!(json.contains("\"type_status\": \"delay_output_matches_signal\""));
        assert!(json.contains("\"component_local_signal_resolved\""));
        assert!(json.contains("\"delay_expression_signal_resolved\""));
        assert!(json.contains("\"predictor_output_typed_identity_contract\""));
        assert!(html.contains("signal=temperature_signal"));
        assert!(html.contains("signal=delay(out.T, 1 s)"));
        assert!(html.contains("inputs=input:temperature_signal:AbsoluteTemperature"));
        assert!(html.contains("inputs=input:delay(out.T, 1 s):AbsoluteTemperature"));
        assert!(html.contains("outputs=output:predicted_temperature:AbsoluteTemperature"));
    }

    #[test]
    fn report_html_uses_runtime_review_document_values_and_validations() {
        let report = check_source("ok.eng", "occupied = 90 min\n", &CheckOptions::default());
        let mut spec =
            report_spec_from_report(&report, "plots/plot_manifest.json", "plot-manifest-hash");
        spec.validations.push(ReportValidationResult {
            expression: "legacy-spec-validation".to_owned(),
            left: "legacy".to_owned(),
            operator: "==".to_owned(),
            right: "legacy".to_owned(),
            left_value: None,
            right_value: None,
            unit: String::new(),
            status: "legacy".to_owned(),
            line: 99,
        });
        let review = json!({
            "review_document": {
                "semantic_hash": "runtime-fingerprint",
                "semantic_hash_scope": "runtime_enriched",
                "section_hashes": {},
                "status": "runtime_ready",
                "runtime_evidence": {
                    "numeric_value_count": 1,
                    "table_count": 0,
                    "timeseries_count": 1,
                    "timeseries_coverage_count": 1,
                    "side_effect_result_count": 2,
                    "validation_count": 1
                },
                "inputs": [{
                    "kind": "arg",
                    "name": "input_path",
                    "line": 1,
                    "runtime_result": {
                        "provenance": "resolved_arg",
                        "value": "data/runtime.csv",
                        "source": "cli",
                        "status": "resolved"
                    }
                }],
                "symbols": [{
                    "kind": "binding",
                    "name": "occupied",
                    "line": 1,
                    "runtime_result": {
                        "provenance": "runtime_numeric_value",
                        "value": 90,
                        "unit": "min",
                        "representation": "native_scalar",
                        "materialization": "computed",
                        "status": "computed"
                    }
                }, {
                    "kind": "binding",
                    "name": "coverage",
                    "line": 3,
                    "runtime_result": {
                        "provenance": "runtime_timeseries_coverage",
                        "source_table": "sensor",
                        "source_column": "time",
                        "actual_count": 4,
                        "expected_count": 4,
                        "missing_count": 0,
                        "status": "complete"
                    }
                }],
                "time_axes": [{
                    "binding": "occupied",
                    "axis": "Time",
                    "line": 1,
                    "runtime_result": {
                        "provenance": "runtime_time_axis",
                        "source_table": "sensor",
                        "source_column": "time",
                        "axis": "Time",
                        "unit": "min",
                        "start": 0,
                        "end": 90,
                        "count": 2,
                        "status": "materialized"
                    }
                }],
                "side_effects": [{
                    "kind": "write_output",
                    "target": "outputs/summary.txt",
                    "line": 4,
                    "runtime_result": {
                        "provenance": "runtime_artifact_record",
                        "artifact_kind": "write_text",
                        "artifact_path": "outputs/summary.txt",
                        "hash": "summary-hash",
                        "status": "generated"
                    }
                }, {
                    "kind": "write_output",
                    "target": r#"db.table("predictions")"#,
                    "line": 5,
                    "runtime_result": {
                        "provenance": "runtime_db_write",
                        "artifact_kind": "db_write_manifest",
                        "binding": "predictions",
                        "database": "outputs/results.sqlite",
                        "manifest_path": "outputs/results.sqlite.db_write_manifest.json",
                        "manifest_hash": "db-manifest-hash",
                        "database_hash_after": "db-file-hash",
                        "transaction_status": "committed",
                        "schema_status": "ok",
                        "table_count": 1,
                        "row_count": 3,
                        "tables": [{
                            "name": "predictions",
                            "mode": "replace",
                            "key": [],
                            "schema": ["case_id", "prediction"],
                            "row_count": 3
                        }],
                        "diagnostic_count": 0,
                        "diagnostics": [],
                        "validation": {
                            "status": "passed",
                            "rule": "sqlite_write_manifest",
                            "message": "SQLite write committed"
                        },
                        "status": "committed"
                    }
                }],
                "validations": [{
                    "line": 2,
                    "expression": "occupied between 60 min and 120 min",
                    "runtime_result": {
                        "provenance": "runtime_validation",
                        "left": "occupied",
                        "operator": "between",
                        "right": "60 min and 120 min",
                        "left_value": 90,
                        "right_value": null,
                        "unit": "min",
                        "status": "passed"
                    }
                }]
            }
        });

        let html = render_html_with_spec_and_review_document(
            &report,
            "plots/timeseries.svg",
            &spec,
            &review,
        )
        .expect("valid runtime ReviewDocument");
        let bare_html = render_html_with_spec_and_review_document(
            &report,
            "plots/timeseries.svg",
            &spec,
            &review["review_document"],
        )
        .expect("valid bare runtime ReviewDocument");

        assert_eq!(bare_html, html);
        assert!(html.contains("<h2>Runtime Review</h2>"));
        assert!(html.contains("runtime_enriched"));
        assert!(html.contains("runtime-fingerprint"));
        assert!(html.contains("data/runtime.csv"));
        assert!(html.contains("source=cli"));
        assert!(html.contains("90 min"));
        assert!(html.contains("runtime_numeric_value"));
        assert!(html.contains("runtime_timeseries_coverage"));
        assert!(html.contains("4/4 samples; missing 0"));
        assert!(html.contains("table=sensor"));
        assert!(html.contains("column=time"));
        assert!(html.contains("axis=Time"));
        assert!(html.contains("<span>Time series</span><strong>1</strong>"));
        assert!(html.contains("<span>Coverage</span><strong>1</strong>"));
        assert!(html.contains("<span>Side effects</span><strong>2</strong>"));
        assert!(html.contains("write_text: outputs/summary.txt"));
        assert!(html.contains("path=outputs/summary.txt"));
        assert!(html.contains("hash=summary-hash"));
        assert!(html.contains("1 table, 3 rows"));
        assert!(html.contains("database=outputs/results.sqlite"));
        assert!(html.contains("manifest=outputs/results.sqlite.db_write_manifest.json"));
        assert!(html.contains("manifest_hash=db-manifest-hash"));
        assert!(html.contains("database_hash_after=db-file-hash"));
        assert!(html.contains("transaction=committed"));
        assert!(html.contains("schema_status=ok"));
        assert!(html.contains("occupied between 60 min and 120 min"));
        assert!(html.contains("90 min between 60 min and 120 min"));
        assert!(!html.contains("120 min min"));
        assert!(!html.contains("legacy-spec-validation"));
        assert_eq!(html.matches("<h2>Validations</h2>").count(), 1);
    }

    #[test]
    fn report_html_rejects_invalid_review_document() {
        let report = check_source("ok.eng", "occupied = 90 min\n", &CheckOptions::default());
        let spec =
            report_spec_from_report(&report, "plots/plot_manifest.json", "plot-manifest-hash");
        let error = render_html_with_spec_and_review_document(
            &report,
            "plots/timeseries.svg",
            &spec,
            &json!({ "semantic_hash": "runtime-fingerprint" }),
        )
        .expect_err("incomplete ReviewDocument must fail");

        assert_eq!(error, ReviewDocumentError::MissingSectionHashes);
    }

    fn sample_plot_spec(plot_type: &str) -> PlotSpec {
        PlotSpec {
            title: "Seed plot".to_owned(),
            plot_type: plot_type.to_owned(),
            x_axis: PlotAxis {
                name: "case".to_owned(),
                label: "Case".to_owned(),
                unit: String::new(),
            },
            y_axis: PlotAxis {
                name: "value".to_owned(),
                label: "Value".to_owned(),
                unit: "kW".to_owned(),
            },
            series: vec![PlotSeries {
                name: "value".to_owned(),
                quantity_kind: "HeatRate".to_owned(),
                display_unit: "kW".to_owned(),
                bins: Vec::new(),
                points: vec![
                    PlotPoint { x: 0.0, y: 1.0 },
                    PlotPoint { x: 1.0, y: 2.5 },
                    PlotPoint { x: 2.0, y: 1.5 },
                ],
                confidence_band: None,
            }],
        }
    }
}
