use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::ExitCode;
use std::time::Instant;

use eng_compiler::{check_file, check_source, review_json, ArgOverride, CheckOptions, Severity};
use eng_runtime::{
    build_standalone, create_project, doctor, run_file, BuildOptions, ExecutionProfile, RunOptions,
    RuntimeError,
};
use serde_json::json;

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
        jit_bench_json(&path, iterations, &plan, &interpreter_runs)
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
    let example_groups: [(&str, &[&str]); 2] = [
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
                "examples/official/18_state_space_preview/main.eng",
                "examples/official/19_class_object_preview/main.eng",
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
    if jit_plan.candidates.len() < 3
        || !jit_plan
            .candidates
            .iter()
            .any(|candidate| candidate.kind == "timeseries_integrate")
    {
        eprintln!(
            "expected official CSV example to expose runtime optimization track kernel candidates"
        );
        return ExitCode::from(2);
    }
    println!("ok: official CSV example produced JIT kernel candidates");

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
        || !domain_review.contains("\"domain_compatible\"")
    {
        eprintln!(
            "expected domain port example to expose domain/component/connection review metadata"
        );
        return ExitCode::from(2);
    }
    println!("ok: examples/official/06_domain_port/main.eng produced domain port metadata");

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
        .any(|diagnostic| diagnostic.code == "E-CONNECT-DOMAIN-001")
    {
        eprintln!("expected port_domain_mismatch.eng to produce E-CONNECT-DOMAIN-001");
        return ExitCode::from(2);
    }
    println!("ok: examples/05_error_messages/port_domain_mismatch.eng produced diagnostics");

    for (fixture, expected_code) in [
        (
            "examples/05_error_messages/medium_mismatch.eng",
            "E-CONNECT-MEDIUM-001",
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
        Ok(output)
            if output.plot_spec_path.exists()
                && output.plot_manifest_path.exists()
                && output.report_spec_path.exists() =>
        {
            println!(
                "ok: examples/official/01_csv_plot/main.eng produced report and PlotSpec artifacts"
            );
        }
        Ok(_) => {
            eprintln!("expected plot example to produce report and PlotSpec artifacts");
            return ExitCode::from(2);
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
                || !manifest.contains("\"kind\": \"csv_export\"")
                || !manifest.contains("\"kind\": \"write_text\"")
                || !manifest.contains("\"kind\": \"write_json\"")
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
                || !process_results.contains("eng-process-ok")
                || !manifest.contains("\"kind\": \"process_results\"")
            {
                eprintln!(
                    "expected process result example to produce review, process_results, and manifest records"
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
                || !test_results.contains("\"failed_count\": 0")
                || !manifest.contains("\"kind\": \"test_results\"")
            {
                eprintln!(
                    "expected test/assert/golden example to produce review, test_results, and manifest records"
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
    match run_file(
        Path::new("examples/official/17_measured_vs_simulated/main.eng"),
        Path::new("build/test-measured-vs-simulated"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let result = std::fs::read_to_string(output.result_path).unwrap_or_default();
            let report_spec = std::fs::read_to_string(output.report_spec_path).unwrap_or_default();
            let report_html = std::fs::read_to_string(output.report_path).unwrap_or_default();
            let plot_spec = std::fs::read_to_string(output.plot_spec_path).unwrap_or_default();
            if !result.contains("\"metrics\"")
                || !result.contains("\"validations\"")
                || !result.contains("\"time_alignments\"")
                || !result.contains("\"binding\": \"rmse_T\"")
                || !report_spec.contains("\"computed_metrics\"")
                || !report_spec.contains("\"status\": \"passed\"")
                || !report_html.contains("Validations")
                || !plot_spec.contains("\"name\": \"measured_data.T_zone\"")
                || !plot_spec.contains("\"name\": \"sim.T_zone\"")
            {
                eprintln!("expected measured-vs-simulated example to produce RMSE, validation, alignment, and multi-series plot artifacts");
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
        Path::new("examples/official/18_state_space_preview/main.eng"),
        Path::new("build/test-state-space-preview"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let review = std::fs::read_to_string(&output.review_path).unwrap_or_default();
            if !review.contains("\"state_space_vectors\"")
                || !review.contains("\"linear_operators\"")
                || !review.contains("\"vector_type\": \"StateVector\"")
                || !review.contains("\"from\": \"InputVector\"")
                || !review.contains("\"to\": \"Derivative[StateVector]\"")
            {
                eprintln!(
                    "expected state-space preview example to record vector and operator metadata"
                );
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/official/18_state_space_preview/main.eng produced state-space metadata"
            );
        }
        Err(error) => {
            eprintln!("state-space preview example failed: {error}");
            return ExitCode::from(2);
        }
    }
    match run_file(
        Path::new("examples/official/19_class_object_preview/main.eng"),
        Path::new("build/test-class-object-preview"),
        &artifact_run_options(),
    ) {
        Ok(output) => {
            let review = std::fs::read_to_string(&output.review_path).unwrap_or_default();
            let report_spec = std::fs::read_to_string(output.report_spec_path).unwrap_or_default();
            let report_html = std::fs::read_to_string(output.report_path).unwrap_or_default();
            if !review.contains("\"class_summary\"")
                || !review.contains("\"object_summary\"")
                || !review.contains("\"Object[Construction]\"")
                || !report_spec.contains("\"class_summary\"")
                || !report_spec.contains("\"object_summary\"")
                || !report_html.contains("Classes")
                || !report_html.contains("Objects")
            {
                eprintln!("expected class object preview to expose class/object artifacts");
                return ExitCode::from(2);
            }
            println!(
                "ok: examples/official/19_class_object_preview/main.eng produced class object metadata"
            );
        }
        Err(error) => {
            eprintln!("class object preview example failed: {error}");
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

            let help_output = Command::new("cmd")
                .arg("/C")
                .arg("run.bat")
                .arg("--help")
                .current_dir(&output.bundle_path)
                .output();
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

            let status = Command::new("cmd")
                .arg("/C")
                .arg("run.bat")
                .current_dir(&output.bundle_path)
                .status();
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
    ExitCode::SUCCESS
}

struct BenchRun {
    iteration: usize,
    elapsed_ms: f64,
    result_path: String,
}

fn jit_bench_json(
    source_path: &str,
    iterations: usize,
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
  eng ide-check <file.eng>
  eng jit-plan <file.eng>
  eng jit-bench <file.eng> [--iterations N] [--<arg> <value>...]
  eng run <file.eng> [--profile safe|normal|repro] [--open-report] [--save-artifacts] [--<arg> <value>...]
  eng build <file.eng> [--standalone] [--profile repro]
  eng view <result.engres>
  eng test <project_or_examples>

This preview intentionally keeps the core path free of Python dependencies.
"#,
        version = env!("CARGO_PKG_VERSION")
    );
}
