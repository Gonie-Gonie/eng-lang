use crate::ast::{FastBinding, SummaryDecl};
use crate::semantic::TypedBinding;
use crate::Diagnostic;

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

    Some(Diagnostic::warning(
        "W-STATS-SUM-001",
        binding.line,
        "Summing HeatRate over Time does not produce Energy.",
        Some("Use `integrate(<heat_rate>, over=Time)` to compute Energy."),
    ))
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
