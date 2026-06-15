use crate::ast::{
    ArgsFieldDecl, AssertDecl, AstItem, ClassFieldDecl, ClassMethodDecl, ClassObjectCopyDecl,
    ClassObjectDecl, ClassObjectFieldDecl, ClassValidationDecl, CommandStyleDecl, ConnectDecl,
    ConstDecl, CsvExportDecl, CsvExportFieldDecl, DomainTypeParameterDecl, DomainVariableDecl,
    ExplicitDecl, FastBinding, FileOperationDecl, FunctionDecl, FunctionParamDecl, GoldenDecl,
    ImportDecl, PortDecl, PrintDecl, ProcessRunDecl, ReturnDecl, StateSpaceVectorDecl,
    SystemVariableDecl, TestDecl, WhereBindingDecl, WithOptionDecl, WriteDecl,
};
use crate::expected::{expected_type_from_explicit_decl, ExpectedType, ExpectedTypeSource};
use crate::hover::HoverHint;
use crate::ml::MlInfo;
use crate::parser::{ParseContext, ParsedProgram};
use crate::quantities::{
    candidates_for_unit, completion_labels, first_unit_in_expression,
    infer_quantity_from_name_and_unit, is_number_literal, QuantityCompletion,
};
use crate::schema::{CsvPromotion, SchemaInfo};
use crate::stats::{AxisInfo, IntegrationInfo, StatsInfo};
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
pub struct LinearOperatorInfo {
    pub system: String,
    pub name: String,
    pub from: String,
    pub to: String,
    pub expression: Option<String>,
    pub row_count: usize,
    pub column_count: usize,
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
pub struct ComponentInfo {
    pub name: String,
    pub ports: Vec<PortInfo>,
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
    pub dependencies: Vec<ComponentResidualDependencyInfo>,
    pub algebraic_loops: Vec<Vec<String>>,
    pub jacobian_sparsity: Vec<ComponentJacobianSparsityInfo>,
    pub solver_plan: String,
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
    pub required: bool,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArgValueInfo {
    pub name: String,
    pub type_name: String,
    pub value: String,
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

#[derive(Clone, Debug, Eq, PartialEq)]
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
    pub where_blocks: Vec<WhereBlockInfo>,
    pub with_blocks: Vec<WithBlockInfo>,
    pub timeseries_kernels: Vec<TimeSeriesKernelInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticOutput {
    pub diagnostics: Vec<Diagnostic>,
    pub inferred_declarations: Vec<InferredDeclaration>,
    pub semantic_program: SemanticProgram,
}

pub fn analyze(program: &ParsedProgram) -> SemanticOutput {
    let mut diagnostics = Vec::new();
    let mut inferred_declarations = Vec::new();
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
    let mut linear_operators = Vec::new();
    let mut current_system_index = None;
    let mut domains = Vec::new();
    let mut current_domain_index = None;
    let mut components = Vec::new();
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
                    ports: Vec::new(),
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
            AstItem::SystemVariable(variable) => {
                if let Some(system_index) = current_system_index {
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
                        if let Some(operator) =
                            analyze_linear_operator_decl(declaration, &systems[system_index].name)
                        {
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
                analyze_command_style_decl(command, &mut command_styles, &mut diagnostics);
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
    let with_blocks =
        analyze_with_blocks(program, &typed_bindings, &command_styles, &mut diagnostics);
    validate_file_operation_options(&file_operations, &with_blocks, &mut diagnostics);
    validate_where_local_uses(program, &where_blocks, &mut diagnostics);
    validate_domain_contracts(&domains, &mut diagnostics);
    validate_class_contracts(&classes, &mut class_objects, &mut diagnostics);
    validate_function_returns(&mut functions, &consts, &mut diagnostics);

    let connections = analyze_connections(
        &domains,
        &mut components,
        &raw_connections,
        &mut diagnostics,
    );
    let component_assemblies = build_component_assembly_graphs(&domains, &components, &connections);
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
            workflow: Workflow::top_level(top_level_workflow_line(program)),
            stats_infos,
            integrations,
            uncertainty_infos,
            ml_infos,
            systems,
            state_space_vectors,
            linear_operators,
            domains,
            components,
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

fn analyze_command_style_decl(
    command: &CommandStyleDecl,
    command_styles: &mut Vec<CommandStyleInfo>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if command.status == "ambiguous_target" {
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
            let options = with_options_for_owner(program, block.owner_line)
                .into_iter()
                .map(|option| analyze_with_option(&option, owner_type.as_ref(), diagnostics))
                .collect::<Vec<_>>();
            WithBlockInfo {
                owner_line: block.owner_line,
                options,
                line: block.line,
            }
        })
        .collect()
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
    diagnostics: &mut Vec<Diagnostic>,
) -> WithOptionInfo {
    if !known_with_option(&option.key) {
        diagnostics.push(Diagnostic::error(
            "E-WITH-OPTION-001",
            option.line,
            &format!("Unknown with option `{}`.", option.key),
            Some("Use supported options such as `method`, `backend`, `title`, `type`, `unit x`, or `unit y`."),
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
    WithOptionInfo {
        key: option.key.clone(),
        value: option.value.clone(),
        status: "accepted".to_owned(),
        line: option.line,
    }
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
            | "T_out"
            | "Q_internal"
            | "solar"
            | "tolerance"
            | "max_iter"
            | "seed"
            | "output"
            | "overwrite"
            | "confirm"
            | "recursive"
            | "args"
            | "cwd"
            | "allow_failure"
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
    if !matches!(write.format.as_str(), "text" | "json") {
        diagnostics.push(Diagnostic::error(
            "E-WRITE-002",
            write.line,
            &format!("Write format `{}` is not supported.", write.format),
            Some("Use `write text` or `write json`."),
        ));
        return None;
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
        let left_dimension = dimension_for_quantity(&left.quantity_kind);
        let right_dimension = dimension_for_quantity(&right.quantity_kind);
        if !dimensions_compatible(&left_dimension, &right_dimension) {
            diagnostics.push(Diagnostic::error(
                "E-ASSERT-UNIT-001",
                assertion.line,
                &format!(
                    "Assert compares `{}` ({}) with `{}` ({}).",
                    assertion.left, left_dimension, assertion.right, right_dimension
                ),
                Some("Compare values with compatible dimensions or convert units first."),
            ));
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
        validate_simulation_solver(declaration.line, options, &mut diagnostics);

        for variable in &system.variables {
            let Some(expected) = expected_dynamic_input(variable) else {
                continue;
            };
            let Some(option) = accepted_option(options, &variable.name) else {
                diagnostics.push(Diagnostic::error(
                    "E-SIM-INPUT-MISSING-001",
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
                    "E-SIM-INPUT-TYPE-001",
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
                    "E-SIM-INPUT-TYPE-001",
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
                    "E-SIM-INPUT-AXIS-001",
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
                    "E-SIM-INPUT-QTY-001",
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

fn validate_simulation_timestep(
    owner_line: usize,
    options: &[WithOptionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(option) = accepted_option(options, "timestep") else {
        diagnostics.push(Diagnostic::error(
            "E-SIM-OPTION-MISSING-001",
            owner_line,
            "`simulate` requires `with { timestep = <duration> }`.",
            Some("Use a duration such as `timestep = 10 min`."),
        ));
        return;
    };
    if parse_duration_option_seconds(&option.value).is_none() {
        diagnostics.push(Diagnostic::error(
            "E-SIM-OPTION-TYPE-001",
            option.line,
            &format!(
                "`timestep` expects a positive duration, got `{}`.",
                option.value
            ),
            Some("Use units such as `s`, `min`, or `h`, for example `10 min`."),
        ));
    }
}

fn validate_simulation_solver(
    owner_line: usize,
    options: &[WithOptionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(option) = accepted_option(options, "solver") else {
        diagnostics.push(Diagnostic::error(
            "E-SIM-OPTION-MISSING-002",
            owner_line,
            "`simulate` requires `with { solver = fixed_step }` in the supported workflow.",
            Some("The supported dynamic runner is the fixed-step one-state solver."),
        ));
        return;
    };
    if option.value.trim() != "fixed_step" {
        diagnostics.push(Diagnostic::error(
            "E-SIM-OPTION-TYPE-002",
            option.line,
            &format!("Unsupported simulation solver `{}`.", option.value),
            Some("Use `solver = fixed_step`; adaptive and nonlinear solvers are deferred."),
        ));
    }
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
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<FormatExpressionInfo> {
    let mut fields = Vec::new();
    let mut cursor = 0usize;
    while let Some(open) = template[cursor..].find('{') {
        let start = cursor + open;
        let Some(close_offset) = template[start + 1..].find('}') else {
            diagnostics.push(Diagnostic::error(
                "E-PRINT-FMT-001",
                line,
                "Print template has an unterminated `{...}` interpolation.",
                Some("Close the interpolation with `}`."),
            ));
            break;
        };
        let close = start + 1 + close_offset;
        let inside = template[start + 1..close].trim();
        if inside.is_empty() {
            diagnostics.push(Diagnostic::error(
                "E-PRINT-FMT-002",
                line,
                "Print interpolation is empty.",
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
                    "E-PRINT-FMT-003",
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
                "E-PRINT-FMT-004",
            ));
        }
        cursor = close + 1;
    }
    fields
}

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
            binding.name == table_name && binding.semantic_type.quantity_kind.starts_with("Table[")
        }) {
            return semantic_type("Count", "count");
        }
    }
    if let Some(semantic_type) = statistic_expression_semantic_type(expression, typed_bindings) {
        return Some(semantic_type);
    }
    if let Some(semantic_type) = function_call_semantic_type(expression, typed_bindings, functions)
    {
        return Some(semantic_type);
    }
    typed_bindings
        .iter()
        .find(|binding| binding.name == expression)
        .map(|binding| binding.semantic_type.clone())
}

fn statistic_expression_semantic_type(
    expression: &str,
    typed_bindings: &[TypedBinding],
) -> Option<SemanticType> {
    let (_statistic, source) = parse_statistic_expression(expression)?;
    let source_binding = typed_bindings
        .iter()
        .find(|binding| binding.name == source)?;
    let (_axis, quantity_kind) =
        crate::stats::time_series_quantity(&source_binding.semantic_type.quantity_kind)?;
    semantic_type(&quantity_kind, &source_binding.semantic_type.display_unit)
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
            | "mlp"
            | "evaluate"
            | "model_card"
            | "leakage_lint"
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
                    "E-CONNECT-DOMAIN-001",
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
                    "E-CONNECT-PORT-001",
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
                    "W-PORT-UNCONNECTED-001",
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
                dependencies,
                status: "assembly_seed".to_owned(),
                line: connection_set.line,
            });
        }
    }

    let algebraic_count = variables
        .iter()
        .filter(|variable| variable.role == "algebraic")
        .count();
    let state_count = variables
        .iter()
        .filter(|variable| variable.role == "state")
        .count();
    let unknown_count = algebraic_count + state_count;
    let equation_count = equations.len();
    let (balance_status, diagnostic_code) = if equation_count < unknown_count {
        (
            "underdetermined_seed".to_owned(),
            Some("W-ASSEMBLY-UNDERDETERMINED-SEED".to_owned()),
        )
    } else if equation_count > unknown_count {
        (
            "overdetermined_seed".to_owned(),
            Some("W-ASSEMBLY-OVERDETERMINED-SEED".to_owned()),
        )
    } else {
        ("balanced_metadata_seed".to_owned(), None)
    };
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
    let residual_graph = ComponentResidualGraphInfo {
        name: "component_residual_graph".to_owned(),
        status: if equations.is_empty() {
            "empty".to_owned()
        } else {
            "metadata_only".to_owned()
        },
        residuals: equations
            .iter()
            .map(|equation| equation.name.clone())
            .collect(),
        dependencies,
        algebraic_loops,
        jacobian_sparsity,
        solver_plan: "metadata_only_no_numeric_solve".to_owned(),
    };
    let domain_plans =
        build_component_domain_plans(domains, &connection_sets, &equations, &variables);
    let domain_count = domain_plans.len();
    let solver_preview =
        build_component_solver_preview(domain_count, state_count, equations.len(), 0);
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
        component_equation_count: 0,
        local_expression_count: 0,
        operator_call_count: 0,
        predictor_call_count: 0,
        domain_count,
        domain_plans,
        solver_preview,
        connection_sets,
        equations,
        variables,
        boundary: ComponentAssemblyBoundaryInfo {
            state_count,
            algebraic_count,
            input_count: 0,
            output_count: 0,
            parameter_count: 0,
            equation_count,
            unknown_count,
            balance_status,
            diagnostic_code,
        },
        residual_graph,
        line,
    }]
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

fn build_component_solver_preview(
    domain_count: usize,
    state_count: usize,
    equation_count: usize,
    predictor_call_count: usize,
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
        delay_history: "deferred_no_delay_calls".to_owned(),
        predictor: if predictor_call_count > 0 {
            "predictor_call_metadata_only"
        } else {
            "deferred_no_predictor_calls"
        }
        .to_owned(),
        external_adapter: "deferred_no_external_behavior_adapter".to_owned(),
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
        "Medium" => ("E-CONNECT-MEDIUM-001", "medium_mismatch", "medium"),
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
) -> Option<LinearOperatorInfo> {
    let (from, to) = parse_linear_operator_type(&declaration.type_name)?;
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
        row_count,
        column_count,
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

fn matrix_shape(expression: &str) -> (usize, usize) {
    let trimmed = expression
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');
    let rows = trimmed
        .split(';')
        .map(str::trim)
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let row_count = rows.len();
    let column_count = rows
        .first()
        .map(|row| {
            row.trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .filter(|column| !column.trim().is_empty())
                .count()
        })
        .unwrap_or(0);
    (row_count, column_count)
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
        accum.diagnostics.push(Diagnostic::error(
            "E-PUBLIC-ANNOTATION-001",
            binding.line,
            "Schema columns require explicit quantity type and source unit.",
            Some("Write `T_supply: AbsoluteTemperature [degC]` instead of assigning a value."),
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
        .or_else(|| path_helper_semantic_type(&binding.expression))
        .or_else(|| statistic_expression_semantic_type(&binding.expression, &available_bindings))
        .or(function_call_type)
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

    if lowered_expression.contains("promote csv") {
        return semantic_type("Table[Time]", "schema-defined");
    }

    if lowered_expression.starts_with("simulate ") {
        return semantic_type("SimulationResult", "object");
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

    let factors = split_top_level(expression, &['*']);
    if factors.len() > 1 {
        let mut dimension = expression_dimension_with_symbols(&factors[0], symbols)?;
        for factor in factors.iter().skip(1) {
            let factor_dimension = expression_dimension_with_symbols(factor, symbols)?;
            dimension = multiply_dimensions(&dimension, &factor_dimension);
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

fn derivative_dimension(dimension: &str) -> String {
    if dimension == "Dimensionless" {
        "1/Time".to_owned()
    } else {
        format!("{dimension}/Time")
    }
}

fn multiply_dimensions(left: &str, right: &str) -> String {
    match (left, right) {
        ("Dimensionless", other) | (other, "Dimensionless") => other.to_owned(),
        ("Energy/Temperature", "Temperature/Time")
        | ("Temperature/Time", "Energy/Temperature")
        | ("Power/Temperature", "Temperature")
        | ("Temperature", "Power/Temperature") => "Power".to_owned(),
        _ => format!("{left}*{right}"),
    }
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
