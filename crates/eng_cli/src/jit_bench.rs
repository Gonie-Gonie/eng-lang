use eng_compiler::CheckReport;
use serde_json::{json, Value};

pub(crate) struct BenchRun {
    pub(crate) iteration: usize,
    pub(crate) elapsed_ms: f64,
    pub(crate) result_path: String,
}

pub(crate) fn jit_bench_json(
    source_path: &str,
    iterations: usize,
    report: &CheckReport,
    plan: &eng_jit::NumericKernelPlan,
    interpreter_runs: &[BenchRun],
) -> String {
    let elapsed = interpreter_runs
        .iter()
        .map(|run| run.elapsed_ms)
        .collect::<Vec<_>>();
    let total_ms = elapsed.iter().sum::<f64>();
    let min_ms = elapsed.iter().copied().reduce(f64::min).unwrap_or_default();
    let max_ms = elapsed.iter().copied().reduce(f64::max).unwrap_or_default();
    let average_ms = if elapsed.is_empty() {
        0.0
    } else {
        total_ms / elapsed.len() as f64
    };

    json!({
        "format": "eng-jit-bench-v1",
        "source_path": source_path,
        "iterations_requested": iterations,
        "comparison_policy": "no-speedup-claim",
        "kernel_plan": eng_jit::plan_json(plan),
        "benchmark_targets": jit_benchmark_targets(report, plan),
        "kernel_executor_samples": jit_kernel_executor_samples(report, plan),
        "interpreter": {
            "status": "measured",
            "runs": interpreter_runs.iter().map(|run| {
                json!({
                    "iteration": run.iteration,
                    "elapsed_ms": rounded_ms(run.elapsed_ms),
                    "result_path": run.result_path,
                })
            }).collect::<Vec<_>>(),
            "summary": {
                "average_ms": rounded_ms(average_ms),
                "min_ms": rounded_ms(min_ms),
                "max_ms": rounded_ms(max_ms),
                "total_ms": rounded_ms(total_ms),
            },
        },
        "jit": {
            "status": "not_available",
            "backend": plan.backend,
            "runs": [],
            "summary": null,
        },
        "notes": [
            "Interpreter timings are local smoke measurements.",
            "JIT timings are intentionally absent until a native backend exists.",
            "Do not use this artifact as a speedup claim."
        ],
    })
    .to_string()
}

fn jit_kernel_executor_samples(
    report: &CheckReport,
    plan: &eng_jit::NumericKernelPlan,
) -> Vec<serde_json::Value> {
    plan.candidates
        .iter()
        .filter(|candidate| candidate.lowering_status == "lowerable_to_numeric_kernel_plan")
        .filter_map(|candidate| jit_kernel_executor_sample(report, candidate))
        .collect()
}

fn jit_kernel_executor_sample(
    report: &CheckReport,
    candidate: &eng_jit::KernelCandidate,
) -> Option<serde_json::Value> {
    match candidate.kind.as_str() {
        "system_residual" => {
            let kernel = system_residual_kernel_for_candidate(report, candidate)?;
            Some(jit_system_residual_kernel_sample(candidate, &kernel))
        }
        "system_residual_jacobian" => {
            let kernel = system_residual_kernel_for_candidate(report, candidate)?;
            Some(jit_system_jacobian_kernel_sample(candidate, &kernel))
        }
        "system_newton_step" => {
            let kernel = system_residual_kernel_for_candidate(report, candidate)?;
            Some(jit_system_newton_step_kernel_sample(candidate, &kernel))
        }
        "component_residual_jacobian" => {
            let assembly = component_assembly_for_kernel_candidate(report, candidate)?;
            let ir = eng_jit::component_residual_ir_from_assembly(assembly)?;
            Some(jit_jacobian_kernel_sample(candidate, &ir))
        }
        "component_newton_step" => {
            let assembly = component_assembly_for_kernel_candidate(report, candidate)?;
            let ir = eng_jit::component_residual_ir_from_assembly(assembly)?;
            Some(jit_newton_step_kernel_sample(candidate, &ir))
        }
        "state_space_solver_step" => {
            let ir = eng_jit::state_space_rhs_ir_for_system(report, candidate.name.as_str())?;
            Some(jit_solver_step_kernel_sample(candidate, &ir))
        }
        _ => {
            let ir = jit_kernel_ir_for_candidate(report, candidate)?;
            Some(jit_interpreter_kernel_sample(candidate, &ir))
        }
    }
}

