use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use eng_compiler::{
    build_bytecode, check_file, check_source, parse_bytecode, review_json, ArgOverride,
    CheckOptions, CheckReport,
};
use serde_json::{json, Value};

mod artifact;
mod runtime_data;
pub mod solver;
mod vm;

use artifact::{ArtifactRecord, ArtifactValidation, ExternalBoundaryRecord, SourceRecord};
use runtime_data::{
    materialize_runtime_data, RuntimeCaseManifest, RuntimeCaseMetric, RuntimeCaseProcessStatus,
    RuntimeComponentResidualEvaluation, RuntimeData, RuntimeNumericUncertaintyPayload,
    RuntimeNumericValue, RuntimeStatisticValue, RuntimeTimeSeries, RuntimeValues,
};
pub use vm::{execute_bytecode, VmExecution, VmObject, VmObjectKind};

pub const RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ExecutionProfile {
    Safe,
    #[default]
    Normal,
    Repro,
}

impl ExecutionProfile {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "safe" => Some(Self::Safe),
            "normal" => Some(Self::Normal),
            "repro" => Some(Self::Repro),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Normal => "normal",
            Self::Repro => "repro",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RunOptions {
    pub open_report: bool,
    pub save_artifacts: bool,
    pub args: Vec<ArgOverride>,
    pub profile: ExecutionProfile,
}

#[derive(Clone, Debug, Default)]
pub struct BuildOptions {
    pub args: Vec<ArgOverride>,
}

#[derive(Clone, Debug)]
pub struct RunOutput {
    pub bytecode_path: PathBuf,
    pub result_path: PathBuf,
    pub review_path: PathBuf,
    pub report_path: PathBuf,
    pub report_spec_path: PathBuf,
    pub plot_path: PathBuf,
    pub plot_spec_path: PathBuf,
    pub plot_manifest_path: PathBuf,
    pub output_manifest_path: PathBuf,
    pub run_log_path: PathBuf,
    pub process_results_path: PathBuf,
    pub test_results_path: PathBuf,
    pub csv_export_paths: Vec<PathBuf>,
    pub write_output_paths: Vec<PathBuf>,
    pub file_operation_paths: Vec<PathBuf>,
    pub artifacts_saved: bool,
    pub stdout: String,
    pub bytecode: String,
    pub result_json: String,
    pub review_json: String,
    pub report_html: String,
    pub report_spec_json: String,
    pub plot_svg: String,
    pub plot_spec_json: String,
    pub plot_manifest_json: String,
    pub output_manifest_json: String,
    pub run_log_json: String,
    pub process_results_json: String,
    pub test_results_json: String,
}

#[derive(Clone, Debug)]
pub struct BuildOutput {
    pub bundle_path: PathBuf,
    pub executable_path: PathBuf,
    pub runner_path: PathBuf,
    pub package_path: PathBuf,
    pub lock_path: PathBuf,
    pub bytecode_path: PathBuf,
    pub source_path: PathBuf,
    pub review_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct DoctorCheck {
    pub name: &'static str,
    pub ok: bool,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    pub fn ready(&self) -> bool {
        self.checks.iter().all(|check| check.ok)
    }
}

#[derive(Debug)]
pub enum RuntimeError {
    Io(std::io::Error),
    Compile(Box<CheckReport>),
    Bytecode(eng_compiler::BytecodeParseError),
    Vm(vm::VmError),
    TestsFailed(String),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Compile(report) => write!(
                formatter,
                "compile failed with {} error(s)",
                report.diagnostic_count(eng_compiler::Severity::Error)
            ),
            Self::Bytecode(error) => write!(formatter, "bytecode decode failed: {error}"),
            Self::Vm(error) => write!(formatter, "VM execution failed: {error}"),
            Self::TestsFailed(message) => write!(formatter, "{message}"),
        }
    }
}

impl Error for RuntimeError {}

impl From<std::io::Error> for RuntimeError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<eng_compiler::BytecodeParseError> for RuntimeError {
    fn from(value: eng_compiler::BytecodeParseError) -> Self {
        Self::Bytecode(value)
    }
}

impl From<vm::VmError> for RuntimeError {
    fn from(value: vm::VmError) -> Self {
        Self::Vm(value)
    }
}

#[derive(Clone, Debug)]
struct ProfileDiagnostic {
    severity: &'static str,
    code: &'static str,
    message: String,
    line: usize,
}

struct ProfileContext<'a> {
    profile: &'a ExecutionProfile,
    diagnostics: &'a [ProfileDiagnostic],
}

struct ResultArtifactHashes<'a> {
    bytecode: &'a str,
    plot_spec: &'a str,
    report_spec: &'a str,
}

fn profile_diagnostics(profile: &ExecutionProfile, report: &CheckReport) -> Vec<ProfileDiagnostic> {
    let mut diagnostics = Vec::new();
    match profile {
        ExecutionProfile::Safe => {
            for export in &report.semantic_program.csv_exports {
                diagnostics.push(ProfileDiagnostic {
                    severity: "error",
                    code: "E-PROFILE-SAFE-EXPORT",
                    message: "safe profile rejects explicit CSV export side effects".to_owned(),
                    line: export.line,
                });
            }
            for write in &report.semantic_program.writes {
                diagnostics.push(ProfileDiagnostic {
                    severity: "error",
                    code: "E-PROFILE-SAFE-WRITE",
                    message: format!(
                        "safe profile rejects explicit write {} output side effects",
                        write.format
                    ),
                    line: write.line,
                });
            }
            for operation in &report.semantic_program.file_operations {
                diagnostics.push(ProfileDiagnostic {
                    severity: "error",
                    code: "E-PROFILE-SAFE-FS",
                    message: format!(
                        "safe profile rejects explicit `{}` file operation side effects",
                        operation.operation
                    ),
                    line: operation.line,
                });
            }
            for process in &report.semantic_program.process_runs {
                diagnostics.push(ProfileDiagnostic {
                    severity: "error",
                    code: "E-PROFILE-SAFE-PROCESS",
                    message: format!(
                        "safe profile rejects external process `{}`",
                        process.command
                    ),
                    line: process.line,
                });
            }
        }
        ExecutionProfile::Repro => {
            for dependency in &report.semantic_program.environment_dependencies {
                diagnostics.push(ProfileDiagnostic {
                    severity: "warning",
                    code: "W-PROFILE-REPRO-ENV",
                    message: format!(
                        "repro profile records environment-dependent `{}` for review",
                        dependency.expression
                    ),
                    line: dependency.line,
                });
            }
            for process in &report.semantic_program.process_runs {
                diagnostics.push(ProfileDiagnostic {
                    severity: "warning",
                    code: "W-PROFILE-REPRO-PROCESS",
                    message: format!(
                        "repro profile records external process `{}` with command, cwd, args, exit code, stdout, and stderr",
                        process.command
                    ),
                    line: process.line,
                });
            }
            for operation in &report.semantic_program.file_operations {
                if matches!(operation.operation.as_str(), "move" | "delete") {
                    diagnostics.push(ProfileDiagnostic {
                        severity: "warning",
                        code: "W-PROFILE-REPRO-FS",
                        message: format!(
                            "repro profile records `{}` mutation in the side-effect manifest",
                            operation.operation
                        ),
                        line: operation.line,
                    });
                }
            }
        }
        ExecutionProfile::Normal => {}
    }
    diagnostics
}

fn ensure_profile_allowed(
    profile: &ExecutionProfile,
    diagnostics: &[ProfileDiagnostic],
) -> Result<(), RuntimeError> {
    if let Some(diagnostic) = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity == "error")
    {
        return Err(invalid_input(&format!(
            "profile `{}` rejected line {} ({}): {}",
            profile.as_str(),
            diagnostic.line,
            diagnostic.code,
            diagnostic.message
        )));
    }
    Ok(())
}

pub fn doctor(repo_root: &Path) -> DoctorReport {
    let mut checks = Vec::new();
    checks.push(DoctorCheck {
        name: "Runtime",
        ok: true,
        detail: format!("EngLang runtime {RUNTIME_VERSION}"),
    });
    checks.push(file_check(
        "Standard library",
        &repo_root.join("stdlib").join("prelude.eng"),
    ));
    checks.push(file_check(
        "Unit registry",
        &repo_root.join("stdlib").join("units.eng"),
    ));
    checks.push(module_registry_check(repo_root));
    checks.push(DoctorCheck {
        name: "Plot renderer",
        ok: !eng_report::render_svg("doctor").is_empty(),
        detail: "SVG renderer available".to_owned(),
    });
    checks.push(DoctorCheck {
        name: "Report generator",
        ok: true,
        detail: "HTML report generator available".to_owned(),
    });
    checks.push(write_permission_check(repo_root));
    checks.push(file_check(
        "Example files",
        &repo_root
            .join("examples")
            .join("official")
            .join("01_csv_plot")
            .join("main.eng"),
    ));

    DoctorReport { checks }
}

pub fn run_file(
    path: &Path,
    build_root: &Path,
    options: &RunOptions,
) -> Result<RunOutput, RuntimeError> {
    let source = fs::read_to_string(path)?;
    run_source(path, &source, build_root, options)
}

pub fn run_source(
    path: &Path,
    source: &str,
    build_root: &Path,
    options: &RunOptions,
) -> Result<RunOutput, RuntimeError> {
    let check_report = check_source(
        path,
        source,
        &CheckOptions {
            review: true,
            args: options.args.clone(),
            require_args: true,
        },
    );
    if check_report.has_errors() {
        return Err(RuntimeError::Compile(Box::new(check_report)));
    }
    let profile_diagnostics = profile_diagnostics(&options.profile, &check_report);
    ensure_profile_allowed(&options.profile, &profile_diagnostics)?;
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("main");
    let result_dir = build_root.join("result");
    let plots_dir = result_dir.join("plots");
    let bytecode_path = build_root.join(format!("{stem}.engbc"));
    let result_path = result_dir.join("result.engres");
    let review_path = result_dir.join("review.json");
    let plot_path = plots_dir.join("timeseries.svg");
    let plot_spec_path = plots_dir.join("plot_spec.json");
    let plot_manifest_path = plots_dir.join("plot_manifest.json");
    let output_manifest_path = result_dir.join("output_manifest.json");
    let run_log_path = result_dir.join("run_log.json");
    let process_results_path = result_dir.join("process_results.json");
    let test_results_path = result_dir.join("test_results.json");
    let report_spec_path = result_dir.join("report_spec.json");
    let report_path = result_dir.join("report.html");

    let bytecode = build_bytecode(&check_report, source);
    let bytecode_hash = hash_text(&bytecode);
    let bytecode_program = parse_bytecode(&bytecode)?;
    let mut execution = execute_bytecode(&bytecode_program)?;
    let runtime_data = materialize_runtime_data(&check_report, source);
    apply_runtime_lengths(&mut execution, &runtime_data);
    let stdout = render_stdout(&check_report, &runtime_data);
    let process_results = execute_process_runs(&check_report)?;
    let external_boundary_records = external_boundary_records_for_processes(&process_results);
    let run_log_json = run_log_json(
        &check_report,
        &runtime_data,
        &options.profile,
        &profile_diagnostics,
        &external_boundary_records,
    );
    let process_results_json =
        process_results_json(&check_report, &process_results, &options.profile);
    let csv_export_artifacts = write_csv_exports(&check_report, &runtime_data, &result_dir)?;
    let write_artifacts = write_outputs(&check_report, &runtime_data, &result_dir)?;
    let file_operation_artifacts = apply_file_operations(&check_report, &result_dir)?;
    let test_results = execute_tests(&check_report, &runtime_data, &result_dir)?;
    let test_results_json = test_results_json(&check_report, &test_results);
    let db_manifest_records = db_manifest_records(&process_results);
    let csv_export_paths = csv_export_artifacts
        .iter()
        .map(|artifact| artifact.absolute_path.clone())
        .collect::<Vec<_>>();
    let write_output_paths = write_artifacts
        .iter()
        .map(|artifact| artifact.absolute_path.clone())
        .collect::<Vec<_>>();
    let file_operation_paths = file_operation_artifacts
        .iter()
        .map(|artifact| artifact.absolute_path.clone())
        .collect::<Vec<_>>();
    let mut plot_spec = eng_report::plot_spec_from_report(&check_report);
    runtime_data.apply_plot_spec(&check_report, &mut plot_spec);
    let plot_spec_json = eng_report::plot_spec_json(&plot_spec);
    let plot_spec_hash = hash_text(&plot_spec_json);
    let plot_svg = eng_report::render_svg_from_spec(&plot_spec);
    let plot_svg_hash = hash_text(&plot_svg);
    let plot_manifest_json = eng_report::plot_manifest_json(
        &plot_spec,
        "timeseries.svg",
        &plot_spec_hash,
        &plot_svg_hash,
    );
    let plot_manifest_hash = hash_text(&plot_manifest_json);
    let mut report_spec = eng_report::report_spec_from_report(
        &check_report,
        "plots/plot_manifest.json",
        &plot_manifest_hash,
    );
    report_spec.computed_statistics = runtime_data.report_computed_statistics();
    report_spec.computed_integrations = runtime_data.report_computed_integrations();
    report_spec.computed_metrics = runtime_data.report_computed_metrics();
    report_spec.validations = runtime_data.report_validations();
    report_spec.time_axes = runtime_data.report_time_axes();
    report_spec.time_alignments = runtime_data.report_time_alignments();
    report_spec.uncertainty = runtime_data.report_uncertainty();
    report_spec.ml = runtime_data.report_ml();
    report_spec.policy_results = runtime_data.report_policy_results();
    runtime_data.apply_system_solutions(&mut report_spec);
    runtime_data.apply_component_solutions(&mut report_spec);
    let report_spec_json = eng_report::report_spec_json(&report_spec);
    let report_spec_hash = hash_text(&report_spec_json);
    let mut output_artifacts = Vec::new();
    output_artifacts.extend(process_expected_output_artifacts(&process_results));
    output_artifacts.extend(csv_export_artifacts);
    output_artifacts.extend(write_artifacts);
    output_artifacts.extend(file_operation_artifacts);
    let review_json = runtime_review_json(
        &review_json(&check_report),
        &runtime_data,
        &process_results,
        &output_artifacts,
    );
    let report_html =
        eng_report::render_html_with_spec(&check_report, "plots/timeseries.svg", &report_spec);
    let result_json = result_json(
        path,
        &check_report,
        &execution,
        &runtime_data,
        &process_results,
        &ResultArtifactHashes {
            bytecode: &bytecode_hash,
            plot_spec: &plot_spec_hash,
            report_spec: &report_spec_hash,
        },
        &ProfileContext {
            profile: &options.profile,
            diagnostics: &profile_diagnostics,
        },
    );

    let artifacts_saved = options.save_artifacts || options.open_report;
    if artifacts_saved {
        fs::create_dir_all(&plots_dir)?;
        fs::write(&bytecode_path, &bytecode)?;
        output_artifacts.push(output_artifact(
            "bytecode",
            path_for_manifest(&bytecode_path),
            &bytecode,
            bytecode_path.clone(),
        ));
        fs::write(&review_path, &review_json)?;
        output_artifacts.push(output_artifact(
            "review",
            "review.json".to_owned(),
            &review_json,
            review_path.clone(),
        ));
        fs::write(&run_log_path, &run_log_json)?;
        output_artifacts.push(output_artifact(
            "run_log",
            "run_log.json".to_owned(),
            &run_log_json,
            run_log_path.clone(),
        ));
        fs::write(&process_results_path, &process_results_json)?;
        output_artifacts.push(output_artifact(
            "process_results",
            "process_results.json".to_owned(),
            &process_results_json,
            process_results_path.clone(),
        ));
        fs::write(&test_results_path, &test_results_json)?;
        output_artifacts.push(output_artifact(
            "test_results",
            "test_results.json".to_owned(),
            &test_results_json,
            test_results_path.clone(),
        ));
        fs::write(&plot_spec_path, &plot_spec_json)?;
        output_artifacts.push(output_artifact(
            "plot_spec",
            "plots/plot_spec.json".to_owned(),
            &plot_spec_json,
            plot_spec_path.clone(),
        ));
        fs::write(&plot_path, &plot_svg)?;
        output_artifacts.push(output_artifact(
            "plot_svg",
            "plots/timeseries.svg".to_owned(),
            &plot_svg,
            plot_path.clone(),
        ));
        fs::write(&plot_manifest_path, &plot_manifest_json)?;
        output_artifacts.push(output_artifact(
            "plot_manifest",
            "plots/plot_manifest.json".to_owned(),
            &plot_manifest_json,
            plot_manifest_path.clone(),
        ));
        fs::write(&report_spec_path, &report_spec_json)?;
        output_artifacts.push(output_artifact(
            "report_spec",
            "report_spec.json".to_owned(),
            &report_spec_json,
            report_spec_path.clone(),
        ));
        fs::write(&report_path, &report_html)?;
        output_artifacts.push(output_artifact(
            "report_html",
            "report.html".to_owned(),
            &report_html,
            report_path.clone(),
        ));
        fs::write(&result_path, &result_json)?;
        output_artifacts.push(output_artifact(
            "result",
            "result.engres".to_owned(),
            &result_json,
            result_path.clone(),
        ));
    }
    let output_manifest_json = output_manifest_json(
        path,
        &output_artifacts,
        &options.profile,
        &profile_diagnostics,
        &ArtifactRegistryContext {
            report: &check_report,
            runtime_data: &runtime_data,
            process_results: &process_results,
            db_manifest_records: &db_manifest_records,
            test_results: &test_results,
        },
    );
    if artifacts_saved || !output_artifacts.is_empty() {
        fs::create_dir_all(&result_dir)?;
        fs::write(&output_manifest_path, &output_manifest_json)?;
    }

    if options.open_report {
        open_path(&report_path);
    }

    if test_results
        .iter()
        .any(|test| test.status.as_str() == "failed")
    {
        let failed_count = test_results
            .iter()
            .filter(|test| test.status.as_str() == "failed")
            .count();
        return Err(RuntimeError::TestsFailed(format!(
            "{failed_count} test block(s) failed; inspect test_results.json"
        )));
    }

    Ok(RunOutput {
        bytecode_path,
        result_path,
        review_path,
        report_path,
        report_spec_path,
        plot_path,
        plot_spec_path,
        plot_manifest_path,
        output_manifest_path,
        run_log_path,
        process_results_path,
        test_results_path,
        csv_export_paths,
        write_output_paths,
        file_operation_paths,
        artifacts_saved,
        stdout,
        bytecode,
        result_json,
        review_json,
        report_html,
        report_spec_json,
        plot_svg,
        plot_spec_json,
        plot_manifest_json,
        output_manifest_json,
        run_log_json,
        process_results_json,
        test_results_json,
    })
}

pub fn build_standalone(
    path: &Path,
    dist_root: &Path,
    options: &BuildOptions,
) -> Result<BuildOutput, RuntimeError> {
    let source = fs::read_to_string(path)?;
    let check_report = check_file(
        path,
        &CheckOptions {
            review: true,
            args: options.args.clone(),
            require_args: false,
        },
    )?;
    if check_report.has_errors() {
        return Err(RuntimeError::Compile(Box::new(check_report)));
    }
    let bytecode = build_bytecode(&check_report, &source);
    let bytecode_hash = hash_text(&bytecode);

    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("model");
    fs::create_dir_all(dist_root)?;
    let bundle_path = dist_root.join(format!("{stem}-standalone"));
    if bundle_path.exists() {
        fs::remove_dir_all(&bundle_path)?;
    }
    fs::create_dir_all(&bundle_path)?;

    let source_dir = bundle_path.join("source");
    fs::create_dir_all(&source_dir)?;
    let source_file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("main.eng");
    let bundled_source_path = source_dir.join(source_file_name);
    fs::write(&bundled_source_path, &source)?;

    let mut bundled_dependencies = Vec::new();
    for promotion in &check_report.semantic_program.csv_promotions {
        let Some(destination) =
            bundled_dependency_path(&source_dir, &bundle_path, &promotion.source_value)
        else {
            return Err(RuntimeError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "CSV dependency `{}` cannot be bundled because it escapes the standalone bundle",
                    promotion.source_value
                ),
            )));
        };
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let dependency_source = Path::new(&promotion.resolved_path);
        fs::copy(dependency_source, &destination)?;
        let relative_path = path_for_manifest(
            destination
                .strip_prefix(&bundle_path)
                .unwrap_or(destination.as_path()),
        );
        let dependency_hash = hash_bytes(&fs::read(dependency_source)?);
        bundled_dependencies.push((relative_path, dependency_hash));
    }
    bundled_dependencies.sort_by(|left, right| left.0.cmp(&right.0));

    let executable_path = bundle_path.join("eng.exe");
    fs::copy(env::current_exe()?, &executable_path)?;

    let runner_path = bundle_path.join("run.bat");
    fs::write(&runner_path, standalone_runner_script(source_file_name))?;
    let args_help_path = bundle_path.join("ARGS_HELP.txt");
    fs::write(args_help_path, args_help_text(&check_report))?;

    let bytecode_path = bundle_path.join(format!("{stem}.engbc"));
    let package_path = bundle_path.join(format!("{stem}.engpkg"));
    let lock_path = bundle_path.join(format!("{stem}.lock"));
    let review_path = bundle_path.join(format!("{stem}.review.html"));

    fs::write(&bytecode_path, &bytecode)?;
    fs::write(
        &package_path,
        format!(
            "format = engpkg-stable-1\npackage_format_version = 1\nruntime_abi = eng-runtime-cli-v1\nprofile = repro\nrunner = run.bat\nengine = eng.exe\nsource_root = source\nartifact_root = build/result\nsource = {}\nbytecode = {}\nsource_hash = {}\nbytecode_hash = {}\nworkflow = {}\nargs_schema = {}\nargs_field_count = {}\nargs_help = ARGS_HELP.txt\ndependency_count = {}\ndependencies = {}\ndependency_hashes = {}\n",
            path_for_manifest(&Path::new("source").join(source_file_name)),
            path_for_manifest(
                bytecode_path
                    .file_name()
                    .map(PathBuf::from)
                    .as_deref()
                    .unwrap_or_else(|| Path::new("model.engbc"))
            ),
            check_report.source_hash,
            bytecode_hash,
            check_report.semantic_program.workflow.signature(),
            check_report.semantic_program.workflow.arg_type.as_deref().unwrap_or("Args"),
            args_field_count(&check_report),
            bundled_dependencies.len(),
            dependency_paths(&bundled_dependencies),
            dependency_hashes(&bundled_dependencies)
        ),
    )?;
    fs::write(
        &lock_path,
        format!(
            "runtime_version = {RUNTIME_VERSION}\ncompiler_version = {}\npackage_format_version = 1\nruntime_abi = eng-runtime-cli-v1\nbytecode_version = {}\nresult_format_version = 1\nreport_schema_version = {}\nplot_spec_version = {}\nprofile = repro\nsource_hash = {}\nbytecode_hash = {}\nworkflow = {}\ndependency_count = {}\ndependency_hashes = {}\n",
            eng_compiler::COMPILER_VERSION,
            eng_compiler::BYTECODE_VERSION,
            eng_report::REPORT_SPEC_VERSION,
            eng_report::PLOT_SPEC_VERSION,
            check_report.source_hash,
            bytecode_hash,
            check_report.semantic_program.workflow.signature(),
            bundled_dependencies.len(),
            dependency_hashes(&bundled_dependencies)
        ),
    )?;
    fs::write(
        &review_path,
        eng_report::render_html(&check_report, "plots/timeseries.svg"),
    )?;

    Ok(BuildOutput {
        bundle_path,
        executable_path,
        runner_path,
        package_path,
        lock_path,
        bytecode_path,
        source_path: bundled_source_path,
        review_path,
    })
}

pub fn create_project(path: &Path) -> std::io::Result<()> {
    fs::create_dir_all(path.join("data"))?;
    fs::write(
        path.join("main.eng"),
        r#"schema SensorData {
    time: DateTime index
    T_supply: AbsoluteTemperature [degC]
    T_return: AbsoluteTemperature [degC]
    m_dot: MassFlowRate [kg/s]
}

sensor = promote csv "data/sensor.csv" as SensorData
cp = 4180 J/kg/K
Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
E_coil = integrate(Q_coil, over=Time)

report {
    summarize Q_coil by [mean, max, p95]
    show E_coil
    plot Q_coil over Time
}
"#,
    )?;
    fs::write(
        path.join("data").join("sensor.csv"),
        "time,T_supply,T_return,m_dot\n2026-01-01T00:00:00Z,7.0,12.0,0.21\n",
    )?;
    Ok(())
}

fn file_check(name: &'static str, path: &Path) -> DoctorCheck {
    DoctorCheck {
        name,
        ok: path.exists(),
        detail: path.display().to_string(),
    }
}

fn module_registry_check(repo_root: &Path) -> DoctorCheck {
    let path = repo_root.join("stdlib").join("eng").join("modules.toml");
    match eng_compiler::load_module_registry(&path) {
        Ok(registry) => DoctorCheck {
            name: "Module registry",
            ok: true,
            detail: format!("{} module(s) in {}", registry.modules.len(), path.display()),
        },
        Err(error) => DoctorCheck {
            name: "Module registry",
            ok: false,
            detail: error.to_string(),
        },
    }
}

fn write_permission_check(repo_root: &Path) -> DoctorCheck {
    let build_root = repo_root.join("build");
    let check_path = build_root.join(".doctor-write.tmp");
    let result = fs::create_dir_all(&build_root)
        .and_then(|_| fs::write(&check_path, "ok"))
        .and_then(|_| fs::remove_file(&check_path));

    DoctorCheck {
        name: "Write permission",
        ok: result.is_ok(),
        detail: build_root.display().to_string(),
    }
}

fn bundled_dependency_path(
    source_dir: &Path,
    bundle_root: &Path,
    source_literal: &str,
) -> Option<PathBuf> {
    let mut destination = source_dir.to_path_buf();
    for component in Path::new(source_literal).components() {
        match component {
            Component::Normal(value) => destination.push(value),
            Component::CurDir => {}
            Component::ParentDir => {
                destination.pop();
                if !destination.starts_with(bundle_root) {
                    return None;
                }
            }
            Component::Prefix(_) | Component::RootDir => return None,
        }
    }
    if destination.starts_with(bundle_root) {
        Some(destination)
    } else {
        None
    }
}

fn standalone_runner_script(source_file_name: &str) -> String {
    format!(
        "@echo off\r\nsetlocal\r\ncd /d \"%~dp0\"\r\nif \"%~1\"==\"--help\" goto help\r\nif \"%~1\"==\"-h\" goto help\r\nif \"%~1\"==\"/?\" goto help\r\n\"%~dp0eng.exe\" run \"%~dp0source\\{}\" --save-artifacts %*\r\nexit /b %ERRORLEVEL%\r\n:help\r\ntype \"%~dp0ARGS_HELP.txt\"\r\nexit /b 0\r\n",
        source_file_name
    )
}

fn args_field_count(report: &CheckReport) -> usize {
    let arg_type = report
        .semantic_program
        .workflow
        .arg_type
        .as_deref()
        .unwrap_or("Args");
    report
        .semantic_program
        .args_blocks
        .iter()
        .find(|args_block| args_block.name == arg_type)
        .map(|args_block| args_block.fields.len())
        .unwrap_or(0)
}

fn args_help_text(report: &CheckReport) -> String {
    let arg_type = report
        .semantic_program
        .workflow
        .arg_type
        .as_deref()
        .unwrap_or("Args");
    let mut text = String::new();
    text.push_str("EngLang standalone package\n\n");
    text.push_str("Workflow:\n");
    text.push_str(&format!(
        "  {}\n\n",
        report.semantic_program.workflow.signature()
    ));
    text.push_str("Args metadata:\n");

    match report
        .semantic_program
        .args_blocks
        .iter()
        .find(|args_block| args_block.name == arg_type)
    {
        Some(args_block) if args_block.fields.is_empty() => {
            text.push_str(&format!(
                "  args block {} has no fields.\n",
                args_block.name
            ));
        }
        Some(args_block) => {
            text.push_str(&format!("  args block {}\n", args_block.name));
            for field in &args_block.fields {
                let required = if field.required {
                    "required"
                } else {
                    "optional"
                };
                text.push_str(&format!(
                    "  --{} <{}>  {}",
                    field.name, field.type_name, required
                ));
                if let Some(default_value) = &field.default_value {
                    text.push_str(&format!("; default = {default_value}"));
                }
                text.push('\n');
            }
        }
        None => {
            text.push_str(&format!(
                "  args {{ ... }} is not declared in this source for {arg_type}.\n"
            ));
        }
    }

    text.push_str("\nFlags are forwarded to eng.exe run and recorded in arg_values.\n");
    text
}

fn path_for_manifest(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn dependency_paths(dependencies: &[(String, String)]) -> String {
    if dependencies.is_empty() {
        return "-".to_owned();
    }
    dependencies
        .iter()
        .map(|dependency| dependency.0.as_str())
        .collect::<Vec<_>>()
        .join(";")
}

fn dependency_hashes(dependencies: &[(String, String)]) -> String {
    if dependencies.is_empty() {
        return "-".to_owned();
    }
    dependencies
        .iter()
        .map(|dependency| format!("{}:{}", dependency.0, dependency.1))
        .collect::<Vec<_>>()
        .join(";")
}

fn apply_runtime_lengths(execution: &mut VmExecution, runtime_data: &RuntimeData) {
    for object in &mut execution.objects {
        if object.kind != VmObjectKind::TimeSeries {
            continue;
        }
        if let Some(series) = runtime_data
            .time_series
            .iter()
            .find(|series| series.name == object.name)
        {
            object.len = Some(series.points.len());
        }
    }
}

fn render_stdout(report: &CheckReport, runtime_data: &RuntimeData) -> String {
    let mut output = String::new();
    for entry in runtime_log_entries(report, runtime_data) {
        if entry.level == "print" {
            output.push_str(&entry.message);
        } else {
            output.push_str(&format!("[{}] {}", entry.level, entry.message));
        }
        output.push('\n');
    }
    output
}

#[derive(Clone, Debug)]
struct RuntimeLogEntry {
    index: usize,
    level: String,
    message: String,
    line: usize,
}

fn runtime_log_entries(report: &CheckReport, runtime_data: &RuntimeData) -> Vec<RuntimeLogEntry> {
    report
        .semantic_program
        .prints
        .iter()
        .enumerate()
        .map(|(index, print)| RuntimeLogEntry {
            index,
            level: print.level.clone(),
            message: render_print_template(print, report, runtime_data),
            line: print.line,
        })
        .collect()
}

fn run_log_json(
    report: &CheckReport,
    runtime_data: &RuntimeData,
    profile: &ExecutionProfile,
    profile_diagnostics: &[ProfileDiagnostic],
    external_boundaries: &[ExternalBoundaryRecord],
) -> String {
    let entries = runtime_log_entries(report, runtime_data);
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"format\": \"eng-run-log-v1\",\n");
    json.push_str(&format!(
        "  \"runtime_version\": \"{}\",\n",
        json_escape(RUNTIME_VERSION)
    ));
    json.push_str(&format!(
        "  \"source_path\": \"{}\",\n",
        json_escape(&report.source_path.display().to_string())
    ));
    json.push_str(&format!(
        "  \"execution_profile\": \"{}\",\n",
        profile.as_str()
    ));
    json.push_str(&format!("  \"message_count\": {},\n", entries.len()));
    json.push_str("  \"messages\": [\n");
    for (entry_index, entry) in entries.iter().enumerate() {
        if entry_index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!("      \"index\": {},\n", entry.index));
        json.push_str(&format!(
            "      \"level\": \"{}\",\n",
            json_escape(&entry.level)
        ));
        json.push_str(&format!(
            "      \"message\": \"{}\",\n",
            json_escape(&entry.message)
        ));
        json.push_str(&format!("      \"line\": {}\n", entry.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str(&format!(
        "  \"external_boundary_event_count\": {},\n",
        external_boundaries.len()
    ));
    json.push_str("  \"external_boundary_events\": [\n");
    for (record_index, record) in external_boundaries.iter().enumerate() {
        if record_index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str("      \"kind\": \"process\",\n");
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&record.binding)
        ));
        json.push_str(&format!(
            "      \"command\": \"{}\",\n",
            json_escape(&record.command)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&record.status)
        ));
        json.push_str(&format!("      \"success\": {},\n", record.success));
        json.push_str(&format!(
            "      \"expected_output_status\": \"{}\",\n",
            json_escape(&record.expected_output_status)
        ));
        json.push_str(&format!(
            "      \"stdout_hash\": \"{}\",\n",
            json_escape(&record.stdout_hash)
        ));
        json.push_str(&format!(
            "      \"stderr_hash\": \"{}\",\n",
            json_escape(&record.stderr_hash)
        ));
        json.push_str(&format!("      \"line\": {}\n", record.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"profile_diagnostics\": [\n");
    push_profile_diagnostics_json(&mut json, profile_diagnostics, "    ");
    json.push_str("\n  ]\n");
    json.push_str("}\n");
    json
}

