use crate::ast::FastBinding;
use crate::quantities::{
    candidates_for_unit, first_unit_in_expression, infer_quantity_from_name_and_unit,
};
use crate::semantic::TypedBinding;
use crate::source::SourceSpan;
use crate::Diagnostic;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UncertaintyValueInfo {
    pub value: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UncertaintyNamedArgumentInfo {
    pub name: String,
    pub value: String,
    pub key_span: SourceSpan,
    pub value_span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UncertaintyInfo {
    pub binding: String,
    pub binding_span: SourceSpan,
    pub kind: String,
    pub positional_arguments: Vec<UncertaintyValueInfo>,
    pub named_arguments: Vec<UncertaintyNamedArgumentInfo>,
    pub quantity_kind: String,
    pub display_unit: String,
    pub expression: String,
    pub expression_span: SourceSpan,
    pub source: Option<String>,
    pub source_span: Option<SourceSpan>,
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct UncertaintyArguments {
    positional: Vec<UncertaintyValueInfo>,
    named: Vec<UncertaintyNamedArgumentInfo>,
}

pub fn uncertainty_info(
    binding: &FastBinding,
    typed_bindings: &[TypedBinding],
) -> Option<UncertaintyInfo> {
    let expression = binding.expression.trim();
    let lowered = expression.to_ascii_lowercase();
    let arguments = uncertainty_arguments(binding);
    if lowered.starts_with("measured(") {
        return Some(measured_info(binding, typed_bindings, &arguments));
    }
    if lowered.starts_with("interval(") {
        return Some(interval_info(binding, typed_bindings, &arguments));
    }
    if lowered.starts_with("normal(")
        || lowered.starts_with("uniform(")
        || lowered.starts_with("distribution(")
    {
        return Some(distribution_info(binding, typed_bindings, &arguments));
    }
    if lowered.starts_with("ensemble(") {
        return Some(ensemble_info(binding, typed_bindings, &arguments));
    }
    if lowered.starts_with("propagate(") {
        return Some(propagation_info(binding, typed_bindings, &arguments));
    }
    arithmetic_info(binding, typed_bindings, &arguments)
}

pub fn source_diagnostic(
    binding: &FastBinding,
    typed_bindings: &[TypedBinding],
) -> Option<Diagnostic> {
    let call = source_required_call(&binding.expression)?;
    let arguments = uncertainty_arguments(binding);
    let source_argument = arguments.positional.first();
    let source = source_argument.map(|argument| argument.value.clone());
    let source_span = source_argument.map(|argument| argument.span);
    let Some(source) = source.filter(|source| is_identifier(source)) else {
        return Some(uncertainty_diagnostic(
            "E-UNC-SOURCE-001",
            source_span,
            binding.expression_span,
            &format!("`{call}` requires a prior uncertainty binding as its first argument."),
            Some("Define a measured, interval, distribution, or ensemble binding first, then reference that name."),
        ));
    };

    let Some(source_binding) = typed_bindings
        .iter()
        .find(|typed_binding| typed_binding.name == source)
    else {
        return Some(uncertainty_diagnostic(
            "E-UNC-SOURCE-001",
            source_span,
            binding.expression_span,
            &format!("Unknown uncertainty source `{source}` for `{call}`."),
            Some("Check the source name or move the source uncertainty binding before this expression."),
        ));
    };

    if uncertainty_inner_quantity(&source_binding.semantic_type.quantity_kind).is_none() {
        return Some(uncertainty_diagnostic(
            "E-UNC-SOURCE-002",
            source_span,
            binding.expression_span,
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

    let arguments = uncertainty_arguments(binding);
    let mut diagnostics = Vec::new();
    validate_sample_count_argument(call, &arguments, binding.expression_span, &mut diagnostics);

    match call {
        "measured" => {
            validate_measured_arguments(&arguments, binding.expression_span, &mut diagnostics)
        }
        "interval" | "uniform" => {
            validate_range_arguments(call, &arguments, binding.expression_span, &mut diagnostics)
        }
        "normal" => {
            validate_normal_arguments(call, &arguments, binding.expression_span, &mut diagnostics)
        }
        "distribution" => validate_distribution_arguments(
            expression,
            &arguments,
            binding.expression_span,
            &mut diagnostics,
        ),
        "propagate" => {
            validate_propagation_arguments(&arguments, binding.expression_span, &mut diagnostics)
        }
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

#[derive(Clone, Copy, Debug)]
struct UncertaintyArgumentValue<'a> {
    value: &'a str,
    span: SourceSpan,
}

fn positional_argument_value(
    arguments: &UncertaintyArguments,
    index: usize,
) -> Option<UncertaintyArgumentValue<'_>> {
    arguments
        .positional
        .get(index)
        .map(|argument| UncertaintyArgumentValue {
            value: &argument.value,
            span: argument.span,
        })
}

fn named_argument_value<'a>(
    arguments: &'a UncertaintyArguments,
    names: &[&str],
) -> Option<UncertaintyArgumentValue<'a>> {
    arguments
        .named
        .iter()
        .find(|argument| {
            names
                .iter()
                .any(|name| argument.name.eq_ignore_ascii_case(name))
        })
        .map(|argument| UncertaintyArgumentValue {
            value: &argument.value,
            span: argument.value_span,
        })
}

fn uncertainty_diagnostic(
    code: &str,
    source_span: Option<SourceSpan>,
    fallback_span: SourceSpan,
    message: &str,
    help: Option<&str>,
) -> Diagnostic {
    let source_span = source_span.unwrap_or(fallback_span);
    Diagnostic::error(code, source_span.line, message, help).with_source_span(source_span)
}

fn validate_measured_arguments(
    arguments: &UncertaintyArguments,
    fallback_span: SourceSpan,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let measured = positional_argument_value(arguments, 0);
    if measured
        .and_then(|argument| numeric_prefix(argument.value))
        .is_none()
    {
        diagnostics.push(uncertainty_diagnostic(
            "E-UNC-ARGS-001",
            measured.map(|argument| argument.span),
            fallback_span,
            "`measured` requires a numeric measured value.",
            Some("Use `measured(12 degC, std=0.2 K)` or another numeric value with a unit."),
        ));
    }

    validate_optional_non_negative_value(
        arguments,
        &["std", "sigma", "uncertainty"],
        "`measured` standard deviation",
        fallback_span,
        diagnostics,
    );
    validate_optional_non_negative_value(
        arguments,
        &["error", "relative_error"],
        "`measured` relative error",
        fallback_span,
        diagnostics,
    );
}

fn validate_distribution_arguments(
    expression: &str,
    arguments: &UncertaintyArguments,
    fallback_span: SourceSpan,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match distribution_kind(expression, arguments).as_str() {
        "normal" => {
            validate_normal_arguments("distribution", arguments, fallback_span, diagnostics)
        }
        "uniform" => {
            validate_range_arguments("distribution", arguments, fallback_span, diagnostics)
        }
        unsupported => diagnostics.push(uncertainty_diagnostic(
            "E-UNC-ARGS-003",
            named_argument_value(arguments, &["kind", "distribution"])
                .map(|argument| argument.span),
            fallback_span,
            &format!("Unsupported uncertainty distribution `{unsupported}`."),
            Some("The current uncertainty track supports `normal` and `uniform` distributions."),
        )),
    }
}

fn validate_normal_arguments(
    call: &str,
    arguments: &UncertaintyArguments,
    fallback_span: SourceSpan,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mean = named_argument_value(arguments, &["mean", "mu"])
        .or_else(|| positional_argument_value(arguments, 0));
    if mean
        .and_then(|argument| numeric_prefix(argument.value))
        .is_none()
    {
        diagnostics.push(uncertainty_diagnostic(
            "E-UNC-ARGS-001",
            mean.map(|argument| argument.span),
            fallback_span,
            &format!("`{call}` requires a numeric `mean` value."),
            Some("Use `normal(mean=5 kW, std=0.8 kW, samples=31)`."),
        ));
    }

    validate_required_non_negative_value(
        arguments,
        &["std", "sigma"],
        &format!("`{call}` standard deviation"),
        fallback_span,
        diagnostics,
    );
}

fn validate_range_arguments(
    call: &str,
    arguments: &UncertaintyArguments,
    fallback_span: SourceSpan,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let lower = named_argument_value(arguments, &["lower", "min"])
        .or_else(|| positional_argument_value(arguments, 0));
    let upper = named_argument_value(arguments, &["upper", "max"])
        .or_else(|| positional_argument_value(arguments, 1));
    let lower_numeric = lower.and_then(|argument| numeric_prefix(argument.value));
    let upper_numeric = upper.and_then(|argument| numeric_prefix(argument.value));

    let (Some(lower_numeric), Some(upper_numeric)) = (lower_numeric, upper_numeric) else {
        let invalid_span = lower
            .filter(|argument| numeric_prefix(argument.value).is_none())
            .or_else(|| upper.filter(|argument| numeric_prefix(argument.value).is_none()))
            .map(|argument| argument.span);
        diagnostics.push(uncertainty_diagnostic(
            "E-UNC-ARGS-001",
            invalid_span,
            fallback_span,
            &format!("`{call}` requires two numeric bounds."),
            Some("Use positional bounds such as `uniform(0.3 kW, 0.7 kW)` or named `lower=`/`upper=` bounds."),
        ));
        return;
    };

    if lower_numeric > upper_numeric {
        diagnostics.push(uncertainty_diagnostic(
            "E-UNC-ARGS-002",
            lower.map(|argument| argument.span),
            fallback_span,
            &format!(
                "`{call}` lower bound {lower_numeric} is greater than upper bound {upper_numeric}."
            ),
            Some("Swap the bounds or correct the declared interval/range."),
        ));
    }
}

fn validate_propagation_arguments(
    arguments: &UncertaintyArguments,
    fallback_span: SourceSpan,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(method) = named_argument_value(arguments, &["method"]) {
        let normalized = method.value.trim().trim_matches('"').to_ascii_lowercase();
        if normalized != "linear" {
            diagnostics.push(uncertainty_diagnostic(
                "E-UNC-ARGS-003",
                Some(method.span),
                fallback_span,
                &format!(
                    "Unsupported uncertainty propagation method `{}`.",
                    method.value
                ),
                Some("The current uncertainty track supports `method=linear`."),
            ));
        }
    }

    validate_optional_numeric_value(
        arguments,
        &["scale", "gain"],
        "`propagate` scale/gain",
        fallback_span,
        diagnostics,
    );
    validate_optional_numeric_value(
        arguments,
        &["offset", "bias"],
        "`propagate` offset/bias",
        fallback_span,
        diagnostics,
    );
}

fn validate_sample_count_argument(
    call: &str,
    arguments: &UncertaintyArguments,
    fallback_span: SourceSpan,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(value) = named_argument_value(arguments, &["samples", "n"]) {
        match value.value.trim().parse::<usize>() {
            Ok(count) if (1..=256).contains(&count) => {}
            _ => diagnostics.push(uncertainty_diagnostic(
                "E-UNC-ARGS-002",
                Some(value.span),
                fallback_span,
                &format!("`{call}` sample count `{}` is invalid.", value.value),
                Some("Use an integer `samples` value between 1 and 256."),
            )),
        }
    }
}

fn validate_required_non_negative_value(
    arguments: &UncertaintyArguments,
    names: &[&str],
    label: &str,
    fallback_span: SourceSpan,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match named_argument_value(arguments, names) {
        Some(value) => validate_non_negative_value(value, label, fallback_span, diagnostics),
        None => diagnostics.push(uncertainty_diagnostic(
            "E-UNC-ARGS-001",
            None,
            fallback_span,
            &format!("{label} is required."),
            Some("Provide a non-negative value such as `std=0.8 kW`."),
        )),
    }
}

fn validate_optional_non_negative_value(
    arguments: &UncertaintyArguments,
    names: &[&str],
    label: &str,
    fallback_span: SourceSpan,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(value) = named_argument_value(arguments, names) {
        validate_non_negative_value(value, label, fallback_span, diagnostics);
    }
}

fn validate_non_negative_value(
    value: UncertaintyArgumentValue<'_>,
    label: &str,
    fallback_span: SourceSpan,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match numeric_prefix(value.value) {
        Some(parsed) if parsed >= 0.0 => {}
        Some(parsed) => diagnostics.push(uncertainty_diagnostic(
            "E-UNC-ARGS-002",
            Some(value.span),
            fallback_span,
            &format!("{label} must be non-negative, but found {parsed}."),
            Some("Use a zero or positive standard deviation."),
        )),
        None => diagnostics.push(uncertainty_diagnostic(
            "E-UNC-ARGS-001",
            Some(value.span),
            fallback_span,
            &format!("{label} must be numeric."),
            Some("Provide a numeric value with an optional unit."),
        )),
    }
}

fn validate_optional_numeric_value(
    arguments: &UncertaintyArguments,
    names: &[&str],
    label: &str,
    fallback_span: SourceSpan,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(value) = named_argument_value(arguments, names) {
        if numeric_prefix(value.value).is_none() {
            diagnostics.push(uncertainty_diagnostic(
                "E-UNC-ARGS-002",
                Some(value.span),
                fallback_span,
                &format!("{label} value `{}` must be numeric.", value.value),
                Some("Use a numeric value such as `scale=1.08` or `offset=0.4 kW`."),
            ));
        }
    }
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

fn measured_info(
    binding: &FastBinding,
    typed_bindings: &[TypedBinding],
    arguments: &UncertaintyArguments,
) -> UncertaintyInfo {
    let display_unit =
        first_unit_in_expression(&binding.expression).unwrap_or_else(|| "1".to_owned());
    let quantity_kind = infer_quantity(&binding.name, &binding.expression, &display_unit);
    let source = first_argument(arguments);
    let source_span = uncertainty_source_span(binding, arguments, source.as_deref());
    UncertaintyInfo {
        binding: binding.name.clone(),
        binding_span: binding.span,
        kind: "Measured".to_owned(),
        positional_arguments: arguments.positional.clone(),
        named_arguments: arguments.named.clone(),
        quantity_kind,
        display_unit,
        expression: binding.expression.clone(),
        expression_span: binding.expression_span,
        source,
        source_span,
        distribution: Some("measured".to_owned()),
        method: None,
        scale: None,
        offset: None,
        mean: first_value_with_unit(arguments),
        stddev: named_value(arguments, &["std", "sigma", "uncertainty"]),
        error: named_value(arguments, &["error", "relative_error"]),
        lower: None,
        upper: None,
        sample_count: 1,
        propagation: propagation_terms(&binding.expression, typed_bindings),
        line: binding.line,
    }
}

fn interval_info(
    binding: &FastBinding,
    typed_bindings: &[TypedBinding],
    arguments: &UncertaintyArguments,
) -> UncertaintyInfo {
    let display_unit =
        first_unit_in_expression(&binding.expression).unwrap_or_else(|| "1".to_owned());
    let quantity_kind = infer_quantity(&binding.name, &binding.expression, &display_unit);
    let values = values_with_unit(arguments);
    let lower = named_value(arguments, &["lower", "min"]).or_else(|| values.first().cloned());
    let upper = named_value(arguments, &["upper", "max"]).or_else(|| values.get(1).cloned());
    let source = first_argument(arguments);
    let source_span = uncertainty_source_span(binding, arguments, source.as_deref());
    UncertaintyInfo {
        binding: binding.name.clone(),
        binding_span: binding.span,
        kind: "Interval".to_owned(),
        positional_arguments: arguments.positional.clone(),
        named_arguments: arguments.named.clone(),
        quantity_kind,
        display_unit,
        expression: binding.expression.clone(),
        expression_span: binding.expression_span,
        source,
        source_span,
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

fn distribution_info(
    binding: &FastBinding,
    typed_bindings: &[TypedBinding],
    arguments: &UncertaintyArguments,
) -> UncertaintyInfo {
    let display_unit =
        first_unit_in_expression(&binding.expression).unwrap_or_else(|| "1".to_owned());
    let quantity_kind = infer_quantity(&binding.name, &binding.expression, &display_unit);
    let distribution = distribution_kind(&binding.expression, arguments);
    let values = values_with_unit(arguments);
    let lower = named_value(arguments, &["lower", "min"]).or_else(|| values.first().cloned());
    let upper = named_value(arguments, &["upper", "max"]).or_else(|| values.get(1).cloned());
    let source = first_argument(arguments);
    let source_span = uncertainty_source_span(binding, arguments, source.as_deref());
    UncertaintyInfo {
        binding: binding.name.clone(),
        binding_span: binding.span,
        kind: "Distribution".to_owned(),
        positional_arguments: arguments.positional.clone(),
        named_arguments: arguments.named.clone(),
        quantity_kind,
        display_unit,
        expression: binding.expression.clone(),
        expression_span: binding.expression_span,
        source,
        source_span,
        distribution: Some(distribution),
        method: None,
        scale: None,
        offset: None,
        mean: named_value(arguments, &["mean", "mu"]).or_else(|| first_value_with_unit(arguments)),
        stddev: named_value(arguments, &["std", "sigma"]),
        error: None,
        lower,
        upper,
        sample_count: sample_count(arguments).unwrap_or(64),
        propagation: propagation_terms(&binding.expression, typed_bindings),
        line: binding.line,
    }
}

fn ensemble_info(
    binding: &FastBinding,
    typed_bindings: &[TypedBinding],
    arguments: &UncertaintyArguments,
) -> UncertaintyInfo {
    let source = first_argument(arguments);
    let source_span = uncertainty_source_span(binding, arguments, source.as_deref());
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
        binding_span: binding.span,
        kind: "Ensemble".to_owned(),
        positional_arguments: arguments.positional.clone(),
        named_arguments: arguments.named.clone(),
        quantity_kind,
        display_unit: display_unit_for_binding(binding, typed_bindings, arguments),
        expression: binding.expression.clone(),
        expression_span: binding.expression_span,
        source,
        source_span,
        distribution: Some("ensemble".to_owned()),
        method: Some(
            named_value(arguments, &["method"])
                .unwrap_or_else(|| "deterministic_resample".to_owned()),
        ),
        scale: None,
        offset: None,
        mean: None,
        stddev: None,
        error: None,
        lower: None,
        upper: None,
        sample_count: sample_count(arguments).unwrap_or(32),
        propagation: propagation_terms(&binding.expression, typed_bindings),
        line: binding.line,
    }
}

fn propagation_info(
    binding: &FastBinding,
    typed_bindings: &[TypedBinding],
    arguments: &UncertaintyArguments,
) -> UncertaintyInfo {
    let source = first_argument(arguments);
    let source_span = uncertainty_source_span(binding, arguments, source.as_deref());
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
        binding_span: binding.span,
        kind: "Distribution".to_owned(),
        positional_arguments: arguments.positional.clone(),
        named_arguments: arguments.named.clone(),
        quantity_kind,
        display_unit: display_unit_for_binding(binding, typed_bindings, arguments),
        expression: binding.expression.clone(),
        expression_span: binding.expression_span,
        source,
        source_span,
        distribution: Some("propagated".to_owned()),
        method: Some(named_value(arguments, &["method"]).unwrap_or_else(|| "linear".to_owned())),
        scale: named_value(arguments, &["scale", "gain"]),
        offset: named_value(arguments, &["offset", "bias"]),
        mean: None,
        stddev: None,
        error: None,
        lower: None,
        upper: None,
        sample_count: sample_count(arguments).unwrap_or(64),
        propagation: propagation_terms(&binding.expression, typed_bindings),
        line: binding.line,
    }
}

fn arithmetic_info(
    binding: &FastBinding,
    typed_bindings: &[TypedBinding],
    arguments: &UncertaintyArguments,
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
    let source = sources[0].name.clone();
    let source_span = uncertainty_source_span(binding, arguments, Some(&source));
    Some(UncertaintyInfo {
        binding: binding.name.clone(),
        binding_span: binding.span,
        kind,
        positional_arguments: arguments.positional.clone(),
        named_arguments: arguments.named.clone(),
        quantity_kind,
        display_unit: sources[0].semantic_type.display_unit.clone(),
        expression: binding.expression.clone(),
        expression_span: binding.expression_span,
        source: Some(source),
        source_span,
        distribution: Some("arithmetic".to_owned()),
        method: Some(method.to_owned()),
        scale: None,
        offset: None,
        mean: None,
        stddev: None,
        error: None,
        lower: None,
        upper: None,
        sample_count: sample_count(arguments).unwrap_or(64),
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

fn display_unit_for_binding(
    binding: &FastBinding,
    typed_bindings: &[TypedBinding],
    arguments: &UncertaintyArguments,
) -> String {
    first_unit_in_expression(&binding.expression)
        .or_else(|| {
            first_argument(arguments).and_then(|source| {
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

fn uncertainty_arguments(binding: &FastBinding) -> UncertaintyArguments {
    if uncertainty_call_name(&binding.expression).is_none() {
        return UncertaintyArguments::default();
    }
    let expression = binding.expression.as_str();
    let Some(inside) = call_inside(expression) else {
        return UncertaintyArguments::default();
    };
    let mut arguments = UncertaintyArguments::default();
    for part in split_top_level_commas(inside) {
        if part.is_empty() {
            continue;
        }
        if let Some(equals) = top_level_equals(part) {
            let name = part[..equals].trim();
            let value = part[equals + 1..].trim();
            if is_identifier(name) && !value.is_empty() {
                let Some(key_span) = subslice_span(expression, binding.expression_span, name)
                else {
                    continue;
                };
                let Some(value_span) = subslice_span(expression, binding.expression_span, value)
                else {
                    continue;
                };
                arguments.named.push(UncertaintyNamedArgumentInfo {
                    name: name.to_owned(),
                    value: value.to_owned(),
                    key_span,
                    value_span,
                });
                continue;
            }
        }
        if let Some(span) = subslice_span(expression, binding.expression_span, part) {
            arguments.positional.push(UncertaintyValueInfo {
                value: part.to_owned(),
                span,
            });
        }
    }
    arguments
}

fn split_top_level_commas(value: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;
    let mut brace_depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in value.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match character {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' => paren_depth += 1,
            ')' => paren_depth -= 1,
            '[' => bracket_depth += 1,
            ']' => bracket_depth -= 1,
            '{' => brace_depth += 1,
            '}' => brace_depth -= 1,
            ',' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                parts.push(value[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(value[start..].trim());
    parts
}

fn top_level_equals(value: &str) -> Option<usize> {
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;
    let mut brace_depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in value.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match character {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' => paren_depth += 1,
            ')' => paren_depth -= 1,
            '[' => bracket_depth += 1,
            ']' => bracket_depth -= 1,
            '{' => brace_depth += 1,
            '}' => brace_depth -= 1,
            '=' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                return Some(index);
            }
            _ => {}
        }
    }
    None
}

fn subslice_span(parent_text: &str, parent_span: SourceSpan, value: &str) -> Option<SourceSpan> {
    if parent_span.end.checked_sub(parent_span.start)? != parent_text.len() {
        return None;
    }
    let start = (value.as_ptr() as usize).checked_sub(parent_text.as_ptr() as usize)?;
    let end = start.checked_add(value.len())?;
    if end > parent_text.len()
        || !parent_text.is_char_boundary(start)
        || !parent_text.is_char_boundary(end)
    {
        return None;
    }
    Some(SourceSpan::new_in_source(
        parent_span.source_id,
        parent_span.start.checked_add(start)?,
        parent_span.start.checked_add(end)?,
        parent_span.line,
        parent_span.column.checked_add(start)?,
    ))
}

fn uncertainty_source_span(
    binding: &FastBinding,
    arguments: &UncertaintyArguments,
    source: Option<&str>,
) -> Option<SourceSpan> {
    let source = source?.trim();
    if let Some(argument) = arguments
        .positional
        .first()
        .filter(|argument| argument.value == source)
    {
        return Some(argument.span);
    }
    identifier_occurrence_span(binding, source)
}

fn identifier_occurrence_span(binding: &FastBinding, identifier: &str) -> Option<SourceSpan> {
    if !is_identifier(identifier)
        || binding
            .expression_span
            .end
            .checked_sub(binding.expression_span.start)
            != Some(binding.expression.len())
    {
        return None;
    }
    let expression = binding.expression.as_str();
    let mut search_start = 0usize;
    while search_start < expression.len() {
        let relative_start = expression.get(search_start..)?.find(identifier)?;
        let start = search_start + relative_start;
        let end = start.checked_add(identifier.len())?;
        let before = expression.get(..start)?.chars().next_back();
        let after = expression.get(end..)?.chars().next();
        if before.is_none_or(|character| !is_identifier_character(character))
            && after.is_none_or(|character| !is_identifier_character(character))
        {
            return Some(SourceSpan::new_in_source(
                binding.expression_span.source_id,
                binding.expression_span.start.checked_add(start)?,
                binding.expression_span.start.checked_add(end)?,
                binding.expression_span.line,
                binding.expression_span.column.checked_add(start)?,
            ));
        }
        search_start = end;
    }
    None
}

fn is_identifier_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn first_argument(arguments: &UncertaintyArguments) -> Option<String> {
    positional_argument_value(arguments, 0).map(|argument| argument.value.to_owned())
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

fn named_value(arguments: &UncertaintyArguments, names: &[&str]) -> Option<String> {
    named_argument_value(arguments, names).map(|argument| argument.value.to_owned())
}

fn distribution_kind(expression: &str, arguments: &UncertaintyArguments) -> String {
    let lowered = expression.trim().to_ascii_lowercase();
    if lowered.starts_with("normal(") {
        return "normal".to_owned();
    }
    if lowered.starts_with("uniform(") {
        return "uniform".to_owned();
    }
    named_value(arguments, &["kind", "distribution"])
        .map(|value| value.trim_matches('"').to_ascii_lowercase())
        .unwrap_or_else(|| "normal".to_owned())
}

fn sample_count(arguments: &UncertaintyArguments) -> Option<usize> {
    named_value(arguments, &["samples", "n"]).and_then(|value| value.parse::<usize>().ok())
}

fn first_value_with_unit(arguments: &UncertaintyArguments) -> Option<String> {
    values_with_unit(arguments).into_iter().next()
}

fn values_with_unit(arguments: &UncertaintyArguments) -> Vec<String> {
    arguments
        .positional
        .iter()
        .map(|argument| argument.value.clone())
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
