use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::ExitCode;
use std::time::Instant;

use eng_compiler::{
    check_file, check_source, format_source, review_json, ArgOverride, CheckOptions, CheckReport,
    Severity,
};
use eng_runtime::{
    build_standalone, create_project, doctor, run_file, BuildOptions, ExecutionProfile, RunOptions,
    RuntimeError,
};
use serde_json::{json, Value};

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_help();
        return ExitCode::SUCCESS;
    }

    let command = args.remove(0);
    match command.as_str() {
        "doctor" => command_doctor(),
        "check" => command_check(args),
        "fmt" => command_fmt(args),
        "ide-check" => command_ide_check(args),
        "jit-plan" => command_jit_plan(args),
        "jit-bench" => command_jit_bench(args),
        "run" => command_run(args),
        "build" => command_build(args),
        "view" => command_view(args),
        "new" => command_new(args),
        "test" => command_test(args),
        "help" | "--help" | "-h" => {
            print_help();
            ExitCode::SUCCESS
        }
        "--version" | "version" => {
            println!("EngLang {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unknown command: {other}");
            print_help();
            ExitCode::from(2)
        }
    }
}

fn command_jit_plan(args: Vec<String>) -> ExitCode {
    let Some(path) = first_non_flag(&args) else {
        eprintln!("usage: eng jit-plan <file.eng> [--backend <name>]");
        return ExitCode::from(2);
    };
    let requested_backend = match parse_jit_backend(&args) {
        Ok(value) => value,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let check_args = match parse_arg_overrides(&args, &["--backend"], &[]) {
        Ok(values) => values,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let report = match check_file(
        &path,
        &CheckOptions {
            review: false,
            args: check_args,
            require_args: false,
        },
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    if report.has_errors() {
        print_diagnostics(&report);
        return ExitCode::from(2);
    }

    let plan =
        eng_jit::plan_for_report_with_options(&report, &eng_jit::PlanOptions { requested_backend });
    println!("{}", eng_jit::plan_json_string(&plan));
    ExitCode::SUCCESS
}

fn command_jit_bench(args: Vec<String>) -> ExitCode {
    let Some(path) = first_non_flag(&args) else {
        eprintln!("usage: eng jit-bench <file.eng> [--iterations N] [--backend <name>]");
        return ExitCode::from(2);
    };
    let iterations = match option_value(&args, "--iterations") {
        Some(value) => match value.parse::<usize>() {
            Ok(value) if (1..=100).contains(&value) => value,
            Ok(_) => {
                eprintln!("--iterations must be between 1 and 100");
                return ExitCode::from(2);
            }
            Err(error) => {
                eprintln!("invalid --iterations value: {error}");
                return ExitCode::from(2);
            }
        },
        None => 3,
    };
    let requested_backend = match parse_jit_backend(&args) {
        Ok(value) => value,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let runtime_args = match parse_arg_overrides(&args, &["--iterations", "--backend"], &[]) {
        Ok(values) => values,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };

    let report = match check_file(
        &path,
        &CheckOptions {
            review: false,
            args: runtime_args.clone(),
            require_args: false,
        },
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    if report.has_errors() {
        print_diagnostics(&report);
        return ExitCode::from(2);
    }
    let plan =
        eng_jit::plan_for_report_with_options(&report, &eng_jit::PlanOptions { requested_backend });

    let mut interpreter_runs = Vec::new();
    for index in 0..iterations {
        let build_root = PathBuf::from("build")
            .join("jit-bench")
            .join(format!("iter-{index:03}"));
        let started = Instant::now();
        match run_file(
            Path::new(&path),
            &build_root,
            &RunOptions {
                open_report: false,
                save_artifacts: true,
                args: runtime_args.clone(),
                ..RunOptions::default()
            },
        ) {
            Ok(output) => {
                interpreter_runs.push(BenchRun {
                    iteration: index + 1,
                    elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
                    result_path: output.result_path.display().to_string(),
                });
            }
            Err(RuntimeError::Compile(report)) => {
                print_diagnostics(&report);
                return ExitCode::from(2);
            }
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
        }
    }

    println!(
        "{}",
        jit_bench_json(&path, iterations, &report, &plan, &interpreter_runs)
    );
    ExitCode::SUCCESS
}

fn command_ide_check(args: Vec<String>) -> ExitCode {
    let Some(path) = first_non_flag(&args) else {
        eprintln!("usage: eng ide-check <file.eng>");
        return ExitCode::from(2);
    };
    let check_args = match parse_arg_overrides(&args, &[], &[]) {
        Ok(values) => values,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let report = match check_file(
        &path,
        &CheckOptions {
            review: true,
            args: check_args,
            require_args: false,
        },
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };

    print!("{}", review_json(&report));

    if report.has_errors() {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}

fn command_doctor() -> ExitCode {
    let repo_root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let report = doctor(&repo_root);

    println!("EngLang {}", env!("CARGO_PKG_VERSION"));
    println!();
    for check in &report.checks {
        println!(
            "{:<20} {}",
            check.name,
            if check.ok { "OK" } else { "FAIL" }
        );
    }
    println!();

    if report.ready() {
        println!("Ready.");
        ExitCode::SUCCESS
    } else {
        println!("Not ready. Run `dev.bat setup` from the repository root.");
        ExitCode::from(1)
    }
}

fn command_check(args: Vec<String>) -> ExitCode {
    let Some(path) = first_non_flag(&args) else {
        eprintln!("usage: eng check <file.eng> [--review]");
        return ExitCode::from(2);
    };
    let write_review = args.iter().any(|arg| arg == "--review");
    let check_args = match parse_arg_overrides(&args, &[], &["--review"]) {
        Ok(values) => values,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let report = match check_file(
        &path,
        &CheckOptions {
            review: write_review,
            args: check_args,
            require_args: false,
        },
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };

    print_diagnostics(&report);

    if write_review {
        let review_path = Path::new("build")
            .join("check")
            .join(format!("{}.review.json", file_stem(&path)));
        if let Some(parent) = review_path.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
        }
        if let Err(error) = std::fs::write(&review_path, review_json(&report)) {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
        println!("review: {}", review_path.display());
    }

    if report.has_errors() {
        ExitCode::from(2)
    } else {
        println!(
            "check passed: {} warning(s)",
            report.diagnostic_count(Severity::Warning)
        );
        ExitCode::SUCCESS
    }
}

fn command_fmt(args: Vec<String>) -> ExitCode {
    let positional: Vec<&String> = args.iter().filter(|arg| !arg.starts_with("--")).collect();
    if positional.len() != 1 {
        eprintln!("usage: eng fmt <file.eng> [--check|--write]");
        return ExitCode::from(2);
    }
    let check = args.iter().any(|arg| arg == "--check");
    let write = args.iter().any(|arg| arg == "--write");
    if check && write {
        eprintln!("eng fmt accepts only one of --check or --write");
        return ExitCode::from(2);
    }
    if let Some(unknown) = args
        .iter()
        .find(|arg| arg.starts_with("--") && *arg != "--check" && *arg != "--write")
    {
        eprintln!("unknown eng fmt option: {unknown}");
        return ExitCode::from(2);
    }

    let path = Path::new(positional[0]);
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let result = format_source(&source);

    if check {
        if result.changed {
            eprintln!("not formatter-clean: {}", path.display());
            return ExitCode::from(2);
        }
        println!("formatter-clean: {}", path.display());
        return ExitCode::SUCCESS;
    }

    if write {
        if result.changed {
            if let Err(error) = std::fs::write(path, result.formatted) {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
            println!("formatted {}", path.display());
        } else {
            println!("unchanged {}", path.display());
        }
        return ExitCode::SUCCESS;
    }

    print!("{}", result.formatted);
    ExitCode::SUCCESS
}

fn command_run(args: Vec<String>) -> ExitCode {
    let Some(path) = first_non_flag(&args) else {
        eprintln!(
            "usage: eng run <file.eng> [--profile safe|normal|repro] [--open-report] [--save-artifacts]"
        );
        return ExitCode::from(2);
    };
    let open_report = args.iter().any(|arg| arg == "--open-report");
    let save_artifacts = open_report || args.iter().any(|arg| arg == "--save-artifacts");
    let profile = match parse_execution_profile(&args) {
        Ok(profile) => profile,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let runtime_args = match parse_arg_overrides(
        &args,
        &["--profile"],
        &["--open-report", "--save-artifacts"],
    ) {
        Ok(values) => values,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };

    match run_file(
        Path::new(&path),
        Path::new("build"),
        &RunOptions {
            open_report,
            save_artifacts,
            args: runtime_args,
            profile,
        },
    ) {
        Ok(output) => {
            if !output.stdout.is_empty() {
                print!("{}", output.stdout);
            }
            if output.artifacts_saved {
                println!("artifacts: saved");
                println!("bytecode: {}", output.bytecode_path.display());
                println!("result:   {}", output.result_path.display());
                println!("review:   {}", output.review_path.display());
                println!("runlog:   {}", output.run_log_path.display());
                println!("process:  {}", output.process_results_path.display());
                println!("tests:    {}", output.test_results_path.display());
                println!("reportspec: {}", output.report_spec_path.display());
                println!("plot:     {}", output.plot_path.display());
                println!("plotspec: {}", output.plot_spec_path.display());
                println!("plotmanifest: {}", output.plot_manifest_path.display());
                println!("outputs:  {}", output.output_manifest_path.display());
                println!("report:   {}", output.report_path.display());
            } else {
                println!("run: ok");
                println!("artifacts: in memory");
                println!("result:   {} bytes", output.result_json.len());
                println!("review:   {} bytes", output.review_json.len());
                println!("runlog:   {} bytes", output.run_log_json.len());
                println!("process:  {} bytes", output.process_results_json.len());
                println!("tests:    {} bytes", output.test_results_json.len());
                println!("reportspec: {} bytes", output.report_spec_json.len());
                println!("plot:     {} bytes", output.plot_svg.len());
                println!("plotspec: {} bytes", output.plot_spec_json.len());
                println!("plotmanifest: {} bytes", output.plot_manifest_json.len());
                println!("outputs:  {} bytes", output.output_manifest_json.len());
                println!("report:   {} bytes", output.report_html.len());
                println!("use --save-artifacts to write build\\result files");
            }
            for path in &output.csv_export_paths {
                println!("export:   {}", path.display());
            }
            for path in &output.write_output_paths {
                println!("write:    {}", path.display());
            }
            for path in &output.file_operation_paths {
                println!("fs:       {}", path.display());
            }
            ExitCode::SUCCESS
        }
        Err(RuntimeError::Compile(report)) => {
            print_diagnostics(&report);
            ExitCode::from(2)
        }
        Err(RuntimeError::TestsFailed(message)) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
        Err(RuntimeError::Io(error)) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
        Err(RuntimeError::Bytecode(error)) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
        Err(RuntimeError::Vm(error)) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn command_build(args: Vec<String>) -> ExitCode {
    let Some(path) = first_non_flag(&args) else {
        eprintln!("usage: eng build <file.eng> [--standalone] [--profile repro]");
        return ExitCode::from(2);
    };
    if let Some(profile) = option_value(&args, "--profile") {
        if profile != "repro" {
            eprintln!("eng build currently supports only `--profile repro`");
            return ExitCode::from(2);
        }
    }
    let build_args = match parse_arg_overrides(&args, &["--profile"], &["--standalone"]) {
        Ok(values) => values,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };

    match build_standalone(
        Path::new(&path),
        Path::new("dist"),
        &BuildOptions { args: build_args },
    ) {
        Ok(output) => {
            println!("standalone package");
            println!("bundle:     {}", output.bundle_path.display());
            println!("executable: {}", output.executable_path.display());
            println!("runner:     {}", output.runner_path.display());
            println!("package:    {}", output.package_path.display());
            println!("lock:       {}", output.lock_path.display());
            println!("bytecode:   {}", output.bytecode_path.display());
            println!("source:     {}", output.source_path.display());
            println!("review:     {}", output.review_path.display());
            ExitCode::SUCCESS
        }
        Err(RuntimeError::Compile(report)) => {
            print_diagnostics(&report);
            ExitCode::from(2)
        }
        Err(RuntimeError::TestsFailed(message)) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
        Err(RuntimeError::Io(error)) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
        Err(RuntimeError::Bytecode(error)) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
        Err(RuntimeError::Vm(error)) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn command_view(args: Vec<String>) -> ExitCode {
    let Some(path) = first_non_flag(&args) else {
        eprintln!("usage: eng view <result.engres>");
        return ExitCode::from(2);
    };

    let result_path = PathBuf::from(path);
    if !result_path.exists() {
        eprintln!("result not found: {}", result_path.display());
        return ExitCode::from(1);
    }

    let report_path = result_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("report.html");
    println!("result: {}", result_path.display());
    if report_path.exists() {
        println!("report: {}", report_path.display());
    } else {
        println!("report: not found next to result");
    }
    let report_spec_path = result_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("report_spec.json");
    if report_spec_path.exists() {
        println!("spec:   {}", report_spec_path.display());
    } else {
        println!("spec:   not found next to result");
    }
    let plot_manifest_path = result_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("plots")
        .join("plot_manifest.json");
    if plot_manifest_path.exists() {
        println!("plots:  {}", plot_manifest_path.display());
    } else {
        println!("plots:  not found next to result");
    }
    ExitCode::SUCCESS
}

fn command_new(args: Vec<String>) -> ExitCode {
    let Some(path) = first_non_flag(&args) else {
        eprintln!("usage: eng new <project_name>");
        return ExitCode::from(2);
    };

    match create_project(Path::new(&path)) {
        Ok(()) => {
            println!("created {}", path);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn command_test(_args: Vec<String>) -> ExitCode {
    let example_groups: [(&str, &[&str]); 3] = [
        (
            "official",
            &[
                "examples/official/01_csv_plot/main.eng",
                "examples/official/02_simple_system/main.eng",
                "examples/official/03_integrated_hvac/main.eng",
                "examples/official/06_domain_port/main.eng",
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
                "examples/official/17_measured_vs_simulated/main.eng",
                "examples/official/19_class_object/main.eng",
                "examples/official/20_multi_state_thermal/main.eng",
                "examples/official/21_thermal_component_assembly/main.eng",
                "examples/official/22_multi_domain_boundary_solve/main.eng",
            ],
        ),
        (
            "internal",
            &[
                "examples/internal/18_state_space_metadata/main.eng",
                "examples/internal/21_unsupported_system_shape/main.eng",
                "examples/internal/26_state_space_discrete/main.eng",
            ],
        ),
        (
            "compatibility regression",
            &[
                "examples/01_units/main.eng",
                "examples/02_csv_plot/main.eng",
                "examples/04_plotting/main.eng",
                "examples/06_simple_system/main.eng",
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

    if !official_examples_are_formatter_clean() {
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

    let domain_port = match check_file(
        "examples/official/06_domain_port/main.eng",
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
    println!("ok: examples/official/06_domain_port/main.eng produced domain assembly metadata");
    match run_file(
        Path::new("examples/official/06_domain_port/main.eng"),
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
                "ok: examples/official/06_domain_port/main.eng produced component constraint-check artifacts"
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
        Path::new("examples/official/21_thermal_component_assembly/main.eng"),
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
                    "expected official thermal component assembly example to solve a square linear residual graph"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/official/21_thermal_component_assembly/main.eng solved thermal component assembly residual graph"
            );
        }
        Err(error) => {
            eprintln!("thermal component assembly example failed: {error}");
            return ExitCode::from(1);
        }
    }
    let thermal_assembly_report = match check_file(
        "examples/official/21_thermal_component_assembly/main.eng",
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
        "examples/official/21_thermal_component_assembly/main.eng",
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
            "expected official thermal component assembly example to expose lowerable component residual, Jacobian, Newton-step kernel candidates, and benchmark target coverage"
        );
        return ExitCode::from(2);
    }
    println!(
        "ok: examples/official/21_thermal_component_assembly/main.eng produced component residual, Jacobian, and Newton-step kernel candidates"
    );
    match run_file(
        Path::new("examples/official/22_multi_domain_boundary_solve/main.eng"),
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
                    "expected official multi-domain boundary fixture to solve a square residual graph across Thermal, Fluid, and MechanicalNode domains"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/official/22_multi_domain_boundary_solve/main.eng solved a small multi-domain boundary residual graph"
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
                    .contains("\"delay_call_runtime_buffer_seed_not_integrated\"")
                || !output
                    .report_spec_json
                    .contains("\"predictor_call_contract_seed_not_integrated\"")
                || !output
                    .report_spec_json
                    .contains("\"external_behavior_wrapper_seed_not_integrated\"")
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
                    "behavior graph nodes are present but not yet integrated into numeric residual evaluation",
                )
                || !output.report_html.contains("solver_policy_not_integrated")
                || !output
                    .report_html
                    .contains("safe_repro_profile_policy_seed")
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
        "examples/05_error_messages/unit_mismatch.eng",
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
    println!("ok: examples/05_error_messages/unit_mismatch.eng produced diagnostics");

    let ambiguous = match check_file(
        "examples/05_error_messages/ambiguous_power.eng",
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
    println!("ok: examples/05_error_messages/ambiguous_power.eng produced warning");

    let heat_rate_sum = match check_file(
        "examples/05_error_messages/heat_rate_sum.eng",
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
    println!("ok: examples/05_error_messages/heat_rate_sum.eng produced warning");

    let missing_column = match check_file(
        "examples/05_error_messages/missing_csv_column.eng",
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
    println!("ok: examples/05_error_messages/missing_csv_column.eng produced diagnostics");

    let eq_boolean = match check_file(
        "examples/05_error_messages/eq_boolean.eng",
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
    println!("ok: examples/05_error_messages/eq_boolean.eng produced diagnostics");

    let equation_unit_mismatch = match check_file(
        "examples/05_error_messages/equation_unit_mismatch.eng",
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
    println!("ok: examples/05_error_messages/equation_unit_mismatch.eng produced diagnostics");

    let port_domain_mismatch = match check_file(
        "examples/05_error_messages/port_domain_mismatch.eng",
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
    println!("ok: examples/05_error_messages/port_domain_mismatch.eng produced diagnostics");

    for (fixture, expected_code) in [
        (
            "examples/05_error_messages/medium_mismatch.eng",
            "E-CONNECT-MEDIUM-MISMATCH",
        ),
        (
            "examples/05_error_messages/frame_mismatch.eng",
            "E-CONNECT-FRAME-001",
        ),
        (
            "examples/05_error_messages/axis_mismatch.eng",
            "E-CONNECT-AXIS-001",
        ),
        (
            "examples/05_error_messages/duplicate_connection.eng",
            "E-CONNECT-DUPLICATE-001",
        ),
        (
            "examples/05_error_messages/connect_unknown_port.eng",
            "E-CONNECT-UNKNOWN-PORT",
        ),
        (
            "examples/05_error_messages/connect_bad_endpoint.eng",
            "E-CONNECT-ENDPOINT-001",
        ),
        (
            "examples/05_error_messages/unconnected_port.eng",
            "W-CONNECT-UNCONNECTED-PORT",
        ),
        (
            "examples/05_error_messages/generic_domain_arity.eng",
            "E-PORT-DOMAIN-002",
        ),
        (
            "examples/05_error_messages/domain_missing_across.eng",
            "E-DOMAIN-CONTRACT-001",
        ),
        (
            "examples/05_error_messages/domain_missing_through.eng",
            "E-DOMAIN-CONTRACT-002",
        ),
        (
            "examples/05_error_messages/domain_missing_conservation.eng",
            "E-DOMAIN-CONTRACT-003",
        ),
        (
            "examples/05_error_messages/domain_unknown_quantity.eng",
            "E-DOMAIN-VAR-001",
        ),
        (
            "examples/05_error_messages/class_missing_field.eng",
            "E-CLASS-FIELD-MISSING-001",
        ),
        (
            "examples/05_error_messages/class_unknown_field.eng",
            "E-CLASS-FIELD-UNKNOWN-001",
        ),
        (
            "examples/05_error_messages/class_field_type_mismatch.eng",
            "E-CLASS-FIELD-TYPE-001",
        ),
        (
            "examples/05_error_messages/class_validation_fail.eng",
            "E-CLASS-VALIDATION-002",
        ),
        (
            "examples/05_error_messages/class_method_return_mismatch.eng",
            "E-CLASS-METHOD-RETURN-001",
        ),
        (
            "examples/05_error_messages/class_method_unknown.eng",
            "E-CLASS-METHOD-CALL-002",
        ),
        (
            "examples/05_error_messages/class_copy_unknown_source.eng",
            "E-CLASS-COPY-001",
        ),
        (
            "examples/05_error_messages/component_delay_bad_call.eng",
            "E-DELAY-CALL-001",
        ),
        (
            "examples/05_error_messages/component_delay_bad_duration.eng",
            "E-DELAY-DURATION-001",
        ),
        (
            "examples/05_error_messages/component_delay_unknown_signal.eng",
            "E-DELAY-SIGNAL-001",
        ),
        (
            "examples/05_error_messages/component_predictor_bad_call.eng",
            "E-PREDICTOR-CALL-001",
        ),
        (
            "examples/05_error_messages/component_predictor_unknown_signal.eng",
            "E-PREDICTOR-SIGNAL-001",
        ),
        (
            "examples/05_error_messages/component_external_bad_call.eng",
            "E-EXTERNAL-BEHAVIOR-CALL-001",
        ),
        (
            "examples/05_error_messages/component_external_unknown_signal.eng",
            "E-EXTERNAL-BEHAVIOR-SIGNAL-001",
        ),
        (
            "examples/05_error_messages/component_boundary_unknown_signal.eng",
            "E-ASSEMBLY-BOUNDARY-SIGNAL-001",
        ),
        (
            "examples/05_error_messages/component_boundary_bad_rhs.eng",
            "E-ASSEMBLY-BOUNDARY-RHS-001",
        ),
        (
            "examples/05_error_messages/component_boundary_unit_mismatch.eng",
            "E-ASSEMBLY-BOUNDARY-UNIT-001",
        ),
        (
            "examples/05_error_messages/simulate_unknown_system.eng",
            "E-SIM-SYSTEM-001",
        ),
        (
            "examples/05_error_messages/simulate_missing_input.eng",
            "E-SIM-MISSING-INPUT",
        ),
        (
            "examples/05_error_messages/simulate_input_not_timeseries.eng",
            "E-SIM-INPUT-AXIS-MISMATCH",
        ),
        (
            "examples/05_error_messages/simulate_input_axis_mismatch.eng",
            "E-SIM-INPUT-AXIS-MISMATCH",
        ),
        (
            "examples/05_error_messages/simulate_input_quantity_mismatch.eng",
            "E-SIM-INPUT-QTY-MISMATCH",
        ),
        (
            "examples/05_error_messages/simulate_missing_timestep.eng",
            "E-SIM-TIMESTEP-INVALID",
        ),
        (
            "examples/05_error_messages/simulate_bad_timestep.eng",
            "E-SIM-TIMESTEP-INVALID",
        ),
        (
            "examples/05_error_messages/simulate_missing_solver.eng",
            "E-SIM-SOLVER-UNSUPPORTED",
        ),
        (
            "examples/05_error_messages/simulate_unsupported_solver.eng",
            "E-SIM-SOLVER-UNSUPPORTED",
        ),
        (
            "examples/05_error_messages/state_space_missing_operator_entry.eng",
            "E-STATE-SPACE-OP-SHAPE-001",
        ),
        (
            "examples/05_error_messages/state_space_operator_unit_mismatch.eng",
            "E-STATE-SPACE-OP-ENTRY-UNIT-001",
        ),
        (
            "examples/05_error_messages/state_space_operator_bad_coefficient.eng",
            "E-STATE-SPACE-OP-ENTRY-VALUE-001",
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
        "examples/05_error_messages/missing_uncertainty_source.eng",
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
    println!("ok: examples/05_error_messages/missing_uncertainty_source.eng produced diagnostics");

    let invalid_uncertainty_arguments = match check_file(
        "examples/05_error_messages/invalid_uncertainty_arguments.eng",
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
        "ok: examples/05_error_messages/invalid_uncertainty_arguments.eng produced diagnostics"
    );

    let missing_ml_source = match check_file(
        "examples/05_error_messages/missing_ml_source.eng",
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
    println!("ok: examples/05_error_messages/missing_ml_source.eng produced diagnostics");

    let invalid_ml_arguments = match check_file(
        "examples/05_error_messages/invalid_ml_arguments.eng",
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
    println!("ok: examples/05_error_messages/invalid_ml_arguments.eng produced diagnostics");

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
            if output.csv_export_paths.is_empty()
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
            if output.file_operation_paths.len() != 4
                || !review.contains("\"file_operations\"")
                || !manifest.contains("\"kind\": \"copy_file\"")
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
        Path::new("examples/official/17_measured_vs_simulated/main.eng"),
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
                || !review.contains("\"states\": [\"T_zone\"]")
                || !review.contains("\"inputs\": [\"T_out\", \"Q_internal\"]")
                || !review.contains("\"parameters\": [\"C\", \"UA\"]")
                || !review.contains("\"outputs\": [\"T_zone\"]")
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
                || !plot_spec.contains("\"name\": \"measured_data.T_zone\"")
                || !plot_spec.contains("\"name\": \"sim.T_zone\"")
            {
                eprintln!("expected measured-vs-simulated example to produce SolverResult state/input/parameter/output, method/timestep/final-state metadata, RMSE TemperatureDelta/K, validation, alignment, and multi-series plot artifacts");
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/official/17_measured_vs_simulated/main.eng produced measured-vs-simulated artifacts"
            );
        }
        Err(error) => {
            eprintln!("measured-vs-simulated example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/official/17_measured_vs_simulated/main.eng"),
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
                "ok: examples/official/17_measured_vs_simulated/main.eng produced repro-profile artifacts"
            );
        }
        Err(error) => {
            eprintln!("measured-vs-simulated repro example failed: {error}");
            return ExitCode::from(2);
        }
    }
    if !measured_fixture_records_time_overlap(
        "examples/official/17_measured_vs_simulated/main.eng",
        "build/test-measured-vs-simulated-time-mismatch",
        "data/measured_zone_time_mismatch.csv",
    ) {
        return ExitCode::from(2);
    }
    if !measured_fixture_records_missing_policy(
        "examples/official/17_measured_vs_simulated/main.eng",
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
                || !result.contains("\"failure_reason\": \"system shape is outside the supported first-order thermal ODE runner\"")
                || !report_spec.contains("\"convergence_status\": \"skipped_unsupported_shape\"")
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
        Path::new("examples/official/20_multi_state_thermal/main.eng"),
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
                "ok: examples/official/20_multi_state_thermal/main.eng produced multi-state solver artifacts"
            );
        }
        Err(error) => {
            eprintln!("multi-state thermal example failed: {error}");
            return ExitCode::from(2);
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
        Path::new("examples/official/02_simple_system/main.eng"),
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
                "ok: examples/official/02_simple_system/main.eng produced system report artifacts"
            );
        }
        Err(error) => {
            eprintln!("simple system example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/official/03_integrated_hvac/main.eng"),
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
                "ok: examples/official/03_integrated_hvac/main.eng produced integrated user-test artifacts"
            );
        }
        Err(error) => {
            eprintln!("integrated HVAC example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/official/04_uncertainty_core/main.eng"),
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
                "ok: examples/official/04_uncertainty_core/main.eng produced uncertainty artifacts"
            );
        }
        Err(error) => {
            eprintln!("uncertainty core example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/official/05_data_driven_modeling/main.eng"),
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
            if !result.contains("\"ml\"")
                || !result.contains("\"rmse\"")
                || !result.contains("\"model_card\"")
                || !result.contains("\"leakage_status\"")
                || !result.contains("\"coefficients\"")
                || !result.contains("\"loss_history\"")
                || !review.contains("\"ml_info\"")
                || !report_spec.contains("\"ml\"")
                || !plot_spec.contains("\"plot_type\": \"scatter\"")
                || !plot_spec.contains("Regression parity")
            {
                eprintln!(
                    "expected data-driven example to produce ML metrics, model card, leakage lint, and parity plot"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/official/05_data_driven_modeling/main.eng produced ML artifacts"
            );
        }
        Err(error) => {
            eprintln!("data-driven modeling example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/official/05_data_driven_modeling/residuals.eng"),
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
                "ok: examples/official/05_data_driven_modeling/residuals.eng produced residual plot artifacts"
            );
        }
        Err(error) => {
            eprintln!("data-driven residual example failed: {error}");
            return ExitCode::from(2);
        }
    }

    if !data_quality_fixture_records_parse_failure(
        "examples/07_data_quality/bad_datetime_cell.eng",
        "build/test-bad-datetime",
        "expected UTC DateTime",
    ) {
        return ExitCode::from(2);
    }
    if !data_quality_fixture_records_parse_failure(
        "examples/07_data_quality/bad_numeric_cell.eng",
        "build/test-bad-numeric",
        "expected finite numeric cell",
    ) {
        return ExitCode::from(2);
    }
    if !data_quality_fixture_records_interpolation(
        "examples/07_data_quality/interpolate_missing.eng",
        "build/test-interpolate-missing",
    ) {
        return ExitCode::from(2);
    }
    if !data_quality_fixture_records_constraint_violation(
        "examples/07_data_quality/constraint_violation.eng",
        "build/test-constraint-violation",
    ) {
        return ExitCode::from(2);
    }
    if !data_quality_fixture_records_conversion_failure(
        "examples/07_data_quality/unsupported_unit_conversion.eng",
        "build/test-unit-conversion-failure",
    ) {
        return ExitCode::from(2);
    }

    let path_smoke_root = Path::new("build").join("path smoke").join("한글 경로");
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
                    if !report_spec.exists() || !plot_spec.exists() {
                        eprintln!(
                            "expected standalone runner to produce report and PlotSpec artifacts"
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
        Path::new("examples/official/17_measured_vs_simulated/main.eng"),
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
                    let result = std::fs::read_to_string(&result_path).unwrap_or_default();
                    let plot_spec = std::fs::read_to_string(&plot_spec_path).unwrap_or_default();
                    if !result.contains("\"binding\": \"rmse_T\"")
                        || !result.contains("\"validations\"")
                        || !result.contains("\"time_alignments\"")
                        || !plot_spec.contains("\"name\": \"measured_data.T_zone\"")
                        || !plot_spec.contains("\"name\": \"sim.T_zone\"")
                    {
                        eprintln!(
                            "expected measured-vs-simulated standalone runner to produce metric, validation, alignment, and multi-series plot artifacts"
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

fn official_examples_are_formatter_clean() -> bool {
    let mut examples = Vec::new();
    if let Err(error) = collect_eng_files(Path::new("examples/official"), &mut examples) {
        eprintln!("failed to enumerate official examples: {error}");
        return false;
    }
    examples.sort();

    for example in examples {
        let source = match std::fs::read_to_string(&example) {
            Ok(source) => source,
            Err(error) => {
                eprintln!(
                    "failed to read official example {}: {error}",
                    example.display()
                );
                return false;
            }
        };
        if format_source(&source).changed {
            eprintln!(
                "expected official example to be formatter-clean: {}",
                example.display()
            );
            return false;
        }
    }

    println!("ok: official examples are formatter-clean");
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
                    expression: eng_runtime::solver::ResidualExpression {
                        text: (*name).to_owned(),
                    },
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
    }
}

fn solver_behavior_smoke() -> Result<(), String> {
    delay_behavior_smoke()?;
    predictor_behavior_smoke()?;
    external_behavior_smoke()?;
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

struct BenchRun {
    iteration: usize,
    elapsed_ms: f64,
    result_path: String,
}

fn jit_bench_json(
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

fn jit_bench_has_target(
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

fn jit_bench_has_executor_sample(bench_json: &str, candidate: &str, status: &str) -> bool {
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
            if has_candidate(plan, "component_residual_graph") {
                "covered_by_current_source"
            } else if has_candidate(plan, "system_residual") {
                "interface_only"
            } else {
                "not_observed_for_source"
            },
            candidates_by_kind(
                plan,
                &[
                    "component_residual_graph",
                    "component_residual_jacobian",
                    "system_residual",
                ],
            ),
            "tracks residual evaluator candidates; system residuals may still be interface-only",
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
            if !result.contains("\"sample_count\": 4")
                || !result.contains("\"matched_count\": 4")
                || !result.contains("\"status\": \"overlap\"")
                || !result.contains("\"violation_count\": 0")
            {
                eprintln!(
                    "expected {source} with {measured_fixture} to record partial TimeSeries overlap without policy violations"
                );
                return false;
            }
            println!("ok: {source} recorded measured/simulated partial TimeSeries overlap");
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
            if !result.contains("\"sample_count\": 6")
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

fn first_non_flag(args: &[String]) -> Option<String> {
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--profile" {
            skip_next = true;
            continue;
        }
        if !arg.starts_with('-') {
            return Some(arg.clone());
        }
    }
    None
}

fn option_value(args: &[String], name: &str) -> Option<String> {
    let inline_prefix = format!("{name}=");
    if let Some(value) = args
        .iter()
        .find_map(|arg| arg.strip_prefix(&inline_prefix).map(str::to_owned))
    {
        return Some(value);
    }
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].clone())
}

fn parse_jit_backend(args: &[String]) -> Result<String, String> {
    let backend = option_value(args, "--backend")
        .unwrap_or_else(|| eng_jit::DEFAULT_BACKEND_REQUEST.to_owned());
    match backend.as_str() {
        eng_jit::DEFAULT_BACKEND_REQUEST
        | eng_jit::INTERPRETER_FALLBACK_BACKEND
        | eng_jit::NATIVE_PREVIEW_BACKEND => Ok(backend),
        _ => Err(format!(
            "unknown JIT backend `{backend}`; expected auto, interpreter-fallback, or native-preview"
        )),
    }
}

fn parse_execution_profile(args: &[String]) -> Result<ExecutionProfile, String> {
    let Some(profile) = option_value(args, "--profile") else {
        return Ok(ExecutionProfile::Normal);
    };
    ExecutionProfile::parse(&profile).ok_or_else(|| {
        format!("unknown execution profile `{profile}`; expected safe, normal, or repro")
    })
}

fn parse_arg_overrides(
    args: &[String],
    known_value_flags: &[&str],
    known_bool_flags: &[&str],
) -> Result<Vec<ArgOverride>, String> {
    let mut values = Vec::new();
    let mut index = 0usize;
    while index < args.len() {
        let arg = &args[index];
        if !arg.starts_with("--") {
            index += 1;
            continue;
        }
        if known_bool_flags.contains(&arg.as_str()) {
            index += 1;
            continue;
        }
        if let Some(flag) = known_value_flags
            .iter()
            .find(|flag| arg.as_str() == **flag || arg.starts_with(&format!("{}=", flag)))
        {
            index += if arg.as_str() == *flag { 2 } else { 1 };
            continue;
        }
        if let Some((name, value)) = arg.split_once('=') {
            values.push(ArgOverride {
                name: name.trim_start_matches("--").replace('-', "_"),
                value: value.to_owned(),
            });
            index += 1;
            continue;
        }
        let Some(value) = args.get(index + 1) else {
            return Err(format!("missing value for Args flag `{arg}`"));
        };
        if value.starts_with("--") {
            return Err(format!("missing value for Args flag `{arg}`"));
        }
        values.push(ArgOverride {
            name: arg.trim_start_matches("--").replace('-', "_"),
            value: value.clone(),
        });
        index += 2;
    }
    Ok(values)
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

fn print_diagnostics(report: &eng_compiler::CheckReport) {
    for diagnostic in &report.diagnostics {
        println!(
            "{}:{}:{}: {}",
            report.source_path.display(),
            diagnostic.line,
            diagnostic.code,
            diagnostic.message
        );
        if let Some(help) = &diagnostic.help {
            println!("  help: {help}");
        }
    }
}

fn file_stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("source")
        .to_owned()
}

fn print_help() {
    println!(
        r#"EngLang {version}

Usage:
  eng doctor
  eng new <project_name>
  eng check <file.eng> [--review]
  eng fmt <file.eng> [--check|--write]
  eng ide-check <file.eng>
  eng jit-plan <file.eng>
  eng jit-bench <file.eng> [--iterations N] [--<arg> <value>...]
  eng run <file.eng> [--profile safe|normal|repro] [--open-report] [--save-artifacts] [--<arg> <value>...]
  eng build <file.eng> [--standalone] [--profile repro]
  eng view <result.engres>
  eng test <project_or_examples>

The supported core path intentionally stays free of Python dependencies.
"#,
        version = env!("CARGO_PKG_VERSION")
    );
}
