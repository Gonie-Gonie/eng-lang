use std::collections::BTreeSet;

use eng_compiler::CheckReport;
use serde_json::{json, Value};

pub const KERNEL_PLAN_FORMAT: &str = "eng-kernel-plan-v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelCandidate {
    pub name: String,
    pub kind: String,
    pub line: usize,
    pub source: String,
    pub reason: String,
    pub lowering_status: String,
    pub operations: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumericKernelPlan {
    pub format: String,
    pub backend: String,
    pub candidates: Vec<KernelCandidate>,
}

pub fn plan_for_report(report: &CheckReport) -> NumericKernelPlan {
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();

    for stats in &report.semantic_program.stats_infos {
        push_candidate(
            &mut candidates,
            &mut seen,
            KernelCandidate {
                name: format!("summary:{}", stats.source),
                kind: "statistics_fusion".to_owned(),
                line: stats.line,
                source: stats.source.clone(),
                reason: format!(
                    "{} statistics over {} can share one TimeSeries scan",
                    stats.statistics.len(),
                    stats.axis
                ),
                lowering_status: "lowerable_to_numeric_kernel_plan".to_owned(),
                operations: stats
                    .statistics
                    .iter()
                    .map(|statistic| format!("stat:{statistic}"))
                    .collect(),
            },
        );
    }

    for integration in &report.semantic_program.integrations {
        push_candidate(
            &mut candidates,
            &mut seen,
            KernelCandidate {
                name: integration.binding.clone(),
                kind: "timeseries_integrate".to_owned(),
                line: integration.line,
                source: integration.source.clone(),
                reason: format!(
                    "{} over {} lowers to a trapezoid-style numeric kernel",
                    integration.input_quantity, integration.over_axis
                ),
                lowering_status: "lowerable_to_numeric_kernel_plan".to_owned(),
                operations: vec![
                    format!("load_timeseries:{}", integration.source),
                    format!("integrate_over:{}", integration.over_axis),
                    format!("store:{}", integration.binding),
                ],
            },
        );
    }

    for hover in &report.semantic_program.hover_hints {
        let Some(expression) = &hover.expression else {
            continue;
        };
        if !hover.quantity_kind.starts_with("TimeSeries[") {
            continue;
        }
        if expression.contains('*') || expression.contains('+') || expression.contains('-') {
            push_candidate(
                &mut candidates,
                &mut seen,
                KernelCandidate {
                    name: hover.name.clone(),
                    kind: "timeseries_arithmetic".to_owned(),
                    line: hover.line,
                    source: expression.clone(),
                    reason: format!(
                        "{} expression can be lowered to element-wise numeric operations",
                        hover.quantity_kind
                    ),
                    lowering_status: "lowerable_to_numeric_kernel_plan".to_owned(),
                    operations: elementwise_operations(expression),
                },
            );
        }
    }

    for system in &report.semantic_program.systems {
        for residual in &system.residuals {
            push_candidate(
                &mut candidates,
                &mut seen,
                KernelCandidate {
                    name: format!("{}:{}", system.name, residual.name),
                    kind: "system_residual".to_owned(),
                    line: residual.line,
                    source: residual.expression.clone(),
                    reason: "system residual can feed a future RHS/Jacobian kernel".to_owned(),
                    lowering_status: "interface_only".to_owned(),
                    operations: vec![
                        format!("normalize_residual:{}", residual.name),
                        "defer_rhs_codegen".to_owned(),
                    ],
                },
            );
        }
    }

    candidates.sort_by(|left, right| {
        left.line
            .cmp(&right.line)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.name.cmp(&right.name))
    });

    NumericKernelPlan {
        format: KERNEL_PLAN_FORMAT.to_owned(),
        backend: "interpreter-fallback".to_owned(),
        candidates,
    }
}

pub fn plan_json(plan: &NumericKernelPlan) -> Value {
    json!({
        "format": plan.format,
        "backend": plan.backend,
        "candidate_count": plan.candidates.len(),
        "candidates": plan.candidates.iter().map(candidate_json).collect::<Vec<_>>(),
    })
}

pub fn plan_json_string(plan: &NumericKernelPlan) -> String {
    plan_json(plan).to_string()
}

fn candidate_json(candidate: &KernelCandidate) -> Value {
    json!({
        "name": candidate.name,
        "kind": candidate.kind,
        "line": candidate.line,
        "source": candidate.source,
        "reason": candidate.reason,
        "lowering_status": candidate.lowering_status,
        "operations": candidate.operations,
    })
}

fn push_candidate(
    candidates: &mut Vec<KernelCandidate>,
    seen: &mut BTreeSet<String>,
    candidate: KernelCandidate,
) {
    let key = format!("{}:{}:{}", candidate.kind, candidate.name, candidate.line);
    if seen.insert(key) {
        candidates.push(candidate);
    }
}

fn elementwise_operations(expression: &str) -> Vec<String> {
    let mut operations = Vec::new();
    if expression.contains('*') {
        operations.push("elementwise_mul".to_owned());
    }
    if expression.contains('/') {
        operations.push("elementwise_div".to_owned());
    }
    if expression.contains('+') {
        operations.push("elementwise_add".to_owned());
    }
    if expression.contains('-') {
        operations.push("elementwise_sub".to_owned());
    }
    if operations.is_empty() {
        operations.push("elementwise_eval".to_owned());
    }
    operations
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use eng_compiler::{check_file, CheckOptions};

    use super::*;

    #[test]
    fn detects_official_csv_hot_kernels() {
        let report = check_file(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/official/01_csv_plot/main.eng"),
            &CheckOptions::default(),
        )
        .expect("official CSV example should check");
        let plan = plan_for_report(&report);

        assert!(plan.candidates.iter().any(
            |candidate| candidate.kind == "timeseries_arithmetic" && candidate.name == "Q_coil"
        ));
        assert!(plan.candidates.iter().any(
            |candidate| candidate.kind == "timeseries_integrate" && candidate.name == "E_coil"
        ));
        assert!(plan
            .candidates
            .iter()
            .any(|candidate| candidate.kind == "statistics_fusion"
                && candidate.name == "summary:Q_coil"));

        let json = plan_json(&plan);
        assert_eq!(json["format"], KERNEL_PLAN_FORMAT);
        assert!(json["candidate_count"].as_u64().unwrap() >= 3);
    }
}
