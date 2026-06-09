use crate::ast::{
    ArgsFieldDecl, AstItem, CommandStyleDecl, ConnectDecl, ConstDecl, CsvExportDecl,
    CsvExportFieldDecl, DomainTypeParameterDecl, DomainVariableDecl, ExplicitDecl, FastBinding,
    FunctionDecl, FunctionParamDecl, ImportDecl, PortDecl, PrintDecl, ReturnDecl,
    SystemVariableDecl, WhereBindingDecl, WithOptionDecl, WriteDecl,
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
use crate::units::{unit_derivation, UnitDerivation};
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
    pub domains: Vec<DomainInfo>,
    pub components: Vec<ComponentInfo>,
    pub connections: Vec<ConnectionInfo>,
    pub args_blocks: Vec<ArgsBlockInfo>,
    pub arg_values: Vec<ArgValueInfo>,
    pub environment_dependencies: Vec<EnvironmentDependencyInfo>,
    pub prints: Vec<PrintInfo>,
    pub csv_exports: Vec<CsvExportInfo>,
    pub writes: Vec<WriteInfo>,
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
    let mut current_system_index = None;
    let mut domains = Vec::new();
    let mut current_domain_index = None;
    let mut components = Vec::new();
    let mut current_component_index = None;
    let mut raw_connections = Vec::new();
    let mut args_blocks = Vec::new();
    let mut current_args_block_index = None;
    let mut prints = Vec::new();
    let mut csv_exports = Vec::new();
    let mut current_csv_export_index = None;
    let mut writes = Vec::new();
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
                analyze_print_decl(
                    print,
                    &typed_bindings,
                    &functions,
                    &mut prints,
                    &mut diagnostics,
                );
            }
            AstItem::CsvExport(export) => {
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
                if let Some(write_info) =
                    analyze_write_decl(write, &typed_bindings, &functions, &mut diagnostics)
                {
                    writes.push(write_info);
                }
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
    validate_where_local_uses(program, &where_blocks, &mut diagnostics);
    validate_domain_contracts(&domains, &mut diagnostics);
    validate_function_returns(&mut functions, &consts, &mut diagnostics);

    let connections = analyze_connections(
        &domains,
        &mut components,
        &raw_connections,
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
            workflow: Workflow::top_level(top_level_workflow_line(program)),
            stats_infos,
            integrations,
            uncertainty_infos,
            ml_infos,
            systems,
            domains,
            components,
            connections,
            args_blocks,
            arg_values: Vec::new(),
            environment_dependencies: Vec::new(),
            prints,
            csv_exports,
            writes,
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
            | "tolerance"
            | "max_iter"
            | "seed"
            | "output"
            | "overwrite"
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
            Some("Use a known quantity kind or preview scalar type such as String, CsvFile, or DirectoryPath."),
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
            Some("Annotate function parameters with known quantity kinds or preview scalar types."),
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
            Some("Keep one explicit `return ...` in the preview function body."),
        ));
        return;
    }
    function.return_expression = Some(return_decl.expression.clone());
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
        if preview_scalar_type(&function.return_quantity_kind) {
            function.status = "scalar_preview".to_owned();
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
        } else {
            function.status = "unit_consistent".to_owned();
        }
    }
}

fn analyze_print_decl(
    print: &PrintDecl,
    typed_bindings: &[TypedBinding],
    functions: &[FunctionInfo],
    prints: &mut Vec<PrintInfo>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let fields = analyze_format_fields(
        &print.template,
        print.line,
        typed_bindings,
        functions,
        diagnostics,
    );
    prints.push(PrintInfo {
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
                "CSV export source `{}` is not supported in the preview.",
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
    connections
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
    let inferred_semantic_type = uncertainty
        .as_ref()
        .and_then(|uncertainty| {
            semantic_type(
                &format!("{}[{}]", uncertainty.kind, uncertainty.quantity_kind),
                &uncertainty.display_unit,
            )
        })
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

    if looks_like_heat_rate_timeseries(&lowered_name, &lowered_expression) {
        return semantic_type(&crate::stats::time_series_type("Time", "HeatRate"), "W");
    }

    if lowered_expression.contains("integrate(") {
        return semantic_type("Energy", "J");
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
    preview_scalar_type(type_name) || default_unit_for_quantity(type_name) != "unknown"
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
        status: "preview_supported".to_owned(),
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