#[derive(Clone, Debug)]
struct ProcessExecutionRecord {
    binding: String,
    command: String,
    tool_version: Option<String>,
    args: Vec<String>,
    cwd: String,
    expected_outputs: Vec<ProcessExpectedOutputRecord>,
    expected_output_status: String,
    exit_code: Option<i32>,
    success: bool,
    stdout: String,
    stdout_hash: String,
    stderr: String,
    stderr_hash: String,
    duration_ms: u128,
    status: String,
    line: usize,
}

#[derive(Clone, Debug)]
struct ProcessExpectedOutputRecord {
    path: String,
    resolved_path: PathBuf,
    artifact_kind: String,
    exists: bool,
    hash: Option<String>,
    status: String,
    validation: ArtifactValidation,
}

#[derive(Clone, Debug)]
struct DbManifestRecord {
    binding: String,
    manifest_path: String,
    resolved_path: String,
    hash: Option<String>,
    database: Option<String>,
    transaction_status: Option<String>,
    schema_status: Option<String>,
    tables: Vec<DbManifestTableRecord>,
    status: String,
}

#[derive(Clone, Debug)]
struct DbManifestTableRecord {
    name: String,
    mode: String,
    key: Vec<String>,
    schema: Vec<String>,
    row_count: Option<u64>,
}

fn execute_process_runs(report: &CheckReport) -> Result<Vec<ProcessExecutionRecord>, RuntimeError> {
    let mut records = Vec::new();
    for process in &report.semantic_program.process_runs {
        let args = process_args_for_owner(report, process.line)?;
        let cwd = process_cwd_for_owner(report, process.line)?;
        let tool_version = process_string_option(report, process.line, "tool_version");
        let allow_failure = process_bool_option(report, process.line, "allow_failure");
        let started = Instant::now();
        let output = Command::new(&process.command)
            .args(&args)
            .current_dir(&cwd)
            .output()
            .map_err(|error| {
                invalid_input(&format!(
                    "process `{}` failed to start: {error}",
                    process.command
                ))
            })?;
        let duration_ms = started.elapsed().as_millis();
        let exit_code = output.status.code();
        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout_hash = hash_text(&stdout);
        let stderr_hash = hash_text(&stderr);
        let expected_outputs = process_expected_outputs_for_owner(report, process.line, &cwd)?;
        let expected_output_status = expected_output_status(&expected_outputs);
        if !success && !allow_failure {
            return Err(invalid_input(&format!(
                "process `{}` exited with code {}; add `with {{ allow_failure = true }}` to record the failure as a ProcessResult",
                process.command,
                exit_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".to_owned())
            )));
        }
        if expected_output_status == "missing" && !allow_failure {
            let missing = expected_outputs
                .iter()
                .filter(|output| !output.exists)
                .map(|output| output.path.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(invalid_input(&format!(
                "process `{}` did not create expected output(s): {}; add `with {{ allow_failure = true }}` to record the missing output contract",
                process.command, missing
            )));
        }
        records.push(ProcessExecutionRecord {
            binding: process.binding.clone(),
            command: process.command.clone(),
            tool_version,
            args,
            cwd: cwd.display().to_string(),
            expected_outputs,
            expected_output_status: expected_output_status.clone(),
            exit_code,
            success,
            stdout,
            stdout_hash,
            stderr,
            stderr_hash,
            duration_ms,
            status: if success && expected_output_status != "missing" {
                "completed".to_owned()
            } else if success {
                "output_missing_allowed".to_owned()
            } else {
                "failed_allowed".to_owned()
            },
            line: process.line,
        });
    }
    Ok(records)
}

fn db_manifest_records(records: &[ProcessExecutionRecord]) -> Vec<DbManifestRecord> {
    records
        .iter()
        .flat_map(|record| {
            record
                .expected_outputs
                .iter()
                .filter(|output| is_db_manifest_output(output))
                .map(|output| db_manifest_record(record, output))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn is_db_manifest_output(output: &ProcessExpectedOutputRecord) -> bool {
    let path = output.path.to_ascii_lowercase();
    let file_name = output
        .resolved_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    (path.contains("db") || path.contains("database") || file_name.contains("db"))
        && (path.contains("manifest") || file_name.contains("manifest"))
}

fn db_manifest_record(
    process: &ProcessExecutionRecord,
    output: &ProcessExpectedOutputRecord,
) -> DbManifestRecord {
    let mut record = DbManifestRecord {
        binding: process.binding.clone(),
        manifest_path: output.path.clone(),
        resolved_path: output.resolved_path.display().to_string(),
        hash: output.hash.clone(),
        database: None,
        transaction_status: None,
        schema_status: None,
        tables: Vec::new(),
        status: if output.exists {
            "manifest_unread".to_owned()
        } else {
            "missing".to_owned()
        },
    };
    if !output.exists {
        return record;
    }
    let Ok(source) = fs::read_to_string(&output.resolved_path) else {
        return record;
    };
    let Ok(value) = serde_json::from_str::<Value>(&source) else {
        record.status = "manifest_parse_failed".to_owned();
        return record;
    };
    record.database = json_field_string(&value, "database");
    record.transaction_status = json_field_string(&value, "transaction_status");
    record.schema_status = json_field_string(&value, "schema_status");
    if let Some(tables) = value.get("tables").and_then(Value::as_array) {
        record.tables = tables
            .iter()
            .map(|table| DbManifestTableRecord {
                name: json_field_string(table, "name").unwrap_or_default(),
                mode: json_field_string(table, "mode").unwrap_or_default(),
                key: json_field_string_array(table, "key"),
                schema: json_field_string_array(table, "schema"),
                row_count: table.get("row_count").and_then(Value::as_u64),
            })
            .collect();
    }
    record.status = "manifest_loaded".to_owned();
    record
}

fn json_field_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn json_field_string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn process_results_json(
    report: &CheckReport,
    records: &[ProcessExecutionRecord],
    profile: &ExecutionProfile,
) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"format\": \"eng-process-results-v1\",\n");
    json.push_str(&format!(
        "  \"runtime_version\": \"{}\",\n",
        json_escape(RUNTIME_VERSION)
    ));
    json.push_str(&format!(
        "  \"source_path\": \"{}\",\n",
        json_escape(&report.source_path.display().to_string())
    ));
    json.push_str(&format!(
        "  \"execution_profile\": \"{}\",\n",
        profile.as_str()
    ));
    json.push_str(&format!("  \"process_count\": {},\n", records.len()));
    json.push_str("  \"processes\": [\n");
    for (index, record) in records.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&record.binding)
        ));
        json.push_str(&format!(
            "      \"command\": \"{}\",\n",
            json_escape(&record.command)
        ));
        push_optional_json_string_runtime(
            &mut json,
            "tool_version",
            record.tool_version.as_deref(),
            6,
        );
        json.push_str("      \"args\": ");
        push_json_string_array_runtime(&mut json, &record.args);
        json.push_str(",\n");
        json.push_str(&format!(
            "      \"cwd\": \"{}\",\n",
            json_escape(&record.cwd)
        ));
        json.push_str("      \"expected_outputs\": [\n");
        for (output_index, output) in record.expected_outputs.iter().enumerate() {
            if output_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"kind\": \"{}\",\n",
                json_escape(&output.artifact_kind)
            ));
            json.push_str(&format!(
                "          \"path\": \"{}\",\n",
                json_escape(&output.path)
            ));
            json.push_str(&format!(
                "          \"resolved_path\": \"{}\",\n",
                json_escape(&output.resolved_path.display().to_string())
            ));
            json.push_str(&format!("          \"exists\": {},\n", output.exists));
            push_optional_json_string_runtime(&mut json, "hash", output.hash.as_deref(), 10);
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&output.status)
            ));
            push_artifact_validation_json(&mut json, &output.validation, 10);
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str(&format!(
            "      \"expected_output_status\": \"{}\",\n",
            json_escape(&record.expected_output_status)
        ));
        match record.exit_code {
            Some(code) => json.push_str(&format!("      \"exit_code\": {code},\n")),
            None => json.push_str("      \"exit_code\": null,\n"),
        }
        json.push_str(&format!("      \"success\": {},\n", record.success));
        json.push_str(&format!(
            "      \"stdout\": \"{}\",\n",
            json_escape(&record.stdout)
        ));
        json.push_str(&format!(
            "      \"stdout_hash\": \"{}\",\n",
            json_escape(&record.stdout_hash)
        ));
        json.push_str(&format!(
            "      \"stderr\": \"{}\",\n",
            json_escape(&record.stderr)
        ));
        json.push_str(&format!(
            "      \"stderr_hash\": \"{}\",\n",
            json_escape(&record.stderr_hash)
        ));
        json.push_str(&format!("      \"duration_ms\": {},\n", record.duration_ms));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&record.status)
        ));
        json.push_str(&format!("      \"line\": {}\n", record.line));
        json.push_str("    }");
    }
    json.push_str("\n  ]\n");
    json.push_str("}\n");
    json
}

#[derive(Clone, Debug)]
struct TestExecutionRecord {
    name: String,
    status: String,
    assertion_records: Vec<AssertionExecutionRecord>,
    golden_records: Vec<GoldenExecutionRecord>,
    line: usize,
}

#[derive(Clone, Debug)]
struct AssertionExecutionRecord {
    left: String,
    operator: String,
    right: String,
    tolerance: Option<String>,
    left_value: String,
    right_value: String,
    tolerance_value: Option<String>,
    success: bool,
    status: String,
    message: String,
    line: usize,
}

#[derive(Clone, Debug)]
struct GoldenExecutionRecord {
    artifact: String,
    expected: String,
    artifact_hash: Option<String>,
    expected_hash: Option<String>,
    success: bool,
    status: String,
    message: String,
    line: usize,
}

fn execute_tests(
    report: &CheckReport,
    runtime_data: &RuntimeData,
    result_dir: &Path,
) -> Result<Vec<TestExecutionRecord>, RuntimeError> {
    let mut records = Vec::new();
    for test in &report.semantic_program.tests {
        let assertion_records = test
            .assertions
            .iter()
            .map(|assertion| execute_assertion(assertion, report, runtime_data))
            .collect::<Vec<_>>();
        let golden_records = test
            .goldens
            .iter()
            .map(|golden| execute_golden(golden, report, result_dir))
            .collect::<Result<Vec<_>, _>>()?;
        let status = if assertion_records.iter().all(|assertion| assertion.success)
            && golden_records.iter().all(|golden| golden.success)
        {
            "passed"
        } else {
            "failed"
        };
        records.push(TestExecutionRecord {
            name: test.name.clone(),
            status: status.to_owned(),
            assertion_records,
            golden_records,
            line: test.line,
        });
    }
    Ok(records)
}

fn execute_assertion(
    assertion: &eng_compiler::AssertInfo,
    report: &CheckReport,
    runtime_data: &RuntimeData,
) -> AssertionExecutionRecord {
    let left = evaluate_runtime_expression(&assertion.left, report, runtime_data);
    let right = evaluate_runtime_expression(&assertion.right, report, runtime_data);
    let tolerance = assertion
        .tolerance
        .as_deref()
        .and_then(|expression| evaluate_runtime_expression(expression, report, runtime_data));
    let (success, status, message) = match (&left, &right) {
        (Some(RuntimeFormatValue::Number { .. }), Some(RuntimeFormatValue::Number { .. })) => {
            compare_numeric_assertion(
                assertion.operator.as_str(),
                left.as_ref().unwrap(),
                right.as_ref().unwrap(),
                tolerance.as_ref(),
            )
        }
        (Some(left), Some(right)) => compare_text_assertion(
            assertion.operator.as_str(),
            &format_runtime_value(left.clone(), None, None, false),
            &format_runtime_value(right.clone(), None, None, false),
        ),
        (None, _) => (
            false,
            "unresolved".to_owned(),
            format!(
                "left expression `{}` could not be evaluated",
                assertion.left
            ),
        ),
        (_, None) => (
            false,
            "unresolved".to_owned(),
            format!(
                "right expression `{}` could not be evaluated",
                assertion.right
            ),
        ),
    };
    AssertionExecutionRecord {
        left: assertion.left.clone(),
        operator: assertion.operator.clone(),
        right: assertion.right.clone(),
        tolerance: assertion.tolerance.clone(),
        left_value: left
            .map(|value| format_runtime_value(value, None, None, true))
            .unwrap_or_default(),
        right_value: right
            .map(|value| format_runtime_value(value, None, None, true))
            .unwrap_or_default(),
        tolerance_value: tolerance.map(|value| format_runtime_value(value, None, None, true)),
        success,
        status,
        message,
        line: assertion.line,
    }
}

fn compare_numeric_assertion(
    operator: &str,
    left: &RuntimeFormatValue,
    right: &RuntimeFormatValue,
    tolerance: Option<&RuntimeFormatValue>,
) -> (bool, String, String) {
    let RuntimeFormatValue::Number {
        value: left_value,
        quantity_kind,
        unit: left_unit,
    } = left
    else {
        unreachable!();
    };
    let RuntimeFormatValue::Number {
        value: right_value,
        unit: right_unit,
        ..
    } = right
    else {
        unreachable!();
    };
    let right_value = convert_between_units(*right_value, right_unit, left_unit, quantity_kind)
        .unwrap_or(*right_value);
    let tolerance_value = match tolerance {
        Some(RuntimeFormatValue::Number {
            value,
            unit: tolerance_unit,
            ..
        }) => convert_between_units(*value, tolerance_unit, left_unit, quantity_kind)
            .unwrap_or(*value),
        _ => 1e-9,
    };
    let difference = (*left_value - right_value).abs();
    let success = match operator {
        "==" => difference <= tolerance_value,
        "!=" => difference > tolerance_value,
        ">" => *left_value > right_value,
        ">=" => *left_value >= right_value,
        "<" => *left_value < right_value,
        "<=" => *left_value <= right_value,
        _ => false,
    };
    let status = if success { "passed" } else { "failed" }.to_owned();
    let message = if success {
        "assertion passed".to_owned()
    } else {
        format!(
            "numeric assertion failed: left={}, right={}, tolerance={}",
            left_value, right_value, tolerance_value
        )
    };
    (success, status, message)
}

fn compare_text_assertion(operator: &str, left: &str, right: &str) -> (bool, String, String) {
    let success = match operator {
        "==" => left == right,
        "!=" => left != right,
        _ => false,
    };
    let status = if success { "passed" } else { "failed" }.to_owned();
    let message = if success {
        "assertion passed".to_owned()
    } else if matches!(operator, "==" | "!=") {
        format!("text assertion failed: left=`{left}`, right=`{right}`")
    } else {
        "text assertions support only == and !=".to_owned()
    };
    (success, status, message)
}

fn execute_golden(
    golden: &eng_compiler::GoldenInfo,
    report: &CheckReport,
    result_dir: &Path,
) -> Result<GoldenExecutionRecord, RuntimeError> {
    let artifact_path = export_output_path(result_dir, &golden.artifact)
        .ok_or_else(|| invalid_input(&format!("invalid golden artifact `{}`", golden.artifact)))?;
    let expected_text = evaluate_runtime_path_expression(&golden.expected, report)
        .ok_or_else(|| invalid_input(&format!("invalid golden expected `{}`", golden.expected)))?;
    let expected_path =
        runtime_resolve_source_relative_path(&expected_text, report.source_path.parent());
    let artifact = fs::read_to_string(&artifact_path);
    let expected = fs::read_to_string(&expected_path);
    let (success, status, message, artifact_hash, expected_hash) = match (artifact, expected) {
        (Ok(artifact), Ok(expected)) => {
            let artifact_hash = hash_text(&artifact);
            let expected_hash = hash_text(&expected);
            if normalize_golden_text(&artifact) == normalize_golden_text(&expected) {
                (
                    true,
                    "passed".to_owned(),
                    "golden matched".to_owned(),
                    Some(artifact_hash),
                    Some(expected_hash),
                )
            } else {
                (
                    false,
                    "failed".to_owned(),
                    "golden contents differ".to_owned(),
                    Some(artifact_hash),
                    Some(expected_hash),
                )
            }
        }
        (Err(error), _) => (
            false,
            "missing_artifact".to_owned(),
            format!("artifact could not be read: {error}"),
            None,
            None,
        ),
        (_, Err(error)) => (
            false,
            "missing_expected".to_owned(),
            format!("expected file could not be read: {error}"),
            None,
            None,
        ),
    };
    Ok(GoldenExecutionRecord {
        artifact: relative_output_path(result_dir, &artifact_path),
        expected: path_for_manifest(&expected_path),
        artifact_hash,
        expected_hash,
        success,
        status,
        message,
        line: golden.line,
    })
}

fn normalize_golden_text(value: &str) -> String {
    value.replace("\r\n", "\n")
}

fn test_results_json(report: &CheckReport, records: &[TestExecutionRecord]) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"format\": \"eng-test-results-v1\",\n");
    json.push_str(&format!(
        "  \"runtime_version\": \"{}\",\n",
        json_escape(RUNTIME_VERSION)
    ));
    json.push_str(&format!(
        "  \"source_path\": \"{}\",\n",
        json_escape(&report.source_path.display().to_string())
    ));
    json.push_str(&format!("  \"test_count\": {},\n", records.len()));
    let failed_count = records
        .iter()
        .filter(|record| record.status == "failed")
        .count();
    json.push_str(&format!("  \"failed_count\": {},\n", failed_count));
    json.push_str("  \"tests\": [\n");
    for (index, record) in records.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&record.name)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&record.status)
        ));
        json.push_str(&format!("      \"line\": {},\n", record.line));
        json.push_str("      \"assertions\": [\n");
        for (assertion_index, assertion) in record.assertion_records.iter().enumerate() {
            if assertion_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"left\": \"{}\",\n",
                json_escape(&assertion.left)
            ));
            json.push_str(&format!(
                "          \"operator\": \"{}\",\n",
                json_escape(&assertion.operator)
            ));
            json.push_str(&format!(
                "          \"right\": \"{}\",\n",
                json_escape(&assertion.right)
            ));
            push_optional_json_string_runtime(
                &mut json,
                "tolerance",
                assertion.tolerance.as_deref(),
                10,
            );
            json.push_str(&format!(
                "          \"left_value\": \"{}\",\n",
                json_escape(&assertion.left_value)
            ));
            json.push_str(&format!(
                "          \"right_value\": \"{}\",\n",
                json_escape(&assertion.right_value)
            ));
            push_optional_json_string_runtime(
                &mut json,
                "tolerance_value",
                assertion.tolerance_value.as_deref(),
                10,
            );
            json.push_str(&format!("          \"success\": {},\n", assertion.success));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&assertion.status)
            ));
            json.push_str(&format!(
                "          \"message\": \"{}\",\n",
                json_escape(&assertion.message)
            ));
            json.push_str(&format!("          \"line\": {}\n", assertion.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"goldens\": [\n");
        for (golden_index, golden) in record.golden_records.iter().enumerate() {
            if golden_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"artifact\": \"{}\",\n",
                json_escape(&golden.artifact)
            ));
            json.push_str(&format!(
                "          \"expected\": \"{}\",\n",
                json_escape(&golden.expected)
            ));
            push_optional_json_string_runtime(
                &mut json,
                "artifact_hash",
                golden.artifact_hash.as_deref(),
                10,
            );
            push_optional_json_string_runtime(
                &mut json,
                "expected_hash",
                golden.expected_hash.as_deref(),
                10,
            );
            json.push_str(&format!("          \"success\": {},\n", golden.success));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&golden.status)
            ));
            json.push_str(&format!(
                "          \"message\": \"{}\",\n",
                json_escape(&golden.message)
            ));
            json.push_str(&format!("          \"line\": {}\n", golden.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ]\n");
    json.push_str("}\n");
    json
}

fn process_args_for_owner(
    report: &CheckReport,
    owner_line: usize,
) -> Result<Vec<String>, RuntimeError> {
    let Some(raw) = process_option(report, owner_line, "args") else {
        return Ok(Vec::new());
    };
    parse_process_args(&raw)
}

fn process_cwd_for_owner(report: &CheckReport, owner_line: usize) -> Result<PathBuf, RuntimeError> {
    let raw = process_option(report, owner_line, "cwd");
    let cwd = if let Some(raw) = raw {
        let path_text = evaluate_runtime_path_expression(&raw, report)
            .ok_or_else(|| invalid_input(&format!("invalid process cwd `{raw}`")))?;
        runtime_resolve_source_relative_path(&path_text, report.source_path.parent())
    } else {
        report
            .source_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };
    Ok(cwd)
}

fn process_expected_outputs_for_owner(
    report: &CheckReport,
    owner_line: usize,
    cwd: &Path,
) -> Result<Vec<ProcessExpectedOutputRecord>, RuntimeError> {
    let Some(raw) = process_option(report, owner_line, "expected_outputs") else {
        return Ok(Vec::new());
    };
    let artifact_kind = artifact_kind_for_owner(report, owner_line, "process_expected_output");
    parse_process_expected_outputs(&raw, report, cwd, &artifact_kind)
}

fn parse_process_expected_outputs(
    raw: &str,
    report: &CheckReport,
    cwd: &Path,
    artifact_kind: &str,
) -> Result<Vec<ProcessExpectedOutputRecord>, RuntimeError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let parts = if let Some(inner) = trimmed
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        if inner.trim().is_empty() {
            Vec::new()
        } else {
            split_top_level(inner, &[','])
        }
    } else {
        vec![trimmed.to_owned()]
    };
    parts
        .into_iter()
        .map(|part| process_expected_output_record(&part, report, cwd, artifact_kind))
        .collect()
}

fn process_expected_output_record(
    raw: &str,
    report: &CheckReport,
    cwd: &Path,
    artifact_kind: &str,
) -> Result<ProcessExpectedOutputRecord, RuntimeError> {
    let raw = raw.trim();
    let path_text = evaluate_runtime_path_expression(raw, report)
        .ok_or_else(|| invalid_input(&format!("invalid process expected output `{raw}`")))?;
    let resolved_path = runtime_resolve_source_relative_path(&path_text, Some(cwd));
    let (exists, hash, status) = match fs::read(&resolved_path) {
        Ok(bytes) => (true, Some(hash_bytes(&bytes)), "exists".to_owned()),
        Err(_) if resolved_path.exists() => (true, None, "exists_unhashed".to_owned()),
        Err(_) => (false, None, "missing".to_owned()),
    };
    let validation = expected_output_validation(exists, hash.as_ref());
    Ok(ProcessExpectedOutputRecord {
        path: runtime_path_text(&path_text),
        resolved_path,
        artifact_kind: artifact_kind.to_owned(),
        exists,
        hash,
        status,
        validation,
    })
}

fn expected_output_validation(exists: bool, hash: Option<&String>) -> ArtifactValidation {
    if exists && hash.is_some() {
        artifact_validation(
            "passed",
            "exists_and_hash",
            "expected output exists and was hashed",
        )
    } else if exists {
        artifact_validation(
            "unavailable",
            "exists_and_hash",
            "expected output exists but could not be hashed",
        )
    } else {
        artifact_validation("failed", "exists_and_hash", "expected output is missing")
    }
}

fn artifact_validation(status: &str, rule: &str, message: &str) -> ArtifactValidation {
    ArtifactValidation::new(status, rule, message)
}

fn expected_output_status(outputs: &[ProcessExpectedOutputRecord]) -> String {
    if outputs.is_empty() {
        "not_declared".to_owned()
    } else if outputs.iter().all(|output| output.exists) {
        "satisfied".to_owned()
    } else {
        "missing".to_owned()
    }
}

fn process_bool_option(report: &CheckReport, owner_line: usize, key: &str) -> bool {
    process_option(report, owner_line, key)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "true" | "yes" | "on" | "1"
            )
        })
        .unwrap_or(false)
}

fn process_string_option(report: &CheckReport, owner_line: usize, key: &str) -> Option<String> {
    process_option(report, owner_line, key).map(|value| strip_runtime_string_value(&value))
}

fn artifact_kind_for_owner(report: &CheckReport, owner_line: usize, default_kind: &str) -> String {
    process_string_option(report, owner_line, "artifact_kind")
        .map(|value| normalize_artifact_kind(&value, default_kind))
        .unwrap_or_else(|| default_kind.to_owned())
}

fn normalize_artifact_kind(value: &str, default_kind: &str) -> String {
    let normalized = value
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_owned();
    if normalized.is_empty() {
        default_kind.to_owned()
    } else {
        normalized
    }
}

fn process_option(report: &CheckReport, owner_line: usize, key: &str) -> Option<String> {
    report
        .semantic_program
        .with_blocks
        .iter()
        .filter(|block| block.owner_line == Some(owner_line))
        .flat_map(|block| block.options.iter())
        .find(|option| option.key == key && option.status == "accepted")
        .map(|option| option.value.clone())
}

fn parse_process_args(raw: &str) -> Result<Vec<String>, RuntimeError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if let Some(inner) = trimmed
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        if inner.trim().is_empty() {
            return Ok(Vec::new());
        }
        return split_top_level(inner, &[','])
            .into_iter()
            .map(|part| parse_process_arg(&part))
            .collect();
    }
    Ok(vec![parse_process_arg(trimmed)?])
}

fn parse_process_arg(raw: &str) -> Result<String, RuntimeError> {
    let trimmed = raw.trim();
    if trimmed.starts_with('"') {
        Ok(strip_runtime_string_value(trimmed))
    } else {
        Err(invalid_input(&format!(
            "process args must be string literals, got `{trimmed}`"
        )))
    }
}

fn push_json_string_array_runtime(json: &mut String, values: &[String]) {
    json.push('[');
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!("\"{}\"", json_escape(value)));
    }
    json.push(']');
}

fn push_optional_json_string_runtime(
    json: &mut String,
    key: &str,
    value: Option<&str>,
    indent: usize,
) {
    let spaces = " ".repeat(indent);
    match value {
        Some(value) => json.push_str(&format!("{spaces}\"{key}\": \"{}\",\n", json_escape(value))),
        None => json.push_str(&format!("{spaces}\"{key}\": null,\n")),
    }
}

fn render_print_template(
    print: &eng_compiler::PrintInfo,
    report: &CheckReport,
    runtime_data: &RuntimeData,
) -> String {
    let mut rendered = String::new();
    let mut cursor = 0usize;
    let mut field_index = 0usize;
    while let Some(open) = print.template[cursor..].find('{') {
        let start = cursor + open;
        rendered.push_str(&print.template[cursor..start]);
        let Some(close_offset) = print.template[start + 1..].find('}') else {
            rendered.push_str(&print.template[start..]);
            return rendered;
        };
        let close = start + 1 + close_offset;
        let field_text = print.template[start + 1..close].trim();
        let fallback = format!("{{{field_text}}}");
        let value = print
            .fields
            .get(field_index)
            .and_then(|field| evaluate_runtime_expression(&field.expression, report, runtime_data))
            .map(|value| {
                let field = &print.fields[field_index];
                format_runtime_value(
                    value,
                    field.requested_unit.as_deref(),
                    field.precision,
                    true,
                )
            })
            .unwrap_or(fallback);
        rendered.push_str(&value);
        field_index += 1;
        cursor = close + 1;
    }
    rendered.push_str(&print.template[cursor..]);
    rendered
}

#[derive(Clone, Debug)]
struct OutputArtifact {
    kind: String,
    path: String,
    hash: String,
    absolute_path: PathBuf,
    validation: ArtifactValidation,
}

struct ArtifactRegistryContext<'a> {
    report: &'a CheckReport,
    runtime_data: &'a RuntimeData,
    process_results: &'a [ProcessExecutionRecord],
    db_manifest_records: &'a [DbManifestRecord],
    test_results: &'a [TestExecutionRecord],
}

fn process_expected_output_artifacts(records: &[ProcessExecutionRecord]) -> Vec<OutputArtifact> {
    records
        .iter()
        .flat_map(|record| record.expected_outputs.iter())
        .filter_map(|expected| {
            let hash = expected.hash.as_ref()?;
            let kind = if is_db_manifest_output(expected) {
                "db_write_manifest"
            } else {
                expected.artifact_kind.as_str()
            };
            Some(OutputArtifact {
                kind: kind.to_owned(),
                path: path_for_manifest(&expected.resolved_path),
                hash: hash.clone(),
                absolute_path: expected.resolved_path.clone(),
                validation: expected.validation.clone(),
            })
        })
        .collect()
}

fn output_artifact(
    kind: &str,
    path: String,
    contents: &str,
    absolute_path: PathBuf,
) -> OutputArtifact {
    OutputArtifact {
        kind: kind.to_owned(),
        path,
        hash: hash_text(contents),
        absolute_path,
        validation: artifact_validation(
            "passed",
            "content_hash",
            "generated artifact was written and hashed",
        ),
    }
}

fn write_csv_exports(
    report: &CheckReport,
    runtime_data: &RuntimeData,
    result_dir: &Path,
) -> Result<Vec<OutputArtifact>, RuntimeError> {
    let mut artifacts = Vec::new();
    for export in &report.semantic_program.csv_exports {
        if export.source != "summary" {
            continue;
        }
        let path = export_output_path(result_dir, &export.path).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid export path `{}`", export.path),
            )
        })?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let headers = export
            .fields
            .iter()
            .map(csv_export_header)
            .collect::<Vec<_>>();
        let values = export
            .fields
            .iter()
            .map(|field| {
                evaluate_runtime_expression(&field.expression, report, runtime_data)
                    .map(|value| {
                        format_runtime_value(
                            value,
                            field.requested_unit.as_deref(),
                            field.precision,
                            false,
                        )
                    })
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();
        let mut csv = String::new();
        csv.push_str(
            &headers
                .iter()
                .map(|value| csv_escape(value))
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');
        csv.push_str(
            &values
                .iter()
                .map(|value| csv_escape(value))
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');
        write_output_file(&path, &csv, overwrite_allowed(report, export.line))?;
        artifacts.push(output_artifact(
            "csv_export",
            relative_output_path(result_dir, &path),
            &csv,
            path,
        ));
    }
    Ok(artifacts)
}

fn write_outputs(
    report: &CheckReport,
    runtime_data: &RuntimeData,
    result_dir: &Path,
) -> Result<Vec<OutputArtifact>, RuntimeError> {
    let mut artifacts = Vec::new();
    for write in &report.semantic_program.writes {
        let path_text = evaluate_runtime_path_expression(&write.path, report).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid write path `{}`", write.path),
            )
        })?;
        let path = export_output_path(result_dir, &path_text).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid write path `{}`", write.path),
            )
        })?;
        let contents = render_write_contents(write, report, runtime_data).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("cannot resolve write expression `{}`", write.expression),
            )
        })?;
        write_output_file(&path, &contents, overwrite_allowed(report, write.line))?;
        let artifact_kind =
            artifact_kind_for_owner(report, write.line, &format!("write_{}", write.format));
        artifacts.push(output_artifact(
            &artifact_kind,
            relative_output_path(result_dir, &path),
            &contents,
            path,
        ));
    }
    Ok(artifacts)
}

fn apply_file_operations(
    report: &CheckReport,
    result_dir: &Path,
) -> Result<Vec<OutputArtifact>, RuntimeError> {
    let mut artifacts = Vec::new();
    for operation in &report.semantic_program.file_operations {
        match operation.operation.as_str() {
            "copy" => {
                let source = resolve_copy_source_path(report, result_dir, &operation.source)?;
                let destination = operation
                    .destination
                    .as_deref()
                    .ok_or_else(|| invalid_input("copy operation is missing destination"))?;
                let destination_text = evaluate_runtime_path_expression(destination, report)
                    .ok_or_else(|| {
                        invalid_input(&format!("invalid copy destination `{destination}`"))
                    })?;
                let destination_path = export_output_path(result_dir, &destination_text)
                    .ok_or_else(|| {
                        invalid_input(&format!("invalid copy destination `{destination}`"))
                    })?;
                let contents = fs::read_to_string(&source)?;
                write_output_file(
                    &destination_path,
                    &contents,
                    overwrite_allowed(report, operation.line),
                )?;
                artifacts.push(output_artifact(
                    "copy_file",
                    relative_output_path(result_dir, &destination_path),
                    &contents,
                    destination_path,
                ));
            }
            "move" => {
                let source_path =
                    resolve_output_operation_path(report, result_dir, &operation.source)
                        .ok_or_else(|| {
                            invalid_input(&format!("invalid move source `{}`", operation.source))
                        })?;
                let destination = operation
                    .destination
                    .as_deref()
                    .ok_or_else(|| invalid_input("move operation is missing destination"))?;
                let destination_path =
                    resolve_output_operation_path(report, result_dir, destination).ok_or_else(
                        || invalid_input(&format!("invalid move destination `{destination}`")),
                    )?;
                let contents = fs::read_to_string(&source_path)?;
                write_output_file(
                    &destination_path,
                    &contents,
                    overwrite_allowed(report, operation.line),
                )?;
                if source_path != destination_path {
                    fs::remove_file(&source_path)?;
                }
                artifacts.push(output_artifact(
                    "move_file",
                    relative_output_path(result_dir, &destination_path),
                    &contents,
                    destination_path,
                ));
            }
            "delete" => {
                let target_path =
                    resolve_output_operation_path(report, result_dir, &operation.source)
                        .ok_or_else(|| {
                            invalid_input(&format!("invalid delete target `{}`", operation.source))
                        })?;
                let relative_path = relative_output_path(result_dir, &target_path);
                if target_path.is_dir() {
                    fs::remove_dir_all(&target_path)?;
                    artifacts.push(OutputArtifact {
                        kind: "delete_dir".to_owned(),
                        path: relative_path,
                        hash: hash_text("deleted_dir"),
                        absolute_path: target_path,
                        validation: artifact_validation(
                            "passed",
                            "file_operation",
                            "generated directory was deleted",
                        ),
                    });
                } else if target_path.exists() {
                    let contents = fs::read_to_string(&target_path).unwrap_or_default();
                    fs::remove_file(&target_path)?;
                    artifacts.push(output_artifact(
                        "delete_file",
                        relative_path,
                        &contents,
                        target_path,
                    ));
                } else {
                    artifacts.push(OutputArtifact {
                        kind: "delete_missing".to_owned(),
                        path: relative_path,
                        hash: hash_text("missing"),
                        absolute_path: target_path,
                        validation: artifact_validation(
                            "passed",
                            "file_operation",
                            "delete target was already absent",
                        ),
                    });
                }
            }
            _ => {}
        }
    }
    Ok(artifacts)
}

