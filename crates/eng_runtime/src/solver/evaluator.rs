use std::collections::HashMap;

use super::{
    diagnostics::SolverFailure,
    expression::{
        parse_arithmetic_expression_with_symbol_metadata_and_unit_converter,
        ArithmeticExpressionProfile, ArithmeticUnitMetadata, ParsedArithmeticExpression,
    },
};

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
pub struct SourceRhsEquation {
    pub state: String,
    pub left: String,
    pub right: String,
}

impl SourceRhsEquation {
    pub fn new(
        state: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
    ) -> Self {
        Self {
            state: state.into(),
            left: left.into(),
            right: right.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceRhsEvaluator {
    states: Vec<RhsStateInfo>,
    input_names: Vec<String>,
    parameter_names: Vec<String>,
    equations: Vec<ParsedSourceRhsEquation>,
}

#[derive(Clone, Debug, PartialEq)]
struct ParsedSourceRhsEquation {
    state: String,
    coefficient_source: String,
    coefficient: ParsedArithmeticExpression,
    rhs_source: String,
    rhs: ParsedArithmeticExpression,
}

impl SourceRhsEvaluator {
    pub fn new(
        states: Vec<RhsStateInfo>,
        input_names: Vec<String>,
        parameter_names: Vec<String>,
        equations: Vec<SourceRhsEquation>,
    ) -> Result<Self, SolverFailure> {
        if states.is_empty() {
            return Err(SolverFailure::new(
                "E-RHS-STATE-LAYOUT",
                "source RHS requires at least one state",
            ));
        }
        if equations.len() != states.len() {
            return Err(SolverFailure::new(
                "E-RHS-EQUATION-LAYOUT",
                "source RHS equation count must match the state layout",
            ));
        }
        for state in &states {
            let count = equations
                .iter()
                .filter(|equation| equation.state == state.name)
                .count();
            if count != 1 {
                return Err(SolverFailure::new(
                    "E-RHS-EQUATION-LAYOUT",
                    format!(
                        "source RHS requires exactly one equation for state `{}`",
                        state.name
                    ),
                ));
            }
        }
        let symbols = source_rhs_parse_symbols(&states, &input_names, &parameter_names);
        let symbol_units = source_rhs_symbol_metadata(&states);
        let equations = equations
            .into_iter()
            .map(|equation| {
                let coefficient_source =
                    derivative_coefficient_expression(&equation.left, &equation.state)?;
                let coefficient =
                    parse_source_rhs_expression(&coefficient_source, &symbols, &symbol_units)
                        .map_err(|failure| {
                            SolverFailure::new(
                                failure.code,
                                format!(
                                    "{} while parsing derivative coefficient for state `{}`",
                                    failure.message, equation.state
                                ),
                            )
                        })?;
                let rhs = parse_source_rhs_expression(&equation.right, &symbols, &symbol_units)
                    .map_err(|failure| {
                        SolverFailure::new(
                            failure.code,
                            format!(
                                "{} while parsing derivative RHS for state `{}`",
                                failure.message, equation.state
                            ),
                        )
                    })?;
                Ok(ParsedSourceRhsEquation {
                    state: equation.state,
                    coefficient_source,
                    coefficient,
                    rhs_source: equation.right,
                    rhs,
                })
            })
            .collect::<Result<Vec<_>, SolverFailure>>()?;

        Ok(Self {
            states,
            input_names,
            parameter_names,
            equations,
        })
    }

    fn evaluation_symbols(&self, input: &RhsInput) -> HashMap<String, f64> {
        let mut symbols = HashMap::new();
        symbols.insert("t".to_owned(), input.t);
        symbols.insert("time".to_owned(), input.t);
        for (state, value) in self.states.iter().zip(input.x.iter().copied()) {
            symbols.insert(state.name.clone(), value);
        }
        for (name, value) in self.input_names.iter().zip(input.u.iter().copied()) {
            symbols.insert(name.clone(), value);
        }
        for (name, value) in self.parameter_names.iter().zip(input.p.iter().copied()) {
            symbols.insert(name.clone(), value);
        }
        symbols
    }
}

impl RhsEvaluator for SourceRhsEvaluator {
    fn evaluate(&self, input: &RhsInput) -> Result<RhsOutput, SolverFailure> {
        if input.x.len() != self.states.len() {
            return Err(SolverFailure::new(
                "E-RHS-STATE-LAYOUT",
                "RHS input state vector length does not match state metadata",
            ));
        }
        if input.u.len() != self.input_names.len() {
            return Err(SolverFailure::new(
                "E-RHS-INPUT-LAYOUT",
                "RHS input vector length does not match input metadata",
            ));
        }
        if input.p.len() != self.parameter_names.len() {
            return Err(SolverFailure::new(
                "E-RHS-PARAMETER-LAYOUT",
                "RHS parameter vector length does not match parameter metadata",
            ));
        }
        if !input.t.is_finite() {
            return Err(SolverFailure::new(
                "E-RHS-TIME-FINITE",
                "RHS evaluation time must be finite",
            ));
        }
        ensure_finite_values("E-RHS-STATE-FINITE", "RHS state", &input.x)?;
        ensure_finite_values("E-RHS-INPUT-FINITE", "RHS input", &input.u)?;
        ensure_finite_values("E-RHS-PARAMETER-FINITE", "RHS parameter", &input.p)?;

        let symbols = self.evaluation_symbols(input);
        let mut derivatives = vec![0.0; self.states.len()];
        for equation in &self.equations {
            let Some(state_index) = self
                .states
                .iter()
                .position(|state| state.name == equation.state)
            else {
                return Err(SolverFailure::new(
                    "E-RHS-EQUATION-LAYOUT",
                    format!("RHS equation references unknown state `{}`", equation.state),
                ));
            };
            let coefficient = equation.coefficient.evaluate(&symbols).map_err(|failure| {
                SolverFailure::new(
                    failure.code,
                    format!(
                        "{} while evaluating derivative coefficient `{}` for state `{}`",
                        failure.message, equation.coefficient_source, equation.state
                    ),
                )
            })?;
            if !coefficient.is_finite() || coefficient.abs() <= f64::EPSILON {
                return Err(SolverFailure::new(
                    "E-RHS-DERIVATIVE-COEFFICIENT",
                    format!(
                        "derivative coefficient for state `{}` must be non-zero and finite",
                        equation.state
                    ),
                ));
            }
            let rhs = equation.rhs.evaluate(&symbols).map_err(|failure| {
                SolverFailure::new(
                    failure.code,
                    format!(
                        "{} while evaluating derivative RHS `{}` for state `{}`",
                        failure.message, equation.rhs_source, equation.state
                    ),
                )
            })?;
            derivatives[state_index] = rhs / coefficient;
        }

        ensure_finite_values("E-RHS-DERIVATIVE-FINITE", "RHS derivative", &derivatives)?;
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

fn source_rhs_parse_symbols(
    states: &[RhsStateInfo],
    input_names: &[String],
    parameter_names: &[String],
) -> HashMap<String, f64> {
    let mut symbols = HashMap::new();
    symbols.insert("t".to_owned(), 0.0);
    symbols.insert("time".to_owned(), 0.0);
    for state in states {
        symbols.insert(state.name.clone(), 0.0);
    }
    for name in input_names {
        symbols.insert(name.clone(), 0.0);
    }
    for name in parameter_names {
        symbols.insert(name.clone(), 0.0);
    }
    symbols
}

fn source_rhs_symbol_metadata(states: &[RhsStateInfo]) -> HashMap<String, ArithmeticUnitMetadata> {
    let mut metadata = HashMap::new();
    metadata.insert(
        "t".to_owned(),
        ArithmeticUnitMetadata {
            display_unit: "s".to_owned(),
            canonical_unit: "s".to_owned(),
            quantity_kind: "Time".to_owned(),
        },
    );
    metadata.insert(
        "time".to_owned(),
        ArithmeticUnitMetadata {
            display_unit: "s".to_owned(),
            canonical_unit: "s".to_owned(),
            quantity_kind: "Time".to_owned(),
        },
    );
    for state in states {
        metadata.insert(
            state.name.clone(),
            ArithmeticUnitMetadata {
                display_unit: state.canonical_unit.clone(),
                canonical_unit: state.canonical_unit.clone(),
                quantity_kind: state.quantity_kind.clone(),
            },
        );
    }
    metadata
}

fn parse_source_rhs_expression(
    expression: &str,
    symbols: &HashMap<String, f64>,
    symbol_units: &HashMap<String, ArithmeticUnitMetadata>,
) -> Result<ParsedArithmeticExpression, SolverFailure> {
    let mut ignore_units = |value: f64, _unit: Option<&str>| Ok(value);
    parse_arithmetic_expression_with_symbol_metadata_and_unit_converter(
        expression,
        symbols,
        symbol_units,
        &mut ignore_units,
        ArithmeticExpressionProfile::SOURCE_RHS,
    )
}

fn derivative_coefficient_expression(left: &str, state: &str) -> Result<String, SolverFailure> {
    let derivative = format!("der({state})");
    if !left.contains(&derivative) {
        return Err(SolverFailure::new(
            "E-RHS-DERIVATIVE-LHS",
            format!("equation left-hand side must contain `{derivative}`"),
        ));
    }
    let expression = left.replace(&derivative, "1");
    if expression.contains("der(") {
        return Err(SolverFailure::new(
            "E-RHS-DERIVATIVE-LHS",
            "source RHS equations must contain exactly one derivative term",
        ));
    }
    Ok(expression)
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
        ensure_finite_matrix("E-RHS-MATRIX-FINITE", "state-space RHS A matrix", &matrix_a)?;
        ensure_finite_matrix("E-RHS-MATRIX-FINITE", "state-space RHS B matrix", &matrix_b)?;
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
        if !input.t.is_finite() {
            return Err(SolverFailure::new(
                "E-RHS-TIME-FINITE",
                "RHS evaluation time must be finite",
            ));
        }
        ensure_finite_values("E-RHS-STATE-FINITE", "RHS state", &input.x)?;
        ensure_finite_values("E-RHS-INPUT-FINITE", "RHS input", &input.u)?;
        ensure_finite_values("E-RHS-PARAMETER-FINITE", "RHS parameter", &input.p)?;

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
        ensure_finite_values("E-RHS-DERIVATIVE-FINITE", "RHS derivative", &derivatives)?;
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

fn ensure_finite_matrix(code: &str, label: &str, matrix: &[Vec<f64>]) -> Result<(), SolverFailure> {
    if matrix.iter().flatten().all(|value| value.is_finite()) {
        return Ok(());
    }
    Err(SolverFailure::new(
        code,
        format!("{label} contains a non-finite value"),
    ))
}

fn ensure_finite_values(code: &str, label: &str, values: &[f64]) -> Result<(), SolverFailure> {
    if values.iter().all(|value| value.is_finite()) {
        return Ok(());
    }
    Err(SolverFailure::new(
        code,
        format!("{label} vector contains a non-finite value"),
    ))
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

impl<F> ResidualEvaluator for ClosureResidualEvaluator<F>
where
    F: Fn(&ResidualInput) -> Result<ResidualOutput, SolverFailure>,
{
    fn evaluate(&self, input: &ResidualInput) -> Result<ResidualOutput, SolverFailure> {
        (self.evaluator)(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_rhs_evaluates_two_state_thermal_derivatives() {
        let evaluator = SourceRhsEvaluator::new(
            vec![
                RhsStateInfo::new("T_air", "AbsoluteTemperature", "K"),
                RhsStateInfo::new("T_wall", "AbsoluteTemperature", "K"),
            ],
            vec!["T_out".to_owned(), "Q_hvac".to_owned()],
            vec![
                "C_air".to_owned(),
                "C_wall".to_owned(),
                "UA_aw".to_owned(),
                "UA_ao".to_owned(),
                "UA_wo".to_owned(),
            ],
            vec![
                SourceRhsEquation::new(
                    "T_air",
                    "C_air * der(T_air)",
                    "UA_aw * (T_wall - T_air) + UA_ao * (T_out - T_air) + Q_hvac",
                ),
                SourceRhsEquation::new(
                    "T_wall",
                    "C_wall * der(T_wall)",
                    "UA_aw * (T_air - T_wall) + UA_wo * (T_out - T_wall)",
                ),
            ],
        )
        .unwrap();

        let output = evaluator
            .evaluate(&RhsInput {
                t: 0.0,
                x: vec![295.0, 293.0],
                u: vec![285.0, 1000.0],
                p: vec![100_000.0, 200_000.0, 50.0, 100.0, 20.0],
            })
            .unwrap();

        assert_eq!(output.derivatives.len(), 2);
        assert!((output.derivatives[0] - (-0.001)).abs() < 1e-12);
        assert!((output.derivatives[1] - (-0.0003)).abs() < 1e-12);
        assert_eq!(output.named_derivatives[0].state, "T_air");
        assert_eq!(output.named_derivatives[1].canonical_unit, "K");
    }

    #[test]
    fn source_rhs_rejects_unresolved_symbols_and_zero_coefficients() {
        let unresolved = SourceRhsEvaluator::new(
            vec![RhsStateInfo::new("T", "AbsoluteTemperature", "K")],
            Vec::new(),
            vec!["C".to_owned()],
            vec![SourceRhsEquation::new("T", "C * der(T)", "Q_missing")],
        )
        .unwrap()
        .evaluate(&RhsInput {
            t: 0.0,
            x: vec![295.0],
            u: Vec::new(),
            p: vec![100_000.0],
        })
        .unwrap_err();
        assert_eq!(unresolved.code, "E-RHS-SYMBOL-UNRESOLVED");

        let zero_coefficient = SourceRhsEvaluator::new(
            vec![RhsStateInfo::new("T", "AbsoluteTemperature", "K")],
            Vec::new(),
            vec!["C".to_owned()],
            vec![SourceRhsEquation::new("T", "0 * der(T)", "0 W")],
        )
        .unwrap()
        .evaluate(&RhsInput {
            t: 0.0,
            x: vec![295.0],
            u: Vec::new(),
            p: vec![100_000.0],
        })
        .unwrap_err();
        assert_eq!(zero_coefficient.code, "E-RHS-DERIVATIVE-COEFFICIENT");
    }

    #[test]
    fn source_rhs_preserves_shared_expression_unit_metadata() {
        let evaluator = SourceRhsEvaluator::new(
            vec![RhsStateInfo::new("T", "AbsoluteTemperature", "K")],
            Vec::new(),
            vec!["C".to_owned()],
            vec![SourceRhsEquation::new("T", "C * der(T)", "1 W")],
        )
        .unwrap();

        let rhs_unit = evaluator.equations[0].rhs.root_unit.as_ref().unwrap();
        assert_eq!(rhs_unit.display_unit, "W");
        assert_eq!(rhs_unit.canonical_unit, "W");
        assert_eq!(rhs_unit.quantity_kind, "Power");

        let output = evaluator
            .evaluate(&RhsInput {
                t: 0.0,
                x: vec![295.0],
                u: Vec::new(),
                p: vec![2.0],
            })
            .unwrap();
        assert_eq!(output.derivatives, vec![0.5]);
    }
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

    #[test]
    fn state_space_rhs_rejects_nonfinite_matrices() {
        let failure = StateSpaceRhsEvaluator::new(
            vec![RhsStateInfo::new("x", "Dimensionless", "1")],
            vec![vec![f64::NAN]],
            vec![vec![0.0]],
            1,
        )
        .unwrap_err();

        assert_eq!(failure.code, "E-RHS-MATRIX-FINITE");
    }

    #[test]
    fn state_space_rhs_rejects_nonfinite_inputs() {
        let evaluator = StateSpaceRhsEvaluator::new(
            vec![RhsStateInfo::new("x", "Dimensionless", "1")],
            vec![vec![1.0]],
            vec![vec![1.0]],
            1,
        )
        .unwrap();

        let failure = evaluator
            .evaluate(&RhsInput {
                t: f64::INFINITY,
                x: vec![1.0],
                u: vec![1.0],
                p: Vec::new(),
            })
            .unwrap_err();
        assert_eq!(failure.code, "E-RHS-TIME-FINITE");

        let failure = evaluator
            .evaluate(&RhsInput {
                t: 0.0,
                x: vec![f64::NAN],
                u: vec![1.0],
                p: Vec::new(),
            })
            .unwrap_err();
        assert_eq!(failure.code, "E-RHS-STATE-FINITE");

        let failure = evaluator
            .evaluate(&RhsInput {
                t: 0.0,
                x: vec![1.0],
                u: vec![f64::INFINITY],
                p: Vec::new(),
            })
            .unwrap_err();
        assert_eq!(failure.code, "E-RHS-INPUT-FINITE");

        let failure = evaluator
            .evaluate(&RhsInput {
                t: 0.0,
                x: vec![1.0],
                u: vec![1.0],
                p: vec![f64::NAN],
            })
            .unwrap_err();
        assert_eq!(failure.code, "E-RHS-PARAMETER-FINITE");
    }

    #[test]
    fn state_space_rhs_rejects_nonfinite_derivatives() {
        let evaluator = StateSpaceRhsEvaluator::new(
            vec![RhsStateInfo::new("x", "Dimensionless", "1")],
            vec![vec![f64::MAX]],
            vec![vec![0.0]],
            1,
        )
        .unwrap();

        let failure = evaluator
            .evaluate(&RhsInput {
                t: 0.0,
                x: vec![2.0],
                u: vec![0.0],
                p: Vec::new(),
            })
            .unwrap_err();

        assert_eq!(failure.code, "E-RHS-DERIVATIVE-FINITE");
    }
}
