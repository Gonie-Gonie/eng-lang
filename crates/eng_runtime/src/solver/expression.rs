use std::collections::{HashMap, HashSet};

use super::SolverFailure;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ArithmeticExpressionProfile {
    parse_code: &'static str,
    finite_code: &'static str,
    unknown_code: &'static str,
    divide_by_zero_code: &'static str,
    label: &'static str,
}

impl ArithmeticExpressionProfile {
    pub(crate) const SOURCE_RESIDUAL: Self = Self {
        parse_code: "E-SOURCE-EXPR-PARSE",
        finite_code: "E-SOURCE-EXPR-FINITE",
        unknown_code: "E-SOURCE-SYMBOL-UNRESOLVED",
        divide_by_zero_code: "E-SOURCE-EXPR-DIVIDE-BY-ZERO",
        label: "source residual expression",
    };

    pub(crate) const DYNAMIC_COMPONENT_RESIDUAL: Self = Self {
        parse_code: "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL",
        finite_code: "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL",
        unknown_code: "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL",
        divide_by_zero_code: "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL",
        label: "dynamic component residual expression",
    };
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LinearizedArithmeticExpression {
    pub(crate) constant: f64,
    pub(crate) terms: Vec<LinearizedArithmeticTerm>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LinearizedArithmeticTerm {
    pub(crate) symbol: String,
    pub(crate) coefficient: f64,
}

#[derive(Clone, Debug, PartialEq)]
enum ArithmeticExpressionToken {
    Number(f64),
    Identifier(String),
    Plus,
    Minus,
    Star,
    Slash,
    LeftParen,
    RightParen,
}

pub(crate) fn evaluate_source_arithmetic_expression(
    expression: &str,
    symbols: &HashMap<String, f64>,
) -> Result<f64, SolverFailure> {
    let mut ignore_units = |value: f64, _unit: Option<&str>| Ok(value);
    evaluate_arithmetic_expression_with_unit_converter(
        expression,
        symbols,
        &mut ignore_units,
        ArithmeticExpressionProfile::SOURCE_RESIDUAL,
    )
}

pub(crate) fn evaluate_arithmetic_expression_with_unit_converter<F>(
    expression: &str,
    symbols: &HashMap<String, f64>,
    convert_number: &mut F,
    profile: ArithmeticExpressionProfile,
) -> Result<f64, SolverFailure>
where
    F: FnMut(f64, Option<&str>) -> Result<f64, SolverFailure>,
{
    let (rewritten, alias_values) = rewrite_derivative_symbols(expression, symbols);
    let tokens = tokenize_arithmetic_expression(&rewritten, convert_number, profile)?;
    let mut parser = ArithmeticExpressionParser {
        tokens,
        position: 0,
        symbols,
        alias_values: &alias_values,
        profile,
    };
    let value = parser.parse_expression()?;
    if parser.position != parser.tokens.len() {
        return Err(SolverFailure::new(
            profile.parse_code,
            format!("unsupported {} near `{expression}`", profile.label),
        ));
    }
    if !value.is_finite() {
        return Err(SolverFailure::new(
            profile.finite_code,
            format!(
                "{} `{expression}` produced a non-finite value",
                profile.label
            ),
        ));
    }
    Ok(value)
}

pub(crate) fn linearize_arithmetic_expression_with_unit_converter<F>(
    expression: &str,
    variable_symbols: &[String],
    constant_symbols: &HashMap<String, f64>,
    convert_number: &mut F,
    profile: ArithmeticExpressionProfile,
    tolerance: f64,
) -> Result<LinearizedArithmeticExpression, SolverFailure>
where
    F: FnMut(f64, Option<&str>) -> Result<f64, SolverFailure>,
{
    let variable_symbols = unique_symbols(variable_symbols);
    let mut symbols = constant_symbols.clone();
    for symbol in &variable_symbols {
        symbols.insert(symbol.clone(), 0.0);
    }

    let constant = evaluate_arithmetic_expression_with_unit_converter(
        expression,
        &symbols,
        convert_number,
        profile,
    )?;
    let mut terms = Vec::new();
    for symbol in &variable_symbols {
        symbols.insert(symbol.clone(), 1.0);
        let value = evaluate_arithmetic_expression_with_unit_converter(
            expression,
            &symbols,
            convert_number,
            profile,
        )?;
        let coefficient = value - constant;
        if !coefficient.is_finite() {
            return Err(SolverFailure::new(
                profile.finite_code,
                format!(
                    "{} `{expression}` produced a non-finite linear coefficient",
                    profile.label
                ),
            ));
        }
        if coefficient.abs() > tolerance.max(1e-12) {
            terms.push(LinearizedArithmeticTerm {
                symbol: symbol.clone(),
                coefficient,
            });
        }
        symbols.insert(symbol.clone(), 0.0);
    }

    verify_linearized_expression(
        expression,
        &variable_symbols,
        &mut symbols,
        constant,
        &terms,
        convert_number,
        profile,
        tolerance,
    )?;

    Ok(LinearizedArithmeticExpression { constant, terms })
}

fn unique_symbols(symbols: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    symbols
        .iter()
        .filter(|symbol| seen.insert((*symbol).clone()))
        .cloned()
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn verify_linearized_expression<F>(
    expression: &str,
    variable_symbols: &[String],
    symbols: &mut HashMap<String, f64>,
    constant: f64,
    terms: &[LinearizedArithmeticTerm],
    convert_number: &mut F,
    profile: ArithmeticExpressionProfile,
    tolerance: f64,
) -> Result<(), SolverFailure>
where
    F: FnMut(f64, Option<&str>) -> Result<f64, SolverFailure>,
{
    if variable_symbols.is_empty() {
        return Ok(());
    }
    let tolerance = tolerance.max(1e-9);
    for sample_index in 0..3 {
        for (index, symbol) in variable_symbols.iter().enumerate() {
            let value = match sample_index {
                0 => (index as f64 + 1.0) * 0.5,
                1 if index % 2 == 0 => -1.25 - index as f64 * 0.25,
                1 => 2.0 + index as f64 * 0.5,
                _ => 1.75 + index as f64 * 0.375,
            };
            symbols.insert(symbol.clone(), value);
        }
        let evaluated = evaluate_arithmetic_expression_with_unit_converter(
            expression,
            symbols,
            convert_number,
            profile,
        )?;
        let predicted = terms.iter().fold(constant, |sum, term| {
            sum + term.coefficient * symbols.get(&term.symbol).copied().unwrap_or_default()
        });
        let scale = evaluated.abs().max(predicted.abs()).max(1.0);
        if (evaluated - predicted).abs() > tolerance * scale {
            return Err(SolverFailure::new(
                profile.parse_code,
                format!(
                    "{} `{expression}` is not linear in its solver symbols",
                    profile.label
                ),
            ));
        }
    }
    for symbol in variable_symbols {
        symbols.insert(symbol.clone(), 0.0);
    }
    Ok(())
}

fn rewrite_derivative_symbols(
    expression: &str,
    symbols: &HashMap<String, f64>,
) -> (String, HashMap<String, f64>) {
    let mut rewritten = expression.to_owned();
    let mut alias_values = HashMap::new();
    let mut derivative_symbols = symbols
        .iter()
        .filter(|(name, _)| name.starts_with("der(") && name.ends_with(')'))
        .collect::<Vec<_>>();
    derivative_symbols.sort_by_key(|(name, _)| std::cmp::Reverse(name.len()));
    for (index, (name, value)) in derivative_symbols.into_iter().enumerate() {
        let alias = format!("__derivative_{index}");
        rewritten = rewritten.replace(name.as_str(), &alias);
        alias_values.insert(alias, *value);
    }
    (rewritten, alias_values)
}

fn tokenize_arithmetic_expression<F>(
    expression: &str,
    convert_number: &mut F,
    profile: ArithmeticExpressionProfile,
) -> Result<Vec<ArithmeticExpressionToken>, SolverFailure>
where
    F: FnMut(f64, Option<&str>) -> Result<f64, SolverFailure>,
{
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
                tokens.push(ArithmeticExpressionToken::Plus);
                index += 1;
            }
            '-' => {
                tokens.push(ArithmeticExpressionToken::Minus);
                index += 1;
            }
            '*' => {
                tokens.push(ArithmeticExpressionToken::Star);
                index += 1;
            }
            '/' => {
                tokens.push(ArithmeticExpressionToken::Slash);
                index += 1;
            }
            '(' => {
                tokens.push(ArithmeticExpressionToken::LeftParen);
                index += 1;
            }
            ')' => {
                tokens.push(ArithmeticExpressionToken::RightParen);
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
                        profile.parse_code,
                        format!("invalid numeric literal `{literal}` in {}", profile.label),
                    )
                })?;
                let (next_index, unit) = consume_optional_unit_suffix(&chars, index);
                let value = convert_number(value, unit.as_deref())?;
                tokens.push(ArithmeticExpressionToken::Number(value));
                index = next_index;
            }
            _ if is_identifier_start(character) => {
                let start = index;
                index += 1;
                while index < chars.len() && is_identifier_continue(chars[index]) {
                    index += 1;
                }
                tokens.push(ArithmeticExpressionToken::Identifier(
                    chars[start..index].iter().collect(),
                ));
            }
            _ => {
                return Err(SolverFailure::new(
                    profile.parse_code,
                    format!("unsupported character `{character}` in {}", profile.label),
                ));
            }
        }
    }
    Ok(tokens)
}

