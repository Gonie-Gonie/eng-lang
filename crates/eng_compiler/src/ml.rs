use crate::ast::FastBinding;
use crate::semantic::TypedBinding;
use crate::Diagnostic;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MlInfo {
    pub binding: String,
    pub kind: String,
    pub source: Option<String>,
    pub prediction_input: Option<String>,
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
    } else if is_table_regression_expression(expression) {
        "RegressionModel"
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
    } else if predict_model_arguments(expression).is_some() {
        "PredictionResult"
    } else {
        return None;
    };
    let prediction_arguments = (kind == "PredictionResult")
        .then(|| predict_model_arguments(expression))
        .flatten();
    let source = prediction_arguments
        .as_ref()
        .map(|(model, _)| model.clone())
        .or_else(|| table_regression_source(expression))
        .or_else(|| first_argument(expression));

    Some(MlInfo {
        binding: binding.name.clone(),
        kind: kind.to_owned(),
        source,
        prediction_input: prediction_arguments.map(|(_, input)| input),
        target: (kind != "PredictionResult")
            .then(|| named_value(expression, &["target"]))
            .flatten(),
        features: if kind == "PredictionResult" {
            Vec::new()
        } else {
            list_value(expression, "features")
        },
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

pub fn argument_diagnostics(binding: &FastBinding) -> Vec<Diagnostic> {
    let Some(info) = ml_info(binding) else {
        return Vec::new();
    };
    let mut diagnostics = Vec::new();
    match info.kind.as_str() {
        "TrainTestSplit" => validate_train_test_split_arguments(&info, &mut diagnostics),
        "RegressionModel" => validate_regression_arguments(&info, &mut diagnostics),
        "MlpModel" => validate_mlp_arguments(&info, &mut diagnostics),
        _ => {}
    }
    diagnostics
}

pub fn ml_semantic_type(expression: &str) -> Option<(String, String)> {
    let lowered = expression.trim().to_ascii_lowercase();
    if lowered.starts_with("train_test_split(") {
        Some(("TrainTestSplit".to_owned(), "split".to_owned()))
    } else if is_table_regression_expression(expression) || lowered.starts_with("regression(") {
        Some(("Model[Regression]".to_owned(), "model".to_owned()))
    } else if lowered.starts_with("mlp(") || lowered.starts_with("ann(") {
        Some(("Model[MLP]".to_owned(), "model".to_owned()))
    } else if lowered.starts_with("evaluate(") || lowered.starts_with("metrics(") {
        Some(("ModelMetrics".to_owned(), "1".to_owned()))
    } else if lowered.starts_with("model_card(") {
        Some(("ModelCard".to_owned(), "text".to_owned()))
    } else if lowered.starts_with("leakage_lint(") {
        Some(("LeakageLint".to_owned(), "lint".to_owned()))
    } else if predict_model_arguments(expression).is_some() {
        Some(("Table[Prediction]".to_owned(), "schema-defined".to_owned()))
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpectedMlSource {
    TimeSeries,
    TrainTestSplit,
    Model,
    Table,
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
        "RegressionModel" if is_table_regression_expression(&info.expression) => {
            vec![MlSourceRequirement {
                call,
                role: "table",
                source: info.source.clone(),
                expected: ExpectedMlSource::Table,
            }]
        }
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
        "PredictionResult" => vec![
            MlSourceRequirement {
                call,
                role: "model",
                source: info.source.clone(),
                expected: ExpectedMlSource::Model,
            },
            MlSourceRequirement {
                call,
                role: "input",
                source: info.prediction_input.clone(),
                expected: ExpectedMlSource::Table,
            },
        ],
        _ => Vec::new(),
    };

    if info.kind == "TrainTestSplit" {
        if let Some(target) = &info.target {
            requirements.push(MlSourceRequirement {
                call,
                role: "target",
                source: Some(target.clone()),
                expected: ExpectedMlSource::TimeSeries,
            });
        }
    }

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
        ExpectedMlSource::Table => {
            quantity_kind.starts_with("Table[") && quantity_kind.ends_with(']')
                || quantity_kind == "TableTransform[Derive]"
        }
    }
}

fn expected_source_label(expected: ExpectedMlSource) -> &'static str {
    match expected {
        ExpectedMlSource::TimeSeries => "TimeSeries",
        ExpectedMlSource::TrainTestSplit => "TrainTestSplit",
        ExpectedMlSource::Model => "Model[Regression] or Model[MLP]",
        ExpectedMlSource::Table => "Table[...] or materialized derive table",
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
        ExpectedMlSource::Table => {
            "Promote, generate, or derive a table first, then pass it as `predict model using samples` or `regression_table(table, ...)`."
        }
    }
}

fn ml_call_name(expression: &str) -> &'static str {
    let lowered = expression.trim_start().to_ascii_lowercase();
    if lowered.starts_with("train_test_split(") {
        "train_test_split"
    } else if is_table_regression_expression(expression) {
        "regression_table"
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
    } else if lowered.starts_with("predict ") {
        "predict"
    } else {
        "ml"
    }
}

