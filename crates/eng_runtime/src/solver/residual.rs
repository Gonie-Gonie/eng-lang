use std::collections::HashMap;

use super::{assembly::EquationAssembly, SolverFailure};

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
                let indices = equation
                    .dependencies
                    .iter()
                    .filter_map(|dependency| variable_indices.get(dependency).copied())
                    .collect::<Vec<_>>();
                let terms = residual_terms_for_generated_equation(
                    &equation.kind,
                    &equation.dependencies,
                    &variable_indices,
                );
                let (unit, quantity_kind) = equation
                    .dependencies
                    .first()
                    .and_then(|dependency| variable_units.get(dependency))
                    .cloned()
                    .unwrap_or_else(|| ("1".to_owned(), "unknown".to_owned()));
                let scale = ResidualScale::from_quantity_unit(&quantity_kind, &unit);
                ResidualEquation {
                    name: equation.name.clone(),
                    expression: ResidualExpression {
                        text: equation.residual.clone(),
                    },
                    rhs_value: equation.rhs_value.unwrap_or(0.0),
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
    fn evaluate(&self, input: &ResidualInput<'_>) -> ResidualOutput;
}

impl ResidualEvaluator for ResidualGraph {
    fn evaluate(&self, input: &ResidualInput<'_>) -> ResidualOutput {
        let values = self
            .residuals
            .iter()
            .map(|residual| {
                let value = residual
                    .terms
                    .iter()
                    .map(|term| {
                        term.coefficient
                            * input
                                .values
                                .get(term.variable_index)
                                .copied()
                                .unwrap_or_default()
                    })
                    .sum::<f64>()
                    - residual.rhs_value;
                let normalized_value = value / residual.scale.value.max(f64::EPSILON);
                NamedResidualValue {
                    name: residual.name.clone(),
                    value,
                    normalized_value,
                }
            })
            .collect::<Vec<_>>();
        let residual_norm = values
            .iter()
            .map(|value| value.normalized_value * value.normalized_value)
            .sum::<f64>()
            .sqrt();
        ResidualOutput {
            values,
            residual_norm,
        }
    }
}

impl super::evaluator::ResidualEvaluator for ResidualGraph {
    fn evaluate(
        &self,
        input: &super::evaluator::ResidualInput,
    ) -> Result<super::evaluator::ResidualOutput, SolverFailure> {
        let values = self.values_from_structured_input(input)?;
        let output = <Self as ResidualEvaluator>::evaluate(
            self,
            &ResidualInput {
                values: values.as_slice(),
            },
        );
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

impl ResidualGraph {
    fn values_from_structured_input(
        &self,
        input: &super::evaluator::ResidualInput,
    ) -> Result<Vec<f64>, SolverFailure> {
        let mut state_index = 0;
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
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualOutput {
    pub values: Vec<NamedResidualValue>,
    pub residual_norm: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NamedResidualValue {
    pub name: String,
    pub value: f64,
    pub normalized_value: f64,
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

impl Default for ResidualScale {
    fn default() -> Self {
        Self {
            value: 1.0,
            policy: "unit_default:dimensionless[1]".to_owned(),
        }
    }
}

impl ResidualScale {
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
    use super::super::assembly::UnknownVariable;
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

        let output = <ResidualGraph as ResidualEvaluator>::evaluate(
            &graph,
            &ResidualInput { values: &[5.0] },
        );
        assert_eq!(output.values[0].value, 1.0);
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
}
