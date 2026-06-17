use std::collections::HashMap;

use super::{assembly::EquationAssembly, SolverFailure};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualGraph {
    pub name: String,
    pub residuals: Vec<ResidualEquation>,
    pub variables: Vec<ResidualVariableRef>,
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
                ResidualEquation {
                    name: equation.name.clone(),
                    expression: ResidualExpression {
                        text: equation.residual.clone(),
                    },
                    unit: ResidualUnit {
                        unit,
                        quantity_kind,
                    },
                    scale: ResidualScale::default(),
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
            rhs: vec![0.0; equation_count],
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
                    .sum::<f64>();
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualEquation {
    pub name: String,
    pub expression: ResidualExpression,
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
            policy: "unit_nominal_default".to_owned(),
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
            dependencies: Vec::new(),
        };

        let failure = graph.assemble_linear_system().unwrap_err();

        assert_eq!(failure.code, "E-LINEAR-RESIDUAL-SHAPE");
    }

    fn residual(name: &str, terms: &[(usize, &str, f64)]) -> ResidualEquation {
        ResidualEquation {
            name: name.to_owned(),
            expression: ResidualExpression {
                text: name.to_owned(),
            },
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