fn resolve_copy_source_path(
    report: &CheckReport,
    result_dir: &Path,
    expression: &str,
) -> Result<PathBuf, RuntimeError> {
    let path_text = evaluate_runtime_path_expression(expression, report)
        .ok_or_else(|| invalid_input(&format!("invalid copy source `{expression}`")))?;
    if let Some(output_path) = export_output_path(result_dir, &path_text) {
        if output_path.exists() {
            return Ok(output_path);
        }
    }
    Ok(runtime_resolve_source_relative_path(
        &path_text,
        report.source_path.parent(),
    ))
}

fn resolve_output_operation_path(
    report: &CheckReport,
    result_dir: &Path,
    expression: &str,
) -> Option<PathBuf> {
    let path_text = evaluate_runtime_path_expression(expression, report)?;
    export_output_path(result_dir, &path_text)
}

fn invalid_input(message: &str) -> RuntimeError {
    RuntimeError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        message.to_owned(),
    ))
}

fn write_output_file(path: &Path, contents: &str, overwrite: bool) -> Result<(), RuntimeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        let existing = fs::read_to_string(path)?;
        if existing == contents {
            return Ok(());
        }
        if !overwrite {
            return Err(RuntimeError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!(
                    "output `{}` already exists with different contents; add `with {{ overwrite = true }}`",
                    path.display()
                ),
            )));
        }
    }
    fs::write(path, contents)?;
    Ok(())
}

fn overwrite_allowed(report: &CheckReport, owner_line: usize) -> bool {
    report.semantic_program.with_blocks.iter().any(|block| {
        block.owner_line == Some(owner_line)
            && block.options.iter().any(|option| {
                option.key == "overwrite"
                    && option.status == "accepted"
                    && option.value.trim().eq_ignore_ascii_case("true")
            })
    })
}

fn render_write_contents(
    write: &eng_compiler::WriteInfo,
    report: &CheckReport,
    runtime_data: &RuntimeData,
) -> Option<String> {
    let value = evaluate_runtime_expression(&write.expression, report, runtime_data)?;
    match write.format.as_str() {
        "text" => Some(format_runtime_value(value, None, None, true)),
        "json" => Some(format_runtime_json_value(value)),
        _ => None,
    }
}

fn format_runtime_json_value(value: RuntimeFormatValue) -> String {
    match value {
        RuntimeFormatValue::Number {
            value,
            quantity_kind,
            unit,
        } => format!(
            "{{\n  \"value\": {},\n  \"quantity_kind\": \"{}\",\n  \"unit\": \"{}\"\n}}\n",
            value,
            json_escape(&quantity_kind),
            json_escape(&unit)
        ),
        RuntimeFormatValue::Text(text) | RuntimeFormatValue::Summary(text) => {
            let trimmed = text.trim();
            if (trimmed.starts_with('{') && trimmed.ends_with('}'))
                || (trimmed.starts_with('[') && trimmed.ends_with(']'))
            {
                format!("{trimmed}\n")
            } else {
                format!("\"{}\"\n", json_escape(&text))
            }
        }
    }
}

fn relative_output_path(result_dir: &Path, path: &Path) -> String {
    path.strip_prefix(result_dir)
        .map(path_for_manifest)
        .unwrap_or_else(|_| path_for_manifest(path))
}

fn export_output_path(result_dir: &Path, raw_path: &str) -> Option<PathBuf> {
    let path = Path::new(raw_path);
    let mut destination = result_dir.to_path_buf();
    for component in path.components() {
        match component {
            Component::Normal(value) => destination.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => return None,
        }
    }
    Some(destination)
}

fn output_manifest_json(
    source_path: &Path,
    artifacts: &[OutputArtifact],
    profile: &ExecutionProfile,
    profile_diagnostics: &[ProfileDiagnostic],
    registry: &ArtifactRegistryContext<'_>,
) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"format\": \"eng-output-manifest-v1\",\n");
    json.push_str(&format!(
        "  \"runtime_version\": \"{}\",\n",
        json_escape(RUNTIME_VERSION)
    ));
    json.push_str(&format!(
        "  \"source_path\": \"{}\",\n",
        json_escape(&source_path.display().to_string())
    ));
    json.push_str(&format!(
        "  \"execution_profile\": \"{}\",\n",
        profile.as_str()
    ));
    json.push_str(&format!("  \"artifact_count\": {},\n", artifacts.len()));
    json.push_str("  \"artifacts\": [\n");
    for (index, artifact) in artifacts.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"kind\": \"{}\",\n",
            json_escape(&artifact.kind)
        ));
        json.push_str(&format!(
            "      \"path\": \"{}\",\n",
            json_escape(&artifact.path)
        ));
        json.push_str(&format!(
            "      \"hash\": \"{}\",\n",
            json_escape(&artifact.hash)
        ));
        push_artifact_validation_json(&mut json, &artifact.validation, 6);
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"artifact_registry\": {\n");
    push_artifact_registry_json(&mut json, artifacts, registry);
    json.push_str("\n  },\n");
    json.push_str("  \"profile_diagnostics\": [\n");
    push_profile_diagnostics_json(&mut json, profile_diagnostics, "    ");
    json.push_str("\n  ]\n");
    json.push_str("}\n");
    json
}

fn source_records_for_registry(registry: &ArtifactRegistryContext<'_>) -> Vec<SourceRecord> {
    let mut records = vec![SourceRecord {
        kind: "source_file".to_owned(),
        binding: "program".to_owned(),
        path: registry.report.source_path.display().to_string(),
        hash: Some(registry.report.source_hash.clone()),
        schema: None,
        row_count: None,
        status: "loaded".to_owned(),
        line: 1,
    }];
    records.extend(
        registry
            .report
            .semantic_program
            .csv_promotions
            .iter()
            .map(|promotion| SourceRecord {
                kind: "source_file".to_owned(),
                binding: promotion.binding.clone(),
                path: promotion.resolved_path.clone(),
                hash: promotion.source_hash.clone(),
                schema: Some(promotion.schema_name.clone()),
                row_count: Some(promotion.row_count),
                status: "promoted_csv".to_owned(),
                line: promotion.line,
            }),
    );
    records.extend(
        registry
            .report
            .semantic_program
            .environment_dependencies
            .iter()
            .filter(|dependency| dependency.kind.starts_with("filesystem_read_"))
            .map(|dependency| SourceRecord {
                kind: "source_file".to_owned(),
                binding: dependency.name.clone(),
                path: dependency.resolved_value.clone(),
                hash: dependency.source_hash.clone(),
                schema: None,
                row_count: None,
                status: dependency.status.clone(),
                line: dependency.line,
            }),
    );
    records
}

fn artifact_records_for_outputs(artifacts: &[OutputArtifact]) -> Vec<ArtifactRecord> {
    artifacts
        .iter()
        .map(|artifact| ArtifactRecord {
            kind: artifact.kind.clone(),
            class: artifact_record_class(&artifact.kind).to_owned(),
            path: artifact.path.clone(),
            hash: artifact.hash.clone(),
            status: "generated".to_owned(),
            validation: artifact.validation.clone(),
        })
        .collect()
}

fn external_boundary_records_for_processes(
    processes: &[ProcessExecutionRecord],
) -> Vec<ExternalBoundaryRecord> {
    processes
        .iter()
        .map(|process| ExternalBoundaryRecord {
            binding: process.binding.clone(),
            command: process.command.clone(),
            tool_version: process.tool_version.clone(),
            args: process.args.clone(),
            cwd: process.cwd.clone(),
            expected_output_count: process.expected_outputs.len(),
            expected_output_status: process.expected_output_status.clone(),
            stdout_hash: process.stdout_hash.clone(),
            stderr_hash: process.stderr_hash.clone(),
            success: process.success,
            status: process.status.clone(),
            line: process.line,
        })
        .collect()
}

fn push_artifact_registry_json(
    json: &mut String,
    artifacts: &[OutputArtifact],
    registry: &ArtifactRegistryContext<'_>,
) {
    json.push_str("    \"format\": \"eng-artifact-registry-v1\",\n");
    json.push_str("    \"source_files\": [\n");
    for (index, record) in source_records_for_registry(registry).iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&record.binding)
        ));
        json.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&record.kind)
        ));
        json.push_str(&format!(
            "        \"path\": \"{}\",\n",
            json_escape(&record.path)
        ));
        push_optional_json_string_runtime(json, "hash", record.hash.as_deref(), 8);
        if let Some(schema) = &record.schema {
            json.push_str(&format!(
                "        \"schema\": \"{}\",\n",
                json_escape(schema)
            ));
        }
        if let Some(row_count) = record.row_count {
            json.push_str(&format!("        \"row_count\": {},\n", row_count));
        }
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&record.status)
        ));
        json.push_str(&format!("        \"line\": {}\n", record.line));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");

    json.push_str("    \"generated_files\": [\n");
    for (index, record) in artifact_records_for_outputs(artifacts).iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&record.kind)
        ));
        json.push_str(&format!(
            "        \"class\": \"{}\",\n",
            json_escape(&record.class)
        ));
        json.push_str(&format!(
            "        \"path\": \"{}\",\n",
            json_escape(&record.path)
        ));
        json.push_str(&format!(
            "        \"hash\": \"{}\",\n",
            json_escape(&record.hash)
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&record.status)
        ));
        push_artifact_validation_json(json, &record.validation, 8);
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");

    json.push_str("    \"external_commands\": [\n");
    for (index, record) in external_boundary_records_for_processes(registry.process_results)
        .iter()
        .enumerate()
    {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&record.binding)
        ));
        json.push_str(&format!(
            "        \"command\": \"{}\",\n",
            json_escape(&record.command)
        ));
        push_optional_json_string_runtime(json, "tool_version", record.tool_version.as_deref(), 8);
        json.push_str("        \"args\": ");
        push_json_string_array_runtime(json, &record.args);
        json.push_str(",\n");
        json.push_str(&format!(
            "        \"cwd\": \"{}\",\n",
            json_escape(&record.cwd)
        ));
        json.push_str(&format!(
            "        \"expected_output_count\": {},\n",
            record.expected_output_count
        ));
        json.push_str(&format!(
            "        \"expected_output_status\": \"{}\",\n",
            json_escape(&record.expected_output_status)
        ));
        json.push_str(&format!(
            "        \"stdout_hash\": \"{}\",\n",
            json_escape(&record.stdout_hash)
        ));
        json.push_str(&format!(
            "        \"stderr_hash\": \"{}\",\n",
            json_escape(&record.stderr_hash)
        ));
        json.push_str(&format!("        \"success\": {},\n", record.success));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&record.status)
        ));
        json.push_str(&format!("        \"line\": {}\n", record.line));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");

    json.push_str("    \"network_requests\": [],\n");

    json.push_str("    \"db_writes\": [\n");
    for (index, record) in registry.db_manifest_records.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&record.binding)
        ));
        json.push_str(&format!(
            "        \"manifest_path\": \"{}\",\n",
            json_escape(&record.manifest_path)
        ));
        push_optional_json_string_runtime(json, "hash", record.hash.as_deref(), 8);
        push_optional_json_string_runtime(json, "database", record.database.as_deref(), 8);
        push_optional_json_string_runtime(
            json,
            "transaction_status",
            record.transaction_status.as_deref(),
            8,
        );
        json.push_str(&format!(
            "        \"table_count\": {},\n",
            record.tables.len()
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\"\n",
            json_escape(&record.status)
        ));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");

    json.push_str("    \"model_artifacts\": [\n");
    let mut model_index = 0usize;
    for artifact in registry
        .runtime_data
        .ml_artifacts
        .iter()
        .filter(|artifact| artifact.model_artifact_hash.is_some() || artifact.model_card.is_some())
    {
        if model_index > 0 {
            json.push_str(",\n");
        }
        model_index += 1;
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&artifact.binding)
        ));
        json.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&artifact.kind)
        ));
        push_optional_json_string_runtime(json, "source", artifact.source.as_deref(), 8);
        push_optional_json_string_runtime(json, "target", artifact.target.as_deref(), 8);
        push_optional_json_string_runtime(
            json,
            "target_quantity",
            artifact.target_quantity.as_deref(),
            8,
        );
        json.push_str(&format!(
            "        \"target_unit\": \"{}\",\n",
            json_escape(&artifact.display_unit)
        ));
        push_optional_json_string_runtime(
            json,
            "training_data_hash",
            artifact.training_data_hash.as_deref(),
            8,
        );
        push_optional_json_string_runtime(
            json,
            "model_artifact_hash",
            artifact.model_artifact_hash.as_deref(),
            8,
        );
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&artifact.status)
        ));
        json.push_str(&format!("        \"line\": {}\n", artifact.line));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");

    json.push_str("    \"caches\": [],\n");

    json.push_str("    \"tests\": [\n");
    for (index, test) in registry.test_results.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&test.name)
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&test.status)
        ));
        json.push_str(&format!(
            "        \"assertion_count\": {},\n",
            test.assertion_records.len()
        ));
        json.push_str(&format!(
            "        \"golden_count\": {},\n",
            test.golden_records.len()
        ));
        json.push_str(&format!("        \"line\": {}\n", test.line));
        json.push_str("      }");
    }
    json.push_str("\n    ]");
}

fn push_artifact_validation_json(
    json: &mut String,
    validation: &ArtifactValidation,
    indent: usize,
) {
    let padding = " ".repeat(indent);
    json.push_str(&format!("{padding}\"validation\": {{\n"));
    json.push_str(&format!(
        "{padding}  \"status\": \"{}\",\n",
        json_escape(&validation.status)
    ));
    json.push_str(&format!(
        "{padding}  \"rule\": \"{}\",\n",
        json_escape(&validation.rule)
    ));
    json.push_str(&format!(
        "{padding}  \"message\": \"{}\"\n",
        json_escape(&validation.message)
    ));
    json.push_str(&format!("{padding}}}\n"));
}

fn artifact_record_class(kind: &str) -> &'static str {
    match kind {
        "review" | "report_spec" | "report_html" | "result" | "plot_spec" | "plot_svg"
        | "plot_manifest" | "bytecode" | "run_log" => "review_artifact",
        "process_results" | "process_expected_output" => "external_boundary",
        "db_write_manifest" => "db_write",
        "test_results" => "test",
        _ => "generated_file",
    }
}

fn csv_export_header(field: &eng_compiler::CsvExportFieldInfo) -> String {
    let unit = field
        .requested_unit
        .as_deref()
        .or_else(|| (!field.display_unit.is_empty()).then_some(field.display_unit.as_str()))
        .filter(|unit| *unit != "count");
    match unit {
        Some(unit) => format!("{} [{}]", field.name, unit),
        None => field.name.clone(),
    }
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

#[derive(Clone, Debug, PartialEq)]
enum RuntimeFormatValue {
    Number {
        value: f64,
        quantity_kind: String,
        unit: String,
    },
    Text(String),
    Summary(String),
}

fn evaluate_runtime_expression(
    expression: &str,
    report: &CheckReport,
    runtime_data: &RuntimeData,
) -> Option<RuntimeFormatValue> {
    let expression = expression.trim();
    if let Some(value) = evaluate_runtime_read_expression(expression, report) {
        return Some(RuntimeFormatValue::Text(value));
    }
    if let Some(value) = evaluate_runtime_exists_expression(expression, report) {
        return Some(RuntimeFormatValue::Text(value));
    }
    if let Some(value) = evaluate_runtime_path_expression(expression, report) {
        return Some(RuntimeFormatValue::Text(value));
    }
    if expression.starts_with('"') {
        return Some(RuntimeFormatValue::Text(strip_runtime_string_value(
            expression,
        )));
    }
    if matches!(expression, "true" | "false") {
        return Some(RuntimeFormatValue::Text(expression.to_owned()));
    }
    if let Some((value, unit)) = number_with_optional_unit(expression) {
        let unit = unit.unwrap_or_default();
        let quantity_kind = unit_info(&unit)
            .map(|info| {
                if info.quantity_hint == "Power" {
                    "HeatRate".to_owned()
                } else {
                    info.quantity_hint.to_owned()
                }
            })
            .unwrap_or_else(|| "DimensionlessNumber".to_owned());
        return Some(RuntimeFormatValue::Number {
            value,
            quantity_kind,
            unit,
        });
    }
    if let Some(arg_name) = expression.strip_prefix("args.") {
        return report
            .semantic_program
            .arg_values
            .iter()
            .find(|arg| arg.name == arg_name)
            .map(|arg| RuntimeFormatValue::Text(arg.value.clone()));
    }
    if let Some(table_name) = expression.strip_suffix(".rows") {
        return runtime_data
            .tables
            .iter()
            .find(|table| table.binding == table_name.trim())
            .map(|table| RuntimeFormatValue::Number {
                value: table.row_count as f64,
                quantity_kind: "Count".to_owned(),
                unit: String::new(),
            });
    }
    if let Some(value) = evaluate_coverage_expression(expression, runtime_data) {
        return Some(value);
    }
    if let Some(value) = evaluate_statistic_expression(expression, runtime_data) {
        return Some(value);
    }
    if let Some(value) = evaluate_function_call_expression(expression, report, runtime_data) {
        return Some(value);
    }
    if let Some(const_info) = report
        .semantic_program
        .consts
        .iter()
        .find(|const_info| const_info.name == expression)
    {
        if let Some((value, unit)) = number_with_optional_unit(&const_info.expression) {
            return Some(RuntimeFormatValue::Number {
                value,
                quantity_kind: const_info.quantity_kind.clone(),
                unit: unit.unwrap_or_else(|| const_info.display_unit.clone()),
            });
        }
        return Some(RuntimeFormatValue::Text(strip_runtime_string_value(
            &const_info.expression,
        )));
    }
    if let Some(metric) = runtime_data
        .metrics
        .iter()
        .find(|metric| metric.binding == expression)
    {
        return Some(RuntimeFormatValue::Number {
            value: metric.value,
            quantity_kind: metric.quantity_kind.clone(),
            unit: metric.unit.clone(),
        });
    }
    if let Some(integration) = runtime_data
        .integrations
        .iter()
        .find(|integration| integration.binding == expression)
    {
        return Some(RuntimeFormatValue::Number {
            value: integration.value,
            quantity_kind: integration.result_quantity.clone(),
            unit: integration.unit.clone(),
        });
    }
    if let Some(declaration) = report
        .inferred_declarations
        .iter()
        .find(|declaration| declaration.name == expression)
    {
        if let Some(value) = evaluate_statistic_expression(&declaration.expression, runtime_data) {
            return Some(value);
        }
        if let Some((value, unit)) = number_with_optional_unit(&declaration.expression) {
            return Some(RuntimeFormatValue::Number {
                value,
                quantity_kind: declaration.quantity_kind.clone(),
                unit: unit.unwrap_or_else(|| declaration.display_unit.clone()),
            });
        }
        if let Some(value) =
            evaluate_function_call_expression(&declaration.expression, report, runtime_data)
        {
            return Some(value);
        }
        if let Some(selection) = runtime_data
            .table_selections
            .iter()
            .find(|selection| selection.binding == declaration.name)
        {
            return Some(RuntimeFormatValue::Text(
                selection.selected_value.clone().unwrap_or_default(),
            ));
        }
        if declaration.expression.trim() != expression {
            if let Some(value) =
                evaluate_runtime_expression(&declaration.expression, report, runtime_data)
            {
                return Some(value);
            }
        }
    }
    if let Some(table) = runtime_data
        .tables
        .iter()
        .find(|table| table.binding == expression)
    {
        return Some(RuntimeFormatValue::Summary(format!(
            "Table {}: {} rows, {} columns",
            table.binding,
            table.row_count,
            table.columns.len()
        )));
    }
    if let Some(series) = runtime_data
        .time_series
        .iter()
        .find(|series| series.name == expression)
    {
        return Some(RuntimeFormatValue::Summary(format!(
            "TimeSeries {}: {} points over {}, {} [{}]",
            series.name,
            series.points.len(),
            series.axis,
            series.quantity_kind,
            series.display_unit
        )));
    }
    None
}

fn evaluate_coverage_expression(
    expression: &str,
    runtime_data: &RuntimeData,
) -> Option<RuntimeFormatValue> {
    let (binding, field) = expression.trim().split_once('.')?;
    let coverage = runtime_data
        .timeseries_coverage
        .iter()
        .find(|coverage| coverage.binding == binding.trim() || coverage.name == binding.trim())?;
    match field.trim() {
        "complete" => Some(RuntimeFormatValue::Text(
            (coverage.status == "complete").to_string(),
        )),
        "status" => Some(RuntimeFormatValue::Text(coverage.status.clone())),
        "missing_count" => Some(RuntimeFormatValue::Number {
            value: coverage.missing_count as f64,
            quantity_kind: "Count".to_owned(),
            unit: String::new(),
        }),
        "actual_count" => Some(RuntimeFormatValue::Number {
            value: coverage.actual_count as f64,
            quantity_kind: "Count".to_owned(),
            unit: String::new(),
        }),
        "expected_count" => coverage
            .expected_count
            .map(|count| RuntimeFormatValue::Number {
                value: count as f64,
                quantity_kind: "Count".to_owned(),
                unit: String::new(),
            }),
        "max_gap" => coverage.max_gap.map(|value| RuntimeFormatValue::Number {
            value,
            quantity_kind: "Duration".to_owned(),
            unit: "s".to_owned(),
        }),
        "max_gap_hours" => coverage.max_gap.map(|value| RuntimeFormatValue::Number {
            value: value / 3600.0,
            quantity_kind: "Duration".to_owned(),
            unit: "h".to_owned(),
        }),
        "expected_step" => coverage
            .expected_step
            .map(|value| RuntimeFormatValue::Number {
                value,
                quantity_kind: "Duration".to_owned(),
                unit: "s".to_owned(),
            }),
        "year" | "coverage_year" => coverage
            .coverage_year
            .map(|year| RuntimeFormatValue::Number {
                value: year as f64,
                quantity_kind: "DimensionlessNumber".to_owned(),
                unit: String::new(),
            }),
        "leap_year_policy" => Some(RuntimeFormatValue::Text(coverage.leap_year_policy.clone())),
        _ => None,
    }
}

fn evaluate_runtime_read_expression(expression: &str, report: &CheckReport) -> Option<String> {
    let (_kind, path_expression) = eng_compiler::read_only_io_expression(expression)?;
    let path_text = evaluate_runtime_path_expression(path_expression, report)?;
    let path = runtime_resolve_source_relative_path(&path_text, report.source_path.parent());
    fs::read_to_string(path).ok()
}

fn evaluate_runtime_exists_expression(expression: &str, report: &CheckReport) -> Option<String> {
    let expression = expression.trim();
    let inner = if let Some(inner) = expression.strip_prefix("exists ") {
        inner.trim()
    } else {
        runtime_strip_call_inner(expression, "exists")?
    };
    let path_text = evaluate_runtime_path_expression(inner, report)?;
    let path = runtime_resolve_source_relative_path(&path_text, report.source_path.parent());
    Some(path.exists().to_string())
}

fn evaluate_runtime_path_expression(expression: &str, report: &CheckReport) -> Option<String> {
    let expression = expression.trim();
    if let Some(arg_name) = expression.strip_prefix("args.") {
        return report
            .semantic_program
            .arg_values
            .iter()
            .find(|arg| arg.name == arg_name.trim())
            .map(|arg| arg.value.clone());
    }
    if let Some(value) = runtime_strip_call_string_arg(expression, "file") {
        return Some(value);
    }
    if let Some(value) = runtime_strip_call_string_arg(expression, "dir") {
        return Some(value);
    }
    if expression.starts_with('"') {
        return Some(strip_runtime_string_value(expression));
    }
    if let Some(inner) = runtime_strip_call_inner(expression, "join") {
        let parts = split_top_level(inner, &[','])
            .into_iter()
            .map(|part| evaluate_runtime_path_expression(&part, report))
            .collect::<Option<Vec<_>>>()?;
        if parts.is_empty() {
            return None;
        }
        return Some(runtime_join_path_text(&parts));
    }
    if let Some(inner) = runtime_strip_call_inner(expression, "parent") {
        let path = evaluate_runtime_path_expression(inner, report)?;
        return Some(runtime_parent_path_text(&path));
    }
    if let Some(inner) = runtime_strip_call_inner(expression, "stem") {
        let path = evaluate_runtime_path_expression(inner, report)?;
        return Some(
            Path::new(&path)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_owned(),
        );
    }
    if let Some(inner) = runtime_strip_call_inner(expression, "extension") {
        let path = evaluate_runtime_path_expression(inner, report)?;
        return Some(
            Path::new(&path)
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_owned(),
        );
    }
    None
}

fn runtime_strip_call_inner<'a>(expression: &'a str, function_name: &str) -> Option<&'a str> {
    let trimmed = expression.trim();
    let prefix = format!("{function_name}(");
    trimmed
        .strip_prefix(&prefix)?
        .strip_suffix(')')
        .map(str::trim)
}

fn runtime_strip_call_string_arg(expression: &str, function_name: &str) -> Option<String> {
    let inner = runtime_strip_call_inner(expression, function_name)?;
    Some(strip_runtime_string_value(inner))
}

fn runtime_join_path_text(parts: &[String]) -> String {
    let mut joined = String::new();
    for part in parts {
        let normalized = runtime_path_text(part);
        let trimmed = normalized.trim_matches('/');
        if trimmed.is_empty() {
            continue;
        }
        if !joined.is_empty() {
            joined.push('/');
        }
        joined.push_str(trimmed);
    }
    joined
}

fn runtime_parent_path_text(path: &str) -> String {
    Path::new(path)
        .parent()
        .and_then(|value| value.to_str())
        .map(runtime_path_text)
        .unwrap_or_default()
}

fn runtime_path_text(path: impl AsRef<str>) -> String {
    path.as_ref().replace('\\', "/")
}

fn runtime_resolve_source_relative_path(path: &str, source_base: Option<&Path>) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    source_base.unwrap_or_else(|| Path::new(".")).join(path)
}

