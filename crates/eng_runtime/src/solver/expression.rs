use std::collections::{HashMap, HashSet};

use eng_compiler::{all_unit_infos, normalize_unit};

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
pub(crate) struct ParsedArithmeticExpression {
    source: String,
    root: ArithmeticExpressionNode,
    alias_symbols: HashMap<String, String>,
    profile: ArithmeticExpressionProfile,
    pub(crate) root_unit: Option<ArithmeticUnitMetadata>,
    pub(crate) unit_literals: Vec<ArithmeticUnitMetadata>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ArithmeticUnitMetadata {
    pub(crate) display_unit: String,
    pub(crate) canonical_unit: String,
    pub(crate) quantity_kind: String,
}

#[derive(Clone, Debug, PartialEq)]
enum ArithmeticExpressionNode {
    Number {
        value: f64,
        unit: Option<ArithmeticUnitMetadata>,
    },
    Symbol {
        name: String,
        unit: Option<ArithmeticUnitMetadata>,
    },
    UnaryMinus {
        value: Box<ArithmeticExpressionNode>,
        unit: Option<ArithmeticUnitMetadata>,
    },
    Binary {
        operator: ArithmeticExpressionBinaryOperator,
        left: Box<ArithmeticExpressionNode>,
        right: Box<ArithmeticExpressionNode>,
        unit: Option<ArithmeticUnitMetadata>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArithmeticExpressionBinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl ParsedArithmeticExpression {
    pub(crate) fn evaluate(&self, symbols: &HashMap<String, f64>) -> Result<f64, SolverFailure> {
        let value = self
            .root
            .evaluate(&self.source, symbols, &self.alias_symbols, self.profile)?;
        if !value.is_finite() {
            return Err(SolverFailure::new(
                self.profile.finite_code,
                format!(
                    "{} `{}` produced a non-finite value",
                    self.profile.label, self.source
                ),
            ));
        }
        Ok(value)
    }
}

impl ArithmeticExpressionNode {
    fn evaluate(
        &self,
        source: &str,
        symbols: &HashMap<String, f64>,
        alias_symbols: &HashMap<String, String>,
        profile: ArithmeticExpressionProfile,
    ) -> Result<f64, SolverFailure> {
        match self {
            Self::Number { value, .. } => Ok(*value),
            Self::Symbol { name, .. } => alias_symbols
                .get(name)
                .and_then(|original| symbols.get(original))
                .or_else(|| symbols.get(name))
                .copied()
                .ok_or_else(|| {
                    SolverFailure::new(
                        profile.unknown_code,
                        format!("{} references unknown symbol `{name}`", profile.label),
                    )
                }),
            Self::UnaryMinus { value, .. } => {
                Ok(-value.evaluate(source, symbols, alias_symbols, profile)?)
            }
            Self::Binary {
                operator,
                left,
                right,
                ..
            } => {
                let left = left.evaluate(source, symbols, alias_symbols, profile)?;
                let right = right.evaluate(source, symbols, alias_symbols, profile)?;
                match operator {
                    ArithmeticExpressionBinaryOperator::Add => Ok(left + right),
                    ArithmeticExpressionBinaryOperator::Subtract => Ok(left - right),
                    ArithmeticExpressionBinaryOperator::Multiply => Ok(left * right),
                    ArithmeticExpressionBinaryOperator::Divide => {
                        if right.abs() <= f64::EPSILON {
                            return Err(SolverFailure::new(
                                profile.divide_by_zero_code,
                                format!("{} `{source}` attempted division by zero", profile.label),
                            ));
                        }
                        Ok(left / right)
                    }
                }
            }
        }
    }

    fn collect_unit_literals(&self, units: &mut Vec<ArithmeticUnitMetadata>) {
        match self {
            Self::Number {
                unit: Some(unit), ..
            } => units.push(unit.clone()),
            Self::Number { unit: None, .. } | Self::Symbol { .. } => {}
            Self::UnaryMinus { value, .. } => value.collect_unit_literals(units),
            Self::Binary { left, right, .. } => {
                left.collect_unit_literals(units);
                right.collect_unit_literals(units);
            }
        }
    }

    fn unit_metadata(&self) -> Option<&ArithmeticUnitMetadata> {
        match self {
            Self::Number { unit, .. }
            | Self::Symbol { unit, .. }
            | Self::UnaryMinus { unit, .. }
            | Self::Binary { unit, .. } => unit.as_ref(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ArithmeticExpressionToken {
    Number {
        value: f64,
        unit: Option<ArithmeticUnitMetadata>,
    },
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

pub(crate) fn parse_arithmetic_expression_with_unit_converter<F>(
    expression: &str,
    symbols: &HashMap<String, f64>,
    convert_number: &mut F,
    profile: ArithmeticExpressionProfile,
) -> Result<ParsedArithmeticExpression, SolverFailure>
where
    F: FnMut(f64, Option<&str>) -> Result<f64, SolverFailure>,
{
    parse_arithmetic_expression_with_symbol_metadata_and_unit_converter(
        expression,
        symbols,
        &HashMap::new(),
        convert_number,
        profile,
    )
}

pub(crate) fn parse_arithmetic_expression_with_symbol_metadata_and_unit_converter<F>(
    expression: &str,
    symbols: &HashMap<String, f64>,
    symbol_units: &HashMap<String, ArithmeticUnitMetadata>,
    convert_number: &mut F,
    profile: ArithmeticExpressionProfile,
) -> Result<ParsedArithmeticExpression, SolverFailure>
where
    F: FnMut(f64, Option<&str>) -> Result<f64, SolverFailure>,
{
    let (rewritten, alias_symbols) = rewrite_derivative_symbols(expression, symbols);
    let symbol_units = alias_symbol_unit_metadata(symbol_units, &alias_symbols);
    let tokens = tokenize_arithmetic_expression(&rewritten, convert_number, profile)?;
    let mut parser = ArithmeticExpressionParser {
        tokens,
        position: 0,
        profile,
        symbol_units: &symbol_units,
    };
    let root = parser.parse_expression()?;
    let root_unit = root.unit_metadata().cloned();
    let mut unit_literals = Vec::new();
    root.collect_unit_literals(&mut unit_literals);
    if parser.position != parser.tokens.len() {
        return Err(SolverFailure::new(
            profile.parse_code,
            format!("unsupported {} near `{expression}`", profile.label),
        ));
    }
    Ok(ParsedArithmeticExpression {
        source: expression.to_owned(),
        root,
        alias_symbols,
        profile,
        root_unit,
        unit_literals,
    })
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
    parse_arithmetic_expression_with_unit_converter(expression, symbols, convert_number, profile)?
        .evaluate(symbols)
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

    let parsed = parse_arithmetic_expression_with_unit_converter(
        expression,
        &symbols,
        convert_number,
        profile,
    )?;
    let constant = parsed.evaluate(&symbols)?;
    let mut terms = Vec::new();
    for symbol in &variable_symbols {
        symbols.insert(symbol.clone(), 1.0);
        let value = parsed.evaluate(&symbols)?;
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
        &parsed,
        &variable_symbols,
        &mut symbols,
        constant,
        &terms,
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
fn verify_linearized_expression(
    expression: &str,
    parsed: &ParsedArithmeticExpression,
    variable_symbols: &[String],
    symbols: &mut HashMap<String, f64>,
    constant: f64,
    terms: &[LinearizedArithmeticTerm],
    profile: ArithmeticExpressionProfile,
    tolerance: f64,
) -> Result<(), SolverFailure> {
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
        let evaluated = parsed.evaluate(symbols)?;
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
) -> (String, HashMap<String, String>) {
    let mut rewritten = expression.to_owned();
    let mut alias_symbols = HashMap::new();
    let mut derivative_symbols = symbols
        .keys()
        .filter(|name| name.starts_with("der(") && name.ends_with(')'))
        .collect::<Vec<_>>();
    derivative_symbols.sort_by_key(|name| std::cmp::Reverse(name.len()));
    for (index, name) in derivative_symbols.into_iter().enumerate() {
        let alias = format!("__derivative_{index}");
        rewritten = rewritten.replace(name.as_str(), &alias);
        alias_symbols.insert(alias, name.clone());
    }
    (rewritten, alias_symbols)
}

fn alias_symbol_unit_metadata(
    symbol_units: &HashMap<String, ArithmeticUnitMetadata>,
    alias_symbols: &HashMap<String, String>,
) -> HashMap<String, ArithmeticUnitMetadata> {
    let mut units = symbol_units.clone();
    for (alias, original) in alias_symbols {
        if let Some(unit) = symbol_units.get(original) {
            units.insert(alias.clone(), unit.clone());
        }
    }
    units
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
                let unit_metadata = arithmetic_unit_metadata(unit.as_deref());
                let value = convert_number(value, unit.as_deref())?;
                tokens.push(ArithmeticExpressionToken::Number {
                    value,
                    unit: unit_metadata,
                });
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

fn arithmetic_unit_metadata(unit: Option<&str>) -> Option<ArithmeticUnitMetadata> {
    let display_unit = unit?.trim();
    if display_unit.is_empty() {
        return None;
    }
    Some(arithmetic_metadata_for_unit(display_unit, None))
}

fn arithmetic_metadata_for_unit(
    display_unit: &str,
    quantity_override: Option<&str>,
) -> ArithmeticUnitMetadata {
    let display_unit = if display_unit.trim().is_empty() {
        "1"
    } else {
        display_unit.trim()
    };
    if normalize_unit(display_unit) == "1" {
        return ArithmeticUnitMetadata {
            display_unit: "1".to_owned(),
            canonical_unit: "1".to_owned(),
            quantity_kind: quantity_override
                .unwrap_or("DimensionlessNumber")
                .to_owned(),
        };
    }
    let normalized = normalize_unit(display_unit);
    let info = all_unit_infos()
        .iter()
        .find(|info| normalize_unit(info.symbol) == normalized);
    ArithmeticUnitMetadata {
        display_unit: display_unit.to_owned(),
        canonical_unit: info
            .map(|info| info.canonical_unit.to_owned())
            .unwrap_or_else(|| display_unit.to_owned()),
        quantity_kind: quantity_override
            .map(str::to_owned)
            .or_else(|| info.map(|info| info.quantity_hint.to_owned()))
            .unwrap_or_else(|| "unknown".to_owned()),
    }
}

fn arithmetic_metadata_for_canonical_unit(canonical_unit: String) -> ArithmeticUnitMetadata {
    if normalize_unit(&canonical_unit) == "1" {
        return arithmetic_metadata_for_unit("1", Some("DimensionlessNumber"));
    }
    let normalized = normalize_unit(&canonical_unit);
    if let Some(info) = all_unit_infos()
        .iter()
        .find(|info| normalize_unit(info.canonical_unit) == normalized)
    {
        return ArithmeticUnitMetadata {
            display_unit: info.canonical_unit.to_owned(),
            canonical_unit: info.canonical_unit.to_owned(),
            quantity_kind: info.quantity_hint.to_owned(),
        };
    }
    ArithmeticUnitMetadata {
        display_unit: canonical_unit.clone(),
        canonical_unit,
        quantity_kind: "unknown".to_owned(),
    }
}

fn binary_unit_metadata(
    operator: ArithmeticExpressionBinaryOperator,
    left: Option<&ArithmeticUnitMetadata>,
    right: Option<&ArithmeticUnitMetadata>,
) -> Option<ArithmeticUnitMetadata> {
    match operator {
        ArithmeticExpressionBinaryOperator::Add => add_unit_metadata(left, right),
        ArithmeticExpressionBinaryOperator::Subtract => subtract_unit_metadata(left, right),
        ArithmeticExpressionBinaryOperator::Multiply => multiply_unit_metadata(left, right),
        ArithmeticExpressionBinaryOperator::Divide => divide_unit_metadata(left, right),
    }
}

fn add_unit_metadata(
    left: Option<&ArithmeticUnitMetadata>,
    right: Option<&ArithmeticUnitMetadata>,
) -> Option<ArithmeticUnitMetadata> {
    match (left, right) {
        (Some(left), Some(right))
            if same_canonical_unit(left, right) && compatible_quantity_kind(left, right) =>
        {
            Some(left.clone())
        }
        (Some(left), Some(right))
            if is_absolute_temperature(left) && is_temperature_delta(right) =>
        {
            Some(left.clone())
        }
        (Some(left), Some(right))
            if is_temperature_delta(left) && is_absolute_temperature(right) =>
        {
            Some(right.clone())
        }
        (Some(left), Some(right)) if is_dimensionless(left) => Some(right.clone()),
        (Some(left), Some(right)) if is_dimensionless(right) => Some(left.clone()),
        (Some(left), None) => Some(left.clone()),
        (None, Some(right)) => Some(right.clone()),
        (None, None) => None,
        _ => None,
    }
}

fn subtract_unit_metadata(
    left: Option<&ArithmeticUnitMetadata>,
    right: Option<&ArithmeticUnitMetadata>,
) -> Option<ArithmeticUnitMetadata> {
    match (left, right) {
        (Some(left), Some(right))
            if same_canonical_unit(left, right)
                && is_absolute_temperature(left)
                && is_absolute_temperature(right) =>
        {
            Some(arithmetic_metadata_for_unit(
                &left.canonical_unit,
                Some("TemperatureDelta"),
            ))
        }
        (Some(left), Some(right))
            if same_canonical_unit(left, right) && compatible_quantity_kind(left, right) =>
        {
            Some(left.clone())
        }
        (Some(left), Some(right))
            if is_absolute_temperature(left) && is_temperature_delta(right) =>
        {
            Some(left.clone())
        }
        (Some(left), Some(right)) if is_dimensionless(left) => Some(right.clone()),
        (Some(left), Some(right)) if is_dimensionless(right) => Some(left.clone()),
        (Some(left), None) => Some(left.clone()),
        (None, Some(right)) => Some(right.clone()),
        (None, None) => None,
        _ => None,
    }
}

fn multiply_unit_metadata(
    left: Option<&ArithmeticUnitMetadata>,
    right: Option<&ArithmeticUnitMetadata>,
) -> Option<ArithmeticUnitMetadata> {
    match (left, right) {
        (Some(left), Some(right)) if is_dimensionless(left) => Some(right.clone()),
        (Some(left), Some(right)) if is_dimensionless(right) => Some(left.clone()),
        (Some(left), Some(right)) => Some(arithmetic_metadata_for_canonical_unit(
            multiply_canonical_units(&left.canonical_unit, &right.canonical_unit),
        )),
        (Some(left), None) => Some(left.clone()),
        (None, Some(right)) => Some(right.clone()),
        (None, None) => None,
    }
}

fn divide_unit_metadata(
    left: Option<&ArithmeticUnitMetadata>,
    right: Option<&ArithmeticUnitMetadata>,
) -> Option<ArithmeticUnitMetadata> {
    match (left, right) {
        (Some(left), Some(right)) if is_dimensionless(right) => Some(left.clone()),
        (Some(left), Some(right)) => Some(arithmetic_metadata_for_canonical_unit(
            divide_canonical_units(&left.canonical_unit, &right.canonical_unit),
        )),
        (Some(left), None) => Some(left.clone()),
        (None, Some(right)) => Some(arithmetic_metadata_for_canonical_unit(format!(
            "1/{}",
            right.canonical_unit
        ))),
        (None, None) => None,
    }
}

fn multiply_canonical_units(left: &str, right: &str) -> String {
    let left = normalized_canonical_unit(left);
    let right = normalized_canonical_unit(right);
    if left == "1" {
        return right;
    }
    if right == "1" {
        return left;
    }
    if let Some((numerator, denominator)) = split_unit_fraction(&left) {
        if denominator == right {
            return numerator.to_owned();
        }
    }
    if let Some((numerator, denominator)) = split_unit_fraction(&right) {
        if denominator == left {
            return numerator.to_owned();
        }
    }
    format!("{left}*{right}")
}

fn divide_canonical_units(left: &str, right: &str) -> String {
    let left = normalized_canonical_unit(left);
    let right = normalized_canonical_unit(right);
    if right == "1" {
        return left;
    }
    if left == right {
        return "1".to_owned();
    }
    if left == "1" {
        return format!("1/{right}");
    }
    format!("{left}/{right}")
}

fn split_unit_fraction(unit: &str) -> Option<(&str, &str)> {
    let (numerator, denominator) = unit.split_once('/')?;
    Some((numerator.trim(), denominator.trim()))
}

fn normalized_canonical_unit(unit: &str) -> String {
    let trimmed = unit.trim();
    if trimmed.is_empty() {
        "1".to_owned()
    } else {
        normalize_unit(trimmed)
    }
}

fn same_canonical_unit(left: &ArithmeticUnitMetadata, right: &ArithmeticUnitMetadata) -> bool {
    normalized_canonical_unit(&left.canonical_unit)
        == normalized_canonical_unit(&right.canonical_unit)
}

fn compatible_quantity_kind(left: &ArithmeticUnitMetadata, right: &ArithmeticUnitMetadata) -> bool {
    left.quantity_kind == right.quantity_kind
        || matches!(
            (left.quantity_kind.as_str(), right.quantity_kind.as_str()),
            ("HeatRate", "Power") | ("Power", "HeatRate")
        )
}

fn is_dimensionless(unit: &ArithmeticUnitMetadata) -> bool {
    normalized_canonical_unit(&unit.canonical_unit) == "1"
        || matches!(
            unit.quantity_kind.as_str(),
            "Dimensionless" | "DimensionlessNumber" | "Number"
        )
}

fn is_absolute_temperature(unit: &ArithmeticUnitMetadata) -> bool {
    unit.quantity_kind == "AbsoluteTemperature"
}

fn is_temperature_delta(unit: &ArithmeticUnitMetadata) -> bool {
    unit.quantity_kind == "TemperatureDelta"
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
    profile: ArithmeticExpressionProfile,
    symbol_units: &'a HashMap<String, ArithmeticUnitMetadata>,
}

impl ArithmeticExpressionParser<'_> {
    fn parse_expression(&mut self) -> Result<ArithmeticExpressionNode, SolverFailure> {
        let mut value = self.parse_term()?;
        loop {
            match self.peek() {
                Some(ArithmeticExpressionToken::Plus) => {
                    self.position += 1;
                    let operator = ArithmeticExpressionBinaryOperator::Add;
                    let right = self.parse_term()?;
                    let unit = binary_unit_metadata(
                        operator,
                        value.unit_metadata(),
                        right.unit_metadata(),
                    );
                    value = ArithmeticExpressionNode::Binary {
                        operator,
                        left: Box::new(value),
                        right: Box::new(right),
                        unit,
                    };
                }
                Some(ArithmeticExpressionToken::Minus) => {
                    self.position += 1;
                    let operator = ArithmeticExpressionBinaryOperator::Subtract;
                    let right = self.parse_term()?;
                    let unit = binary_unit_metadata(
                        operator,
                        value.unit_metadata(),
                        right.unit_metadata(),
                    );
                    value = ArithmeticExpressionNode::Binary {
                        operator,
                        left: Box::new(value),
                        right: Box::new(right),
                        unit,
                    };
                }
                _ => return Ok(value),
            }
        }
    }

    fn parse_term(&mut self) -> Result<ArithmeticExpressionNode, SolverFailure> {
        let mut value = self.parse_factor()?;
        loop {
            match self.peek() {
                Some(ArithmeticExpressionToken::Star) => {
                    self.position += 1;
                    let operator = ArithmeticExpressionBinaryOperator::Multiply;
                    let right = self.parse_factor()?;
                    let unit = binary_unit_metadata(
                        operator,
                        value.unit_metadata(),
                        right.unit_metadata(),
                    );
                    value = ArithmeticExpressionNode::Binary {
                        operator,
                        left: Box::new(value),
                        right: Box::new(right),
                        unit,
                    };
                }
                Some(ArithmeticExpressionToken::Slash) => {
                    self.position += 1;
                    let operator = ArithmeticExpressionBinaryOperator::Divide;
                    let right = self.parse_factor()?;
                    let unit = binary_unit_metadata(
                        operator,
                        value.unit_metadata(),
                        right.unit_metadata(),
                    );
                    value = ArithmeticExpressionNode::Binary {
                        operator,
                        left: Box::new(value),
                        right: Box::new(right),
                        unit,
                    };
                }
                _ => return Ok(value),
            }
        }
    }

    fn parse_factor(&mut self) -> Result<ArithmeticExpressionNode, SolverFailure> {
        let Some(token) = self.next().cloned() else {
            return Err(SolverFailure::new(
                self.profile.parse_code,
                format!("{} ended unexpectedly", self.profile.label),
            ));
        };
        match token {
            ArithmeticExpressionToken::Number { value, unit } => {
                Ok(ArithmeticExpressionNode::Number { value, unit })
            }
            ArithmeticExpressionToken::Identifier(name) => Ok(ArithmeticExpressionNode::Symbol {
                unit: self.symbol_units.get(&name).cloned(),
                name,
            }),
            ArithmeticExpressionToken::Minus => {
                let value = self.parse_factor()?;
                let unit = value.unit_metadata().cloned();
                Ok(ArithmeticExpressionNode::UnaryMinus {
                    value: Box::new(value),
                    unit,
                })
            }
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
    fn parsed_expression_preserves_unit_literal_metadata() {
        let symbols = HashMap::from([("x".to_owned(), 2.0)]);
        let mut ignore_units = |value: f64, _unit: Option<&str>| Ok(value);
        let parsed = parse_arithmetic_expression_with_unit_converter(
            "x + 3 kW + 4 W/K",
            &symbols,
            &mut ignore_units,
            ArithmeticExpressionProfile::SOURCE_RESIDUAL,
        )
        .unwrap();

        assert_eq!(parsed.evaluate(&symbols).unwrap(), 9.0);
        assert_eq!(
            parsed.unit_literals,
            vec![
                ArithmeticUnitMetadata {
                    display_unit: "kW".to_owned(),
                    canonical_unit: "W".to_owned(),
                    quantity_kind: "Power".to_owned(),
                },
                ArithmeticUnitMetadata {
                    display_unit: "W/K".to_owned(),
                    canonical_unit: "W/K".to_owned(),
                    quantity_kind: "Conductance".to_owned(),
                },
            ]
        );
    }
    #[test]
    fn parsed_expression_reuses_tree_for_updated_symbols() {
        let mut symbols = HashMap::from([
            ("x".to_owned(), 2.0),
            ("der(x)".to_owned(), 3.0),
            ("gain".to_owned(), 4.0),
        ]);
        let mut ignore_units = |value: f64, _unit: Option<&str>| Ok(value);
        let parsed = parse_arithmetic_expression_with_unit_converter(
            "gain * x + der(x)",
            &symbols,
            &mut ignore_units,
            ArithmeticExpressionProfile::SOURCE_RESIDUAL,
        )
        .unwrap();

        assert_eq!(parsed.evaluate(&symbols).unwrap(), 11.0);

        symbols.insert("x".to_owned(), 5.0);
        symbols.insert("der(x)".to_owned(), -1.0);

        assert_eq!(parsed.evaluate(&symbols).unwrap(), 19.0);
    }
    #[test]
    fn parsed_expression_propagates_symbol_unary_and_binary_unit_metadata() {
        let symbols = HashMap::from([
            ("UA".to_owned(), 2.0),
            ("T_hot".to_owned(), 310.0),
            ("T_cold".to_owned(), 300.0),
            ("der(T_hot)".to_owned(), -0.5),
        ]);
        let symbol_units = HashMap::from([
            (
                "UA".to_owned(),
                ArithmeticUnitMetadata {
                    display_unit: "W/K".to_owned(),
                    canonical_unit: "W/K".to_owned(),
                    quantity_kind: "Conductance".to_owned(),
                },
            ),
            (
                "T_hot".to_owned(),
                ArithmeticUnitMetadata {
                    display_unit: "K".to_owned(),
                    canonical_unit: "K".to_owned(),
                    quantity_kind: "AbsoluteTemperature".to_owned(),
                },
            ),
            (
                "T_cold".to_owned(),
                ArithmeticUnitMetadata {
                    display_unit: "K".to_owned(),
                    canonical_unit: "K".to_owned(),
                    quantity_kind: "AbsoluteTemperature".to_owned(),
                },
            ),
            (
                "der(T_hot)".to_owned(),
                ArithmeticUnitMetadata {
                    display_unit: "K/s".to_owned(),
                    canonical_unit: "K/s".to_owned(),
                    quantity_kind: "Derivative[AbsoluteTemperature]".to_owned(),
                },
            ),
        ]);
        let mut ignore_units = |value: f64, _unit: Option<&str>| Ok(value);

        let parsed = parse_arithmetic_expression_with_symbol_metadata_and_unit_converter(
            "UA * (T_hot - T_cold)",
            &symbols,
            &symbol_units,
            &mut ignore_units,
            ArithmeticExpressionProfile::SOURCE_RESIDUAL,
        )
        .unwrap();

        assert_eq!(parsed.evaluate(&symbols).unwrap(), 20.0);
        assert_eq!(
            parsed.root_unit.as_ref(),
            Some(&ArithmeticUnitMetadata {
                display_unit: "W".to_owned(),
                canonical_unit: "W".to_owned(),
                quantity_kind: "Power".to_owned(),
            })
        );
        let ArithmeticExpressionNode::Binary {
            operator: ArithmeticExpressionBinaryOperator::Multiply,
            left,
            right,
            unit: Some(root_unit),
        } = &parsed.root
        else {
            panic!("expected multiply root");
        };
        assert_eq!(root_unit.quantity_kind, "Power");
        let ArithmeticExpressionNode::Symbol {
            unit: Some(left_unit),
            ..
        } = left.as_ref()
        else {
            panic!("expected left symbol");
        };
        assert_eq!(left_unit.quantity_kind, "Conductance");
        let ArithmeticExpressionNode::Binary {
            operator: ArithmeticExpressionBinaryOperator::Subtract,
            unit: Some(right_unit),
            ..
        } = right.as_ref()
        else {
            panic!("expected temperature subtraction");
        };
        assert_eq!(right_unit.quantity_kind, "TemperatureDelta");

        let parsed_derivative =
            parse_arithmetic_expression_with_symbol_metadata_and_unit_converter(
                "-der(T_hot)",
                &symbols,
                &symbol_units,
                &mut ignore_units,
                ArithmeticExpressionProfile::SOURCE_RESIDUAL,
            )
            .unwrap();
        assert_eq!(
            parsed_derivative.root_unit.as_ref(),
            Some(&ArithmeticUnitMetadata {
                display_unit: "K/s".to_owned(),
                canonical_unit: "K/s".to_owned(),
                quantity_kind: "Derivative[AbsoluteTemperature]".to_owned(),
            })
        );
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
