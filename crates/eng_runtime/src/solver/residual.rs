use std::collections::{HashMap, HashSet};

use eng_compiler::{all_unit_infos, normalize_unit, UnitInfo};

use super::{assembly::EquationAssembly, euclidean_norm, SolverFailure};

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
            .map(|variable| {
                (
                    variable.name.clone(),
                    (variable.unit.clone(), variable.quantity_kind.clone()),
                )
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
                let parsed_component_equation = if equation.kind == "component_equation" {
                    parse_dynamic_linear_residual_terms(
                        &equation.residual,
                        &equation.dependencies,
                        &variable_indices,
                        &variables,
                        Some((&unit, &quantity_kind)),
                    )
                    .ok()
                } else {
                    None
                };
                let terms = parsed_component_equation
                    .as_ref()
                    .map(|parsed| parsed.terms.clone())
                    .unwrap_or_else(|| {
                        residual_terms_for_generated_equation(
                            &equation.kind,
                            &equation.dependencies,
                            &variable_indices,
                        )
                    });
                let indices = terms
                    .iter()
                    .map(|term| term.variable_index)
                    .collect::<Vec<_>>();
                let scale = ResidualScale::from_quantity_unit(&quantity_kind, &unit);
                ResidualEquation {
                    name: equation.name.clone(),
                    expression: ResidualExpression {
                        text: equation.residual.clone(),
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
                let parsed = parse_dynamic_linear_residual_terms(
                    &equation.residual,
                    &equation.dependencies,
                    &variable_aliases,
                    &variables,
                    Some((&unit, &quantity_kind)),
                )?;
                let rhs_value = dynamic_residual_rhs_value(equation, parsed.constant)?;
                let scale = ResidualScale::from_quantity_unit(&quantity_kind, &unit);

                Ok(ResidualEquation {
                    name: equation.name.clone(),
                    expression: ResidualExpression {
                        text: equation.residual.clone(),
                    },
                    rhs_value,
                    unit: ResidualUnit {
                        unit,
                        quantity_kind,
                    },
                    scale,
                    source: ResidualSource {
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
struct ParsedDynamicResidual {
    terms: Vec<ResidualTerm>,
    constant: f64,
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

fn parse_dynamic_linear_residual_terms(
    expression: &str,
    dependencies: &[String],
    variable_aliases: &HashMap<String, usize>,
    variables: &[ResidualVariableRef],
    residual_unit: Option<(&str, &str)>,
) -> Result<ParsedDynamicResidual, SolverFailure> {
    let dependency_indices = dependencies
        .iter()
        .filter_map(|dependency| variable_aliases.get(dependency).copied())
        .collect::<HashSet<_>>();
    if dependency_indices.is_empty() {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL",
            format!("dynamic component residual `{expression}` has no solver dependencies"),
        ));
    }

    let mut parsed = ParsedDynamicResidual::default();
    let mut sign = 1.0;
    let mut tokens = expression.split_whitespace().peekable();
    while let Some(token) = tokens.next() {
        match token {
            "+" => {
                sign = 1.0;
                continue;
            }
            "-" => {
                sign = -1.0;
                continue;
            }
            _ => {}
        }

        let mut parts = vec![token];
        while let Some(next) = tokens.peek().copied() {
            if next == "+" || next == "-" {
                break;
            }
            parts.push(tokens.next().unwrap());
        }
        parse_dynamic_linear_term(
            &parts.join(" "),
            sign,
            &dependency_indices,
            variable_aliases,
            variables,
            residual_unit,
            &mut parsed,
        )?;
        sign = 1.0;
    }

    if parsed.terms.is_empty() {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL",
            format!("dynamic component residual `{expression}` has no linear variable terms"),
        ));
    }
    Ok(parsed)
}

fn parse_dynamic_linear_term(
    raw: &str,
    sign: f64,
    dependency_indices: &HashSet<usize>,
    variable_aliases: &HashMap<String, usize>,
    variables: &[ResidualVariableRef],
    residual_unit: Option<(&str, &str)>,
    parsed: &mut ParsedDynamicResidual,
) -> Result<(), SolverFailure> {
    let term = raw.trim();
    if term.is_empty() {
        return Ok(());
    }
    if let Some(index) = variable_aliases.get(term).copied() {
        push_dynamic_linear_term(term, index, sign, dependency_indices, variables, parsed)?;
        return Ok(());
    }
    if let Ok(value) = term.parse::<f64>() {
        parsed.constant += sign * value;
        return Ok(());
    }

    let factors = term
        .split('*')
        .map(str::trim)
        .filter(|factor| !factor.is_empty())
        .collect::<Vec<_>>();
    if factors.len() > 1 {
        let mut coefficient = sign;
        let mut variable_index = None;
        let mut variable_label = "";
        for factor in factors {
            if let Ok(value) = factor.parse::<f64>() {
                coefficient *= value;
            } else if let Some(index) = variable_aliases.get(factor).copied() {
                if variable_index.is_some() {
                    return Err(unsupported_dynamic_linear_term(term));
                }
                variable_index = Some(index);
                variable_label = factor;
            } else {
                return Err(unsupported_dynamic_linear_term(term));
            }
        }
        let Some(index) = variable_index else {
            parsed.constant += coefficient;
            return Ok(());
        };
        push_dynamic_linear_term(
            variable_label,
            index,
            coefficient,
            dependency_indices,
            variables,
            parsed,
        )?;
        return Ok(());
    }

    let pieces = term.split_whitespace().collect::<Vec<_>>();
    if pieces.len() == 2 {
        if let (Ok(coefficient), Some(index)) = (
            pieces[0].parse::<f64>(),
            variable_aliases.get(pieces[1]).copied(),
        ) {
            push_dynamic_linear_term(
                pieces[1],
                index,
                sign * coefficient,
                dependency_indices,
                variables,
                parsed,
            )?;
            return Ok(());
        }
        if let Ok(value) = pieces[0].parse::<f64>() {
            let value = convert_residual_constant(value, pieces[1], residual_unit)?;
            parsed.constant += sign * value;
            return Ok(());
        }
    }

    Err(unsupported_dynamic_linear_term(term))
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
        return Err(unsupported_dynamic_linear_term(&format!(
            "{value} {source_unit}"
        )));
    };
    let Some(target_info) = residual_unit_info(target_unit) else {
        return Err(unsupported_dynamic_linear_term(&format!(
            "{value} {source_unit}"
        )));
    };
    if source_info.affine_offset.is_some() || target_info.affine_offset.is_some() {
        return Err(unsupported_dynamic_linear_term(&format!(
            "{value} {source_unit}"
        )));
    }
    if normalize_unit(source_info.canonical_unit) != normalize_unit(target_info.canonical_unit) {
        return Err(unsupported_dynamic_linear_term(&format!(
            "{value} {source_unit}"
        )));
    }
    let source_scale = source_info.scale_to_canonical.parse::<f64>().map_err(|_| {
        SolverFailure::new(
            "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL",
            format!("unit `{source_unit}` has an invalid residual conversion scale"),
        )
    })?;
    let target_scale = target_info.scale_to_canonical.parse::<f64>().map_err(|_| {
        SolverFailure::new(
            "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL",
            format!("unit `{target_unit}` has an invalid residual conversion scale"),
        )
    })?;
    Ok(value * source_scale / target_scale)
}

fn push_dynamic_linear_term(
    label: &str,
    index: usize,
    coefficient: f64,
    dependency_indices: &HashSet<usize>,
    variables: &[ResidualVariableRef],
    parsed: &mut ParsedDynamicResidual,
) -> Result<(), SolverFailure> {
    if !dependency_indices.contains(&index) {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL",
            format!("dynamic component residual term `{label}` is not listed as a dependency"),
        ));
    }
    if !coefficient.is_finite() {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL",
            format!("dynamic component residual term `{label}` has a non-finite coefficient"),
        ));
    }
    let Some(variable) = variables.get(index) else {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL",
            format!("dynamic component residual term `{label}` references an unknown variable"),
        ));
    };
    parsed.terms.push(ResidualTerm {
        variable_index: index,
        variable: variable.name.clone(),
        coefficient,
    });
    Ok(())
}

fn unsupported_dynamic_linear_term(term: &str) -> SolverFailure {
    SolverFailure::new(
        "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL",
        format!("dynamic component assembly supports only simple linear residual terms; unsupported term `{term}`"),
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
                },
                UnknownVariable {
                    name: "y".to_owned(),
                    role: "algebraic".to_owned(),
                    quantity_kind: "Dimensionless".to_owned(),
                    unit: "1".to_owned(),
                    source: "node.y".to_owned(),
                    status: "classified".to_owned(),
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
                },
                UnknownVariable {
                    name: "pipe.inlet.p".to_owned(),
                    role: "algebraic".to_owned(),
                    quantity_kind: "Pressure".to_owned(),
                    unit: "Pa".to_owned(),
                    source: "Fluid.p".to_owned(),
                    status: "classified".to_owned(),
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
            }],
            parameters: vec![UnknownVariable {
                name: "R".to_owned(),
                role: "parameter".to_owned(),
                quantity_kind: "ThermalResistance".to_owned(),
                unit: "K/kW".to_owned(),
                source: "wall.R".to_owned(),
                status: "classified".to_owned(),
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
            expression: ResidualExpression {
                text: name.to_owned(),
            },
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
        }
    }
}