fn evaluate_function_call_expression(
    expression: &str,
    report: &CheckReport,
    runtime_data: &RuntimeData,
) -> Option<RuntimeFormatValue> {
    let call = parse_runtime_function_call(expression)?;
    let function = report
        .semantic_program
        .functions
        .iter()
        .find(|function| function.name == call.name)?;
    if call.args.len() != function.parameters.len() {
        return None;
    }
    let mut values = HashMap::new();
    for const_info in &report.semantic_program.consts {
        if !const_info.importable {
            continue;
        }
        if let Some((value, _unit)) = number_with_optional_unit(&const_info.expression) {
            values.insert(const_info.name.clone(), value);
        }
    }
    for (arg, parameter) in call.args.iter().zip(&function.parameters) {
        let RuntimeFormatValue::Number { value, .. } =
            evaluate_runtime_expression(arg, report, runtime_data)?
        else {
            return None;
        };
        values.insert(parameter.name.clone(), value);
    }
    for local in &function.locals {
        let value = evaluate_numeric_function_expression(&local.expression, &values)?;
        values.insert(local.name.clone(), value);
    }
    let body = function.return_expression.as_deref()?;
    let value = evaluate_numeric_function_expression(body, &values)?;
    Some(RuntimeFormatValue::Number {
        value,
        quantity_kind: function.return_quantity_kind.clone(),
        unit: function.return_canonical_unit.clone(),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RuntimeFunctionCall {
    name: String,
    args: Vec<String>,
}

fn parse_runtime_function_call(expression: &str) -> Option<RuntimeFunctionCall> {
    let expression = strip_outer_parens(expression.trim());
    let open = expression.find('(')?;
    if !expression.ends_with(')') {
        return None;
    }
    let name = expression[..open].trim();
    if !is_identifier(name) {
        return None;
    }
    let args_text = &expression[open + 1..expression.len() - 1];
    let args = if args_text.trim().is_empty() {
        Vec::new()
    } else {
        split_top_level(args_text, &[','])
    };
    Some(RuntimeFunctionCall {
        name: name.to_owned(),
        args,
    })
}

fn evaluate_numeric_function_expression(
    expression: &str,
    values: &HashMap<String, f64>,
) -> Option<f64> {
    let expression = strip_outer_parens(expression.trim());
    if let Some(value) = values.get(expression) {
        return Some(*value);
    }
    if let Some((value, _unit)) = number_with_optional_unit(expression) {
        return Some(value);
    }
    let terms = split_top_level(expression, &['+', '-']);
    if terms.len() > 1 {
        return evaluate_additive_numeric_expression(expression, values);
    }
    let factors = split_top_level(expression, &['*']);
    if factors.len() > 1 {
        let mut product = 1.0;
        for factor in factors {
            product *= evaluate_numeric_function_expression(&factor, values)?;
        }
        return Some(product);
    }
    None
}

fn evaluate_additive_numeric_expression(
    expression: &str,
    values: &HashMap<String, f64>,
) -> Option<f64> {
    let mut sum = 0.0;
    let mut start = 0usize;
    let mut sign = 1.0;
    let mut depth = 0i32;
    for (index, character) in expression.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            '+' | '-' if depth == 0 && index > 0 => {
                let term = expression[start..index].trim();
                if !term.is_empty() {
                    sum += sign * evaluate_numeric_function_expression(term, values)?;
                }
                sign = if character == '-' { -1.0 } else { 1.0 };
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    let term = expression[start..].trim();
    if !term.is_empty() {
        sum += sign * evaluate_numeric_function_expression(term, values)?;
    }
    Some(sum)
}

fn evaluate_statistic_expression(
    expression: &str,
    runtime_data: &RuntimeData,
) -> Option<RuntimeFormatValue> {
    let (statistic, source) = parse_statistic_expression(expression)?;
    let series = runtime_data
        .time_series
        .iter()
        .find(|series| series.name == source)?;
    let value = runtime_statistic_value(&statistic, series)?;
    Some(RuntimeFormatValue::Number {
        value: value.value,
        quantity_kind: series.quantity_kind.clone(),
        unit: value.unit,
    })
}

fn parse_statistic_expression(expression: &str) -> Option<(String, String)> {
    let trimmed = expression.trim();
    let open = trimmed.find('(')?;
    let statistic = trimmed[..open].trim();
    if !matches!(
        statistic,
        "mean" | "time_weighted_mean" | "max" | "min" | "median" | "std"
    ) && !statistic.starts_with('p')
    {
        return None;
    }
    let rest = trimmed[open + 1..].trim();
    let source = rest
        .split([',', ')'])
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some((statistic.to_owned(), source.to_owned()))
}

fn runtime_statistic_value(
    name: &str,
    series: &RuntimeTimeSeries,
) -> Option<RuntimeStatisticValue> {
    let values = series
        .points
        .iter()
        .map(|point| point.y)
        .collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    let value = match name {
        "mean" => values.iter().sum::<f64>() / values.len() as f64,
        "time_weighted_mean" => time_weighted_mean(series)?,
        "max" => values.iter().copied().reduce(f64::max)?,
        "min" => values.iter().copied().reduce(f64::min)?,
        "median" => median(&values)?,
        "std" => population_std(&values)?,
        percentile if percentile_fraction(percentile).is_some() => {
            nearest_rank_percentile(&values, percentile_fraction(percentile)?)?
        }
        _ => return None,
    };
    Some(RuntimeStatisticValue {
        name: name.to_owned(),
        value,
        unit: series.display_unit.clone(),
    })
}

fn time_weighted_mean(series: &RuntimeTimeSeries) -> Option<f64> {
    let total_duration = series.points.last()?.x - series.points.first()?.x;
    if series.x_unit != "s" || total_duration <= 0.0 {
        return None;
    }
    Some(trapezoidal_integral(series)? / total_duration)
}

fn trapezoidal_integral(series: &RuntimeTimeSeries) -> Option<f64> {
    if series.x_unit != "s" || series.points.len() < 2 {
        return None;
    }
    let mut integral = 0.0;
    for window in series.points.windows(2) {
        let dt = window[1].x - window[0].x;
        if dt <= 0.0 {
            return None;
        }
        integral += (window[0].y + window[1].y) * 0.5 * dt;
    }
    Some(integral)
}

fn median(values: &[f64]) -> Option<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let len = sorted.len();
    if len == 0 {
        return None;
    }
    if len % 2 == 1 {
        sorted.get(len / 2).copied()
    } else {
        Some((sorted[len / 2 - 1] + sorted[len / 2]) * 0.5)
    }
}

fn population_std(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>()
        / values.len() as f64;
    Some(variance.sqrt())
}

fn percentile_fraction(name: &str) -> Option<f64> {
    let value = name.strip_prefix('p')?.parse::<f64>().ok()?;
    (0.0..=100.0).contains(&value).then_some(value / 100.0)
}

fn nearest_rank_percentile(values: &[f64], fraction: f64) -> Option<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let len = sorted.len();
    if len == 0 {
        return None;
    }
    let rank = (fraction * len as f64).ceil().max(1.0) as usize;
    sorted.get(rank.saturating_sub(1).min(len - 1)).copied()
}

fn format_runtime_value(
    value: RuntimeFormatValue,
    requested_unit: Option<&str>,
    precision: Option<usize>,
    include_unit: bool,
) -> String {
    match value {
        RuntimeFormatValue::Text(value) | RuntimeFormatValue::Summary(value) => value,
        RuntimeFormatValue::Number {
            value,
            quantity_kind,
            unit,
        } => {
            let display_unit = requested_unit.unwrap_or(unit.as_str());
            let converted = if display_unit.is_empty() {
                value
            } else {
                convert_between_units(value, &unit, display_unit, &quantity_kind).unwrap_or(value)
            };
            let mut text = format_number_with_precision(converted, precision);
            if include_unit && !display_unit.is_empty() && display_unit != "count" {
                text.push(' ');
                text.push_str(display_unit);
            }
            text
        }
    }
}

fn format_number_with_precision(value: f64, precision: Option<usize>) -> String {
    if let Some(precision) = precision {
        return format!("{value:.precision$}", precision = precision);
    }
    let mut text = format!("{value:.6}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn convert_between_units(
    value: f64,
    from_unit: &str,
    to_unit: &str,
    quantity_kind: &str,
) -> Option<f64> {
    let from_unit = if from_unit.is_empty() {
        to_unit
    } else {
        from_unit
    };
    let normalized_from = eng_compiler::normalize_unit(from_unit);
    let normalized_to = eng_compiler::normalize_unit(to_unit);
    if normalized_from == normalized_to {
        return Some(value);
    }
    let from_info = unit_info(from_unit)?;
    let to_info = unit_info(to_unit)?;
    if eng_compiler::normalize_unit(from_info.canonical_unit)
        != eng_compiler::normalize_unit(to_info.canonical_unit)
        || !runtime_unit_seed_matches_quantity(from_info.quantity_hint, quantity_kind)
        || !runtime_unit_seed_matches_quantity(to_info.quantity_hint, quantity_kind)
    {
        return None;
    }
    let scale_from = from_info.scale_to_canonical.parse::<f64>().ok()?;
    let offset_from = from_info
        .affine_offset
        .and_then(|offset| offset.parse::<f64>().ok())
        .unwrap_or(0.0);
    let canonical = value * scale_from + offset_from;
    let scale_to = to_info.scale_to_canonical.parse::<f64>().ok()?;
    let offset_to = to_info
        .affine_offset
        .and_then(|offset| offset.parse::<f64>().ok())
        .unwrap_or(0.0);
    Some((canonical - offset_to) / scale_to)
}

fn unit_info(unit: &str) -> Option<eng_compiler::UnitInfo> {
    let normalized = eng_compiler::normalize_unit(unit);
    eng_compiler::all_unit_infos()
        .iter()
        .find(|info| eng_compiler::normalize_unit(info.symbol) == normalized)
        .copied()
}

fn runtime_unit_seed_matches_quantity(seed_quantity: &str, quantity_kind: &str) -> bool {
    seed_quantity == quantity_kind
        || seed_quantity == "Power"
            && matches!(
                quantity_kind,
                "HeatRate" | "ElectricPower" | "MechanicalPower"
            )
        || seed_quantity == "TemperatureDelta" && quantity_kind == "AbsoluteTemperature"
}

fn number_with_optional_unit(text: &str) -> Option<(f64, Option<String>)> {
    let mut words = text.split_whitespace();
    let value = words.next()?.parse::<f64>().ok()?;
    let unit = words.next().map(str::to_owned);
    Some((value, unit))
}

fn strip_runtime_string_value(text: &str) -> String {
    let trimmed = text.trim();
    for function_name in ["file", "dir"] {
        let prefix = format!("{function_name}(");
        if let Some(inner) = trimmed
            .strip_prefix(&prefix)
            .and_then(|value| value.strip_suffix(')'))
        {
            return strip_runtime_string_value(inner);
        }
    }
    if let Some(inner) = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        inner.to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn strip_outer_parens(mut expression: &str) -> &str {
    loop {
        let trimmed = expression.trim();
        if !(trimmed.starts_with('(') && trimmed.ends_with(')')) {
            return trimmed;
        }
        let inner = &trimmed[1..trimmed.len() - 1];
        if !is_balanced(inner) {
            return trimmed;
        }
        expression = inner;
    }
}

fn is_balanced(expression: &str) -> bool {
    let mut depth = 0i32;
    for character in expression.chars() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

fn split_top_level(expression: &str, operators: &[char]) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;

    for (index, character) in expression.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            other if depth == 0 && operators.contains(&other) => {
                if index == 0 {
                    continue;
                }
                let part = expression[start..index].trim();
                if !part.is_empty() {
                    parts.push(part.to_owned());
                }
                start = index + other.len_utf8();
            }
            _ => {}
        }
    }

    let tail = expression[start..].trim();
    if !tail.is_empty() {
        parts.push(tail.to_owned());
    }
    parts
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn result_json(
    path: &Path,
    report: &CheckReport,
    execution: &VmExecution,
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
    hashes: &ResultArtifactHashes<'_>,
    profile_context: &ProfileContext<'_>,
) -> String {
    let mut data_hashes = String::new();
    for (index, promotion) in report.semantic_program.csv_promotions.iter().enumerate() {
        if index > 0 {
            data_hashes.push_str(",\n");
        }
        data_hashes.push_str("      {\n");
        data_hashes.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&promotion.binding)
        ));
        data_hashes.push_str(&format!(
            "        \"source\": \"{}\",\n",
            json_escape(&promotion.source_literal)
        ));
        data_hashes.push_str(&format!(
            "        \"source_value\": \"{}\",\n",
            json_escape(&promotion.source_value)
        ));
        if let Some(hash) = &promotion.source_hash {
            data_hashes.push_str(&format!("        \"hash\": \"{}\"\n", json_escape(hash)));
        } else {
            data_hashes.push_str("        \"hash\": null\n");
        }
        data_hashes.push_str("      }");
    }

    let mut args_schema = String::new();
    for (args_index, args_block) in report.semantic_program.args_blocks.iter().enumerate() {
        if args_index > 0 {
            args_schema.push_str(",\n");
        }
        args_schema.push_str("    {\n");
        args_schema.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&args_block.name)
        ));
        args_schema.push_str(&format!("      \"line\": {},\n", args_block.line));
        args_schema.push_str(&format!(
            "      \"field_count\": {},\n",
            args_block.fields.len()
        ));
        args_schema.push_str("      \"fields\": [\n");
        for (field_index, field) in args_block.fields.iter().enumerate() {
            if field_index > 0 {
                args_schema.push_str(",\n");
            }
            args_schema.push_str("        {\n");
            args_schema.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&field.name)
            ));
            args_schema.push_str(&format!(
                "          \"type\": \"{}\",\n",
                json_escape(&field.type_name)
            ));
            if let Some(default_value) = &field.default_value {
                args_schema.push_str(&format!(
                    "          \"default\": \"{}\",\n",
                    json_escape(default_value)
                ));
            } else {
                args_schema.push_str("          \"default\": null,\n");
            }
            args_schema.push_str(&format!("          \"required\": {},\n", field.required));
            args_schema.push_str(&format!("          \"line\": {}\n", field.line));
            args_schema.push_str("        }");
        }
        args_schema.push_str("\n      ]\n");
        args_schema.push_str("    }");
    }
    let mut arg_values = String::new();
    for (index, arg) in report.semantic_program.arg_values.iter().enumerate() {
        if index > 0 {
            arg_values.push_str(",\n");
        }
        arg_values.push_str("    {\n");
        arg_values.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&arg.name)
        ));
        arg_values.push_str(&format!(
            "      \"type\": \"{}\",\n",
            json_escape(&arg.type_name)
        ));
        arg_values.push_str(&format!(
            "      \"value\": \"{}\",\n",
            json_escape(&arg.value)
        ));
        arg_values.push_str(&format!(
            "      \"source\": \"{}\",\n",
            json_escape(&arg.source)
        ));
        arg_values.push_str(&format!("      \"required\": {},\n", arg.required));
        arg_values.push_str(&format!("      \"line\": {}\n", arg.line));
        arg_values.push_str("    }");
    }

    let mut environment_dependencies = String::new();
    for (index, dependency) in report
        .semantic_program
        .environment_dependencies
        .iter()
        .enumerate()
    {
        if index > 0 {
            environment_dependencies.push_str(",\n");
        }
        environment_dependencies.push_str("      {\n");
        environment_dependencies.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&dependency.name)
        ));
        environment_dependencies.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&dependency.kind)
        ));
        environment_dependencies.push_str(&format!(
            "        \"expression\": \"{}\",\n",
            json_escape(&dependency.expression)
        ));
        environment_dependencies.push_str(&format!(
            "        \"resolved_value\": \"{}\",\n",
            json_escape(&dependency.resolved_value)
        ));
        match &dependency.source_hash {
            Some(source_hash) => environment_dependencies.push_str(&format!(
                "        \"source_hash\": \"{}\",\n",
                json_escape(source_hash)
            )),
            None => environment_dependencies.push_str("        \"source_hash\": null,\n"),
        }
        environment_dependencies.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&dependency.status)
        ));
        environment_dependencies.push_str(&format!("        \"line\": {}\n", dependency.line));
        environment_dependencies.push_str("      }");
    }

    let mut profile_diagnostics_json = String::new();
    push_profile_diagnostics_json(
        &mut profile_diagnostics_json,
        profile_context.diagnostics,
        "      ",
    );

    let mut objects = String::new();
    for (index, object) in execution.objects.iter().enumerate() {
        if index > 0 {
            objects.push_str(",\n");
        }
        objects.push_str("      {\n");
        objects.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&object.name)
        ));
        objects.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            vm_object_kind(object)
        ));
        objects.push_str(&format!(
            "        \"type\": \"{}\"",
            json_escape(&object.type_name)
        ));
        if let Some(axis) = &object.axis {
            objects.push_str(&format!(",\n        \"axis\": \"{}\"", json_escape(axis)));
        }
        if let Some(display_unit) = &object.display_unit {
            objects.push_str(&format!(
                ",\n        \"display_unit\": \"{}\"",
                json_escape(display_unit)
            ));
        }
        if let Some(row_count) = object.row_count {
            objects.push_str(&format!(",\n        \"row_count\": {row_count}"));
        }
        if let Some(len) = object.len {
            objects.push_str(&format!(",\n        \"len\": {len}"));
        }
        if let Some(source_hash) = &object.source_hash {
            objects.push_str(&format!(
                ",\n        \"source_hash\": \"{}\"",
                json_escape(source_hash)
            ));
        }
        if let Some(numeric) = runtime_data
            .numeric_values
            .iter()
            .find(|numeric| numeric.binding == object.name)
        {
            objects.push_str(",\n        \"numeric\": ");
            push_runtime_numeric_link(&mut objects, numeric, "        ");
        }
        if let Some(table) = runtime_data
            .tables
            .iter()
            .find(|table| table.binding == object.name)
        {
            objects.push_str(",\n        \"columns\": [\n");
            push_runtime_columns(&mut objects, table);
            objects.push_str("\n        ],\n        \"parse_failures\": [\n");
            push_parse_failures(&mut objects, table);
            objects.push_str("\n        ]");
        }
        if let Some(series) = runtime_data
            .time_series
            .iter()
            .find(|series| series.name == object.name)
        {
            objects.push_str(&format!(
                ",\n        \"source_table\": \"{}\"",
                json_escape(&series.source_table)
            ));
            objects.push_str(&format!(
                ",\n        \"source_expression\": \"{}\"",
                json_escape(&series.source_expression)
            ));
            objects.push_str(",\n        \"points\": [");
            push_runtime_points(&mut objects, &series.points);
            objects.push(']');
        }
        objects.push_str("\n      }");
    }

    let mut numeric_values = String::new();
    for (index, numeric) in runtime_data.numeric_values.iter().enumerate() {
        if index > 0 {
            numeric_values.push_str(",\n");
        }
        numeric_values.push_str("      {\n");
        numeric_values.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&numeric.binding)
        ));
        numeric_values.push_str(&format!(
            "        \"value_kind\": \"{}\",\n",
            json_escape(&numeric.value_kind)
        ));
        numeric_values.push_str(&format!(
            "        \"quantity_kind\": \"{}\",\n",
            json_escape(&numeric.quantity_kind)
        ));
        numeric_values.push_str(&format!(
            "        \"display_unit\": \"{}\",\n",
            json_escape(&numeric.display_unit)
        ));
        numeric_values.push_str(&format!(
            "        \"representation\": \"{}\",\n",
            json_escape(&numeric.representation)
        ));
        push_optional_json_number(&mut numeric_values, "value", numeric.value, 8);
        numeric_values.push_str("        \"uncertainty\": ");
        match &numeric.uncertainty {
            Some(uncertainty) => {
                push_runtime_numeric_uncertainty(&mut numeric_values, uncertainty, "        ");
                numeric_values.push_str(",\n");
            }
            None => numeric_values.push_str("null,\n"),
        }
        numeric_values.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&numeric.status)
        ));
        numeric_values.push_str(&format!("        \"line\": {}\n", numeric.line));
        numeric_values.push_str("      }");
    }

    let mut steps = String::new();
    for (index, step) in execution.steps.iter().enumerate() {
        if index > 0 {
            steps.push_str(", ");
        }
        steps.push_str(&format!("\"{}\"", json_escape(step)));
    }

    let mut statistics = String::new();
    for (index, stats) in report.semantic_program.stats_infos.iter().enumerate() {
        if index > 0 {
            statistics.push_str(",\n");
        }
        statistics.push_str("      {\n");
        statistics.push_str(&format!(
            "        \"source\": \"{}\",\n",
            json_escape(&stats.source)
        ));
        statistics.push_str(&format!(
            "        \"quantity_kind\": \"{}\",\n",
            json_escape(&stats.quantity_kind)
        ));
        statistics.push_str(&format!(
            "        \"axis\": \"{}\",\n",
            json_escape(&stats.axis)
        ));
        if let Some(computed) = runtime_data
            .statistics
            .iter()
            .find(|summary| summary.source == stats.source)
        {
            statistics.push_str("        \"statistics\": [\n");
            for (value_index, value) in computed.values.iter().enumerate() {
                if value_index > 0 {
                    statistics.push_str(",\n");
                }
                statistics.push_str("          {\n");
                statistics.push_str(&format!(
                    "            \"name\": \"{}\",\n",
                    json_escape(&value.name)
                ));
                statistics.push_str(&format!("            \"value\": {},\n", value.value));
                statistics.push_str(&format!(
                    "            \"unit\": \"{}\"\n",
                    json_escape(&value.unit)
                ));
                statistics.push_str("          }");
            }
            statistics.push_str("\n        ],\n");
            statistics.push_str(&format!(
                "        \"cache_key\": \"{}\",\n",
                json_escape(&computed.cache_key)
            ));
            statistics.push_str(&format!(
                "        \"status\": \"{}\"\n",
                json_escape(&computed.status)
            ));
        } else {
            statistics.push_str("        \"statistics\": [");
            for (stat_index, statistic) in stats.statistics.iter().enumerate() {
                if stat_index > 0 {
                    statistics.push_str(", ");
                }
                statistics.push_str(&format!("\"{}\"", json_escape(statistic)));
            }
            statistics.push_str("],\n");
            statistics.push_str(&format!(
                "        \"cache_key\": \"{}\",\n",
                json_escape(&stats.cache_key)
            ));
            statistics.push_str("        \"status\": \"lazy\"\n");
        }
        statistics.push_str("      }");
    }

    let mut integrations = String::new();
    for (index, integration) in report.semantic_program.integrations.iter().enumerate() {
        if index > 0 {
            integrations.push_str(",\n");
        }
        integrations.push_str("      {\n");
        integrations.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&integration.binding)
        ));
        integrations.push_str(&format!(
            "        \"source\": \"{}\",\n",
            json_escape(&integration.source)
        ));
        integrations.push_str(&format!(
            "        \"input_quantity\": \"{}\",\n",
            json_escape(&integration.input_quantity)
        ));
        integrations.push_str(&format!(
            "        \"over_axis\": \"{}\",\n",
            json_escape(&integration.over_axis)
        ));
        integrations.push_str(&format!(
            "        \"result_quantity\": \"{}\"",
            json_escape(&integration.result_quantity)
        ));
        if let Some(computed) = runtime_data
            .integrations
            .iter()
            .find(|computed| computed.binding == integration.binding)
        {
            integrations.push_str(&format!(",\n        \"value\": {}", computed.value));
            integrations.push_str(&format!(
                ",\n        \"unit\": \"{}\"",
                json_escape(&computed.unit)
            ));
            integrations.push_str(&format!(
                ",\n        \"method\": \"{}\"",
                json_escape(&computed.method)
            ));
            integrations.push_str(&format!(
                ",\n        \"interval_count\": {}",
                computed.interval_count
            ));
            integrations.push_str(&format!(
                ",\n        \"status\": \"{}\"",
                json_escape(&computed.status)
            ));
        }
        integrations.push('\n');
        integrations.push_str("      }");
    }

    let table_diagnostics = table_diagnostics_json(runtime_data);
    let structured_reads = structured_reads_json(runtime_data);
    let table_selections = table_selections_json(runtime_data, "      ");
    let timeseries_coverage = timeseries_coverage_json(runtime_data, "      ");
    let sample_tables = sample_tables_json(runtime_data);
    let case_manifests = case_manifests_json(runtime_data, process_results);
    let db_manifest_records = db_manifest_records(process_results);
    let db_manifests = db_manifests_json(&db_manifest_records);
    let model_cards = model_cards_json(runtime_data);

    let mut timeseries_uncertainty_calculations = String::new();
    for (index, calculation) in runtime_data
        .timeseries_uncertainty_calculations
        .iter()
        .enumerate()
    {
        if index > 0 {
            timeseries_uncertainty_calculations.push_str(",\n");
        }
        timeseries_uncertainty_calculations.push_str("      {\n");
        timeseries_uncertainty_calculations.push_str(&format!(
            "        \"source\": \"{}\",\n",
            json_escape(&calculation.source)
        ));
        timeseries_uncertainty_calculations.push_str(&format!(
            "        \"operation\": \"{}\",\n",
            json_escape(&calculation.operation)
        ));
        push_optional_json_string(
            &mut timeseries_uncertainty_calculations,
            "statistic",
            calculation.statistic.as_deref(),
            8,
        );
        push_optional_json_string(
            &mut timeseries_uncertainty_calculations,
            "binding",
            calculation.binding.as_deref(),
            8,
        );
        push_optional_json_number(
            &mut timeseries_uncertainty_calculations,
            "nominal_value",
            calculation.nominal_value,
            8,
        );
        push_optional_json_number(
            &mut timeseries_uncertainty_calculations,
            "stddev",
            calculation.stddev,
            8,
        );
        timeseries_uncertainty_calculations.push_str(&format!(
            "        \"unit\": \"{}\",\n",
            json_escape(&calculation.unit)
        ));
        timeseries_uncertainty_calculations.push_str(&format!(
            "        \"sensor_std\": {},\n",
            calculation.sensor_std
        ));
        timeseries_uncertainty_calculations.push_str(&format!(
            "        \"sensor_std_unit\": \"{}\",\n",
            json_escape(&calculation.sensor_std_unit)
        ));
        timeseries_uncertainty_calculations.push_str(&format!(
            "        \"method\": \"{}\",\n",
            json_escape(&calculation.method)
        ));
        timeseries_uncertainty_calculations.push_str(&format!(
            "        \"status\": \"{}\"\n",
            json_escape(&calculation.status)
        ));
        timeseries_uncertainty_calculations.push_str("      }");
    }

    let mut uncertainties = String::new();
    for (index, uncertainty) in runtime_data.uncertainties.iter().enumerate() {
        if index > 0 {
            uncertainties.push_str(",\n");
        }
        uncertainties.push_str("      {\n");
        uncertainties.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&uncertainty.binding)
        ));
        uncertainties.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&uncertainty.kind)
        ));
        uncertainties.push_str(&format!(
            "        \"quantity_kind\": \"{}\",\n",
            json_escape(&uncertainty.quantity_kind)
        ));
        uncertainties.push_str(&format!(
            "        \"display_unit\": \"{}\",\n",
            json_escape(&uncertainty.display_unit)
        ));
        uncertainties.push_str(&format!(
            "        \"expression\": \"{}\",\n",
            json_escape(&uncertainty.expression)
        ));
        if let Some(source) = &uncertainty.source {
            uncertainties.push_str(&format!(
                "        \"source\": \"{}\",\n",
                json_escape(source)
            ));
        } else {
            uncertainties.push_str("        \"source\": null,\n");
        }
        push_optional_json_string(
            &mut uncertainties,
            "distribution",
            uncertainty.distribution.as_deref(),
            8,
        );
        push_optional_json_string(
            &mut uncertainties,
            "method",
            uncertainty.method.as_deref(),
            8,
        );
        push_optional_json_number(&mut uncertainties, "scale", uncertainty.scale, 8);
        push_optional_json_number(&mut uncertainties, "offset", uncertainty.offset, 8);
        push_optional_json_number(&mut uncertainties, "mean", uncertainty.mean, 8);
        push_optional_json_number(&mut uncertainties, "stddev", uncertainty.stddev, 8);
        push_optional_json_string(&mut uncertainties, "error", uncertainty.error.as_deref(), 8);
        push_optional_json_number(&mut uncertainties, "lower", uncertainty.lower, 8);
        push_optional_json_number(&mut uncertainties, "upper", uncertainty.upper, 8);
        push_optional_json_number(&mut uncertainties, "p05", uncertainty.p05, 8);
        push_optional_json_number(&mut uncertainties, "p50", uncertainty.p50, 8);
        push_optional_json_number(&mut uncertainties, "p95", uncertainty.p95, 8);
        uncertainties.push_str(&format!(
            "        \"sample_count\": {},\n",
            uncertainty.sample_count
        ));
        uncertainties.push_str(&format!(
            "        \"propagation_count\": {},\n",
            uncertainty.propagation_count
        ));
        uncertainties.push_str("        \"propagation\": [");
        push_uncertainty_propagation_terms(&mut uncertainties, &uncertainty.propagation);
        uncertainties.push_str("],\n");
        uncertainties.push_str("        \"samples\": [");
        for (sample_index, sample) in uncertainty.samples.iter().enumerate() {
            if sample_index > 0 {
                uncertainties.push_str(", ");
            }
            uncertainties.push_str(&sample.to_string());
        }
        uncertainties.push_str("],\n");
        uncertainties.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&uncertainty.status)
        ));
        uncertainties.push_str(&format!("        \"line\": {}\n", uncertainty.line));
        uncertainties.push_str("      }");
    }

    let mut ml = String::new();
    for (index, artifact) in runtime_data.ml_artifacts.iter().enumerate() {
        if index > 0 {
            ml.push_str(",\n");
        }
        ml.push_str("      {\n");
        ml.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&artifact.binding)
        ));
        ml.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&artifact.kind)
        ));
        push_optional_json_string(&mut ml, "source", artifact.source.as_deref(), 8);
        push_optional_json_string(&mut ml, "target", artifact.target.as_deref(), 8);
        push_optional_json_string(
            &mut ml,
            "target_quantity",
            artifact.target_quantity.as_deref(),
            8,
        );
        ml.push_str(&format!(
            "        \"target_unit\": \"{}\",\n",
            json_escape(&artifact.display_unit)
        ));
        ml.push_str("        \"features\": [");
        push_json_string_array(&mut ml, &artifact.features);
        ml.push_str("],\n");
        push_optional_json_string(&mut ml, "algorithm", artifact.algorithm.as_deref(), 8);
        push_optional_json_string(
            &mut ml,
            "test_fraction",
            artifact.test_fraction.as_deref(),
            8,
        );
        push_optional_json_string(&mut ml, "seed", artifact.seed.as_deref(), 8);
        ml.push_str("        \"hidden_layers\": [");
        for (layer_index, layer) in artifact.hidden_layers.iter().enumerate() {
            if layer_index > 0 {
                ml.push_str(", ");
            }
            ml.push_str(&layer.to_string());
        }
        ml.push_str("],\n");
        push_optional_json_usize(&mut ml, "epochs", artifact.epochs, 8);
        ml.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&artifact.status)
        ));
        push_optional_json_usize(&mut ml, "train_count", artifact.train_count, 8);
        push_optional_json_usize(&mut ml, "test_count", artifact.test_count, 8);
        push_optional_json_number(&mut ml, "rmse", artifact.rmse, 8);
        push_optional_json_number(&mut ml, "mae", artifact.mae, 8);
        push_optional_json_number(&mut ml, "r2", artifact.r2, 8);
        push_optional_json_string(
            &mut ml,
            "leakage_status",
            artifact.leakage_status.as_deref(),
            8,
        );
        ml.push_str("        \"leakage_findings\": [");
        push_json_string_array(&mut ml, &artifact.leakage_findings);
        ml.push_str("],\n");
        ml.push_str("        \"coefficients\": [");
        for (coefficient_index, coefficient) in artifact.coefficients.iter().enumerate() {
            if coefficient_index > 0 {
                ml.push_str(", ");
            }
            ml.push_str(&format!(
                "{{\"feature\":\"{}\",\"value\":{}}}",
                json_escape(&coefficient.feature),
                coefficient.value
            ));
        }
        ml.push_str("],\n");
        push_optional_json_number(&mut ml, "intercept", artifact.intercept, 8);
        ml.push_str("        \"loss_history\": [");
        for (loss_index, loss) in artifact.loss_history.iter().enumerate() {
            if loss_index > 0 {
                ml.push_str(", ");
            }
            ml.push_str(&loss.to_string());
        }
        ml.push_str("],\n");
        push_optional_json_string(&mut ml, "model_card", artifact.model_card.as_deref(), 8);
        push_optional_json_string(
            &mut ml,
            "training_data_hash",
            artifact.training_data_hash.as_deref(),
            8,
        );
        push_optional_json_string(
            &mut ml,
            "model_artifact_hash",
            artifact.model_artifact_hash.as_deref(),
            8,
        );
        ml.push_str("        \"parity_points\": [");
        push_runtime_points(&mut ml, &artifact.parity_points);
        ml.push_str("],\n");
        ml.push_str("        \"residual_points\": [");
        push_runtime_points(&mut ml, &artifact.residual_points);
        ml.push_str("],\n");
        ml.push_str(&format!(
            "        \"expression\": \"{}\",\n",
            json_escape(&artifact.expression)
        ));
        ml.push_str(&format!("        \"line\": {}\n", artifact.line));
        ml.push_str("      }");
    }

    let mut policy_results = String::new();
    for (index, policy) in runtime_data.policy_results.iter().enumerate() {
        if index > 0 {
            policy_results.push_str(",\n");
        }
        policy_results.push_str("      {\n");
        policy_results.push_str(&format!(
            "        \"schema\": \"{}\",\n",
            json_escape(&policy.schema)
        ));
        policy_results.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&policy.binding)
        ));
        policy_results.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&policy.kind)
        ));
        policy_results.push_str(&format!(
            "        \"target\": \"{}\",\n",
            json_escape(&policy.target)
        ));
        policy_results.push_str(&format!(
            "        \"policy\": \"{}\",\n",
            json_escape(&policy.policy)
        ));
        policy_results.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&policy.status)
        ));
        policy_results.push_str(&format!(
            "        \"checked_rows\": {},\n",
            policy.checked_rows
        ));
        policy_results.push_str(&format!(
            "        \"violation_count\": {},\n",
            policy.violations.len()
        ));
        policy_results.push_str("        \"violations\": [\n");
        for (violation_index, violation) in policy.violations.iter().enumerate() {
            if violation_index > 0 {
                policy_results.push_str(",\n");
            }
            policy_results.push_str("          {\n");
            policy_results.push_str(&format!("            \"row\": {},\n", violation.row));
            policy_results.push_str(&format!(
                "            \"column\": \"{}\",\n",
                json_escape(&violation.column)
            ));
            policy_results.push_str(&format!(
                "            \"value\": \"{}\",\n",
                json_escape(&violation.value)
            ));
            policy_results.push_str(&format!(
                "            \"message\": \"{}\"\n",
                json_escape(&violation.message)
            ));
            policy_results.push_str("          }");
        }
        policy_results.push_str("\n        ],\n");
        policy_results.push_str(&format!("        \"line\": {}\n", policy.line));
        policy_results.push_str("      }");
    }

    let mut metrics = String::new();
    for (index, metric) in runtime_data.metrics.iter().enumerate() {
        if index > 0 {
            metrics.push_str(",\n");
        }
        metrics.push_str("      {\n");
        metrics.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&metric.binding)
        ));
        metrics.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&metric.kind)
        ));
        metrics.push_str(&format!(
            "        \"left\": \"{}\",\n",
            json_escape(&metric.left)
        ));
        metrics.push_str(&format!(
            "        \"right\": \"{}\",\n",
            json_escape(&metric.right)
        ));
        metrics.push_str(&format!(
            "        \"quantity_kind\": \"{}\",\n",
            json_escape(&metric.quantity_kind)
        ));
        metrics.push_str(&format!(
            "        \"unit\": \"{}\",\n",
            json_escape(&metric.unit)
        ));
        metrics.push_str(&format!("        \"value\": {},\n", metric.value));
        metrics.push_str(&format!(
            "        \"sample_count\": {},\n",
            metric.sample_count
        ));
        push_optional_json_string(
            &mut metrics,
            "alignment_reference",
            metric.alignment_reference.as_deref(),
            8,
        );
        push_optional_json_string(
            &mut metrics,
            "alignment_status",
            metric.alignment_status.as_deref(),
            8,
        );
        push_optional_json_string(
            &mut metrics,
            "alignment_step_status",
            metric.alignment_step_status.as_deref(),
            8,
        );
        metrics.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&metric.status)
        ));
        metrics.push_str(&format!("        \"line\": {}\n", metric.line));
        metrics.push_str("      }");
    }

    let mut validations = String::new();
    for (index, validation) in runtime_data.validations.iter().enumerate() {
        if index > 0 {
            validations.push_str(",\n");
        }
        validations.push_str("      {\n");
        validations.push_str(&format!(
            "        \"expression\": \"{}\",\n",
            json_escape(&validation.expression)
        ));
        validations.push_str(&format!(
            "        \"left\": \"{}\",\n",
            json_escape(&validation.left)
        ));
        validations.push_str(&format!(
            "        \"operator\": \"{}\",\n",
            json_escape(&validation.operator)
        ));
        validations.push_str(&format!(
            "        \"right\": \"{}\",\n",
            json_escape(&validation.right)
        ));
        push_optional_json_number(&mut validations, "left_value", validation.left_value, 8);
        push_optional_json_number(&mut validations, "right_value", validation.right_value, 8);
        validations.push_str(&format!(
            "        \"unit\": \"{}\",\n",
            json_escape(&validation.unit)
        ));
        validations.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&validation.status)
        ));
        validations.push_str(&format!("        \"line\": {}\n", validation.line));
        validations.push_str("      }");
    }

    let mut time_axes = String::new();
    for (index, axis) in runtime_data.time_axes.iter().enumerate() {
        if index > 0 {
            time_axes.push_str(",\n");
        }
        time_axes.push_str("      {\n");
        time_axes.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&axis.name)
        ));
        time_axes.push_str(&format!(
            "        \"source_table\": \"{}\",\n",
            json_escape(&axis.source_table)
        ));
        time_axes.push_str(&format!(
            "        \"source_column\": \"{}\",\n",
            json_escape(&axis.source_column)
        ));
        time_axes.push_str(&format!(
            "        \"axis\": \"{}\",\n",
            json_escape(&axis.axis)
        ));
        time_axes.push_str(&format!(
            "        \"unit\": \"{}\",\n",
            json_escape(&axis.unit)
        ));
        push_optional_json_number(&mut time_axes, "start", axis.start, 8);
        push_optional_json_number(&mut time_axes, "end", axis.end, 8);
        time_axes.push_str(&format!("        \"count\": {},\n", axis.count));
        push_optional_json_number(&mut time_axes, "nominal_step", axis.nominal_step, 8);
        time_axes.push_str(&format!("        \"irregular\": {},\n", axis.irregular));
        time_axes.push_str(&format!(
            "        \"missing_count\": {}\n",
            axis.missing_count
        ));
        time_axes.push_str("      }");
    }

    let mut time_alignments = String::new();
    for (index, alignment) in runtime_data.time_alignments.iter().enumerate() {
        if index > 0 {
            time_alignments.push_str(",\n");
        }
        time_alignments.push_str("      {\n");
        time_alignments.push_str(&format!(
            "        \"left\": \"{}\",\n",
            json_escape(&alignment.left)
        ));
        time_alignments.push_str(&format!(
            "        \"right\": \"{}\",\n",
            json_escape(&alignment.right)
        ));
        time_alignments.push_str(&format!(
            "        \"axis\": \"{}\",\n",
            json_escape(&alignment.axis)
        ));
        time_alignments.push_str(&format!(
            "        \"left_count\": {},\n",
            alignment.left_count
        ));
        time_alignments.push_str(&format!(
            "        \"right_count\": {},\n",
            alignment.right_count
        ));
        time_alignments.push_str(&format!(
            "        \"matched_count\": {},\n",
            alignment.matched_count
        ));
        push_optional_json_number(
            &mut time_alignments,
            "left_nominal_step",
            alignment.left_nominal_step,
            8,
        );
        push_optional_json_number(
            &mut time_alignments,
            "right_nominal_step",
            alignment.right_nominal_step,
            8,
        );
        time_alignments.push_str(&format!(
            "        \"left_irregular\": {},\n",
            alignment.left_irregular
        ));
        time_alignments.push_str(&format!(
            "        \"right_irregular\": {},\n",
            alignment.right_irregular
        ));
        time_alignments.push_str(&format!(
            "        \"step_status\": \"{}\",\n",
            json_escape(&alignment.step_status)
        ));
        push_optional_json_number(
            &mut time_alignments,
            "overlap_start",
            alignment.overlap_start,
            8,
        );
        push_optional_json_number(
            &mut time_alignments,
            "overlap_end",
            alignment.overlap_end,
            8,
        );
        time_alignments.push_str(&format!(
            "        \"status\": \"{}\"\n",
            json_escape(&alignment.status)
        ));
        time_alignments.push_str("      }");
    }

    let mut systems = String::new();
    for (system_index, system) in report.semantic_program.systems.iter().enumerate() {
        if system_index > 0 {
            systems.push_str(",\n");
        }
        systems.push_str("      {\n");
        systems.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&system.name)
        ));
        systems.push_str(&format!(
            "        \"variable_count\": {},\n",
            system.variables.len()
        ));
        systems.push_str("        \"equations\": [\n");
        for (equation_index, equation) in system.equations.iter().enumerate() {
            if equation_index > 0 {
                systems.push_str(",\n");
            }
            systems.push_str("          {\n");
            systems.push_str(&format!(
                "            \"left\": \"{}\",\n",
                json_escape(&equation.left)
            ));
            systems.push_str(&format!(
                "            \"relation\": \"{}\",\n",
                json_escape(&equation.relation)
            ));
            systems.push_str(&format!(
                "            \"right\": \"{}\",\n",
                json_escape(&equation.right)
            ));
            systems.push_str(&format!(
                "            \"left_dimension\": \"{}\",\n",
                json_escape(&equation.left_dimension)
            ));
            systems.push_str(&format!(
                "            \"right_dimension\": \"{}\",\n",
                json_escape(&equation.right_dimension)
            ));
            systems.push_str(&format!(
                "            \"residual\": \"{}\",\n",
                json_escape(&equation.residual)
            ));
            systems.push_str(&format!(
                "            \"status\": \"{}\"\n",
                json_escape(&equation.status)
            ));
            systems.push_str("          }");
        }
        systems.push_str("\n        ],\n");
        systems.push_str("        \"residuals\": [\n");
        for (residual_index, residual) in system.residuals.iter().enumerate() {
            if residual_index > 0 {
                systems.push_str(",\n");
            }
            systems.push_str("          {\n");
            systems.push_str(&format!(
                "            \"name\": \"{}\",\n",
                json_escape(&residual.name)
            ));
            systems.push_str(&format!(
                "            \"expression\": \"{}\",\n",
                json_escape(&residual.expression)
            ));
            systems.push_str(&format!(
                "            \"dimension\": \"{}\"\n",
                json_escape(&residual.dimension)
            ));
            systems.push_str("          }");
        }
        systems.push_str("\n        ]");
        let system_solutions = runtime_data
            .system_solutions
            .iter()
            .filter(|solution| solution.system == system.name)
            .collect::<Vec<_>>();
        if let Some(solution) = system_solutions.first() {
            systems.push_str(",\n        \"solver_result\": ");
            push_system_solution_json(&mut systems, solution, "        ");
        }
        if !system_solutions.is_empty() {
            systems.push_str(",\n        \"solver_results\": [\n");
            for (solution_index, solution) in system_solutions.iter().enumerate() {
                if solution_index > 0 {
                    systems.push_str(",\n");
                }
                systems.push_str("          ");
                push_system_solution_json(&mut systems, solution, "          ");
            }
            systems.push_str("\n        ]");
        }
        systems.push('\n');
        systems.push_str("      }");
    }
    let component_solutions = component_solutions_json(runtime_data);
    let solver_boundaries = solver_boundaries_json(report, runtime_data);
    let system_ir = system_ir_json(report, runtime_data);

    format!(
        "{{\n  \"format\": \"engres-v1\",\n  \"result_format_version\": 1,\n  \"runtime_version\": \"{RUNTIME_VERSION}\",\n  \"compiler_version\": \"{}\",\n  \"bytecode_version\": {},\n  \"source_path\": \"{}\",\n  \"source_hash\": \"{}\",\n  \"bytecode_hash\": \"{}\",\n  \"numeric_profile\": \"preview-f64\",\n  \"execution_profile\": \"{}\",\n  \"workflow\": {{\n    \"kind\": \"{}\",\n    \"arg_name\": \"{}\",\n    \"arg_type\": \"{}\",\n    \"return_type\": \"{}\"\n  }},\n  \"args_schema\": [\n{}\n  ],\n  \"arg_values\": [\n{}\n  ],\n  \"object_store\": {{\n    \"scalar_count\": {},\n    \"table_count\": {},\n    \"timeseries_count\": {},\n    \"array_count\": {},\n    \"objects\": [\n{}\n    ]\n  }},\n  \"typed_payload\": {{\n    \"kind\": \"{}\",\n    \"status\": \"ok\",\n    \"result_format\": \"{}\",\n    \"vm_steps\": [{}],\n    \"numeric_values\": [\n{}\n    ],\n    \"statistics\": [\n{}\n    ],\n    \"integrations\": [\n{}\n    ],\n    \"table_diagnostics\": [\n{}\n    ],\n    \"structured_reads\": [\n{}\n    ],\n    \"table_selections\": [\n{}\n    ],\n    \"sample_tables\": [\n{}\n    ],\n    \"case_manifests\": [\n{}\n    ],\n    \"db_manifests\": [\n{}\n    ],\n    \"timeseries_uncertainty_calculations\": [\n{}\n    ],\n    \"metrics\": [\n{}\n    ],\n    \"validations\": [\n{}\n    ],\n    \"time_axes\": [\n{}\n    ],\n    \"timeseries_coverage\": [\n{}\n    ],\n    \"time_alignments\": [\n{}\n    ],\n    \"uncertainties\": [\n{}\n    ],\n    \"ml\": [\n{}\n    ],\n    \"model_cards\": [\n{}\n    ],\n    \"policy_results\": [\n{}\n    ],\n    \"systems\": [\n{}\n    ],\n    \"component_solutions\": [\n{}\n    ],\n    \"solver_boundaries\": [\n{}\n    ],\n    \"system_ir\": [\n{}\n    ]\n  }},\n  \"provenance\": {{\n    \"schema_count\": {},\n    \"csv_promotion_count\": {},\n    \"system_count\": {},\n    \"equation_count\": {},\n    \"residual_count\": {},\n    \"component_solution_count\": {},\n    \"environment_dependencies\": [\n{}\n    ],\n    \"profile_diagnostics\": [\n{}\n    ],\n    \"data_hashes\": [\n{}\n    ],\n    \"unit_conversion_history\": [],\n    \"plot_spec_hash\": \"{}\",\n    \"report_spec_hash\": \"{}\",\n    \"schema_hash\": \"preview\"\n  }}\n}}\n",
        eng_compiler::COMPILER_VERSION,
        eng_compiler::BYTECODE_VERSION,
        json_escape(&path.display().to_string()),
        report.source_hash,
        hashes.bytecode,
        profile_context.profile.as_str(),
        json_escape(&execution.workflow.kind),
        json_escape(execution.workflow.arg_name.as_deref().unwrap_or("args")),
        json_escape(execution.workflow.arg_type.as_deref().unwrap_or("Args")),
        json_escape(execution.workflow.return_type.as_deref().unwrap_or("Report")),
        args_schema,
        arg_values,
        execution.scalar_count(),
        execution.table_count(),
        execution.timeseries_count(),
        execution.array_count(),
        objects,
        json_escape(execution.workflow.return_type.as_deref().unwrap_or("Report")),
        json_escape(&execution.result_format),
        steps,
        numeric_values,
        statistics,
        integrations,
        table_diagnostics,
        structured_reads,
        table_selections,
        sample_tables,
        case_manifests,
        db_manifests,
        timeseries_uncertainty_calculations,
        metrics,
        validations,
        time_axes,
        timeseries_coverage,
        time_alignments,
        uncertainties,
        ml,
        model_cards,
        policy_results,
        systems,
        component_solutions,
        solver_boundaries,
        system_ir,
        report.semantic_program.schemas.len(),
        report.semantic_program.csv_promotions.len(),
        report.semantic_program.systems.len(),
        report
            .semantic_program
            .systems
            .iter()
            .map(|system| system.equations.len())
            .sum::<usize>(),
        report
            .semantic_program
            .systems
            .iter()
            .map(|system| system.residuals.len())
            .sum::<usize>(),
        runtime_data.component_solutions.len(),
        environment_dependencies,
        profile_diagnostics_json,
        data_hashes,
        hashes.plot_spec,
        hashes.report_spec
    )
}