fn consume_optional_unit_suffix(chars: &[char], index: usize) -> (usize, Option<String>) {
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
        return (index, None);
    }

    let suffix_start = cursor;
    while let Some(character) = chars.get(cursor) {
        if character.is_ascii_whitespace() || matches!(character, '+' | '-' | '*' | '(' | ')') {
            break;
        }
        if *character == '/' || character.is_ascii_alphanumeric() || *character == '\u{00b0}' {
            cursor += 1;
        } else {
            break;
        }
    }
    let suffix = chars[suffix_start..cursor].iter().collect::<String>();
    if !suffix.is_empty()
        && suffix
            .chars()
            .any(|character| character.is_ascii_alphabetic() || character == '\u{00b0}')
    {
        (cursor, Some(suffix))
    } else {
        (index, None)
    }
}

fn is_identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
}

fn is_identifier_continue(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_' || character == '.'
}

struct ArithmeticExpressionParser<'a> {
    tokens: Vec<ArithmeticExpressionToken>,
    position: usize,
    symbols: &'a HashMap<String, f64>,
    alias_values: &'a HashMap<String, f64>,
    profile: ArithmeticExpressionProfile,
}

impl ArithmeticExpressionParser<'_> {
    fn parse_expression(&mut self) -> Result<f64, SolverFailure> {
        let mut value = self.parse_term()?;
        loop {
            match self.peek() {
                Some(ArithmeticExpressionToken::Plus) => {
                    self.position += 1;
                    value += self.parse_term()?;
                }
                Some(ArithmeticExpressionToken::Minus) => {
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
                Some(ArithmeticExpressionToken::Star) => {
                    self.position += 1;
                    value *= self.parse_factor()?;
                }
                Some(ArithmeticExpressionToken::Slash) => {
                    self.position += 1;
                    let divisor = self.parse_factor()?;
                    if divisor.abs() <= f64::EPSILON {
                        return Err(SolverFailure::new(
                            self.profile.divide_by_zero_code,
                            format!("{} attempted division by zero", self.profile.label),
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
                self.profile.parse_code,
                format!("{} ended unexpectedly", self.profile.label),
            ));
        };
        match token {
            ArithmeticExpressionToken::Number(value) => Ok(value),
            ArithmeticExpressionToken::Identifier(name) => {
                self.symbol_value(&name).ok_or_else(|| {
                    SolverFailure::new(
                        self.profile.unknown_code,
                        format!("{} references unknown symbol `{name}`", self.profile.label),
                    )
                })
            }
            ArithmeticExpressionToken::Minus => Ok(-self.parse_factor()?),
            ArithmeticExpressionToken::Plus => self.parse_factor(),
            ArithmeticExpressionToken::LeftParen => {
                let value = self.parse_expression()?;
                match self.next() {
                    Some(ArithmeticExpressionToken::RightParen) => Ok(value),
                    _ => Err(SolverFailure::new(
                        self.profile.parse_code,
                        format!("{} has an unclosed parenthesis", self.profile.label),
                    )),
                }
            }
            _ => Err(SolverFailure::new(
                self.profile.parse_code,
                format!("{} expected a value", self.profile.label),
            )),
        }
    }

    fn symbol_value(&self, name: &str) -> Option<f64> {
        self.alias_values
            .get(name)
            .copied()
            .or_else(|| self.symbols.get(name).copied())
    }

    fn peek(&self) -> Option<&ArithmeticExpressionToken> {
        self.tokens.get(self.position)
    }

    fn next(&mut self) -> Option<&ArithmeticExpressionToken> {
        let token = self.tokens.get(self.position);
        if token.is_some() {
            self.position += 1;
        }
        token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_source_expression_with_derivative_aliases() {
        let symbols = HashMap::from([
            ("x".to_owned(), 2.0),
            ("der(x)".to_owned(), 3.0),
            ("k".to_owned(), 4.0),
        ]);

        let value =
            evaluate_source_arithmetic_expression("k * (der(x) + x) / 2", &symbols).unwrap();

        assert_eq!(value, 10.0);
    }

    #[test]
    fn linearizes_parenthesized_expression_with_unit_literals() {
        let variable_symbols = vec!["x".to_owned(), "y".to_owned()];
        let constants = HashMap::from([("bias".to_owned(), 3.0)]);
        let mut convert = |value: f64, unit: Option<&str>| {
            Ok(if unit == Some("kPa") {
                value * 1000.0
            } else {
                value
            })
        };

        let linearized = linearize_arithmetic_expression_with_unit_converter(
            "2 * (x - y) + bias + 1 kPa",
            &variable_symbols,
            &constants,
            &mut convert,
            ArithmeticExpressionProfile::DYNAMIC_COMPONENT_RESIDUAL,
            1e-9,
        )
        .unwrap();

        assert_eq!(linearized.constant, 1003.0);
        assert_eq!(
            linearized.terms,
            vec![
                LinearizedArithmeticTerm {
                    symbol: "x".to_owned(),
                    coefficient: 2.0,
                },
                LinearizedArithmeticTerm {
                    symbol: "y".to_owned(),
                    coefficient: -2.0,
                },
            ]
        );
    }

    #[test]
    fn rejects_nonlinear_linearization() {
        let variable_symbols = vec!["x".to_owned(), "y".to_owned()];
        let constants = HashMap::new();
        let mut convert = |value: f64, _unit: Option<&str>| Ok(value);

        let failure = linearize_arithmetic_expression_with_unit_converter(
            "x * y",
            &variable_symbols,
            &constants,
            &mut convert,
            ArithmeticExpressionProfile::DYNAMIC_COMPONENT_RESIDUAL,
            1e-9,
        )
        .unwrap_err();

        assert_eq!(failure.code, "E-DYNAMIC-COMPONENT-ASSEMBLY-RESIDUAL");
        assert!(failure.message.contains("not linear"));
    }
}