fn validate_train_test_split_arguments(info: &MlInfo, diagnostics: &mut Vec<Diagnostic>) {
    if info
        .target
        .as_deref()
        .filter(|target| is_identifier(target))
        .is_none()
    {
        diagnostics.push(Diagnostic::error(
            "E-ML-ARGS-001",
            info.line,
            "`train_test_split` requires `target=<TimeSeriesName>`.",
            Some("Pass the target series explicitly, for example `target=Q_coil`."),
        ));
    }

    if info.features.is_empty() {
        diagnostics.push(Diagnostic::error(
            "E-ML-ARGS-001",
            info.line,
            "`train_test_split` requires at least one feature column in `features=[...]`.",
            Some("List feature columns such as `features=[T_supply, T_return, m_dot]`."),
        ));
    } else if let Some(feature) = info.features.iter().find(|feature| !is_identifier(feature)) {
        diagnostics.push(Diagnostic::error(
            "E-ML-ARGS-001",
            info.line,
            &format!("Feature `{feature}` is not a valid identifier."),
            Some("Use bare schema column names in `features=[...]`; runtime leakage lint checks whether they exist in the source table."),
        ));
    }

    match info.test_fraction.as_deref() {
        Some(value) if parse_test_fraction(value).is_some() => {}
        Some(value) => diagnostics.push(Diagnostic::error(
            "E-ML-ARGS-002",
            info.line,
            &format!("`test={value}` is not a valid held-out fraction."),
            Some("Use a value between 0 and 1, or a percentage such as `20%`."),
        )),
        None => diagnostics.push(Diagnostic::error(
            "E-ML-ARGS-001",
            info.line,
            "`train_test_split` requires `test=<fraction>`.",
            Some("Use an explicit held-out fraction such as `test=0.25`."),
        )),
    }

    validate_optional_integer(&info.expression, "seed", info.line, diagnostics);
}

fn validate_regression_arguments(info: &MlInfo, diagnostics: &mut Vec<Diagnostic>) {
    if is_table_regression_expression(&info.expression) {
        if info
            .target
            .as_deref()
            .filter(|target| is_identifier(target))
            .is_none()
        {
            diagnostics.push(Diagnostic::error(
                "E-ML-ARGS-001",
                info.line,
                "`regression_table` requires `target=<column>`.",
                Some("Pass the target table column, for example `target=annual_electricity`."),
            ));
        }
        if info.features.is_empty() {
            diagnostics.push(Diagnostic::error(
                "E-ML-ARGS-001",
                info.line,
                "`regression_table` requires `features=[...]`.",
                Some("List feature columns such as `features=[people_density, cooling_cop]`."),
            ));
        } else if let Some(feature) = info.features.iter().find(|feature| !is_identifier(feature)) {
            diagnostics.push(Diagnostic::error(
                "E-ML-ARGS-001",
                info.line,
                &format!("Feature `{feature}` is not a valid identifier."),
                Some("Use bare table column names in `features=[...]`."),
            ));
        }
        if let Some(value) = info.test_fraction.as_deref() {
            if parse_test_fraction(value).is_none() {
                diagnostics.push(Diagnostic::error(
                    "E-ML-ARGS-002",
                    info.line,
                    &format!("`test={value}` is not a valid held-out fraction."),
                    Some("Use a value between 0 and 1, or a percentage such as `20%`."),
                ));
            }
        }
        validate_optional_integer(&info.expression, "seed", info.line, diagnostics);
    }
    if let Some(algorithm) = named_value(&info.expression, &["algorithm"]) {
        if algorithm != "linear" {
            diagnostics.push(Diagnostic::error(
                "E-ML-ARGS-003",
                info.line,
                &format!("Unsupported regression algorithm `{algorithm}`."),
                Some("The current data-driven modeling track supports `algorithm=linear`."),
            ));
        }
    }
}

