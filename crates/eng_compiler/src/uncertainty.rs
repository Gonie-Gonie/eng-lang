use crate::ast::FastBinding;
use crate::quantities::{
    candidates_for_unit, first_unit_in_expression, infer_quantity_from_name_and_unit,
};
use crate::semantic::TypedBinding;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UncertaintyInfo {
    pub binding: String,
    pub kind: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub expression: String,
    pub source: Option<String>,
    pub distribution: Option<String>,
    pub method: Option<String>,
    pub mean: Option<String>,
    pub stddev: Option<String>,
    pub lower: Option<String>,
    pub upper: Option<String>,
    pub sample_count: usize,
    pub propagation: Vec<UncertaintyPropagationTerm>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UncertaintyPropagationTerm {
    pub source: String,
    pub role: String,
    pub quantity_kind: String,
}

pub fn uncertainty_info(
    binding: &FastBinding,
    typed_bindings: &[TypedBinding],
) -> Option<UncertaintyInfo> {
    let expression = binding.expression.trim();
    let lowered = expression.to_ascii_lowercase();
    if lowered.starts_with("measured(") {
        return Some(measured_info(binding, typed_bindings));
    }
    if lowered.starts_with("interval(") {
        return Some(interval_info(binding, typed_bindings));
    }
    if lowered.starts_with("normal(")
        || lowered.starts_with("uniform(")
        || lowered.starts_with("distribution(")
    {
        return Some(distribution_info(binding, typed_bindings));
    }
    if lowered.starts_with("ensemble(") {
        return Some(ensemble_info(binding, typed_bindings));
    }
    if lowered.starts_with("propagate(") {
        return Some(propagation_info(binding, typed_bindings));
    }
    None
}

pub fn uncertainty_semantic_type(name: &str, expression: &str) -> Option<(String, String)> {
    let lowered = expression.trim().to_ascii_lowercase();
    let kind = if lowered.starts_with("measured(") {
        "Measured"
    } else if lowered.starts_with("interval(") {
        "Interval"
    } else if lowered.starts_with("normal(")
        || lowered.starts_with("uniform(")
        || lowered.starts_with("distribution(")
    {
        "Distribution"
    } else if lowered.starts_with("ensemble(") {
        "Ensemble"
    } else if lowered.starts_with("propagate(") {
        "Distribution"
    } else {
        return None;
    };
    let unit = first_unit_in_expression(expression).unwrap_or_else(|| default_unit_hint(name));
    let quantity_kind = infer_quantity(name, expression, &unit);
    Some((format!("{kind}[{quantity_kind}]"), unit))
}

pub fn uncertainty_inner_quantity(quantity_kind: &str) -> Option<(String, String)> {
    for kind in ["Measured", "Interval", "Distribution", "Ensemble"] {
        let prefix = format!("{kind}[");
        let inner = quantity_kind
            .strip_prefix(&prefix)
            .and_then(|rest| rest.strip_suffix(']'));
        if let Some(inner) = inner {
            return Some((kind.to_owned(), inner.to_owned()));
        }
    }
    None
}

fn measured_info(binding: &FastBinding, typed_bindings: &[TypedBinding]) -> UncertaintyInfo {
    let display_unit =
        first_unit_in_expression(&binding.expression).unwrap_or_else(|| "1".to_owned());
    let quantity_kind = infer_quantity(&binding.name, &binding.expression, &display_unit);
    UncertaintyInfo {
        binding: binding.name.clone(),
        kind: "Measured".to_owned(),
        quantity_kind,
        display_unit,
        expression: binding.expression.clone(),
        source: first_argument(&binding.expression),
        distribution: Some("measured".to_owned()),
        method: None,
        mean: first_value_with_unit(&binding.expression),
        stddev: named_value(&binding.expression, &["std", "sigma", "uncertainty"]),
        lower: None,
        upper: None,
        sample_count: 1,
        propagation: propagation_terms(&binding.expression, typed_bindings),
        line: binding.line,
    }
}

fn interval_info(binding: &FastBinding, typed_bindings: &[TypedBinding]) -> UncertaintyInfo {
    let display_unit =
        first_unit_in_expression(&binding.expression).unwrap_or_else(|| "1".to_owned());
    let quantity_kind = infer_quantity(&binding.name, &binding.expression, &display_unit);
    let values = values_with_unit(&binding.expression);
    let lower =
        named_value(&binding.expression, &["lower", "min"]).or_else(|| values.first().cloned());
    let upper =
        named_value(&binding.expression, &["upper", "max"]).or_else(|| values.get(1).cloned());
    UncertaintyInfo {
        binding: binding.name.clone(),
        kind: "Interval".to_owned(),
        quantity_kind,
        display_unit,
        expression: binding.expression.clone(),
        source: first_argument(&binding.expression),
        distribution: Some("interval".to_owned()),
        method: None,
        mean: None,
        stddev: None,
        lower,
        upper,
        sample_count: 2,
        propagation: propagation_terms(&binding.expression, typed_bindings),
        line: binding.line,
    }
}

fn distribution_info(binding: &FastBinding, typed_bindings: &[TypedBinding]) -> UncertaintyInfo {
    let display_unit =
        first_unit_in_expression(&binding.expression).unwrap_or_else(|| "1".to_owned());
    let quantity_kind = infer_quantity(&binding.name, &binding.expression, &display_unit);
    let distribution = distribution_kind(&binding.expression);
    let values = values_with_unit(&binding.expression);
    let lower =
        named_value(&binding.expression, &["lower", "min"]).or_else(|| values.first().cloned());
    let upper =
        named_value(&binding.expression, &["upper", "max"]).or_else(|| values.get(1).cloned());
    UncertaintyInfo {
        binding: binding.name.clone(),
        kind: "Distribution".to_owned(),
        quantity_kind,
        display_unit,
        expression: binding.expression.clone(),
        source: first_argument(&binding.expression),
        distribution: Some(distribution),
        method: None,
        mean: named_value(&binding.expression, &["mean", "mu"])
            .or_else(|| first_value_with_unit(&binding.expression)),
        stddev: named_value(&binding.expression, &["std", "sigma"]),
        lower,
        upper,
        sample_count: sample_count(&binding.expression).unwrap_or(64),
        propagation: propagation_terms(&binding.expression, typed_bindings),
        line: binding.line,
    }
}

fn ensemble_info(binding: &FastBinding, typed_bindings: &[TypedBinding]) -> UncertaintyInfo {
    let source = first_argument(&binding.expression);
    let source_quantity = source
        .as_deref()
        .and_then(|source| typed_bindings.iter().find(|binding| binding.name == source))
        .map(|binding| binding.semantic_type.quantity_kind.clone());
    let quantity_kind = source_quantity
        .and_then(|quantity| {
            uncertainty_inner_quantity(&quantity)
                .map(|(_, inner)| inner)
                .or(Some(quantity))
        })
        .unwrap_or_else(|| infer_quantity(&binding.name, &binding.expression, "1"));
    UncertaintyInfo {
        binding: binding.name.clone(),
        kind: "Ensemble".to_owned(),
        quantity_kind,
        display_unit: display_unit_for_binding(binding, typed_bindings),
        expression: binding.expression.clone(),
        source,
        distribution: Some("ensemble".to_owned()),
        method: Some(
            named_value(&binding.expression, &["method"])
                .unwrap_or_else(|| "deterministic_resample".to_owned()),
        ),
        mean: None,
        stddev: None,
        lower: None,
        upper: None,
        sample_count: sample_count(&binding.expression).unwrap_or(32),
        propagation: propagation_terms(&binding.expression, typed_bindings),
        line: binding.line,
    }
}

fn propagation_info(binding: &FastBinding, typed_bindings: &[TypedBinding]) -> UncertaintyInfo {
    let source = first_argument(&binding.expression);
    let source_quantity = source
        .as_deref()
        .and_then(|source| typed_bindings.iter().find(|binding| binding.name == source))
        .map(|binding| binding.semantic_type.quantity_kind.clone());
    let quantity_kind = source_quantity
        .and_then(|quantity| {
            uncertainty_inner_quantity(&quantity)
                .map(|(_, inner)| inner)
                .or(Some(quantity))
        })
        .unwrap_or_else(|| {
            let unit =
                first_unit_in_expression(&binding.expression).unwrap_or_else(|| "1".to_owned());
            infer_quantity(&binding.name, &binding.expression, &unit)
        });
    UncertaintyInfo {
        binding: binding.name.clone(),
        kind: "Distribution".to_owned(),
        quantity_kind,
        display_unit: display_unit_for_binding(binding, typed_bindings),
        expression: binding.expression.clone(),
        source,
        distribution: Some("propagated".to_owned()),
        method: Some(
            named_value(&binding.expression, &["method"]).unwrap_or_else(|| "linear".to_owned()),
        ),
        mean: None,
        stddev: None,
        lower: None,
        upper: None,
        sample_count: sample_count(&binding.expression).unwrap_or(64),
        propagation: propagation_terms(&binding.expression, typed_bindings),
        line: binding.line,
    }
}

fn infer_quantity(name: &str, expression: &str, unit: &str) -> String {
    if let Some(quantity) = infer_quantity_from_name_and_unit(name, unit) {
        return quantity.quantity_kind.to_owned();
    }
    let candidates = candidates_for_unit(unit);
    if candidates.len() == 1 {
        return candidates[0].quantity_kind.to_owned();
    }
    if let Some(quantity) = name_quantity_hint(name) {
        return quantity.to_owned();
    }
    if expression.to_ascii_lowercase().contains("q_") || name.to_ascii_lowercase().starts_with('q')
    {
        return "HeatRate".to_owned();
    }
    "DimensionlessNumber".to_owned()
}

fn display_unit_for_binding(binding: &FastBinding, typed_bindings: &[TypedBinding]) -> String {
    first_unit_in_expression(&binding.expression)
        .or_else(|| {
            first_argument(&binding.expression).and_then(|source| {
                typed_bindings
                    .iter()
                    .find(|binding| binding.name == source)
                    .map(|binding| binding.semantic_type.display_unit.clone())
            })
        })
        .unwrap_or_else(|| default_unit_hint(&binding.name))
}

fn default_unit_hint(name: &str) -> String {
    match name_quantity_hint(name) {
        Some("AbsoluteTemperature") => "degC".to_owned(),
        Some("Energy") => "J".to_owned(),
        Some("HeatRate") => "W".to_owned(),
        Some("MassFlowRate") => "kg/s".to_owned(),
        _ => "1".to_owned(),
    }
}

fn name_quantity_hint(name: &str) -> Option<&'static str> {
    let lowered = name.to_ascii_lowercase();
    if lowered.contains("temp") || lowered.starts_with('t') {
        Some("AbsoluteTemperature")
    } else if lowered.contains("energy") || lowered.starts_with('e') {
        Some("Energy")
    } else if lowered.contains("heat") || lowered.starts_with('q') {
        Some("HeatRate")
    } else if lowered.contains("mass") || lowered.contains("m_dot") {
        Some("MassFlowRate")
    } else {
        None
    }
}

