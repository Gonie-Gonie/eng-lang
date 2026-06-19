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
    equations: Vec<SourceRhsEquation>,
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
        Ok(Self {
            states,
            input_names,
            parameter_names,
            equations,
        })
    }

    fn evaluate_expression(
        &self,
        expression: &str,
        input: &RhsInput,
    ) -> Result<f64, SolverFailure> {
        let tokens = tokenize_expression(expression)?;
        let mut parser = ExpressionParser {
            tokens,
            position: 0,
            evaluator: self,
            input,
        };
        let value = parser.parse_expression()?;
        if parser.position != parser.tokens.len() {
            return Err(SolverFailure::new(
                "E-RHS-EXPR-PARSE",
                format!("unsupported RHS expression near `{expression}`"),
            ));
        }
        if !value.is_finite() {
            return Err(SolverFailure::new(
                "E-RHS-EXPR-FINITE",
                format!("RHS expression `{expression}` produced a non-finite value"),
            ));
        }
        Ok(value)
    }

    fn symbol_value(&self, name: &str, input: &RhsInput) -> Option<f64> {
        if matches!(name, "t" | "time") {
            return Some(input.t);
        }
        if let Some(index) = self.states.iter().position(|state| state.name == name) {
            return input.x.get(index).copied();
        }
        if let Some(index) = self
            .input_names
            .iter()
            .position(|input_name| input_name == name)
        {
            return input.u.get(index).copied();
        }
        if let Some(index) = self
            .parameter_names
            .iter()
            .position(|parameter_name| parameter_name == name)
        {
            return input.p.get(index).copied();
        }
        None
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
            let coefficient_expression =
                derivative_coefficient_expression(&equation.left, &equation.state)?;
            let coefficient = self.evaluate_expression(&coefficient_expression, input)?;
            if !coefficient.is_finite() || coefficient.abs() <= f64::EPSILON {
                return Err(SolverFailure::new(
                    "E-RHS-DERIVATIVE-COEFFICIENT",
                    format!(
                        "derivative coefficient for state `{}` must be non-zero and finite",
                        equation.state
                    ),
                ));
            }
            let rhs = self.evaluate_expression(&equation.right, input)?;
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
enum ExpressionToken {
    Number(f64),
    Identifier(String),
    Plus,
    Minus,
    Star,
    Slash,
    LeftParen,
    RightParen,
}

fn tokenize_expression(expression: &str) -> Result<Vec<ExpressionToken>, SolverFailure> {
    let chars = expression.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let character = chars[index];
        if character.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        match character {
            '+' => {
                tokens.push(ExpressionToken::Plus);
                index += 1;
            }
            '-' => {
                tokens.push(ExpressionToken::Minus);
                index += 1;
            }
            '*' => {
                tokens.push(ExpressionToken::Star);
                index += 1;
            }
            '/' => {
                tokens.push(ExpressionToken::Slash);
                index += 1;
            }
            '(' => {
                tokens.push(ExpressionToken::LeftParen);
                index += 1;
            }
            ')' => {
                tokens.push(ExpressionToken::RightParen);
                index += 1;
            }
            _ if character.is_ascii_digit()
                || character == '.'
                    && chars
                        .get(index + 1)
                        .is_some_and(|next| next.is_ascii_digit()) =>
            {
                let start = index;
                index += 1;
                while index < chars.len() {
                    let current = chars[index];
                    if current.is_ascii_digit() || current == '.' {
                        index += 1;
                    } else if matches!(current, 'e' | 'E') {
                        index += 1;
                        if chars
                            .get(index)
                            .is_some_and(|next| matches!(next, '+' | '-'))
                        {
                            index += 1;
                        }
                    } else {
                        break;
                    }
                }
                let literal = chars[start..index].iter().collect::<String>();
                let value = literal.parse::<f64>().map_err(|_| {
                    SolverFailure::new(
                        "E-RHS-EXPR-PARSE",
                        format!("invalid numeric literal `{literal}` in RHS expression"),
                    )
                })?;
                tokens.push(ExpressionToken::Number(value));
                index = consume_optional_unit_suffix(&chars, index);
            }
            _ if is_identifier_start(character) => {
                let start = index;
                index += 1;
                while index < chars.len() && is_identifier_continue(chars[index]) {
                    index += 1;
                }
                tokens.push(ExpressionToken::Identifier(
                    chars[start..index].iter().collect(),
                ));
            }
            _ => {
                return Err(SolverFailure::new(
                    "E-RHS-EXPR-PARSE",
                    format!("unsupported character `{character}` in RHS expression"),
                ));
            }
        }
    }
    Ok(tokens)
}