fn jit_kernel_ir_for_candidate(
    report: &CheckReport,
    candidate: &eng_jit::KernelCandidate,
) -> Option<eng_jit::KernelIr> {
    match candidate.kind.as_str() {
        "timeseries_arithmetic" => {
            eng_jit::timeseries_arithmetic_ir_for_binding(report, candidate.name.as_str())
        }
        "timeseries_integrate" => {
            eng_jit::timeseries_integrate_ir_for_binding(report, candidate.name.as_str(), 300.0)
        }
        "statistics_fusion" => {
            eng_jit::timeseries_statistics_ir_for_source(report, candidate.source.as_str(), 300.0)
        }
        "component_residual_graph" => component_assembly_for_kernel_candidate(report, candidate)
            .and_then(eng_jit::component_residual_ir_from_assembly),
        "system_residual" => {
            system_residual_kernel_for_candidate(report, candidate).map(|kernel| kernel.ir)
        }
        "state_space_rhs" => {
            eng_jit::state_space_rhs_ir_for_system(report, candidate.name.as_str())
        }
        _ => None,
    }
}

fn jit_interpreter_kernel_sample(
    candidate: &eng_jit::KernelCandidate,
    ir: &eng_jit::KernelIr,
) -> serde_json::Value {
    let input = sample_kernel_input(ir);
    match eng_jit::execute_interpreter_kernel(ir, &input) {
        Ok(output) => json!({
            "candidate": format!("{}:{}", candidate.kind, candidate.name),
            "kind": candidate.kind,
            "status": "executed",
            "backend": output.backend,
            "fallback_reason": output.fallback_reason,
            "series_input_count": ir.input_count,
            "scalar_input_count": ir.scalar_input_count,
            "output_count": output.outputs.len(),
            "outputs": jit_kernel_output_summary(&output.outputs),
        }),
        Err(failure) => json!({
            "candidate": format!("{}:{}", candidate.kind, candidate.name),
            "kind": candidate.kind,
            "status": "failed",
            "failure_code": failure.code,
            "failure_message": failure.message,
        }),
    }
}

fn jit_system_residual_kernel_sample(
    candidate: &eng_jit::KernelCandidate,
    kernel: &eng_jit::SystemResidualKernel,
) -> serde_json::Value {
    let mut sample = jit_interpreter_kernel_sample(candidate, &kernel.ir);
    if let Some(object) = sample.as_object_mut() {
        object.insert(
            "scalar_inputs".to_owned(),
            json!(kernel.scalar_inputs.clone()),
        );
        object.insert(
            "residual_outputs".to_owned(),
            json!(kernel.residual_outputs.clone()),
        );
        object.insert(
            "derivative_inputs".to_owned(),
            json!(kernel.derivative_inputs.clone()),
        );
    }
    sample
}

fn jit_system_jacobian_kernel_sample(
    candidate: &eng_jit::KernelCandidate,
    kernel: &eng_jit::SystemResidualKernel,
) -> serde_json::Value {
    let values = sample_scalar_values(kernel.ir.scalar_input_count);
    let active_inputs = kernel
        .unknown_input_indices
        .iter()
        .filter_map(|index| kernel.scalar_inputs.get(*index))
        .cloned()
        .collect::<Vec<_>>();
    match eng_jit::execute_partial_finite_difference_jacobian_kernel(
        &kernel.ir,
        &values,
        &kernel.unknown_input_indices,
        1e-6,
    ) {
        Ok(output) => json!({
            "candidate": format!("{}:{}", candidate.kind, candidate.name),
            "kind": candidate.kind,
            "status": "executed",
            "backend": output.backend,
            "fallback_reason": output.fallback_reason,
            "scalar_inputs": kernel.scalar_inputs,
            "active_inputs": active_inputs,
            "residual_outputs": kernel.residual_outputs,
            "rows": output.values.len(),
            "columns": output.values.first().map(Vec::len).unwrap_or_default(),
        }),
        Err(failure) => json!({
            "candidate": format!("{}:{}", candidate.kind, candidate.name),
            "kind": candidate.kind,
            "status": "failed",
            "failure_code": failure.code,
            "failure_message": failure.message,
        }),
    }
}

