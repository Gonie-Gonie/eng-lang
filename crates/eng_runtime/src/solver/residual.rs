use std::collections::{HashMap, HashSet};

use eng_compiler::{all_unit_infos, normalize_unit, UnitInfo};

use super::{
    assembly::EquationAssembly,
    euclidean_norm,
    expression::{
        linearize_arithmetic_expression_with_symbol_metadata_and_unit_converter,
        ArithmeticExpressionProfile, ArithmeticUnitMetadata,
    },
    SolverFailure,
};

pub const DEFAULT_RESIDUAL_TOLERANCE: f64 = 1e-9;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualGraph {
    pub name: String,
    pub residuals: Vec<ResidualEquation>,
    pub variables: Vec<ResidualVariableRef>,
    pub parameters: Vec<ResidualParameterRef>,
    pub dependencies: Vec<(String, String)>,
}

impl ResidualGraph {
    pub fn from_assembly(assembly: &EquationAssembly) -> Self {
        let variables = assembly
            .unknowns
            .iter()
            .enumerate()
            .map(|(index, variable)| ResidualVariableRef {
                index,
                name: variable.name.clone(),
                role: variable.role.clone(),
                unit: variable.unit.clone(),
            })
            .collect::<Vec<_>>();
        let parameters = assembly
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| ResidualParameterRef {
                index,
                name: parameter.name.clone(),
                role: parameter.role.clone(),
                unit: parameter.unit.clone(),
            })
            .collect::<Vec<_>>();
        let variable_indices = variables
            .iter()
            .map(|variable| (variable.name.clone(), variable.index))
            .collect::<HashMap<_, _>>();
        let variable_units = assembly
            .unknowns
            .iter()
            .chain(assembly.parameters.iter())
            .map(|variable| {
                (
                    variable.name.clone(),
                    (variable.unit.clone(), variable.quantity_kind.clone()),
                )
            })
            .collect::<HashMap<_, _>>();
        let parameter_values = assembly
            .parameters
            .iter()
            .filter_map(|parameter| {
                parameter.value.map(|value| {
                    (
                        parameter.name.clone(),
                        ResidualConstantAlias {
                            value,
                            unit: parameter.unit.clone(),
                        },
                    )
                })
            })
            .collect::<HashMap<_, _>>();
        let residuals = assembly
            .generated_equations
            .iter()
            .map(|equation| {
                let (unit, quantity_kind) = equation
                    .dependencies
                    .first()
                    .and_then(|dependency| variable_units.get(dependency))
                    .cloned()
                    .unwrap_or_else(|| ("1".to_owned(), "unknown".to_owned()));
                let should_parse_component_residual = equation.kind == "component_equation"
                    || (equation.kind == "component_boundary" && equation.rhs_value.is_none());
                let mut lowering_failure = None;
                let parsed_component_equation = if should_parse_component_residual {
                    match lower_linear_residual_expression(
                        &equation.residual,
                        &equation.dependencies,
                        &variable_indices,
                        &variables,
                        &variable_units,
                        Some((&unit, &quantity_kind)),
                        Some(&parameter_values),
                        COMPONENT_RESIDUAL_LOWERING,
                    ) {
                        Ok(parsed) => Some(parsed),
                        Err(failure) => {
                            lowering_failure = Some(failure);
                            None
                        }
                    }
                } else {
                    None
                };
                let terms = parsed_component_equation
                    .as_ref()
                    .map(|parsed| parsed.terms.clone())
                    .unwrap_or_else(|| {
                        if should_parse_component_residual {
                            Vec::new()
                        } else {
                            residual_terms_for_generated_equation(
                                &equation.kind,
                                &equation.dependencies,
                                &variable_indices,
                            )
                        }
                    });
                let indices = terms
                    .iter()
                    .map(|term| term.variable_index)
                    .collect::<Vec<_>>();
                let scale = ResidualScale::from_quantity_unit(&quantity_kind, &unit);
                let inferred_unit = parsed_component_equation
                    .as_ref()
                    .and_then(|parsed| parsed.expression_unit.clone())
                    .unwrap_or_else(|| ResidualUnit {
                        unit: unit.clone(),
                        quantity_kind: quantity_kind.clone(),
                    });
                ResidualEquation {
                    name: equation.name.clone(),
                    expression: ResidualExpression {
                        text: equation.residual.clone(),
                        inferred_unit: Some(inferred_unit),
                        lowering_status: residual_expression_lowering_status(
                            should_parse_component_residual,
                            parsed_component_equation.is_some(),
                            lowering_failure.as_ref(),
                        )
                        .to_owned(),
                        lowering_failure_code: lowering_failure
                            .as_ref()
                            .map(|failure| failure.code.clone()),
                        lowering_failure_reason: lowering_failure
                            .as_ref()
                            .map(|failure| failure.message.clone()),
                    },
                    rhs_value: parsed_component_equation
                        .as_ref()
                        .map(|parsed| equation.rhs_value.unwrap_or(-parsed.constant))
                        .unwrap_or_else(|| equation.rhs_value.unwrap_or(0.0)),
                    unit: ResidualUnit {
                        unit,
                        quantity_kind,
                    },
                    scale,
                    source: ResidualSource {
                        expression: equation.expression.clone(),
                        line: equation.source_line,
                        generated_reason: Some(equation.reason.clone()),
                    },
                    variable_indices: indices,
                    terms,
                }
            })
            .collect::<Vec<_>>();
        let dependencies = assembly
            .generated_equations
            .iter()
            .flat_map(|equation| {
                equation
                    .dependencies
                    .iter()
                    .map(|dependency| (equation.name.clone(), dependency.clone()))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        Self {
            name: format!("{}.residual_graph", assembly.name),
            residuals,
            variables,
            parameters,
            dependencies,
        }
    }

    pub fn from_dynamic_component_assembly(
        assembly: &EquationAssembly,
    ) -> Result<Self, SolverFailure> {
        assembly.dynamic_component_split()?;

        let mut variables = Vec::new();
        let mut variable_aliases = HashMap::new();
        let mut variable_units = HashMap::new();

        for variable in &assembly.states {
            push_dynamic_residual_variable(
                &mut variables,
                &mut variable_aliases,
                &mut variable_units,
                DynamicResidualVariableSpec {
                    name: &variable.name,
                    role: "state",
                    quantity_kind: &variable.quantity_kind,
                    unit: &variable.unit,
                    aliases: &[variable.name.as_str()],
                },
            );
        }
        for variable in &assembly.algebraic_variables {
            push_dynamic_residual_variable(
                &mut variables,
                &mut variable_aliases,
                &mut variable_units,
                DynamicResidualVariableSpec {
                    name: &variable.name,
                    role: "algebraic",
                    quantity_kind: &variable.quantity_kind,
                    unit: &variable.unit,
                    aliases: &[variable.name.as_str()],
                },
            );
        }
        for variable in &assembly.inputs {
            push_dynamic_residual_variable(
                &mut variables,
                &mut variable_aliases,
                &mut variable_units,
                DynamicResidualVariableSpec {
                    name: &variable.name,
                    role: "input",
                    quantity_kind: &variable.quantity_kind,
                    unit: &variable.unit,
                    aliases: &[variable.name.as_str()],
                },
            );
        }
        for variable in &assembly.parameters {
            push_dynamic_residual_variable(
                &mut variables,
                &mut variable_aliases,
                &mut variable_units,
                DynamicResidualVariableSpec {
                    name: &variable.name,
                    role: "parameter",
                    quantity_kind: &variable.quantity_kind,
                    unit: &variable.unit,
                    aliases: &[variable.name.as_str()],
                },
            );
        }
        for state in &assembly.states {
            let derivative_name = dynamic_state_derivative_name(&state.name);
            let der_call_alias = format!("der({})", state.name);
            let differential_alias = format!("d{}", state.name);
            push_dynamic_residual_variable(
                &mut variables,
                &mut variable_aliases,
                &mut variable_units,
                DynamicResidualVariableSpec {
                    name: &derivative_name,
                    role: "state_derivative",
                    quantity_kind: &state.quantity_kind,
                    unit: &dynamic_derivative_unit(&state.unit),
                    aliases: &[
                        derivative_name.as_str(),
                        der_call_alias.as_str(),
                        differential_alias.as_str(),
                    ],
                },
            );
        }

        let parameters = assembly
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| ResidualParameterRef {
                index,
                name: parameter.name.clone(),
                role: parameter.role.clone(),
                unit: parameter.unit.clone(),
            })
            .collect::<Vec<_>>();
        let parameter_values = assembly
            .parameters
            .iter()
            .filter_map(|parameter| {
                parameter.value.map(|value| {
                    (
                        parameter.name.clone(),
                        ResidualConstantAlias {
                            value,
                            unit: parameter.unit.clone(),
                        },
                    )
                })
            })
            .collect::<HashMap<_, _>>();
        let residuals = assembly
            .generated_equations
            .iter()
            .map(|equation| {
                let (unit, quantity_kind) = dynamic_residual_unit(
                    equation
                        .dependencies
                        .first()
                        .map(String::as_str)
                        .unwrap_or_default(),
                    &variable_aliases,
                    &variables,
                    &variable_units,
                );
                let parsed = lower_linear_residual_expression(
                    &equation.residual,
                    &equation.dependencies,
                    &variable_aliases,
                    &variables,
                    &variable_units,
                    Some((&unit, &quantity_kind)),
                    Some(&parameter_values),
                    DYNAMIC_COMPONENT_RESIDUAL_LOWERING,
                )?;
                let rhs_value = dynamic_residual_rhs_value(equation, parsed.constant)?;
                let scale = ResidualScale::from_quantity_unit(&quantity_kind, &unit);

                let inferred_unit =
                    parsed
                        .expression_unit
                        .clone()
                        .unwrap_or_else(|| ResidualUnit {
                            unit: unit.clone(),
                            quantity_kind: quantity_kind.clone(),
                        });
                Ok(ResidualEquation {
                    name: equation.name.clone(),
                    expression: ResidualExpression {
                        text: equation.residual.clone(),
                        inferred_unit: Some(inferred_unit),
                        lowering_status: "linearized".to_owned(),
                        lowering_failure_code: None,
                        lowering_failure_reason: None,
                    },
                    rhs_value,
                    unit: ResidualUnit {
                        unit,
                        quantity_kind,
                    },
                    scale,
                    source: ResidualSource {
                        expression: equation.expression.clone(),
                        line: equation.source_line,
                        generated_reason: Some(equation.reason.clone()),
                    },
                    variable_indices: parsed
                        .terms
                        .iter()
                        .map(|term| term.variable_index)
                        .collect(),
                    terms: parsed.terms,
                })
            })
            .collect::<Result<Vec<_>, SolverFailure>>()?;
        let dependencies = assembly
            .generated_equations
            .iter()
            .flat_map(|equation| {
                equation
                    .dependencies
                    .iter()
                    .filter_map(|dependency| {
                        variable_aliases
                            .get(dependency)
                            .and_then(|index| variables.get(*index))
                            .map(|variable| (equation.name.clone(), variable.name.clone()))
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        Ok(Self {
            name: format!("{}.dynamic_component_residual_graph", assembly.name),
            residuals,
            variables,
            parameters,
            dependencies,
        })
    }

    pub fn assemble_linear_system(&self) -> Result<LinearResidualSystem, SolverFailure> {
        let equation_count = self.residuals.len();
        let unknown_count = self.variables.len();
        if equation_count == 0 || unknown_count == 0 || equation_count != unknown_count {
            return Err(SolverFailure::new(
                "E-LINEAR-RESIDUAL-SHAPE",
                format!(
                    "linear residual solve requires a non-empty square residual system, got {equation_count} residual(s) and {unknown_count} unknown(s)"
                ),
            ));
        }

        let mut matrix = vec![vec![0.0; unknown_count]; equation_count];
        for (row_index, residual) in self.residuals.iter().enumerate() {
            if !residual.rhs_value.is_finite() {
                return Err(SolverFailure::new(
                    "E-LINEAR-RESIDUAL-FINITE",
                    format!("residual `{}` has a non-finite RHS value", residual.name),
                ));
            }
            ensure_residual_expression_lowered(residual)?;
            for term in &residual.terms {
                if term.variable_index >= unknown_count {
                    return Err(SolverFailure::new(
                        "E-LINEAR-RESIDUAL-INDEX",
                        format!(
                            "residual `{}` references variable index {} outside {} unknown(s)",
                            residual.name, term.variable_index, unknown_count
                        ),
                    ));
                }
                if !term.coefficient.is_finite() {
                    return Err(SolverFailure::new(
                        "E-LINEAR-RESIDUAL-FINITE",
                        format!(
                            "residual `{}` term for `{}` has a non-finite coefficient",
                            residual.name, term.variable
                        ),
                    ));
                }
                matrix[row_index][term.variable_index] += term.coefficient;
            }
        }

        Ok(LinearResidualSystem {
            matrix,
            rhs: self
                .residuals
                .iter()
                .map(|residual| residual.rhs_value)
                .collect(),
            residual_names: self
                .residuals
                .iter()
                .map(|residual| residual.name.clone())
                .collect(),
            variable_names: self
                .variables
                .iter()
                .map(|variable| variable.name.clone())
                .collect(),
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct LoweredLinearResidualExpression {
    terms: Vec<ResidualTerm>,
    constant: f64,
    expression_unit: Option<ResidualUnit>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResidualExpressionLoweringProfile {
    expression_profile: ArithmeticExpressionProfile,
    failure_code: &'static str,
    residual_label: &'static str,
    term_label: &'static str,
    assembly_label: &'static str,
}

const COMPONENT_RESIDUAL_LOWERING: ResidualExpressionLoweringProfile =
    ResidualExpressionLoweringProfile {
        expression_profile: ArithmeticExpressionProfile::COMPONENT_RESIDUAL,
        failure_code: "E-COMPONENT-ASSEMBLY-RESIDUAL",
        residual_label: "component residual",
        term_label: "component residual term",
        assembly_label: "component assembly",
    };

const DYNAMIC_COMPONENT_RESIDUAL_LOWERING: ResidualExpressionLoweringProfile =
    ResidualExpressionLoweringProfile {
        expression_profile: ArithmeticExpressionProfile::DYNAMIC_COMPONENT_RESIDUAL,
        failure_code: "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL",
        residual_label: "dynamic component residual",
        term_label: "dynamic component residual term",
        assembly_label: "dynamic component assembly",
    };

#[derive(Clone, Debug, PartialEq)]
struct ResidualConstantAlias {
    value: f64,
    unit: String,
}

struct DynamicResidualVariableSpec<'a> {
    name: &'a str,
    role: &'a str,
    quantity_kind: &'a str,
    unit: &'a str,
    aliases: &'a [&'a str],
}

fn push_dynamic_residual_variable(
    variables: &mut Vec<ResidualVariableRef>,
    variable_aliases: &mut HashMap<String, usize>,
    variable_units: &mut HashMap<String, (String, String)>,
    spec: DynamicResidualVariableSpec<'_>,
) {
    let index = variables.len();
    variables.push(ResidualVariableRef {
        index,
        name: spec.name.to_owned(),
        role: spec.role.to_owned(),
        unit: spec.unit.to_owned(),
    });
    variable_units.insert(
        spec.name.to_owned(),
        (spec.unit.to_owned(), spec.quantity_kind.to_owned()),
    );
    for alias in spec.aliases {
        variable_aliases.insert((*alias).to_owned(), index);
    }
}

fn dynamic_state_derivative_name(state_name: &str) -> String {
    format!("der_{state_name}")
}

fn dynamic_derivative_unit(unit: &str) -> String {
    let unit = unit.trim();
    if unit.is_empty() || unit == "1" {
        "1/s".to_owned()
    } else {
        format!("{unit}/s")
    }
}

fn dynamic_residual_unit(
    dependency: &str,
    variable_aliases: &HashMap<String, usize>,
    variables: &[ResidualVariableRef],
    variable_units: &HashMap<String, (String, String)>,
) -> (String, String) {
    variable_aliases
        .get(dependency)
        .and_then(|index| variables.get(*index))
        .and_then(|variable| variable_units.get(&variable.name))
        .cloned()
        .unwrap_or_else(|| ("1".to_owned(), "unknown".to_owned()))
}

fn dynamic_residual_rhs_value(
    equation: &super::assembly::GeneratedEquation,
    constant: f64,
) -> Result<f64, SolverFailure> {
    if !constant.is_finite() {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL",
            format!(
                "dynamic component assembly residual `{}` has a non-finite constant",
                equation.name
            ),
        ));
    }
    let inferred_rhs = -constant;
    if let Some(rhs_value) = equation.rhs_value {
        if !rhs_value.is_finite() {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL",
                format!(
                    "dynamic component assembly residual `{}` has a non-finite RHS value",
                    equation.name
                ),
            ));
        }
        if constant.abs() > 1e-12 && (rhs_value - inferred_rhs).abs() > 1e-9 {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL",
                format!(
                    "dynamic component assembly residual `{}` has inconsistent literal RHS metadata",
                    equation.name
                ),
            ));
        }
        Ok(rhs_value)
    } else {
        Ok(inferred_rhs)
    }
}

fn lower_linear_residual_expression(
    expression: &str,
    dependencies: &[String],
    variable_aliases: &HashMap<String, usize>,
    variables: &[ResidualVariableRef],
    variable_units: &HashMap<String, (String, String)>,
    residual_unit: Option<(&str, &str)>,
    constant_aliases: Option<&HashMap<String, ResidualConstantAlias>>,
    profile: ResidualExpressionLoweringProfile,
) -> Result<LoweredLinearResidualExpression, SolverFailure> {
    let dependency_indices = dependencies
        .iter()
        .filter_map(|dependency| variable_aliases.get(dependency).copied())
        .collect::<HashSet<_>>();
    if dependency_indices.is_empty() {
        return Err(SolverFailure::new(
            profile.failure_code,
            format!(
                "{} `{expression}` has no solver dependencies",
                profile.residual_label
            ),
        ));
    }

    let variable_symbols = dependencies
        .iter()
        .filter(|dependency| variable_aliases.contains_key(*dependency))
        .cloned()
        .collect::<Vec<_>>();
    let constant_symbols =
        dynamic_constant_symbols(expression, constant_aliases, residual_unit, profile)?;
    let symbol_units = dynamic_expression_symbol_metadata(
        variable_aliases,
        variables,
        variable_units,
        constant_aliases,
    );
    let mut convert_number = |value: f64, unit: Option<&str>| match unit {
        Some(unit) => convert_residual_constant(value, unit, residual_unit, profile),
        None => Ok(value),
    };
    let linearized = linearize_arithmetic_expression_with_symbol_metadata_and_unit_converter(
        expression,
        &variable_symbols,
        &constant_symbols,
        &symbol_units,
        &mut convert_number,
        profile.expression_profile,
        1e-9,
    )
    .map_err(|failure| linear_residual_lowering_failure(expression, failure, profile))?;

    let mut parsed = LoweredLinearResidualExpression {
        constant: linearized.constant,
        terms: Vec::new(),
        expression_unit: linearized
            .root_unit
            .as_ref()
            .map(residual_unit_from_arithmetic_metadata),
    };
    for term in linearized.terms {
        let Some(index) = variable_aliases.get(&term.symbol).copied() else {
            return Err(unsupported_linear_residual_term(&term.symbol, profile));
        };
        push_linear_residual_term(
            &term.symbol,
            index,
            term.coefficient,
            &dependency_indices,
            variables,
            &mut parsed,
            profile,
        )?;
    }

    if parsed.terms.is_empty() {
        return Err(SolverFailure::new(
            profile.failure_code,
            format!(
                "{} `{expression}` has no linear variable terms",
                profile.residual_label
            ),
        ));
    }
    Ok(parsed)
}

fn dynamic_expression_symbol_metadata(
    variable_aliases: &HashMap<String, usize>,
    variables: &[ResidualVariableRef],
    variable_units: &HashMap<String, (String, String)>,
    constant_aliases: Option<&HashMap<String, ResidualConstantAlias>>,
) -> HashMap<String, ArithmeticUnitMetadata> {
    let mut metadata = HashMap::new();
    for (alias, index) in variable_aliases {
        let Some(variable) = variables.get(*index) else {
            continue;
        };
        let Some((unit, quantity_kind)) = variable_units.get(&variable.name) else {
            continue;
        };
        metadata.insert(
            alias.clone(),
            arithmetic_metadata_for_residual_symbol(unit, Some(quantity_kind)),
        );
    }
    if let Some(constant_aliases) = constant_aliases {
        for (name, alias) in constant_aliases {
            metadata.insert(
                name.clone(),
                arithmetic_metadata_for_residual_symbol(&alias.unit, None),
            );
        }
    }
    metadata
}

fn arithmetic_metadata_for_residual_symbol(
    unit: &str,
    quantity_kind: Option<&str>,
) -> ArithmeticUnitMetadata {
    let display_unit = if unit.trim().is_empty() {
        "1"
    } else {
        unit.trim()
    };
    if normalize_unit(display_unit) == "1" {
        return ArithmeticUnitMetadata {
            display_unit: "1".to_owned(),
            canonical_unit: "1".to_owned(),
            quantity_kind: quantity_kind.unwrap_or("DimensionlessNumber").to_owned(),
        };
    }
    let info = residual_unit_info(display_unit);
    ArithmeticUnitMetadata {
        display_unit: display_unit.to_owned(),
        canonical_unit: info
            .map(|info| info.canonical_unit.to_owned())
            .unwrap_or_else(|| display_unit.to_owned()),
        quantity_kind: quantity_kind
            .map(str::to_owned)
            .or_else(|| info.map(|info| info.quantity_hint.to_owned()))
            .unwrap_or_else(|| "unknown".to_owned()),
    }
}

fn residual_unit_from_arithmetic_metadata(metadata: &ArithmeticUnitMetadata) -> ResidualUnit {
    ResidualUnit {
        unit: metadata.canonical_unit.clone(),
        quantity_kind: metadata.quantity_kind.clone(),
    }
}
fn dynamic_constant_symbols(
    expression: &str,
    constant_aliases: Option<&HashMap<String, ResidualConstantAlias>>,
    residual_unit: Option<(&str, &str)>,
    profile: ResidualExpressionLoweringProfile,
) -> Result<HashMap<String, f64>, SolverFailure> {
    let mut symbols = HashMap::new();
    if let Some(constant_aliases) = constant_aliases {
        for (name, alias) in constant_aliases {
            if !expression_mentions_symbol(expression, name) {
                continue;
            }
            symbols.insert(
                name.clone(),
                convert_residual_parameter_constant(
                    alias.value,
                    &alias.unit,
                    residual_unit,
                    profile,
                )?,
            );
        }
    }
    Ok(symbols)
}

fn convert_residual_parameter_constant(
    value: f64,
    source_unit: &str,
    residual_unit: Option<(&str, &str)>,
    profile: ResidualExpressionLoweringProfile,
) -> Result<f64, SolverFailure> {
    let Some((target_unit, _quantity_kind)) = residual_unit else {
        return Ok(value);
    };
    if normalize_unit(source_unit) == "1" {
        return Ok(value);
    }
    let Some(source_info) = residual_unit_info(source_unit) else {
        return Err(unsupported_linear_residual_term(
            &format!("{value} {source_unit}"),
            profile,
        ));
    };
    let Some(target_info) = residual_unit_info(target_unit) else {
        return Err(unsupported_linear_residual_term(
            &format!("{value} {source_unit}"),
            profile,
        ));
    };
    if normalize_unit(source_info.canonical_unit) == normalize_unit(target_info.canonical_unit) {
        return convert_residual_constant(value, source_unit, residual_unit, profile);
    }
    if let Some(converted) = convert_residual_compound_parameter_numerator(
        value,
        source_info,
        target_unit,
        target_info,
        profile,
    )? {
        return Ok(converted);
    }
    Ok(value)
}

fn convert_residual_compound_parameter_numerator(
    value: f64,
    source_info: UnitInfo,
    target_unit: &str,
    target_info: UnitInfo,
    profile: ResidualExpressionLoweringProfile,
) -> Result<Option<f64>, SolverFailure> {
    let Some((source_numerator, _source_denominator)) = source_info.symbol.split_once('/') else {
        return Ok(None);
    };
    let Some(source_numerator_info) = residual_unit_info(source_numerator.trim()) else {
        return Ok(None);
    };
    if normalize_unit(source_numerator_info.canonical_unit)
        != normalize_unit(target_info.canonical_unit)
    {
        return Ok(None);
    }
    convert_residual_constant(
        value,
        source_numerator_info.symbol,
        Some((target_unit, target_info.quantity_hint)),
        profile,
    )
    .map(Some)
}
fn expression_mentions_symbol(expression: &str, symbol: &str) -> bool {
    expression
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || character == '_' || character == '.')
        })
        .any(|token| token == symbol)
}
fn linear_residual_lowering_failure(
    expression: &str,
    failure: SolverFailure,
    profile: ResidualExpressionLoweringProfile,
) -> SolverFailure {
    if failure.code == profile.failure_code
        && (failure.message.contains("not linear")
            || failure.message.contains("division by zero")
            || failure.message.contains("unknown symbol"))
    {
        unsupported_linear_residual_term(expression, profile)
    } else {
        failure
    }
}
fn residual_unit_info(unit: &str) -> Option<UnitInfo> {
    let normalized = normalize_unit(unit);
    all_unit_infos()
        .iter()
        .find(|info| normalize_unit(info.symbol) == normalized)
        .copied()
}
fn convert_residual_constant(
    value: f64,
    source_unit: &str,
    residual_unit: Option<(&str, &str)>,
    profile: ResidualExpressionLoweringProfile,
) -> Result<f64, SolverFailure> {
    let Some((target_unit, _quantity_kind)) = residual_unit else {
        return Ok(value);
    };
    let normalized_source = normalize_unit(source_unit);
    let normalized_target = normalize_unit(target_unit);
    if normalized_source == normalized_target {
        return Ok(value);
    }
    let Some(source_info) = residual_unit_info(source_unit) else {
        return Err(unsupported_linear_residual_term(
            &format!("{value} {source_unit}"),
            profile,
        ));
    };
    let Some(target_info) = residual_unit_info(target_unit) else {
        return Err(unsupported_linear_residual_term(
            &format!("{value} {source_unit}"),
            profile,
        ));
    };
    if source_info.affine_offset.is_some() || target_info.affine_offset.is_some() {
        return Err(unsupported_linear_residual_term(
            &format!("{value} {source_unit}"),
            profile,
        ));
    }
    if normalize_unit(source_info.canonical_unit) != normalize_unit(target_info.canonical_unit) {
        return Err(unsupported_linear_residual_term(
            &format!("{value} {source_unit}"),
            profile,
        ));
    }
    let source_scale = source_info.scale_to_canonical.parse::<f64>().map_err(|_| {
        SolverFailure::new(
            profile.failure_code,
            format!("unit `{source_unit}` has an invalid residual conversion scale"),
        )
    })?;
    let target_scale = target_info.scale_to_canonical.parse::<f64>().map_err(|_| {
        SolverFailure::new(
            profile.failure_code,
            format!("unit `{target_unit}` has an invalid residual conversion scale"),
        )
    })?;
    Ok(value * source_scale / target_scale)
}