fn vm_object_kind(object: &VmObject) -> &'static str {
    match object.kind {
        VmObjectKind::Scalar => "scalar",
        VmObjectKind::Table => "table",
        VmObjectKind::TimeSeries => "timeseries",
        VmObjectKind::Array => "array",
    }
}

fn push_runtime_numeric_link(json: &mut String, numeric: &RuntimeNumericValue, indent: &str) {
    json.push_str("{\n");
    json.push_str(&format!(
        "{indent}  \"representation\": \"{}\",\n",
        json_escape(&numeric.representation)
    ));
    match &numeric.uncertainty {
        Some(uncertainty) => json.push_str(&format!(
            "{indent}  \"uncertainty_binding\": \"{}\",\n",
            json_escape(&uncertainty.binding)
        )),
        None => json.push_str(&format!("{indent}  \"uncertainty_binding\": null,\n")),
    }
    json.push_str(&format!(
        "{indent}  \"status\": \"{}\"\n",
        json_escape(&numeric.status)
    ));
    json.push_str(&format!("{indent}}}"));
}

fn push_runtime_numeric_uncertainty(
    json: &mut String,
    uncertainty: &RuntimeNumericUncertaintyPayload,
    indent: &str,
) {
    json.push_str("{\n");
    json.push_str(&format!(
        "{indent}  \"binding\": \"{}\",\n",
        json_escape(&uncertainty.binding)
    ));
    json.push_str(&format!(
        "{indent}  \"kind\": \"{}\",\n",
        json_escape(&uncertainty.kind)
    ));
    push_optional_json_string(
        json,
        "distribution",
        uncertainty.distribution.as_deref(),
        indent.len() + 2,
    );
    push_optional_json_string(
        json,
        "method",
        uncertainty.method.as_deref(),
        indent.len() + 2,
    );
    push_optional_json_number(json, "mean", uncertainty.mean, indent.len() + 2);
    push_optional_json_number(json, "stddev", uncertainty.stddev, indent.len() + 2);
    push_optional_json_string(
        json,
        "error",
        uncertainty.error.as_deref(),
        indent.len() + 2,
    );
    push_optional_json_number(json, "lower", uncertainty.lower, indent.len() + 2);
    push_optional_json_number(json, "upper", uncertainty.upper, indent.len() + 2);
    push_optional_json_number(json, "p05", uncertainty.p05, indent.len() + 2);
    push_optional_json_number(json, "p50", uncertainty.p50, indent.len() + 2);
    push_optional_json_number(json, "p95", uncertainty.p95, indent.len() + 2);
    json.push_str(&format!(
        "{indent}  \"sample_count\": {},\n",
        uncertainty.sample_count
    ));
    json.push_str(&format!(
        "{indent}  \"status\": \"{}\"\n",
        json_escape(&uncertainty.status)
    ));
    json.push_str(&format!("{indent}}}"));
}

fn push_system_solution_json(
    json: &mut String,
    solution: &runtime_data::RuntimeSystemSolution,
    indent: &str,
) {
    json.push_str("{\n");
    json.push_str(&format!(
        "{indent}  \"status\": \"{}\",\n",
        json_escape(&solution.status)
    ));
    match &solution.binding {
        Some(binding) => json.push_str(&format!(
            "{indent}  \"binding\": \"{}\",\n",
            json_escape(binding)
        )),
        None => json.push_str(&format!("{indent}  \"binding\": null,\n")),
    }
    json.push_str(&format!(
        "{indent}  \"method\": \"{}\",\n",
        json_escape(&solution.method)
    ));
    json.push_str(&format!(
        "{indent}  \"reason\": \"{}\",\n",
        json_escape(&solution.reason)
    ));
    json.push_str(&format!("{indent}  \"states\": ["));
    push_json_string_array(json, &solution.states);
    json.push_str("],\n");
    json.push_str(&format!("{indent}  \"algebraic_variables\": ["));
    push_json_string_array(json, &solution.algebraic_variables);
    json.push_str("],\n");
    json.push_str(&format!("{indent}  \"inputs\": ["));
    push_json_string_array(json, &solution.inputs);
    json.push_str("],\n");
    json.push_str(&format!("{indent}  \"parameters\": ["));
    push_json_string_array(json, &solution.parameters);
    json.push_str("],\n");
    json.push_str(&format!("{indent}  \"outputs\": ["));
    push_json_string_array(json, &solution.outputs);
    json.push_str("],\n");
    push_system_source_equations_json(json, &solution.source_equations, indent);
    json.push_str(&format!(
        "{indent}  \"state\": \"{}\",\n",
        json_escape(&solution.state)
    ));
    json.push_str(&format!(
        "{indent}  \"quantity_kind\": \"{}\",\n",
        json_escape(&solution.quantity_kind)
    ));
    json.push_str(&format!(
        "{indent}  \"display_unit\": \"{}\",\n",
        json_escape(&solution.display_unit)
    ));
    json.push_str(&format!(
        "{indent}  \"canonical_unit\": \"{}\",\n",
        json_escape(&solution.canonical_unit)
    ));
    json.push_str(&format!(
        "{indent}  \"time_unit\": \"{}\",\n",
        json_escape(&solution.time_unit)
    ));
    json.push_str(&format!(
        "{indent}  \"duration\": {},\n",
        solution.duration_s
    ));
    json.push_str(&format!(
        "{indent}  \"time_step\": {},\n",
        solution.time_step_s
    ));
    json.push_str(&format!(
        "{indent}  \"step_count\": {},\n",
        solution.step_count
    ));
    json.push_str(&format!(
        "{indent}  \"tolerance\": {},\n",
        solution.tolerance
    ));
    json.push_str(&format!(
        "{indent}  \"max_iterations\": {},\n",
        solution.max_iterations
    ));
    json.push_str(&format!(
        "{indent}  \"iteration_count\": {},\n",
        solution.iteration_count
    ));
    json.push_str(&format!(
        "{indent}  \"convergence_status\": \"{}\",\n",
        json_escape(&solution.convergence_status)
    ));
    push_optional_json_string(
        json,
        "failure_code",
        solution.failure_code.as_deref(),
        indent.len() + 2,
    );
    push_optional_json_string(
        json,
        "failure_reason",
        solution.failure_reason.as_deref(),
        indent.len() + 2,
    );
    json.push_str(&format!(
        "{indent}  \"initial_value\": {},\n",
        solution.initial_value
    ));
    json.push_str(&format!(
        "{indent}  \"final_value\": {},\n",
        solution.final_value
    ));
    json.push_str(&format!(
        "{indent}  \"canonical_initial_value\": {},\n",
        solution.canonical_initial_value
    ));
    json.push_str(&format!(
        "{indent}  \"canonical_final_value\": {},\n",
        solution.canonical_final_value
    ));
    push_system_step_diagnostics_json(json, &solution.step_diagnostics, indent);
    json.push_str(&format!("{indent}  \"points\": ["));
    push_runtime_points(json, &solution.points);
    json.push_str("]\n");
    json.push_str(&format!("{indent}}}"));
}

fn push_system_source_equations_json(
    json: &mut String,
    equations: &[runtime_data::RuntimeSystemEquationMetadata],
    indent: &str,
) {
    json.push_str(&format!("{indent}  \"source_equations\": [\n"));
    for (equation_index, equation) in equations.iter().enumerate() {
        if equation_index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}    {{\n"));
        json.push_str(&format!(
            "{indent}      \"kind\": \"{}\",\n",
            json_escape(&equation.kind)
        ));
        json.push_str(&format!(
            "{indent}      \"target\": \"{}\",\n",
            json_escape(&equation.target)
        ));
        json.push_str(&format!(
            "{indent}      \"left\": \"{}\",\n",
            json_escape(&equation.left)
        ));
        json.push_str(&format!(
            "{indent}      \"right\": \"{}\",\n",
            json_escape(&equation.right)
        ));
        json.push_str(&format!(
            "{indent}      \"residual_expression\": \"{}\",\n",
            json_escape(&equation.residual_expression)
        ));
        json.push_str(&format!(
            "{indent}      \"quantity_kind\": \"{}\",\n",
            json_escape(&equation.quantity_kind)
        ));
        json.push_str(&format!(
            "{indent}      \"display_unit\": \"{}\",\n",
            json_escape(&equation.display_unit)
        ));
        json.push_str(&format!(
            "{indent}      \"canonical_unit\": \"{}\",\n",
            json_escape(&equation.canonical_unit)
        ));
        match equation.source_line {
            Some(line) => json.push_str(&format!("{indent}      \"source_line\": {}\n", line)),
            None => json.push_str(&format!("{indent}      \"source_line\": null\n")),
        }
        json.push_str(&format!("{indent}    }}"));
    }
    json.push_str(&format!("\n{indent}  ],\n"));
}

fn push_system_source_equation_json(
    json: &mut String,
    equation: &runtime_data::RuntimeSystemEquationMetadata,
    indent: &str,
) {
    json.push_str("{\n");
    json.push_str(&format!(
        "{indent}  \"kind\": \"{}\",\n",
        json_escape(&equation.kind)
    ));
    json.push_str(&format!(
        "{indent}  \"target\": \"{}\",\n",
        json_escape(&equation.target)
    ));
    json.push_str(&format!(
        "{indent}  \"left\": \"{}\",\n",
        json_escape(&equation.left)
    ));
    json.push_str(&format!(
        "{indent}  \"right\": \"{}\",\n",
        json_escape(&equation.right)
    ));
    json.push_str(&format!(
        "{indent}  \"residual_expression\": \"{}\",\n",
        json_escape(&equation.residual_expression)
    ));
    json.push_str(&format!(
        "{indent}  \"quantity_kind\": \"{}\",\n",
        json_escape(&equation.quantity_kind)
    ));
    json.push_str(&format!(
        "{indent}  \"display_unit\": \"{}\",\n",
        json_escape(&equation.display_unit)
    ));
    json.push_str(&format!(
        "{indent}  \"canonical_unit\": \"{}\",\n",
        json_escape(&equation.canonical_unit)
    ));
    match equation.source_line {
        Some(line) => json.push_str(&format!("{indent}  \"source_line\": {}\n", line)),
        None => json.push_str(&format!("{indent}  \"source_line\": null\n")),
    }
    json.push_str(&format!("{indent}}}"));
}

fn push_system_step_diagnostics_json(
    json: &mut String,
    diagnostics: &[runtime_data::RuntimeSystemStepDiagnostic],
    indent: &str,
) {
    json.push_str(&format!("{indent}  \"step_diagnostics\": [\n"));
    for (diagnostic_index, diagnostic) in diagnostics.iter().enumerate() {
        if diagnostic_index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}    {{\n"));
        json.push_str(&format!(
            "{indent}      \"output_index\": {},\n",
            diagnostic.output_index
        ));
        json.push_str(&format!(
            "{indent}      \"start_time_s\": {},\n",
            diagnostic.start_time_s
        ));
        json.push_str(&format!(
            "{indent}      \"end_time_s\": {},\n",
            diagnostic.end_time_s
        ));
        json.push_str(&format!("{indent}      \"dt_s\": {},\n", diagnostic.dt_s));
        json.push_str(&format!(
            "{indent}      \"error_norm\": {},\n",
            diagnostic.error_norm
        ));
        json.push_str(&format!(
            "{indent}      \"status\": \"{}\"\n",
            json_escape(&diagnostic.status)
        ));
        json.push_str(&format!("{indent}    }}"));
    }
    json.push_str(&format!("\n{indent}  ],\n"));
}

fn system_step_diagnostic_review_summary(
    solutions: &[&runtime_data::RuntimeSystemSolution],
) -> (usize, usize, usize, Option<f64>) {
    let diagnostics = solutions
        .iter()
        .find(|solution| !solution.step_diagnostics.is_empty())
        .map(|solution| solution.step_diagnostics.as_slice())
        .unwrap_or(&[]);
    let accepted = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.status == "accepted")
        .count();
    let rejected = diagnostics.len().saturating_sub(accepted);
    let max_error_norm = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.error_norm.abs())
        .reduce(f64::max);
    (diagnostics.len(), accepted, rejected, max_error_norm)
}

fn runtime_review_json(
    base_review: &str,
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
    artifacts: &[OutputArtifact],
) -> String {
    let enriched_review = enrich_runtime_review_side_effects(
        &enrich_runtime_review_boundaries(base_review, process_results),
        artifacts,
    );
    let trimmed = enriched_review.trim_end();
    let Some(prefix) = trimmed.strip_suffix('}') else {
        return base_review.to_owned();
    };
    let mut groups: Vec<Vec<&runtime_data::RuntimeSystemSolution>> = Vec::new();
    for solution in &runtime_data.system_solutions {
        if let Some(group) = groups.iter_mut().find(|group| {
            group.first().is_some_and(|first| {
                first.system == solution.system
                    && first.binding == solution.binding
                    && first.method == solution.method
            })
        }) {
            group.push(solution);
        } else {
            groups.push(vec![solution]);
        }
    }

    let mut json = prefix.trim_end().to_owned();
    json.push_str(",\n  \"simulation_results\": [\n");
    for (group_index, group) in groups.iter().enumerate() {
        if group_index > 0 {
            json.push_str(",\n");
        }
        let first = group[0];
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"system\": \"{}\",\n",
            json_escape(&first.system)
        ));
        match &first.binding {
            Some(binding) => json.push_str(&format!(
                "      \"binding\": \"{}\",\n",
                json_escape(binding)
            )),
            None => json.push_str("      \"binding\": null,\n"),
        }
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&first.status)
        ));
        json.push_str(&format!(
            "      \"method\": \"{}\",\n",
            json_escape(&first.method)
        ));
        json.push_str(&format!(
            "      \"reason\": \"{}\",\n",
            json_escape(&first.reason)
        ));
        json.push_str("      \"variables\": {\n");
        json.push_str("        \"states\": [");
        push_json_string_array(&mut json, &first.states);
        json.push_str("],\n");
        json.push_str("        \"algebraic_variables\": [");
        push_json_string_array(&mut json, &first.algebraic_variables);
        json.push_str("],\n");
        json.push_str("        \"inputs\": [");
        push_json_string_array(&mut json, &first.inputs);
        json.push_str("],\n");
        json.push_str("        \"parameters\": [");
        push_json_string_array(&mut json, &first.parameters);
        json.push_str("],\n");
        json.push_str("        \"outputs\": [");
        push_json_string_array(&mut json, &first.outputs);
        json.push_str("]\n");
        json.push_str("      },\n");
        json.push_str("      \"source_equations\": [\n");
        for (equation_index, equation) in first.source_equations.iter().enumerate() {
            if equation_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        ");
            push_system_source_equation_json(&mut json, equation, "        ");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"diagnostics\": {\n");
        json.push_str(&format!("        \"tolerance\": {},\n", first.tolerance));
        json.push_str(&format!(
            "        \"max_iterations\": {},\n",
            first.max_iterations
        ));
        json.push_str(&format!(
            "        \"iteration_count\": {},\n",
            first.iteration_count
        ));
        json.push_str(&format!(
            "        \"convergence_status\": \"{}\",\n",
            json_escape(&first.convergence_status)
        ));
        match &first.failure_code {
            Some(code) => json.push_str(&format!(
                "        \"failure_code\": \"{}\",\n",
                json_escape(code)
            )),
            None => json.push_str("        \"failure_code\": null,\n"),
        }
        match &first.failure_reason {
            Some(reason) => json.push_str(&format!(
                "        \"failure_reason\": \"{}\",\n",
                json_escape(reason)
            )),
            None => json.push_str("        \"failure_reason\": null,\n"),
        }
        let (substep_count, accepted_substep_count, rejected_substep_count, max_error_norm) =
            system_step_diagnostic_review_summary(group);
        json.push_str(&format!("        \"substep_count\": {},\n", substep_count));
        json.push_str(&format!(
            "        \"accepted_substep_count\": {},\n",
            accepted_substep_count
        ));
        json.push_str(&format!(
            "        \"rejected_substep_count\": {},\n",
            rejected_substep_count
        ));
        match max_error_norm {
            Some(value) => {
                json.push_str(&format!("        \"max_substep_error_norm\": {}\n", value))
            }
            None => json.push_str("        \"max_substep_error_norm\": null\n"),
        }
        json.push_str("      },\n");
        json.push_str("      \"time_grid\": {\n");
        json.push_str(&format!(
            "        \"unit\": \"{}\",\n",
            json_escape(&first.time_unit)
        ));
        json.push_str(&format!("        \"duration\": {},\n", first.duration_s));
        json.push_str(&format!("        \"timestep\": {},\n", first.time_step_s));
        json.push_str(&format!("        \"step_count\": {}\n", first.step_count));
        json.push_str("      },\n");
        json.push_str("      \"solver_results\": [\n");
        for (solution_index, solution) in group.iter().enumerate() {
            if solution_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        ");
            push_system_solution_json(&mut json, solution, "        ");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"states\": [\n");
        for (state_index, solution) in group.iter().enumerate() {
            if state_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&solution.state)
            ));
            json.push_str(&format!(
                "          \"quantity_kind\": \"{}\",\n",
                json_escape(&solution.quantity_kind)
            ));
            json.push_str(&format!(
                "          \"display_unit\": \"{}\",\n",
                json_escape(&solution.display_unit)
            ));
            json.push_str(&format!(
                "          \"canonical_unit\": \"{}\",\n",
                json_escape(&solution.canonical_unit)
            ));
            json.push_str(&format!(
                "          \"initial_value\": {},\n",
                solution.initial_value
            ));
            json.push_str(&format!(
                "          \"final_value\": {},\n",
                solution.final_value
            ));
            json.push_str(&format!(
                "          \"canonical_initial_value\": {},\n",
                solution.canonical_initial_value
            ));
            json.push_str(&format!(
                "          \"canonical_final_value\": {},\n",
                solution.canonical_final_value
            ));
            json.push_str(&format!(
                "          \"point_count\": {}\n",
                solution.points.len()
            ));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    let db_manifest_records = db_manifest_records(process_results);
    json.push_str("\n  ],\n  \"table_selections\": [\n");
    json.push_str(&table_selections_json(runtime_data, "    "));
    json.push_str("\n  ],\n  \"timeseries_coverage\": [\n");
    json.push_str(&timeseries_coverage_json(runtime_data, "    "));
    json.push_str("\n  ],\n  \"db_manifests\": [\n");
    json.push_str(&db_manifests_json(&db_manifest_records));
    json.push_str("\n  ]\n}\n");
    json
}

fn enrich_runtime_review_boundaries(
    base_review: &str,
    process_results: &[ProcessExecutionRecord],
) -> String {
    if process_results.is_empty() {
        return base_review.to_owned();
    }

    let Ok(mut review) = serde_json::from_str::<Value>(base_review) else {
        return base_review.to_owned();
    };
    let Some(boundaries) = review
        .pointer_mut("/review_document/external_boundaries")
        .and_then(Value::as_array_mut)
    else {
        return base_review.to_owned();
    };

    for process in process_results {
        let Some(boundary) = boundaries.iter_mut().find(|boundary| {
            boundary.get("kind").and_then(Value::as_str) == Some("process")
                && boundary.get("name").and_then(Value::as_str) == Some(process.binding.as_str())
        }) else {
            continue;
        };
        let Some(object) = boundary.as_object_mut() else {
            continue;
        };
        let outputs = process
            .expected_outputs
            .iter()
            .map(|output| output.path.clone())
            .collect::<Vec<_>>();
        let output_artifacts = process
            .expected_outputs
            .iter()
            .map(|output| {
                json!({
                    "kind": output.artifact_kind.clone(),
                    "path": output.path.clone(),
                    "hash": output.hash.clone(),
                    "status": output.status.clone(),
                    "validation": {
                        "status": output.validation.status.clone(),
                        "rule": output.validation.rule.clone(),
                        "message": output.validation.message.clone()
                    }
                })
            })
            .collect::<Vec<_>>();

        object.insert(
            "provenance".to_owned(),
            Value::String("runtime_process_result".to_owned()),
        );
        object.insert("success".to_owned(), Value::Bool(process.success));
        object.insert("status".to_owned(), Value::String(process.status.clone()));
        object.insert(
            "expected_output_status".to_owned(),
            Value::String(process.expected_output_status.clone()),
        );
        object.insert(
            "stdout_hash".to_owned(),
            Value::String(process.stdout_hash.clone()),
        );
        object.insert(
            "stderr_hash".to_owned(),
            Value::String(process.stderr_hash.clone()),
        );
        object.insert("exit_code".to_owned(), json!(process.exit_code));
        object.insert("duration_ms".to_owned(), json!(process.duration_ms));
        object.insert("outputs".to_owned(), json!(outputs));
        object.insert("output_artifacts".to_owned(), json!(output_artifacts));
    }

    serde_json::to_string_pretty(&review)
        .map(|mut json| {
            json.push('\n');
            json
        })
        .unwrap_or_else(|_| base_review.to_owned())
}

fn enrich_runtime_review_side_effects(base_review: &str, artifacts: &[OutputArtifact]) -> String {
    if artifacts.is_empty() {
        return base_review.to_owned();
    }

    let artifact_records = artifact_records_for_outputs(artifacts);
    let Ok(mut review) = serde_json::from_str::<Value>(base_review) else {
        return base_review.to_owned();
    };
    let Some(side_effects) = review
        .pointer_mut("/review_document/side_effects")
        .and_then(Value::as_array_mut)
    else {
        return base_review.to_owned();
    };

    for side_effect in side_effects {
        let effect_kind = side_effect
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(target) = side_effect.get("target").and_then(Value::as_str) else {
            continue;
        };
        let Some(record) = artifact_records.iter().find(|record| {
            side_effect_artifact_kind_matches(effect_kind, &record.kind)
                && review_artifact_paths_match(target, &record.path)
        }) else {
            continue;
        };
        let Some(object) = side_effect.as_object_mut() else {
            continue;
        };

        object.insert(
            "provenance".to_owned(),
            Value::String("runtime_artifact_record".to_owned()),
        );
        object.insert(
            "artifact_kind".to_owned(),
            Value::String(record.kind.clone()),
        );
        object.insert(
            "artifact_class".to_owned(),
            Value::String(record.class.clone()),
        );
        object.insert(
            "artifact_path".to_owned(),
            Value::String(record.path.clone()),
        );
        object.insert("hash".to_owned(), Value::String(record.hash.clone()));
        object.insert("status".to_owned(), Value::String(record.status.clone()));
        object.insert(
            "validation".to_owned(),
            json!({
                "status": record.validation.status.clone(),
                "rule": record.validation.rule.clone(),
                "message": record.validation.message.clone()
            }),
        );
    }

    serde_json::to_string_pretty(&review)
        .map(|mut json| {
            json.push('\n');
            json
        })
        .unwrap_or_else(|_| base_review.to_owned())
}

fn side_effect_artifact_kind_matches(effect_kind: &str, artifact_kind: &str) -> bool {
    match effect_kind {
        "csv_export" => artifact_kind == "csv_export",
        "write_output" => artifact_kind.starts_with("write_"),
        "file_copy" => artifact_kind == "copy_file",
        "file_move" => artifact_kind == "move_file",
        "file_delete" => matches!(
            artifact_kind,
            "delete_file" | "delete_dir" | "delete_missing"
        ),
        _ => false,
    }
}

fn review_artifact_paths_match(target: &str, artifact_path: &str) -> bool {
    normalize_review_artifact_path(target) == normalize_review_artifact_path(artifact_path)
}

fn normalize_review_artifact_path(path: &str) -> String {
    let mut normalized = strip_runtime_string_value(path).replace('\\', "/");
    while let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped.to_owned();
    }
    normalized
}

