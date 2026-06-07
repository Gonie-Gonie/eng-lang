use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::ExitCode;

use eng_compiler::{check_file, review_json, ArgOverride, CheckOptions, Severity};
use eng_runtime::{
    build_standalone, create_project, doctor, run_file, BuildOptions, RunOptions, RuntimeError,
};

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
        "entries" => command_entries(args),
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

fn command_entries(args: Vec<String>) -> ExitCode {
    let Some(path) = first_non_flag(&args) else {
        eprintln!("usage: eng entries <file.eng>");
        return ExitCode::from(2);
    };
    let report = match check_file(&path, &CheckOptions::default()) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };

    if report.semantic_program.entry_points.is_empty() {
        println!("No entry points found.");
    } else {
        for entry in &report.semantic_program.entry_points {
            println!("{}:{}: {}", path, entry.line, entry.signature());
        }
    }

    if report.has_errors() {
        print_diagnostics(&report);
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}

fn command_run(args: Vec<String>) -> ExitCode {
    let Some(path) = first_non_flag(&args) else {
        eprintln!("usage: eng run <file.eng> [--entry <name>] [--open-report]");
        return ExitCode::from(2);
    };
    let open_report = args.iter().any(|arg| arg == "--open-report");
    let entry = option_value(&args, "--entry");
    let runtime_args = match parse_arg_overrides(&args, &["--entry"], &["--open-report"]) {
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
            entry,
            args: runtime_args,
        },
    ) {
        Ok(output) => {
            println!("bytecode: {}", output.bytecode_path.display());
            println!("result:   {}", output.result_path.display());
            println!("review:   {}", output.review_path.display());
            println!("reportspec: {}", output.report_spec_path.display());
            println!("plot:     {}", output.plot_path.display());
            println!("plotspec: {}", output.plot_spec_path.display());
            println!("manifest: {}", output.plot_manifest_path.display());
            println!("report:   {}", output.report_path.display());
            ExitCode::SUCCESS
        }
        Err(RuntimeError::Compile(report)) => {
            print_diagnostics(&report);
            ExitCode::from(2)
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
        eprintln!("usage: eng build <file.eng> [--entry <name>] [--standalone] [--profile repro]");
        return ExitCode::from(2);
    };
    let entry = option_value(&args, "--entry");
    let build_args = match parse_arg_overrides(&args, &["--entry", "--profile"], &["--standalone"])
    {
        Ok(values) => values,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };

    match build_standalone(
        Path::new(&path),
        Path::new("dist"),
        &BuildOptions {
            entry,
            args: build_args,
        },
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
    let examples = [
        "examples/01_units/main.eng",
        "examples/official/01_csv_plot/main.eng",
        "examples/official/02_simple_system/main.eng",
        "examples/02_csv_plot/main.eng",
        "examples/04_plotting/main.eng",
        "examples/06_simple_system/main.eng",
    ];

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
        println!("ok: {example}");
    }

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

    match run_file(
        Path::new("examples/05_error_messages/missing_entry.eng"),
        Path::new("build/test-missing-entry"),
        &RunOptions::default(),
    ) {
        Err(RuntimeError::Compile(report))
            if report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-ENTRY-NOT-FOUND-001") =>
        {
            println!("ok: examples/05_error_messages/missing_entry.eng requires an entry point");
        }
        Err(error) => {
            eprintln!("expected missing_entry.eng to fail with E-ENTRY-NOT-FOUND-001: {error}");
            return ExitCode::from(2);
        }
        Ok(_) => {
            eprintln!("expected missing_entry.eng to fail");
            return ExitCode::from(2);
        }
    }

    match run_file(
        Path::new("examples/official/01_csv_plot/main.eng"),
        Path::new("build/test-plot"),
        &RunOptions::default(),
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
        Path::new("examples/official/01_csv_plot/main.eng"),
        Path::new("build/test-plot-args"),
        &RunOptions {
            open_report: false,
            entry: Some("main".to_owned()),
            args: vec![ArgOverride {
                name: "input".to_owned(),
                value: "data/sensor.csv".to_owned(),
            }],
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
    match run_file(
        Path::new("examples/official/02_simple_system/main.eng"),
        Path::new("build/test-system"),
        &RunOptions::default(),
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
    let source = r#"script main(args: Args) -> Report {
    L = 1 m + 20 cm

    return report {
        show L
    }
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
        &RunOptions::default(),
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
        &BuildOptions {
            entry: Some("main".to_owned()),
            args: Vec::new(),
        },
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
            let result = std::fs::read_to_string(output.result_path).unwrap_or_default();
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
            let result = std::fs::read_to_string(output.result_path).unwrap_or_default();
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

fn data_quality_fixture_records_constraint_violation(source: &str, build_root: &str) -> bool {
    match run_file(
        Path::new(source),
        Path::new(build_root),
        &RunOptions::default(),
    ) {
        Ok(output) => {
            let result = std::fs::read_to_string(output.result_path).unwrap_or_default();
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
            let result = std::fs::read_to_string(output.result_path).unwrap_or_default();
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
        if matches!(arg.as_str(), "--entry" | "--profile") {
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
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].clone())
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
        if known_value_flags.contains(&arg.as_str()) {
            index += 2;
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
  eng entries <file.eng>
  eng run <file.eng> [--entry <name>] [--open-report] [--<arg> <value>...]
  eng build <file.eng> [--entry <name>] [--standalone] [--profile repro]
  eng view <result.engres>
  eng test <project_or_examples>

This preview intentionally keeps the core path free of Python dependencies.
"#,
        version = env!("CARGO_PKG_VERSION")
    );
}