fn push_linear_residual_term(
    label: &str,
    index: usize,
    coefficient: f64,
    dependency_indices: &HashSet<usize>,
    variables: &[ResidualVariableRef],
    parsed: &mut LoweredLinearResidualExpression,
    profile: ResidualExpressionLoweringProfile,
) -> Result<(), SolverFailure> {
    if !dependency_indices.contains(&index) {
        return Err(SolverFailure::new(
            profile.failure_code,
            format!(
                "{} `{label}` is not listed as a dependency",
                profile.term_label
            ),
        ));
    }
    if !coefficient.is_finite() {
        return Err(SolverFailure::new(
            profile.failure_code,
            format!(
                "{} `{label}` has a non-finite coefficient",
                profile.term_label
            ),
        ));
    }
    let Some(variable) = variables.get(index) else {
        return Err(SolverFailure::new(
            profile.failure_code,
            format!(
                "{} `{label}` references an unknown variable",
                profile.term_label
            ),
        ));
    };
    parsed.terms.push(ResidualTerm {
        variable_index: index,
        variable: variable.name.clone(),
        coefficient,
    });
    Ok(())
}

fn unsupported_linear_residual_term(
    term: &str,
    profile: ResidualExpressionLoweringProfile,
) -> SolverFailure {
    SolverFailure::new(
        profile.failure_code,
        format!(
            "{} supports only simple linear residual terms; unsupported term `{term}`",
            profile.assembly_label
        ),
    )
}

