use crate::ast::FastBinding;
use crate::quantities::{
    candidates_for_unit, first_unit_in_expression, infer_quantity_from_name_and_unit,
};
use crate::semantic::TypedBinding;
use crate::Diagnostic;

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
    pub scale: Option<String>,
    pub offset: Option<String>,
    pub mean: Option<String>,
    pub stddev: Option<String>,
    pub error: Option<String>,
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
    arithmetic_info(binding, typed_bindings)
}

pub fn source_diagnostic(
    binding: &FastBinding,
    typed_bindings: &[TypedBinding],
) -> Option<Diagnostic> {
    let call = source_required_call(&binding.expression)?;
    let source = first_argument(&binding.expression);
    let Some(source) = source.filter(|source| is_identifier(source)) else {
        return Some(Diagnostic::error(
            "E-UNC-SOURCE-001",
            binding.line,
            &format!("`{call}` requires a prior uncertainty binding as its first argument."),
            Some("Define a measured, interval, distribution, or ensemble binding first, then reference that name."),
        ));
    };

    let Some(source_binding) = typed_bindings
        .iter()
        .find(|typed_binding| typed_binding.name == source)
    else {
        return Some(Diagnostic::error(
            "E-UNC-SOURCE-001",
            binding.line,
            &format!("Unknown uncertainty source `{source}` for `{call}`."),
            Some("Check the source name or move the source uncertainty binding before this expression."),
        ));
    };

    if uncertainty_inner_quantity(&source_binding.semantic_type.quantity_kind).is_none() {
        return Some(Diagnostic::error(
            "E-UNC-SOURCE-002",
            binding.line,
            &format!(
                "`{source}` is {}, not an uncertainty source.",
                source_binding.semantic_type.quantity_kind
            ),
            Some("Use measured(...), interval(...), normal(...), uniform(...), or ensemble(...) to create the source uncertainty."),
        ));
    }

    None
}