fn first_argument(expression: &str) -> Option<String> {
    let inside = call_inside(expression)?;
    inside
        .split(',')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.contains('='))
        .map(str::to_owned)
}

fn named_value(expression: &str, names: &[&str]) -> Option<String> {
    let inside = call_inside(expression)?;
    for part in inside.split(',').map(str::trim) {
        let Some((name, value)) = part.split_once('=') else {
            continue;
        };
        if names.iter().any(|candidate| name.trim() == *candidate) {
            return Some(value.trim().to_owned());
        }
    }
    None
}

fn distribution_kind(expression: &str) -> String {
    let lowered = expression.trim().to_ascii_lowercase();
    if lowered.starts_with("normal(") {
        return "normal".to_owned();
    }
    if lowered.starts_with("uniform(") {
        return "uniform".to_owned();
    }
    named_value(expression, &["kind", "distribution"])
        .map(|value| value.trim_matches('"').to_ascii_lowercase())
        .unwrap_or_else(|| "normal".to_owned())
}

fn sample_count(expression: &str) -> Option<usize> {
    named_value(expression, &["samples", "n"]).and_then(|value| value.parse::<usize>().ok())
}

fn first_value_with_unit(expression: &str) -> Option<String> {
    values_with_unit(expression).into_iter().next()
}