fn jit_system_newton_step_kernel_sample(
    candidate: &eng_jit::KernelCandidate,
    kernel: &eng_jit::SystemResidualKernel,
) -> serde_json::Value {
    let values = sample_scalar_values(kernel.ir.scalar_input_count);
    let residuals = match eng_jit::execute_interpreter_kernel(
        &kernel.ir,
        &eng_jit::KernelExecutionInput {
            series_inputs: Vec::new(),
            scalar_inputs: values.clone(),
        },
    ) {
        Ok(output) => output
            .outputs
            .into_iter()
            .filter_map(|value| match value {
                eng_jit::KernelOutputValue::Scalar(value) => Some(value),
                eng_jit::KernelOutputValue::Series(_) => None,
            })
            .collect::<Vec<_>>(),
        Err(failure) => {
            return json!({
                "candidate": format!("{}:{}", candidate.kind, candidate.name),
                "kind": candidate.kind,
                "status": "failed",
                "failure_code": failure.code,
                "failure_message": failure.message,
            });
        }
    };
    let jacobian = match eng_jit::execute_partial_finite_difference_jacobian_kernel(
        &kernel.ir,
        &values,
        &kernel.unknown_input_indices,
        1e-6,
    ) {
        Ok(output) => output.values,
        Err(failure) => {
            return json!({
                "candidate": format!("{}:{}", candidate.kind, candidate.name),
                "kind": candidate.kind,
                "status": "failed",
                "failure_code": failure.code,
                "failure_message": failure.message,
            });
        }
    };
    match eng_jit::execute_newton_step_kernel(&jacobian, &residuals, 1e-9) {
        Ok(output) => json!({
            "candidate": format!("{}:{}", candidate.kind, candidate.name),
            "kind": candidate.kind,
            "status": "executed",
            "backend": output.backend,
            "fallback_reason": output.fallback_reason,
            "scalar_inputs": kernel.scalar_inputs,
            "active_input_indices": kernel.unknown_input_indices,
            "residual_outputs": kernel.residual_outputs,
            "step_count": output.step.len(),
            "residual_norm": output.residual_norm,
        }),
        Err(failure) => json!({
            "candidate": format!("{}:{}", candidate.kind, candidate.name),
            "kind": candidate.kind,
            "status": "failed",
            "failure_code": failure.code,
            "failure_message": failure.message,
        }),
    }
}

fn jit_jacobian_kernel_sample(
    candidate: &eng_jit::KernelCandidate,
    ir: &eng_jit::KernelIr,
) -> serde_json::Value {
    let values = sample_scalar_values(ir.scalar_input_count);
    match eng_jit::execute_finite_difference_jacobian_kernel(ir, &values, 1e-6) {
        Ok(output) => json!({
            "candidate": format!("{}:{}", candidate.kind, candidate.name),
            "kind": candidate.kind,
            "status": "executed",
            "backend": output.backend,
            "fallback_reason": output.fallback_reason,
            "rows": output.values.len(),
            "columns": output.values.first().map(Vec::len).unwrap_or_default(),
        }),
        Err(failure) => json!({
            "candidate": format!("{}:{}", candidate.kind, candidate.name),
            "kind": candidate.kind,
            "status": "failed",
            "failure_code": failure.code,
            "failure_message": failure.message,
        }),
    }
}

fn jit_newton_step_kernel_sample(
    candidate: &eng_jit::KernelCandidate,
    ir: &eng_jit::KernelIr,
) -> serde_json::Value {
    let values = sample_scalar_values(ir.scalar_input_count);
    let residuals = match eng_jit::execute_interpreter_kernel(
        ir,
        &eng_jit::KernelExecutionInput {
            series_inputs: Vec::new(),
            scalar_inputs: values.clone(),
        },
    ) {
        Ok(output) => output
            .outputs
            .into_iter()
            .filter_map(|value| match value {
                eng_jit::KernelOutputValue::Scalar(value) => Some(value),
                eng_jit::KernelOutputValue::Series(_) => None,
            })
            .collect::<Vec<_>>(),
        Err(failure) => {
            return json!({
                "candidate": format!("{}:{}", candidate.kind, candidate.name),
                "kind": candidate.kind,
                "status": "failed",
                "failure_code": failure.code,
                "failure_message": failure.message,
            });
        }
    };
    let jacobian = match eng_jit::execute_finite_difference_jacobian_kernel(ir, &values, 1e-6) {
        Ok(output) => output.values,
        Err(failure) => {
            return json!({
                "candidate": format!("{}:{}", candidate.kind, candidate.name),
                "kind": candidate.kind,
                "status": "failed",
                "failure_code": failure.code,
                "failure_message": failure.message,
            });
        }
    };
    match eng_jit::execute_newton_step_kernel(&jacobian, &residuals, 1e-9) {
        Ok(output) => json!({
            "candidate": format!("{}:{}", candidate.kind, candidate.name),
            "kind": candidate.kind,
            "status": "executed",
            "backend": output.backend,
            "fallback_reason": output.fallback_reason,
            "step_count": output.step.len(),
            "residual_norm": output.residual_norm,
        }),
        Err(failure) => json!({
            "candidate": format!("{}:{}", candidate.kind, candidate.name),
            "kind": candidate.kind,
            "status": "failed",
            "failure_code": failure.code,
            "failure_message": failure.message,
        }),
    }
}