fn component_solutions_json(runtime_data: &RuntimeData) -> String {
    let mut json = String::new();
    for (solution_index, solution) in runtime_data.component_solutions.iter().enumerate() {
        if solution_index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"assembly\": \"{}\",\n",
            json_escape(&solution.assembly)
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&solution.status)
        ));
        json.push_str(&format!(
            "        \"method\": \"{}\",\n",
            json_escape(&solution.method)
        ));
        json.push_str(&format!(
            "        \"reason\": \"{}\",\n",
            json_escape(&solution.reason)
        ));
        json.push_str(&format!(
            "        \"equation_count\": {},\n",
            solution.equation_count
        ));
        json.push_str(&format!(
            "        \"unknown_count\": {},\n",
            solution.unknown_count
        ));
        json.push_str(&format!(
            "        \"residual_norm\": {},\n",
            format_number_with_precision(solution.residual_norm, Some(8))
        ));
        push_optional_json_f64(
            &mut json,
            "linear_condition_estimate",
            solution.linear_condition_estimate,
            8,
        );
        push_optional_json_f64(
            &mut json,
            "linear_minimum_pivot_abs",
            solution.linear_minimum_pivot_abs,
            8,
        );
        push_optional_json_f64(
            &mut json,
            "linear_maximum_pivot_abs",
            solution.linear_maximum_pivot_abs,
            8,
        );
        json.push_str(&format!(
            "        \"variable_scale_policy\": \"{}\",\n",
            json_escape(&solution.variable_scale_policy)
        ));
        push_optional_json_f64(
            &mut json,
            "variable_scale_min",
            solution.variable_scale_min,
            8,
        );
        push_optional_json_f64(
            &mut json,
            "variable_scale_max",
            solution.variable_scale_max,
            8,
        );
        json.push_str(&format!("        \"tolerance\": {},\n", solution.tolerance));
        json.push_str(&format!(
            "        \"max_iterations\": {},\n",
            solution.max_iterations
        ));
        json.push_str(&format!(
            "        \"iteration_count\": {},\n",
            solution.iteration_count
        ));
        json.push_str(&format!(
            "        \"convergence_status\": \"{}\",\n",
            json_escape(&solution.convergence_status)
        ));
        push_optional_json_string(
            &mut json,
            "failure_code",
            solution
                .failure_artifact
                .as_ref()
                .map(|failure| failure.code.as_str()),
            8,
        );
        push_optional_json_string(
            &mut json,
            "failure_reason",
            solution
                .failure_artifact
                .as_ref()
                .map(|failure| failure.message.as_str()),
            8,
        );
        json.push_str("        \"variables\": [\n");
        for (variable_index, variable) in solution.variables.iter().enumerate() {
            if variable_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"name\": \"{}\",\n",
                json_escape(&variable.name)
            ));
            json.push_str(&format!(
                "            \"role\": \"{}\",\n",
                json_escape(&variable.role)
            ));
            json.push_str(&format!(
                "            \"value\": {},\n",
                format_number_with_precision(variable.value, Some(8))
            ));
            json.push_str(&format!(
                "            \"unit\": \"{}\",\n",
                json_escape(&variable.unit)
            ));
            json.push_str(&format!(
                "            \"status\": \"{}\"\n",
                json_escape(&variable.status)
            ));
            json.push_str("          }");
        }
        json.push_str("\n        ],\n");
        json.push_str("        \"trajectories\": [\n");
        for (trajectory_index, trajectory) in solution.trajectories.iter().enumerate() {
            if trajectory_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"name\": \"{}\",\n",
                json_escape(&trajectory.name)
            ));
            json.push_str(&format!(
                "            \"role\": \"{}\",\n",
                json_escape(&trajectory.role)
            ));
            json.push_str(&format!(
                "            \"quantity_kind\": \"{}\",\n",
                json_escape(&trajectory.quantity_kind)
            ));
            json.push_str(&format!(
                "            \"unit\": \"{}\",\n",
                json_escape(&trajectory.unit)
            ));
            json.push_str(&format!(
                "            \"initial_value\": {},\n",
                format_number_with_precision(trajectory.initial_value, Some(8))
            ));
            json.push_str(&format!(
                "            \"final_value\": {},\n",
                format_number_with_precision(trajectory.final_value, Some(8))
            ));
            json.push_str(&format!(
                "            \"point_count\": {},\n",
                trajectory.point_count
            ));
            json.push_str("            \"points\": [\n");
            for (point_index, point) in trajectory.points.iter().enumerate() {
                if point_index > 0 {
                    json.push_str(",\n");
                }
                json.push_str("              {\n");
                json.push_str(&format!(
                    "                \"x\": {},\n",
                    format_number_with_precision(point.x, Some(8))
                ));
                json.push_str(&format!(
                    "                \"y\": {}\n",
                    format_number_with_precision(point.y, Some(8))
                ));
                json.push_str("              }");
            }
            json.push_str("\n            ]\n");
            json.push_str("          }");
        }
        json.push_str("\n        ],\n");
        json.push_str("        \"step_diagnostics\": [\n");
        for (diagnostic_index, diagnostic) in solution.step_diagnostics.iter().enumerate() {
            if diagnostic_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"step_index\": {},\n",
                diagnostic.step_index
            ));
            json.push_str(&format!(
                "            \"time_s\": {},\n",
                format_number_with_precision(diagnostic.time_s, Some(8))
            ));
            json.push_str(&format!(
                "            \"algebraic_iteration_count\": {},\n",
                diagnostic.algebraic_iteration_count
            ));
            json.push_str(&format!(
                "            \"residual_norm\": {},\n",
                format_number_with_precision(diagnostic.residual_norm, Some(8))
            ));
            json.push_str("            \"residual_values\": [");
            push_json_f64_array(&mut json, &diagnostic.residual_values);
            json.push_str("],\n");
            json.push_str("            \"normalized_residual_values\": [");
            push_json_f64_array(&mut json, &diagnostic.normalized_residual_values);
            json.push_str("],\n");
            push_optional_json_f64(
                &mut json,
                "line_search_scale",
                diagnostic.line_search_scale,
                12,
            );
            push_optional_json_usize(
                &mut json,
                "line_search_trial_count",
                diagnostic.line_search_trial_count,
                12,
            );
            push_optional_json_string(
                &mut json,
                "jacobian_policy",
                diagnostic.jacobian_policy.as_deref(),
                12,
            );
            push_optional_json_string(
                &mut json,
                "variable_scale_policy",
                diagnostic.variable_scale_policy.as_deref(),
                12,
            );
            push_optional_json_f64(
                &mut json,
                "linear_condition_estimate",
                diagnostic.linear_condition_estimate,
                12,
            );
            push_optional_json_f64(
                &mut json,
                "linear_minimum_pivot_abs",
                diagnostic.linear_minimum_pivot_abs,
                12,
            );
            push_optional_json_f64(
                &mut json,
                "linear_maximum_pivot_abs",
                diagnostic.linear_maximum_pivot_abs,
                12,
            );
            push_optional_json_usize(
                &mut json,
                "largest_residual_index",
                diagnostic.largest_residual_index,
                12,
            );
            push_optional_json_string(
                &mut json,
                "largest_residual_name",
                diagnostic.largest_residual_name.as_deref(),
                12,
            );
            let largest_residual_source = component_largest_residual_source_context(
                diagnostic.largest_residual_name.as_deref(),
                &solution.residuals,
            );
            push_optional_json_string(
                &mut json,
                "largest_residual_source_expression",
                largest_residual_source.map(|residual| residual.source_expression.as_str()),
                12,
            );
            push_optional_json_usize(
                &mut json,
                "largest_residual_source_line",
                largest_residual_source.and_then(|residual| residual.source_line),
                12,
            );
            push_optional_json_string(
                &mut json,
                "largest_residual_source_reason",
                largest_residual_source.and_then(|residual| residual.source_reason.as_deref()),
                12,
            );
            push_optional_json_f64(
                &mut json,
                "largest_residual_value",
                diagnostic.largest_residual_value,
                12,
            );
            push_optional_json_f64(
                &mut json,
                "largest_residual_abs_value",
                diagnostic.largest_residual_abs_value,
                12,
            );
            json.push_str(&format!(
                "            \"convergence_status\": \"{}\",\n",
                json_escape(&diagnostic.convergence_status)
            ));
            push_optional_json_string(
                &mut json,
                "failure_code",
                diagnostic
                    .failure_artifact
                    .as_ref()
                    .map(|failure| failure.code.as_str()),
                12,
            );
            push_optional_json_string(
                &mut json,
                "failure_reason",
                diagnostic
                    .failure_artifact
                    .as_ref()
                    .map(|failure| failure.message.as_str()),
                12,
            );
            match &diagnostic.failure_artifact {
                Some(failure) => {
                    json.push_str("            \"failure_artifact\": {\n");
                    json.push_str(&format!(
                        "              \"code\": \"{}\",\n",
                        json_escape(&failure.code)
                    ));
                    json.push_str(&format!(
                        "              \"message\": \"{}\"\n",
                        json_escape(&failure.message)
                    ));
                    json.push_str("            }\n");
                }
                None => json.push_str("            \"failure_artifact\": null\n"),
            }
            json.push_str("          }");
        }
        json.push_str("\n        ],\n");
        json.push_str("        \"residuals\": [\n");
        push_component_residual_evaluations_json(&mut json, &solution.residuals, "          ");
        json.push_str("\n        ],\n");
        json.push_str("        \"largest_residuals\": [\n");
        push_component_residual_evaluations_json(
            &mut json,
            &solution.largest_residuals,
            "          ",
        );
        json.push_str("\n        ],\n");
        match &solution.failure_artifact {
            Some(failure) => {
                json.push_str("        \"failure_artifact\": {\n");
                json.push_str(&format!(
                    "          \"code\": \"{}\",\n",
                    json_escape(&failure.code)
                ));
                json.push_str(&format!(
                    "          \"message\": \"{}\"\n",
                    json_escape(&failure.message)
                ));
                json.push_str("        }\n");
            }
            None => json.push_str("        \"failure_artifact\": null\n"),
        }
        json.push_str("      }");
    }
    json
}

fn component_largest_residual_source_context<'a>(
    name: Option<&str>,
    residuals: &'a [RuntimeComponentResidualEvaluation],
) -> Option<&'a RuntimeComponentResidualEvaluation> {
    let name = name?;
    residuals.iter().find(|residual| residual.name == name)
}

fn push_component_residual_evaluations_json(
    json: &mut String,
    residuals: &[RuntimeComponentResidualEvaluation],
    item_indent: &str,
) {
    for (residual_index, residual) in residuals.iter().enumerate() {
        if residual_index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{item_indent}{{\n"));
        json.push_str(&format!(
            "{item_indent}  \"name\": \"{}\",\n",
            json_escape(&residual.name)
        ));
        json.push_str(&format!(
            "{item_indent}  \"expression\": \"{}\",\n",
            json_escape(&residual.expression)
        ));
        json.push_str(&format!(
            "{item_indent}  \"source_expression\": \"{}\",\n",
            json_escape(&residual.source_expression)
        ));
        push_optional_json_usize(
            json,
            "source_line",
            residual.source_line,
            item_indent.len() + 2,
        );
        push_optional_json_string(
            json,
            "source_reason",
            residual.source_reason.as_deref(),
            item_indent.len() + 2,
        );
        json.push_str(&format!("{item_indent}  \"dependencies\": ["));
        for (dependency_index, dependency) in residual.dependencies.iter().enumerate() {
            if dependency_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(dependency)));
        }
        json.push_str("],\n");
        json.push_str(&format!(
            "{item_indent}  \"value\": {},\n",
            format_number_with_precision(residual.value, Some(8))
        ));
        json.push_str(&format!(
            "{item_indent}  \"unit\": \"{}\",\n",
            json_escape(&residual.unit)
        ));
        json.push_str(&format!(
            "{item_indent}  \"expression_unit\": \"{}\",\n",
            json_escape(&residual.expression_unit)
        ));
        json.push_str(&format!(
            "{item_indent}  \"expression_quantity_kind\": \"{}\",\n",
            json_escape(&residual.expression_quantity_kind)
        ));
        json.push_str(&format!(
            "{item_indent}  \"normalized_value\": {},\n",
            format_number_with_precision(residual.normalized_value, Some(8))
        ));
        json.push_str(&format!(
            "{item_indent}  \"scale\": {},\n",
            format_number_with_precision(residual.scale, Some(8))
        ));
        json.push_str(&format!(
            "{item_indent}  \"scale_policy\": \"{}\",\n",
            json_escape(&residual.scale_policy)
        ));
        json.push_str(&format!(
            "{item_indent}  \"lowering_status\": \"{}\",\n",
            json_escape(&residual.lowering_status)
        ));
        push_optional_json_string(
            json,
            "lowering_failure_code",
            residual.lowering_failure_code.as_deref(),
            item_indent.len() + 2,
        );
        push_optional_json_string(
            json,
            "lowering_failure_reason",
            residual.lowering_failure_reason.as_deref(),
            item_indent.len() + 2,
        );
        json.push_str(&format!(
            "{item_indent}  \"status\": \"{}\"\n",
            json_escape(&residual.status)
        ));
        json.push_str(&format!("{item_indent}}}"));
    }
}

fn solver_boundaries_json(report: &CheckReport, runtime_data: &RuntimeData) -> String {
    let mut json = String::new();
    for (index, system) in report.semantic_program.systems.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        let solution = runtime_data
            .system_solutions
            .iter()
            .find(|solution| solution.system == system.name);
        let status = solution
            .map(|solution| solution.status.as_str())
            .unwrap_or("unsolved");
        let reason = solution
            .map(|solution| solution.reason.as_str())
            .unwrap_or("numeric solver deferred until the solver milestone");
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"system\": \"{}\",\n",
            json_escape(&system.name)
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(status)
        ));
        json.push_str(&format!(
            "        \"reason\": \"{}\",\n",
            json_escape(reason)
        ));
        json.push_str(&format!(
            "        \"parameter_count\": {},\n",
            role_count(system, "parameter")
        ));
        json.push_str(&format!(
            "        \"state_count\": {},\n",
            role_count(system, "state")
        ));
        json.push_str(&format!(
            "        \"input_count\": {},\n",
            role_count(system, "input")
        ));
        json.push_str(&format!(
            "        \"equation_count\": {},\n",
            system.equations.len()
        ));
        json.push_str(&format!(
            "        \"residual_count\": {},\n",
            system.residuals.len()
        ));
        json.push_str(&format!("        \"line\": {}\n", system.line));
        json.push_str("      }");
    }
    json
}

fn system_ir_json(report: &CheckReport, runtime_data: &RuntimeData) -> String {
    let mut json = String::new();
    for (index, system) in report.semantic_program.systems.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        let solution = runtime_data
            .system_solutions
            .iter()
            .find(|solution| solution.system == system.name);
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&system.name)
        ));
        push_solver_plan_json(&mut json, &system.solver_plan, "        ", solution);
        json.push_str(",\n");
        json.push_str("        \"equations\": [\n");
        for (equation_index, equation) in system.equation_ir.iter().enumerate() {
            if equation_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"residual\": \"{}\",\n",
                json_escape(&equation.residual)
            ));
            json.push_str(&format!(
                "            \"relation\": \"{}\",\n",
                json_escape(&equation.relation)
            ));
            json.push_str(&format!(
                "            \"normalized_residual\": \"{}\",\n",
                json_escape(&equation.normalized_residual)
            ));
            json.push_str(&format!(
                "            \"status\": \"{}\",\n",
                json_escape(&equation.status)
            ));
            json.push_str("            \"dependencies\": [\n");
            for (dependency_index, dependency) in equation.dependencies.iter().enumerate() {
                if dependency_index > 0 {
                    json.push_str(",\n");
                }
                json.push_str("              {\n");
                json.push_str(&format!(
                    "                \"name\": \"{}\",\n",
                    json_escape(&dependency.name)
                ));
                json.push_str(&format!(
                    "                \"role\": \"{}\",\n",
                    json_escape(&dependency.role)
                ));
                json.push_str(&format!(
                    "                \"quantity_kind\": \"{}\"\n",
                    json_escape(&dependency.quantity_kind)
                ));
                json.push_str("              }");
            }
            json.push_str("\n            ],\n");
            json.push_str("            \"derivative_states\": [");
            for (state_index, state) in equation.derivative_states.iter().enumerate() {
                if state_index > 0 {
                    json.push_str(", ");
                }
                json.push_str(&format!("\"{}\"", json_escape(state)));
            }
            json.push_str("],\n");
            json.push_str(&format!("            \"line\": {}\n", equation.line));
            json.push_str("          }");
        }
        json.push_str("\n        ],\n");
        json.push_str(&format!("        \"line\": {}\n", system.line));
        json.push_str("      }");
    }
    json
}

fn push_solver_plan_json(
    json: &mut String,
    plan: &eng_compiler::SolverPlanInfo,
    indent: &str,
    solution: Option<&runtime_data::RuntimeSystemSolution>,
) {
    let status = solution
        .map(|solution| solution.status.as_str())
        .unwrap_or(&plan.status);
    let method = solution
        .map(|solution| solution.method.as_str())
        .unwrap_or(&plan.method);
    let ode_status = solution
        .map(|solution| solution.status.as_str())
        .unwrap_or(&plan.ode_runner.status);
    let ode_reason = solution
        .map(|solution| solution.reason.as_str())
        .unwrap_or(&plan.ode_runner.reason);

    json.push_str(&format!("{indent}\"solver_plan\": {{\n"));
    json.push_str(&format!(
        "{indent}  \"status\": \"{}\",\n",
        json_escape(status)
    ));
    json.push_str(&format!(
        "{indent}  \"method\": \"{}\",\n",
        json_escape(method)
    ));
    json.push_str(&format!("{indent}  \"solve_order\": ["));
    for (index, residual) in plan.solve_order.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!("\"{}\"", json_escape(residual)));
    }
    json.push_str("],\n");
    json.push_str(&format!("{indent}  \"ode_runner\": {{\n"));
    json.push_str(&format!(
        "{indent}    \"status\": \"{}\",\n",
        json_escape(ode_status)
    ));
    json.push_str(&format!(
        "{indent}    \"reason\": \"{}\"\n",
        json_escape(ode_reason)
    ));
    json.push_str(&format!("{indent}  }},\n"));
    json.push_str(&format!("{indent}  \"jacobian_seed\": [\n"));
    for (seed_index, seed) in plan.jacobian_seed.iter().enumerate() {
        if seed_index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}    {{\n"));
        json.push_str(&format!(
            "{indent}      \"residual\": \"{}\",\n",
            json_escape(&seed.residual)
        ));
        json.push_str(&format!("{indent}      \"with_respect_to\": ["));
        for (variable_index, variable) in seed.with_respect_to.iter().enumerate() {
            if variable_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(variable)));
        }
        json.push_str("],\n");
        json.push_str(&format!("{indent}      \"derivative_states\": ["));
        for (state_index, state) in seed.derivative_states.iter().enumerate() {
            if state_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(state)));
        }
        json.push_str("],\n");
        json.push_str(&format!(
            "{indent}      \"status\": \"{}\"\n",
            json_escape(&seed.status)
        ));
        json.push_str(&format!("{indent}    }}"));
    }
    json.push_str(&format!("\n{indent}  ]\n"));
    json.push_str(&format!("{indent}}}"));
}

fn role_count(system: &eng_compiler::SystemInfo, role: &str) -> usize {
    system
        .variables
        .iter()
        .filter(|variable| variable.role == role)
        .count()
}

fn sample_tables_json(runtime_data: &RuntimeData) -> String {
    let mut json = String::new();
    for (index, sample_table) in runtime_data.sample_tables.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&sample_table.binding)
        ));
        json.push_str(&format!(
            "        \"schema_name\": \"{}\",\n",
            json_escape(&sample_table.schema_name)
        ));
        json.push_str(&format!(
            "        \"source\": \"{}\",\n",
            json_escape(&sample_table.source)
        ));
        push_optional_json_string(
            &mut json,
            "source_hash",
            sample_table.source_hash.as_deref(),
            8,
        );
        json.push_str(&format!(
            "        \"sample_count\": {},\n",
            sample_table.sample_count
        ));
        push_optional_json_string(
            &mut json,
            "case_id_column",
            sample_table.case_id_column.as_deref(),
            8,
        );
        json.push_str("        \"parameter_columns\": [\n");
        for (column_index, column) in sample_table.parameter_columns.iter().enumerate() {
            if column_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"name\": \"{}\",\n",
                json_escape(&column.name)
            ));
            json.push_str(&format!(
                "            \"quantity_kind\": \"{}\",\n",
                json_escape(&column.quantity_kind)
            ));
            json.push_str(&format!(
                "            \"display_unit\": \"{}\",\n",
                json_escape(&column.display_unit)
            ));
            push_optional_json_number(&mut json, "min", column.min, 12);
            push_optional_json_number(&mut json, "max", column.max, 12);
            json.push_str(&format!(
                "            \"missing_count\": {}\n",
                column.missing_count
            ));
            json.push_str("          }");
        }
        json.push_str("\n        ],\n");
        json.push_str("        \"duplicate_case_ids\": [");
        push_json_string_array(&mut json, &sample_table.duplicate_case_ids);
        json.push_str("],\n");
        json.push_str(&format!(
            "        \"row_hash_count\": {},\n",
            sample_table.row_hash_count
        ));
        json.push_str("        \"row_hash_preview\": [");
        push_json_string_array(&mut json, &sample_table.row_hash_preview);
        json.push_str("],\n");
        json.push_str(&format!(
            "        \"generation\": \"{}\",\n",
            json_escape(&sample_table.generation)
        ));
        push_optional_json_string(&mut json, "seed", sample_table.seed.as_deref(), 8);
        json.push_str(&format!(
            "        \"status\": \"{}\"\n",
            json_escape(&sample_table.status)
        ));
        json.push_str("      }");
    }
    json
}

fn case_manifests_json(
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
) -> String {
    let case_manifests = materialized_case_manifests(runtime_data, process_results);
    let mut json = String::new();
    for (index, manifest) in case_manifests.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"case_id\": \"{}\",\n",
            json_escape(&manifest.case_id)
        ));
        json.push_str(&format!(
            "        \"sample_table\": \"{}\",\n",
            json_escape(&manifest.sample_table)
        ));
        json.push_str(&format!(
            "        \"schema_name\": \"{}\",\n",
            json_escape(&manifest.schema_name)
        ));
        json.push_str(&format!(
            "        \"source\": \"{}\",\n",
            json_escape(&manifest.source)
        ));
        push_optional_json_string(&mut json, "source_hash", manifest.source_hash.as_deref(), 8);
        json.push_str(&format!(
            "        \"sample_row_number\": {},\n",
            manifest.sample_row_number
        ));
        json.push_str(&format!(
            "        \"source_row\": {},\n",
            manifest.source_row
        ));
        json.push_str(&format!(
            "        \"sample_row_hash\": \"{}\",\n",
            json_escape(&manifest.sample_row_hash)
        ));
        push_optional_json_string(&mut json, "case_dir", manifest.case_dir.as_deref(), 8);
        push_optional_json_string(
            &mut json,
            "generated_input_file",
            manifest.generated_input_file.as_deref(),
            8,
        );
        json.push_str("        \"process_bindings\": [");
        push_json_string_array(&mut json, &manifest.process_bindings);
        json.push_str("],\n");
        json.push_str("        \"process_statuses\": [");
        push_case_process_statuses_json(&mut json, &manifest.process_statuses);
        json.push_str("],\n");
        json.push_str("        \"output_artifacts\": [");
        push_json_string_array(&mut json, &manifest.output_artifacts);
        json.push_str("],\n");
        json.push_str("        \"result_files\": [");
        push_json_string_array(&mut json, &manifest.result_files);
        json.push_str("],\n");
        json.push_str("        \"metrics\": [");
        push_case_metrics_json(&mut json, &manifest.metrics);
        json.push_str("],\n");
        push_optional_json_string(
            &mut json,
            "failure_reason",
            manifest.failure_reason.as_deref(),
            8,
        );
        json.push_str(&format!(
            "        \"status\": \"{}\"\n",
            json_escape(&manifest.status)
        ));
        json.push_str("      }");
    }
    json
}

fn materialized_case_manifests(
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
) -> Vec<RuntimeCaseManifest> {
    let mut manifests = runtime_data.case_manifests.clone();
    if manifests.is_empty() || process_results.is_empty() {
        return manifests;
    }

    for process in process_results {
        let linked_case_ids = linked_case_ids_for_process(&manifests, process);
        for case_id in linked_case_ids {
            for manifest in manifests
                .iter_mut()
                .filter(|manifest| manifest.case_id == case_id)
            {
                apply_process_outputs_to_case_manifest(manifest, process);
            }
        }
    }

    for manifest in &mut manifests {
        finalize_case_manifest_status(manifest);
    }

    manifests
}

fn linked_case_ids_for_process(
    manifests: &[RuntimeCaseManifest],
    process: &ProcessExecutionRecord,
) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut linked = Vec::new();
    for output in &process.expected_outputs {
        for manifest in manifests {
            if manifest.case_id.trim().is_empty() {
                continue;
            }
            if output_path_matches_case(&output.path, &manifest.case_id)
                && seen.insert(manifest.case_id.clone())
            {
                linked.push(manifest.case_id.clone());
            }
        }
    }
    linked
}

fn apply_process_outputs_to_case_manifest(
    manifest: &mut RuntimeCaseManifest,
    process: &ProcessExecutionRecord,
) {
    let mut linked = false;
    let case_id = manifest.case_id.clone();
    for output in process
        .expected_outputs
        .iter()
        .filter(|output| output_path_matches_case(&output.path, &case_id))
    {
        linked = true;
        apply_expected_output_to_case_manifest(manifest, process, output);
    }

    if linked {
        push_unique_string(&mut manifest.process_bindings, process.binding.clone());
        push_unique_case_process_status(
            &mut manifest.process_statuses,
            RuntimeCaseProcessStatus {
                name: process.binding.clone(),
                command: process_command_line(process),
                status: process.status.clone(),
            },
        );
        if !process.success && manifest.failure_reason.is_none() {
            manifest.failure_reason = Some(format!(
                "process `{}` reported status `{}`",
                process.binding, process.status
            ));
        }
    }
}

fn apply_expected_output_to_case_manifest(
    manifest: &mut RuntimeCaseManifest,
    process: &ProcessExecutionRecord,
    output: &ProcessExpectedOutputRecord,
) {
    if manifest.case_dir.is_none() {
        manifest.case_dir = infer_case_dir_from_output_path(&output.path, &manifest.case_id);
    }
    push_unique_string(&mut manifest.output_artifacts, output.path.clone());
    if is_case_generated_input_output(&output.path) && manifest.generated_input_file.is_none() {
        manifest.generated_input_file = Some(output.path.clone());
    }
    if is_case_result_file_output(&output.path) {
        push_unique_string(&mut manifest.result_files, output.path.clone());
    }
    if !output.exists && manifest.failure_reason.is_none() {
        manifest.failure_reason = Some(format!(
            "process `{}` did not create expected output `{}`",
            process.binding, output.path
        ));
    }
    if is_case_manifest_output(&output.path) {
        apply_external_case_manifest_payload(manifest, output);
    }
}

fn apply_external_case_manifest_payload(
    manifest: &mut RuntimeCaseManifest,
    output: &ProcessExpectedOutputRecord,
) {
    if !output.exists {
        return;
    }
    let source = match fs::read_to_string(&output.resolved_path) {
        Ok(source) => source,
        Err(error) => {
            if manifest.failure_reason.is_none() {
                manifest.failure_reason = Some(format!(
                    "case manifest `{}` could not be read: {error}",
                    output.path
                ));
            }
            return;
        }
    };
    let value = match serde_json::from_str::<Value>(&source) {
        Ok(value) => value,
        Err(error) => {
            if manifest.failure_reason.is_none() {
                manifest.failure_reason = Some(format!(
                    "case manifest `{}` could not be parsed: {error}",
                    output.path
                ));
            }
            return;
        }
    };

    if let Some(case_id) = json_field_string(&value, "case_id") {
        if !case_id.is_empty() && case_id != manifest.case_id {
            return;
        }
    }
    if let Some(sample_row_hash) = json_field_string(&value, "sample_row_hash") {
        manifest.sample_row_hash = sample_row_hash;
    }
    if let Some(case_dir) = json_field_string(&value, "case_dir") {
        manifest.case_dir = Some(case_dir);
    }
    if let Some(path) = value
        .get("generated_input_file")
        .and_then(case_manifest_file_path)
    {
        manifest.generated_input_file = Some(path);
    }
    if let Some(processes) = value.get("processes").and_then(Value::as_array) {
        for process in processes {
            let name = json_field_string(process, "name").unwrap_or_default();
            let command = json_field_string(process, "command").unwrap_or_default();
            let status = json_field_string(process, "status").unwrap_or_default();
            if !name.is_empty() || !command.is_empty() || !status.is_empty() {
                push_unique_case_process_status(
                    &mut manifest.process_statuses,
                    RuntimeCaseProcessStatus {
                        name: name.clone(),
                        command,
                        status: status.clone(),
                    },
                );
                if !case_process_status_is_success(&status) && manifest.failure_reason.is_none() {
                    manifest.failure_reason = Some(format!(
                        "case process `{}` reported status `{}`",
                        name, status
                    ));
                }
            }
        }
    }
    if let Some(files) = value.get("result_files").and_then(Value::as_array) {
        for file in files {
            if let Some(path) = case_manifest_file_path(file) {
                push_unique_string(&mut manifest.result_files, path);
            }
        }
    }
    if let Some(metrics) = value.get("metrics").and_then(Value::as_object) {
        let mut names = metrics.keys().cloned().collect::<Vec<_>>();
        names.sort();
        for name in names {
            if let Some(value) = metrics.get(&name).and_then(Value::as_f64) {
                push_unique_case_metric(&mut manifest.metrics, RuntimeCaseMetric { name, value });
            }
        }
    }
    if let Some(failure_reason) = value.get("failure_reason") {
        if let Some(reason) = failure_reason.as_str() {
            if !reason.is_empty() {
                manifest.failure_reason = Some(reason.to_owned());
            }
        } else if !failure_reason.is_null() {
            manifest.failure_reason = Some(failure_reason.to_string());
        }
    }
}

fn finalize_case_manifest_status(manifest: &mut RuntimeCaseManifest) {
    if manifest.failure_reason.is_some() {
        manifest.status = "case_failed".to_owned();
    } else if manifest.status == "sample_row_manifest_seed"
        && (manifest.case_dir.is_some()
            || manifest.generated_input_file.is_some()
            || !manifest.process_bindings.is_empty()
            || !manifest.process_statuses.is_empty()
            || !manifest.output_artifacts.is_empty()
            || !manifest.result_files.is_empty()
            || !manifest.metrics.is_empty())
    {
        manifest.status = "case_materialized".to_owned();
    }
}

fn push_case_process_statuses_json(json: &mut String, processes: &[RuntimeCaseProcessStatus]) {
    for (index, process) in processes.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str("{");
        json.push_str(&format!("\"name\": \"{}\"", json_escape(&process.name)));
        json.push_str(&format!(
            ", \"command\": \"{}\"",
            json_escape(&process.command)
        ));
        json.push_str(&format!(
            ", \"status\": \"{}\"",
            json_escape(&process.status)
        ));
        json.push_str("}");
    }
}

fn push_case_metrics_json(json: &mut String, metrics: &[RuntimeCaseMetric]) {
    for (index, metric) in metrics.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!(
            "{{\"name\": \"{}\", \"value\": {}}}",
            json_escape(&metric.name),
            format_number_with_precision(metric.value, Some(8))
        ));
    }
}

fn push_unique_case_process_status(
    processes: &mut Vec<RuntimeCaseProcessStatus>,
    process: RuntimeCaseProcessStatus,
) {
    if let Some(existing) = processes
        .iter_mut()
        .find(|existing| existing.name == process.name)
    {
        *existing = process;
    } else {
        processes.push(process);
    }
}

fn push_unique_case_metric(metrics: &mut Vec<RuntimeCaseMetric>, metric: RuntimeCaseMetric) {
    if let Some(existing) = metrics
        .iter_mut()
        .find(|existing| existing.name == metric.name)
    {
        *existing = metric;
    } else {
        metrics.push(metric);
    }
}

