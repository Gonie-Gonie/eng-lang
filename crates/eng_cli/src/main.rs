use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

mod example_smoke;
mod jit_bench;

use eng_compiler::{check_file, format_source, review_json, ArgOverride, CheckOptions, Severity};
use eng_runtime::{
    build_standalone, create_project, doctor, run_file, BuildOptions, ExecutionProfile, RunOptions,
    RuntimeError,
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
        "review" => command_review(args),
        "fmt" => command_fmt(args),
        "ide-check" => command_ide_check(args),
        "jit-plan" => command_jit_plan(args),
        "jit-bench" => command_jit_bench(args),
        "run" => command_run(args),
        "cache" => command_cache(args),
        "build" => command_build(args),
        "view" => command_view(args),
        "new" => command_new(args),
        "test" => example_smoke::command_test(args),
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
                interpreter_runs.push(jit_bench::BenchRun {
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
        jit_bench::jit_bench_json(&path, iterations, &report, &plan, &interpreter_runs)
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

fn command_review(args: Vec<String>) -> ExitCode {
    let Some(path) = first_review_source_path(&args) else {
        eprintln!(
            "usage: eng review <file.eng> [--json] [--output <dir>] [--against <review.json>] [--<arg> <value>...]"
        );
        return ExitCode::from(2);
    };
    let json_only = args.iter().any(|arg| arg == "--json");
    let output_dir = option_value(&args, "--output");
    let against_path = option_value(&args, "--against");
    let check_args = match parse_arg_overrides(&args, &["--output", "--against"], &["--json"]) {
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
    let review = review_json(&report);
    let value = match serde_json::from_str::<serde_json::Value>(&review) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to parse generated review artifact: {error}");
            return ExitCode::from(1);
        }
    };
    let document = value
        .get("review_document")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let semantic_diff = match against_path.as_deref() {
        Some(path) => match read_review_document(path) {
            Ok(previous) => Some(review_semantic_diff(&previous, &document)),
            Err(message) => {
                eprintln!("{message}");
                return ExitCode::from(1);
            }
        },
        None => None,
    };

    if let Some(output_dir) = output_dir.as_deref() {
        if let Err(message) =
            write_static_review_outputs(output_dir, &document, semantic_diff.as_ref())
        {
            eprintln!("{message}");
            return ExitCode::from(1);
        }
    }

    if json_only {
        let output = if let Some(diff) = semantic_diff {
            serde_json::json!({
                "review_document": document,
                "semantic_diff": diff
            })
        } else {
            document
        };
        match serde_json::to_string_pretty(&output) {
            Ok(text) => println!("{text}"),
            Err(error) => {
                eprintln!("failed to serialize review document: {error}");
                return ExitCode::from(1);
            }
        }
    } else {
        print_review_document_summary(&document);
        if let Some(diff) = &semantic_diff {
            print_review_diff_summary(diff);
        }
    }

    if report.has_errors() {
        ExitCode::from(2)
    } else {
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
            "usage: eng run <file.eng> [--profile safe|normal|repro] [--open-report] [--save-artifacts] [--skip-unchanged]"
        );
        return ExitCode::from(2);
    };
    let open_report = args.iter().any(|arg| arg == "--open-report");
    let save_artifacts = open_report || args.iter().any(|arg| arg == "--save-artifacts");
    let skip_unchanged = args.iter().any(|arg| arg == "--skip-unchanged");
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
        &["--open-report", "--save-artifacts", "--skip-unchanged"],
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
            skip_unchanged,
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
                print_run_artifact_path("bytecode", &output.bytecode_path);
                print_run_artifact_path("result data", &output.result_path);
                print_run_artifact_path("review data", &output.review_path);
                print_run_artifact_path("static run graph", &output.static_run_plan_path);
                print_run_artifact_path("run graph", &output.run_plan_path);
                print_run_artifact_path("reproducibility lock", &output.run_lock_path);
                print_run_artifact_path("run log", &output.run_log_path);
                print_run_artifact_path("external process results", &output.process_results_path);
                print_run_artifact_path("cache records", &output.cache_manifest_path);
                print_run_artifact_path("test results", &output.test_results_path);
                print_run_artifact_path("report data", &output.report_spec_path);
                print_run_artifact_path("plot svg", &output.plot_path);
                print_run_artifact_path("plot data", &output.plot_spec_path);
                print_run_artifact_path("plot output list", &output.plot_manifest_path);
                print_run_artifact_path("output list", &output.output_manifest_path);
                print_run_artifact_path("report html", &output.report_path);
            } else {
                println!("run: ok");
                println!("artifacts: in memory");
                print_run_artifact_bytes("result data", output.result_json.len());
                print_run_artifact_bytes("review data", output.review_json.len());
                print_run_artifact_bytes("static run graph", output.static_run_plan_json.len());
                print_run_artifact_bytes("run graph", output.run_plan_json.len());
                print_run_artifact_bytes("reproducibility lock", output.run_lock_json.len());
                print_run_artifact_bytes("run log", output.run_log_json.len());
                print_run_artifact_bytes(
                    "external process results",
                    output.process_results_json.len(),
                );
                print_run_artifact_bytes("cache records", output.cache_manifest_json.len());
                print_run_artifact_bytes("test results", output.test_results_json.len());
                print_run_artifact_bytes("report data", output.report_spec_json.len());
                print_run_artifact_bytes("plot svg", output.plot_svg.len());
                print_run_artifact_bytes("plot data", output.plot_spec_json.len());
                print_run_artifact_bytes("plot output list", output.plot_manifest_json.len());
                print_run_artifact_bytes("output list", output.output_manifest_json.len());
                print_run_artifact_bytes("report html", output.report_html.len());
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

fn print_run_artifact_path(label: &str, path: &Path) {
    print_run_artifact_value(label, &path.display().to_string());
}

fn print_run_artifact_bytes(label: &str, byte_count: usize) {
    print_run_artifact_value(label, &format!("{byte_count} bytes"));
}

fn print_run_artifact_value(label: &str, value: &str) {
    println!("{:<27} {}", format!("{label}:"), value);
}

#[derive(Clone, Debug, Default)]
struct CacheInvalidateOptions {
    manifest_path: PathBuf,
    dry_run: bool,
    all: bool,
    owner_kind: Option<String>,
    owner_name: Option<String>,
    cache_key_hash: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct CacheInvalidateSummary {
    matched: usize,
    deleted: usize,
    missing: usize,
}

fn command_cache(mut args: Vec<String>) -> ExitCode {
    let Some(action) = args.first().cloned() else {
        eprintln!("usage: eng cache invalidate [--manifest <cache_manifest.json>] [--all|--owner-kind <kind>|--owner-name <name>|--cache-key-hash <hash>] [--dry-run]");
        return ExitCode::from(2);
    };
    args.remove(0);
    match action.as_str() {
        "invalidate" => command_cache_invalidate(args),
        other => {
            eprintln!("unknown cache command: {other}");
            eprintln!("usage: eng cache invalidate [--manifest <cache_manifest.json>] [--all|--owner-kind <kind>|--owner-name <name>|--cache-key-hash <hash>] [--dry-run]");
            ExitCode::from(2)
        }
    }
}

fn command_cache_invalidate(args: Vec<String>) -> ExitCode {
    let options = match parse_cache_invalidate_options(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    match invalidate_cache_manifest(&options) {
        Ok(summary) => {
            println!(
                "cache invalidate: matched {}, deleted {}, missing {}{}",
                summary.matched,
                summary.deleted,
                summary.missing,
                if options.dry_run { " (dry-run)" } else { "" }
            );
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}

fn parse_cache_invalidate_options(args: &[String]) -> Result<CacheInvalidateOptions, String> {
    let known_value_flags = [
        "--manifest",
        "--owner-kind",
        "--owner-name",
        "--cache-key-hash",
    ];
    let known_bool_flags = ["--all", "--dry-run"];
    validate_known_flags(args, &known_value_flags, &known_bool_flags)?;

    let options = CacheInvalidateOptions {
        manifest_path: option_value(args, "--manifest")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("build/result/cache_manifest.json")),
        dry_run: args.iter().any(|arg| arg == "--dry-run"),
        all: args.iter().any(|arg| arg == "--all"),
        owner_kind: option_value(args, "--owner-kind"),
        owner_name: option_value(args, "--owner-name"),
        cache_key_hash: option_value(args, "--cache-key-hash"),
    };
    let has_filter = options.owner_kind.is_some()
        || options.owner_name.is_some()
        || options.cache_key_hash.is_some();
    if options.all && has_filter {
        return Err("--all cannot be combined with cache invalidation filters".to_owned());
    }
    if !options.all && !has_filter {
        return Err(
            "cache invalidate requires --all or a filter such as --owner-name or --cache-key-hash"
                .to_owned(),
        );
    }
    Ok(options)
}

fn validate_known_flags(
    args: &[String],
    known_value_flags: &[&str],
    known_bool_flags: &[&str],
) -> Result<(), String> {
    let mut index = 0usize;
    while index < args.len() {
        let arg = &args[index];
        if !arg.starts_with("--") {
            return Err(format!("unexpected positional cache argument `{arg}`"));
        }
        if known_bool_flags.contains(&arg.as_str()) {
            index += 1;
            continue;
        }
        if let Some(flag) = known_value_flags
            .iter()
            .find(|flag| arg.as_str() == **flag || arg.starts_with(&format!("{}=", flag)))
        {
            if arg.as_str() == *flag {
                let Some(value) = args.get(index + 1) else {
                    return Err(format!("missing value for `{arg}`"));
                };
                if value.starts_with("--") {
                    return Err(format!("missing value for `{arg}`"));
                }
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        return Err(format!("unknown cache invalidate flag `{arg}`"));
    }
    Ok(())
}

fn invalidate_cache_manifest(
    options: &CacheInvalidateOptions,
) -> Result<CacheInvalidateSummary, String> {
    let text = std::fs::read_to_string(&options.manifest_path).map_err(|error| {
        format!(
            "failed to read cache manifest `{}`: {error}",
            options.manifest_path.display()
        )
    })?;
    let manifest = serde_json::from_str::<serde_json::Value>(&text).map_err(|error| {
        format!(
            "failed to parse cache manifest `{}`: {error}",
            options.manifest_path.display()
        )
    })?;
    let records = manifest
        .get("cache_records")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "cache manifest is missing `cache_records` array".to_owned())?;
    let mut summary = CacheInvalidateSummary::default();
    for record in records {
        if !cache_record_matches_filter(record, options) {
            continue;
        }
        summary.matched += 1;
        let Some(path) = cache_record_invalidation_path(record) else {
            summary.missing += 1;
            continue;
        };
        if options.dry_run {
            println!("would invalidate: {}", path.display());
            continue;
        }
        if !path.exists() {
            summary.missing += 1;
            continue;
        }
        let safe_path = safe_cache_invalidation_path(&path)?;
        if safe_path.is_dir() {
            std::fs::remove_dir_all(&safe_path)
        } else {
            std::fs::remove_file(&safe_path)
        }
        .map_err(|error| format!("failed to invalidate `{}`: {error}", safe_path.display()))?;
        println!("invalidated: {}", safe_path.display());
        summary.deleted += 1;
    }
    Ok(summary)
}

fn cache_record_matches_filter(
    record: &serde_json::Value,
    options: &CacheInvalidateOptions,
) -> bool {
    if options.all {
        return true;
    }
    if let Some(owner_kind) = &options.owner_kind {
        if json_string(record, "owner_kind") != Some(owner_kind.as_str()) {
            return false;
        }
    }
    if let Some(owner_name) = &options.owner_name {
        if json_string(record, "owner_name") != Some(owner_name.as_str()) {
            return false;
        }
    }
    if let Some(cache_key_hash) = &options.cache_key_hash {
        if json_string(record, "cache_key_hash") != Some(cache_key_hash.as_str()) {
            return false;
        }
    }
    true
}

fn cache_record_invalidation_path(record: &serde_json::Value) -> Option<PathBuf> {
    json_string(record, "resolved_path")
        .or_else(|| json_string(record, "cache_path"))
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
}

fn safe_cache_invalidation_path(path: &Path) -> Result<PathBuf, String> {
    let current_dir =
        env::current_dir().map_err(|error| format!("failed to read current directory: {error}"))?;
    let workspace = current_dir
        .canonicalize()
        .map_err(|error| format!("failed to resolve current directory: {error}"))?;
    let target = path
        .canonicalize()
        .map_err(|error| format!("failed to resolve cache path `{}`: {error}", path.display()))?;
    if target == workspace {
        return Err(format!(
            "refusing to invalidate current directory `{}`",
            target.display()
        ));
    }
    if !target.starts_with(&workspace) {
        return Err(format!(
            "refusing to invalidate cache path outside current directory `{}`",
            target.display()
        ));
    }
    Ok(target)
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

fn first_review_source_path(args: &[String]) -> Option<String> {
    let mut index = 0usize;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--json" {
            index += 1;
            continue;
        }
        if arg == "--output" || arg == "--against" {
            index += 2;
            continue;
        }
        if arg.starts_with("--output=") || arg.starts_with("--against=") {
            index += 1;
            continue;
        }
        if !arg.starts_with('-') {
            return Some(arg.clone());
        }
        index += 1;
    }
    None
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

pub(crate) fn print_diagnostics(report: &eng_compiler::CheckReport) {
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

fn read_review_document(path: &str) -> Result<serde_json::Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read baseline review `{path}`: {error}"))?;
    let value = serde_json::from_str::<serde_json::Value>(&text)
        .map_err(|error| format!("failed to parse baseline review `{path}`: {error}"))?;
    Ok(value.get("review_document").cloned().unwrap_or(value))
}

fn review_semantic_diff(
    previous: &serde_json::Value,
    current: &serde_json::Value,
) -> serde_json::Value {
    let previous_hash = json_string(previous, "semantic_hash");
    let current_hash = json_string(current, "semantic_hash");
    let changed_sections = review_changed_sections(previous, current);
    let section_changes = review_section_changes(previous, current, &changed_sections);
    let status = if previous_hash.is_some()
        && current_hash.is_some()
        && previous_hash == current_hash
        && changed_sections.is_empty()
    {
        "unchanged"
    } else {
        "changed"
    };
    serde_json::json!({
        "format": "eng-review-semantic-diff-preview-1",
        "status": status,
        "semantic_hash_before": previous_hash,
        "semantic_hash_after": current_hash,
        "changed_sections": changed_sections,
        "section_changes": section_changes
    })
}

fn review_section_changes(
    previous: &serde_json::Value,
    current: &serde_json::Value,
    changed_sections: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    changed_sections
        .iter()
        .filter_map(|row| json_string(row, "section"))
        .filter_map(|section| review_array_section_change(section, previous, current))
        .collect()
}

fn review_array_section_change(
    section: &str,
    previous: &serde_json::Value,
    current: &serde_json::Value,
) -> Option<serde_json::Value> {
    let previous_items = previous
        .get(section)
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let current_items = current
        .get(section)
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if previous_items.is_empty() && current_items.is_empty() {
        return None;
    }

    let previous_map = review_item_map(section, previous_items);
    let current_map = review_item_map(section, current_items);
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    for (key, current_item) in &current_map {
        match previous_map.get(key) {
            None => added.push(review_diff_item(key, current_item)),
            Some(previous_item) if *previous_item != *current_item => {
                changed.push(serde_json::json!({
                    "key": key,
                    "before": *previous_item,
                    "after": *current_item
                }));
            }
            _ => {}
        }
    }
    for (key, previous_item) in &previous_map {
        if !current_map.contains_key(key) {
            removed.push(review_diff_item(key, previous_item));
        }
    }

    if added.is_empty() && removed.is_empty() && changed.is_empty() {
        return None;
    }
    Some(serde_json::json!({
        "section": section,
        "added": added,
        "removed": removed,
        "changed": changed
    }))
}

fn review_item_map<'a>(
    section: &str,
    items: &'a [serde_json::Value],
) -> std::collections::BTreeMap<String, &'a serde_json::Value> {
    let mut map = std::collections::BTreeMap::new();
    for (index, item) in items.iter().enumerate() {
        let key = review_item_key(section, item, index);
        map.insert(key, item);
    }
    map
}

fn review_diff_item(key: &str, item: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "key": key,
        "item": item
    })
}

fn review_item_key(section: &str, item: &serde_json::Value, index: usize) -> String {
    let kind = json_string(item, "kind").unwrap_or(section);
    for field in ["name", "binding", "target", "source"] {
        if let Some(value) = json_string(item, field) {
            return format!("{kind}:{field}:{value}");
        }
    }
    if let Some(line) = item.get("line").and_then(serde_json::Value::as_u64) {
        return format!("{kind}:line:{line}");
    }
    if let Some(line) = item.get("source_line").and_then(serde_json::Value::as_u64) {
        return format!("{kind}:source_line:{line}");
    }
    if let Some(expression) = json_string(item, "expression") {
        return format!("{kind}:expression:{expression}");
    }
    if let Some(category) = json_string(item, "category") {
        return format!("{kind}:category:{category}:{index}");
    }
    format!("{section}:{index}")
}

fn review_changed_sections(
    previous: &serde_json::Value,
    current: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let mut sections = Vec::new();
    let previous_hashes = previous
        .get("section_hashes")
        .and_then(serde_json::Value::as_object);
    let current_hashes = current
        .get("section_hashes")
        .and_then(serde_json::Value::as_object);
    let Some(current_hashes) = current_hashes else {
        return sections;
    };
    for (section, current_hash) in current_hashes {
        let previous_hash = previous_hashes.and_then(|hashes| hashes.get(section));
        if previous_hash != Some(current_hash) {
            sections.push(serde_json::json!({
                "section": section,
                "before": previous_hash.cloned().unwrap_or(serde_json::Value::Null),
                "after": current_hash
            }));
        }
    }
    sections
}

fn write_static_review_outputs(
    output_dir: &str,
    document: &serde_json::Value,
    semantic_diff: Option<&serde_json::Value>,
) -> Result<(), String> {
    let output_dir = Path::new(output_dir);
    std::fs::create_dir_all(output_dir).map_err(|error| {
        format!(
            "failed to create review output directory `{}`: {error}",
            output_dir.display()
        )
    })?;
    let review_path = output_dir.join("static_review.json");
    let review_text = serde_json::to_string_pretty(document)
        .map_err(|error| format!("failed to serialize static review: {error}"))?;
    std::fs::write(&review_path, review_text).map_err(|error| {
        format!(
            "failed to write static review `{}`: {error}",
            review_path.display()
        )
    })?;
    if let Some(diff) = semantic_diff {
        let diff_path = output_dir.join("semantic_diff.json");
        let diff_text = serde_json::to_string_pretty(diff)
            .map_err(|error| format!("failed to serialize semantic diff: {error}"))?;
        std::fs::write(&diff_path, diff_text).map_err(|error| {
            format!(
                "failed to write semantic diff `{}`: {error}",
                diff_path.display()
            )
        })?;
    }
    Ok(())
}

fn print_review_document_summary(document: &serde_json::Value) {
    let status = json_string(document, "status").unwrap_or("-");
    let signature = json_string(document, "workflow_signature").unwrap_or("-");
    println!("review: {status}");
    println!("workflow: {signature}");

    let contract = document
        .get("root_contract")
        .unwrap_or(&serde_json::Value::Null);
    println!(
        "modules: {}  inputs: {}  symbols: {}  calculations: {}  validations: {}",
        json_usize(contract, "workflow_module_count").unwrap_or(0),
        json_usize(contract, "input_count").unwrap_or(0),
        json_usize(contract, "symbol_count").unwrap_or(0),
        json_usize(contract, "calculation_count").unwrap_or(0),
        json_usize(contract, "validation_count").unwrap_or(0)
    );
    println!(
        "schemas: {}  units: {}  time axes: {}  report outputs: {}",
        json_usize(contract, "schema_count").unwrap_or(0),
        json_usize(contract, "unit_quantity_count").unwrap_or(0),
        json_usize(contract, "time_axis_count").unwrap_or(0),
        json_usize(contract, "report_output_count").unwrap_or(0)
    );
    println!(
        "side effects: {}  external boundaries: {}  fallbacks: {}  risks: {}",
        json_usize(contract, "side_effect_count").unwrap_or(0),
        json_usize(contract, "external_boundary_count").unwrap_or(0),
        json_usize(contract, "fallback_count").unwrap_or(0),
        json_usize(contract, "risk_count").unwrap_or(0)
    );

    print_review_rows(document, "workflow_modules", "workflow modules");
    print_review_rows(document, "external_boundaries", "external boundaries");
    print_review_rows(document, "fallbacks", "fallbacks");
    print_review_rows(document, "risks", "risks");
}

fn print_review_diff_summary(diff: &serde_json::Value) {
    let status = json_string(diff, "status").unwrap_or("-");
    let before = json_string(diff, "semantic_hash_before").unwrap_or("-");
    let after = json_string(diff, "semantic_hash_after").unwrap_or("-");
    println!("semantic diff: {status}");
    println!("semantic hash: {before} -> {after}");
    let changed = diff
        .get("changed_sections")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if changed.is_empty() {
        return;
    }
    println!("changed sections:");
    for row in changed.iter().take(12) {
        let section = json_string(row, "section").unwrap_or("-");
        println!("  {section}");
    }
    if changed.len() > 12 {
        println!("  ... {} more", changed.len() - 12);
    }
}

fn print_review_rows(document: &serde_json::Value, key: &str, label: &str) {
    let Some(rows) = document.get(key).and_then(serde_json::Value::as_array) else {
        return;
    };
    if rows.is_empty() {
        return;
    }
    println!("{label}:");
    for row in rows.iter().take(8) {
        let line = json_usize(row, "line").unwrap_or(0);
        let kind = json_string(row, "kind")
            .or_else(|| json_string(row, "category"))
            .unwrap_or("-");
        let summary = json_string(row, "summary")
            .or_else(|| json_string(row, "reason"))
            .or_else(|| json_string(row, "target"))
            .or_else(|| json_string(row, "name"))
            .unwrap_or("-");
        println!("  L{line}: {kind}: {summary}");
    }
    if rows.len() > 8 {
        println!("  ... {} more", rows.len() - 8);
    }
}

fn json_string<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(serde_json::Value::as_str)
}

fn json_usize(value: &serde_json::Value, key: &str) -> Option<usize> {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_semantic_diff_reports_changed_sections() {
        let previous = serde_json::json!({
            "semantic_hash": "before",
            "section_hashes": {
                "inputs": "same",
                "calculations": "old"
            },
            "calculations": [
                {
                    "kind": "binding",
                    "name": "Q_total",
                    "expression": "Q + 1 kW",
                    "quantity_kind": "HeatRate",
                    "line": 3
                }
            ]
        });
        let current = serde_json::json!({
            "semantic_hash": "after",
            "section_hashes": {
                "inputs": "same",
                "calculations": "new"
            },
            "calculations": [
                {
                    "kind": "binding",
                    "name": "Q_total",
                    "expression": "Q + 2 kW",
                    "quantity_kind": "HeatRate",
                    "line": 3
                }
            ]
        });

        let diff = review_semantic_diff(&previous, &current);

        assert_eq!(diff["status"], "changed");
        assert_eq!(diff["changed_sections"][0]["section"], "calculations");
        assert_eq!(diff["changed_sections"][0]["before"], "old");
        assert_eq!(diff["changed_sections"][0]["after"], "new");
        assert_eq!(diff["section_changes"].as_array().map(Vec::len), Some(1));
        assert_eq!(diff["section_changes"][0]["section"], "calculations");
        assert_eq!(
            diff["section_changes"][0]["changed"][0]["key"],
            "binding:name:Q_total"
        );
        assert_eq!(
            diff["section_changes"][0]["changed"][0]["before"]["expression"],
            "Q + 1 kW"
        );
        assert_eq!(
            diff["section_changes"][0]["changed"][0]["after"]["expression"],
            "Q + 2 kW"
        );
    }

    #[test]
    fn review_semantic_diff_compares_workflow_modules() {
        let previous = serde_json::json!({
            "semantic_hash": "before",
            "section_hashes": {
                "workflow_modules": "old"
            },
            "workflow_modules": [
                {
                    "kind": "native_module",
                    "name": "eng.net",
                    "status": "planned",
                    "backing": "none",
                    "purpose": "Network boundary records",
                    "artifacts": [],
                    "symbols": [],
                    "artifact_count": 0,
                    "symbol_count": 0
                }
            ]
        });
        let current = serde_json::json!({
            "semantic_hash": "after",
            "section_hashes": {
                "workflow_modules": "new"
            },
            "workflow_modules": [
                {
                    "kind": "native_module",
                    "name": "eng.net",
                    "status": "native_preview",
                    "backing": "compiler_runtime_builtin",
                    "purpose": "Network boundary records",
                    "artifacts": ["review.external_boundaries"],
                    "symbols": [],
                    "artifact_count": 1,
                    "symbol_count": 0
                }
            ]
        });

        let diff = review_semantic_diff(&previous, &current);

        assert_eq!(diff["status"], "changed");
        assert_eq!(diff["changed_sections"][0]["section"], "workflow_modules");
        assert_eq!(diff["section_changes"][0]["section"], "workflow_modules");
        assert_eq!(
            diff["section_changes"][0]["changed"][0]["key"],
            "native_module:name:eng.net"
        );
        assert_eq!(
            diff["section_changes"][0]["changed"][0]["before"]["status"],
            "planned"
        );
        assert_eq!(
            diff["section_changes"][0]["changed"][0]["after"]["status"],
            "native_preview"
        );
    }

    #[test]
    fn review_semantic_diff_reports_unchanged_document() {
        let previous = serde_json::json!({
            "semantic_hash": "same",
            "section_hashes": {
                "inputs": "a"
            }
        });
        let current = previous.clone();

        let diff = review_semantic_diff(&previous, &current);

        assert_eq!(diff["status"], "unchanged");
        assert_eq!(diff["changed_sections"].as_array().map(Vec::len), Some(0));
        assert_eq!(diff["section_changes"].as_array().map(Vec::len), Some(0));
    }

    #[test]
    fn first_review_source_path_skips_review_options() {
        let args = vec![
            "--against".to_string(),
            "build/review/static_review.json".to_string(),
            "--json".to_string(),
            "--output=build/review-next".to_string(),
            "examples/official/01_csv_plot/main.eng".to_string(),
        ];

        assert_eq!(
            first_review_source_path(&args),
            Some("examples/official/01_csv_plot/main.eng".to_string())
        );
    }

    #[test]
    fn cache_invalidate_removes_matching_manifest_path() {
        let root = env::current_dir()
            .expect("current dir")
            .join("build")
            .join(format!("cli-cache-invalidate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let target = root.join("cache").join("entry");
        std::fs::create_dir_all(&target).expect("target dir");
        std::fs::write(target.join("payload.txt"), "cached").expect("payload");
        let manifest_path = root.join("cache_manifest.json");
        std::fs::write(
            &manifest_path,
            format!(
                "{{\n  \"cache_records\": [{{\n    \"owner_kind\": \"model\",\n    \"owner_name\": \"reg_model\",\n    \"cache_key_hash\": \"abc123\",\n    \"resolved_path\": \"{}\"\n  }}]\n}}\n",
                target.display().to_string().replace('\\', "\\\\")
            ),
        )
        .expect("manifest");

        let summary = invalidate_cache_manifest(&CacheInvalidateOptions {
            manifest_path,
            owner_name: Some("reg_model".to_owned()),
            ..CacheInvalidateOptions::default()
        })
        .expect("invalidate");

        assert_eq!(
            summary,
            CacheInvalidateSummary {
                matched: 1,
                deleted: 1,
                missing: 0
            }
        );
        assert!(!target.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn cache_invalidate_dry_run_keeps_matching_path() {
        let root = env::current_dir()
            .expect("current dir")
            .join("build")
            .join(format!(
                "cli-cache-invalidate-dry-run-{}",
                std::process::id()
            ));
        let _ = std::fs::remove_dir_all(&root);
        let target = root.join("cache").join("entry");
        std::fs::create_dir_all(&target).expect("target dir");
        let manifest_path = root.join("cache_manifest.json");
        std::fs::write(
            &manifest_path,
            format!(
                "{{\n  \"cache_records\": [{{\n    \"owner_kind\": \"case\",\n    \"owner_name\": \"case_001\",\n    \"cache_key_hash\": \"def456\",\n    \"resolved_path\": \"{}\"\n  }}]\n}}\n",
                target.display().to_string().replace('\\', "\\\\")
            ),
        )
        .expect("manifest");

        let summary = invalidate_cache_manifest(&CacheInvalidateOptions {
            manifest_path,
            dry_run: true,
            all: true,
            ..CacheInvalidateOptions::default()
        })
        .expect("dry run");

        assert_eq!(
            summary,
            CacheInvalidateSummary {
                matched: 1,
                deleted: 0,
                missing: 0
            }
        );
        assert!(target.exists());
        let _ = std::fs::remove_dir_all(&root);
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
  eng review <file.eng> [--json] [--output <dir>] [--against <review.json>]
  eng fmt <file.eng> [--check|--write]
  eng ide-check <file.eng>
  eng jit-plan <file.eng>
  eng jit-bench <file.eng> [--iterations N] [--<arg> <value>...]
  eng run <file.eng> [--profile safe|normal|repro] [--open-report] [--save-artifacts] [--skip-unchanged] [--<arg> <value>...]
  eng cache invalidate [--manifest build/result/cache_manifest.json] [--all|--owner-kind <kind>|--owner-name <name>|--cache-key-hash <hash>] [--dry-run]
  eng build <file.eng> [--standalone] [--profile repro]
  eng view <result.engres>
  eng test <project_or_examples>

The supported core path intentionally stays free of Python dependencies.
"#,
        version = env!("CARGO_PKG_VERSION")
    );
}
