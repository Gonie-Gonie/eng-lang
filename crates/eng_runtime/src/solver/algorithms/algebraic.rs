use crate::solver::{
    algorithms::linear::{solve_dense_linear_system, LinearSolveDiagnostics},
    NamedResidualValue, ResidualEvaluator, ResidualGraph, ResidualInput, SolverFailure,
};

#[derive(Clone, Debug, PartialEq)]
pub struct LinearResidualGraphSolution {
    pub variables: Vec<LinearResidualVariableSolution>,
    pub residuals: Vec<NamedResidualValue>,
    pub residual_norm: f64,
    pub status: String,
    pub iteration_count: usize,
    pub linear_diagnostics: LinearSolveDiagnostics,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinearResidualVariableSolution {
    pub name: String,
    pub value: f64,
}

pub fn solve_linear_residual_graph(
    graph: &ResidualGraph,
    tolerance: f64,
) -> Result<LinearResidualGraphSolution, SolverFailure> {
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(SolverFailure::new(
            "E-LINEAR-RESIDUAL-TOLERANCE",
            "linear residual graph solve requires a positive finite tolerance",
        ));
    }

    let system = graph.assemble_linear_system()?;
    let linear_result = solve_dense_linear_system(&system.matrix, &system.rhs, tolerance)?;
    let residual_output = graph
        .evaluate(&ResidualInput::new(&linear_result.values).try_with_tolerance(tolerance)?)?;
    let variables = system
        .variable_names
        .iter()
        .zip(linear_result.values.iter())
        .map(|(name, value)| LinearResidualVariableSolution {
            name: name.clone(),
            value: *value,
        })
        .collect::<Vec<_>>();

    Ok(LinearResidualGraphSolution {
        variables,
        residuals: residual_output.values,
        residual_norm: residual_output.residual_norm,
        status: if residual_output.residual_norm <= tolerance {
            "converged".to_owned()
        } else {
            "residual_above_tolerance".to_owned()
        },
        iteration_count: 1,
        linear_diagnostics: linear_result.diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{
        ResidualEquation, ResidualExpression, ResidualScale, ResidualSource, ResidualTerm,
        ResidualUnit, ResidualVariableRef,
    };

    #[test]
    fn solves_square_linear_residual_graph() {
        let graph = ResidualGraph {
            name: "linear.residual_graph".to_owned(),
            variables: vec![variable(0, "x"), variable(1, "y")],
            residuals: vec![
                residual("r1", &[(0, "x", 2.0), (1, "y", 1.0)], 5.0),
                residual("r2", &[(0, "x", 1.0), (1, "y", -1.0)], 1.0),
            ],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        };

        let solution = solve_linear_residual_graph(&graph, 1e-9).unwrap();

        assert_eq!(solution.status, "converged");
        assert_eq!(solution.iteration_count, 1);
        assert!(solution.linear_diagnostics.pivot_condition_estimate >= 1.0);
        assert!((solution.variables[0].value - 2.0).abs() <= 1e-9);
        assert!((solution.variables[1].value - 1.0).abs() <= 1e-9);
        assert!(solution.residual_norm <= 1e-9);
        assert_eq!(solution.residuals[0].status, "satisfied");
    }

    #[test]
    fn reports_singular_linear_residual_graph_failure() {
        let graph = ResidualGraph {
            name: "singular.residual_graph".to_owned(),
            variables: vec![variable(0, "x"), variable(1, "y")],
            residuals: vec![
                residual("r1", &[(0, "x", 1.0), (1, "y", 2.0)], 3.0),
                residual("r2", &[(0, "x", 2.0), (1, "y", 4.0)], 6.0),
            ],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        };

        let failure = solve_linear_residual_graph(&graph, 1e-9).unwrap_err();

        assert_eq!(failure.code, "E-LINEAR-SINGULAR");
    }

    #[test]
    fn reports_ill_conditioned_linear_residual_graph_failure() {
        let graph = ResidualGraph {
            name: "ill_conditioned.residual_graph".to_owned(),
            variables: vec![variable(0, "x"), variable(1, "y")],
            residuals: vec![
                residual("r1", &[(0, "x", 1.0), (1, "y", 1.0)], 2.0),
                residual("r2", &[(0, "x", 1.0), (1, "y", 1.0 + 1e-12)], 2.0 + 1e-12),
            ],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        };

        let failure = solve_linear_residual_graph(&graph, 1e-9).unwrap_err();

        assert_eq!(failure.code, "E-LINEAR-ILL-CONDITIONED");
        assert!(failure.message.contains("pivot 1"));
    }

    fn variable(index: usize, name: &str) -> ResidualVariableRef {
        ResidualVariableRef {
            index,
            name: name.to_owned(),
            role: "algebraic".to_owned(),
            unit: "1".to_owned(),
        }
    }

    fn residual(name: &str, terms: &[(usize, &str, f64)], rhs_value: f64) -> ResidualEquation {
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
}