fn push_unique_string(values: &mut Vec<String>, value: String) {
    if !value.is_empty() && !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn case_manifest_file_path(value: &Value) -> Option<String> {
    if let Some(path) = value.as_str() {
        return Some(path.to_owned());
    }
    json_field_string(value, "path")
}

fn output_path_matches_case(path: &str, case_id: &str) -> bool {
    output_path_segments(path)
        .into_iter()
        .any(|segment| segment == case_id)
}

fn infer_case_dir_from_output_path(path: &str, case_id: &str) -> Option<String> {
    let segments = output_path_segments(path);
    let index = segments.iter().position(|segment| *segment == case_id)?;
    Some(segments[..=index].join("/"))
}

fn output_path_segments(path: &str) -> Vec<&str> {
    path.split(|ch| ch == '/' || ch == '\\')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn output_file_name(path: &str) -> String {
    output_path_segments(path)
        .last()
        .copied()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn is_case_manifest_output(path: &str) -> bool {
    let file_name = output_file_name(path);
    file_name == "case_manifest.json"
        || (file_name.contains("case_manifest") && file_name.ends_with(".json"))
}

fn is_case_generated_input_output(path: &str) -> bool {
    let file_name = output_file_name(path);
    file_name == "input.txt" || file_name.starts_with("input.")
}

fn is_case_result_file_output(path: &str) -> bool {
    let file_name = output_file_name(path);
    file_name == "result.json" || file_name.starts_with("result.")
}

fn process_command_line(process: &ProcessExecutionRecord) -> String {
    if process.args.is_empty() {
        process.command.clone()
    } else {
        format!("{} {}", process.command, process.args.join(" "))
    }
}

fn case_process_status_is_success(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "" | "ok" | "passed" | "success" | "succeeded" | "completed"
    )
}

fn model_cards_json(runtime_data: &RuntimeData) -> String {
    let mut json = String::new();
    let mut emitted = 0usize;
    for artifact in runtime_data
        .ml_artifacts
        .iter()
        .filter(|artifact| artifact.model_card.is_some())
    {
        if emitted > 0 {
            json.push_str(",\n");
        }
        emitted += 1;
        let model_kind = artifact.algorithm.as_deref().unwrap_or(&artifact.kind);
        let residual_plot = if artifact.residual_points.is_empty() {
            None
        } else {
            Some("residual_points")
        };
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&artifact.binding)
        ));
        push_optional_json_string(&mut json, "source", artifact.source.as_deref(), 8);
        json.push_str(&format!(
            "        \"model_kind\": \"{}\",\n",
            json_escape(model_kind)
        ));
        json.push_str("        \"features\": [");
        push_json_string_array(&mut json, &artifact.features);
        json.push_str("],\n");
        push_optional_json_string(&mut json, "target", artifact.target.as_deref(), 8);
        push_optional_json_string(
            &mut json,
            "target_quantity",
            artifact.target_quantity.as_deref(),
            8,
        );
        json.push_str(&format!(
            "        \"target_unit\": \"{}\",\n",
            json_escape(&artifact.display_unit)
        ));
        push_optional_json_string(
            &mut json,
            "test_fraction",
            artifact.test_fraction.as_deref(),
            8,
        );
        push_optional_json_usize(&mut json, "train_count", artifact.train_count, 8);
        push_optional_json_usize(&mut json, "test_count", artifact.test_count, 8);
        json.push_str("        \"metrics\": {\n");
        json.push_str(&format!(
            "          \"rmse\": {},\n",
            optional_json_number(artifact.rmse)
        ));
        json.push_str(&format!(
            "          \"mae\": {},\n",
            optional_json_number(artifact.mae)
        ));
        json.push_str(&format!(
            "          \"r2\": {}\n",
            optional_json_number(artifact.r2)
        ));
        json.push_str("        },\n");
        push_optional_json_string(&mut json, "residual_plot", residual_plot, 8);
        json.push_str(&format!(
            "        \"residual_point_count\": {},\n",
            artifact.residual_points.len()
        ));
        push_optional_json_string(
            &mut json,
            "training_data_hash",
            artifact.training_data_hash.as_deref(),
            8,
        );
        push_optional_json_string(
            &mut json,
            "model_artifact_hash",
            artifact.model_artifact_hash.as_deref(),
            8,
        );
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&artifact.status)
        ));
        json.push_str(&format!("        \"line\": {}\n", artifact.line));
        json.push_str("      }");
    }
    json
}

fn db_manifests_json(records: &[DbManifestRecord]) -> String {
    let mut json = String::new();
    for (index, record) in records.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&record.binding)
        ));
        json.push_str(&format!(
            "        \"manifest_path\": \"{}\",\n",
            json_escape(&record.manifest_path)
        ));
        json.push_str(&format!(
            "        \"resolved_path\": \"{}\",\n",
            json_escape(&record.resolved_path)
        ));
        push_optional_json_string(&mut json, "hash", record.hash.as_deref(), 8);
        push_optional_json_string(&mut json, "database", record.database.as_deref(), 8);
        push_optional_json_string(
            &mut json,
            "transaction_status",
            record.transaction_status.as_deref(),
            8,
        );
        push_optional_json_string(
            &mut json,
            "schema_status",
            record.schema_status.as_deref(),
            8,
        );
        json.push_str("        \"tables\": [\n");
        for (table_index, table) in record.tables.iter().enumerate() {
            if table_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"name\": \"{}\",\n",
                json_escape(&table.name)
            ));
            json.push_str(&format!(
                "            \"mode\": \"{}\",\n",
                json_escape(&table.mode)
            ));
            json.push_str("            \"key\": [");
            push_json_string_array(&mut json, &table.key);
            json.push_str("],\n");
            json.push_str("            \"schema\": [");
            push_json_string_array(&mut json, &table.schema);
            json.push_str("],\n");
            match table.row_count {
                Some(row_count) => {
                    json.push_str(&format!("            \"row_count\": {}\n", row_count))
                }
                None => json.push_str("            \"row_count\": null\n"),
            }
            json.push_str("          }");
        }
        json.push_str("\n        ],\n");
        json.push_str(&format!(
            "        \"status\": \"{}\"\n",
            json_escape(&record.status)
        ));
        json.push_str("      }");
    }
    json
}

fn table_selections_json(runtime_data: &RuntimeData, indent: &str) -> String {
    let mut json = String::new();
    for (index, selection) in runtime_data.table_selections.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        push_table_selection_json(&mut json, selection, indent);
    }
    json
}

fn push_table_selection_json(
    json: &mut String,
    selection: &runtime_data::RuntimeTableSelection,
    indent: &str,
) {
    let field_indent = format!("{indent}  ");
    let nested_indent = format!("{indent}    ");
    json.push_str(&format!("{indent}{{\n"));
    json.push_str(&format!(
        "{field_indent}\"binding\": \"{}\",\n",
        json_escape(&selection.binding)
    ));
    json.push_str(&format!(
        "{field_indent}\"source_table\": \"{}\",\n",
        json_escape(&selection.source_table)
    ));
    json.push_str(&format!(
        "{field_indent}\"return_column\": \"{}\",\n",
        json_escape(&selection.return_column)
    ));
    push_optional_json_string(
        json,
        "selected_value",
        selection.selected_value.as_deref(),
        field_indent.len(),
    );
    match selection.selected_row_index {
        Some(row) => json.push_str(&format!("{field_indent}\"selected_row_index\": {},\n", row)),
        None => json.push_str(&format!("{field_indent}\"selected_row_index\": null,\n")),
    }
    json.push_str(&format!(
        "{field_indent}\"matched_count\": {},\n",
        selection.matched_count
    ));
    json.push_str(&format!("{field_indent}\"filters\": [\n"));
    for (filter_index, filter) in selection.filters.iter().enumerate() {
        if filter_index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{nested_indent}{{\n"));
        json.push_str(&format!(
            "{nested_indent}  \"column\": \"{}\",\n",
            json_escape(&filter.column)
        ));
        json.push_str(&format!(
            "{nested_indent}  \"operator\": \"{}\",\n",
            json_escape(&filter.operator)
        ));
        json.push_str(&format!(
            "{nested_indent}  \"value\": \"{}\",\n",
            json_escape(&filter.value)
        ));
        json.push_str(&format!(
            "{nested_indent}  \"matched\": {}\n",
            filter.matched
        ));
        json.push_str(&format!("{nested_indent}}}"));
    }
    json.push_str(&format!("\n{field_indent}],\n"));
    json.push_str(&format!("{field_indent}\"selected_row\": [\n"));
    for (value_index, value) in selection.selected_row.iter().enumerate() {
        if value_index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{nested_indent}{{\n"));
        json.push_str(&format!(
            "{nested_indent}  \"column\": \"{}\",\n",
            json_escape(&value.column)
        ));
        json.push_str(&format!(
            "{nested_indent}  \"value\": \"{}\"\n",
            json_escape(&value.value)
        ));
        json.push_str(&format!("{nested_indent}}}"));
    }
    json.push_str(&format!("\n{field_indent}],\n"));
    json.push_str(&format!(
        "{field_indent}\"status\": \"{}\",\n",
        json_escape(&selection.status)
    ));
    json.push_str(&format!(
        "{field_indent}\"reason\": \"{}\",\n",
        json_escape(&selection.reason)
    ));
    json.push_str(&format!("{field_indent}\"line\": {}\n", selection.line));
    json.push_str(&format!("{indent}}}"));
}

fn timeseries_coverage_json(runtime_data: &RuntimeData, indent: &str) -> String {
    let mut json = String::new();
    for (index, coverage) in runtime_data.timeseries_coverage.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        push_timeseries_coverage_json(&mut json, coverage, indent);
    }
    json
}

fn push_timeseries_coverage_json(
    json: &mut String,
    coverage: &runtime_data::RuntimeTimeSeriesCoverage,
    indent: &str,
) {
    let field_indent = format!("{indent}  ");
    let nested_indent = format!("{indent}    ");
    json.push_str(&format!("{indent}{{\n"));
    json.push_str(&format!(
        "{field_indent}\"binding\": \"{}\",\n",
        json_escape(&coverage.binding)
    ));
    json.push_str(&format!(
        "{field_indent}\"name\": \"{}\",\n",
        json_escape(&coverage.name)
    ));
    json.push_str(&format!(
        "{field_indent}\"source_table\": \"{}\",\n",
        json_escape(&coverage.source_table)
    ));
    json.push_str(&format!(
        "{field_indent}\"source_column\": \"{}\",\n",
        json_escape(&coverage.source_column)
    ));
    json.push_str(&format!(
        "{field_indent}\"unit\": \"{}\",\n",
        json_escape(&coverage.unit)
    ));
    push_optional_json_number(json, "start", coverage.start, field_indent.len());
    push_optional_json_number(json, "end", coverage.end, field_indent.len());
    push_optional_json_string(
        json,
        "source_start",
        coverage.source_start.as_deref(),
        field_indent.len(),
    );
    push_optional_json_string(
        json,
        "source_end",
        coverage.source_end.as_deref(),
        field_indent.len(),
    );
    push_optional_json_number(
        json,
        "expected_step",
        coverage.expected_step,
        field_indent.len(),
    );
    match coverage.expected_count {
        Some(count) => json.push_str(&format!("{field_indent}\"expected_count\": {},\n", count)),
        None => json.push_str(&format!("{field_indent}\"expected_count\": null,\n")),
    }
    json.push_str(&format!(
        "{field_indent}\"actual_count\": {},\n",
        coverage.actual_count
    ));
    json.push_str(&format!(
        "{field_indent}\"missing_count\": {},\n",
        coverage.missing_count
    ));
    json.push_str(&format!("{field_indent}\"missing_intervals\": [\n"));
    for (interval_index, interval) in coverage.missing_intervals.iter().enumerate() {
        if interval_index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{nested_indent}{{\n"));
        json.push_str(&format!(
            "{nested_indent}  \"start\": {},\n",
            interval.start
        ));
        json.push_str(&format!("{nested_indent}  \"end\": {},\n", interval.end));
        json.push_str(&format!(
            "{nested_indent}  \"missing_count\": {}\n",
            interval.missing_count
        ));
        json.push_str(&format!("{nested_indent}}}"));
    }
    json.push_str(&format!("\n{field_indent}],\n"));
    push_optional_json_number(json, "max_gap", coverage.max_gap, field_indent.len());
    match coverage.coverage_year {
        Some(year) => json.push_str(&format!("{field_indent}\"coverage_year\": {},\n", year)),
        None => json.push_str(&format!("{field_indent}\"coverage_year\": null,\n")),
    }
    json.push_str(&format!(
        "{field_indent}\"leap_year_policy\": \"{}\",\n",
        json_escape(&coverage.leap_year_policy)
    ));
    json.push_str(&format!(
        "{field_indent}\"status\": \"{}\",\n",
        json_escape(&coverage.status)
    ));
    json.push_str(&format!("{field_indent}\"line\": {}\n", coverage.line));
    json.push_str(&format!("{indent}}}"));
}

fn structured_reads_json(runtime_data: &RuntimeData) -> String {
    let mut json = String::new();
    for (index, read) in runtime_data.structured_reads.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&read.binding)
        ));
        json.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&read.kind)
        ));
        json.push_str(&format!(
            "        \"path\": \"{}\",\n",
            json_escape(&read.path)
        ));
        push_optional_json_string(&mut json, "source_hash", read.source_hash.as_deref(), 8);
        json.push_str(&format!(
            "        \"parse_status\": \"{}\",\n",
            json_escape(&read.parse_status)
        ));
        json.push_str(&format!(
            "        \"root_type\": \"{}\",\n",
            json_escape(&read.root_type)
        ));
        match read.field_count {
            Some(count) => json.push_str(&format!("        \"field_count\": {},\n", count)),
            None => json.push_str("        \"field_count\": null,\n"),
        }
        match read.item_count {
            Some(count) => json.push_str(&format!("        \"item_count\": {},\n", count)),
            None => json.push_str("        \"item_count\": null,\n"),
        }
        push_optional_json_string(&mut json, "error", read.error.as_deref(), 8);
        json.push_str(&format!("        \"line\": {}\n", read.line));
        json.push_str("      }");
    }
    json
}

fn table_diagnostics_json(runtime_data: &RuntimeData) -> String {
    let mut json = String::new();
    for (index, diagnostic) in runtime_data.table_diagnostics.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&diagnostic.binding)
        ));
        json.push_str(&format!(
            "        \"schema_name\": \"{}\",\n",
            json_escape(&diagnostic.schema_name)
        ));
        json.push_str(&format!(
            "        \"source\": \"{}\",\n",
            json_escape(&diagnostic.source)
        ));
        push_optional_json_string(
            &mut json,
            "source_hash",
            diagnostic.source_hash.as_deref(),
            8,
        );
        json.push_str(&format!(
            "        \"row_count\": {},\n",
            diagnostic.row_count
        ));
        json.push_str(&format!(
            "        \"column_count\": {},\n",
            diagnostic.column_count
        ));
        json.push_str(&format!(
            "        \"missing_cell_count\": {},\n",
            diagnostic.missing_cell_count
        ));
        json.push_str(&format!(
            "        \"parse_failure_count\": {},\n",
            diagnostic.parse_failure_count
        ));
        json.push_str(&format!(
            "        \"conversion_failure_count\": {},\n",
            diagnostic.conversion_failure_count
        ));
        json.push_str("        \"columns\": [\n");
        for (column_index, column) in diagnostic.columns.iter().enumerate() {
            if column_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"name\": \"{}\",\n",
                json_escape(&column.name)
            ));
            json.push_str(&format!(
                "            \"type\": \"{}\",\n",
                json_escape(&column.type_name)
            ));
            push_optional_json_string(&mut json, "unit", column.unit.as_deref(), 12);
            push_optional_json_string(
                &mut json,
                "canonical_unit",
                column.canonical_unit.as_deref(),
                12,
            );
            json.push_str(&format!("            \"is_index\": {},\n", column.is_index));
            json.push_str(&format!("            \"len\": {},\n", column.len));
            json.push_str(&format!(
                "            \"missing_count\": {},\n",
                column.missing_count
            ));
            json.push_str(&format!(
                "            \"conversion_failure_count\": {},\n",
                column.conversion_failure_count
            ));
            json.push_str(&format!(
                "            \"status\": \"{}\"\n",
                json_escape(&column.status)
            ));
            json.push_str("          }");
        }
        json.push_str("\n        ],\n");
        json.push_str("        \"time_axis\": ");
        if let Some(axis) = &diagnostic.time_axis {
            json.push_str("{\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&axis.name)
            ));
            json.push_str(&format!(
                "          \"source_column\": \"{}\",\n",
                json_escape(&axis.source_column)
            ));
            json.push_str(&format!(
                "          \"unit\": \"{}\",\n",
                json_escape(&axis.unit)
            ));
            push_optional_json_number(&mut json, "start", axis.start, 10);
            push_optional_json_number(&mut json, "end", axis.end, 10);
            json.push_str(&format!("          \"count\": {},\n", axis.count));
            push_optional_json_number(&mut json, "nominal_step", axis.nominal_step, 10);
            json.push_str(&format!("          \"irregular\": {},\n", axis.irregular));
            json.push_str(&format!(
                "          \"missing_count\": {},\n",
                axis.missing_count
            ));
            json.push_str(&format!(
                "          \"coverage_status\": \"{}\"\n",
                json_escape(&axis.coverage_status)
            ));
            json.push_str("        },\n");
        } else {
            json.push_str("null,\n");
        }
        json.push_str(&format!(
            "        \"status\": \"{}\"\n",
            json_escape(&diagnostic.status)
        ));
        json.push_str("      }");
    }
    json
}

fn push_runtime_columns(json: &mut String, table: &runtime_data::RuntimeTable) {
    for (index, column) in table.columns.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("          {\n");
        json.push_str(&format!(
            "            \"name\": \"{}\",\n",
            json_escape(&column.name)
        ));
        json.push_str(&format!(
            "            \"type\": \"{}\",\n",
            json_escape(&column.type_name)
        ));
        if let Some(unit) = &column.unit {
            json.push_str(&format!(
                "            \"unit\": \"{}\",\n",
                json_escape(unit)
            ));
        } else {
            json.push_str("            \"unit\": null,\n");
        }
        if let Some(unit) = &column.canonical_unit {
            json.push_str(&format!(
                "            \"canonical_unit\": \"{}\",\n",
                json_escape(unit)
            ));
        } else {
            json.push_str("            \"canonical_unit\": null,\n");
        }
        json.push_str(&format!("            \"is_index\": {},\n", column.is_index));
        json.push_str(&format!("            \"len\": {},\n", column.len()));
        json.push_str(&format!(
            "            \"missing_count\": {},\n",
            column.missing_count
        ));
        json.push_str("            \"values\": [");
        match &column.values {
            RuntimeValues::Text(values) => {
                for (value_index, value) in values.iter().enumerate() {
                    if value_index > 0 {
                        json.push_str(", ");
                    }
                    json.push_str(&format!("\"{}\"", json_escape(value)));
                }
            }
            RuntimeValues::Number(values) => {
                for (value_index, value) in values.iter().enumerate() {
                    if value_index > 0 {
                        json.push_str(", ");
                    }
                    if let Some(value) = value {
                        json.push_str(&value.to_string());
                    } else {
                        json.push_str("null");
                    }
                }
            }
        }
        json.push_str("],\n");
        json.push_str("            \"canonical_values\": [");
        for (value_index, value) in column.canonical_values.iter().enumerate() {
            if value_index > 0 {
                json.push_str(", ");
            }
            if let Some(value) = value {
                json.push_str(&value.to_string());
            } else {
                json.push_str("null");
            }
        }
        json.push_str("],\n");
        json.push_str("            \"conversion_failures\": [\n");
        for (failure_index, failure) in column.conversion_failures.iter().enumerate() {
            if failure_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("              {\n");
            json.push_str(&format!("                \"row\": {},\n", failure.row));
            json.push_str(&format!(
                "                \"column\": \"{}\",\n",
                json_escape(&failure.column)
            ));
            json.push_str(&format!(
                "                \"value\": \"{}\",\n",
                json_escape(&failure.value)
            ));
            json.push_str(&format!(
                "                \"source_unit\": \"{}\",\n",
                json_escape(&failure.source_unit)
            ));
            json.push_str(&format!(
                "                \"target_unit\": \"{}\",\n",
                json_escape(&failure.target_unit)
            ));
            json.push_str(&format!(
                "                \"message\": \"{}\"\n",
                json_escape(&failure.message)
            ));
            json.push_str("              }");
        }
        json.push_str("\n            ]\n");
        json.push_str("          }");
    }
}

fn push_parse_failures(json: &mut String, table: &runtime_data::RuntimeTable) {
    for (index, failure) in table.parse_failures.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("          {\n");
        json.push_str(&format!("            \"row\": {},\n", failure.row));
        json.push_str(&format!(
            "            \"column\": \"{}\",\n",
            json_escape(&failure.column)
        ));
        json.push_str(&format!(
            "            \"value\": \"{}\",\n",
            json_escape(&failure.value)
        ));
        json.push_str(&format!(
            "            \"message\": \"{}\"\n",
            json_escape(&failure.message)
        ));
        json.push_str("          }");
    }
}

fn push_runtime_points(json: &mut String, points: &[runtime_data::RuntimePoint]) {
    for (index, point) in points.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!("[{}, {}]", point.x, point.y));
    }
}

fn hash_text(source: &str) -> String {
    hash_bytes(source.as_bytes())
}

fn hash_bytes(source: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in source {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn push_profile_diagnostics_json(
    json: &mut String,
    diagnostics: &[ProfileDiagnostic],
    indent: &str,
) {
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}{{\n"));
        json.push_str(&format!(
            "{indent}  \"severity\": \"{}\",\n",
            json_escape(diagnostic.severity)
        ));
        json.push_str(&format!(
            "{indent}  \"code\": \"{}\",\n",
            json_escape(diagnostic.code)
        ));
        json.push_str(&format!(
            "{indent}  \"message\": \"{}\",\n",
            json_escape(&diagnostic.message)
        ));
        json.push_str(&format!("{indent}  \"line\": {}\n", diagnostic.line));
        json.push_str(&format!("{indent}}}"));
    }
}

fn optional_json_number(value: Option<f64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn push_optional_json_number(json: &mut String, key: &str, value: Option<f64>, indent: usize) {
    let spaces = " ".repeat(indent);
    match value {
        Some(value) => json.push_str(&format!("{spaces}\"{key}\": {value},\n")),
        None => json.push_str(&format!("{spaces}\"{key}\": null,\n")),
    }
}

fn push_optional_json_f64(json: &mut String, key: &str, value: Option<f64>, indent: usize) {
    let spaces = " ".repeat(indent);
    match value {
        Some(value) => json.push_str(&format!(
            "{spaces}\"{key}\": {},\n",
            format_number_with_precision(value, Some(8))
        )),
        None => json.push_str(&format!("{spaces}\"{key}\": null,\n")),
    }
}
fn push_optional_json_usize(json: &mut String, key: &str, value: Option<usize>, indent: usize) {
    let spaces = " ".repeat(indent);
    match value {
        Some(value) => json.push_str(&format!("{spaces}\"{key}\": {value},\n")),
        None => json.push_str(&format!("{spaces}\"{key}\": null,\n")),
    }
}

fn push_optional_json_string(json: &mut String, key: &str, value: Option<&str>, indent: usize) {
    let spaces = " ".repeat(indent);
    match value {
        Some(value) => json.push_str(&format!("{spaces}\"{key}\": \"{}\",\n", json_escape(value))),
        None => json.push_str(&format!("{spaces}\"{key}\": null,\n")),
    }
}

fn push_json_f64_array(json: &mut String, values: &[f64]) {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format_number_with_precision(*value, Some(8)));
    }
}

fn push_json_string_array(json: &mut String, values: &[String]) {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!("\"{}\"", json_escape(value)));
    }
}

fn push_uncertainty_propagation_terms(
    json: &mut String,
    terms: &[eng_report::ReportUncertaintyPropagationTerm],
) {
    for (index, term) in terms.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!(
            "{{\"source\": \"{}\", \"role\": \"{}\", \"quantity_kind\": \"{}\"}}",
            json_escape(&term.source),
            json_escape(&term.role),
            json_escape(&term.quantity_kind)
        ));
    }
}