fn residual_terms_for_generated_equation(
    kind: &str,
    dependencies: &[String],
    variable_indices: &HashMap<String, usize>,
) -> Vec<ResidualTerm> {
    dependencies
        .iter()
        .enumerate()
        .filter_map(|(index, variable)| {
            let variable_index = variable_indices.get(variable).copied()?;
            let coefficient = match kind {
                "across_equality" if index == 1 => -1.0,
                _ => 1.0,
            };
            Some(ResidualTerm {
                variable_index,
                variable: variable.clone(),
                coefficient,
            })
        })
        .collect()
}

pub trait ResidualEvaluator {
    fn evaluate(&self, input: &ResidualInput<'_>) -> Result<ResidualOutput, SolverFailure>;
}

impl ResidualEvaluator for ResidualGraph {
    fn evaluate(&self, input: &ResidualInput<'_>) -> Result<ResidualOutput, SolverFailure> {
        let tolerance = input.tolerance.unwrap_or(DEFAULT_RESIDUAL_TOLERANCE);
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(SolverFailure::new(
                "E-RESIDUAL-TOLERANCE-001",
                "residual evaluator tolerance must be a positive finite number",
            ));
        }
        ensure_finite_values(
            "E-RESIDUAL-INPUT-FINITE",
            "residual evaluator input",
            input.values,
        )?;