pub fn argument_diagnostics(binding: &FastBinding) -> Vec<Diagnostic> {
    let expression = binding.expression.trim();
    let Some(call) = uncertainty_call_name(expression) else {
        return Vec::new();
    };

    let mut diagnostics = Vec::new();
    validate_sample_count_argument(call, expression, binding.line, &mut diagnostics);

    match call {
        "measured" => validate_measured_arguments(expression, binding.line, &mut diagnostics),
        "interval" => validate_range_arguments(call, expression, binding.line, &mut diagnostics),
        "normal" => validate_normal_arguments(call, expression, binding.line, &mut diagnostics),
        "uniform" => validate_range_arguments(call, expression, binding.line, &mut diagnostics),
        "distribution" => {
            validate_distribution_arguments(expression, binding.line, &mut diagnostics)
        }
        "propagate" => validate_propagation_arguments(expression, binding.line, &mut diagnostics),
        "ensemble" => {}
        _ => {}
    }

    diagnostics
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

fn validate_measured_arguments(expression: &str, line: usize, diagnostics: &mut Vec<Diagnostic>) {
    if first_value_with_unit(expression)
        .and_then(|value| numeric_prefix(&value))
        .is_none()
    {
        diagnostics.push(Diagnostic::error(
            "E-UNC-ARGS-001",
            line,
            "`measured` requires a numeric measured value.",
            Some("Use `measured(12 degC, std=0.2 K)` or another numeric value with a unit."),
        ));
    }

    validate_optional_non_negative_value(
        expression,
        &["std", "sigma", "uncertainty"],
        "`measured` standard deviation",
        line,
        diagnostics,
    );
    validate_optional_non_negative_value(
        expression,
        &["error", "relative_error"],
        "`measured` relative error",
        line,
        diagnostics,
    );
}

fn validate_distribution_arguments(
    expression: &str,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match distribution_kind(expression).as_str() {
        "normal" => validate_normal_arguments("distribution", expression, line, diagnostics),
        "uniform" => validate_range_arguments("distribution", expression, line, diagnostics),
        unsupported => diagnostics.push(Diagnostic::error(
            "E-UNC-ARGS-003",
            line,
            &format!("Unsupported uncertainty distribution `{unsupported}`."),
            Some("The current uncertainty track supports `normal` and `uniform` distributions."),
        )),
    }
}

fn validate_normal_arguments(
    call: &str,
    expression: &str,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if distribution_mean(expression)
        .and_then(|value| numeric_prefix(&value))
        .is_none()
    {
        diagnostics.push(Diagnostic::error(
            "E-UNC-ARGS-001",
            line,
            &format!("`{call}` requires a numeric `mean` value."),
            Some("Use `normal(mean=5 kW, std=0.8 kW, samples=31)`."),
        ));
    }

    validate_required_non_negative_value(
        expression,
        &["std", "sigma"],
        &format!("`{call}` standard deviation"),
        line,
        diagnostics,
    );
}

fn validate_range_arguments(
    call: &str,
    expression: &str,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (lower, upper) = range_values(expression);
    let lower_numeric = lower.as_deref().and_then(numeric_prefix);
    let upper_numeric = upper.as_deref().and_then(numeric_prefix);

    let (Some(lower_numeric), Some(upper_numeric)) = (lower_numeric, upper_numeric) else {
        diagnostics.push(Diagnostic::error(
            "E-UNC-ARGS-001",
            line,
            &format!("`{call}` requires two numeric bounds."),
            Some("Use positional bounds such as `uniform(0.3 kW, 0.7 kW)` or named `lower=`/`upper=` bounds."),
        ));
        return;
    };

    if lower_numeric > upper_numeric {
        diagnostics.push(Diagnostic::error(
            "E-UNC-ARGS-002",
            line,
            &format!(
                "`{call}` lower bound {lower_numeric} is greater than upper bound {upper_numeric}."
            ),
            Some("Swap the bounds or correct the declared interval/range."),
        ));
    }
}

fn validate_propagation_arguments(
    expression: &str,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(method) = named_value(expression, &["method"]) {
        let normalized = method.trim().trim_matches('"').to_ascii_lowercase();
        if normalized != "linear" {
            diagnostics.push(Diagnostic::error(
                "E-UNC-ARGS-003",
                line,
                &format!("Unsupported uncertainty propagation method `{method}`."),
                Some("The current uncertainty track supports `method=linear`."),
            ));
        }
    }

    validate_optional_numeric_value(
        expression,
        &["scale", "gain"],
        "`propagate` scale/gain",
        line,
        diagnostics,
    );
    validate_optional_numeric_value(
        expression,
        &["offset", "bias"],
        "`propagate` offset/bias",
        line,
        diagnostics,
    );
}

fn validate_sample_count_argument(
    call: &str,
    expression: &str,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(value) = named_value(expression, &["samples", "n"]) {
        match value.trim().parse::<usize>() {
            Ok(count) if (1..=256).contains(&count) => {}
            _ => diagnostics.push(Diagnostic::error(
                "E-UNC-ARGS-002",
                line,
                &format!("`{call}` sample count `{value}` is invalid."),
                Some("Use an integer `samples` value between 1 and 256."),
            )),
        }
    }
}

fn validate_required_non_negative_value(
    expression: &str,
    names: &[&str],
    label: &str,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match named_value(expression, names) {
        Some(value) => validate_non_negative_value(&value, label, line, diagnostics),
        None => diagnostics.push(Diagnostic::error(
            "E-UNC-ARGS-001",
            line,
            &format!("{label} is required."),
            Some("Provide a non-negative value such as `std=0.8 kW`."),
        )),
    }
}

fn validate_optional_non_negative_value(
    expression: &str,
    names: &[&str],
    label: &str,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(value) = named_value(expression, names) {
        validate_non_negative_value(&value, label, line, diagnostics);
    }
}

fn validate_non_negative_value(
    value: &str,
    label: &str,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match numeric_prefix(value) {
        Some(parsed) if parsed >= 0.0 => {}
        Some(parsed) => diagnostics.push(Diagnostic::error(
            "E-UNC-ARGS-002",
            line,
            &format!("{label} must be non-negative, but found {parsed}."),
            Some("Use a zero or positive standard deviation."),
        )),
        None => diagnostics.push(Diagnostic::error(
            "E-UNC-ARGS-001",
            line,
            &format!("{label} must be numeric."),
            Some("Provide a numeric value with an optional unit."),
        )),
    }
}

fn validate_optional_numeric_value(
    expression: &str,
    names: &[&str],
    label: &str,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(value) = named_value(expression, names) {
        if numeric_prefix(&value).is_none() {
            diagnostics.push(Diagnostic::error(
                "E-UNC-ARGS-002",
                line,
                &format!("{label} value `{value}` must be numeric."),
                Some("Use a numeric value such as `scale=1.08` or `offset=0.4 kW`."),
            ));
        }
    }
}

fn distribution_mean(expression: &str) -> Option<String> {
    named_value(expression, &["mean", "mu"]).or_else(|| first_value_with_unit(expression))
}

fn range_values(expression: &str) -> (Option<String>, Option<String>) {
    let values = values_with_unit(expression);
    let lower = named_value(expression, &["lower", "min"]).or_else(|| values.first().cloned());
    let upper = named_value(expression, &["upper", "max"]).or_else(|| values.get(1).cloned());
    (lower, upper)
}

fn uncertainty_call_name(expression: &str) -> Option<&'static str> {
    let lowered = expression.trim_start().to_ascii_lowercase();
    if lowered.starts_with("measured(") {
        Some("measured")
    } else if lowered.starts_with("interval(") {
        Some("interval")
    } else if lowered.starts_with("normal(") {
        Some("normal")
    } else if lowered.starts_with("uniform(") {
        Some("uniform")
    } else if lowered.starts_with("distribution(") {
        Some("distribution")
    } else if lowered.starts_with("ensemble(") {
        Some("ensemble")
    } else if lowered.starts_with("propagate(") {
        Some("propagate")
    } else {
        None
    }
}