fn jit_solver_step_kernel_sample(
    candidate: &eng_jit::KernelCandidate,
    ir: &eng_jit::KernelIr,
) -> serde_json::Value {
    let scalar_inputs = sample_scalar_values(ir.scalar_input_count);
    let state_count = ir.output_count;
    if scalar_inputs.len() < state_count {
        return json!({
            "candidate": format!("{}:{}", candidate.kind, candidate.name),
            "kind": candidate.kind,
            "status": "failed",
            "failure_code": "E-KERNEL-SOLVER-STEP-LAYOUT",
            "failure_message": "solver step sample requires at least one state input per RHS output",
        });
    }
    let state = scalar_inputs[..state_count].to_vec();
    let inputs = scalar_inputs[state_count..].to_vec();
    match eng_jit::execute_explicit_euler_step_kernel(ir, &state, &inputs, 60.0) {
        Ok(output) => json!({
            "candidate": format!("{}:{}", candidate.kind, candidate.name),
            "kind": candidate.kind,
            "status": "executed",
            "backend": output.backend,
            "fallback_reason": output.fallback_reason,
            "state_count": output.state.len(),
            "derivative_count": output.derivatives.len(),
        }),
        Err(failure) => json!({
            "candidate": format!("{}:{}", candidate.kind, candidate.name),
            "kind": candidate.kind,
            "status": "failed",
            "failure_code": failure.code,
            "failure_message": failure.message,
        }),
    }
}

fn component_assembly_for_kernel_candidate<'a>(
    report: &'a CheckReport,
    candidate: &eng_jit::KernelCandidate,
) -> Option<&'a eng_compiler::ComponentAssemblyInfo> {
    report
        .semantic_program
        .component_assemblies
        .iter()
        .find(|assembly| candidate.name.starts_with(&format!("{}:", assembly.name)))
}

fn system_residual_kernel_for_candidate(
    report: &CheckReport,
    candidate: &eng_jit::KernelCandidate,
) -> Option<eng_jit::SystemResidualKernel> {
    let system = report.semantic_program.systems.iter().find(|system| {
        candidate.name == system.name || candidate.name.starts_with(&format!("{}:", system.name))
    })?;
    eng_jit::system_residual_kernel_for_system(report, &system.name)
}

fn sample_kernel_input(ir: &eng_jit::KernelIr) -> eng_jit::KernelExecutionInput {
    eng_jit::KernelExecutionInput {
        series_inputs: (0..ir.input_count)
            .map(|index| {
                let base = index as f64 + 1.0;
                vec![base, base + 1.0, base + 2.0, base + 3.0]
            })
            .collect(),
        scalar_inputs: sample_scalar_values(ir.scalar_input_count),
    }
}

fn sample_scalar_values(count: usize) -> Vec<f64> {
    (0..count).map(|index| index as f64 + 1.0).collect()
}

fn jit_kernel_output_summary(outputs: &[eng_jit::KernelOutputValue]) -> Vec<serde_json::Value> {
    outputs
        .iter()
        .map(|output| match output {
            eng_jit::KernelOutputValue::Series(values) => json!({
                "kind": "series",
                "len": values.len(),
                "first": values.first().copied(),
                "last": values.last().copied(),
            }),
            eng_jit::KernelOutputValue::Scalar(value) => json!({
                "kind": "scalar",
                "value": value,
            }),
        })
        .collect()
}

pub(crate) fn jit_bench_has_target(
    bench_json: &str,
    name: &str,
    status: &str,
    candidate_fragment: Option<&str>,
) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(bench_json) else {
        return false;
    };
    value["benchmark_targets"]
        .as_array()
        .is_some_and(|targets| {
            targets.iter().any(|target| {
                target["name"] == name
                    && target["status"] == status
                    && candidate_fragment.is_none_or(|fragment| {
                        target["candidates"].as_array().is_some_and(|candidates| {
                            candidates
                                .iter()
                                .filter_map(Value::as_str)
                                .any(|candidate| candidate.contains(fragment))
                        })
                    })
            })
        })
}

pub(crate) fn jit_bench_has_executor_sample(
    bench_json: &str,
    candidate: &str,
    status: &str,
) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(bench_json) else {
        return false;
    };
    value["kernel_executor_samples"]
        .as_array()
        .is_some_and(|samples| {
            samples.iter().any(|sample| {
                sample["candidate"] == candidate
                    && sample["status"] == status
                    && sample["backend"] == eng_jit::INTERPRETER_FALLBACK_BACKEND
            })
        })
}

