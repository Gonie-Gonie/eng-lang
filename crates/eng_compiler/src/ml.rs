use crate::ast::FastBinding;

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