fn validate_mlp_arguments(info: &MlInfo, diagnostics: &mut Vec<Diagnostic>) {
    match named_value(&info.expression, &["hidden", "layers"]) {
        Some(value) if valid_positive_usize_list(&value) => {}
        Some(value) => diagnostics.push(Diagnostic::error(
            "E-ML-ARGS-002",
            info.line,
            &format!("`hidden={value}` must contain positive integer layer sizes."),
            Some("Use a list such as `hidden=[4]` or `hidden=[8, 4]`."),
        )),
        None => diagnostics.push(Diagnostic::error(
            "E-ML-ARGS-001",
            info.line,
            "`mlp` requires `hidden=[...]`.",
            Some("Use explicit hidden layer sizes such as `hidden=[4]`."),
        )),
    }

    match named_value(&info.expression, &["epochs"]) {
        Some(value) if parse_positive_usize(&value).is_some() => {}
        Some(value) => diagnostics.push(Diagnostic::error(
            "E-ML-ARGS-002",
            info.line,
            &format!("`epochs={value}` must be a positive integer."),
            Some("Use an explicit training budget such as `epochs=80`."),
        )),
        None => diagnostics.push(Diagnostic::error(
            "E-ML-ARGS-001",
            info.line,
            "`mlp` requires `epochs=<positive integer>`.",
            Some("Use an explicit training budget such as `epochs=80`."),
        )),
    }

    validate_optional_integer(&info.expression, "seed", info.line, diagnostics);
}

fn parse_test_fraction(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    let parsed = if let Some(percent) = trimmed.strip_suffix('%') {
        percent.trim().parse::<f64>().ok()? / 100.0
    } else {
        trimmed.parse::<f64>().ok()?
    };
    (parsed > 0.0 && parsed < 1.0).then_some(parsed)
}

fn validate_optional_integer(
    expression: &str,
    name: &str,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(value) = named_value(expression, &[name]) {
        if value.trim().parse::<usize>().is_err() {
            diagnostics.push(Diagnostic::error(
                "E-ML-ARGS-002",
                line,
                &format!("`{name}={value}` must be a non-negative integer."),
                Some("Use an integer seed such as `seed=7`."),
            ));
        }
    }
}

fn valid_positive_usize_list(value: &str) -> bool {
    let values = list_items(value);
    !values.is_empty()
        && values
            .iter()
            .all(|value| parse_positive_usize(value).is_some())
}

fn parse_positive_usize(value: &str) -> Option<usize> {
    let parsed = value.trim().parse::<usize>().ok()?;
    (parsed > 0).then_some(parsed)
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

fn is_table_regression_expression(expression: &str) -> bool {
    let lowered = expression.trim_start().to_ascii_lowercase();
    lowered.starts_with("regression_table(") || lowered.starts_with("train_regression(")
}

fn table_regression_source(expression: &str) -> Option<String> {
    is_table_regression_expression(expression)
        .then(|| first_argument(expression))
        .flatten()
}

fn predict_model_arguments(expression: &str) -> Option<(String, String)> {
    let trimmed = expression.trim();
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let keyword = parts.next()?;
    if !keyword.eq_ignore_ascii_case("predict") {
        return None;
    }
    let rest = parts.next()?.trim();
    let rest_lower = rest.to_ascii_lowercase();
    let marker = " using ";
    let using_index = rest_lower.find(marker)?;
    let model = rest[..using_index].trim();
    let input = rest[using_index + marker.len()..].trim();
    if model.is_empty() || input.is_empty() {
        return None;
    }
    Some((model.to_owned(), input.to_owned()))
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
        .map(|value| list_items(&value).into_iter().map(str::to_owned).collect())
        .unwrap_or_default()
}

fn parse_usize_list(value: &str) -> Vec<usize> {
    list_items(value)
        .into_iter()
        .filter_map(|part| part.parse::<usize>().ok())
        .collect()
}

fn list_items(value: &str) -> Vec<&str> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
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