fn jit_benchmark_targets(
    report: &CheckReport,
    plan: &eng_jit::NumericKernelPlan,
) -> Vec<serde_json::Value> {
    let state_space_items = state_space_target_items(report);
    let state_space_candidates =
        candidates_by_kind(plan, &["state_space_rhs", "state_space_solver_step"]);
    vec![
        benchmark_target(
            "csv_heat_rate_workflow",
            if has_candidate(plan, "timeseries_arithmetic")
                && has_candidate(plan, "timeseries_integrate")
            {
                "covered_by_current_source"
            } else {
                "not_observed_for_source"
            },
            candidates_by_kind(
                plan,
                &[
                    "timeseries_arithmetic",
                    "statistics_fusion",
                    "timeseries_integrate",
                ],
            ),
            "covers checked TimeSeries arithmetic/statistics/integration candidates when present",
        ),
        benchmark_target(
            "multi_statistics_fusion",
            if has_candidate(plan, "statistics_fusion") {
                "covered_by_current_source"
            } else {
                "not_observed_for_source"
            },
            candidates_by_kind(plan, &["statistics_fusion"]),
            "tracks summarize-by statistics fusion candidates",
        ),
        benchmark_target(
            "residual_evaluation",
            if has_candidate(plan, "component_residual_graph")
                || has_candidate(plan, "system_residual")
            {
                "covered_by_current_source"
            } else {
                "not_observed_for_source"
            },
            candidates_by_kind(
                plan,
                &[
                    "component_residual_graph",
                    "component_residual_jacobian",
                    "system_residual",
                    "system_residual_jacobian",
                ],
            ),
            "tracks executable component and source-system residual evaluator candidates",
        ),
        benchmark_target(
            "component_graph_solver_small_case",
            if has_candidate(plan, "component_residual_graph") {
                "covered_by_current_source"
            } else {
                "not_observed_for_source"
            },
            candidates_by_kind(
                plan,
                &[
                    "component_residual_graph",
                    "component_residual_jacobian",
                    "component_newton_step",
                ],
            ),
            "tracks small component residual graph candidates, not production multi-domain solving",
        ),
        benchmark_target(
            "source_system_solver_small_case",
            if has_candidate(plan, "system_newton_step") {
                "covered_by_current_source"
            } else {
                "not_observed_for_source"
            },
            candidates_by_kind(
                plan,
                &[
                    "system_residual",
                    "system_residual_jacobian",
                    "system_newton_step",
                ],
            ),
            "tracks executable source-system residual, partial Jacobian, and single Newton-step kernels",
        ),
        benchmark_target(
            "state_space_simulation",
            if !state_space_candidates.is_empty() {
                "covered_by_current_source"
            } else if state_space_items.is_empty() {
                "not_observed_for_source"
            } else {
                "metadata_observed"
            },
            if state_space_candidates.is_empty() {
                state_space_items
            } else {
                state_space_candidates
            },
            "tracks continuous state-space RHS and explicit-Euler solver-step kernel coverage; simulation still runs on the normal runtime path",
        ),
    ]
}

fn benchmark_target(
    name: &str,
    status: &str,
    candidates: Vec<String>,
    note: &str,
) -> serde_json::Value {
    json!({
        "name": name,
        "status": status,
        "candidate_count": candidates.len(),
        "candidates": candidates,
        "note": note,
    })
}

fn has_candidate(plan: &eng_jit::NumericKernelPlan, kind: &str) -> bool {
    plan.candidates
        .iter()
        .any(|candidate| candidate.kind == kind)
}

fn candidates_by_kind(plan: &eng_jit::NumericKernelPlan, kinds: &[&str]) -> Vec<String> {
    plan.candidates
        .iter()
        .filter(|candidate| kinds.contains(&candidate.kind.as_str()))
        .map(|candidate| format!("{}:{}", candidate.kind, candidate.name))
        .collect()
}

fn state_space_target_items(report: &CheckReport) -> Vec<String> {
    let mut items = report
        .semantic_program
        .state_space_vectors
        .iter()
        .map(|vector| format!("state_space_vector:{}:{}", vector.system, vector.name))
        .collect::<Vec<_>>();
    items.extend(
        report
            .semantic_program
            .linear_operators
            .iter()
            .map(|operator| format!("linear_operator:{}:{}", operator.system, operator.name)),
    );
    items
}

fn rounded_ms(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}
