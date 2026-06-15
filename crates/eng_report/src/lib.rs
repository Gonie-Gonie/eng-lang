use eng_compiler::{CheckReport, DomainTypeParameterInfo, Severity};

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
    pub time_alignments: Vec<ReportTimeAlignment>,
    pub uncertainty: Vec<ReportUncertaintyInfo>,
    pub ml: Vec<ReportMlInfo>,
    pub policy_results: Vec<ReportPolicyResult>,
    pub domains: Vec<ReportDomainSummary>,
    pub components: Vec<ReportComponentSummary>,
    pub connections: Vec<ReportConnectionSummary>,
    pub assemblies: Vec<ReportAssemblySummary>,
    pub classes: Vec<ReportClassSummary>,
    pub class_objects: Vec<ReportClassObjectSummary>,
    pub systems: Vec<ReportSystemSummary>,
    pub system_ir: Vec<ReportSystemIr>,
    pub plot_manifest: ReportPlotManifest,
    pub warnings: Vec<ReportWarning>,
    pub provenance: ReportProvenance,
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
pub struct ReportTimeAlignment {
    pub left: String,
    pub right: String,
    pub axis: String,
    pub left_count: usize,
    pub right_count: usize,
    pub matched_count: usize,
    pub overlap_start: Option<f64>,
    pub overlap_end: Option<f64>,
    pub status: String,
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
    pub ports: Vec<ReportPort>,
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
pub struct ReportAssemblySummary {
    pub name: String,
    pub status: String,
    pub component_count: usize,
    pub port_count: usize,
    pub connection_count: usize,
    pub component_equation_count: usize,
    pub local_expression_count: usize,
    pub operator_call_count: usize,
    pub predictor_call_count: usize,
    pub connection_sets: Vec<ReportConnectionSet>,
    pub equations: Vec<ReportAssemblyEquation>,
    pub variables: Vec<ReportAssemblyVariable>,
    pub boundary: ReportAssemblyBoundary,
    pub residual_graph: ReportResidualGraph,
    pub line: usize,
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
    pub dependencies: Vec<ReportResidualDependency>,
    pub algebraic_loops: Vec<Vec<String>>,
    pub jacobian_sparsity: Vec<ReportAssemblyJacobianSeed>,
    pub solver_plan: String,
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
    pub jacobian_seed: Vec<ReportJacobianSeed>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportOdeRunner {
    pub status: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportJacobianSeed {
    pub residual: String,
    pub with_respect_to: Vec<String>,
    pub derivative_states: Vec<String>,
    pub status: String,
}

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
        .components
        .iter()
        .map(|component| ReportComponentSummary {
            name: component.name.clone(),
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
            component_count: assembly.component_count,
            port_count: assembly.port_count,
            connection_count: assembly.connection_count,
            component_equation_count: assembly.component_equation_count,
            local_expression_count: assembly.local_expression_count,
            operator_call_count: assembly.operator_call_count,
            predictor_call_count: assembly.predictor_call_count,
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
            line: assembly.line,
        })
        .collect::<Vec<_>>();
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
        time_alignments: Vec::new(),
        uncertainty,
        ml,
        policy_results: Vec::new(),
        domains,
        components,
        connections,
        assemblies,
        classes,
        class_objects,
        systems,
        system_ir,
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
            component_count: report.semantic_program.components.len(),
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

    json.push_str("  \"time_alignments\": [\n");
    for (index, alignment) in spec.time_alignments.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
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
        push_optional_json_f64(&mut json, "overlap_start", alignment.overlap_start, 6);
        push_optional_json_f64(&mut json, "overlap_end", alignment.overlap_end, 6);
        json.push_str(&format!(
            "      \"status\": \"{}\"\n",
            json_escape(&alignment.status)
        ));
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
        json.push_str(&format!("      \"line\": {},\n", component.line));
        json.push_str(&format!(
            "      \"port_count\": {},\n",
            component.ports.len()
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
        json.push_str("      }\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
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
            "preview".to_owned(),
            "Time".to_owned(),
            "Value".to_owned(),
            "unit".to_owned(),
        )
    });

    PlotSpec {
        title: if name == "preview" {
            "EngLang preview plot".to_owned()
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
            points: preview_points(),
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
            "      \"bins\": [{}]\n",
            plot_bins_json(&series.bins)
        ));
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

pub fn plot_manifest_json(
    spec: &PlotSpec,
    svg_relative_path: &str,
    plot_spec_hash: &str,
    svg_hash: &str,
) -> String {
    format!(
        "{{\n  \"format\": \"eng-plot-manifest-v1\",\n  \"plot_spec_version\": {PLOT_SPEC_VERSION},\n  \"plots\": [\n    {{\n      \"title\": \"{}\",\n      \"plot_type\": \"{}\",\n      \"plot_spec\": \"plot_spec.json\",\n      \"plot_spec_hash\": \"{}\",\n      \"svg\": \"{}\",\n      \"svg_hash\": \"{}\",\n      \"x_axis_label\": \"{}\",\n      \"y_axis_label\": \"{}\"\n    }}\n  ]\n}}\n",
        json_escape(&spec.title),
        json_escape(&spec.plot_type),
        json_escape(plot_spec_hash),
        json_escape(svg_relative_path),
        json_escape(svg_hash),
        json_escape(&axis_label(&spec.x_axis)),
        json_escape(&axis_label(&spec.y_axis))
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
    json.push_str(&format!("{indent}  \"jacobian_seed\": [\n"));
    for (seed_index, seed) in plan.jacobian_seed.iter().enumerate() {
        if seed_index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}    {{\n"));
        json.push_str(&format!(
            "{indent}      \"residual\": \"{}\",\n",
            json_escape(&seed.residual)
        ));
        json.push_str(&format!("{indent}      \"with_respect_to\": ["));
        for (variable_index, variable) in seed.with_respect_to.iter().enumerate() {
            if variable_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(variable)));
        }
        json.push_str("],\n");
        json.push_str(&format!("{indent}      \"derivative_states\": ["));
        for (state_index, state) in seed.derivative_states.iter().enumerate() {
            if state_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(state)));
        }
        json.push_str("],\n");
        json.push_str(&format!(
            "{indent}      \"status\": \"{}\"\n",
            json_escape(&seed.status)
        ));
        json.push_str(&format!("{indent}    }}"));
    }
    json.push_str(&format!("\n{indent}  ]\n"));
    json.push_str(&format!("{indent}}}"));
}

pub fn render_html(report: &CheckReport, plot_relative_path: &str) -> String {
    render_html_inner(report, plot_relative_path, None)
}

pub fn render_html_with_spec(
    report: &CheckReport,
    plot_relative_path: &str,
    spec: &ReportSpec,
) -> String {
    render_html_inner(report, plot_relative_path, Some(spec))
}

fn render_html_inner(
    report: &CheckReport,
    plot_relative_path: &str,
    spec: Option<&ReportSpec>,
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
        let transform = uncertainty_transform_label(info.scale.as_deref(), info.offset.as_deref());
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
    for component in &report.semantic_program.components {
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
    }
    if component_summary.is_empty() {
        component_summary.push_str("<tr><td colspan=\"5\">No component ports.</td></tr>");
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
        assembly_summary.push_str("<tr>");
        assembly_summary.push_str(&format!(
            "<td>{}</td><td>graph</td><td>{}</td><td>components={}</td><td>ports={}, connections={}, component equations={}, local expressions={}, operators={}, predictors={}</td><td>{}</td>",
            assembly.line,
            html_escape(&assembly.name),
            assembly.component_count,
            assembly.port_count,
            assembly.connection_count,
            assembly.component_equation_count,
            assembly.local_expression_count,
            assembly.operator_call_count,
            assembly.predictor_call_count,
            html_escape(&assembly.status)
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
                "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code><br><small>residual: {}</small></td><td>{}</td>",
                equation.line,
                html_escape(&equation.kind),
                html_escape(&equation.name),
                html_escape(&equation.domain),
                html_escape(&equation.expression),
                html_escape(&equation.residual),
                html_escape(&equation.status)
            ));
            assembly_summary.push_str("</tr>");
        }
        assembly_summary.push_str("<tr>");
        assembly_summary.push_str(&format!(
            "<td>{}</td><td>residual graph</td><td>{}</td><td>residuals={}</td><td>dependencies={}, algebraic loop seeds={}, jacobian seeds={}, solver plan={}</td><td>{}</td>",
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
    let component_count = report.semantic_program.components.len();
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
    let validations_section = spec.map(render_validations_section).unwrap_or_default();
    let time_alignments_section = spec.map(render_time_alignments_section).unwrap_or_default();
    let component_solver_section = spec
        .map(render_component_solver_section)
        .unwrap_or_default();

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
    }}
    th {{
      background: #eef2f7;
      font-weight: 600;
    }}
    code {{
      font-family: Consolas, "SFMono-Regular", monospace;
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
    <p>Reviewable EngLang preview artifact with source hash <code>{source_hash}</code>.</p>
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

fn render_computed_metrics_section(spec: &ReportSpec) -> String {
    if spec.computed_metrics.is_empty() {
        return String::new();
    }
    let rows = spec
        .computed_metrics
        .iter()
        .map(|metric| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.6}</td><td>{}</td><td>{}</td></tr>",
                metric.line,
                html_escape(&metric.binding),
                html_escape(&metric.kind),
                html_escape(&format!("{} vs {}", metric.left, metric.right)),
                metric.value,
                html_escape(&metric.unit),
                html_escape(&metric.status)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<h2>Computed Metrics</h2>
    <table>
      <thead><tr><th>Line</th><th>Binding</th><th>Kind</th><th>Comparison</th><th>Value</th><th>Unit</th><th>Status</th></tr></thead>
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

fn render_time_alignments_section(spec: &ReportSpec) -> String {
    if spec.time_alignments.is_empty() {
        return String::new();
    }
    let rows = spec
        .time_alignments
        .iter()
        .map(|alignment| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}/{}</td><td>{}</td></tr>",
                html_escape(&alignment.left),
                html_escape(&alignment.right),
                html_escape(&alignment.axis),
                alignment.matched_count,
                alignment.left_count.min(alignment.right_count),
                html_escape(&alignment.status)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<h2>Time Alignments</h2>
    <table>
      <thead><tr><th>Left</th><th>Right</th><th>Axis</th><th>Matched</th><th>Status</th></tr></thead>
      <tbody>{rows}</tbody>
    </table>"#
    )
}

fn render_component_solver_section(spec: &ReportSpec) -> String {
    if spec.assemblies.is_empty() {
        return String::new();
    }
    let rows = spec
        .assemblies
        .iter()
        .map(|assembly| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}/{}</td><td>{}</td><td>{}</td></tr>",
                assembly.line,
                html_escape(&assembly.name),
                html_escape(&assembly.status),
                assembly.boundary.equation_count,
                assembly.boundary.unknown_count,
                html_escape(&assembly.residual_graph.status),
                html_escape(&assembly.residual_graph.solver_plan)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<h2>Component Solver Preview</h2>
    <table>
      <thead><tr><th>Line</th><th>Assembly</th><th>Status</th><th>Eq/Unknowns</th><th>Convergence</th><th>Method</th></tr></thead>
      <tbody>{rows}</tbody>
    </table>"#
    )
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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

fn uncertainty_transform_label(scale: Option<&str>, offset: Option<&str>) -> String {
    match (scale, offset) {
        (Some(scale), Some(offset)) => format!("scale={scale}, offset={offset}"),
        (Some(scale), None) => format!("scale={scale}"),
        (None, Some(offset)) => format!("offset={offset}"),
        (None, None) => String::new(),
    }
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
            unit: "preview".to_owned(),
        },
        series: vec![PlotSeries {
            name: "preview".to_owned(),
            quantity_kind: "Value".to_owned(),
            display_unit: "preview".to_owned(),
            bins: Vec::new(),
            points: preview_points(),
        }],
    }
}

fn preview_points() -> Vec<PlotPoint> {
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
        .flat_map(|series| series.points.iter())
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
            spec.system_ir[0].solver_plan.jacobian_seed[0].with_respect_to,
            vec!["T".to_owned()]
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
        assert!(json.contains("\"jacobian_seed\""));
        assert!(json.contains("\"RoomThermal.residual_1\""));
        assert!(html.contains("System Equations"));
        assert!(html.contains("unit_consistent"));
    }

    #[test]
    fn report_spec_and_html_include_domain_component_sections() {
        let report = check_source(
            "ok.eng",
            "domain Fluid[Medium M] package \"eng.std.domains.fluid\" version \"0.1.0\" {\n    across height: Length [m]\n    through m_dot: MassFlowRate [kg/s]\n    conservation sum(m_dot) = 0\n}\n\ncomponent Supply {\n    port outlet: Fluid[Water]\n}\n\ncomponent Return {\n    port inlet: Fluid[Water]\n}\n\nconnect Supply.outlet -> Return.inlet\n",
            &CheckOptions::default(),
        );

        let spec = report_spec_from_report(&report, "plots/plot_manifest.json", "abc123");
        let json = report_spec_json(&spec);
        let html = render_html(&report, "plots/timeseries.svg");

        assert_eq!(spec.provenance.domain_count, 1);
        assert_eq!(spec.provenance.component_count, 2);
        assert_eq!(spec.provenance.connection_count, 1);
        assert_eq!(spec.provenance.assembly_count, 1);
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
        assert_eq!(spec.connections[0].status, "domain_compatible");
        assert_eq!(spec.assemblies[0].connection_sets.len(), 1);
        assert_eq!(spec.assemblies[0].equations.len(), 2);
        assert_eq!(spec.assemblies[0].boundary.unknown_count, 4);
        assert_eq!(
            spec.assemblies[0].residual_graph.solver_plan,
            "metadata_only_no_numeric_solve"
        );
        assert!(json.contains("\"domain_summary\""));
        assert!(json.contains("\"component_summary\""));
        assert!(json.contains("\"connection_summary\""));
        assert!(json.contains("\"assembly_summary\""));
        assert!(json.contains("\"assembly_count\": 1"));
        assert!(json.contains("\"through_conservation\""));
        assert!(json.contains("\"jacobian_sparsity\""));
        assert!(json.contains("\"package\": \"eng.std.domains.fluid\""));
        assert!(json.contains("\"kind\": \"Medium\""));
        assert!(json.contains("\"display\": \"Medium M\""));
        assert!(json.contains("\"type_arguments\": [\"Water\"]"));
        assert!(json.contains("\"domain_count\": 1"));
        assert!(html.contains("Domains"));
        assert!(html.contains("Fluid[Medium M]"));
        assert!(html.contains("eng.std.domains.fluid"));
        assert!(html.contains("Component Ports"));
        assert!(html.contains("Connections"));
        assert!(html.contains("Component Assembly"));
        assert!(html.contains("component_residual_graph"));
        assert!(html.contains("domain_compatible"));
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
            }],
        }
    }
}
