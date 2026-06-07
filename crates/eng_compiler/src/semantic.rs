use crate::ast::{
    AstItem, ConnectDecl, DomainTypeParameterDecl, DomainVariableDecl, ExplicitDecl, FastBinding,
    PortDecl, StructFieldDecl, SystemVariableDecl,
};
use crate::entry::EntryPoint;
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
use crate::{Diagnostic, InferredDeclaration};

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
pub struct ArgsStructInfo {
    pub name: String,
    pub fields: Vec<ArgsFieldInfo>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticProgram {
    pub typed_bindings: Vec<TypedBinding>,
    pub expected_types: Vec<ExpectedType>,
    pub hover_hints: Vec<HoverHint>,
    pub type_infos: Vec<TypeInfo>,
    pub unit_derivations: Vec<UnitDerivation>,
    pub schemas: Vec<SchemaInfo>,
    pub csv_promotions: Vec<CsvPromotion>,
    pub entry_points: Vec<EntryPoint>,
    pub axis_infos: Vec<AxisInfo>,
    pub stats_infos: Vec<StatsInfo>,
    pub integrations: Vec<IntegrationInfo>,
    pub uncertainty_infos: Vec<UncertaintyInfo>,
    pub ml_infos: Vec<MlInfo>,
    pub systems: Vec<SystemInfo>,
    pub domains: Vec<DomainInfo>,
    pub components: Vec<ComponentInfo>,
    pub connections: Vec<ConnectionInfo>,
    pub args_structs: Vec<ArgsStructInfo>,
    pub arg_values: Vec<ArgValueInfo>,
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
    let mut typed_bindings = Vec::new();
    let mut expected_types = Vec::new();
    let mut hover_hints = Vec::new();
    let mut type_infos = Vec::new();
    let mut unit_derivations = Vec::new();
    let mut entry_points = Vec::new();
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
    let mut args_structs = Vec::new();
    let mut current_args_struct_index = None;

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
                args_structs.push(ArgsStructInfo {
                    name: struct_decl.name.clone(),
                    fields: Vec::new(),
                    line: struct_decl.span.line,
                });
                current_args_struct_index = Some(args_structs.len() - 1);
            }
            AstItem::StructField(field) => {
                if let Some(args_struct_index) = current_args_struct_index {
                    analyze_struct_field(field, &mut args_structs[args_struct_index]);
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
                entry_points.push(EntryPoint::from_script(script));
                if script.name != "main" {
                    diagnostics.push(Diagnostic::warning(
                        "W-ENTRY-MAIN-001",
                        script.span.line,
                        "Preview execution expects `script main(args: Args) -> Report`.",
                        Some("Rename this entry to `main` or run with `--entry <name>`."),
                    ));
                }
            }
            AstItem::ExplicitDecl(declaration) => analyze_explicit_decl(
                declaration,
                &mut diagnostics,
                &mut expected_types,
                &mut hover_hints,
                &mut typed_bindings,
                &mut type_infos,
                &mut unit_derivations,
            ),
            AstItem::FastBinding(binding) => {
                let mut accum = SemanticAccum {
                    diagnostics: &mut diagnostics,
                    inferred_declarations: &mut inferred_declarations,
                    typed_bindings: &mut typed_bindings,
                    hover_hints: &mut hover_hints,
                    type_infos: &mut type_infos,
                    unit_derivations: &mut unit_derivations,
                    integrations: &mut integrations,
                    uncertainty_infos: &mut uncertainty_infos,
                    ml_infos: &mut ml_infos,
                };
                analyze_fast_binding(binding, &mut accum);
            }
            AstItem::Summary(summary) => {
                if let Some(info) = crate::stats::stats_info(summary, &typed_bindings) {
                    stats_infos.push(info);
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

    validate_domain_contracts(&domains, &mut diagnostics);

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
            axis_infos: crate::stats::axis_infos(&typed_bindings),
            typed_bindings,
            expected_types,
            hover_hints,
            type_infos,
            unit_derivations,
            schemas: Vec::new(),
            csv_promotions: Vec::new(),
            entry_points,
            stats_infos,
            integrations,
            uncertainty_infos,
            ml_infos,
            systems,
            domains,
            components,
            connections,
            args_structs,
            arg_values: Vec::new(),
        },
    }
}

fn analyze_struct_field(field: &StructFieldDecl, args_struct: &mut ArgsStructInfo) {
    args_struct.fields.push(ArgsFieldInfo {
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
    if let Some(diagnostic) = crate::stats::heat_rate_sum_diagnostic(binding, accum.typed_bindings)
    {
        accum.diagnostics.push(diagnostic);
    }
    if let Some(integration) = crate::stats::integration_info(binding, accum.typed_bindings) {
        accum.integrations.push(integration);
    }
    if let Some(diagnostic) = crate::uncertainty::source_diagnostic(binding, accum.typed_bindings) {
        accum.diagnostics.push(diagnostic);
    }
    for diagnostic in crate::uncertainty::argument_diagnostics(binding) {
        accum.diagnostics.push(diagnostic);
    }
    for diagnostic in crate::ml::source_diagnostics(binding, accum.typed_bindings) {
        accum.diagnostics.push(diagnostic);
    }
    for diagnostic in crate::ml::argument_diagnostics(binding) {
        accum.diagnostics.push(diagnostic);
    }
    let uncertainty = crate::uncertainty::uncertainty_info(binding, accum.typed_bindings);
    if let Some(uncertainty) = &uncertainty {
        accum.uncertainty_infos.push(uncertainty.clone());
    }
    if let Some(ml_info) = crate::ml::ml_info(binding) {
        accum.ml_infos.push(ml_info);
    }

    let inferred_semantic_type = uncertainty
        .as_ref()
        .and_then(|uncertainty| {
            semantic_type(
                &format!("{}[{}]", uncertainty.kind, uncertainty.quantity_kind),
                &uncertainty.display_unit,
            )
        })
        .or_else(|| infer_quantity(&binding.name, &binding.expression));

    if let Some(semantic_type) = inferred_semantic_type {
        let canonical_unit = default_unit_for_quantity(&semantic_type.quantity_kind);
        let dimension = dimension_for_quantity(&semantic_type.quantity_kind);
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
    hover_hints: &'a mut Vec<HoverHint>,
    type_infos: &'a mut Vec<TypeInfo>,
    unit_derivations: &'a mut Vec<UnitDerivation>,
    integrations: &'a mut Vec<IntegrationInfo>,
    uncertainty_infos: &'a mut Vec<UncertaintyInfo>,
    ml_infos: &'a mut Vec<MlInfo>,
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

fn expression_dimension(expression: &str, variables: &[SystemVariableInfo]) -> Option<String> {
    let expression = strip_outer_parens(expression.trim());
    if expression.is_empty() {
        return None;
    }

    let additive_terms = split_top_level(expression, &['+', '-']);
    if additive_terms.len() > 1 {
        let mut dimensions = Vec::new();
        for term in additive_terms {
            dimensions.push(expression_dimension(&term, variables)?);
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
        let mut dimension = expression_dimension(&factors[0], variables)?;
        for factor in factors.iter().skip(1) {
            let factor_dimension = expression_dimension(factor, variables)?;
            dimension = multiply_dimensions(&dimension, &factor_dimension);
        }
        return Some(dimension);
    }

    if let Some(inner) = expression
        .strip_prefix("der(")
        .and_then(|value| value.strip_suffix(')'))
    {
        let inner_dimension = expression_dimension(inner, variables)?;
        return Some(derivative_dimension(&inner_dimension));
    }

    if is_identifier(expression) {
        return variables
            .iter()
            .find(|variable| variable.name == expression)
            .map(|variable| variable.dimension.clone());
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

fn looks_like_heat_rate_timeseries(name: &str, expression: &str) -> bool {
    let name_suggests_heat_rate =
        name.starts_with('q') || name.contains("heat") || name.contains("coil");
    let expression_uses_table_fields = expression.contains(".m_dot")
        && (expression.contains(".t_return") || expression.contains(".t_supply"));
    let expression_uses_specific_heat = expression.contains("cp") || expression.contains("j/kg/k");

    name_suggests_heat_rate && expression_uses_table_fields && expression_uses_specific_heat
}