fn consume_optional_unit_suffix(chars: &[char], index: usize) -> usize {
    let mut cursor = index;
    let mut saw_whitespace = false;
    while chars
        .get(cursor)
        .is_some_and(|character| character.is_ascii_whitespace())
    {
        saw_whitespace = true;
        cursor += 1;
    }
    if !saw_whitespace {
        return index;
    }

    let suffix_start = cursor;
    while let Some(character) = chars.get(cursor) {
        if character.is_ascii_whitespace() || matches!(character, '+' | '-' | '*' | '(' | ')') {
            break;
        }
        if *character == '/' || character.is_ascii_alphanumeric() || *character == '°' {
            cursor += 1;
        } else {
            break;
        }
    }
    let suffix = chars[suffix_start..cursor].iter().collect::<String>();
    if !suffix.is_empty()
        && suffix
            .chars()
            .any(|character| character.is_ascii_alphabetic() || character == '°')
    {
        cursor
    } else {
        index
    }
}

fn is_identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
}

fn is_identifier_continue(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_' || character == '.'
}

struct ExpressionParser<'a> {
    tokens: Vec<ExpressionToken>,
    position: usize,
    evaluator: &'a SourceRhsEvaluator,
    input: &'a RhsInput,
}

impl ExpressionParser<'_> {
    fn parse_expression(&mut self) -> Result<f64, SolverFailure> {
        let mut value = self.parse_term()?;
        loop {
            match self.peek() {
                Some(ExpressionToken::Plus) => {
                    self.position += 1;
                    value += self.parse_term()?;
                }
                Some(ExpressionToken::Minus) => {
                    self.position += 1;
                    value -= self.parse_term()?;
                }
                _ => return Ok(value),
            }
        }
    }

    fn parse_term(&mut self) -> Result<f64, SolverFailure> {
        let mut value = self.parse_factor()?;
        loop {
            match self.peek() {
                Some(ExpressionToken::Star) => {
                    self.position += 1;
                    value *= self.parse_factor()?;
                }
                Some(ExpressionToken::Slash) => {
                    self.position += 1;
                    let divisor = self.parse_factor()?;
                    if divisor.abs() <= f64::EPSILON {
                        return Err(SolverFailure::new(
                            "E-RHS-EXPR-DIVIDE-BY-ZERO",
                            "RHS expression attempted division by zero",
                        ));
                    }
                    value /= divisor;
                }
                _ => return Ok(value),
            }
        }
    }

    fn parse_factor(&mut self) -> Result<f64, SolverFailure> {
        let Some(token) = self.next().cloned() else {
            return Err(SolverFailure::new(
                "E-RHS-EXPR-PARSE",
                "RHS expression ended unexpectedly",
            ));
        };
        match token {
            ExpressionToken::Number(value) => Ok(value),
            ExpressionToken::Identifier(name) => self
                .evaluator
                .symbol_value(&name, self.input)
                .ok_or_else(|| {
                    SolverFailure::new(
                        "E-RHS-SYMBOL-UNRESOLVED",
                        format!("RHS expression references unknown symbol `{name}`"),
                    )
                }),
            ExpressionToken::Minus => Ok(-self.parse_factor()?),
            ExpressionToken::Plus => self.parse_factor(),
            ExpressionToken::LeftParen => {
                let value = self.parse_expression()?;
                match self.next() {
                    Some(ExpressionToken::RightParen) => Ok(value),
                    _ => Err(SolverFailure::new(
                        "E-RHS-EXPR-PARSE",
                        "RHS expression has an unclosed parenthesis",
                    )),
                }
            }
            _ => Err(SolverFailure::new(
                "E-RHS-EXPR-PARSE",
                "RHS expression expected a value",
            )),
        }
    }

    fn peek(&self) -> Option<&ExpressionToken> {
        self.tokens.get(self.position)
    }

    fn next(&mut self) -> Option<&ExpressionToken> {
        let token = self.tokens.get(self.position);
        if token.is_some() {
            self.position += 1;
        }
        token
    }
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
