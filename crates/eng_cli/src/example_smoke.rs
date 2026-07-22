use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use eng_compiler::{
    check_file, check_source, format_source, review_json, ArgOverride, CheckOptions,
};
use eng_runtime::{build_standalone, run_file, BuildOptions, ExecutionProfile, RunOptions};
use serde_json::Value;

use crate::jit_bench::{
    jit_bench_has_executor_sample, jit_bench_has_target, jit_bench_json, BenchRun,
};
use crate::print_diagnostics;

pub(crate) fn command_test(_args: Vec<String>) -> ExitCode {
    if let Err(error) = ensure_required_workflow_main_sources() {
        eprintln!("{error}");
        return ExitCode::from(2);
    }
    let workflow_examples = match workflow_main_sources() {
        Ok(examples) => examples,
        Err(error) => {
            eprintln!("failed to enumerate workflow examples: {error}");
            return ExitCode::from(2);
        }
    };

    let example_groups: [(&str, &[&str]); 4] = [
        (
            "official",
            &[
                "examples/official/01_csv_plot/main.eng",
                "examples/official/07_functions_imports/main.eng",
                "examples/official/08_print_export_summary/main.eng",
                "examples/official/09_command_where_with/main.eng",
                "examples/official/10_path_policy/main.eng",
                "examples/official/11_read_only_io/main.eng",
                "examples/official/12_write_output_manifest/main.eng",
                "examples/official/13_file_operations/main.eng",
                "examples/official/14_run_log/main.eng",
                "examples/official/15_process_result/main.eng",
                "examples/official/16_test_assert_golden/main.eng",
                "examples/official/19_class_object/main.eng",
            ],
        ),
        (
            "advanced solver smoke",
            &[
                "examples/advanced_solver/20_multi_state_thermal/main.eng",
                "examples/advanced_solver/21_state_space_discrete/main.eng",
                "examples/advanced_solver/22_state_space_continuous/main.eng",
                "examples/advanced_solver/23_thermal_component_assembly/main.eng",
                "examples/advanced_solver/24_linear_algebraic_thermal_node/main.eng",
                "examples/advanced_solver/25_fixed_point_loop/main.eng",
                "examples/advanced_solver/26_dynamic_component_room/main.eng",
                "examples/advanced_solver/27_nonlinear_algebraic/main.eng",
                "examples/advanced_solver/28_small_dae/main.eng",
                "examples/advanced_solver/29_delay_component_solver/main.eng",
                "examples/advanced_solver/30_predictor_component_solver/main.eng",
                "examples/advanced_solver/31_external_behavior_solver/main.eng",
                "examples/advanced_solver/32_small_thermal_fluid_loop/main.eng",
                "examples/advanced_solver/33_unit_parameterized_wall/main.eng",
                "examples/advanced_solver/34_three_state_source_ode/main.eng",
            ],
        ),
        (
            "internal",
            &[
                "examples/internal/02_simple_system/main.eng",
                "examples/internal/03_integrated_hvac/main.eng",
                "examples/internal/04_uncertainty_core/main.eng",
                "examples/internal/05_data_driven_modeling/main.eng",
                "examples/internal/06_domain_port/main.eng",
                "examples/internal/17_measured_vs_simulated/main.eng",
                "examples/internal/18_state_space_metadata/main.eng",
                "examples/internal/20_multi_state_thermal/main.eng",
                "examples/internal/21_unsupported_system_shape/main.eng",
                "examples/internal/21_thermal_component_assembly/main.eng",
                "examples/internal/22_multi_domain_boundary_solve/main.eng",
                "examples/internal/26_state_space_discrete/main.eng",
                "examples/internal/27_adaptive_heun_thermal/main.eng",
                "examples/internal/28_adaptive_state_space/main.eng",
            ],
        ),
        (
            "compatibility regression",
            &[
                "examples/compat/01_units/main.eng",
                "examples/compat/02_csv_plot/main.eng",
                "examples/compat/04_plotting/main.eng",
                "examples/compat/06_simple_system/main.eng",
            ],
        ),
    ];

    for (group, examples) in example_groups {
        for example in examples {
            let report = match check_file(example, &CheckOptions::default()) {
                Ok(report) => report,
                Err(error) => {
                    eprintln!("{example}: {error}");
                    return ExitCode::from(1);
                }
            };
            if report.has_errors() {
                print_diagnostics(&report);
                return ExitCode::from(2);
            }
            println!("ok: {group} example {example}");
        }
    }
    for example in &workflow_examples {
        let report = match check_file(example, &CheckOptions::default()) {
            Ok(report) => report,
            Err(error) => {
                eprintln!("{}: {error}", example.display());
                return ExitCode::from(1);
            }
        };
        if report.has_errors() {
            print_diagnostics(&report);
            return ExitCode::from(2);
        }
        println!("ok: workflow example {}", example.display());
    }
    if !native_workflow_sources_avoid_external_processes() {
        return ExitCode::from(2);
    }

    if !review_examples_are_formatter_clean() {
        return ExitCode::from(2);
    }
    if !review_cli_smoke() {
        return ExitCode::from(2);
    }

    let jit_report = match check_file(
        "examples/official/01_csv_plot/main.eng",
        &CheckOptions::default(),
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let jit_plan = eng_jit::plan_for_report(&jit_report);
    let jit_plan_json = eng_jit::plan_json(&jit_plan);
    let lowerable_executor_recorded = jit_plan_json["candidates"]
        .as_array()
        .map(|candidates| {
            candidates.iter().any(|candidate| {
                candidate["executor"]["backend"] == eng_jit::INTERPRETER_FALLBACK_BACKEND
                    && candidate["executor"]["status"] == "interpreter_supported"
                    && candidate["executor"]["fallback_reason"]
                        .as_str()
                        .is_some_and(|reason| reason.contains("runtime inputs"))
            })
        })
        .unwrap_or(false);
    let native_preview_plan = eng_jit::plan_for_report_with_options(
        &jit_report,
        &eng_jit::PlanOptions {
            requested_backend: eng_jit::NATIVE_PREVIEW_BACKEND.to_owned(),
        },
    );
    let jit_bench_smoke = jit_bench_json(
        "examples/official/01_csv_plot/main.eng",
        1,
        &jit_report,
        &jit_plan,
        &[BenchRun {
            iteration: 1,
            elapsed_ms: 1.0,
            result_path: "build/jit-bench/iter-000/result/result.engres".to_owned(),
        }],
    );
    let csv_bench_targets_recorded = jit_bench_has_target(
        &jit_bench_smoke,
        "csv_heat_rate_workflow",
        "covered_by_current_source",
        Some("timeseries_integrate:E_coil"),
    ) && jit_bench_has_target(
        &jit_bench_smoke,
        "multi_statistics_fusion",
        "covered_by_current_source",
        Some("statistics_fusion:summary:Q_coil"),
    );
    let csv_executor_samples_recorded =
        jit_bench_has_executor_sample(&jit_bench_smoke, "timeseries_integrate:E_coil", "executed")
            && jit_bench_has_executor_sample(
                &jit_bench_smoke,
                "statistics_fusion:summary:Q_coil",
                "executed",
            );
    if jit_plan.candidates.len() < 3
        || jit_plan.backend_selection.selected != eng_jit::INTERPRETER_FALLBACK_BACKEND
        || jit_plan.backend_selection.status != "selected"
        || !jit_plan
            .candidates
            .iter()
            .any(|candidate| candidate.kind == "timeseries_integrate")
        || !lowerable_executor_recorded
        || !csv_bench_targets_recorded
        || !csv_executor_samples_recorded
        || native_preview_plan.backend_selection.status != "not_available"
        || native_preview_plan.backend_selection.selected != eng_jit::INTERPRETER_FALLBACK_BACKEND
    {
        eprintln!(
            "expected official CSV example to expose kernel candidates, executor fallback metadata, executable CSV/statistics kernel samples, benchmark target coverage, and native backend non-availability"
        );
        return ExitCode::from(2);
    }
    println!(
        "ok: official CSV example produced JIT kernel candidates with executor fallback and benchmark target metadata"
    );

    let state_space_jit_report = match check_file(
        "examples/internal/18_state_space_metadata/main.eng",
        &CheckOptions::default(),
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let state_space_jit_plan = eng_jit::plan_for_report(&state_space_jit_report);
    let state_space_bench_smoke = jit_bench_json(
        "examples/internal/18_state_space_metadata/main.eng",
        1,
        &state_space_jit_report,
        &state_space_jit_plan,
        &[BenchRun {
            iteration: 1,
            elapsed_ms: 1.0,
            result_path: "build/jit-bench/iter-000/result/result.engres".to_owned(),
        }],
    );
    if !jit_bench_has_target(
        &state_space_bench_smoke,
        "state_space_simulation",
        "covered_by_current_source",
        Some("state_space_rhs:ThermalStateSpaceMetadata"),
    ) {
        eprintln!(
            "expected internal state-space example to expose JIT benchmark state-space RHS coverage"
        );
        return ExitCode::from(2);
    }
    println!(
        "ok: examples/internal/18_state_space_metadata/main.eng produced JIT benchmark state-space RHS coverage"
    );

    let source_system_path = "tests/runtime/source_system_newton_solve.eng";
    let source_system_report = match check_file(source_system_path, &CheckOptions::default()) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let source_system_plan = eng_jit::plan_for_report(&source_system_report);
    let source_system_bench_smoke = jit_bench_json(
        source_system_path,
        1,
        &source_system_report,
        &source_system_plan,
        &[BenchRun {
            iteration: 1,
            elapsed_ms: 1.0,
            result_path: "build/jit-bench/iter-000/result/result.engres".to_owned(),
        }],
    );
    let source_system_targets_recorded = jit_bench_has_target(
        &source_system_bench_smoke,
        "residual_evaluation",
        "covered_by_current_source",
        Some("system_residual_jacobian:StaticNonlinearSourceSystem:jacobian"),
    ) && jit_bench_has_target(
        &source_system_bench_smoke,
        "source_system_solver_small_case",
        "covered_by_current_source",
        Some("system_newton_step:StaticNonlinearSourceSystem:newton_step"),
    );
    let source_system_executor_samples_recorded = [
        "system_residual:StaticNonlinearSourceSystem",
        "system_residual_jacobian:StaticNonlinearSourceSystem:jacobian",
        "system_newton_step:StaticNonlinearSourceSystem:newton_step",
    ]
    .iter()
    .all(|candidate| {
        jit_bench_has_executor_sample(&source_system_bench_smoke, candidate, "executed")
    });
    if !source_system_targets_recorded || !source_system_executor_samples_recorded {
        eprintln!(
            "expected source-system fixture to execute residual, partial Jacobian, and Newton-step JIT interpreter kernels"
        );
        return ExitCode::from(2);
    }
    println!(
        "ok: tests/runtime/source_system_newton_solve.eng executed source-system residual, partial Jacobian, and Newton-step kernels"
    );

    let domain_port = match check_file(
        "examples/internal/06_domain_port/main.eng",
        &CheckOptions::default(),
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let domain_review = review_json(&domain_port);
    if !domain_review.contains("\"domain_summary\"")
        || !domain_review.contains("\"component_summary\"")
        || !domain_review.contains("\"connection_summary\"")
        || !domain_review.contains("\"assembly_summary\"")
        || !domain_review.contains("\"connection_equations_generated\"")
        || !domain_review.contains("\"component_residual_graph\"")
        || !domain_review.contains("\"multi_domain_preview\"")
        || !domain_review.contains("\"domain_count\": 3")
        || !domain_review.contains("\"domain_compatible\"")
    {
        eprintln!(
            "expected domain port example to expose domain/component/connection/assembly review metadata"
        );
        return ExitCode::from(2);
    }
    println!("ok: examples/internal/06_domain_port/main.eng produced domain assembly metadata");
    match run_file(
        Path::new("examples/internal/06_domain_port/main.eng"),
        Path::new("build/test-domain-assembly-solver"),
        &RunOptions {
            save_artifacts: true,
            ..RunOptions::default()
        },
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"component_solutions\"")
                || !output
                    .result_json
                    .contains("\"constraint_satisfied_nonunique\"")
                || !output
                    .report_spec_json
                    .contains("\"linear_residual_satisfied_nonunique\"")
                || !output.report_spec_json.contains("\"solver_result\"")
                || !output.report_spec_json.contains("\"residual_norm\"")
                || !output.result_json.contains("\"largest_residuals\"")
                || !output.report_spec_json.contains("\"largest_residuals\"")
                || !output.report_spec_json.contains("\"residual_metadata\"")
                || !output.report_spec_json.contains("\"source_expression\"")
                || !output
                    .report_spec_json
                    .contains("\"expression_unit\": \"kW\"")
                || !output
                    .report_spec_json
                    .contains("\"expression_quantity_kind\": \"HeatRate\"")
                || !output.report_spec_json.contains("\"failure_artifact\"")
                || !output.report_spec_json.contains("\"failure_code\"")
                || !output.report_spec_json.contains("\"failure_reason\"")
                || !output.report_spec_json.contains("\"domain_count\": 3")
                || !output.report_spec_json.contains("\"multi_domain_preview\"")
                || !output
                    .report_spec_json
                    .contains("\"not_production_multi_domain\"")
                || !output.report_html.contains("Connection Constraint Check")
                || !output.report_html.contains("Residual Norm")
                || !output.report_html.contains("E-ASSEMBLY-UNDERDETERMINED")
                || !output.report_html.contains("domain plan")
            {
                eprintln!(
                    "expected domain port run to expose component assembly constraint-check artifacts"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/06_domain_port/main.eng produced component constraint-check artifacts"
            );
        }
        Err(error) => {
            eprintln!("domain assembly constraint check failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("examples/internal/22_component_boundary_solve/main.eng"),
        Path::new("build/test-component-boundary-solve"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"solved_linear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dense_linear_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"name\": \"RoomBoundary.heat.T\"")
                || !output.result_json.contains("\"value\": 22.00000000")
                || !output
                    .result_json
                    .contains("\"name\": \"AmbientBoundary.heat.Q\"")
                || !output.result_json.contains("\"value\": -1.00000000")
                || !output
                    .report_spec_json
                    .contains("\"kind\": \"component_boundary\"")
                || !output.report_spec_json.contains("\"rhs\": \"22 degC\"")
                || !output.report_spec_json.contains("\"rhs\": \"1 kW\"")
                || !output
                    .report_spec_json
                    .contains("\"component_equation_count\": 2")
                || !output.result_json.contains("\"largest_residuals\"")
                || !output.report_spec_json.contains("\"largest_residuals\"")
                || !output.report_html.contains("solved_linear")
                || !output.report_html.contains("component_boundary")
            {
                eprintln!(
                    "expected component boundary fixture to solve a square linear residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/22_component_boundary_solve/main.eng solved component boundary residual graph"
            );
        }
        Err(error) => {
            eprintln!("component boundary solve fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("examples/internal/21_thermal_component_assembly/main.eng"),
        Path::new("build/test-thermal-component-assembly"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"solved_linear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dense_linear_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"name\": \"RoomBoundary.heat.T\"")
                || !output.result_json.contains("\"value\": 22.00000000")
                || !output
                    .result_json
                    .contains("\"name\": \"AmbientBoundary.heat.Q\"")
                || !output.result_json.contains("\"value\": -1.00000000")
                || !output
                    .report_spec_json
                    .contains("\"kind\": \"component_boundary\"")
                || !output.report_spec_json.contains("\"rhs\": \"22 degC\"")
                || !output.report_spec_json.contains("\"rhs\": \"1 kW\"")
                || !output
                    .report_spec_json
                    .contains("\"component_equation_count\": 2")
                || !output.report_spec_json.contains("\"residual_norm\"")
                || !output.result_json.contains("\"largest_residuals\"")
                || !output.report_spec_json.contains("\"largest_residuals\"")
                || !output.report_html.contains("solved_linear")
                || !output.report_html.contains("component_boundary")
            {
                eprintln!(
                    "expected internal thermal component assembly fixture to solve a square linear residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/21_thermal_component_assembly/main.eng solved thermal component assembly residual graph"
            );
        }
        Err(error) => {
            eprintln!("thermal component assembly example failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("examples/advanced_solver/23_thermal_component_assembly/main.eng"),
        Path::new("build/test-official-thermal-component-assembly"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"solved_linear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dense_linear_residual_graph\"")
                || !output.result_json.contains("\"name\": \"room.heat.T\"")
                || !output.result_json.contains("\"value\": 22.00000000")
                || !output.result_json.contains("\"name\": \"ambient.heat.Q\"")
                || !output.result_json.contains("\"value\": -0.00000000")
                || !output
                    .report_spec_json
                    .contains("\"component_equation_count\": 2")
                || !output
                    .report_spec_json
                    .contains("\"kind\": \"component_boundary\"")
                || !output
                    .report_spec_json
                    .contains("\"kind\": \"component_equation\"")
                || !output.report_spec_json.contains("\"rhs\": \"22 degC\"")
                || !output.report_spec_json.contains("\"rhs\": \"0 kW\"")
                || !output.report_spec_json.contains("\"left\": \"room.heat\"")
                || !output
                    .report_spec_json
                    .contains("\"right\": \"ambient.heat\"")
                || !output.report_spec_json.contains("\"residual_norm\"")
                || !output.result_json.contains("\"largest_residuals\"")
                || !output.result_json.contains("\"expression_unit\": \"W\"")
                || !output
                    .result_json
                    .contains("\"expression_quantity_kind\": \"HeatRate\"")
                || !output
                    .report_spec_json
                    .contains("\"expression_unit\": \"W\"")
                || !output
                    .report_spec_json
                    .contains("\"expression_quantity_kind\": \"HeatRate\"")
                || !output.report_html.contains("solved_linear")
                || !output.report_html.contains("component_boundary")
                || !output.report_html.contains("component_equation")
            {
                eprintln!(
                    "expected official thermal component assembly example to solve a system-local instance residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/advanced_solver/23_thermal_component_assembly/main.eng solved system-local component assembly residual graph"
            );
        }
        Err(error) => {
            eprintln!("official thermal component assembly example failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("examples/advanced_solver/24_linear_algebraic_thermal_node/main.eng"),
        Path::new("build/test-official-linear-algebraic-thermal-node"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"solved_linear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dense_linear_residual_graph\"")
                || !output.result_json.contains("\"name\": \"room.heat.T\"")
                || !output.result_json.contains("\"value\": 21.00000000")
                || !output.result_json.contains("\"name\": \"room.heat.Q\"")
                || !output.result_json.contains("\"value\": -2.00000000")
                || !output.result_json.contains("\"name\": \"load.heat.Q\"")
                || !output.result_json.contains("\"value\": 2.00000000")
                || !output
                    .report_spec_json
                    .contains("\"component_equation_count\": 2")
                || !output
                    .report_spec_json
                    .contains("\"kind\": \"component_boundary\"")
                || !output.report_spec_json.contains("\"rhs\": \"21 degC\"")
                || !output.report_spec_json.contains("\"rhs\": \"2 kW\"")
                || !output.report_spec_json.contains("\"left\": \"room.heat\"")
                || !output.report_spec_json.contains("\"right\": \"load.heat\"")
                || !output.report_spec_json.contains("\"residual_norm\"")
                || !output.result_json.contains("\"largest_residuals\"")
                || !output.report_spec_json.contains("\"largest_residuals\"")
                || !output.report_html.contains("solved_linear")
                || !output.report_html.contains("component_boundary")
            {
                eprintln!(
                    "expected official linear algebraic thermal node example to solve a source residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/advanced_solver/24_linear_algebraic_thermal_node/main.eng solved source linear algebraic residual graph"
            );
        }
        Err(error) => {
            eprintln!("official linear algebraic thermal node example failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("examples/advanced_solver/32_small_thermal_fluid_loop/main.eng"),
        Path::new("build/test-official-small-thermal-fluid-loop"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !small_thermal_fluid_solve_artifacts_are_structured(&output) {
                eprintln!(
                    "expected official small thermal/fluid loop to solve a square multi-domain residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/advanced_solver/32_small_thermal_fluid_loop/main.eng solved a constrained Thermal/Fluid residual graph"
            );
        }
        Err(error) => {
            eprintln!("official small thermal/fluid loop example failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("examples/advanced_solver/33_unit_parameterized_wall/main.eng"),
        Path::new("build/test-official-unit-parameterized-wall"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"solved_linear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dense_linear_residual_graph\"")
                || !output.result_json.contains("\"name\": \"wall.inside.Q\"")
                || !output.result_json.contains("\"value\": 5.00000000")
                || !output.result_json.contains("\"name\": \"wall.outside.Q\"")
                || !output.result_json.contains("\"value\": -5.00000000")
                || !output
                    .result_json
                    .contains("wall.UA * (wall.inside.T - wall.outside.T)")
                || !output
                    .report_spec_json
                    .contains("\"quantity_kind\": \"Conductance\"")
                || !output
                    .report_spec_json
                    .contains("\"display_unit\": \"W/K\"")
                || !output
                    .report_spec_json
                    .contains("\"kind\": \"component_equation\"")
                || !output.report_spec_json.contains("\"wall.equation_1\"")
                || !output.report_spec_json.contains("\"residual_norm\"")
                || !output.result_json.contains("\"largest_residuals\"")
                || !output.result_json.contains("\"expression_unit\": \"W\"")
                || !output
                    .result_json
                    .contains("\"expression_quantity_kind\": \"HeatRate\"")
                || !output
                    .report_spec_json
                    .contains("\"expression_unit\": \"W\"")
                || !output
                    .report_spec_json
                    .contains("\"expression_quantity_kind\": \"HeatRate\"")
                || !output.report_html.contains("solved_linear")
                || !output.report_html.contains("wall.inside.Q=5 kW")
            {
                eprintln!(
                    "expected official unit-parameterized wall example to solve a conductance residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/advanced_solver/33_unit_parameterized_wall/main.eng solved unit-parameterized wall residual graph"
            );
        }
        Err(error) => {
            eprintln!("official unit-parameterized wall example failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("examples/advanced_solver/25_fixed_point_loop/main.eng"),
        Path::new("build/test-official-fixed-point-loop"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"solved_fixed_point\"")
                || !output
                    .result_json
                    .contains("\"method\": \"fixed_point_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"fixed_point_converged\"")
                || !output.report_spec_json.contains("\"tolerance\": 0.000001")
                || !output.report_spec_json.contains("\"max_iterations\": 80")
                || !output.report_spec_json.contains(
                    "\"variable_scale_policy\": \"unit_default_from_fixed_point_unknowns\"",
                )
                || !output.result_json.contains("\"residual_values\"")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\"")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_name\":")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_source_expression\":")
                || !output.result_json.contains("\"name\": \"relax.source.q\"")
                || !output.result_json.contains("\"value\": 1.999")
                || !output
                    .report_spec_json
                    .contains("source solve binding `fixed_point_result`")
                || !output.report_spec_json.contains("\"largest_residuals\"")
                || !output.report_html.contains("solved_fixed_point")
                || !output.report_html.contains("fixed_point_residual_graph")
            {
                eprintln!(
                    "expected official fixed-point loop example to converge through unitful affine source solve artifacts"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/advanced_solver/25_fixed_point_loop/main.eng solved source fixed-point residual graph"
            );
        }
        Err(error) => {
            eprintln!("official fixed-point loop example failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("examples/advanced_solver/26_dynamic_component_room/main.eng"),
        Path::new("build/test-official-dynamic-component-room"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_semi_implicit_euler\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dynamic_component_fixed_step_completed\"")
                || !output.result_json.contains("\"name\": \"zone.wall.T\"")
                || !output.result_json.contains("\"role\": \"state\"")
                || !output.result_json.contains("\"role\": \"algebraic\"")
                || !output.result_json.contains("\"step_diagnostics\"")
                || !output.result_json.contains("\"residual_values\"")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\"")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_name\":")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_source_expression\":")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"linear_algebraic_converged\"")
                || !output
                    .report_spec_json
                    .contains("source solve binding `dynamic_room_result`")
                || !output.report_html.contains("Trajectories")
                || !output.report_html.contains("Step Diagnostics")
            {
                eprintln!(
                    "expected official dynamic component room example to solve assembled residual graph trajectories"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/advanced_solver/26_dynamic_component_room/main.eng solved dynamic component source residual graph"
            );
        }
        Err(error) => {
            eprintln!("official dynamic component room example failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("examples/advanced_solver/27_nonlinear_algebraic/main.eng"),
        Path::new("build/test-official-nonlinear-algebraic"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"solved_nonlinear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"newton_source_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output.result_json.contains("\"equation_count\": 4")
                || !output.result_json.contains("\"unknown_count\": 4")
                || !output.result_json.contains("\"name\": \"node.source.q\"")
                || !output.result_json.contains("\"name\": \"node.target.q\"")
                || !output.result_json.contains("\"unit\": \"kW\"")
                || !output
                    .report_spec_json
                    .contains("\"name\": \"target_load\"")
                || !output
                    .report_spec_json
                    .contains("\"status\": \"constructor_override\"")
                || !output.result_json.contains(
                    "node.source.q * node.source.q / 1 kW + node.target.q - (node.target_load)",
                )
                || !output.result_json.contains(
                    "node.target.q * node.target.q / 1 kW + node.source.q - (node.target_load)",
                )
                || !output.result_json.contains("\"value\": 2.000")
                || !output.result_json.contains("\"step_diagnostics\"")
                || !output.result_json.contains("\"residual_values\"")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\"")
                || !output
                    .report_spec_json
                    .contains("\"jacobian_policy\": \"finite_difference\"")
                || !output
                    .report_spec_json
                    .contains("\"variable_scale_policy\": \"unit_default_from_assembly_unknowns\"")
                || !output.report_spec_json.contains("\"variable_scale_min\":")
                || !output.report_spec_json.contains("\"variable_scale_max\":")
                || !output
                    .report_spec_json
                    .contains("\"linear_condition_estimate\":")
                || !output
                    .report_spec_json
                    .contains("\"linear_minimum_pivot_abs\":")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_name\":")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_source_expression\":")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_abs_value\":")
                || !output.result_json.contains("\"largest_residuals\"")
                || !output.result_json.contains("\"expression_unit\": \"kW\"")
                || !output
                    .result_json
                    .contains("\"expression_quantity_kind\": \"HeatRate\"")
                || !output
                    .report_spec_json
                    .contains("source solve binding `nonlinear_result`")
                || !output.report_html.contains("newton_source_residual_graph")
                || !output.report_html.contains("Step Diagnostics")
                || !output.report_html.contains("largest_step_residual")
            {
                eprintln!(
                    "expected official nonlinear algebraic example to solve a coupled multi-variable unitful source Newton residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/advanced_solver/27_nonlinear_algebraic/main.eng solved source nonlinear residual graph"
            );
        }
        Err(error) => {
            eprintln!("official nonlinear algebraic example failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("examples/advanced_solver/28_small_dae/main.eng"),
        Path::new("build/test-official-small-dae"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"implicit_euler_dae_source_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dae_converged\"")
                || !output.result_json.contains("\"equation_count\": 6")
                || !output.result_json.contains("\"unknown_count\": 6")
                || !output.result_json.contains("\"name\": \"node.hot.T\"")
                || !output.result_json.contains("\"name\": \"node.cold.T\"")
                || !output.result_json.contains("\"name\": \"node.hot.T_ref\"")
                || !output.result_json.contains("\"name\": \"node.cold.T_ref\"")
                || !output.result_json.contains("\"unit\": \"K\"")
                || !output
                    .result_json
                    .contains("der(node.hot.T) + (node.hot.T - node.hot.T_ref) / 1 s - (0 K/s)")
                || !output
                    .result_json
                    .contains("der(node.cold.T) + (node.cold.T - node.cold.T_ref) / 2 s - (0 K/s)")
                || !output.result_json.contains("\"value\": 302.500")
                || !output.result_json.contains("\"value\": 299.444")
                || !output.result_json.contains("\"role\": \"state\"")
                || !output.result_json.contains("\"role\": \"algebraic\"")
                || !output.result_json.contains("\"step_diagnostics\"")
                || !output.result_json.contains("\"residual_values\"")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\"")
                || !output
                    .report_spec_json
                    .contains("\"jacobian_policy\": \"finite_difference\"")
                || !output.report_spec_json.contains(
                    "\"variable_scale_policy\": \"unit_default_from_dae_state_algebraic_layout\"",
                )
                || !output.report_spec_json.contains("\"variable_scale_min\":")
                || !output.report_spec_json.contains("\"variable_scale_max\":")
                || !output
                    .report_spec_json
                    .contains("\"linear_condition_estimate\":")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_name\":")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_source_expression\":")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_abs_value\":")
                || !output.result_json.contains("\"largest_residuals\"")
                || !output
                    .report_spec_json
                    .contains("source solve binding `dae_result`")
                || !output
                    .report_spec_json
                    .contains("configured source mass matrix")
                || !output
                    .report_html
                    .contains("implicit_euler_dae_source_residual_graph")
                || !output.report_html.contains("Trajectories")
            {
                eprintln!(
                    "expected official DAE example to solve a multi-state unitful source DAE residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/advanced_solver/28_small_dae/main.eng solved source DAE residual graph"
            );
        }
        Err(error) => {
            eprintln!("official DAE example failed: {error}");
            return ExitCode::from(1);
        }
    }
    for (source, build_dir, binding) in [
        (
            "examples/advanced_solver/29_delay_component_solver/main.eng",
            "build/test-official-delay-behavior",
            "delay_result",
        ),
        (
            "examples/advanced_solver/30_predictor_component_solver/main.eng",
            "build/test-official-predictor-behavior",
            "predictor_result",
        ),
        (
            "examples/advanced_solver/31_external_behavior_solver/main.eng",
            "build/test-official-external-behavior",
            "external_result",
        ),
    ] {
        match run_file(
            Path::new(source),
            Path::new(build_dir),
            &artifact_run_options(),
        ) {
            Ok(output) => {
                if !output.result_json.contains("\"status\": \"computed\"")
                    || !output
                        .result_json
                        .contains("\"method\": \"behavior_graph_explicit_euler_source\"")
                    || !output
                        .result_json
                        .contains("\"convergence_status\": \"behavior_graph_executed\"")
                    || !output.result_json.contains("\"name\": \"node.node.T\"")
                    || !output.result_json.contains("\"unit\": \"K\"")
                    || !output.result_json.contains("\"y\": 300.00000000")
                    || !output.result_json.contains("\"step_diagnostics\"")
                    || !output
                        .report_spec_json
                        .contains(&format!("source solve binding `{binding}`"))
                    || !output
                        .report_spec_json
                        .contains("\"status\": \"executed_in_behavior_graph\"")
                    || !output
                        .report_spec_json
                        .contains("runtime_diagnostics_available")
                    || !output.report_html.contains("executed in behavior graph")
                    || !output.report_html.contains("runtime diagnostics available")
                    || !output.report_html.contains("behavior graph executed")
                {
                    eprintln!("expected {source} to execute the source behavior graph");
                    return ExitCode::from(2);
                }
                println!("ok: {source} executed source behavior graph");
            }
            Err(error) => {
                eprintln!("official behavior graph example failed: {source}: {error}");
                return ExitCode::from(1);
            }
        }
    }
    match run_file(
        Path::new("tests/runtime/linear_algebraic_solve_from_source.eng"),
        Path::new("build/test-runtime-linear-algebraic-source"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"solved_linear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dense_linear_residual_graph\"")
                || !output.result_json.contains("\"name\": \"room.heat.Q\"")
                || !output.result_json.contains("\"value\": -2.00000000")
                || !output.result_json.contains("\"largest_residuals\"")
                || !output.result_json.contains("\"expression_unit\": \"kW\"")
                || !output
                    .result_json
                    .contains("\"expression_quantity_kind\": \"HeatRate\"")
                || !output
                    .report_spec_json
                    .contains("\"expression_unit\": \"kW\"")
                || !output
                    .report_spec_json
                    .contains("\"expression_quantity_kind\": \"HeatRate\"")
                || !output.report_html.contains("solved_linear")
            {
                eprintln!(
                    "expected tests/runtime/linear_algebraic_solve_from_source.eng to solve through the dense linear source path"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/linear_algebraic_solve_from_source.eng solved dense linear source residual graph"
            );
        }
        Err(error) => {
            eprintln!("linear algebraic source runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/source_system_dense_linear_solve.eng"),
        Path::new("build/test-runtime-source-system-dense-linear"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"assembly\": \"StaticLinearSourceSystem\"")
                || !output.result_json.contains("\"status\": \"solved_linear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dense_linear_residual_graph\"")
                || !output.result_json.contains("\"name\": \"x\"")
                || !output.result_json.contains("\"value\": 2.00000000")
                || !output.result_json.contains("\"name\": \"y\"")
                || !output.result_json.contains("\"value\": 3.00000000")
                || !output.report_html.contains("StaticLinearSourceSystem")
                || !output.report_html.contains("solved_linear")
            {
                eprintln!(
                    "expected tests/runtime/source_system_dense_linear_solve.eng to solve a static source system through the dense linear residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/source_system_dense_linear_solve.eng solved dense linear source system residual graph"
            );
        }
        Err(error) => {
            eprintln!("source system dense linear runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/source_system_fixed_point_solve.eng"),
        Path::new("build/test-runtime-source-system-fixed-point"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"assembly\": \"StaticFixedPointSourceSystem\"")
                || !output
                    .result_json
                    .contains("\"status\": \"solved_fixed_point\"")
                || !output
                    .result_json
                    .contains("\"method\": \"fixed_point_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"fixed_point_converged\"")
                || !output.result_json.contains("\"name\": \"x\"")
                || !output.result_json.contains("\"name\": \"y\"")
                || !output.result_json.contains("x - (cos(y))")
                || !output.report_html.contains("StaticFixedPointSourceSystem")
                || !output.report_html.contains("solved_fixed_point")
                || !output.report_html.contains("fixed_point_residual_graph")
            {
                eprintln!(
                    "expected tests/runtime/source_system_fixed_point_solve.eng to solve a static source system through expression-mapped fixed-point residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/source_system_fixed_point_solve.eng solved fixed-point source system residual graph"
            );
        }
        Err(error) => {
            eprintln!("source system fixed-point runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/source_system_fixed_point_coupled_affine_side.eng"),
        Path::new("build/test-runtime-source-system-fixed-point-coupled-affine-side"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"assembly\": \"StaticCoupledAffineSideFixedPointSystem\"")
                || !output
                    .result_json
                    .contains("\"status\": \"solved_fixed_point\"")
                || !output
                    .result_json
                    .contains("\"method\": \"fixed_point_residual_graph\"")
                || !output.result_json.contains("x + 0.25 * y - (cos(y))")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_source_expression\": \"x + 0.25 * y eq cos(y)\"")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_source_expression\": \"y eq x\"")
                || !output
                    .report_html
                    .contains("StaticCoupledAffineSideFixedPointSystem")
                || !output.report_html.contains("solved_fixed_point")
                || !output.report_html.contains("fixed_point_residual_graph")
            {
                eprintln!(
                    "expected tests/runtime/source_system_fixed_point_coupled_affine_side.eng to solve a coupled affine-side static source system with fixed-point source context"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/source_system_fixed_point_coupled_affine_side.eng solved coupled affine-side fixed-point source system with source context"
            );
        }
        Err(error) => {
            eprintln!(
                "source system coupled affine-side fixed-point runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/source_system_fixed_point_variable_scales.eng"),
        Path::new("build/test-runtime-source-system-fixed-point-variable-scales"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"assembly\": \"StaticScaledFixedPointSourceSystem\"")
                || !output
                    .result_json
                    .contains("\"status\": \"solved_fixed_point\"")
                || !output
                    .result_json
                    .contains("\"method\": \"fixed_point_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"variable_scale_policy\": \"user_provided_variable_scales\"")
                || !output.result_json.contains("\"variable_scale_min\": 2")
                || !output.result_json.contains("\"variable_scale_max\": 4")
                || !output
                    .report_html
                    .contains("StaticScaledFixedPointSourceSystem")
                || !output.report_html.contains("solved_fixed_point")
                || !output.report_html.contains("fixed_point_residual_graph")
            {
                eprintln!(
                    "expected tests/runtime/source_system_fixed_point_variable_scales.eng to solve a static source system with user-provided fixed-point variable scales"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/source_system_fixed_point_variable_scales.eng solved fixed-point source system with user-provided variable scales"
            );
        }
        Err(error) => {
            eprintln!("source system fixed-point variable-scale runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/source_system_fixed_point_affine_solve.eng"),
        Path::new("build/test-runtime-source-system-fixed-point-affine"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"assembly\": \"StaticAffineFixedPointSourceSystem\"")
                || !output
                    .result_json
                    .contains("\"status\": \"solved_fixed_point\"")
                || !output
                    .result_json
                    .contains("\"method\": \"fixed_point_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"fixed_point_converged\"")
                || !output.result_json.contains("\"name\": \"x\"")
                || !output.result_json.contains("\"name\": \"y\"")
                || !output.result_json.contains("2 * x + 0.1 - (cos(y))")
                || !output
                    .report_html
                    .contains("StaticAffineFixedPointSourceSystem")
                || !output.report_html.contains("solved_fixed_point")
                || !output.report_html.contains("fixed_point_residual_graph")
            {
                eprintln!(
                    "expected tests/runtime/source_system_fixed_point_affine_solve.eng to solve a static source system through affine expression-mapped fixed-point residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/source_system_fixed_point_affine_solve.eng solved affine fixed-point source system residual graph"
            );
        }
        Err(error) => {
            eprintln!("source system affine fixed-point runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/source_system_newton_solve.eng"),
        Path::new("build/test-runtime-source-system-newton"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"assembly\": \"StaticNonlinearSourceSystem\"")
                || !output
                    .result_json
                    .contains("\"status\": \"solved_nonlinear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"newton_source_residual_graph\"")
                || !output.result_json.contains("\"name\": \"x\"")
                || !output.result_json.contains("\"value\": 2.000")
                || !output.result_json.contains("\"name\": \"y\"")
                || !output.result_json.contains("\"value\": 3.000")
                || !output.report_html.contains("StaticNonlinearSourceSystem")
                || !output.report_html.contains("solved_nonlinear")
                || !output.report_html.contains("newton_source_residual_graph")
            {
                eprintln!(
                    "expected tests/runtime/source_system_newton_solve.eng to solve a static nonlinear source system through Newton residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/source_system_newton_solve.eng solved nonlinear source system residual graph"
            );
        }
        Err(error) => {
            eprintln!("source system Newton runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/source_system_newton_source_linear_jacobian.eng"),
        Path::new("build/test-runtime-source-system-newton-source-linear-jacobian"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"assembly\": \"StaticLinearNewtonJacobianSourceSystem\"")
                || !output
                    .result_json
                    .contains("\"status\": \"solved_nonlinear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"newton_source_residual_graph_with_provided_jacobian\"")
                || !output
                    .result_json
                    .contains("\"jacobian_policy\": \"source_linear_terms\"")
                || !output.result_json.contains("\"name\": \"x\"")
                || !output.result_json.contains("\"value\": 3.000")
                || !output.result_json.contains("\"name\": \"y\"")
                || !output.result_json.contains("\"value\": 2.000")
                || !output
                    .report_html
                    .contains("StaticLinearNewtonJacobianSourceSystem")
                || !output.report_html.contains("solved_nonlinear")
                || !output
                    .report_html
                    .contains("newton_source_residual_graph_with_provided_jacobian")
            {
                eprintln!(
                    "expected tests/runtime/source_system_newton_source_linear_jacobian.eng to solve a static source system through source-linear Newton Jacobian"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/source_system_newton_source_linear_jacobian.eng solved source system Newton residual graph with source-linear Jacobian"
            );
        }
        Err(error) => {
            eprintln!("source system Newton source-linear Jacobian fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/source_system_newton_variable_scales.eng"),
        Path::new("build/test-runtime-source-system-newton-variable-scales"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"assembly\": \"StaticScaledNewtonSourceSystem\"")
                || !output
                    .result_json
                    .contains("\"status\": \"solved_nonlinear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"newton_source_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"variable_scale_policy\": \"user_provided_variable_scales\"")
                || !output.result_json.contains("\"variable_scale_min\": 2")
                || !output.result_json.contains("\"variable_scale_max\": 4")
                || !output
                    .report_html
                    .contains("StaticScaledNewtonSourceSystem")
                || !output.report_html.contains("solved_nonlinear")
                || !output.report_html.contains("newton_source_residual_graph")
            {
                eprintln!(
                    "expected tests/runtime/source_system_newton_variable_scales.eng to solve a static nonlinear source system with user-provided Newton variable scales"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/source_system_newton_variable_scales.eng solved nonlinear source system with user-provided Newton variable scales"
            );
        }
        Err(error) => {
            eprintln!("source system Newton variable-scale runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/linear_residual_scale_override.eng"),
        Path::new("build/test-runtime-linear-residual-scale-override"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"solved_linear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dense_linear_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"linear_converged\"")
                || !output
                    .result_json
                    .contains("\"scale_policy\": \"user_provided:connection_set_1.across_T_1\"")
                || !output
                    .result_json
                    .contains("\"scale_policy\": \"user_provided:load.boundary_boundary_Q\"")
                || !output
                    .report_spec_json
                    .contains("\"scale_policy\": \"user_provided:connection_set_1.across_T_1\"")
                || !output
                    .report_spec_json
                    .contains("\"scale_policy\": \"user_provided:load.boundary_boundary_Q\"")
                || !output.report_html.contains("dense_linear_residual_graph")
            {
                eprintln!(
                    "expected tests/runtime/linear_residual_scale_override.eng to apply user-provided source residual scales to dense linear residual artifacts"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/linear_residual_scale_override.eng applied dense linear residual scale overrides"
            );
        }
        Err(error) => {
            eprintln!("dense linear residual scale override runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/multi_domain_thermal_fluid_from_source.eng"),
        Path::new("build/test-runtime-multi-domain-thermal-fluid-source"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !small_thermal_fluid_solve_artifacts_are_structured(&output) {
                eprintln!(
                    "expected multi-domain thermal/fluid runtime fixture to solve a square residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/multi_domain_thermal_fluid_from_source.eng solved a constrained Thermal/Fluid residual graph"
            );
        }
        Err(error) => {
            eprintln!("multi-domain thermal/fluid source runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/fixed_point_solve_from_source.eng"),
        Path::new("build/test-runtime-fixed-point-source"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"solved_fixed_point\"")
                || !output
                    .result_json
                    .contains("\"method\": \"fixed_point_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"fixed_point_converged\"")
                || !output.report_spec_json.contains("\"max_iterations\": 80")
                || !output.report_html.contains("solved_fixed_point")
            {
                eprintln!(
                    "expected tests/runtime/fixed_point_solve_from_source.eng to solve through the fixed-point source path"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/fixed_point_solve_from_source.eng solved fixed-point source residual graph"
            );
        }
        Err(error) => {
            eprintln!("fixed-point source runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/fixed_point_expression_mapping.eng"),
        Path::new("build/test-runtime-fixed-point-expression-mapping"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"solved_fixed_point\"")
                || !output
                    .result_json
                    .contains("\"method\": \"fixed_point_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"fixed_point_converged\"")
                || !output.result_json.contains("\"name\": \"loop.source.x\"")
                || !output.result_json.contains("\"name\": \"loop.target.x\"")
                || !output
                    .result_json
                    .contains("loop.source.x - (cos(loop.target.x))")
                || !output
                    .report_spec_json
                    .contains("loop.source.x eq cos(loop.target.x)")
                || !output.report_html.contains("solved_fixed_point")
                || !output.report_html.contains("cos(loop.target.x)")
            {
                eprintln!(
                    "expected tests/runtime/fixed_point_expression_mapping.eng to solve a nonlinear expression-mapped fixed-point residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/fixed_point_expression_mapping.eng solved expression-mapped fixed-point residual graph"
            );
        }
        Err(error) => {
            eprintln!("fixed-point expression-mapping runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/fixed_point_affine_expression_mapping.eng"),
        Path::new("build/test-runtime-fixed-point-affine-expression-mapping"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"solved_fixed_point\"")
                || !output
                    .result_json
                    .contains("\"method\": \"fixed_point_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"fixed_point_converged\"")
                || !output
                    .result_json
                    .contains("2 * loop.source.x + 0.1 - (cos(loop.target.x))")
                || !output.report_html.contains("solved_fixed_point")
                || !output.report_html.contains("fixed_point_residual_graph")
                || !output.report_html.contains("cos(loop.target.x)")
            {
                eprintln!(
                    "expected tests/runtime/fixed_point_affine_expression_mapping.eng to solve an affine expression-mapped fixed-point residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/fixed_point_affine_expression_mapping.eng solved affine expression-mapped fixed-point residual graph"
            );
        }
        Err(error) => {
            eprintln!("fixed-point affine expression-mapping runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/fixed_point_residual_scale_override.eng"),
        Path::new("build/test-runtime-fixed-point-residual-scale-override"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"solved_fixed_point\"")
                || !output
                    .result_json
                    .contains("\"method\": \"fixed_point_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"fixed_point_converged\"")
                || !output
                    .result_json
                    .contains("\"scale_policy\": \"user_provided:connection_set_1.across_q_1\"")
                || !output
                    .result_json
                    .contains("\"scale_policy\": \"user_provided:relax.equation_1\"")
                || !output
                    .report_spec_json
                    .contains("\"scale_policy\": \"user_provided:connection_set_1.across_q_1\"")
                || !output
                    .report_spec_json
                    .contains("\"scale_policy\": \"user_provided:relax.equation_1\"")
                || !output.report_html.contains("fixed_point_residual_graph")
            {
                eprintln!(
                    "expected tests/runtime/fixed_point_residual_scale_override.eng to apply user-provided source residual scales to fixed-point residual artifacts"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/fixed_point_residual_scale_override.eng applied fixed-point residual scale overrides"
            );
        }
        Err(error) => {
            eprintln!("fixed-point residual scale override runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/nonlinear_residual_from_source.eng"),
        Path::new("build/test-runtime-nonlinear-source"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"solved_nonlinear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"newton_source_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output.result_json.contains("\"equation_count\": 4")
                || !output.result_json.contains("\"unknown_count\": 4")
                || !output.result_json.contains("\"name\": \"node.source.q\"")
                || !output.result_json.contains("\"name\": \"node.target.q\"")
                || !output.result_json.contains("\"unit\": \"kW\"")
                || !output
                    .result_json
                    .contains("node.source.q * node.source.q / 1 kW + node.target.q - (6 kW)")
                || !output
                    .result_json
                    .contains("node.target.q * node.target.q / 1 kW + node.source.q - (6 kW)")
                || !output.result_json.contains("\"value\": 2.000")
                || !output.result_json.contains("\"step_diagnostics\"")
                || !output.result_json.contains("\"residual_values\"")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\"")
                || !output.result_json.contains("\"largest_residuals\"")
                || !output.report_html.contains("newton_source_residual_graph")
            {
                eprintln!(
                    "expected tests/runtime/nonlinear_residual_from_source.eng to solve a coupled multi-variable unitful source Newton residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/nonlinear_residual_from_source.eng solved source nonlinear residual graph"
            );
        }
        Err(error) => {
            eprintln!("nonlinear source runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/newton_source_linear_jacobian.eng"),
        Path::new("build/test-runtime-newton-source-linear-jacobian"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"solved_nonlinear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"newton_source_residual_graph_with_provided_jacobian\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output
                    .result_json
                    .contains("\"jacobian_policy\": \"source_linear_terms\"")
                || !output
                    .result_json
                    .contains("\"variable_scale_policy\": \"unit_default_from_assembly_unknowns\"")
                || !output.result_json.contains("\"tolerance\": 0.000000001")
                || !output.result_json.contains("\"max_iterations\": 20")
                || !output
                    .result_json
                    .contains("\"linear_condition_estimate\":")
                || !output.result_json.contains("\"equation_count\": 4")
                || !output.result_json.contains("\"unknown_count\": 4")
                || !output.result_json.contains("\"name\": \"node.source.q\"")
                || !output.result_json.contains("\"name\": \"node.target.q\"")
                || !output.result_json.contains("\"value\": 3.000")
                || !output.result_json.contains("\"value\": 2.000")
                || !output.result_json.contains("\"residual_values\"")
                || !output.result_json.contains("\"source_expression\"")
                || !output.result_json.contains("\"source_line\":")
                || !output.result_json.contains("\"dependencies\": [")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\"")
                || !output
                    .report_spec_json
                    .contains("\"jacobian_policy\": \"source_linear_terms\"")
                || !output
                    .report_spec_json
                    .contains("\"linear_condition_estimate\":")
                || !output.report_spec_json.contains("\"source_expression\"")
                || !output.report_spec_json.contains("\"source_line\":")
                || !output.report_spec_json.contains("\"dependencies\": [")
                || !output.report_html.contains("source_linear_terms")
                || !output.report_html.contains("source_line=")
                || !output.report_html.contains("deps=[")
            {
                eprintln!(
                    "expected tests/runtime/newton_source_linear_jacobian.eng to solve a linear source Newton residual graph with the source-linear Jacobian hook"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/newton_source_linear_jacobian.eng solved source Newton residual graph with source-linear Jacobian"
            );
        }
        Err(error) => {
            eprintln!("source-linear Newton Jacobian runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/newton_residual_scale_override.eng"),
        Path::new("build/test-runtime-newton-residual-scale-override"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"solved_nonlinear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"newton_source_residual_graph_with_provided_jacobian\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output
                    .result_json
                    .contains("\"scale_policy\": \"user_provided:node.equation_1\"")
                || !output
                    .result_json
                    .contains("\"scale_policy\": \"user_provided:node.equation_2\"")
                || !output
                    .result_json
                    .contains("\"residual_values\": [0.00000000, 0.00000000, -5.00000000, -1.00000000]")
                || !output
                    .result_json
                    .contains("\"normalized_residual_values\": [0.00000000, 0.00000000, -2.50000000, -0.25000000]")
                || !output
                    .report_spec_json
                    .contains("\"scale_policy\": \"user_provided:node.equation_1\"")
                || !output
                    .report_spec_json
                    .contains("\"scale_policy\": \"user_provided:node.equation_2\"")
                || !output
                    .report_spec_json
                    .contains("\"residual_values\": [0, 0, -5, -1]")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\": [0, 0, -2.5, -0.25]")
            {
                eprintln!(
                    "expected tests/runtime/newton_residual_scale_override.eng to apply user-provided source residual scales"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/newton_residual_scale_override.eng applied user-provided source residual scales"
            );
        }
        Err(error) => {
            eprintln!("Newton residual scale override runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/newton_dimensionless_function_residual.eng"),
        Path::new("build/test-runtime-newton-dimensionless-function-residual"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"solved_nonlinear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"newton_source_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output.result_json.contains("\"name\": \"node.node.x\"")
                || !output.result_json.contains("\"value\": 4.000")
                || !output.result_json.contains("sqrt(node.node.x) - (2)")
                || !output.result_json.contains("\"source_expression\"")
                || !output.result_json.contains("\"source_line\":")
                || !output
                    .result_json
                    .contains("\"largest_residual_source_expression\": \"sqrt(node.node.x) eq 2\"")
                || !output
                    .result_json
                    .contains("\"largest_residual_source_line\": 9")
                || !output.result_json.contains(
                    "\"largest_residual_source_reason\": \"component-local equation source\"",
                )
                || !output.report_spec_json.contains("\"source_expression\"")
                || !output.report_spec_json.contains("\"source_line\":")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_source_expression\": \"sqrt(node.node.x) eq 2\"")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_source_line\": 9")
                || !output.report_html.contains("newton_source_residual_graph")
                || !output.report_html.contains("source_line=")
            {
                eprintln!(
                    "expected tests/runtime/newton_dimensionless_function_residual.eng to solve a dimensionless source Newton residual with sqrt()"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/newton_dimensionless_function_residual.eng solved dimensionless source Newton function residual"
            );
        }
        Err(error) => {
            eprintln!("dimensionless function Newton source runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/newton_dimensionless_trig_residual.eng"),
        Path::new("build/test-runtime-newton-dimensionless-trig-residual"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"solved_nonlinear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"newton_source_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output.result_json.contains("\"name\": \"node.node.x\"")
                || !output.result_json.contains("\"value\": 0.785")
                || !output.result_json.contains("tan(node.node.x) - (1)")
                || !output
                    .result_json
                    .contains("\"largest_residual_source_expression\": \"tan(node.node.x) eq 1\"")
                || !output.report_spec_json.contains("tan(node.node.x) eq 1")
                || !output.report_spec_json.contains("\"source_line\":")
                || !output.report_html.contains("newton_source_residual_graph")
                || !output.report_html.contains("source_line=")
            {
                eprintln!(
                    "expected tests/runtime/newton_dimensionless_trig_residual.eng to solve a dimensionless source Newton residual with tan()"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/newton_dimensionless_trig_residual.eng solved dimensionless source Newton trig residual"
            );
        }
        Err(error) => {
            eprintln!("dimensionless trig Newton source runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/source_rhs_dimensionless_functions.eng"),
        Path::new("build/test-runtime-source-rhs-dimensionless-functions"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let plot_spec = std::fs::read_to_string(&output.plot_spec_path).unwrap_or_default();
            if !output.result_json.contains("\"binding\": \"sim\"")
                || !output
                    .result_json
                    .contains("\"method\": \"rk4_fixed_step\"")
                || !output
                    .result_json
                    .contains("recognized source derivative equations and executed fixed-step RHS")
                || !output.result_json.contains("\"state\": \"x\"")
                || !output.result_json.contains("\"state\": \"damping\"")
                || !output.result_json.contains("der(x) - (-sin(x) / 1 s)")
                || !output
                    .result_json
                    .contains("damping - (cos(x) + atan(x) + tan(x) / 10)")
                || !output.report_spec_json.contains("\"solver_results\"")
                || !output.report_spec_json.contains("\"state\": \"damping\"")
                || !output
                    .report_spec_json
                    .contains("damping - (cos(x) + atan(x) + tan(x) / 10)")
                || !plot_spec.contains("\"name\": \"sim.x\"")
                || !output.report_html.contains("System Solver Results")
                || !output.report_html.contains("rk4_fixed_step")
            {
                eprintln!(
                    "expected tests/runtime/source_rhs_dimensionless_functions.eng to simulate a source RHS using dimensionless trig functions"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/source_rhs_dimensionless_functions.eng simulated dimensionless source RHS trig functions"
            );
        }
        Err(error) => {
            eprintln!("dimensionless source RHS function runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/source_ode_two_state_adaptive.eng"),
        Path::new("build/test-runtime-source-ode-two-state-adaptive"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let plot_spec = std::fs::read_to_string(&output.plot_spec_path).unwrap_or_default();
            if !output.result_json.contains("\"binding\": \"sim\"")
                || !output.result_json.contains("\"method\": \"adaptive_heun\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"adaptive_heun_completed\"")
                || !output.result_json.contains(
                    "recognized source derivative equations and executed adaptive Heun RHS",
                )
                || !output.result_json.contains("\"state_count\": 2")
                || !output
                    .result_json
                    .contains("\"outputs\": [\"T_air\", \"T_wall\", \"Q_load\"]")
                || !output.result_json.contains("\"state\": \"T_air\"")
                || !output.result_json.contains("\"state\": \"T_wall\"")
                || !output.result_json.contains("\"state\": \"Q_load\"")
                || !output
                    .result_json
                    .contains("\"final_value\": 21.373435452969773")
                || !output
                    .result_json
                    .contains("\"final_value\": 19.606950814289405")
                || !output
                    .result_json
                    .contains("\"final_value\": 0.06265645470302275")
                || !output.result_json.contains("\"status\": \"accepted\"")
                || !output.report_spec_json.contains("\"step_diagnostics\"")
                || !output
                    .report_spec_json
                    .contains("Q_load - (Q_hvac + UA_ao * (T_out - T_air))")
                || !plot_spec.contains("\"name\": \"sim.T_air\"")
                || !plot_spec.contains("\"name\": \"sim.T_wall\"")
                || !output.report_html.contains("adaptive_heun")
                || !output.report_html.contains("Q_load")
            {
                eprintln!(
                    "expected tests/runtime/source_ode_two_state_adaptive.eng to solve a two-state source ODE with adaptive Heun substep diagnostics and scalar output"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/source_ode_two_state_adaptive.eng solved two-state source ODE with adaptive Heun"
            );
        }
        Err(error) => {
            eprintln!("two-state adaptive source ODE runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("examples/advanced_solver/34_three_state_source_ode/main.eng"),
        Path::new("build/test-official-three-state-source-ode"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let plot_spec = std::fs::read_to_string(&output.plot_spec_path).unwrap_or_default();
            if !output.result_json.contains("\"binding\": \"sim\"")
                || !output.result_json.contains("\"method\": \"adaptive_heun\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"adaptive_heun_completed\"")
                || !output.result_json.contains(
                    "recognized source derivative equations and executed adaptive Heun RHS with TimeSeries input materialization",
                )
                || !output.result_json.contains("\"states\": [\"x\", \"y\", \"z\"]")
                || !output
                    .result_json
                    .contains("\"outputs\": [\"x\", \"y\", \"z\", \"total\"]")
                || !output.result_json.contains("\"state\": \"total\"")
                || !output.result_json.contains("\"status\": \"accepted\"")
                || !output.report_spec_json.contains("\"step_diagnostics\"")
                || !output.report_spec_json.contains("total - (x + y + z)")
                || !plot_spec.contains("\"name\": \"sim.x\"")
                || !plot_spec.contains("\"name\": \"sim.y\"")
                || !plot_spec.contains("\"name\": \"sim.z\"")
                || !plot_spec.contains("\"name\": \"sim.total\"")
                || !output.report_html.contains("ThreeStateSourceOde")
                || !output.report_html.contains("states=x, y, z")
            {
                eprintln!(
                    "expected examples/advanced_solver/34_three_state_source_ode/main.eng to solve a three-state source ODE with TimeSeries input materialization"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/advanced_solver/34_three_state_source_ode/main.eng solved three-state source ODE with adaptive Heun"
            );
        }
        Err(error) => {
            eprintln!("official three-state source ODE example failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/source_ode_three_state_nonthermal.eng"),
        Path::new("build/test-runtime-source-ode-three-state-nonthermal"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let plot_spec = std::fs::read_to_string(&output.plot_spec_path).unwrap_or_default();
            if !output.result_json.contains("\"binding\": \"sim\"")
                || !output.result_json.contains("\"method\": \"adaptive_heun\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"adaptive_heun_completed\"")
                || !output.result_json.contains(
                    "recognized source derivative equations and executed adaptive Heun RHS with TimeSeries input materialization",
                )
                || !output.result_json.contains("\"states\": [\"x\", \"y\", \"z\"]")
                || !output
                    .result_json
                    .contains("\"outputs\": [\"x\", \"y\", \"z\", \"total\"]")
                || !output.result_json.contains("\"state\": \"x\"")
                || !output.result_json.contains("\"state\": \"y\"")
                || !output.result_json.contains("\"state\": \"z\"")
                || !output.result_json.contains("\"state\": \"total\"")
                || !output.result_json.contains("\"status\": \"accepted\"")
                || !output.report_spec_json.contains("\"step_diagnostics\"")
                || !output.report_spec_json.contains("total - (x + y + z)")
                || !plot_spec.contains("\"name\": \"sim.x\"")
                || !plot_spec.contains("\"name\": \"sim.y\"")
                || !plot_spec.contains("\"name\": \"sim.z\"")
                || !output.report_html.contains("states=x, y, z")
                || !output.report_html.contains("adaptive_heun")
            {
                eprintln!(
                    "expected tests/runtime/source_ode_three_state_nonthermal.eng to solve a three-state non-thermal source ODE with TimeSeries input materialization"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/source_ode_three_state_nonthermal.eng solved three-state non-thermal source ODE with adaptive Heun"
            );
        }
        Err(error) => {
            eprintln!("three-state non-thermal source ODE runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/source_ode_parameter_override.eng"),
        Path::new("build/test-runtime-source-ode-parameter-override"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let plot_spec = std::fs::read_to_string(&output.plot_spec_path).unwrap_or_default();
            if !output.result_json.contains("\"binding\": \"sim\"")
                || !output
                    .result_json
                    .contains("\"method\": \"rk4_fixed_step\"")
                || !output.result_json.contains("\"parameters\": [\"gain\"]")
                || !output
                    .result_json
                    .contains("\"outputs\": [\"x\", \"shifted\"]")
                || !output.result_json.contains("\"state\": \"x\"")
                || !output.result_json.contains("\"state\": \"shifted\"")
                || !output.result_json.contains("\"final_value\": 4")
                || !output.result_json.contains("\"final_value\": 6")
                || !output.report_spec_json.contains("der(x) - (gain / 1 s)")
                || !output.report_spec_json.contains("shifted - (x + gain)")
                || !plot_spec.contains("\"name\": \"sim.x\"")
                || !plot_spec.contains("\"name\": \"sim.shifted\"")
                || !output.report_html.contains("SourceParameterOverride")
            {
                eprintln!(
                    "expected tests/runtime/source_ode_parameter_override.eng to apply simulate parameter overrides in source ODE evaluation"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/source_ode_parameter_override.eng applied source ODE parameter override"
            );
        }
        Err(error) => {
            eprintln!("source ODE parameter override runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/source_ode_scalar_input_override.eng"),
        Path::new("build/test-runtime-source-ode-scalar-input-override"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let plot_spec = std::fs::read_to_string(&output.plot_spec_path).unwrap_or_default();
            if !output.result_json.contains("\"binding\": \"sim\"")
                || !output
                    .result_json
                    .contains("\"method\": \"rk4_fixed_step\"")
                || !output.result_json.contains("\"inputs\": [\"drive\"]")
                || !output
                    .result_json
                    .contains("\"outputs\": [\"x\", \"shifted\"]")
                || !output.result_json.contains("\"state\": \"x\"")
                || !output.result_json.contains("\"state\": \"shifted\"")
                || !output.result_json.contains("\"final_value\": 4")
                || !output.result_json.contains("\"final_value\": 6")
                || !output.report_spec_json.contains("der(x) - (drive / 1 s)")
                || !output.report_spec_json.contains("shifted - (x + drive)")
                || !plot_spec.contains("\"name\": \"sim.x\"")
                || !plot_spec.contains("\"name\": \"sim.shifted\"")
                || !output.report_html.contains("SourceScalarInputOverride")
            {
                eprintln!(
                    "expected tests/runtime/source_ode_scalar_input_override.eng to apply simulate scalar input overrides in source ODE evaluation"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/source_ode_scalar_input_override.eng applied source ODE scalar input override"
            );
        }
        Err(error) => {
            eprintln!("source ODE scalar input override runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/source_ode_output_dependency.eng"),
        Path::new("build/test-runtime-source-ode-output-dependency"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let plot_spec = std::fs::read_to_string(&output.plot_spec_path).unwrap_or_default();
            if !output.result_json.contains("\"binding\": \"sim\"")
                || !output
                    .result_json
                    .contains("\"method\": \"rk4_fixed_step\"")
                || !output
                    .result_json
                    .contains("recognized source derivative equations and executed fixed-step RHS")
                || !output
                    .result_json
                    .contains("\"outputs\": [\"x\", \"total\", \"shifted\"]")
                || !output.result_json.contains("\"state\": \"shifted\"")
                || !output.result_json.contains("\"state\": \"total\"")
                || !output.result_json.contains("\"final_value\": 2")
                || !output.result_json.contains("\"final_value\": 3")
                || !output.report_spec_json.contains("total - (shifted + x)")
                || !output.report_spec_json.contains("shifted - (x + 1)")
                || !output.result_json.contains("\"source_equations\"")
                || !output.result_json.contains("\"kind\": \"derivative\"")
                || !output
                    .result_json
                    .contains("\"quantity_kind\": \"Derivative[DimensionlessNumber]\"")
                || !output.result_json.contains("\"source_line\": 7")
                || !output
                    .report_spec_json
                    .contains("\"kind\": \"algebraic_output\"")
                || !output.review_json.contains("\"source_equations\"")
                || !plot_spec.contains("\"name\": \"sim.shifted\"")
                || !plot_spec.contains("\"name\": \"sim.total\"")
                || !output.report_html.contains("SourceOutputDependency")
                || !output.report_html.contains("Source Equations")
                || !output.report_html.contains("derivative:x")
                || !output.report_html.contains("rk4_fixed_step")
            {
                eprintln!(
                    "expected tests/runtime/source_ode_output_dependency.eng to solve output-to-output source dependencies through an acyclic output DAG"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/source_ode_output_dependency.eng solved acyclic source output dependencies"
            );
        }
        Err(error) => {
            eprintln!("source output dependency runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dae_dimensionless_function_residual.eng"),
        Path::new("build/test-runtime-dae-dimensionless-function-residual"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"implicit_euler_dae_source_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dae_converged\"")
                || !output.result_json.contains("\"name\": \"node.node.x\"")
                || !output.result_json.contains("\"name\": \"node.node.z\"")
                || !output
                    .result_json
                    .contains("der(node.node.x) + sin(node.node.x) - (0)")
                || !output
                    .result_json
                    .contains("node.node.z - atan(node.node.x) - (0)")
                || !output
                    .result_json
                    .contains("\"normalized_residual_values\"")
                || !output.result_json.contains("\"source_expression\"")
                || !output.result_json.contains("\"source_line\":")
                || !output
                    .report_spec_json
                    .contains("\"method\": \"implicit_euler_dae_source_residual_graph\"")
                || !output.report_spec_json.contains("\"step_diagnostics\"")
                || !output
                    .report_spec_json
                    .contains("der(node.node.x) + sin(node.node.x) eq 0")
                || !output
                    .report_spec_json
                    .contains("node.node.z - atan(node.node.x) eq 0")
                || !output.report_spec_json.contains("\"source_line\":")
                || !output
                    .report_html
                    .contains("implicit_euler_dae_source_residual_graph")
                || !output.report_html.contains("source_line=")
            {
                eprintln!(
                    "expected tests/runtime/dae_dimensionless_function_residual.eng to solve a dimensionless DAE residual graph using trig functions"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dae_dimensionless_function_residual.eng solved dimensionless DAE function residual graph"
            );
        }
        Err(error) => {
            eprintln!("dimensionless DAE function runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dae_residual_scale_override.eng"),
        Path::new("build/test-runtime-dae-residual-scale-override"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"implicit_euler_dae_source_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dae_converged\"")
                || !output
                    .result_json
                    .contains("\"scale_policy\": \"user_provided:node.equation_1\"")
                || !output
                    .result_json
                    .contains("\"scale_policy\": \"user_provided:node.equation_2\"")
                || !output.result_json.contains("\"residual_values\"")
                || !output
                    .result_json
                    .contains("\"normalized_residual_values\"")
                || !output
                    .report_spec_json
                    .contains("\"scale_policy\": \"user_provided:node.equation_1\"")
                || !output
                    .report_spec_json
                    .contains("\"scale_policy\": \"user_provided:node.equation_2\"")
                || !output.report_spec_json.contains("\"residual_values\"")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\"")
                || !output
                    .report_html
                    .contains("implicit_euler_dae_source_residual_graph")
            {
                eprintln!(
                    "expected tests/runtime/dae_residual_scale_override.eng to preserve raw and normalized DAE residual diagnostics under source scale overrides"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dae_residual_scale_override.eng preserved DAE residual scale override diagnostics"
            );
        }
        Err(error) => {
            eprintln!("DAE residual scale override runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dae_timeseries_input_from_source.eng"),
        Path::new("build/test-runtime-dae-timeseries-input-source"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"implicit_euler_dae_source_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dae_converged\"")
                || !output
                    .result_json
                    .contains("TimeSeries input materialization")
                || !output.result_json.contains("\"name\": \"node.node.x\"")
                || !output.result_json.contains("\"value\": 0.806")
                || !output.result_json.contains("\"name\": \"node.node.z\"")
                || !output.result_json.contains("\"value\": 0.900")
                || !output
                    .result_json
                    .contains("der(node.node.x) + node.node.x - node.drive - (0)")
                || !output
                    .result_json
                    .contains("node.node.z - node.drive - (0)")
                || !output.report_spec_json.contains("\"step_diagnostics\"")
                || !output.report_spec_json.contains("\"name\": \"node.drive\"")
                || !output.report_spec_json.contains("\"node.drive\"")
                || !output
                    .report_html
                    .contains("implicit_euler_dae_source_residual_graph")
            {
                eprintln!(
                    "expected tests/runtime/dae_timeseries_input_from_source.eng to solve a source DAE residual graph with TimeSeries input materialization"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dae_timeseries_input_from_source.eng solved source DAE residual graph with TimeSeries inputs"
            );
        }
        Err(error) => {
            eprintln!("DAE TimeSeries-input source runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dae_predictor_behavior_from_source.eng"),
        Path::new("build/test-runtime-dae-predictor-behavior-source"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"behavior_graph_implicit_euler_dae_source\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dae_converged\"")
                || !output
                    .result_json
                    .contains("behavior graph residual evaluation")
                || !output.result_json.contains("\"name\": \"node.node.x\"")
                || !output.result_json.contains("\"final_value\": 0.75000000")
                || !output.result_json.contains("\"name\": \"node.node.z\"")
                || !output
                    .result_json
                    .contains("der(node.node.x) + predicted - 1 - (0)")
                || !output.result_json.contains("node.node.z - predicted - (0)")
                || !output
                    .report_spec_json
                    .contains("\"status\": \"executed_in_behavior_graph\"")
                || !output
                    .report_spec_json
                    .contains("runtime_diagnostics_available")
                || !output
                    .report_html
                    .contains("behavior_graph_implicit_euler_dae_source")
            {
                eprintln!(
                    "expected tests/runtime/dae_predictor_behavior_from_source.eng to solve DAE residuals with Predictor behavior output symbols"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dae_predictor_behavior_from_source.eng solved source DAE residual graph with Predictor behavior"
            );
        }
        Err(error) => {
            eprintln!("DAE Predictor behavior source runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dae_external_behavior_from_source.eng"),
        Path::new("build/test-runtime-dae-external-behavior-source"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"behavior_graph_implicit_euler_dae_source\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dae_converged\"")
                || !output
                    .result_json
                    .contains("behavior graph residual evaluation")
                || !output.result_json.contains("\"name\": \"node.node.x\"")
                || !output.result_json.contains("\"final_value\": 0.75000000")
                || !output.result_json.contains("\"name\": \"node.node.z\"")
                || !output
                    .result_json
                    .contains("der(node.node.x) + adapted - 1 - (0)")
                || !output.result_json.contains("node.node.z - adapted - (0)")
                || !output
                    .report_spec_json
                    .contains("\"status\": \"executed_in_behavior_graph\"")
                || !output
                    .report_spec_json
                    .contains("runtime_diagnostics_available")
                || !output
                    .report_html
                    .contains("behavior_graph_implicit_euler_dae_source")
            {
                eprintln!(
                    "expected tests/runtime/dae_external_behavior_from_source.eng to solve DAE residuals with external behavior output symbols"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dae_external_behavior_from_source.eng solved source DAE residual graph with external behavior"
            );
        }
        Err(error) => {
            eprintln!("DAE external behavior source runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/dae_delay_behavior_unsupported.eng"),
        Path::new("build/test-diagnostic-dae-delay-behavior-unsupported"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"failed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"behavior_graph_implicit_euler_dae_source\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dae_source_failed\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-BEHAVIOR-SOURCE-DAE-DELAY\"")
                || !output
                    .report_spec_json
                    .contains("\"diagnostic_code\": \"E-BEHAVIOR-SOURCE-DAE-DELAY\"")
                || !output
                    .report_spec_json
                    .contains("\"status\": \"declared_not_executed\"")
                || !output
                    .report_spec_json
                    .contains("runtime_diagnostics_not_available")
                || output
                    .report_spec_json
                    .contains("\"runtime_warning_status\": \"runtime_diagnostics_available\"")
                || !output.report_html.contains("E-BEHAVIOR-SOURCE-DAE-DELAY")
            {
                eprintln!(
                    "expected tests/diagnostics/dae_delay_behavior_unsupported.eng to report explicit unsupported DAE delay behavior"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/dae_delay_behavior_unsupported.eng reported unsupported DAE delay behavior"
            );
        }
        Err(error) => {
            eprintln!("DAE delay behavior unsupported diagnostic fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/small_dae_from_source.eng"),
        Path::new("build/test-runtime-small-dae-source"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"implicit_euler_dae_source_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dae_converged\"")
                || !output.result_json.contains("\"equation_count\": 6")
                || !output.result_json.contains("\"unknown_count\": 6")
                || !output.result_json.contains("\"name\": \"node.hot.T\"")
                || !output.result_json.contains("\"name\": \"node.cold.T\"")
                || !output.result_json.contains("\"name\": \"node.hot.T_ref\"")
                || !output.result_json.contains("\"name\": \"node.cold.T_ref\"")
                || !output.result_json.contains("\"unit\": \"K\"")
                || !output
                    .result_json
                    .contains("der(node.hot.T) + (node.hot.T - node.hot.T_ref) / 1 s - (0 K/s)")
                || !output
                    .result_json
                    .contains("der(node.cold.T) + (node.cold.T - node.cold.T_ref) / 2 s - (0 K/s)")
                || !output.result_json.contains("\"value\": 302.500")
                || !output.result_json.contains("\"value\": 299.444")
                || !output.result_json.contains("\"step_diagnostics\"")
                || !output.result_json.contains("\"residual_values\"")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\"")
                || !output
                    .report_spec_json
                    .contains("\"jacobian_policy\": \"finite_difference\"")
                || !output
                    .report_spec_json
                    .contains("\"linear_condition_estimate\":")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_name\":")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_source_expression\":")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_abs_value\":")
                || !output.result_json.contains("\"largest_residuals\"")
                || !output
                    .report_html
                    .contains("implicit_euler_dae_source_residual_graph")
            {
                eprintln!(
                    "expected tests/runtime/small_dae_from_source.eng to solve a multi-state unitful source DAE residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/small_dae_from_source.eng solved source DAE residual graph"
            );
        }
        Err(error) => {
            eprintln!("small DAE source runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    for (source, build_dir) in [
        (
            "tests/runtime/delay_behavior_from_source.eng",
            "build/test-runtime-delay-behavior-source",
        ),
        (
            "tests/runtime/predictor_behavior_from_source.eng",
            "build/test-runtime-predictor-behavior-source",
        ),
        (
            "tests/runtime/external_behavior_from_source.eng",
            "build/test-runtime-external-behavior-source",
        ),
    ] {
        match run_file(
            Path::new(source),
            Path::new(build_dir),
            &artifact_run_options(),
        ) {
            Ok(output) => {
                if !output.result_json.contains("\"status\": \"computed\"")
                    || !output
                        .result_json
                        .contains("\"method\": \"behavior_graph_explicit_euler_source\"")
                    || !output
                        .result_json
                        .contains("\"convergence_status\": \"behavior_graph_executed\"")
                    || !output.result_json.contains("\"name\": \"node.node.T\"")
                    || !output.result_json.contains("\"unit\": \"K\"")
                    || !output.result_json.contains("\"y\": 300.00000000")
                    || !output.result_json.contains("\"step_diagnostics\"")
                    || !output
                        .report_spec_json
                        .contains("\"status\": \"executed_in_behavior_graph\"")
                    || !output
                        .report_spec_json
                        .contains("runtime_diagnostics_available")
                    || !output.report_html.contains("executed in behavior graph")
                    || !output.report_html.contains("runtime diagnostics available")
                    || !output.report_html.contains("behavior graph executed")
                {
                    eprintln!("expected {source} to execute the source behavior graph");
                    return ExitCode::from(2);
                }
                println!("ok: {source} executed source behavior graph");
            }
            Err(error) => {
                eprintln!("behavior graph source runtime fixture failed: {source}: {error}");
                return ExitCode::from(1);
            }
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_explicit.eng"),
        Path::new("build/test-runtime-dynamic-component-explicit"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_explicit_euler\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"algebraic_not_required\"")
                || !output.result_json.contains("\"name\": \"node.node.x\"")
                || !output.result_json.contains("\"role\": \"state\"")
                || !output
                    .report_html
                    .contains("dynamic_component_assembly_explicit_euler")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_explicit.eng to solve through explicit dynamic component source path"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_explicit.eng solved explicit dynamic component source graph"
            );
        }
        Err(error) => {
            eprintln!("dynamic component explicit runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_function_explicit.eng"),
        Path::new("build/test-runtime-dynamic-component-function-explicit"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_explicit_euler\"")
                || !output.result_json.contains(
                    "dynamic component source solve executed parsed derivative residual expressions",
                )
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"algebraic_not_required\"")
                || !output.result_json.contains("\"name\": \"node.node.x\"")
                || !output.result_json.contains("\"role\": \"state\"")
                || !output.result_json.contains("\"final_value\": 0.16281192")
                || !output
                    .report_spec_json
                    .contains("der(node.node.x) + sin(node.node.x) eq 0")
                || !output.report_spec_json.contains("step_diagnostics")
                || !output
                    .report_html
                    .contains("dynamic_component_assembly_explicit_euler")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_function_explicit.eng to solve a function RHS through the parsed explicit dynamic component path"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_function_explicit.eng solved explicit dynamic component function RHS"
            );
        }
        Err(error) => {
            eprintln!("dynamic component function explicit runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_nonlinear_derivative_explicit.eng"),
        Path::new("build/test-runtime-dynamic-component-nonlinear-derivative-explicit"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_explicit_euler\"")
                || !output
                    .result_json
                    .contains("parsed nonlinear derivative residual Newton solves")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dynamic_component_fixed_step_completed\"")
                || !output.result_json.contains("\"name\": \"node.node.x\"")
                || !output.result_json.contains("\"role\": \"state\"")
                || !output.result_json.contains("\"final_value\": 1.49999900")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output
                    .result_json
                    .contains("\"largest_residual_name\": \"node.equation_1\"")
                || !output
                    .report_spec_json
                    .contains("der(node.node.x) * der(node.node.x) + der(node.node.x) eq 2")
                || !output.report_spec_json.contains("newton_converged")
                || !output.report_html.contains("node.node.x=1.499999")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_nonlinear_derivative_explicit.eng to solve fixed-step explicit nonlinear derivative residuals through Newton"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_nonlinear_derivative_explicit.eng solved fixed-step explicit nonlinear derivative residuals"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component nonlinear derivative explicit runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_derivative_residual_scale_override.eng"),
        Path::new("build/test-runtime-dynamic-component-derivative-residual-scale-override"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_explicit_euler\"")
                || !output
                    .result_json
                    .contains("parsed nonlinear derivative residual Newton solves")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dynamic_component_fixed_step_completed\"")
                || !output.result_json.contains("\"final_value\": 1.49999900")
                || !output
                    .result_json
                    .contains("\"residual_values\": [-2.00000000]")
                || !output
                    .result_json
                    .contains("\"normalized_residual_values\": [-1.00000000]")
                || !output.result_json.contains(
                    "\"variable_scale_policy\": \"derivative_residual_finite_difference\"",
                )
                || !output
                    .result_json
                    .contains("\"largest_residual_name\": \"node.equation_1\"")
                || !output
                    .report_spec_json
                    .contains("\"residual_values\": [-2]")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\": [-1]")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_derivative_residual_scale_override.eng to apply residual scales to derivative Newton diagnostics"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_derivative_residual_scale_override.eng applied derivative residual scales"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component derivative residual scale override runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new(
            "tests/runtime/dynamic_component_nonlinear_derivative_timeseries_input_explicit.eng",
        ),
        Path::new(
            "build/test-runtime-dynamic-component-nonlinear-derivative-timeseries-input-explicit",
        ),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_explicit_euler\"")
                || !output.result_json.contains(
                    "parsed nonlinear derivative residual Newton solves with TimeSeries input materialization",
                )
                || !output.result_json.contains("\"final_value\": 4.01123078")
                || !output.result_json.contains("drive_data.drive")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output
                    .result_json
                    .contains("\"largest_residual_name\": \"node.equation_1\"")
                || !output.report_spec_json.contains(
                    "der(node.node.x) * der(node.node.x) + der(node.node.x) eq node.drive + 1",
                )
                || !output.report_spec_json.contains("\"name\": \"node.drive\"")
                || !output.report_spec_json.contains("\"role\": \"input\"")
                || !output.report_spec_json.contains("\"input_count\": 1")
                || !output.report_spec_json.contains("drive_data.drive")
                || !output.report_html.contains("node.node.x=4.011231")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_nonlinear_derivative_timeseries_input_explicit.eng to solve fixed-step explicit nonlinear derivative residuals with a TimeSeries component input"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_nonlinear_derivative_timeseries_input_explicit.eng solved explicit TimeSeries-driven nonlinear derivative residuals"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component nonlinear derivative TimeSeries input explicit runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_adaptive_explicit.eng"),
        Path::new("build/test-runtime-dynamic-component-adaptive-explicit"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_adaptive_heun_source\"")
                || !output.result_json.contains(
                    "dynamic component source solve executed adaptive Heun parsed derivative residual expressions",
                )
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"adaptive_heun_completed\"")
                || !output.result_json.contains("\"name\": \"node.node.x\"")
                || !output.result_json.contains("\"role\": \"state\"")
                || !output.result_json.contains("\"final_value\": 0.18733663")
                || !output
                    .result_json
                    .contains("\"largest_residual_name\": \"adaptive_error_norm\"")
                || !output
                    .report_spec_json
                    .contains("der(node.node.x) + sin(node.node.x) eq 0")
                || !output.report_spec_json.contains("adaptive_heun_accepted")
                || !output.report_spec_json.contains("adaptive_error_norm")
                || !output
                    .report_html
                    .contains("dynamic_component_adaptive_heun_source")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_adaptive_explicit.eng to solve an algebraic-free dynamic component graph with adaptive Heun"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_adaptive_explicit.eng solved algebraic-free adaptive dynamic component RHS"
            );
        }
        Err(error) => {
            eprintln!("dynamic component adaptive explicit runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_adaptive_nonlinear_derivative.eng"),
        Path::new("build/test-runtime-dynamic-component-adaptive-nonlinear-derivative"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_adaptive_heun_source\"")
                || !output
                    .result_json
                    .contains("adaptive Heun parsed nonlinear derivative residual Newton solves")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"adaptive_heun_completed\"")
                || !output.result_json.contains("\"name\": \"node.node.x\"")
                || !output.result_json.contains("\"role\": \"state\"")
                || !output.result_json.contains("\"final_value\": 1.49999900")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output
                    .result_json
                    .contains("\"largest_residual_name\": \"node.equation_1\"")
                || !output
                    .report_spec_json
                    .contains("der(node.node.x) * der(node.node.x) + der(node.node.x) eq 2")
                || !output.report_spec_json.contains("adaptive_heun_accepted")
                || !output.report_spec_json.contains("newton_converged")
                || !output.report_html.contains("node.node.x=1.499999")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_adaptive_nonlinear_derivative.eng to solve adaptive dynamic component nonlinear derivative residuals through Newton"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_adaptive_nonlinear_derivative.eng solved adaptive dynamic component nonlinear derivative residuals"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component adaptive nonlinear-derivative runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new(
            "tests/runtime/dynamic_component_adaptive_nonlinear_derivative_timeseries_input.eng",
        ),
        Path::new(
            "build/test-runtime-dynamic-component-adaptive-nonlinear-derivative-timeseries-input",
        ),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_adaptive_heun_source\"")
                || !output.result_json.contains(
                    "adaptive Heun parsed nonlinear derivative residual Newton solves with TimeSeries input materialization",
                )
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"adaptive_heun_completed\"")
                || !output.result_json.contains("\"final_value\": 4.08325026")
                || !output.result_json.contains("drive_data.drive")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output
                    .result_json
                    .contains("\"largest_residual_name\": \"node.equation_1\"")
                || !output
                    .result_json
                    .contains("\"largest_residual_name\": \"adaptive_error_norm\"")
                || !output.report_spec_json.contains(
                    "der(node.node.x) * der(node.node.x) + der(node.node.x) eq node.drive + 1",
                )
                || !output.report_spec_json.contains("\"name\": \"node.drive\"")
                || !output.report_spec_json.contains("\"role\": \"input\"")
                || !output.report_spec_json.contains("\"input_count\": 1")
                || !output.report_spec_json.contains("drive_data.drive")
                || !output.report_spec_json.contains("adaptive_heun_accepted")
                || !output.report_spec_json.contains("newton_converged")
                || !output.report_html.contains("node.node.x=4.08325")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_adaptive_nonlinear_derivative_timeseries_input.eng to solve adaptive nonlinear derivative residuals with a TimeSeries component input"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_adaptive_nonlinear_derivative_timeseries_input.eng solved adaptive TimeSeries-driven nonlinear derivative residuals"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component adaptive nonlinear-derivative TimeSeries input runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_adaptive_algebraic_output.eng"),
        Path::new("build/test-runtime-dynamic-component-adaptive-algebraic-output"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_adaptive_heun_source\"")
                || !output.result_json.contains(
                    "adaptive Heun parsed derivative residual expressions with algebraic residual materialization",
                )
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"adaptive_heun_completed\"")
                || !output.result_json.contains("\"name\": \"node.node.y\"")
                || !output.result_json.contains("\"role\": \"algebraic\"")
                || !output.result_json.contains("\"final_value\": 0.98250375")
                || !output
                    .report_spec_json
                    .contains("node.node.y eq cos(node.node.x)")
                || !output.report_spec_json.contains("adaptive_heun_accepted")
                || !output.report_html.contains("node.node.y=0.982504")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_adaptive_algebraic_output.eng to solve an adaptive dynamic component RHS with an algebraic output"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_adaptive_algebraic_output.eng solved adaptive dynamic component algebraic output trajectory"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component adaptive algebraic-output runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_adaptive_nonlinear_algebraic.eng"),
        Path::new("build/test-runtime-dynamic-component-adaptive-nonlinear-algebraic"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_adaptive_heun_source\"")
                || !output.result_json.contains(
                    "adaptive Heun parsed derivative residual expressions with algebraic residual materialization",
                )
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"adaptive_heun_completed\"")
                || !output.result_json.contains("\"name\": \"node.node.y\"")
                || !output.result_json.contains("\"role\": \"algebraic\"")
                || !output.result_json.contains("\"final_value\": 0.99122926")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output
                    .result_json
                    .contains("\"largest_residual_name\": \"node.equation_2\"")
                || !output
                    .report_spec_json
                    .contains("node.node.y * node.node.y eq cos(node.node.x)")
                || !output.report_spec_json.contains("adaptive_heun_accepted")
                || !output.report_spec_json.contains("newton_converged")
                || !output.report_html.contains("node.node.y=0.991229")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_adaptive_nonlinear_algebraic.eng to solve an adaptive dynamic component RHS with a Newton algebraic output"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_adaptive_nonlinear_algebraic.eng solved adaptive dynamic component Newton algebraic trajectory"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component adaptive nonlinear-algebraic runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_adaptive_algebraic_residual_scale_override.eng"),
        Path::new(
            "build/test-runtime-dynamic-component-adaptive-algebraic-residual-scale-override",
        ),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_adaptive_heun_source\"")
                || !output.result_json.contains(
                    "adaptive Heun parsed derivative residual expressions with algebraic residual materialization",
                )
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"adaptive_heun_completed\"")
                || !output.result_json.contains("\"name\": \"node.node.y\"")
                || !output.result_json.contains("\"final_value\": 0.99122926")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output
                    .result_json
                    .contains("\"largest_residual_name\": \"node.equation_2\"")
                || !output.report_spec_json.contains(
                    "\"residual_values\": [0.00000001882245681539274]",
                )
                || !output.report_spec_json.contains(
                    "\"normalized_residual_values\": [0.00000000941122840769637]",
                )
                || !output
                    .report_spec_json
                    .contains("node.node.y * node.node.y eq cos(node.node.x)")
                || !output.report_html.contains("node.node.y=0.991229")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_adaptive_algebraic_residual_scale_override.eng to apply algebraic residual scales to adaptive Newton diagnostics"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_adaptive_algebraic_residual_scale_override.eng applied adaptive algebraic residual scales"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component adaptive algebraic residual scale override runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_adaptive_timeseries_input.eng"),
        Path::new("build/test-runtime-dynamic-component-adaptive-timeseries-input"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_adaptive_heun_source\"")
                || !output.result_json.contains(
                    "dynamic component source solve executed adaptive Heun parsed derivative residual expressions with TimeSeries input materialization",
                )
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"adaptive_heun_completed\"")
                || !output.result_json.contains("\"final_value\": 0.87597917")
                || !output.result_json.contains("drive_data.drive")
                || !output
                    .result_json
                    .contains("\"largest_residual_name\": \"adaptive_error_norm\"")
                || !output
                    .report_spec_json
                    .contains("der(node.node.x) + sin(node.node.x) - node.drive eq 0")
                || !output.report_spec_json.contains("\"name\": \"node.drive\"")
                || !output.report_spec_json.contains("\"input_count\": 1")
                || !output.report_spec_json.contains("drive_data.drive")
                || !output.report_spec_json.contains("adaptive_heun_accepted")
                || !output.report_html.contains("node.node.x=0.875979")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_adaptive_timeseries_input.eng to solve an adaptive dynamic component RHS with a TimeSeries component input"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_adaptive_timeseries_input.eng solved adaptive dynamic component TimeSeries input RHS"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component adaptive TimeSeries input runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new(
            "tests/runtime/dynamic_component_adaptive_nonlinear_algebraic_timeseries_input.eng",
        ),
        Path::new(
            "build/test-runtime-dynamic-component-adaptive-nonlinear-algebraic-timeseries-input",
        ),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_adaptive_heun_source\"")
                || !output.result_json.contains(
                    "algebraic residual materialization and TimeSeries input materialization",
                )
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"adaptive_heun_completed\"")
                || !output.result_json.contains("\"name\": \"node.node.y\"")
                || !output.result_json.contains("\"role\": \"algebraic\"")
                || !output.result_json.contains("\"final_value\": 1.24107213")
                || !output.result_json.contains("drive_data.drive")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output
                    .result_json
                    .contains("\"largest_residual_name\": \"node.equation_2\"")
                || !output
                    .report_spec_json
                    .contains("node.node.y * node.node.y eq cos(node.node.x) + node.drive")
                || !output.report_spec_json.contains("\"name\": \"node.drive\"")
                || !output.report_spec_json.contains("\"input_count\": 1")
                || !output.report_spec_json.contains("drive_data.drive")
                || !output.report_spec_json.contains("adaptive_heun_accepted")
                || !output.report_spec_json.contains("newton_converged")
                || !output.report_html.contains("node.node.y=1.241072")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_adaptive_nonlinear_algebraic_timeseries_input.eng to solve adaptive TimeSeries-driven Newton algebraic output"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_adaptive_nonlinear_algebraic_timeseries_input.eng solved adaptive TimeSeries-driven Newton algebraic output"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component adaptive TimeSeries Newton algebraic runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_parameterized_function_explicit.eng"),
        Path::new("build/test-runtime-dynamic-component-parameterized-function-explicit"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_explicit_euler\"")
                || !output.result_json.contains(
                    "dynamic component source solve executed parsed derivative residual expressions",
                )
                || !output.result_json.contains("\"final_value\": 0.29801899")
                || !output
                    .report_spec_json
                    .contains("der(node.node.x) + node.k * sin(node.node.x) eq 0")
                || !output.report_spec_json.contains("\"dependencies\": [\"der(node.node.x)\", \"node.node.x\", \"node.k\"]")
                || !output.report_spec_json.contains("\"status\": \"constructor_override\"")
                || !output.report_html.contains("node.k * sin(node.node.x)")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_parameterized_function_explicit.eng to solve a parameterized function RHS through the parsed explicit dynamic component path"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_parameterized_function_explicit.eng solved parameterized explicit dynamic component function RHS"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component parameterized function explicit runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_input_explicit.eng"),
        Path::new("build/test-runtime-dynamic-component-input-explicit"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_explicit_euler\"")
                || !output.result_json.contains(
                    "dynamic component source solve executed parsed derivative residual expressions",
                )
                || !output.result_json.contains("\"final_value\": 0.69648787")
                || !output
                    .report_spec_json
                    .contains("der(node.node.x) + sin(node.node.x) - node.drive eq 0")
                || !output.report_spec_json.contains("\"name\": \"node.drive\"")
                || !output.report_spec_json.contains("\"role\": \"input\"")
                || !output.report_spec_json.contains("\"input_count\": 1")
                || !output.report_spec_json.contains(
                    "\"dependencies\": [\"der(node.node.x)\", \"node.node.x\", \"node.drive\"]",
                )
                || !output.report_html.contains("node.drive")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_input_explicit.eng to solve an explicit dynamic component RHS with a declared component input"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_input_explicit.eng solved explicit dynamic component input RHS"
            );
        }
        Err(error) => {
            eprintln!("dynamic component input explicit runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_timeseries_input_explicit.eng"),
        Path::new("build/test-runtime-dynamic-component-timeseries-input-explicit"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_explicit_euler\"")
                || !output.result_json.contains(
                    "dynamic component source solve executed parsed derivative residual expressions with TimeSeries input materialization",
                )
                || !output.result_json.contains("\"final_value\": 0.86559777")
                || !output.result_json.contains("drive_data.drive")
                || !output
                    .report_spec_json
                    .contains("der(node.node.x) + sin(node.node.x) - node.drive eq 0")
                || !output.report_spec_json.contains("\"name\": \"node.drive\"")
                || !output.report_spec_json.contains("\"role\": \"input\"")
                || !output.report_spec_json.contains("\"input_count\": 1")
                || !output.report_spec_json.contains("drive_data.drive")
                || !output.report_html.contains("node.node.x=0.865598")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_timeseries_input_explicit.eng to solve an explicit dynamic component RHS with a TimeSeries component input"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_timeseries_input_explicit.eng solved explicit dynamic component TimeSeries input RHS"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component TimeSeries input explicit runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_multistate_function_explicit.eng"),
        Path::new("build/test-runtime-dynamic-component-multistate-function-explicit"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_explicit_euler\"")
                || !output.result_json.contains("\"equation_count\": 2")
                || !output.result_json.contains("\"unknown_count\": 2")
                || !output.result_json.contains("\"name\": \"node.left.x\"")
                || !output.result_json.contains("\"name\": \"node.right.x\"")
                || !output.result_json.contains("\"final_value\": 0.17279477")
                || !output.result_json.contains("\"final_value\": 0.48002640")
                || !output
                    .report_spec_json
                    .contains("der(node.left.x) + sin(node.right.x) eq 0")
                || !output
                    .report_spec_json
                    .contains("der(node.right.x) - cos(node.left.x) / 4 eq 0")
                || !output.report_spec_json.contains("\"state_count\": 2")
                || !output.report_spec_json.contains("\"step_diagnostics\"")
                || !output.report_html.contains("node.left.x=0.172795")
                || !output.report_html.contains("node.right.x=0.480026")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_multistate_function_explicit.eng to solve coupled multi-state function RHS through the parsed explicit dynamic component path"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_multistate_function_explicit.eng solved coupled multi-state explicit dynamic component function RHS"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component multistate function explicit runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_algebraic_output_explicit.eng"),
        Path::new("build/test-runtime-dynamic-component-algebraic-output-explicit"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_explicit_euler\"")
                || !output.result_json.contains(
                    "dynamic component source solve executed parsed derivative residual expressions",
                )
                || !output.result_json.contains("\"equation_count\": 2")
                || !output.result_json.contains("\"unknown_count\": 2")
                || !output.result_json.contains("\"name\": \"node.node.x\"")
                || !output.result_json.contains("\"role\": \"state\"")
                || !output.result_json.contains("\"final_value\": 0.16281192")
                || !output.result_json.contains("\"name\": \"node.node.y\"")
                || !output.result_json.contains("\"role\": \"algebraic\"")
                || !output.result_json.contains("\"final_value\": 0.98677539")
                || !output.result_json.contains("\"largest_residual_name\": \"node.node.y\"")
                || !output
                    .report_spec_json
                    .contains("node.node.y eq cos(node.node.x)")
                || !output.report_spec_json.contains("\"state_count\": 1")
                || !output.report_spec_json.contains("\"step_diagnostics\"")
                || !output.report_html.contains("node.node.y=0.986775")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_algebraic_output_explicit.eng to solve an explicit dynamic component RHS with a selected algebraic output trajectory"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_algebraic_output_explicit.eng solved explicit dynamic component algebraic output trajectory"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component algebraic-output explicit runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_semi_implicit.eng"),
        Path::new("build/test-runtime-dynamic-component-semi-implicit"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_semi_implicit_euler\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"linear_algebraic_converged\"")
                || !output.result_json.contains("\"name\": \"zone.heat.T\"")
                || !output.result_json.contains("\"name\": \"zone.heat.Q\"")
                || !output.result_json.contains("\"step_diagnostics\"")
                || !output.result_json.contains("\"residual_values\"")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\"")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_name\":")
                || !output
                    .report_spec_json
                    .contains("\"largest_residual_source_expression\":")
                || !output
                    .report_html
                    .contains("dynamic_component_assembly_semi_implicit_euler")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_semi_implicit.eng to solve through semi-implicit dynamic component source path"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_semi_implicit.eng solved semi-implicit dynamic component source graph"
            );
        }
        Err(error) => {
            eprintln!("dynamic component semi-implicit runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_timeseries_input_semi_implicit.eng"),
        Path::new("build/test-runtime-dynamic-component-timeseries-input-semi-implicit"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_semi_implicit_euler\"")
                || !output.result_json.contains(
                    "semi-implicit algebraic residual graph with parsed derivative residual expressions and TimeSeries input materialization",
                )
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dynamic_component_fixed_step_completed\"")
                || !output.result_json.contains("\"name\": \"zone.heat.T\"")
                || !output.result_json.contains("\"name\": \"zone.heat.Q\"")
                || !output.report_spec_json.contains("\"name\": \"boundary.q\"")
                || !output.result_json.contains("\"final_value\": 19.99400000")
                || !output.result_json.contains("\"final_value\": -4.00000000")
                || !output.result_json.contains("heat_data.Q_drive")
                || !output.report_spec_json.contains("\"input_count\": 1")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\"")
                || !output
                    .report_spec_json
                    .contains("boundary.heat.Q - (boundary.q)")
                || !output.report_html.contains("zone.heat.T=19.994")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_timeseries_input_semi_implicit.eng to solve a semi-implicit dynamic component graph with a TimeSeries component input"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_timeseries_input_semi_implicit.eng solved semi-implicit dynamic component TimeSeries input graph"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component TimeSeries input semi-implicit runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_function_semi_implicit.eng"),
        Path::new("build/test-runtime-dynamic-component-function-semi-implicit"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_semi_implicit_euler\"")
                || !output.result_json.contains(
                    "semi-implicit algebraic residual graph with parsed derivative residual expressions",
                )
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dynamic_component_fixed_step_completed\"")
                || !output.result_json.contains("\"name\": \"node.node.x\"")
                || !output.result_json.contains("\"name\": \"node.node.balance\"")
                || !output.result_json.contains("\"final_value\": 0.16281192")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"linear_algebraic_converged\"")
                || !output.report_spec_json.contains("sin(node.node.x)")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\"")
                || !output.report_html.contains("node.node.x=0.162812")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_function_semi_implicit.eng to solve a semi-implicit dynamic component graph with parsed function RHS"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_function_semi_implicit.eng solved semi-implicit dynamic component function RHS"
            );
        }
        Err(error) => {
            eprintln!("dynamic component function semi-implicit runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_nonlinear_derivative_semi_implicit.eng"),
        Path::new("build/test-runtime-dynamic-component-nonlinear-derivative-semi-implicit"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_semi_implicit_euler\"")
                || !output.result_json.contains(
                    "semi-implicit algebraic residual graph with parsed nonlinear derivative residual Newton solves",
                )
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dynamic_component_fixed_step_completed\"")
                || !output.result_json.contains("\"name\": \"node.node.x\"")
                || !output.result_json.contains("\"name\": \"node.node.balance\"")
                || !output.result_json.contains("\"final_value\": 1.49999900")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"linear_algebraic_converged\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output
                    .result_json
                    .contains("\"largest_residual_name\": \"node.equation_1\"")
                || !output
                    .report_spec_json
                    .contains("der(node.node.x) * der(node.node.x) + der(node.node.x) eq node.node.balance + 2")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\"")
                || !output.report_html.contains("node.node.x=1.499999")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_nonlinear_derivative_semi_implicit.eng to solve semi-implicit nonlinear derivative residuals through Newton"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_nonlinear_derivative_semi_implicit.eng solved semi-implicit nonlinear derivative residuals"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component nonlinear derivative semi-implicit runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new(
            "tests/runtime/dynamic_component_nonlinear_derivative_timeseries_input_semi_implicit.eng",
        ),
        Path::new(
            "build/test-runtime-dynamic-component-nonlinear-derivative-timeseries-input-semi-implicit",
        ),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_semi_implicit_euler\"")
                || !output.result_json.contains(
                    "semi-implicit algebraic residual graph with parsed nonlinear derivative residual Newton solves and TimeSeries input materialization",
                )
                || !output.result_json.contains("\"final_value\": 4.01123078")
                || !output.result_json.contains("drive_data.drive")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"linear_algebraic_converged\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output
                    .result_json
                    .contains("\"largest_residual_name\": \"node.equation_1\"")
                || !output.report_spec_json.contains(
                    "der(node.node.x) * der(node.node.x) + der(node.node.x) eq node.node.balance + node.drive + 1",
                )
                || !output.report_spec_json.contains("\"name\": \"node.drive\"")
                || !output.report_spec_json.contains("\"role\": \"input\"")
                || !output.report_spec_json.contains("\"input_count\": 1")
                || !output.report_spec_json.contains("drive_data.drive")
                || !output.report_html.contains("node.node.x=4.011231")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_nonlinear_derivative_timeseries_input_semi_implicit.eng to solve semi-implicit nonlinear derivative residuals with a TimeSeries component input"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_nonlinear_derivative_timeseries_input_semi_implicit.eng solved semi-implicit TimeSeries-driven nonlinear derivative residuals"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component nonlinear derivative TimeSeries input semi-implicit runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_algebraic_output_semi_implicit.eng"),
        Path::new("build/test-runtime-dynamic-component-algebraic-output-semi-implicit"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_semi_implicit_euler\"")
                || !output
                    .result_json
                    .contains("semi-implicit Newton algebraic residuals")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dynamic_component_fixed_step_completed\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output.result_json.contains("\"name\": \"node.node.y\"")
                || !output.result_json.contains("\"final_value\": 0.98677539")
                || !output
                    .report_spec_json
                    .contains("node.node.y eq cos(node.node.x)")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\"")
                || !output.report_html.contains("node.node.y=0.986775")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_algebraic_output_semi_implicit.eng to solve a semi-implicit algebraic output trajectory"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_algebraic_output_semi_implicit.eng solved semi-implicit algebraic output trajectory"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component algebraic output semi-implicit runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_parameterized_function_semi_implicit.eng"),
        Path::new("build/test-runtime-dynamic-component-parameterized-function-semi-implicit"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_semi_implicit_euler\"")
                || !output.result_json.contains(
                    "semi-implicit algebraic residual graph with parsed derivative residual expressions",
                )
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dynamic_component_fixed_step_completed\"")
                || !output.result_json.contains("\"name\": \"node.node.x\"")
                || !output.result_json.contains("\"name\": \"node.node.balance\"")
                || !output.result_json.contains("\"final_value\": 0.29801899")
                || !output.report_spec_json.contains("node.k * sin(node.node.x)")
                || !output.report_spec_json.contains("\"node.k\"")
                || !output.report_spec_json.contains("\"status\": \"constructor_override\"")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\"")
                || !output.report_html.contains("node.node.x=0.298019")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_parameterized_function_semi_implicit.eng to solve a semi-implicit parameterized function RHS"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_parameterized_function_semi_implicit.eng solved semi-implicit parameterized function RHS"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component parameterized function semi-implicit runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_nonlinear_algebraic_semi_implicit.eng"),
        Path::new("build/test-runtime-dynamic-component-nonlinear-algebraic-semi-implicit"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_semi_implicit_euler\"")
                || !output
                    .result_json
                    .contains("semi-implicit Newton algebraic residuals")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dynamic_component_fixed_step_completed\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output.result_json.contains("\"name\": \"node.node.x\"")
                || !output
                    .result_json
                    .contains("\"name\": \"boundary.node.balance\"")
                || !output.result_json.contains("\"final_value\": 1.38154357")
                || !output.result_json.contains("\"final_value\": 1.17539082")
                || !output
                    .report_spec_json
                    .contains("boundary.node.balance * boundary.node.balance")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\"")
                || !output.report_html.contains("node.node.x=1.381544")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_nonlinear_algebraic_semi_implicit.eng to solve a semi-implicit dynamic component graph with Newton algebraic residuals"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_nonlinear_algebraic_semi_implicit.eng solved semi-implicit dynamic component Newton algebraic residuals"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component nonlinear algebraic semi-implicit runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/runtime/dynamic_component_algebraic_residual_scale_override.eng"),
        Path::new("build/test-runtime-dynamic-component-algebraic-residual-scale-override"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_semi_implicit_euler\"")
                || !output
                    .result_json
                    .contains("semi-implicit Newton algebraic residuals")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dynamic_component_fixed_step_completed\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output.result_json.contains("\"final_value\": 1.38154357")
                || !output.result_json.contains("\"final_value\": 1.17539082")
                || !output
                    .report_spec_json
                    .contains("\"residual_values\": [0, 0, 0.0000000013371082019375535]")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\": [0, 0, 0.0000000006685541009687768]")
                || !output
                    .report_spec_json
                    .contains("boundary.node.balance * boundary.node.balance")
                || !output.report_html.contains("node.node.x=1.381544")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_algebraic_residual_scale_override.eng to apply algebraic residual scales to semi-implicit Newton diagnostics"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_algebraic_residual_scale_override.eng applied semi-implicit algebraic residual scales"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component algebraic residual scale override runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new(
            "tests/runtime/dynamic_component_nonlinear_algebraic_timeseries_input_semi_implicit.eng",
        ),
        Path::new(
            "build/test-runtime-dynamic-component-nonlinear-algebraic-timeseries-input-semi-implicit",
        ),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"computed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_semi_implicit_euler\"")
                || !output.result_json.contains(
                    "semi-implicit Newton algebraic residuals with parsed derivative residual expressions and TimeSeries input materialization",
                )
                || !output.result_json.contains("drive_data.drive")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dynamic_component_fixed_step_completed\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_converged\"")
                || !output.result_json.contains("\"name\": \"node.node.x\"")
                || !output
                    .result_json
                    .contains("\"name\": \"boundary.node.balance\"")
                || !output.result_json.contains("\"final_value\": 7.23836070")
                || !output.result_json.contains("\"final_value\": 2.85278125")
                || !output
                    .report_spec_json
                    .contains("boundary.node.balance * boundary.node.balance - (boundary.node.x + boundary.drive)")
                || !output
                    .report_spec_json
                    .contains("\"dependencies\": [\"boundary.node.balance\", \"boundary.node.x\", \"boundary.drive\"]")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\"")
                || !output.report_html.contains("node.node.x=7.238361")
            {
                eprintln!(
                    "expected tests/runtime/dynamic_component_nonlinear_algebraic_timeseries_input_semi_implicit.eng to solve a semi-implicit dynamic component graph with TimeSeries inputs and Newton algebraic residuals"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/dynamic_component_nonlinear_algebraic_timeseries_input_semi_implicit.eng solved semi-implicit TimeSeries-input Newton algebraic residuals"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component nonlinear algebraic TimeSeries-input semi-implicit runtime fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    let thermal_assembly_report = match check_file(
        "examples/internal/21_thermal_component_assembly/main.eng",
        &CheckOptions::default(),
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let thermal_assembly_jit_plan = eng_jit::plan_for_report(&thermal_assembly_report);
    let thermal_assembly_bench_smoke = jit_bench_json(
        "examples/internal/21_thermal_component_assembly/main.eng",
        1,
        &thermal_assembly_report,
        &thermal_assembly_jit_plan,
        &[BenchRun {
            iteration: 1,
            elapsed_ms: 1.0,
            result_path: "build/jit-bench/iter-000/result/result.engres".to_owned(),
        }],
    );
    if !thermal_assembly_jit_plan
        .candidates
        .iter()
        .any(|candidate| {
            candidate.kind == "component_residual_graph"
                && candidate.lowering_status == "lowerable_to_numeric_kernel_plan"
                && candidate.estimate.input_count == 4
                && candidate.estimate.output_count == 4
                && candidate
                    .operations
                    .iter()
                    .any(|operation| operation == "finite_difference_jacobian_ready")
        })
        || !thermal_assembly_jit_plan
            .candidates
            .iter()
            .any(|candidate| {
                candidate.kind == "component_residual_jacobian"
                    && candidate.lowering_status == "lowerable_to_numeric_kernel_plan"
                    && candidate.estimate.input_count == 4
                    && candidate.estimate.output_count == 16
                    && candidate
                        .operations
                        .iter()
                        .any(|operation| operation == "store_dense_jacobian:4x4")
            })
        || !thermal_assembly_jit_plan
            .candidates
            .iter()
            .any(|candidate| {
                candidate.kind == "component_newton_step"
                    && candidate.lowering_status == "lowerable_to_numeric_kernel_plan"
                    && candidate.estimate.input_count == 20
                    && candidate.estimate.output_count == 4
                    && candidate
                        .operations
                        .iter()
                        .any(|operation| operation == "solve_newton_step:4")
            })
        || !jit_bench_has_target(
            &thermal_assembly_bench_smoke,
            "residual_evaluation",
            "covered_by_current_source",
            Some("component_residual_jacobian"),
        )
        || !jit_bench_has_target(
            &thermal_assembly_bench_smoke,
            "component_graph_solver_small_case",
            "covered_by_current_source",
            Some("component_newton_step"),
        )
    {
        eprintln!(
            "expected internal thermal component assembly fixture to expose lowerable component residual, Jacobian, Newton-step kernel candidates, and benchmark target coverage"
        );
        return ExitCode::from(2);
    }
    println!(
        "ok: examples/internal/21_thermal_component_assembly/main.eng produced component residual, Jacobian, and Newton-step kernel candidates"
    );
    match run_file(
        Path::new("examples/internal/22_multi_domain_boundary_solve/main.eng"),
        Path::new("build/test-multi-domain-boundary-solve"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"solved_linear\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dense_linear_residual_graph\"")
                || !output.result_json.contains("\"equation_count\": 12")
                || !output.result_json.contains("\"unknown_count\": 12")
                || !output.result_json.contains("\"residual_norm\": 0.00000000")
                || !output
                    .result_json
                    .contains("\"name\": \"SupplyPipe.outlet.m_dot\"")
                || !output.result_json.contains("\"value\": -0.20000000")
                || !output.result_json.contains("\"name\": \"ShaftB.shaft.P\"")
                || !output.result_json.contains("\"value\": -100.00000000")
                || !output.result_json.contains("\"largest_residuals\"")
                || !output.report_spec_json.contains("\"domain_count\": 3")
                || !output.report_spec_json.contains("\"multi_domain_preview\"")
                || !output
                    .report_spec_json
                    .contains("\"not_production_multi_domain\"")
                || !output
                    .report_spec_json
                    .contains("\"solver_plan\": \"dense_linear_residual_graph\"")
                || !output.report_html.contains("dense_linear_residual_graph")
                || !output.report_html.contains("multi_domain_preview")
            {
                eprintln!(
                    "expected internal multi-domain boundary fixture to solve a square residual graph across Thermal, Fluid, and MechanicalNode domains"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/22_multi_domain_boundary_solve/main.eng solved a small multi-domain boundary residual graph"
            );
        }
        Err(error) => {
            eprintln!("multi-domain boundary solve fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("examples/internal/23_component_boundary_singular/main.eng"),
        Path::new("build/test-component-boundary-singular"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"linear_solve_failed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dense_linear_residual_graph\"")
                || !output.result_json.contains("\"E-LINEAR-SINGULAR\"")
                || !output.report_spec_json.contains("\"failure_artifact\"")
                || !output
                    .report_spec_json
                    .contains("\"failure_code\": \"E-LINEAR-SINGULAR\"")
                || !output
                    .report_spec_json
                    .contains("\"failure_reason\": \"linear system is singular")
                || !output
                    .report_spec_json
                    .contains("\"convergence_status\": \"linear_failed\"")
                || !output.report_html.contains("linear_solve_failed")
                || !output.report_html.contains("E-LINEAR-SINGULAR")
            {
                eprintln!(
                    "expected singular component boundary fixture to report a dense linear solve failure artifact"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/23_component_boundary_singular/main.eng reported singular component residual graph failure"
            );
        }
        Err(error) => {
            eprintln!("component boundary singular fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("examples/diagnostics/error_messages/fixed_point_nonconvergence.eng"),
        Path::new("build/test-fixed-point-nonconvergence"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"fixed_point_not_converged\"")
                || !output
                    .result_json
                    .contains("\"method\": \"fixed_point_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"fixed_point_not_converged\"")
                || !output.result_json.contains("\"iteration_count\": 3")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-FIXED-POINT-NONCONVERGENCE\"")
                || !output.report_spec_json.contains("\"failure_artifact\"")
                || !output.report_html.contains("fixed_point_not_converged")
                || !output.report_html.contains("E-FIXED-POINT-NONCONVERGENCE")
            {
                eprintln!(
                    "expected fixed-point nonconvergence fixture to report a SolverFailure artifact"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/diagnostics/error_messages/fixed_point_nonconvergence.eng reported fixed-point nonconvergence"
            );
        }
        Err(error) => {
            eprintln!("fixed-point nonconvergence fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/algebraic_singular_system.eng"),
        Path::new("build/test-diagnostics-algebraic-singular-system"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"linear_solve_failed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dense_linear_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-LINEAR-SINGULAR\"")
                || !output.report_html.contains("linear_solve_failed")
            {
                eprintln!(
                    "expected tests/diagnostics/algebraic_singular_system.eng to report a dense linear SolverFailure artifact"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/algebraic_singular_system.eng reported dense linear singular failure"
            );
        }
        Err(error) => {
            eprintln!("algebraic singular diagnostics fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/algebraic_ill_conditioned_system.eng"),
        Path::new("build/test-diagnostics-algebraic-ill-conditioned-system"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"linear_solve_failed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dense_linear_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-LINEAR-ILL-CONDITIONED\"")
                || !output
                    .report_spec_json
                    .contains("\"failure_reason\": \"linear system is ill-conditioned")
                || !output.report_html.contains("E-LINEAR-ILL-CONDITIONED")
            {
                eprintln!(
                    "expected tests/diagnostics/algebraic_ill_conditioned_system.eng to report a dense linear ill-conditioned SolverFailure artifact"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/algebraic_ill_conditioned_system.eng reported dense linear ill-conditioned failure"
            );
        }
        Err(error) => {
            eprintln!("algebraic ill-conditioned diagnostics fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/source_ode_output_algebraic_loop.eng"),
        Path::new("build/test-diagnostics-source-ode-output-algebraic-loop"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"failed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"rk4_fixed_step\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-RHS-OUTPUT-ALGEBRAIC-LOOP\"")
                || !output
                    .result_json
                    .contains("a depends on [b]; b depends on [a]")
                || !output.result_json.contains("\"source_equations\"")
                || !output.result_json.contains("\"kind\": \"derivative\"")
                || !output.result_json.contains("\"target\": \"x\"")
                || !output
                    .report_spec_json
                    .contains("\"failure_code\": \"E-RHS-OUTPUT-ALGEBRAIC-LOOP\"")
                || !output.report_spec_json.contains("\"source_equations\"")
                || !output.review_json.contains("\"source_equations\"")
                || !output.report_html.contains("Source Equations")
                || !output.report_html.contains("derivative:x")
                || !output.report_html.contains("E-RHS-OUTPUT-ALGEBRAIC-LOOP")
            {
                eprintln!(
                    "expected tests/diagnostics/source_ode_output_algebraic_loop.eng to report an explicit source output algebraic loop failure"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/source_ode_output_algebraic_loop.eng reported source output algebraic loop"
            );
        }
        Err(error) => {
            eprintln!("source output algebraic loop diagnostics fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/fixed_point_nonconvergence.eng"),
        Path::new("build/test-diagnostics-fixed-point-nonconvergence"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"fixed_point_not_converged\"")
                || !output
                    .result_json
                    .contains("\"method\": \"fixed_point_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-FIXED-POINT-NONCONVERGENCE\"")
                || !output.report_html.contains("fixed_point_not_converged")
            {
                eprintln!(
                    "expected tests/diagnostics/fixed_point_nonconvergence.eng to report a fixed-point SolverFailure artifact"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/fixed_point_nonconvergence.eng reported fixed-point nonconvergence"
            );
        }
        Err(error) => {
            eprintln!("fixed-point nonconvergence diagnostics fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/source_system_fixed_point_nonconvergence.eng"),
        Path::new("build/test-diagnostics-source-system-fixed-point-nonconvergence"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"assembly\": \"StaticDivergingFixedPointSourceSystem\"")
                || !output
                    .result_json
                    .contains("\"status\": \"fixed_point_not_converged\"")
                || !output
                    .result_json
                    .contains("\"method\": \"fixed_point_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"fixed_point_not_converged\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-FIXED-POINT-NONCONVERGENCE\"")
                || !output.result_json.contains("\"largest_residuals\"")
                || !output.result_json.contains(
                    "\"variable_scale_policy\": \"unit_default_from_fixed_point_unknowns\"",
                )
                || !output
                    .report_spec_json
                    .contains("\"failure_code\": \"E-FIXED-POINT-NONCONVERGENCE\"")
                || !output
                    .report_html
                    .contains("StaticDivergingFixedPointSourceSystem")
                || !output.report_html.contains("fixed_point_not_converged")
                || !output.report_html.contains("E-FIXED-POINT-NONCONVERGENCE")
            {
                eprintln!(
                    "expected tests/diagnostics/source_system_fixed_point_nonconvergence.eng to report source-system fixed-point nonconvergence"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/source_system_fixed_point_nonconvergence.eng reported source-system fixed-point nonconvergence"
            );
        }
        Err(error) => {
            eprintln!(
                "source-system fixed-point nonconvergence diagnostics fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/newton_nonconvergence.eng"),
        Path::new("build/test-diagnostics-newton-nonconvergence"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"newton_not_converged\"")
                || !output
                    .result_json
                    .contains("\"method\": \"newton_source_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-NEWTON-NONCONVERGENCE\"")
                || !output.result_json.contains("\"largest_residuals\"")
                || !output.report_html.contains("E-NEWTON-NONCONVERGENCE")
            {
                eprintln!(
                    "expected tests/diagnostics/newton_nonconvergence.eng to report a Newton SolverFailure artifact"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/newton_nonconvergence.eng reported Newton nonconvergence"
            );
        }
        Err(error) => {
            eprintln!("Newton nonconvergence diagnostics fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/source_system_newton_nonconvergence.eng"),
        Path::new("build/test-diagnostics-source-system-newton-nonconvergence"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"assembly\": \"StaticNewtonNonconvergenceSourceSystem\"")
                || !output
                    .result_json
                    .contains("\"status\": \"newton_not_converged\"")
                || !output
                    .result_json
                    .contains("\"method\": \"newton_source_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-NEWTON-NONCONVERGENCE\"")
                || !output.result_json.contains("\"largest_residuals\"")
                || !output
                    .report_html
                    .contains("StaticNewtonNonconvergenceSourceSystem")
                || !output.report_html.contains("E-NEWTON-NONCONVERGENCE")
            {
                eprintln!(
                    "expected tests/diagnostics/source_system_newton_nonconvergence.eng to report a source-system Newton SolverFailure artifact"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/source_system_newton_nonconvergence.eng reported source-system Newton nonconvergence"
            );
        }
        Err(error) => {
            eprintln!("source-system Newton nonconvergence diagnostics fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/newton_initial_vector_mismatch.eng"),
        Path::new("build/test-diagnostics-newton-initial-vector-mismatch"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"failed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"newton_source_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_source_failed\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-NEWTON-SOURCE-INITIAL-LAYOUT\"")
                || !output
                    .result_json
                    .contains("provided 2 initial value(s) for 4 variable(s)")
                || !output
                    .report_html
                    .contains("E-NEWTON-SOURCE-INITIAL-LAYOUT")
            {
                eprintln!(
                    "expected tests/diagnostics/newton_initial_vector_mismatch.eng to report an initial-vector SolverFailure artifact"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/newton_initial_vector_mismatch.eng reported Newton initial vector mismatch"
            );
        }
        Err(error) => {
            eprintln!("Newton initial vector mismatch diagnostics fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/residual_scale_invalid.eng"),
        Path::new("build/test-diagnostics-residual-scale-invalid"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"failed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"newton_source_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_source_failed\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-SOURCE-RESIDUAL-SCALE\"")
                || !output.result_json.contains("\"failure_artifact\"")
                || !output.report_html.contains("E-SOURCE-RESIDUAL-SCALE")
            {
                eprintln!(
                    "expected tests/diagnostics/residual_scale_invalid.eng to report a residual-scale SolverFailure artifact"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/residual_scale_invalid.eng reported residual scale validation failure"
            );
        }
        Err(error) => {
            eprintln!("residual scale invalid diagnostics fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/linear_residual_scale_invalid.eng"),
        Path::new("build/test-diagnostics-linear-residual-scale-invalid"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"failed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dense_linear_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dense_linear_source_failed\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-SOURCE-RESIDUAL-SCALE\"")
                || !output.report_html.contains("E-SOURCE-RESIDUAL-SCALE")
            {
                eprintln!(
                    "expected tests/diagnostics/linear_residual_scale_invalid.eng to report a dense-linear residual-scale SolverFailure artifact"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/linear_residual_scale_invalid.eng reported dense linear residual scale validation failure"
            );
        }
        Err(error) => {
            eprintln!("dense linear residual scale invalid diagnostics fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/fixed_point_residual_scale_invalid.eng"),
        Path::new("build/test-diagnostics-fixed-point-residual-scale-invalid"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"failed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"fixed_point_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"fixed_point_source_failed\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-SOURCE-RESIDUAL-SCALE\"")
                || !output.report_html.contains("E-SOURCE-RESIDUAL-SCALE")
            {
                eprintln!(
                    "expected tests/diagnostics/fixed_point_residual_scale_invalid.eng to report a fixed-point residual-scale SolverFailure artifact"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/fixed_point_residual_scale_invalid.eng reported fixed-point residual scale validation failure"
            );
        }
        Err(error) => {
            eprintln!("fixed-point residual scale invalid diagnostics fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/dynamic_component_residual_scale_invalid.eng"),
        Path::new("build/test-diagnostics-dynamic-component-residual-scale-invalid"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"failed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_explicit_euler\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dynamic_component_source_failed\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-SOURCE-RESIDUAL-SCALE\"")
                || !output.report_html.contains("E-SOURCE-RESIDUAL-SCALE")
            {
                eprintln!(
                    "expected tests/diagnostics/dynamic_component_residual_scale_invalid.eng to report a dynamic component residual-scale SolverFailure artifact"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/dynamic_component_residual_scale_invalid.eng reported dynamic component residual scale validation failure"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component residual scale invalid diagnostics fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/dynamic_component_affine_residual_scale_unsupported.eng"),
        Path::new("build/test-diagnostics-dynamic-component-affine-residual-scale-unsupported"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"failed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_explicit_euler\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dynamic_component_source_failed\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-SOURCE-RESIDUAL-SCALE\"")
                || !output
                    .result_json
                    .contains("require derivative Newton fallback")
                || !output.report_html.contains("E-SOURCE-RESIDUAL-SCALE")
            {
                eprintln!(
                    "expected tests/diagnostics/dynamic_component_affine_residual_scale_unsupported.eng to reject affine dynamic residual scale overrides"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/dynamic_component_affine_residual_scale_unsupported.eng rejected affine dynamic residual scale overrides"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component affine residual scale unsupported diagnostics fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/dae_inconsistent_initial.eng"),
        Path::new("build/test-diagnostics-dae-inconsistent-initial"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"failed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"implicit_euler_dae_source_residual_graph\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"dae_source_failed\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-DAE-INCONSISTENT-INITIAL-CONDITIONS\"")
                || !output
                    .report_html
                    .contains("E-DAE-INCONSISTENT-INITIAL-CONDITIONS")
            {
                eprintln!(
                    "expected tests/diagnostics/dae_inconsistent_initial.eng to report a DAE consistency SolverFailure artifact"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/dae_inconsistent_initial.eng reported DAE inconsistent initial conditions"
            );
        }
        Err(error) => {
            eprintln!("DAE inconsistent initial diagnostics fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/dynamic_component_nonconvergence.eng"),
        Path::new("build/test-diagnostics-dynamic-component-nonconvergence"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"failed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_semi_implicit_euler\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"algebraic_solve_failed\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-LINEAR-SINGULAR\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"linear_algebraic_solve_failed\"")
                || !output.report_html.contains("E-LINEAR-SINGULAR")
            {
                eprintln!(
                    "expected tests/diagnostics/dynamic_component_nonconvergence.eng to report a dynamic component algebraic failure artifact"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/dynamic_component_nonconvergence.eng reported dynamic component algebraic failure"
            );
        }
        Err(error) => {
            eprintln!("dynamic component diagnostics fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/dynamic_component_nonlinear_derivative_nonconvergence.eng"),
        Path::new("build/test-diagnostics-dynamic-component-nonlinear-derivative-nonconvergence"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"failed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_explicit_euler\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-NEWTON-NONCONVERGENCE\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_not_converged\"")
                || !output.result_json.contains("\"step_diagnostics\"")
                || !output.result_json.contains("\"failure_artifact\"")
                || !output
                    .report_spec_json
                    .contains("\"diagnostic_code\": \"E-NEWTON-NONCONVERGENCE\"")
                || !output
                    .report_spec_json
                    .contains("der(node.node.x) * der(node.node.x) + der(node.node.x) eq 2")
                || !output
                    .report_html
                    .contains("steps=2 failed@2 E-NEWTON-NONCONVERGENCE")
            {
                eprintln!(
                    "expected tests/diagnostics/dynamic_component_nonlinear_derivative_nonconvergence.eng to report fixed-step component Newton derivative nonconvergence"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/dynamic_component_nonlinear_derivative_nonconvergence.eng reported fixed-step component Newton derivative nonconvergence"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component nonlinear derivative nonconvergence diagnostics fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("tests/diagnostics/dynamic_component_nonlinear_algebraic_nonconvergence.eng"),
        Path::new("build/test-diagnostics-dynamic-component-nonlinear-algebraic-nonconvergence"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"failed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_assembly_semi_implicit_euler\"")
                || !output
                    .result_json
                    .contains("semi-implicit Newton algebraic residuals")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"algebraic_solve_failed\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_not_converged\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-NEWTON-NONCONVERGENCE\"")
                || !output
                    .report_spec_json
                    .contains("\"normalized_residual_values\"")
                || !output.report_html.contains("E-NEWTON-NONCONVERGENCE")
            {
                eprintln!(
                    "expected tests/diagnostics/dynamic_component_nonlinear_algebraic_nonconvergence.eng to report a semi-implicit Newton algebraic failure artifact"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/dynamic_component_nonlinear_algebraic_nonconvergence.eng reported semi-implicit Newton algebraic nonconvergence"
            );
        }
        Err(error) => {
            eprintln!("dynamic component nonlinear algebraic diagnostics fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new(
            "tests/diagnostics/dynamic_component_adaptive_nonlinear_algebraic_nonconvergence.eng",
        ),
        Path::new(
            "build/test-diagnostics-dynamic-component-adaptive-nonlinear-algebraic-nonconvergence",
        ),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"failed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_adaptive_heun\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-NEWTON-NONCONVERGENCE\"")
                || !output
                    .report_spec_json
                    .contains("\"diagnostic_code\": \"E-NEWTON-NONCONVERGENCE\"")
                || !output.report_html.contains("E-NEWTON-NONCONVERGENCE")
            {
                eprintln!(
                    "expected tests/diagnostics/dynamic_component_adaptive_nonlinear_algebraic_nonconvergence.eng to report adaptive component Newton algebraic nonconvergence"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/dynamic_component_adaptive_nonlinear_algebraic_nonconvergence.eng reported adaptive component Newton algebraic nonconvergence"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component adaptive nonlinear algebraic nonconvergence diagnostics fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new(
            "tests/diagnostics/dynamic_component_adaptive_nonlinear_derivative_nonconvergence.eng",
        ),
        Path::new(
            "build/test-diagnostics-dynamic-component-adaptive-nonlinear-derivative-nonconvergence",
        ),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.result_json.contains("\"status\": \"failed\"")
                || !output
                    .result_json
                    .contains("\"method\": \"dynamic_component_adaptive_heun\"")
                || !output
                    .result_json
                    .contains("\"failure_code\": \"E-NEWTON-NONCONVERGENCE\"")
                || !output
                    .result_json
                    .contains("\"convergence_status\": \"newton_not_converged\"")
                || !output.result_json.contains("\"step_diagnostics\"")
                || !output.result_json.contains("\"failure_artifact\"")
                || !output.result_json.contains("final residual norm")
                || !output
                    .report_spec_json
                    .contains("\"diagnostic_code\": \"E-NEWTON-NONCONVERGENCE\"")
                || !output
                    .report_spec_json
                    .contains("\"convergence_status\": \"newton_not_converged\"")
                || !output
                    .report_spec_json
                    .contains("der(node.node.x) * der(node.node.x) + der(node.node.x) eq 2")
                || !output
                    .report_html
                    .contains("steps=2 failed@2 E-NEWTON-NONCONVERGENCE")
            {
                eprintln!(
                    "expected tests/diagnostics/dynamic_component_adaptive_nonlinear_derivative_nonconvergence.eng to report adaptive component Newton derivative nonconvergence"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/diagnostics/dynamic_component_adaptive_nonlinear_derivative_nonconvergence.eng reported adaptive component Newton derivative nonconvergence"
            );
        }
        Err(error) => {
            eprintln!(
                "dynamic component adaptive nonlinear derivative nonconvergence diagnostics fixture failed: {error}"
            );
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("examples/internal/24_component_boundary_overdetermined/main.eng"),
        Path::new("build/test-component-boundary-overdetermined"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"status\": \"not_solved_overdetermined\"")
                || !output
                    .result_json
                    .contains("\"method\": \"linear_residual_graph_shape_check\"")
                || !output.result_json.contains("\"E-ASSEMBLY-OVERDETERMINED\"")
                || !output.report_spec_json.contains("\"failure_artifact\"")
                || !output
                    .report_spec_json
                    .contains("\"failure_code\": \"E-ASSEMBLY-OVERDETERMINED\"")
                || !output
                    .report_spec_json
                    .contains("\"failure_reason\": \"assembly has more equations than unknowns")
                || !output.report_spec_json.contains(
                    "\"convergence_status\": \"linear_residual_not_attempted_overdetermined\"",
                )
                || !output.report_html.contains("not_solved_overdetermined")
                || !output.report_html.contains("E-ASSEMBLY-OVERDETERMINED")
            {
                eprintln!(
                    "expected overdetermined component boundary fixture to report a limitation artifact"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/24_component_boundary_overdetermined/main.eng reported overdetermined component residual graph limitation"
            );
        }
        Err(error) => {
            eprintln!("component boundary overdetermined fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("examples/internal/25_component_behavior_nodes/main.eng"),
        Path::new("build/test-component-behavior-nodes"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.report_spec_json.contains("\"behavior_nodes\"")
                || !output
                    .report_spec_json
                    .contains("\"status\": \"declared_not_executed\"")
                || !output
                    .report_spec_json
                    .contains("\"signal\": \"temperature_signal\"")
                || !output.report_spec_json.contains("\"signal\": \"out.Q\"")
                || !output.report_spec_json.contains("\"contract_inputs\"")
                || !output
                    .report_spec_json
                    .contains("\"component_local_signal_resolved\"")
                || !output
                    .report_spec_json
                    .contains("\"quantity_kind\": \"AbsoluteTemperature\"")
                || !output
                    .report_spec_json
                    .contains("\"quantity_kind\": \"HeatRate\"")
                || !output
                    .report_spec_json
                    .contains("\"status\": \"typed_identity_contract\"")
                || !output.report_spec_json.contains("\"diagnostic_channels\"")
                || !output
                    .report_spec_json
                    .contains("\"predictor_valid_range_warning\"")
                || !output.report_html.contains("Component Behavior")
                || !output
                    .report_html
                    .contains("inputs=input:temperature_signal")
                || !output
                    .report_html
                    .contains("diagnostics=predictor_valid_range_warning")
                || !output.report_spec_json.contains(
                    "behavior nodes are declared, but this component solve path does not execute the behavior graph",
                )
                || !output.report_html.contains("declared, not executed")
                || !output
                    .report_html
                    .contains("finite difference on execution")
                || !output
                    .report_html
                    .contains("safe/repro policy checked on execution")
                || !output
                    .report_html
                    .contains("runtime diagnostics unavailable")
            {
                eprintln!(
                    "expected component behavior fixture to expose delay, Predictor, and external behavior nodes"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/25_component_behavior_nodes/main.eng exposed component behavior node artifacts"
            );
        }
        Err(error) => {
            eprintln!("component behavior nodes fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    if let Err(message) = solver_algorithm_smoke() {
        eprintln!("{message}");
        return ExitCode::from(2);
    }
    println!(
        "ok: solver API linear residual, fixed/adaptive ODE, fixed-point, nonlinear Newton, implicit-Euler DAE, and dynamic component assembly smokes produced numeric results and failure artifacts"
    );
    if let Err(message) = solver_behavior_smoke() {
        eprintln!("{message}");
        return ExitCode::from(2);
    }
    println!(
        "ok: solver API delay, Predictor, and external behavior smokes produced numeric results, warnings, and failure artifacts"
    );

    let bad = match check_file(
        "examples/diagnostics/error_messages/unit_mismatch.eng",
        &CheckOptions::default(),
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    if !bad.has_errors() {
        eprintln!("expected unit_mismatch.eng to fail");
        return ExitCode::from(2);
    }
    println!("ok: examples/diagnostics/error_messages/unit_mismatch.eng produced diagnostics");

    let ambiguous = match check_file(
        "examples/diagnostics/error_messages/ambiguous_power.eng",
        &CheckOptions::default(),
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    if ambiguous.diagnostic_count(eng_compiler::Severity::Warning) == 0 {
        eprintln!("expected ambiguous_power.eng to produce a warning");
        return ExitCode::from(2);
    }
    println!("ok: examples/diagnostics/error_messages/ambiguous_power.eng produced warning");

    let heat_rate_sum = match check_file(
        "examples/diagnostics/error_messages/heat_rate_sum.eng",
        &CheckOptions::default(),
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    if !heat_rate_sum
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "W-STATS-SUM-001")
    {
        eprintln!("expected heat_rate_sum.eng to produce W-STATS-SUM-001");
        return ExitCode::from(2);
    }
    println!("ok: examples/diagnostics/error_messages/heat_rate_sum.eng produced warning");

    let missing_column = match check_file(
        "examples/diagnostics/error_messages/missing_csv_column.eng",
        &CheckOptions::default(),
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    if !missing_column.has_errors() {
        eprintln!("expected missing_csv_column.eng to fail");
        return ExitCode::from(2);
    }
    println!("ok: examples/diagnostics/error_messages/missing_csv_column.eng produced diagnostics");

    let eq_boolean = match check_file(
        "examples/diagnostics/error_messages/eq_boolean.eng",
        &CheckOptions::default(),
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    if !eq_boolean
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "E-EQ-BOOL-001")
    {
        eprintln!("expected eq_boolean.eng to produce E-EQ-BOOL-001");
        return ExitCode::from(2);
    }
    println!("ok: examples/diagnostics/error_messages/eq_boolean.eng produced diagnostics");

    let equation_unit_mismatch = match check_file(
        "examples/diagnostics/error_messages/equation_unit_mismatch.eng",
        &CheckOptions::default(),
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    if !equation_unit_mismatch
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "E-EQ-UNIT-001")
    {
        eprintln!("expected equation_unit_mismatch.eng to produce E-EQ-UNIT-001");
        return ExitCode::from(2);
    }
    println!(
        "ok: examples/diagnostics/error_messages/equation_unit_mismatch.eng produced diagnostics"
    );

    let port_domain_mismatch = match check_file(
        "examples/diagnostics/error_messages/port_domain_mismatch.eng",
        &CheckOptions::default(),
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    if !port_domain_mismatch
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "E-CONNECT-DOMAIN-MISMATCH")
    {
        eprintln!("expected port_domain_mismatch.eng to produce E-CONNECT-DOMAIN-MISMATCH");
        return ExitCode::from(2);
    }
    println!(
        "ok: examples/diagnostics/error_messages/port_domain_mismatch.eng produced diagnostics"
    );

    for (fixture, expected_code) in [
        (
            "examples/diagnostics/error_messages/missing_derivative_equation.eng",
            "E-SYS-DER-MISSING",
        ),
        (
            "examples/diagnostics/error_messages/duplicate_derivative_equation.eng",
            "E-SYS-DER-DUPLICATE",
        ),
        (
            "examples/diagnostics/error_messages/unsupported_state_quantity.eng",
            "E-SYS-STATE-UNSUPPORTED",
        ),
        (
            "examples/diagnostics/error_messages/medium_mismatch.eng",
            "E-CONNECT-MEDIUM-MISMATCH",
        ),
        (
            "examples/diagnostics/error_messages/frame_mismatch.eng",
            "E-CONNECT-FRAME-001",
        ),
        (
            "examples/diagnostics/error_messages/axis_mismatch.eng",
            "E-CONNECT-AXIS-001",
        ),
        (
            "examples/diagnostics/error_messages/duplicate_connection.eng",
            "E-CONNECT-DUPLICATE-001",
        ),
        (
            "examples/diagnostics/error_messages/connect_unknown_port.eng",
            "E-CONNECT-UNKNOWN-PORT",
        ),
        (
            "examples/diagnostics/error_messages/connect_bad_endpoint.eng",
            "E-CONNECT-ENDPOINT-001",
        ),
        (
            "examples/diagnostics/error_messages/unconnected_port.eng",
            "W-CONNECT-UNCONNECTED-PORT",
        ),
        (
            "examples/diagnostics/error_messages/generic_domain_arity.eng",
            "E-PORT-DOMAIN-002",
        ),
        (
            "examples/diagnostics/error_messages/domain_missing_across.eng",
            "E-DOMAIN-CONTRACT-001",
        ),
        (
            "examples/diagnostics/error_messages/domain_missing_through.eng",
            "E-DOMAIN-CONTRACT-002",
        ),
        (
            "examples/diagnostics/error_messages/domain_missing_conservation.eng",
            "E-DOMAIN-CONTRACT-003",
        ),
        (
            "examples/diagnostics/error_messages/domain_unknown_quantity.eng",
            "E-DOMAIN-VAR-001",
        ),
        (
            "examples/diagnostics/error_messages/class_missing_field.eng",
            "E-CLASS-FIELD-MISSING-001",
        ),
        (
            "examples/diagnostics/error_messages/class_unknown_field.eng",
            "E-CLASS-FIELD-UNKNOWN-001",
        ),
        (
            "examples/diagnostics/error_messages/class_field_type_mismatch.eng",
            "E-CLASS-FIELD-TYPE-001",
        ),
        (
            "examples/diagnostics/error_messages/class_validation_fail.eng",
            "E-CLASS-VALIDATION-002",
        ),
        (
            "examples/diagnostics/error_messages/class_method_return_mismatch.eng",
            "E-CLASS-METHOD-RETURN-001",
        ),
        (
            "examples/diagnostics/error_messages/class_method_unknown.eng",
            "E-CLASS-METHOD-CALL-002",
        ),
        (
            "examples/diagnostics/error_messages/class_copy_unknown_source.eng",
            "E-CLASS-COPY-001",
        ),
        (
            "examples/diagnostics/error_messages/component_delay_bad_call.eng",
            "E-DELAY-CALL-001",
        ),
        (
            "examples/diagnostics/error_messages/component_delay_bad_duration.eng",
            "E-DELAY-DURATION-001",
        ),
        (
            "examples/diagnostics/error_messages/component_delay_unknown_signal.eng",
            "E-DELAY-SIGNAL-001",
        ),
        (
            "examples/diagnostics/error_messages/component_predictor_bad_call.eng",
            "E-PREDICTOR-CALL-001",
        ),
        (
            "examples/diagnostics/error_messages/component_predictor_unknown_signal.eng",
            "E-PREDICTOR-SIGNAL-001",
        ),
        (
            "examples/diagnostics/error_messages/component_external_bad_call.eng",
            "E-EXTERNAL-BEHAVIOR-CALL-001",
        ),
        (
            "examples/diagnostics/error_messages/component_external_unknown_signal.eng",
            "E-EXTERNAL-BEHAVIOR-SIGNAL-001",
        ),
        (
            "examples/diagnostics/error_messages/component_boundary_unknown_signal.eng",
            "E-ASSEMBLY-BOUNDARY-SIGNAL-001",
        ),
        (
            "examples/diagnostics/error_messages/component_boundary_bad_rhs.eng",
            "E-ASSEMBLY-BOUNDARY-RHS-001",
        ),
        (
            "examples/diagnostics/error_messages/component_boundary_unit_mismatch.eng",
            "E-ASSEMBLY-BOUNDARY-UNIT-001",
        ),
        (
            "examples/diagnostics/error_messages/component_equation_unit_mismatch.eng",
            "E-COMPONENT-EQUATION-UNIT-001",
        ),
        (
            "examples/diagnostics/error_messages/component_math_function_unit_mismatch.eng",
            "E-COMPONENT-EQUATION-UNIT-001",
        ),
        (
            "examples/diagnostics/error_messages/component_parameter_unit_mismatch.eng",
            "E-COMPONENT-PARAM-UNIT-001",
        ),
        (
            "examples/diagnostics/error_messages/simulate_unknown_system.eng",
            "E-SIM-SYSTEM-001",
        ),
        (
            "examples/diagnostics/error_messages/simulate_missing_input.eng",
            "E-SIM-MISSING-INPUT",
        ),
        (
            "examples/diagnostics/error_messages/simulate_input_not_timeseries.eng",
            "E-SIM-INPUT-AXIS-MISMATCH",
        ),
        (
            "examples/diagnostics/error_messages/simulate_input_axis_mismatch.eng",
            "E-SIM-INPUT-AXIS-MISMATCH",
        ),
        (
            "examples/diagnostics/error_messages/simulate_input_quantity_mismatch.eng",
            "E-SIM-INPUT-QTY-MISMATCH",
        ),
        (
            "examples/diagnostics/error_messages/simulate_missing_timestep.eng",
            "E-SIM-TIMESTEP-INVALID",
        ),
        (
            "examples/diagnostics/error_messages/simulate_bad_timestep.eng",
            "E-SIM-TIMESTEP-INVALID",
        ),
        (
            "examples/diagnostics/error_messages/simulate_bad_tolerance.eng",
            "E-SIM-TOLERANCE-INVALID",
        ),
        (
            "examples/diagnostics/error_messages/simulate_missing_solver.eng",
            "E-SIM-SOLVER-UNSUPPORTED",
        ),
        (
            "examples/diagnostics/error_messages/simulate_unsupported_solver.eng",
            "E-SIM-SOLVER-UNSUPPORTED",
        ),
        (
            "examples/diagnostics/error_messages/simulate_unsupported_system_shape.eng",
            "E-SIM-SYSTEM-SHAPE-UNSUPPORTED",
        ),
        (
            "examples/diagnostics/error_messages/simulate_adaptive_discrete_state_space.eng",
            "E-SIM-SYSTEM-SHAPE-UNSUPPORTED",
        ),
        (
            "examples/diagnostics/error_messages/state_space_vector_member_role.eng",
            "E-STATE-SPACE-VECTOR-MEMBER-ROLE",
        ),
        (
            "examples/diagnostics/error_messages/state_space_missing_operator_entry.eng",
            "E-STATE-SPACE-OP-SHAPE-001",
        ),
        (
            "examples/diagnostics/error_messages/state_space_operator_unit_mismatch.eng",
            "E-STATE-SPACE-OP-ENTRY-UNIT-001",
        ),
        (
            "examples/diagnostics/error_messages/state_space_operator_bad_coefficient.eng",
            "E-STATE-SPACE-OP-ENTRY-VALUE-001",
        ),
        (
            "examples/diagnostics/error_messages/fixed_point_bad_options.eng",
            "E-SOLVE-TOLERANCE-INVALID",
        ),
    ] {
        let report = match check_file(fixture, &CheckOptions::default()) {
            Ok(report) => report,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
        };
        if !report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == expected_code)
        {
            eprintln!("expected {fixture} to produce {expected_code}");
            return ExitCode::from(2);
        }
        println!("ok: {fixture} produced {expected_code}");
    }

    let missing_uncertainty_source = match check_file(
        "examples/diagnostics/error_messages/missing_uncertainty_source.eng",
        &CheckOptions::default(),
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    if !missing_uncertainty_source
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "E-UNC-SOURCE-001")
    {
        eprintln!("expected missing_uncertainty_source.eng to produce E-UNC-SOURCE-001");
        return ExitCode::from(2);
    }
    println!("ok: examples/diagnostics/error_messages/missing_uncertainty_source.eng produced diagnostics");

    let invalid_uncertainty_arguments = match check_file(
        "examples/diagnostics/error_messages/invalid_uncertainty_arguments.eng",
        &CheckOptions::default(),
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    for expected_code in ["E-UNC-ARGS-001", "E-UNC-ARGS-002", "E-UNC-ARGS-003"] {
        if !invalid_uncertainty_arguments
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == expected_code)
        {
            eprintln!("expected invalid_uncertainty_arguments.eng to produce {expected_code}");
            return ExitCode::from(2);
        }
    }
    println!(
        "ok: examples/diagnostics/error_messages/invalid_uncertainty_arguments.eng produced diagnostics"
    );

    let missing_ml_source = match check_file(
        "examples/diagnostics/error_messages/missing_ml_source.eng",
        &CheckOptions::default(),
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    if !missing_ml_source
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "E-ML-SOURCE-001")
    {
        eprintln!("expected missing_ml_source.eng to produce E-ML-SOURCE-001");
        return ExitCode::from(2);
    }
    println!("ok: examples/diagnostics/error_messages/missing_ml_source.eng produced diagnostics");

    let invalid_ml_arguments = match check_file(
        "examples/diagnostics/error_messages/invalid_ml_arguments.eng",
        &CheckOptions::default(),
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    if !invalid_ml_arguments
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "E-ML-ARGS-001")
    {
        eprintln!("expected invalid_ml_arguments.eng to produce E-ML-ARGS-001");
        return ExitCode::from(2);
    }
    println!(
        "ok: examples/diagnostics/error_messages/invalid_ml_arguments.eng produced diagnostics"
    );

    match run_file(
        Path::new("examples/official/01_csv_plot/main.eng"),
        Path::new("build/test-plot"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.plot_spec_path.exists()
                || !output.plot_manifest_path.exists()
                || !output.report_spec_path.exists()
                || !output.review_json.contains("\"csv_promotions\"")
                || !output.review_json.contains("\"source_hash\": \"")
                || !output.review_json.contains("\"axis_info\"")
                || !output.review_json.contains("\"binding\": \"Q_coil\"")
                || !output.review_json.contains("\"axis\": \"Time\"")
                || !output.review_json.contains("\"review_document\"")
                || !output.review_json.contains("\"semantic_hash\"")
                || !output.review_json.contains("\"section_hashes\"")
                || !output.review_json.contains("\"schemas\"")
                || !output.review_json.contains("\"units_quantities\"")
                || !output.review_json.contains("\"time_axes\"")
                || !output.review_json.contains("\"derived_values\"")
                || !output.review_json.contains("\"report_outputs\"")
                || !output.review_json.contains("\"input_symbols\"")
                || !output.review_json.contains("\"level\": \"medium\"")
                || !output
                    .review_json
                    .contains("\"result_quantity\": \"Energy\"")
                || !output.result_json.contains("\"data_hashes\"")
                || !output.result_json.contains("\"source_hash\": \"")
                || !output.result_json.contains("\"time_axes\"")
                || !output
                    .result_json
                    .contains("\"input_quantity\": \"HeatRate\"")
                || !output
                    .result_json
                    .contains("\"result_quantity\": \"Energy\"")
                || !output
                    .report_spec_json
                    .contains("\"computed_integrations\"")
                || !output.report_spec_json.contains("\"time_axes\"")
                || !output
                    .report_spec_json
                    .contains("\"input_quantity\": \"HeatRate\"")
                || !output
                    .report_spec_json
                    .contains("\"result_quantity\": \"Energy\"")
                || !output.report_spec_json.contains("\"kernel_plan\"")
                || !output
                    .report_spec_json
                    .contains("\"kind\": \"timeseries_integrate\"")
                || !output
                    .report_spec_json
                    .contains("\"status\": \"interpreter_supported\"")
                || !output
                    .report_spec_json
                    .contains("candidate can execute through the interpreter kernel IR")
                || !output.report_html.contains("CSV Promotions")
                || !output.report_html.contains("Source Hash")
                || !output.report_html.contains("Axis Info")
                || !output
                    .report_html
                    .contains("Runtime Optimization Kernel Plan")
                || !output.report_html.contains("interpreter_supported")
                || !output.report_html.contains("Energy")
            {
                eprintln!(
                    "expected plot example to expose source hashes, TimeSeries axes, HeatRate-to-Energy integration artifacts, and runtime optimization kernel plan fallback metadata"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/official/01_csv_plot/main.eng produced report, PlotSpec, provenance, axis, integration, and kernel plan artifacts"
            );
        }
        Err(error) => {
            eprintln!("plot example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/official/01_csv_plot/histogram.eng"),
        Path::new("build/test-plot-histogram"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let plot_spec = std::fs::read_to_string(&output.plot_spec_path).unwrap_or_default();
            if !plot_spec.contains("\"plot_type\": \"histogram\"")
                || !plot_spec.contains("\"bins\": [{")
                || !plot_spec.contains("Coil heat-rate distribution")
            {
                eprintln!("expected histogram example to produce binned PlotSpec artifacts");
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/official/01_csv_plot/histogram.eng produced histogram PlotSpec artifacts"
            );
        }
        Err(error) => {
            eprintln!("histogram plot example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/official/09_command_where_with/main.eng"),
        Path::new("build/test-command-where-with"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let review = std::fs::read_to_string(&output.review_path).unwrap_or_default();
            let plot_spec = std::fs::read_to_string(&output.plot_spec_path).unwrap_or_default();
            if (output.csv_export_paths.is_empty()
                && !output
                    .output_manifest_json
                    .contains("\"kind\": \"csv_export\"")
                && !output.output_manifest_json.contains("summary.csv"))
                || !review.contains("\"command_styles\"")
                || !review.contains("\"where_blocks\"")
                || !review.contains("\"with_blocks\"")
                || !plot_spec.contains("Command-style coil heat rate")
            {
                eprintln!("expected command/where/with example to produce review, CSV, and plot artifacts");
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/official/09_command_where_with/main.eng produced command/where/with artifacts"
            );
        }
        Err(error) => {
            eprintln!("command/where/with example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/official/12_write_output_manifest/main.eng"),
        Path::new("build/test-write-output-manifest"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let manifest =
                std::fs::read_to_string(&output.output_manifest_path).unwrap_or_default();
            if output.csv_export_paths.is_empty()
                || output.write_output_paths.len() != 2
                || !manifest.contains("\"execution_profile\": \"normal\"")
                || !manifest.contains("\"artifact_count\":")
                || !manifest.contains("\"kind\": \"csv_export\"")
                || !manifest.contains("\"path\": \"outputs/summary.csv\"")
                || !manifest.contains("\"kind\": \"write_text\"")
                || !manifest.contains("\"path\": \"outputs/run_note.txt\"")
                || !manifest.contains("\"kind\": \"write_json\"")
                || !manifest.contains("\"path\": \"outputs/energy.json\"")
                || !manifest.contains("\"artifact_registry\"")
                || !manifest.contains("\"source_files\"")
                || !manifest.contains("\"generated_files\"")
            {
                eprintln!("expected write/output manifest example to produce export, write, and output manifest artifacts");
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/official/12_write_output_manifest/main.eng produced write/export manifest artifacts"
            );
        }
        Err(error) => {
            eprintln!("write/output manifest example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/official/13_file_operations/main.eng"),
        Path::new("build/test-file-operations"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let review = std::fs::read_to_string(&output.review_path).unwrap_or_default();
            let manifest =
                std::fs::read_to_string(&output.output_manifest_path).unwrap_or_default();
            if output.file_operation_paths.len() != 5
                || !review.contains("\"file_operations\"")
                || !review.contains("\"kind\": \"file_mkdir\"")
                || !manifest.contains("\"kind\": \"copy_file\"")
                || !manifest.contains("\"kind\": \"mkdir_dir\"")
                || !manifest.contains("\"kind\": \"move_file\"")
                || !manifest.contains("\"kind\": \"delete_file\"")
                || !manifest.contains("\"kind\": \"delete_dir\"")
            {
                eprintln!("expected file operations example to produce review and output manifest records");
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/official/13_file_operations/main.eng produced file operation artifacts"
            );
        }
        Err(error) => {
            eprintln!("file operations example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/official/14_run_log/main.eng"),
        Path::new("build/test-run-log"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let review = std::fs::read_to_string(&output.review_path).unwrap_or_default();
            let run_log = std::fs::read_to_string(&output.run_log_path).unwrap_or_default();
            let manifest =
                std::fs::read_to_string(&output.output_manifest_path).unwrap_or_default();
            if !review.contains("\"level\": \"warn\"")
                || !run_log.contains("\"format\": \"eng-run-log-v1\"")
                || !run_log.contains("\"level\": \"error\"")
                || !manifest.contains("\"kind\": \"run_log\"")
            {
                eprintln!(
                    "expected run log example to produce review, run_log, and manifest records"
                );
                return ExitCode::from(2);
            }
            println!("ok: examples/official/14_run_log/main.eng produced run log artifacts");
        }
        Err(error) => {
            eprintln!("run log example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/official/15_process_result/main.eng"),
        Path::new("build/test-process-result"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let review = std::fs::read_to_string(&output.review_path).unwrap_or_default();
            let process_results =
                std::fs::read_to_string(&output.process_results_path).unwrap_or_default();
            let manifest =
                std::fs::read_to_string(&output.output_manifest_path).unwrap_or_default();
            if !review.contains("\"process_runs\"")
                || !review.contains("\"binding\": \"echo_result\"")
                || !process_results.contains("\"format\": \"eng-process-results-v1\"")
                || !process_results.contains("\"execution_profile\": \"normal\"")
                || !process_results.contains("\"process_count\": 1")
                || !process_results.contains("\"command\": \"cmd\"")
                || !process_results.contains("\"args\": [\"/C\", \"echo\", \"eng-process-ok\"]")
                || !process_results.contains("\"cwd\": \"examples/official/15_process_result\"")
                || !process_results.contains("\"exit_code\": 0")
                || !process_results.contains("\"status\": \"completed\"")
                || !process_results.contains("eng-process-ok")
                || !manifest.contains("\"kind\": \"process_results\"")
                || !manifest.contains("\"external_commands\"")
                || !manifest.contains("\"stdout_hash\"")
            {
                eprintln!(
                    "expected process result example to produce review, process_results command/cwd/args/exit-code fields, and manifest records"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/official/15_process_result/main.eng produced process result artifacts"
            );
        }
        Err(error) => {
            eprintln!("process result example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/official/16_test_assert_golden/main.eng"),
        Path::new("build/test-assert-golden"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let review = std::fs::read_to_string(&output.review_path).unwrap_or_default();
            let test_results =
                std::fs::read_to_string(&output.test_results_path).unwrap_or_default();
            let manifest =
                std::fs::read_to_string(&output.output_manifest_path).unwrap_or_default();
            if !review.contains("\"tests\"")
                || !test_results.contains("\"format\": \"eng-test-results-v1\"")
                || !test_results.contains("\"test_count\": 1")
                || !test_results.contains("\"failed_count\": 0")
                || !test_results.contains("\"name\": \"summary values\"")
                || !test_results.contains("\"left\": \"Q\"")
                || !test_results.contains("\"tolerance\": \"0.001 kW\"")
                || !test_results.contains("\"artifact\": \"summary.csv\"")
                || !test_results.contains("\"message\": \"golden matched\"")
                || !manifest.contains("\"kind\": \"test_results\"")
                || !manifest.contains("\"tests\"")
                || !manifest.contains("\"assertion_count\"")
                || !manifest.contains("\"golden_count\"")
            {
                eprintln!(
                    "expected test/assert/golden example to produce named tests, assertions, golden comparison status, and manifest records"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/official/16_test_assert_golden/main.eng produced test result artifacts"
            );
        }
        Err(error) => {
            eprintln!("test/assert/golden example failed: {error}");
            return ExitCode::from(2);
        }
    }
    if !safe_profile_rejects_path(
        Path::new("examples/official/12_write_output_manifest/main.eng"),
        Path::new("build/test-safe-profile-export"),
        "E-PROFILE-SAFE-EXPORT",
    ) {
        return ExitCode::from(2);
    }
    if !safe_profile_rejects_source(
        "test-safe-profile-write",
        "write text \"note.txt\", \"blocked\"\n",
        "E-PROFILE-SAFE-WRITE",
    ) {
        return ExitCode::from(2);
    }
    if !safe_profile_rejects_source(
        "test-safe-profile-file-operation",
        "copy file(\"template.txt\") to \"copied.txt\"\n",
        "E-PROFILE-SAFE-FS",
    ) {
        return ExitCode::from(2);
    }
    if !safe_profile_rejects_path(
        Path::new("examples/official/15_process_result/main.eng"),
        Path::new("build/test-safe-profile-process"),
        "E-PROFILE-SAFE-PROCESS",
    ) {
        return ExitCode::from(2);
    }
    match run_file(
        Path::new("examples/official/15_process_result/main.eng"),
        Path::new("build/test-repro-profile-process"),
        &RunOptions {
            save_artifacts: true,
            profile: ExecutionProfile::Repro,
            ..RunOptions::default()
        },
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"execution_profile\": \"repro\"")
                || !output.result_json.contains("W-PROFILE-REPRO-PROCESS")
                || !output.run_log_json.contains("\"profile_diagnostics\"")
                || !output.run_log_json.contains("W-PROFILE-REPRO-PROCESS")
                || !output
                    .output_manifest_json
                    .contains("\"execution_profile\": \"repro\"")
                || !output
                    .output_manifest_json
                    .contains("\"profile_diagnostics\"")
            {
                eprintln!(
                    "expected repro profile process run to record profile diagnostics in result, run log, and output manifest"
                );
                return ExitCode::from(2);
            }
            println!("ok: repro profile recorded process diagnostics in saved artifacts");
        }
        Err(error) => {
            eprintln!("repro profile process smoke failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/internal/17_measured_vs_simulated/main.eng"),
        Path::new("build/test-measured-vs-simulated"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let review = std::fs::read_to_string(&output.review_path).unwrap_or_default();
            let result = std::fs::read_to_string(output.result_path).unwrap_or_default();
            let report_spec = std::fs::read_to_string(output.report_spec_path).unwrap_or_default();
            let report_html = std::fs::read_to_string(output.report_path).unwrap_or_default();
            let plot_spec = std::fs::read_to_string(output.plot_spec_path).unwrap_or_default();
            if !review.contains("\"simulation_results\"")
                || !review.contains("\"solver_results\"")
                || !review.contains("\"time_grid\"")
                || !review.contains("\"binding\": \"sim\"")
                || !review.contains("\"name\": \"T_zone\"")
                || !review.contains("\"name\": \"C\"")
                || !review.contains("\"name\": \"UA\"")
                || !review.contains("\"method\": \"explicit_euler_fixed_step\"")
                || !review.contains("\"step_count\": 6")
                || !review.contains("\"final_value\"")
                || !review.contains("\"name\": \"rmse_T\"")
                || !review.contains("\"quantity_kind\": \"TemperatureDelta\"")
                || !review.contains("\"display_unit\": \"K\"")
                || !review.contains("\"canonical\": \"validate(rmse_T < 5 K)\"")
                || !result.contains("\"metrics\"")
                || !result.contains("\"validations\"")
                || !result.contains("\"time_alignments\"")
                || !result.contains("\"binding\": \"measured_on_sim\"")
                || !result.contains("\"materialization_status\": \"materialized\"")
                || !result.contains("\"output_count\": 7")
                || !result.contains("\"binding\": \"rmse_T\"")
                || !result.contains("\"quantity_kind\": \"TemperatureDelta\"")
                || !result.contains("\"unit\": \"K\"")
                || !result.contains("\"expression\": \"rmse_T < 5 K\"")
                || !result.contains("\"method\": \"explicit_euler_fixed_step\"")
                || !result.contains("\"states\": [\"T_zone\"]")
                || !result.contains("\"inputs\": [\"T_out\", \"Q_internal\"]")
                || !result.contains("\"parameters\": [\"C\", \"UA\"]")
                || !result.contains("\"outputs\": [\"T_zone\"]")
                || !result.contains("\"time_step\": 600")
                || !result.contains("\"step_count\": 6")
                || !result.contains("\"final_value\"")
                || !report_spec.contains("\"computed_metrics\"")
                || !report_spec.contains("\"quantity_kind\": \"TemperatureDelta\"")
                || !report_spec.contains("\"unit\": \"K\"")
                || !report_spec.contains("\"expression\": \"rmse_T < 5 K\"")
                || !report_spec.contains("\"method\": \"explicit_euler_fixed_step\"")
                || !report_spec.contains("\"states\": [\"T_zone\"]")
                || !report_spec.contains("\"inputs\": [\"T_out\", \"Q_internal\"]")
                || !report_spec.contains("\"parameters\": [\"C\", \"UA\"]")
                || !report_spec.contains("\"outputs\": [\"T_zone\"]")
                || !report_spec.contains("\"time_step_s\": 600")
                || !report_spec.contains("\"step_count\": 6")
                || !report_spec.contains("\"final_value\"")
                || !report_spec.contains("\"status\": \"passed\"")
                || !report_html.contains("System Solver Results")
                || !report_html
                    .contains("states=T_zone algebraic=- inputs=T_out, Q_internal parameters=C, UA outputs=T_zone")
                || !report_html.contains("explicit_euler_fixed_step")
                || !report_html.contains("Computed Metrics")
                || !report_html.contains("Validations")
                || !report_html.contains("rmse_T")
                || !report_html.contains("rmse_T &lt; 5 K")
                || !plot_spec.contains("\"name\": \"measured_on_sim\"")
                || !plot_spec.contains("\"name\": \"sim.T_zone\"")
            {
                eprintln!("expected measured-vs-simulated example to produce SolverResult state/input/parameter/output, method/timestep/final-state metadata, a materialized native resampling output, RMSE TemperatureDelta/K, validation, alignment, and multi-series plot artifacts");
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/17_measured_vs_simulated/main.eng produced measured-vs-simulated artifacts"
            );
        }
        Err(error) => {
            eprintln!("measured-vs-simulated example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("tests/runtime/thermal_scalar_input_override.eng"),
        Path::new("build/test-runtime-thermal-scalar-input-override"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let result = std::fs::read_to_string(&output.result_path).unwrap_or_default();
            let plot_spec = std::fs::read_to_string(&output.plot_spec_path).unwrap_or_default();
            let report_spec = std::fs::read_to_string(&output.report_spec_path).unwrap_or_default();
            let review = std::fs::read_to_string(&output.review_path).unwrap_or_default();
            let report_html = std::fs::read_to_string(&output.report_path).unwrap_or_default();
            if !result.contains("\"binding\": \"sim\"")
                || !result.contains("\"method\": \"rk4_fixed_step\"")
                || !result.contains("first-order thermal ODE")
                || !result.contains("\"inputs\": [\"T_out\", \"Q_internal\"]")
                || !result.contains("\"state\": \"T_zone\"")
                || !result.contains("\"final_value\": 304")
                || !result.contains("\"source_equations\"")
                || !result.contains("\"kind\": \"first_order_thermal_balance\"")
                || !result.contains("\"quantity_kind\": \"HeatRate\"")
                || !report_spec.contains("\"source_equations\"")
                || !report_spec.contains("C * der(T_zone) - (UA * (T_out - T_zone) + Q_internal)")
                || !review.contains("\"source_equations\"")
                || !plot_spec.contains("\"name\": \"sim.T_zone\"")
                || !plot_spec.contains("[2, 304]")
                || !report_html.contains("ThermalScalarInputOverride")
                || !report_html.contains("Source Equations")
                || !report_html.contains("first_order_thermal_balance:T_zone")
            {
                eprintln!(
                    "expected tests/runtime/thermal_scalar_input_override.eng to apply simulate scalar input overrides in one-state thermal evaluation"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/thermal_scalar_input_override.eng applied one-state thermal scalar input override"
            );
        }
        Err(error) => {
            eprintln!("thermal scalar input override runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("examples/internal/17_measured_vs_simulated/main.eng"),
        Path::new("build/test-measured-vs-simulated-repro"),
        &RunOptions {
            save_artifacts: true,
            profile: ExecutionProfile::Repro,
            ..RunOptions::default()
        },
    ) {
        Ok(output) => {
            if !output
                .result_json
                .contains("\"execution_profile\": \"repro\"")
                || !output.report_spec_json.contains("\"computed_metrics\"")
                || !output.report_html.contains("Computed Metrics")
                || !output.plot_spec_json.contains("\"name\": \"sim.T_zone\"")
                || !output
                    .output_manifest_json
                    .contains("\"execution_profile\": \"repro\"")
            {
                eprintln!(
                    "expected measured-vs-simulated repro run to save metrics, plot, and repro-profile artifacts"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/17_measured_vs_simulated/main.eng produced repro-profile artifacts"
            );
        }
        Err(error) => {
            eprintln!("measured-vs-simulated repro example failed: {error}");
            return ExitCode::from(2);
        }
    }
    if !measured_fixture_records_time_overlap(
        "examples/internal/17_measured_vs_simulated/main.eng",
        "build/test-measured-vs-simulated-time-mismatch",
        "data/measured_zone_time_mismatch.csv",
    ) {
        return ExitCode::from(2);
    }
    if !measured_fixture_records_missing_policy(
        "examples/internal/17_measured_vs_simulated/main.eng",
        "build/test-measured-vs-simulated-missing",
        "data/measured_zone_missing.csv",
    ) {
        return ExitCode::from(2);
    }
    match run_file(
        Path::new("examples/internal/18_state_space_metadata/main.eng"),
        Path::new("build/test-state-space-metadata"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let review = std::fs::read_to_string(&output.review_path).unwrap_or_default();
            let result = std::fs::read_to_string(&output.result_path).unwrap_or_default();
            if !review.contains("\"state_space_vectors\"")
                || !review.contains("\"linear_operators\"")
                || !review.contains("\"canonical_entries\"")
                || !review.contains("\"vector_type\": \"StateVector\"")
                || !review.contains("\"from\": \"InputVector\"")
                || !review.contains("\"to\": \"Derivative[StateVector]\"")
                || !result.contains("\"binding\": \"sim\"")
                || !result.contains("\"method\": \"state_space_explicit_euler_fixed_step\"")
                || !result.contains("TimeSeries input materialization")
            {
                eprintln!(
                    "expected internal state-space example to record vector/operator metadata and a TimeSeries-input trajectory"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/18_state_space_metadata/main.eng produced state-space metadata and TimeSeries-input trajectory"
            );
        }
        Err(error) => {
            eprintln!("state-space metadata example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/internal/21_unsupported_system_shape/main.eng"),
        Path::new("build/test-unsupported-system-shape"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let result = std::fs::read_to_string(&output.result_path).unwrap_or_default();
            let report_spec = std::fs::read_to_string(&output.report_spec_path).unwrap_or_default();
            let report_html = std::fs::read_to_string(&output.report_path).unwrap_or_default();
            if !result.contains("\"status\": \"skipped_unsupported_shape\"")
                || !result.contains("\"failure_reason\": \"system shape is outside the supported state-space/source ODE/first-order thermal runners\"")
                || !report_spec.contains("\"convergence_status\": \"skipped_unsupported_shape\"")
                || !skipped_solver_has_empty_source_equations(&result, &report_spec)
                || !report_html.contains("skipped_unsupported_shape")
            {
                eprintln!(
                    "expected unsupported system-shape example to produce an explicit skipped solver artifact"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/21_unsupported_system_shape/main.eng recorded an explicit skipped solver artifact"
            );
        }
        Err(error) => {
            eprintln!("unsupported system-shape example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/internal/27_adaptive_heun_thermal/main.eng"),
        Path::new("build/test-adaptive-heun-thermal"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let review = std::fs::read_to_string(&output.review_path).unwrap_or_default();
            let result = std::fs::read_to_string(&output.result_path).unwrap_or_default();
            let report_spec = std::fs::read_to_string(&output.report_spec_path).unwrap_or_default();
            let report_html = std::fs::read_to_string(&output.report_path).unwrap_or_default();
            if !adaptive_solver_artifacts_are_structured(
                &result,
                &review,
                &report_spec,
                "RoomThermal",
                Some("adaptive Heun"),
            ) || !report_html.contains("adaptive_heun")
                || !report_html.contains("substeps=")
            {
                eprintln!(
                    "expected adaptive Heun thermal fixture to produce adaptive solver artifacts"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/27_adaptive_heun_thermal/main.eng produced adaptive Heun solver artifacts"
            );
        }
        Err(error) => {
            eprintln!("adaptive Heun thermal fixture failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/internal/28_adaptive_state_space/main.eng"),
        Path::new("build/test-adaptive-state-space"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let review = std::fs::read_to_string(&output.review_path).unwrap_or_default();
            let result = std::fs::read_to_string(&output.result_path).unwrap_or_default();
            let report_spec = std::fs::read_to_string(&output.report_spec_path).unwrap_or_default();
            let report_html = std::fs::read_to_string(&output.report_path).unwrap_or_default();
            if !adaptive_solver_artifacts_are_structured(
                &result,
                &review,
                &report_spec,
                "AdaptiveStateSpace",
                Some("continuous state-space A/B operators"),
            ) || !result.contains("\"state\": \"Q_total\"")
                || !report_spec.contains("sim.Q_total")
                || !report_html.contains("adaptive_heun")
                || !report_html.contains("substeps=")
            {
                eprintln!(
                    "expected adaptive state-space fixture to produce adaptive solver artifacts"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/28_adaptive_state_space/main.eng produced adaptive state-space solver artifacts"
            );
        }
        Err(error) => {
            eprintln!("adaptive state-space fixture failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/internal/26_state_space_discrete/main.eng"),
        Path::new("build/test-state-space-discrete"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let review = std::fs::read_to_string(&output.review_path).unwrap_or_default();
            let result = std::fs::read_to_string(&output.result_path).unwrap_or_default();
            let report_spec = std::fs::read_to_string(output.report_spec_path).unwrap_or_default();
            let report_html = std::fs::read_to_string(output.report_path).unwrap_or_default();
            if !review.contains("\"canonical_matrix\"")
                || !review.contains("\"canonical_entries\"")
                || !review.contains("\"name\": \"T_air\"")
                || !review.contains("\"name\": \"T_wall\"")
                || !result.contains("\"binding\": \"sim\"")
                || !result.contains("\"state\": \"T_air\"")
                || !result.contains("\"state\": \"T_wall\"")
                || !result.contains("\"method\": \"state_space_discrete_fixed_step\"")
                || !result.contains("recognized discrete-time state-space")
                || !report_spec.contains("\"canonical_matrix\"")
                || !report_spec.contains("\"canonical_entries\"")
                || !report_spec.contains("\"solver_results\"")
                || !report_spec.contains("\"state_space_discrete_fixed_step\"")
                || !report_html.contains("State-Space Metadata")
                || !report_html.contains("Canonical Matrix")
                || !report_html.contains("state_space_discrete_fixed_step")
            {
                eprintln!(
                    "expected discrete state-space fixture to produce two state trajectories and operator matrices across artifacts"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/26_state_space_discrete/main.eng produced discrete state-space solver artifacts"
            );
        }
        Err(error) => {
            eprintln!("discrete state-space fixture failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/internal/20_multi_state_thermal/main.eng"),
        Path::new("build/test-multi-state-thermal"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let review = std::fs::read_to_string(&output.review_path).unwrap_or_default();
            let result = std::fs::read_to_string(&output.result_path).unwrap_or_default();
            let plot_spec = std::fs::read_to_string(output.plot_spec_path).unwrap_or_default();
            let report_spec = std::fs::read_to_string(output.report_spec_path).unwrap_or_default();
            let report_html = std::fs::read_to_string(output.report_path).unwrap_or_default();
            if !review.contains("\"simulation_results\"")
                || !review.contains("\"solver_results\"")
                || !review.contains("\"time_grid\"")
                || !review.contains("\"name\": \"T_air\"")
                || !review.contains("\"name\": \"T_wall\"")
                || !result.contains("\"binding\": \"sim\"")
                || !result.contains("\"state\": \"T_air\"")
                || !result.contains("\"state\": \"T_wall\"")
                || !result.contains("\"method\": \"state_space_rk4_fixed_step\"")
                || !result.contains("multi-state state-space")
                || !plot_spec.contains("\"name\": \"sim.T_air\"")
                || !plot_spec.contains("\"name\": \"sim.T_wall\"")
                || !report_spec.contains("\"state_space_vectors\"")
                || !report_spec.contains("\"linear_operators\"")
                || !report_spec.contains("\"solver_results\"")
                || !report_spec.contains("\"state\": \"T_air\"")
                || !report_spec.contains("\"state\": \"T_wall\"")
                || !report_html.contains("State-Space Metadata")
                || !report_html.contains("StateVector")
                || !report_html.contains("state_space_rk4_fixed_step")
                || !report_html.contains("System Solver Results")
                || !report_html.contains("T_air")
                || !report_html.contains("T_wall")
            {
                eprintln!("expected multi-state thermal example to produce two simulated state trajectories across result, plot, and report artifacts");
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/20_multi_state_thermal/main.eng produced multi-state solver artifacts"
            );
        }
        Err(error) => {
            eprintln!("multi-state thermal example failed: {error}");
            return ExitCode::from(2);
        }
    }
    for (example, build_dir, expected_method, expected_reason) in [
        (
            "examples/advanced_solver/21_state_space_discrete/main.eng",
            "build/test-official-state-space-discrete",
            "state_space_discrete_fixed_step",
            "discrete-time state-space",
        ),
        (
            "examples/advanced_solver/22_state_space_continuous/main.eng",
            "build/test-official-state-space-continuous",
            "state_space_rk4_fixed_step",
            "multi-state state-space",
        ),
    ] {
        match run_file(
            Path::new(example),
            Path::new(build_dir),
            &artifact_run_options(),
        ) {
            Ok(output) => {
                let review = std::fs::read_to_string(&output.review_path).unwrap_or_default();
                let result = std::fs::read_to_string(&output.result_path).unwrap_or_default();
                let plot_spec = std::fs::read_to_string(&output.plot_spec_path).unwrap_or_default();
                let report_spec =
                    std::fs::read_to_string(&output.report_spec_path).unwrap_or_default();
                let report_html = std::fs::read_to_string(&output.report_path).unwrap_or_default();
                if !review.contains("\"state_space_vectors\"")
                    || !review.contains("\"linear_operators\"")
                    || !review.contains("\"canonical_matrix\"")
                    || !result.contains("\"binding\": \"sim\"")
                    || !result.contains("\"state\": \"T_air\"")
                    || !result.contains("\"state\": \"T_wall\"")
                    || !result.contains("\"state\": \"Q_total\"")
                    || !result.contains(expected_method)
                    || !result.contains(expected_reason)
                    || !result.contains("\"source_equations\"")
                    || !result.contains("\"kind\": \"state_space_operator\"")
                    || !(result.contains("\"kind\": \"state_space_update\"")
                        || result.contains("\"kind\": \"state_space_derivative\""))
                    || !result.contains("A[T_air,T_air] * T_air")
                    || !report_spec.contains("\"source_equations\"")
                    || !review.contains("\"source_equations\"")
                    || !plot_spec.contains("\"name\": \"sim.T_air\"")
                    || !plot_spec.contains("\"name\": \"sim.T_wall\"")
                    || !report_spec.contains("\"solver_results\"")
                    || !report_spec.contains("\"state_space_vectors\"")
                    || !report_spec.contains("\"linear_operators\"")
                    || !report_html.contains("State-Space Metadata")
                    || !report_html.contains(expected_method)
                {
                    eprintln!(
                        "expected {example} to produce typed-block state-space solver artifacts"
                    );
                    return ExitCode::from(2);
                }
                println!("ok: {example} produced typed-block state-space solver artifacts");
            }
            Err(error) => {
                eprintln!("official state-space example {example} failed: {error}");
                return ExitCode::from(2);
            }
        }
    }
    match run_file(
        Path::new("tests/runtime/state_space_scalar_input_override.eng"),
        Path::new("build/test-runtime-state-space-scalar-input-override"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let result = std::fs::read_to_string(&output.result_path).unwrap_or_default();
            let plot_spec = std::fs::read_to_string(&output.plot_spec_path).unwrap_or_default();
            let report_spec = std::fs::read_to_string(&output.report_spec_path).unwrap_or_default();
            let report_html = std::fs::read_to_string(&output.report_path).unwrap_or_default();
            if !result.contains("\"binding\": \"sim\"")
                || !result.contains("\"method\": \"state_space_rk4_fixed_step\"")
                || !result.contains("\"inputs\": [\"u\"]")
                || !result.contains("\"outputs\": [\"x\", \"y\"]")
                || !result.contains("\"state\": \"x\"")
                || !result.contains("\"state\": \"y\"")
                || !result.contains("\"final_value\": 4")
                || !result.contains("\"final_value\": 6")
                || !report_spec.contains("y - (x + u)")
                || !plot_spec.contains("\"name\": \"sim.x\"")
                || !plot_spec.contains("\"name\": \"sim.y\"")
                || !report_html.contains("ScalarStateSpaceInputOverride")
            {
                eprintln!(
                    "expected tests/runtime/state_space_scalar_input_override.eng to apply simulate scalar input overrides in state-space evaluation"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: tests/runtime/state_space_scalar_input_override.eng applied state-space scalar input override"
            );
        }
        Err(error) => {
            eprintln!("state-space scalar input override runtime fixture failed: {error}");
            return ExitCode::from(1);
        }
    }
    match run_file(
        Path::new("examples/official/19_class_object/main.eng"),
        Path::new("build/test-class-object"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let review = std::fs::read_to_string(&output.review_path).unwrap_or_default();
            let report_spec = std::fs::read_to_string(output.report_spec_path).unwrap_or_default();
            let report_html = std::fs::read_to_string(output.report_path).unwrap_or_default();
            if !review.contains("\"class_summary\"")
                || !review.contains("\"object_summary\"")
                || !review.contains("\"Object[Construction]\"")
                || !review.contains("\"validation_count\"")
                || !review.contains("\"method_count\"")
                || !review.contains("\"construction\": \"copy_with\"")
                || !review.contains("\"status\": \"pass\"")
                || !report_spec.contains("\"class_summary\"")
                || !report_spec.contains("\"object_summary\"")
                || !report_spec.contains("\"validation_count\"")
                || !report_spec.contains("\"method_count\"")
                || !report_spec.contains("\"copy_with\"")
                || !report_html.contains("Classes")
                || !report_html.contains("Objects")
                || !report_html.contains("validate")
                || !report_html.contains("copy-with")
            {
                eprintln!("expected class object example to expose class/object artifacts");
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/official/19_class_object/main.eng produced class object metadata"
            );
        }
        Err(error) => {
            eprintln!("class object example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/official/01_csv_plot/main.eng"),
        Path::new("build/test-plot-args"),
        &RunOptions {
            open_report: false,
            save_artifacts: true,
            args: vec![ArgOverride {
                name: "input".to_owned(),
                value: "data/sensor.csv".to_owned(),
            }],
            ..RunOptions::default()
        },
    ) {
        Ok(output) => {
            let result = std::fs::read_to_string(output.result_path).unwrap_or_default();
            let review = std::fs::read_to_string(&output.review_path).unwrap_or_default();
            if !result.contains("\"source\": \"cli\"")
                || !result.contains("\"value\": \"data/sensor.csv\"")
                || !review.contains("\"source_literal\": \"args.input\"")
                || !review.contains("\"source_value\": \"data/sensor.csv\"")
            {
                eprintln!("expected Args CLI binding to be recorded in run artifacts");
                return ExitCode::from(2);
            }
            println!("ok: Args CLI binding produced CSV run artifacts");
        }
        Err(error) => {
            eprintln!("Args CLI binding example failed: {error}");
            return ExitCode::from(2);
        }
    }
    let typed_args_report = check_source(
        "typed_args.eng",
        "args {\n    enabled: Bool = false\n    count: Count = 3\n    gain: Float = 1.0\n    window: Duration = 5 min\n}\n\nL = 1 m\n",
        &CheckOptions {
            args: vec![
                ArgOverride {
                    name: "enabled".to_owned(),
                    value: "yes".to_owned(),
                },
                ArgOverride {
                    name: "count".to_owned(),
                    value: "12".to_owned(),
                },
                ArgOverride {
                    name: "gain".to_owned(),
                    value: "1.25".to_owned(),
                },
                ArgOverride {
                    name: "window".to_owned(),
                    value: "10 min".to_owned(),
                },
            ],
            ..CheckOptions::default()
        },
    );
    if typed_args_report.has_errors()
        || !typed_args_report
            .semantic_program
            .arg_values
            .iter()
            .any(|value| value.name == "enabled" && value.value == "true")
        || !typed_args_report
            .semantic_program
            .arg_values
            .iter()
            .any(|value| value.name == "window" && value.value == "600 s")
    {
        eprintln!("expected typed Args values to be normalized");
        return ExitCode::from(2);
    }
    println!("ok: typed Args values were normalized");

    let invalid_typed_args_report = check_source(
        "invalid_typed_args.eng",
        "args {\n    enabled: Bool = maybe\n}\n\nL = 1 m\n",
        &CheckOptions::default(),
    );
    if !invalid_typed_args_report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "E-ARGS-TYPE-001")
    {
        eprintln!("expected invalid typed Args values to produce E-ARGS-TYPE-001");
        return ExitCode::from(2);
    }
    println!("ok: invalid typed Args values produced diagnostics");

    match run_file(
        Path::new("examples/internal/02_simple_system/main.eng"),
        Path::new("build/test-system"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let report_html = std::fs::read_to_string(&output.report_path).unwrap_or_default();
            let report_spec = std::fs::read_to_string(&output.report_spec_path).unwrap_or_default();
            if !report_html.contains("System Equations")
                || !report_spec.contains("\"system_summary\"")
                || !report_spec.contains("\"unit_consistent\"")
            {
                eprintln!("expected simple system run to produce system equation report data");
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/02_simple_system/main.eng produced system report artifacts"
            );
        }
        Err(error) => {
            eprintln!("simple system example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/internal/03_integrated_hvac/main.eng"),
        Path::new("build/test-integrated-hvac"),
        &RunOptions {
            open_report: false,
            save_artifacts: true,
            args: Vec::new(),
            ..RunOptions::default()
        },
    ) {
        Ok(output) => {
            let result = std::fs::read_to_string(output.result_path).unwrap_or_default();
            let plot_spec = std::fs::read_to_string(output.plot_spec_path).unwrap_or_default();
            if !result.contains("\"policy_results\"")
                || !result.contains("\"solver_result\"")
                || !plot_spec.contains("\"Integrated HVAC coil heat rate\"")
            {
                eprintln!(
                    "expected integrated HVAC example to produce policies, solver result, and plot title"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/03_integrated_hvac/main.eng produced integrated user-test artifacts"
            );
        }
        Err(error) => {
            eprintln!("integrated HVAC example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/internal/04_uncertainty_core/main.eng"),
        Path::new("build/test-uncertainty-core"),
        &RunOptions {
            open_report: false,
            save_artifacts: true,
            args: Vec::new(),
            ..RunOptions::default()
        },
    ) {
        Ok(output) => {
            let result = std::fs::read_to_string(output.result_path).unwrap_or_default();
            let review = std::fs::read_to_string(output.review_path).unwrap_or_default();
            let report_spec = std::fs::read_to_string(output.report_spec_path).unwrap_or_default();
            let plot_spec = std::fs::read_to_string(output.plot_spec_path).unwrap_or_default();
            if !result.contains("\"uncertainties\"")
                || !result.contains("\"propagated_linear\"")
                || !result.contains("\"distribution\": \"uniform\"")
                || !result.contains("\"propagation\"")
                || !result.contains("\"p95\"")
                || !uncertainty_example_has_native_scalar_arithmetic(&result)
                || !review.contains("\"uncertainty_info\"")
                || !report_spec.contains("\"uncertainty\"")
                || !plot_spec.contains("\"plot_type\": \"histogram\"")
                || !plot_spec.contains("\"bins\"")
                || !plot_spec.contains("Coil heat-rate uncertainty")
            {
                eprintln!(
                    "expected uncertainty example to produce review/report/result metadata and histogram plot"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/04_uncertainty_core/main.eng produced uncertainty artifacts"
            );
        }
        Err(error) => {
            eprintln!("uncertainty core example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/workflows/03_uncertain_sensor_report/main.eng"),
        Path::new("build/test-uncertain-sensor-report"),
        &RunOptions {
            open_report: false,
            save_artifacts: true,
            args: Vec::new(),
            ..RunOptions::default()
        },
    ) {
        Ok(output) => {
            if !output.review_json.contains("\"timeseries_uncertainty\"")
                || !output
                    .review_json
                    .contains("\"method\": \"pointwise_measured_std\"")
                || !output
                    .review_json
                    .contains("\"operation\": \"duration_above\"")
                || !output.review_json.contains("\"operation\": \"integrate\"")
                || !output.report_spec_json.contains("\"uncertainty\"")
                || !output
                    .result_json
                    .contains("\"timeseries_uncertainty_calculations\"")
                || !output
                    .result_json
                    .contains("\"status\": \"propagated_sensor_std\"")
                || !output.result_json.contains(
                    "\"method\": \"independent_pointwise_sensor_std_duration_above_finite_difference\"",
                )
                || !output
                    .result_json
                    .contains("\"statistic\": \"duration_above(5 kW)\"")
                || !output.result_json.contains(
                    "\"method\": \"independent_pointwise_sensor_std_percentile_finite_difference\"",
                )
                || !output.result_json.contains("\"statistic\": \"p95\"")
                || !native_workflow_has_zero_process_results(&output.process_results_json)
                || !native_workflow_run_graphs_avoid_external_processes(
                    &output.static_run_plan_json,
                    &output.run_plan_json,
                )
                || !output.result_json.contains("\"timeseries_coverage\"")
                || !output.result_json.contains("\"binding\": \"coverage\"")
                || !output.result_json.contains("\"status\": \"complete\"")
                || !output
                    .output_manifest_json
                    .contains("outputs/sensor_summary.csv")
                || !output
                    .output_manifest_json
                    .contains("outputs/sensor_quality_summary.txt")
                || !output
                    .output_manifest_json
                    .contains("\"kind\": \"csv_export\"")
                || !output
                    .output_manifest_json
                    .contains("\"kind\": \"write_text\"")
                || !output.plot_spec_json.contains("\"confidence_band\"")
                || !output.plot_spec_json.contains("\"source\": \"sensor_std\"")
                || !output.plot_svg.contains("data-confidence-band")
                || !output
                    .plot_spec_json
                    .contains("Sensor heat-rate with uncertainty band")
            {
                eprintln!(
                    "expected uncertain sensor workflow to produce native TimeSeries uncertainty metadata, zero process executions, and confidence-band plot artifacts"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/workflows/03_uncertain_sensor_report/main.eng produced uncertainty workflow artifacts"
            );
        }
        Err(error) => {
            eprintln!("uncertain sensor workflow example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/workflows/01_weather_api_to_standard_file/main.eng"),
        Path::new("build/test-workflow-weather-standard-file"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.review_json.contains("WeatherApiRecord")
                || !output
                    .result_json
                    .contains("\"source_format\": \"json_records\"")
                || !output
                    .run_plan_json
                    .contains("\"source:json_records:weather\"")
                || !output.result_json.contains("\"network_boundaries\"")
                || !output.result_json.contains("\"binding\": \"api_response\"")
                || !output
                    .result_json
                    .contains("\"source_value\": \"api_response.body\"")
                || !output
                    .result_json
                    .contains("\"source\": \"api_response.body\"")
                || !output
                    .cache_manifest_json
                    .contains("\"owner_kind\": \"network_request\"")
                || !output
                    .cache_manifest_json
                    .contains("\"owner_name\": \"api_response\"")
                || output
                    .result_json
                    .contains("{ \"key\": \"station\", \"value\": \"station.station_id\"")
                || !native_workflow_has_zero_process_results(&output.process_results_json)
                || !native_workflow_run_graphs_avoid_external_processes(
                    &output.static_run_plan_json,
                    &output.run_plan_json,
                )
                || !output
                    .output_manifest_json
                    .contains("outputs/fetched_weather.json")
                || !output
                    .output_manifest_json
                    .contains("outputs/standard_weather_file.txt")
                || !output
                    .output_manifest_json
                    .contains("outputs/weather_quality_summary.txt")
                || !output
                    .output_manifest_json
                    .contains("\"kind\": \"write_text\"")
                || !output
                    .output_manifest_json
                    .contains("\"kind\": \"report_html\"")
                || !output.review_json.contains("selected_station_id")
                || !output.result_json.contains("\"timeseries_coverage\"")
                || !output.result_json.contains("\"source_table\": \"weather\"")
                || !output.result_json.contains("\"binding\": \"coverage\"")
                || !output.result_json.contains("\"coverage_year\": 2024")
                || !output.result_json.contains("\"expected_count\": 8784")
                || !output.result_json.contains("\"actual_count\": 2")
                || !output.result_json.contains("\"missing_count\": 8782")
                || !output.result_json.contains("\"max_gap\": 3600")
                || !output
                    .result_json
                    .contains("\"leap_year_policy\": \"gregorian_year\"")
                || !output.review_json.contains("\"timeseries_coverage\"")
            {
                eprintln!(
                    "expected weather workflow to produce review, zero external process results, output manifest, and report artifacts"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/workflows/01_weather_api_to_standard_file/main.eng produced workflow manifest artifacts"
            );
        }
        Err(error) => {
            eprintln!("weather standard-file workflow example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/workflows/02_native_surrogate_case_workflow/main.eng"),
        Path::new("build/test-workflow-native-surrogate-case-workflow"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            if !output.review_json.contains("PredictionResult")
                || !native_workflow_has_zero_process_results(&output.process_results_json)
                || !native_workflow_run_graphs_avoid_external_processes(
                    &output.static_run_plan_json,
                    &output.run_plan_json,
                )
                || !output.result_json.contains("\"sample_tables\"")
                || !native_workflow_has_sample_table(
                    &output.result_json,
                    "training_designs",
                    "42",
                    8,
                )
                || !native_workflow_has_sample_table(&output.result_json, "designs", "84", 3)
                || !output
                    .result_json
                    .contains("\"binding\": \"training_designs\"")
                || !output.result_json.contains("\"binding\": \"cases\"")
                || !output
                    .result_json
                    .contains("\"schema_name\": \"CaseTable\"")
                || !output.result_json.contains("\"binding\": \"case_inputs\"")
                || !output
                    .result_json
                    .contains("\"schema_name\": \"CaseOutput\"")
                || !output.result_json.contains("\"binding\": \"case_runs\"")
                || !output
                    .result_json
                    .contains("\"schema_name\": \"CaseRunResult\"")
                || !output
                    .result_json
                    .contains("\"binding\": \"case_result_collection\"")
                || !output
                    .result_json
                    .contains("\"schema_name\": \"CaseResultCollection\"")
                || !output
                    .result_json
                    .contains("\"source\": \"collect results case_runs\"")
                || !output
                    .result_json
                    .contains("\"source\": \"apply(case_input_template, over=cases)\"")
                || !output
                    .result_json
                    .contains("\"source\": \"apply run_case over case_inputs\"")
                || !output
                    .result_json
                    .contains("\"source\": \"materialize cases training_designs\"")
                || !output
                    .result_json
                    .contains("\"generation\": \"sample_lhs\"")
                || !output
                    .result_json
                    .contains("\"schema_name\": \"PredictionResult\"")
                || !output.result_json.contains("\"case_manifests\"")
                || !output
                    .result_json
                    .contains("\"case_dir\": \"outputs/case_001\"")
                || !output.result_json.contains("\"case_tables\"")
                || !output.result_json.contains("\"parameter_columns\"")
                || !output
                    .output_manifest_json
                    .contains("outputs/case_001/input.txt")
                || !output
                    .output_manifest_json
                    .contains("\"kind\": \"case_input\"")
                || !output
                    .output_manifest_json
                    .contains("\"kind\": \"native_case_result\"")
                || !output
                    .output_manifest_json
                    .contains("\"kind\": \"native_case_run_manifest\"")
                || !output
                    .output_manifest_json
                    .contains("outputs/case_003/input.txt.render_manifest.json")
                || !output
                    .output_manifest_json
                    .contains("outputs/workflow_summary.csv")
                || !output.result_json.contains("\"model_cards\"")
                || !output.result_json.contains("\"model_specs\"")
                || !output.result_json.contains("\"prediction_manifests\"")
                || !output.result_json.contains("\"model_diagnostics\"")
                || !output.result_json.contains("\"kind\": \"RegressionModel\"")
                || !output
                    .result_json
                    .contains("\"target\": \"annual_electricity\"")
                || !output.result_json.contains("\"model_artifact_hash\"")
                || !output
                    .result_json
                    .contains("\"confidence_column\": \"confidence\"")
                || !output
                    .result_json
                    .contains("\"predicted_annual_electricity\"")
                || !output.review_json.contains("\"model_specs\"")
                || !output.review_json.contains("\"prediction_manifests\"")
                || !output
                    .output_manifest_json
                    .contains("\"kind\": \"report_html\"")
                || !output.output_manifest_json.contains("\"db_writes\"")
                || !output.result_json.contains("\"db_manifests\"")
                || !output
                    .result_json
                    .contains("\"name\": \"simulation_results\"")
                || !output.result_json.contains("\"name\": \"predictions\"")
                || !output
                    .result_json
                    .contains("\"binding\": \"persisted_predictions\"")
                || !output.result_json.contains("\"kind\": \"sqlite\"")
                || !output.result_json.contains("\"parse_status\": \"parsed\"")
                || !output
                    .result_json
                    .contains("\"transaction_status\": \"committed\"")
                || !output.output_manifest_json.contains("\"model_artifacts\"")
                || !output.review_json.contains("database_target")
                || !output.review_json.contains("persisted_predictions.rows")
                || !output.review_json.contains("cases.rows")
                || !output.review_json.contains("case_inputs.rows")
                || !output.review_json.contains("case_runs.rows")
                || !output.review_json.contains("case_run_succeeded_count")
                || !output.review_json.contains("case_result_collection.rows")
                || !output.review_json.contains("predictions.rows")
                || !output.review_json.contains("db_tables_written")
                || !output
                    .result_json
                    .contains("\"quantity_kind\": \"PeopleDensity\"")
                || !output
                    .result_json
                    .contains("\"display_unit\": \"person/m2\"")
            {
                eprintln!(
                    "expected native surrogate workflow to produce case-run, prediction, DB, review, output manifest, and report artifacts"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/workflows/02_native_surrogate_case_workflow/main.eng produced workflow manifest artifacts"
            );
        }
        Err(error) => {
            eprintln!("native surrogate workflow example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/internal/05_data_driven_modeling/main.eng"),
        Path::new("build/test-data-driven-modeling"),
        &RunOptions {
            open_report: false,
            save_artifacts: true,
            args: Vec::new(),
            ..RunOptions::default()
        },
    ) {
        Ok(output) => {
            let result = std::fs::read_to_string(output.result_path).unwrap_or_default();
            let review = std::fs::read_to_string(output.review_path).unwrap_or_default();
            let report_spec = std::fs::read_to_string(output.report_spec_path).unwrap_or_default();
            let plot_spec = std::fs::read_to_string(output.plot_spec_path).unwrap_or_default();
            let output_manifest =
                std::fs::read_to_string(output.output_manifest_path).unwrap_or_default();
            if !result.contains("\"ml\"")
                || !result.contains("\"model_cards\"")
                || !result.contains("\"rmse\"")
                || !result.contains("\"model_card\"")
                || !result.contains("\"target_quantity\"")
                || !result.contains("\"training_data_hash\"")
                || !result.contains("\"model_artifact_hash\"")
                || !result.contains("\"leakage_status\"")
                || !result.contains("\"coefficients\"")
                || !result.contains("\"loss_history\"")
                || !review.contains("\"ml_info\"")
                || !report_spec.contains("\"ml\"")
                || !report_spec.contains("\"target_quantity\"")
                || !plot_spec.contains("\"plot_type\": \"scatter\"")
                || !plot_spec.contains("Regression parity")
                || !output_manifest.contains("\"artifact_registry\"")
                || !output_manifest.contains("\"model_artifacts\"")
                || !output_manifest.contains("\"training_data_hash\"")
            {
                eprintln!(
                    "expected data-driven example to produce ML metrics, model-card summary, target contract, leakage lint, and parity plot"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/05_data_driven_modeling/main.eng produced ML artifacts"
            );
        }
        Err(error) => {
            eprintln!("data-driven modeling example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/internal/05_data_driven_modeling/residuals.eng"),
        Path::new("build/test-data-driven-modeling-residuals"),
        &RunOptions {
            open_report: false,
            save_artifacts: true,
            args: Vec::new(),
            ..RunOptions::default()
        },
    ) {
        Ok(output) => {
            let result = std::fs::read_to_string(output.result_path).unwrap_or_default();
            let plot_spec = std::fs::read_to_string(output.plot_spec_path).unwrap_or_default();
            if !result.contains("\"residual_points\"")
                || !plot_spec.contains("\"plot_type\": \"bar\"")
                || !plot_spec.contains("Regression residuals")
            {
                eprintln!(
                    "expected data-driven residual example to produce residual points and bar plot"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/internal/05_data_driven_modeling/residuals.eng produced residual plot artifacts"
            );
        }
        Err(error) => {
            eprintln!("data-driven residual example failed: {error}");
            return ExitCode::from(2);
        }
    }

    if !data_quality_fixture_records_parse_failure(
        "examples/diagnostics/data_quality/bad_datetime_cell.eng",
        "build/test-bad-datetime",
        "expected DateTime",
    ) {
        return ExitCode::from(2);
    }
    if !data_quality_fixture_records_parse_failure(
        "examples/diagnostics/data_quality/bad_numeric_cell.eng",
        "build/test-bad-numeric",
        "expected finite numeric cell",
    ) {
        return ExitCode::from(2);
    }
    if !data_quality_fixture_records_interpolation(
        "examples/diagnostics/data_quality/interpolate_missing.eng",
        "build/test-interpolate-missing",
    ) {
        return ExitCode::from(2);
    }
    if !data_quality_fixture_records_constraint_violation(
        "examples/diagnostics/data_quality/constraint_violation.eng",
        "build/test-constraint-violation",
    ) {
        return ExitCode::from(2);
    }
    if !data_quality_fixture_records_sample_constraint_violation(
        "examples/diagnostics/data_quality/sample_constraint_violation.eng",
        "build/test-sample-constraint-violation",
    ) {
        return ExitCode::from(2);
    }
    if !data_quality_fixture_records_conversion_failure(
        "examples/diagnostics/data_quality/unsupported_unit_conversion.eng",
        "build/test-unit-conversion-failure",
    ) {
        return ExitCode::from(2);
    }

    let korean_path = String::from_utf16(&[0xD55C, 0xAE00, 0x0020, 0xACBD, 0xB85C])
        .expect("Korean path smoke label should be valid UTF-16");
    let path_smoke_root = Path::new("build").join("path smoke").join(korean_path);
    if let Err(error) = std::fs::create_dir_all(&path_smoke_root) {
        eprintln!(
            "failed to create Korean and space-containing path smoke folder {}: {error}",
            path_smoke_root.display()
        );
        return ExitCode::from(2);
    }
    let path_smoke_source = path_smoke_root.join("main.eng");
    let path_smoke_build = path_smoke_root.join("build output");
    let source = r#"L = 1 m + 20 cm

report {
    show L
}
"#;
    if let Err(error) = std::fs::write(&path_smoke_source, source) {
        eprintln!(
            "failed to write Korean and space-containing path smoke source {}: {error}",
            path_smoke_source.display()
        );
        return ExitCode::from(2);
    }
    match run_file(
        &path_smoke_source,
        &path_smoke_build,
        &artifact_run_options(),
    ) {
        Ok(output) if output.result_path.exists() && output.report_spec_path.exists() => {
            println!("ok: Korean and space-containing path run smoke produced artifacts");
        }
        Ok(_) => {
            eprintln!("expected Korean and space-containing path smoke to produce artifacts");
            return ExitCode::from(2);
        }
        Err(error) => {
            eprintln!("Korean and space-containing path smoke failed: {error}");
            return ExitCode::from(2);
        }
    }

    match build_standalone(
        Path::new("examples/official/01_csv_plot/main.eng"),
        Path::new("build/test-standalone"),
        &BuildOptions { args: Vec::new() },
    ) {
        Ok(output) => {
            let package_text = std::fs::read_to_string(&output.package_path).unwrap_or_default();
            let lock_text = std::fs::read_to_string(&output.lock_path).unwrap_or_default();
            let args_help_path = output.bundle_path.join("ARGS_HELP.txt");
            let args_help_text = std::fs::read_to_string(&args_help_path).unwrap_or_default();
            if !output.runner_path.exists()
                || !output.executable_path.exists()
                || !output.bytecode_path.exists()
                || !args_help_path.exists()
                || !package_text.contains("format = engpkg-stable-1")
                || !package_text.contains("runner = run.bat")
                || !package_text.contains("args_help = ARGS_HELP.txt")
                || !lock_text.contains("bytecode_version = 1")
                || !lock_text.contains("result_format_version = 1")
                || !args_help_text.contains("Args metadata")
            {
                eprintln!("expected standalone build to create a stable runnable bundle");
                return ExitCode::from(2);
            }

            let help_output = {
                let mut command = standalone_runner_command(&output.bundle_path);
                command.arg("--help").output()
            };
            match help_output {
                Ok(output)
                    if output.status.success()
                        && String::from_utf8_lossy(&output.stdout).contains("Args metadata") => {}
                Ok(_) => {
                    eprintln!("expected standalone runner --help to print Args metadata");
                    return ExitCode::from(2);
                }
                Err(error) => {
                    eprintln!("standalone runner --help failed: {error}");
                    return ExitCode::from(2);
                }
            }

            let status = standalone_runner_command(&output.bundle_path).status();
            match status {
                Ok(status) if status.success() => {
                    let report_spec = output
                        .bundle_path
                        .join("build")
                        .join("result")
                        .join("report_spec.json");
                    let plot_spec = output
                        .bundle_path
                        .join("build")
                        .join("result")
                        .join("plots")
                        .join("plot_spec.json");
                    let output_manifest = output
                        .bundle_path
                        .join("build")
                        .join("result")
                        .join("output_manifest.json");
                    let manifest = std::fs::read_to_string(&output_manifest).unwrap_or_default();
                    if !report_spec.exists()
                        || !plot_spec.exists()
                        || !manifest.contains("\"artifact_registry\"")
                    {
                        eprintln!(
                            "expected standalone runner to produce report, PlotSpec, and output manifest artifacts"
                        );
                        return ExitCode::from(2);
                    }
                    println!(
                        "ok: standalone packaged runner produced report and PlotSpec artifacts"
                    );
                }
                Ok(status) => {
                    eprintln!("standalone runner failed with status {status}");
                    return ExitCode::from(2);
                }
                Err(error) => {
                    eprintln!("failed to run standalone runner: {error}");
                    return ExitCode::from(2);
                }
            }
        }
        Err(error) => {
            eprintln!("standalone build smoke failed: {error}");
            return ExitCode::from(2);
        }
    }
    match build_standalone(
        Path::new("examples/internal/17_measured_vs_simulated/main.eng"),
        Path::new("build/test-standalone-measured-vs-simulated"),
        &BuildOptions { args: Vec::new() },
    ) {
        Ok(output) => {
            let status = standalone_runner_command(&output.bundle_path).status();
            match status {
                Ok(status) if status.success() => {
                    let result_path = output
                        .bundle_path
                        .join("build")
                        .join("result")
                        .join("result.engres");
                    let plot_spec_path = output
                        .bundle_path
                        .join("build")
                        .join("result")
                        .join("plots")
                        .join("plot_spec.json");
                    let output_manifest_path = output
                        .bundle_path
                        .join("build")
                        .join("result")
                        .join("output_manifest.json");
                    let result = std::fs::read_to_string(&result_path).unwrap_or_default();
                    let plot_spec = std::fs::read_to_string(&plot_spec_path).unwrap_or_default();
                    let output_manifest =
                        std::fs::read_to_string(&output_manifest_path).unwrap_or_default();
                    if !result.contains("\"binding\": \"rmse_T\"")
                        || !result.contains("\"validations\"")
                        || !result.contains("\"time_alignments\"")
                        || !result.contains("\"binding\": \"measured_on_sim\"")
                        || !result.contains("\"materialization_status\": \"materialized\"")
                        || !plot_spec.contains("\"name\": \"measured_on_sim\"")
                        || !plot_spec.contains("\"name\": \"sim.T_zone\"")
                        || !output_manifest.contains("\"artifact_registry\"")
                    {
                        eprintln!(
                            "expected measured-vs-simulated standalone runner to produce metric, validation, alignment, multi-series plot, and output manifest artifacts"
                        );
                        return ExitCode::from(2);
                    }
                    println!(
                        "ok: measured-vs-simulated standalone packaged runner produced metric and multi-series plot artifacts"
                    );
                }
                Ok(status) => {
                    eprintln!(
                        "measured-vs-simulated standalone runner failed with status {status}"
                    );
                    return ExitCode::from(2);
                }
                Err(error) => {
                    eprintln!("failed to run measured-vs-simulated standalone runner: {error}");
                    return ExitCode::from(2);
                }
            }
        }
        Err(error) => {
            eprintln!("measured-vs-simulated standalone build smoke failed: {error}");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

const NATIVE_WORKFLOW_BANNED_MARKERS: &[(&str, &str)] = &[
    ("run command", "external process adapter"),
    ("python", "Python runtime dependency"),
    ("python2", "Python runtime dependency"),
    ("python3", "Python runtime dependency"),
    ("py.exe", "Python launcher dependency"),
    (".py", "Python script path"),
    (".pyw", "Python GUI script path"),
    (".ipynb", "Jupyter notebook path"),
    ("pip", "Python package manager dependency"),
    ("conda", "Python environment dependency"),
    ("poetry", "Python package manager dependency"),
    ("pyenv", "Python environment dependency"),
    ("mamba", "Python environment dependency"),
    ("micromamba", "Python environment dependency"),
    ("virtualenv", "Python environment dependency"),
    ("venv", "Python environment dependency"),
    ("ipython", "Python notebook/runtime dependency"),
    ("pytest", "Python test dependency"),
    ("tox", "Python test environment dependency"),
    ("nox", "Python test environment dependency"),
    ("mypy", "Python type-check dependency"),
    ("ruff", "Python lint dependency"),
    ("subprocess", "Python/process adapter"),
    ("pandas", "Python data-frame dependency"),
    ("numpy", "Python numeric dependency"),
    ("scipy", "Python scientific dependency"),
    ("sklearn", "Python ML dependency"),
    ("statsmodels", "Python statistics dependency"),
    ("polars", "Python data-frame dependency"),
    ("matplotlib", "Python plotting dependency"),
    ("requests", "Python HTTP dependency"),
    ("urllib", "Python HTTP dependency"),
    ("pyarrow", "Python data-frame dependency"),
    ("xarray", "Python array dependency"),
    ("tensorflow", "Python ML dependency"),
    ("pytorch", "Python ML dependency"),
    ("torch", "Python ML dependency"),
    ("jupyter", "notebook workflow dependency"),
    ("jupyterlab", "notebook workflow dependency"),
    ("notebook", "notebook workflow dependency"),
    (
        "select_first_row",
        "compatibility-only row selection helper",
    ),
];

fn native_workflow_sources_avoid_external_processes() -> bool {
    if let Err(error) = ensure_required_workflow_main_sources() {
        eprintln!("{error}");
        return false;
    }
    let workflow_sources = match native_workflow_sources() {
        Ok(sources) => sources,
        Err(error) => {
            eprintln!("failed to enumerate native workflow sources: {error}");
            return false;
        }
    };
    for source_path in workflow_sources {
        let Ok(source) = std::fs::read_to_string(&source_path) else {
            eprintln!(
                "failed to read native workflow source {}",
                source_path.display()
            );
            return false;
        };
        if let Some((banned, reason)) = native_workflow_external_process_marker(&source) {
            eprintln!(
                "native workflow source {} must not contain `{banned}` ({reason})",
                source_path.display()
            );
            return false;
        }
    }
    native_workflow_support_files_avoid_external_processes()
}

fn native_workflow_support_files_avoid_external_processes() -> bool {
    let mut files = Vec::new();
    for source in REQUIRED_WORKFLOW_MAIN_SOURCES {
        let Some(root) = Path::new(source).parent() else {
            eprintln!("native workflow source {source} should have a parent directory");
            return false;
        };
        if let Err(error) = collect_workflow_files(root, &mut files) {
            eprintln!(
                "failed to enumerate native workflow support files under {}: {error}",
                root.display()
            );
            return false;
        }
    }
    files.sort();
    files.dedup();

    for path in files {
        if let Some((banned, reason)) =
            native_workflow_external_process_marker(&path.to_string_lossy())
        {
            eprintln!(
                "native workflow file path {} must not contain `{banned}` ({reason})",
                path.display()
            );
            return false;
        }
        if !is_native_workflow_support_text_audit_path(&path) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            eprintln!(
                "failed to read native workflow support file {}",
                path.display()
            );
            return false;
        };
        if let Some((banned, reason)) = native_workflow_external_process_marker(&content) {
            eprintln!(
                "native workflow support file {} must not contain `{banned}` ({reason})",
                path.display()
            );
            return false;
        }
    }
    true
}

fn native_workflow_external_process_marker(source: &str) -> Option<(&'static str, &'static str)> {
    let lowered = source.to_ascii_lowercase();
    NATIVE_WORKFLOW_BANNED_MARKERS
        .iter()
        .copied()
        .find(|(banned, _)| contains_native_workflow_banned_marker(&lowered, banned))
}

fn is_native_workflow_support_text_audit_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "md" | "txt"))
}

fn contains_native_workflow_banned_marker(source: &str, marker: &str) -> bool {
    if marker == "run command" || marker.starts_with('.') {
        return source.contains(marker);
    }
    contains_ascii_word(source, marker)
}

fn contains_ascii_word(source: &str, marker: &str) -> bool {
    let mut offset = 0;
    while let Some(index) = source[offset..].find(marker) {
        let start = offset + index;
        let end = start + marker.len();
        let before = start
            .checked_sub(1)
            .and_then(|index| source.as_bytes().get(index));
        let after = source.as_bytes().get(end);
        if before.is_none_or(|byte| !is_ascii_word_byte(*byte))
            && after.is_none_or(|byte| !is_ascii_word_byte(*byte))
        {
            return true;
        }
        offset = end;
    }
    false
}

fn is_ascii_word_byte(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

fn native_workflow_has_sample_table(
    result_json: &str,
    binding: &str,
    seed: &str,
    sample_count: u64,
) -> bool {
    let Ok(result) = serde_json::from_str::<Value>(result_json) else {
        return false;
    };
    let sample_tables = result
        .get("sample_tables")
        .or_else(|| {
            result
                .get("typed_payload")
                .and_then(|payload| payload.get("sample_tables"))
        })
        .and_then(Value::as_array);
    let Some(sample_tables) = sample_tables else {
        return false;
    };
    sample_tables.iter().any(|table| {
        table.get("binding").and_then(Value::as_str) == Some(binding)
            && table.get("schema_name").and_then(Value::as_str) == Some("GeneratedSample")
            && table.get("source").and_then(Value::as_str) == Some("sample lhs")
            && table.get("generation").and_then(Value::as_str) == Some("sample_lhs")
            && table.get("method").and_then(Value::as_str) == Some("lhs")
            && table.get("seed").and_then(Value::as_str) == Some(seed)
            && table.get("status").and_then(Value::as_str) == Some("generated_sample_table")
            && table.get("sample_count").and_then(Value::as_u64) == Some(sample_count)
            && table.get("row_hash_count").and_then(Value::as_u64) == Some(sample_count)
            && table
                .get("duplicate_case_ids")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
            && table
                .get("parameter_columns")
                .and_then(Value::as_array)
                .is_some_and(|columns| columns.len() == 6)
    })
}

fn uncertainty_example_has_native_scalar_arithmetic(result_json: &str) -> bool {
    let Ok(result) = serde_json::from_str::<Value>(result_json) else {
        return false;
    };
    let Some(numeric_values) = result
        .pointer("/typed_payload/numeric_values")
        .and_then(Value::as_array)
    else {
        return false;
    };
    let has_numeric = |binding: &str, expected: f64, unit: &str| {
        numeric_values.iter().any(|numeric| {
            numeric.get("binding").and_then(Value::as_str) == Some(binding)
                && numeric.get("display_unit").and_then(Value::as_str) == Some(unit)
                && numeric
                    .get("value")
                    .and_then(Value::as_f64)
                    .is_some_and(|value| (value - expected).abs() < 1.0e-9)
        })
    };
    let propagated = result
        .pointer("/typed_payload/uncertainties")
        .and_then(Value::as_array)
        .is_some_and(|uncertainties| {
            uncertainties.iter().any(|uncertainty| {
                uncertainty.get("binding").and_then(Value::as_str) == Some("Q_arithmetic_unc")
                    && uncertainty.get("status").and_then(Value::as_str)
                        == Some("propagated_linear_arithmetic")
                    && uncertainty
                        .get("mean")
                        .and_then(Value::as_f64)
                        .is_some_and(|mean| (mean - 5.9).abs() < 0.01)
            })
        });
    has_numeric("gain", 1.08, "1") && has_numeric("Q_offset", 500.0, "W") && propagated
}

fn native_workflow_has_zero_process_results(process_results_json: &str) -> bool {
    let Ok(process_results) = serde_json::from_str::<Value>(process_results_json) else {
        return false;
    };
    let format = process_results.get("format").and_then(Value::as_str);
    let execution_profile = process_results
        .get("execution_profile")
        .and_then(Value::as_str);
    let process_count = process_results.get("process_count").and_then(Value::as_u64);
    let processes_empty = process_results
        .get("processes")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty);
    format == Some("eng-process-results-v1")
        && execution_profile == Some("normal")
        && process_count == Some(0)
        && processes_empty
}

fn native_workflow_run_graphs_avoid_external_processes(
    static_run_plan_json: &str,
    run_plan_json: &str,
) -> bool {
    native_workflow_run_graph_avoids_external_processes(static_run_plan_json)
        && native_workflow_run_graph_avoids_external_processes(run_plan_json)
}

fn native_workflow_run_graph_avoids_external_processes(run_plan_json: &str) -> bool {
    let Ok(run_plan) = serde_json::from_str::<Value>(run_plan_json) else {
        return false;
    };
    let Some(graph) = run_plan.get("graph") else {
        return false;
    };
    let Some(nodes) = graph.get("nodes").and_then(Value::as_array) else {
        return false;
    };
    if nodes.is_empty() {
        return false;
    }
    if nodes.iter().any(|node| {
        ["id", "kind", "label"]
            .iter()
            .filter_map(|field| node.get(*field).and_then(Value::as_str))
            .any(native_workflow_run_graph_field_mentions_external_process)
    }) {
        return false;
    }
    let Some(edges) = graph.get("edges").and_then(Value::as_array) else {
        return false;
    };
    !edges.iter().any(|edge| {
        ["from", "to", "kind"]
            .iter()
            .filter_map(|field| edge.get(*field).and_then(Value::as_str))
            .any(native_workflow_run_graph_field_mentions_external_process)
    })
}

fn native_workflow_run_graph_field_mentions_external_process(field: &str) -> bool {
    let lowered = field.to_ascii_lowercase();
    if lowered == "process" || lowered.starts_with("process:") {
        return true;
    }
    NATIVE_WORKFLOW_BANNED_MARKERS
        .iter()
        .any(|(marker, _)| contains_native_workflow_banned_marker(&lowered, marker))
}

const REQUIRED_WORKFLOW_MAIN_SOURCES: &[&str] = &[
    "examples/workflows/01_weather_api_to_standard_file/main.eng",
    "examples/workflows/02_native_surrogate_case_workflow/main.eng",
    "examples/workflows/03_uncertain_sensor_report/main.eng",
];

fn ensure_required_workflow_main_sources() -> Result<(), std::io::Error> {
    for source in REQUIRED_WORKFLOW_MAIN_SOURCES {
        let path = Path::new(source);
        if !path.is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("missing required workflow example {}", path.display()),
            ));
        }
    }
    Ok(())
}

fn workflow_main_sources() -> Result<Vec<PathBuf>, std::io::Error> {
    let mut sources = Vec::new();
    for entry in std::fs::read_dir("examples/workflows")? {
        let path = entry?.path();
        if !path.is_dir() {
            continue;
        }
        let main = path.join("main.eng");
        if main.is_file() {
            sources.push(main);
        }
    }
    sources.sort();
    Ok(sources)
}

fn native_workflow_sources() -> Result<Vec<PathBuf>, std::io::Error> {
    let mut sources = Vec::new();
    collect_eng_files(Path::new("examples/workflows"), &mut sources)?;
    sources.sort();
    Ok(sources)
}

fn review_examples_are_formatter_clean() -> bool {
    let mut examples = Vec::new();
    if let Err(error) = collect_eng_files(Path::new("examples/official"), &mut examples) {
        eprintln!("failed to enumerate official examples: {error}");
        return false;
    }
    if let Err(error) = collect_eng_files(Path::new("examples/workflows"), &mut examples) {
        eprintln!("failed to enumerate workflow examples: {error}");
        return false;
    }
    examples.push(PathBuf::from(
        "examples/internal/04_uncertainty_core/main.eng",
    ));
    examples.sort();

    for example in examples {
        let source = match std::fs::read_to_string(&example) {
            Ok(source) => source,
            Err(error) => {
                eprintln!(
                    "failed to read review example {}: {error}",
                    example.display()
                );
                return false;
            }
        };
        if format_source(&source).changed {
            eprintln!(
                "expected review example to be formatter-clean: {}",
                example.display()
            );
            return false;
        }
    }

    println!("ok: review examples are formatter-clean");
    true
}

fn collect_eng_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_eng_files(&path, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("eng") {
            files.push(path);
        }
    }
    Ok(())
}

fn collect_workflow_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_workflow_files(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

fn review_cli_smoke() -> bool {
    let root = Path::new("build").join("test-review-cli");
    let source_root = root.join("source");
    let base_output = root.join("base");
    let changed_output = root.join("changed");
    let direct_diff_output = root.join("direct-diff");
    let base_source = source_root.join("base.eng");
    let changed_source = source_root.join("changed.eng");

    if let Err(error) = std::fs::create_dir_all(&source_root) {
        eprintln!(
            "failed to create review CLI smoke directory {}: {error}",
            source_root.display()
        );
        return false;
    }
    if let Err(error) = std::fs::create_dir_all(&base_output) {
        eprintln!(
            "failed to create review CLI smoke output directory {}: {error}",
            base_output.display()
        );
        return false;
    }
    if let Err(error) = std::fs::create_dir_all(&changed_output) {
        eprintln!(
            "failed to create review CLI smoke output directory {}: {error}",
            changed_output.display()
        );
        return false;
    }

    let base_text =
        "Q = 10 kW\nlimit: HeatRate [kW] = 12 kW\nvalidate Q < limit\nreport {\n    show Q\n}\n";
    let changed_text =
        "Q = 11 kW\nlimit: HeatRate [kW] = 12 kW\nvalidate Q < limit\nreport {\n    show Q\n}\n";
    if let Err(error) = std::fs::write(&base_source, base_text) {
        eprintln!(
            "failed to write review CLI smoke source {}: {error}",
            base_source.display()
        );
        return false;
    }
    if let Err(error) = std::fs::write(&changed_source, changed_text) {
        eprintln!(
            "failed to write review CLI smoke source {}: {error}",
            changed_source.display()
        );
        return false;
    }

    let exe = match env::current_exe() {
        Ok(exe) => exe,
        Err(error) => {
            eprintln!("failed to resolve current eng executable for review CLI smoke: {error}");
            return false;
        }
    };

    let base_status = Command::new(&exe)
        .arg("review")
        .arg(&base_source)
        .arg("--output")
        .arg(&base_output)
        .arg("--json")
        .stdout(Stdio::null())
        .status();
    match base_status {
        Ok(status) if status.success() => {}
        Ok(status) => {
            eprintln!("review CLI static smoke failed with status {status}");
            return false;
        }
        Err(error) => {
            eprintln!("failed to run review CLI static smoke: {error}");
            return false;
        }
    }

    let static_review_path = base_output.join("static_review.json");
    let static_review = std::fs::read_to_string(&static_review_path).unwrap_or_default();
    if !static_review.contains("\"format\": \"eng-review-document-preview-1\"")
        || !static_review.contains("\"semantic_hash\"")
        || !static_review.contains("\"section_hashes\"")
        || !static_review.contains("\"validations\"")
    {
        eprintln!(
            "expected review CLI static smoke to write a normalized ReviewDocument at {}",
            static_review_path.display()
        );
        return false;
    }

    let changed_status = Command::new(&exe)
        .arg("review")
        .arg(&changed_source)
        .arg("--output")
        .arg(&changed_output)
        .arg("--against")
        .arg(&static_review_path)
        .arg("--json")
        .stdout(Stdio::null())
        .status();
    match changed_status {
        Ok(status) if status.success() => {}
        Ok(status) => {
            eprintln!("review CLI diff smoke failed with status {status}");
            return false;
        }
        Err(error) => {
            eprintln!("failed to run review CLI diff smoke: {error}");
            return false;
        }
    }

    let diff_path = changed_output.join("semantic_diff.json");
    let semantic_diff = std::fs::read_to_string(&diff_path).unwrap_or_default();
    if !semantic_diff.contains("\"format\": \"eng-review-semantic-diff-preview-1\"")
        || !semantic_diff.contains("\"status\": \"changed\"")
        || !semantic_diff.contains("\"changed_sections\"")
        || !semantic_diff.contains("\"section_changes\"")
        || !semantic_diff.contains("\"calculations\"")
    {
        eprintln!(
            "expected review CLI diff smoke to write a changed semantic diff at {}",
            diff_path.display()
        );
        return false;
    }

    let changed_review_path = changed_output.join("static_review.json");
    let direct_status = Command::new(&exe)
        .arg("review")
        .arg("diff")
        .arg(&static_review_path)
        .arg(&changed_review_path)
        .arg("--output")
        .arg(&direct_diff_output)
        .arg("--json")
        .stdout(Stdio::null())
        .status();
    match direct_status {
        Ok(status) if status.success() => {}
        Ok(status) => {
            eprintln!("standalone review diff CLI smoke failed with status {status}");
            return false;
        }
        Err(error) => {
            eprintln!("failed to run standalone review diff CLI smoke: {error}");
            return false;
        }
    }
    let direct_diff_path = direct_diff_output.join("semantic_diff.json");
    let direct_semantic_diff = std::fs::read_to_string(&direct_diff_path).unwrap_or_default();
    if direct_semantic_diff != semantic_diff {
        eprintln!(
            "standalone review diff did not match --against output at {}",
            direct_diff_path.display()
        );
        return false;
    }

    println!(
        "ok: eng review CLI wrote static ReviewDocument and matching --against/direct semantic diff artifacts"
    );
    true
}

fn standalone_runner_command(bundle_path: &Path) -> Command {
    let mut command = Command::new(standalone_cmd_path());
    command.arg("/C").arg("run.bat").current_dir(bundle_path);
    apply_standalone_smoke_env(&mut command);
    command
}

fn standalone_cmd_path() -> PathBuf {
    if let Some(comspec) = env::var_os("ComSpec") {
        return PathBuf::from(comspec);
    }
    if let Some(system_root) = env::var_os("SystemRoot").or_else(|| env::var_os("WINDIR")) {
        return PathBuf::from(system_root).join("System32").join("cmd.exe");
    }
    PathBuf::from("cmd.exe")
}

fn apply_standalone_smoke_env(command: &mut Command) {
    for variable in [
        "CARGO",
        "CARGO_HOME",
        "RUSTUP_HOME",
        "PYTHONHOME",
        "PYTHONPATH",
        "VIRTUAL_ENV",
        "ENG_REPO_ROOT",
    ] {
        command.env_remove(variable);
    }

    if let Some(system_root) = env::var_os("SystemRoot").or_else(|| env::var_os("WINDIR")) {
        let system_root_path = PathBuf::from(&system_root);
        let system_path = format!(
            "{};{}",
            system_root_path.join("System32").display(),
            system_root_path.display()
        );
        command.env("SystemRoot", &system_root);
        command.env("WINDIR", &system_root);
        command.env("PATH", system_path);
    } else {
        command.env("PATH", "");
    }

    if let Some(comspec) = env::var_os("ComSpec") {
        command.env("ComSpec", comspec);
    }
}

fn solver_algorithm_smoke() -> Result<(), String> {
    let fixed_point = eng_runtime::solver::solve_fixed_point(
        &[0.0],
        &eng_runtime::solver::FixedPointOptions::default(),
        |values| Ok(vec![0.5 * values[0] + 1.0]),
    )
    .map_err(|failure| format!("fixed-point convergence smoke failed: {}", failure.message))?;
    if fixed_point.convergence_status != "fixed_point_converged"
        || fixed_point.failure.is_some()
        || fixed_point.residual_history.is_empty()
        || fixed_point.residual_value_history.len() != fixed_point.residual_history.len()
        || fixed_point.residual_value_history[0].len() != 1
        || (fixed_point.values[0] - 2.0).abs() > 1e-6
    {
        return Err(
            "fixed-point smoke did not converge to the expected small-loop solution".to_owned(),
        );
    }

    let fixed_point_nonconverged = eng_runtime::solver::solve_fixed_point(
        &[0.0],
        &eng_runtime::solver::FixedPointOptions {
            tolerance: 1e-12,
            max_iterations: 3,
            relaxation: 1.0,
        },
        |values| Ok(vec![values[0] + 1.0]),
    )
    .map_err(|failure| {
        format!(
            "fixed-point nonconvergence smoke errored: {}",
            failure.message
        )
    })?;
    if fixed_point_nonconverged.convergence_status != "fixed_point_not_converged"
        || fixed_point_nonconverged.iteration_count != 3
        || fixed_point_nonconverged.residual_history.len() != 3
        || fixed_point_nonconverged.residual_value_history.len() != 3
        || fixed_point_nonconverged
            .failure
            .as_ref()
            .map(|failure| failure.code.as_str())
            != Some("E-FIXED-POINT-NONCONVERGENCE")
    {
        return Err(
            "fixed-point nonconvergence smoke did not return a failure artifact".to_owned(),
        );
    }

    let fixed_step_input = solver_smoke_fixed_step_input(
        "FixedStepSmoke",
        eng_runtime::solver::FixedStepMethod::ExplicitEuler,
        vec![0.0, 10.0],
    );
    let mut euler_sample_times = Vec::new();
    let euler = eng_runtime::solver::solve_fixed_step_ode(
        eng_runtime::solver::FixedStepMethod::ExplicitEuler,
        &fixed_step_input,
        |sample| {
            euler_sample_times.push(sample.time_s);
            Ok(vec![2.0, -4.0])
        },
    )
    .map_err(|failure| format!("fixed-step Euler smoke failed: {}", failure.message))?;
    if euler.diagnostics.status != "computed"
        || euler.diagnostics.iteration_count != 3
        || euler_sample_times != vec![0.0, 1.0, 2.0]
        || euler.output.state_trajectories.len() != 2
        || euler.output.state_trajectories[0].values != vec![0.0, 2.0, 4.0, 5.0]
        || euler.output.state_trajectories[1].values != vec![10.0, 6.0, 2.0, 0.0]
    {
        return Err(
            "fixed-step Euler smoke did not produce the expected two-state trajectory".to_owned(),
        );
    }

    let rk4_input = solver_smoke_fixed_step_input(
        "FixedStepSmoke",
        eng_runtime::solver::FixedStepMethod::Rk4,
        vec![0.0, 10.0],
    );
    let rk4 = eng_runtime::solver::solve_fixed_step_ode(
        eng_runtime::solver::FixedStepMethod::Rk4,
        &rk4_input,
        |_sample| Ok(vec![2.0, -4.0]),
    )
    .map_err(|failure| format!("fixed-step RK4 smoke failed: {}", failure.message))?;
    if rk4.diagnostics.status != "computed"
        || rk4.diagnostics.iteration_count != 3
        || rk4.output.state_trajectories[0].final_value() != Some(5.0)
        || rk4.output.state_trajectories[1].final_value() != Some(0.0)
    {
        return Err(
            "fixed-step RK4 smoke did not honor the final partial TimeGrid step".to_owned(),
        );
    }

    let adaptive_input = solver_smoke_adaptive_input();
    let adaptive = eng_runtime::solver::solve_adaptive_heun_ode(
        &adaptive_input,
        &eng_runtime::solver::AdaptiveOdeOptions {
            tolerance: 1e-4,
            initial_step_s: 0.5,
            min_step_s: 1e-4,
            max_step_s: 0.5,
            safety_factor: 0.9,
            max_steps: 100,
        },
        |sample| Ok(vec![-sample.state[0]]),
    )
    .map_err(|failure| format!("adaptive Heun smoke failed: {}", failure.message))?;
    let adaptive_final = adaptive.solver_result.output.state_trajectories[0]
        .final_value()
        .unwrap_or(f64::INFINITY);
    if adaptive.solver_result.diagnostics.status != "computed"
        || adaptive.solver_result.diagnostics.convergence_status != "adaptive_heun_completed"
        || adaptive.solver_result.output.state_trajectories[0]
            .values
            .len()
            != 3
        || (adaptive_final - (-1.0_f64).exp()).abs() > 0.01
        || !adaptive
            .step_reports
            .iter()
            .any(|report| report.status == "rejected_error_above_tolerance")
    {
        return Err(
            "adaptive Heun smoke did not produce the expected fixed-output trajectory and substep diagnostics"
                .to_owned(),
        );
    }

    let fixed_step_rhs_failure = eng_runtime::solver::solve_fixed_step_ode(
        eng_runtime::solver::FixedStepMethod::ExplicitEuler,
        &fixed_step_input,
        |_sample| Ok(vec![f64::NAN, 0.0]),
    )
    .unwrap_err();
    if fixed_step_rhs_failure.code != "E-SOLVER-RHS-VALUE-INVALID" {
        return Err("fixed-step RHS failure smoke returned the wrong failure code".to_owned());
    }

    let fixed_step_update_failure_input = solver_smoke_fixed_step_input(
        "FixedStepOverflowSmoke",
        eng_runtime::solver::FixedStepMethod::ExplicitEuler,
        vec![f64::MAX, 0.0],
    );
    let fixed_step_update_failure = eng_runtime::solver::solve_fixed_step_ode(
        eng_runtime::solver::FixedStepMethod::ExplicitEuler,
        &fixed_step_update_failure_input,
        |_sample| Ok(vec![f64::MAX, 0.0]),
    )
    .unwrap_err();
    if fixed_step_update_failure.code != "E-SOLVER-STATE-VALUE-INVALID" {
        return Err("fixed-step update failure smoke returned the wrong failure code".to_owned());
    }

    let linear_graph = solver_smoke_linear_residual_graph(
        "linear.residual_graph",
        &["x", "y"],
        &[
            ("r_energy", &[(0, "x", 2.0), (1, "y", 1.0)], 5.0),
            ("r_balance", &[(0, "x", 1.0), (1, "y", -1.0)], 1.0),
        ],
    );
    let linear = eng_runtime::solver::solve_linear_residual_graph(&linear_graph, 1e-9)
        .map_err(|failure| format!("linear residual graph smoke failed: {}", failure.message))?;
    if linear.status != "converged"
        || linear.iteration_count != 1
        || linear.residual_norm > 1e-9
        || linear.residuals.is_empty()
        || linear
            .residuals
            .iter()
            .any(|residual| residual.status != "satisfied")
        || !linear
            .variables
            .iter()
            .any(|variable| variable.name == "x" && (variable.value - 2.0).abs() <= 1e-9)
        || !linear
            .variables
            .iter()
            .any(|variable| variable.name == "y" && (variable.value - 1.0).abs() <= 1e-9)
    {
        return Err(
            "linear residual graph smoke did not solve the expected square system".to_owned(),
        );
    }

    let singular_linear_graph = solver_smoke_linear_residual_graph(
        "singular.residual_graph",
        &["x", "y"],
        &[
            ("r_energy", &[(0, "x", 1.0), (1, "y", 2.0)], 3.0),
            ("r_balance", &[(0, "x", 2.0), (1, "y", 4.0)], 6.0),
        ],
    );
    let singular_linear =
        eng_runtime::solver::solve_linear_residual_graph(&singular_linear_graph, 1e-9).unwrap_err();
    if singular_linear.code != "E-LINEAR-SINGULAR" {
        return Err(
            "linear residual graph singular smoke returned the wrong failure code".to_owned(),
        );
    }

    let dynamic_assembly = solver_smoke_dynamic_component_assembly();
    let dynamic_component = eng_runtime::solver::solve_dynamic_component_assembly(
        &dynamic_assembly,
        eng_runtime::solver::DynamicComponentAssemblySolveInput {
            duration_s: 1.0,
            timestep_s: 1.0,
            initial_state: vec![1.0],
            initial_algebraic: vec![0.0],
            inputs: vec![eng_runtime::solver::SolverScalar::new(
                "u",
                "Dimensionless",
                "1",
                5.0,
            )],
            parameters: vec![eng_runtime::solver::SolverScalar::new(
                "k",
                "Dimensionless",
                "1",
                2.0,
            )],
        },
        eng_runtime::solver::DynamicComponentOptions::default(),
    )
    .map_err(|failure| {
        format!(
            "dynamic component assembly smoke failed: {}",
            failure.message
        )
    })?;
    if dynamic_component.solver_result.diagnostics.status != "computed"
        || dynamic_component.solver_result.plan.options.method
            != "dynamic_component_assembly_semi_implicit_euler"
        || dynamic_component.solver_result.output.state_trajectories[0].values != vec![1.0, 3.0]
        || dynamic_component.algebraic_trajectories[0].values != vec![2.0, 0.0]
    {
        return Err(
            "dynamic component assembly smoke did not solve the expected residual graph".to_owned(),
        );
    }

    let newton_options = eng_runtime::solver::NewtonOptions::default();
    let nonlinear = eng_runtime::solver::solve_newton(&[0.8, 2.1], &newton_options, |values| {
        let x = values[0];
        let y = values[1];
        Ok(vec![x + y - 3.0, x * x + y * y - 5.0])
    })
    .map_err(|failure| format!("nonlinear Newton smoke failed: {}", failure.message))?;
    if nonlinear.convergence_status != "newton_converged"
        || nonlinear.failure.is_some()
        || (nonlinear.values[0] - 1.0).abs() > 1e-7
        || (nonlinear.values[1] - 2.0).abs() > 1e-7
        || nonlinear
            .residual_history
            .last()
            .copied()
            .unwrap_or(f64::INFINITY)
            > 1e-9
        || nonlinear.largest_residual.is_none()
    {
        return Err("nonlinear Newton smoke did not converge to the expected two-variable solution with residual metadata".to_owned());
    }

    let mut jacobian_calls = 0;
    let analytic = eng_runtime::solver::solve_newton_with_jacobian(
        &[1.0],
        &newton_options,
        |values| Ok(vec![values[0] * values[0] - 2.0]),
        |values, _baseline_residuals| {
            jacobian_calls += 1;
            Ok(vec![vec![2.0 * values[0]]])
        },
    )
    .map_err(|failure| format!("analytic Newton smoke failed: {}", failure.message))?;
    if analytic.convergence_status != "newton_converged"
        || jacobian_calls == 0
        || (analytic.values[0] - 2.0_f64.sqrt()).abs() > 1e-7
    {
        return Err(
            "analytic Newton smoke did not use the supplied Jacobian hook correctly".to_owned(),
        );
    }

    let nonconverged = eng_runtime::solver::solve_newton(
        &[10.0],
        &eng_runtime::solver::NewtonOptions {
            tolerance: 1e-15,
            max_iterations: 1,
            finite_difference_step: 1e-6,
            damping: 1.0,
            line_search_steps: 1,
            ..Default::default()
        },
        |values| Ok(vec![values[0] * values[0] - 2.0]),
    )
    .map_err(|failure| format!("Newton nonconvergence smoke errored: {}", failure.message))?;
    if nonconverged.convergence_status != "newton_not_converged"
        || nonconverged
            .failure
            .as_ref()
            .map(|failure| failure.code.as_str())
            != Some("E-NEWTON-NONCONVERGENCE")
        || nonconverged.largest_residual.is_none()
    {
        return Err("Newton nonconvergence smoke did not return a failure artifact".to_owned());
    }

    let dae_input = eng_runtime::solver::DaeInput {
        states: vec![eng_runtime::solver::DaeVariable::new("x", 1.0)],
        initial_state_derivatives: vec![-2.0],
        algebraic: vec![eng_runtime::solver::DaeVariable::new("z", 2.0)],
        inputs: Vec::new(),
        parameters: Vec::new(),
    };
    let dae = eng_runtime::solver::solve_implicit_euler_dae(
        &dae_input,
        &eng_runtime::solver::DaeOptions::default(),
        |sample| {
            Ok(vec![
                sample.state_derivative[0] + sample.algebraic[0],
                sample.algebraic[0] - 2.0 * sample.state[0],
            ])
        },
    )
    .map_err(|failure| format!("implicit Euler DAE smoke failed: {}", failure.message))?;
    if dae.convergence_status != "dae_converged"
        || dae.failure.is_some()
        || dae.step_reports.len() != 1
        || (dae.state_trajectories[0].values[1] - (1.0 / 3.0)).abs() > 1e-9
        || (dae.algebraic_trajectories[0].values[1] - (2.0 / 3.0)).abs() > 1e-9
    {
        return Err("implicit Euler DAE smoke did not solve the state/algebraic system".to_owned());
    }

    let mass_matrix_input = eng_runtime::solver::DaeInput {
        states: vec![eng_runtime::solver::DaeVariable::new("x", 1.0)],
        initial_state_derivatives: vec![-0.5],
        algebraic: Vec::new(),
        inputs: Vec::new(),
        parameters: Vec::new(),
    };
    let mass_matrix = eng_runtime::solver::solve_implicit_euler_dae(
        &mass_matrix_input,
        &eng_runtime::solver::DaeOptions {
            mass_matrix: Some(eng_runtime::solver::DaeMassMatrix::new(vec![vec![2.0]])),
            ..Default::default()
        },
        |sample| {
            Ok(vec![
                sample.mass_state_derivative.unwrap()[0] + sample.state[0],
            ])
        },
    )
    .map_err(|failure| format!("DAE mass-matrix smoke failed: {}", failure.message))?;
    if mass_matrix.convergence_status != "dae_converged"
        || (mass_matrix.state_trajectories[0].values[1] - (2.0 / 3.0)).abs() > 1e-9
    {
        return Err("DAE mass-matrix smoke did not use the mass derivative".to_owned());
    }

    let inconsistent = eng_runtime::solver::solve_implicit_euler_dae(
        &eng_runtime::solver::DaeInput {
            states: vec![eng_runtime::solver::DaeVariable::new("x", 1.0)],
            initial_state_derivatives: vec![0.0],
            algebraic: Vec::new(),
            inputs: Vec::new(),
            parameters: Vec::new(),
        },
        &eng_runtime::solver::DaeOptions::default(),
        |sample| Ok(vec![sample.state_derivative[0] + sample.state[0]]),
    )
    .unwrap_err();
    if inconsistent.code != "E-DAE-INCONSISTENT-INITIAL-CONDITIONS" {
        return Err(
            "DAE inconsistent-initial-condition smoke returned the wrong failure code".to_owned(),
        );
    }

    let bdf_policy = eng_runtime::solver::solve_implicit_euler_dae(
        &eng_runtime::solver::DaeInput {
            states: vec![eng_runtime::solver::DaeVariable::new("x", 1.0)],
            initial_state_derivatives: vec![-1.0],
            algebraic: Vec::new(),
            inputs: Vec::new(),
            parameters: Vec::new(),
        },
        &eng_runtime::solver::DaeOptions {
            method: eng_runtime::solver::DaeMethod::Bdf { order: 2 },
            ..eng_runtime::solver::DaeOptions::default()
        },
        |sample| Ok(vec![sample.state_derivative[0] + sample.state[0]]),
    )
    .unwrap_err();
    if bdf_policy.code != "E-DAE-METHOD-UNSUPPORTED" {
        return Err("DAE BDF policy smoke returned the wrong failure code".to_owned());
    }

    let dae_nonconverged = eng_runtime::solver::solve_implicit_euler_dae(
        &eng_runtime::solver::DaeInput {
            states: vec![eng_runtime::solver::DaeVariable::new("x", 1.0)],
            initial_state_derivatives: vec![-1.0],
            algebraic: Vec::new(),
            inputs: Vec::new(),
            parameters: Vec::new(),
        },
        &eng_runtime::solver::DaeOptions {
            newton: eng_runtime::solver::NewtonOptions {
                tolerance: 1e-15,
                max_iterations: 1,
                finite_difference_step: 1e-6,
                damping: 1.0,
                line_search_steps: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        |sample| {
            Ok(vec![
                sample.state_derivative[0] + sample.state[0] * sample.state[0],
            ])
        },
    )
    .map_err(|failure| format!("DAE nonconvergence smoke errored: {}", failure.message))?;
    if dae_nonconverged.convergence_status != "dae_not_converged"
        || dae_nonconverged
            .failure
            .as_ref()
            .map(|failure| failure.code.as_str())
            != Some("E-DAE-STEP-NONCONVERGENCE")
        || dae_nonconverged.step_reports.is_empty()
    {
        return Err(
            "DAE nonconvergence smoke did not return a timestep failure artifact".to_owned(),
        );
    }

    Ok(())
}

fn solver_smoke_fixed_step_input(
    system: &str,
    method: eng_runtime::solver::FixedStepMethod,
    initial_state: Vec<f64>,
) -> eng_runtime::solver::SolverInput {
    eng_runtime::solver::SolverInput {
        plan: eng_runtime::solver::SolverPlan::new(
            system,
            eng_runtime::solver::SimulationPlan {
                states: vec!["x".to_owned(), "y".to_owned()],
                outputs: vec!["x".to_owned(), "y".to_owned()],
                inputs: Vec::new(),
                parameters: Vec::new(),
            },
            eng_runtime::solver::SolverOptions::fixed_step(method.method_name(""), 1.0),
        ),
        time_grid: eng_runtime::solver::TimeGrid::fixed_step(2.5, 1.0).unwrap(),
        state_layout: eng_runtime::solver::StateLayout::new(vec![
            eng_runtime::solver::LayoutEntry::new(0, "x", "Dimensionless", "1", "1"),
            eng_runtime::solver::LayoutEntry::new(1, "y", "Dimensionless", "1", "1"),
        ]),
        input_layout: eng_runtime::solver::InputLayout::default(),
        parameter_layout: eng_runtime::solver::ParameterLayout::default(),
        output_layout: eng_runtime::solver::OutputLayout {
            entries: vec![
                eng_runtime::solver::LayoutEntry::new(0, "x", "Dimensionless", "1", "1"),
                eng_runtime::solver::LayoutEntry::new(1, "y", "Dimensionless", "1", "1"),
            ],
        },
        initial_state,
        inputs: Vec::new(),
        parameters: Vec::new(),
    }
}

fn solver_smoke_adaptive_input() -> eng_runtime::solver::SolverInput {
    eng_runtime::solver::SolverInput {
        plan: eng_runtime::solver::SolverPlan::new(
            "AdaptiveDecaySmoke",
            eng_runtime::solver::SimulationPlan {
                states: vec!["x".to_owned()],
                outputs: vec!["x".to_owned()],
                inputs: Vec::new(),
                parameters: Vec::new(),
            },
            eng_runtime::solver::SolverOptions {
                method: "adaptive_heun".to_owned(),
                timestep_s: 0.5,
                tolerance: 1e-4,
                max_iterations: 100,
            },
        ),
        time_grid: eng_runtime::solver::TimeGrid::fixed_step(1.0, 0.5).unwrap(),
        state_layout: eng_runtime::solver::StateLayout::new(vec![
            eng_runtime::solver::LayoutEntry::new(0, "x", "Dimensionless", "1", "1"),
        ]),
        input_layout: eng_runtime::solver::InputLayout::default(),
        parameter_layout: eng_runtime::solver::ParameterLayout::default(),
        output_layout: eng_runtime::solver::OutputLayout {
            entries: vec![eng_runtime::solver::LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )],
        },
        initial_state: vec![1.0],
        inputs: Vec::new(),
        parameters: Vec::new(),
    }
}

type SolverSmokeLinearTerm<'a> = (usize, &'a str, f64);
type SolverSmokeLinearResidualSpec<'a> = (&'a str, &'a [SolverSmokeLinearTerm<'a>], f64);

fn solver_smoke_linear_residual_graph(
    name: &str,
    variable_names: &[&str],
    residual_specs: &[SolverSmokeLinearResidualSpec<'_>],
) -> eng_runtime::solver::ResidualGraph {
    eng_runtime::solver::ResidualGraph {
        name: name.to_owned(),
        variables: variable_names
            .iter()
            .enumerate()
            .map(
                |(index, variable)| eng_runtime::solver::ResidualVariableRef {
                    index,
                    name: (*variable).to_owned(),
                    role: "algebraic".to_owned(),
                    unit: "1".to_owned(),
                },
            )
            .collect(),
        residuals: residual_specs
            .iter()
            .map(
                |(name, terms, rhs_value)| eng_runtime::solver::ResidualEquation {
                    name: (*name).to_owned(),
                    expression: eng_runtime::solver::ResidualExpression::manual(*name),
                    rhs_value: *rhs_value,
                    unit: eng_runtime::solver::ResidualUnit {
                        unit: "1".to_owned(),
                        quantity_kind: "Dimensionless".to_owned(),
                    },
                    scale: eng_runtime::solver::ResidualScale::default(),
                    source: eng_runtime::solver::ResidualSource::default(),
                    variable_indices: terms.iter().map(|(index, _, _)| *index).collect(),
                    terms: terms
                        .iter()
                        .map(
                            |(index, variable, coefficient)| eng_runtime::solver::ResidualTerm {
                                variable_index: *index,
                                variable: (*variable).to_owned(),
                                coefficient: *coefficient,
                            },
                        )
                        .collect(),
                },
            )
            .collect(),
        parameters: Vec::new(),
        dependencies: residual_specs
            .iter()
            .flat_map(|(residual, terms, _)| {
                terms
                    .iter()
                    .map(|(_, variable, _)| ((*residual).to_owned(), (*variable).to_owned()))
            })
            .collect(),
    }
}

fn solver_smoke_dynamic_component_assembly() -> eng_runtime::solver::assembly::EquationAssembly {
    let x = solver_smoke_component_variable("x", "state");
    let z = solver_smoke_component_variable("z", "algebraic");
    let u = solver_smoke_component_variable("u", "input");
    let k = solver_smoke_component_variable("k", "parameter");
    eng_runtime::solver::assembly::EquationAssembly {
        name: "component_graph".to_owned(),
        generated_equations: vec![
            eng_runtime::solver::assembly::GeneratedEquation {
                name: "x_rhs".to_owned(),
                kind: "dynamic_rhs".to_owned(),
                domain: "Test".to_owned(),
                expression: "der(x) eq z".to_owned(),
                residual: "der_x - z".to_owned(),
                rhs_value: None,
                dependencies: vec!["der_x".to_owned(), "z".to_owned()],
                source: "test".to_owned(),
                reason: "solver smoke dynamic component derivative residual".to_owned(),
                source_line: Some(1),
                status: "generated".to_owned(),
            },
            eng_runtime::solver::assembly::GeneratedEquation {
                name: "z_balance".to_owned(),
                kind: "dynamic_algebraic".to_owned(),
                domain: "Test".to_owned(),
                expression: "z + x + k eq u".to_owned(),
                residual: "z + x + k - u".to_owned(),
                rhs_value: None,
                dependencies: vec![
                    "z".to_owned(),
                    "x".to_owned(),
                    "k".to_owned(),
                    "u".to_owned(),
                ],
                source: "test".to_owned(),
                reason: "solver smoke dynamic component algebraic residual".to_owned(),
                source_line: Some(2),
                status: "generated".to_owned(),
            },
        ],
        unknowns: vec![x.clone(), z.clone()],
        states: vec![x],
        algebraic_variables: vec![z],
        inputs: vec![u],
        parameters: vec![k],
        ..eng_runtime::solver::assembly::EquationAssembly::default()
    }
}

fn solver_smoke_component_variable(
    name: &str,
    role: &str,
) -> eng_runtime::solver::assembly::UnknownVariable {
    eng_runtime::solver::assembly::UnknownVariable {
        name: name.to_owned(),
        role: role.to_owned(),
        quantity_kind: "Dimensionless".to_owned(),
        unit: "1".to_owned(),
        source: format!("Test.{name}"),
        status: "classified".to_owned(),
        value: None,
    }
}

fn solver_behavior_smoke() -> Result<(), String> {
    delay_behavior_smoke()?;
    predictor_behavior_smoke()?;
    external_behavior_smoke()?;
    behavior_graph_rhs_smoke()?;
    Ok(())
}

fn delay_behavior_smoke() -> Result<(), String> {
    let buffer = eng_runtime::solver::DelayBuffer::new(
        "temperature",
        "AbsoluteTemperature",
        "K",
        1.0,
        eng_runtime::solver::DelayInterpolationPolicy::Linear,
        eng_runtime::solver::DelayInitialHistoryPolicy::HoldInitial,
    )
    .map_err(|failure| format!("delay buffer smoke setup failed: {}", failure.message))?;
    let mut node = eng_runtime::solver::DelayBehaviorNode::new(buffer);

    let first = node.evaluate(0.0, 10.0).map_err(|failure| {
        format!(
            "delay behavior initial evaluation failed: {}",
            failure.message
        )
    })?;
    let second = node.evaluate(1.0, 20.0).map_err(|failure| {
        format!(
            "delay behavior sample evaluation failed: {}",
            failure.message
        )
    })?;
    let third = node
        .evaluate(1.5, 30.0)
        .map_err(|failure| format!("delay behavior interpolation failed: {}", failure.message))?;
    if first.status != "initial_history"
        || (first.value - 10.0).abs() > 1e-9
        || (second.value - 10.0).abs() > 1e-9
        || third.status != "interpolated"
        || (third.value - 15.0).abs() > 1e-9
        || (third.relationship.delay_s - 1.0).abs() > 1e-9
        || third.relationship.sample_count != 3
    {
        return Err(
            "delay behavior smoke did not preserve history/interpolation artifacts".to_owned(),
        );
    }

    let mut underflow = eng_runtime::solver::DelayBuffer::new(
        "flow",
        "MassFlowRate",
        "kg/s",
        5.0,
        eng_runtime::solver::DelayInterpolationPolicy::PreviousSample,
        eng_runtime::solver::DelayInitialHistoryPolicy::ErrorBeforeHistory,
    )
    .map_err(|failure| format!("delay underflow smoke setup failed: {}", failure.message))?;
    underflow
        .record(0.0, 1.0)
        .map_err(|failure| format!("delay underflow sample record failed: {}", failure.message))?;
    let failure = underflow.evaluate(2.0).unwrap_err();
    if failure.code != "E-DELAY-HISTORY-UNDERFLOW" {
        return Err("delay underflow smoke returned the wrong failure code".to_owned());
    }

    Ok(())
}

fn predictor_behavior_smoke() -> Result<(), String> {
    let contract = eng_runtime::solver::PredictorContract::new(
        "range_checked_predictor",
        vec![
            eng_runtime::solver::BehaviorSignalContract::new("x", "Dimensionless", "1")
                .with_valid_range(Some(0.0), Some(1.0))
                .map_err(|failure| {
                    format!("predictor input range setup failed: {}", failure.message)
                })?,
        ],
        vec![
            eng_runtime::solver::BehaviorSignalContract::new("y", "Dimensionless", "1")
                .with_valid_range(Some(0.0), Some(2.0))
                .map_err(|failure| {
                    format!("predictor output range setup failed: {}", failure.message)
                })?,
        ],
        "sha256:predictor-smoke",
        eng_runtime::solver::PredictorDifferentiability::Differentiable,
        eng_runtime::solver::PredictorSolverPolicy {
            explicit_call_only: true,
            finite_difference_allowed: false,
            jacobian_policy: eng_runtime::solver::PredictorJacobianPolicy::Supplied,
        },
    )
    .map_err(|failure| format!("predictor contract smoke setup failed: {}", failure.message))?;
    let node = eng_runtime::solver::PredictorBehaviorNode::new(contract, |inputs| {
        Ok(vec![inputs[0] * 4.0])
    });
    let evaluation = node
        .evaluate(&[2.0])
        .map_err(|failure| format!("predictor behavior evaluation failed: {}", failure.message))?;
    if evaluation.status != "range_warning"
        || evaluation.outputs.len() != 1
        || (evaluation.outputs[0] - 8.0).abs() > 1e-9
        || evaluation.warnings.len() != 2
        || evaluation
            .warnings
            .iter()
            .any(|warning| warning.code != "W-BEHAVIOR-RANGE")
        || evaluation.contract.model_hash != "sha256:predictor-smoke"
        || evaluation.contract.differentiability != "differentiable"
        || evaluation.contract.jacobian_policy != "supplied"
    {
        return Err(
            "predictor behavior smoke did not expose range warnings and contract metadata"
                .to_owned(),
        );
    }

    let bad_contract = eng_runtime::solver::PredictorContract::new(
        "bad_shape_predictor",
        vec![eng_runtime::solver::BehaviorSignalContract::new(
            "x",
            "Dimensionless",
            "1",
        )],
        vec![eng_runtime::solver::BehaviorSignalContract::new(
            "y",
            "Dimensionless",
            "1",
        )],
        "sha256:bad-shape",
        eng_runtime::solver::PredictorDifferentiability::Unknown,
        eng_runtime::solver::PredictorSolverPolicy::default(),
    )
    .map_err(|failure| {
        format!(
            "bad predictor contract smoke setup failed: {}",
            failure.message
        )
    })?;
    let bad_node =
        eng_runtime::solver::PredictorBehaviorNode::new(bad_contract, |_inputs| Ok(vec![1.0, 2.0]));
    let failure = bad_node.evaluate(&[1.0]).unwrap_err();
    if failure.code != "E-PREDICTOR-OUTPUT-LAYOUT" {
        return Err("predictor layout smoke returned the wrong failure code".to_owned());
    }

    Ok(())
}

fn external_behavior_smoke() -> Result<(), String> {
    let contract = eng_runtime::solver::ExternalBehaviorContract::new(
        "legacy_heat_loss",
        eng_runtime::solver::ExternalBehaviorKind::Function,
        vec![eng_runtime::solver::BehaviorSignalContract::new(
            "temperature",
            "AbsoluteTemperature",
            "K",
        )],
        vec![eng_runtime::solver::BehaviorSignalContract::new(
            "loss", "HeatRate", "W",
        )],
        "sha256:external-smoke",
        eng_runtime::solver::ExternalBehaviorDeterminism::Deterministic,
        eng_runtime::solver::ExternalBehaviorProfilePolicy {
            safe_allowed: true,
            repro_allowed: true,
        },
    )
    .map_err(|failure| format!("external contract smoke setup failed: {}", failure.message))?;
    let node = eng_runtime::solver::ExternalBehaviorNode::new(contract, |inputs| {
        Ok(vec![inputs[0] * 2.0])
    });
    let evaluation = node
        .evaluate(
            eng_runtime::solver::BehaviorExecutionProfile::Repro,
            &[300.0],
        )
        .map_err(|failure| {
            format!(
                "external behavior repro evaluation failed: {}",
                failure.message
            )
        })?;
    if evaluation.status != "ok"
        || evaluation.outputs != vec![600.0]
        || evaluation.contract.kind != "function"
        || evaluation.contract.provenance_hash != "sha256:external-smoke"
        || !evaluation.contract.repro_allowed
    {
        return Err(
            "external behavior smoke did not evaluate deterministic repro contract".to_owned(),
        );
    }

    let blocked_contract = eng_runtime::solver::ExternalBehaviorContract::new(
        "process_adapter",
        eng_runtime::solver::ExternalBehaviorKind::Process,
        vec![eng_runtime::solver::BehaviorSignalContract::new(
            "x",
            "Dimensionless",
            "1",
        )],
        vec![eng_runtime::solver::BehaviorSignalContract::new(
            "y",
            "Dimensionless",
            "1",
        )],
        "sha256:process-smoke",
        eng_runtime::solver::ExternalBehaviorDeterminism::Deterministic,
        eng_runtime::solver::ExternalBehaviorProfilePolicy::default(),
    )
    .map_err(|failure| {
        format!(
            "blocked external contract smoke setup failed: {}",
            failure.message
        )
    })?;
    let blocked_node = eng_runtime::solver::ExternalBehaviorNode::new(blocked_contract, |inputs| {
        Ok(inputs.to_vec())
    });
    let failure = blocked_node
        .evaluate(eng_runtime::solver::BehaviorExecutionProfile::Safe, &[1.0])
        .unwrap_err();
    if failure.code != "E-EXTERNAL-BEHAVIOR-PROFILE" {
        return Err("external profile smoke returned the wrong failure code".to_owned());
    }

    let failing_contract = eng_runtime::solver::ExternalBehaviorContract::new(
        "failing_adapter",
        eng_runtime::solver::ExternalBehaviorKind::Function,
        vec![eng_runtime::solver::BehaviorSignalContract::new(
            "x",
            "Dimensionless",
            "1",
        )],
        vec![eng_runtime::solver::BehaviorSignalContract::new(
            "y",
            "Dimensionless",
            "1",
        )],
        "sha256:failing-smoke",
        eng_runtime::solver::ExternalBehaviorDeterminism::Deterministic,
        eng_runtime::solver::ExternalBehaviorProfilePolicy {
            safe_allowed: true,
            repro_allowed: true,
        },
    )
    .map_err(|failure| {
        format!(
            "failing external contract smoke setup failed: {}",
            failure.message
        )
    })?;
    let failing_node =
        eng_runtime::solver::ExternalBehaviorNode::new(failing_contract, |_inputs| {
            Err(eng_runtime::solver::SolverFailure::new(
                "E-ADAPTER-BOOM",
                "adapter failed",
            ))
        });
    let failure = failing_node
        .evaluate(
            eng_runtime::solver::BehaviorExecutionProfile::Normal,
            &[1.0],
        )
        .unwrap_err();
    if failure.code != "E-EXTERNAL-BEHAVIOR-FAILURE" || !failure.message.contains("E-ADAPTER-BOOM")
    {
        return Err("external adapter failure smoke did not wrap adapter failure".to_owned());
    }

    Ok(())
}

fn behavior_graph_rhs_smoke() -> Result<(), String> {
    let delay = eng_runtime::solver::DelayBehaviorNode::new(
        eng_runtime::solver::DelayBuffer::new(
            "x",
            "Dimensionless",
            "1",
            1.0,
            eng_runtime::solver::DelayInterpolationPolicy::PreviousSample,
            eng_runtime::solver::DelayInitialHistoryPolicy::HoldInitial,
        )
        .map_err(|failure| format!("behavior graph delay setup failed: {}", failure.message))?,
    );
    let predictor_contract = eng_runtime::solver::PredictorContract::new(
        "graph_feedback_predictor",
        vec![eng_runtime::solver::BehaviorSignalContract::new(
            "x_delay",
            "Dimensionless",
            "1",
        )],
        vec![eng_runtime::solver::BehaviorSignalContract::new(
            "x_feedback",
            "Dimensionless",
            "1",
        )],
        "sha256:graph-predictor-smoke",
        eng_runtime::solver::PredictorDifferentiability::Differentiable,
        eng_runtime::solver::PredictorSolverPolicy {
            explicit_call_only: true,
            finite_difference_allowed: true,
            jacobian_policy: eng_runtime::solver::PredictorJacobianPolicy::FiniteDifferenceAllowed,
        },
    )
    .map_err(|failure| {
        format!(
            "behavior graph predictor contract setup failed: {}",
            failure.message
        )
    })?;
    let external_contract = eng_runtime::solver::ExternalBehaviorContract::new(
        "graph_feedback_adapter",
        eng_runtime::solver::ExternalBehaviorKind::Function,
        vec![eng_runtime::solver::BehaviorSignalContract::new(
            "x_feedback",
            "Dimensionless",
            "1",
        )],
        vec![eng_runtime::solver::BehaviorSignalContract::new(
            "x_adjusted_feedback",
            "Dimensionless",
            "1",
        )],
        "sha256:graph-external-smoke",
        eng_runtime::solver::ExternalBehaviorDeterminism::Deterministic,
        eng_runtime::solver::ExternalBehaviorProfilePolicy {
            safe_allowed: true,
            repro_allowed: true,
        },
    )
    .map_err(|failure| {
        format!(
            "behavior graph external contract setup failed: {}",
            failure.message
        )
    })?;
    let mut graph = eng_runtime::solver::BehaviorGraphRhsAdapter::new(vec![
        eng_runtime::solver::BehaviorRhsNode::delay(
            "x_delay",
            eng_runtime::solver::BehaviorSignalSource::State(0),
            delay,
        ),
        eng_runtime::solver::BehaviorRhsNode::predictor(
            "feedback_predictor",
            vec![eng_runtime::solver::BehaviorSignalSource::BehaviorOutput {
                node_index: 0,
                output_index: 0,
            }],
            predictor_contract,
            |inputs| Ok(vec![inputs[0] * 2.0]),
        ),
        eng_runtime::solver::BehaviorRhsNode::external(
            "feedback_adapter",
            eng_runtime::solver::BehaviorExecutionProfile::Repro,
            vec![eng_runtime::solver::BehaviorSignalSource::BehaviorOutput {
                node_index: 1,
                output_index: 0,
            }],
            external_contract,
            |inputs| Ok(vec![inputs[0] + 0.5]),
        ),
    ]);
    let sample = eng_runtime::solver::BehaviorRhsSample::new(0.0, &[1.0], &[], &[]);
    let evaluation = graph
        .evaluate_rhs(&sample, |behavior| {
            if behavior.status != "ok"
                || behavior.nodes.len() != 3
                || behavior.nodes[0].kind != "delay"
                || behavior.nodes[1].kind != "predictor"
                || behavior.nodes[2].kind != "external"
            {
                return Err(eng_runtime::solver::SolverFailure::new(
                    "E-BEHAVIOR-GRAPH-SMOKE",
                    "behavior graph smoke produced unexpected node metadata",
                ));
            }
            Ok(vec![-behavior.output(2, 0).unwrap_or(f64::NAN)])
        })
        .map_err(|failure| format!("behavior graph RHS smoke failed: {}", failure.message))?;

    if evaluation.status != "ok"
        || evaluation.derivatives.len() != 1
        || (evaluation.derivatives[0] + 2.5).abs() > 1e-9
        || evaluation.behavior.nodes[0].delay_relationship.is_none()
        || evaluation.behavior.nodes[1].predictor_contract.is_none()
        || evaluation.behavior.nodes[2].external_contract.is_none()
    {
        return Err(
            "behavior graph RHS smoke did not preserve derivatives and node artifacts".to_owned(),
        );
    }

    Ok(())
}

fn small_thermal_fluid_solve_artifacts_are_structured(output: &eng_runtime::RunOutput) -> bool {
    output.result_json.contains("\"status\": \"solved_linear\"")
        && output
            .result_json
            .contains("\"method\": \"dense_linear_residual_graph\"")
        && output.result_json.contains("\"equation_count\": 12")
        && output.result_json.contains("\"unknown_count\": 12")
        && output.result_json.contains("\"residual_norm\": 0.00000000")
        && output
            .result_json
            .contains("\"name\": \"pump.supply.m_dot\"")
        && output.result_json.contains("\"value\": 0.20000000")
        && output
            .result_json
            .contains("\"name\": \"pipe.inlet.m_dot\"")
        && output.result_json.contains("\"value\": -0.20000000")
        && output.result_json.contains("\"name\": \"pipe.outlet.p\"")
        && output.result_json.contains("\"value\": 200000.00000000")
        && output
            .result_json
            .contains("\"name\": \"return_node.inlet.m_dot\"")
        && output.result_json.contains("\"largest_residuals\"")
        && output.report_spec_json.contains("\"domain_count\": 2")
        && output.report_spec_json.contains("\"domain\": \"Thermal\"")
        && output
            .report_spec_json
            .contains("\"domain\": \"Fluid[Water]\"")
        && output
            .report_spec_json
            .contains("\"component_equation_count\": 6")
        && output
            .report_spec_json
            .contains("\"solver_plan\": \"dense_linear_residual_graph\"")
        && output.report_spec_json.contains("\"pipe.equation_1\"")
        && output.report_spec_json.contains("\"pipe.equation_2\"")
        && output
            .report_spec_json
            .contains("\"not_production_multi_domain\"")
        && output.report_spec_json.contains("20000 Pa")
        && output.report_spec_json.contains("\"parameter_count\": 3")
        && output.report_spec_json.contains("\"name\": \"p_supply\"")
        && output
            .report_spec_json
            .contains("\"status\": \"constructor_override\"")
        && output.report_spec_json.contains("\"name\": \"dp\"")
        && output
            .report_spec_json
            .contains("\"status\": \"defaulted\"")
        && output.report_html.contains("dense_linear_residual_graph")
        && output.report_html.contains("Fluid[Water]")
        && output.report_html.contains("domain plan")
        && output.report_html.contains("component_equation")
}

fn skipped_solver_has_empty_source_equations(result_json: &str, report_spec_json: &str) -> bool {
    let Ok(result) = serde_json::from_str::<Value>(result_json) else {
        return false;
    };
    let Ok(report_spec) = serde_json::from_str::<Value>(report_spec_json) else {
        return false;
    };

    let result_has_empty_source_equations = result["typed_payload"]["systems"]
        .as_array()
        .is_some_and(|systems| {
            systems.iter().any(|system| {
                let solver_result_empty = system["solver_result"]["source_equations"]
                    .as_array()
                    .is_some_and(|source_equations| source_equations.is_empty());
                let solver_results_empty =
                    system["solver_results"]
                        .as_array()
                        .is_some_and(|solver_results| {
                            solver_results.iter().any(|solver| {
                                solver["source_equations"]
                                    .as_array()
                                    .is_some_and(|source_equations| source_equations.is_empty())
                            })
                        });
                solver_result_empty && solver_results_empty
            })
        });

    let report_spec_has_empty_source_equations =
        report_spec["system_ir"].as_array().is_some_and(|systems| {
            systems.iter().any(|system| {
                system["solver_results"]
                    .as_array()
                    .is_some_and(|solver_results| {
                        solver_results.iter().any(|solver| {
                            solver["source_equations"]
                                .as_array()
                                .is_some_and(|source_equations| source_equations.is_empty())
                        })
                    })
            })
        });

    result_has_empty_source_equations && report_spec_has_empty_source_equations
}
fn adaptive_solver_artifacts_are_structured(
    result_json: &str,
    review_json: &str,
    report_spec_json: &str,
    expected_system: &str,
    expected_reason_fragment: Option<&str>,
) -> bool {
    let Ok(result) = serde_json::from_str::<Value>(result_json) else {
        return false;
    };
    let Ok(review) = serde_json::from_str::<Value>(review_json) else {
        return false;
    };
    let Ok(report_spec) = serde_json::from_str::<Value>(report_spec_json) else {
        return false;
    };

    let Some(result_solver) = result["typed_payload"]["systems"]
        .as_array()
        .and_then(|systems| {
            systems
                .iter()
                .find(|system| system["name"] == expected_system)
        })
        .and_then(|system| system.get("solver_result"))
    else {
        return false;
    };
    if !adaptive_solver_result_is_complete(result_solver, expected_reason_fragment) {
        return false;
    }

    let review_summary_ok = review["simulation_results"]
        .as_array()
        .and_then(|results| {
            results.iter().find(|result| {
                result["system"] == expected_system && result["status"] == "computed"
            })
        })
        .and_then(|result| result.get("diagnostics"))
        .is_some_and(adaptive_review_diagnostics_are_complete);
    if !review_summary_ok {
        return false;
    }

    report_spec["system_ir"]
        .as_array()
        .and_then(|systems| {
            systems
                .iter()
                .find(|system| system["name"] == expected_system)
        })
        .and_then(|system| system["solver_results"].as_array())
        .and_then(|solver_results| {
            solver_results
                .iter()
                .find(|solver| adaptive_solver_result_is_complete(solver, expected_reason_fragment))
        })
        .is_some()
}

fn adaptive_solver_result_is_complete(
    solver: &Value,
    expected_reason_fragment: Option<&str>,
) -> bool {
    if solver["status"] != "computed"
        || solver["method"] != "adaptive_heun"
        || solver["convergence_status"] != "adaptive_heun_completed"
        || !solver["failure_code"].is_null()
        || !solver["failure_reason"].is_null()
    {
        return false;
    }
    if !solver["tolerance"]
        .as_f64()
        .is_some_and(|tolerance| (tolerance - 0.0001).abs() < 1e-12)
    {
        return false;
    }
    if !expected_reason_fragment.is_none_or(|fragment| {
        solver["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains(fragment))
    }) {
        return false;
    }

    let step_count = solver["step_count"].as_u64().unwrap_or(0);
    let iteration_count = solver["iteration_count"].as_u64().unwrap_or(0);
    let Some(step_diagnostics) = solver["step_diagnostics"].as_array() else {
        return false;
    };
    step_count > 0
        && iteration_count >= step_count
        && !step_diagnostics.is_empty()
        && step_diagnostics
            .iter()
            .any(|step| step["status"] == "accepted")
        && step_diagnostics.iter().all(|step| {
            step["output_index"]
                .as_u64()
                .is_some_and(|output_index| output_index <= step_count)
                && step["dt_s"].as_f64().is_some_and(|dt| dt > 0.0)
                && step["error_norm"]
                    .as_f64()
                    .is_some_and(|error_norm| error_norm.is_finite())
        })
}

fn adaptive_review_diagnostics_are_complete(diagnostics: &Value) -> bool {
    diagnostics["failure_code"].is_null()
        && diagnostics["failure_reason"].is_null()
        && diagnostics["substep_count"].as_u64().unwrap_or(0) > 0
        && diagnostics["accepted_substep_count"].as_u64().unwrap_or(0) > 0
        && diagnostics["rejected_substep_count"].as_u64().is_some()
        && diagnostics["max_substep_error_norm"]
            .as_f64()
            .is_some_and(|error_norm| error_norm.is_finite())
}

fn data_quality_fixture_records_parse_failure(
    source: &str,
    build_root: &str,
    expected_message: &str,
) -> bool {
    match run_file(
        Path::new(source),
        Path::new(build_root),
        &RunOptions::default(),
    ) {
        Ok(output) => {
            let result = output.result_json;
            if !result.contains("\"parse_failures\"") || !result.contains(expected_message) {
                eprintln!("expected {source} to record parse_failures with `{expected_message}`");
                return false;
            }
            println!("ok: {source} recorded parse_failures");
            true
        }
        Err(error) => {
            eprintln!("data quality fixture failed: {error}");
            false
        }
    }
}

fn data_quality_fixture_records_interpolation(source: &str, build_root: &str) -> bool {
    match run_file(
        Path::new(source),
        Path::new(build_root),
        &RunOptions::default(),
    ) {
        Ok(output) => {
            let result = output.result_json;
            if !result.contains("\"policy\": \"interpolate max_gap=10 min\"")
                || !result.contains("\"status\": \"executed\"")
                || !result.contains("[300,")
                || !result.contains("[600, 4180]")
            {
                eprintln!(
                    "expected {source} to execute interpolation and keep 3 TimeSeries points"
                );
                return false;
            }
            println!("ok: {source} executed missing-value interpolation");
            true
        }
        Err(error) => {
            eprintln!("interpolation fixture failed: {error}");
            false
        }
    }
}

fn measured_fixture_records_time_overlap(
    source: &str,
    build_root: &str,
    measured_fixture: &str,
) -> bool {
    match run_file(
        Path::new(source),
        Path::new(build_root),
        &RunOptions {
            save_artifacts: true,
            args: vec![ArgOverride {
                name: "measured".to_owned(),
                value: measured_fixture.to_owned(),
            }],
            ..RunOptions::default()
        },
    ) {
        Ok(output) => {
            let result = output.result_json;
            if !result.contains("\"sample_count\": 7")
                || !result.contains("\"matched_count\": 4")
                || !result.contains("\"status\": \"overlap\"")
                || !result.contains("\"materialization_status\": \"materialized\"")
                || !result.contains("\"output_count\": 7")
                || !result.contains("\"violation_count\": 0")
            {
                eprintln!(
                    "expected {source} with {measured_fixture} to materialize the simulated axis and retain exact-match overlap metadata without policy violations"
                );
                return false;
            }
            println!("ok: {source} materialized measured data on the simulated Time axis");
            true
        }
        Err(error) => {
            eprintln!("measured/simulated time-overlap fixture failed: {error}");
            false
        }
    }
}

fn measured_fixture_records_missing_policy(
    source: &str,
    build_root: &str,
    measured_fixture: &str,
) -> bool {
    match run_file(
        Path::new(source),
        Path::new(build_root),
        &RunOptions {
            save_artifacts: true,
            args: vec![ArgOverride {
                name: "measured".to_owned(),
                value: measured_fixture.to_owned(),
            }],
            ..RunOptions::default()
        },
    ) {
        Ok(output) => {
            let result = output.result_json;
            if !result.contains("\"sample_count\": 7")
                || !result.contains("\"target\": \"T_zone\"")
                || !result.contains("\"policy\": \"error\"")
                || !result.contains("\"violation_count\": 1")
                || !result.contains("missing value violates `error` policy")
            {
                eprintln!(
                    "expected {source} with {measured_fixture} to record one measured-data missing policy violation"
                );
                return false;
            }
            println!("ok: {source} recorded measured-data missing policy violation");
            true
        }
        Err(error) => {
            eprintln!("measured/simulated missing-policy fixture failed: {error}");
            false
        }
    }
}

fn data_quality_fixture_records_constraint_violation(source: &str, build_root: &str) -> bool {
    match run_file(
        Path::new(source),
        Path::new(build_root),
        &RunOptions::default(),
    ) {
        Ok(output) => {
            let result = output.result_json;
            if !result.contains("\"policy\": \"m_dot <= 0.25 kg/s\"")
                || !result.contains("\"violation_count\": 1")
                || !result.contains("value is above upper bound 0.25")
            {
                eprintln!(
                    "expected {source} to execute upper-bound constraint and record one violation"
                );
                return false;
            }
            println!("ok: {source} recorded constraint violation");
            true
        }
        Err(error) => {
            eprintln!("constraint violation fixture failed: {error}");
            false
        }
    }
}

fn data_quality_fixture_records_sample_constraint_violation(
    source: &str,
    build_root: &str,
) -> bool {
    match run_file(
        Path::new(source),
        Path::new(build_root),
        &RunOptions::default(),
    ) {
        Ok(output) => {
            let result = output.result_json;
            let report_spec = output.report_spec_json;
            if !result.contains("\"sample_tables\"")
                || !result.contains("\"schema_name\": \"DesignSample\"")
                || !result.contains("\"case_id_column\": \"case_id\"")
                || !result.contains("\"policy\": \"cooling_cop > 0\"")
                || !result.contains("\"violation_count\": 1")
                || !result.contains("\"row\": 3")
                || !result.contains("\"column\": \"cooling_cop\"")
                || !result.contains("value is not greater than 0")
                || !report_spec.contains("\"schema\": \"DesignSample\"")
                || !report_spec.contains("\"policy\": \"cooling_cop > 0\"")
                || !report_spec.contains("\"violation_count\": 1")
            {
                eprintln!(
                    "expected {source} to record a sample-table constraint violation in result and report spec artifacts"
                );
                return false;
            }
            println!("ok: {source} recorded sample-table constraint violation");
            true
        }
        Err(error) => {
            eprintln!("sample constraint violation fixture failed: {error}");
            false
        }
    }
}

fn data_quality_fixture_records_conversion_failure(source: &str, build_root: &str) -> bool {
    match run_file(
        Path::new(source),
        Path::new(build_root),
        &RunOptions::default(),
    ) {
        Ok(output) => {
            let result = output.result_json;
            if !result.contains("\"conversion_failures\"")
                || !result.contains("\"source_unit\": \"lb/s\"")
                || !result.contains("\"target_unit\": \"kg/s\"")
                || !result.contains("unsupported source unit")
            {
                eprintln!("expected {source} to record per-cell unit conversion failures");
                return false;
            }
            println!("ok: {source} recorded unit conversion failures");
            true
        }
        Err(error) => {
            eprintln!("unit conversion fixture failed: {error}");
            false
        }
    }
}

fn artifact_run_options() -> RunOptions {
    RunOptions {
        save_artifacts: true,
        ..RunOptions::default()
    }
}

fn safe_profile_rejects_path(source: &Path, build_root: &Path, expected_code: &str) -> bool {
    match run_file(
        source,
        build_root,
        &RunOptions {
            profile: ExecutionProfile::Safe,
            ..RunOptions::default()
        },
    ) {
        Err(error) if error.to_string().contains(expected_code) => {
            println!(
                "ok: safe profile rejected {} with {expected_code}",
                source.display()
            );
            true
        }
        Err(error) => {
            eprintln!(
                "expected safe profile to reject {} with {expected_code}, got: {error}",
                source.display()
            );
            false
        }
        Ok(_) => {
            eprintln!(
                "expected safe profile to reject {} with {expected_code}",
                source.display()
            );
            false
        }
    }
}

fn safe_profile_rejects_source(name: &str, source: &str, expected_code: &str) -> bool {
    let source_root = Path::new("build").join(name).join("source");
    let build_root = Path::new("build").join(name).join("output");
    let source_path = source_root.join("main.eng");
    if let Err(error) = std::fs::create_dir_all(&source_root) {
        eprintln!(
            "failed to create safe-profile source folder {}: {error}",
            source_root.display()
        );
        return false;
    }
    if let Err(error) = std::fs::write(source_root.join("template.txt"), "template") {
        eprintln!(
            "failed to write safe-profile fixture data in {}: {error}",
            source_root.display()
        );
        return false;
    }
    if let Err(error) = std::fs::write(&source_path, source) {
        eprintln!(
            "failed to write safe-profile fixture {}: {error}",
            source_path.display()
        );
        return false;
    }
    safe_profile_rejects_path(&source_path, &build_root, expected_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_workflow_banned_marker_covers_python_notebook_and_process_markers() {
        for (source, marker) in [
            ("run command \"tool\"", "run command"),
            ("python3.11 workflow.py", "python3"),
            ("workflow.pyw", ".pyw"),
            ("analysis.ipynb", ".ipynb"),
            ("poetry run adapter", "poetry"),
            ("ruff check", "ruff"),
            ("jupyterlab execute", "jupyterlab"),
            ("import numpy as np", "numpy"),
        ] {
            assert!(
                contains_native_workflow_banned_marker(source, marker),
                "expected marker {marker} in {source}"
            );
        }
        assert!(!contains_native_workflow_banned_marker(
            "numpy_score = 1",
            "numpy"
        ));
    }

    #[test]
    fn native_workflow_support_file_guard_rejects_python_paths_and_doc_markers() {
        assert_eq!(
            native_workflow_external_process_marker("run command \"tool\"")
                .map(|(marker, _)| marker),
            Some("run command")
        );
        assert_eq!(
            native_workflow_external_process_marker("adapter.py").map(|(marker, _)| marker),
            Some(".py")
        );
        assert_eq!(
            native_workflow_external_process_marker("notes.ipynb").map(|(marker, _)| marker),
            Some(".ipynb")
        );
        assert!(native_workflow_external_process_marker("WeatherApiPayload contract").is_none());
        assert!(is_native_workflow_support_text_audit_path(Path::new(
            "README.md"
        )));
        assert!(is_native_workflow_support_text_audit_path(Path::new(
            "notes.TXT"
        )));
        assert!(!is_native_workflow_support_text_audit_path(Path::new(
            "data.json"
        )));
    }

    #[test]
    fn native_workflow_run_graph_guard_rejects_process_and_python_nodes() {
        let ok = r#"{
            "graph": {
                "nodes": [
                    {"id": "source:json_records:weather", "kind": "source", "label": "weather"}
                ],
                "edges": [
                    {"from": "source:json_records:weather", "to": "artifact:standard_file", "kind": "data"}
                ]
            }
        }"#;
        assert!(native_workflow_run_graph_avoids_external_processes(ok));

        let process_node = r#"{
            "graph": {
                "nodes": [
                    {"id": "process:adapter", "kind": "process", "label": "run command python"}
                ],
                "edges": []
            }
        }"#;
        assert!(!native_workflow_run_graph_avoids_external_processes(
            process_node
        ));

        let process_edge = r#"{
            "graph": {
                "nodes": [
                    {"id": "source:designs", "kind": "source", "label": "designs"}
                ],
                "edges": [
                    {"from": "source:designs", "to": "python3 adapter", "kind": "data"}
                ]
            }
        }"#;
        assert!(!native_workflow_run_graph_avoids_external_processes(
            process_edge
        ));
    }
}
