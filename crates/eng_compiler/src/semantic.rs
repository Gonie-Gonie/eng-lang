use crate::ast::{
    ArgsFieldDecl, AssertDecl, AstItem, ClassFieldDecl, ClassMethodDecl, ClassObjectCopyDecl,
    ClassObjectDecl, ClassObjectFieldDecl, ClassValidationDecl, CommandStyleDecl, ConnectDecl,
    ConstDecl, CsvExportDecl, CsvExportFieldDecl, DomainTypeParameterDecl, DomainVariableDecl,
    ExpectationDecl, ExplicitDecl, FastBinding, FileOperationDecl, FunctionDecl, FunctionParamDecl,
    GoldenDecl, ImportDecl, PortDecl, PrintDecl, ProcessRunDecl, ReturnDecl,
    StateSpaceTypeBlockDecl, StateSpaceTypeMemberDecl, StateSpaceVectorDecl, SystemVariableDecl,
    TestDecl, WhereBindingDecl, WithOptionDecl, WriteDecl,
};
use crate::cache::CacheRecordInfo;
use crate::expected::{expected_type_from_explicit_decl, ExpectedType, ExpectedTypeSource};
use crate::hover::HoverHint;
use crate::ml::MlInfo;
use crate::net::{NetDownloadInfo, NetRequestInfo};
use crate::parser::{ParseContext, ParsedProgram};
use crate::quantities::{
    candidates_for_unit, completion_labels, first_unit_in_expression,
    infer_quantity_from_name_and_unit, is_number_literal, normalize_unit, QuantityCompletion,
};
use crate::schema::{ConfigPromotion, CsvPromotion, SchemaInfo};
use crate::stats::{AxisInfo, IntegrationInfo, StatsInfo};
use crate::table::TableTransformInfo;
use crate::type_info::{TypeInfo, TypeInfoSource};
use crate::uncertainty::UncertaintyInfo;
use crate::units::{unit_derivation, unit_info_for_symbol, UnitDerivation};
use crate::workflow::Workflow;
use crate::{Diagnostic, InferredDeclaration};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticType {
    pub quantity_kind: String,
    pub display_unit: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedBinding {
    pub name: String,
    pub semantic_type: SemanticType,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportInfo {
    pub target: String,
    pub kind: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionParamInfo {
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub dimension: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionLocalInfo {
    pub name: String,
    pub expression: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionInfo {
    pub name: String,
    pub parameters: Vec<FunctionParamInfo>,
    pub locals: Vec<FunctionLocalInfo>,
    pub return_quantity_kind: String,
    pub return_display_unit: String,
    pub return_canonical_unit: String,
    pub return_dimension: String,
    pub return_expression: Option<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstInfo {
    pub name: String,
    pub type_name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub dimension: String,
    pub expression: String,
    pub importable: bool,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemVariableInfo {
    pub role: String,
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub dimension: String,
    pub initial_value: Option<String>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EquationInfo {
    pub system: String,
    pub left: String,
    pub right: String,
    pub relation: String,
    pub left_dimension: String,
    pub right_dimension: String,
    pub residual: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidualInfo {
    pub system: String,
    pub name: String,
    pub expression: String,
    pub dimension: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EquationDependencyInfo {
    pub name: String,
    pub role: String,
    pub quantity_kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EquationIrInfo {
    pub system: String,
    pub residual: String,
    pub relation: String,
    pub normalized_residual: String,
    pub dependencies: Vec<EquationDependencyInfo>,
    pub derivative_states: Vec<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SolverPlanInfo {
    pub status: String,
    pub method: String,
    pub solve_order: Vec<String>,
    pub ode_runner: OdeRunnerInfo,
    pub jacobian_seed: Vec<JacobianSeedInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OdeRunnerInfo {
    pub status: String,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JacobianSeedInfo {
    pub residual: String,
    pub with_respect_to: Vec<String>,
    pub derivative_states: Vec<String>,
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemInfo {
    pub name: String,
    pub variables: Vec<SystemVariableInfo>,
    pub equations: Vec<EquationInfo>,
    pub residuals: Vec<ResidualInfo>,
    pub equation_ir: Vec<EquationIrInfo>,
    pub solver_plan: SolverPlanInfo,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomainVariableInfo {
    pub role: String,
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub dimension: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConservationInfo {
    pub domain: String,
    pub text: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateSpaceVectorInfo {
    pub system: String,
    pub role: String,
    pub name: String,
    pub vector_type: String,
    pub members: Vec<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StateSpaceTypeBlockInfo {
    role: String,
    name: String,
    members: Vec<StateSpaceTypeMemberInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StateSpaceTypeMemberInfo {
    name: String,
    type_name: String,
    unit: Option<String>,
    line: usize,
    span: crate::source::SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinearOperatorEntryInfo {
    pub row_index: usize,
    pub column_index: usize,
    pub row_member: String,
    pub column_member: String,
    pub coefficient: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinearOperatorInfo {
    pub system: String,
    pub name: String,
    pub from: String,
    pub to: String,
    pub expression: Option<String>,
    pub canonical_matrix: Option<Vec<Vec<f64>>>,
    pub canonical_entries: Vec<LinearOperatorEntryInfo>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomainTypeParameterInfo {
    pub kind: String,
    pub name: String,
    pub display: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomainInfo {
    pub name: String,
    pub type_parameters: Vec<DomainTypeParameterInfo>,
    pub package: Option<String>,
    pub version: Option<String>,
    pub variables: Vec<DomainVariableInfo>,
    pub conservations: Vec<ConservationInfo>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortInfo {
    pub name: String,
    pub domain: String,
    pub domain_name: String,
    pub type_arguments: Vec<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentConstructorArgumentInfo {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentParameterInfo {
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub dimension: String,
    pub default_value: Option<String>,
    pub value: Option<String>,
    pub source: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentInfo {
    pub name: String,
    pub template_name: Option<String>,
    pub constructor_arguments: Vec<ComponentConstructorArgumentInfo>,
    pub parameters: Vec<ComponentParameterInfo>,
    pub inputs: Vec<ComponentParameterInfo>,
    pub ports: Vec<PortInfo>,
    pub local_expressions: Vec<ComponentLocalExpressionInfo>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentLocalExpressionInfo {
    pub name: String,
    pub expression: String,
    pub status: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub type_status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectionInfo {
    pub left: String,
    pub right: String,
    pub left_component: String,
    pub left_port: String,
    pub right_component: String,
    pub right_port: String,
    pub domain: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentAssemblyInfo {
    pub name: String,
    pub status: String,
    pub component_count: usize,
    pub port_count: usize,
    pub connection_count: usize,
    pub component_equation_count: usize,
    pub local_expression_count: usize,
    pub operator_call_count: usize,
    pub predictor_call_count: usize,
    pub domain_count: usize,
    pub domain_plans: Vec<ComponentDomainPlanInfo>,
    pub solver_preview: ComponentSolverPreviewInfo,
    pub connection_sets: Vec<ComponentConnectionSetInfo>,
    pub equations: Vec<ComponentAssemblyEquationInfo>,
    pub variables: Vec<ComponentAssemblyVariableInfo>,
    pub boundary: ComponentAssemblyBoundaryInfo,
    pub residual_graph: ComponentResidualGraphInfo,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentConnectionSetInfo {
    pub name: String,
    pub domain: String,
    pub ports: Vec<String>,
    pub connection_count: usize,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentDomainPlanInfo {
    pub domain: String,
    pub connection_set_count: usize,
    pub equation_count: usize,
    pub variable_count: usize,
    pub conservation_status: String,
    pub solver_role: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentSolverPreviewInfo {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentAssemblyEquationInfo {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentAssemblyVariableInfo {
    pub name: String,
    pub role: String,
    pub domain: String,
    pub source: String,
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentAssemblyBoundaryInfo {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentResidualGraphInfo {
    pub name: String,
    pub status: String,
    pub residuals: Vec<String>,
    pub residual_metadata: Vec<ComponentResidualGraphResidualInfo>,
    pub dependencies: Vec<ComponentResidualDependencyInfo>,
    pub algebraic_loops: Vec<Vec<String>>,
    pub jacobian_sparsity: Vec<ComponentJacobianSparsityInfo>,
    pub solver_plan: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentResidualGraphResidualInfo {
    pub name: String,
    pub kind: String,
    pub domain: String,
    pub source_expression: String,
    pub residual_expression: String,
    pub rhs: Option<String>,
    pub dependencies: Vec<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentResidualDependencyInfo {
    pub residual: String,
    pub variable: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentJacobianSparsityInfo {
    pub residual: String,
    pub with_respect_to: Vec<String>,
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassFieldInfo {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassValidationInfo {
    pub expression: String,
    pub left: String,
    pub operator: String,
    pub right: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassMethodInfo {
    pub name: String,
    pub return_type: String,
    pub return_quantity_kind: String,
    pub return_display_unit: String,
    pub return_canonical_unit: String,
    pub expression: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassInfo {
    pub name: String,
    pub fields: Vec<ClassFieldInfo>,
    pub validations: Vec<ClassValidationInfo>,
    pub methods: Vec<ClassMethodInfo>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassObjectFieldInfo {
    pub name: String,
    pub expression: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassObjectValidationInfo {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassObjectInfo {
    pub name: String,
    pub class_name: String,
    pub source_object: Option<String>,
    pub construction: String,
    pub fields: Vec<ClassObjectFieldInfo>,
    pub validations: Vec<ClassObjectValidationInfo>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArgsFieldInfo {
    pub name: String,
    pub type_name: String,
    pub default_value: Option<String>,
    pub redacted: bool,
    pub required: bool,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArgValueInfo {
    pub name: String,
    pub type_name: String,
    pub value: String,
    pub redacted: bool,
    pub source: String,
    pub required: bool,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvironmentDependencyInfo {
    pub name: String,
    pub kind: String,
    pub expression: String,
    pub resolved_value: String,
    pub source_hash: Option<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArgsBlockInfo {
    pub name: String,
    pub fields: Vec<ArgsFieldInfo>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormatExpressionInfo {
    pub expression: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub requested_unit: Option<String>,
    pub precision: Option<usize>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrintInfo {
    pub level: String,
    pub template: String,
    pub fields: Vec<FormatExpressionInfo>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CsvExportFieldInfo {
    pub name: String,
    pub expression: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub requested_unit: Option<String>,
    pub precision: Option<usize>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CsvExportInfo {
    pub source: String,
    pub format: String,
    pub path: String,
    pub fields: Vec<CsvExportFieldInfo>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteInfo {
    pub format: String,
    pub path: String,
    pub expression: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileOperationInfo {
    pub operation: String,
    pub source: String,
    pub destination: Option<String>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessRunInfo {
    pub binding: String,
    pub command: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssertInfo {
    pub left: String,
    pub operator: String,
    pub right: String,
    pub tolerance: Option<String>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoldenInfo {
    pub artifact: String,
    pub expected: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestInfo {
    pub name: String,
    pub assertions: Vec<AssertInfo>,
    pub goldens: Vec<GoldenInfo>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandClauseInfo {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandStyleInfo {
    pub verb: String,
    pub target: String,
    pub clauses: Vec<CommandClauseInfo>,
    pub canonical: String,
    pub status: String,
    pub owner: Option<String>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectationInfo {
    pub target: String,
    pub text: String,
    pub kind: String,
    pub subject: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectationSuiteInfo {
    pub binding: String,
    pub target: String,
    pub expectations: Vec<ExpectationInfo>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhereBindingInfo {
    pub name: String,
    pub expression: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhereBlockInfo {
    pub owner_line: Option<usize>,
    pub bindings: Vec<WhereBindingInfo>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithOptionInfo {
    pub key: String,
    pub value: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithBlockInfo {
    pub owner_line: Option<usize>,
    pub options: Vec<WithOptionInfo>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeSeriesKernelInfo {
    pub binding: String,
    pub kind: String,
    pub source_table: Option<String>,
    pub axis: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub expression: String,
    pub operations: Vec<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SampleGenerationInfo {
    pub binding: String,
    pub method: String,
    pub count: usize,
    pub seed: Option<u64>,
    pub distributions: Vec<SampleDistributionInfo>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SampleDistributionInfo {
    pub name: String,
    pub kind: String,
    pub lower: f64,
    pub upper: f64,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticProgram {
    pub imports: Vec<ImportInfo>,
    pub consts: Vec<ConstInfo>,
    pub functions: Vec<FunctionInfo>,
    pub typed_bindings: Vec<TypedBinding>,
    pub expected_types: Vec<ExpectedType>,
    pub hover_hints: Vec<HoverHint>,
    pub type_infos: Vec<TypeInfo>,
    pub unit_derivations: Vec<UnitDerivation>,
    pub schemas: Vec<SchemaInfo>,
    pub csv_promotions: Vec<CsvPromotion>,
    pub config_promotions: Vec<ConfigPromotion>,
    pub sample_generations: Vec<SampleGenerationInfo>,
    pub table_transforms: Vec<TableTransformInfo>,
    pub net_requests: Vec<NetRequestInfo>,
    pub net_downloads: Vec<NetDownloadInfo>,
    pub cache_records: Vec<CacheRecordInfo>,
    pub workflow: Workflow,
    pub axis_infos: Vec<AxisInfo>,
    pub stats_infos: Vec<StatsInfo>,
    pub integrations: Vec<IntegrationInfo>,
    pub uncertainty_infos: Vec<UncertaintyInfo>,
    pub ml_infos: Vec<MlInfo>,
    pub systems: Vec<SystemInfo>,
    pub state_space_vectors: Vec<StateSpaceVectorInfo>,
    pub linear_operators: Vec<LinearOperatorInfo>,
    pub domains: Vec<DomainInfo>,
    pub components: Vec<ComponentInfo>,
    pub connections: Vec<ConnectionInfo>,
    pub component_assemblies: Vec<ComponentAssemblyInfo>,
    pub classes: Vec<ClassInfo>,
    pub class_objects: Vec<ClassObjectInfo>,
    pub args_blocks: Vec<ArgsBlockInfo>,
    pub arg_values: Vec<ArgValueInfo>,
    pub environment_dependencies: Vec<EnvironmentDependencyInfo>,
    pub prints: Vec<PrintInfo>,
    pub csv_exports: Vec<CsvExportInfo>,
    pub writes: Vec<WriteInfo>,
    pub file_operations: Vec<FileOperationInfo>,
    pub process_runs: Vec<ProcessRunInfo>,
    pub tests: Vec<TestInfo>,
    pub command_styles: Vec<CommandStyleInfo>,
    pub expectation_suites: Vec<ExpectationSuiteInfo>,
    pub where_blocks: Vec<WhereBlockInfo>,
    pub with_blocks: Vec<WithBlockInfo>,
    pub timeseries_kernels: Vec<TimeSeriesKernelInfo>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticOutput {
    pub diagnostics: Vec<Diagnostic>,
    pub inferred_declarations: Vec<InferredDeclaration>,
    pub semantic_program: SemanticProgram,
}

pub fn analyze(program: &ParsedProgram) -> SemanticOutput {
    let mut diagnostics = Vec::new();
    let mut inferred_declarations = Vec::new();
    let state_space_type_blocks = collect_state_space_type_blocks(program);
    let mut imports = Vec::new();
    let mut consts = Vec::new();
    let mut functions = Vec::new();
    let mut current_function_index = None;
    let mut typed_bindings = Vec::new();
    let mut expected_types = Vec::new();
    let mut hover_hints = Vec::new();
    let mut type_infos = Vec::new();
    let mut unit_derivations = Vec::new();
    let mut stats_infos = Vec::new();
    let mut integrations = Vec::new();
    let mut uncertainty_infos = Vec::new();
    let mut ml_infos = Vec::new();
    let mut systems = Vec::new();
    let mut state_space_vectors = Vec::new();
    let mut state_space_type_aliases: HashMap<(String, String), String> = HashMap::new();
    let mut linear_operators = Vec::new();
    let mut current_system_index = None;
    let mut domains = Vec::new();
    let mut current_domain_index = None;
    let mut components = Vec::new();
    let mut component_instances = Vec::new();
    let mut current_component_index = None;
    let mut raw_connections = Vec::new();
    let mut classes = Vec::new();
    let mut current_class_index = None;
    let mut class_objects = Vec::new();
    let mut args_blocks = Vec::new();
    let mut current_args_block_index = None;
    let mut prints = Vec::new();
    let mut csv_exports = Vec::new();
    let mut current_csv_export_index = None;
    let mut writes = Vec::new();
    let mut file_operations = Vec::new();
    let mut process_runs = Vec::new();
    let mut tests = Vec::new();
    let mut current_test_index = None;
    let mut command_styles = Vec::new();
    let mut expectation_suites = Vec::new();
    let mut current_expectation_suite_index = None;
    let mut timeseries_kernels = Vec::new();

    for line in &program.lines {
        if line.tokens.iter().any(|token| {
            matches!(
                token.kind,
                crate::lexer::TokenKind::Symbol(crate::lexer::Symbol::ColonEqual)
            )
        }) {
            diagnostics.push(Diagnostic::error(
                "E-SYNTAX-DECL-001",
                line.line,
                "`:=` is not part of EngLang syntax.",
                Some("Use `name = ...` for local declaration or assignment."),
            ));
        }
        if line.context == ParseContext::Equation
            && line.tokens.iter().any(|token| {
                matches!(
                    token.kind,
                    crate::lexer::TokenKind::Symbol(crate::lexer::Symbol::EqualEqual)
                )
            })
        {
            diagnostics.push(Diagnostic::error(
                "E-EQ-BOOL-001",
                line.line,
                "Use `eq` for physical equations. `==` returns Bool.",
                Some("Replace `==` with `eq` inside equation blocks."),
            ));
        }
    }

    for item in &program.items {
        match item {
            AstItem::Import(import) => imports.push(analyze_import_decl(import)),
            AstItem::Function(function) => {
                functions.push(analyze_function_decl(function, &mut diagnostics));
                current_function_index = Some(functions.len() - 1);
            }
            AstItem::Return(return_decl) => {
                if let Some(function_index) = current_function_index {
                    analyze_function_return(
                        return_decl,
                        &mut functions[function_index],
                        &mut diagnostics,
                    );
                }
            }
            AstItem::System(system) => {
                systems.push(SystemInfo {
                    name: system.name.clone(),
                    variables: Vec::new(),
                    equations: Vec::new(),
                    residuals: Vec::new(),
                    equation_ir: Vec::new(),
                    solver_plan: SolverPlanInfo {
                        status: "metadata_only".to_owned(),
                        method: "source_order_symbolic_seed".to_owned(),
                        solve_order: Vec::new(),
                        ode_runner: OdeRunnerInfo {
                            status: "deferred".to_owned(),
                            reason: "numeric ODE runner deferred until the solver milestone"
                                .to_owned(),
                        },
                        jacobian_seed: Vec::new(),
                    },
                    line: system.span.line,
                });
                current_system_index = Some(systems.len() - 1);
            }
            AstItem::StateSpaceTypeBlock(_) | AstItem::StateSpaceTypeMember(_) => {}
            AstItem::Domain(domain) => {
                domains.push(DomainInfo {
                    name: domain.name.clone(),
                    type_parameters: domain
                        .type_parameters
                        .iter()
                        .map(domain_type_parameter_info)
                        .collect(),
                    package: domain.package.clone(),
                    version: domain.version.clone(),
                    variables: Vec::new(),
                    conservations: Vec::new(),
                    line: domain.span.line,
                });
                current_domain_index = Some(domains.len() - 1);
            }
            AstItem::DomainVariable(variable) => {
                if let Some(domain_index) = current_domain_index {
                    analyze_domain_variable(variable, &mut domains[domain_index]);
                }
            }
            AstItem::Conservation(conservation) => {
                if let Some(domain_index) = current_domain_index {
                    let domain = &mut domains[domain_index];
                    domain.conservations.push(ConservationInfo {
                        domain: domain.name.clone(),
                        text: conservation.text.clone(),
                        status: "recorded".to_owned(),
                        line: conservation.line,
                    });
                }
            }
            AstItem::Component(component) => {
                components.push(ComponentInfo {
                    name: component.name.clone(),
                    template_name: None,
                    constructor_arguments: Vec::new(),
                    parameters: Vec::new(),
                    inputs: Vec::new(),
                    ports: Vec::new(),
                    local_expressions: Vec::new(),
                    line: component.span.line,
                });
                current_component_index = Some(components.len() - 1);
            }
            AstItem::Port(port) => {
                if let Some(component_index) = current_component_index {
                    analyze_port(port, &mut components[component_index]);
                }
            }
            AstItem::Connect(connect) => raw_connections.push(connect.clone()),
            AstItem::Class(class_decl) => {
                classes.push(ClassInfo {
                    name: class_decl.name.clone(),
                    fields: Vec::new(),
                    validations: Vec::new(),
                    methods: Vec::new(),
                    status: "metadata_only".to_owned(),
                    line: class_decl.span.line,
                });
                current_class_index = Some(classes.len() - 1);
            }
            AstItem::ClassField(field) => {
                if let Some(class_index) = current_class_index {
                    analyze_class_field(field, &mut classes[class_index], &mut diagnostics);
                }
            }
            AstItem::ClassValidation(validation) => {
                if let Some(class_index) = current_class_index {
                    analyze_class_validation(
                        validation,
                        &mut classes[class_index],
                        &mut diagnostics,
                    );
                }
            }
            AstItem::ClassMethod(method) => {
                if let Some(class_index) = current_class_index {
                    analyze_class_method(method, &mut classes[class_index], &mut diagnostics);
                }
            }
            AstItem::ClassObject(object) => {
                analyze_class_object_decl(
                    object,
                    &classes,
                    &mut class_objects,
                    &mut diagnostics,
                    &mut typed_bindings,
                    &mut hover_hints,
                    &mut type_infos,
                );
            }
            AstItem::ClassObjectCopy(object) => {
                analyze_class_object_copy_decl(
                    object,
                    &classes,
                    &mut class_objects,
                    &mut diagnostics,
                    &mut typed_bindings,
                    &mut hover_hints,
                    &mut type_infos,
                );
            }
            AstItem::ClassObjectField(field) => {
                analyze_class_object_field(
                    field,
                    &classes,
                    &mut class_objects,
                    &typed_bindings,
                    &functions,
                    &mut diagnostics,
                );
            }
            AstItem::Struct(struct_decl) => {
                current_args_block_index = None;
                diagnostics.push(Diagnostic::error(
                    "E-STRUCT-ARGS-001",
                    struct_decl.span.line,
                    "`struct Args` is no longer supported for execution arguments.",
                    Some("Use `args { ... }` as the only root argument declaration syntax."),
                ));
            }
            AstItem::Args(args_decl) => {
                args_blocks.push(ArgsBlockInfo {
                    name: args_decl.name.clone(),
                    fields: Vec::new(),
                    line: args_decl.span.line,
                });
                current_args_block_index = Some(args_blocks.len() - 1);
            }
            AstItem::ArgsField(field) => {
                if let Some(args_block_index) = current_args_block_index {
                    analyze_args_field(field, &mut args_blocks[args_block_index]);
                }
            }
            AstItem::SystemVariable(variable) => match variable.context {
                ParseContext::System => {
                    if let Some(system_index) = current_system_index {
                        if let Some((vector_role, type_name)) =
                            state_space_vector_type_parameter(&variable.type_name)
                        {
                            analyze_typed_state_space_vector_variable(
                                variable,
                                vector_role,
                                type_name,
                                &state_space_type_blocks,
                                &mut systems[system_index],
                                &mut state_space_vectors,
                                &mut state_space_type_aliases,
                                &mut diagnostics,
                                &mut expected_types,
                                &mut hover_hints,
                                &mut typed_bindings,
                                &mut type_infos,
                                &mut unit_derivations,
                            );
                        } else {
                            analyze_system_variable(
                                variable,
                                &mut systems[system_index],
                                &mut expected_types,
                                &mut hover_hints,
                                &mut typed_bindings,
                                &mut type_infos,
                                &mut unit_derivations,
                            );
                        }
                    }
                }
                ParseContext::Component => {
                    if let Some(component_index) = current_component_index {
                        if variable.role == "input" {
                            analyze_component_input(
                                variable,
                                &mut components[component_index],
                                &consts,
                                &mut diagnostics,
                            );
                        } else {
                            analyze_component_parameter(
                                variable,
                                &mut components[component_index],
                                &consts,
                                &mut diagnostics,
                            );
                        }
                    }
                }
                _ => {}
            },
            AstItem::StateSpaceVector(vector) => {
                if let Some(system_index) = current_system_index {
                    analyze_state_space_vector_decl(
                        vector,
                        &systems[system_index].name,
                        &mut state_space_vectors,
                        &mut typed_bindings,
                        &mut hover_hints,
                        &mut type_infos,
                    );
                }
            }
            AstItem::Equation(equation) => {
                if equation.context == ParseContext::Component {
                    if let Some(component_index) = current_component_index {
                        analyze_component_equation(equation, &mut components[component_index]);
                    }
                    continue;
                }
                if let Some(system_index) = current_system_index {
                    analyze_equation(equation, &mut systems[system_index], &mut diagnostics);
                }
            }
            AstItem::Script(script) => {
                diagnostics.push(Diagnostic::error(
                    "E-SCRIPT-001",
                    script.span.line,
                    "`script` blocks are no longer supported as execution roots.",
                    Some("Move the body to top-level statements and use `args { ... }` for CLI arguments."),
                ));
            }
            AstItem::ExplicitDecl(declaration) => {
                if declaration.context != ParseContext::Function {
                    if let Some(system_index) = current_system_index {
                        if let Some(operator) = analyze_linear_operator_decl(
                            declaration,
                            &systems[system_index].name,
                            &state_space_type_aliases,
                        ) {
                            linear_operators.push(operator);
                        }
                    }
                    analyze_explicit_decl(
                        declaration,
                        &mut diagnostics,
                        &mut expected_types,
                        &mut hover_hints,
                        &mut typed_bindings,
                        &mut type_infos,
                        &mut unit_derivations,
                        &mut inferred_declarations,
                    );
                }
            }
            AstItem::Const(const_decl) => {
                analyze_const_decl(
                    const_decl,
                    &mut consts,
                    &mut diagnostics,
                    &mut expected_types,
                    &mut hover_hints,
                    &mut typed_bindings,
                    &mut type_infos,
                    &mut unit_derivations,
                );
            }
            AstItem::FastBinding(binding) => {
                if binding.context == ParseContext::Function {
                    if let Some(function_index) = current_function_index {
                        functions[function_index].locals.push(FunctionLocalInfo {
                            name: binding.name.clone(),
                            expression: binding.expression.clone(),
                            line: binding.line,
                        });
                    }
                    continue;
                }
                if binding.context == ParseContext::System {
                    match analyze_component_instance_binding(
                        binding,
                        &components,
                        &component_instances,
                        &consts,
                        &mut diagnostics,
                    ) {
                        ComponentInstanceBindingAnalysis::Instance(instance) => {
                            component_instances.push(instance);
                            continue;
                        }
                        ComponentInstanceBindingAnalysis::HandledInvalid => continue,
                        ComponentInstanceBindingAnalysis::NotComponentConstructor => {}
                    }
                }
                if binding.context == ParseContext::Component {
                    if let Some(component_index) = current_component_index {
                        analyze_component_local_expression(
                            binding,
                            &mut components[component_index],
                            &domains,
                        );
                    }
                    continue;
                }
                let scoped_bindings = scoped_where_bindings_for_owner(
                    binding.line,
                    program,
                    &typed_bindings,
                    &functions,
                    &mut diagnostics,
                );
                let mut accum = SemanticAccum {
                    diagnostics: &mut diagnostics,
                    inferred_declarations: &mut inferred_declarations,
                    typed_bindings: &mut typed_bindings,
                    scoped_bindings,
                    hover_hints: &mut hover_hints,
                    type_infos: &mut type_infos,
                    unit_derivations: &mut unit_derivations,
                    integrations: &mut integrations,
                    uncertainty_infos: &mut uncertainty_infos,
                    ml_infos: &mut ml_infos,
                    functions: &functions,
                    classes: &classes,
                    class_objects: &class_objects,
                    timeseries_kernels: &mut timeseries_kernels,
                };
                analyze_fast_binding(binding, &mut accum);
            }
            AstItem::Summary(summary) => {
                if let Some(info) = crate::stats::stats_info(summary, &typed_bindings) {
                    stats_infos.push(info);
                }
            }
            AstItem::Print(print) => {
                if reject_function_side_effect(
                    print.context,
                    "print/log statement",
                    print.line,
                    current_function_index,
                    &functions,
                    &mut diagnostics,
                ) {
                    continue;
                }
                analyze_print_decl(
                    print,
                    &typed_bindings,
                    &functions,
                    &mut prints,
                    &mut diagnostics,
                );
            }
            AstItem::CsvExport(export) => {
                if reject_function_side_effect(
                    export.context,
                    "CSV export",
                    export.line,
                    current_function_index,
                    &functions,
                    &mut diagnostics,
                ) {
                    current_csv_export_index = None;
                    continue;
                }
                csv_exports.push(analyze_csv_export_decl(export, &mut diagnostics));
                current_csv_export_index = Some(csv_exports.len() - 1);
            }
            AstItem::CsvExportField(field) => {
                if let Some(export_index) = current_csv_export_index {
                    if let Some(field_info) = analyze_csv_export_field_decl(
                        field,
                        &typed_bindings,
                        &functions,
                        &mut diagnostics,
                    ) {
                        csv_exports[export_index].fields.push(field_info);
                    }
                } else {
                    diagnostics.push(Diagnostic::error(
                        "E-EXPORT-CSV-001",
                        field.line,
                        "CSV export field is not inside an export block.",
                        Some("Write fields inside `export summary to csv \"path.csv\" { ... }`."),
                    ));
                }
            }
            AstItem::Write(write) => {
                if reject_function_side_effect(
                    write.context,
                    "write statement",
                    write.line,
                    current_function_index,
                    &functions,
                    &mut diagnostics,
                ) {
                    continue;
                }
                if let Some(write_info) =
                    analyze_write_decl(write, &typed_bindings, &functions, &mut diagnostics)
                {
                    writes.push(write_info);
                }
            }
            AstItem::FileOperation(operation) => {
                if reject_function_side_effect(
                    operation.context,
                    "file operation",
                    operation.line,
                    current_function_index,
                    &functions,
                    &mut diagnostics,
                ) {
                    continue;
                }
                if let Some(operation_info) =
                    analyze_file_operation_decl(operation, &mut diagnostics)
                {
                    file_operations.push(operation_info);
                }
            }
            AstItem::ProcessRun(process) => {
                if reject_function_side_effect(
                    process.context,
                    "process execution",
                    process.line,
                    current_function_index,
                    &functions,
                    &mut diagnostics,
                ) {
                    continue;
                }
                if let Some(process_info) = analyze_process_run_decl(
                    process,
                    &mut diagnostics,
                    &mut typed_bindings,
                    &mut hover_hints,
                    &mut type_infos,
                ) {
                    process_runs.push(process_info);
                }
            }
            AstItem::Test(test) => {
                if let Some(test_info) = analyze_test_decl(test, &mut diagnostics) {
                    tests.push(test_info);
                    current_test_index = Some(tests.len() - 1);
                }
            }
            AstItem::Assert(assertion) => {
                analyze_assert_decl(
                    assertion,
                    current_test_index,
                    &typed_bindings,
                    &functions,
                    &mut tests,
                    &mut diagnostics,
                );
            }
            AstItem::Golden(golden) => {
                analyze_golden_decl(golden, current_test_index, &mut tests, &mut diagnostics);
            }
            AstItem::CommandStyle(command) => {
                analyze_command_style_decl(
                    command,
                    &typed_bindings,
                    &functions,
                    &mut command_styles,
                    &mut diagnostics,
                );
            }
            AstItem::ExpectationSuite(suite) => {
                expectation_suites.push(ExpectationSuiteInfo {
                    binding: expectation_suite_binding(&suite.target, expectation_suites.len()),
                    target: suite.target.clone(),
                    expectations: Vec::new(),
                    status: "recorded".to_owned(),
                    line: suite.line,
                });
                current_expectation_suite_index = Some(expectation_suites.len() - 1);
            }
            AstItem::Expectation(expectation) => {
                if let Some(suite_index) = expectation
                    .suite_line
                    .and_then(|line| {
                        expectation_suites
                            .iter()
                            .position(|suite| suite.line == line)
                    })
                    .or(current_expectation_suite_index)
                {
                    let target = expectation_suites[suite_index].target.clone();
                    expectation_suites[suite_index]
                        .expectations
                        .push(expectation_info(expectation, &target));
                }
            }
            AstItem::ReservedKeywordUse { keyword, span } => diagnostics.push(Diagnostic::error(
                "E-RESERVED-KEYWORD-001",
                span.line,
                &format!("`{keyword}` is reserved for EngLang syntax."),
                Some(
                    "Use another identifier. The `eq` keyword is reserved for physical equations.",
                ),
            )),
            _ => {}
        }
    }

    let where_blocks = analyze_where_blocks(program, &typed_bindings, &functions, &mut diagnostics);
    let with_blocks = analyze_with_blocks(
        program,
        &typed_bindings,
        &command_styles,
        &systems,
        &mut diagnostics,
    );
    validate_file_operation_options(&file_operations, &with_blocks, &mut diagnostics);
    validate_process_options(&process_runs, &with_blocks, &mut diagnostics);
    let sample_generations = analyze_sample_generations(program, &with_blocks, &mut diagnostics);
    validate_write_options(&writes, &with_blocks, &mut diagnostics);
    validate_where_local_uses(program, &where_blocks, &mut diagnostics);
    validate_domain_contracts(&domains, &mut diagnostics);
    validate_component_behavior_calls(&domains, &components, &mut diagnostics);
    validate_class_contracts(&classes, &mut class_objects, &mut diagnostics);
    validate_function_returns(&mut functions, &consts, &mut diagnostics);
    validate_state_space_vector_members(&systems, &mut state_space_vectors, &mut diagnostics);
    validate_system_state_declarations(&systems, &mut diagnostics);
    validate_system_derivative_equations(&systems, &mut diagnostics);
    validate_linear_operator_shapes(
        &systems,
        &state_space_vectors,
        &mut linear_operators,
        &mut diagnostics,
    );
    validate_json_read_field_access_policy(program, &mut diagnostics);

    let mut assembly_components = if component_instances.is_empty() {
        components.clone()
    } else {
        component_instances
    };
    let connections = analyze_connections(
        &domains,
        &mut assembly_components,
        &raw_connections,
        &mut diagnostics,
    );
    let component_assemblies = build_component_assembly_graphs(
        &domains,
        &assembly_components,
        &connections,
        &mut diagnostics,
    );
    emit_component_assembly_boundary_warnings(&component_assemblies, &mut diagnostics);
    validate_algebraic_solve_contracts(
        &component_assemblies,
        &systems,
        &inferred_declarations,
        &with_blocks,
        &mut diagnostics,
    );
    SemanticOutput {
        diagnostics,
        inferred_declarations,
        semantic_program: SemanticProgram {
            imports,
            consts,
            functions,
            axis_infos: crate::stats::axis_infos(&typed_bindings),
            typed_bindings,
            expected_types,
            hover_hints,
            type_infos,
            unit_derivations,
            schemas: Vec::new(),
            csv_promotions: Vec::new(),
            config_promotions: Vec::new(),
            sample_generations,
            table_transforms: Vec::new(),
            net_requests: Vec::new(),
            net_downloads: Vec::new(),
            cache_records: Vec::new(),
            workflow: Workflow::top_level(top_level_workflow_line(program)),
            stats_infos,
            integrations,
            uncertainty_infos,
            ml_infos,
            systems,
            state_space_vectors,
            linear_operators,
            domains,
            components: assembly_components,
            connections,
            component_assemblies,
            classes,
            class_objects,
            args_blocks,
            arg_values: Vec::new(),
            environment_dependencies: Vec::new(),
            prints,
            csv_exports,
            writes,
            file_operations,
            process_runs,
            tests,
            command_styles,
            expectation_suites,
            where_blocks,
            with_blocks,
            timeseries_kernels,
        },
    }
}

fn analyze_import_decl(import: &ImportDecl) -> ImportInfo {
    ImportInfo {
        target: import.target.clone(),
        kind: import.kind.clone(),
        status: if import.kind == "file" {
            "resolved_by_compiler".to_owned()
        } else {
            "declared".to_owned()
        },
        line: import.line,
    }
}

fn expectation_suite_binding(target: &str, index: usize) -> String {
    let slug = target
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_owned();
    let slug = if slug.is_empty() {
        format!("suite_{}", index + 1)
    } else {
        slug
    };
    format!("{slug}.expectations")
}

fn expectation_info(expectation: &ExpectationDecl, target: &str) -> ExpectationInfo {
    let text = expectation.text.trim().to_owned();
    let kind = expectation_kind(&text).to_owned();
    let subject = expectation_subject(&text).unwrap_or_else(|| target.to_owned());
    ExpectationInfo {
        target: target.to_owned(),
        text,
        kind,
        subject,
        status: "recorded".to_owned(),
        line: expectation.line,
    }
}

fn expectation_kind(text: &str) -> &'static str {
    if text.contains(" is continuous") {
        "continuous"
    } else if text.contains(" between ") {
        "between"
    } else if text.contains(" is monotonic") {
        "monotonic"
    } else {
        "constraint"
    }
}

fn expectation_subject(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if let Some((left, _)) = trimmed.split_once(" is ") {
        return Some(left.trim().to_owned());
    }
    if let Some((left, _)) = trimmed.split_once(" between ") {
        return Some(left.trim().to_owned());
    }
    for operator in ["<=", ">=", "==", "!=", "<", ">"] {
        if let Some((left, _)) = trimmed.split_once(operator) {
            return Some(left.trim().to_owned());
        }
    }
    trimmed
        .split_whitespace()
        .next()
        .filter(|subject| !subject.is_empty())
        .map(str::to_owned)
}

fn analyze_command_style_decl(
    command: &CommandStyleDecl,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
    command_styles: &mut Vec<CommandStyleInfo>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if command.status == "unknown_verb" {
        diagnostics.push(Diagnostic::error(
            "E-CMD-UNKNOWN-VERB",
            command.line,
            &format!("`{}` is not a supported command-style verb.", command.verb),
            Some("Use a supported built-in command verb or call a function with parentheses."),
        ));
    } else if command.status == "ambiguous_target" {
        diagnostics.push(Diagnostic::error(
            "E-CMD-AMBIG-001",
            command.line,
            &format!(
                "Command target `{}` is ambiguous without parentheses.",
                command.target
            ),
            Some("Wrap complex command targets, for example `integrate (Q1 + Q2) over Time`."),
        ));
    } else if command.status == "missing_target" {
        diagnostics.push(Diagnostic::error(
            "E-CMD-AMBIG-001",
            command.line,
            &format!("Command `{}` needs a target expression.", command.verb),
            Some("Write a binding, table, series, or parenthesized expression after the command verb."),
        ));
    } else if command.verb == "validate" {
        validate_command_expression(command, typed_bindings, functions, diagnostics);
    }
    command_styles.push(CommandStyleInfo {
        verb: command.verb.clone(),
        target: command.target.clone(),
        clauses: command
            .clauses
            .iter()
            .map(|clause| CommandClauseInfo {
                name: clause.name.clone(),
                value: clause.value.clone(),
            })
            .collect(),
        canonical: command.canonical.clone(),
        status: command.status.clone(),
        owner: command.owner.clone(),
        line: command.line,
    });
}

fn validate_command_expression(
    command: &CommandStyleDecl,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(between) = command
        .clauses
        .iter()
        .find(|clause| clause.name == "between")
    {
        validate_between_command_expression(
            command,
            &between.value,
            typed_bindings,
            functions,
            diagnostics,
        );
        return;
    }

    if is_coverage_complete_validation_target(&command.target, typed_bindings) {
        return;
    }

    let Some((left, _operator, right)) = split_validation_expression(&command.target) else {
        diagnostics.push(Diagnostic::error(
            "E-VALIDATE-BOOL-001",
            command.line,
            &format!(
                "`validate {}` must be a comparison expression.",
                command.target
            ),
            Some(
                "Write forms such as `validate rmse_T < 5 K` so the expression evaluates to Bool.",
            ),
        ));
        return;
    };

    validate_probability_expression(&left, command.line, typed_bindings, functions, diagnostics);
    validate_probability_expression(&right, command.line, typed_bindings, functions, diagnostics);
    let left_type = assert_expression_semantic_type(&left, typed_bindings, functions);
    let right_type = assert_expression_semantic_type(&right, typed_bindings, functions);
    if left_type.is_none() {
        diagnostics.push(Diagnostic::error(
            "E-VALIDATE-EXPR-001",
            command.line,
            &format!("Cannot resolve validation expression `{left}`."),
            Some("Validate a typed metric, integration result, function call, or literal."),
        ));
    }
    if right_type.is_none() {
        diagnostics.push(Diagnostic::error(
            "E-VALIDATE-EXPR-001",
            command.line,
            &format!("Cannot resolve validation expression `{right}`."),
            Some("Use a typed threshold such as `5 K` or a compatible binding."),
        ));
    }
    if let (Some(left_type), Some(right_type)) = (&left_type, &right_type) {
        if push_direct_uncertainty_comparison_diagnostic(
            "Validation",
            &left,
            &right,
            left_type,
            right_type,
            command.line,
            diagnostics,
        ) {
            return;
        }
        validate_comparison_dimensions(
            "Validation",
            &left,
            &right,
            left_type,
            right_type,
            command.line,
            typed_bindings,
            diagnostics,
        );
    }
}

fn is_coverage_complete_validation_target(
    expression: &str,
    typed_bindings: &[TypedBinding],
) -> bool {
    let Some((binding, field)) = expression.trim().split_once('.') else {
        return false;
    };
    field.trim() == "complete"
        && typed_bindings.iter().any(|typed_binding| {
            typed_binding.name == binding.trim()
                && typed_binding.semantic_type.quantity_kind == "CoverageResult"
        })
}

fn validate_between_command_expression(
    command: &CommandStyleDecl,
    between: &str,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some((lower, upper)) = split_between_bounds(between) else {
        diagnostics.push(Diagnostic::error(
            "E-VALIDATE-BOOL-001",
            command.line,
            &format!(
                "`validate {}` has an invalid `between` clause.",
                command.target
            ),
            Some("Write forms such as `validate mean(Q) between 4 kW and 6 kW`."),
        ));
        return;
    };

    let value = command.target.trim().to_owned();
    validate_probability_expression(&value, command.line, typed_bindings, functions, diagnostics);
    let value_type = assert_expression_semantic_type(&value, typed_bindings, functions);
    let lower_type = assert_expression_semantic_type(&lower, typed_bindings, functions);
    let upper_type = assert_expression_semantic_type(&upper, typed_bindings, functions);
    if value_type.is_none() {
        diagnostics.push(Diagnostic::error(
            "E-VALIDATE-EXPR-001",
            command.line,
            &format!("Cannot resolve validation expression `{value}`."),
            Some("Validate a typed metric, integration result, function call, or literal."),
        ));
    }
    if lower_type.is_none() {
        diagnostics.push(Diagnostic::error(
            "E-VALIDATE-EXPR-001",
            command.line,
            &format!("Cannot resolve validation expression `{lower}`."),
            Some("Use a typed lower bound such as `4 kW` or a compatible binding."),
        ));
    }
    if upper_type.is_none() {
        diagnostics.push(Diagnostic::error(
            "E-VALIDATE-EXPR-001",
            command.line,
            &format!("Cannot resolve validation expression `{upper}`."),
            Some("Use a typed upper bound such as `6 kW` or a compatible binding."),
        ));
    }
    if let (Some(value_type), Some(lower_type), Some(upper_type)) =
        (&value_type, &lower_type, &upper_type)
    {
        let lower_direct = push_direct_uncertainty_comparison_diagnostic(
            "Validation",
            &value,
            &lower,
            value_type,
            lower_type,
            command.line,
            diagnostics,
        );
        let upper_direct = push_direct_uncertainty_comparison_diagnostic(
            "Validation",
            &value,
            &upper,
            value_type,
            upper_type,
            command.line,
            diagnostics,
        );
        if lower_direct || upper_direct {
            return;
        }
        validate_comparison_dimensions(
            "Validation",
            &value,
            &lower,
            value_type,
            lower_type,
            command.line,
            typed_bindings,
            diagnostics,
        );
        validate_comparison_dimensions(
            "Validation",
            &value,
            &upper,
            value_type,
            upper_type,
            command.line,
            typed_bindings,
            diagnostics,
        );
    }
}

fn split_between_bounds(value: &str) -> Option<(String, String)> {
    let (lower, upper) = value.split_once(" and ")?;
    let lower = lower.trim();
    let upper = upper.trim();
    if lower.is_empty() || upper.is_empty() {
        return None;
    }
    Some((lower.to_owned(), upper.to_owned()))
}

fn split_validation_expression(expression: &str) -> Option<(String, String, String)> {
    let (index, operator) = top_level_comparison_operator(expression)?;
    let left = expression[..index].trim();
    let right = expression[index + operator.len()..].trim();
    if left.is_empty() || right.is_empty() {
        return None;
    }
    Some((left.to_owned(), operator.to_owned(), right.to_owned()))
}

fn top_level_comparison_operator(expression: &str) -> Option<(usize, &'static str)> {
    let mut parens = 0i32;
    let mut brackets = 0i32;
    let mut in_string = false;
    let mut previous = '\0';
    for (index, character) in expression.char_indices() {
        if !in_string && parens == 0 && brackets == 0 {
            for operator in ["<=", ">=", "==", "!=", "<", ">"] {
                if expression[index..].starts_with(operator) {
                    return Some((index, operator));
                }
            }
        }
        if character == '"' && previous != '\\' {
            in_string = !in_string;
        } else if !in_string {
            match character {
                '(' => parens += 1,
                ')' => parens -= 1,
                '[' => brackets += 1,
                ']' => brackets -= 1,
                _ => {}
            }
        }
        previous = character;
    }
    None
}

fn validate_probability_expression(
    expression: &str,
    line: usize,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(call) = parse_function_call(expression) else {
        return;
    };
    if call.name != "probability" {
        return;
    }
    if call.args.len() != 1 {
        diagnostics.push(Diagnostic::error(
            "E-UNC-PROBABILITY-EXPR-INVALID",
            line,
            "`probability(...)` requires one comparison expression.",
            Some("Write `probability(Q < 10 kW)` with an uncertain value and a compatible threshold."),
        ));
        return;
    }
    let Some((left, _operator, right)) = split_validation_expression(&call.args[0]) else {
        diagnostics.push(Diagnostic::error(
            "E-UNC-PROBABILITY-EXPR-INVALID",
            line,
            &format!(
                "`probability({})` must contain a comparison expression.",
                call.args[0]
            ),
            Some("Write `probability(Q < 10 kW)` with `<`, `<=`, `>`, or `>=`."),
        ));
        return;
    };
    let left_type = assert_expression_semantic_type(&left, typed_bindings, functions);
    let right_type = assert_expression_semantic_type(&right, typed_bindings, functions);
    let Some(left_type) = left_type else {
        diagnostics.push(Diagnostic::error(
            "E-UNC-PROBABILITY-EXPR-INVALID",
            line,
            &format!("Cannot resolve probability expression side `{left}`."),
            Some("Use a prior uncertainty binding and a typed threshold."),
        ));
        return;
    };
    let Some(right_type) = right_type else {
        diagnostics.push(Diagnostic::error(
            "E-UNC-PROBABILITY-EXPR-INVALID",
            line,
            &format!("Cannot resolve probability expression side `{right}`."),
            Some("Use a prior uncertainty binding and a typed threshold."),
        ));
        return;
    };
    let left_uncertain = uncertainty_inner_semantic_type(&left_type);
    let right_uncertain = uncertainty_inner_semantic_type(&right_type);
    let probability_contract = match (left_uncertain, right_uncertain) {
        (Some(inner), None) => Some((inner, right_type)),
        (None, Some(inner)) => Some((inner, left_type)),
        _ => None,
    };
    let Some((uncertain_inner, threshold_type)) = probability_contract else {
        diagnostics.push(Diagnostic::error(
            "E-UNC-PROBABILITY-EXPR-INVALID",
            line,
            &format!("`probability({})` must compare exactly one uncertain value with a threshold.", call.args[0]),
            Some("Compare forms such as `probability(Q < 10 kW)` are supported for the current uncertainty track."),
        ));
        return;
    };
    let uncertain_dimension = dimension_for_quantity(&uncertain_inner.quantity_kind);
    let threshold_dimension = dimension_for_quantity(&threshold_type.quantity_kind);
    if !dimensions_compatible(&uncertain_dimension, &threshold_dimension) {
        diagnostics.push(Diagnostic::error(
            "E-UNC-PROBABILITY-EXPR-INVALID",
            line,
            &format!(
                "`probability({})` compares {uncertain_dimension} uncertainty with {threshold_dimension} threshold.",
                call.args[0]
            ),
            Some("Use a probability threshold with the same physical dimension as the uncertain value."),
        ));
    }
}

fn push_direct_uncertainty_comparison_diagnostic(
    context: &str,
    left: &str,
    right: &str,
    left_type: &SemanticType,
    right_type: &SemanticType,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let left_uncertain = uncertainty_inner_semantic_type(left_type).is_some();
    let right_uncertain = uncertainty_inner_semantic_type(right_type).is_some();
    if !left_uncertain && !right_uncertain {
        return false;
    }
    let (uncertain_expression, compared_expression) = if right_uncertain && !left_uncertain {
        (right, left)
    } else {
        (left, right)
    };
    diagnostics.push(Diagnostic::error(
        "E-UNC-DIRECT-COMPARE",
        line,
        &format!(
            "{context} compares uncertain value directly: `{uncertain_expression}` vs `{compared_expression}`."
        ),
        Some("Use an explicit uncertainty statistic such as `mean(Q)`, `p95(Q)`, or `probability(Q < threshold)`."),
    ));
    true
}

fn validate_comparison_dimensions(
    context: &str,
    left: &str,
    right: &str,
    left_type: &SemanticType,
    right_type: &SemanticType,
    line: usize,
    typed_bindings: &[TypedBinding],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let left_dimension = dimension_for_quantity(&left_type.quantity_kind);
    let right_dimension = dimension_for_quantity(&right_type.quantity_kind);
    if dimensions_compatible(&left_dimension, &right_dimension) {
        return;
    }
    let percentile_mismatch = uncertainty_percentile_expression(left, typed_bindings)
        || uncertainty_percentile_expression(right, typed_bindings);
    let code = if percentile_mismatch {
        "E-UNC-PERCENTILE-UNIT-MISMATCH"
    } else if context == "Assert" {
        "E-ASSERT-UNIT-001"
    } else {
        "E-VALIDATE-UNIT-001"
    };
    let help = if percentile_mismatch {
        "Use a threshold with the same physical dimension as the uncertainty percentile."
    } else if context == "Assert" {
        "Compare values with compatible dimensions or convert units first."
    } else {
        "Use a threshold with the same physical dimension as the validated value."
    };
    diagnostics.push(Diagnostic::error(
        code,
        line,
        &format!(
            "{context} compares `{left}` ({left_dimension}) with `{right}` ({right_dimension})."
        ),
        Some(help),
    ));
}

fn uncertainty_inner_semantic_type(value_type: &SemanticType) -> Option<SemanticType> {
    let (_kind, inner) = crate::uncertainty::uncertainty_inner_quantity(&value_type.quantity_kind)?;
    semantic_type(&inner, &value_type.display_unit)
}

fn scoped_where_bindings_for_owner(
    owner_line: usize,
    program: &ParsedProgram,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
    _diagnostics: &mut Vec<Diagnostic>,
) -> Vec<TypedBinding> {
    let bindings = where_bindings_for_owner(program, Some(owner_line));
    if bindings.is_empty() {
        return Vec::new();
    }
    let mut ignored_diagnostics = Vec::new();
    analyze_where_binding_scope(
        &bindings,
        typed_bindings,
        functions,
        &mut ignored_diagnostics,
    )
    .into_iter()
    .map(|(binding, _info)| binding)
    .collect()
}

fn analyze_where_blocks(
    program: &ParsedProgram,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<WhereBlockInfo> {
    program
        .items
        .iter()
        .filter_map(|item| match item {
            AstItem::WhereBlock(block) => Some(block),
            _ => None,
        })
        .map(|block| {
            let bindings = where_bindings_for_owner(program, block.owner_line);
            let infos =
                analyze_where_binding_scope(&bindings, typed_bindings, functions, diagnostics)
                    .into_iter()
                    .map(|(_binding, info)| info)
                    .collect::<Vec<_>>();
            WhereBlockInfo {
                owner_line: block.owner_line,
                bindings: infos,
                line: block.line,
            }
        })
        .collect()
}

fn analyze_where_binding_scope(
    bindings: &[WhereBindingDecl],
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<(TypedBinding, WhereBindingInfo)> {
    let all_local_names = bindings
        .iter()
        .map(|binding| binding.name.clone())
        .collect::<HashSet<_>>();
    let mut defined_local_names = HashSet::new();
    let mut available_bindings = typed_bindings.to_vec();
    let mut resolved = Vec::new();

    for binding in bindings {
        for identifier in expression_identifiers(&binding.expression) {
            if all_local_names.contains(&identifier) && !defined_local_names.contains(&identifier) {
                diagnostics.push(Diagnostic::error(
                    "E-WHERE-FWD-001",
                    binding.line,
                    &format!("Where-local `{identifier}` is used before it is defined."),
                    Some("Define where locals before dependent locals in the same block."),
                ));
            }
        }

        let temporary = FastBinding {
            name: binding.name.clone(),
            expression: binding.expression.clone(),
            line: binding.line,
            span: binding.span,
            context: ParseContext::Where,
        };
        check_ambiguous_quantity(&temporary, diagnostics);

        let semantic_type = infer_scoped_binding_semantic_type(
            &binding.name,
            &binding.expression,
            &available_bindings,
            functions,
        );
        let info = if let Some(semantic_type) = semantic_type {
            let typed = TypedBinding {
                name: binding.name.clone(),
                semantic_type: semantic_type.clone(),
                line: binding.line,
            };
            available_bindings.push(typed.clone());
            defined_local_names.insert(binding.name.clone());
            WhereBindingInfo {
                name: binding.name.clone(),
                expression: binding.expression.clone(),
                quantity_kind: semantic_type.quantity_kind.clone(),
                display_unit: semantic_type.display_unit.clone(),
                status: "typed".to_owned(),
                line: binding.line,
            }
        } else {
            defined_local_names.insert(binding.name.clone());
            WhereBindingInfo {
                name: binding.name.clone(),
                expression: binding.expression.clone(),
                quantity_kind: "unknown".to_owned(),
                display_unit: "unknown".to_owned(),
                status: "unresolved".to_owned(),
                line: binding.line,
            }
        };
        if let Some(typed) = available_bindings
            .iter()
            .rev()
            .find(|candidate| candidate.name == binding.name && candidate.line == binding.line)
            .cloned()
        {
            resolved.push((typed, info));
        } else {
            resolved.push((
                TypedBinding {
                    name: binding.name.clone(),
                    semantic_type: SemanticType {
                        quantity_kind: "unknown".to_owned(),
                        display_unit: "unknown".to_owned(),
                    },
                    line: binding.line,
                },
                info,
            ));
        }
    }
    resolved
}

fn infer_scoped_binding_semantic_type(
    name: &str,
    expression: &str,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
) -> Option<SemanticType> {
    path_helper_semantic_type(expression)
        .or_else(|| statistic_expression_semantic_type(expression, typed_bindings))
        .or_else(|| function_call_semantic_type(expression, typed_bindings, functions))
        .or_else(|| binding_alias_semantic_type(expression, typed_bindings))
        .or_else(|| infer_quantity(name, expression))
}

fn where_bindings_for_owner(
    program: &ParsedProgram,
    owner_line: Option<usize>,
) -> Vec<WhereBindingDecl> {
    program
        .items
        .iter()
        .filter_map(|item| match item {
            AstItem::WhereBinding(binding) if binding.owner_line == owner_line => {
                Some(binding.clone())
            }
            _ => None,
        })
        .collect()
}

fn analyze_with_blocks(
    program: &ParsedProgram,
    typed_bindings: &[TypedBinding],
    command_styles: &[CommandStyleInfo],
    systems: &[SystemInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<WithBlockInfo> {
    program
        .items
        .iter()
        .filter_map(|item| match item {
            AstItem::WithBlock(block) => Some(block),
            _ => None,
        })
        .map(|block| {
            let owner_type =
                with_owner_semantic_type(block.owner_line, typed_bindings, command_styles);
            let mut extra_known_options =
                with_owner_simulation_variable_options(program, block.owner_line, systems);
            extra_known_options.extend(with_owner_coverage_options(
                command_styles,
                block.owner_line,
            ));
            extra_known_options.extend(with_owner_timeseries_fill_options(
                command_styles,
                block.owner_line,
            ));
            extra_known_options.extend(with_owner_timeseries_alignment_options(
                command_styles,
                block.owner_line,
            ));
            extra_known_options.extend(with_owner_net_options(program, block.owner_line));
            extra_known_options.extend(with_owner_template_options(
                command_styles,
                block.owner_line,
            ));
            extra_known_options.extend(with_owner_apply_options(command_styles, block.owner_line));
            extra_known_options.extend(with_owner_process_options(program, block.owner_line));
            extra_known_options.extend(with_owner_sample_options(program, block.owner_line));
            extra_known_options.extend(with_owner_db_write_options(program, block.owner_line));
            let mut options = with_options_for_owner(program, block.owner_line)
                .into_iter()
                .map(|option| {
                    analyze_with_option(
                        &option,
                        owner_type.as_ref(),
                        &extra_known_options,
                        diagnostics,
                    )
                })
                .collect::<Vec<_>>();
            validate_uncertainty_policy_options(&mut options, diagnostics);
            WithBlockInfo {
                owner_line: block.owner_line,
                options,
                line: block.line,
            }
        })
        .collect()
}

fn with_owner_simulation_variable_options(
    program: &ParsedProgram,
    owner_line: Option<usize>,
    systems: &[SystemInfo],
) -> HashSet<String> {
    let Some(owner_line) = owner_line else {
        return HashSet::new();
    };
    let Some(system_name) = program.items.iter().find_map(|item| match item {
        AstItem::FastBinding(binding) if binding.line == owner_line => binding
            .expression
            .trim()
            .strip_prefix("simulate ")
            .map(str::trim)
            .map(str::to_owned),
        _ => None,
    }) else {
        return HashSet::new();
    };
    systems
        .iter()
        .find(|system| system.name == system_name)
        .map(|system| {
            system
                .variables
                .iter()
                .filter(|variable| matches!(variable.role.as_str(), "input" | "parameter"))
                .map(|variable| variable.name.clone())
                .collect()
        })
        .unwrap_or_default()
}
fn with_owner_coverage_options(
    command_styles: &[CommandStyleInfo],
    owner_line: Option<usize>,
) -> HashSet<String> {
    let Some(owner_line) = owner_line else {
        return HashSet::new();
    };
    let Some(command) = command_styles
        .iter()
        .find(|command| command.line == owner_line)
    else {
        return HashSet::new();
    };
    if command.verb != "check" || !command.target.trim().starts_with("coverage ") {
        return HashSet::new();
    }
    ["expected_step", "step", "year"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn with_owner_timeseries_fill_options(
    command_styles: &[CommandStyleInfo],
    owner_line: Option<usize>,
) -> HashSet<String> {
    let Some(owner_line) = owner_line else {
        return HashSet::new();
    };
    let Some(command) = command_styles
        .iter()
        .find(|command| command.line == owner_line)
    else {
        return HashSet::new();
    };
    if command.verb != "fill" || !command.target.trim().starts_with("missing ") {
        return HashSet::new();
    }
    ["method", "max_gap", "expected_step", "step"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn with_owner_timeseries_alignment_options(
    command_styles: &[CommandStyleInfo],
    owner_line: Option<usize>,
) -> HashSet<String> {
    let Some(owner_line) = owner_line else {
        return HashSet::new();
    };
    let Some(command) = command_styles
        .iter()
        .find(|command| command.line == owner_line)
    else {
        return HashSet::new();
    };
    if !matches!(command.verb.as_str(), "align" | "resample") {
        return HashSet::new();
    }
    ["method", "step", "target_step", "tolerance"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn with_owner_template_options(
    command_styles: &[CommandStyleInfo],
    owner_line: Option<usize>,
) -> HashSet<String> {
    let Some(owner_line) = owner_line else {
        return HashSet::new();
    };
    let Some(command) = command_styles
        .iter()
        .find(|command| command.line == owner_line)
    else {
        return HashSet::new();
    };
    if command.verb != "render" || !command.target.trim().starts_with("template ") {
        return HashSet::new();
    }
    ["values", "missing", "output", "overwrite", "artifact_kind"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn with_owner_apply_options(
    command_styles: &[CommandStyleInfo],
    owner_line: Option<usize>,
) -> HashSet<String> {
    let Some(owner_line) = owner_line else {
        return HashSet::new();
    };
    let Some(command) = command_styles
        .iter()
        .find(|command| command.line == owner_line)
    else {
        return HashSet::new();
    };
    if command.verb != "apply" {
        return HashSet::new();
    }
    [
        "template",
        "values",
        "output",
        "missing",
        "overwrite",
        "artifact_kind",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn with_owner_process_options(
    program: &ParsedProgram,
    owner_line: Option<usize>,
) -> HashSet<String> {
    let Some(owner_line) = owner_line else {
        return HashSet::new();
    };
    let is_process_owner = program.items.iter().any(|item| match item {
        AstItem::ProcessRun(process) => process.line == owner_line,
        _ => false,
    });
    if !is_process_owner {
        return HashSet::new();
    }
    [
        "args",
        "cwd",
        "env",
        "tool_version",
        "expected_outputs",
        "timeout",
        "retry",
        "allow_failure",
        "artifact_kind",
        "cache",
        "cache_key",
        "cache_dir",
        "cache_ttl",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn with_owner_db_write_options(
    program: &ParsedProgram,
    owner_line: Option<usize>,
) -> HashSet<String> {
    let Some(owner_line) = owner_line else {
        return HashSet::new();
    };
    let is_db_write_owner = program.items.iter().any(|item| match item {
        AstItem::Write(write) => write.line == owner_line && write.format == "db",
        _ => false,
    });
    if !is_db_write_owner {
        return HashSet::new();
    }
    ["mode", "key", "transaction", "overwrite"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn with_owner_sample_options(
    program: &ParsedProgram,
    owner_line: Option<usize>,
) -> HashSet<String> {
    let Some(owner_line) = owner_line else {
        return HashSet::new();
    };
    let is_sample_owner = program.items.iter().any(|item| match item {
        AstItem::FastBinding(binding) if binding.line == owner_line => {
            sample_generation_method(&binding.expression).is_some()
        }
        _ => false,
    });
    if !is_sample_owner {
        return HashSet::new();
    }
    let mut options = program
        .items
        .iter()
        .filter_map(|item| match item {
            AstItem::WithOption(option) if option.owner_line == Some(owner_line) => {
                Some(option.key.clone())
            }
            _ => None,
        })
        .collect::<HashSet<_>>();
    options.insert("count".to_owned());
    options.insert("seed".to_owned());
    options
}

fn with_owner_net_options(program: &ParsedProgram, owner_line: Option<usize>) -> HashSet<String> {
    let Some(owner_line) = owner_line else {
        return HashSet::new();
    };
    let is_net_owner = program.items.iter().any(|item| match item {
        AstItem::FastBinding(binding) if binding.line == owner_line => {
            crate::net::is_http_request_expression(&binding.expression)
        }
        AstItem::NetDownload(download) => download.line == owner_line,
        _ => false,
    });
    if !is_net_owner {
        return HashSet::new();
    }
    let mut options = program
        .items
        .iter()
        .filter_map(|item| match item {
            AstItem::WithOption(option) if option.owner_line == Some(owner_line) => {
                Some(option.key.clone())
            }
            _ => None,
        })
        .collect::<HashSet<_>>();
    options.extend(
        [
            "query",
            "retry",
            "cache",
            "body",
            "expected_sha256",
            "timeout",
            "fixture",
            "status_code",
            "body_size_limit",
            "response_body_limit",
            "cache_key",
            "cache_dir",
            "cache_ttl",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    options
}

fn with_options_for_owner(
    program: &ParsedProgram,
    owner_line: Option<usize>,
) -> Vec<WithOptionDecl> {
    program
        .items
        .iter()
        .filter_map(|item| match item {
            AstItem::WithOption(option) if option.owner_line == owner_line => Some(option.clone()),
            _ => None,
        })
        .collect()
}

fn analyze_with_option(
    option: &WithOptionDecl,
    owner_type: Option<&SemanticType>,
    extra_known_options: &HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) -> WithOptionInfo {
    let mut status = "accepted".to_owned();
    if !known_with_option(&option.key) && !extra_known_options.contains(&option.key) {
        diagnostics.push(Diagnostic::error(
            "E-WITH-OPTION-001",
            option.line,
            &format!("Unknown with option `{}`.", option.key),
            Some("Use supported options such as `method`, `backend`, `title`, `type`, `uncertainty`, `unit x`, or `unit y`."),
        ));
        return WithOptionInfo {
            key: option.key.clone(),
            value: option.value.clone(),
            status: "unknown_option".to_owned(),
            line: option.line,
        };
    }
    if option.key == "display_unit" || option.key.starts_with("unit ") {
        if let Some(owner_type) = owner_type {
            validate_requested_unit(
                &option.key,
                &owner_type.quantity_kind,
                &option.value,
                option.line,
                "E-WITH-UNIT-001",
                diagnostics,
            );
        }
    }
    if option.key == "sensor_std"
        && !validate_timeseries_sensor_std_option(option, owner_type, diagnostics)
    {
        status = "invalid_sensor_std".to_owned();
    }
    WithOptionInfo {
        key: option.key.clone(),
        value: option.value.clone(),
        status,
        line: option.line,
    }
}

fn validate_timeseries_sensor_std_option(
    option: &WithOptionDecl,
    owner_type: Option<&SemanticType>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(owner_type) = owner_type else {
        diagnostics.push(Diagnostic::error(
            "E-UNC-TS-STD-001",
            option.line,
            "`sensor_std` must be attached to a typed TimeSeries binding.",
            Some("Attach `with { sensor_std = 0.2 K }` to a `TimeSeries[...] of ...` binding."),
        ));
        return false;
    };
    let Some((_axis, value_quantity)) =
        crate::stats::time_series_quantity(&owner_type.quantity_kind)
    else {
        diagnostics.push(Diagnostic::error(
            "E-UNC-TS-STD-001",
            option.line,
            "`sensor_std` is supported only for TimeSeries uncertainty metadata.",
            Some("Use scalar uncertainty constructors for scalar values."),
        ));
        return false;
    };
    let Some((stddev, unit)) = numeric_literal_with_optional_unit(&option.value) else {
        diagnostics.push(Diagnostic::error(
            "E-UNC-TS-STD-001",
            option.line,
            &format!(
                "`sensor_std` must be a numeric value with a unit, got `{}`.",
                option.value
            ),
            Some("Use a form such as `sensor_std = 0.2 K`."),
        ));
        return false;
    };
    if stddev < 0.0 {
        diagnostics.push(Diagnostic::error(
            "E-UNC-TS-STD-001",
            option.line,
            "`sensor_std` must be non-negative.",
            Some("Use zero or a positive standard deviation."),
        ));
        return false;
    }
    let Some(unit) = unit else {
        diagnostics.push(Diagnostic::error(
            "E-UNC-TS-STD-001",
            option.line,
            "`sensor_std` must include a unit.",
            Some("Use a form such as `sensor_std = 0.2 K`."),
        ));
        return false;
    };
    let Some(unit_quantity) = candidates_for_unit(&unit).first().copied() else {
        diagnostics.push(Diagnostic::error(
            "E-UNC-TS-STD-001",
            option.line,
            &format!("`sensor_std` unit `{unit}` is not supported."),
            Some("Use a registered unit compatible with the TimeSeries value quantity."),
        ));
        return false;
    };
    let expected_dimension = dimension_for_quantity(&value_quantity);
    let actual_dimension = dimension_for_quantity(unit_quantity.quantity_kind);
    if !dimensions_compatible(&expected_dimension, &actual_dimension) {
        diagnostics.push(Diagnostic::error(
            "E-UNC-TS-STD-001",
            option.line,
            &format!(
                "`sensor_std` has dimension {actual_dimension}, expected {expected_dimension}."
            ),
            Some("Use a sensor standard deviation unit compatible with the TimeSeries value."),
        ));
        return false;
    }
    true
}

fn validate_uncertainty_policy_options(
    options: &mut [WithOptionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let has_uncertainty_policy = options
        .iter()
        .any(|option| option.key == "uncertainty" && option.status == "accepted");
    if !has_uncertainty_policy {
        return;
    }

    let mut monte_carlo_policy_line = None;
    for option in options.iter_mut() {
        match option.key.as_str() {
            "uncertainty" => {
                let policy = option.value.trim().to_ascii_lowercase();
                if matches!(
                    policy.as_str(),
                    "linear" | "interval" | "monte_carlo" | "ensemble"
                ) {
                    if policy == "monte_carlo" {
                        monte_carlo_policy_line = Some(option.line);
                    }
                } else {
                    option.status = "invalid_uncertainty_policy".to_owned();
                    diagnostics.push(Diagnostic::error(
                        "E-WITH-UNCERTAINTY-POLICY-001",
                        option.line,
                        &format!("Unknown uncertainty propagation policy `{}`.", option.value),
                        Some("Use `linear`, `interval`, `monte_carlo`, or `ensemble`."),
                    ));
                }
            }
            "samples" => {
                if parse_positive_count(&option.value).is_none() {
                    option.status = "invalid_samples".to_owned();
                    diagnostics.push(Diagnostic::error(
                        "E-WITH-UNCERTAINTY-SAMPLES-001",
                        option.line,
                        &format!(
                            "Uncertainty propagation samples must be a positive integer, got `{}`.",
                            option.value
                        ),
                        Some("Use `samples = 64` or another positive count."),
                    ));
                }
            }
            "seed" => {
                if parse_deterministic_seed(&option.value).is_none() {
                    option.status = "invalid_seed".to_owned();
                    diagnostics.push(Diagnostic::error(
                        "E-WITH-UNCERTAINTY-SEED-001",
                        option.line,
                        &format!(
                            "Uncertainty propagation seed must be a deterministic integer, got `{}`.",
                            option.value
                        ),
                        Some("Use `seed = 7` or another non-negative integer seed."),
                    ));
                }
            }
            _ => {}
        }
    }

    if let Some(line) = monte_carlo_policy_line {
        let has_seed = options
            .iter()
            .any(|option| option.key == "seed" && option.status == "accepted");
        if !has_seed {
            diagnostics.push(Diagnostic::warning(
                "W-WITH-UNCERTAINTY-SEED-001",
                line,
                "`monte_carlo` uncertainty propagation is not reproducible without a seed.",
                Some("Add `seed = 7` or choose a deterministic policy such as `linear`."),
            ));
        }
    }
}

fn parse_positive_count(value: &str) -> Option<usize> {
    let count = value.trim().parse::<usize>().ok()?;
    (count > 0).then_some(count)
}

fn parse_deterministic_seed(value: &str) -> Option<u64> {
    value.trim().parse::<u64>().ok()
}

fn known_with_option(key: &str) -> bool {
    matches!(
        key,
        "method"
            | "backend"
            | "title"
            | "type"
            | "unit x"
            | "unit y"
            | "display_unit"
            | "solver"
            | "timestep"
            | "duration"
            | "T_out"
            | "Q_internal"
            | "solar"
            | "tolerance"
            | "max_iter"
            | "relaxation"
            | "initial"
            | "initial_algebraic"
            | "initial_derivative"
            | "inputs"
            | "mass_matrix"
            | "finite_difference_step"
            | "damping"
            | "line_search_steps"
            | "variable_scale"
            | "variable_scales"
            | "jacobian"
            | "residual_scale"
            | "residual_scales"
            | "consistency_tolerance"
            | "algebraic_initialization"
            | "seed"
            | "uncertainty"
            | "samples"
            | "sensor_std"
            | "confidence_band"
            | "output"
            | "overwrite"
            | "confirm"
            | "recursive"
            | "args"
            | "cwd"
            | "tool_version"
            | "expected_outputs"
            | "artifact_kind"
            | "allow_failure"
            | "cache"
            | "cache_key"
            | "cache_dir"
            | "cache_ttl"
            | "expected_sha256"
            | "on_none"
            | "on_many"
    )
}

fn with_owner_semantic_type(
    owner_line: Option<usize>,
    typed_bindings: &[TypedBinding],
    command_styles: &[CommandStyleInfo],
) -> Option<SemanticType> {
    let owner_line = owner_line?;
    if let Some(binding) = typed_bindings
        .iter()
        .find(|binding| binding.line == owner_line)
    {
        return Some(binding.semantic_type.clone());
    }
    let command = command_styles
        .iter()
        .find(|command| command.line == owner_line)?;
    typed_bindings
        .iter()
        .find(|binding| binding.name == command.target)
        .map(|binding| binding.semantic_type.clone())
}

fn validate_where_local_uses(
    program: &ParsedProgram,
    where_blocks: &[WhereBlockInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut local_owners = HashMap::new();
    let mut local_lines = HashSet::new();
    for block in where_blocks {
        for binding in &block.bindings {
            local_owners.insert(binding.name.clone(), block.owner_line);
            local_lines.insert(binding.line);
        }
    }
    if local_owners.is_empty() {
        return;
    }
    for item in &program.items {
        match item {
            AstItem::FastBinding(binding) => validate_where_expression_scope(
                &binding.expression,
                binding.line,
                &local_owners,
                &local_lines,
                diagnostics,
            ),
            AstItem::ExplicitDecl(declaration) => {
                if let Some(expression) = &declaration.expression {
                    validate_where_expression_scope(
                        expression,
                        declaration.line,
                        &local_owners,
                        &local_lines,
                        diagnostics,
                    );
                }
            }
            AstItem::Print(print) => validate_where_expression_scope(
                &print.template,
                print.line,
                &local_owners,
                &local_lines,
                diagnostics,
            ),
            AstItem::CsvExportField(field) => validate_where_expression_scope(
                &field.expression,
                field.line,
                &local_owners,
                &local_lines,
                diagnostics,
            ),
            AstItem::Summary(summary) => validate_where_expression_scope(
                &summary.source,
                summary.line,
                &local_owners,
                &local_lines,
                diagnostics,
            ),
            AstItem::CommandStyle(command) => validate_where_expression_scope(
                &command.target,
                command.line,
                &local_owners,
                &local_lines,
                diagnostics,
            ),
            _ => {}
        }
    }
}

fn validate_where_expression_scope(
    expression: &str,
    line: usize,
    local_owners: &HashMap<String, Option<usize>>,
    local_lines: &HashSet<usize>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if local_lines.contains(&line) {
        return;
    }
    for (name, owner_line) in local_owners {
        if owner_line.is_some_and(|owner| owner == line) {
            continue;
        }
        if expression_mentions_identifier(expression, name) {
            diagnostics.push(Diagnostic::error(
                "E-NAME-LOCAL-001",
                line,
                &format!("Where-local `{name}` is not visible outside its owner expression."),
                Some("Move the binding to top-level if it should be reused."),
            ));
        }
    }
}

fn expression_identifiers(expression: &str) -> Vec<String> {
    expression
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || character == '_' || character == '.')
        })
        .filter_map(|token| token.split('.').next())
        .filter(|token| !token.is_empty())
        .filter(|token| {
            token
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
        })
        .map(str::to_owned)
        .collect()
}

fn top_level_workflow_line(program: &ParsedProgram) -> usize {
    program
        .items
        .iter()
        .find_map(|item| match item {
            AstItem::FastBinding(binding) if binding.context == ParseContext::TopLevel => {
                Some(binding.line)
            }
            AstItem::ExplicitDecl(declaration) if declaration.context == ParseContext::TopLevel => {
                Some(declaration.line)
            }
            AstItem::Const(declaration) if declaration.context == ParseContext::TopLevel => {
                Some(declaration.line)
            }
            AstItem::Print(print) if print.context == ParseContext::TopLevel => Some(print.line),
            AstItem::CsvExport(export) if export.context == ParseContext::TopLevel => {
                Some(export.line)
            }
            _ => None,
        })
        .unwrap_or(1)
}

fn analyze_function_decl(
    function: &FunctionDecl,
    diagnostics: &mut Vec<Diagnostic>,
) -> FunctionInfo {
    let parameters = function
        .parameters
        .iter()
        .map(|parameter| analyze_function_parameter(parameter, function.span.line, diagnostics))
        .collect::<Vec<_>>();
    let return_display_unit = function
        .return_unit
        .clone()
        .unwrap_or_else(|| default_unit_for_type(&function.return_type));
    let return_canonical_unit = default_unit_for_type(&function.return_type);
    let return_dimension = dimension_for_type(&function.return_type);
    if !known_decl_type(&function.return_type) {
        diagnostics.push(Diagnostic::error(
            "E-FN-TYPE-001",
            function.span.line,
            &format!(
                "Function `{}` returns unknown quantity kind `{}`.",
                function.name, function.return_type
            ),
            Some("Use a known quantity kind or supported scalar type such as String, CsvFile, or DirectoryPath."),
        ));
    }
    FunctionInfo {
        name: function.name.clone(),
        parameters,
        locals: Vec::new(),
        return_quantity_kind: function.return_type.clone(),
        return_display_unit,
        return_canonical_unit,
        return_dimension,
        return_expression: None,
        status: "declared".to_owned(),
        line: function.span.line,
    }
}

fn analyze_function_parameter(
    parameter: &FunctionParamDecl,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> FunctionParamInfo {
    let display_unit = parameter
        .unit
        .clone()
        .unwrap_or_else(|| default_unit_for_type(&parameter.type_name));
    let canonical_unit = default_unit_for_type(&parameter.type_name);
    let dimension = dimension_for_type(&parameter.type_name);
    if !known_decl_type(&parameter.type_name) {
        diagnostics.push(Diagnostic::error(
            "E-FN-TYPE-002",
            line,
            &format!(
                "Function parameter `{}` has unknown quantity kind `{}`.",
                parameter.name, parameter.type_name
            ),
            Some(
                "Annotate function parameters with known quantity kinds or supported scalar types.",
            ),
        ));
    }
    FunctionParamInfo {
        name: parameter.name.clone(),
        quantity_kind: parameter.type_name.clone(),
        display_unit,
        canonical_unit,
        dimension,
    }
}

fn analyze_function_return(
    return_decl: &ReturnDecl,
    function: &mut FunctionInfo,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if function.return_expression.is_some() {
        diagnostics.push(Diagnostic::error(
            "E-FN-RETURN-001",
            return_decl.line,
            &format!(
                "Function `{}` has more than one return expression.",
                function.name
            ),
            Some("Keep one explicit `return ...` in the function body."),
        ));
        return;
    }
    function.return_expression = Some(return_decl.expression.clone());
}

fn reject_function_side_effect(
    context: ParseContext,
    operation: &str,
    line: usize,
    current_function_index: Option<usize>,
    functions: &[FunctionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if context != ParseContext::Function {
        return false;
    }
    let function_name = current_function_index
        .and_then(|index| functions.get(index))
        .map(|function| function.name.as_str())
        .unwrap_or("<unknown>");
    diagnostics.push(function_side_effect_diagnostic(
        function_name,
        operation,
        line,
    ));
    true
}

fn function_side_effect_diagnostic(
    function_name: &str,
    operation: &str,
    line: usize,
) -> Diagnostic {
    Diagnostic::error(
        "E-FN-SIDE-EFFECT-001",
        line,
        &format!("Function `{function_name}` cannot perform side-effecting {operation}."),
        Some("Keep functions pure; move side effects to top-level workflow statements."),
    )
}

fn validate_function_returns(
    functions: &mut [FunctionInfo],
    consts: &[ConstInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for function in functions.iter_mut() {
        let Some(expression) = function.return_expression.clone() else {
            diagnostics.push(Diagnostic::error(
                "E-FN-RETURN-002",
                function.line,
                &format!("Function `{}` does not return a value.", function.name),
                Some("Add `return <expression>` inside the function body."),
            ));
            function.status = "missing_return".to_owned();
            continue;
        };
        let has_side_effect = validate_function_purity(function, &expression, diagnostics);
        if preview_scalar_type(&function.return_quantity_kind) {
            if !has_side_effect {
                function.status = "scalar_preview".to_owned();
            }
            continue;
        }
        let mut symbols = consts
            .iter()
            .filter(|const_info| const_info.importable)
            .map(|const_info| DimensionSymbol {
                name: const_info.name.clone(),
                dimension: const_info.dimension.clone(),
            })
            .collect::<Vec<_>>();
        symbols.extend(
            function
                .parameters
                .iter()
                .map(|parameter| DimensionSymbol {
                    name: parameter.name.clone(),
                    dimension: parameter.dimension.clone(),
                })
                .collect::<Vec<_>>(),
        );
        for local in &function.locals {
            let Some(local_dimension) =
                expression_dimension_with_symbols(&local.expression, &symbols)
            else {
                diagnostics.push(Diagnostic::error(
                    "E-FN-LOCAL-001",
                    local.line,
                    &format!(
                        "Function `{}` local `{}` could not be type-checked.",
                        function.name, local.name
                    ),
                    Some("Use parameters, previous locals, const values, and literals with units."),
                ));
                continue;
            };
            symbols.push(DimensionSymbol {
                name: local.name.clone(),
                dimension: local_dimension,
            });
        }
        let Some(actual_dimension) = expression_dimension_with_symbols(&expression, &symbols)
        else {
            diagnostics.push(Diagnostic::error(
                "E-FN-RETURN-003",
                function.line,
                &format!(
                    "Function `{}` return expression could not be type-checked.",
                    function.name
                ),
                Some("Use parameters, literals with units, and supported arithmetic in the return expression."),
            ));
            function.status = "unit_unresolved".to_owned();
            continue;
        };
        if !dimensions_compatible(&function.return_dimension, &actual_dimension) {
            diagnostics.push(Diagnostic::error(
                "E-FN-RETURN-004",
                function.line,
                &format!(
                    "Function `{}` returns {}, but its body has dimension {}.",
                    function.name, function.return_dimension, actual_dimension
                ),
                Some("Make the return annotation match the expression quantity or fix the expression units."),
            ));
            function.status = "unit_mismatch".to_owned();
        } else if !has_side_effect {
            function.status = "unit_consistent".to_owned();
        }
    }
}

fn validate_function_purity(
    function: &mut FunctionInfo,
    return_expression: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let mut has_side_effect = false;
    for local in &function.locals {
        if expression_has_side_effect(&local.expression) {
            diagnostics.push(function_side_effect_diagnostic(
                &function.name,
                "local expression",
                local.line,
            ));
            has_side_effect = true;
        }
    }
    if expression_has_side_effect(return_expression) {
        diagnostics.push(function_side_effect_diagnostic(
            &function.name,
            "return expression",
            function.line,
        ));
        has_side_effect = true;
    }
    if has_side_effect {
        function.status = "side_effect_rejected".to_owned();
    }
    has_side_effect
}

fn analyze_print_decl(
    print: &PrintDecl,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
    prints: &mut Vec<PrintInfo>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if print.level != "print"
        && !matches!(print.level.as_str(), "debug" | "info" | "warn" | "error")
    {
        diagnostics.push(Diagnostic::error(
            "E-LOG-LEVEL-001",
            print.line,
            if print.level.is_empty() {
                "`log` requires a level."
            } else {
                "Unsupported `log` level."
            },
            Some("Use `log debug \"...\"`, `log info \"...\"`, `log warn \"...\"`, or `log error \"...\"`."),
        ));
    }
    let fields = analyze_format_fields(
        &print.template,
        print.line,
        typed_bindings,
        functions,
        PRINT_FORMAT_DIAGNOSTICS,
        diagnostics,
    );
    prints.push(PrintInfo {
        level: print.level.clone(),
        template: print.template.clone(),
        fields,
        line: print.line,
    });
}

fn analyze_csv_export_decl(
    export: &CsvExportDecl,
    diagnostics: &mut Vec<Diagnostic>,
) -> CsvExportInfo {
    if export.source != "summary" {
        diagnostics.push(Diagnostic::error(
            "E-EXPORT-CSV-002",
            export.line,
            &format!(
                "CSV export source `{}` is not supported in the current runtime.",
                export.source
            ),
            Some("Use `export summary to csv \"summary.csv\" { ... }` for scalar summary exports."),
        ));
    }
    CsvExportInfo {
        source: export.source.clone(),
        format: export.format.clone(),
        path: export.path.clone(),
        fields: Vec::new(),
        line: export.line,
    }
}

fn analyze_csv_export_field_decl(
    field: &CsvExportFieldDecl,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CsvExportFieldInfo> {
    let semantic_type =
        resolve_format_expression_type(&field.expression, typed_bindings, functions).or_else(
            || {
                diagnostics.push(unknown_format_expression_diagnostic(
                    &field.expression,
                    field.line,
                    "E-EXPORT-CSV-003",
                ));
                None
            },
        )?;
    let precision = field
        .format
        .as_deref()
        .and_then(|format| parse_format_spec(format).precision);
    if let Some(unit) = &field.display_unit {
        validate_requested_unit(
            &field.expression,
            &semantic_type.quantity_kind,
            unit,
            field.line,
            "E-EXPORT-CSV-004",
            diagnostics,
        );
    }
    Some(CsvExportFieldInfo {
        name: export_field_name(&field.expression),
        expression: field.expression.clone(),
        quantity_kind: semantic_type.quantity_kind.clone(),
        display_unit: semantic_type.display_unit.clone(),
        requested_unit: field.display_unit.clone(),
        precision,
        line: field.line,
    })
}

fn analyze_write_decl(
    write: &WriteDecl,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<WriteInfo> {
    if write.context != ParseContext::TopLevel {
        diagnostics.push(Diagnostic::error(
            "E-WRITE-001",
            write.line,
            "`write` is supported only in the top-level workflow.",
            Some("Move the write statement to the root workflow so the output is reviewable."),
        ));
        return None;
    }
    if write.format == "db" {
        return analyze_db_write_decl(write, typed_bindings, diagnostics);
    }
    if !matches!(write.format.as_str(), "text" | "json" | "standard_text") {
        diagnostics.push(Diagnostic::error(
            "E-WRITE-002",
            write.line,
            &format!("Write format `{}` is not supported.", write.format),
            Some("Use `write text`, `write json`, or `write standard_text`."),
        ));
        return None;
    }
    if write.format == "standard_text" {
        let source = write.expression.trim();
        let source_type = typed_bindings
            .iter()
            .find(|binding| binding.name == source)
            .map(|binding| binding.semantic_type.quantity_kind.as_str());
        if !source_type.is_some_and(is_standard_text_table_quantity_kind) {
            diagnostics.push(Diagnostic::error(
                "E-WRITE-STANDARD-TEXT-001",
                write.line,
                &format!("Standard text source `{source}` is not a typed table."),
                Some(
                    "Write a promoted, generated, derived, or joined table with `write standard_text <table>`.",
                ),
            ));
            return None;
        }
    }
    if write.format == "text" {
        if let Some(template) = string_literal_content(&write.expression) {
            analyze_format_fields(
                template,
                write.line,
                typed_bindings,
                functions,
                WRITE_TEXT_FORMAT_DIAGNOSTICS,
                diagnostics,
            );
        }
    }
    let semantic_type = resolve_write_expression_type(&write.expression, typed_bindings, functions)
        .or_else(|| {
            diagnostics.push(unknown_format_expression_diagnostic(
                &write.expression,
                write.line,
                "E-WRITE-003",
            ));
            None
        })?;
    Some(WriteInfo {
        format: write.format.clone(),
        path: write.path.clone(),
        expression: write.expression.clone(),
        quantity_kind: semantic_type.quantity_kind,
        display_unit: semantic_type.display_unit,
        line: write.line,
    })
}

fn string_literal_content(expression: &str) -> Option<&str> {
    expression
        .trim()
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
}

fn analyze_db_write_decl(
    write: &WriteDecl,
    typed_bindings: &[TypedBinding],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<WriteInfo> {
    let source = write.expression.trim();
    let source_type = typed_bindings
        .iter()
        .find(|binding| binding.name == source)
        .map(|binding| binding.semantic_type.quantity_kind.as_str());
    if !source_type.is_some_and(is_materialized_table_quantity_kind) {
        diagnostics.push(Diagnostic::error(
            "E-DB-SCHEMA-MISMATCH",
            write.line,
            &format!("DB write source `{source}` is not a typed table."),
            Some("Write a promoted, generated, or derived table to `db.table(\"name\")`."),
        ));
        return None;
    }
    let Some((connection, _table)) = db_table_target_expression(&write.path) else {
        diagnostics.push(Diagnostic::error(
            "E-DB-CONNECT",
            write.line,
            &format!("DB write target `{}` is not a SQLite table reference.", write.path),
            Some("Use `write <table_binding> to db.table(\"table_name\")` after `db = open sqlite file(\"...\")`."),
        ));
        return None;
    };
    let connection_type = typed_bindings
        .iter()
        .find(|binding| binding.name == connection)
        .map(|binding| binding.semantic_type.quantity_kind.as_str());
    if connection_type != Some("DbConnection") {
        diagnostics.push(Diagnostic::error(
            "E-DB-CONNECT",
            write.line,
            &format!("DB connection `{connection}` is not a SQLite connection binding."),
            Some("Declare it with `db = open sqlite file(\"outputs/results.sqlite\")`."),
        ));
        return None;
    }
    Some(WriteInfo {
        format: "db".to_owned(),
        path: write.path.clone(),
        expression: write.expression.clone(),
        quantity_kind: "DbWrite".to_owned(),
        display_unit: "sqlite".to_owned(),
        line: write.line,
    })
}

fn analyze_file_operation_decl(
    operation: &FileOperationDecl,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<FileOperationInfo> {
    if operation.context != ParseContext::TopLevel {
        diagnostics.push(Diagnostic::error(
            "E-FS-001",
            operation.line,
            "File operations are supported only in the top-level workflow.",
            Some("Move the operation to the root workflow so the filesystem change is reviewable."),
        ));
        return None;
    }
    if !matches!(operation.operation.as_str(), "copy" | "move" | "delete") {
        diagnostics.push(Diagnostic::error(
            "E-FS-002",
            operation.line,
            &format!("File operation `{}` is not supported.", operation.operation),
            Some("Use `copy`, `move`, or `delete`."),
        ));
        return None;
    }
    if matches!(operation.operation.as_str(), "copy" | "move") && operation.destination.is_none() {
        diagnostics.push(Diagnostic::error(
            "E-FS-003",
            operation.line,
            &format!("`{}` requires a destination path.", operation.operation),
            Some(&format!(
                "Write `{} <source> to <destination>`.",
                operation.operation
            )),
        ));
        return None;
    }
    Some(FileOperationInfo {
        operation: operation.operation.clone(),
        source: operation.source.clone(),
        destination: operation.destination.clone(),
        line: operation.line,
    })
}

fn analyze_process_run_decl(
    process: &ProcessRunDecl,
    diagnostics: &mut Vec<Diagnostic>,
    typed_bindings: &mut Vec<TypedBinding>,
    hover_hints: &mut Vec<HoverHint>,
    type_infos: &mut Vec<TypeInfo>,
) -> Option<ProcessRunInfo> {
    if process.context != ParseContext::TopLevel {
        diagnostics.push(Diagnostic::error(
            "E-PROCESS-001",
            process.line,
            "`run command` is supported only in the top-level workflow.",
            Some("Move the process statement to the root workflow so it is reviewable."),
        ));
        return None;
    }
    let Some(binding) = process.binding.as_ref().filter(|value| !value.is_empty()) else {
        diagnostics.push(Diagnostic::error(
            "E-PROCESS-BINDING-001",
            process.line,
            "`run command` must bind a ProcessResult.",
            Some(
                "Write `result = run command \"tool\"` so the exit code and output are reviewable.",
            ),
        ));
        return None;
    };
    if process.command.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "E-PROCESS-CMD-001",
            process.line,
            "`run command` requires a command string.",
            Some("Write `result = run command \"tool\"` and pass arguments with `with { args = [...] }`."),
        ));
        return None;
    }
    if typed_bindings
        .iter()
        .any(|existing| existing.name == *binding && existing.line != process.line)
    {
        diagnostics.push(Diagnostic::error(
            "E-PROCESS-BINDING-002",
            process.line,
            &format!("ProcessResult binding `{binding}` conflicts with an existing binding."),
            Some("Use a unique result binding name for the process run."),
        ));
        return None;
    }

    let semantic_type = SemanticType {
        quantity_kind: "ProcessResult".to_owned(),
        display_unit: String::new(),
    };
    typed_bindings.push(TypedBinding {
        name: binding.clone(),
        semantic_type: semantic_type.clone(),
        line: process.line,
    });
    hover_hints.push(HoverHint::inferred(
        binding.clone(),
        semantic_type.quantity_kind.clone(),
        semantic_type.display_unit.clone(),
        format!("run command \"{}\"", process.command),
        process.span,
    ));
    type_infos.push(TypeInfo {
        name: binding.clone(),
        quantity_kind: semantic_type.quantity_kind,
        display_unit: semantic_type.display_unit,
        canonical_unit: String::new(),
        dimension: "ExternalProcess".to_owned(),
        source: TypeInfoSource::Inferred,
        line: process.line,
        span: process.span,
    });
    Some(ProcessRunInfo {
        binding: binding.clone(),
        command: process.command.clone(),
        line: process.line,
    })
}

fn analyze_test_decl(test: &TestDecl, diagnostics: &mut Vec<Diagnostic>) -> Option<TestInfo> {
    if test.context != ParseContext::TopLevel {
        diagnostics.push(Diagnostic::error(
            "E-TEST-001",
            test.line,
            "`test` blocks are supported only at top level.",
            Some("Move the test block to the root workflow so it can inspect public results."),
        ));
        return None;
    }
    if test.name.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "E-TEST-NAME-001",
            test.line,
            "`test` requires a name.",
            Some("Write `test \"name\" { ... }`."),
        ));
        return None;
    }
    Some(TestInfo {
        name: test.name.clone(),
        assertions: Vec::new(),
        goldens: Vec::new(),
        line: test.line,
    })
}

fn analyze_assert_decl(
    assertion: &AssertDecl,
    current_test_index: Option<usize>,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
    tests: &mut [TestInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if assertion.context != ParseContext::Test {
        diagnostics.push(Diagnostic::error(
            "E-ASSERT-001",
            assertion.line,
            "`assert` is supported only inside a `test` block.",
            Some("Wrap assertions in `test \"name\" { ... }`."),
        ));
        return;
    }
    let Some(test_index) = current_test_index else {
        diagnostics.push(Diagnostic::error(
            "E-ASSERT-001",
            assertion.line,
            "`assert` is not attached to a test block.",
            Some("Place the assertion inside `test \"name\" { ... }`."),
        ));
        return;
    };
    if assertion.left.is_empty() || assertion.operator.is_empty() || assertion.right.is_empty() {
        diagnostics.push(Diagnostic::error(
            "E-ASSERT-002",
            assertion.line,
            "`assert` requires `left operator right`.",
            Some("Write forms such as `assert E_coil > 0 kWh` or `assert E_coil == 1.2 kWh within 0.1 kWh`."),
        ));
        return;
    }
    let left_type = assert_expression_semantic_type(&assertion.left, typed_bindings, functions);
    let right_type = assert_expression_semantic_type(&assertion.right, typed_bindings, functions);
    if left_type.is_none() {
        diagnostics.push(Diagnostic::error(
            "E-ASSERT-EXPR-001",
            assertion.line,
            &format!("Cannot resolve assert expression `{}`.", assertion.left),
            Some("Assert a typed binding, statistic, function call, literal, path, Bool, or String value."),
        ));
    }
    if right_type.is_none() {
        diagnostics.push(Diagnostic::error(
            "E-ASSERT-EXPR-001",
            assertion.line,
            &format!("Cannot resolve assert expression `{}`.", assertion.right),
            Some("Assert a typed binding, statistic, function call, literal, path, Bool, or String value."),
        ));
    }
    if let (Some(left), Some(right)) = (&left_type, &right_type) {
        if !push_direct_uncertainty_comparison_diagnostic(
            "Assert",
            &assertion.left,
            &assertion.right,
            left,
            right,
            assertion.line,
            diagnostics,
        ) {
            validate_comparison_dimensions(
                "Assert",
                &assertion.left,
                &assertion.right,
                left,
                right,
                assertion.line,
                typed_bindings,
                diagnostics,
            );
        }
    }
    if let Some(tolerance) = &assertion.tolerance {
        if !matches!(assertion.operator.as_str(), "==" | "!=") {
            diagnostics.push(Diagnostic::error(
                "E-ASSERT-TOL-001",
                assertion.line,
                "`within` is supported only with equality assertions.",
                Some("Use `assert value == expected within tolerance`."),
            ));
        }
        let tolerance_type = assert_expression_semantic_type(tolerance, typed_bindings, functions);
        if tolerance_type.is_none() {
            diagnostics.push(Diagnostic::error(
                "E-ASSERT-EXPR-001",
                assertion.line,
                &format!("Cannot resolve assert tolerance `{tolerance}`."),
                Some("Use a numeric tolerance literal such as `0.01 kWh`."),
            ));
        }
        if let (Some(left), Some(tolerance_type)) = (&left_type, &tolerance_type) {
            let left_dimension = dimension_for_quantity(&left.quantity_kind);
            let tolerance_dimension = dimension_for_quantity(&tolerance_type.quantity_kind);
            if !dimensions_compatible(&left_dimension, &tolerance_dimension) {
                diagnostics.push(Diagnostic::error(
                    "E-ASSERT-TOL-002",
                    assertion.line,
                    &format!(
                        "Assert tolerance `{tolerance}` has dimension {tolerance_dimension}, expected {left_dimension}."
                    ),
                    Some("Use a tolerance with the same dimension as the asserted value."),
                ));
            }
        }
    }
    if let Some(test) = tests.get_mut(test_index) {
        test.assertions.push(AssertInfo {
            left: assertion.left.clone(),
            operator: assertion.operator.clone(),
            right: assertion.right.clone(),
            tolerance: assertion.tolerance.clone(),
            line: assertion.line,
        });
    }
}

fn analyze_golden_decl(
    golden: &GoldenDecl,
    current_test_index: Option<usize>,
    tests: &mut [TestInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if golden.context != ParseContext::Test {
        diagnostics.push(Diagnostic::error(
            "E-GOLDEN-001",
            golden.line,
            "`golden` is supported only inside a `test` block.",
            Some("Wrap golden checks in `test \"name\" { ... }`."),
        ));
        return;
    }
    let Some(test_index) = current_test_index else {
        diagnostics.push(Diagnostic::error(
            "E-GOLDEN-001",
            golden.line,
            "`golden` is not attached to a test block.",
            Some("Place the golden check inside `test \"name\" { ... }`."),
        ));
        return;
    };
    if golden.artifact.trim().is_empty() || golden.expected.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "E-GOLDEN-002",
            golden.line,
            "`golden` requires an artifact path and expected path.",
            Some("Write `golden \"summary.csv\" matches file(\"golden/summary.csv\")`."),
        ));
        return;
    }
    if let Some(test) = tests.get_mut(test_index) {
        test.goldens.push(GoldenInfo {
            artifact: golden.artifact.clone(),
            expected: golden.expected.clone(),
            line: golden.line,
        });
    }
}

fn validate_file_operation_options(
    operations: &[FileOperationInfo],
    with_blocks: &[WithBlockInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for operation in operations {
        if matches!(operation.operation.as_str(), "move" | "delete")
            && !with_option_bool(with_blocks, operation.line, "confirm")
        {
            diagnostics.push(Diagnostic::error(
                "E-FS-CONFIRM-001",
                operation.line,
                &format!(
                    "`{}` requires `with {{ confirm = true }}`.",
                    operation.operation
                ),
                Some(
                    "Attach an explicit confirmation block so the filesystem mutation is visible.",
                ),
            ));
        }
        if operation.operation == "delete"
            && operation.source.trim_start().starts_with("dir(")
            && !with_option_bool(with_blocks, operation.line, "recursive")
        {
            diagnostics.push(Diagnostic::error(
                "E-FS-DELETE-001",
                operation.line,
                "`delete dir(...)` requires `with { recursive = true }`.",
                Some("Delete directory trees only with both `recursive = true` and `confirm = true`."),
            ));
        }
    }
}

fn validate_process_options(
    processes: &[ProcessRunInfo],
    with_blocks: &[WithBlockInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for process in processes {
        let options = with_blocks
            .iter()
            .find(|block| block.owner_line == Some(process.line))
            .map(|block| block.options.as_slice())
            .unwrap_or(&[]);
        if let Some(option) = accepted_option(options, "env") {
            validate_process_env_option(option, diagnostics);
        }
        if let Some(option) = accepted_option(options, "cwd") {
            validate_process_cwd_option(option, diagnostics);
        }
        if let Some(option) = accepted_option(options, "timeout") {
            validate_process_timeout_option(option, diagnostics);
        }
        if let Some(option) = accepted_option(options, "retry") {
            validate_process_retry_option(option, diagnostics);
        }
        if let Some(option) = accepted_option(options, "allow_failure") {
            validate_process_allow_failure_option(option, diagnostics);
        }
    }
}

fn validate_process_env_option(option: &WithOptionInfo, diagnostics: &mut Vec<Diagnostic>) {
    let trimmed = option.value.trim();
    let Some(inner) = trimmed
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
    else {
        diagnostics.push(Diagnostic::error(
            "E-PROCESS-ENV-001",
            option.line,
            "`env` expects an inline object.",
            Some("Use `env = { NAME = \"value\" }` with portable environment variable names."),
        ));
        return;
    };
    for entry in split_top_level(inner, &[',', ';']) {
        let Some((key, value)) = entry.split_once('=') else {
            diagnostics.push(Diagnostic::error(
                "E-PROCESS-ENV-001",
                option.line,
                &format!("Process env entry `{entry}` must use `NAME = value`."),
                Some("Use `env = { NAME = \"value\" }`."),
            ));
            continue;
        };
        let key = key.trim();
        if !is_process_env_key(key) {
            diagnostics.push(Diagnostic::error(
                "E-PROCESS-ENV-001",
                option.line,
                &format!("Process env key `{key}` is not portable."),
                Some("Use ASCII names such as `OMP_NUM_THREADS` or `CASE_ID`."),
            ));
        }
        if value.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "E-PROCESS-ENV-001",
                option.line,
                &format!("Process env key `{key}` has an empty value expression."),
                Some("Provide a string, args value, path expression, or numeric literal."),
            ));
        }
    }
}

fn is_process_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn validate_process_cwd_option(option: &WithOptionInfo, diagnostics: &mut Vec<Diagnostic>) {
    let value = option.value.trim();
    if value.is_empty()
        || matches!(value, "true" | "false")
        || numeric_literal_with_optional_unit(value).is_some()
    {
        diagnostics.push(Diagnostic::error(
            "E-PROCESS-CWD-001",
            option.line,
            &format!("Process cwd `{value}` is not a path expression."),
            Some("Use a string, `dir(...)`, `join(...)`, `args.<name>`, or a bound DirectoryPath."),
        ));
    }
}

fn validate_process_timeout_option(option: &WithOptionInfo, diagnostics: &mut Vec<Diagnostic>) {
    if parse_duration_option_seconds(&option.value).is_none() {
        diagnostics.push(Diagnostic::error(
            "E-PROCESS-TIMEOUT",
            option.line,
            &format!("Process timeout `{}` is invalid.", option.value.trim()),
            Some("Use a positive duration with units such as `10 s`, `10 min`, or `1 h`."),
        ));
    }
}

fn validate_process_retry_option(option: &WithOptionInfo, diagnostics: &mut Vec<Diagnostic>) {
    const MAX_PROCESS_RETRY_ATTEMPTS: usize = 5;
    let raw = option.value.trim();
    let Ok(value) = raw.parse::<usize>() else {
        diagnostics.push(Diagnostic::error(
            "E-PROCESS-RETRY-POLICY",
            option.line,
            &format!("Process retry policy `{raw}` is not a whole number."),
            Some("Use `retry = 0` to disable retries or an integer from 1 to 5."),
        ));
        return;
    };
    if value > MAX_PROCESS_RETRY_ATTEMPTS {
        diagnostics.push(Diagnostic::error(
            "E-PROCESS-RETRY-POLICY",
            option.line,
            &format!("Process retry policy `{value}` exceeds the maximum of {MAX_PROCESS_RETRY_ATTEMPTS}."),
            Some("Use a retry count from 0 to 5."),
        ));
    }
}

fn validate_process_allow_failure_option(
    option: &WithOptionInfo,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !matches!(
        option.value.trim().to_ascii_lowercase().as_str(),
        "true" | "false"
    ) {
        diagnostics.push(Diagnostic::error(
            "E-PROCESS-ALLOW-FAILURE",
            option.line,
            &format!(
                "`allow_failure` expects true or false, got `{}`.",
                option.value.trim()
            ),
            Some("Use `allow_failure = true` only when a failed process is expected data."),
        ));
    }
}

fn analyze_sample_generations(
    program: &ParsedProgram,
    with_blocks: &[WithBlockInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<SampleGenerationInfo> {
    program
        .items
        .iter()
        .filter_map(|item| {
            let AstItem::FastBinding(binding) = item else {
                return None;
            };
            let method = sample_generation_method(&binding.expression)?;
            Some(sample_generation_info(
                binding,
                &method,
                with_blocks,
                diagnostics,
            ))
        })
        .collect()
}

fn sample_generation_method(expression: &str) -> Option<String> {
    let method = expression.trim().strip_prefix("sample ")?.trim();
    let method = match method.to_ascii_lowercase().as_str() {
        "grid" => "grid",
        "random" | "uniform" => "random",
        "lhs" | "latin_hypercube" | "latin-hypercube" => "lhs",
        _ => return None,
    };
    Some(method.to_owned())
}

fn sample_generation_info(
    binding: &FastBinding,
    method: &str,
    with_blocks: &[WithBlockInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> SampleGenerationInfo {
    let options = with_blocks
        .iter()
        .find(|block| block.owner_line == Some(binding.line))
        .map(|block| block.options.as_slice())
        .unwrap_or(&[]);
    let count = sample_count_option(options, binding.line, diagnostics).unwrap_or(0);
    let seed = sample_seed_option(options, diagnostics);
    let mut distributions = Vec::new();
    for option in options
        .iter()
        .filter(|option| option.status == "accepted")
        .filter(|option| !matches!(option.key.as_str(), "count" | "seed"))
    {
        if let Some(distribution) = sample_distribution_option(option, diagnostics) {
            distributions.push(distribution);
        }
    }
    if distributions.is_empty() {
        diagnostics.push(Diagnostic::error(
            "E-SAMPLING-RANGE-UNIT",
            binding.line,
            "Sample generation requires at least one `uniform(lower, upper)` parameter.",
            Some("Add a parameter option such as `cooling_cop = uniform(2.5, 5.0)`."),
        ));
    }
    SampleGenerationInfo {
        binding: binding.name.clone(),
        method: method.to_owned(),
        count,
        seed,
        distributions,
        status: if count == 0 {
            "invalid_count".to_owned()
        } else {
            "declared".to_owned()
        },
        line: binding.line,
    }
}

fn sample_count_option(
    options: &[WithOptionInfo],
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<usize> {
    let Some(option) = accepted_option(options, "count") else {
        diagnostics.push(Diagnostic::error(
            "E-SAMPLING-COUNT-INVALID",
            line,
            "Sample generation requires a positive `count`.",
            Some("Use `count = 100` or another positive integer."),
        ));
        return None;
    };
    let raw = option.value.trim();
    let Ok(count) = raw.parse::<usize>() else {
        diagnostics.push(Diagnostic::error(
            "E-SAMPLING-COUNT-INVALID",
            option.line,
            &format!("Sample count `{raw}` is not a positive integer."),
            Some("Use `count = 100` or another positive integer."),
        ));
        return None;
    };
    if count == 0 {
        diagnostics.push(Diagnostic::error(
            "E-SAMPLING-COUNT-INVALID",
            option.line,
            "Sample count must be greater than zero.",
            Some("Use `count = 1` or larger."),
        ));
        return None;
    }
    Some(count)
}

fn sample_seed_option(
    options: &[WithOptionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<u64> {
    let option = accepted_option(options, "seed")?;
    let raw = option.value.trim();
    let Ok(seed) = raw.parse::<u64>() else {
        diagnostics.push(Diagnostic::error(
            "E-SAMPLING-SEED-INVALID",
            option.line,
            &format!("Sample seed `{raw}` is not a non-negative integer."),
            Some("Use `seed = 42` for reproducible random or LHS sampling."),
        ));
        return None;
    };
    Some(seed)
}

fn sample_distribution_option(
    option: &WithOptionInfo,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<SampleDistributionInfo> {
    let value = option.value.trim();
    let inner = value
        .strip_prefix("uniform(")
        .and_then(|rest| rest.strip_suffix(')'));
    let Some(inner) = inner else {
        diagnostics.push(Diagnostic::error(
            "E-SAMPLING-RANGE-UNIT",
            option.line,
            &format!(
                "Sample parameter `{}` must use `uniform(lower, upper)`.",
                option.key
            ),
            Some("Use endpoints with compatible units, for example `uniform(2.5, 5.0)`."),
        ));
        return None;
    };
    let parts = split_top_level(inner, &[',']);
    let [lower_raw, upper_raw] = parts.as_slice() else {
        diagnostics.push(Diagnostic::error(
            "E-SAMPLING-RANGE-UNIT",
            option.line,
            &format!(
                "Sample parameter `{}` requires lower and upper endpoints.",
                option.key
            ),
            Some("Use `uniform(lower, upper)`."),
        ));
        return None;
    };
    let lower = sample_distribution_endpoint(lower_raw, option.line, diagnostics)?;
    let upper = sample_distribution_endpoint(upper_raw, option.line, diagnostics)?;
    if lower.quantity_kind != upper.quantity_kind
        || normalize_sample_unit(&lower.display_unit) != normalize_sample_unit(&upper.display_unit)
    {
        diagnostics.push(Diagnostic::error(
            "E-SAMPLING-RANGE-UNIT",
            option.line,
            &format!(
                "Sample parameter `{}` endpoints have incompatible units `{}` and `{}`.",
                option.key, lower.display_unit, upper.display_unit
            ),
            Some("Use endpoints with the same quantity and unit."),
        ));
        return None;
    }
    if upper.value < lower.value {
        diagnostics.push(Diagnostic::error(
            "E-SAMPLING-RANGE-UNIT",
            option.line,
            &format!(
                "Sample parameter `{}` upper endpoint is below the lower endpoint.",
                option.key
            ),
            Some("Write `uniform(lower, upper)` with lower <= upper."),
        ));
        return None;
    }
    Some(SampleDistributionInfo {
        name: option.key.clone(),
        kind: "uniform".to_owned(),
        lower: lower.value,
        upper: upper.value,
        quantity_kind: lower.quantity_kind,
        display_unit: lower.display_unit,
        canonical_unit: lower.canonical_unit,
        line: option.line,
    })
}

struct SampleDistributionEndpoint {
    value: f64,
    quantity_kind: String,
    display_unit: String,
    canonical_unit: String,
}

fn sample_distribution_endpoint(
    expression: &str,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<SampleDistributionEndpoint> {
    let Some((value, unit)) = numeric_literal_with_optional_unit(expression.trim()) else {
        diagnostics.push(Diagnostic::error(
            "E-SAMPLING-RANGE-UNIT",
            line,
            &format!(
                "Sample endpoint `{}` is not a numeric literal.",
                expression.trim()
            ),
            Some("Use numeric endpoints such as `2.5` or `5 W/m2`."),
        ));
        return None;
    };
    if !value.is_finite() {
        diagnostics.push(Diagnostic::error(
            "E-SAMPLING-RANGE-UNIT",
            line,
            &format!("Sample endpoint `{}` is not finite.", expression.trim()),
            Some("Use finite numeric endpoints."),
        ));
        return None;
    }
    let Some(unit) = unit else {
        return Some(SampleDistributionEndpoint {
            value,
            quantity_kind: "DimensionlessNumber".to_owned(),
            display_unit: "1".to_owned(),
            canonical_unit: "1".to_owned(),
        });
    };
    let Some(quantity) = candidates_for_unit(&unit).first().copied() else {
        diagnostics.push(Diagnostic::error(
            "E-SAMPLING-RANGE-UNIT",
            line,
            &format!("Sample endpoint unit `{unit}` is unknown."),
            Some("Use a known EngLang unit or make the range dimensionless."),
        ));
        return None;
    };
    Some(SampleDistributionEndpoint {
        value,
        quantity_kind: quantity.quantity_kind.to_owned(),
        display_unit: unit,
        canonical_unit: quantity.canonical_unit.to_owned(),
    })
}

fn normalize_sample_unit(unit: &str) -> String {
    if unit == "1" {
        "1".to_owned()
    } else {
        normalize_unit(unit)
    }
}

pub fn validate_simulation_contracts(
    program: &SemanticProgram,
    inferred_declarations: &[InferredDeclaration],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for declaration in inferred_declarations {
        let Some(system_name) = declaration
            .expression
            .trim()
            .strip_prefix("simulate ")
            .map(str::trim)
        else {
            continue;
        };
        let Some(system) = program
            .systems
            .iter()
            .find(|system| system.name == system_name)
        else {
            diagnostics.push(Diagnostic::error(
                "E-SIM-SYSTEM-001",
                declaration.line,
                &format!("Simulation references unknown system `{system_name}`."),
                Some("Define the system before the `simulate` binding."),
            ));
            continue;
        };
        let options = program
            .with_blocks
            .iter()
            .find(|block| block.owner_line == Some(declaration.line))
            .map(|block| block.options.as_slice())
            .unwrap_or(&[]);

        validate_simulation_timestep(declaration.line, options, &mut diagnostics);
        validate_simulation_duration(options, &mut diagnostics);
        validate_simulation_tolerance(options, &mut diagnostics);
        validate_simulation_solver(declaration.line, program, system, options, &mut diagnostics);
        validate_simulation_parameter_options(system, options, &mut diagnostics);
        validate_simulation_scalar_input_options(system, options, &mut diagnostics);

        for variable in &system.variables {
            let Some(expected) = expected_dynamic_input(variable) else {
                continue;
            };
            let Some(option) = accepted_option(options, &variable.name) else {
                diagnostics.push(Diagnostic::error(
                    "E-SIM-MISSING-INPUT",
                    declaration.line,
                    &format!(
                        "Simulation of `{system_name}` requires TimeSeries input `{}`.",
                        variable.name
                    ),
                    Some(&format!(
                        "Add `{} = <TimeSeries[{}] of {}>` in the attached `with` block.",
                        variable.name, expected.axis, expected.quantity_kind
                    )),
                ));
                continue;
            };
            let Some(actual) = resolve_simulation_option_type(program, &option.value) else {
                diagnostics.push(Diagnostic::error(
                    "E-SIM-MISSING-INPUT",
                    option.line,
                    &format!(
                        "Simulation input `{}` cannot resolve `{}` as a typed value.",
                        variable.name, option.value
                    ),
                    Some("Bind the option to a prior TimeSeries value or a promoted CSV column."),
                ));
                continue;
            };
            let Some(actual_axis) = actual.axis.as_deref() else {
                diagnostics.push(Diagnostic::error(
                    "E-SIM-INPUT-AXIS-MISMATCH",
                    option.line,
                    &format!(
                        "Simulation input `{}` expects TimeSeries[{}] of {}, but `{}` is {}.",
                        variable.name,
                        expected.axis,
                        expected.quantity_kind,
                        option.value,
                        actual.quantity_kind
                    ),
                    Some("Use a promoted CSV column such as `weather_data.T_out`."),
                ));
                continue;
            };
            if actual_axis != expected.axis {
                diagnostics.push(Diagnostic::error(
                    "E-SIM-INPUT-AXIS-MISMATCH",
                    option.line,
                    &format!(
                        "Simulation input `{}` expects axis `{}`, but `{}` has axis `{actual_axis}`.",
                        variable.name, expected.axis, option.value
                    ),
                    Some("Use a DateTime-indexed TimeSeries for dynamic system inputs."),
                ));
            }
            if actual.quantity_kind != expected.quantity_kind {
                diagnostics.push(Diagnostic::error(
                    "E-SIM-INPUT-QTY-MISMATCH",
                    option.line,
                    &format!(
                        "Simulation input `{}` expects {}, but `{}` is {}.",
                        variable.name, expected.quantity_kind, option.value, actual.quantity_kind
                    ),
                    Some("Bind the option to a TimeSeries with the same quantity kind as the system input."),
                ));
            }
        }
    }
    diagnostics
}

fn validate_algebraic_solve_contracts(
    assemblies: &[ComponentAssemblyInfo],
    systems: &[SystemInfo],
    inferred_declarations: &[InferredDeclaration],
    with_blocks: &[WithBlockInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for declaration in inferred_declarations {
        let Some(target_name) = declaration
            .expression
            .trim()
            .strip_prefix("solve ")
            .map(str::trim)
        else {
            continue;
        };
        let target_is_component = assemblies
            .iter()
            .any(|assembly| assembly.name == target_name);
        let target_system = systems.iter().find(|system| system.name == target_name);
        if !target_is_component && target_system.is_none() {
            diagnostics.push(Diagnostic::error(
                "E-SOLVE-ASSEMBLY-001",
                declaration.line,
                &format!("Algebraic solve references unknown assembly or source system `{target_name}`."),
                Some("Use `solve component_graph` for the current component assembly artifact, or `solve <SystemName>` for a supported source system."),
            ));
            continue;
        }
        let options = with_blocks
            .iter()
            .find(|block| block.owner_line == Some(declaration.line))
            .map(|block| block.options.as_slice())
            .unwrap_or(&[]);
        let Some(solver_option) = accepted_option(options, "solver") else {
            diagnostics.push(Diagnostic::error(
                "E-SOLVE-SOLVER-UNSUPPORTED",
                declaration.line,
                "`solve` requires a supported solver in the attached `with` block.",
                Some(
                    "Use `solver = dense_linear`/`linear` for linear source systems, `fixed_point` for direct fixed-point source systems, `newton` for nonlinear source systems, or one of the supported component solve solvers for `solve component_graph`.",
                ),
            ));
            continue;
        };
        let solver = solver_option.value.trim();
        if let Some(system) = target_system {
            if !is_supported_system_solve_solver(solver) {
                diagnostics.push(Diagnostic::error(
                    "E-SOLVE-SOLVER-UNSUPPORTED",
                    solver_option.line,
                    &format!(
                        "Unsupported source system solve solver `{}`.",
                        solver_option.value
                    ),
                    Some("Use `dense_linear`/`linear` for linear source system solves, `fixed_point` for direct fixed-point source system solves, or `newton`/`nonlinear_newton` for nonlinear source system solves."),
                ));
                continue;
            }
            if system
                .equation_ir
                .iter()
                .any(|equation| !equation.derivative_states.is_empty())
            {
                diagnostics.push(Diagnostic::error(
                    "E-SOLVE-SYSTEM-SHAPE-UNSUPPORTED",
                    declaration.line,
                    &format!("Source system `{}` has derivative equations, so it is not a static algebraic solve target.", system.name),
                    Some("Use `simulate <SystemName>` for supported ODE shapes, or remove derivative equations before using `solve <SystemName>`."),
                ));
                continue;
            }
        } else if !is_supported_component_solve_solver(solver) {
            diagnostics.push(Diagnostic::error(
                "E-SOLVE-SOLVER-UNSUPPORTED",
                solver_option.line,
                &format!("Unsupported component solve solver `{}`.", solver_option.value),
                Some(
                    "Use `dense_linear`/`linear`, `fixed_point`, `newton`, `implicit_euler_dae`, `dynamic_component_explicit_euler`, `dynamic_component_semi_implicit_euler`, or `dynamic_component_adaptive_heun`.",
                ),
            ));
            continue;
        }
        let dynamic_component_solver =
            target_is_component && is_dynamic_component_solve_solver(solver);
        let dae_solver = target_is_component && is_dae_component_solve_solver(solver);
        if dynamic_component_solver || dae_solver {
            validate_component_solve_duration_options(declaration.line, options, diagnostics);
            validate_component_solve_initial_option(options, diagnostics);
        }
        validate_algebraic_solve_numeric_option(
            options,
            "tolerance",
            |value| value.is_finite() && value > 0.0,
            "E-SOLVE-TOLERANCE-INVALID",
            "`tolerance` expects a positive finite number.",
            diagnostics,
        );
        validate_algebraic_solve_numeric_option(
            options,
            "relaxation",
            |value| value.is_finite() && value > 0.0 && value <= 1.0,
            "E-SOLVE-RELAXATION-INVALID",
            "`relaxation` expects a finite number in the interval (0, 1].",
            diagnostics,
        );
        if !dynamic_component_solver && !dae_solver {
            validate_component_solve_initial_option(options, diagnostics);
        }
        validate_component_solve_initial_list_option(
            options,
            "initial_algebraic",
            "`initial_algebraic` expects a finite numeric initial guess or bracketed list.",
            diagnostics,
        );
        validate_component_solve_initial_list_option(
            options,
            "initial_derivative",
            "`initial_derivative` expects a finite numeric initial derivative or bracketed list.",
            diagnostics,
        );
        validate_algebraic_solve_numeric_option(
            options,
            "finite_difference_step",
            |value| value.is_finite() && value > 0.0,
            "E-SOLVE-FD-STEP-INVALID",
            "`finite_difference_step` expects a positive finite number.",
            diagnostics,
        );
        validate_algebraic_solve_numeric_option(
            options,
            "damping",
            |value| value.is_finite() && value > 0.0 && value <= 1.0,
            "E-SOLVE-DAMPING-INVALID",
            "`damping` expects a finite number in the interval (0, 1].",
            diagnostics,
        );
        validate_algebraic_solve_numeric_option(
            options,
            "consistency_tolerance",
            |value| value.is_finite() && value > 0.0,
            "E-SOLVE-CONSISTENCY-TOLERANCE-INVALID",
            "`consistency_tolerance` expects a positive finite number.",
            diagnostics,
        );
        if let Some(option) = accepted_option(options, "max_iter") {
            let valid = option
                .value
                .trim()
                .parse::<usize>()
                .is_ok_and(|value| value > 0);
            if !valid {
                diagnostics.push(Diagnostic::error(
                    "E-SOLVE-MAX-ITER-INVALID",
                    option.line,
                    &format!(
                        "`max_iter` expects a positive integer, got `{}`.",
                        option.value
                    ),
                    Some("Use a positive integer such as `max_iter = 50`."),
                ));
            }
        }
        if let Some(option) = accepted_option(options, "line_search_steps") {
            let valid = option
                .value
                .trim()
                .parse::<usize>()
                .is_ok_and(|value| value > 0);
            if !valid {
                diagnostics.push(Diagnostic::error(
                    "E-SOLVE-LINE-SEARCH-STEPS-INVALID",
                    option.line,
                    &format!(
                        "`line_search_steps` expects a positive integer, got `{}`.",
                        option.value
                    ),
                    Some("Use a positive integer such as `line_search_steps = 8`."),
                ));
            }
        }
        validate_component_solve_positive_numeric_list_option(
            options,
            "variable_scale",
            "`variable_scale` expects a positive finite numeric scale or bracketed list with optional units.",
            diagnostics,
        );
        validate_component_solve_positive_numeric_list_option(
            options,
            "variable_scales",
            "`variable_scales` expects a positive finite numeric scale or bracketed list with optional units.",
            diagnostics,
        );
        if let Some(option) = accepted_option(options, "jacobian") {
            let valid = matches!(
                option.value.trim(),
                "finite_difference" | "source_linear_terms"
            );
            if !valid {
                diagnostics.push(Diagnostic::error(
                    "E-SOLVE-JACOBIAN-UNSUPPORTED",
                    option.line,
                    &format!(
                        "Unsupported source solve Jacobian policy `{}`.",
                        option.value
                    ),
                    Some("Use `jacobian = finite_difference` or `jacobian = source_linear_terms`."),
                ));
            }
        }
        if dae_solver {
            validate_component_solve_mass_matrix_option(options, diagnostics);
        }
        if let Some(option) = accepted_option(options, "algebraic_initialization") {
            let valid = matches!(option.value.trim(), "newton" | "none");
            if !valid {
                diagnostics.push(Diagnostic::error(
                    "E-SOLVE-ALGEBRAIC-INITIALIZATION-UNSUPPORTED",
                    option.line,
                    &format!(
                        "Unsupported algebraic initialization policy `{}`.",
                        option.value
                    ),
                    Some("Use `algebraic_initialization = newton` or `algebraic_initialization = none`."),
                ));
            }
        }
    }
}

fn is_supported_component_solve_solver(solver: &str) -> bool {
    matches!(
        solver,
        "fixed_point"
            | "dense_linear"
            | "linear"
            | "newton"
            | "nonlinear_newton"
            | "implicit_euler_dae"
            | "dae_implicit_euler"
            | "dynamic_component_explicit_euler"
            | "dynamic_component_semi_implicit_euler"
            | "dynamic_component_adaptive_heun"
    )
}

fn is_supported_system_solve_solver(solver: &str) -> bool {
    matches!(
        solver,
        "dense_linear" | "linear" | "fixed_point" | "newton" | "nonlinear_newton"
    )
}

fn is_dynamic_component_solve_solver(solver: &str) -> bool {
    matches!(
        solver,
        "dynamic_component_explicit_euler"
            | "dynamic_component_semi_implicit_euler"
            | "dynamic_component_adaptive_heun"
    )
}

fn is_dae_component_solve_solver(solver: &str) -> bool {
    matches!(solver, "implicit_euler_dae" | "dae_implicit_euler")
}

fn validate_component_solve_duration_options(
    owner_line: usize,
    options: &[WithOptionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(timestep) = accepted_option(options, "timestep") else {
        diagnostics.push(Diagnostic::error(
            "E-SOLVE-TIMESTEP-INVALID",
            owner_line,
            "Dynamic component `solve` requires `with { timestep = <duration> }`.",
            Some("Use a positive duration such as `timestep = 1 s`."),
        ));
        return;
    };
    if parse_duration_option_seconds(&timestep.value).is_none() {
        diagnostics.push(Diagnostic::error(
            "E-SOLVE-TIMESTEP-INVALID",
            timestep.line,
            &format!(
                "`timestep` expects a positive duration, got `{}`.",
                timestep.value
            ),
            Some("Use units such as `s`, `min`, or `h`, for example `1 s`."),
        ));
    }
    let Some(duration) = accepted_option(options, "duration") else {
        diagnostics.push(Diagnostic::error(
            "E-SOLVE-DURATION-INVALID",
            owner_line,
            "Dynamic component `solve` requires `with { duration = <duration> }`.",
            Some("Use a positive duration such as `duration = 10 s`."),
        ));
        return;
    };
    if parse_duration_option_seconds(&duration.value).is_none() {
        diagnostics.push(Diagnostic::error(
            "E-SOLVE-DURATION-INVALID",
            duration.line,
            &format!(
                "`duration` expects a positive duration, got `{}`.",
                duration.value
            ),
            Some("Use units such as `s`, `min`, or `h`, for example `10 s`."),
        ));
    }
}

fn validate_component_solve_initial_option(
    options: &[WithOptionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(option) = accepted_option(options, "initial") else {
        return;
    };
    if initial_literal_values(&option.value).is_none() {
        diagnostics.push(Diagnostic::error(
            "E-SOLVE-INITIAL-INVALID",
            option.line,
            &format!(
                "`initial` expects a finite numeric literal or bracketed list with optional units, got `{}`.",
                option.value
            ),
            Some("Use a literal such as `initial = 20 degC`, `initial = 1`, or `initial = [1, 3]`."),
        ));
    }
}

fn validate_component_solve_initial_list_option(
    options: &[WithOptionInfo],
    key: &str,
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(option) = accepted_option(options, key) else {
        return;
    };
    if initial_literal_values(&option.value).is_none() {
        diagnostics.push(Diagnostic::error(
            "E-SOLVE-INITIAL-INVALID",
            option.line,
            &format!("{message} Got `{}`.", option.value),
            Some("Use a literal such as `1`, `20 degC`, or a bracketed list such as `[1, 3]`."),
        ));
    }
}
fn validate_component_solve_positive_numeric_list_option(
    options: &[WithOptionInfo],
    key: &str,
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(option) = accepted_option(options, key) else {
        return;
    };
    let valid = initial_literal_values(&option.value).is_some_and(|values| {
        values
            .iter()
            .all(|(value, _unit)| value.is_finite() && *value > 0.0)
    });
    if !valid {
        diagnostics.push(Diagnostic::error(
            "E-SOLVE-VARIABLE-SCALE-INVALID",
            option.line,
            &format!("{message} Got `{}`.", option.value),
            Some("Use a positive literal such as `1`, `20 degC`, or a bracketed list such as `[1, 3]`."),
        ));
    }
}
fn validate_component_solve_mass_matrix_option(
    options: &[WithOptionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(option) = accepted_option(options, "mass_matrix") else {
        return;
    };
    if !mass_matrix_literal_values(&option.value) {
        diagnostics.push(Diagnostic::error(
            "E-SOLVE-MASS-MATRIX-INVALID",
            option.line,
            &format!(
                "`mass_matrix` expects `identity`, a finite scalar, a finite vector of diagonal coefficients, or a finite square matrix with no units. Got `{}`.",
                option.value
            ),
            Some("Use `mass_matrix = identity`, `mass_matrix = 1`, `mass_matrix = [1, 1]`, or `mass_matrix = [[1, 0], [0, 1]]`."),
        ));
    }
}

fn mass_matrix_literal_values(expression: &str) -> bool {
    let trimmed = expression.trim();
    if trimmed.eq_ignore_ascii_case("identity") {
        return true;
    }
    if unitless_numeric_literal(trimmed).is_some() {
        return true;
    }
    if mass_matrix_row_literal_values(trimmed).is_some() {
        return true;
    }
    let Some(inner) = trimmed
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    else {
        return false;
    };
    let items = split_vector_literal_items(inner);
    !items.is_empty()
        && items
            .iter()
            .all(|item| unitless_numeric_literal(item).is_some())
}

fn mass_matrix_row_literal_values(expression: &str) -> Option<Vec<Vec<f64>>> {
    let inner = expression.strip_prefix('[')?.strip_suffix(']')?;
    let rows = split_vector_literal_items(inner);
    if rows.is_empty() {
        return None;
    }
    rows.iter()
        .map(|row| {
            let row_inner = row.trim().strip_prefix('[')?.strip_suffix(']')?;
            let entries = split_vector_literal_items(row_inner);
            if entries.is_empty() {
                return None;
            }
            entries
                .iter()
                .map(|entry| unitless_numeric_literal(entry))
                .collect::<Option<Vec<_>>>()
        })
        .collect::<Option<Vec<_>>>()
}

fn unitless_numeric_literal(expression: &str) -> Option<f64> {
    let (value, unit) = initial_numeric_literal_with_optional_unit(expression)?;
    if unit.is_some() || !value.is_finite() {
        return None;
    }
    Some(value)
}

fn initial_numeric_literal_with_optional_unit(expression: &str) -> Option<(f64, Option<String>)> {
    let mut parts = expression.split_whitespace();
    let value = parts.next()?.parse::<f64>().ok()?;
    let unit = parts.next().map(str::to_owned);
    if parts.next().is_some() {
        return None;
    }
    Some((value, unit))
}
fn initial_literal_values(expression: &str) -> Option<Vec<(f64, Option<String>)>> {
    let trimmed = expression.trim();
    if trimmed.is_empty() {
        return None;
    }
    let values = if let Some(inner) = trimmed
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    {
        let items = split_vector_literal_items(inner);
        if items.is_empty() {
            return None;
        }
        items
            .iter()
            .map(|item| initial_numeric_literal_with_optional_unit(item))
            .collect::<Option<Vec<_>>>()?
    } else {
        vec![initial_numeric_literal_with_optional_unit(trimmed)?]
    };
    if values.iter().all(|(value, _unit)| value.is_finite()) {
        Some(values)
    } else {
        None
    }
}

fn validate_algebraic_solve_numeric_option(
    options: &[WithOptionInfo],
    key: &str,
    predicate: impl Fn(f64) -> bool,
    code: &str,
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(option) = accepted_option(options, key) else {
        return;
    };
    let valid = option.value.trim().parse::<f64>().is_ok_and(predicate);
    if !valid {
        diagnostics.push(Diagnostic::error(
            code,
            option.line,
            &format!("{message} Got `{}`.", option.value),
            Some("Use a plain dimensionless numeric option value."),
        ));
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExpectedSimulationInput {
    axis: String,
    quantity_kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SimulationOptionType {
    axis: Option<String>,
    quantity_kind: String,
}

fn expected_dynamic_input(variable: &SystemVariableInfo) -> Option<ExpectedSimulationInput> {
    if variable.role != "input" {
        return None;
    }
    if let Some((axis, quantity_kind)) = crate::stats::time_series_quantity(&variable.quantity_kind)
    {
        return Some(ExpectedSimulationInput {
            axis,
            quantity_kind,
        });
    }
    if matches!(
        variable.quantity_kind.as_str(),
        "AbsoluteTemperature" | "Irradiance"
    ) {
        return Some(ExpectedSimulationInput {
            axis: "Time".to_owned(),
            quantity_kind: variable.quantity_kind.clone(),
        });
    }
    None
}

fn validate_simulation_scalar_input_options(
    system: &SystemInfo,
    options: &[WithOptionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for input in system
        .variables
        .iter()
        .filter(|variable| variable.role == "input")
        .filter(|variable| expected_dynamic_input(variable).is_none())
    {
        let Some(option) = accepted_option(options, &input.name) else {
            continue;
        };
        let Some((value, unit)) = numeric_literal_with_optional_unit(&option.value) else {
            diagnostics.push(Diagnostic::error(
                "E-SIM-INPUT-VALUE",
                option.line,
                &format!(
                    "Simulation input `{}` expects a numeric literal, got `{}`.",
                    input.name, option.value
                ),
                Some("Use a finite numeric literal with an optional compatible unit."),
            ));
            continue;
        };
        if !value.is_finite() {
            diagnostics.push(Diagnostic::error(
                "E-SIM-INPUT-VALUE",
                option.line,
                &format!(
                    "Simulation input `{}` expects a finite numeric literal, got `{}`.",
                    input.name, option.value
                ),
                Some("Use a finite numeric literal with an optional compatible unit."),
            ));
            continue;
        }
        let Some(unit) = unit else {
            continue;
        };
        let Some(actual_quantity) = candidates_for_unit(&unit)
            .first()
            .map(|completion| completion.quantity_kind.to_owned())
        else {
            diagnostics.push(Diagnostic::error(
                "E-SIM-INPUT-QTY-MISMATCH",
                option.line,
                &format!(
                    "Simulation input `{}` uses unsupported unit `{unit}`.",
                    input.name
                ),
                Some("Use a unit from the built-in unit registry."),
            ));
            continue;
        };
        let expected_quantity = scalar_quantity_kind(&input.quantity_kind);
        let expected_dimension = dimension_for_quantity(&expected_quantity);
        let actual_dimension = dimension_for_quantity(&actual_quantity);
        if !dimensions_compatible(&expected_dimension, &actual_dimension) {
            diagnostics.push(Diagnostic::error(
                "E-SIM-INPUT-QTY-MISMATCH",
                option.line,
                &format!(
                    "Simulation input `{}` expects {}, but `{}` is {}.",
                    input.name, expected_quantity, option.value, actual_quantity
                ),
                Some("Use a numeric literal with a unit compatible with the declared input."),
            ));
        }
    }
}
fn validate_simulation_parameter_options(
    system: &SystemInfo,
    options: &[WithOptionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for parameter in system
        .variables
        .iter()
        .filter(|variable| variable.role == "parameter")
    {
        let Some(option) = accepted_option(options, &parameter.name) else {
            continue;
        };
        let Some((value, unit)) = numeric_literal_with_optional_unit(&option.value) else {
            diagnostics.push(Diagnostic::error(
                "E-SIM-PARAMETER-INVALID",
                option.line,
                &format!(
                    "Simulation parameter `{}` expects a numeric literal, got `{}`.",
                    parameter.name, option.value
                ),
                Some("Use a finite numeric literal with an optional compatible unit."),
            ));
            continue;
        };
        if !value.is_finite() {
            diagnostics.push(Diagnostic::error(
                "E-SIM-PARAMETER-INVALID",
                option.line,
                &format!(
                    "Simulation parameter `{}` expects a finite numeric literal, got `{}`.",
                    parameter.name, option.value
                ),
                Some("Use a finite numeric literal with an optional compatible unit."),
            ));
            continue;
        }
        let Some(unit) = unit else {
            continue;
        };
        let Some(actual_quantity) = candidates_for_unit(&unit)
            .first()
            .map(|completion| completion.quantity_kind.to_owned())
        else {
            diagnostics.push(Diagnostic::error(
                "E-SIM-PARAMETER-UNIT",
                option.line,
                &format!(
                    "Simulation parameter `{}` uses unsupported unit `{unit}`.",
                    parameter.name
                ),
                Some("Use a unit from the built-in unit registry."),
            ));
            continue;
        };
        let expected_dimension = dimension_for_quantity(&parameter.quantity_kind);
        let actual_dimension = dimension_for_quantity(&actual_quantity);
        if !dimensions_compatible(&expected_dimension, &actual_dimension) {
            diagnostics.push(Diagnostic::error(
                "E-SIM-PARAMETER-QTY-MISMATCH",
                option.line,
                &format!(
                    "Simulation parameter `{}` expects {}, but `{}` is {}.",
                    parameter.name, parameter.quantity_kind, option.value, actual_quantity
                ),
                Some("Use a numeric literal with a unit compatible with the declared parameter."),
            ));
        }
    }
}
fn validate_simulation_timestep(
    owner_line: usize,
    options: &[WithOptionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(option) = accepted_option(options, "timestep") else {
        diagnostics.push(Diagnostic::error(
            "E-SIM-TIMESTEP-INVALID",
            owner_line,
            "`simulate` requires `with { timestep = <duration> }`.",
            Some("Use a duration such as `timestep = 10 min`."),
        ));
        return;
    };
    if parse_duration_option_seconds(&option.value).is_none() {
        diagnostics.push(Diagnostic::error(
            "E-SIM-TIMESTEP-INVALID",
            option.line,
            &format!(
                "`timestep` expects a positive duration, got `{}`.",
                option.value
            ),
            Some("Use units such as `s`, `min`, or `h`, for example `10 min`."),
        ));
    }
}

fn validate_simulation_duration(options: &[WithOptionInfo], diagnostics: &mut Vec<Diagnostic>) {
    let Some(option) = accepted_option(options, "duration") else {
        return;
    };
    if parse_duration_option_seconds(&option.value).is_none() {
        diagnostics.push(Diagnostic::error(
            "E-SIM-DURATION-INVALID",
            option.line,
            &format!(
                "`duration` expects a positive duration, got `{}`.",
                option.value
            ),
            Some("Use units such as `s`, `min`, or `h`, for example `30 min`."),
        ));
    }
}

fn validate_simulation_tolerance(options: &[WithOptionInfo], diagnostics: &mut Vec<Diagnostic>) {
    let Some(option) = accepted_option(options, "tolerance") else {
        return;
    };
    let valid_tolerance = match option.value.trim().parse::<f64>() {
        Ok(value) => value.is_finite() && value > 0.0,
        Err(_) => false,
    };
    if !valid_tolerance {
        diagnostics.push(Diagnostic::error(
            "E-SIM-TOLERANCE-INVALID",
            option.line,
            &format!(
                "`tolerance` expects a positive finite number, got `{}`.",
                option.value
            ),
            Some("Use a dimensionless numeric tolerance such as `0.0001`."),
        ));
    }
}

fn validate_simulation_solver(
    owner_line: usize,
    program: &SemanticProgram,
    system: &SystemInfo,
    options: &[WithOptionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(option) = accepted_option(options, "solver") else {
        diagnostics.push(Diagnostic::error(
            "E-SIM-SOLVER-UNSUPPORTED",
            owner_line,
            "`simulate` requires a supported solver in the attached `with` block.",
            Some("Use `solver = fixed_step`, `solver = explicit_euler`, `solver = rk4`, or `solver = adaptive_heun`."),
        ));
        return;
    };
    let solver_name = option.value.trim();
    if !matches!(
        solver_name,
        "fixed_step" | "explicit_euler" | "rk4" | "adaptive_heun"
    ) {
        diagnostics.push(Diagnostic::error(
            "E-SIM-SOLVER-UNSUPPORTED",
            option.line,
            &format!("Unsupported simulation solver `{}`.", option.value),
            Some("Use fixed-step Euler/RK4 or `adaptive_heun` for the one-state thermal workflow; general adaptive and nonlinear solvers are deferred."),
        ));
    } else if solver_name == "adaptive_heun" && !supports_adaptive_heun_simulation(program, system)
    {
        diagnostics.push(Diagnostic::error(
            "E-SIM-SYSTEM-SHAPE-UNSUPPORTED",
            option.line,
            "`adaptive_heun` requires source derivative equations or a continuous state-space workflow.",
            Some("Use one `der(state)` equation per state, or use a continuous state-space system with shape-checked A/B operators."),
        ));
    }
}

fn supports_adaptive_heun_simulation(program: &SemanticProgram, system: &SystemInfo) -> bool {
    supports_one_state_adaptive_heun(system)
        || supports_continuous_state_space_adaptive_heun(program, system)
        || supports_source_ode_adaptive_heun(system)
}

fn supports_source_ode_adaptive_heun(system: &SystemInfo) -> bool {
    let states = system
        .variables
        .iter()
        .filter(|variable| variable.role == "state")
        .collect::<Vec<_>>();
    if states.is_empty()
        || states
            .iter()
            .any(|state| unsupported_state_quantity(&state.quantity_kind))
    {
        return false;
    }
    if system
        .equations
        .iter()
        .any(|equation| equation.left.trim().starts_with("next("))
    {
        return false;
    }
    states.iter().all(|state| {
        let derivative = format!("der({})", state.name);
        system
            .equations
            .iter()
            .filter(|equation| equation.left.contains(&derivative))
            .count()
            == 1
    })
}

fn supports_one_state_adaptive_heun(system: &SystemInfo) -> bool {
    let states = system
        .variables
        .iter()
        .filter(|variable| variable.role == "state")
        .collect::<Vec<_>>();
    if states.len() != 1 || simulation_value_quantity(states[0]) != "AbsoluteTemperature" {
        return false;
    }
    let has_heat_capacity = system.variables.iter().any(|variable| {
        variable.role == "parameter" && simulation_value_quantity(variable) == "HeatCapacity"
    });
    let has_conductance = system.variables.iter().any(|variable| {
        variable.role == "parameter" && simulation_value_quantity(variable) == "Conductance"
    });
    let has_outdoor_temperature = system.variables.iter().any(|variable| {
        variable.role == "input" && simulation_value_quantity(variable) == "AbsoluteTemperature"
    });
    let has_internal_heat = system.variables.iter().any(|variable| {
        variable.role == "input" && simulation_value_quantity(variable) == "HeatRate"
    });
    let derivative = format!("der({})", states[0].name);
    let has_derivative_equation = system
        .equations
        .iter()
        .any(|equation| equation.left.contains(&derivative));

    has_heat_capacity
        && has_conductance
        && has_outdoor_temperature
        && has_internal_heat
        && has_derivative_equation
}

fn supports_continuous_state_space_adaptive_heun(
    program: &SemanticProgram,
    system: &SystemInfo,
) -> bool {
    let has_continuous_state_space_equation = system
        .equations
        .iter()
        .any(|equation| equation.left.trim().starts_with("der("));
    let has_discrete_state_space_equation = system
        .equations
        .iter()
        .any(|equation| equation.left.trim().starts_with("next("));
    if !has_continuous_state_space_equation || has_discrete_state_space_equation {
        return false;
    }
    let has_state_vector = program.state_space_vectors.iter().any(|vector| {
        vector.system == system.name && vector.role == "states" && !vector.members.is_empty()
    });
    let has_input_vector = program.state_space_vectors.iter().any(|vector| {
        vector.system == system.name && vector.role == "inputs" && !vector.members.is_empty()
    });
    let has_state_operator = program.linear_operators.iter().any(|operator| {
        operator.system == system.name
            && operator.from == "StateVector"
            && operator.to == "Derivative[StateVector]"
            && operator.status == "shape_checked"
    });
    let has_input_operator = program.linear_operators.iter().any(|operator| {
        operator.system == system.name
            && operator.from == "InputVector"
            && operator.to == "Derivative[StateVector]"
            && operator.status == "shape_checked"
    });

    has_state_vector && has_input_vector && has_state_operator && has_input_operator
}

fn simulation_value_quantity(variable: &SystemVariableInfo) -> String {
    crate::stats::time_series_quantity(&variable.quantity_kind)
        .map(|(_, quantity_kind)| quantity_kind)
        .unwrap_or_else(|| variable.quantity_kind.clone())
}

fn accepted_option<'a>(options: &'a [WithOptionInfo], key: &str) -> Option<&'a WithOptionInfo> {
    options
        .iter()
        .find(|option| option.key == key && option.status == "accepted")
}

fn resolve_simulation_option_type(
    program: &SemanticProgram,
    expression: &str,
) -> Option<SimulationOptionType> {
    let expression = expression.trim();
    if let Some(binding) = program
        .typed_bindings
        .iter()
        .find(|binding| binding.name == expression)
    {
        return Some(simulation_option_type_from_semantic(&binding.semantic_type));
    }
    if let Some((table_binding, column_name)) = expression.split_once('.') {
        let promotion = program
            .csv_promotions
            .iter()
            .find(|promotion| promotion.binding == table_binding.trim())?;
        let schema = program
            .schemas
            .iter()
            .find(|schema| schema.name == promotion.schema_name)?;
        let column = schema
            .columns
            .iter()
            .find(|column| column.name == column_name.trim())?;
        if column.is_index {
            return Some(SimulationOptionType {
                axis: None,
                quantity_kind: column.type_name.clone(),
            });
        }
        return Some(SimulationOptionType {
            axis: Some(schema_time_axis(schema)),
            quantity_kind: column.type_name.clone(),
        });
    }
    numeric_literal_with_optional_unit(expression).map(|(_, unit)| {
        let quantity_kind = unit
            .as_deref()
            .and_then(|unit| candidates_for_unit(unit).first().copied())
            .map(|completion| completion.quantity_kind.to_owned())
            .unwrap_or_else(|| "DimensionlessNumber".to_owned());
        SimulationOptionType {
            axis: None,
            quantity_kind,
        }
    })
}

fn simulation_option_type_from_semantic(semantic_type: &SemanticType) -> SimulationOptionType {
    if let Some((axis, quantity_kind)) =
        crate::stats::time_series_quantity(&semantic_type.quantity_kind)
    {
        return SimulationOptionType {
            axis: Some(axis),
            quantity_kind,
        };
    }
    SimulationOptionType {
        axis: None,
        quantity_kind: semantic_type.quantity_kind.clone(),
    }
}

fn schema_time_axis(schema: &SchemaInfo) -> String {
    schema
        .columns
        .iter()
        .find(|column| column.is_index)
        .map(|column| {
            if column.type_name == "DateTime" {
                "Time".to_owned()
            } else {
                column.type_name.clone()
            }
        })
        .unwrap_or_else(|| "Sample".to_owned())
}

fn parse_duration_option_seconds(value: &str) -> Option<f64> {
    let (amount, unit) = numeric_literal_with_optional_unit(value)?;
    if amount <= 0.0 {
        return None;
    }
    let unit = unit?;
    let seconds = match unit.trim().to_ascii_lowercase().as_str() {
        "s" | "sec" | "second" | "seconds" => amount,
        "min" | "minute" | "minutes" => amount * 60.0,
        "h" | "hr" | "hour" | "hours" => amount * 3600.0,
        _ => return None,
    };
    Some(seconds)
}

fn with_option_bool(with_blocks: &[WithBlockInfo], owner_line: usize, key: &str) -> bool {
    with_blocks.iter().any(|block| {
        block.owner_line == Some(owner_line)
            && block.options.iter().any(|option| {
                option.key == key
                    && option.status == "accepted"
                    && option.value.trim().eq_ignore_ascii_case("true")
            })
    })
}

fn validate_write_options(
    writes: &[WriteInfo],
    with_blocks: &[WithBlockInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for write in writes {
        if write.format != "standard_text" || !write.path.trim().is_empty() {
            continue;
        }
        let has_output = with_blocks.iter().any(|block| {
            block.owner_line == Some(write.line)
                && block
                    .options
                    .iter()
                    .any(|option| option.key == "output" && option.status == "accepted")
        });
        if !has_output {
            diagnostics.push(Diagnostic::error(
                "E-WRITE-STANDARD-TEXT-OUTPUT",
                write.line,
                "`write standard_text` needs an output path.",
                Some(
                    "Add `with { output = join(args.output, \"standard_weather_file.txt\") }` or write `write standard_text <table> to \"outputs/file.txt\"`.",
                ),
            ));
        }
    }
}

fn resolve_write_expression_type(
    expression: &str,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
) -> Option<SemanticType> {
    let expression = expression.trim();
    if expression.starts_with('"') {
        return semantic_type("String", "");
    }
    resolve_format_expression_type(expression, typed_bindings, functions)
}

fn assert_expression_semantic_type(
    expression: &str,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
) -> Option<SemanticType> {
    let expression = expression.trim();
    if matches!(expression, "true" | "false") {
        return semantic_type("Bool", "");
    }
    if expression.starts_with('"') {
        return semantic_type("String", "");
    }
    if let Some((_value, unit)) = numeric_literal_with_optional_unit(expression) {
        if let Some(unit) = unit {
            let quantity = candidates_for_unit(&unit).first().copied()?;
            return semantic_type(quantity.quantity_kind, quantity.canonical_unit);
        }
        return semantic_type("DimensionlessNumber", "");
    }
    resolve_format_expression_type(expression, typed_bindings, functions)
}

fn numeric_literal_with_optional_unit(expression: &str) -> Option<(f64, Option<String>)> {
    let mut parts = expression.split_whitespace();
    let value_text = parts.next()?;
    if !is_number_literal(value_text) {
        return None;
    }
    let value = value_text.parse::<f64>().ok()?;
    let unit = parts.next().map(str::to_owned);
    if parts.next().is_some() {
        return None;
    }
    Some((value, unit))
}

fn analyze_format_fields(
    template: &str,
    line: usize,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
    context: FormatDiagnosticContext,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<FormatExpressionInfo> {
    let mut fields = Vec::new();
    let mut cursor = 0usize;
    while let Some(open) = template[cursor..].find('{') {
        let start = cursor + open;
        let Some(close_offset) = template[start + 1..].find('}') else {
            diagnostics.push(Diagnostic::error(
                context.unterminated_code,
                line,
                &format!(
                    "{} has an unterminated `{{...}}` interpolation.",
                    context.template_label
                ),
                Some("Close the interpolation with `}`."),
            ));
            break;
        };
        let close = start + 1 + close_offset;
        let inside = template[start + 1..close].trim();
        if inside.is_empty() {
            diagnostics.push(Diagnostic::error(
                context.empty_code,
                line,
                &format!("{} is empty.", context.interpolation_label),
                Some("Put an expression inside `{...}`."),
            ));
            cursor = close + 1;
            continue;
        }
        let (expression, spec) = split_format_field(inside);
        let format_spec = parse_format_spec(spec.unwrap_or_default());
        if let Some(semantic_type) =
            resolve_format_expression_type(expression, typed_bindings, functions)
        {
            if let Some(unit) = &format_spec.unit {
                validate_requested_unit(
                    expression,
                    &semantic_type.quantity_kind,
                    unit,
                    line,
                    context.unit_code,
                    diagnostics,
                );
            }
            fields.push(FormatExpressionInfo {
                expression: expression.to_owned(),
                quantity_kind: semantic_type.quantity_kind,
                display_unit: semantic_type.display_unit,
                requested_unit: format_spec.unit,
                precision: format_spec.precision,
                line,
            });
        } else {
            diagnostics.push(unknown_format_expression_diagnostic(
                expression,
                line,
                context.unknown_code,
            ));
        }
        cursor = close + 1;
    }
    fields
}

#[derive(Clone, Copy)]
struct FormatDiagnosticContext {
    template_label: &'static str,
    interpolation_label: &'static str,
    unterminated_code: &'static str,
    empty_code: &'static str,
    unit_code: &'static str,
    unknown_code: &'static str,
}

const PRINT_FORMAT_DIAGNOSTICS: FormatDiagnosticContext = FormatDiagnosticContext {
    template_label: "Print template",
    interpolation_label: "Print interpolation",
    unterminated_code: "E-PRINT-FMT-001",
    empty_code: "E-PRINT-FMT-002",
    unit_code: "E-PRINT-FMT-003",
    unknown_code: "E-PRINT-FMT-004",
};

const WRITE_TEXT_FORMAT_DIAGNOSTICS: FormatDiagnosticContext = FormatDiagnosticContext {
    template_label: "Text write template",
    interpolation_label: "Text write interpolation",
    unterminated_code: "E-WRITE-FMT-001",
    empty_code: "E-WRITE-FMT-002",
    unit_code: "E-WRITE-FMT-003",
    unknown_code: "E-WRITE-FMT-004",
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ParsedFormatSpec {
    precision: Option<usize>,
    unit: Option<String>,
}

fn parse_format_spec(spec: &str) -> ParsedFormatSpec {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return ParsedFormatSpec::default();
    }
    let Some(after_dot) = trimmed.strip_prefix('.') else {
        return ParsedFormatSpec {
            precision: None,
            unit: Some(trimmed.to_owned()),
        };
    };
    let digit_count = after_dot
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .count();
    let precision = if digit_count > 0 {
        after_dot[..digit_count].parse::<usize>().ok()
    } else {
        None
    };
    let unit_text = after_dot[digit_count..].trim();
    let unit = if unit_text.is_empty() {
        None
    } else {
        Some(unit_text.to_owned())
    };
    ParsedFormatSpec { precision, unit }
}

fn split_format_field(field: &str) -> (&str, Option<&str>) {
    field
        .split_once(':')
        .map(|(expression, spec)| (expression.trim(), Some(spec.trim())))
        .unwrap_or((field.trim(), None))
}

fn resolve_format_expression_type(
    expression: &str,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
) -> Option<SemanticType> {
    let expression = expression.trim();
    if expression.starts_with("args.") {
        return semantic_type("String", "");
    }
    if let Some(semantic_type) = path_helper_semantic_type(expression) {
        return Some(semantic_type);
    }
    if let Some(table_name) = expression.strip_suffix(".rows") {
        let table_name = table_name.trim();
        if typed_bindings.iter().any(|binding| {
            binding.name == table_name
                && is_materialized_table_quantity_kind(&binding.semantic_type.quantity_kind)
        }) {
            return semantic_type("Count", "count");
        }
    }
    if let Some(semantic_type) = probability_expression_semantic_type(expression) {
        return Some(semantic_type);
    }
    if let Some(semantic_type) = statistic_expression_semantic_type(expression, typed_bindings) {
        return Some(semantic_type);
    }
    if let Some(semantic_type) = function_call_semantic_type(expression, typed_bindings, functions)
    {
        return Some(semantic_type);
    }
    if let Some(semantic_type) = net_response_field_semantic_type(expression, typed_bindings) {
        return Some(semantic_type);
    }
    if let Some(semantic_type) = coverage_result_field_semantic_type(expression, typed_bindings) {
        return Some(semantic_type);
    }
    typed_bindings
        .iter()
        .find(|binding| binding.name == expression)
        .map(|binding| binding.semantic_type.clone())
}

fn is_materialized_table_quantity_kind(quantity_kind: &str) -> bool {
    quantity_kind.starts_with("Table[") || quantity_kind == "TableTransform[Derive]"
}

fn is_standard_text_table_quantity_kind(quantity_kind: &str) -> bool {
    quantity_kind.starts_with("Table[") || quantity_kind.starts_with("TableTransform[")
}

fn coverage_result_field_semantic_type(
    expression: &str,
    typed_bindings: &[TypedBinding],
) -> Option<SemanticType> {
    let (binding_name, field) = expression.trim().split_once('.')?;
    let has_coverage_binding = typed_bindings.iter().any(|binding| {
        binding.name == binding_name.trim()
            && binding.semantic_type.quantity_kind == "CoverageResult"
    });
    if !has_coverage_binding {
        return None;
    }
    match field.trim() {
        "complete" => semantic_type("Bool", ""),
        "status" | "leap_year_policy" => semantic_type("String", ""),
        "missing_count" | "actual_count" | "expected_count" => semantic_type("Count", "count"),
        "max_gap" | "expected_step" => semantic_type("Duration", "s"),
        "max_gap_hours" => semantic_type("Duration", "h"),
        "year" | "coverage_year" => semantic_type("DimensionlessNumber", "1"),
        _ => None,
    }
}

fn net_response_field_semantic_type(
    expression: &str,
    typed_bindings: &[TypedBinding],
) -> Option<SemanticType> {
    let (binding_name, field) = expression.trim().split_once('.')?;
    let has_response_binding = typed_bindings.iter().any(|binding| {
        binding.name == binding_name.trim() && binding.semantic_type.quantity_kind == "HttpResponse"
    });
    if !has_response_binding {
        return None;
    }
    match field.trim() {
        "body" | "text" | "status" | "status_class" | "response_hash" | "hash" | "url" => {
            semantic_type("String", "")
        }
        "status_code" => semantic_type("DimensionlessNumber", "1"),
        _ => None,
    }
}

fn statistic_expression_semantic_type(
    expression: &str,
    typed_bindings: &[TypedBinding],
) -> Option<SemanticType> {
    let (statistic, source) = parse_statistic_expression(expression)?;
    let source_binding = typed_bindings
        .iter()
        .find(|binding| binding.name == source)?;
    if uncertainty_statistic_supported(&statistic) {
        if let Some((_kind, quantity_kind)) = crate::uncertainty::uncertainty_inner_quantity(
            &source_binding.semantic_type.quantity_kind,
        ) {
            return semantic_type(&quantity_kind, &source_binding.semantic_type.display_unit);
        }
    }
    let (_axis, quantity_kind) =
        crate::stats::time_series_quantity(&source_binding.semantic_type.quantity_kind)?;
    semantic_type(&quantity_kind, &source_binding.semantic_type.display_unit)
}

fn probability_expression_semantic_type(expression: &str) -> Option<SemanticType> {
    let call = parse_function_call(expression)?;
    if call.name == "probability" && call.args.len() == 1 {
        return semantic_type("DimensionlessNumber", "1");
    }
    None
}

fn uncertainty_statistic_supported(statistic: &str) -> bool {
    statistic == "mean" || percentile_statistic(statistic)
}

fn uncertainty_percentile_expression(expression: &str, typed_bindings: &[TypedBinding]) -> bool {
    let Some((statistic, source)) = parse_statistic_expression(expression) else {
        return false;
    };
    if !percentile_statistic(&statistic) {
        return false;
    }
    typed_bindings
        .iter()
        .find(|binding| binding.name == source)
        .is_some_and(|binding| {
            crate::uncertainty::uncertainty_inner_quantity(&binding.semantic_type.quantity_kind)
                .is_some()
        })
}

fn percentile_statistic(statistic: &str) -> bool {
    let Some(percentile) = statistic.strip_prefix('p') else {
        return false;
    };
    !percentile.is_empty()
        && percentile
            .chars()
            .all(|character| character.is_ascii_digit())
}

fn binding_alias_semantic_type(
    expression: &str,
    typed_bindings: &[TypedBinding],
) -> Option<SemanticType> {
    let expression = expression.trim();
    typed_bindings
        .iter()
        .find(|binding| binding.name == expression)
        .map(|binding| binding.semantic_type.clone())
}

fn function_call_semantic_type(
    expression: &str,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
) -> Option<SemanticType> {
    let call = parse_function_call(expression)?;
    let function = functions
        .iter()
        .find(|function| function.name == call.name)?;
    let display_unit = function.return_display_unit.clone();
    if call.args.len() == function.parameters.len()
        && function_call_args_dimensionally_valid(&call, function, typed_bindings)
    {
        return semantic_type(&function.return_quantity_kind, &display_unit);
    }
    semantic_type(&function.return_quantity_kind, &display_unit)
}

fn validate_object_method_call_expression(
    expression: &str,
    line: usize,
    classes: &[ClassInfo],
    class_objects: &[ClassObjectInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<SemanticType> {
    let call = parse_object_method_call(expression)?;
    if !call.args.is_empty() {
        diagnostics.push(Diagnostic::error(
            "E-CLASS-METHOD-CALL-003",
            line,
            &format!(
                "Method call `{}.{}` passes arguments, but method arguments are not supported yet.",
                call.object_name, call.method_name
            ),
            Some("Use zero-argument metadata methods such as `building.summary()`."),
        ));
    }
    let Some(object) = class_objects
        .iter()
        .find(|object| object.name == call.object_name)
    else {
        diagnostics.push(Diagnostic::error(
            "E-CLASS-METHOD-CALL-001",
            line,
            &format!(
                "Method call references unknown object `{}`.",
                call.object_name
            ),
            Some("Declare the object before calling a class method."),
        ));
        return None;
    };
    let class_info = classes
        .iter()
        .find(|class_info| class_info.name == object.class_name)?;
    let Some(method) = class_info
        .methods
        .iter()
        .find(|method| method.name == call.method_name)
    else {
        diagnostics.push(Diagnostic::error(
            "E-CLASS-METHOD-CALL-002",
            line,
            &format!(
                "Class `{}` has no method `{}`.",
                class_info.name, call.method_name
            ),
            Some("Use a method declared by the object's class."),
        ));
        return None;
    };
    semantic_type(&method.return_quantity_kind, &method.return_display_unit)
}

fn validate_function_call_expression(
    expression: &str,
    line: usize,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<SemanticType> {
    let call = parse_function_call(expression)?;
    let Some(function) = functions.iter().find(|function| function.name == call.name) else {
        diagnostics.push(Diagnostic::error(
            "E-FN-CALL-001",
            line,
            &format!("Function `{}` is not defined.", call.name),
            Some("Define the function before use or import a file that defines it."),
        ));
        return None;
    };
    if call.args.len() != function.parameters.len() {
        diagnostics.push(Diagnostic::error(
            "E-FN-CALL-002",
            line,
            &format!(
                "Function `{}` expects {} argument(s), but got {}.",
                function.name,
                function.parameters.len(),
                call.args.len()
            ),
            Some("Pass one expression for each function parameter."),
        ));
        return semantic_type(
            &function.return_quantity_kind,
            &function.return_display_unit,
        );
    }
    for (arg, parameter) in call.args.iter().zip(&function.parameters) {
        let Some(actual_dimension) = expression_dimension_for_bindings(arg, typed_bindings) else {
            diagnostics.push(Diagnostic::error(
                "E-FN-CALL-003",
                line,
                &format!(
                    "Argument `{arg}` for `{}` could not be type-checked.",
                    function.name
                ),
                Some("Use a typed binding, a literal with units, or a compatible expression."),
            ));
            continue;
        };
        if !dimensions_compatible(&parameter.dimension, &actual_dimension) {
            diagnostics.push(Diagnostic::error(
                "E-FN-CALL-004",
                line,
                &format!(
                    "Argument `{arg}` for `{}` has dimension {}, but parameter `{}` expects {}.",
                    function.name, actual_dimension, parameter.name, parameter.dimension
                ),
                Some("Pass a quantity with a compatible unit and dimension."),
            ));
        }
    }
    semantic_type(
        &function.return_quantity_kind,
        &function.return_display_unit,
    )
}

fn function_call_args_dimensionally_valid(
    call: &FunctionCall,
    function: &FunctionInfo,
    typed_bindings: &[TypedBinding],
) -> bool {
    call.args
        .iter()
        .zip(&function.parameters)
        .all(|(arg, parameter)| {
            expression_dimension_for_bindings(arg, typed_bindings)
                .is_some_and(|dimension| dimensions_compatible(&parameter.dimension, &dimension))
        })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FunctionCall {
    name: String,
    args: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ObjectMethodCall {
    object_name: String,
    method_name: String,
    args: Vec<String>,
}

fn parse_function_call(expression: &str) -> Option<FunctionCall> {
    let expression = strip_outer_parens(expression.trim());
    let open = expression.find('(')?;
    if !expression.ends_with(')') {
        return None;
    }
    let name = expression[..open].trim();
    if !is_identifier(name) {
        return None;
    }
    let args_text = &expression[open + 1..expression.len() - 1];
    let args = if args_text.trim().is_empty() {
        Vec::new()
    } else {
        split_top_level(args_text, &[','])
    };
    Some(FunctionCall {
        name: name.to_owned(),
        args,
    })
}

fn parse_object_method_call(expression: &str) -> Option<ObjectMethodCall> {
    let expression = strip_outer_parens(expression.trim());
    let open = expression.find('(')?;
    if !expression.ends_with(')') {
        return None;
    }
    let receiver = expression[..open].trim();
    let (object_name, method_name) = receiver.split_once('.')?;
    if !is_identifier(object_name.trim()) || !is_identifier(method_name.trim()) {
        return None;
    }
    let args_text = &expression[open + 1..expression.len() - 1];
    let args = if args_text.trim().is_empty() {
        Vec::new()
    } else {
        split_top_level(args_text, &[','])
    };
    Some(ObjectMethodCall {
        object_name: object_name.trim().to_owned(),
        method_name: method_name.trim().to_owned(),
        args,
    })
}

fn should_validate_function_call(expression: &str, functions: &[FunctionInfo]) -> bool {
    let Some(call) = parse_function_call(expression) else {
        return false;
    };
    functions.iter().any(|function| function.name == call.name) || !is_builtin_function(&call.name)
}

fn is_builtin_function(name: &str) -> bool {
    matches!(
        name,
        "integrate"
            | "mean"
            | "time_weighted_mean"
            | "max"
            | "min"
            | "median"
            | "std"
            | "sum"
            | "normal"
            | "uniform"
            | "interval"
            | "measured"
            | "propagate"
            | "ensemble"
            | "distribution"
            | "train_test_split"
            | "regression"
            | "regression_table"
            | "train_regression"
            | "mlp"
            | "ann"
            | "evaluate"
            | "model_card"
            | "leakage_lint"
            | "select_first_row"
            | "date"
            | "check"
            | "fill"
            | "align"
            | "resample"
            | "render"
            | "apply"
            | "file"
            | "dir"
            | "join"
            | "parent"
            | "stem"
            | "extension"
            | "exists"
    ) || name.starts_with('p')
}

fn expression_dimension_for_bindings(
    expression: &str,
    typed_bindings: &[TypedBinding],
) -> Option<String> {
    let symbols = typed_bindings
        .iter()
        .map(|binding| DimensionSymbol {
            name: binding.name.clone(),
            dimension: dimension_for_quantity(&binding.semantic_type.quantity_kind),
        })
        .collect::<Vec<_>>();
    expression_dimension_with_symbols(expression, &symbols)
}

fn parse_statistic_expression(expression: &str) -> Option<(String, String)> {
    let trimmed = expression.trim();
    let open = trimmed.find('(')?;
    let statistic = trimmed[..open].trim();
    if !matches!(
        statistic,
        "mean" | "time_weighted_mean" | "max" | "min" | "median" | "std"
    ) && !statistic.starts_with('p')
    {
        return None;
    }
    let rest = trimmed[open + 1..].trim();
    let source = rest
        .split([',', ')'])
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some((statistic.to_owned(), source.to_owned()))
}

fn validate_requested_unit(
    expression: &str,
    quantity_kind: &str,
    unit: &str,
    line: usize,
    code: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !unit_is_supported(unit) {
        diagnostics.push(Diagnostic::error(
            code,
            line,
            &format!("Requested display unit `{unit}` is not supported."),
            Some("Use a unit from the built-in unit registry, such as kW, kWh, degC, or kg/s."),
        ));
        return;
    }
    if !unit_compatible_with_quantity(quantity_kind, unit) {
        diagnostics.push(Diagnostic::error(
            code,
            line,
            &format!(
                "`{expression}` has quantity `{quantity_kind}` and cannot be displayed as `{unit}`."
            ),
            Some("Choose a display unit compatible with the expression quantity."),
        ));
    }
}

fn unit_is_supported(unit: &str) -> bool {
    !candidates_for_unit(unit).is_empty()
}

fn unit_compatible_with_quantity(quantity_kind: &str, unit: &str) -> bool {
    let quantity_kind = scalar_quantity_kind(quantity_kind);
    candidates_for_unit(unit).iter().any(|candidate| {
        candidate.quantity_kind == quantity_kind.as_str()
            || candidate.quantity_kind == "HeatRate"
                && matches!(quantity_kind.as_str(), "ElectricPower" | "MechanicalPower")
            || candidate.quantity_kind == "AbsoluteTemperature"
                && quantity_kind == "TemperatureDelta"
    })
}

fn scalar_quantity_kind(quantity_kind: &str) -> String {
    crate::stats::time_series_quantity(quantity_kind)
        .map(|(_, quantity)| quantity)
        .unwrap_or_else(|| quantity_kind.to_owned())
}

fn unknown_format_expression_diagnostic(expression: &str, line: usize, code: &str) -> Diagnostic {
    Diagnostic::error(
        code,
        line,
        &format!("Cannot resolve formatted expression `{expression}`."),
        Some("Bind the value first, or use a supported expression such as `args.input`, `table.rows`, or `mean(Q, axis=Time)`."),
    )
}

fn export_field_name(expression: &str) -> String {
    let trimmed = expression.trim();
    if is_identifier(trimmed) {
        return trimmed.to_owned();
    }
    if let Some((statistic, source)) = parse_statistic_expression(trimmed) {
        return format!("{statistic}_{source}");
    }
    trimmed
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_owned()
}

fn analyze_args_field(field: &ArgsFieldDecl, args_block: &mut ArgsBlockInfo) {
    args_block.fields.push(ArgsFieldInfo {
        name: field.name.clone(),
        type_name: field.type_name.clone(),
        default_value: field.default_value.clone(),
        redacted: secret_type_inner(&field.type_name).is_some(),
        required: field.default_value.is_none(),
        line: field.line,
    });
}

fn analyze_class_field(
    field: &ClassFieldDecl,
    class_info: &mut ClassInfo,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let display_unit = field
        .unit
        .clone()
        .unwrap_or_else(|| default_unit_for_type(&field.type_name));
    let canonical_unit = default_unit_for_type(&field.type_name);
    let dimension = dimension_for_type(&field.type_name);

    if let Some(default_value) = &field.default_value {
        if let Some(actual) =
            class_field_expression_semantic_type(&field.name, default_value, &[], &[], &[], &[])
        {
            let expected = SemanticType {
                quantity_kind: field.type_name.clone(),
                display_unit: display_unit.clone(),
            };
            if !class_field_types_compatible(&expected, &actual) {
                diagnostics.push(class_field_type_diagnostic(
                    &class_info.name,
                    &field.name,
                    &expected.quantity_kind,
                    &actual.quantity_kind,
                    field.line,
                ));
            }
        }
    }

    class_info.fields.push(ClassFieldInfo {
        name: field.name.clone(),
        type_name: field.type_name.clone(),
        quantity_kind: field.type_name.clone(),
        display_unit,
        canonical_unit,
        dimension,
        default_value: field.default_value.clone(),
        required: field.default_value.is_none(),
        status: "declared".to_owned(),
        line: field.line,
    });
}

fn analyze_class_validation(
    validation: &ClassValidationDecl,
    class_info: &mut ClassInfo,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some((left, operator, right)) = split_class_validation_expression(&validation.expression)
    else {
        diagnostics.push(Diagnostic::error(
            "E-CLASS-VALIDATION-001",
            validation.line,
            &format!(
                "Class `{}` validation `{}` is not a supported comparison.",
                class_info.name, validation.expression
            ),
            Some("Use a simple comparison such as `u_value > 0 W/K` or `name != \"\"`."),
        ));
        return;
    };
    class_info.validations.push(ClassValidationInfo {
        expression: validation.expression.clone(),
        left,
        operator,
        right,
        status: "declared".to_owned(),
        line: validation.line,
    });
}

fn analyze_class_method(
    method: &ClassMethodDecl,
    class_info: &mut ClassInfo,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let return_display_unit = method
        .return_unit
        .clone()
        .unwrap_or_else(|| default_unit_for_type(&method.return_type));
    let return_canonical_unit = default_unit_for_type(&method.return_type);
    let expected = SemanticType {
        quantity_kind: method.return_type.clone(),
        display_unit: return_display_unit.clone(),
    };
    let actual = class_method_expression_semantic_type(&method.expression, class_info);
    let status = match actual {
        Some(actual) if class_field_types_compatible(&expected, &actual) => "typed",
        Some(actual) => {
            diagnostics.push(Diagnostic::error(
                "E-CLASS-METHOD-RETURN-001",
                method.line,
                &format!(
                    "Class `{}` method `{}` returns `{}`, but declaration expects `{}`.",
                    class_info.name, method.name, actual.quantity_kind, expected.quantity_kind
                ),
                Some("Adjust the method return type or return a compatible `self.<field>` value."),
            ));
            "return_mismatch"
        }
        None => {
            diagnostics.push(Diagnostic::error(
                "E-CLASS-METHOD-SELF-001",
                method.line,
                &format!(
                    "Class `{}` method `{}` cannot resolve `{}`.",
                    class_info.name, method.name, method.expression
                ),
                Some(
                    "The current method support accepts direct `self.<field>` return expressions.",
                ),
            ));
            "unresolved_expression"
        }
    };
    class_info.methods.push(ClassMethodInfo {
        name: method.name.clone(),
        return_type: method.return_type.clone(),
        return_quantity_kind: method.return_type.clone(),
        return_display_unit,
        return_canonical_unit,
        expression: method.expression.clone(),
        status: status.to_owned(),
        line: method.line,
    });
}

#[allow(clippy::too_many_arguments)]
fn analyze_class_object_decl(
    object: &ClassObjectDecl,
    classes: &[ClassInfo],
    class_objects: &mut Vec<ClassObjectInfo>,
    diagnostics: &mut Vec<Diagnostic>,
    typed_bindings: &mut Vec<TypedBinding>,
    hover_hints: &mut Vec<HoverHint>,
    type_infos: &mut Vec<TypeInfo>,
) {
    let class_exists = classes
        .iter()
        .any(|class_info| class_info.name == object.class_name);
    if !class_exists {
        diagnostics.push(Diagnostic::error(
            "E-CLASS-OBJECT-001",
            object.line,
            &format!(
                "Object `{}` references unknown class `{}`.",
                object.name, object.class_name
            ),
            Some("Declare the class before constructing an object literal."),
        ));
    }
    let quantity_kind = object_type_name(&object.class_name);
    typed_bindings.push(TypedBinding {
        name: object.name.clone(),
        semantic_type: SemanticType {
            quantity_kind: quantity_kind.clone(),
            display_unit: "object".to_owned(),
        },
        line: object.line,
    });
    hover_hints.push(HoverHint::explicit(
        object.name.clone(),
        quantity_kind.clone(),
        "object".to_owned(),
        Some(format!("{} literal", object.class_name)),
        object.span,
    ));
    type_infos.push(TypeInfo {
        name: object.name.clone(),
        quantity_kind,
        display_unit: "object".to_owned(),
        canonical_unit: "object".to_owned(),
        dimension: "Object".to_owned(),
        source: TypeInfoSource::ObjectLiteral,
        line: object.line,
        span: object.span,
    });
    class_objects.push(ClassObjectInfo {
        name: object.name.clone(),
        class_name: object.class_name.clone(),
        source_object: None,
        construction: "literal".to_owned(),
        fields: Vec::new(),
        validations: Vec::new(),
        status: if class_exists {
            "class_resolved".to_owned()
        } else {
            "unknown_class".to_owned()
        },
        line: object.line,
    });
}

#[allow(clippy::too_many_arguments)]
fn analyze_class_object_copy_decl(
    object: &ClassObjectCopyDecl,
    classes: &[ClassInfo],
    class_objects: &mut Vec<ClassObjectInfo>,
    diagnostics: &mut Vec<Diagnostic>,
    typed_bindings: &mut Vec<TypedBinding>,
    hover_hints: &mut Vec<HoverHint>,
    type_infos: &mut Vec<TypeInfo>,
) {
    let source = class_objects
        .iter()
        .find(|candidate| candidate.name == object.source_name)
        .cloned();
    let Some(source) = source else {
        diagnostics.push(Diagnostic::error(
            "E-CLASS-COPY-001",
            object.line,
            &format!(
                "Copy-with object `{}` references unknown source object `{}`.",
                object.name, object.source_name
            ),
            Some("Declare the source object before using copy-with syntax."),
        ));
        return;
    };
    let class_exists = classes
        .iter()
        .any(|class_info| class_info.name == source.class_name);
    let quantity_kind = object_type_name(&source.class_name);
    typed_bindings.push(TypedBinding {
        name: object.name.clone(),
        semantic_type: SemanticType {
            quantity_kind: quantity_kind.clone(),
            display_unit: "object".to_owned(),
        },
        line: object.line,
    });
    hover_hints.push(HoverHint::explicit(
        object.name.clone(),
        quantity_kind.clone(),
        "object".to_owned(),
        Some(format!(
            "{} copy of {}",
            source.class_name, object.source_name
        )),
        object.span,
    ));
    type_infos.push(TypeInfo {
        name: object.name.clone(),
        quantity_kind,
        display_unit: "object".to_owned(),
        canonical_unit: "object".to_owned(),
        dimension: "Object".to_owned(),
        source: TypeInfoSource::ObjectLiteral,
        line: object.line,
        span: object.span,
    });
    let mut fields = source.fields.clone();
    for field in &mut fields {
        field.status = "copied".to_owned();
    }
    class_objects.push(ClassObjectInfo {
        name: object.name.clone(),
        class_name: source.class_name.clone(),
        source_object: Some(object.source_name.clone()),
        construction: "copy_with".to_owned(),
        fields,
        validations: Vec::new(),
        status: if class_exists {
            "copied".to_owned()
        } else {
            "unknown_class".to_owned()
        },
        line: object.line,
    });
}

fn analyze_class_object_field(
    field: &ClassObjectFieldDecl,
    classes: &[ClassInfo],
    class_objects: &mut [ClassObjectInfo],
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(owner_line) = field.owner_line else {
        diagnostics.push(Diagnostic::error(
            "E-CLASS-OBJECT-002",
            field.line,
            &format!(
                "Object field `{}` is not attached to an object literal.",
                field.name
            ),
            Some("Write object fields inside `name = ClassName { ... }`."),
        ));
        return;
    };
    let Some(object_index) = class_objects
        .iter()
        .position(|object| object.line == owner_line)
    else {
        diagnostics.push(Diagnostic::error(
            "E-CLASS-OBJECT-002",
            field.line,
            &format!(
                "Object field `{}` is not attached to an object literal.",
                field.name
            ),
            Some("Write object fields inside `name = ClassName { ... }`."),
        ));
        return;
    };
    let object_name = class_objects[object_index].name.clone();
    let class_name = class_objects[object_index].class_name.clone();
    let class_info = classes
        .iter()
        .find(|class_info| class_info.name == class_name);
    let expected_field = class_info.and_then(|class_info| {
        class_info
            .fields
            .iter()
            .find(|item| item.name == field.name)
    });
    if expected_field.is_none() {
        diagnostics.push(Diagnostic::error(
            "E-CLASS-FIELD-UNKNOWN-001",
            field.line,
            &format!(
                "Object `{}` sets unknown field `{}` for class `{}`.",
                object_name, field.name, class_name
            ),
            Some("Use a field declared by the class or remove this object field."),
        ));
    }

    let actual = class_field_expression_semantic_type(
        &field.name,
        &field.expression,
        typed_bindings,
        functions,
        classes,
        class_objects,
    );
    let (quantity_kind, display_unit, status) = match (expected_field, actual) {
        (Some(expected), Some(actual)) => {
            let expected_type = class_field_expected_semantic_type(expected, classes);
            if class_field_types_compatible(&expected_type, &actual) {
                (
                    actual.quantity_kind,
                    actual.display_unit,
                    "typed".to_owned(),
                )
            } else {
                diagnostics.push(class_field_type_diagnostic(
                    &class_name,
                    &field.name,
                    &expected_type.quantity_kind,
                    &actual.quantity_kind,
                    field.line,
                ));
                (
                    actual.quantity_kind,
                    actual.display_unit,
                    "type_mismatch".to_owned(),
                )
            }
        }
        (Some(expected), None) => {
            diagnostics.push(Diagnostic::error(
                "E-CLASS-FIELD-TYPE-001",
                field.line,
                &format!(
                    "Cannot type-check field `{}` for class `{}`.",
                    field.name, class_name
                ),
                Some("Use a typed binding, object, string literal, bool, or numeric literal with a compatible unit."),
            ));
            let expected_type = class_field_expected_semantic_type(expected, classes);
            (
                expected_type.quantity_kind,
                expected_type.display_unit,
                "unresolved_expression".to_owned(),
            )
        }
        (None, Some(actual)) => (
            actual.quantity_kind,
            actual.display_unit,
            "unknown_field".to_owned(),
        ),
        (None, None) => (
            "unknown".to_owned(),
            "unknown".to_owned(),
            "unknown_field".to_owned(),
        ),
    };
    if class_objects[object_index].construction == "copy_with" {
        class_objects[object_index]
            .fields
            .retain(|existing| !(existing.name == field.name && existing.status == "copied"));
    }
    class_objects[object_index]
        .fields
        .push(ClassObjectFieldInfo {
            name: field.name.clone(),
            expression: field.expression.clone(),
            quantity_kind,
            display_unit,
            status,
            line: field.line,
        });
}

fn validate_class_contracts(
    classes: &[ClassInfo],
    class_objects: &mut [ClassObjectInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for class_info in classes {
        for field in &class_info.fields {
            if !class_field_type_is_known(&field.type_name, classes) {
                diagnostics.push(Diagnostic::error(
                    "E-CLASS-FIELD-TYPE-002",
                    field.line,
                    &format!(
                        "Class `{}` field `{}` uses unknown type `{}`.",
                        class_info.name, field.name, field.type_name
                    ),
                    Some("Use a known quantity/scalar type or another declared class name."),
                ));
            }
        }
    }

    for object in class_objects.iter_mut() {
        let Some(class_info) = classes
            .iter()
            .find(|class_info| class_info.name == object.class_name)
        else {
            continue;
        };
        for field in class_info.fields.iter().filter(|field| field.required) {
            if !object
                .fields
                .iter()
                .any(|object_field| object_field.name == field.name)
            {
                diagnostics.push(Diagnostic::error(
                    "E-CLASS-FIELD-MISSING-001",
                    object.line,
                    &format!(
                        "Object `{}` is missing required field `{}` for class `{}`.",
                        object.name, field.name, class_info.name
                    ),
                    Some("Provide the field in the object literal or add a class field default."),
                ));
            }
        }
        evaluate_class_object_validations(class_info, object, diagnostics);
    }
}

fn evaluate_class_object_validations(
    class_info: &ClassInfo,
    object: &mut ClassObjectInfo,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for validation in &class_info.validations {
        let result = evaluate_class_validation(class_info, object, validation);
        if result.status == "fail" {
            diagnostics.push(Diagnostic::error(
                "E-CLASS-VALIDATION-002",
                object.line,
                &format!(
                    "Object `{}` violates class `{}` validation `{}`.",
                    object.name, class_info.name, validation.expression
                ),
                Some("Adjust the object field values or the class validation rule."),
            ));
        }
        object.validations.push(result);
    }
}

fn evaluate_class_validation(
    class_info: &ClassInfo,
    object: &ClassObjectInfo,
    validation: &ClassValidationInfo,
) -> ClassObjectValidationInfo {
    let left_value = resolve_class_validation_value(&validation.left, class_info, object);
    let right_value = resolve_class_validation_value(&validation.right, class_info, object);
    let (status, unit) = match (&left_value, &right_value) {
        (Some(ClassValidationValue::Number(left)), Some(ClassValidationValue::Number(right))) => {
            if dimensions_compatible(&left.dimension, &right.dimension) {
                (
                    if compare_numeric_values(left.value, right.value, &validation.operator) {
                        "pass"
                    } else {
                        "fail"
                    },
                    if left.unit == "1" {
                        right.unit.clone()
                    } else {
                        left.unit.clone()
                    },
                )
            } else {
                ("unresolved", "unit_mismatch".to_owned())
            }
        }
        (Some(ClassValidationValue::Text(left)), Some(ClassValidationValue::Text(right))) => (
            if compare_text_values(left, right, &validation.operator) {
                "pass"
            } else {
                "fail"
            },
            "string".to_owned(),
        ),
        (Some(ClassValidationValue::Bool(left)), Some(ClassValidationValue::Bool(right))) => (
            if compare_bool_values(*left, *right, &validation.operator) {
                "pass"
            } else {
                "fail"
            },
            "bool".to_owned(),
        ),
        _ => ("unresolved", "unknown".to_owned()),
    };
    ClassObjectValidationInfo {
        expression: validation.expression.clone(),
        left: validation.left.clone(),
        operator: validation.operator.clone(),
        right: validation.right.clone(),
        left_value: left_value.as_ref().map(ClassValidationValue::display),
        right_value: right_value.as_ref().map(ClassValidationValue::display),
        unit,
        status: status.to_owned(),
        line: validation.line,
    }
}

#[derive(Clone, Debug)]
enum ClassValidationValue {
    Number(ClassValidationNumber),
    Text(String),
    Bool(bool),
}

#[derive(Clone, Debug)]
struct ClassValidationNumber {
    value: f64,
    unit: String,
    dimension: String,
    display: String,
}

impl ClassValidationValue {
    fn display(&self) -> String {
        match self {
            Self::Number(value) => value.display.clone(),
            Self::Text(value) => value.clone(),
            Self::Bool(value) => value.to_string(),
        }
    }
}

fn split_class_validation_expression(expression: &str) -> Option<(String, String, String)> {
    for operator in ["==", "!=", ">=", "<=", ">", "<"] {
        if let Some((left, right)) = expression.split_once(operator) {
            let left = left.trim();
            let right = right.trim();
            if !left.is_empty() && !right.is_empty() {
                return Some((left.to_owned(), operator.to_owned(), right.to_owned()));
            }
        }
    }
    None
}

fn resolve_class_validation_value(
    expression: &str,
    class_info: &ClassInfo,
    object: &ClassObjectInfo,
) -> Option<ClassValidationValue> {
    let expression = expression.trim();
    if let Some(field) = class_info
        .fields
        .iter()
        .find(|field| field.name == expression)
    {
        let value_expression = object
            .fields
            .iter()
            .find(|object_field| object_field.name == field.name)
            .map(|object_field| object_field.expression.as_str())
            .or(field.default_value.as_deref())?;
        return literal_class_validation_value(value_expression);
    }
    literal_class_validation_value(expression)
}

fn literal_class_validation_value(expression: &str) -> Option<ClassValidationValue> {
    let expression = expression.trim();
    if let Some(value) = expression
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        return Some(ClassValidationValue::Text(value.to_owned()));
    }
    if matches!(expression, "true" | "false") {
        return Some(ClassValidationValue::Bool(expression == "true"));
    }
    let (value, unit) = numeric_literal_with_optional_unit(expression)?;
    let Some(unit) = unit else {
        return Some(ClassValidationValue::Number(ClassValidationNumber {
            value,
            unit: "1".to_owned(),
            dimension: "Dimensionless".to_owned(),
            display: format_number_with_unit(value, "1"),
        }));
    };
    let unit_info = unit_info_for_symbol(&unit)?;
    let scale = unit_info.scale_to_canonical.parse::<f64>().ok()?;
    let offset = unit_info
        .affine_offset
        .and_then(|offset| offset.parse::<f64>().ok())
        .unwrap_or(0.0);
    Some(ClassValidationValue::Number(ClassValidationNumber {
        value: value * scale + offset,
        unit: unit_info.canonical_unit.to_owned(),
        dimension: unit_info.dimension.to_owned(),
        display: format_number_with_unit(value, &unit),
    }))
}

fn format_number_with_unit(value: f64, unit: &str) -> String {
    if unit == "1" {
        return value.to_string();
    }
    format!("{value} {unit}")
}

fn compare_numeric_values(left: f64, right: f64, operator: &str) -> bool {
    match operator {
        "==" => (left - right).abs() <= f64::EPSILON,
        "!=" => (left - right).abs() > f64::EPSILON,
        ">" => left > right,
        "<" => left < right,
        ">=" => left >= right,
        "<=" => left <= right,
        _ => false,
    }
}

fn compare_text_values(left: &str, right: &str, operator: &str) -> bool {
    match operator {
        "==" => left == right,
        "!=" => left != right,
        _ => false,
    }
}

fn compare_bool_values(left: bool, right: bool, operator: &str) -> bool {
    match operator {
        "==" => left == right,
        "!=" => left != right,
        _ => false,
    }
}

fn class_field_type_is_known(type_name: &str, classes: &[ClassInfo]) -> bool {
    known_decl_type(type_name)
        || classes
            .iter()
            .any(|class_info| class_info.name == type_name)
}

fn class_field_expected_semantic_type(
    field: &ClassFieldInfo,
    classes: &[ClassInfo],
) -> SemanticType {
    if classes
        .iter()
        .any(|class_info| class_info.name == field.type_name)
    {
        return SemanticType {
            quantity_kind: object_type_name(&field.type_name),
            display_unit: "object".to_owned(),
        };
    }
    SemanticType {
        quantity_kind: field.type_name.clone(),
        display_unit: field.display_unit.clone(),
    }
}

fn class_method_expression_semantic_type(
    expression: &str,
    class_info: &ClassInfo,
) -> Option<SemanticType> {
    let field_name = expression.trim().strip_prefix("self.")?.trim();
    if field_name.is_empty() || field_name.contains('.') {
        return None;
    }
    let field = class_info
        .fields
        .iter()
        .find(|field| field.name == field_name)?;
    Some(SemanticType {
        quantity_kind: field.type_name.clone(),
        display_unit: field.display_unit.clone(),
    })
}

fn class_field_expression_semantic_type(
    field_name: &str,
    expression: &str,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
    classes: &[ClassInfo],
    class_objects: &[ClassObjectInfo],
) -> Option<SemanticType> {
    let expression = expression.trim();
    if expression.starts_with('"') && expression.ends_with('"') {
        return semantic_type("String", "");
    }
    if matches!(expression, "true" | "false") {
        return semantic_type("Bool", "");
    }
    if let Some((_, unit)) = numeric_literal_with_optional_unit(expression) {
        if let Some(unit) = unit {
            let quantity = candidates_for_unit(&unit).first().copied()?;
            return semantic_type(quantity.quantity_kind, quantity.canonical_unit);
        }
        return semantic_type("DimensionlessNumber", "1");
    }
    object_field_access_semantic_type(expression, classes, class_objects)
        .or_else(|| path_helper_semantic_type(expression))
        .or_else(|| statistic_expression_semantic_type(expression, typed_bindings))
        .or_else(|| function_call_semantic_type(expression, typed_bindings, functions))
        .or_else(|| binding_alias_semantic_type(expression, typed_bindings))
        .or_else(|| infer_quantity(field_name, expression))
}

fn object_field_access_semantic_type(
    expression: &str,
    classes: &[ClassInfo],
    class_objects: &[ClassObjectInfo],
) -> Option<SemanticType> {
    let (object_name, field_name) = expression.trim().split_once('.')?;
    if object_name.contains('.') || field_name.contains('.') {
        return None;
    }
    let object = class_objects
        .iter()
        .find(|object| object.name == object_name.trim())?;
    if let Some(field) = object
        .fields
        .iter()
        .find(|field| field.name == field_name.trim() && field.status != "unknown_field")
    {
        return semantic_type(&field.quantity_kind, &field.display_unit);
    }
    let class_info = classes
        .iter()
        .find(|class_info| class_info.name == object.class_name)?;
    let field = class_info
        .fields
        .iter()
        .find(|field| field.name == field_name.trim())?;
    let expected = class_field_expected_semantic_type(field, classes);
    semantic_type(&expected.quantity_kind, &expected.display_unit)
}

fn class_field_types_compatible(expected: &SemanticType, actual: &SemanticType) -> bool {
    if expected.quantity_kind == actual.quantity_kind {
        return true;
    }
    if matches!(
        expected.quantity_kind.as_str(),
        "Int" | "Integer" | "Count" | "Float" | "Number"
    ) && actual.quantity_kind == "DimensionlessNumber"
    {
        return true;
    }
    if matches!(expected.quantity_kind.as_str(), "Bool" | "String") {
        return false;
    }
    dimensions_compatible(
        &dimension_for_quantity(&expected.quantity_kind),
        &dimension_for_quantity(&actual.quantity_kind),
    )
}

fn class_field_type_diagnostic(
    class_name: &str,
    field_name: &str,
    expected: &str,
    actual: &str,
    line: usize,
) -> Diagnostic {
    Diagnostic::error(
        "E-CLASS-FIELD-TYPE-001",
        line,
        &format!(
            "Class `{class_name}` field `{field_name}` expects `{expected}`, but got `{actual}`."
        ),
        Some("Use a compatible literal, typed binding, or object field value."),
    )
}

fn object_type_name(class_name: &str) -> String {
    format!("Object[{class_name}]")
}

fn analyze_domain_variable(declaration: &DomainVariableDecl, domain: &mut DomainInfo) {
    let display_unit = declaration
        .unit
        .clone()
        .unwrap_or_else(|| default_unit_for_quantity(&declaration.type_name));
    let canonical_unit = default_unit_for_quantity(&declaration.type_name);
    let dimension = dimension_for_quantity(&declaration.type_name);
    domain.variables.push(DomainVariableInfo {
        role: declaration.role.clone(),
        name: declaration.name.clone(),
        quantity_kind: declaration.type_name.clone(),
        display_unit,
        canonical_unit,
        dimension,
        line: declaration.line,
    });
}

fn domain_type_parameter_info(parameter: &DomainTypeParameterDecl) -> DomainTypeParameterInfo {
    let display = if parameter.kind == parameter.name {
        parameter.kind.clone()
    } else {
        format!("{} {}", parameter.kind, parameter.name)
    };
    DomainTypeParameterInfo {
        kind: parameter.kind.clone(),
        name: parameter.name.clone(),
        display,
    }
}

fn validate_domain_contracts(domains: &[DomainInfo], diagnostics: &mut Vec<Diagnostic>) {
    for domain in domains {
        if !domain
            .variables
            .iter()
            .any(|variable| variable.role == "across")
        {
            diagnostics.push(Diagnostic::error(
                "E-DOMAIN-CONTRACT-001",
                domain.line,
                &format!("Domain `{}` has no across variable.", domain.name),
                Some("Add at least one `across <name>: <Quantity> [unit]` declaration."),
            ));
        }
        if !domain
            .variables
            .iter()
            .any(|variable| variable.role == "through")
        {
            diagnostics.push(Diagnostic::error(
                "E-DOMAIN-CONTRACT-002",
                domain.line,
                &format!("Domain `{}` has no through variable.", domain.name),
                Some("Add at least one `through <name>: <Quantity> [unit]` declaration."),
            ));
        }
        if domain.conservations.is_empty() {
            diagnostics.push(Diagnostic::error(
                "E-DOMAIN-CONTRACT-003",
                domain.line,
                &format!("Domain `{}` has no conservation contract.", domain.name),
                Some("Add a `conservation ...` line that records the domain balance contract."),
            ));
        }
        for variable in &domain.variables {
            if variable.dimension == "unknown" {
                diagnostics.push(Diagnostic::error(
                    "E-DOMAIN-VAR-001",
                    variable.line,
                    &format!(
                        "Domain variable `{}.{}` uses unknown quantity kind `{}`.",
                        domain.name, variable.name, variable.quantity_kind
                    ),
                    Some("Use a known quantity kind from the EngLang quantity registry."),
                ));
            }
        }
    }
}

fn analyze_port(declaration: &PortDecl, component: &mut ComponentInfo) {
    let domain_ref = parse_domain_reference(&declaration.domain);
    component.ports.push(PortInfo {
        name: declaration.name.clone(),
        domain: domain_ref.canonical(),
        domain_name: domain_ref.name,
        type_arguments: domain_ref.type_arguments,
        status: "unvalidated".to_owned(),
        line: declaration.line,
    });
}

fn analyze_component_parameter(
    declaration: &SystemVariableDecl,
    component: &mut ComponentInfo,
    consts: &[ConstInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let parameter = analyze_component_scalar_declaration(
        declaration,
        &component.name,
        consts,
        diagnostics,
        "parameter",
        "E-COMPONENT-PARAM-001",
    );
    component.parameters.push(parameter);
}

fn analyze_component_input(
    declaration: &SystemVariableDecl,
    component: &mut ComponentInfo,
    consts: &[ConstInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let input = analyze_component_scalar_declaration(
        declaration,
        &component.name,
        consts,
        diagnostics,
        "input",
        "E-COMPONENT-INPUT-001",
    );
    component.inputs.push(input);
}

fn analyze_component_scalar_declaration(
    declaration: &SystemVariableDecl,
    component_name: &str,
    consts: &[ConstInfo],
    diagnostics: &mut Vec<Diagnostic>,
    declaration_kind: &str,
    diagnostic_code: &'static str,
) -> ComponentParameterInfo {
    let display_unit = declaration
        .unit
        .clone()
        .or_else(|| {
            declaration
                .expression
                .as_deref()
                .and_then(first_unit_in_expression)
        })
        .unwrap_or_else(|| default_unit_for_quantity(&declaration.type_name));
    let canonical_unit = default_unit_for_quantity(&declaration.type_name);
    let dimension = dimension_for_quantity(&declaration.type_name);
    let mut resolved_value = declaration.expression.clone();
    let status = if dimension == "unknown" {
        diagnostics.push(Diagnostic::error(
            diagnostic_code,
            declaration.line,
            &format!(
                "Component {declaration_kind} `{}` on `{component_name}` uses unknown quantity kind `{}`.",
                declaration.name, declaration.type_name
            ),
            Some("Use a known quantity kind from the EngLang quantity registry."),
        ));
        "unknown_quantity"
    } else if let Some(default_value) = &declaration.expression {
        if let Some(value) = resolve_component_parameter_value(
            &declaration.name,
            default_value,
            &declaration.type_name,
            declaration.line,
            consts,
            diagnostics,
        ) {
            resolved_value = Some(value);
            "defaulted"
        } else {
            "default_type_mismatch"
        }
    } else {
        "required"
    };

    ComponentParameterInfo {
        name: declaration.name.clone(),
        quantity_kind: declaration.type_name.clone(),
        display_unit,
        canonical_unit,
        dimension,
        default_value: declaration.expression.clone(),
        value: resolved_value,
        source: if declaration.expression.is_some() {
            "default".to_owned()
        } else {
            "required".to_owned()
        },
        status: status.to_owned(),
        line: declaration.line,
    }
}

fn resolve_component_parameter_value(
    parameter_name: &str,
    value: &str,
    quantity_kind: &str,
    line: usize,
    consts: &[ConstInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    let value = value.trim();
    if numeric_literal_with_optional_unit(value).is_some() {
        return component_parameter_literal_compatible(
            parameter_name,
            value,
            quantity_kind,
            line,
            diagnostics,
        )
        .then(|| value.to_owned());
    }
    if let Some(const_info) = consts
        .iter()
        .find(|const_info| const_info.importable && const_info.name == value)
    {
        let expected_dimension = dimension_for_quantity(quantity_kind);
        if !dimensions_compatible(&expected_dimension, &const_info.dimension) {
            diagnostics.push(Diagnostic::error(
                "E-COMPONENT-PARAM-UNIT-001",
                line,
                &format!(
                    "Component parameter `{parameter_name}` expects `{quantity_kind}`, but const `{}` has quantity `{}`.",
                    const_info.name, const_info.quantity_kind
                ),
                Some("Use a constructor/default const with a compatible quantity."),
            ));
            return None;
        }
        let resolved = const_info.expression.trim();
        if numeric_literal_with_optional_unit(resolved).is_some() {
            return component_parameter_literal_compatible(
                parameter_name,
                resolved,
                quantity_kind,
                const_info.line,
                diagnostics,
            )
            .then(|| resolved.to_owned());
        }
    }

    match evaluate_component_parameter_expression(value, consts) {
        Ok(evaluated) => {
            let expected_dimension = dimension_for_quantity(quantity_kind);
            if dimensions_compatible(&expected_dimension, &evaluated.dimension) {
                Some(format_number_with_unit(
                    evaluated.value,
                    &default_unit_for_quantity(quantity_kind),
                ))
            } else {
                diagnostics.push(Diagnostic::error(
                    "E-COMPONENT-PARAM-UNIT-001",
                    line,
                    &format!(
                        "Component parameter `{parameter_name}` expects `{quantity_kind}`, but expression `{value}` has dimension `{}`.",
                        evaluated.dimension
                    ),
                    Some("Use a constructor/default expression whose units reduce to the declared parameter quantity."),
                ));
                None
            }
        }
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                error.code,
                line,
                &format!(
                    "Component parameter `{parameter_name}` expects a numeric literal, importable const, or pure arithmetic expression, got `{value}`: {}.",
                    error.message
                ),
                Some(error.help),
            ));
            None
        }
    }
}

fn component_parameter_literal_compatible(
    parameter_name: &str,
    value: &str,
    quantity_kind: &str,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some((_amount, unit)) = numeric_literal_with_optional_unit(value) else {
        return false;
    };
    let Some(unit) = unit else {
        return true;
    };
    if unit_compatible_with_quantity(quantity_kind, &unit) {
        true
    } else {
        diagnostics.push(Diagnostic::error(
            "E-COMPONENT-PARAM-UNIT-001",
            line,
            &format!(
                "Component parameter `{parameter_name}` value unit `{unit}` is not compatible with `{quantity_kind}`."
            ),
            Some("Use a constructor/default value with a unit compatible with the declared parameter quantity."),
        ));
        false
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ComponentParameterExpressionValue {
    value: f64,
    dimension: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ComponentParameterExpressionError {
    code: &'static str,
    message: String,
    help: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
enum ComponentParameterExpressionToken {
    Number(ComponentParameterExpressionValue),
    Identifier(String),
    Plus,
    Minus,
    Star,
    Slash,
    LeftParen,
    RightParen,
}

fn evaluate_component_parameter_expression(
    expression: &str,
    consts: &[ConstInfo],
) -> Result<ComponentParameterExpressionValue, ComponentParameterExpressionError> {
    evaluate_component_parameter_expression_inner(expression, consts, 0)
}

fn evaluate_component_parameter_expression_inner(
    expression: &str,
    consts: &[ConstInfo],
    depth: usize,
) -> Result<ComponentParameterExpressionValue, ComponentParameterExpressionError> {
    if depth > 16 {
        return Err(component_parameter_expression_error(
            "E-COMPONENT-PARAM-002",
            "const expression expansion exceeded the supported depth",
            "Reduce nested const references in the component parameter expression.",
        ));
    }
    let tokens = tokenize_component_parameter_expression(expression, consts, depth)?;
    if tokens.is_empty() {
        return Err(component_parameter_expression_error(
            "E-COMPONENT-PARAM-002",
            "expression is empty",
            "Use a finite numeric literal, importable const, or pure arithmetic expression.",
        ));
    }
    let mut parser = ComponentParameterExpressionParser {
        tokens,
        position: 0,
    };
    let value = parser.parse_expression()?;
    if parser.position != parser.tokens.len() {
        return Err(component_parameter_expression_error(
            "E-COMPONENT-PARAM-002",
            "expression has trailing unsupported tokens",
            "Use only numeric literals, importable consts, parentheses, and +, -, *, /.",
        ));
    }
    if !value.value.is_finite() {
        return Err(component_parameter_expression_error(
            "E-COMPONENT-PARAM-002",
            "expression produced a non-finite value",
            "Use finite numeric values and avoid overflow or division by zero.",
        ));
    }
    Ok(value)
}

fn tokenize_component_parameter_expression(
    expression: &str,
    consts: &[ConstInfo],
    depth: usize,
) -> Result<Vec<ComponentParameterExpressionToken>, ComponentParameterExpressionError> {
    let chars = expression.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut index = 0usize;
    while index < chars.len() {
        let character = chars[index];
        if character.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        match character {
            '+' => {
                tokens.push(ComponentParameterExpressionToken::Plus);
                index += 1;
            }
            '-' => {
                tokens.push(ComponentParameterExpressionToken::Minus);
                index += 1;
            }
            '*' => {
                tokens.push(ComponentParameterExpressionToken::Star);
                index += 1;
            }
            '/' => {
                tokens.push(ComponentParameterExpressionToken::Slash);
                index += 1;
            }
            '(' => {
                tokens.push(ComponentParameterExpressionToken::LeftParen);
                index += 1;
            }
            ')' => {
                tokens.push(ComponentParameterExpressionToken::RightParen);
                index += 1;
            }
            _ if character.is_ascii_digit()
                || character == '.'
                    && chars
                        .get(index + 1)
                        .is_some_and(|next| next.is_ascii_digit()) =>
            {
                let start = index;
                index += 1;
                while index < chars.len() {
                    let current = chars[index];
                    if current.is_ascii_digit() || current == '.' {
                        index += 1;
                    } else if matches!(current, 'e' | 'E') {
                        index += 1;
                        if chars
                            .get(index)
                            .is_some_and(|next| matches!(next, '+' | '-'))
                        {
                            index += 1;
                        }
                    } else {
                        break;
                    }
                }
                let literal = chars[start..index].iter().collect::<String>();
                let amount = literal.parse::<f64>().map_err(|_| {
                    component_parameter_expression_error(
                        "E-COMPONENT-PARAM-002",
                        format!("invalid numeric literal `{literal}`"),
                        "Use finite decimal numeric literals in component parameter expressions.",
                    )
                })?;
                let (next_index, unit) = consume_component_parameter_unit_suffix(&chars, index);
                tokens.push(ComponentParameterExpressionToken::Number(
                    component_parameter_number_value(amount, unit.as_deref())?,
                ));
                index = next_index;
            }
            _ if is_identifier_start_character(character) => {
                let start = index;
                index += 1;
                while index < chars.len() && is_identifier_continue_character(chars[index]) {
                    index += 1;
                }
                let name = chars[start..index].iter().collect::<String>();
                if let Some(const_info) = consts
                    .iter()
                    .find(|const_info| const_info.importable && const_info.name == name)
                {
                    tokens.push(ComponentParameterExpressionToken::Number(
                        component_parameter_const_value(const_info, consts, depth + 1)?,
                    ));
                } else {
                    tokens.push(ComponentParameterExpressionToken::Identifier(name));
                }
            }
            _ => {
                return Err(component_parameter_expression_error(
                    "E-COMPONENT-PARAM-002",
                    format!("unsupported character `{character}`"),
                    "Use only numeric literals, importable consts, parentheses, and +, -, *, /.",
                ));
            }
        }
    }
    Ok(tokens)
}

fn consume_component_parameter_unit_suffix(
    chars: &[char],
    index: usize,
) -> (usize, Option<String>) {
    let mut cursor = index;
    let mut saw_whitespace = false;
    while chars
        .get(cursor)
        .is_some_and(|character| character.is_ascii_whitespace())
    {
        saw_whitespace = true;
        cursor += 1;
    }
    if !saw_whitespace {
        return (index, None);
    }

    let suffix_start = cursor;
    while let Some(character) = chars.get(cursor) {
        if character.is_ascii_whitespace() || matches!(character, '+' | '-' | '*' | '(' | ')') {
            break;
        }
        if *character == '/'
            || *character == '^'
            || character.is_ascii_alphanumeric()
            || *character == '\u{00b0}'
        {
            cursor += 1;
        } else {
            break;
        }
    }
    let suffix = chars[suffix_start..cursor].iter().collect::<String>();
    if !suffix.is_empty()
        && suffix
            .chars()
            .any(|character| character.is_ascii_alphabetic() || character == '\u{00b0}')
    {
        (cursor, Some(suffix))
    } else {
        (index, None)
    }
}

fn component_parameter_number_value(
    amount: f64,
    unit: Option<&str>,
) -> Result<ComponentParameterExpressionValue, ComponentParameterExpressionError> {
    let Some(unit) = unit else {
        return Ok(ComponentParameterExpressionValue {
            value: amount,
            dimension: "Dimensionless".to_owned(),
        });
    };
    component_parameter_unit_value(amount, unit)
}

fn component_parameter_const_value(
    const_info: &ConstInfo,
    consts: &[ConstInfo],
    depth: usize,
) -> Result<ComponentParameterExpressionValue, ComponentParameterExpressionError> {
    if let Some((amount, unit)) = numeric_literal_with_optional_unit(&const_info.expression) {
        let unit = unit.as_deref().unwrap_or(&const_info.display_unit);
        let value = component_parameter_unit_value(amount, unit)?;
        if dimensions_compatible(&const_info.dimension, &value.dimension) {
            return Ok(ComponentParameterExpressionValue {
                value: value.value,
                dimension: const_info.dimension.clone(),
            });
        }
        return Err(component_parameter_expression_error(
            "E-COMPONENT-PARAM-UNIT-001",
            format!(
                "const `{}` has declared dimension `{}` but literal dimension `{}`",
                const_info.name, const_info.dimension, value.dimension
            ),
            "Make imported const annotations and unit literals compatible.",
        ));
    }

    let value =
        evaluate_component_parameter_expression_inner(&const_info.expression, consts, depth)?;
    if dimensions_compatible(&const_info.dimension, &value.dimension) {
        Ok(ComponentParameterExpressionValue {
            value: value.value,
            dimension: const_info.dimension.clone(),
        })
    } else {
        Err(component_parameter_expression_error(
            "E-COMPONENT-PARAM-UNIT-001",
            format!(
                "const `{}` has declared dimension `{}` but expression dimension `{}`",
                const_info.name, const_info.dimension, value.dimension
            ),
            "Make imported const annotations and arithmetic expression units compatible.",
        ))
    }
}

fn component_parameter_unit_value(
    amount: f64,
    unit: &str,
) -> Result<ComponentParameterExpressionValue, ComponentParameterExpressionError> {
    if normalize_unit(unit) == "1" {
        return Ok(ComponentParameterExpressionValue {
            value: amount,
            dimension: "Dimensionless".to_owned(),
        });
    }
    let Some(unit_info) = unit_info_for_symbol(unit) else {
        return Err(component_parameter_expression_error(
            "E-COMPONENT-PARAM-UNIT-001",
            format!("unit `{unit}` is not supported"),
            "Use units from the built-in unit registry in component parameter expressions.",
        ));
    };
    let scale = unit_info.scale_to_canonical.parse::<f64>().map_err(|_| {
        component_parameter_expression_error(
            "E-COMPONENT-PARAM-002",
            format!("unit `{unit}` has an invalid scale seed"),
            "Use units with finite conversion metadata.",
        )
    })?;
    let offset = unit_info
        .affine_offset
        .and_then(|offset| offset.parse::<f64>().ok())
        .unwrap_or(0.0);
    Ok(ComponentParameterExpressionValue {
        value: amount * scale + offset,
        dimension: unit_info.dimension.to_owned(),
    })
}

struct ComponentParameterExpressionParser {
    tokens: Vec<ComponentParameterExpressionToken>,
    position: usize,
}

impl ComponentParameterExpressionParser {
    fn parse_expression(
        &mut self,
    ) -> Result<ComponentParameterExpressionValue, ComponentParameterExpressionError> {
        let mut value = self.parse_term()?;
        loop {
            match self.peek() {
                Some(ComponentParameterExpressionToken::Plus) => {
                    self.position += 1;
                    let right = self.parse_term()?;
                    value = add_component_parameter_values(value, right, 1.0)?;
                }
                Some(ComponentParameterExpressionToken::Minus) => {
                    self.position += 1;
                    let right = self.parse_term()?;
                    value = add_component_parameter_values(value, right, -1.0)?;
                }
                _ => return Ok(value),
            }
        }
    }

    fn parse_term(
        &mut self,
    ) -> Result<ComponentParameterExpressionValue, ComponentParameterExpressionError> {
        let mut value = self.parse_factor()?;
        loop {
            match self.peek() {
                Some(ComponentParameterExpressionToken::Star) => {
                    self.position += 1;
                    let right = self.parse_factor()?;
                    value = ComponentParameterExpressionValue {
                        value: value.value * right.value,
                        dimension: multiply_dimensions(&value.dimension, &right.dimension),
                    };
                }
                Some(ComponentParameterExpressionToken::Slash) => {
                    self.position += 1;
                    let right = self.parse_factor()?;
                    if right.value.abs() <= f64::EPSILON {
                        return Err(component_parameter_expression_error(
                            "E-COMPONENT-PARAM-002",
                            "expression attempted division by zero",
                            "Avoid division by zero in component parameter expressions.",
                        ));
                    }
                    value = ComponentParameterExpressionValue {
                        value: value.value / right.value,
                        dimension: divide_dimensions(&value.dimension, &right.dimension),
                    };
                }
                _ => return Ok(value),
            }
        }
    }

    fn parse_factor(
        &mut self,
    ) -> Result<ComponentParameterExpressionValue, ComponentParameterExpressionError> {
        let Some(token) = self.next().cloned() else {
            return Err(component_parameter_expression_error(
                "E-COMPONENT-PARAM-002",
                "expression ended unexpectedly",
                "Use complete numeric literals, const references, or parenthesized expressions.",
            ));
        };
        match token {
            ComponentParameterExpressionToken::Number(value) => Ok(value),
            ComponentParameterExpressionToken::Identifier(name) => {
                Err(component_parameter_expression_error(
                    "E-COMPONENT-PARAM-002",
                    format!("unknown symbol `{name}`"),
                    "Reference only importable top-level consts from component parameter expressions.",
                ))
            }
            ComponentParameterExpressionToken::Minus => {
                let mut value = self.parse_factor()?;
                value.value = -value.value;
                Ok(value)
            }
            ComponentParameterExpressionToken::Plus => self.parse_factor(),
            ComponentParameterExpressionToken::LeftParen => {
                let value = self.parse_expression()?;
                match self.next() {
                    Some(ComponentParameterExpressionToken::RightParen) => Ok(value),
                    _ => Err(component_parameter_expression_error(
                        "E-COMPONENT-PARAM-002",
                        "expression has an unclosed parenthesis",
                        "Close parenthesized component parameter expressions.",
                    )),
                }
            }
            _ => Err(component_parameter_expression_error(
                "E-COMPONENT-PARAM-002",
                "expression expected a value",
                "Use complete numeric literals, const references, or parenthesized expressions.",
            )),
        }
    }

    fn peek(&self) -> Option<&ComponentParameterExpressionToken> {
        self.tokens.get(self.position)
    }

    fn next(&mut self) -> Option<&ComponentParameterExpressionToken> {
        let token = self.tokens.get(self.position);
        if token.is_some() {
            self.position += 1;
        }
        token
    }
}

fn add_component_parameter_values(
    left: ComponentParameterExpressionValue,
    right: ComponentParameterExpressionValue,
    right_sign: f64,
) -> Result<ComponentParameterExpressionValue, ComponentParameterExpressionError> {
    if !dimensions_compatible(&left.dimension, &right.dimension) {
        return Err(component_parameter_expression_error(
            "E-COMPONENT-PARAM-UNIT-001",
            format!(
                "cannot add dimensions `{}` and `{}`",
                left.dimension, right.dimension
            ),
            "Add or subtract only component parameter terms with compatible units.",
        ));
    }
    Ok(ComponentParameterExpressionValue {
        value: left.value + right_sign * right.value,
        dimension: left.dimension,
    })
}

fn divide_dimensions(left: &str, right: &str) -> String {
    combine_dimensions(left, right, DimensionOperator::Divide)
}

fn component_parameter_expression_error(
    code: &'static str,
    message: impl Into<String>,
    help: &'static str,
) -> ComponentParameterExpressionError {
    ComponentParameterExpressionError {
        code,
        message: message.into(),
        help,
    }
}
fn analyze_component_local_expression(
    binding: &crate::ast::FastBinding,
    component: &mut ComponentInfo,
    domains: &[DomainInfo],
) {
    let signal_contract = infer_component_local_expression_signal_contract(
        domains,
        component,
        binding.line,
        &binding.expression,
    );
    component
        .local_expressions
        .push(ComponentLocalExpressionInfo {
            name: binding.name.clone(),
            expression: binding.expression.clone(),
            status: "metadata_only".to_owned(),
            quantity_kind: signal_contract.quantity_kind,
            display_unit: signal_contract.display_unit,
            canonical_unit: signal_contract.canonical_unit,
            type_status: signal_contract.status,
            line: binding.line,
        });
}

fn analyze_component_equation(equation: &crate::ast::EquationDecl, component: &mut ComponentInfo) {
    let equation_index = component
        .local_expressions
        .iter()
        .filter(|local| local.status == "component_equation_seed")
        .count()
        + 1;
    let left = strip_component_equation_label(&equation.left);
    component
        .local_expressions
        .push(ComponentLocalExpressionInfo {
            name: format!("equation_{equation_index}"),
            expression: format!("{} eq {}", left, equation.right),
            status: "component_equation_seed".to_owned(),
            quantity_kind: "unknown".to_owned(),
            display_unit: "unknown".to_owned(),
            canonical_unit: "unknown".to_owned(),
            type_status: "component_equation_pending_assembly".to_owned(),
            line: equation.line,
        });
}

fn strip_component_equation_label(left: &str) -> &str {
    let trimmed = left.trim();
    let Some((label, expression)) = trimmed.split_once(':') else {
        return trimmed;
    };
    if is_identifier(label.trim()) && !expression.trim().is_empty() {
        expression.trim()
    } else {
        trimmed
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ComponentInstanceBindingAnalysis {
    Instance(ComponentInfo),
    HandledInvalid,
    NotComponentConstructor,
}

fn analyze_component_instance_binding(
    binding: &crate::ast::FastBinding,
    templates: &[ComponentInfo],
    existing_instances: &[ComponentInfo],
    consts: &[ConstInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> ComponentInstanceBindingAnalysis {
    let Some((template_name, arguments)) = component_constructor_call(&binding.expression) else {
        return ComponentInstanceBindingAnalysis::NotComponentConstructor;
    };
    if existing_instances
        .iter()
        .any(|instance| instance.name == binding.name)
    {
        diagnostics.push(Diagnostic::error(
            "E-COMPONENT-INSTANCE-DUPLICATE",
            binding.line,
            &format!("Component instance `{}` is already declared.", binding.name),
            Some("Use a unique system-local instance name."),
        ));
        return ComponentInstanceBindingAnalysis::HandledInvalid;
    }
    let Some(template) = templates
        .iter()
        .find(|component| component.name == template_name)
    else {
        diagnostics.push(Diagnostic::error(
            "E-COMPONENT-INSTANCE-UNKNOWN",
            binding.line,
            &format!(
                "Component instance `{}` references unknown component `{template_name}`.",
                binding.name
            ),
            Some("Declare the component before instantiating it inside a system block."),
        ));
        return ComponentInstanceBindingAnalysis::HandledInvalid;
    };

    let Some(arguments) = parse_component_constructor_arguments(
        &binding.name,
        template,
        &arguments,
        binding.line,
        diagnostics,
    ) else {
        return ComponentInstanceBindingAnalysis::HandledInvalid;
    };
    let Some(instance) =
        instantiate_component_template(template, binding, &arguments, consts, diagnostics)
    else {
        return ComponentInstanceBindingAnalysis::HandledInvalid;
    };
    ComponentInstanceBindingAnalysis::Instance(instance)
}

fn parse_component_constructor_arguments(
    instance_name: &str,
    template: &ComponentInfo,
    arguments: &str,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<ComponentConstructorArgumentInfo>> {
    let arguments = arguments.trim();
    if arguments.is_empty() {
        return Some(Vec::new());
    }
    let mut parsed = Vec::new();
    let mut seen = HashSet::new();
    let mut positional_index = 0usize;
    let mut saw_named = false;
    for raw_argument in split_component_constructor_arguments(arguments) {
        let (name, value) = if let Some((name, value)) = raw_argument.split_once('=') {
            saw_named = true;
            (name.trim().to_owned(), value.trim().to_owned())
        } else {
            if template.parameters.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "E-COMPONENT-INSTANCE-ARGS",
                    line,
                    &format!(
                        "Component instance `{instance_name}` calls `{}` with positional arguments, but `{}` has no declared component parameters.",
                        template.name, template.name
                    ),
                    Some("Declare component parameters or use named arguments that are referenced by component-local boundary/equation seeds."),
                ));
                return None;
            }
            if saw_named {
                diagnostics.push(Diagnostic::error(
                    "E-COMPONENT-INSTANCE-ARGS",
                    line,
                    &format!(
                        "Component instance `{instance_name}` passes positional argument `{raw_argument}` after named constructor arguments."
                    ),
                    Some("Place positional component constructor arguments before named arguments."),
                ));
                return None;
            }
            let Some(parameter) = template.parameters.get(positional_index) else {
                diagnostics.push(Diagnostic::error(
                    "E-COMPONENT-INSTANCE-ARGS",
                    line,
                    &format!(
                        "Component instance `{instance_name}` passes too many positional constructor arguments to `{}`.",
                        template.name
                    ),
                    Some("Pass at most one positional argument per declared component parameter, or use named arguments."),
                ));
                return None;
            };
            positional_index += 1;
            (parameter.name.clone(), raw_argument.trim().to_owned())
        };
        if !is_identifier(&name) || value.is_empty() {
            diagnostics.push(Diagnostic::error(
                "E-COMPONENT-INSTANCE-ARGS",
                line,
                &format!(
                    "Component instance `{instance_name}` has invalid constructor argument `{raw_argument}`."
                ),
                Some("Use `name=value` arguments with identifier names and non-empty values."),
            ));
            return None;
        }
        if !seen.insert(name.clone()) {
            diagnostics.push(Diagnostic::error(
                "E-COMPONENT-INSTANCE-ARGS",
                line,
                &format!(
                    "Component instance `{instance_name}` repeats constructor argument `{name}`."
                ),
                Some("Pass each named component constructor argument at most once."),
            ));
            return None;
        }
        parsed.push(ComponentConstructorArgumentInfo { name, value });
    }
    Some(parsed)
}

fn split_component_constructor_arguments(arguments: &str) -> Vec<String> {
    split_top_level(arguments, &[','])
}

fn instantiate_component_template(
    template: &ComponentInfo,
    binding: &crate::ast::FastBinding,
    arguments: &[ComponentConstructorArgumentInfo],
    consts: &[ConstInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ComponentInfo> {
    let mut instance = template.clone();
    instance.name = binding.name.clone();
    instance.template_name = Some(template.name.clone());
    instance.constructor_arguments = arguments.to_vec();
    instance.line = binding.line;

    if template.parameters.is_empty() {
        if arguments.is_empty() {
            return Some(instance);
        }
        let mut used_arguments = HashSet::new();
        for local in &mut instance.local_expressions {
            local.expression = substitute_component_constructor_arguments(
                &local.expression,
                arguments,
                &mut used_arguments,
            );
        }
        let unused_arguments = arguments
            .iter()
            .filter(|argument| !used_arguments.contains(argument.name.as_str()))
            .map(|argument| argument.name.as_str())
            .collect::<Vec<_>>();
        if !unused_arguments.is_empty() {
            diagnostics.push(Diagnostic::error(
                "E-COMPONENT-INSTANCE-ARGS",
                binding.line,
                &format!(
                    "Component instance `{}` passes unused constructor argument(s): {}.",
                    binding.name,
                    unused_arguments.join(", ")
                ),
                Some("Declare component parameters for typed constructor arguments, or pass only names referenced by component-local boundary/equation seeds."),
            ));
            return None;
        }
        return Some(instance);
    }

    let mut parameter_values = HashMap::new();
    for parameter in &template.parameters {
        if let Some(default_value) = &parameter.value {
            parameter_values.insert(parameter.name.clone(), (default_value.clone(), "default"));
        }
    }

    for argument in arguments {
        let Some(parameter) = template
            .parameters
            .iter()
            .find(|parameter| parameter.name == argument.name)
        else {
            diagnostics.push(Diagnostic::error(
                "E-COMPONENT-INSTANCE-ARGS",
                binding.line,
                &format!(
                    "Component instance `{}` passes unknown constructor parameter `{}` to `{}`.",
                    binding.name, argument.name, template.name
                ),
                Some("Pass only parameters declared inside the component template."),
            ));
            return None;
        };
        let Some(resolved_argument) = resolve_component_parameter_value(
            &argument.name,
            &argument.value,
            &parameter.quantity_kind,
            binding.line,
            consts,
            diagnostics,
        ) else {
            return None;
        };
        parameter_values.insert(argument.name.clone(), (resolved_argument, "constructor"));
    }

    for parameter in &mut instance.parameters {
        if let Some((value, source)) = parameter_values.get(&parameter.name) {
            parameter.value = Some(value.clone());
            parameter.source = (*source).to_owned();
            parameter.status = if *source == "constructor" {
                "constructor_override".to_owned()
            } else {
                "defaulted".to_owned()
            };
        } else {
            diagnostics.push(Diagnostic::error(
                "E-COMPONENT-INSTANCE-ARGS",
                binding.line,
                &format!(
                    "Component instance `{}` does not provide required constructor parameter `{}` for `{}`.",
                    binding.name, parameter.name, template.name
                ),
                Some("Pass the required parameter by name or add a default value to the component parameter declaration."),
            ));
            return None;
        }
    }

    Some(instance)
}

fn substitute_component_constructor_arguments(
    expression: &str,
    arguments: &[ComponentConstructorArgumentInfo],
    used_arguments: &mut HashSet<String>,
) -> String {
    let mut output = String::with_capacity(expression.len());
    let mut chars = expression.char_indices().peekable();
    while let Some((start, character)) = chars.next() {
        if !is_identifier_start_character(character) {
            output.push(character);
            continue;
        }
        let mut end = start + character.len_utf8();
        while let Some((next_index, next_character)) = chars.peek().copied() {
            if !is_identifier_continue_character(next_character) {
                break;
            }
            chars.next();
            end = next_index + next_character.len_utf8();
        }
        let token = &expression[start..end];
        if let Some(argument) = arguments.iter().find(|argument| argument.name == token) {
            output.push_str(&argument.value);
            used_arguments.insert(argument.name.clone());
        } else {
            output.push_str(token);
        }
    }
    output
}
fn is_identifier_start_character(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
}

fn is_identifier_continue_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn component_constructor_call(expression: &str) -> Option<(String, String)> {
    let trimmed = expression.trim();
    let open = trimmed.find('(')?;
    if !trimmed.ends_with(')') {
        return None;
    }
    let name = trimmed[..open].trim();
    if !is_identifier(name) {
        return None;
    }
    if !name
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_uppercase())
    {
        return None;
    }
    let arguments = trimmed[open + 1..trimmed.len() - 1].trim();
    Some((name.to_owned(), arguments.to_owned()))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ComponentSignalContract {
    quantity_kind: String,
    display_unit: String,
    canonical_unit: String,
    status: String,
}

fn infer_component_local_expression_signal_contract(
    domains: &[DomainInfo],
    component: &ComponentInfo,
    current_line: usize,
    expression: &str,
) -> ComponentSignalContract {
    component_behavior_signal_contract(domains, component, current_line, expression)
        .unwrap_or_else(unknown_component_signal_contract)
}

fn analyze_connections(
    domains: &[DomainInfo],
    components: &mut [ComponentInfo],
    raw_connections: &[ConnectDecl],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<ConnectionInfo> {
    let mut connections = Vec::new();
    let mut seen_connections = HashSet::new();
    let mut connected_ports = HashSet::new();
    for component in components.iter_mut() {
        for port in &mut component.ports {
            let Some(domain) = domains
                .iter()
                .find(|domain| domain.name == port.domain_name)
            else {
                port.status = "unknown_domain".to_owned();
                diagnostics.push(Diagnostic::error(
                    "E-PORT-DOMAIN-001",
                    port.line,
                    &format!(
                        "Port `{}` on component `{}` references unknown domain `{}`.",
                        port.name, component.name, port.domain
                    ),
                    Some("Declare the domain before using it in a component port."),
                ));
                continue;
            };
            if port.type_arguments.len() != domain.type_parameters.len() {
                port.status = "generic_arity_mismatch".to_owned();
                diagnostics.push(Diagnostic::error(
                    "E-PORT-DOMAIN-002",
                    port.line,
                    &format!(
                        "Port `{}` on component `{}` references `{}` with {} type argument(s), but domain `{}` expects {}.",
                        port.name,
                        component.name,
                        port.domain,
                        port.type_arguments.len(),
                        domain.name,
                        domain.type_parameters.len()
                    ),
                    Some(
                        "Use `Domain[Argument]` for generic domains or remove arguments for non-generic domains.",
                    ),
                ));
                continue;
            }
            port.status = "domain_resolved".to_owned();
        }
    }

    for connection in raw_connections {
        let Some((left_component, left_port)) = split_endpoint(&connection.left) else {
            diagnostics.push(connection_endpoint_diagnostic(
                &connection.left,
                connection.line,
            ));
            continue;
        };
        let Some((right_component, right_port)) = split_endpoint(&connection.right) else {
            diagnostics.push(connection_endpoint_diagnostic(
                &connection.right,
                connection.line,
            ));
            continue;
        };
        let duplicate_key =
            normalized_connection_key(&left_component, &left_port, &right_component, &right_port);
        if !seen_connections.insert(duplicate_key) {
            diagnostics.push(Diagnostic::error(
                "E-CONNECT-DUPLICATE-001",
                connection.line,
                &format!(
                    "Connection `{}` -> `{}` duplicates an existing connection.",
                    connection.left, connection.right
                ),
                Some("Remove the duplicate connection so the graph has one edge per port pair."),
            ));
        }
        let left_resolved = resolved_port(components, &left_component, &left_port);
        let right_resolved = resolved_port(components, &right_component, &right_port);
        let (domain, status) = match (left_resolved, right_resolved) {
            (Some(left), Some(right))
                if left.domain_name == right.domain_name
                    && left.type_arguments == right.type_arguments =>
            {
                (left.domain.clone(), "domain_compatible".to_owned())
            }
            (Some(left), Some(right)) if left.domain_name == right.domain_name => {
                let parameter_name =
                    first_mismatched_parameter(domains, &left.domain_name, left, right)
                        .unwrap_or_else(|| "Parameter".to_owned());
                let (code, status, label) = parameter_mismatch_diagnostic(&parameter_name);
                diagnostics.push(Diagnostic::error(
                    code,
                    connection.line,
                    &format!(
                        "Cannot connect `{}` ({}) to `{}` ({}): {} differs.",
                        connection.left, left.domain, connection.right, right.domain, label
                    ),
                    Some(
                        "Use matching generic domain arguments on both connected component ports.",
                    ),
                ));
                (
                    format!("{} != {}", left.domain, right.domain),
                    status.to_owned(),
                )
            }
            (Some(left), Some(right)) => {
                diagnostics.push(Diagnostic::error(
                    "E-CONNECT-DOMAIN-MISMATCH",
                    connection.line,
                    &format!(
                        "Cannot connect `{}` ({}) to `{}` ({}).",
                        connection.left, left.domain, connection.right, right.domain
                    ),
                    Some("Connect ports only when they declare the same domain."),
                ));
                ("mismatch".to_owned(), "domain_mismatch".to_owned())
            }
            _ => {
                diagnostics.push(Diagnostic::error(
                    "E-CONNECT-UNKNOWN-PORT",
                    connection.line,
                    "Connection endpoint does not resolve to a declared component port.",
                    Some(
                        "Use `connect Component.port -> Other.port` with declared component ports.",
                    ),
                ));
                ("unknown".to_owned(), "unresolved_endpoint".to_owned())
            }
        };
        if left_resolved.is_some() {
            connected_ports.insert(format!("{}.{}", left_component, left_port));
        }
        if right_resolved.is_some() {
            connected_ports.insert(format!("{}.{}", right_component, right_port));
        }

        connections.push(ConnectionInfo {
            left: connection.left.clone(),
            right: connection.right.clone(),
            left_component,
            left_port,
            right_component,
            right_port,
            domain,
            status,
            line: connection.line,
        });
    }
    for component in components {
        for port in &component.ports {
            if port.status == "domain_resolved"
                && !connected_ports.contains(&format!("{}.{}", component.name, port.name))
            {
                diagnostics.push(Diagnostic::warning(
                    "W-CONNECT-UNCONNECTED-PORT",
                    port.line,
                    &format!("Port `{}.{}` is not connected.", component.name, port.name),
                    Some("Connect the port explicitly or leave a review note explaining the boundary assumption."),
                ));
            }
        }
    }
    connections
}

fn build_component_assembly_graphs(
    domains: &[DomainInfo],
    components: &[ComponentInfo],
    connections: &[ConnectionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<ComponentAssemblyInfo> {
    if components.is_empty() {
        return Vec::new();
    }

    let mut port_lookup = HashMap::new();
    let mut source_order_ports = Vec::new();
    for component in components {
        for port in &component.ports {
            let path = format!("{}.{}", component.name, port.name);
            source_order_ports.push(path.clone());
            port_lookup.insert(path, port.clone());
        }
    }

    let compatible_connections = connections
        .iter()
        .filter(|connection| connection.status == "domain_compatible")
        .collect::<Vec<_>>();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for connection in &compatible_connections {
        adjacency
            .entry(connection.left.clone())
            .or_default()
            .push(connection.right.clone());
        adjacency
            .entry(connection.right.clone())
            .or_default()
            .push(connection.left.clone());
    }

    let mut visited = HashSet::new();
    let mut connection_sets = Vec::new();
    for port_path in &source_order_ports {
        if visited.contains(port_path) || !adjacency.contains_key(port_path) {
            continue;
        }
        let mut stack = vec![port_path.clone()];
        let mut set = HashSet::new();
        while let Some(current) = stack.pop() {
            if !set.insert(current.clone()) {
                continue;
            }
            if let Some(neighbors) = adjacency.get(&current) {
                for neighbor in neighbors {
                    if !set.contains(neighbor) {
                        stack.push(neighbor.clone());
                    }
                }
            }
        }
        visited.extend(set.iter().cloned());
        let ports = source_order_ports
            .iter()
            .filter(|path| set.contains(*path))
            .cloned()
            .collect::<Vec<_>>();
        let connection_count = compatible_connections
            .iter()
            .filter(|connection| set.contains(&connection.left) && set.contains(&connection.right))
            .count();
        let line = compatible_connections
            .iter()
            .filter(|connection| set.contains(&connection.left) && set.contains(&connection.right))
            .map(|connection| connection.line)
            .min()
            .unwrap_or_else(|| {
                ports
                    .iter()
                    .filter_map(|path| port_lookup.get(path).map(|port| port.line))
                    .min()
                    .unwrap_or(1)
            });
        let domain = ports
            .first()
            .and_then(|path| port_lookup.get(path))
            .map(|port| port.domain.clone())
            .unwrap_or_else(|| "unknown".to_owned());
        let status = if ports
            .first()
            .and_then(|path| port_lookup.get(path))
            .and_then(|port| {
                domains
                    .iter()
                    .find(|domain| domain.name == port.domain_name)
            })
            .map(|domain| {
                domain
                    .variables
                    .iter()
                    .any(|variable| variable.role == "across")
                    && domain
                        .variables
                        .iter()
                        .any(|variable| variable.role == "through")
            })
            .unwrap_or(false)
        {
            "connection_equations_generated"
        } else {
            "metadata_only"
        };
        connection_sets.push(ComponentConnectionSetInfo {
            name: format!("connection_set_{}", connection_sets.len() + 1),
            domain,
            ports,
            connection_count,
            status: status.to_owned(),
            line,
        });
    }

    let mut variables = Vec::new();
    let mut seen_variables = HashSet::new();
    let mut equations = Vec::new();
    for connection_set in &connection_sets {
        let Some(first_port) = connection_set.ports.first() else {
            continue;
        };
        let Some(port_info) = port_lookup.get(first_port) else {
            continue;
        };
        let Some(domain) = domains
            .iter()
            .find(|domain| domain.name == port_info.domain_name)
        else {
            continue;
        };
        for port_path in &connection_set.ports {
            for variable in &domain.variables {
                let variable_name = format!("{port_path}.{}", variable.name);
                if seen_variables.insert(variable_name.clone()) {
                    variables.push(ComponentAssemblyVariableInfo {
                        name: variable_name,
                        role: "algebraic".to_owned(),
                        domain: connection_set.domain.clone(),
                        source: format!("{}.{}", domain.name, variable.name),
                        status: "classified".to_owned(),
                    });
                }
            }
        }

        for variable in domain
            .variables
            .iter()
            .filter(|variable| variable.role == "across")
        {
            let Some(anchor) = connection_set.ports.first() else {
                continue;
            };
            for (index, port_path) in connection_set.ports.iter().skip(1).enumerate() {
                let left = format!("{anchor}.{}", variable.name);
                let right = format!("{port_path}.{}", variable.name);
                let dependencies = vec![left.clone(), right.clone()];
                let residual = format!("{left} - {right}");
                equations.push(ComponentAssemblyEquationInfo {
                    name: format!(
                        "{}.across_{}_{}",
                        connection_set.name,
                        variable.name,
                        index + 1
                    ),
                    kind: "across_equality".to_owned(),
                    domain: connection_set.domain.clone(),
                    expression: format!("{left} eq {right}"),
                    residual,
                    rhs: None,
                    reason: component_generated_equation_reason("across_equality"),
                    dependencies,
                    status: "assembly_seed".to_owned(),
                    line: connection_set.line,
                });
            }
        }

        for variable in domain
            .variables
            .iter()
            .filter(|variable| variable.role == "through")
        {
            if connection_set.ports.is_empty() {
                continue;
            }
            let dependencies = connection_set
                .ports
                .iter()
                .map(|port_path| format!("{port_path}.{}", variable.name))
                .collect::<Vec<_>>();
            equations.push(ComponentAssemblyEquationInfo {
                name: format!(
                    "{}.through_{}_conservation",
                    connection_set.name, variable.name
                ),
                kind: "through_conservation".to_owned(),
                domain: connection_set.domain.clone(),
                expression: format!("sum({}) eq 0", dependencies.join(", ")),
                residual: dependencies.join(" + "),
                rhs: None,
                reason: component_generated_equation_reason("through_conservation"),
                dependencies,
                status: "assembly_seed".to_owned(),
                line: connection_set.line,
            });
        }
    }

    let component_boundary_equations =
        component_boundary_equations(domains, components, &connection_sets, diagnostics);
    equations.extend(component_boundary_equations);
    let component_local_equations =
        component_local_equations(domains, components, &connection_sets, diagnostics);
    equations.extend(component_local_equations);
    for component in components {
        for parameter in &component.parameters {
            let parameter_name = format!("{}.{}", component.name, parameter.name);
            if seen_variables.insert(parameter_name.clone()) {
                variables.push(ComponentAssemblyVariableInfo {
                    name: parameter_name,
                    role: "parameter".to_owned(),
                    domain: "component_parameter".to_owned(),
                    source: format!("component_parameter.{}", parameter.quantity_kind),
                    status: parameter.status.clone(),
                });
            }
        }
        for input in &component.inputs {
            let input_name = format!("{}.{}", component.name, input.name);
            if seen_variables.insert(input_name.clone()) {
                variables.push(ComponentAssemblyVariableInfo {
                    name: input_name,
                    role: "input".to_owned(),
                    domain: "component_input".to_owned(),
                    source: format!("component_input.{}", input.quantity_kind),
                    status: input.status.clone(),
                });
            }
        }
    }
    classify_dynamic_component_states(&mut variables, &equations);

    let algebraic_count = variables
        .iter()
        .filter(|variable| variable.role == "algebraic")
        .count();
    let state_count = variables
        .iter()
        .filter(|variable| variable.role == "state")
        .count();
    let parameter_count = variables
        .iter()
        .filter(|variable| variable.role == "parameter")
        .count();
    let input_count = variables
        .iter()
        .filter(|variable| variable.role == "input")
        .count();
    let unknown_count = algebraic_count + state_count;
    let equation_count = equations.len();
    let (balance_status, diagnostic_code) = if equation_count < unknown_count {
        (
            "underdetermined_seed".to_owned(),
            Some("E-ASSEMBLY-UNDERDETERMINED".to_owned()),
        )
    } else if equation_count > unknown_count {
        (
            "overdetermined_seed".to_owned(),
            Some("E-ASSEMBLY-OVERDETERMINED".to_owned()),
        )
    } else {
        ("balanced_metadata_seed".to_owned(), None)
    };
    let component_equation_count = equations
        .iter()
        .filter(|equation| {
            equation.kind == "component_boundary" || equation.kind == "component_equation"
        })
        .count();
    let local_expression_count = components
        .iter()
        .map(|component| component.local_expressions.len())
        .sum::<usize>();
    let operator_call_count = count_component_expression_calls(components, &["operator("]);
    let predictor_call_count =
        count_component_expression_calls(components, &["predict(", "predictor("]);
    let delay_call_count = count_component_expression_calls(components, &["delay("]);
    let external_call_count =
        count_component_expression_calls(components, &["external(", "adapter("]);
    let dependencies = equations
        .iter()
        .flat_map(|equation| {
            equation
                .dependencies
                .iter()
                .map(|variable| ComponentResidualDependencyInfo {
                    residual: equation.name.clone(),
                    variable: variable.clone(),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let algebraic_loops = equations
        .iter()
        .filter(|equation| equation.dependencies.len() > 1)
        .map(|equation| equation.dependencies.clone())
        .collect::<Vec<_>>();
    let jacobian_sparsity = equations
        .iter()
        .map(|equation| ComponentJacobianSparsityInfo {
            residual: equation.name.clone(),
            with_respect_to: equation.dependencies.clone(),
            status: "placeholder".to_owned(),
        })
        .collect::<Vec<_>>();
    let (residual_graph_status, residual_solver_plan) = if equations.is_empty() {
        ("empty", "no_residual_equations")
    } else if equation_count == unknown_count {
        (
            "linear_residual_graph_candidate",
            "dense_linear_residual_graph_candidate",
        )
    } else {
        ("metadata_only", "metadata_only_no_numeric_solve")
    };
    let residual_graph = ComponentResidualGraphInfo {
        name: "component_residual_graph".to_owned(),
        status: residual_graph_status.to_owned(),
        residuals: equations
            .iter()
            .map(|equation| equation.name.clone())
            .collect(),
        residual_metadata: equations
            .iter()
            .map(|equation| ComponentResidualGraphResidualInfo {
                name: equation.name.clone(),
                kind: equation.kind.clone(),
                domain: equation.domain.clone(),
                source_expression: equation.expression.clone(),
                residual_expression: equation.residual.clone(),
                rhs: equation.rhs.clone(),
                dependencies: equation.dependencies.clone(),
                status: equation.status.clone(),
                line: equation.line,
            })
            .collect(),
        dependencies,
        algebraic_loops,
        jacobian_sparsity,
        solver_plan: residual_solver_plan.to_owned(),
    };
    let domain_plans =
        build_component_domain_plans(domains, &connection_sets, &equations, &variables);
    let domain_count = domain_plans.len();
    let solver_preview = build_component_solver_preview(
        domain_count,
        state_count,
        equations.len(),
        predictor_call_count,
        delay_call_count,
        external_call_count,
    );
    let line = components
        .iter()
        .map(|component| component.line)
        .chain(connections.iter().map(|connection| connection.line))
        .min()
        .unwrap_or(1);
    vec![ComponentAssemblyInfo {
        name: "component_graph".to_owned(),
        status: if equations.is_empty() {
            "no_compatible_connections".to_owned()
        } else {
            "assembly_seed".to_owned()
        },
        component_count: components.len(),
        port_count: source_order_ports.len(),
        connection_count: connections.len(),
        component_equation_count,
        local_expression_count,
        operator_call_count,
        predictor_call_count,
        domain_count,
        domain_plans,
        solver_preview,
        connection_sets,
        equations,
        variables,
        boundary: ComponentAssemblyBoundaryInfo {
            state_count,
            algebraic_count,
            input_count,
            output_count: 0,
            parameter_count,
            equation_count,
            unknown_count,
            balance_status,
            diagnostic_code,
        },
        residual_graph,
        line,
    }]
}

fn emit_component_assembly_boundary_warnings(
    assemblies: &[ComponentAssemblyInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for assembly in assemblies {
        for algebraic_loop in &assembly.residual_graph.algebraic_loops {
            diagnostics.push(Diagnostic::warning(
                "W-ASSEMBLY-ALGEBRAIC-LOOP",
                assembly.line,
                &format!(
                    "Component assembly `{}` contains an algebraic dependency loop: {}.",
                    assembly.name,
                    algebraic_loop.join(" -> ")
                ),
                Some(
                    "Review the residual dependency graph before treating the assembly as a physical solve.",
                ),
            ));
        }
        let Some(code) = assembly.boundary.diagnostic_code.as_deref() else {
            continue;
        };
        let message = format!(
            "Component assembly `{}` is {} with {} equation(s) and {} unknown(s).",
            assembly.name,
            assembly.boundary.balance_status,
            assembly.boundary.equation_count,
            assembly.boundary.unknown_count
        );
        diagnostics.push(Diagnostic::warning(
            code,
            assembly.line,
            &message,
            Some(
                "The current component graph path records a reviewable assembly seed and limitation artifact; numeric multi-domain solving remains planned.",
            ),
        ));
    }
}

fn build_component_domain_plans(
    domains: &[DomainInfo],
    connection_sets: &[ComponentConnectionSetInfo],
    equations: &[ComponentAssemblyEquationInfo],
    variables: &[ComponentAssemblyVariableInfo],
) -> Vec<ComponentDomainPlanInfo> {
    let mut seen = HashSet::new();
    let mut ordered_domains = Vec::new();
    for connection_set in connection_sets {
        if seen.insert(connection_set.domain.clone()) {
            ordered_domains.push(connection_set.domain.clone());
        }
    }

    ordered_domains
        .iter()
        .map(|domain_signature| {
            let domain_reference = parse_domain_reference(domain_signature);
            let conservation_status = domains
                .iter()
                .find(|domain| domain.name == domain_reference.name)
                .map(|domain| {
                    if domain.conservations.is_empty() {
                        "missing_conservation"
                    } else {
                        "conservation_recorded"
                    }
                })
                .unwrap_or("unknown_domain");
            let equation_count = equations
                .iter()
                .filter(|equation| equation.domain == *domain_signature)
                .count();
            let variable_count = variables
                .iter()
                .filter(|variable| variable.domain == *domain_signature)
                .count();
            ComponentDomainPlanInfo {
                domain: domain_signature.clone(),
                connection_set_count: connection_sets
                    .iter()
                    .filter(|connection_set| connection_set.domain == *domain_signature)
                    .count(),
                equation_count,
                variable_count,
                conservation_status: conservation_status.to_owned(),
                solver_role: if equation_count > 0 {
                    "homogeneous_connection_constraints".to_owned()
                } else {
                    "metadata_only".to_owned()
                },
            }
        })
        .collect()
}

fn count_component_expression_calls(components: &[ComponentInfo], needles: &[&str]) -> usize {
    components
        .iter()
        .flat_map(|component| component.local_expressions.iter())
        .filter(|local| {
            let expression = local.expression.to_ascii_lowercase();
            needles.iter().any(|needle| expression.contains(needle))
        })
        .count()
}

fn validate_component_behavior_calls(
    domains: &[DomainInfo],
    components: &[ComponentInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for component in components {
        for local in &component.local_expressions {
            for arguments in extract_call_arguments(&local.expression, "delay") {
                validate_delay_call(domains, component, local, &arguments, diagnostics);
            }
            for arguments in extract_call_arguments(&local.expression, "predict") {
                validate_predictor_call(domains, component, local, &arguments, diagnostics);
            }
            for arguments in extract_call_arguments(&local.expression, "predictor") {
                validate_predictor_call(domains, component, local, &arguments, diagnostics);
            }
            for arguments in extract_call_arguments(&local.expression, "external") {
                validate_external_behavior_call(domains, component, local, &arguments, diagnostics);
            }
            for arguments in extract_call_arguments(&local.expression, "adapter") {
                validate_external_behavior_call(domains, component, local, &arguments, diagnostics);
            }
        }
    }
}

fn validate_delay_call(
    domains: &[DomainInfo],
    component: &ComponentInfo,
    local: &ComponentLocalExpressionInfo,
    arguments: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let parts = split_top_level(arguments, &[',']);
    if parts.len() != 2 {
        diagnostics.push(Diagnostic::error(
            "E-DELAY-CALL-001",
            local.line,
            &format!(
                "Delay expression `{}` must use `delay(signal, duration)`.",
                local.expression
            ),
            Some("Provide a component port signal and a positive duration such as `delay(outlet.m_dot, 5 s)`."),
        ));
        return;
    }
    let signal = parts[0].trim();
    if component_behavior_signal_contract(domains, component, local.line, signal).is_none() {
        diagnostics.push(Diagnostic::error(
            "E-DELAY-SIGNAL-001",
            local.line,
            &format!(
                "Delay signal `{signal}` is not a known component signal in `{}`.",
                component.name
            ),
            Some("Use a component signal such as `port.variable`, a prior component-local expression, or a behavior expression with resolved quantity/unit metadata."),
        ));
    }
    if parse_duration_option_seconds(parts[1].trim()).is_none() {
        diagnostics.push(Diagnostic::error(
            "E-DELAY-DURATION-001",
            local.line,
            &format!(
                "Delay duration `{}` is not a positive duration.",
                parts[1].trim()
            ),
            Some("Use a duration with time units such as `s`, `min`, or `h`, for example `5 s`."),
        ));
    }
}

fn validate_predictor_call(
    domains: &[DomainInfo],
    component: &ComponentInfo,
    local: &ComponentLocalExpressionInfo,
    arguments: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_single_signal_behavior_call(
        domains,
        component,
        local,
        arguments,
        &BehaviorCallSpec {
            label: "Predictor",
            signature: "predictor(signal)` or `predict(signal)",
            call_code: "E-PREDICTOR-CALL-001",
            signal_code: "E-PREDICTOR-SIGNAL-001",
        },
        diagnostics,
    );
}

fn validate_external_behavior_call(
    domains: &[DomainInfo],
    component: &ComponentInfo,
    local: &ComponentLocalExpressionInfo,
    arguments: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_single_signal_behavior_call(
        domains,
        component,
        local,
        arguments,
        &BehaviorCallSpec {
            label: "External behavior",
            signature: "external(signal)` or `adapter(signal)",
            call_code: "E-EXTERNAL-BEHAVIOR-CALL-001",
            signal_code: "E-EXTERNAL-BEHAVIOR-SIGNAL-001",
        },
        diagnostics,
    );
}

struct BehaviorCallSpec {
    label: &'static str,
    signature: &'static str,
    call_code: &'static str,
    signal_code: &'static str,
}

fn validate_single_signal_behavior_call(
    domains: &[DomainInfo],
    component: &ComponentInfo,
    local: &ComponentLocalExpressionInfo,
    arguments: &str,
    spec: &BehaviorCallSpec,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let parts = split_top_level(arguments, &[',']);
    if parts.len() != 1 {
        diagnostics.push(Diagnostic::error(
            spec.call_code,
            local.line,
            &format!(
                "{} expression `{}` must use `{}`.",
                spec.label, local.expression, spec.signature
            ),
            Some("Pass one component signal such as `outlet.T`, a prior component-local signal, or a behavior expression while full behavior contracts remain runtime-wrapper seeds."),
        ));
        return;
    }
    let signal = parts[0].trim();
    if component_behavior_signal_contract(domains, component, local.line, signal).is_none() {
        diagnostics.push(Diagnostic::error(
            spec.signal_code,
            local.line,
            &format!(
                "{} signal `{signal}` is not a known component signal in `{}`.",
                spec.label, component.name
            ),
            Some("Use a component signal such as `port.variable`, a prior component-local expression, or a behavior expression with resolved quantity/unit metadata."),
        ));
    }
}

fn component_behavior_signal_contract(
    domains: &[DomainInfo],
    component: &ComponentInfo,
    current_line: usize,
    signal: &str,
) -> Option<ComponentSignalContract> {
    component_behavior_expression_contract(domains, component, current_line, signal, 0)
}

fn component_behavior_expression_contract(
    domains: &[DomainInfo],
    component: &ComponentInfo,
    current_line: usize,
    expression: &str,
    depth: usize,
) -> Option<ComponentSignalContract> {
    if depth > 8 {
        return None;
    }
    let trimmed_expression = strip_outer_parens(expression.trim());
    if let Some(contract) =
        component_named_signal_contract(domains, component, current_line, trimmed_expression)
    {
        return Some(contract);
    }
    if let Some(arguments) = behavior_call_arguments_expression(trimmed_expression, "delay") {
        let parts = split_top_level(&arguments, &[',']);
        if parts.len() != 2 || parse_duration_option_seconds(parts[1].trim()).is_none() {
            return None;
        }
        let signal_contract = component_behavior_expression_contract(
            domains,
            component,
            current_line,
            parts[0].trim(),
            depth + 1,
        )?;
        return Some(ComponentSignalContract {
            quantity_kind: signal_contract.quantity_kind,
            display_unit: signal_contract.display_unit,
            canonical_unit: signal_contract.canonical_unit,
            status: "delay_output_matches_signal".to_owned(),
        });
    }
    None
}

fn component_named_signal_contract(
    domains: &[DomainInfo],
    component: &ComponentInfo,
    current_line: usize,
    signal: &str,
) -> Option<ComponentSignalContract> {
    if let Some(variable) = component_signal_type(domains, component, signal) {
        return Some(domain_variable_signal_contract(
            variable,
            "domain_signal_resolved",
        ));
    }
    let trimmed_signal = signal.trim();
    if !is_identifier(trimmed_signal) {
        return None;
    }
    let local = component
        .local_expressions
        .iter()
        .find(|local| local.name == trimmed_signal && local.line < current_line)?;
    if local.quantity_kind == "unknown" || local.type_status == "signal_contract_unresolved" {
        return None;
    }
    Some(ComponentSignalContract {
        quantity_kind: local.quantity_kind.clone(),
        display_unit: local.display_unit.clone(),
        canonical_unit: local.canonical_unit.clone(),
        status: "component_local_signal_resolved".to_owned(),
    })
}

fn behavior_call_arguments_expression(expression: &str, call_name: &str) -> Option<String> {
    let trimmed = strip_outer_parens(expression.trim());
    let lowered = trimmed.to_ascii_lowercase();
    let prefix = format!("{call_name}(");
    if !lowered.starts_with(&prefix) || !trimmed.ends_with(')') {
        return None;
    }
    let inner = &trimmed[call_name.len() + 1..trimmed.len() - 1];
    is_balanced(inner).then(|| inner.to_owned())
}

fn domain_variable_signal_contract(
    variable: &DomainVariableInfo,
    status: &str,
) -> ComponentSignalContract {
    ComponentSignalContract {
        quantity_kind: variable.quantity_kind.clone(),
        display_unit: variable.display_unit.clone(),
        canonical_unit: variable.canonical_unit.clone(),
        status: status.to_owned(),
    }
}

fn unknown_component_signal_contract() -> ComponentSignalContract {
    ComponentSignalContract {
        quantity_kind: "unknown".to_owned(),
        display_unit: "unknown".to_owned(),
        canonical_unit: "unknown".to_owned(),
        status: "signal_contract_unresolved".to_owned(),
    }
}

fn component_signal_type<'a>(
    domains: &'a [DomainInfo],
    component: &ComponentInfo,
    signal: &str,
) -> Option<&'a DomainVariableInfo> {
    let (port_name, variable_name) = signal.split_once('.')?;
    if variable_name.contains('.') {
        return None;
    }
    let port = component
        .ports
        .iter()
        .find(|port| port.name == port_name.trim())?;
    let domain = domains
        .iter()
        .find(|domain| domain.name == port.domain_name)?;
    domain
        .variables
        .iter()
        .find(|variable| variable.name == variable_name.trim())
}

fn extract_call_arguments(expression: &str, call_name: &str) -> Vec<String> {
    let mut arguments = Vec::new();
    let lowered = expression.to_ascii_lowercase();
    let needle = format!("{call_name}(");
    let mut cursor = 0usize;
    while let Some(relative_start) = lowered[cursor..].find(&needle) {
        let call_start = cursor + relative_start;
        if call_start > 0 {
            let previous = lowered[..call_start].chars().next_back();
            if previous.is_some_and(|character| {
                character.is_ascii_alphanumeric() || character == '_' || character == '.'
            }) {
                cursor = call_start + needle.len();
                continue;
            }
        }
        let open_index = call_start + call_name.len();
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
        if let Some(close_index) = close_index {
            arguments.push(expression[open_index + 1..close_index].to_owned());
            cursor = close_index + 1;
        } else {
            arguments.push(expression[open_index + 1..].to_owned());
            break;
        }
    }
    arguments
}

fn component_generated_equation_reason(kind: &str) -> String {
    match kind {
        "across_equality" => {
            "generated from across variable equality within a connection set".to_owned()
        }
        "through_conservation" => {
            "generated from through variable conservation within a connection set".to_owned()
        }
        _ => "generated from component assembly metadata".to_owned(),
    }
}

fn component_boundary_equations(
    domains: &[DomainInfo],
    components: &[ComponentInfo],
    connection_sets: &[ComponentConnectionSetInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<ComponentAssemblyEquationInfo> {
    let connected_variables = connection_sets
        .iter()
        .flat_map(|connection_set| {
            connection_set
                .ports
                .iter()
                .flat_map(|port| {
                    domains
                        .iter()
                        .find(|domain| connection_set.domain.starts_with(domain.name.as_str()))
                        .map(|domain| {
                            domain
                                .variables
                                .iter()
                                .map(|variable| format!("{port}.{}", variable.name))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
        })
        .collect::<HashSet<_>>();
    let mut equations = Vec::new();
    for component in components {
        let parameter_refs = component_parameter_refs(component);
        for local in &component.local_expressions {
            let Some((signal, rhs)) = local.expression.split_once('=') else {
                continue;
            };
            let signal = signal.trim();
            let rhs = rhs.trim();
            let Some((port_name, variable_name)) = signal.split_once('.') else {
                continue;
            };
            let port_name = port_name.trim();
            let variable_name = variable_name.trim();
            let Some(port) = component
                .ports
                .iter()
                .find(|port| port.name == port_name && port.status == "domain_resolved")
            else {
                diagnostics.push(Diagnostic::error(
                    "E-ASSEMBLY-BOUNDARY-SIGNAL-001",
                    local.line,
                    &format!(
                        "Component boundary expression `{signal}` is not a known port signal in `{}`.",
                        component.name
                    ),
                    Some(
                        "Use `name = port.variable = literal` with a declared component port and domain variable.",
                    ),
                ));
                continue;
            };
            let Some(domain) = domains
                .iter()
                .find(|domain| domain.name == port.domain_name)
            else {
                continue;
            };
            let Some(domain_variable) = domain
                .variables
                .iter()
                .find(|variable| variable.name == variable_name)
            else {
                diagnostics.push(Diagnostic::error(
                    "E-ASSEMBLY-BOUNDARY-SIGNAL-001",
                    local.line,
                    &format!(
                        "Component boundary expression `{signal}` is not a known domain variable on `{}`.",
                        port.domain
                    ),
                    Some(
                        "Use a variable declared by the connected port domain, such as `heat.T`.",
                    ),
                ));
                continue;
            };
            let variable = format!("{}.{}.{}", component.name, port_name, variable_name);
            if !connected_variables.contains(&variable) {
                continue;
            }
            if let Some((_value, unit)) = numeric_literal_with_optional_unit(rhs) {
                if let Some(unit) = unit {
                    if !unit_compatible_with_quantity(&domain_variable.quantity_kind, &unit) {
                        diagnostics.push(Diagnostic::error(
                            "E-ASSEMBLY-BOUNDARY-UNIT-001",
                            local.line,
                            &format!(
                                "Component boundary RHS unit `{unit}` is not compatible with `{}`.",
                                domain_variable.quantity_kind
                            ),
                            Some("Use a unit compatible with the connected port signal quantity."),
                        ));
                        continue;
                    }
                }
                equations.push(ComponentAssemblyEquationInfo {
                    name: format!("{}.boundary_{}", component.name, local.name),
                    kind: "component_boundary".to_owned(),
                    domain: port.domain.clone(),
                    expression: format!("{variable} eq {rhs}"),
                    residual: format!("{variable} - ({rhs})"),
                    rhs: Some(rhs.to_owned()),
                    reason: "component-local boundary equation seed".to_owned(),
                    dependencies: vec![variable],
                    status: "component_boundary_seed".to_owned(),
                    line: local.line,
                });
                continue;
            }
            if let Some(parameter) = parameter_refs
                .iter()
                .find(|parameter| parameter.local == rhs)
            {
                if !component_parameter_compatible_with_quantity(
                    parameter,
                    &domain_variable.quantity_kind,
                ) {
                    diagnostics.push(Diagnostic::error(
                        "E-ASSEMBLY-BOUNDARY-UNIT-001",
                        local.line,
                        &format!(
                            "Component boundary parameter `{rhs}` is not compatible with `{}`.",
                            domain_variable.quantity_kind
                        ),
                        Some("Use a component parameter with the same physical dimension as the connected port signal."),
                    ));
                    continue;
                }
                equations.push(ComponentAssemblyEquationInfo {
                    name: format!("{}.boundary_{}", component.name, local.name),
                    kind: "component_boundary".to_owned(),
                    domain: port.domain.clone(),
                    expression: format!("{variable} eq {}", parameter.qualified),
                    residual: format!("{variable} - {}", parameter.qualified),
                    rhs: None,
                    reason: "component-local boundary equation seed".to_owned(),
                    dependencies: vec![variable, parameter.qualified.clone()],
                    status: "component_boundary_seed".to_owned(),
                    line: local.line,
                });
                continue;
            }
            diagnostics.push(Diagnostic::error(
                "E-ASSEMBLY-BOUNDARY-RHS-001",
                local.line,
                &format!("Component boundary RHS `{rhs}` is not a numeric literal or declared parameter."),
                Some("Use a numeric literal such as `22 degC` or a declared component parameter such as `T_room`."),
            ));
        }
    }
    equations
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ComponentSignalRef {
    local: String,
    qualified: String,
    domain: String,
    quantity_kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ComponentParameterRef {
    local: String,
    qualified: String,
    quantity_kind: String,
    dimension: String,
}

fn component_local_equations(
    domains: &[DomainInfo],
    components: &[ComponentInfo],
    connection_sets: &[ComponentConnectionSetInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<ComponentAssemblyEquationInfo> {
    let connected_variables = connected_component_variables(domains, connection_sets);
    let mut equations = Vec::new();
    for component in components {
        let signal_refs = component_signal_refs(domains, component);
        let parameter_refs = component_parameter_refs(component);
        for local in &component.local_expressions {
            if local.status != "component_equation_seed" {
                continue;
            }
            let Some((left, right)) = local.expression.split_once(" eq ") else {
                continue;
            };
            let left = left.trim();
            let right = right.trim();
            let expression = format!("{left} eq {right}");
            let local_expression = format!("{left} {right}");
            let unknown_signals =
                unknown_component_equation_signals(&local_expression, &signal_refs);
            if !unknown_signals.is_empty() {
                let unknown_signal_labels = unknown_signals
                    .iter()
                    .map(|signal| format!("`{signal}`"))
                    .collect::<Vec<_>>()
                    .join(", ");
                diagnostics.push(Diagnostic::error(
                    "E-COMPONENT-EQUATION-SIGNAL-001",
                    local.line,
                    &format!(
                        "Component equation references unknown signal(s) {unknown_signal_labels} in `{expression}`."
                    ),
                    Some("Use declared component port signals such as `heat.T` or `heat.Q`."),
                ));
                continue;
            }
            let mut dependencies = signal_refs
                .iter()
                .filter(|signal| {
                    expression_mentions_component_signal(&local_expression, &signal.local)
                })
                .filter(|signal| connected_variables.contains(&signal.qualified))
                .map(|signal| signal.qualified.clone())
                .collect::<Vec<_>>();
            dependencies.extend(signal_refs.iter().filter_map(|signal| {
                if !expression_mentions_component_derivative(&local_expression, &signal.local) {
                    return None;
                }
                if !connected_variables.contains(&signal.qualified) {
                    return None;
                }
                Some(format!("der({})", signal.qualified))
            }));
            dependencies.extend(parameter_refs.iter().filter_map(|parameter| {
                if expression_mentions_component_parameter(&local_expression, &parameter.local) {
                    Some(parameter.qualified.clone())
                } else {
                    None
                }
            }));
            sort_component_equation_dependencies(&mut dependencies, &parameter_refs);
            if dependencies.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "E-COMPONENT-EQUATION-SIGNAL-001",
                    local.line,
                    &format!("Component equation `{expression}` has no connected port signal."),
                    Some(
                        "Reference a signal from a port that participates in the component graph.",
                    ),
                ));
                continue;
            }
            if numeric_literal_with_optional_unit(right).is_some()
                && signal_refs.iter().any(|signal| signal.local == left)
            {
                if let Some(rhs_equation) = component_equation_literal_rhs(
                    component,
                    local,
                    left,
                    right,
                    &signal_refs,
                    &connected_variables,
                    diagnostics,
                ) {
                    equations.push(rhs_equation);
                }
                continue;
            }
            let is_dynamic_equation = local_expression.contains("der(");
            if !is_dynamic_equation {
                let dimension_symbols =
                    component_equation_dimension_symbols(&signal_refs, &parameter_refs);
                if let Some(function_error) =
                    dimensionless_math_function_dimension_error(left, &dimension_symbols).or_else(
                        || dimensionless_math_function_dimension_error(right, &dimension_symbols),
                    )
                {
                    diagnostics.push(Diagnostic::error(
                        "E-COMPONENT-EQUATION-UNIT-001",
                        local.line,
                        &format!(
                            "Component equation `{expression}` calls `{}` with non-dimensionless argument `{}` ({}).",
                            function_error.function,
                            function_error.argument,
                            function_error.argument_dimension
                        ),
                        Some("Use sqrt, exp, ln, sin, and cos only with DimensionlessNumber expressions, or nondimensionalize the argument explicitly."),
                    ));
                    continue;
                }
                let left_dimension = expression_dimension_with_symbols(left, &dimension_symbols);
                let right_dimension = expression_dimension_with_symbols(right, &dimension_symbols);
                if let (Some(left_dimension), Some(right_dimension)) =
                    (left_dimension.as_deref(), right_dimension.as_deref())
                {
                    if !component_equation_dimensions_compatible(
                        left_dimension,
                        right_dimension,
                        left,
                        right,
                    ) {
                        diagnostics.push(Diagnostic::error(
                            "E-COMPONENT-EQUATION-UNIT-001",
                            local.line,
                            &format!(
                                "Component equation `{expression}` has incompatible dimensions `{left_dimension}` and `{right_dimension}`."
                            ),
                            Some("Make both sides reduce to compatible units, such as HeatRate = Conductance * TemperatureDelta."),
                        ));
                        continue;
                    }
                } else {
                    let mut dependency_kinds = signal_refs
                        .iter()
                        .filter(|signal| dependencies.contains(&signal.qualified))
                        .map(|signal| signal.quantity_kind.clone())
                        .collect::<HashSet<_>>();
                    dependency_kinds.extend(
                        parameter_refs
                            .iter()
                            .filter(|parameter| dependencies.contains(&parameter.qualified))
                            .map(|parameter| parameter.quantity_kind.clone()),
                    );
                    if dependency_kinds.len() > 1 {
                        diagnostics.push(Diagnostic::error(
                            "E-COMPONENT-EQUATION-UNIT-001",
                            local.line,
                            &format!("Component equation `{expression}` mixes incompatible signal quantities."),
                            Some("Use unit-compatible arithmetic, or keep unsupported expressions to one signal quantity kind."),
                        ));
                        continue;
                    }
                    if let Some(quantity_kind) = dependency_kinds.iter().next() {
                        let incompatible_units =
                            numeric_units_in_component_expression(&local_expression)
                                .into_iter()
                                .filter(|unit| !unit_compatible_with_quantity(quantity_kind, unit))
                                .collect::<Vec<_>>();
                        if !incompatible_units.is_empty() {
                            diagnostics.push(Diagnostic::error(
                                "E-COMPONENT-EQUATION-UNIT-001",
                                local.line,
                                &format!(
                                    "Component equation `{expression}` uses unit(s) incompatible with `{quantity_kind}`: {}.",
                                    incompatible_units.join(", ")
                                ),
                                Some("Use numeric constants with units compatible with the referenced port signal quantity."),
                            ));
                            continue;
                        }
                    }
                }
            }
            let qualified_left =
                qualify_component_equation_expression(left, &signal_refs, &parameter_refs);
            let qualified_right =
                qualify_component_equation_expression(right, &signal_refs, &parameter_refs);
            equations.push(ComponentAssemblyEquationInfo {
                name: format!("{}.{}", component.name, local.name),
                kind: "component_equation".to_owned(),
                domain: component_equation_domain(&dependencies, &signal_refs),
                expression: format!("{qualified_left} eq {qualified_right}"),
                residual: format!("{qualified_left} - ({qualified_right})"),
                rhs: None,
                reason: "component-local equation seed".to_owned(),
                dependencies,
                status: "component_equation_seed".to_owned(),
                line: local.line,
            });
        }
    }
    equations
}

fn classify_dynamic_component_states(
    variables: &mut [ComponentAssemblyVariableInfo],
    equations: &[ComponentAssemblyEquationInfo],
) {
    let states = equations
        .iter()
        .flat_map(|equation| equation.dependencies.iter())
        .filter_map(|dependency| derivative_dependency_signal(dependency))
        .collect::<HashSet<_>>();
    if states.is_empty() {
        return;
    }
    for variable in variables {
        if states.contains(variable.name.as_str()) {
            variable.role = "state".to_owned();
        }
    }
}

fn derivative_dependency_signal(dependency: &str) -> Option<&str> {
    dependency
        .trim()
        .strip_prefix("der(")
        .and_then(|value| value.strip_suffix(')'))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn connected_component_variables(
    domains: &[DomainInfo],
    connection_sets: &[ComponentConnectionSetInfo],
) -> HashSet<String> {
    connection_sets
        .iter()
        .flat_map(|connection_set| {
            connection_set
                .ports
                .iter()
                .flat_map(|port| {
                    domains
                        .iter()
                        .find(|domain| connection_set.domain.starts_with(domain.name.as_str()))
                        .map(|domain| {
                            domain
                                .variables
                                .iter()
                                .map(|variable| format!("{port}.{}", variable.name))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn component_signal_refs(
    domains: &[DomainInfo],
    component: &ComponentInfo,
) -> Vec<ComponentSignalRef> {
    component
        .ports
        .iter()
        .filter_map(|port| {
            domains
                .iter()
                .find(|domain| domain.name == port.domain_name)
                .map(|domain| {
                    domain
                        .variables
                        .iter()
                        .map(|variable| ComponentSignalRef {
                            local: format!("{}.{}", port.name, variable.name),
                            qualified: format!(
                                "{}.{}.{}",
                                component.name, port.name, variable.name
                            ),
                            domain: port.domain.clone(),
                            quantity_kind: variable.quantity_kind.clone(),
                        })
                        .collect::<Vec<_>>()
                })
        })
        .flatten()
        .collect()
}

fn component_parameter_refs(component: &ComponentInfo) -> Vec<ComponentParameterRef> {
    component
        .parameters
        .iter()
        .chain(component.inputs.iter())
        .map(|parameter| ComponentParameterRef {
            local: parameter.name.clone(),
            qualified: format!("{}.{}", component.name, parameter.name),
            quantity_kind: parameter.quantity_kind.clone(),
            dimension: parameter.dimension.clone(),
        })
        .collect()
}

fn component_parameter_compatible_with_quantity(
    parameter: &ComponentParameterRef,
    quantity_kind: &str,
) -> bool {
    parameter.quantity_kind == quantity_kind
        || parameter.dimension == dimension_for_quantity(quantity_kind)
}

fn sort_component_equation_dependencies(
    dependencies: &mut Vec<String>,
    parameter_refs: &[ComponentParameterRef],
) {
    dependencies.sort_by(|left, right| {
        let left_is_parameter = parameter_refs
            .iter()
            .any(|parameter| parameter.qualified == *left);
        let right_is_parameter = parameter_refs
            .iter()
            .any(|parameter| parameter.qualified == *right);
        left_is_parameter
            .cmp(&right_is_parameter)
            .then(left.cmp(right))
    });
    dependencies.dedup();
}
fn component_equation_dimension_symbols(
    signal_refs: &[ComponentSignalRef],
    parameter_refs: &[ComponentParameterRef],
) -> Vec<DimensionSymbol> {
    signal_refs
        .iter()
        .map(|signal| DimensionSymbol {
            name: signal.local.clone(),
            dimension: dimension_for_quantity(&signal.quantity_kind),
        })
        .chain(parameter_refs.iter().map(|parameter| DimensionSymbol {
            name: parameter.local.clone(),
            dimension: parameter.dimension.clone(),
        }))
        .collect()
}

fn component_equation_dimensions_compatible(
    left_dimension: &str,
    right_dimension: &str,
    left_expression: &str,
    right_expression: &str,
) -> bool {
    dimensions_compatible(left_dimension, right_dimension)
        || left_dimension == "Dimensionless" && unitless_zero_literal(left_expression)
        || right_dimension == "Dimensionless" && unitless_zero_literal(right_expression)
}

fn unitless_zero_literal(expression: &str) -> bool {
    numeric_literal_with_optional_unit(strip_outer_parens(expression.trim()))
        .is_some_and(|(value, unit)| unit.is_none() && value.abs() <= f64::EPSILON)
}
fn component_equation_literal_rhs(
    component: &ComponentInfo,
    local: &ComponentLocalExpressionInfo,
    left: &str,
    right: &str,
    signal_refs: &[ComponentSignalRef],
    connected_variables: &HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ComponentAssemblyEquationInfo> {
    let signal = signal_refs
        .iter()
        .find(|signal| signal.local == left.trim())?;
    if !connected_variables.contains(&signal.qualified) {
        diagnostics.push(Diagnostic::error(
            "E-COMPONENT-EQUATION-SIGNAL-001",
            local.line,
            &format!(
                "Component equation references unconnected signal `{left}` in `{}`.",
                local.expression
            ),
            Some("Connect the port before solving a component-local equation over that signal."),
        ));
        return None;
    }
    let Some((_value, unit)) = numeric_literal_with_optional_unit(right) else {
        return None;
    };
    if let Some(unit) = unit {
        if !unit_compatible_with_quantity(&signal.quantity_kind, &unit) {
            diagnostics.push(Diagnostic::error(
                "E-COMPONENT-EQUATION-UNIT-001",
                local.line,
                &format!(
                    "Component equation RHS unit `{unit}` is not compatible with `{}`.",
                    signal.quantity_kind
                ),
                Some("Use a unit compatible with the connected port signal quantity."),
            ));
            return None;
        }
    }
    Some(ComponentAssemblyEquationInfo {
        name: format!("{}.{}", component.name, local.name),
        kind: "component_equation".to_owned(),
        domain: signal.domain.clone(),
        expression: format!("{} eq {right}", signal.qualified),
        residual: signal.qualified.clone(),
        rhs: Some(right.to_owned()),
        reason: "component-local equation seed".to_owned(),
        dependencies: vec![signal.qualified.clone()],
        status: "component_equation_seed".to_owned(),
        line: local.line,
    })
}

fn component_equation_domain(
    dependencies: &[String],
    signal_refs: &[ComponentSignalRef],
) -> String {
    dependencies
        .iter()
        .find_map(|dependency| {
            signal_refs
                .iter()
                .find(|signal| &signal.qualified == dependency)
                .map(|signal| signal.domain.clone())
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

fn qualify_component_equation_expression(
    expression: &str,
    signal_refs: &[ComponentSignalRef],
    parameter_refs: &[ComponentParameterRef],
) -> String {
    let mut output = String::new();
    let mut token = String::new();
    let flush_token = |token: &mut String, output: &mut String| {
        if token.is_empty() {
            return;
        }
        if let Some(signal) = signal_refs.iter().find(|signal| signal.local == *token) {
            output.push_str(&signal.qualified);
        } else if let Some(parameter) = parameter_refs
            .iter()
            .find(|parameter| parameter.local == *token)
        {
            output.push_str(&parameter.qualified);
        } else {
            output.push_str(token);
        }
        token.clear();
    };
    for character in expression.chars() {
        if character.is_ascii_alphanumeric() || character == '_' || character == '.' {
            token.push(character);
        } else {
            flush_token(&mut token, &mut output);
            output.push(character);
        }
    }
    flush_token(&mut token, &mut output);
    output
}

fn unknown_component_equation_signals(
    expression: &str,
    signal_refs: &[ComponentSignalRef],
) -> Vec<String> {
    let mut signals = expression
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || character == '_' || character == '.')
        })
        .filter(|token| token.contains('.'))
        .filter(|token| {
            token
                .chars()
                .any(|character| character.is_ascii_alphabetic())
        })
        .filter(|token| !signal_refs.iter().any(|signal| signal.local == *token))
        .map(str::to_owned)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    signals.sort();
    signals
}

fn numeric_units_in_component_expression(expression: &str) -> Vec<String> {
    let normalized = expression
        .chars()
        .map(|character| match character {
            '(' | ')' | ',' | ';' | '[' | ']' | '{' | '}' | '"' | '\'' | '=' | '+' | '-' | '*'
            | ':' => ' ',
            other => other,
        })
        .collect::<String>();
    let words = normalized
        .split_whitespace()
        .map(trim_component_expression_punctuation)
        .collect::<Vec<_>>();
    let mut units = Vec::new();
    for pair in words.windows(2) {
        let [number, unit] = pair else {
            continue;
        };
        if is_number_literal(number) && unit_is_supported(unit) {
            units.push((*unit).to_owned());
        }
    }
    units.sort();
    units.dedup();
    units
}

fn trim_component_expression_punctuation(value: &str) -> &str {
    value.trim_matches(|character: char| {
        matches!(
            character,
            ',' | ';' | ')' | '(' | ']' | '[' | '{' | '}' | '"' | '\''
        )
    })
}

fn expression_mentions_component_signal(expression: &str, signal: &str) -> bool {
    expression
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || character == '_' || character == '.')
        })
        .any(|token| token == signal)
}

fn expression_mentions_component_derivative(expression: &str, signal: &str) -> bool {
    expression.contains(&format!("der({signal})"))
}

fn expression_mentions_component_parameter(expression: &str, parameter: &str) -> bool {
    expression
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || character == '_' || character == '.')
        })
        .any(|token| token == parameter)
}

fn build_component_solver_preview(
    domain_count: usize,
    state_count: usize,
    equation_count: usize,
    predictor_call_count: usize,
    delay_call_count: usize,
    external_call_count: usize,
) -> ComponentSolverPreviewInfo {
    let status = if equation_count == 0 {
        "no_numeric_preview"
    } else if domain_count > 1 {
        "multi_domain_preview"
    } else {
        "single_domain_preview"
    };
    ComponentSolverPreviewInfo {
        status: status.to_owned(),
        method: if equation_count == 0 {
            "metadata_only"
        } else {
            "homogeneous_connection_constraint_preview"
        }
        .to_owned(),
        mixed_algebraic_dynamic: if state_count > 0 {
            "mixed_state_algebraic_seed"
        } else {
            "algebraic_only_seed"
        }
        .to_owned(),
        nonlinear_residual: "symbolic_residual_seed_no_nonlinear_iteration".to_owned(),
        dae_split: if state_count > 0 {
            "dae_split_seed_deferred"
        } else {
            "algebraic_split_seed"
        }
        .to_owned(),
        delay_history: if delay_call_count > 0 {
            "delay_call_runtime_buffer_seed_not_integrated"
        } else {
            "deferred_no_delay_calls"
        }
        .to_owned(),
        predictor: if predictor_call_count > 0 {
            "predictor_call_contract_seed_not_integrated"
        } else {
            "deferred_no_predictor_calls"
        }
        .to_owned(),
        external_adapter: if external_call_count > 0 {
            "external_behavior_wrapper_seed_not_integrated"
        } else {
            "deferred_no_external_behavior_adapter"
        }
        .to_owned(),
        limitations: vec![
            "not_full_dae".to_owned(),
            "not_general_nonlinear".to_owned(),
            "not_adaptive".to_owned(),
            "not_production_multi_domain".to_owned(),
            "no_jit_speed_claim".to_owned(),
        ],
    }
}

fn normalized_connection_key(
    left_component: &str,
    left_port: &str,
    right_component: &str,
    right_port: &str,
) -> String {
    let left = format!("{left_component}.{left_port}");
    let right = format!("{right_component}.{right_port}");
    if left <= right {
        format!("{left}->{right}")
    } else {
        format!("{right}->{left}")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DomainReference {
    name: String,
    type_arguments: Vec<String>,
}

impl DomainReference {
    fn canonical(&self) -> String {
        domain_signature(&self.name, &self.type_arguments)
    }
}

fn parse_domain_reference(raw: &str) -> DomainReference {
    let trimmed = raw.split("//").next().unwrap_or(raw).trim();
    let Some((name, rest)) = trimmed.split_once('[') else {
        return DomainReference {
            name: trimmed.to_owned(),
            type_arguments: Vec::new(),
        };
    };
    let arguments = rest
        .split_once(']')
        .map(|(inside, _)| inside)
        .unwrap_or(rest)
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    DomainReference {
        name: name.trim().to_owned(),
        type_arguments: arguments,
    }
}

fn domain_signature(name: &str, arguments: &[String]) -> String {
    if arguments.is_empty() {
        name.to_owned()
    } else {
        format!("{name}[{}]", arguments.join(", "))
    }
}

fn collect_state_space_type_blocks(program: &ParsedProgram) -> Vec<StateSpaceTypeBlockInfo> {
    let mut blocks = Vec::new();
    let mut current_block_index = None;
    for item in &program.items {
        match item {
            AstItem::StateSpaceTypeBlock(block) => {
                blocks.push(state_space_type_block_info(block));
                current_block_index = Some(blocks.len() - 1);
            }
            AstItem::StateSpaceTypeMember(member) => {
                if let Some(index) = current_block_index {
                    blocks[index]
                        .members
                        .push(state_space_type_member_info(member));
                }
            }
            _ => {}
        }
    }
    blocks
}

fn state_space_type_block_info(block: &StateSpaceTypeBlockDecl) -> StateSpaceTypeBlockInfo {
    StateSpaceTypeBlockInfo {
        role: block.role.clone(),
        name: block.name.clone(),
        members: Vec::new(),
    }
}

fn state_space_type_member_info(member: &StateSpaceTypeMemberDecl) -> StateSpaceTypeMemberInfo {
    StateSpaceTypeMemberInfo {
        name: member.name.clone(),
        type_name: member.type_name.clone(),
        unit: member.unit.clone(),
        line: member.line,
        span: member.span,
    }
}

fn resolved_port<'a>(
    components: &'a [ComponentInfo],
    component_name: &str,
    port_name: &str,
) -> Option<&'a PortInfo> {
    components
        .iter()
        .find(|component| component.name == component_name)?
        .ports
        .iter()
        .find(|port| port.name == port_name && port.status == "domain_resolved")
}

fn first_mismatched_parameter(
    domains: &[DomainInfo],
    domain_name: &str,
    left: &PortInfo,
    right: &PortInfo,
) -> Option<String> {
    let domain = domains.iter().find(|domain| domain.name == domain_name)?;
    left.type_arguments
        .iter()
        .zip(&right.type_arguments)
        .enumerate()
        .find_map(|(index, (left, right))| {
            (left != right).then(|| {
                domain
                    .type_parameters
                    .get(index)
                    .map(|parameter| parameter.kind.clone())
                    .unwrap_or_else(|| "Parameter".to_owned())
            })
        })
}

fn parameter_mismatch_diagnostic(
    parameter_name: &str,
) -> (&'static str, &'static str, &'static str) {
    match parameter_name {
        "Medium" => ("E-CONNECT-MEDIUM-MISMATCH", "medium_mismatch", "medium"),
        "Frame" => ("E-CONNECT-FRAME-001", "frame_mismatch", "frame"),
        "Axis" => ("E-CONNECT-AXIS-001", "axis_mismatch", "axis"),
        _ => (
            "E-CONNECT-DOMAIN-PARAM-001",
            "domain_parameter_mismatch",
            "domain parameter",
        ),
    }
}

fn split_endpoint(endpoint: &str) -> Option<(String, String)> {
    let (component, port) = endpoint.split_once('.')?;
    let component = component.trim();
    let port = port.trim();
    if component.is_empty() || port.is_empty() {
        return None;
    }
    Some((component.to_owned(), port.to_owned()))
}

fn connection_endpoint_diagnostic(endpoint: &str, line: usize) -> Diagnostic {
    Diagnostic::error(
        "E-CONNECT-ENDPOINT-001",
        line,
        &format!("Connection endpoint `{endpoint}` is not a component port path."),
        Some("Use `Component.port` on both sides of `connect ... -> ...`."),
    )
}

#[allow(clippy::too_many_arguments)]
fn analyze_const_decl(
    declaration: &ConstDecl,
    consts: &mut Vec<ConstInfo>,
    diagnostics: &mut Vec<Diagnostic>,
    expected_types: &mut Vec<ExpectedType>,
    hover_hints: &mut Vec<HoverHint>,
    typed_bindings: &mut Vec<TypedBinding>,
    type_infos: &mut Vec<TypeInfo>,
    unit_derivations: &mut Vec<UnitDerivation>,
) {
    if expression_mentions_args(&declaration.expression) {
        diagnostics.push(Diagnostic::error(
            "E-CONST-ARGS-001",
            declaration.line,
            &format!("const `{}` depends on args.", declaration.name),
            Some("Args belong to the root execution context and are not imported."),
        ));
    }
    if expression_has_side_effect(&declaration.expression) {
        diagnostics.push(Diagnostic::error(
            "E-CONST-SIDE-EFFECT-001",
            declaration.line,
            "const expressions must not perform side-effecting operations.",
            Some("Use top-level executable code or args default instead."),
        ));
    }
    if expression_depends_on_runtime(&declaration.expression) {
        diagnostics.push(Diagnostic::warning(
            "W-CONST-RUNTIME-001",
            declaration.line,
            &format!(
                "const `{}` depends on runtime environment/time/current directory.",
                declaration.name
            ),
            Some("Prefer an args default for environment- or time-dependent values."),
        ));
    }

    let display_unit = declaration
        .unit
        .clone()
        .unwrap_or_else(|| default_unit_for_type(&declaration.type_name));
    let canonical_unit = default_unit_for_type(&declaration.type_name);
    let dimension = dimension_for_type(&declaration.type_name);
    let importable = declaration.context == ParseContext::TopLevel
        && !expression_mentions_args(&declaration.expression)
        && !expression_has_side_effect(&declaration.expression);

    expected_types.push(ExpectedType {
        name: declaration.name.clone(),
        quantity_kind: declaration.type_name.clone(),
        display_unit: Some(display_unit.clone()),
        source: ExpectedTypeSource::ExplicitAnnotation,
        line: declaration.line,
        span: declaration.span,
    });
    typed_bindings.push(TypedBinding {
        name: declaration.name.clone(),
        semantic_type: SemanticType {
            quantity_kind: declaration.type_name.clone(),
            display_unit: display_unit.clone(),
        },
        line: declaration.line,
    });
    hover_hints.push(HoverHint::importable_const(
        declaration.name.clone(),
        declaration.type_name.clone(),
        display_unit.clone(),
        declaration.expression.clone(),
        declaration.span,
    ));
    type_infos.push(TypeInfo {
        name: declaration.name.clone(),
        quantity_kind: declaration.type_name.clone(),
        display_unit: display_unit.clone(),
        canonical_unit: canonical_unit.clone(),
        dimension: dimension.clone(),
        source: TypeInfoSource::Const,
        line: declaration.line,
        span: declaration.span,
    });
    unit_derivations.push(unit_derivation(
        &declaration.name,
        Some(&declaration.expression),
        &declaration.type_name,
        &display_unit,
        &canonical_unit,
        declaration.line,
    ));
    consts.push(ConstInfo {
        name: declaration.name.clone(),
        type_name: declaration.type_name.clone(),
        quantity_kind: declaration.type_name.clone(),
        display_unit,
        canonical_unit,
        dimension,
        expression: declaration.expression.clone(),
        importable,
        line: declaration.line,
    });
}

fn analyze_explicit_decl(
    declaration: &ExplicitDecl,
    diagnostics: &mut Vec<Diagnostic>,
    expected_types: &mut Vec<ExpectedType>,
    hover_hints: &mut Vec<HoverHint>,
    typed_bindings: &mut Vec<TypedBinding>,
    type_infos: &mut Vec<TypeInfo>,
    unit_derivations: &mut Vec<UnitDerivation>,
    inferred_declarations: &mut Vec<InferredDeclaration>,
) {
    expected_types.push(expected_type_from_explicit_decl(declaration));

    if let Some(expression) = &declaration.expression {
        check_dimensionless_operation(expression, declaration.line, diagnostics);
    }

    let display_unit = declaration
        .unit
        .clone()
        .unwrap_or_else(|| default_unit_for_quantity(&declaration.type_name));
    let canonical_unit = default_unit_for_quantity(&declaration.type_name);
    let dimension = dimension_for_quantity(&declaration.type_name);
    typed_bindings.push(TypedBinding {
        name: declaration.name.clone(),
        semantic_type: SemanticType {
            quantity_kind: declaration.type_name.clone(),
            display_unit: display_unit.clone(),
        },
        line: declaration.line,
    });
    hover_hints.push(HoverHint::explicit(
        declaration.name.clone(),
        declaration.type_name.clone(),
        display_unit.clone(),
        declaration.expression.clone(),
        declaration.span,
    ));
    type_infos.push(TypeInfo {
        name: declaration.name.clone(),
        quantity_kind: declaration.type_name.clone(),
        display_unit: display_unit.clone(),
        canonical_unit: canonical_unit.clone(),
        dimension,
        source: if declaration.context == ParseContext::Schema {
            TypeInfoSource::PublicBoundary
        } else {
            TypeInfoSource::Explicit
        },
        line: declaration.line,
        span: declaration.span,
    });
    unit_derivations.push(unit_derivation(
        &declaration.name,
        declaration.expression.as_deref(),
        &declaration.type_name,
        &display_unit,
        &canonical_unit,
        declaration.line,
    ));
    if declaration.context == ParseContext::TopLevel
        && declaration.expression.as_deref().is_some_and(|expression| {
            expression.trim_start().starts_with("select_first_row(")
                || is_simple_member_expression(expression)
        })
    {
        inferred_declarations.push(InferredDeclaration {
            name: declaration.name.clone(),
            quantity_kind: declaration.type_name.clone(),
            display_unit,
            expression: declaration.expression.clone().unwrap_or_default(),
            line: declaration.line,
        });
    }
}

fn is_simple_member_expression(expression: &str) -> bool {
    let Some((receiver, field)) = expression.trim().split_once('.') else {
        return false;
    };
    is_identifier(receiver.trim()) && is_identifier(field.trim())
}

fn analyze_system_variable(
    declaration: &SystemVariableDecl,
    system: &mut SystemInfo,
    expected_types: &mut Vec<ExpectedType>,
    hover_hints: &mut Vec<HoverHint>,
    typed_bindings: &mut Vec<TypedBinding>,
    type_infos: &mut Vec<TypeInfo>,
    unit_derivations: &mut Vec<UnitDerivation>,
) {
    let display_unit = declaration
        .unit
        .clone()
        .or_else(|| {
            declaration
                .expression
                .as_deref()
                .and_then(first_unit_in_expression)
        })
        .unwrap_or_else(|| default_unit_for_quantity(&declaration.type_name));
    let canonical_unit = default_unit_for_quantity(&declaration.type_name);
    let dimension = dimension_for_quantity(&declaration.type_name);

    expected_types.push(ExpectedType {
        name: declaration.name.clone(),
        quantity_kind: declaration.type_name.clone(),
        display_unit: Some(display_unit.clone()),
        source: ExpectedTypeSource::SystemBoundary,
        line: declaration.line,
        span: declaration.span,
    });
    typed_bindings.push(TypedBinding {
        name: declaration.name.clone(),
        semantic_type: SemanticType {
            quantity_kind: declaration.type_name.clone(),
            display_unit: display_unit.clone(),
        },
        line: declaration.line,
    });
    hover_hints.push(HoverHint::explicit(
        declaration.name.clone(),
        declaration.type_name.clone(),
        display_unit.clone(),
        declaration.expression.clone(),
        declaration.span,
    ));
    type_infos.push(TypeInfo {
        name: declaration.name.clone(),
        quantity_kind: declaration.type_name.clone(),
        display_unit: display_unit.clone(),
        canonical_unit: canonical_unit.clone(),
        dimension: dimension.clone(),
        source: TypeInfoSource::SystemBoundary,
        line: declaration.line,
        span: declaration.span,
    });
    unit_derivations.push(unit_derivation(
        &declaration.name,
        declaration.expression.as_deref(),
        &declaration.type_name,
        &display_unit,
        &canonical_unit,
        declaration.line,
    ));
    system.variables.push(SystemVariableInfo {
        role: declaration.role.clone(),
        name: declaration.name.clone(),
        quantity_kind: declaration.type_name.clone(),
        display_unit,
        canonical_unit,
        dimension,
        initial_value: declaration.expression.clone(),
        line: declaration.line,
    });
}

#[allow(clippy::too_many_arguments)]
fn analyze_typed_state_space_vector_variable(
    declaration: &SystemVariableDecl,
    vector_role: &str,
    type_name: &str,
    type_blocks: &[StateSpaceTypeBlockInfo],
    system: &mut SystemInfo,
    state_space_vectors: &mut Vec<StateSpaceVectorInfo>,
    type_aliases: &mut HashMap<(String, String), String>,
    diagnostics: &mut Vec<Diagnostic>,
    expected_types: &mut Vec<ExpectedType>,
    hover_hints: &mut Vec<HoverHint>,
    typed_bindings: &mut Vec<TypedBinding>,
    type_infos: &mut Vec<TypeInfo>,
    unit_derivations: &mut Vec<UnitDerivation>,
) {
    let Some(type_block) = type_blocks
        .iter()
        .find(|block| block.role == vector_role && block.name == type_name)
    else {
        diagnostics.push(Diagnostic::error(
            "E-STATE-SPACE-VECTOR-TYPE-001",
            declaration.line,
            &format!(
                "State-space vector `{}` references undeclared {} type `{}`.",
                declaration.name, vector_role, type_name
            ),
            Some("Declare a matching top-level `states Name { ... }` or `inputs Name { ... }` block before the system."),
        ));
        return;
    };

    let vector_type = state_space_vector_type(vector_role);
    type_aliases.insert(
        (system.name.clone(), type_name.to_owned()),
        vector_type.to_owned(),
    );
    let initial_values = vector_literal_values(declaration.expression.as_deref());
    if !initial_values.is_empty() && initial_values.len() != type_block.members.len() {
        diagnostics.push(Diagnostic::error(
            "E-STATE-SPACE-VECTOR-INIT-001",
            declaration.line,
            &format!(
                "State-space vector `{}` has {} initial value(s), expected {} for `{}`.",
                declaration.name,
                initial_values.len(),
                type_block.members.len(),
                type_name
            ),
            Some("Provide one initial value per member, for example `[20 degC, 19 degC]`."),
        ));
    }

    if let Some(scalar_role) = scalar_role_for_state_space_vector(vector_role) {
        for (index, member) in type_block.members.iter().enumerate() {
            let generated = SystemVariableDecl {
                role: scalar_role.to_owned(),
                name: member.name.clone(),
                type_name: member.type_name.clone(),
                unit: member.unit.clone(),
                expression: initial_values.get(index).cloned(),
                line: member.line,
                span: member.span,
                context: ParseContext::System,
            };
            analyze_system_variable(
                &generated,
                system,
                expected_types,
                hover_hints,
                typed_bindings,
                type_infos,
                unit_derivations,
            );
        }
    }

    let vector_decl = StateSpaceVectorDecl {
        role: vector_role.to_owned(),
        name: declaration.name.clone(),
        members: type_block
            .members
            .iter()
            .map(|member| member.name.clone())
            .collect(),
        line: declaration.line,
        span: declaration.span,
        context: ParseContext::System,
    };
    analyze_state_space_vector_decl(
        &vector_decl,
        &system.name,
        state_space_vectors,
        typed_bindings,
        hover_hints,
        type_infos,
    );
}

fn scalar_role_for_state_space_vector(vector_role: &str) -> Option<&'static str> {
    match vector_role {
        "states" => Some("state"),
        "inputs" => Some("input"),
        _ => None,
    }
}

fn state_space_vector_type_parameter(type_name: &str) -> Option<(&'static str, &str)> {
    let trimmed = type_name.trim();
    for (prefix, role) in [
        ("StateVector[", "states"),
        ("InputVector[", "inputs"),
        ("OutputVector[", "outputs"),
    ] {
        if let Some(inner) = trimmed
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_suffix(']'))
        {
            let inner = inner.trim();
            if !inner.is_empty() {
                return Some((role, inner));
            }
        }
    }
    None
}

fn vector_literal_values(expression: Option<&str>) -> Vec<String> {
    let Some(expression) = expression else {
        return Vec::new();
    };
    let trimmed = expression.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(trimmed)
        .trim();
    if inner.is_empty() {
        return Vec::new();
    }
    split_vector_literal_items(inner)
}

fn split_vector_literal_items(text: &str) -> Vec<String> {
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

fn analyze_state_space_vector_decl(
    declaration: &StateSpaceVectorDecl,
    system_name: &str,
    state_space_vectors: &mut Vec<StateSpaceVectorInfo>,
    typed_bindings: &mut Vec<TypedBinding>,
    hover_hints: &mut Vec<HoverHint>,
    type_infos: &mut Vec<TypeInfo>,
) {
    let vector_type = state_space_vector_type(&declaration.role);
    typed_bindings.push(TypedBinding {
        name: declaration.name.clone(),
        semantic_type: SemanticType {
            quantity_kind: vector_type.to_owned(),
            display_unit: "vector".to_owned(),
        },
        line: declaration.line,
    });
    hover_hints.push(HoverHint::explicit(
        declaration.name.clone(),
        vector_type.to_owned(),
        "vector".to_owned(),
        Some(format!("[{}]", declaration.members.join(", "))),
        declaration.span,
    ));
    type_infos.push(TypeInfo {
        name: declaration.name.clone(),
        quantity_kind: vector_type.to_owned(),
        display_unit: "vector".to_owned(),
        canonical_unit: "vector".to_owned(),
        dimension: "StateSpace".to_owned(),
        source: TypeInfoSource::SystemBoundary,
        line: declaration.line,
        span: declaration.span,
    });
    state_space_vectors.push(StateSpaceVectorInfo {
        system: system_name.to_owned(),
        role: declaration.role.clone(),
        name: declaration.name.clone(),
        vector_type: vector_type.to_owned(),
        members: declaration.members.clone(),
        status: if declaration.members.is_empty() {
            "empty".to_owned()
        } else {
            "recorded".to_owned()
        },
        line: declaration.line,
    });
}

fn state_space_vector_type(role: &str) -> &'static str {
    match role {
        "states" => "StateVector",
        "inputs" => "InputVector",
        "outputs" => "OutputVector",
        _ => "StateVector",
    }
}

fn analyze_linear_operator_decl(
    declaration: &ExplicitDecl,
    system_name: &str,
    type_aliases: &HashMap<(String, String), String>,
) -> Option<LinearOperatorInfo> {
    let (from, to) = parse_linear_operator_type(&declaration.type_name)?;
    let from = normalize_linear_operator_endpoint(&from, system_name, type_aliases);
    let to = normalize_linear_operator_endpoint(&to, system_name, type_aliases);
    let (row_count, column_count) = declaration
        .expression
        .as_deref()
        .map(matrix_shape)
        .unwrap_or((0, 0));
    Some(LinearOperatorInfo {
        system: system_name.to_owned(),
        name: declaration.name.clone(),
        from,
        to,
        expression: declaration.expression.clone(),
        canonical_matrix: None,
        canonical_entries: Vec::new(),
        row_count,
        column_count,
        row_members: Vec::new(),
        column_members: Vec::new(),
        row_quantity_kinds: Vec::new(),
        column_quantity_kinds: Vec::new(),
        row_units: Vec::new(),
        column_units: Vec::new(),
        compatibility_status: "unresolved".to_owned(),
        status: "metadata_only".to_owned(),
        line: declaration.line,
    })
}

fn parse_linear_operator_type(type_name: &str) -> Option<(String, String)> {
    let rest = type_name
        .trim()
        .strip_prefix("LinearOperator[")?
        .strip_suffix(']')?;
    let (from, to) = rest.split_once("->")?;
    Some((from.trim().to_owned(), to.trim().to_owned()))
}

fn normalize_linear_operator_endpoint(
    endpoint: &str,
    system_name: &str,
    type_aliases: &HashMap<(String, String), String>,
) -> String {
    let trimmed = endpoint.trim();
    if let Some(inner) = trimmed
        .strip_prefix("Derivative[")
        .and_then(|rest| rest.strip_suffix(']'))
    {
        let normalized = normalize_linear_operator_endpoint(inner, system_name, type_aliases);
        return format!("Derivative[{normalized}]");
    }
    type_aliases
        .get(&(system_name.to_owned(), trimmed.to_owned()))
        .cloned()
        .unwrap_or_else(|| trimmed.to_owned())
}

fn matrix_shape(expression: &str) -> (usize, usize) {
    let rows = matrix_rows(expression);
    let row_count = rows.len();
    let column_count = rows.first().map(Vec::len).unwrap_or(0);
    (row_count, column_count)
}

fn matrix_rows(expression: &str) -> Vec<Vec<String>> {
    let trimmed = expression
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');
    trimmed
        .split(';')
        .map(str::trim)
        .filter(|row| !row.is_empty())
        .map(|row| {
            row.trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .map(str::trim)
                .filter(|column| !column.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .collect()
}

fn validate_state_space_vector_members(
    systems: &[SystemInfo],
    vectors: &mut [StateSpaceVectorInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for vector in vectors {
        if vector.members.is_empty() {
            vector.status = "empty".to_owned();
            continue;
        }
        let missing_members = vector
            .members
            .iter()
            .filter(|member| system_variable(systems, &vector.system, member).is_none())
            .cloned()
            .collect::<Vec<_>>();
        if !missing_members.is_empty() {
            vector.status = "member_unresolved".to_owned();
            diagnostics.push(Diagnostic::error(
                "E-STATE-SPACE-VECTOR-MEMBER-001",
                vector.line,
                &format!(
                    "State-space vector `{}` references undeclared member(s): {}.",
                    vector.name,
                    missing_members.join(", ")
                ),
                Some("List only state/input/output names declared in the same system."),
            ));
            continue;
        }

        let role_mismatches = vector
            .members
            .iter()
            .filter_map(|member| {
                let variable = system_variable(systems, &vector.system, member)?;
                (!state_space_vector_member_role_allowed(&vector.role, &variable.role))
                    .then(|| format!("{} ({})", member, variable.role))
            })
            .collect::<Vec<_>>();
        if role_mismatches.is_empty() {
            vector.status = "members_checked".to_owned();
        } else {
            vector.status = "member_role_mismatch".to_owned();
            diagnostics.push(Diagnostic::error(
                "E-STATE-SPACE-VECTOR-MEMBER-ROLE",
                vector.line,
                &format!(
                    "State-space vector `{}` has member(s) with incompatible role(s): {}.",
                    vector.name,
                    role_mismatches.join(", ")
                ),
                Some(state_space_vector_member_role_help(&vector.role)),
            ));
        }
    }
}

fn state_space_vector_member_role_allowed(vector_role: &str, member_role: &str) -> bool {
    match vector_role {
        "states" => member_role == "state",
        "inputs" => member_role == "input",
        "outputs" => matches!(member_role, "state" | "output"),
        _ => true,
    }
}

fn state_space_vector_member_role_help(vector_role: &str) -> &'static str {
    match vector_role {
        "states" => "List only `state` variables in a `states` vector.",
        "inputs" => "List only `input` variables in an `inputs` vector.",
        "outputs" => "List only `state` or `output` variables in an `outputs` vector.",
        _ => "List only variables with roles compatible with the vector declaration.",
    }
}

fn validate_system_state_declarations(systems: &[SystemInfo], diagnostics: &mut Vec<Diagnostic>) {
    for system in systems {
        for state in system
            .variables
            .iter()
            .filter(|variable| variable.role == "state")
        {
            if !unsupported_state_quantity(&state.quantity_kind) {
                continue;
            }
            diagnostics.push(Diagnostic::error(
                "E-SYS-STATE-UNSUPPORTED",
                state.line,
                &format!(
                    "State `{}` in system `{}` uses unsupported state type `{}`.",
                    state.name, system.name, state.quantity_kind
                ),
                Some("Use a numeric scalar quantity as a state, a TimeSeries as an input, or a `states x = [...]` vector declaration for the state-space path."),
            ));
        }
    }
}

fn unsupported_state_quantity(quantity_kind: &str) -> bool {
    let trimmed = quantity_kind.trim();
    crate::stats::time_series_quantity(trimmed).is_some()
        || state_space_vector_type_name(trimmed)
        || derivative_type_name(trimmed)
        || linear_operator_type_name(trimmed)
        || object_type_name_kind(trimmed)
        || matches!(
            trimmed.to_ascii_lowercase().as_str(),
            "string"
                | "path"
                | "filepath"
                | "csvfile"
                | "jsonfile"
                | "tomlfile"
                | "textfile"
                | "reportfile"
                | "plotfile"
                | "directorypath"
                | "bool"
                | "boolean"
                | "processresult"
        )
}

fn validate_system_derivative_equations(systems: &[SystemInfo], diagnostics: &mut Vec<Diagnostic>) {
    for system in systems {
        let states = system
            .variables
            .iter()
            .filter(|variable| variable.role == "state")
            .collect::<Vec<_>>();
        if states.is_empty() {
            continue;
        }

        let state_equations = states
            .iter()
            .map(|state| {
                let equations = system
                    .equations
                    .iter()
                    .filter(|equation| scalar_derivative_on_lhs(&equation.left, &state.name))
                    .collect::<Vec<_>>();
                (*state, equations)
            })
            .collect::<Vec<_>>();

        if state_equations
            .iter()
            .all(|(_state, equations)| equations.is_empty())
        {
            continue;
        }

        for (state, equations) in state_equations {
            if equations.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "E-SYS-DER-MISSING",
                    state.line,
                    &format!(
                        "State `{}` in system `{}` has no derivative equation.",
                        state.name, system.name
                    ),
                    Some(&format!(
                        "Add exactly one equation with `der({})` on the left-hand side, or use a checked state-space vector equation.",
                        state.name
                    )),
                ));
                continue;
            }
            for duplicate in equations.iter().skip(1) {
                diagnostics.push(Diagnostic::error(
                    "E-SYS-DER-DUPLICATE",
                    duplicate.line,
                    &format!(
                        "State `{}` in system `{}` has multiple derivative equations.",
                        state.name, system.name
                    ),
                    Some(&format!(
                        "Keep one RHS equation for `der({})` and combine sources on the right-hand side.",
                        state.name
                    )),
                ));
            }
        }
    }
}

fn scalar_derivative_on_lhs(left: &str, state_name: &str) -> bool {
    left.contains(&format!("der({state_name})"))
}

fn validate_linear_operator_shapes(
    systems: &[SystemInfo],
    vectors: &[StateSpaceVectorInfo],
    operators: &mut [LinearOperatorInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for operator in operators {
        if operator.expression.is_none() {
            continue;
        }
        let expected_rows = state_space_vector_size(vectors, &operator.system, &operator.to);
        let expected_columns = state_space_vector_size(vectors, &operator.system, &operator.from);
        populate_linear_operator_compatibility(systems, vectors, operator);
        let (Some(expected_rows), Some(expected_columns)) = (expected_rows, expected_columns)
        else {
            operator.status = "shape_unresolved".to_owned();
            operator.compatibility_status = "shape_unresolved".to_owned();
            diagnostics.push(Diagnostic::error(
                "E-STATE-SPACE-OP-SHAPE-001",
                operator.line,
                &format!(
                    "Linear operator `{}` references undeclared vector type `{}` -> `{}`.",
                    operator.name, operator.from, operator.to
                ),
                Some("Declare matching `states`, `inputs`, or `outputs` vectors before the operator."),
            ));
            continue;
        };
        if [operator.to.as_str(), operator.from.as_str()]
            .iter()
            .filter_map(|type_name| state_space_vector_status(vectors, &operator.system, type_name))
            .any(|status| status == "member_role_mismatch")
        {
            operator.status = "member_role_mismatch".to_owned();
            operator.compatibility_status = "member_role_mismatch".to_owned();
            continue;
        }
        if operator.row_count != expected_rows || operator.column_count != expected_columns {
            operator.status = "shape_mismatch".to_owned();
            operator.compatibility_status = "shape_mismatch".to_owned();
            diagnostics.push(Diagnostic::error(
                "E-STATE-SPACE-OP-SHAPE-001",
                operator.line,
                &format!(
                    "Linear operator `{}` is {}x{}, expected {}x{} for `{}` -> `{}`.",
                    operator.name,
                    operator.row_count,
                    operator.column_count,
                    expected_rows,
                    expected_columns,
                    operator.from,
                    operator.to
                ),
                Some("Make the matrix rows match the target vector and columns match the source vector."),
            ));
        } else if !linear_operator_rows_are_rectangular(operator, expected_columns, diagnostics) {
            operator.status = "shape_mismatch".to_owned();
            operator.compatibility_status = "shape_mismatch".to_owned();
        } else if operator
            .row_quantity_kinds
            .iter()
            .chain(operator.column_quantity_kinds.iter())
            .any(|quantity_kind| quantity_kind == "unknown")
        {
            operator.status = "member_unresolved".to_owned();
            operator.compatibility_status = "member_unresolved".to_owned();
        } else {
            match validate_linear_operator_entries(operator, diagnostics) {
                LinearOperatorEntryValidation::InvalidValue => {
                    operator.status = "entry_value_invalid".to_owned();
                    operator.compatibility_status = "entry_value_invalid".to_owned();
                }
                LinearOperatorEntryValidation::UnsupportedUnit => {
                    operator.status = "entry_unit_unsupported".to_owned();
                    operator.compatibility_status = "entry_unit_unsupported".to_owned();
                }
                LinearOperatorEntryValidation::Valid => {
                    let canonical_matrix = canonical_linear_operator_matrix(operator);
                    operator.canonical_entries = canonical_matrix
                        .as_deref()
                        .map(|matrix| canonical_linear_operator_entries(operator, matrix))
                        .unwrap_or_default();
                    operator.canonical_matrix = canonical_matrix;
                    operator.status = "shape_checked".to_owned();
                    operator.compatibility_status = "coefficient_units_checked".to_owned();
                }
            }
        }
    }
}

enum LinearOperatorEntryValidation {
    Valid,
    InvalidValue,
    UnsupportedUnit,
}

fn validate_linear_operator_entries(
    operator: &LinearOperatorInfo,
    diagnostics: &mut Vec<Diagnostic>,
) -> LinearOperatorEntryValidation {
    let Some(expression) = operator.expression.as_deref() else {
        return LinearOperatorEntryValidation::Valid;
    };
    for (row_index, row) in matrix_rows(expression).iter().enumerate() {
        for (column_index, entry) in row.iter().enumerate() {
            let Some((value, unit)) = matrix_entry_number_with_optional_unit(entry) else {
                diagnostics.push(Diagnostic::error(
                    "E-STATE-SPACE-OP-ENTRY-VALUE-001",
                    operator.line,
                    &format!(
                        "Linear operator `{}` entry ({}, {}) must be a numeric coefficient with an optional unit.",
                        operator.name,
                        row_index + 1,
                        column_index + 1
                    ),
                    Some("Use entries such as `0.1`, `0.1 1/s`, `0.1 1/min`, or `0.1 1/h`."),
                ));
                return LinearOperatorEntryValidation::InvalidValue;
            };
            if !value.is_finite() {
                diagnostics.push(Diagnostic::error(
                    "E-STATE-SPACE-OP-ENTRY-VALUE-001",
                    operator.line,
                    &format!(
                        "Linear operator `{}` entry ({}, {}) must be finite.",
                        operator.name,
                        row_index + 1,
                        column_index + 1
                    ),
                    Some("Use a finite numeric coefficient before runtime state-space execution."),
                ));
                return LinearOperatorEntryValidation::InvalidValue;
            }
            let Some(unit) = unit else {
                continue;
            };
            if linear_operator_entry_unit_supported(operator, row_index, column_index, &unit) {
                continue;
            }
            diagnostics.push(Diagnostic::error(
                "E-STATE-SPACE-OP-ENTRY-UNIT-001",
                operator.line,
                &format!(
                    "Linear operator `{}` entry ({}, {}) uses unit `{}`; that coefficient unit is not supported for `{}` -> `{}`.",
                    operator.name,
                    row_index + 1,
                    column_index + 1,
                    unit,
                    operator.from,
                    operator.to
                ),
                Some("Use canonical numeric coefficients, or an inverse-time coefficient such as `1/s`, `1/min`, or `1/h` only when the target derivative unit is exactly the source unit per second."),
            ));
            return LinearOperatorEntryValidation::UnsupportedUnit;
        }
    }
    LinearOperatorEntryValidation::Valid
}

fn linear_operator_rows_are_rectangular(
    operator: &LinearOperatorInfo,
    expected_columns: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(expression) = operator.expression.as_deref() else {
        return true;
    };
    let rows = matrix_rows(expression);
    if rows.iter().all(|row| row.len() == expected_columns) {
        return true;
    }
    let row_summary = rows
        .iter()
        .enumerate()
        .map(|(index, row)| format!("row {} has {}", index + 1, row.len()))
        .collect::<Vec<_>>()
        .join(", ");
    diagnostics.push(Diagnostic::error(
        "E-STATE-SPACE-OP-SHAPE-001",
        operator.line,
        &format!(
            "Linear operator `{}` has a non-rectangular matrix: {} column(s) expected, {}.",
            operator.name, expected_columns, row_summary
        ),
        Some("Provide every row with the same number of entries as the source vector."),
    ));
    false
}

fn linear_operator_entry_unit_supported(
    operator: &LinearOperatorInfo,
    row_index: usize,
    column_index: usize,
    unit: &str,
) -> bool {
    if inverse_time_coefficient_scale_to_per_second(unit).is_none() {
        return false;
    }
    let Some(row_unit) = operator.row_units.get(row_index) else {
        return false;
    };
    let Some(column_unit) = operator.column_units.get(column_index) else {
        return false;
    };
    row_unit == &format!("{column_unit}/s")
}

fn canonical_linear_operator_matrix(operator: &LinearOperatorInfo) -> Option<Vec<Vec<f64>>> {
    let expression = operator.expression.as_deref()?;
    let rows = matrix_rows(expression)
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|entry| {
                    let (value, unit) = matrix_entry_number_with_optional_unit(&entry)?;
                    let scale = unit
                        .as_deref()
                        .and_then(inverse_time_coefficient_scale_to_per_second)
                        .unwrap_or(1.0);
                    Some(value * scale)
                })
                .collect::<Option<Vec<_>>>()
        })
        .collect::<Option<Vec<_>>>()?;
    (!rows.is_empty() && rows.iter().all(|row| !row.is_empty())).then_some(rows)
}

fn canonical_linear_operator_entries(
    operator: &LinearOperatorInfo,
    matrix: &[Vec<f64>],
) -> Vec<LinearOperatorEntryInfo> {
    matrix
        .iter()
        .enumerate()
        .flat_map(|(row_index, row)| {
            row.iter()
                .enumerate()
                .filter(|(_, coefficient)| **coefficient != 0.0)
                .map(move |(column_index, coefficient)| LinearOperatorEntryInfo {
                    row_index,
                    column_index,
                    row_member: operator
                        .row_members
                        .get(row_index)
                        .cloned()
                        .unwrap_or_else(|| format!("row[{row_index}]")),
                    column_member: operator
                        .column_members
                        .get(column_index)
                        .cloned()
                        .unwrap_or_else(|| format!("column[{column_index}]")),
                    coefficient: *coefficient,
                })
        })
        .collect()
}

fn inverse_time_coefficient_scale_to_per_second(unit: &str) -> Option<f64> {
    match normalize_unit(unit).as_str() {
        "1/s" | "1/sec" | "1/second" => Some(1.0),
        "1/min" | "1/minute" => Some(1.0 / 60.0),
        "1/h" | "1/hr" | "1/hour" => Some(1.0 / 3600.0),
        _ => None,
    }
}

fn matrix_entry_number_with_optional_unit(expression: &str) -> Option<(f64, Option<String>)> {
    let mut parts = expression.split_whitespace();
    let value_text = parts.next()?;
    let value = value_text.parse::<f64>().ok()?;
    let unit = parts.next().map(str::to_owned);
    if parts.next().is_some() {
        return None;
    }
    Some((value, unit))
}

fn populate_linear_operator_compatibility(
    systems: &[SystemInfo],
    vectors: &[StateSpaceVectorInfo],
    operator: &mut LinearOperatorInfo,
) {
    operator.row_members =
        state_space_vector_members(vectors, &operator.system, &operator.to).unwrap_or_default();
    operator.column_members =
        state_space_vector_members(vectors, &operator.system, &operator.from).unwrap_or_default();
    let row_is_derivative = operator.to.trim().starts_with("Derivative[");
    operator.row_quantity_kinds = operator
        .row_members
        .iter()
        .map(|member| {
            system_variable(systems, &operator.system, member)
                .map(|variable| {
                    if row_is_derivative {
                        format!("Derivative[{}]", variable.quantity_kind)
                    } else {
                        variable.quantity_kind.clone()
                    }
                })
                .unwrap_or_else(|| "unknown".to_owned())
        })
        .collect();
    operator.column_quantity_kinds = operator
        .column_members
        .iter()
        .map(|member| {
            system_variable(systems, &operator.system, member)
                .map(|variable| variable.quantity_kind.clone())
                .unwrap_or_else(|| "unknown".to_owned())
        })
        .collect();
    operator.row_units = operator
        .row_members
        .iter()
        .map(|member| {
            system_variable(systems, &operator.system, member)
                .map(|variable| {
                    if row_is_derivative {
                        format!("{}/s", variable.canonical_unit)
                    } else {
                        variable.canonical_unit.clone()
                    }
                })
                .unwrap_or_else(|| "unknown".to_owned())
        })
        .collect();
    operator.column_units = operator
        .column_members
        .iter()
        .map(|member| {
            system_variable(systems, &operator.system, member)
                .map(|variable| variable.canonical_unit.clone())
                .unwrap_or_else(|| "unknown".to_owned())
        })
        .collect();
}

fn state_space_vector_size(
    vectors: &[StateSpaceVectorInfo],
    system: &str,
    type_name: &str,
) -> Option<usize> {
    let trimmed = type_name.trim();
    if let Some(inner) = trimmed
        .strip_prefix("Derivative[")
        .and_then(|value| value.strip_suffix(']'))
    {
        return state_space_vector_size(vectors, system, inner);
    }
    vectors
        .iter()
        .find(|vector| vector.system == system && vector.vector_type == trimmed)
        .map(|vector| vector.members.len())
}

fn state_space_vector_members(
    vectors: &[StateSpaceVectorInfo],
    system: &str,
    type_name: &str,
) -> Option<Vec<String>> {
    let trimmed = type_name.trim();
    if let Some(inner) = trimmed
        .strip_prefix("Derivative[")
        .and_then(|value| value.strip_suffix(']'))
    {
        return state_space_vector_members(vectors, system, inner);
    }
    vectors
        .iter()
        .find(|vector| vector.system == system && vector.vector_type == trimmed)
        .map(|vector| vector.members.clone())
}

fn state_space_vector_status<'a>(
    vectors: &'a [StateSpaceVectorInfo],
    system: &str,
    type_name: &str,
) -> Option<&'a str> {
    let trimmed = type_name.trim();
    if let Some(inner) = trimmed
        .strip_prefix("Derivative[")
        .and_then(|value| value.strip_suffix(']'))
    {
        return state_space_vector_status(vectors, system, inner);
    }
    vectors
        .iter()
        .find(|vector| vector.system == system && vector.vector_type == trimmed)
        .map(|vector| vector.status.as_str())
}

fn system_variable<'a>(
    systems: &'a [SystemInfo],
    system_name: &str,
    variable_name: &str,
) -> Option<&'a SystemVariableInfo> {
    systems
        .iter()
        .find(|system| system.name == system_name)?
        .variables
        .iter()
        .find(|variable| variable.name == variable_name)
}

fn analyze_equation(
    equation: &crate::ast::EquationDecl,
    system: &mut SystemInfo,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let left_dimension = expression_dimension(&equation.left, &system.variables)
        .unwrap_or_else(|| "unknown".to_owned());
    let right_dimension = expression_dimension(&equation.right, &system.variables)
        .unwrap_or_else(|| "unknown".to_owned());
    let status = if left_dimension != "unknown"
        && right_dimension != "unknown"
        && dimensions_compatible(&left_dimension, &right_dimension)
    {
        "unit_consistent"
    } else {
        if left_dimension != "unknown" && right_dimension != "unknown" {
            diagnostics.push(Diagnostic::error(
                "E-EQ-UNIT-001",
                equation.line,
                &format!(
                    "Equation dimensions do not match: left is {}, right is {}.",
                    left_dimension, right_dimension
                ),
                Some("Both sides of a physical equation must have the same dimension."),
            ));
        }
        "unit_unresolved"
    };
    let residual_name = format!("{}.residual_{}", system.name, system.residuals.len() + 1);
    let residual_expression = format!("{} - ({})", equation.left, equation.right);
    let residual_dimension = if status == "unit_consistent" {
        left_dimension.clone()
    } else {
        "unknown".to_owned()
    };
    let dependencies = equation_dependencies(&equation.left, &equation.right, &system.variables);
    let derivative_states = derivative_states(&equation.left, &equation.right, &system.variables);
    let jacobian_variables = dependencies
        .iter()
        .filter(|dependency| dependency.role == "state")
        .map(|dependency| dependency.name.clone())
        .collect::<Vec<_>>();

    system.equations.push(EquationInfo {
        system: system.name.clone(),
        left: equation.left.clone(),
        right: equation.right.clone(),
        relation: "eq".to_owned(),
        left_dimension,
        right_dimension,
        residual: residual_name.clone(),
        status: status.to_owned(),
        line: equation.line,
    });
    system.residuals.push(ResidualInfo {
        system: system.name.clone(),
        name: residual_name.clone(),
        expression: residual_expression.clone(),
        dimension: residual_dimension,
        line: equation.line,
    });
    system.solver_plan.solve_order.push(residual_name.clone());
    system.solver_plan.jacobian_seed.push(JacobianSeedInfo {
        residual: residual_name.clone(),
        with_respect_to: jacobian_variables,
        derivative_states: derivative_states.clone(),
        status: "symbolic_seed".to_owned(),
    });
    system.equation_ir.push(EquationIrInfo {
        system: system.name.clone(),
        residual: residual_name,
        relation: "eq".to_owned(),
        normalized_residual: residual_expression,
        dependencies,
        derivative_states,
        status: status.to_owned(),
        line: equation.line,
    });
}

fn equation_dependencies(
    left: &str,
    right: &str,
    variables: &[SystemVariableInfo],
) -> Vec<EquationDependencyInfo> {
    let expression = format!("{left} {right}");
    variables
        .iter()
        .filter(|variable| expression_mentions_identifier(&expression, &variable.name))
        .map(|variable| EquationDependencyInfo {
            name: variable.name.clone(),
            role: variable.role.clone(),
            quantity_kind: variable.quantity_kind.clone(),
        })
        .collect()
}

fn validate_json_read_field_access_policy(
    program: &ParsedProgram,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let json_read_bindings = program
        .items
        .iter()
        .filter_map(|item| {
            let AstItem::FastBinding(binding) = item else {
                return None;
            };
            read_only_io_expression(&binding.expression)
                .is_some_and(|(kind, _)| kind == "json")
                .then(|| binding.name.clone())
        })
        .collect::<HashSet<_>>();
    if json_read_bindings.is_empty() {
        return;
    }

    let mut reported = HashSet::new();
    for line in &program.lines {
        if line
            .text
            .to_ascii_lowercase()
            .contains("promote json records ")
        {
            continue;
        }
        for window in line.tokens.windows(3) {
            let [crate::lexer::Token {
                kind: crate::lexer::TokenKind::Identifier(binding),
                ..
            }, crate::lexer::Token {
                kind: crate::lexer::TokenKind::Symbol(crate::lexer::Symbol::Dot),
                ..
            }, crate::lexer::Token {
                kind: crate::lexer::TokenKind::Identifier(field),
                ..
            }] = window
            else {
                continue;
            };
            if !json_read_bindings.contains(binding) {
                continue;
            }
            if !reported.insert((line.line, binding.clone(), field.clone())) {
                continue;
            }
            diagnostics.push(Diagnostic::error(
                "E-IO-JSON-FIELD-ACCESS-001",
                line.line,
                &format!(
                    "`read json` binding `{binding}` does not support direct field access `{binding}.{field}`."
                ),
                Some("Promote the JSON payload to a schema first, for example `typed = promote json payload as SchemaName`."),
            ));
        }
    }
}

fn derivative_states(left: &str, right: &str, variables: &[SystemVariableInfo]) -> Vec<String> {
    let expression = format!("{left} {right}");
    variables
        .iter()
        .filter(|variable| variable.role == "state")
        .filter(|variable| expression.contains(&format!("der({})", variable.name)))
        .map(|variable| variable.name.clone())
        .collect()
}

fn expression_mentions_identifier(expression: &str, identifier: &str) -> bool {
    expression
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .any(|token| token == identifier)
}

fn analyze_fast_binding(binding: &FastBinding, accum: &mut SemanticAccum<'_>) {
    if binding.context == ParseContext::Function {
        return;
    }

    if binding.context == ParseContext::Schema {
        let help = schema_fast_assignment_help(binding);
        accum.diagnostics.push(Diagnostic::error(
            "E-PUBLIC-ANNOTATION-001",
            binding.line,
            "Schema columns require explicit quantity type and source unit.",
            Some(&help),
        ));
        return;
    }

    check_dimensionless_operation(&binding.expression, binding.line, accum.diagnostics);
    check_ambiguous_quantity(binding, accum.diagnostics);
    let available_bindings = accum.available_bindings();
    if let Some(diagnostic) = crate::stats::heat_rate_sum_diagnostic(binding, &available_bindings) {
        accum.diagnostics.push(diagnostic);
    }
    if let Some(integration) = crate::stats::integration_info(binding, &available_bindings) {
        accum.integrations.push(integration);
    }
    if let Some(diagnostic) = crate::uncertainty::source_diagnostic(binding, &available_bindings) {
        accum.diagnostics.push(diagnostic);
    }
    for diagnostic in crate::uncertainty::argument_diagnostics(binding) {
        accum.diagnostics.push(diagnostic);
    }
    for diagnostic in crate::ml::source_diagnostics(binding, &available_bindings) {
        accum.diagnostics.push(diagnostic);
    }
    for diagnostic in crate::ml::argument_diagnostics(binding) {
        accum.diagnostics.push(diagnostic);
    }
    let uncertainty = crate::uncertainty::uncertainty_info(binding, &available_bindings);
    if let Some(uncertainty) = &uncertainty {
        accum.uncertainty_infos.push(uncertainty.clone());
    }
    if let Some(ml_info) = crate::ml::ml_info(binding) {
        accum.ml_infos.push(ml_info);
    }

    let function_call_type = if should_validate_function_call(&binding.expression, accum.functions)
    {
        validate_function_call_expression(
            &binding.expression,
            binding.line,
            &available_bindings,
            accum.functions,
            accum.diagnostics,
        )
    } else {
        None
    };
    let method_call_type = validate_object_method_call_expression(
        &binding.expression,
        binding.line,
        accum.classes,
        accum.class_objects,
        accum.diagnostics,
    );
    let inferred_semantic_type = uncertainty
        .as_ref()
        .and_then(|uncertainty| {
            semantic_type(
                &format!("{}[{}]", uncertainty.kind, uncertainty.quantity_kind),
                &uncertainty.display_unit,
            )
        })
        .or_else(|| {
            object_field_access_semantic_type(
                &binding.expression,
                accum.classes,
                accum.class_objects,
            )
        })
        .or(method_call_type)
        .or_else(|| db_connection_semantic_type(&binding.expression))
        .or_else(|| path_helper_semantic_type(&binding.expression))
        .or_else(|| statistic_expression_semantic_type(&binding.expression, &available_bindings))
        .or(function_call_type)
        .or_else(|| net_response_field_semantic_type(&binding.expression, &available_bindings))
        .or_else(|| binding_alias_semantic_type(&binding.expression, &available_bindings))
        .or_else(|| infer_quantity(&binding.name, &binding.expression));

    if let Some(semantic_type) = inferred_semantic_type {
        let canonical_unit = default_unit_for_quantity(&semantic_type.quantity_kind);
        let dimension = dimension_for_quantity(&semantic_type.quantity_kind);
        if let Some(kernel) = preview_timeseries_kernel_info(binding, &semantic_type) {
            accum.timeseries_kernels.push(kernel);
        }
        accum.inferred_declarations.push(InferredDeclaration {
            name: binding.name.clone(),
            quantity_kind: semantic_type.quantity_kind.clone(),
            display_unit: semantic_type.display_unit.clone(),
            expression: binding.expression.clone(),
            line: binding.line,
        });
        accum.typed_bindings.push(TypedBinding {
            name: binding.name.clone(),
            semantic_type: semantic_type.clone(),
            line: binding.line,
        });
        accum.hover_hints.push(HoverHint::inferred(
            binding.name.clone(),
            semantic_type.quantity_kind.clone(),
            semantic_type.display_unit.clone(),
            binding.expression.clone(),
            binding.span,
        ));
        accum.type_infos.push(TypeInfo {
            name: binding.name.clone(),
            quantity_kind: semantic_type.quantity_kind.clone(),
            display_unit: semantic_type.display_unit.clone(),
            canonical_unit: canonical_unit.clone(),
            dimension,
            source: TypeInfoSource::Inferred,
            line: binding.line,
            span: binding.span,
        });
        accum.unit_derivations.push(unit_derivation(
            &binding.name,
            Some(&binding.expression),
            &semantic_type.quantity_kind,
            &semantic_type.display_unit,
            &canonical_unit,
            binding.line,
        ));
    }
}

struct SemanticAccum<'a> {
    diagnostics: &'a mut Vec<Diagnostic>,
    inferred_declarations: &'a mut Vec<InferredDeclaration>,
    typed_bindings: &'a mut Vec<TypedBinding>,
    scoped_bindings: Vec<TypedBinding>,
    hover_hints: &'a mut Vec<HoverHint>,
    type_infos: &'a mut Vec<TypeInfo>,
    unit_derivations: &'a mut Vec<UnitDerivation>,
    integrations: &'a mut Vec<IntegrationInfo>,
    uncertainty_infos: &'a mut Vec<UncertaintyInfo>,
    ml_infos: &'a mut Vec<MlInfo>,
    functions: &'a [FunctionInfo],
    classes: &'a [ClassInfo],
    class_objects: &'a [ClassObjectInfo],
    timeseries_kernels: &'a mut Vec<TimeSeriesKernelInfo>,
}

impl SemanticAccum<'_> {
    fn available_bindings(&self) -> Vec<TypedBinding> {
        let mut bindings = self.typed_bindings.clone();
        bindings.extend(self.scoped_bindings.clone());
        bindings
    }
}

fn schema_fast_assignment_help(binding: &FastBinding) -> String {
    if let Some(unit) = first_unit_in_expression(&binding.expression) {
        if let Some(quantity) = infer_quantity_from_name_and_unit(&binding.name, &unit) {
            return format!(
                "Write `{}: {} [{}]` instead of assigning a value.",
                binding.name, quantity.quantity_kind, unit
            );
        }
    }
    "Write `T_supply: AbsoluteTemperature [degC]` instead of assigning a value.".to_owned()
}

fn check_ambiguous_quantity(binding: &FastBinding, diagnostics: &mut Vec<Diagnostic>) {
    let Some(unit) = first_unit_in_expression(&binding.expression) else {
        return;
    };
    let candidates = candidates_for_unit(&unit);
    if candidates.len() <= 1 {
        return;
    }
    if infer_quantity_from_name_and_unit(&binding.name, &unit).is_some() {
        return;
    }

    diagnostics.push(Diagnostic::warning(
        "W-QTY-AMBIG-001",
        binding.line,
        &format!(
            "`{}` has unit {}, but quantity kind is ambiguous.",
            binding.name, unit
        ),
        Some(&format!(
            "Candidate quantity kinds: {}. Add an explicit annotation.",
            completion_labels(&candidates)
        )),
    ));
}

fn check_dimensionless_operation(expression: &str, line: usize, diagnostics: &mut Vec<Diagnostic>) {
    let terms = additive_terms(expression);

    for pair in terms.windows(2) {
        let [left, right] = pair else {
            continue;
        };
        if left.operator.is_none() && right.operator.is_none() {
            continue;
        }

        let left_category = categorize_term(&left.text);
        let right_category = categorize_term(&right.text);
        let Some(physical) = physical_dimensionless_pair(&left_category, &right_category) else {
            continue;
        };

        diagnostics.push(dimensionless_diagnostic(physical, line));
        return;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AdditiveTerm {
    operator: Option<char>,
    text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TermCategory {
    Physical(String),
    Dimensionless,
    Unknown,
}

fn additive_terms(expression: &str) -> Vec<AdditiveTerm> {
    let mut terms = Vec::new();
    let mut start = 0usize;
    let mut operator = None;

    for (index, character) in expression.char_indices() {
        if character != '+' && character != '-' {
            continue;
        }
        if index == 0 {
            continue;
        }

        let text = expression[start..index].trim();
        if !text.is_empty() {
            terms.push(AdditiveTerm {
                operator,
                text: text.to_owned(),
            });
        }
        start = index + character.len_utf8();
        operator = Some(character);
    }

    let text = expression[start..].trim();
    if !text.is_empty() {
        terms.push(AdditiveTerm {
            operator,
            text: text.to_owned(),
        });
    }

    terms
}

fn categorize_term(term: &str) -> TermCategory {
    if let Some(unit) = first_unit_in_expression(term) {
        if let Some(quantity) = choose_term_quantity(&unit) {
            return TermCategory::Physical(quantity.quantity_kind.to_owned());
        }
    }

    if is_number_literal(term.trim()) {
        return TermCategory::Dimensionless;
    }

    TermCategory::Unknown
}

fn choose_term_quantity(unit: &str) -> Option<QuantityCompletion> {
    let candidates = candidates_for_unit(unit);
    candidates.first().copied()
}

fn physical_dimensionless_pair<'a>(
    left: &'a TermCategory,
    right: &'a TermCategory,
) -> Option<&'a str> {
    match (left, right) {
        (TermCategory::Physical(quantity), TermCategory::Dimensionless)
        | (TermCategory::Dimensionless, TermCategory::Physical(quantity)) => Some(quantity),
        _ => None,
    }
}

fn dimensionless_diagnostic(quantity_kind: &str, line: usize) -> Diagnostic {
    match quantity_kind {
        "Length" => Diagnostic::error(
            "E-DIM-ADD-001",
            line,
            "Cannot add or subtract Length and DimensionlessNumber.",
            Some("If the dimensionless literal has a unit, write the unit explicitly."),
        ),
        "HeatRate" | "ElectricPower" | "MechanicalPower" => Diagnostic::error(
            "E-DIM-ADD-002",
            line,
            "Cannot add or subtract DimensionlessNumber and Power.",
            Some("If the literal is a power, write a unit such as `kW`."),
        ),
        "AbsoluteTemperature" => Diagnostic::error(
            "E-DIM-ADD-003",
            line,
            "Cannot add AbsoluteTemperature and DimensionlessNumber.",
            Some("If the literal is a temperature difference, write `K`."),
        ),
        other => Diagnostic::error(
            "E-DIM-ADD-004",
            line,
            &format!("Cannot add or subtract {other} and DimensionlessNumber."),
            Some("Add an explicit unit or conversion before combining physical and dimensionless values."),
        ),
    }
}

fn infer_quantity(name: &str, expression: &str) -> Option<SemanticType> {
    let lowered_name = name.to_ascii_lowercase();
    let lowered_expression = expression.to_ascii_lowercase();

    if let Some(semantic_type) = path_helper_semantic_type(expression) {
        return Some(semantic_type);
    }

    if let Some((quantity_kind, display_unit)) =
        crate::uncertainty::uncertainty_semantic_type(name, expression)
    {
        return semantic_type(&quantity_kind, &display_unit);
    }

    if let Some((quantity_kind, display_unit)) = crate::ml::ml_semantic_type(expression) {
        return semantic_type(&quantity_kind, &display_unit);
    }

    if sample_generation_method(expression).is_some() {
        return semantic_type("Table[Sample]", "schema-defined");
    }

    if materialize_cases_source_table(expression).is_some() {
        return semantic_type("Table[Case]", "eng.case");
    }

    if case_apply_cases_binding(expression).is_some() {
        return semantic_type("Table[CaseOutput]", "eng.case");
    }

    if lowered_expression.contains("promote csv")
        || lowered_expression.contains("promote json records")
    {
        return semantic_type("Table[Time]", "schema-defined");
    }

    if lowered_expression.contains("promote json") || lowered_expression.contains("promote toml") {
        return semantic_type("ConfigObject", "schema-defined");
    }

    if crate::net::is_http_request_expression(&lowered_expression) {
        return semantic_type("HttpResponse", "eng.net");
    }

    if lowered_expression.starts_with("url(") {
        return semantic_type("Url", "eng.net");
    }

    if lowered_expression.starts_with("secret env(") {
        return semantic_type("Secret[String]", "redacted");
    }

    if lowered_expression.starts_with("select_first_row(") {
        return semantic_type("String", "");
    }

    if crate::table::is_filter_expression(expression) {
        return semantic_type("TableTransform[Filter]", "eng.table");
    }

    if crate::table::is_require_one_expression(expression) {
        return semantic_type("TableRow", "eng.table");
    }
    if crate::table::is_select_expression(expression) {
        return semantic_type("TableTransform[Select]", "eng.table");
    }
    if crate::table::is_sort_expression(expression) {
        return semantic_type("TableTransform[Sort]", "eng.table");
    }
    if crate::table::is_derive_expression(expression) {
        return semantic_type("TableTransform[Derive]", "eng.table");
    }
    if crate::table::is_join_expression(expression) {
        return semantic_type("TableTransform[Join]", "eng.table");
    }

    if lowered_expression.starts_with("check(") && lowered_expression.contains("coverage ") {
        return semantic_type("CoverageResult", "");
    }

    if lowered_expression.starts_with("fill(") && lowered_expression.contains("missing ") {
        return semantic_type("TimeSeriesFillResult", "");
    }

    if lowered_expression.starts_with("align(") || lowered_expression.starts_with("resample(") {
        return semantic_type("TimeSeriesAlignmentResult", "");
    }

    if lowered_expression.starts_with("render(") && lowered_expression.contains("template ") {
        return semantic_type("TemplateFile", "");
    }

    if lowered_expression.starts_with("simulate ") {
        return semantic_type("SimulationResult", "object");
    }

    if lowered_expression.starts_with("solve ") {
        return semantic_type("ComponentSolveResult", "object");
    }

    if lowered_expression.starts_with("rmse ") && lowered_expression.contains(" vs ") {
        return semantic_type("TemperatureDelta", "K");
    }

    if looks_like_heat_rate_timeseries(&lowered_name, &lowered_expression) {
        return semantic_type(&crate::stats::time_series_type("Time", "HeatRate"), "W");
    }

    if lowered_expression.contains("integrate(") {
        return semantic_type("Energy", "J");
    }

    if numeric_literal_with_optional_unit(expression).is_some_and(|(_value, unit)| unit.is_none()) {
        return semantic_type("DimensionlessNumber", "1");
    }

    if let Some(unit) = first_unit_in_expression(expression) {
        if let Some(completion) = infer_quantity_from_name_and_unit(name, &unit) {
            return semantic_type(completion.quantity_kind, completion.canonical_unit);
        }

        let candidates = candidates_for_unit(&unit);
        if candidates.len() == 1 {
            let completion = candidates[0];
            return semantic_type(completion.quantity_kind, completion.canonical_unit);
        }
    }

    if lowered_name == "eta" || lowered_name.contains("ratio") {
        return semantic_type("Ratio", "1");
    }

    None
}

fn path_helper_semantic_type(expression: &str) -> Option<SemanticType> {
    let expression = expression.trim();
    if read_only_io_expression(expression).is_some() {
        return semantic_type("String", "");
    }
    if expression
        .strip_prefix("exists ")
        .is_some_and(|inner| !inner.trim().is_empty())
        || expression.starts_with("exists(")
    {
        return semantic_type("Bool", "");
    }
    if expression.starts_with("file(") {
        return semantic_type("FilePath", "");
    }
    if expression.starts_with("dir(") || expression.starts_with("parent(") {
        return semantic_type("DirectoryPath", "");
    }
    if expression.starts_with("join(") {
        return semantic_type("FilePath", "");
    }
    if expression.starts_with("stem(") || expression.starts_with("extension(") {
        return semantic_type("String", "");
    }
    None
}

fn materialize_cases_source_table(expression: &str) -> Option<&str> {
    let source = expression.trim().strip_prefix("materialize cases ")?.trim();
    if is_simple_binding_name(source) {
        Some(source)
    } else {
        None
    }
}

fn case_apply_cases_binding(expression: &str) -> Option<&str> {
    let inner = expression
        .trim()
        .strip_prefix("apply(")?
        .strip_suffix(')')?;
    inner.split(',').find_map(|part| {
        part.trim()
            .strip_prefix("over=")
            .map(str::trim)
            .filter(|value| is_simple_binding_name(value))
    })
}

fn is_simple_binding_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn db_connection_semantic_type(expression: &str) -> Option<SemanticType> {
    expression
        .trim()
        .strip_prefix("open sqlite ")
        .filter(|path| !path.trim().is_empty())
        .and_then(|_| semantic_type("DbConnection", "sqlite"))
}

fn db_table_target_expression(expression: &str) -> Option<(&str, &str)> {
    let (connection, table_call) = expression.trim().split_once(".table(")?;
    let table = table_call.trim().strip_suffix(')')?.trim();
    if connection.trim().is_empty() || !table.starts_with('"') {
        return None;
    }
    Some((connection.trim(), table))
}

pub fn read_only_io_expression(expression: &str) -> Option<(&'static str, &str)> {
    let expression = expression.trim();
    if let Some(path) = expression.strip_prefix("read text ") {
        return Some(("text", path.trim()));
    }
    if let Some(path) = expression.strip_prefix("read json ") {
        return Some(("json", path.trim()));
    }
    if let Some(path) = expression.strip_prefix("read toml ") {
        return Some(("toml", path.trim()));
    }
    if let Some(path) = read_call_inner(expression, "read_text") {
        return Some(("text", path.trim()));
    }
    if let Some(path) = read_call_inner(expression, "read_json") {
        return Some(("json", path.trim()));
    }
    if let Some(path) = read_call_inner(expression, "read_toml") {
        return Some(("toml", path.trim()));
    }
    None
}

fn read_call_inner<'a>(expression: &'a str, function_name: &str) -> Option<&'a str> {
    let trimmed = expression.trim();
    let prefix = format!("{function_name}(");
    trimmed
        .strip_prefix(&prefix)?
        .strip_suffix(')')
        .map(str::trim)
}

fn default_unit_for_quantity(quantity_kind: &str) -> String {
    if let Some((_, inner_quantity)) = crate::uncertainty::uncertainty_inner_quantity(quantity_kind)
    {
        return default_unit_for_quantity(&inner_quantity);
    }
    if let Some((_, value_quantity)) = crate::stats::time_series_quantity(quantity_kind) {
        return default_unit_for_quantity(&value_quantity);
    }
    if state_space_vector_type_name(quantity_kind) || derivative_type_name(quantity_kind) {
        return "vector".to_owned();
    }
    if linear_operator_type_name(quantity_kind) {
        return "operator".to_owned();
    }
    if object_type_name_kind(quantity_kind) {
        return "object".to_owned();
    }
    if secret_type_inner(quantity_kind).is_some() {
        return "redacted".to_owned();
    }

    crate::quantities::all_quantity_completions()
        .iter()
        .find(|completion| completion.quantity_kind == quantity_kind)
        .map(|completion| completion.canonical_unit.to_owned())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn default_unit_for_type(type_name: &str) -> String {
    if preview_scalar_type(type_name) {
        String::new()
    } else {
        default_unit_for_quantity(type_name)
    }
}

fn dimension_for_quantity(quantity_kind: &str) -> String {
    if let Some((_, inner_quantity)) = crate::uncertainty::uncertainty_inner_quantity(quantity_kind)
    {
        return dimension_for_quantity(&inner_quantity);
    }
    if let Some((_, value_quantity)) = crate::stats::time_series_quantity(quantity_kind) {
        return dimension_for_quantity(&value_quantity);
    }
    if state_space_vector_type_name(quantity_kind)
        || derivative_type_name(quantity_kind)
        || linear_operator_type_name(quantity_kind)
    {
        return "StateSpace".to_owned();
    }
    if object_type_name_kind(quantity_kind) {
        return "Object".to_owned();
    }
    if secret_type_inner(quantity_kind).is_some() {
        return "Secret".to_owned();
    }

    crate::quantities::all_quantity_completions()
        .iter()
        .find(|completion| completion.quantity_kind == quantity_kind)
        .map(|completion| completion.dimension.to_owned())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn dimension_for_type(type_name: &str) -> String {
    if preview_scalar_type(type_name) {
        "Dimensionless".to_owned()
    } else {
        dimension_for_quantity(type_name)
    }
}

fn known_decl_type(type_name: &str) -> bool {
    if let Some(inner) = secret_type_inner(type_name) {
        return known_decl_type(inner);
    }
    preview_scalar_type(type_name)
        || state_space_vector_type_name(type_name)
        || derivative_type_name(type_name)
        || linear_operator_type_name(type_name)
        || object_type_name_kind(type_name)
        || default_unit_for_quantity(type_name) != "unknown"
}

fn state_space_vector_type_name(type_name: &str) -> bool {
    matches!(
        type_name.trim(),
        "StateVector" | "InputVector" | "OutputVector"
    )
}

fn derivative_type_name(type_name: &str) -> bool {
    type_name.trim().starts_with("Derivative[") && type_name.trim().ends_with(']')
}

fn linear_operator_type_name(type_name: &str) -> bool {
    type_name.trim().starts_with("LinearOperator[") && type_name.trim().ends_with(']')
}

fn object_type_name_kind(type_name: &str) -> bool {
    type_name.trim().starts_with("Object[") && type_name.trim().ends_with(']')
}

pub(crate) fn secret_type_inner(type_name: &str) -> Option<&str> {
    let inner = type_name
        .trim()
        .strip_prefix("Secret[")?
        .strip_suffix(']')?
        .trim();
    (!inner.is_empty()).then_some(inner)
}

fn preview_scalar_type(type_name: &str) -> bool {
    matches!(
        type_name.trim().to_ascii_lowercase().as_str(),
        "string"
            | "path"
            | "filepath"
            | "csvfile"
            | "jsonfile"
            | "tomlfile"
            | "textfile"
            | "reportfile"
            | "plotfile"
            | "directorypath"
            | "bool"
            | "boolean"
            | "int"
            | "integer"
            | "count"
            | "float"
            | "number"
            | "duration"
            | "processresult"
    )
}

fn expression_mentions_args(expression: &str) -> bool {
    expression
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || character == '_' || character == '.')
        })
        .any(|token| token == "args" || token.starts_with("args."))
}

fn expression_has_side_effect(expression: &str) -> bool {
    let lowered = expression.to_ascii_lowercase();
    [
        "download(",
        "read text ",
        "read json ",
        "read toml ",
        "read_text(",
        "read_json(",
        "read_toml(",
        "read_csv(",
        "write text ",
        "write json ",
        "write_file(",
        "run command",
        "save(",
        "export(",
        "create_temp_dir(",
        "promote ",
        "promote(",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn expression_depends_on_runtime(expression: &str) -> bool {
    let lowered = expression.to_ascii_lowercase();
    ["env(", "today(", "now(", "current_dir(", "cwd("]
        .iter()
        .any(|needle| lowered.contains(needle))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DimensionSymbol {
    name: String,
    dimension: String,
}

fn expression_dimension(expression: &str, variables: &[SystemVariableInfo]) -> Option<String> {
    let symbols = variables
        .iter()
        .map(|variable| DimensionSymbol {
            name: variable.name.clone(),
            dimension: variable.dimension.clone(),
        })
        .collect::<Vec<_>>();
    expression_dimension_with_symbols(expression, &symbols)
}

fn expression_dimension_with_symbols(
    expression: &str,
    symbols: &[DimensionSymbol],
) -> Option<String> {
    let expression = strip_outer_parens(expression.trim());
    if expression.is_empty() {
        return None;
    }

    if let Some((_value, unit)) = numeric_literal_with_optional_unit(expression) {
        let Some(unit) = unit else {
            return Some("Dimensionless".to_owned());
        };
        if normalize_unit(&unit) == "1" {
            return Some("Dimensionless".to_owned());
        }
        if let Some(quantity) = candidates_for_unit(&unit).first() {
            return Some(quantity.dimension.to_owned());
        }
    }

    if let Some(symbol) = symbols.iter().find(|symbol| symbol.name == expression) {
        return Some(symbol.dimension.clone());
    }

    if let Some((_function, argument)) = dimensionless_math_function_call(expression) {
        let argument_dimension = expression_dimension_with_symbols(argument, symbols)?;
        if dimensions_compatible(&argument_dimension, "Dimensionless") {
            return Some("Dimensionless".to_owned());
        }
        return Some("mismatch".to_owned());
    }

    let additive_terms = split_top_level(expression, &['+', '-']);
    if additive_terms.len() > 1 {
        let mut dimensions = Vec::new();
        for term in additive_terms {
            dimensions.push(expression_dimension_with_symbols(&term, symbols)?);
        }
        let first = dimensions.first()?.clone();
        if dimensions
            .iter()
            .all(|dimension| dimensions_compatible(&first, dimension))
        {
            return Some(first);
        }
        return Some("mismatch".to_owned());
    }

    let factors = split_top_level_with_operators(expression, &['*', '/']);
    if factors.len() > 1 {
        let mut dimension = expression_dimension_with_symbols(&factors[0].1, symbols)?;
        for (operator, factor) in factors.iter().skip(1) {
            let factor_dimension = expression_dimension_with_symbols(factor, symbols)?;
            dimension = match operator {
                Some('*') => multiply_dimensions(&dimension, &factor_dimension),
                Some('/') => divide_dimensions(&dimension, &factor_dimension),
                _ => dimension,
            };
        }
        return Some(dimension);
    }

    if let Some(inner) = expression
        .strip_prefix("der(")
        .and_then(|value| value.strip_suffix(')'))
    {
        let inner_dimension = expression_dimension_with_symbols(inner, symbols)?;
        return Some(derivative_dimension(&inner_dimension));
    }

    if is_identifier(expression) {
        return symbols
            .iter()
            .find(|symbol| symbol.name == expression)
            .map(|symbol| symbol.dimension.clone());
    }

    if let Some(unit) = first_unit_in_expression(expression) {
        if let Some(quantity) = candidates_for_unit(&unit).first() {
            return Some(quantity.dimension.to_owned());
        }
    }

    None
}

const DIMENSIONLESS_MATH_FUNCTIONS: [&str; 9] = [
    "sqrt", "exp", "ln", "sin", "cos", "tan", "asin", "acos", "atan",
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct MathFunctionDimensionError {
    function: &'static str,
    argument: String,
    argument_dimension: String,
}

fn dimensionless_math_function_dimension_error(
    expression: &str,
    symbols: &[DimensionSymbol],
) -> Option<MathFunctionDimensionError> {
    let expression = strip_outer_parens(expression.trim());
    if expression.is_empty() {
        return None;
    }
    if let Some(rest) = expression
        .strip_prefix('-')
        .or_else(|| expression.strip_prefix('+'))
    {
        return dimensionless_math_function_dimension_error(rest.trim(), symbols);
    }
    if let Some((function, argument)) = dimensionless_math_function_call(expression) {
        if let Some(error) = dimensionless_math_function_dimension_error(argument, symbols) {
            return Some(error);
        }
        let argument_dimension = expression_dimension_with_symbols(argument, symbols)?;
        if !dimensions_compatible(&argument_dimension, "Dimensionless") {
            return Some(MathFunctionDimensionError {
                function,
                argument: argument.trim().to_owned(),
                argument_dimension,
            });
        }
        return None;
    }

    let additive_terms = split_top_level(expression, &['+', '-']);
    if additive_terms.len() > 1 {
        for term in additive_terms {
            if let Some(error) = dimensionless_math_function_dimension_error(&term, symbols) {
                return Some(error);
            }
        }
        return None;
    }

    let factors = split_top_level_with_operators(expression, &['*', '/']);
    if factors.len() > 1 {
        for (_operator, factor) in factors {
            if let Some(error) = dimensionless_math_function_dimension_error(&factor, symbols) {
                return Some(error);
            }
        }
        return None;
    }

    if let Some(inner) = expression
        .strip_prefix("der(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return dimensionless_math_function_dimension_error(inner, symbols);
    }

    None
}

fn dimensionless_math_function_call<'a>(expression: &'a str) -> Option<(&'static str, &'a str)> {
    let expression = strip_outer_parens(expression.trim());
    for function in DIMENSIONLESS_MATH_FUNCTIONS {
        let Some(rest) = expression.strip_prefix(function) else {
            continue;
        };
        let rest = rest.trim_start();
        if !rest.starts_with('(') {
            continue;
        }
        let Some(close_index) = matching_closing_paren_end(rest) else {
            continue;
        };
        if close_index != rest.len() {
            continue;
        }
        return Some((function, rest[1..rest.len() - 1].trim()));
    }
    None
}

fn matching_closing_paren_end(expression: &str) -> Option<usize> {
    let mut depth = 0i32;
    for (index, character) in expression.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index + character.len_utf8());
                }
                if depth < 0 {
                    return None;
                }
            }
            _ => {}
        }
    }
    None
}

fn strip_outer_parens(mut expression: &str) -> &str {
    loop {
        let trimmed = expression.trim();
        if !(trimmed.starts_with('(') && trimmed.ends_with(')')) {
            return trimmed;
        }
        let inner = &trimmed[1..trimmed.len() - 1];
        if !is_balanced(inner) {
            return trimmed;
        }
        expression = inner;
    }
}

fn is_balanced(expression: &str) -> bool {
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

fn split_top_level(expression: &str, operators: &[char]) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;

    for (index, character) in expression.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            other if depth == 0 && operators.contains(&other) => {
                if index == 0 {
                    continue;
                }
                let part = expression[start..index].trim();
                if !part.is_empty() {
                    parts.push(part.to_owned());
                }
                start = index + other.len_utf8();
            }
            _ => {}
        }
    }

    let tail = expression[start..].trim();
    if !tail.is_empty() {
        parts.push(tail.to_owned());
    }
    parts
}

fn split_top_level_with_operators(
    expression: &str,
    operators: &[char],
) -> Vec<(Option<char>, String)> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut pending_operator = None;

    for (index, character) in expression.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            other if depth == 0 && operators.contains(&other) => {
                if index == 0 {
                    continue;
                }
                let part = expression[start..index].trim();
                if !part.is_empty() {
                    parts.push((pending_operator, part.to_owned()));
                    pending_operator = Some(other);
                }
                start = index + other.len_utf8();
            }
            _ => {}
        }
    }

    let tail = expression[start..].trim();
    if !tail.is_empty() {
        parts.push((pending_operator, tail.to_owned()));
    }
    parts
}
fn derivative_dimension(dimension: &str) -> String {
    if dimension == "Dimensionless" {
        "1/Time".to_owned()
    } else {
        format!("{dimension}/Time")
    }
}

fn multiply_dimensions(left: &str, right: &str) -> String {
    combine_dimensions(left, right, DimensionOperator::Multiply)
}

#[derive(Clone, Copy)]
enum DimensionOperator {
    Multiply,
    Divide,
}

fn combine_dimensions(left: &str, right: &str, operator: DimensionOperator) -> String {
    let (mut numerator, mut denominator) = dimension_factors(left);
    let (right_numerator, right_denominator) = dimension_factors(right);
    match operator {
        DimensionOperator::Multiply => {
            numerator.extend(right_numerator);
            denominator.extend(right_denominator);
        }
        DimensionOperator::Divide => {
            numerator.extend(right_denominator);
            denominator.extend(right_numerator);
        }
    }
    canonical_dimension(numerator, denominator)
}

fn dimension_factors(dimension: &str) -> (Vec<String>, Vec<String>) {
    if dimension == "Dimensionless" || dimension.trim().is_empty() {
        return (Vec::new(), Vec::new());
    }
    let mut pieces = dimension.split('/');
    let numerator = pieces
        .next()
        .unwrap_or_default()
        .split('*')
        .filter_map(dimension_factor)
        .collect::<Vec<_>>();
    let denominator = pieces
        .flat_map(|piece| piece.split('*'))
        .filter_map(dimension_factor)
        .collect::<Vec<_>>();
    (numerator, denominator)
}

fn dimension_factor(factor: &str) -> Option<String> {
    let factor = factor.trim();
    if factor.is_empty() || factor == "1" || factor == "Dimensionless" {
        None
    } else {
        Some(factor.to_owned())
    }
}

fn canonical_dimension(mut numerator: Vec<String>, mut denominator: Vec<String>) -> String {
    let mut index = 0;
    while index < numerator.len() {
        if let Some(denominator_index) = denominator
            .iter()
            .position(|factor| factor == &numerator[index])
        {
            numerator.remove(index);
            denominator.remove(denominator_index);
        } else {
            index += 1;
        }
    }
    if numerator.len() == 1 && numerator[0] == "Energy" && denominator == ["Time"] {
        return "Power".to_owned();
    }
    if numerator.is_empty() && denominator.is_empty() {
        return "Dimensionless".to_owned();
    }
    if numerator.is_empty() {
        return format!("1/{}", denominator.join("/"));
    }
    if denominator.is_empty() {
        return numerator.join("*");
    }
    format!("{}/{}", numerator.join("*"), denominator.join("/"))
}

fn dimensions_compatible(left: &str, right: &str) -> bool {
    left == right
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn semantic_type(quantity_kind: &str, display_unit: &str) -> Option<SemanticType> {
    Some(SemanticType {
        quantity_kind: quantity_kind.to_owned(),
        display_unit: display_unit.to_owned(),
    })
}

fn preview_timeseries_kernel_info(
    binding: &FastBinding,
    semantic_type: &SemanticType,
) -> Option<TimeSeriesKernelInfo> {
    let (axis, quantity_kind) = crate::stats::time_series_quantity(&semantic_type.quantity_kind)?;
    if quantity_kind != "HeatRate" {
        return None;
    }
    let kernel = preview_heat_rate_kernel_match(&binding.expression)?;
    Some(TimeSeriesKernelInfo {
        binding: binding.name.clone(),
        kind: "table_heat_rate_from_mass_flow_cp_delta_t".to_owned(),
        source_table: kernel.source_table,
        axis,
        quantity_kind,
        display_unit: semantic_type.display_unit.clone(),
        expression: binding.expression.clone(),
        operations: vec![
            "load_table_column:MassFlowRate".to_owned(),
            "load_scalar:SpecificHeat".to_owned(),
            "temperature_delta:return_minus_supply".to_owned(),
            "multiply:m_dot_cp_delta_t".to_owned(),
            "store_timeseries".to_owned(),
        ],
        status: "supported".to_owned(),
        line: binding.line,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PreviewHeatRateKernelMatch {
    source_table: Option<String>,
}

fn looks_like_heat_rate_timeseries(name: &str, expression: &str) -> bool {
    let name_suggests_heat_rate =
        name.starts_with('q') || name.contains("heat") || name.contains("coil");
    name_suggests_heat_rate && preview_heat_rate_kernel_match(expression).is_some()
}

fn preview_heat_rate_kernel_match(expression: &str) -> Option<PreviewHeatRateKernelMatch> {
    let lowered = expression.to_ascii_lowercase();
    let expression_uses_mass_flow = lowered.contains(".m_dot");
    let expression_uses_supply =
        lowered.contains(".t_supply") || lowered.contains(".supply") || lowered.contains("_supply");
    let expression_uses_return =
        lowered.contains(".t_return") || lowered.contains(".return") || lowered.contains("_return");
    let expression_uses_specific_heat = lowered.contains("cp") || lowered.contains("j/kg/k");
    (expression_uses_mass_flow
        && expression_uses_supply
        && expression_uses_return
        && expression_uses_specific_heat)
        .then(|| PreviewHeatRateKernelMatch {
            source_table: first_table_reference(expression),
        })
}

fn first_table_reference(expression: &str) -> Option<String> {
    expression
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || character == '_' || character == '.')
        })
        .filter_map(|token| token.split_once('.').map(|(table, _)| table.trim()))
        .find(|table| is_identifier(table))
        .map(str::to_owned)
}