        let mut values = Vec::with_capacity(self.residuals.len());
        for residual in &self.residuals {
            if !residual.rhs_value.is_finite() {
                return Err(SolverFailure::new(
                    "E-RESIDUAL-FINITE",
                    format!("residual `{}` has a non-finite RHS value", residual.name),
                ));
            }
            ensure_residual_expression_lowered(residual)?;
            let mut value = -residual.rhs_value;
            for term in &residual.terms {
                if !term.coefficient.is_finite() {
                    return Err(SolverFailure::new(
                        "E-RESIDUAL-FINITE",
                        format!(
                            "residual `{}` term for `{}` has a non-finite coefficient",
                            residual.name, term.variable
                        ),
                    ));
                }
                value += term.coefficient
                    * input
                        .values
                        .get(term.variable_index)
                        .copied()
                        .unwrap_or_default();
            }
            if !value.is_finite() {
                return Err(SolverFailure::new(
                    "E-RESIDUAL-FINITE",
                    format!(
                        "residual `{}` evaluated to a non-finite value",
                        residual.name
                    ),
                ));
            }
            let scale = input
                .scale_overrides
                .iter()
                .find(|scale| scale.residual == residual.name)
                .map(|scale| scale.scale.value)
                .unwrap_or(residual.scale.value);
            if !scale.is_finite() || scale <= 0.0 {
                return Err(SolverFailure::new(
                    "E-RESIDUAL-SCALE-001",
                    format!(
                        "residual `{}` scale must be a positive finite number",
                        residual.name
                    ),
                ));
            }
            let normalized_value = value / scale.max(f64::EPSILON);
            if !normalized_value.is_finite() {
                return Err(SolverFailure::new(
                    "E-RESIDUAL-FINITE",
                    format!(
                        "residual `{}` normalized value is non-finite",
                        residual.name
                    ),
                ));
            }
            let status = if normalized_value.abs() <= tolerance {
                "satisfied"
            } else {
                "unsatisfied"
            };
            values.push(NamedResidualValue {
                name: residual.name.clone(),
                value,
                normalized_value,
                status: status.to_owned(),
            });
        }
        let normalized_residuals = values
            .iter()
            .map(|value| value.normalized_value)
            .collect::<Vec<_>>();
        let residual_norm = euclidean_norm(&normalized_residuals);
        if !residual_norm.is_finite() {
            return Err(SolverFailure::new(
                "E-RESIDUAL-FINITE",
                "residual norm is non-finite",
            ));
        }
        Ok(ResidualOutput {
            values,
            residual_norm,
            tolerance,
        })
    }
}

impl super::evaluator::ResidualEvaluator for ResidualGraph {
    fn evaluate(
        &self,
        input: &super::evaluator::ResidualInput,
    ) -> Result<super::evaluator::ResidualOutput, SolverFailure> {
        let values = self.values_from_structured_input(input)?;
        let output =
            <Self as ResidualEvaluator>::evaluate(self, &ResidualInput::new(values.as_slice()))?;
        Ok(super::evaluator::ResidualOutput {
            residuals: output.values.iter().map(|value| value.value).collect(),
            named_residuals: output
                .values
                .iter()
                .map(|value| super::evaluator::NamedResidualValue {
                    name: value.name.clone(),
                    value: value.value,
                    normalized_value: value.normalized_value,
                })
                .collect(),
        })
    }
}

fn residual_expression_lowering_status(
    attempted_lowering: bool,
    lowered: bool,
    failure: Option<&SolverFailure>,
) -> &'static str {
    if failure.is_some() {
        "unsupported_linearization"
    } else if lowered {
        "linearized"
    } else if attempted_lowering {
        "not_linearized"
    } else {
        "generated_linear_terms"
    }
}

fn ensure_residual_expression_lowered(residual: &ResidualEquation) -> Result<(), SolverFailure> {
    if residual.expression.lowering_status == "unsupported_linearization" {
        let code = residual
            .expression
            .lowering_failure_code
            .as_deref()
            .unwrap_or("E-RESIDUAL-LOWERING");
        let reason = residual
            .expression
            .lowering_failure_reason
            .as_deref()
            .unwrap_or("residual expression could not be lowered into supported solver terms");
        return Err(SolverFailure::new(
            code,
            format!(
                "residual `{}` could not be lowered into supported solver terms: {}",
                residual.name, reason
            ),
        ));
    }
    Ok(())
}
fn ensure_finite_values(code: &str, label: &str, values: &[f64]) -> Result<(), SolverFailure> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(SolverFailure::new(
            code,
            format!("{label} vector contains a non-finite value"),
        ))
    }
}