fn open_path(path: &Path) {
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("cmd")
            .args(["/C", "start", "", &path.display().to_string()])
            .status();
    }

    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(path).status();
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = Command::new("xdg-open").arg(path).status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_file_prints_and_writes_explicit_summary_csv_export() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-print-export");
        let build_root = repo_root.join("build").join("runtime-print-export-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "schema SensorData {\n    time: DateTime index\n    T_supply: AbsoluteTemperature [degC]\n    T_return: AbsoluteTemperature [degC]\n    m_dot: MassFlowRate [kg/s]\n}\n\nargs {\n    input: CsvFile = file(\"../../examples/official/01_csv_plot/data/sensor.csv\")\n}\n\nsensor = promote csv args.input as SensorData\ncp = 4180 J/kg/K\nQ_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\nE_coil = integrate(Q_coil, over=Time)\nmean_Q = mean(Q_coil, axis=Time)\n\nprint \"Loaded {sensor.rows} rows from {args.input}\"\nlog info \"Q mean = {mean(Q_coil, axis=Time): .2 kW}\"\nlog warn \"E total = {E_coil: .2 kWh}\"\n\nexport summary to csv \"summary.csv\" {\n    E_coil as kWh with \".2\"\n    mean_Q as kW with \".2\"\n}\nwith {\n    overwrite = true\n}\nwrite text \"summary.txt\", mean_Q\nwrite json \"energy.json\", E_coil\n",
        )
        .expect("write source");

        let output = run_file(&source_path, &build_root, &RunOptions::default()).expect("run file");
        let second_output =
            run_file(&source_path, &build_root, &RunOptions::default()).expect("run file again");

        assert!(output.stdout.contains("Loaded 4 rows"));
        assert!(output.stdout.contains("[info] Q mean = "));
        assert!(output.stdout.contains(" kW"));
        assert!(output.stdout.contains("[warn] E total = "));
        assert!(output
            .run_log_json
            .contains("\"format\": \"eng-run-log-v1\""));
        assert!(output.run_log_json.contains("\"level\": \"info\""));
        assert!(output.run_log_json.contains("\"level\": \"warn\""));
        assert_eq!(output.csv_export_paths.len(), 1);
        assert_eq!(output.write_output_paths.len(), 2);
        assert!(!output.artifacts_saved);
        let csv =
            fs::read_to_string(build_root.join("result").join("summary.csv")).expect("summary csv");
        assert!(csv.contains("E_coil [kWh]"));
        assert!(csv.contains("mean_Q [kW]"));
        assert_eq!(csv.lines().count(), 2);
        let text =
            fs::read_to_string(build_root.join("result").join("summary.txt")).expect("summary txt");
        assert!(text.contains('W'));
        let json =
            fs::read_to_string(build_root.join("result").join("energy.json")).expect("energy json");
        assert!(json.contains("\"quantity_kind\": \"Energy\""));
        assert!(output
            .output_manifest_json
            .contains("\"format\": \"eng-output-manifest-v1\""));
        assert!(output
            .output_manifest_json
            .contains("\"kind\": \"csv_export\""));
        assert!(output
            .output_manifest_json
            .contains("\"kind\": \"write_text\""));
        assert!(output
            .output_manifest_json
            .contains("\"artifact_registry\""));
        assert!(output.output_manifest_json.contains("\"source_files\""));
        assert!(output.output_manifest_json.contains("\"generated_files\""));
        assert!(output
            .review_json
            .contains("\"provenance\": \"runtime_artifact_record\""));
        assert!(output
            .review_json
            .contains("\"artifact_kind\": \"csv_export\""));
        assert!(output
            .review_json
            .contains("\"artifact_kind\": \"write_text\""));
        assert!(output.review_json.contains("\"hash\": \""));
        assert!(output.review_json.contains("\"rule\": \"content_hash\""));
        assert!(output.output_manifest_path.exists());
        assert_eq!(second_output.csv_export_paths.len(), 1);
    }

    #[test]
    fn run_source_resolves_imports_relative_to_virtual_path() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-virtual-source");
        let build_root = repo_root
            .join("build")
            .join("runtime-virtual-source-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        fs::write(
            source_dir.join("thermal.eng"),
            "const UA_default: Conductance [W/K] = 150 W/K\n\nfn heat_loss(UA: Conductance [W/K], dT: TemperatureDelta [K]) -> HeatRate [W] {\n    return UA * dT\n}\n",
        )
        .expect("write import");
        let virtual_path = source_dir.join("__ide_terminal__.eng");
        let source =
            "use \"thermal.eng\"\n\nQ = heat_loss(UA_default, 8 K)\nprint \"Q = {Q: .2 kW}\"\n";

        let output =
            run_source(&virtual_path, source, &build_root, &RunOptions::default()).expect("run");

        assert!(output.stdout.contains("Q = 1.20 kW"));
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_source_prints_expression_command() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-expression-print");
        let build_root = repo_root
            .join("build")
            .join("runtime-expression-print-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let virtual_path = source_dir.join("__ide_terminal__.eng");

        let output = run_source(
            &virtual_path,
            "Q = 10 kW\nprint Q: .1 kW\n",
            &build_root,
            &RunOptions::default(),
        )
        .expect("run");

        assert_eq!(output.stdout.trim(), "10.0 kW");
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_source_accepts_terminal_scalar_assignment() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-terminal-scalar");
        let build_root = repo_root
            .join("build")
            .join("runtime-terminal-scalar-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let virtual_path = source_dir.join("__ide_terminal__.eng");

        let output =
            run_source(&virtual_path, "x =3\n", &build_root, &RunOptions::default()).expect("run");

        assert!(output.result_json.contains("\"scalar_count\": 1"));
        assert!(output.result_json.contains("\"name\": \"x\""));
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_source_accepts_terminal_explicit_declaration() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-terminal-explicit");
        let build_root = repo_root
            .join("build")
            .join("runtime-terminal-explicit-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let virtual_path = source_dir.join("__ide_terminal__.eng");

        let output = run_source(
            &virtual_path,
            "x: AbsoluteTemperature = 3 degC\n",
            &build_root,
            &RunOptions::default(),
        )
        .expect("run");

        assert!(output.result_json.contains("\"scalar_count\": 1"));
        assert!(output.result_json.contains("\"name\": \"x\""));
        assert!(output
            .result_json
            .contains("\"type\": \"AbsoluteTemperature\""));
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_source_records_numeric_uncertainty_payload() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-numeric-uncertainty");
        let build_root = repo_root
            .join("build")
            .join("runtime-numeric-uncertainty-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let virtual_path = source_dir.join("__ide_terminal__.eng");

        let output = run_source(
            &virtual_path,
            "Q = 10 kW\nQ_meas = measured(10 kW, std=1 kW)\n",
            &build_root,
            &RunOptions::default(),
        )
        .expect("run");

        assert!(output.result_json.contains("\"numeric_values\""));
        assert!(output
            .result_json
            .contains("\"representation\": \"Certain\""));
        assert!(output
            .result_json
            .contains("\"representation\": \"Measured\""));
        assert!(output
            .result_json
            .contains("\"status\": \"uncertainty_attached\""));
        assert!(output
            .result_json
            .contains("\"uncertainty_binding\": \"Q_meas\""));
        assert!(output.result_json.contains("\"stddev\": 1"));
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_source_materializes_table_diagnostics() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-table-diagnostics");
        let build_root = repo_root
            .join("build")
            .join("runtime-table-diagnostics-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("sensor.csv"),
            "time,T_zone\n2026-01-01T00:00:00Z,21\n2026-01-01T00:05:00Z,\n2026-01-01T00:20:00Z,23\n",
        )
        .expect("sensor csv");
        let virtual_path = source_dir.join("__ide_terminal__.eng");

        let output = run_source(
            &virtual_path,
            concat!(
                "schema SensorData {\n",
                "    time: DateTime index\n",
                "    T_zone: AbsoluteTemperature [degC]\n",
                "    missing {\n",
                "        T_zone: interpolate max_gap=20 min\n",
                "    }\n",
                "}\n\n",
                "args {\n",
                "    input: CsvFile = file(\"data/sensor.csv\")\n",
                "}\n\n",
                "sensor = promote csv args.input as SensorData\n",
                "print \"rows={sensor.rows}\"\n",
            ),
            &build_root,
            &RunOptions::default(),
        )
        .expect("run");

        assert!(output.result_json.contains("\"table_diagnostics\""));
        assert!(output.result_json.contains("\"binding\": \"sensor\""));
        assert!(output
            .result_json
            .contains("\"schema_name\": \"SensorData\""));
        assert!(output.result_json.contains("\"row_count\": 3"));
        assert!(output.result_json.contains("\"column_count\": 2"));
        assert!(output
            .result_json
            .contains("\"coverage_status\": \"missing_or_irregular\""));
        assert!(output
            .result_json
            .contains("\"status\": \"time_axis_irregular\""));
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_source_materializes_table_selection() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root
            .join("build")
            .join("runtime-table-selection-source");
        let build_root = repo_root
            .join("build")
            .join("runtime-table-selection-source-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("station.csv"),
            "region,station_id,valid_from,valid_to,latitude,longitude\ndemo,STN001,2020-01-01T00:00:00+09:00,2030-12-31T23:00:00+09:00,37.5,126.9\ndemo-east,STN002,2020-01-01T00:00:00+09:00,,35.1,129.0\n",
        )
        .expect("station csv");
        let virtual_path = source_dir.join("__ide_terminal__.eng");

        let output = run_source(
            &virtual_path,
            concat!(
                "schema StationMap {\n",
                "    region: String\n",
                "    station_id: String\n",
                "    valid_from: DateTime\n",
                "    valid_to: DateTime\n",
                "    latitude: DimensionlessNumber [1]\n",
                "    longitude: DimensionlessNumber [1]\n",
                "}\n\n",
                "args {\n",
                "    year: Int = 2024\n",
                "    region: String = \"demo\"\n",
                "    station_map: CsvFile = file(\"data/station.csv\")\n",
                "}\n\n",
                "stations = promote csv args.station_map as StationMap\n",
                "selected_station_id = select_first_row(stations, return_column=\"station_id\", region=args.region, start=date(args.year, 1, 1), end=date(args.year, 12, 31))\n",
                "print \"station={selected_station_id}\"\n",
                "report {\n",
                "    show selected_station_id\n",
                "}\n",
            ),
            &build_root,
            &RunOptions::default(),
        )
        .expect("run");

        assert!(output.stdout.contains("station=STN001"));
        assert!(output.result_json.contains("\"table_selections\""));
        assert!(output
            .result_json
            .contains("\"binding\": \"selected_station_id\""));
        assert!(output
            .result_json
            .contains("\"source_table\": \"stations\""));
        assert!(output
            .result_json
            .contains("\"selected_value\": \"STN001\""));
        assert!(output.result_json.contains("\"selected_row_index\": 1"));
        assert!(output.result_json.contains("\"matched_count\": 1"));
        assert!(output
            .result_json
            .contains("\"reason\": \"matched equality filters and validity period\""));
        assert!(output.review_json.contains("\"table_selections\""));
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_source_materializes_table_selection_artifacts() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-table-selection-run");
        let build_root = repo_root
            .join("build")
            .join("runtime-table-selection-run-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("station_map.csv"),
            concat!(
                "region,station_id,valid_from,valid_to,latitude,longitude\n",
                "demo,STN001,2020-01-01T00:00:00+09:00,2030-12-31T23:00:00+09:00,37.5665,126.9780\n",
                "demo-east,STN002,2020-01-01T00:00:00+09:00,2030-12-31T23:00:00+09:00,35.1796,129.0756\n",
            ),
        )
        .expect("station map csv");
        let virtual_path = source_dir.join("__ide_terminal__.eng");

        let output = run_source(
            &virtual_path,
            concat!(
                "schema StationMap {\n",
                "    region: String\n",
                "    station_id: String\n",
                "    valid_from: DateTime\n",
                "    valid_to: DateTime\n",
                "    latitude: DimensionlessNumber [1]\n",
                "    longitude: DimensionlessNumber [1]\n",
                "}\n\n",
                "args {\n",
                "    year: Int = 2024\n",
                "    region: String = \"demo\"\n",
                "    station_map: CsvFile = file(\"data/station_map.csv\")\n",
                "}\n\n",
                "stations = promote csv args.station_map as StationMap\n",
                "selected_station_id = select_first_row(stations, return_column=\"station_id\", region=args.region, start=date(args.year, 1, 1), end=date(args.year, 12, 31))\n",
                "print \"selected={selected_station_id}\"\n",
                "report {\n",
                "    show selected_station_id\n",
                "}\n",
            ),
            &build_root,
            &RunOptions::default(),
        )
        .expect("run");

        assert!(output.stdout.contains("selected=STN001"));
        assert!(output.result_json.contains("\"table_selections\""));
        assert!(output
            .result_json
            .contains("\"binding\": \"selected_station_id\""));
        assert!(output
            .result_json
            .contains("\"selected_value\": \"STN001\""));
        assert!(output
            .result_json
            .contains("matched equality filters and validity period"));
        assert!(output.review_json.contains("\"table_selections\""));
        assert!(output.review_json.contains("\"selected_row\""));
        assert!(output.report_html.contains("selected_station_id"));
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_source_materializes_sample_table_artifacts() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-sample-table");
        let build_root = repo_root.join("build").join("runtime-sample-table-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("samples")).expect("sample data dir");
        fs::write(
            source_dir.join("samples").join("design_samples.csv"),
            "case_id,people_density,cooling_cop,lighting_power_density\ncase_001,0.08,3.2,8.0\ncase_002,0.10,3.4,10.0\ncase_003,0.12,3.6,12.0\n",
        )
        .expect("sample csv");
        let virtual_path = source_dir.join("__ide_terminal__.eng");

        let output = run_source(
            &virtual_path,
            concat!(
                "schema DesignSample {\n",
                "    case_id: String\n",
                "    people_density: PeopleDensity [person/m2]\n",
                "    cooling_cop: Ratio [1]\n",
                "    lighting_power_density: Irradiance [W/m2]\n",
                "}\n\n",
                "args {\n",
                "    samples: CsvFile = file(\"samples/design_samples.csv\")\n",
                "}\n\n",
                "designs = promote csv args.samples as DesignSample\n",
                "print \"samples={designs.rows}\"\n",
            ),
            &build_root,
            &RunOptions::default(),
        )
        .expect("run");

        assert!(output.result_json.contains("\"sample_tables\""));
        assert!(output.result_json.contains("\"binding\": \"designs\""));
        assert!(output
            .result_json
            .contains("\"schema_name\": \"DesignSample\""));
        assert!(output.result_json.contains("\"sample_count\": 3"));
        assert!(output
            .result_json
            .contains("\"case_id_column\": \"case_id\""));
        assert!(output.result_json.contains("\"parameter_columns\""));
        assert!(output
            .result_json
            .contains("\"quantity_kind\": \"PeopleDensity\""));
        assert!(output
            .result_json
            .contains("\"display_unit\": \"person/m2\""));
        assert!(output.result_json.contains("\"row_hash_count\": 3"));
        assert!(output
            .result_json
            .contains("\"generation\": \"promoted_csv\""));
        assert!(output
            .result_json
            .contains("\"status\": \"promoted_sample_table\""));
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_source_materializes_case_manifest_seeds() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-case-manifest-seed");
        let build_root = repo_root
            .join("build")
            .join("runtime-case-manifest-seed-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("samples")).expect("sample data dir");
        fs::write(
            source_dir.join("samples").join("design_samples.csv"),
            "case_id,cooling_cop\ncase_001,3.2\ncase_002,3.4\n",
        )
        .expect("sample csv");
        let virtual_path = source_dir.join("__ide_terminal__.eng");

        let output = run_source(
            &virtual_path,
            concat!(
                "schema DesignSample {\n",
                "    case_id: String\n",
                "    cooling_cop: Ratio [1]\n",
                "}\n\n",
                "args {\n",
                "    samples: CsvFile = file(\"samples/design_samples.csv\")\n",
                "}\n\n",
                "designs = promote csv args.samples as DesignSample\n",
                "print \"samples={designs.rows}\"\n",
            ),
            &build_root,
            &RunOptions::default(),
        )
        .expect("run");

        assert!(output.result_json.contains("\"case_manifests\""));
        assert!(output.result_json.contains("\"case_id\": \"case_001\""));
        assert!(output.result_json.contains("\"sample_table\": \"designs\""));
        assert!(output.result_json.contains("\"sample_row_number\": 1"));
        assert!(output.result_json.contains("\"source_row\": 2"));
        assert!(output.result_json.contains("\"sample_row_hash\""));
        assert!(output.result_json.contains("\"case_dir\": null"));
        assert!(output
            .result_json
            .contains("\"generated_input_file\": null"));
        assert!(output.result_json.contains("\"process_statuses\": []"));
        assert!(output.result_json.contains("\"result_files\": []"));
        assert!(output.result_json.contains("\"metrics\": []"));
        assert!(output.result_json.contains("\"failure_reason\": null"));
        assert!(output
            .result_json
            .contains("\"status\": \"sample_row_manifest_seed\""));
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_file_enriches_case_manifest_from_expected_output() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root
            .join("build")
            .join("runtime-case-manifest-enrichment");
        let build_root = repo_root
            .join("build")
            .join("runtime-case-manifest-enrichment-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("samples")).expect("sample data dir");
        fs::create_dir_all(source_dir.join("outputs").join("case_001")).expect("case dir");
        fs::write(
            source_dir.join("samples").join("design_samples.csv"),
            "case_id,cooling_cop\ncase_001,3.2\n",
        )
        .expect("sample csv");
        fs::write(
            source_dir
                .join("outputs")
                .join("case_001")
                .join("input.txt"),
            "cop=3.2\n",
        )
        .expect("case input");
        fs::write(
            source_dir
                .join("outputs")
                .join("case_001")
                .join("result.json"),
            "{\"annual_electricity\":42.5}\n",
        )
        .expect("case result");
        fs::write(
            source_dir
                .join("outputs")
                .join("case_001")
                .join("case_manifest.json"),
            concat!(
                "{\n",
                "  \"case_id\": \"case_001\",\n",
                "  \"sample_row_hash\": \"external-row-hash\",\n",
                "  \"case_dir\": \"outputs/case_001\",\n",
                "  \"generated_input_file\": {\"path\": \"outputs/case_001/input.txt\", \"sha256\": \"input-hash\", \"bytes\": 8},\n",
                "  \"processes\": [\n",
                "    {\"name\": \"patch_input\", \"command\": \"python tools/patch_input.py\", \"status\": \"success\"},\n",
                "    {\"name\": \"external_simulation\", \"command\": \"python tools/run_external_sim.py\", \"status\": \"success\"}\n",
                "  ],\n",
                "  \"result_files\": [{\"path\": \"outputs/case_001/result.json\", \"sha256\": \"result-hash\", \"bytes\": 28}],\n",
                "  \"metrics\": {\"annual_electricity_kwh\": 42.5, \"peak_cooling_kw\": 7.25},\n",
                "  \"failure_reason\": null\n",
                "}\n",
            ),
        )
        .expect("case manifest");
        let source_path = source_dir.join("main.eng");
        let process_source = if cfg!(windows) {
            "case_manifest_result = run command \"cmd\"\nwith {\n    args = [\"/C\", \"echo\", \"case-manifest\"]\n    expected_outputs = [\"outputs/case_001/case_manifest.json\"]\n}\n"
        } else {
            "case_manifest_result = run command \"sh\"\nwith {\n    args = [\"-c\", \"printf case-manifest\"]\n    expected_outputs = [\"outputs/case_001/case_manifest.json\"]\n}\n"
        };
        fs::write(
            &source_path,
            format!(
                "{}{}",
                concat!(
                    "schema DesignSample {\n",
                    "    case_id: String\n",
                    "    cooling_cop: Ratio [1]\n",
                    "}\n\n",
                    "args {\n",
                    "    samples: CsvFile = file(\"samples/design_samples.csv\")\n",
                    "}\n\n",
                    "designs = promote csv args.samples as DesignSample\n\n",
                ),
                process_source,
            ),
        )
        .expect("write source");

        let output = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                save_artifacts: true,
                ..RunOptions::default()
            },
        )
        .expect("process run");

        assert!(output.result_json.contains("\"case_id\": \"case_001\""));
        assert!(output
            .result_json
            .contains("\"sample_row_hash\": \"external-row-hash\""));
        assert!(output
            .result_json
            .contains("\"case_dir\": \"outputs/case_001\""));
        assert!(output
            .result_json
            .contains("\"generated_input_file\": \"outputs/case_001/input.txt\""));
        assert!(output
            .result_json
            .contains("\"process_bindings\": [\"case_manifest_result\"]"));
        assert!(output.result_json.contains("\"process_statuses\""));
        assert!(output
            .result_json
            .contains("\"name\": \"external_simulation\""));
        assert!(output
            .result_json
            .contains("\"command\": \"python tools/run_external_sim.py\""));
        assert!(output
            .result_json
            .contains("\"result_files\": [\"outputs/case_001/result.json\"]"));
        assert!(output
            .result_json
            .contains("\"name\": \"annual_electricity_kwh\""));
        assert!(output.result_json.contains("\"value\": 42.5"));
        assert!(output.result_json.contains("\"failure_reason\": null"));
        assert!(output
            .result_json
            .contains("\"status\": \"case_materialized\""));
    }

    #[test]
    fn run_source_materializes_uncertainty_validation_results() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root
            .join("build")
            .join("runtime-uncertainty-validation");
        let build_root = repo_root
            .join("build")
            .join("runtime-uncertainty-validation-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let virtual_path = source_dir.join("__ide_terminal__.eng");

        let output = run_source(
            &virtual_path,
            concat!(
                "Q = normal(mean=5 kW, std=0.5 kW, samples=31)\n",
                "validate p95(Q) < 7 kW\n",
                "validate probability(Q < 7 kW) > 0.95\n",
                "validate mean(Q) between 4 kW and 6 kW\n",
            ),
            &build_root,
            &RunOptions::default(),
        )
        .expect("run");

        assert!(output.result_json.contains("\"validations\""));
        assert!(output.result_json.contains("\"left\": \"p95(Q)\""));
        assert!(output
            .result_json
            .contains("\"left\": \"probability(Q < 7 kW)\""));
        assert!(output.result_json.contains("\"operator\": \"between\""));
        assert_eq!(
            output.result_json.matches("\"status\": \"passed\"").count(),
            3
        );
        assert!(output.report_spec_json.contains("probability(Q < 7 kW)"));
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_file_executes_test_assert_and_golden_checks() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-test-assert-golden");
        let build_root = repo_root
            .join("build")
            .join("runtime-test-assert-golden-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("golden")).expect("golden dir");
        fs::write(
            source_dir.join("golden").join("summary.csv"),
            "Q [kW]\n10.0\n",
        )
        .expect("write golden");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "Q = 10 kW\n\nexport summary to csv \"summary.csv\" {\n    Q as kW with \".1\"\n}\nwith {\n    overwrite = true\n}\n\ntest \"summary values\" {\n    assert Q == 10 kW within 0.001 kW\n    assert Q > 5 kW\n    golden \"summary.csv\" matches file(\"golden/summary.csv\")\n}\n",
        )
        .expect("write source");

        let output = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                open_report: false,
                save_artifacts: true,
                args: Vec::new(),
                ..RunOptions::default()
            },
        )
        .expect("run file");

        assert!(output.review_json.contains("\"tests\""));
        assert!(output
            .test_results_json
            .contains("\"format\": \"eng-test-results-v1\""));
        assert!(output.test_results_json.contains("\"status\": \"passed\""));
        assert!(output.test_results_json.contains("\"goldens\""));
        assert!(output
            .output_manifest_json
            .contains("\"kind\": \"test_results\""));
        assert!(output.test_results_path.exists());
    }

    #[test]
    fn run_file_review_summarizes_adaptive_substeps() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_path = repo_root.join("examples/internal/27_adaptive_heun_thermal/main.eng");
        let build_root = repo_root
            .join("build")
            .join("runtime-adaptive-review-summary");
        let _ = fs::remove_dir_all(&build_root);

        let output = run_file(&source_path, &build_root, &RunOptions::default()).expect("run file");

        assert!(output.review_json.contains("\"substep_count\": "));
        assert!(output.review_json.contains("\"accepted_substep_count\": "));
        assert!(output.review_json.contains("\"rejected_substep_count\": "));
        assert!(output.review_json.contains("\"max_substep_error_norm\": "));
        assert!(!output
            .review_json
            .contains("\"max_substep_error_norm\": null"));
    }

    #[test]
    fn run_file_evaluates_imported_function_for_print_and_csv_export() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-function-import");
        let build_root = repo_root
            .join("build")
            .join("runtime-function-import-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        fs::write(
            source_dir.join("thermal.eng"),
            "fn heat_loss(UA: Conductance [W/K], dT: TemperatureDelta [K]) -> HeatRate [W] {\n    return UA * dT\n}\n",
        )
        .expect("write thermal");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "use \"thermal.eng\"\n\nUA_wall = 150 W/K\ndT_wall = 8 K\nQ_wall = heat_loss(UA_wall, dT_wall)\n\nprint \"Q wall = {Q_wall: .2 kW}\"\n\nexport summary to csv \"summary.csv\" {\n    Q_wall as kW with \".2\"\n}\n",
        )
        .expect("write source");

        let output = run_file(&source_path, &build_root, &RunOptions::default()).expect("run file");

        assert!(output.stdout.contains("Q wall = 1.20 kW"));
        let csv =
            fs::read_to_string(build_root.join("result").join("summary.csv")).expect("summary csv");
        assert!(csv.contains("Q_wall [kW]"));
        assert!(csv.contains("1.20"));
    }

    #[test]
    fn run_file_applies_file_operations_and_records_manifest() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-file-ops");
        let build_root = repo_root.join("build").join("runtime-file-ops-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        fs::write(source_dir.join("template.txt"), "template note").expect("template");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "copy file(\"template.txt\") to \"ops/copied.txt\"\nmove \"ops/copied.txt\" to \"ops/moved.txt\"\nwith {\n    confirm = true\n    overwrite = true\n}\nwrite text \"ops/scratch.txt\", \"remove me\"\ndelete \"ops/scratch.txt\"\nwith {\n    confirm = true\n}\n",
        )
        .expect("write source");

        let output = run_file(&source_path, &build_root, &RunOptions::default()).expect("run file");

        assert_eq!(output.file_operation_paths.len(), 3);
        assert!(build_root
            .join("result")
            .join("ops")
            .join("moved.txt")
            .exists());
        assert!(!build_root
            .join("result")
            .join("ops")
            .join("scratch.txt")
            .exists());
        assert!(output
            .output_manifest_json
            .contains("\"kind\": \"copy_file\""));
        assert!(output
            .output_manifest_json
            .contains("\"kind\": \"move_file\""));
        assert!(output
            .output_manifest_json
            .contains("\"kind\": \"delete_file\""));
        assert!(output.review_json.contains("\"file_operations\""));
        assert!(output
            .review_json
            .contains("\"provenance\": \"runtime_artifact_record\""));
        assert!(output
            .review_json
            .contains("\"artifact_kind\": \"copy_file\""));
        assert!(output
            .review_json
            .contains("\"artifact_kind\": \"move_file\""));
        assert!(output
            .review_json
            .contains("\"artifact_kind\": \"delete_file\""));
        assert!(output
            .review_json
            .contains("\"artifact_class\": \"generated_file\""));
        assert!(output.review_json.contains("\"validation\""));
    }

    #[test]
    fn run_file_executes_process_and_records_result() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-process-result");
        let build_root = repo_root
            .join("build")
            .join("runtime-process-result-output");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let source_path = source_dir.join("main.eng");
        let source = if cfg!(windows) {
            "process_result = run command \"cmd\"\nwith {\n    args = [\"/C\", \"echo\", \"process-ok\"]\n    tool_version = \"cmd-test 1.0\"\n}\n"
        } else {
            "process_result = run command \"sh\"\nwith {\n    args = [\"-c\", \"echo process-ok\"]\n    tool_version = \"sh-test 1.0\"\n}\n"
        };
        fs::write(&source_path, source).expect("write source");

        let output = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                save_artifacts: true,
                ..RunOptions::default()
            },
        )
        .expect("process run");

        assert!(output.review_json.contains("\"process_runs\""));
        assert!(output
            .review_json
            .contains("\"binding\": \"process_result\""));
        assert!(output
            .process_results_json
            .contains("\"format\": \"eng-process-results-v1\""));
        assert!(output.review_json.contains("\"tool_version\": \""));
        assert!(output
            .review_json
            .contains("\"provenance\": \"runtime_process_result\""));
        assert!(output.review_json.contains("\"success\": true"));
        assert!(output.review_json.contains("\"stdout_hash\""));
        assert!(output
            .review_json
            .contains("\"expected_output_status\": \"not_declared\""));
        assert!(output.process_results_json.contains("\"tool_version\": \""));
        assert!(output.process_results_json.contains("process-ok"));
        assert!(output.process_results_json.contains("\"stdout_hash\""));
        assert!(output.process_results_json.contains("\"stderr_hash\""));
        assert!(output.run_log_json.contains("\"external_boundary_events\""));
        assert!(output
            .run_log_json
            .contains("\"binding\": \"process_result\""));
        assert!(output.run_log_json.contains("\"stdout_hash\""));
        assert!(output
            .output_manifest_json
            .contains("\"kind\": \"process_results\""));
        assert!(output
            .output_manifest_json
            .contains("\"external_commands\""));
    }

    #[test]
    fn run_file_records_process_expected_outputs() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root
            .join("build")
            .join("runtime-process-expected-output");
        let build_root = repo_root
            .join("build")
            .join("runtime-process-expected-output-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let source_path = source_dir.join("main.eng");
        let source = if cfg!(windows) {
            "process_result = run command \"cmd\"\nwith {\n    args = [\"/C\", \"if not exist outputs mkdir outputs && echo process-ok>outputs/out.txt\"]\n    expected_outputs = [\"outputs/out.txt\"]\n}\n"
        } else {
            "process_result = run command \"sh\"\nwith {\n    args = [\"-c\", \"mkdir -p outputs && printf process-ok > outputs/out.txt\"]\n    expected_outputs = [\"outputs/out.txt\"]\n}\n"
        };
        fs::write(&source_path, source).expect("write source");

        let output = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                save_artifacts: true,
                ..RunOptions::default()
            },
        )
        .expect("process run");

        assert!(source_dir.join("outputs").join("out.txt").exists());
        assert!(output.review_json.contains("\"expected_outputs\""));
        assert!(output
            .review_json
            .contains("\"expected_output_status\": \"satisfied\""));
        assert!(output.review_json.contains("\"output_artifacts\""));
        assert!(output.review_json.contains("\"exists_and_hash\""));
        assert!(output.process_results_json.contains("\"expected_outputs\""));
        assert!(output
            .process_results_json
            .contains("\"expected_output_status\": \"satisfied\""));
        assert!(output
            .process_results_json
            .contains("\"status\": \"exists\""));
        assert!(output
            .process_results_json
            .contains("\"kind\": \"process_expected_output\""));
        assert!(output.process_results_json.contains("\"validation\""));
        assert!(output
            .process_results_json
            .contains("\"rule\": \"exists_and_hash\""));
        assert!(output
            .output_manifest_json
            .contains("\"kind\": \"process_expected_output\""));
        assert!(output.output_manifest_json.contains("\"generated_files\""));
        assert!(output.output_manifest_json.contains("\"validation\""));
    }

    #[test]
    fn run_file_materializes_db_manifest_expected_output() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-db-manifest");
        let build_root = repo_root.join("build").join("runtime-db-manifest-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("outputs")).expect("outputs dir");
        fs::write(
            source_dir.join("outputs").join("db_write_manifest.json"),
            concat!(
                "{\n",
                "  \"database\": \"outputs/results.sqlite\",\n",
                "  \"transaction_status\": \"committed-fixture\",\n",
                "  \"schema_status\": \"ok\",\n",
                "  \"tables\": [\n",
                "    {\n",
                "      \"name\": \"simulation_results\",\n",
                "      \"mode\": \"upsert\",\n",
                "      \"key\": [\"case_id\"],\n",
                "      \"schema\": [\"case_id\", \"annual_electricity\"],\n",
                "      \"row_count\": 2\n",
                "    }\n",
                "  ]\n",
                "}\n",
            ),
        )
        .expect("db manifest");
        let source_path = source_dir.join("main.eng");
        let source = if cfg!(windows) {
            "db_result = run command \"cmd\"\nwith {\n    args = [\"/C\", \"echo db\"]\n    expected_outputs = [\"outputs/db_write_manifest.json\"]\n}\n"
        } else {
            "db_result = run command \"sh\"\nwith {\n    args = [\"-c\", \"printf db\"]\n    expected_outputs = [\"outputs/db_write_manifest.json\"]\n}\n"
        };
        fs::write(&source_path, source).expect("write source");

        let output = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                save_artifacts: true,
                ..RunOptions::default()
            },
        )
        .expect("db manifest process run");

        assert!(output.result_json.contains("\"db_manifests\""));
        assert!(output.result_json.contains("\"binding\": \"db_result\""));
        assert!(output
            .result_json
            .contains("\"database\": \"outputs/results.sqlite\""));
        assert!(output
            .result_json
            .contains("\"transaction_status\": \"committed-fixture\""));
        assert!(output
            .result_json
            .contains("\"name\": \"simulation_results\""));
        assert!(output.result_json.contains("\"mode\": \"upsert\""));
        assert!(output.result_json.contains("\"key\": [\"case_id\"]"));
        assert!(output.result_json.contains("\"row_count\": 2"));
        assert!(output
            .result_json
            .contains("\"status\": \"manifest_loaded\""));
        assert!(output.review_json.contains("\"db_manifests\""));
        assert!(output
            .output_manifest_json
            .contains("\"kind\": \"db_write_manifest\""));
        assert!(output.output_manifest_json.contains("\"db_writes\""));
    }

    #[test]
    fn run_file_records_allowed_process_failures() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root
            .join("build")
            .join("runtime-process-allow-failure");
        let build_root = repo_root
            .join("build")
            .join("runtime-process-allow-failure-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let source_path = source_dir.join("main.eng");
        let source = if cfg!(windows) {
            "process_result = run command \"cmd\"\nwith {\n    args = [\"/C\", \"exit /B 7\"]\n    allow_failure = true\n}\n"
        } else {
            "process_result = run command \"sh\"\nwith {\n    args = [\"-c\", \"exit 7\"]\n    allow_failure = true\n}\n"
        };
        fs::write(&source_path, source).expect("write source");

        let output = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                save_artifacts: true,
                ..RunOptions::default()
            },
        )
        .expect("allowed process failure");

        assert!(output.process_results_json.contains("\"exit_code\": 7"));
        assert!(output
            .process_results_json
            .contains("\"status\": \"failed_allowed\""));
    }

    #[test]
    fn run_file_rejects_missing_process_expected_outputs() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root
            .join("build")
            .join("runtime-process-missing-output");
        let build_root = repo_root
            .join("build")
            .join("runtime-process-missing-output-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let source_path = source_dir.join("main.eng");
        let source = if cfg!(windows) {
            "process_result = run command \"cmd\"\nwith {\n    args = [\"/C\", \"echo done\"]\n    expected_outputs = [\"missing/out.txt\"]\n}\n"
        } else {
            "process_result = run command \"sh\"\nwith {\n    args = [\"-c\", \"printf done\"]\n    expected_outputs = [\"missing/out.txt\"]\n}\n"
        };
        fs::write(&source_path, source).expect("write source");

        let error = run_file(&source_path, &build_root, &RunOptions::default())
            .expect_err("missing process output should fail");
        let message = error.to_string();
        assert!(message.contains("did not create expected output(s)"));
        assert!(message.contains("missing/out.txt"));
    }

    #[test]
    fn run_file_records_allowed_missing_process_expected_outputs() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root
            .join("build")
            .join("runtime-process-missing-output-allowed");
        let build_root = repo_root
            .join("build")
            .join("runtime-process-missing-output-allowed-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let source_path = source_dir.join("main.eng");
        let source = if cfg!(windows) {
            "process_result = run command \"cmd\"\nwith {\n    args = [\"/C\", \"echo done\"]\n    expected_outputs = [\"missing/out.txt\"]\n    allow_failure = true\n}\n"
        } else {
            "process_result = run command \"sh\"\nwith {\n    args = [\"-c\", \"printf done\"]\n    expected_outputs = [\"missing/out.txt\"]\n    allow_failure = true\n}\n"
        };
        fs::write(&source_path, source).expect("write source");

        let output = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                save_artifacts: true,
                ..RunOptions::default()
            },
        )
        .expect("allowed missing process output");

        assert!(output
            .process_results_json
            .contains("\"expected_output_status\": \"missing\""));
        assert!(output
            .process_results_json
            .contains("\"status\": \"output_missing_allowed\""));
    }

    #[test]
    fn run_file_safe_profile_rejects_explicit_side_effects() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-safe-profile");
        let build_root = repo_root.join("build").join("runtime-safe-profile-output");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let source_path = source_dir.join("main.eng");
        fs::write(&source_path, "write text \"out.txt\", \"not in safe\"\n").expect("write source");

        let error = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                profile: ExecutionProfile::Safe,
                ..RunOptions::default()
            },
        )
        .expect_err("safe profile should reject write");

        assert!(error.to_string().contains("profile `safe` rejected"));
        assert!(error.to_string().contains("E-PROFILE-SAFE-WRITE"));
    }

    #[test]
    fn run_file_repro_profile_records_profile_diagnostics() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-repro-profile");
        let build_root = repo_root.join("build").join("runtime-repro-profile-output");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "input_exists = exists file(\"missing.txt\")\nprint \"exists = {input_exists}\"\n",
        )
        .expect("write source");

        let output = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                profile: ExecutionProfile::Repro,
                ..RunOptions::default()
            },
        )
        .expect("repro profile run");

        assert!(output
            .result_json
            .contains("\"execution_profile\": \"repro\""));
        assert!(output.result_json.contains("W-PROFILE-REPRO-ENV"));
        assert!(output.run_log_json.contains("\"profile_diagnostics\""));
    }

    #[test]
    fn run_file_requires_overwrite_for_changed_write_outputs() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-write-overwrite");
        let build_root = repo_root
            .join("build")
            .join("runtime-write-overwrite-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        fs::create_dir_all(build_root.join("result")).expect("result dir");
        fs::write(build_root.join("result").join("note.txt"), "old").expect("old note");
        let source_path = source_dir.join("main.eng");
        fs::write(&source_path, "write text \"note.txt\", \"fresh\"\n").expect("write source");

        let error = run_file(&source_path, &build_root, &RunOptions::default())
            .expect_err("changed output should require overwrite");
        assert!(error.to_string().contains("overwrite = true"));

        fs::write(
            &source_path,
            "write text \"note.txt\", \"fresh\"\nwith {\n    overwrite = true\n}\n",
        )
        .expect("write overwrite source");
        let output =
            run_file(&source_path, &build_root, &RunOptions::default()).expect("overwrite run");
        assert_eq!(output.write_output_paths.len(), 1);
        let text = fs::read_to_string(build_root.join("result").join("note.txt")).expect("note");
        assert_eq!(text, "fresh");
    }

    #[test]
    fn run_file_evaluates_path_helpers_and_records_environment_provenance() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-path-policy");
        let build_root = repo_root.join("build").join("runtime-path-policy-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(source_dir.join("data").join("sensor.csv"), "time,T\n0,20\n")
            .expect("sensor csv");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "args {\n    input: CsvFile = file(\"data/sensor.csv\")\n    output: DirectoryPath = dir(\"build/out\")\n}\n\ninput_exists = exists args.input\nsummary_file = join(args.output, \"summary.csv\")\ninput_parent = parent(args.input)\ninput_stem = stem(args.input)\ninput_ext = extension(args.input)\n\nprint \"exists={input_exists} summary={summary_file} parent={input_parent} stem={input_stem} ext={input_ext}\"\n",
        )
        .expect("write source");

        let output = run_file(&source_path, &build_root, &RunOptions::default()).expect("run file");

        assert!(output.stdout.contains("exists=true"));
        assert!(output.stdout.contains("summary=build/out/summary.csv"));
        assert!(output.stdout.contains("parent=data"));
        assert!(output.stdout.contains("stem=sensor"));
        assert!(output.stdout.contains("ext=csv"));
        assert!(output.result_json.contains("\"environment_dependencies\""));
        assert!(output.result_json.contains("\"filesystem_exists\""));
        assert!(output.result_json.contains("\"resolved_value\": \"true\""));
        assert!(output
            .report_spec_json
            .contains("\"environment_dependencies\""));
    }

    #[test]
    fn run_file_reads_text_json_and_toml_with_source_hash_provenance() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-read-only-io");
        let build_root = repo_root.join("build").join("runtime-read-only-io-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(source_dir.join("data").join("notes.txt"), "calibrated run").expect("notes");
        fs::write(
            source_dir.join("data").join("case.json"),
            "{ \"case\": \"baseline\" }",
        )
        .expect("json");
        fs::write(
            source_dir.join("data").join("case.toml"),
            "case = \"baseline\"",
        )
        .expect("toml");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "args {\n    notes: TextFile = file(\"data/notes.txt\")\n    config_json: JsonFile = file(\"data/case.json\")\n    config_toml: TomlFile = file(\"data/case.toml\")\n}\n\nnotes_text = read text args.notes\njson_text = read json args.config_json\ntoml_text = read toml args.config_toml\n\nprint \"notes={notes_text}\"\nprint \"json={json_text}\"\nprint \"toml={toml_text}\"\n",
        )
        .expect("write source");

        let output = run_file(&source_path, &build_root, &RunOptions::default()).expect("run file");

        assert!(output.stdout.contains("notes=calibrated run"));
        assert!(output.stdout.contains("json={ \"case\": \"baseline\" }"));
        assert!(output.stdout.contains("toml=case = \"baseline\""));
        assert!(output.result_json.contains("\"filesystem_read_text\""));
        assert!(output.result_json.contains("\"filesystem_read_json\""));
        assert!(output.result_json.contains("\"filesystem_read_toml\""));
        assert!(output.result_json.contains("\"source_hash\": \""));
        assert!(output.result_json.contains("\"structured_reads\""));
        assert!(output.result_json.contains("\"binding\": \"json_text\""));
        assert!(output.result_json.contains("\"binding\": \"toml_text\""));
        assert!(output.result_json.contains("\"parse_status\": \"parsed\""));
        assert!(output.result_json.contains("\"root_type\": \"object\""));
        assert!(output.result_json.contains("\"root_type\": \"table\""));
        assert!(output.report_spec_json.contains("\"filesystem_read_text\""));
        assert!(output
            .output_manifest_json
            .contains("\"binding\": \"notes_text\""));
        assert!(output.output_manifest_json.contains("notes.txt"));
        assert!(output.output_manifest_json.contains("\"hash\": \""));
    }
}