fn numeric_prefix(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    let mut end = 0;
    let mut saw_digit = false;
    let mut previous = '\0';
    for (index, character) in trimmed.char_indices() {
        let allowed = character.is_ascii_digit()
            || character == '.'
            || ((character == '-' || character == '+')
                && (index == 0 || previous == 'e' || previous == 'E'))
            || ((character == 'e' || character == 'E') && saw_digit);
        if !allowed {
            break;
        }
        if character.is_ascii_digit() {
            saw_digit = true;
        }
        end = index + character.len_utf8();
        previous = character;
    }
    if !saw_digit {
        return None;
    }
    trimmed[..end].parse::<f64>().ok()
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
        scale: None,
        offset: None,
        mean: first_value_with_unit(&binding.expression),
        stddev: named_value(&binding.expression, &["std", "sigma", "uncertainty"]),
        error: named_value(&binding.expression, &["error", "relative_error"]),
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
        scale: None,
        offset: None,
        mean: None,
        stddev: None,
        error: None,
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
        scale: None,
        offset: None,
        mean: named_value(&binding.expression, &["mean", "mu"])
            .or_else(|| first_value_with_unit(&binding.expression)),
        stddev: named_value(&binding.expression, &["std", "sigma"]),
        error: None,
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
        scale: None,
        offset: None,
        mean: None,
        stddev: None,
        error: None,
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
        scale: named_value(&binding.expression, &["scale", "gain"]),
        offset: named_value(&binding.expression, &["offset", "bias"]),
        mean: None,
        stddev: None,
        error: None,
        lower: None,
        upper: None,
        sample_count: sample_count(&binding.expression).unwrap_or(64),
        propagation: propagation_terms(&binding.expression, typed_bindings),
        line: binding.line,
    }
}