fn values_with_unit(expression: &str) -> Vec<String> {
    let Some(inside) = call_inside(expression) else {
        return Vec::new();
    };
    inside
        .split(',')
        .map(str::trim)
        .filter(|part| !part.contains('='))
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

fn propagation_terms(
    expression: &str,
    typed_bindings: &[TypedBinding],
) -> Vec<UncertaintyPropagationTerm> {
    typed_bindings
        .iter()
        .filter(|binding| expression_mentions_identifier(expression, &binding.name))
        .map(|binding| UncertaintyPropagationTerm {
            source: binding.name.clone(),
            role: uncertainty_inner_quantity(&binding.semantic_type.quantity_kind)
                .map(|(kind, _)| kind)
                .unwrap_or_else(|| "deterministic".to_owned()),
            quantity_kind: uncertainty_inner_quantity(&binding.semantic_type.quantity_kind)
                .map(|(_, inner)| inner)
                .unwrap_or_else(|| binding.semantic_type.quantity_kind.clone()),
        })
        .collect()
}

fn call_inside(expression: &str) -> Option<&str> {
    let open = expression.find('(')?;
    let close = expression.rfind(')')?;
    (close > open).then(|| &expression[open + 1..close])
}

fn expression_mentions_identifier(expression: &str, identifier: &str) -> bool {
    expression
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .any(|token| token == identifier)
}