impl ResidualGraph {
    fn values_from_structured_input(
        &self,
        input: &super::evaluator::ResidualInput,
    ) -> Result<Vec<f64>, SolverFailure> {
        let mut state_index = 0;
        let mut derivative_index = 0;
        let mut algebraic_index = 0;
        let mut input_index = 0;
        let mut parameter_index = 0;
        self.variables
            .iter()
            .map(|variable| {
                let value = match variable.role.as_str() {
                    "state" => {
                        let value = input.x.get(state_index).copied();
                        state_index += 1;
                        value
                    }
                    "derivative" | "state_derivative" | "xdot" => {
                        let value = input
                            .xdot
                            .as_ref()
                            .and_then(|values| values.get(derivative_index))
                            .copied();
                        derivative_index += 1;
                        value
                    }
                    "algebraic" => {
                        let value = input.z.get(algebraic_index).copied();
                        algebraic_index += 1;
                        value
                    }
                    "input" => {
                        let value = input.u.get(input_index).copied();
                        input_index += 1;
                        value
                    }
                    "parameter" => {
                        let value = input.p.get(parameter_index).copied();
                        parameter_index += 1;
                        value
                    }
                    "time" => Some(input.t),
                    _ => input.z.get(variable.index).copied(),
                };
                value.ok_or_else(|| {
                    SolverFailure::new(
                        "E-RESIDUAL-INPUT-LAYOUT",
                        format!(
                            "residual variable `{}` with role `{}` has no matching structured input value",
                            variable.name, variable.role
                        ),
                    )
                })
            })
            .collect()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualEquation {
    pub name: String,
    pub expression: ResidualExpression,
    pub rhs_value: f64,
    pub unit: ResidualUnit,
    pub scale: ResidualScale,
    pub source: ResidualSource,
    pub variable_indices: Vec<usize>,
    pub terms: Vec<ResidualTerm>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResidualTerm {
    pub variable_index: usize,
    pub variable: String,
    pub coefficient: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResidualInput<'a> {
    pub values: &'a [f64],
    pub scale_overrides: &'a [ResidualScaleOverride],
    pub tolerance: Option<f64>,
}

impl<'a> ResidualInput<'a> {
    pub fn new(values: &'a [f64]) -> Self {
        Self {
            values,
            scale_overrides: &[],
            tolerance: None,
        }
    }

    pub fn with_scale_overrides(mut self, scale_overrides: &'a [ResidualScaleOverride]) -> Self {
        self.scale_overrides = scale_overrides;
        self
    }

    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        if tolerance.is_finite() && tolerance > 0.0 {
            self.tolerance = Some(tolerance);
        }
        self
    }

    pub fn try_with_tolerance(mut self, tolerance: f64) -> Result<Self, SolverFailure> {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(SolverFailure::new(
                "E-RESIDUAL-TOLERANCE-001",
                "user-provided residual tolerance must be a positive finite number",
            ));
        }
        self.tolerance = Some(tolerance);
        Ok(self)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualOutput {
    pub values: Vec<NamedResidualValue>,
    pub residual_norm: f64,
    pub tolerance: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NamedResidualValue {
    pub name: String,
    pub value: f64,
    pub normalized_value: f64,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinearResidualSystem {
    pub matrix: Vec<Vec<f64>>,
    pub rhs: Vec<f64>,
    pub residual_names: Vec<String>,
    pub variable_names: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualVariableRef {
    pub index: usize,
    pub name: String,
    pub role: String,
    pub unit: String,
}

pub type ResidualParameterRef = ResidualVariableRef;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualExpression {
    pub text: String,
    pub inferred_unit: Option<ResidualUnit>,
    pub lowering_status: String,
    pub lowering_failure_code: Option<String>,
    pub lowering_failure_reason: Option<String>,
}

impl ResidualExpression {
    pub fn manual(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            inferred_unit: None,
            lowering_status: "manual".to_owned(),
            lowering_failure_code: None,
            lowering_failure_reason: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResidualScale {
    pub value: f64,
    pub policy: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResidualScaleOverride {
    pub residual: String,
    pub scale: ResidualScale,
}

impl Default for ResidualScale {
    fn default() -> Self {
        Self {
            value: 1.0,
            policy: "unit_default:dimensionless[1]".to_owned(),
        }
    }
}

impl ResidualScale {
    pub fn user_provided(value: f64, label: &str) -> Result<Self, SolverFailure> {
        if !value.is_finite() || value <= 0.0 {
            return Err(SolverFailure::new(
                "E-RESIDUAL-SCALE-001",
                "user-provided residual scale must be a positive finite number",
            ));
        }
        let label = label.trim();
        let label = if label.is_empty() { "unnamed" } else { label };
        Ok(Self {
            value,
            policy: format!("user_provided:{label}"),
        })
    }

    pub fn from_quantity_unit(quantity_kind: &str, unit: &str) -> Self {
        let trimmed_unit = unit.trim();
        let normalized_unit = trimmed_unit.to_ascii_lowercase();
        let normalized_quantity = quantity_kind.trim().to_ascii_lowercase();
        let value = match normalized_unit.as_str() {
            "kw" => 1.0,
            "w" if matches!(
                normalized_quantity.as_str(),
                "heatrate" | "mechanicalpower" | "power"
            ) =>
            {
                1000.0
            }
            "k" | "degc" | "c" => 1.0,
            "m" | "kg/s" | "1" | "" => 1.0,
            _ if normalized_quantity.contains("pressure") => 1000.0,
            _ if normalized_quantity.contains("energy") => 1000.0,
            _ => 1.0,
        };
        let unit_label = if trimmed_unit.is_empty() {
            "1"
        } else {
            trimmed_unit
        };
        let quantity_label = if quantity_kind.trim().is_empty() {
            "unknown"
        } else {
            quantity_kind.trim()
        };
        Self {
            value,
            policy: format!("unit_default:{quantity_label}[{unit_label}]"),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualUnit {
    pub unit: String,
    pub quantity_kind: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualSource {
    pub expression: String,
    pub line: Option<usize>,
    pub generated_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::super::assembly::{EquationAssembly, GeneratedEquation, UnknownVariable};
    use super::*;

    #[test]
    fn assembles_square_linear_residual_system() {
        let graph = ResidualGraph {
            name: "test.residual_graph".to_owned(),
            variables: vec![
                ResidualVariableRef {
                    index: 0,
                    name: "x".to_owned(),
                    role: "algebraic".to_owned(),
                    unit: "1".to_owned(),
                },
                ResidualVariableRef {
                    index: 1,
                    name: "y".to_owned(),
                    role: "algebraic".to_owned(),
                    unit: "1".to_owned(),
                },
            ],
            residuals: vec![
                residual("r1", &[(0, "x", 1.0), (1, "y", -1.0)]),
                residual("r2", &[(0, "x", 1.0), (1, "y", 1.0)]),
            ],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        };

        let system = graph.assemble_linear_system().unwrap();

        assert_eq!(system.variable_names, vec!["x", "y"]);
        assert_eq!(system.residual_names, vec!["r1", "r2"]);
        assert_eq!(system.matrix, vec![vec![1.0, -1.0], vec![1.0, 1.0]]);
        assert_eq!(system.rhs, vec![0.0, 0.0]);
    }

    #[test]
    fn rejects_non_square_linear_residual_system() {
        let graph = ResidualGraph {
            name: "test.residual_graph".to_owned(),
            variables: vec![
                ResidualVariableRef {
                    index: 0,
                    name: "x".to_owned(),
                    role: "algebraic".to_owned(),
                    unit: "1".to_owned(),
                },
                ResidualVariableRef {
                    index: 1,
                    name: "y".to_owned(),
                    role: "algebraic".to_owned(),
                    unit: "1".to_owned(),
                },
            ],
            residuals: vec![residual("r1", &[(0, "x", 1.0), (1, "y", 1.0)])],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        };

        let failure = graph.assemble_linear_system().unwrap_err();

        assert_eq!(failure.code, "E-LINEAR-RESIDUAL-SHAPE");
    }

    #[test]
    fn rejects_nonfinite_linear_residual_system_values() {
        let graph = ResidualGraph {
            name: "test.residual_graph".to_owned(),
            variables: vec![ResidualVariableRef {
                index: 0,
                name: "x".to_owned(),
                role: "algebraic".to_owned(),
                unit: "1".to_owned(),
            }],
            residuals: vec![residual("r1", &[(0, "x", f64::NAN)])],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        };
        let failure = graph.assemble_linear_system().unwrap_err();
        assert_eq!(failure.code, "E-LINEAR-RESIDUAL-FINITE");

        let graph = ResidualGraph {
            name: "test.residual_graph".to_owned(),
            variables: vec![ResidualVariableRef {
                index: 0,
                name: "x".to_owned(),
                role: "algebraic".to_owned(),
                unit: "1".to_owned(),
            }],
            residuals: vec![residual_with_rhs("r1", &[(0, "x", 1.0)], f64::INFINITY)],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        };
        let failure = graph.assemble_linear_system().unwrap_err();
        assert_eq!(failure.code, "E-LINEAR-RESIDUAL-FINITE");
    }

    #[test]
    fn linear_residual_system_and_evaluator_use_rhs_values() {
        let graph = ResidualGraph {
            name: "test.residual_graph".to_owned(),
            variables: vec![ResidualVariableRef {
                index: 0,
                name: "x".to_owned(),
                role: "algebraic".to_owned(),
                unit: "1".to_owned(),
            }],
            residuals: vec![residual_with_rhs("r1", &[(0, "x", 1.0)], 4.0)],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        };

        let system = graph.assemble_linear_system().unwrap();
        assert_eq!(system.matrix, vec![vec![1.0]]);
        assert_eq!(system.rhs, vec![4.0]);

        let output =
            <ResidualGraph as ResidualEvaluator>::evaluate(&graph, &ResidualInput::new(&[5.0]))
                .unwrap();
        assert_eq!(output.values[0].value, 1.0);
    }

    #[test]
    fn component_equations_from_assembly_preserve_linear_terms() {
        let assembly = EquationAssembly {
            name: "component_graph".to_owned(),
            unknowns: vec![
                UnknownVariable {
                    name: "x".to_owned(),
                    role: "algebraic".to_owned(),
                    quantity_kind: "Dimensionless".to_owned(),
                    unit: "1".to_owned(),
                    source: "node.x".to_owned(),
                    status: "classified".to_owned(),
                    value: None,
                },
                UnknownVariable {
                    name: "y".to_owned(),
                    role: "algebraic".to_owned(),
                    quantity_kind: "Dimensionless".to_owned(),
                    unit: "1".to_owned(),
                    source: "node.y".to_owned(),
                    status: "classified".to_owned(),
                    value: None,
                },
            ],
            generated_equations: vec![
                GeneratedEquation {
                    name: "component.eq1".to_owned(),
                    kind: "component_equation".to_owned(),
                    residual: "x - 2 * y".to_owned(),
                    rhs_value: Some(5.0),
                    dependencies: vec!["x".to_owned(), "y".to_owned()],
                    ..Default::default()
                },
                GeneratedEquation {
                    name: "component.eq2".to_owned(),
                    kind: "component_equation".to_owned(),
                    residual: "y".to_owned(),
                    rhs_value: Some(1.0),
                    dependencies: vec!["y".to_owned()],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let graph = ResidualGraph::from_assembly(&assembly);
        let system = graph.assemble_linear_system().unwrap();

        assert_eq!(system.matrix, vec![vec![1.0, -2.0], vec![0.0, 1.0]]);
        assert_eq!(system.rhs, vec![5.0, 1.0]);

        let output = <ResidualGraph as ResidualEvaluator>::evaluate(
            &graph,
            &ResidualInput::new(&[7.0, 1.0]),
        )
        .unwrap();
        assert!(output.values.iter().all(|residual| residual.value == 0.0));
    }

    #[test]
    fn unsupported_component_equations_do_not_invent_linear_terms() {
        let assembly = EquationAssembly {
            name: "component_graph".to_owned(),
            unknowns: vec![
                UnknownVariable {
                    name: "x".to_owned(),
                    role: "algebraic".to_owned(),
                    quantity_kind: "Dimensionless".to_owned(),
                    unit: "1".to_owned(),
                    source: "node.x".to_owned(),
                    status: "classified".to_owned(),
                    value: None,
                },
                UnknownVariable {
                    name: "y".to_owned(),
                    role: "algebraic".to_owned(),
                    quantity_kind: "Dimensionless".to_owned(),
                    unit: "1".to_owned(),
                    source: "node.y".to_owned(),
                    status: "classified".to_owned(),
                    value: None,
                },
            ],
            generated_equations: vec![
                GeneratedEquation {
                    name: "component.nonlinear".to_owned(),
                    kind: "component_equation".to_owned(),
                    residual: "x * y".to_owned(),
                    dependencies: vec!["x".to_owned(), "y".to_owned()],
                    ..Default::default()
                },
                GeneratedEquation {
                    name: "component.boundary".to_owned(),
                    kind: "component_boundary".to_owned(),
                    residual: "y".to_owned(),
                    rhs_value: Some(1.0),
                    dependencies: vec!["y".to_owned()],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let graph = ResidualGraph::from_assembly(&assembly);

        assert!(graph.residuals[0].terms.is_empty());
        assert!(graph.residuals[0].variable_indices.is_empty());
        assert_eq!(
            graph.residuals[0].expression.lowering_status,
            "unsupported_linearization"
        );
        assert_eq!(
            graph.residuals[0]
                .expression
                .lowering_failure_code
                .as_deref(),
            Some("E-COMPONENT-ASSEMBLY-RESIDUAL")
        );

        let linear_failure = graph.assemble_linear_system().unwrap_err();
        assert_eq!(linear_failure.code, "E-COMPONENT-ASSEMBLY-RESIDUAL");
        assert!(linear_failure.message.contains("component.nonlinear"));

        let evaluation_failure = <ResidualGraph as ResidualEvaluator>::evaluate(
            &graph,
            &ResidualInput::new(&[1.0, 2.0]),
        )
        .unwrap_err();
        assert_eq!(evaluation_failure.code, "E-COMPONENT-ASSEMBLY-RESIDUAL");
        assert!(evaluation_failure.message.contains("component.nonlinear"));
    }
    #[test]
    fn shared_linear_residual_lowering_uses_profile_specific_failures() {
        let variables = vec![
            ResidualVariableRef {
                index: 0,
                name: "x".to_owned(),
                role: "algebraic".to_owned(),
                unit: "1".to_owned(),
            },
            ResidualVariableRef {
                index: 1,
                name: "y".to_owned(),
                role: "algebraic".to_owned(),
                unit: "1".to_owned(),
            },
        ];
        let variable_aliases = variables
            .iter()
            .map(|variable| (variable.name.clone(), variable.index))
            .collect::<HashMap<_, _>>();
        let variable_units = variables
            .iter()
            .map(|variable| {
                (
                    variable.name.clone(),
                    (variable.unit.clone(), "Dimensionless".to_owned()),
                )
            })
            .collect::<HashMap<_, _>>();
        let dependencies = vec!["x".to_owned(), "y".to_owned()];

        let component_failure = lower_linear_residual_expression(
            "x * y",
            &dependencies,
            &variable_aliases,
            &variables,
            &variable_units,
            Some(("1", "Dimensionless")),
            None,
            COMPONENT_RESIDUAL_LOWERING,
        )
        .unwrap_err();
        assert_eq!(component_failure.code, "E-COMPONENT-ASSEMBLY-RESIDUAL");
        assert!(component_failure
            .message
            .contains("component assembly supports only simple linear residual terms"));

        let dynamic_failure = lower_linear_residual_expression(
            "x * y",
            &dependencies,
            &variable_aliases,
            &variables,
            &variable_units,
            Some(("1", "Dimensionless")),
            None,
            DYNAMIC_COMPONENT_RESIDUAL_LOWERING,
        )
        .unwrap_err();
        assert_eq!(
            dynamic_failure.code,
            "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL"
        );
        assert!(dynamic_failure
            .message
            .contains("dynamic component assembly supports only simple linear residual terms"));
    }

    #[test]
    fn component_equation_constants_convert_to_residual_unit() {
        let assembly = EquationAssembly {
            name: "component_graph".to_owned(),
            unknowns: vec![
                UnknownVariable {
                    name: "pipe.outlet.p".to_owned(),
                    role: "algebraic".to_owned(),
                    quantity_kind: "Pressure".to_owned(),
                    unit: "Pa".to_owned(),
                    source: "Fluid.p".to_owned(),
                    status: "classified".to_owned(),
                    value: None,
                },
                UnknownVariable {
                    name: "pipe.inlet.p".to_owned(),
                    role: "algebraic".to_owned(),
                    quantity_kind: "Pressure".to_owned(),
                    unit: "Pa".to_owned(),
                    source: "Fluid.p".to_owned(),
                    status: "classified".to_owned(),
                    value: None,
                },
            ],
            generated_equations: vec![GeneratedEquation {
                name: "pipe.pressure_drop".to_owned(),
                kind: "component_equation".to_owned(),
                residual: "pipe.outlet.p + 20 kPa - pipe.inlet.p".to_owned(),
                dependencies: vec!["pipe.outlet.p".to_owned(), "pipe.inlet.p".to_owned()],
                ..Default::default()
            }],
            ..Default::default()
        };

        let graph = ResidualGraph::from_assembly(&assembly);
        let residual = &graph.residuals[0];

        assert_eq!(residual.terms[0].coefficient, 1.0);
        assert_eq!(residual.terms[0].variable, "pipe.outlet.p");
        assert_eq!(residual.terms[1].coefficient, -1.0);
        assert_eq!(residual.terms[1].variable, "pipe.inlet.p");
        assert_eq!(residual.rhs_value, -20000.0);
        assert_eq!(residual.scale.value, 1000.0);
        assert_eq!(residual.scale.policy, "unit_default:Pressure[Pa]");
    }
    #[test]
    fn component_boundary_parameter_aliases_become_rhs_values() {
        let assembly = EquationAssembly {
            name: "component_graph".to_owned(),
            unknowns: vec![UnknownVariable {
                name: "pump.supply.p".to_owned(),
                role: "algebraic".to_owned(),
                quantity_kind: "Pressure".to_owned(),
                unit: "Pa".to_owned(),
                source: "Fluid.p".to_owned(),
                status: "classified".to_owned(),
                value: None,
            }],
            parameters: vec![UnknownVariable {
                name: "pump.p_supply".to_owned(),
                role: "parameter".to_owned(),
                quantity_kind: "Pressure".to_owned(),
                unit: "Pa".to_owned(),
                source: "component_parameter.Pressure".to_owned(),
                status: "constructor_override".to_owned(),
                value: Some(220000.0),
            }],
            generated_equations: vec![GeneratedEquation {
                name: "pump.boundary_supply_pressure".to_owned(),
                kind: "component_boundary".to_owned(),
                residual: "pump.supply.p - pump.p_supply".to_owned(),
                dependencies: vec!["pump.supply.p".to_owned(), "pump.p_supply".to_owned()],
                ..Default::default()
            }],
            ..Default::default()
        };

        let graph = ResidualGraph::from_assembly(&assembly);
        let residual = &graph.residuals[0];

        assert_eq!(residual.terms.len(), 1);
        assert_eq!(residual.terms[0].variable, "pump.supply.p");
        assert_eq!(residual.terms[0].coefficient, 1.0);
        assert_eq!(residual.rhs_value, 220000.0);
    }
    #[test]
    fn component_equation_parameter_aliases_become_linear_constants() {
        let assembly = EquationAssembly {
            name: "component_graph".to_owned(),
            unknowns: vec![
                UnknownVariable {
                    name: "pipe.outlet.p".to_owned(),
                    role: "algebraic".to_owned(),
                    quantity_kind: "Pressure".to_owned(),
                    unit: "Pa".to_owned(),
                    source: "Fluid.p".to_owned(),
                    status: "classified".to_owned(),
                    value: None,
                },
                UnknownVariable {
                    name: "pipe.inlet.p".to_owned(),
                    role: "algebraic".to_owned(),
                    quantity_kind: "Pressure".to_owned(),
                    unit: "Pa".to_owned(),
                    source: "Fluid.p".to_owned(),
                    status: "classified".to_owned(),
                    value: None,
                },
            ],
            parameters: vec![UnknownVariable {
                name: "pipe.dp".to_owned(),
                role: "parameter".to_owned(),
                quantity_kind: "Pressure".to_owned(),
                unit: "kPa".to_owned(),
                source: "component_parameter.Pressure".to_owned(),
                status: "defaulted".to_owned(),
                value: Some(20.0),
            }],
            generated_equations: vec![GeneratedEquation {
                name: "pipe.pressure_drop".to_owned(),
                kind: "component_equation".to_owned(),
                residual: "pipe.outlet.p + pipe.dp - pipe.inlet.p".to_owned(),
                dependencies: vec![
                    "pipe.outlet.p".to_owned(),
                    "pipe.dp".to_owned(),
                    "pipe.inlet.p".to_owned(),
                ],
                ..Default::default()
            }],
            ..Default::default()
        };

        let graph = ResidualGraph::from_assembly(&assembly);
        let residual = &graph.residuals[0];

        assert_eq!(residual.terms.len(), 2);
        assert_eq!(residual.terms[0].variable, "pipe.outlet.p");
        assert_eq!(residual.terms[0].coefficient, 1.0);
        assert_eq!(residual.terms[1].variable, "pipe.inlet.p");
        assert_eq!(residual.terms[1].coefficient, -1.0);
        assert_eq!(residual.rhs_value, -20000.0);
    }
    #[test]
    fn component_equation_conductance_parameter_converts_numerator_to_residual_unit() {
        let assembly = EquationAssembly {
            name: "component_graph".to_owned(),
            unknowns: vec![
                UnknownVariable {
                    name: "wall.inside.Q".to_owned(),
                    role: "algebraic".to_owned(),
                    quantity_kind: "HeatRate".to_owned(),
                    unit: "kW".to_owned(),
                    source: "Thermal.Q".to_owned(),
                    status: "classified".to_owned(),
                    value: None,
                },
                UnknownVariable {
                    name: "wall.inside.T".to_owned(),
                    role: "algebraic".to_owned(),
                    quantity_kind: "AbsoluteTemperature".to_owned(),
                    unit: "degC".to_owned(),
                    source: "Thermal.T".to_owned(),
                    status: "classified".to_owned(),
                    value: None,
                },
                UnknownVariable {
                    name: "wall.outside.T".to_owned(),
                    role: "algebraic".to_owned(),
                    quantity_kind: "AbsoluteTemperature".to_owned(),
                    unit: "degC".to_owned(),
                    source: "Thermal.T".to_owned(),
                    status: "classified".to_owned(),
                    value: None,
                },
            ],
            parameters: vec![UnknownVariable {
                name: "wall.UA".to_owned(),
                role: "parameter".to_owned(),
                quantity_kind: "Conductance".to_owned(),
                unit: "W/K".to_owned(),
                source: "component_parameter.Conductance".to_owned(),
                status: "defaulted".to_owned(),
                value: Some(500.0),
            }],
            generated_equations: vec![GeneratedEquation {
                name: "wall.conductance".to_owned(),
                kind: "component_equation".to_owned(),
                residual: "wall.inside.Q - wall.UA * (wall.inside.T - wall.outside.T)".to_owned(),
                dependencies: vec![
                    "wall.inside.Q".to_owned(),
                    "wall.inside.T".to_owned(),
                    "wall.outside.T".to_owned(),
                    "wall.UA".to_owned(),
                ],
                ..Default::default()
            }],
            ..Default::default()
        };

        let graph = ResidualGraph::from_assembly(&assembly);
        let residual = &graph.residuals[0];

        assert_eq!(residual.terms.len(), 3);
        assert_eq!(
            residual.expression.inferred_unit,
            Some(ResidualUnit {
                unit: "W".to_owned(),
                quantity_kind: "HeatRate".to_owned(),
            })
        );
        assert_eq!(residual.terms[0].variable, "wall.inside.Q");
        assert_eq!(residual.terms[0].coefficient, 1.0);
        assert_eq!(residual.terms[1].variable, "wall.inside.T");
        assert!((residual.terms[1].coefficient + 0.5).abs() <= 1e-12);
        assert_eq!(residual.terms[2].variable, "wall.outside.T");
        assert!((residual.terms[2].coefficient - 0.5).abs() <= 1e-12);

        let output = <ResidualGraph as ResidualEvaluator>::evaluate(
            &graph,
            &ResidualInput::new(&[5.0, 22.0, 12.0]),
        )
        .unwrap();
        assert!(output.values[0].value.abs() <= 1e-12);
    }
    #[test]
    fn residual_scales_use_quantity_unit_defaults() {
        let heat_rate = ResidualScale::from_quantity_unit("HeatRate", "W");
        assert_eq!(heat_rate.value, 1000.0);
        assert_eq!(heat_rate.policy, "unit_default:HeatRate[W]");

        let temperature = ResidualScale::from_quantity_unit("AbsoluteTemperature", "degC");
        assert_eq!(temperature.value, 1.0);
        assert_eq!(temperature.policy, "unit_default:AbsoluteTemperature[degC]");
    }

    #[test]
    fn evaluator_applies_user_provided_scale_and_tolerance() {
        let graph = ResidualGraph {
            name: "test.residual_graph".to_owned(),
            variables: vec![ResidualVariableRef {
                index: 0,
                name: "Q".to_owned(),
                role: "algebraic".to_owned(),
                unit: "kW".to_owned(),
            }],
            residuals: vec![residual_with_rhs("heat_balance", &[(0, "Q", 1.0)], 2.0)],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        };
        let overrides = vec![ResidualScaleOverride {
            residual: "heat_balance".to_owned(),
            scale: ResidualScale::user_provided(4.0, "heat_balance_nominal").unwrap(),
        }];

        let output = <ResidualGraph as ResidualEvaluator>::evaluate(
            &graph,
            &ResidualInput::new(&[4.0])
                .with_scale_overrides(&overrides)
                .with_tolerance(0.51),
        )
        .unwrap();

        assert_eq!(output.tolerance, 0.51);
        assert_eq!(output.values[0].value, 2.0);
        assert_eq!(output.values[0].normalized_value, 0.5);
        assert_eq!(output.values[0].status, "satisfied");
        assert_eq!(output.residual_norm, 0.5);
    }

    #[test]
    fn rejects_invalid_user_provided_residual_scale() {
        let failure = ResidualScale::user_provided(0.0, "bad").unwrap_err();

        assert_eq!(failure.code, "E-RESIDUAL-SCALE-001");
    }

    #[test]
    fn rejects_invalid_user_provided_residual_tolerance() {
        let values = [0.0];
        let failure = ResidualInput::new(&values)
            .try_with_tolerance(f64::NAN)
            .unwrap_err();

        assert_eq!(failure.code, "E-RESIDUAL-TOLERANCE-001");
    }

    #[test]
    fn evaluator_rejects_nonfinite_inputs_scales_and_values() {
        let graph = ResidualGraph {
            name: "test.residual_graph".to_owned(),
            variables: vec![ResidualVariableRef {
                index: 0,
                name: "x".to_owned(),
                role: "algebraic".to_owned(),
                unit: "1".to_owned(),
            }],
            residuals: vec![residual("r1", &[(0, "x", 1.0)])],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        };

        let failure = <ResidualGraph as ResidualEvaluator>::evaluate(
            &graph,
            &ResidualInput::new(&[f64::NAN]),
        )
        .unwrap_err();
        assert_eq!(failure.code, "E-RESIDUAL-INPUT-FINITE");

        let bad_scale = vec![ResidualScaleOverride {
            residual: "r1".to_owned(),
            scale: ResidualScale {
                value: f64::NAN,
                policy: "test".to_owned(),
            },
        }];
        let failure = <ResidualGraph as ResidualEvaluator>::evaluate(
            &graph,
            &ResidualInput::new(&[1.0]).with_scale_overrides(&bad_scale),
        )
        .unwrap_err();
        assert_eq!(failure.code, "E-RESIDUAL-SCALE-001");

        let graph = ResidualGraph {
            name: "test.residual_graph".to_owned(),
            variables: vec![ResidualVariableRef {
                index: 0,
                name: "x".to_owned(),
                role: "algebraic".to_owned(),
                unit: "1".to_owned(),
            }],
            residuals: vec![residual("r1", &[(0, "x", f64::MAX)])],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        };
        let failure =
            <ResidualGraph as ResidualEvaluator>::evaluate(&graph, &ResidualInput::new(&[2.0]))
                .unwrap_err();
        assert_eq!(failure.code, "E-RESIDUAL-FINITE");
    }

    #[test]
    fn rich_residual_evaluator_uses_structured_state_and_algebraic_values() {
        let graph = ResidualGraph {
            name: "test.residual_graph".to_owned(),
            variables: vec![
                ResidualVariableRef {
                    index: 0,
                    name: "T_state".to_owned(),
                    role: "state".to_owned(),
                    unit: "K".to_owned(),
                },
                ResidualVariableRef {
                    index: 1,
                    name: "T_node".to_owned(),
                    role: "algebraic".to_owned(),
                    unit: "K".to_owned(),
                },
            ],
            residuals: vec![residual(
                "state_node_delta",
                &[(0, "T_state", 1.0), (1, "T_node", -1.0)],
            )],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        };

        let output = super::super::evaluator::ResidualEvaluator::evaluate(
            &graph,
            &super::super::evaluator::ResidualInput {
                x: vec![300.0],
                z: vec![299.5],
                t: 12.0,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(output.residuals, vec![0.5]);
        assert_eq!(output.named_residuals[0].name, "state_node_delta");
        assert_eq!(output.named_residuals[0].normalized_value, 0.5);
    }

    #[test]
    fn rich_residual_evaluator_uses_derivative_input_parameter_and_time_values() {
        let graph = ResidualGraph {
            name: "test.residual_graph".to_owned(),
            variables: vec![
                ResidualVariableRef {
                    index: 0,
                    name: "dT".to_owned(),
                    role: "derivative".to_owned(),
                    unit: "K/s".to_owned(),
                },
                ResidualVariableRef {
                    index: 1,
                    name: "z_node".to_owned(),
                    role: "algebraic".to_owned(),
                    unit: "K".to_owned(),
                },
                ResidualVariableRef {
                    index: 2,
                    name: "u_heat".to_owned(),
                    role: "input".to_owned(),
                    unit: "kW".to_owned(),
                },
                ResidualVariableRef {
                    index: 3,
                    name: "p_gain".to_owned(),
                    role: "parameter".to_owned(),
                    unit: "1".to_owned(),
                },
                ResidualVariableRef {
                    index: 4,
                    name: "t".to_owned(),
                    role: "time".to_owned(),
                    unit: "s".to_owned(),
                },
            ],
            residuals: vec![residual(
                "dynamic_balance",
                &[
                    (0, "dT", 1.0),
                    (1, "z_node", 1.0),
                    (2, "u_heat", 1.0),
                    (3, "p_gain", 1.0),
                    (4, "t", 1.0),
                ],
            )],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        };
        let input = super::super::evaluator::ResidualInput {
            xdot: Some(vec![2.0]),
            z: vec![3.0],
            u: vec![4.0],
            p: vec![5.0],
            t: 6.0,
            ..Default::default()
        };

        let first = super::super::evaluator::ResidualEvaluator::evaluate(&graph, &input).unwrap();
        let second = super::super::evaluator::ResidualEvaluator::evaluate(&graph, &input).unwrap();

        assert_eq!(first.residuals, vec![20.0]);
        assert_eq!(first.named_residuals[0].name, "dynamic_balance");
        assert_eq!(first.named_residuals[0].normalized_value, 20.0);
        assert_eq!(second, first);
    }

    #[test]
    fn rich_residual_evaluator_requires_derivative_values_when_role_uses_xdot() {
        let graph = ResidualGraph {
            name: "test.residual_graph".to_owned(),
            variables: vec![ResidualVariableRef {
                index: 0,
                name: "dT".to_owned(),
                role: "derivative".to_owned(),
                unit: "K/s".to_owned(),
            }],
            residuals: vec![residual("dynamic_balance", &[(0, "dT", 1.0)])],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        };

        let failure = super::super::evaluator::ResidualEvaluator::evaluate(
            &graph,
            &super::super::evaluator::ResidualInput::default(),
        )
        .unwrap_err();

        assert_eq!(failure.code, "E-RESIDUAL-INPUT-LAYOUT");
    }

    #[test]
    fn rich_residual_evaluator_reports_missing_structured_values() {
        let graph = ResidualGraph {
            name: "test.residual_graph".to_owned(),
            variables: vec![ResidualVariableRef {
                index: 0,
                name: "T_state".to_owned(),
                role: "state".to_owned(),
                unit: "K".to_owned(),
            }],
            residuals: vec![residual("state_residual", &[(0, "T_state", 1.0)])],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        };

        let failure = super::super::evaluator::ResidualEvaluator::evaluate(
            &graph,
            &super::super::evaluator::ResidualInput::default(),
        )
        .unwrap_err();

        assert_eq!(failure.code, "E-RESIDUAL-INPUT-LAYOUT");
    }

    #[test]
    fn residual_graph_preserves_parameter_indices_from_assembly() {
        let assembly = EquationAssembly {
            name: "parametric".to_owned(),
            unknowns: vec![UnknownVariable {
                name: "T".to_owned(),
                role: "algebraic".to_owned(),
                quantity_kind: "AbsoluteTemperature".to_owned(),
                unit: "K".to_owned(),
                source: "node.T".to_owned(),
                status: "classified".to_owned(),
                value: None,
            }],
            parameters: vec![UnknownVariable {
                name: "R".to_owned(),
                role: "parameter".to_owned(),
                quantity_kind: "ThermalResistance".to_owned(),
                unit: "K/kW".to_owned(),
                source: "wall.R".to_owned(),
                status: "classified".to_owned(),
                value: None,
            }],
            ..Default::default()
        };

        let graph = ResidualGraph::from_assembly(&assembly);

        assert_eq!(graph.variables.len(), 1);
        assert_eq!(graph.variables[0].index, 0);
        assert_eq!(graph.variables[0].name, "T");
        assert_eq!(graph.parameters.len(), 1);
        assert_eq!(graph.parameters[0].index, 0);
        assert_eq!(graph.parameters[0].name, "R");
        assert_eq!(graph.parameters[0].role, "parameter");
        assert_eq!(graph.parameters[0].unit, "K/kW");
    }

    #[test]
    fn dynamic_component_residual_graph_from_assembly_preserves_solver_roles() {
        let assembly = dynamic_component_test_assembly("component_graph");

        let graph = ResidualGraph::from_dynamic_component_assembly(&assembly).unwrap();

        assert_eq!(
            graph.name,
            "component_graph.dynamic_component_residual_graph"
        );
        assert_eq!(
            graph
                .variables
                .iter()
                .map(|variable| (variable.name.as_str(), variable.role.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("x", "state"),
                ("z", "algebraic"),
                ("u", "input"),
                ("k", "parameter"),
                ("der_x", "state_derivative"),
            ]
        );
        assert_eq!(graph.parameters[0].name, "k");
        let rhs = graph
            .residuals
            .iter()
            .find(|residual| residual.name == "x_rhs")
            .unwrap();
        assert_eq!(rhs.rhs_value, 0.0);
        assert_eq!(
            rhs.expression.inferred_unit,
            Some(ResidualUnit {
                unit: "1/s".to_owned(),
                quantity_kind: "Dimensionless".to_owned(),
            })
        );
        assert_eq!(
            rhs.terms
                .iter()
                .map(|term| (term.variable.as_str(), term.coefficient))
                .collect::<Vec<_>>(),
            vec![("der_x", 1.0), ("z", -1.0)]
        );
        let algebraic = graph
            .residuals
            .iter()
            .find(|residual| residual.name == "z_balance")
            .unwrap();
        assert_eq!(
            algebraic.expression.inferred_unit,
            Some(ResidualUnit {
                unit: "1".to_owned(),
                quantity_kind: "Dimensionless".to_owned(),
            })
        );
        assert_eq!(
            algebraic
                .terms
                .iter()
                .map(|term| (term.variable.as_str(), term.coefficient))
                .collect::<Vec<_>>(),
            vec![("z", 1.0), ("x", 1.0), ("k", 1.0), ("u", -1.0)]
        );
        assert!(graph
            .dependencies
            .iter()
            .any(|(residual, variable)| residual == "x_rhs" && variable == "der_x"));
    }

    #[test]
    fn dynamic_component_residual_graph_linearizes_parenthesized_arithmetic() {
        let mut assembly = dynamic_component_test_assembly("component_graph");
        assembly.generated_equations[0].residual = "(der_x - z) / 2".to_owned();
        assembly.generated_equations[1].residual = "z + (2 * (x - u)) + k".to_owned();

        let graph = ResidualGraph::from_dynamic_component_assembly(&assembly).unwrap();

        let rhs = graph
            .residuals
            .iter()
            .find(|residual| residual.name == "x_rhs")
            .unwrap();
        assert_eq!(
            rhs.terms
                .iter()
                .map(|term| (term.variable.as_str(), term.coefficient))
                .collect::<Vec<_>>(),
            vec![("der_x", 0.5), ("z", -0.5)]
        );

        let algebraic = graph
            .residuals
            .iter()
            .find(|residual| residual.name == "z_balance")
            .unwrap();
        assert_eq!(
            algebraic.expression.inferred_unit,
            Some(ResidualUnit {
                unit: "1".to_owned(),
                quantity_kind: "Dimensionless".to_owned(),
            })
        );
        assert_eq!(
            algebraic
                .terms
                .iter()
                .map(|term| (term.variable.as_str(), term.coefficient))
                .collect::<Vec<_>>(),
            vec![("z", 1.0), ("x", 2.0), ("k", 1.0), ("u", -2.0)]
        );
    }
    #[test]
    fn dynamic_component_residual_graph_folds_parameterized_derivative_coefficient() {
        let mut assembly = dynamic_component_test_assembly("component_graph");
        assembly.parameters[0].value = Some(2.0);
        assembly.generated_equations[0].expression = "k * der(x) eq z".to_owned();
        assembly.generated_equations[0].residual = "k * der_x - z".to_owned();
        assembly.generated_equations[0].dependencies =
            vec!["k".to_owned(), "der_x".to_owned(), "z".to_owned()];

        let graph = ResidualGraph::from_dynamic_component_assembly(&assembly).unwrap();

        let rhs = graph
            .residuals
            .iter()
            .find(|residual| residual.name == "x_rhs")
            .unwrap();
        assert_eq!(rhs.rhs_value, 0.0);
        assert_eq!(
            rhs.terms
                .iter()
                .map(|term| (term.variable.as_str(), term.coefficient))
                .collect::<Vec<_>>(),
            vec![("der_x", 2.0), ("z", -1.0)]
        );
        assert!(!rhs.terms.iter().any(|term| term.variable == "k"));
    }
    #[test]
    fn dynamic_component_residual_graph_rejects_unsupported_residual_terms() {
        let mut assembly = dynamic_component_test_assembly("component_graph");
        assembly.generated_equations[0].residual = "der_x / z".to_owned();

        let failure = ResidualGraph::from_dynamic_component_assembly(&assembly).unwrap_err();

        assert_eq!(failure.code, "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL");
        assert!(failure.message.contains("unsupported term"));
    }

    fn residual(name: &str, terms: &[(usize, &str, f64)]) -> ResidualEquation {
        residual_with_rhs(name, terms, 0.0)
    }

    fn residual_with_rhs(
        name: &str,
        terms: &[(usize, &str, f64)],
        rhs_value: f64,
    ) -> ResidualEquation {
        ResidualEquation {
            name: name.to_owned(),
            expression: ResidualExpression::manual(name),
            rhs_value,
            unit: ResidualUnit {
                unit: "1".to_owned(),
                quantity_kind: "Dimensionless".to_owned(),
            },
            scale: ResidualScale::default(),
            source: ResidualSource::default(),
            variable_indices: terms.iter().map(|(index, _, _)| *index).collect(),
            terms: terms
                .iter()
                .map(|(index, variable, coefficient)| ResidualTerm {
                    variable_index: *index,
                    variable: (*variable).to_owned(),
                    coefficient: *coefficient,
                })
                .collect(),
        }
    }

    fn dynamic_component_test_assembly(name: &str) -> EquationAssembly {
        let x = unknown("x", "state");
        let z = unknown("z", "algebraic");
        let u = unknown("u", "input");
        let k = unknown("k", "parameter");
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

    fn unknown(name: &str, role: &str) -> UnknownVariable {
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
}