fn arithmetic_info(
    binding: &FastBinding,
    typed_bindings: &[TypedBinding],
) -> Option<UncertaintyInfo> {
    if !arithmetic_expression_candidate(&binding.expression) {
        return None;
    }
    let sources = uncertain_sources_in_expression(&binding.expression, typed_bindings);
    if sources.is_empty() {
        return None;
    }
    let kind = arithmetic_result_kind(&sources);
    let (_, quantity_kind) = uncertainty_inner_quantity(&sources[0].semantic_type.quantity_kind)
        .unwrap_or_else(|| {
            (
                "Measured".to_owned(),
                sources[0].semantic_type.quantity_kind.clone(),
            )
        });
    let method = if kind == "Interval" {
        "interval"
    } else {
        "linear"
    };
    Some(UncertaintyInfo {
        binding: binding.name.clone(),
        kind,
        quantity_kind,
        display_unit: sources[0].semantic_type.display_unit.clone(),
        expression: binding.expression.clone(),
        source: Some(sources[0].name.clone()),
        distribution: Some("arithmetic".to_owned()),
        method: Some(method.to_owned()),
        scale: None,
        offset: None,
        mean: None,
        stddev: None,
        error: None,
        lower: None,
        upper: None,
        sample_count: sample_count(&binding.expression).unwrap_or(64),
        propagation: sources
            .iter()
            .map(|source| UncertaintyPropagationTerm {
                source: source.name.clone(),
                role: "arithmetic_source".to_owned(),
                quantity_kind: uncertainty_inner_quantity(&source.semantic_type.quantity_kind)
                    .map(|(_, inner)| inner)
                    .unwrap_or_else(|| source.semantic_type.quantity_kind.clone()),
            })
            .collect(),
        line: binding.line,
    })
}

fn arithmetic_expression_candidate(expression: &str) -> bool {
    let trimmed = expression.trim();
    if trimmed.is_empty() || uncertainty_call_name(trimmed).is_some() {
        return false;
    }
    trimmed
        .chars()
        .any(|character| matches!(character, '+' | '-' | '*' | '/'))
}

fn uncertain_sources_in_expression<'a>(
    expression: &str,
    typed_bindings: &'a [TypedBinding],
) -> Vec<&'a TypedBinding> {
    let mut sources = Vec::new();
    for binding in typed_bindings {
        if uncertainty_inner_quantity(&binding.semantic_type.quantity_kind).is_some()
            && expression_mentions_identifier(expression, &binding.name)
            && !sources
                .iter()
                .any(|source: &&TypedBinding| source.name == binding.name)
        {
            sources.push(binding);
        }
    }
    sources
}

fn arithmetic_result_kind(sources: &[&TypedBinding]) -> String {
    if sources.iter().any(|source| {
        uncertainty_inner_quantity(&source.semantic_type.quantity_kind)
            .is_some_and(|(kind, _)| kind == "Ensemble")
    }) {
        return "Ensemble".to_owned();
    }
    if sources.iter().any(|source| {
        uncertainty_inner_quantity(&source.semantic_type.quantity_kind)
            .is_some_and(|(kind, _)| kind == "Distribution")
    }) {
        return "Distribution".to_owned();
    }
    if sources.iter().all(|source| {
        uncertainty_inner_quantity(&source.semantic_type.quantity_kind)
            .is_some_and(|(kind, _)| kind == "Interval")
    }) {
        return "Interval".to_owned();
    }
    "Measured".to_owned()
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

fn source_required_call(expression: &str) -> Option<&'static str> {
    let lowered = expression.trim_start().to_ascii_lowercase();
    if lowered.starts_with("ensemble(") {
        Some("ensemble")
    } else if lowered.starts_with("propagate(") {
        Some("propagate")
    } else {
        None
    }
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
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
