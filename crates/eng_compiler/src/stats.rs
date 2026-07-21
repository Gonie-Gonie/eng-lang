use crate::ast::{FastBinding, SummaryDecl};
use crate::semantic::TypedBinding;
use crate::{Diagnostic, SourceSpan};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AxisInfo {
    pub binding: String,
    pub axis: String,
    pub role: String,
    pub source: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatsInfo {
    pub source: String,
    pub source_type: String,
    pub quantity_kind: String,
    pub axis: String,
    pub statistics: Vec<String>,
    pub cache_key: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntegrationInfo {
    pub binding: String,
    pub source: String,
    pub input_quantity: String,
    pub over_axis: String,
    pub result_quantity: String,
    pub line: usize,
}

/// Identifier pattern shared with editor grammars for nearest-rank percentile statistics.
pub const PERCENTILE_STATISTIC_PATTERN: &str = r"p0*(?:[1-9][0-9]?|100)";

/// Parses a `p1` through `p100` statistic identifier into a fraction.
pub fn parse_percentile_fraction(name: &str) -> Option<f64> {
    let digits = name.strip_prefix('p')?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let percentile = digits.parse::<u32>().ok()?;
    (1..=100)
        .contains(&percentile)
        .then_some(percentile as f64 / 100.0)
}

/// Returns whether a name is a supported nearest-rank percentile statistic.
pub fn is_percentile_statistic(name: &str) -> bool {
    parse_percentile_fraction(name).is_some()
}

pub fn axis_infos(bindings: &[TypedBinding]) -> Vec<AxisInfo> {
    bindings
        .iter()
        .filter_map(|binding| {
            if binding.semantic_type.quantity_kind == "Table[Time]" {
                return Some(AxisInfo {
                    binding: binding.name.clone(),
                    axis: "Time".to_owned(),
                    role: "index".to_owned(),
                    source: "schema".to_owned(),
                    line: binding.line,
                });
            }

            time_series_quantity(&binding.semantic_type.quantity_kind).map(|(axis, _)| AxisInfo {
                binding: binding.name.clone(),
                axis,
                role: "sample_axis".to_owned(),
                source: "timeseries".to_owned(),
                line: binding.line,
            })
        })
        .collect()
}

pub fn stats_info(summary: &SummaryDecl, bindings: &[TypedBinding]) -> Option<StatsInfo> {
    let binding = bindings
        .iter()
        .find(|binding| binding.name == summary.source)?;
    let (axis, quantity_kind) = time_series_quantity(&binding.semantic_type.quantity_kind)
        .or_else(|| runtime_materialized_time_series(&binding.semantic_type.quantity_kind))?;
    let statistics = if summary.statistics.is_empty() {
        vec!["mean".to_owned(), "max".to_owned(), "p95".to_owned()]
    } else {
        summary.statistics.clone()
    };

    Some(StatsInfo {
        source: summary.source.clone(),
        source_type: binding.semantic_type.quantity_kind.clone(),
        quantity_kind,
        axis: axis.clone(),
        statistics,
        cache_key: format!("summary:{}:{axis}", summary.source),
        line: summary.line,
    })
}

pub fn integration_info(
    binding: &FastBinding,
    bindings: &[TypedBinding],
) -> Option<IntegrationInfo> {
    let source = integrate_source(&binding.expression)?;
    let over_axis = integrate_axis(&binding.expression).unwrap_or_else(|| "Time".to_owned());
    let source_binding = bindings.iter().find(|candidate| candidate.name == source)?;
    let (_, input_quantity) = time_series_quantity(&source_binding.semantic_type.quantity_kind)
        .or_else(|| {
            runtime_materialized_time_series(&source_binding.semantic_type.quantity_kind)
        })?;

    Some(IntegrationInfo {
        binding: binding.name.clone(),
        source,
        input_quantity,
        over_axis,
        result_quantity: "Energy".to_owned(),
        line: binding.line,
    })
}

pub fn heat_rate_sum_diagnostic(
    binding: &FastBinding,
    bindings: &[TypedBinding],
) -> Option<Diagnostic> {
    let source = sum_source(&binding.expression)?;
    let source_binding = bindings.iter().find(|candidate| candidate.name == source)?;
    let (_, input_quantity) = time_series_quantity(&source_binding.semantic_type.quantity_kind)?;
    if input_quantity != "HeatRate" || !binding.expression.contains("Time") {
        return None;
    }

    let expression = binding.expression.as_str();
    let trimmed = expression.trim_start();
    let start = expression.len() - trimmed.len();
    let sum_span = SourceSpan::new_in_source(
        binding.expression_span.source_id,
        binding.expression_span.start + start,
        binding.expression_span.start + start + "sum".len(),
        binding.expression_span.line,
        binding.expression_span.column + start,
    );

    Some(
        Diagnostic::warning(
            "W-STATS-SUM-001",
            binding.line,
            "Summing HeatRate over Time does not produce Energy.",
            Some("Use `integrate(<heat_rate>, over=Time)` to compute Energy."),
        )
        .with_source_span(sum_span),
    )
}

pub fn time_series_quantity(quantity_kind: &str) -> Option<(String, String)> {
    let rest = quantity_kind.strip_prefix("TimeSeries[")?;
    let (axis, after_axis) = rest.split_once(']')?;
    let quantity = after_axis.trim().strip_prefix("of ")?;
    Some((axis.trim().to_owned(), quantity.trim().to_owned()))
}

pub fn time_series_type(axis: &str, quantity_kind: &str) -> String {
    format!("TimeSeries[{axis}] of {quantity_kind}")
}

fn runtime_materialized_time_series(quantity_kind: &str) -> Option<(String, String)> {
    matches!(
        quantity_kind,
        "TimeSeriesAlignmentResult" | "TimeSeriesFillResult"
    )
    .then(|| ("Time".to_owned(), "runtime-resolved".to_owned()))
}

fn integrate_source(expression: &str) -> Option<String> {
    let rest = expression.trim().strip_prefix("integrate(")?;
    let source = rest
        .split([',', ')'])
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(source.to_owned())
}

fn integrate_axis(expression: &str) -> Option<String> {
    let after_over = expression.split_once("over=")?.1;
    Some(
        after_over
            .split([',', ')'])
            .next()
            .unwrap_or("Time")
            .trim()
            .to_owned(),
    )
}

fn sum_source(expression: &str) -> Option<String> {
    let rest = expression.trim().strip_prefix("sum(")?;
    let source = rest
        .split([',', ')'])
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(source.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{is_percentile_statistic, parse_percentile_fraction};

    #[test]
    fn percentile_statistics_use_one_bounded_integer_contract() {
        for (name, expected) in [
            ("p1", 0.01),
            ("p05", 0.05),
            ("p50", 0.5),
            ("p100", 1.0),
            ("p00095", 0.95),
        ] {
            assert_eq!(parse_percentile_fraction(name), Some(expected), "{name}");
            assert!(is_percentile_statistic(name), "{name}");
        }
        for name in [
            "p",
            "p0",
            "p101",
            "p95.5",
            "p+95",
            "percentile95",
            "p999999999999999999999999999999",
        ] {
            assert_eq!(parse_percentile_fraction(name), None, "{name}");
            assert!(!is_percentile_statistic(name), "{name}");
        }
    }
}
