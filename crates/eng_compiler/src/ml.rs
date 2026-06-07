use crate::ast::FastBinding;
use crate::semantic::TypedBinding;
use crate::Diagnostic;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MlInfo {
    pub binding: String,
    pub kind: String,
    pub source: Option<String>,
    pub target: Option<String>,
    pub features: Vec<String>,
    pub algorithm: Option<String>,
    pub test_fraction: Option<String>,
    pub seed: Option<String>,
    pub hidden_layers: Vec<usize>,
    pub epochs: Option<usize>,
    pub expression: String,
    pub line: usize,
}

pub fn ml_info(binding: &FastBinding) -> Option<MlInfo> {
    let expression = binding.expression.trim();
    let lowered = expression.to_ascii_lowercase();
    let kind = if lowered.starts_with("train_test_split(") {
        "TrainTestSplit"
    } else if lowered.starts_with("regression(") {
        "RegressionModel"
    } else if lowered.starts_with("mlp(") || lowered.starts_with("ann(") {
        "MlpModel"
    } else if lowered.starts_with("evaluate(") || lowered.starts_with("metrics(") {
        "ModelMetrics"
    } else if lowered.starts_with("model_card(") {
        "ModelCard"
    } else if lowered.starts_with("leakage_lint(") {
        "LeakageLint"
    } else {
        return None;
    };

    Some(MlInfo {
        binding: binding.name.clone(),
        kind: kind.to_owned(),
        source: first_argument(expression),
        target: named_value(expression, &["target"]),
        features: list_value(expression, "features"),
        algorithm: named_value(expression, &["algorithm"]).or_else(|| default_algorithm(kind)),
        test_fraction: named_value(expression, &["test", "test_fraction"]),
        seed: named_value(expression, &["seed"]),
        hidden_layers: named_value(expression, &["hidden", "layers"])
            .map(|value| parse_usize_list(&value))
            .unwrap_or_default(),
        epochs: named_value(expression, &["epochs"]).and_then(|value| value.parse::<usize>().ok()),
        expression: binding.expression.clone(),
        line: binding.line,
    })
}

pub fn source_diagnostics(
    binding: &FastBinding,
    typed_bindings: &[TypedBinding],
) -> Vec<Diagnostic> {
    let Some(info) = ml_info(binding) else {
        return Vec::new();
    };

    source_requirements(&info)
        .into_iter()
        .filter_map(|requirement| {
            validate_source_requirement(&requirement, binding.line, typed_bindings)
        })
        .collect()
}

