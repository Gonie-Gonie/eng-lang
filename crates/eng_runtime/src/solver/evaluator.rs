use super::diagnostics::SolverFailure;

#[derive(Clone, Debug, PartialEq)]
pub struct RhsStateInfo {
    pub name: String,
    pub quantity_kind: String,
    pub canonical_unit: String,
}

impl RhsStateInfo {
    pub fn new(
        name: impl Into<String>,
        quantity_kind: impl Into<String>,
        canonical_unit: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            quantity_kind: quantity_kind.into(),
            canonical_unit: canonical_unit.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RhsInput {
    pub t: f64,
    pub x: Vec<f64>,
    pub u: Vec<f64>,
    pub p: Vec<f64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RhsOutput {
    pub derivatives: Vec<f64>,
    pub named_derivatives: Vec<NamedDerivative>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NamedDerivative {
    pub state: String,
    pub quantity_kind: String,
    pub canonical_unit: String,
    pub value: f64,
}

pub trait RhsEvaluator {
    fn evaluate(&self, input: &RhsInput) -> Result<RhsOutput, SolverFailure>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateSpaceRhsEvaluator {
    states: Vec<RhsStateInfo>,
    matrix_a: Vec<Vec<f64>>,
    matrix_b: Vec<Vec<f64>>,
}

impl StateSpaceRhsEvaluator {
    pub fn new(
        states: Vec<RhsStateInfo>,
        matrix_a: Vec<Vec<f64>>,
        matrix_b: Vec<Vec<f64>>,
        input_count: usize,
    ) -> Result<Self, SolverFailure> {
        if states.is_empty() {
            return Err(SolverFailure::new(
                "E-RHS-STATE-LAYOUT",
                "state-space RHS requires at least one state",
            ));
        }
        if matrix_a.len() != states.len()
            || matrix_a.iter().any(|row| row.len() != states.len())
            || matrix_b.len() != states.len()
            || matrix_b.iter().any(|row| row.len() != input_count)
        {
            return Err(SolverFailure::new(
                "E-RHS-MATRIX-SHAPE",
                "state-space RHS matrix dimensions do not match state/input layouts",
            ));
        }
        Ok(Self {
            states,
            matrix_a,
            matrix_b,
        })
    }
}

impl RhsEvaluator for StateSpaceRhsEvaluator {
    fn evaluate(&self, input: &RhsInput) -> Result<RhsOutput, SolverFailure> {
        if input.x.len() != self.states.len() {
            return Err(SolverFailure::new(
                "E-RHS-STATE-LAYOUT",
                "RHS input state vector length does not match state metadata",
            ));
        }
        let expected_input_count = self.matrix_b.first().map(|row| row.len()).unwrap_or(0);
        if input.u.len() != expected_input_count {
            return Err(SolverFailure::new(
                "E-RHS-INPUT-LAYOUT",
                "RHS input vector length does not match input metadata",
            ));
        }

        let derivatives = self
            .matrix_a
            .iter()
            .zip(self.matrix_b.iter())
            .map(|(a_row, b_row)| {
                let state_term = a_row
                    .iter()
                    .zip(input.x.iter())
                    .map(|(coefficient, value)| coefficient * value)
                    .sum::<f64>();
                let input_term = b_row
                    .iter()
                    .zip(input.u.iter())
                    .map(|(coefficient, value)| coefficient * value)
                    .sum::<f64>();
                state_term + input_term
            })
            .collect::<Vec<_>>();
        let named_derivatives = self
            .states
            .iter()
            .zip(derivatives.iter().copied())
            .map(|(state, value)| NamedDerivative {
                state: state.name.clone(),
                quantity_kind: state.quantity_kind.clone(),
                canonical_unit: state.canonical_unit.clone(),
                value,
            })
            .collect();

        Ok(RhsOutput {
            derivatives,
            named_derivatives,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualInput {
    pub x: Vec<f64>,
    pub xdot: Option<Vec<f64>>,
    pub z: Vec<f64>,
    pub u: Vec<f64>,
    pub p: Vec<f64>,
    pub t: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualOutput {
    pub residuals: Vec<f64>,
    pub named_residuals: Vec<NamedResidualValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NamedResidualValue {
    pub name: String,
    pub value: f64,
    pub normalized_value: f64,
}

pub trait ResidualEvaluator {
    fn evaluate(&self, input: &ResidualInput) -> Result<ResidualOutput, SolverFailure>;
}

pub struct ClosureResidualEvaluator<F>
where
    F: Fn(&ResidualInput) -> Result<ResidualOutput, SolverFailure>,
{
    evaluator: F,
}

impl<F> ClosureResidualEvaluator<F>
where
    F: Fn(&ResidualInput) -> Result<ResidualOutput, SolverFailure>,
{
    pub fn new(evaluator: F) -> Self {
        Self { evaluator }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_space_rhs_evaluates_named_derivatives() {
        let evaluator = StateSpaceRhsEvaluator::new(
            vec![
                RhsStateInfo::new("T_air", "AbsoluteTemperature", "K"),
                RhsStateInfo::new("T_wall", "AbsoluteTemperature", "K"),
            ],
            vec![vec![-0.2, 0.1], vec![0.05, -0.1]],
            vec![vec![0.2, 0.01], vec![0.03, 0.0]],
            2,
        )
        .unwrap();

        let output = evaluator
            .evaluate(&RhsInput {
                t: 0.0,
                x: vec![300.0, 295.0],
                u: vec![280.0, 1000.0],
                p: Vec::new(),
            })
            .unwrap();

        assert_eq!(output.derivatives.len(), 2);
        assert!((output.derivatives[0] - 35.5).abs() < 1e-9);
        assert!((output.derivatives[1] - (-6.1)).abs() < 1e-9);
        assert_eq!(output.named_derivatives[0].state, "T_air");
        assert_eq!(output.named_derivatives[1].canonical_unit, "K");
    }
}

impl<F> ResidualEvaluator for ClosureResidualEvaluator<F>
where
    F: Fn(&ResidualInput) -> Result<ResidualOutput, SolverFailure>,
{
    fn evaluate(&self, input: &ResidualInput) -> Result<ResidualOutput, SolverFailure> {
        (self.evaluator)(input)
    }
}