pub fn ml_semantic_type(expression: &str) -> Option<(String, String)> {
    let lowered = expression.trim().to_ascii_lowercase();
    if lowered.starts_with("train_test_split(") {
        Some(("TrainTestSplit".to_owned(), "split".to_owned()))
    } else if lowered.starts_with("regression(") {
        Some(("Model[Regression]".to_owned(), "model".to_owned()))
    } else if lowered.starts_with("mlp(") || lowered.starts_with("ann(") {
        Some(("Model[MLP]".to_owned(), "model".to_owned()))
    } else if lowered.starts_with("evaluate(") || lowered.starts_with("metrics(") {
        Some(("ModelMetrics".to_owned(), "1".to_owned()))
    } else if lowered.starts_with("model_card(") {
        Some(("ModelCard".to_owned(), "text".to_owned()))
    } else if lowered.starts_with("leakage_lint(") {
        Some(("LeakageLint".to_owned(), "lint".to_owned()))
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpectedMlSource {
    TimeSeries,
    TrainTestSplit,
    Model,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MlSourceRequirement {
    call: &'static str,
    role: &'static str,
    source: Option<String>,
    expected: ExpectedMlSource,
}

fn source_requirements(info: &MlInfo) -> Vec<MlSourceRequirement> {
    let call = ml_call_name(&info.expression);
    let mut requirements = match info.kind.as_str() {
        "TrainTestSplit" => vec![MlSourceRequirement {
            call,
            role: "source",
            source: info.source.clone(),
            expected: ExpectedMlSource::TimeSeries,
        }],
        "RegressionModel" | "MlpModel" | "LeakageLint" => vec![MlSourceRequirement {
            call,
            role: "split",
            source: info.source.clone(),
            expected: ExpectedMlSource::TrainTestSplit,
        }],
        "ModelMetrics" => vec![MlSourceRequirement {
            call,
            role: "model",
            source: info.source.clone(),
            expected: ExpectedMlSource::Model,
        }],
        "ModelCard" => vec![MlSourceRequirement {
            call,
            role: "model",
            source: info.source.clone(),
            expected: ExpectedMlSource::Model,
        }],
        _ => Vec::new(),
    };

    if info.kind == "ModelMetrics" {
        if let Some(split) = named_value(&info.expression, &["split"]) {
            requirements.push(MlSourceRequirement {
                call,
                role: "split",
                source: Some(split),
                expected: ExpectedMlSource::TrainTestSplit,
            });
        }
    }

    requirements
}

fn validate_source_requirement(
    requirement: &MlSourceRequirement,
    line: usize,
    typed_bindings: &[TypedBinding],
) -> Option<Diagnostic> {
    let Some(source) = requirement
        .source
        .as_deref()
        .filter(|source| is_identifier(source))
    else {
        return Some(Diagnostic::error(
            "E-ML-SOURCE-001",
            line,
            &format!(
                "`{}` requires a prior {} binding as its {} argument.",
                requirement.call,
                expected_source_label(requirement.expected),
                requirement.role
            ),
            Some(expected_source_help(requirement.expected)),
        ));
    };

    let Some(source_binding) = typed_bindings
        .iter()
        .find(|typed_binding| typed_binding.name == source)
    else {
        return Some(Diagnostic::error(
            "E-ML-SOURCE-001",
            line,
            &format!(
                "Unknown ML {} `{source}` for `{}`.",
                requirement.role, requirement.call
            ),
            Some("Check the source name or move the referenced ML binding before this expression."),
        ));
    };

    if !matches_expected_source(
        &source_binding.semantic_type.quantity_kind,
        requirement.expected,
    ) {
        return Some(Diagnostic::error(
            "E-ML-SOURCE-002",
            line,
            &format!(
                "`{source}` is {}, but `{}` expects {} for its {} argument.",
                source_binding.semantic_type.quantity_kind,
                requirement.call,
                expected_source_label(requirement.expected),
                requirement.role
            ),
            Some(expected_source_help(requirement.expected)),
        ));
    }

    None
}

fn matches_expected_source(quantity_kind: &str, expected: ExpectedMlSource) -> bool {
    match expected {
        ExpectedMlSource::TimeSeries => crate::stats::time_series_quantity(quantity_kind).is_some(),
        ExpectedMlSource::TrainTestSplit => quantity_kind == "TrainTestSplit",
        ExpectedMlSource::Model => {
            quantity_kind.starts_with("Model[") && quantity_kind.ends_with(']')
        }
    }
}

fn expected_source_label(expected: ExpectedMlSource) -> &'static str {
    match expected {
        ExpectedMlSource::TimeSeries => "TimeSeries",
        ExpectedMlSource::TrainTestSplit => "TrainTestSplit",
        ExpectedMlSource::Model => "Model[Regression] or Model[MLP]",
    }
}

fn expected_source_help(expected: ExpectedMlSource) -> &'static str {
    match expected {
        ExpectedMlSource::TimeSeries => {
            "Compute a TimeSeries such as `Q_coil` before calling `train_test_split(Q_coil, ...)`."
        }
        ExpectedMlSource::TrainTestSplit => {
            "Create `split = train_test_split(...)` first, then pass `split` to this ML call."
        }
        ExpectedMlSource::Model => {
            "Create `reg_model = regression(split, ...)` or `mlp_model = mlp(split, ...)` first."
        }
    }
}

fn ml_call_name(expression: &str) -> &'static str {
    let lowered = expression.trim_start().to_ascii_lowercase();
    if lowered.starts_with("train_test_split(") {
        "train_test_split"
    } else if lowered.starts_with("regression(") {
        "regression"
    } else if lowered.starts_with("mlp(") {
        "mlp"
    } else if lowered.starts_with("ann(") {
        "ann"
    } else if lowered.starts_with("evaluate(") {
        "evaluate"
    } else if lowered.starts_with("metrics(") {
        "metrics"
    } else if lowered.starts_with("model_card(") {
        "model_card"
    } else if lowered.starts_with("leakage_lint(") {
        "leakage_lint"
    } else {
        "ml"
    }
}

fn default_algorithm(kind: &str) -> Option<String> {
    match kind {
        "RegressionModel" => Some("linear".to_owned()),
        "MlpModel" => Some("mlp".to_owned()),
        _ => None,
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
    for part in split_top_level_commas(inside) {
        let Some((name, value)) = part.split_once('=') else {
            continue;
        };
        if names.iter().any(|candidate| name.trim() == *candidate) {
            return Some(value.trim().to_owned());
        }
    }
    None
}

fn list_value(expression: &str, name: &str) -> Vec<String> {
    named_value(expression, &[name])
        .map(|value| {
            value
                .trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_usize_list(value: &str) -> Vec<usize> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .filter_map(|part| part.trim().parse::<usize>().ok())
        .collect()
}

fn split_top_level_commas(value: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut bracket_depth = 0i32;
    for (index, character) in value.char_indices() {
        match character {
            '[' => bracket_depth += 1,
            ']' => bracket_depth -= 1,
            ',' if bracket_depth == 0 => {
                parts.push(value[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(value[start..].trim());
    parts
}

fn call_inside(expression: &str) -> Option<&str> {
    let open = expression.find('(')?;
    let close = expression.rfind(')')?;
    (close > open).then(|| &expression[open + 1..close])
}
