use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use eng_compiler::{
    build_bytecode, canonical_path_text, check_file, check_source,
    classify_workflow_node_review_risk, parse_bytecode, review_json, ArgOverride, CheckOptions,
    CheckReport, ReviewFallbackRecord,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

mod artifact;
mod runtime_data;
pub mod solver;
mod vm;

use artifact::{
    ArtifactRecord, ArtifactValidation, ExternalBoundaryRecord, ModelArtifactRecord,
    OutputArtifact, OutputManifest, SourceRecord,
};
use runtime_data::{
    materialize_runtime_data, RuntimeCaseDiagnostic, RuntimeCaseManifest, RuntimeCaseMetric,
    RuntimeCaseProcessStatus, RuntimeCaseTable, RuntimeComponentResidualEvaluation, RuntimeData,
    RuntimeMlArtifact, RuntimeNumericUncertaintyPayload, RuntimeNumericValue,
    RuntimeStatisticValue, RuntimeTable, RuntimeTimeSeries, RuntimeValues,
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
    pub skip_unchanged: bool,
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
    pub static_run_plan_path: PathBuf,
    pub run_plan_path: PathBuf,
    pub run_lock_path: PathBuf,
    pub run_log_path: PathBuf,
    pub process_results_path: PathBuf,
    pub cache_manifest_path: PathBuf,
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
    pub static_run_plan_json: String,
    pub run_plan_json: String,
    pub run_lock_json: String,
    pub run_log_json: String,
    pub process_results_json: String,
    pub cache_manifest_json: String,
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
                if write.format == "db" {
                    diagnostics.push(ProfileDiagnostic {
                        severity: "error",
                        code: "E-DB-SAFE-PROFILE",
                        message: "safe profile rejects native DB write side effects".to_owned(),
                        line: write.line,
                    });
                    continue;
                }
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
            for write in report
                .semantic_program
                .writes
                .iter()
                .filter(|write| write.format == "db")
            {
                diagnostics.push(ProfileDiagnostic {
                    severity: "warning",
                    code: "W-PROFILE-REPRO-DB",
                    message: format!(
                        "repro profile records DB write `{}` with manifest and database hash before/after",
                        write.expression
                    ),
                    line: write.line,
                });
            }
            for generation in &report.semantic_program.sample_generations {
                if matches!(generation.method.as_str(), "random" | "lhs")
                    && generation.seed.is_none()
                {
                    diagnostics.push(ProfileDiagnostic {
                        severity: "error",
                        code: "E-SAMPLING-SEED-MISSING",
                        message: format!(
                            "repro profile requires sample `{}` to declare `seed`",
                            generation.binding
                        ),
                        line: generation.line,
                    });
                }
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
            for request in &report.semantic_program.net_requests {
                if request.fixture.is_none() || request.expected_sha256.is_none() {
                    diagnostics.push(ProfileDiagnostic {
                        severity: "error",
                        code: "E-NET-UNPINNED-REPRO",
                        message: format!(
                            "repro profile requires network request `{}` to declare fixture and expected_sha256",
                            request.binding
                        ),
                        line: request.line,
                    });
                }
            }
            for download in &report.semantic_program.net_downloads {
                if download.fixture.is_none() || download.expected_sha256.is_none() {
                    diagnostics.push(ProfileDiagnostic {
                        severity: "error",
                        code: "E-NET-UNPINNED-REPRO",
                        message: format!(
                            "repro profile requires network download `{}` to declare fixture and expected_sha256",
                            download.target_value
                        ),
                        line: download.line,
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
    let working_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let plots_dir = result_dir.join("plots");
    let bytecode_path = build_root.join(format!("{stem}.engbc"));
    let result_path = result_dir.join("result.engres");
    let review_path = result_dir.join("review.json");
    let plot_path = plots_dir.join("timeseries.svg");
    let plot_spec_path = plots_dir.join("plot_spec.json");
    let plot_manifest_path = plots_dir.join("plot_manifest.json");
    let output_manifest_path = result_dir.join("output_manifest.json");
    let static_run_plan_path = result_dir.join("static_run_plan.json");
    let run_plan_path = result_dir.join("run_plan.json");
    let run_lock_path = result_dir.join("run_lock.json");
    let run_log_path = result_dir.join("run_log.json");
    let process_results_path = result_dir.join("process_results.json");
    let cache_manifest_path = result_dir.join("cache_manifest.json");
    let test_results_path = result_dir.join("test_results.json");
    let report_spec_path = result_dir.join("report_spec.json");
    let report_path = result_dir.join("report.html");
    let artifacts_saved = options.save_artifacts || options.open_report;
    let run_lock_input = run_lock_input(path, &check_report, options);
    let mut rerun_decision = rerun_decision_for_run(
        &run_lock_path,
        &run_lock_input,
        options.skip_unchanged,
        artifacts_saved,
    );
    let saved_artifacts_ready = saved_run_artifacts_available(&[
        &bytecode_path,
        &result_path,
        &review_path,
        &report_path,
        &report_spec_path,
        &plot_path,
        &plot_spec_path,
        &plot_manifest_path,
        &output_manifest_path,
        &static_run_plan_path,
        &run_plan_path,
        &run_lock_path,
        &run_log_path,
        &process_results_path,
        &cache_manifest_path,
        &test_results_path,
    ]);
    if rerun_decision.decision == "skip" && !saved_artifacts_ready {
        rerun_decision = RerunDecision {
            decision: "run".to_owned(),
            reason: "missing_saved_artifact".to_owned(),
            prior_input_hash: rerun_decision.prior_input_hash.clone(),
        };
    }
    if rerun_decision.decision == "skip" && saved_artifacts_ready {
        if let Some(kind) = saved_run_artifact_hash_mismatch(
            &run_lock_path,
            &[
                ("result", &result_path),
                ("review", &review_path),
                ("static_run_plan", &static_run_plan_path),
                ("run_plan", &run_plan_path),
            ],
        ) {
            rerun_decision = RerunDecision {
                decision: "run".to_owned(),
                reason: format!("artifact_hash_mismatch:{kind}"),
                prior_input_hash: rerun_decision.prior_input_hash.clone(),
            };
        }
    }
    let static_run_plan_json =
        static_run_plan_json(path, &check_report, &options.profile, &rerun_decision);
    if rerun_decision.decision == "skip" && saved_artifacts_ready {
        return skipped_saved_run_output(
            path,
            &check_report,
            &run_lock_input,
            &rerun_decision,
            &static_run_plan_json,
            &options.profile,
            bytecode_path,
            result_path,
            review_path,
            report_path,
            report_spec_path,
            plot_path,
            plot_spec_path,
            plot_manifest_path,
            output_manifest_path,
            static_run_plan_path,
            run_plan_path,
            run_lock_path,
            run_log_path,
            process_results_path,
            cache_manifest_path,
            test_results_path,
        );
    }
    if artifacts_saved {
        fs::create_dir_all(&result_dir)?;
        fs::write(&static_run_plan_path, &static_run_plan_json)?;
    }

    let bytecode = build_bytecode(&check_report, source);
    let bytecode_hash = hash_text(&bytecode);
    let bytecode_program = parse_bytecode(&bytecode)?;
    let mut execution = execute_bytecode(&bytecode_program)?;
    let runtime_data = materialize_runtime_data(&check_report, source);
    apply_runtime_lengths(&mut execution, &runtime_data);
    let stdout = render_stdout(&check_report, &runtime_data);
    let template_render_output =
        render_template_outputs(&check_report, &runtime_data, &result_dir)?;
    let process_results = execute_process_runs(&check_report)?;
    let native_db_write_output =
        execute_native_db_writes(&check_report, &runtime_data, &result_dir)?;
    let mut db_manifest_records = db_manifest_records(&process_results);
    db_manifest_records.extend(native_db_write_output.records.clone());
    let mut cache_manifest_records =
        cache_manifest_records(&check_report, &runtime_data, &process_results, build_root);
    materialize_network_cache_entries(&check_report, &mut cache_manifest_records, build_root)?;
    ensure_cache_hashes_valid(&cache_manifest_records)?;
    ensure_cache_reproducible(&cache_manifest_records, &options.profile)?;
    let cache_diagnostics = cache_stale_diagnostics(&cache_manifest_records, build_root);
    let external_boundary_records = external_boundary_records_for_run(
        &check_report,
        &process_results,
        &db_manifest_records,
        &cache_manifest_records,
    );
    let cache_manifest_json = cache_manifest_json(
        &check_report,
        &cache_manifest_records,
        &cache_diagnostics,
        &options.profile,
    );
    let run_log_json = run_log_json(
        &check_report,
        &runtime_data,
        &options.profile,
        &profile_diagnostics,
        &external_boundary_records,
        &cache_manifest_records,
        &working_dir,
        &result_dir,
    );
    let process_results_json =
        process_results_json(&check_report, &process_results, &options.profile);
    let csv_export_artifacts = write_csv_exports(&check_report, &runtime_data, &result_dir)?;
    let write_artifacts = write_outputs(&check_report, &runtime_data, &result_dir)?;
    let file_operation_artifacts = apply_file_operations(&check_report, &result_dir)?;
    let test_results = execute_tests(&check_report, &runtime_data, &result_dir)?;
    let test_results_json = test_results_json(&check_report, &test_results);
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
    report_spec.quality_report = runtime_data.report_quality_report();
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
    output_artifacts.extend(template_render_output.artifacts.clone());
    output_artifacts.extend(native_db_write_output.artifacts.clone());
    output_artifacts.extend(process_expected_output_artifacts(&process_results));
    output_artifacts.extend(csv_export_artifacts);
    output_artifacts.extend(write_artifacts);
    output_artifacts.extend(file_operation_artifacts);
    let mut review_json = runtime_review_json(
        &review_json(&check_report),
        &runtime_data,
        &process_results,
        &external_boundary_records,
        &output_artifacts,
        &cache_manifest_records,
        &template_render_output.records,
        &db_manifest_records,
    );
    let report_html =
        eng_report::render_html_with_spec(&check_report, "plots/timeseries.svg", &report_spec);
    let result_json = result_json(
        path,
        &check_report,
        &execution,
        &runtime_data,
        &process_results,
        &db_manifest_records,
        &cache_manifest_records,
        &template_render_output.records,
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
    let initial_run_plan_json = run_plan_json(
        path,
        &check_report,
        &runtime_data,
        &process_results,
        &external_boundary_records,
        &cache_manifest_records,
        &db_manifest_records,
        &output_artifacts,
        &static_run_plan_json,
        &result_json,
        &review_json,
        &options.profile,
        &rerun_decision,
    );
    review_json = enrich_runtime_review_workflow_graph(&review_json, &initial_run_plan_json);
    let run_plan_json = run_plan_json(
        path,
        &check_report,
        &runtime_data,
        &process_results,
        &external_boundary_records,
        &cache_manifest_records,
        &db_manifest_records,
        &output_artifacts,
        &static_run_plan_json,
        &result_json,
        &review_json,
        &options.profile,
        &rerun_decision,
    );
    let result_artifact_hash = hash_text(&result_json);
    let review_artifact_hash = hash_text(&review_json);
    let static_run_plan_artifact_hash = hash_text(&static_run_plan_json);
    let run_plan_artifact_hash = hash_text(&run_plan_json);
    let run_lock_json = run_lock_json(
        path,
        &check_report,
        &run_lock_input,
        &rerun_decision,
        &RunLockArtifactHashes {
            result: &result_artifact_hash,
            review: &review_artifact_hash,
            static_run_plan: &static_run_plan_artifact_hash,
            run_plan: &run_plan_artifact_hash,
        },
        &options.profile,
    );
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
        output_artifacts.push(output_artifact(
            "static_run_plan",
            "static_run_plan.json".to_owned(),
            &static_run_plan_json,
            static_run_plan_path.clone(),
        ));
        fs::write(&run_plan_path, &run_plan_json)?;
        output_artifacts.push(output_artifact(
            "run_plan",
            "run_plan.json".to_owned(),
            &run_plan_json,
            run_plan_path.clone(),
        ));
        fs::write(&run_lock_path, &run_lock_json)?;
        output_artifacts.push(output_artifact(
            "run_lock",
            "run_lock.json".to_owned(),
            &run_lock_json,
            run_lock_path.clone(),
        ));
        fs::write(&process_results_path, &process_results_json)?;
        output_artifacts.push(output_artifact(
            "process_results",
            "process_results.json".to_owned(),
            &process_results_json,
            process_results_path.clone(),
        ));
        fs::write(&cache_manifest_path, &cache_manifest_json)?;
        output_artifacts.push(output_artifact(
            "cache_manifest",
            "cache_manifest.json".to_owned(),
            &cache_manifest_json,
            cache_manifest_path.clone(),
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
        &working_dir,
        &result_dir,
        &output_artifacts,
        &options.profile,
        &profile_diagnostics,
        &ArtifactRegistryContext {
            report: &check_report,
            runtime_data: &runtime_data,
            external_boundary_records: &external_boundary_records,
            cache_manifest_records: &cache_manifest_records,
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
        static_run_plan_path,
        run_plan_path,
        run_lock_path,
        run_log_path,
        process_results_path,
        cache_manifest_path,
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
        static_run_plan_json,
        run_plan_json,
        run_lock_json,
        run_log_json,
        process_results_json,
        cache_manifest_json,
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
                    let default_value = if field.redacted {
                        "<redacted>"
                    } else {
                        default_value
                    };
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
    canonical_path_text(&path.display().to_string())
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
        if let Some(series) = runtime_data
            .time_series
            .iter()
            .find(|series| series.name == object.name)
        {
            object.kind = VmObjectKind::TimeSeries;
            object.type_name = format!("TimeSeries[{}] of {}", series.axis, series.quantity_kind);
            object.axis = Some(series.axis.clone());
            object.display_unit = Some(series.display_unit.clone());
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
    cache_records: &[CacheManifestRecord],
    working_dir: &Path,
    output_dir: &Path,
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
        json_escape(&path_for_manifest(&report.source_path))
    ));
    json.push_str(&format!(
        "  \"working_dir\": \"{}\",\n",
        json_escape(&path_for_manifest(working_dir))
    ));
    json.push_str(&format!(
        "  \"output_dir\": \"{}\",\n",
        json_escape(&path_for_manifest(output_dir))
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
    push_external_boundary_events_json(&mut json, external_boundaries, "    ");
    json.push_str("\n  ],\n");
    let network_event_count = external_boundaries
        .iter()
        .filter(|record| is_network_boundary_record(record))
        .count();
    json.push_str(&format!(
        "  \"network_event_count\": {},\n",
        network_event_count
    ));
    json.push_str("  \"network_events\": [\n");
    push_network_events_json(&mut json, external_boundaries, "    ");
    json.push_str("\n  ],\n");
    json.push_str(&format!(
        "  \"cache_event_count\": {},\n",
        cache_records.len()
    ));
    json.push_str("  \"cache_events\": [\n");
    push_cache_events_json(&mut json, cache_records, "    ");
    json.push_str("\n  ],\n");
    json.push_str("  \"profile_diagnostics\": [\n");
    push_profile_diagnostics_json(&mut json, profile_diagnostics, "    ");
    json.push_str("\n  ]\n");
    json.push_str("}\n");
    json
}

fn push_cache_events_json(json: &mut String, records: &[CacheManifestRecord], indent: &str) {
    for (index, record) in records.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}{{\n"));
        json.push_str(&format!("{indent}  \"kind\": \"cache\",\n"));
        json.push_str(&format!(
            "{indent}  \"owner_kind\": \"{}\",\n",
            json_escape(&record.owner_kind)
        ));
        json.push_str(&format!(
            "{indent}  \"owner_name\": \"{}\",\n",
            json_escape(&record.owner_name)
        ));
        json.push_str(&format!(
            "{indent}  \"cache_key_hash\": \"{}\",\n",
            json_escape(&record.cache_key_hash)
        ));
        json.push_str(&format!(
            "{indent}  \"cache_path\": \"{}\",\n",
            json_escape(&record.cache_path)
        ));
        json.push_str(&format!(
            "{indent}  \"lookup_status\": \"{}\",\n",
            json_escape(&record.lookup_status)
        ));
        json.push_str(&format!(
            "{indent}  \"status\": \"{}\",\n",
            json_escape(&record.status)
        ));
        json.push_str(&format!("{indent}  \"line\": {}\n", record.line));
        json.push_str(&format!("{indent}}}"));
    }
}

fn push_external_boundary_events_json(
    json: &mut String,
    records: &[ExternalBoundaryRecord],
    indent: &str,
) {
    for (index, record) in records.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}{{\n"));
        json.push_str(&format!(
            "{indent}  \"kind\": \"{}\",\n",
            json_escape(&record.kind)
        ));
        json.push_str(&format!(
            "{indent}  \"binding\": \"{}\",\n",
            json_escape(&record.binding)
        ));
        json.push_str(&format!(
            "{indent}  \"command\": \"{}\",\n",
            json_escape(&record.command)
        ));
        json.push_str(&format!(
            "{indent}  \"target\": \"{}\",\n",
            json_escape(&record.target)
        ));
        if record.kind == "process" {
            json.push_str(&format!("{indent}  \"env\": "));
            push_json_string_array_runtime(json, &record.env_keys);
            json.push_str(",\n");
            push_optional_json_string(json, "timeout", record.timeout.as_deref(), indent.len() + 2);
            json.push_str(&format!("{indent}  \"retry\": {},\n", record.retry));
            json.push_str(&format!(
                "{indent}  \"attempt_count\": {},\n",
                record.attempt_count
            ));
            json.push_str(&format!(
                "{indent}  \"allow_failure\": {},\n",
                record.allow_failure
            ));
            json.push_str(&format!("{indent}  \"timed_out\": {},\n", record.timed_out));
        }
        push_optional_json_string(
            json,
            "response_hash",
            record.response_hash.as_deref(),
            indent.len() + 2,
        );
        push_optional_json_string(
            json,
            "expected_sha256",
            record.expected_hash.as_deref(),
            indent.len() + 2,
        );
        json.push_str(&format!(
            "{indent}  \"status\": \"{}\",\n",
            json_escape(&record.status)
        ));
        json.push_str(&format!("{indent}  \"success\": {},\n", record.success));
        json.push_str(&format!(
            "{indent}  \"expected_output_status\": \"{}\",\n",
            json_escape(&record.expected_output_status)
        ));
        json.push_str(&format!(
            "{indent}  \"stdout_hash\": \"{}\",\n",
            json_escape(&record.stdout_hash)
        ));
        json.push_str(&format!(
            "{indent}  \"stderr_hash\": \"{}\",\n",
            json_escape(&record.stderr_hash)
        ));
        json.push_str(&format!("{indent}  \"line\": {}\n", record.line));
        json.push_str(&format!("{indent}}}"));
    }
}

fn push_network_events_json(json: &mut String, records: &[ExternalBoundaryRecord], indent: &str) {
    let mut first = true;
    for record in records
        .iter()
        .filter(|record| is_network_boundary_record(record))
    {
        if !first {
            json.push_str(",\n");
        }
        first = false;
        json.push_str(&format!("{indent}{{\n"));
        json.push_str(&format!(
            "{indent}  \"kind\": \"{}\",\n",
            json_escape(run_log_network_event_kind(record))
        ));
        if !record.binding.is_empty() {
            json.push_str(&format!(
                "{indent}  \"binding\": \"{}\",\n",
                json_escape(&record.binding)
            ));
        }
        json.push_str(&format!(
            "{indent}  \"url\": \"{}\",\n",
            json_escape(&record.target)
        ));
        if let Some(target) = record.output_paths.first() {
            json.push_str(&format!(
                "{indent}  \"target\": \"{}\",\n",
                json_escape(target)
            ));
        }
        push_optional_json_string(
            json,
            "response_hash",
            record.response_hash.as_deref(),
            indent.len() + 2,
        );
        json.push_str(&format!(
            "{indent}  \"status\": \"{}\",\n",
            json_escape(&record.status)
        ));
        json.push_str(&format!("{indent}  \"line\": {}\n", record.line));
        json.push_str(&format!("{indent}}}"));
    }
}

fn is_network_boundary_record(record: &ExternalBoundaryRecord) -> bool {
    matches!(record.kind.as_str(), "network_request" | "network_download")
}

fn run_log_network_event_kind(record: &ExternalBoundaryRecord) -> &str {
    match record.kind.as_str() {
        "network_request" => "http_get",
        "network_download" => "download",
        _ => record.kind.as_str(),
    }
}

#[derive(Clone, Debug)]
struct ProcessExecutionRecord {
    binding: String,
    command: String,
    tool_version: Option<String>,
    args: Vec<String>,
    env_keys: Vec<String>,
    cwd: String,
    expected_outputs: Vec<ProcessExpectedOutputRecord>,
    expected_output_status: String,
    timeout: Option<String>,
    retry: usize,
    attempt_count: usize,
    allow_failure: bool,
    timed_out: bool,
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
struct ProcessSpec {
    binding: String,
    command: String,
    tool_version: Option<String>,
    args: Vec<String>,
    env: Vec<ProcessEnvVar>,
    cwd: PathBuf,
    cwd_text: String,
    timeout: Option<Duration>,
    timeout_label: Option<String>,
    retry: usize,
    allow_failure: bool,
    line: usize,
}

#[derive(Clone, Debug)]
struct ProcessEnvVar {
    key: String,
    value: String,
}

#[derive(Clone, Debug)]
struct ProcessAttemptOutput {
    exit_code: Option<i32>,
    success: bool,
    stdout: String,
    stderr: String,
    duration_ms: u128,
    timed_out: bool,
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
struct CacheManifestRecord {
    owner_kind: String,
    owner_name: String,
    cache_key: String,
    cache_key_parts: Vec<String>,
    cache_key_hash: String,
    cache_path: String,
    cache_dir: String,
    resolved_path: String,
    source_hash: String,
    expected_hash: Option<String>,
    observed_hash: Option<String>,
    lookup_status: String,
    status: String,
    line: usize,
}

#[derive(Clone, Debug)]
struct CacheDiagnosticRecord {
    severity: &'static str,
    code: &'static str,
    message: String,
    cache_dir: String,
    cache_path: String,
    resolved_path: String,
    cache_key_hash: String,
    status: &'static str,
    line: usize,
}

#[derive(Clone, Debug)]
struct DbManifestRecord {
    binding: String,
    manifest_path: String,
    resolved_path: String,
    hash: Option<String>,
    database: Option<String>,
    database_hash_before: Option<String>,
    database_hash_after: Option<String>,
    transaction_status: Option<String>,
    schema_status: Option<String>,
    tables: Vec<DbManifestTableRecord>,
    diagnostics: Vec<DbManifestDiagnosticRecord>,
    status: String,
    line: usize,
}

#[derive(Clone, Debug)]
struct DbManifestDiagnosticRecord {
    code: String,
    message: String,
    table: Option<String>,
}

#[derive(Clone, Debug)]
struct CaseCollectionRecord {
    manifest_path: String,
    status: String,
    expected_case_ids: Vec<String>,
    result_case_ids: Vec<String>,
    missing_case_ids: Vec<String>,
    failed_case_count: usize,
}

#[derive(Clone, Debug)]
struct ExternalModelJsonRecord {
    binding: String,
    path: String,
    value: Value,
    line: usize,
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
        let spec = process_spec_for_run(report, process)?;
        let mut attempt_count = 0usize;
        let mut final_attempt = None;
        let mut final_expected_outputs = Vec::new();
        let mut final_expected_output_status = "not_declared".to_owned();

        for attempt_index in 0..=spec.retry {
            attempt_count += 1;
            let attempt = execute_process_attempt(&spec)?;
            let expected_outputs =
                process_expected_outputs_for_owner(report, spec.line, &spec.cwd)?;
            let expected_output_status = expected_output_status(&expected_outputs);
            let should_retry = process_attempt_should_retry(&attempt, &expected_output_status)
                && attempt_index < spec.retry;
            final_expected_outputs = expected_outputs;
            final_expected_output_status = expected_output_status;
            final_attempt = Some(attempt);
            if !should_retry {
                break;
            }
        }

        let Some(attempt) = final_attempt else {
            continue;
        };
        let exit_code = attempt.exit_code;
        let success = attempt.success;
        let stdout = attempt.stdout;
        let stderr = attempt.stderr;
        let stdout_hash = hash_text(&stdout);
        let stderr_hash = hash_text(&stderr);
        if attempt.timed_out && !spec.allow_failure {
            return Err(invalid_input(&format!(
                "E-PROCESS-TIMEOUT: process `{}` timed out after {}; add `with {{ allow_failure = true }}` to record the timeout as a ProcessResult",
                spec.command,
                spec.timeout_label.as_deref().unwrap_or("the configured timeout")
            )));
        }
        if !success && !spec.allow_failure {
            return Err(invalid_input(&format!(
                "process `{}` exited with code {}; add `with {{ allow_failure = true }}` to record the failure as a ProcessResult",
                spec.command,
                exit_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".to_owned())
            )));
        }
        if final_expected_output_status == "missing" && !spec.allow_failure {
            let missing = final_expected_outputs
                .iter()
                .filter(|output| !output.exists)
                .map(|output| output.path.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(invalid_input(&format!(
                "process `{}` did not create expected output(s): {}; add `with {{ allow_failure = true }}` to record the missing output contract",
                spec.command, missing
            )));
        }
        records.push(ProcessExecutionRecord {
            binding: spec.binding,
            command: spec.command,
            tool_version: spec.tool_version,
            args: spec.args,
            env_keys: spec.env.iter().map(|entry| entry.key.clone()).collect(),
            cwd: spec.cwd_text,
            expected_outputs: final_expected_outputs,
            expected_output_status: final_expected_output_status.clone(),
            timeout: spec.timeout_label,
            retry: spec.retry,
            attempt_count,
            allow_failure: spec.allow_failure,
            timed_out: attempt.timed_out,
            exit_code,
            success,
            stdout,
            stdout_hash,
            stderr,
            stderr_hash,
            duration_ms: attempt.duration_ms,
            status: if attempt.timed_out {
                "timed_out_allowed".to_owned()
            } else if success && final_expected_output_status != "missing" {
                "completed".to_owned()
            } else if success {
                "output_missing_allowed".to_owned()
            } else {
                "failed_allowed".to_owned()
            },
            line: spec.line,
        });
    }
    Ok(records)
}

fn process_spec_for_run(
    report: &CheckReport,
    process: &eng_compiler::ProcessRunInfo,
) -> Result<ProcessSpec, RuntimeError> {
    let args = process_args_for_owner(report, process.line)?;
    let cwd = process_cwd_for_owner(report, process.line)?;
    let timeout_label = process_timeout_label(report, process.line)?;
    let timeout = timeout_label
        .as_deref()
        .map(parse_process_timeout_duration)
        .transpose()?;
    Ok(ProcessSpec {
        binding: process.binding.clone(),
        command: process.command.clone(),
        tool_version: process_string_option(report, process.line, "tool_version"),
        args,
        env: process_env_for_owner(report, process.line)?,
        cwd_text: cwd.display().to_string(),
        cwd,
        timeout,
        timeout_label,
        retry: process_retry_for_owner(report, process.line)?,
        allow_failure: process_bool_option(report, process.line, "allow_failure"),
        line: process.line,
    })
}

fn execute_process_attempt(spec: &ProcessSpec) -> Result<ProcessAttemptOutput, RuntimeError> {
    let started = Instant::now();
    let mut command = Command::new(&spec.command);
    command
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for env_var in &spec.env {
        command.env(&env_var.key, &env_var.value);
    }
    let mut child = command.spawn().map_err(|error| {
        invalid_input(&format!(
            "process `{}` failed to start: {error}",
            spec.command
        ))
    })?;

    let mut timed_out = false;
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if spec
            .timeout
            .is_some_and(|timeout| started.elapsed() >= timeout)
        {
            timed_out = true;
            let _ = child.kill();
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    let output = child.wait_with_output()?;
    let duration_ms = started.elapsed().as_millis();
    Ok(ProcessAttemptOutput {
        exit_code: output.status.code(),
        success: output.status.success() && !timed_out,
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        duration_ms,
        timed_out,
    })
}

fn process_attempt_should_retry(attempt: &ProcessAttemptOutput, expected_status: &str) -> bool {
    attempt.timed_out || !attempt.success || expected_status == "missing"
}

fn cache_manifest_records(
    report: &CheckReport,
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
    build_root: &Path,
) -> Vec<CacheManifestRecord> {
    let mut records = report
        .semantic_program
        .cache_records
        .iter()
        .map(|record| cache_manifest_record(record, runtime_data, process_results, build_root))
        .collect::<Vec<_>>();
    records.extend(case_cache_manifest_records(runtime_data, build_root));
    records
}

fn materialize_network_cache_entries(
    report: &CheckReport,
    records: &mut [CacheManifestRecord],
    build_root: &Path,
) -> Result<(), RuntimeError> {
    let source_base = report.source_path.parent();
    for request in &report.semantic_program.net_requests {
        if request.response_hash.is_none() {
            continue;
        }
        let Some(fixture) = &request.fixture else {
            continue;
        };
        let Some(record) = records.iter_mut().find(|record| {
            record.owner_kind == "network_request" && record.owner_name == request.binding
        }) else {
            continue;
        };
        materialize_network_cache_entry(fixture, source_base, record, build_root)?;
    }
    for download in &report.semantic_program.net_downloads {
        if download.response_hash.is_none() {
            continue;
        }
        let Some(fixture) = &download.fixture else {
            continue;
        };
        let Some(record) = records.iter_mut().find(|record| {
            record.owner_kind == "network_download" && record.owner_name == download.target_value
        }) else {
            continue;
        };
        materialize_network_cache_entry(fixture, source_base, record, build_root)?;
    }
    Ok(())
}

fn materialize_network_cache_entry(
    fixture: &str,
    source_base: Option<&Path>,
    record: &mut CacheManifestRecord,
    build_root: &Path,
) -> Result<(), RuntimeError> {
    if record.lookup_status == "hit" {
        return Ok(());
    }
    let source = runtime_resolve_source_relative_path(fixture, source_base);
    if !source.exists() {
        return Ok(());
    }
    let destination = resolve_cache_path(build_root, &record.cache_path);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&source, &destination)?;
    record.status = "miss_materialized".to_owned();
    record.resolved_path = destination.display().to_string();
    if record.observed_hash.is_none() {
        record.observed_hash = sha256_file_if_exists(&source);
    }
    Ok(())
}

fn cache_manifest_record(
    record: &eng_compiler::CacheRecordInfo,
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
    build_root: &Path,
) -> CacheManifestRecord {
    let resolved_path = resolve_cache_path(build_root, &record.cache_path);
    let lookup_status = if resolved_path.exists() {
        "hit"
    } else {
        "miss"
    };
    let observed_hash = cache_observed_hash(record, runtime_data, process_results).or_else(|| {
        (network_cache_owner(&record.owner_kind) && resolved_path.exists())
            .then(|| sha256_file_if_exists(&resolved_path))
            .flatten()
    });
    let status = cache_manifest_status(record, observed_hash.as_deref(), lookup_status);
    CacheManifestRecord {
        owner_kind: record.owner_kind.clone(),
        owner_name: record.owner_name.clone(),
        cache_key: record.cache_key.clone(),
        cache_key_parts: record.cache_key_parts.clone(),
        cache_key_hash: record.cache_key_hash.clone(),
        cache_path: record.cache_path.clone(),
        cache_dir: record.cache_dir.clone(),
        resolved_path: resolved_path.display().to_string(),
        source_hash: record.source_hash.clone(),
        expected_hash: record.expected_hash.clone(),
        observed_hash,
        lookup_status: lookup_status.to_owned(),
        status,
        line: record.line,
    }
}

fn cache_observed_hash(
    record: &eng_compiler::CacheRecordInfo,
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
) -> Option<String> {
    if record.owner_kind == "process" {
        return process_results
            .iter()
            .find(|process| process.binding == record.owner_name)
            .map(|process| process.stdout_hash.clone())
            .or_else(|| record.observed_hash.clone());
    }
    if record.owner_kind == "model" {
        return runtime_data
            .ml_artifacts
            .iter()
            .find(|artifact| artifact.binding == record.owner_name)
            .and_then(model_cache_observed_hash)
            .or_else(|| record.observed_hash.clone());
    }
    record.observed_hash.clone()
}

fn model_cache_observed_hash(artifact: &RuntimeMlArtifact) -> Option<String> {
    artifact
        .model_artifact_hash
        .clone()
        .or_else(|| artifact.model_card.as_ref().map(|card| hash_text(card)))
}

fn cache_manifest_status(
    record: &eng_compiler::CacheRecordInfo,
    observed_hash: Option<&str>,
    lookup_status: &str,
) -> String {
    cache_status(
        record.expected_hash.as_deref(),
        observed_hash,
        lookup_status,
        record.status == "fixture_available",
    )
}

fn cache_status(
    expected_hash: Option<&str>,
    observed_hash: Option<&str>,
    lookup_status: &str,
    fixture_available: bool,
) -> String {
    if cache_hash_mismatch(expected_hash, observed_hash) {
        "hash_mismatch".to_owned()
    } else if lookup_status == "hit" {
        "hit".to_owned()
    } else if fixture_available {
        "miss_fixture_available".to_owned()
    } else if observed_hash.is_some() {
        "miss_observed".to_owned()
    } else {
        "miss_declared".to_owned()
    }
}

fn case_cache_manifest_records(
    runtime_data: &RuntimeData,
    build_root: &Path,
) -> Vec<CacheManifestRecord> {
    runtime_data
        .case_manifests
        .iter()
        .filter(|manifest| !manifest.case_id.trim().is_empty())
        .map(|manifest| case_cache_manifest_record(manifest, build_root))
        .collect()
}

fn case_cache_manifest_record(
    manifest: &RuntimeCaseManifest,
    build_root: &Path,
) -> CacheManifestRecord {
    let cache_dir = "cache".to_owned();
    let source_hash = manifest
        .source_hash
        .clone()
        .unwrap_or_else(|| manifest.sample_row_hash.clone());
    let cache_key_parts = vec![
        "case".to_owned(),
        format!("sample_table={}", manifest.sample_table),
        format!("case_id={}", manifest.case_id),
        format!("sample_row_hash={}", manifest.sample_row_hash),
        format!("source_hash={source_hash}"),
    ];
    let cache_key = serialize_runtime_cache_key(&cache_key_parts);
    let cache_key_hash = hash_text(&cache_key);
    let cache_path = format!("{}/{}", cache_dir, cache_key_hash);
    let resolved_path = resolve_cache_path(build_root, &cache_path);
    let lookup_status = if resolved_path.exists() {
        "hit"
    } else {
        "miss"
    };
    let status = cache_status(
        Some(&manifest.sample_row_hash),
        Some(&manifest.sample_row_hash),
        lookup_status,
        false,
    );
    CacheManifestRecord {
        owner_kind: "case".to_owned(),
        owner_name: manifest.case_id.clone(),
        cache_key,
        cache_key_parts,
        cache_key_hash,
        cache_path,
        cache_dir,
        resolved_path: resolved_path.display().to_string(),
        source_hash,
        expected_hash: Some(manifest.sample_row_hash.clone()),
        observed_hash: Some(manifest.sample_row_hash.clone()),
        lookup_status: lookup_status.to_owned(),
        status,
        line: manifest.line,
    }
}

fn serialize_runtime_cache_key(parts: &[String]) -> String {
    parts
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("|")
}

fn cache_stale_diagnostics(
    records: &[CacheManifestRecord],
    build_root: &Path,
) -> Vec<CacheDiagnosticRecord> {
    let mut cache_dirs: HashMap<String, (HashSet<String>, usize)> = HashMap::new();
    for record in records {
        let entry = cache_dirs
            .entry(record.cache_dir.clone())
            .or_insert_with(|| (HashSet::new(), record.line));
        entry.0.insert(record.cache_key_hash.clone());
        entry.1 = entry.1.min(record.line);
    }

    let mut diagnostics = Vec::new();
    for (cache_dir, (active_hashes, line)) in cache_dirs {
        let cache_dir_path = resolve_cache_path(build_root, &cache_dir);
        let Ok(entries) = fs::read_dir(&cache_dir_path) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_file() && !file_type.is_dir() {
                continue;
            }
            let cache_key_hash = entry.file_name().to_string_lossy().to_string();
            if active_hashes.contains(&cache_key_hash) {
                continue;
            }
            let cache_path = format!("{}/{}", cache_dir.trim_end_matches('/'), cache_key_hash);
            let resolved_path = entry.path().display().to_string();
            diagnostics.push(CacheDiagnosticRecord {
                severity: "warning",
                code: "W-CACHE-STALE",
                message: format!(
                    "cache entry `{resolved_path}` is not referenced by the current cache manifest"
                ),
                cache_dir: cache_dir.clone(),
                cache_path,
                resolved_path,
                cache_key_hash,
                status: "stale",
                line,
            });
        }
    }
    diagnostics.sort_by(|left, right| left.resolved_path.cmp(&right.resolved_path));
    diagnostics
}

fn ensure_cache_hashes_valid(records: &[CacheManifestRecord]) -> Result<(), RuntimeError> {
    if let Some(record) = records
        .iter()
        .find(|record| record.status.as_str() == "hash_mismatch")
    {
        return Err(invalid_input(&format!(
            "cache hash mismatch at line {} (E-CACHE-HASH-MISMATCH): {} `{}` expected `{}` but observed `{}`",
            record.line,
            record.owner_kind,
            record.owner_name,
            record.expected_hash.as_deref().unwrap_or("-"),
            record.observed_hash.as_deref().unwrap_or("-")
        )));
    }
    Ok(())
}

fn ensure_cache_reproducible(
    records: &[CacheManifestRecord],
    profile: &ExecutionProfile,
) -> Result<(), RuntimeError> {
    if !matches!(profile, ExecutionProfile::Repro) {
        return Ok(());
    }
    if let Some(record) = records
        .iter()
        .find(|record| record.observed_hash.as_deref().is_none())
    {
        return Err(invalid_input(&format!(
            "repro profile rejected cache record at line {} (E-CACHE-UNHASHED-REPRO): {} `{}` has no observed hash",
            record.line, record.owner_kind, record.owner_name
        )));
    }
    Ok(())
}

fn cache_hash_mismatch(expected_hash: Option<&str>, observed_hash: Option<&str>) -> bool {
    match (expected_hash, observed_hash) {
        (Some(expected), Some(observed)) => {
            normalize_cache_hash(expected) != normalize_cache_hash(observed)
        }
        _ => false,
    }
}

fn normalize_cache_hash(value: &str) -> String {
    let trimmed = value.trim();
    let normalized = trimmed.to_ascii_lowercase();
    normalized
        .strip_prefix("sha256:")
        .unwrap_or(&normalized)
        .to_owned()
}

fn resolve_cache_path(build_root: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        build_root.join(path)
    }
}

fn cache_manifest_json(
    report: &CheckReport,
    records: &[CacheManifestRecord],
    diagnostics: &[CacheDiagnosticRecord],
    profile: &ExecutionProfile,
) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"format\": \"eng-cache-manifest-v1\",\n");
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
    json.push_str(&format!("  \"diagnostic_count\": {},\n", diagnostics.len()));
    json.push_str("  \"diagnostics\": [\n");
    push_cache_diagnostics_json(&mut json, diagnostics, "    ");
    json.push_str("\n  ],\n");
    json.push_str(&format!("  \"cache_record_count\": {},\n", records.len()));
    json.push_str("  \"cache_records\": [\n");
    push_cache_manifest_records_json(&mut json, records, "    ");
    json.push_str("\n  ]\n");
    json.push_str("}\n");
    json
}

fn push_cache_diagnostics_json(
    json: &mut String,
    diagnostics: &[CacheDiagnosticRecord],
    indent: &str,
) {
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}{{\n"));
        json.push_str(&format!(
            "{indent}  \"severity\": \"{}\",\n",
            diagnostic.severity
        ));
        json.push_str(&format!("{indent}  \"code\": \"{}\",\n", diagnostic.code));
        json.push_str(&format!(
            "{indent}  \"message\": \"{}\",\n",
            json_escape(&diagnostic.message)
        ));
        json.push_str(&format!(
            "{indent}  \"cache_dir\": \"{}\",\n",
            json_escape(&diagnostic.cache_dir)
        ));
        json.push_str(&format!(
            "{indent}  \"cache_path\": \"{}\",\n",
            json_escape(&diagnostic.cache_path)
        ));
        json.push_str(&format!(
            "{indent}  \"resolved_path\": \"{}\",\n",
            json_escape(&diagnostic.resolved_path)
        ));
        json.push_str(&format!(
            "{indent}  \"cache_key_hash\": \"{}\",\n",
            json_escape(&diagnostic.cache_key_hash)
        ));
        json.push_str(&format!(
            "{indent}  \"status\": \"{}\",\n",
            diagnostic.status
        ));
        json.push_str(&format!("{indent}  \"line\": {}\n", diagnostic.line));
        json.push_str(&format!("{indent}}}"));
    }
}

fn push_cache_manifest_records_json(
    json: &mut String,
    records: &[CacheManifestRecord],
    indent: &str,
) {
    for (index, record) in records.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}{{\n"));
        json.push_str(&format!(
            "{indent}  \"owner_kind\": \"{}\",\n",
            json_escape(&record.owner_kind)
        ));
        json.push_str(&format!(
            "{indent}  \"owner_name\": \"{}\",\n",
            json_escape(&record.owner_name)
        ));
        json.push_str(&format!(
            "{indent}  \"cache_key\": \"{}\",\n",
            json_escape(&record.cache_key)
        ));
        json.push_str(&format!("{indent}  \"cache_key_parts\": "));
        push_json_string_array_runtime(json, &record.cache_key_parts);
        json.push_str(",\n");
        json.push_str(&format!(
            "{indent}  \"cache_key_hash\": \"{}\",\n",
            json_escape(&record.cache_key_hash)
        ));
        json.push_str(&format!(
            "{indent}  \"cache_path\": \"{}\",\n",
            json_escape(&record.cache_path)
        ));
        json.push_str(&format!(
            "{indent}  \"cache_dir\": \"{}\",\n",
            json_escape(&record.cache_dir)
        ));
        json.push_str(&format!(
            "{indent}  \"resolved_path\": \"{}\",\n",
            json_escape(&record.resolved_path)
        ));
        json.push_str(&format!(
            "{indent}  \"source_hash\": \"{}\",\n",
            json_escape(&record.source_hash)
        ));
        push_optional_json_string_runtime(
            json,
            "expected_hash",
            record.expected_hash.as_deref(),
            indent.len() + 2,
        );
        push_optional_json_string_runtime(
            json,
            "observed_hash",
            record.observed_hash.as_deref(),
            indent.len() + 2,
        );
        json.push_str(&format!(
            "{indent}  \"lookup_status\": \"{}\",\n",
            json_escape(&record.lookup_status)
        ));
        json.push_str(&format!(
            "{indent}  \"status\": \"{}\",\n",
            json_escape(&record.status)
        ));
        json.push_str(&format!("{indent}  \"line\": {}\n", record.line));
        json.push_str(&format!("{indent}}}"));
    }
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

fn case_collection_records(records: &[ProcessExecutionRecord]) -> Vec<CaseCollectionRecord> {
    records
        .iter()
        .flat_map(|record| {
            record
                .expected_outputs
                .iter()
                .filter(|output| is_case_collection_output(output))
                .map(|output| case_collection_record(record, output))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn is_case_collection_output(output: &ProcessExpectedOutputRecord) -> bool {
    let path = output.path.to_ascii_lowercase();
    output.artifact_kind == "result_collection"
        || path.contains("result_collection_manifest")
        || path.contains("case_collection_manifest")
}

fn case_collection_record(
    _process: &ProcessExecutionRecord,
    output: &ProcessExpectedOutputRecord,
) -> CaseCollectionRecord {
    let mut record = CaseCollectionRecord {
        manifest_path: output.path.clone(),
        status: if output.exists {
            "manifest_unread".to_owned()
        } else {
            "missing".to_owned()
        },
        expected_case_ids: Vec::new(),
        result_case_ids: Vec::new(),
        missing_case_ids: Vec::new(),
        failed_case_count: 0,
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
    record.expected_case_ids = json_field_string_array(&value, "expected_case_ids");
    record.result_case_ids = json_field_string_array(&value, "result_case_ids");
    record.missing_case_ids = json_field_string_array(&value, "missing_case_ids");
    record.failed_case_count = value
        .get("failed_case_count")
        .and_then(Value::as_u64)
        .map(|count| count as usize)
        .unwrap_or(record.missing_case_ids.len());
    record.status =
        json_field_string(&value, "status").unwrap_or_else(|| "manifest_loaded".to_owned());
    record
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
        database_hash_before: None,
        database_hash_after: None,
        transaction_status: None,
        schema_status: None,
        tables: Vec::new(),
        diagnostics: Vec::new(),
        status: if output.exists {
            "manifest_unread".to_owned()
        } else {
            "missing".to_owned()
        },
        line: process.line,
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
    record.database_hash_before = json_field_string(&value, "database_hash_before");
    record.database_hash_after = json_field_string(&value, "database_hash_after");
    record.transaction_status = json_field_string(&value, "transaction_status");
    record.schema_status = json_field_string(&value, "schema_status");
    record.diagnostics = db_manifest_diagnostics_from_json(&value);
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

fn db_manifest_diagnostics_from_json(value: &Value) -> Vec<DbManifestDiagnosticRecord> {
    let mut diagnostics = Vec::new();
    if let Some(items) = value.get("diagnostics").and_then(Value::as_array) {
        diagnostics.extend(
            items.iter().map(|item| DbManifestDiagnosticRecord {
                code: json_field_string(item, "code")
                    .unwrap_or_else(|| "E-DB-SCHEMA-MISMATCH".to_owned()),
                message: json_field_string(item, "message").unwrap_or_default(),
                table: json_field_string(item, "table"),
            }),
        );
    }
    if let Some(items) = value
        .get("schema_mismatch_diagnostics")
        .and_then(Value::as_array)
    {
        diagnostics.extend(items.iter().filter_map(Value::as_str).map(|message| {
            DbManifestDiagnosticRecord {
                code: "E-DB-SCHEMA-MISMATCH".to_owned(),
                message: message.to_owned(),
                table: None,
            }
        }));
    }
    diagnostics
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
        json.push_str("      \"env\": ");
        push_json_string_array_runtime(&mut json, &record.env_keys);
        json.push_str(",\n");
        json.push_str(&format!(
            "      \"cwd\": \"{}\",\n",
            json_escape(&record.cwd)
        ));
        push_optional_json_string_runtime(&mut json, "timeout", record.timeout.as_deref(), 6);
        json.push_str(&format!("      \"retry\": {},\n", record.retry));
        json.push_str(&format!(
            "      \"attempt_count\": {},\n",
            record.attempt_count
        ));
        json.push_str(&format!(
            "      \"allow_failure\": {},\n",
            record.allow_failure
        ));
        json.push_str(&format!("      \"timed_out\": {},\n", record.timed_out));
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

fn process_env_for_owner(
    report: &CheckReport,
    owner_line: usize,
) -> Result<Vec<ProcessEnvVar>, RuntimeError> {
    let Some(raw) = process_option(report, owner_line, "env") else {
        return Ok(Vec::new());
    };
    let trimmed = raw.trim();
    let inner = trimmed
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .ok_or_else(|| invalid_input("process env must be an inline object"))?;
    let mut env_vars = Vec::new();
    for entry in split_template_top_level(inner, &[',', ';']) {
        let (key, value_expression) = split_template_assignment(&entry).ok_or_else(|| {
            invalid_input(&format!(
                "process env entry `{entry}` must use `NAME = expression`"
            ))
        })?;
        let key = key.trim();
        if !is_process_env_key_runtime(key) {
            return Err(invalid_input(&format!(
                "process env key `{key}` is not portable"
            )));
        }
        let value = process_env_value(value_expression, report).ok_or_else(|| {
            invalid_input(&format!(
                "cannot resolve process env value `{}`",
                value_expression.trim()
            ))
        })?;
        env_vars.push(ProcessEnvVar {
            key: key.to_owned(),
            value,
        });
    }
    Ok(env_vars)
}

fn process_env_value(expression: &str, report: &CheckReport) -> Option<String> {
    evaluate_runtime_expression(expression, report, &RuntimeData::default())
        .map(|value| format_runtime_value(value, None, None, true))
        .or_else(|| evaluate_runtime_path_expression(expression, report))
        .or_else(|| Some(strip_runtime_string_value(expression)).filter(|value| !value.is_empty()))
}

fn is_process_env_key_runtime(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn process_timeout_label(
    report: &CheckReport,
    owner_line: usize,
) -> Result<Option<String>, RuntimeError> {
    process_option(report, owner_line, "timeout")
        .map(|value| normalize_process_timeout(&value).map_err(|message| invalid_input(&message)))
        .transpose()
}

fn normalize_process_timeout(value: &str) -> Result<String, String> {
    let seconds = parse_process_timeout_seconds(value)?;
    Ok(format!("{} s", format_number_with_precision(seconds, None)))
}

fn parse_process_timeout_duration(value: &str) -> Result<Duration, RuntimeError> {
    let seconds =
        parse_process_timeout_seconds(value).map_err(|message| invalid_input(&message))?;
    Ok(Duration::from_secs_f64(seconds))
}

fn parse_process_timeout_seconds(value: &str) -> Result<f64, String> {
    let (amount, unit) = number_with_optional_unit(value).ok_or_else(|| {
        "process timeout must be a duration such as `10 s` or `10 min`".to_owned()
    })?;
    if !amount.is_finite() || amount <= 0.0 {
        return Err("process timeout must be positive and finite".to_owned());
    }
    let Some(unit) = unit else {
        return Err("process timeout must include a duration unit".to_owned());
    };
    match unit.trim().to_ascii_lowercase().as_str() {
        "s" | "sec" | "second" | "seconds" => Ok(amount),
        "min" | "minute" | "minutes" => Ok(amount * 60.0),
        "h" | "hr" | "hour" | "hours" => Ok(amount * 3600.0),
        _ => Err("process timeout units are s, min, and h".to_owned()),
    }
}

fn process_retry_for_owner(report: &CheckReport, owner_line: usize) -> Result<usize, RuntimeError> {
    const MAX_PROCESS_RETRY_ATTEMPTS: usize = 5;
    let Some(raw) = process_option(report, owner_line, "retry") else {
        return Ok(0);
    };
    let value = raw.trim().parse::<usize>().map_err(|_| {
        invalid_input(&format!(
            "process retry policy `{}` is not a whole number",
            raw.trim()
        ))
    })?;
    if value > MAX_PROCESS_RETRY_ATTEMPTS {
        return Err(invalid_input(&format!(
            "process retry policy `{value}` exceeds the maximum of {MAX_PROCESS_RETRY_ATTEMPTS}"
        )));
    }
    Ok(value)
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

fn render_runtime_text_template(
    template: &str,
    report: &CheckReport,
    runtime_data: &RuntimeData,
) -> String {
    let mut rendered = String::new();
    let mut cursor = 0usize;
    while let Some(open) = template[cursor..].find('{') {
        let start = cursor + open;
        rendered.push_str(&template[cursor..start]);
        let Some(close_offset) = template[start + 1..].find('}') else {
            rendered.push_str(&template[start..]);
            return rendered;
        };
        let close = start + 1 + close_offset;
        let field_text = template[start + 1..close].trim();
        let fallback = format!("{{{field_text}}}");
        let (expression, spec) = split_runtime_format_field(field_text);
        let format_spec = parse_runtime_format_spec(spec.unwrap_or_default());
        let value = evaluate_runtime_expression(expression, report, runtime_data)
            .map(|value| {
                format_runtime_value(
                    value,
                    format_spec.unit.as_deref(),
                    format_spec.precision,
                    true,
                )
            })
            .unwrap_or(fallback);
        rendered.push_str(&value);
        cursor = close + 1;
    }
    rendered.push_str(&template[cursor..]);
    rendered
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct RuntimeFormatSpec {
    precision: Option<usize>,
    unit: Option<String>,
}

fn split_runtime_format_field(field: &str) -> (&str, Option<&str>) {
    field
        .split_once(':')
        .map(|(expression, spec)| (expression.trim(), Some(spec.trim())))
        .unwrap_or((field.trim(), None))
}

fn parse_runtime_format_spec(spec: &str) -> RuntimeFormatSpec {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return RuntimeFormatSpec::default();
    }
    let Some(after_dot) = trimmed.strip_prefix('.') else {
        return RuntimeFormatSpec {
            precision: None,
            unit: Some(trimmed.to_owned()),
        };
    };
    let digit_count = after_dot
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .count();
    let precision = if digit_count > 0 {
        after_dot[..digit_count].parse::<usize>().ok()
    } else {
        None
    };
    let unit_text = after_dot[digit_count..].trim();
    let unit = if unit_text.is_empty() {
        None
    } else {
        Some(unit_text.to_owned())
    };
    RuntimeFormatSpec { precision, unit }
}

struct ArtifactRegistryContext<'a> {
    report: &'a CheckReport,
    runtime_data: &'a RuntimeData,
    external_boundary_records: &'a [ExternalBoundaryRecord],
    cache_manifest_records: &'a [CacheManifestRecord],
    db_manifest_records: &'a [DbManifestRecord],
    test_results: &'a [TestExecutionRecord],
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TemplatePlaceholderRecord {
    name: String,
    value: Option<String>,
    status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TemplateRenderRecord {
    binding: String,
    template_path: String,
    output_path: String,
    manifest_path: String,
    template_hash: String,
    values_hash: String,
    output_hash: String,
    placeholder_count: usize,
    substituted_count: usize,
    missing_count: usize,
    placeholders: Vec<TemplatePlaceholderRecord>,
    status: String,
    line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TemplateRenderOutput {
    artifacts: Vec<OutputArtifact>,
    records: Vec<TemplateRenderRecord>,
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
            Some(OutputArtifact::new(
                kind.to_owned(),
                path_for_manifest(&expected.resolved_path),
                hash.clone(),
                expected.resolved_path.clone(),
                expected.validation.clone(),
            ))
        })
        .collect()
}

fn output_artifact(
    kind: &str,
    path: String,
    contents: &str,
    absolute_path: PathBuf,
) -> OutputArtifact {
    OutputArtifact::new(
        kind.to_owned(),
        path,
        hash_text(contents),
        absolute_path,
        artifact_validation(
            "passed",
            "content_hash",
            "generated artifact was written and hashed",
        ),
    )
}

fn output_artifact_with_overwrite_policy(
    kind: &str,
    path: String,
    contents: &str,
    absolute_path: PathBuf,
    overwrite_policy: &str,
) -> OutputArtifact {
    output_artifact(kind, path, contents, absolute_path)
        .with_overwrite_policy(overwrite_policy.to_owned())
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
        let overwrite_policy = overwrite_policy(report, export.line);
        write_output_file(&path, &csv, overwrite_policy.allowed)?;
        artifacts.push(output_artifact_with_overwrite_policy(
            "csv_export",
            relative_output_path(result_dir, &path),
            &csv,
            path,
            overwrite_policy.label,
        ));
    }
    Ok(artifacts)
}

fn render_template_outputs(
    report: &CheckReport,
    runtime_data: &RuntimeData,
    result_dir: &Path,
) -> Result<TemplateRenderOutput, RuntimeError> {
    let mut artifacts = Vec::new();
    let mut records = Vec::new();
    for command in report
        .semantic_program
        .command_styles
        .iter()
        .filter(|command| {
            command.verb == "render"
                && command.status == "lowered"
                && command.target.trim().starts_with("template ")
        })
    {
        let source_expression = command
            .target
            .trim()
            .strip_prefix("template ")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| invalid_input("render template command is missing a template source"))?;
        let source_path_text = evaluate_runtime_path_expression(source_expression, report)
            .ok_or_else(|| {
                invalid_input(&format!(
                    "invalid render template source `{source_expression}`"
                ))
            })?;
        let source_path =
            runtime_resolve_source_relative_path(&source_path_text, report.source_path.parent());
        let template_text = fs::read_to_string(&source_path)?;

        let output_expression = template_command_output_expression(report, command)
            .ok_or_else(|| invalid_input("render template command is missing output path"))?;
        let output_path_text = evaluate_runtime_output_path_expression(&output_expression, report)
            .ok_or_else(|| {
                invalid_input(&format!(
                    "invalid render template output `{output_expression}`"
                ))
            })?;
        let output_path = export_output_path(result_dir, &output_path_text).ok_or_else(|| {
            invalid_input(&format!(
                "render template output `{output_path_text}` is outside the output boundary"
            ))
        })?;
        let output_relative_path = relative_output_path(result_dir, &output_path);
        let manifest_relative_path =
            runtime_path_text(format!("{output_relative_path}.render_manifest.json"));
        let manifest_path = export_output_path(result_dir, &manifest_relative_path).ok_or_else(
            || {
                invalid_input(&format!(
                    "render template manifest `{manifest_relative_path}` is outside the output boundary"
                ))
            },
        )?;

        let explicit_values = template_values_for_owner(report, runtime_data, command.line)?;
        let missing_policy = template_missing_policy(report, command.line)?;
        let placeholders = template_placeholders(&template_text);
        let mut rendered = String::new();
        let mut cursor = 0usize;
        let mut placeholder_records = Vec::new();
        let mut substituted_count = 0usize;
        let mut missing_count = 0usize;
        let mut resolved_values = HashMap::new();
        for placeholder in &placeholders {
            rendered.push_str(&template_text[cursor..placeholder.start]);
            let replacement =
                template_value_for_placeholder(placeholder, &explicit_values, report, runtime_data);
            match replacement {
                Some(value) => {
                    rendered.push_str(&value);
                    substituted_count += 1;
                    resolved_values
                        .entry(placeholder.name.clone())
                        .or_insert_with(|| value.clone());
                    placeholder_records.push(TemplatePlaceholderRecord {
                        name: placeholder.name.clone(),
                        value: Some(value),
                        status: "substituted".to_owned(),
                    });
                }
                None => match missing_policy {
                    TemplateMissingPolicy::Error => {
                        return Err(invalid_input(&format!(
                            "E-TEMPLATE-MISSING-VALUE: missing template value `{}` at line {}",
                            placeholder.name, command.line
                        )));
                    }
                    TemplateMissingPolicy::Keep => {
                        rendered.push_str(&placeholder.raw);
                        missing_count += 1;
                        placeholder_records.push(TemplatePlaceholderRecord {
                            name: placeholder.name.clone(),
                            value: None,
                            status: "missing_kept".to_owned(),
                        });
                    }
                    TemplateMissingPolicy::Empty => {
                        missing_count += 1;
                        placeholder_records.push(TemplatePlaceholderRecord {
                            name: placeholder.name.clone(),
                            value: Some(String::new()),
                            status: "missing_empty".to_owned(),
                        });
                    }
                },
            }
            cursor = placeholder.end;
        }
        rendered.push_str(&template_text[cursor..]);

        let overwrite_policy = overwrite_policy(report, command.line);
        write_output_file(&output_path, &rendered, overwrite_policy.allowed)?;
        let output_hash = hash_text(&rendered);
        let template_hash = hash_text(&template_text);
        let values_hash = template_string_values_hash(&resolved_values);
        let record = TemplateRenderRecord {
            binding: command
                .owner
                .clone()
                .unwrap_or_else(|| format!("render:{}", command.line)),
            template_path: path_for_manifest(&source_path),
            output_path: output_relative_path.clone(),
            manifest_path: manifest_relative_path.clone(),
            template_hash,
            values_hash,
            output_hash,
            placeholder_count: placeholders.len(),
            substituted_count,
            missing_count,
            placeholders: placeholder_records,
            status: "rendered".to_owned(),
            line: command.line,
        };
        let manifest_json = template_render_manifest_json(&record);
        write_output_file(&manifest_path, &manifest_json, overwrite_policy.allowed)?;
        let artifact_kind = artifact_kind_for_owner(report, command.line, "template_render");
        artifacts.push(output_artifact_with_overwrite_policy(
            &artifact_kind,
            output_relative_path,
            &rendered,
            output_path,
            overwrite_policy.label,
        ));
        artifacts.push(output_artifact_with_overwrite_policy(
            "template_render_manifest",
            manifest_relative_path,
            &manifest_json,
            manifest_path,
            overwrite_policy.label,
        ));
        records.push(record);
    }
    Ok(TemplateRenderOutput { artifacts, records })
}

#[derive(Clone, Debug, PartialEq)]
struct TemplateValue {
    value: RuntimeFormatValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TemplatePlaceholder {
    start: usize,
    end: usize,
    raw: String,
    name: String,
    requested_unit: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TemplateMissingPolicy {
    Error,
    Keep,
    Empty,
}

fn template_render_node_id(command: &eng_compiler::CommandStyleInfo) -> String {
    format!("template_render:{}", command.line)
}

fn template_command_output_expression(
    report: &CheckReport,
    command: &eng_compiler::CommandStyleInfo,
) -> Option<String> {
    command
        .clauses
        .iter()
        .find(|clause| clause.name == "to")
        .map(|clause| clause.value.clone())
        .or_else(|| process_option(report, command.line, "output"))
}

fn evaluate_runtime_output_path_expression(
    expression: &str,
    report: &CheckReport,
) -> Option<String> {
    evaluate_runtime_path_expression(expression, report).or_else(|| {
        let value = strip_runtime_string_value(expression);
        (!value.trim().is_empty()).then(|| runtime_path_text(value))
    })
}

fn template_missing_policy(
    report: &CheckReport,
    owner_line: usize,
) -> Result<TemplateMissingPolicy, RuntimeError> {
    let raw = process_option(report, owner_line, "missing").unwrap_or_else(|| "error".to_owned());
    match strip_runtime_string_value(&raw)
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "error" => Ok(TemplateMissingPolicy::Error),
        "keep" => Ok(TemplateMissingPolicy::Keep),
        "empty" => Ok(TemplateMissingPolicy::Empty),
        other => Err(invalid_input(&format!(
            "invalid render template missing policy `{other}`; use error, keep, or empty"
        ))),
    }
}

fn template_values_for_owner(
    report: &CheckReport,
    runtime_data: &RuntimeData,
    owner_line: usize,
) -> Result<HashMap<String, TemplateValue>, RuntimeError> {
    let Some(raw) = process_option(report, owner_line, "values") else {
        return Ok(HashMap::new());
    };
    let trimmed = raw.trim();
    let inner = trimmed
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .ok_or_else(|| {
            invalid_input(
                "render template `values` must be an inline object such as `{ name = value }`",
            )
        })?;
    let mut values = HashMap::new();
    for entry in split_template_top_level(inner, &[',', ';']) {
        let (key, expression) = split_template_assignment(&entry).ok_or_else(|| {
            invalid_input(&format!(
                "render template values entry `{entry}` must use `name = expression`"
            ))
        })?;
        let key = strip_runtime_string_value(key.trim());
        if !is_template_placeholder_name(&key) {
            return Err(invalid_input(&format!(
                "render template value key `{key}` is not a valid placeholder name"
            )));
        }
        let value = evaluate_runtime_expression(expression.trim(), report, runtime_data)
            .ok_or_else(|| {
                invalid_input(&format!(
                    "cannot resolve render template value `{}`",
                    expression.trim()
                ))
            })?;
        values.insert(key, TemplateValue { value });
    }
    Ok(values)
}

fn template_value_for_placeholder(
    placeholder: &TemplatePlaceholder,
    explicit_values: &HashMap<String, TemplateValue>,
    report: &CheckReport,
    runtime_data: &RuntimeData,
) -> Option<String> {
    explicit_values
        .get(&placeholder.name)
        .map(|value| {
            format_runtime_value(
                value.value.clone(),
                placeholder.requested_unit.as_deref(),
                None,
                true,
            )
        })
        .or_else(|| {
            evaluate_runtime_expression(&placeholder.name, report, runtime_data).map(|value| {
                format_runtime_value(value, placeholder.requested_unit.as_deref(), None, true)
            })
        })
}

fn template_placeholders(template: &str) -> Vec<TemplatePlaceholder> {
    let mut placeholders = Vec::new();
    let mut cursor = 0usize;
    while let Some(open_offset) = template[cursor..].find("{{") {
        let start = cursor + open_offset;
        let content_start = start + 2;
        let Some(close_offset) = template[content_start..].find("}}") else {
            break;
        };
        let end = content_start + close_offset + 2;
        let raw = template[start..end].to_owned();
        let content = template[content_start..content_start + close_offset].trim();
        if let Some((name, requested_unit)) = parse_template_placeholder_content(content) {
            placeholders.push(TemplatePlaceholder {
                start,
                end,
                raw,
                name,
                requested_unit,
            });
        }
        cursor = end;
    }
    placeholders
}

fn parse_template_placeholder_content(content: &str) -> Option<(String, Option<String>)> {
    let (name, requested_unit) = content
        .split_once(':')
        .map(|(name, unit)| (name.trim(), Some(unit.trim())))
        .unwrap_or((content.trim(), None));
    if !is_template_placeholder_name(name) {
        return None;
    }
    Some((
        name.to_owned(),
        requested_unit
            .filter(|unit| !unit.is_empty())
            .map(str::to_owned),
    ))
}

fn is_template_placeholder_name(name: &str) -> bool {
    let mut parts = name.split('.');
    parts.next().is_some_and(|part| is_identifier(part)) && parts.all(is_identifier)
}

fn split_template_assignment(entry: &str) -> Option<(&str, &str)> {
    let mut paren_depth = 0i32;
    let mut brace_depth = 0i32;
    let mut bracket_depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in entry.char_indices() {
        if in_string {
            escaped = character == '\\' && !escaped;
            if character == '"' && !escaped {
                in_string = false;
            }
            if character != '\\' {
                escaped = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' => paren_depth += 1,
            ')' => paren_depth -= 1,
            '{' => brace_depth += 1,
            '}' => brace_depth -= 1,
            '[' => bracket_depth += 1,
            ']' => bracket_depth -= 1,
            '=' if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 => {
                let key = entry[..index].trim();
                let value = entry[index + 1..].trim();
                if key.is_empty() || value.is_empty() {
                    return None;
                }
                return Some((key, value));
            }
            _ => {}
        }
    }
    None
}

fn split_template_top_level(expression: &str, delimiters: &[char]) -> Vec<String> {
    let mut parts = Vec::new();
    let mut paren_depth = 0i32;
    let mut brace_depth = 0i32;
    let mut bracket_depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    let mut start = 0usize;
    for (index, character) in expression.char_indices() {
        if in_string {
            escaped = character == '\\' && !escaped;
            if character == '"' && !escaped {
                in_string = false;
            }
            if character != '\\' {
                escaped = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' => paren_depth += 1,
            ')' => paren_depth -= 1,
            '{' => brace_depth += 1,
            '}' => brace_depth -= 1,
            '[' => bracket_depth += 1,
            ']' => bracket_depth -= 1,
            other
                if paren_depth == 0
                    && brace_depth == 0
                    && bracket_depth == 0
                    && delimiters.contains(&other) =>
            {
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

fn template_string_values_hash(values: &HashMap<String, String>) -> String {
    let mut entries = values.iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(right.0));
    let mut payload = String::new();
    payload.push('{');
    for (index, (key, value)) in entries.iter().enumerate() {
        if index > 0 {
            payload.push(',');
        }
        payload.push_str(&format!(
            "\"{}\":\"{}\"",
            json_escape(key),
            json_escape(value)
        ));
    }
    payload.push('}');
    hash_text(&payload)
}

fn template_render_manifest_json(record: &TemplateRenderRecord) -> String {
    let document = template_render_record_value(record);
    format!(
        "{}\n",
        serde_json::to_string_pretty(&document).expect("serialize template render manifest")
    )
}

fn template_render_record_value(record: &TemplateRenderRecord) -> Value {
    let placeholders = record
        .placeholders
        .iter()
        .map(|placeholder| {
            json!({
                "name": placeholder.name,
                "value": placeholder.value,
                "status": placeholder.status
            })
        })
        .collect::<Vec<_>>();
    json!({
        "format": "eng-template-render-manifest-v1",
        "binding": record.binding,
        "template_path": record.template_path,
        "output_path": record.output_path,
        "manifest_path": record.manifest_path,
        "template_hash": record.template_hash,
        "values_hash": record.values_hash,
        "output_hash": record.output_hash,
        "placeholder_count": record.placeholder_count,
        "substituted_count": record.substituted_count,
        "missing_count": record.missing_count,
        "placeholders": placeholders,
        "status": record.status,
        "line": record.line
    })
}

fn template_render_records_json(records: &[TemplateRenderRecord], indent: &str) -> String {
    let mut json = String::new();
    for (index, record) in records.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        let object = serde_json::to_string_pretty(&template_render_record_value(record))
            .expect("serialize template render record");
        for (line_index, line) in object.lines().enumerate() {
            if line_index > 0 {
                json.push('\n');
            }
            json.push_str(indent);
            json.push_str(line);
        }
    }
    json
}

fn write_outputs(
    report: &CheckReport,
    runtime_data: &RuntimeData,
    result_dir: &Path,
) -> Result<Vec<OutputArtifact>, RuntimeError> {
    let mut artifacts = Vec::new();
    for write in report
        .semantic_program
        .writes
        .iter()
        .filter(|write| write.format != "db")
    {
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
        let overwrite_policy = overwrite_policy(report, write.line);
        write_output_file(&path, &contents, overwrite_policy.allowed)?;
        let artifact_kind =
            artifact_kind_for_owner(report, write.line, &format!("write_{}", write.format));
        artifacts.push(output_artifact_with_overwrite_policy(
            &artifact_kind,
            relative_output_path(result_dir, &path),
            &contents,
            path,
            overwrite_policy.label,
        ));
    }
    Ok(artifacts)
}

#[derive(Clone, Debug)]
struct NativeDbWriteOutput {
    artifacts: Vec<OutputArtifact>,
    records: Vec<DbManifestRecord>,
}

fn execute_native_db_writes(
    report: &CheckReport,
    runtime_data: &RuntimeData,
    result_dir: &Path,
) -> Result<NativeDbWriteOutput, RuntimeError> {
    let mut output = NativeDbWriteOutput {
        artifacts: Vec::new(),
        records: Vec::new(),
    };
    for write in report
        .semantic_program
        .writes
        .iter()
        .filter(|write| write.format == "db")
    {
        let (artifacts, record) = execute_native_db_write(report, runtime_data, result_dir, write)?;
        output.artifacts.extend(artifacts);
        output.records.push(record);
    }
    Ok(output)
}

fn execute_native_db_write(
    report: &CheckReport,
    runtime_data: &RuntimeData,
    result_dir: &Path,
    write: &eng_compiler::WriteInfo,
) -> Result<(Vec<OutputArtifact>, DbManifestRecord), RuntimeError> {
    let (connection_binding, table_name) = db_table_target_parts(&write.path).ok_or_else(|| {
        invalid_input(&format!(
            "E-DB-CONNECT: invalid DB table target `{}`",
            write.path
        ))
    })?;
    let db_path_text = db_connection_path_text(report, &connection_binding).ok_or_else(|| {
        invalid_input(&format!(
            "E-DB-CONNECT: cannot resolve SQLite connection `{connection_binding}`"
        ))
    })?;
    let db_path = export_output_path(result_dir, &db_path_text).ok_or_else(|| {
        invalid_input(&format!(
            "E-DB-CONNECT: SQLite path `{db_path_text}` is outside the output root"
        ))
    })?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let db_relative_path = relative_output_path(result_dir, &db_path);
    let manifest_relative_path = format!("{db_relative_path}.db_write_manifest.json");
    let manifest_path =
        export_output_path(result_dir, &manifest_relative_path).ok_or_else(|| {
            invalid_input(&format!(
            "E-DB-CONNECT: DB manifest path `{manifest_relative_path}` is outside the output root"
        ))
        })?;
    let table = runtime_data
        .tables
        .iter()
        .find(|table| table.binding == write.expression.trim())
        .ok_or_else(|| {
            invalid_input(&format!(
                "E-DB-SCHEMA-MISMATCH: DB write source `{}` was not materialized",
                write.expression
            ))
        })?;
    let mode = db_write_mode(report, write.line)?;
    let key = db_write_key(report, write.line);
    let transaction_policy = db_transaction_policy(report, write.line)?;
    let database_hash_before = hash_file_if_exists(&db_path);
    let mut diagnostics =
        db_write_preflight_diagnostics(&db_path, &table_name, table, &mode, &key)?;
    let mut transaction_status = "committed".to_owned();
    let mut rows_written = table.row_count as u64;

    if diagnostics.is_empty() {
        execute_sqlite_write_payload(
            &db_path,
            &table_name,
            table,
            &mode,
            &key,
            &transaction_policy,
        )?;
        if transaction_policy == "rollback" {
            transaction_status = "rolled_back".to_owned();
        }
    } else {
        transaction_status = "rolled_back".to_owned();
        rows_written = 0;
    }

    let database_hash_after = hash_file_if_exists(&db_path);
    let schema_status = if diagnostics.is_empty() {
        "ok"
    } else {
        "mismatch"
    }
    .to_owned();
    let mut record = DbManifestRecord {
        binding: write.expression.clone(),
        manifest_path: manifest_relative_path.clone(),
        resolved_path: manifest_path.display().to_string(),
        hash: None,
        database: Some(db_relative_path.clone()),
        database_hash_before,
        database_hash_after: database_hash_after.clone(),
        transaction_status: Some(transaction_status),
        schema_status: Some(schema_status),
        tables: vec![DbManifestTableRecord {
            name: table_name.clone(),
            mode: mode.clone(),
            key: key.clone(),
            schema: table
                .columns
                .iter()
                .map(|column| column.name.clone())
                .collect(),
            row_count: Some(rows_written),
        }],
        diagnostics: std::mem::take(&mut diagnostics),
        status: "manifest_loaded".to_owned(),
        line: write.line,
    };
    let manifest_json = native_db_write_manifest_json(&record);
    write_output_file(&manifest_path, &manifest_json, true)?;
    let manifest_hash = hash_text(&manifest_json);
    record.hash = Some(manifest_hash.clone());

    let mut artifacts = Vec::new();
    if let Some(hash) = database_hash_after {
        artifacts.push(
            OutputArtifact::new(
                "sqlite_database".to_owned(),
                db_relative_path,
                hash,
                db_path,
                artifact_validation("passed", "sqlite_write", "SQLite database was written"),
            )
            .with_overwrite_policy("append_or_upsert".to_owned()),
        );
    }
    artifacts.push(output_artifact_with_overwrite_policy(
        "db_write_manifest",
        manifest_relative_path,
        &manifest_json,
        manifest_path,
        "allowed",
    ));
    Ok((artifacts, record))
}

fn db_table_target_parts(expression: &str) -> Option<(String, String)> {
    let (connection, table_call) = expression.trim().split_once(".table(")?;
    let table = table_call.trim().strip_suffix(')')?.trim();
    Some((
        connection.trim().to_owned(),
        strip_runtime_string_value(table),
    ))
    .filter(|(connection, table)| !connection.is_empty() && !table.is_empty())
}

fn db_connection_path_text(report: &CheckReport, binding: &str) -> Option<String> {
    let declaration = report
        .inferred_declarations
        .iter()
        .find(|declaration| declaration.name == binding)?;
    let path_expression = declaration
        .expression
        .trim()
        .strip_prefix("open sqlite ")?
        .trim();
    evaluate_runtime_output_path_expression(path_expression, report)
}

fn db_write_mode(report: &CheckReport, owner_line: usize) -> Result<String, RuntimeError> {
    let raw = process_option(report, owner_line, "mode").unwrap_or_else(|| "append".to_owned());
    match strip_runtime_string_value(&raw)
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "" | "append" | "insert" => Ok("append".to_owned()),
        "upsert" => Ok("upsert".to_owned()),
        other => Err(invalid_input(&format!(
            "E-DB-SCHEMA-MISMATCH: unsupported DB write mode `{other}`; use append or upsert"
        ))),
    }
}

fn db_transaction_policy(report: &CheckReport, owner_line: usize) -> Result<String, RuntimeError> {
    let raw =
        process_option(report, owner_line, "transaction").unwrap_or_else(|| "commit".to_owned());
    match strip_runtime_string_value(&raw)
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "" | "commit" => Ok("commit".to_owned()),
        "rollback" => Ok("rollback".to_owned()),
        other => Err(invalid_input(&format!(
            "E-DB-TRANSACTION-FAILED: unsupported transaction policy `{other}`; use commit or rollback"
        ))),
    }
}

fn db_write_key(report: &CheckReport, owner_line: usize) -> Vec<String> {
    let Some(raw) = process_option(report, owner_line, "key") else {
        return Vec::new();
    };
    let trimmed = raw.trim().trim_start_matches('[').trim_end_matches(']');
    trimmed
        .split(',')
        .map(str::trim)
        .map(strip_runtime_string_value)
        .filter(|value| !value.is_empty())
        .collect()
}

fn db_write_preflight_diagnostics(
    db_path: &Path,
    table_name: &str,
    table: &runtime_data::RuntimeTable,
    mode: &str,
    key: &[String],
) -> Result<Vec<DbManifestDiagnosticRecord>, RuntimeError> {
    let mut diagnostics = Vec::new();
    let desired_columns = table
        .columns
        .iter()
        .map(|column| column.name.clone())
        .collect::<Vec<_>>();
    if mode == "upsert" && key.is_empty() {
        diagnostics.push(DbManifestDiagnosticRecord {
            code: "E-DB-KEY-MISSING".to_owned(),
            message: format!("upsert table `{table_name}` requires at least one key column"),
            table: Some(table_name.to_owned()),
        });
    }
    for key_column in key {
        if !desired_columns.iter().any(|column| column == key_column) {
            diagnostics.push(DbManifestDiagnosticRecord {
                code: "E-DB-KEY-MISSING".to_owned(),
                message: format!(
                    "upsert key `{key_column}` is not present in source table `{}`",
                    table.binding
                ),
                table: Some(table_name.to_owned()),
            });
        }
    }
    if db_path.exists() {
        let existing_columns = sqlite_table_columns(db_path, table_name)?;
        if !existing_columns.is_empty() && existing_columns != desired_columns {
            diagnostics.push(DbManifestDiagnosticRecord {
                code: "E-DB-SCHEMA-MISMATCH".to_owned(),
                message: format!(
                    "existing table `{table_name}` columns [{}] do not match source schema [{}]",
                    existing_columns.join(", "),
                    desired_columns.join(", ")
                ),
                table: Some(table_name.to_owned()),
            });
        }
    }
    Ok(diagnostics)
}

fn execute_sqlite_write_payload(
    db_path: &Path,
    table_name: &str,
    table: &runtime_data::RuntimeTable,
    mode: &str,
    key: &[String],
    transaction_policy: &str,
) -> Result<(), RuntimeError> {
    let mut connection = sqlite_open_or_create(db_path)?;
    connection.execute("BEGIN").map_err(|error| {
        invalid_input(&format!(
            "E-DB-TRANSACTION-FAILED: failed to begin SQLite transaction for `{table_name}`: {error}"
        ))
    })?;

    let mut definitions = table
        .columns
        .iter()
        .map(|column| {
            format!(
                "{} {}",
                sqlite_quote_identifier(&column.name),
                sqlite_column_type(column)
            )
        })
        .collect::<Vec<_>>();
    if !key.is_empty() {
        definitions.push(format!(
            "PRIMARY KEY ({})",
            key.iter()
                .map(|column| sqlite_quote_identifier(column))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    connection
        .execute(&format!(
            "CREATE TABLE IF NOT EXISTS {} ({})",
            sqlite_quote_identifier(table_name),
            definitions.join(", ")
        ))
        .map_err(|error| {
            invalid_input(&format!(
                "E-DB-TRANSACTION-FAILED: failed to create SQLite table `{table_name}`: {error}"
            ))
        })?;
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS \"_eng_schema_metadata\" (db_table TEXT NOT NULL, column_name TEXT NOT NULL, type_name TEXT NOT NULL, quantity_kind TEXT, unit TEXT, canonical_unit TEXT, is_index INTEGER NOT NULL, PRIMARY KEY (db_table, column_name))",
        )
        .map_err(|error| {
            invalid_input(&format!(
                "E-DB-TRANSACTION-FAILED: failed to create SQLite schema metadata table: {error}"
            ))
        })?;
    connection
        .execute(&format!(
            "DELETE FROM \"_eng_schema_metadata\" WHERE db_table = {}",
            sqlite_literal(&GraphiteSqlValue::Text(table_name.to_owned()))
        ))
        .map_err(|error| {
            invalid_input(&format!(
                "E-DB-TRANSACTION-FAILED: failed to clear SQLite schema metadata for `{table_name}`: {error}"
            ))
        })?;
    for column in &table.columns {
        let values = [
            GraphiteSqlValue::Text(table_name.to_owned()),
            GraphiteSqlValue::Text(column.name.clone()),
            GraphiteSqlValue::Text(column.type_name.clone()),
            GraphiteSqlValue::Text(column.type_name.clone()),
            sqlite_value_text(column.unit.as_ref()),
            sqlite_value_text(column.canonical_unit.as_ref()),
            GraphiteSqlValue::Integer(if column.is_index { 1 } else { 0 }),
        ];
        connection
            .execute(&format!(
                "INSERT INTO \"_eng_schema_metadata\" (db_table, column_name, type_name, quantity_kind, unit, canonical_unit, is_index) VALUES ({})",
                sqlite_values_clause(&values)
            ))
            .map_err(|error| {
                invalid_input(&format!(
                    "E-DB-TRANSACTION-FAILED: failed to write SQLite schema metadata for `{}`: {error}",
                    column.name
                ))
            })?;
    }

    let column_names = table
        .columns
        .iter()
        .map(|column| column.name.clone())
        .collect::<Vec<_>>();
    let quoted_columns = column_names
        .iter()
        .map(|column| sqlite_quote_identifier(column))
        .collect::<Vec<_>>()
        .join(", ");
    let insert_prefix = format!(
        "INSERT INTO {} ({quoted_columns}) VALUES",
        sqlite_quote_identifier(table_name)
    );
    let mut upsert_clause = String::new();
    if mode == "upsert" && !key.is_empty() {
        let assignments = column_names
            .iter()
            .filter(|column| !key.iter().any(|key_column| key_column == *column))
            .map(|column| {
                format!(
                    "{} = excluded.{}",
                    sqlite_quote_identifier(column),
                    sqlite_quote_identifier(column)
                )
            })
            .collect::<Vec<_>>();
        upsert_clause = format!(
            " ON CONFLICT ({}) DO {}",
            key.iter()
                .map(|column| sqlite_quote_identifier(column))
                .collect::<Vec<_>>()
                .join(", "),
            if assignments.is_empty() {
                "NOTHING".to_owned()
            } else {
                format!("UPDATE SET {}", assignments.join(", "))
            }
        );
    }
    for row_index in 0..table.row_count {
        let values = sqlite_row_values(table, row_index);
        connection
            .execute(&format!(
                "{insert_prefix} ({}){upsert_clause}",
                sqlite_values_clause(&values)
            ))
            .map_err(|error| {
                invalid_input(&format!(
                    "E-DB-TRANSACTION-FAILED: failed to write SQLite row {} into `{table_name}`: {error}",
                    row_index + 1
                ))
            })?;
    }

    if transaction_policy == "rollback" {
        connection.execute("ROLLBACK").map(|_| ()).map_err(|error| {
            invalid_input(&format!(
                "E-DB-TRANSACTION-FAILED: failed to roll back SQLite transaction for `{table_name}`: {error}"
            ))
        })
    } else {
        connection.execute("COMMIT").map(|_| ()).map_err(|error| {
            invalid_input(&format!(
                "E-DB-TRANSACTION-FAILED: failed to commit SQLite transaction for `{table_name}`: {error}"
            ))
        })
    }
}

fn sqlite_table_columns(db_path: &Path, table_name: &str) -> Result<Vec<String>, RuntimeError> {
    let connection = sqlite_open_existing(db_path)?;
    let sql = format!("PRAGMA table_info({})", sqlite_quote_identifier(table_name));
    let result = connection.query(&sql).map_err(|error| {
        invalid_input(&format!(
            "E-DB-SCHEMA-MISMATCH: failed to inspect SQLite table `{table_name}`: {error}"
        ))
    })?;
    let mut columns = Vec::new();
    for row in result.rows {
        if let Some(GraphiteSqlValue::Text(column_name)) = row.get(1) {
            columns.push(column_name.clone());
        }
    }
    Ok(columns)
}

type GraphiteSqlConnection = graphitesql::Connection;
type GraphiteSqlValue = graphitesql::Value;

fn sqlite_open_or_create(db_path: &Path) -> Result<GraphiteSqlConnection, RuntimeError> {
    if db_path.exists() {
        sqlite_open_existing(db_path)
    } else {
        let path = db_path.to_string_lossy();
        GraphiteSqlConnection::create(path.as_ref()).map_err(|error| {
            invalid_input(&format!(
                "E-DB-CONNECT: failed to create SQLite database `{}`: {error}",
                db_path.display()
            ))
        })
    }
}

fn sqlite_open_existing(db_path: &Path) -> Result<GraphiteSqlConnection, RuntimeError> {
    let path = db_path.to_string_lossy();
    GraphiteSqlConnection::open(path.as_ref()).map_err(|error| {
        invalid_input(&format!(
            "E-DB-CONNECT: failed to open SQLite database `{}`: {error}",
            db_path.display()
        ))
    })
}

fn sqlite_quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn sqlite_literal(value: &GraphiteSqlValue) -> String {
    match value {
        GraphiteSqlValue::Null => "NULL".to_owned(),
        GraphiteSqlValue::Integer(value) => value.to_string(),
        GraphiteSqlValue::Real(value) if value.is_finite() => value.to_string(),
        GraphiteSqlValue::Real(_) => "NULL".to_owned(),
        GraphiteSqlValue::Text(value) => format!("'{}'", value.replace('\'', "''")),
        GraphiteSqlValue::Blob(bytes) => {
            let mut literal = String::from("X'");
            for byte in bytes {
                literal.push_str(&format!("{byte:02x}"));
            }
            literal.push('\'');
            literal
        }
    }
}

fn sqlite_values_clause(values: &[GraphiteSqlValue]) -> String {
    values
        .iter()
        .map(sqlite_literal)
        .collect::<Vec<_>>()
        .join(", ")
}

fn sqlite_value_text(value: Option<&String>) -> GraphiteSqlValue {
    value
        .cloned()
        .map(GraphiteSqlValue::Text)
        .unwrap_or(GraphiteSqlValue::Null)
}

fn sqlite_value_number(value: Option<f64>) -> GraphiteSqlValue {
    value
        .map(GraphiteSqlValue::Real)
        .unwrap_or(GraphiteSqlValue::Null)
}

fn sqlite_column_type(column: &runtime_data::RuntimeColumn) -> &'static str {
    match &column.values {
        RuntimeValues::Number(_) => "REAL",
        RuntimeValues::Text(_) => "TEXT",
    }
}

fn sqlite_row_values(
    table: &runtime_data::RuntimeTable,
    row_index: usize,
) -> Vec<GraphiteSqlValue> {
    table
        .columns
        .iter()
        .map(|column| match &column.values {
            RuntimeValues::Text(values) => sqlite_value_text(values.get(row_index)),
            RuntimeValues::Number(values) => {
                sqlite_value_number(values.get(row_index).and_then(|value| *value))
            }
        })
        .collect()
}
fn hash_file_if_exists(path: &Path) -> Option<String> {
    fs::read(path).ok().map(|bytes| hash_bytes(&bytes))
}

fn sha256_file_if_exists(path: &Path) -> Option<String> {
    fs::read(path).ok().map(|bytes| sha256_bytes(&bytes))
}

fn sha256_bytes(source: &[u8]) -> String {
    format!("{:x}", Sha256::digest(source))
}

fn network_cache_owner(owner_kind: &str) -> bool {
    matches!(owner_kind, "network_request" | "network_download")
}

fn native_db_write_manifest_json(record: &DbManifestRecord) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"format\": \"db-write-manifest-v1\",\n");
    json.push_str(&format!(
        "  \"database\": \"{}\",\n",
        json_escape(record.database.as_deref().unwrap_or_default())
    ));
    push_optional_json_string(
        &mut json,
        "database_hash_before",
        record.database_hash_before.as_deref(),
        2,
    );
    push_optional_json_string(
        &mut json,
        "database_hash_after",
        record.database_hash_after.as_deref(),
        2,
    );
    push_optional_json_string(
        &mut json,
        "transaction_status",
        record.transaction_status.as_deref(),
        2,
    );
    push_optional_json_string(
        &mut json,
        "schema_status",
        record.schema_status.as_deref(),
        2,
    );
    json.push_str("  \"diagnostics\": [\n");
    push_db_manifest_diagnostics_json(&mut json, &record.diagnostics, "    ");
    json.push_str("\n  ],\n");
    json.push_str("  \"tables\": [\n");
    for (index, table) in record.tables.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&table.name)
        ));
        json.push_str(&format!(
            "      \"mode\": \"{}\",\n",
            json_escape(&table.mode)
        ));
        json.push_str("      \"key\": [");
        push_json_string_array(&mut json, &table.key);
        json.push_str("],\n");
        json.push_str("      \"schema\": [");
        push_json_string_array(&mut json, &table.schema);
        json.push_str("],\n");
        json.push_str(&format!(
            "      \"row_count\": {}\n",
            table.row_count.unwrap_or(0)
        ));
        json.push_str("    }");
    }
    json.push_str("\n  ]\n");
    json.push_str("}\n");
    json
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
                let overwrite_policy = overwrite_policy(report, operation.line);
                write_output_file(&destination_path, &contents, overwrite_policy.allowed)?;
                artifacts.push(output_artifact_with_overwrite_policy(
                    "copy_file",
                    relative_output_path(result_dir, &destination_path),
                    &contents,
                    destination_path,
                    overwrite_policy.label,
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
                let overwrite_policy = overwrite_policy(report, operation.line);
                write_output_file(&destination_path, &contents, overwrite_policy.allowed)?;
                if source_path != destination_path {
                    fs::remove_file(&source_path)?;
                }
                artifacts.push(output_artifact_with_overwrite_policy(
                    "move_file",
                    relative_output_path(result_dir, &destination_path),
                    &contents,
                    destination_path,
                    overwrite_policy.label,
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
                        overwrite_policy: None,
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
                        overwrite_policy: None,
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

struct OverwritePolicy {
    allowed: bool,
    label: &'static str,
}

fn overwrite_policy(report: &CheckReport, owner_line: usize) -> OverwritePolicy {
    if overwrite_allowed(report, owner_line) {
        OverwritePolicy {
            allowed: true,
            label: "allowed",
        }
    } else {
        OverwritePolicy {
            allowed: false,
            label: "not_allowed",
        }
    }
}

fn render_write_contents(
    write: &eng_compiler::WriteInfo,
    report: &CheckReport,
    runtime_data: &RuntimeData,
) -> Option<String> {
    if write.format == "text" && write.expression.trim_start().starts_with('"') {
        let template = strip_runtime_string_value(&write.expression);
        return Some(render_runtime_text_template(
            &template,
            report,
            runtime_data,
        ));
    }

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
    working_dir: &Path,
    output_dir: &Path,
    artifacts: &[OutputArtifact],
    profile: &ExecutionProfile,
    profile_diagnostics: &[ProfileDiagnostic],
    registry: &ArtifactRegistryContext<'_>,
) -> String {
    let artifact_records = artifact_records_for_outputs(artifacts);
    let mut artifact_registry_json = String::new();
    push_artifact_registry_json(&mut artifact_registry_json, &artifact_records, registry);
    let mut profile_diagnostics_json = String::new();
    push_profile_diagnostics_json(&mut profile_diagnostics_json, profile_diagnostics, "    ");
    OutputManifest {
        runtime_version: RUNTIME_VERSION,
        source_path,
        working_dir,
        output_dir,
        execution_profile: profile.as_str(),
        artifacts: &artifact_records,
        artifact_registry_json,
        profile_diagnostics_json,
    }
    .to_json()
}

#[derive(Clone, Debug)]
struct RunLockInput {
    input_hash: String,
    args_hash: String,
    dependency_hash: String,
    dependencies: Vec<Value>,
}

#[derive(Clone, Debug)]
struct RerunDecision {
    decision: String,
    reason: String,
    prior_input_hash: Option<String>,
}

struct RunLockArtifactHashes<'a> {
    result: &'a str,
    review: &'a str,
    static_run_plan: &'a str,
    run_plan: &'a str,
}

fn run_lock_input(source_path: &Path, report: &CheckReport, options: &RunOptions) -> RunLockInput {
    let args = run_lock_args(&options.args);
    let dependencies = run_lock_dependencies(report);
    let args_json = serde_json::to_string(&args).unwrap_or_else(|_| "[]".to_owned());
    let dependencies_json =
        serde_json::to_string(&dependencies).unwrap_or_else(|_| "[]".to_owned());
    let args_hash = hash_text(&args_json);
    let dependency_hash = hash_text(&dependencies_json);
    let input = json!({
        "source_path": path_for_manifest(source_path),
        "source_hash": &report.source_hash,
        "execution_profile": options.profile.as_str(),
        "args_hash": &args_hash,
        "dependency_hash": &dependency_hash
    });
    let input_json = serde_json::to_string(&input).unwrap_or_else(|_| "{}".to_owned());
    RunLockInput {
        input_hash: hash_text(&input_json),
        args_hash,
        dependency_hash,
        dependencies,
    }
}

fn run_lock_args(args: &[ArgOverride]) -> Vec<Value> {
    let mut values = args
        .iter()
        .map(|arg| json!({ "name": arg.name, "value": arg.value }))
        .collect::<Vec<_>>();
    values.sort_by(|left, right| {
        let left_name = left.get("name").and_then(Value::as_str).unwrap_or_default();
        let right_name = right
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        left_name.cmp(right_name)
    });
    values
}

fn run_lock_dependencies(report: &CheckReport) -> Vec<Value> {
    let mut dependencies = Vec::new();
    for promotion in &report.semantic_program.csv_promotions {
        dependencies.push(json!({
            "kind": "csv",
            "binding": &promotion.binding,
            "path": &promotion.resolved_path,
            "hash": &promotion.source_hash
        }));
    }
    for promotion in &report.semantic_program.config_promotions {
        dependencies.push(json!({
            "kind": format!("config_{}", promotion.format),
            "binding": &promotion.binding,
            "path": &promotion.resolved_path,
            "hash": &promotion.source_hash
        }));
    }
    for dependency in &report.semantic_program.environment_dependencies {
        if let Some(hash) = &dependency.source_hash {
            dependencies.push(json!({
                "kind": &dependency.kind,
                "binding": &dependency.name,
                "path": &dependency.resolved_value,
                "hash": hash
            }));
        }
    }
    dependencies
}

fn rerun_decision_for_run(
    run_lock_path: &Path,
    input: &RunLockInput,
    skip_unchanged: bool,
    artifacts_saved: bool,
) -> RerunDecision {
    let prior = fs::read_to_string(run_lock_path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    let prior_input_hash = prior
        .as_ref()
        .and_then(|value| value.get("input_hash"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let Some(prior_hash) = prior_input_hash.as_deref() else {
        return RerunDecision {
            decision: "run".to_owned(),
            reason: "no_prior_run_lock".to_owned(),
            prior_input_hash,
        };
    };
    if prior_hash != input.input_hash {
        return RerunDecision {
            decision: "run".to_owned(),
            reason: "run_lock_changed".to_owned(),
            prior_input_hash,
        };
    }
    if !skip_unchanged {
        return RerunDecision {
            decision: "run".to_owned(),
            reason: "unchanged_run_lock_skip_disabled".to_owned(),
            prior_input_hash,
        };
    }
    if !artifacts_saved {
        return RerunDecision {
            decision: "run".to_owned(),
            reason: "unchanged_run_lock_requires_saved_artifacts".to_owned(),
            prior_input_hash,
        };
    }
    RerunDecision {
        decision: "skip".to_owned(),
        reason: "unchanged_run_lock".to_owned(),
        prior_input_hash,
    }
}

fn run_lock_json(
    source_path: &Path,
    report: &CheckReport,
    input: &RunLockInput,
    decision: &RerunDecision,
    artifact_hashes: &RunLockArtifactHashes<'_>,
    profile: &ExecutionProfile,
) -> String {
    let document = json!({
        "format": "eng-run-lock-v1",
        "runtime_version": RUNTIME_VERSION,
        "source_path": path_for_manifest(source_path),
        "source_hash": &report.source_hash,
        "execution_profile": profile.as_str(),
        "input_hash": &input.input_hash,
        "args_hash": &input.args_hash,
        "dependency_hash": &input.dependency_hash,
        "dependencies": &input.dependencies,
        "rerun_decision": rerun_decision_json(decision),
        "artifact_hashes": {
            "result": artifact_hashes.result,
            "review": artifact_hashes.review,
            "static_run_plan": artifact_hashes.static_run_plan,
            "run_plan": artifact_hashes.run_plan
        }
    });
    format!(
        "{}\n",
        serde_json::to_string_pretty(&document).expect("serialize run lock")
    )
}

fn saved_run_artifacts_available(paths: &[&Path]) -> bool {
    paths.iter().all(|path| path.is_file())
}

fn saved_run_artifact_hash_mismatch(
    run_lock_path: &Path,
    artifacts: &[(&str, &Path)],
) -> Option<String> {
    let Some(prior_lock) = fs::read_to_string(run_lock_path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
    else {
        return Some("run_lock".to_owned());
    };
    let Some(prior_hashes) = prior_lock.get("artifact_hashes") else {
        return Some("artifact_hashes".to_owned());
    };
    for (kind, path) in artifacts {
        let Some(prior_hash) = prior_hashes.get(*kind).and_then(Value::as_str) else {
            return Some((*kind).to_owned());
        };
        let Ok(contents) = fs::read_to_string(path) else {
            return Some((*kind).to_owned());
        };
        if hash_text(&contents) != prior_hash {
            return Some((*kind).to_owned());
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn skipped_saved_run_output(
    source_path: &Path,
    report: &CheckReport,
    input: &RunLockInput,
    decision: &RerunDecision,
    static_run_plan_json: &str,
    profile: &ExecutionProfile,
    bytecode_path: PathBuf,
    result_path: PathBuf,
    review_path: PathBuf,
    report_path: PathBuf,
    report_spec_path: PathBuf,
    plot_path: PathBuf,
    plot_spec_path: PathBuf,
    plot_manifest_path: PathBuf,
    output_manifest_path: PathBuf,
    static_run_plan_path: PathBuf,
    run_plan_path: PathBuf,
    run_lock_path: PathBuf,
    run_log_path: PathBuf,
    process_results_path: PathBuf,
    cache_manifest_path: PathBuf,
    test_results_path: PathBuf,
) -> Result<RunOutput, RuntimeError> {
    let bytecode = fs::read_to_string(&bytecode_path)?;
    let result_json = fs::read_to_string(&result_path)?;
    let previous_review_json = fs::read_to_string(&review_path)?;
    let report_html = fs::read_to_string(&report_path)?;
    let report_spec_json = fs::read_to_string(&report_spec_path)?;
    let plot_svg = fs::read_to_string(&plot_path)?;
    let plot_spec_json = fs::read_to_string(&plot_spec_path)?;
    let plot_manifest_json = fs::read_to_string(&plot_manifest_path)?;
    let run_log_json = fs::read_to_string(&run_log_path)?;
    let process_results_json = fs::read_to_string(&process_results_path)?;
    let cache_manifest_json = fs::read_to_string(&cache_manifest_path)?;
    let test_results_json = fs::read_to_string(&test_results_path)?;
    fs::write(&static_run_plan_path, static_run_plan_json)?;
    let previous_run_plan_json = fs::read_to_string(&run_plan_path)?;
    let run_plan_json = mark_run_plan_rerun_decision(&previous_run_plan_json, decision);
    fs::write(&run_plan_path, &run_plan_json)?;
    let review_json = mark_review_workflow_rerun_decision(&previous_review_json, decision);
    fs::write(&review_path, &review_json)?;

    let result_artifact_hash = hash_text(&result_json);
    let review_artifact_hash = hash_text(&review_json);
    let static_run_plan_artifact_hash = hash_text(static_run_plan_json);
    let run_plan_artifact_hash = hash_text(&run_plan_json);
    let run_lock_json = run_lock_json(
        source_path,
        report,
        input,
        decision,
        &RunLockArtifactHashes {
            result: &result_artifact_hash,
            review: &review_artifact_hash,
            static_run_plan: &static_run_plan_artifact_hash,
            run_plan: &run_plan_artifact_hash,
        },
        profile,
    );
    fs::write(&run_lock_path, &run_lock_json)?;

    let previous_output_manifest_json = fs::read_to_string(&output_manifest_path)?;
    let output_manifest_json = update_output_manifest_artifact_hashes(
        &previous_output_manifest_json,
        &[
            ("review", "review.json", hash_text(&review_json)),
            (
                "static_run_plan",
                "static_run_plan.json",
                hash_text(static_run_plan_json),
            ),
            ("run_plan", "run_plan.json", hash_text(&run_plan_json)),
            ("run_lock", "run_lock.json", hash_text(&run_lock_json)),
        ],
    );
    fs::write(&output_manifest_path, &output_manifest_json)?;

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
        static_run_plan_path,
        run_plan_path,
        run_lock_path,
        run_log_path,
        process_results_path,
        cache_manifest_path,
        test_results_path,
        csv_export_paths: Vec::new(),
        write_output_paths: Vec::new(),
        file_operation_paths: Vec::new(),
        artifacts_saved: true,
        stdout: "run skipped: unchanged run_lock\n".to_owned(),
        bytecode,
        result_json,
        review_json,
        report_html,
        report_spec_json,
        plot_svg,
        plot_spec_json,
        plot_manifest_json,
        output_manifest_json,
        static_run_plan_json: static_run_plan_json.to_owned(),
        run_plan_json,
        run_lock_json,
        run_log_json,
        process_results_json,
        cache_manifest_json,
        test_results_json,
    })
}

fn mark_run_plan_rerun_decision(run_plan_json: &str, decision: &RerunDecision) -> String {
    let Ok(mut run_plan) = serde_json::from_str::<Value>(run_plan_json) else {
        return run_plan_json.to_owned();
    };
    let rerun_decision = rerun_decision_json(decision);
    if let Some(object) = run_plan.as_object_mut() {
        object.insert("rerun_status".to_owned(), json!(rerun_status(decision)));
        object.insert("rerun_decision".to_owned(), rerun_decision.clone());
    }
    if let Some(nodes) = run_plan
        .get_mut("graph")
        .and_then(|graph| graph.get_mut("nodes"))
        .and_then(Value::as_array_mut)
    {
        for node in nodes {
            if let Some(object) = node.as_object_mut() {
                object.insert("rerun_status".to_owned(), json!(rerun_status(decision)));
                object.insert("rerun_decision".to_owned(), rerun_decision.clone());
            }
        }
    }
    serde_json::to_string_pretty(&run_plan)
        .map(|mut text| {
            text.push('\n');
            text
        })
        .unwrap_or_else(|_| run_plan_json.to_owned())
}

fn mark_review_workflow_rerun_decision(review_json: &str, decision: &RerunDecision) -> String {
    let Ok(mut review) = serde_json::from_str::<Value>(review_json) else {
        return review_json.to_owned();
    };
    let rerun_decision = rerun_decision_json(decision);
    if let Some(nodes) = review
        .get_mut("workflow_graph")
        .and_then(|graph| graph.get_mut("nodes"))
        .and_then(Value::as_array_mut)
    {
        for node in nodes {
            if let Some(object) = node.as_object_mut() {
                object.insert("rerun_status".to_owned(), json!(rerun_status(decision)));
                object.insert("rerun_decision".to_owned(), rerun_decision.clone());
            }
        }
    }
    serde_json::to_string_pretty(&review)
        .map(|mut text| {
            text.push('\n');
            text
        })
        .unwrap_or_else(|_| review_json.to_owned())
}

fn update_output_manifest_artifact_hashes(
    output_manifest_json: &str,
    hashes: &[(&str, &str, String)],
) -> String {
    let Ok(mut output_manifest) = serde_json::from_str::<Value>(output_manifest_json) else {
        return output_manifest_json.to_owned();
    };
    update_artifact_array_hashes(
        output_manifest
            .get_mut("artifacts")
            .and_then(Value::as_array_mut),
        hashes,
    );
    update_artifact_array_hashes(
        output_manifest
            .get_mut("artifact_registry")
            .and_then(|registry| registry.get_mut("generated_files"))
            .and_then(Value::as_array_mut),
        hashes,
    );
    serde_json::to_string_pretty(&output_manifest)
        .map(|mut text| {
            text.push('\n');
            text
        })
        .unwrap_or_else(|_| output_manifest_json.to_owned())
}

fn update_artifact_array_hashes(
    artifacts: Option<&mut Vec<Value>>,
    hashes: &[(&str, &str, String)],
) {
    let Some(artifacts) = artifacts else {
        return;
    };
    for artifact in artifacts {
        let Some(object) = artifact.as_object_mut() else {
            continue;
        };
        let kind = object
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let path = object
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if let Some((_, _, hash)) = hashes.iter().find(|(expected_kind, expected_path, _)| {
            kind == *expected_kind || path == *expected_path
        }) {
            object.insert("hash".to_owned(), Value::String(hash.clone()));
        }
    }
}

fn rerun_decision_json(decision: &RerunDecision) -> Value {
    json!({
        "decision": decision.decision,
        "reason": decision.reason,
        "prior_input_hash": decision.prior_input_hash
    })
}

fn static_run_plan_json(
    source_path: &Path,
    report: &CheckReport,
    profile: &ExecutionProfile,
    rerun_decision: &RerunDecision,
) -> String {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    nodes.push(run_plan_node(
        "source:program",
        "source_file",
        "program",
        "loaded",
        "static",
        "low",
        1,
        vec![json!({"kind": "source_hash", "hash": &report.source_hash})],
        rerun_decision,
    ));

    for schema in &report.semantic_program.schemas {
        let id = format!("schema:{}", schema.name);
        nodes.push(run_plan_node(
            &id,
            "schema",
            &schema.name,
            "planned",
            "static",
            "low",
            schema.line,
            vec![json!({
                "column_count": schema.columns.len(),
                "constraint_count": schema.constraints.len(),
                "missing_policy_count": schema.missing_policies.len()
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for promotion in &report.semantic_program.csv_promotions {
        let id = format!("source:csv:{}", promotion.binding);
        nodes.push(run_plan_node(
            &id,
            "csv_promotion",
            &promotion.binding,
            "planned",
            "static",
            "medium",
            promotion.line,
            vec![json!({
                "schema": &promotion.schema_name,
                "path": &promotion.resolved_path,
                "hash": &promotion.source_hash,
                "row_count": promotion.row_count
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for promotion in &report.semantic_program.config_promotions {
        let id = format!("source:config:{}", promotion.binding);
        nodes.push(run_plan_node(
            &id,
            "config_promotion",
            &promotion.binding,
            "planned",
            "static",
            "medium",
            promotion.line,
            vec![json!({
                "format": &promotion.format,
                "path": &promotion.resolved_path,
                "hash": &promotion.source_hash,
                "field_count": promotion.field_count
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for transform in &report.semantic_program.table_transforms {
        let id = format!("table_transform:{}", transform.binding);
        nodes.push(run_plan_node(
            &id,
            "table_transform",
            &transform.binding,
            "planned",
            "static",
            "low",
            transform.line,
            vec![json!({
                "operation": &transform.operation,
                "source_table": &transform.source_table,
                "secondary_table": &transform.secondary_table,
                "predicate_count": transform.predicates.len(),
                "selected_column_count": transform.selected_columns.len()
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for kernel in &report.semantic_program.timeseries_kernels {
        let id = format!("timeseries_kernel:{}", kernel.binding);
        nodes.push(run_plan_node(
            &id,
            "timeseries_kernel",
            &kernel.binding,
            "planned",
            "static",
            "medium",
            kernel.line,
            vec![json!({
                "kind": &kernel.kind,
                "source_table": &kernel.source_table,
                "axis": &kernel.axis,
                "quantity_kind": &kernel.quantity_kind,
                "operations": &kernel.operations
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for request in &report.semantic_program.net_requests {
        let id = format!("network_request:{}", request.binding);
        nodes.push(run_plan_node(
            &id,
            "network_request",
            &request.binding,
            "planned",
            "static",
            "high",
            request.line,
            vec![json!({
                "method": &request.method,
                "url": &request.url_value,
                "query_count": request.query.len(),
                "retry": request.retry,
                "cache": request.cache,
                "timeout": &request.timeout,
                "body_size_limit_bytes": request.body_size_limit_bytes,
                "fixture": &request.fixture
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for download in &report.semantic_program.net_downloads {
        let id = format!("network_download:{}", download.target_value);
        nodes.push(run_plan_node(
            &id,
            "network_download",
            &download.target_value,
            "planned",
            "static",
            "high",
            download.line,
            vec![json!({
                "url": &download.url_value,
                "target": &download.target_value,
                "query_count": download.query.len(),
                "retry": download.retry,
                "cache": download.cache,
                "timeout": &download.timeout,
                "body_size_limit_bytes": download.body_size_limit_bytes,
                "fixture": &download.fixture
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for cache in &report.semantic_program.cache_records {
        let id = format!("cache:{}:{}", cache.owner_kind, cache.owner_name);
        nodes.push(run_plan_node(
            &id,
            "cache",
            &cache.owner_name,
            "planned",
            "static",
            "medium",
            cache.line,
            vec![json!({
                "owner_kind": &cache.owner_kind,
                "cache_key_hash": &cache.cache_key_hash,
                "cache_path": &cache.cache_path,
                "cache_dir": &cache.cache_dir,
                "source_hash": &cache.source_hash
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for dependency in &report.semantic_program.environment_dependencies {
        let id = format!("dependency:{}:{}", dependency.kind, dependency.name);
        nodes.push(run_plan_node(
            &id,
            "environment_dependency",
            &dependency.name,
            "planned",
            "static",
            "medium",
            dependency.line,
            vec![json!({
                "kind": &dependency.kind,
                "expression": &dependency.expression,
                "resolved_value": &dependency.resolved_value,
                "source_hash": &dependency.source_hash
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "depends_on"));
    }
    for export in &report.semantic_program.csv_exports {
        let id = format!("csv_export:{}:{}", export.source, export.line);
        nodes.push(run_plan_node(
            &id,
            "csv_export",
            &export.path,
            "planned",
            "static",
            "low",
            export.line,
            vec![json!({
                "source": &export.source,
                "format": &export.format,
                "path": &export.path,
                "field_count": export.fields.len()
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "emits"));
    }
    for write in &report.semantic_program.writes {
        let id = format!("write:{}:{}", write.format, write.line);
        nodes.push(run_plan_node(
            &id,
            "write_output",
            &write.path,
            "planned",
            "static",
            "low",
            write.line,
            vec![json!({
                "format": &write.format,
                "path": &write.path,
                "quantity_kind": &write.quantity_kind,
                "display_unit": &write.display_unit
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "emits"));
    }
    for command in report
        .semantic_program
        .command_styles
        .iter()
        .filter(|command| {
            command.verb == "render" && command.target.trim().starts_with("template ")
        })
    {
        let id = template_render_node_id(command);
        let source = command
            .target
            .trim()
            .strip_prefix("template ")
            .map(str::trim)
            .unwrap_or_default();
        let output = template_command_output_expression(report, command);
        nodes.push(run_plan_node(
            &id,
            "template_render",
            command.owner.as_deref().unwrap_or("render template"),
            "planned",
            "static",
            "low",
            command.line,
            vec![json!({
                "source": source,
                "output": output,
                "placeholder_syntax": "{{name}}"
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "emits"));
    }
    for operation in &report.semantic_program.file_operations {
        let id = format!("file_operation:{}:{}", operation.operation, operation.line);
        nodes.push(run_plan_node(
            &id,
            "file_operation",
            &operation.operation,
            "planned",
            "static",
            "medium",
            operation.line,
            vec![json!({
                "operation": &operation.operation,
                "source": &operation.source,
                "destination": &operation.destination
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "emits"));
    }
    for process in &report.semantic_program.process_runs {
        let id = format!("process:{}", process.binding);
        nodes.push(run_plan_node(
            &id,
            "process",
            &process.binding,
            "planned",
            "static",
            "high",
            process.line,
            vec![json!({
                "command": &process.command
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for ml in &report.semantic_program.ml_infos {
        let id = format!("model:{}", ml.binding);
        nodes.push(run_plan_node(
            &id,
            "model",
            &ml.binding,
            "planned",
            "static",
            "medium",
            ml.line,
            vec![json!({
                "kind": &ml.kind,
                "source": &ml.source,
                "target": &ml.target,
                "features": &ml.features,
                "algorithm": &ml.algorithm,
                "seed": &ml.seed
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for test in &report.semantic_program.tests {
        let id = format!("test:{}", test.name);
        nodes.push(run_plan_node(
            &id,
            "test",
            &test.name,
            "planned",
            "static",
            "low",
            test.line,
            vec![json!({
                "assertion_count": test.assertions.len(),
                "golden_count": test.goldens.len()
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }

    let node_ids = run_plan_node_ids(&nodes);
    add_static_dependency_edges(report, &node_ids, &mut edges);

    let node_count = nodes.len();
    let edge_count = edges.len();
    let document = json!({
        "format": "eng-static-run-plan-v1",
        "runtime_version": RUNTIME_VERSION,
        "source_path": path_for_manifest(source_path),
        "source_hash": &report.source_hash,
        "execution_profile": profile.as_str(),
        "execution_stage": "pre_execution",
        "status": "planned",
        "rerun_status": rerun_status(rerun_decision),
        "rerun_decision": rerun_decision_json(rerun_decision),
        "graph": {
            "node_count": node_count,
            "edge_count": edge_count,
            "nodes": nodes,
            "edges": edges
        }
    });
    format!(
        "{}\n",
        serde_json::to_string_pretty(&document).expect("serialize static run plan")
    )
}

fn run_plan_json(
    source_path: &Path,
    report: &CheckReport,
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
    external_boundary_records: &[ExternalBoundaryRecord],
    cache_records: &[CacheManifestRecord],
    db_records: &[DbManifestRecord],
    output_artifacts: &[OutputArtifact],
    static_run_plan_json: &str,
    result_json: &str,
    review_json: &str,
    profile: &ExecutionProfile,
    rerun_decision: &RerunDecision,
) -> String {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    nodes.push(run_plan_node(
        "source:program",
        "source_file",
        "program",
        "loaded",
        "static",
        "low",
        1,
        vec![json!({"kind": "source_hash", "hash": &report.source_hash})],
        rerun_decision,
    ));

    for promotion in &report.semantic_program.csv_promotions {
        let id = format!("source:csv:{}", promotion.binding);
        nodes.push(run_plan_node(
            &id,
            "csv_promotion",
            &promotion.binding,
            "promoted",
            "static",
            "medium",
            promotion.line,
            vec![json!({
                "kind": "source_file",
                "path": &promotion.resolved_path,
                "hash": &promotion.source_hash,
                "row_count": promotion.row_count
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for promotion in &report.semantic_program.config_promotions {
        let id = format!("source:config:{}", promotion.binding);
        nodes.push(run_plan_node(
            &id,
            "config_promotion",
            &promotion.binding,
            &promotion.status,
            "static",
            "medium",
            promotion.line,
            vec![json!({
                "kind": format!("config_{}", promotion.format),
                "path": &promotion.resolved_path,
                "hash": &promotion.source_hash,
                "field_count": promotion.field_count
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for selection in &runtime_data.table_selections {
        let id = format!("table_selection:{}", selection.binding);
        nodes.push(run_plan_node(
            &id,
            "table_selection",
            &selection.binding,
            &selection.status,
            "runtime",
            "low",
            selection.line,
            vec![json!({
                "source_table": &selection.source_table,
                "matched_row_count": selection.matched_count
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for transform in &runtime_data.table_transforms {
        let id = format!("table_transform:{}", transform.binding);
        nodes.push(run_plan_node(
            &id,
            "table_transform",
            &transform.binding,
            &transform.status,
            "runtime",
            "low",
            transform.line,
            vec![json!({
                "operation": &transform.operation,
                "source_table": &transform.source_table,
                "output_row_count": transform.output_row_count
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for coverage in &runtime_data.timeseries_coverage {
        let id = format!("timeseries_coverage:{}", coverage.binding);
        nodes.push(run_plan_node(
            &id,
            "timeseries_coverage",
            &coverage.binding,
            &coverage.status,
            "runtime",
            if coverage.status == "complete" {
                "low"
            } else {
                "medium"
            },
            coverage.line,
            vec![json!({
                "source_table": &coverage.source_table,
                "source_column": &coverage.source_column,
                "expected_count": coverage.expected_count,
                "actual_count": coverage.actual_count,
                "missing_count": coverage.missing_count
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for fill in &runtime_data.timeseries_fill {
        let id = format!("timeseries_fill:{}", fill.binding);
        nodes.push(run_plan_node(
            &id,
            "timeseries_fill",
            &fill.binding,
            &fill.status,
            "runtime",
            if fill.fallback_required {
                "medium"
            } else {
                "low"
            },
            fill.line,
            vec![json!({
                "source_table": &fill.source_table,
                "source_column": &fill.source_column,
                "time_column": &fill.time_column,
                "method": &fill.method,
                "missing_count": fill.missing_count,
                "fillable_count": fill.fillable_count,
                "filled_count": fill.filled_count
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for quality in &runtime_data.timeseries_quality {
        let id = format!("timeseries_quality:{}", quality.binding);
        nodes.push(run_plan_node(
            &id,
            "timeseries_quality",
            &quality.binding,
            &quality.status,
            "runtime",
            if quality.status == "passed" {
                "low"
            } else {
                "medium"
            },
            quality.line,
            vec![json!({
                "source_table": &quality.source_table,
                "source_column": &quality.source_column,
                "time_column": &quality.time_column,
                "coverage_binding": &quality.coverage_binding,
                "fill_binding": &quality.fill_binding,
                "quality_score": quality.quality_score,
                "remaining_missing_count": quality.remaining_missing_count
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for suite in &runtime_data.expectation_suites {
        let id = format!("expectation_suite:{}", suite.binding);
        nodes.push(run_plan_node(
            &id,
            "expectation_suite",
            &suite.binding,
            &suite.status,
            "runtime",
            if suite.status == "passed" {
                "low"
            } else {
                "medium"
            },
            suite.line,
            vec![json!({
                "target": &suite.target,
                "expectation_count": suite.expectation_count,
                "passed_count": suite.passed_count,
                "warning_count": suite.warning_count,
                "failed_count": suite.failed_count
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for quality in &runtime_data.quality_results {
        let id = format!("quality_result:{}", quality.binding);
        nodes.push(run_plan_node(
            &id,
            "quality_result",
            &quality.binding,
            &quality.status,
            "runtime",
            if quality.status == "passed" {
                "low"
            } else {
                "medium"
            },
            quality.line,
            vec![json!({
                "kind": &quality.kind,
                "category": &quality.category,
                "target": &quality.target,
                "subject": &quality.subject,
                "score": quality.score,
                "passed_count": quality.passed_count,
                "warning_count": quality.warning_count,
                "failed_count": quality.failed_count
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for boundary in external_boundary_records {
        let id = format!("boundary:{}:{}", boundary.kind, boundary.binding);
        nodes.push(run_plan_node(
            &id,
            &boundary.kind,
            &boundary.binding,
            &boundary.status,
            "runtime",
            "high",
            boundary.line,
            vec![json!({
                "target": &boundary.target,
                "env": &boundary.env_keys,
                "timeout": &boundary.timeout,
                "retry": boundary.retry,
                "attempt_count": boundary.attempt_count,
                "allow_failure": boundary.allow_failure,
                "timed_out": boundary.timed_out,
                "response_hash": &boundary.response_hash,
                "stdout_hash": &boundary.stdout_hash,
                "stderr_hash": &boundary.stderr_hash,
                "output_paths": &boundary.output_paths
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for process in process_results {
        let id = format!("process:{}", process.binding);
        nodes.push(run_plan_node(
            &id,
            "process",
            &process.binding,
            &process.status,
            "runtime",
            "high",
            process.line,
            vec![json!({
                "command": &process.command,
                "env": &process.env_keys,
                "timeout": &process.timeout,
                "retry": process.retry,
                "attempt_count": process.attempt_count,
                "allow_failure": process.allow_failure,
                "timed_out": process.timed_out,
                "exit_code": process.exit_code,
                "stdout_hash": &process.stdout_hash,
                "stderr_hash": &process.stderr_hash,
                "expected_output_status": &process.expected_output_status
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for cache in cache_records {
        let id = format!("cache:{}:{}", cache.owner_kind, cache.owner_name);
        nodes.push(run_plan_node(
            &id,
            "cache",
            &cache.owner_name,
            &cache.lookup_status,
            "runtime",
            "medium",
            cache.line,
            vec![json!({
                "owner_kind": &cache.owner_kind,
                "cache_key_hash": &cache.cache_key_hash,
                "cache_path": &cache.cache_path,
                "cache_dir": &cache.cache_dir,
                "source_hash": &cache.source_hash,
                "observed_hash": &cache.observed_hash
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for case_manifest in &runtime_data.case_manifests {
        let id = format!("case:{}", case_manifest.case_id);
        nodes.push(run_plan_node(
            &id,
            "case",
            &case_manifest.case_id,
            &case_manifest.status,
            "runtime",
            "medium",
            0,
            vec![json!({
                "sample_table": &case_manifest.sample_table,
                "case_dir": &case_manifest.case_dir,
                "sample_row_hash": &case_manifest.sample_row_hash,
                "process_count": case_manifest.process_statuses.len()
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for db in db_records {
        let id = format!("db:{}", db.binding);
        nodes.push(run_plan_node(
            &id,
            "db_write",
            &db.binding,
            &db.status,
            "runtime",
            "high",
            db.line,
            vec![json!({
                "manifest_path": &db.manifest_path,
                "hash": &db.hash,
                "transaction_status": &db.transaction_status,
                "table_count": db.tables.len()
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for artifact in &runtime_data.ml_artifacts {
        if artifact.model_artifact_hash.is_none() && artifact.model_card.is_none() {
            continue;
        }
        let id = format!("model:{}", artifact.binding);
        nodes.push(run_plan_node(
            &id,
            "model",
            &artifact.binding,
            &artifact.status,
            "runtime",
            "medium",
            artifact.line,
            vec![json!({
                "kind": &artifact.kind,
                "model_artifact_hash": &artifact.model_artifact_hash,
                "training_data_hash": &artifact.training_data_hash,
                "model_card_hash": artifact.model_card.as_ref().map(|card| hash_text(card))
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for test in &report.semantic_program.tests {
        let id = format!("test:{}", test.name);
        nodes.push(run_plan_node(
            &id,
            "test",
            &test.name,
            "declared",
            "static",
            "low",
            test.line,
            vec![json!({
                "assertion_count": test.assertions.len(),
                "golden_count": test.goldens.len()
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "declares"));
    }
    for command in report
        .semantic_program
        .command_styles
        .iter()
        .filter(|command| {
            command.verb == "render" && command.target.trim().starts_with("template ")
        })
    {
        let id = template_render_node_id(command);
        let source = command
            .target
            .trim()
            .strip_prefix("template ")
            .map(str::trim)
            .unwrap_or_default();
        let output = template_command_output_expression(report, command);
        nodes.push(run_plan_node(
            &id,
            "template_render",
            command.owner.as_deref().unwrap_or("render template"),
            "rendered",
            "runtime",
            "low",
            command.line,
            vec![json!({
                "source": source,
                "output": output,
                "placeholder_syntax": "{{name}}"
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "emits"));
    }
    for artifact in output_artifacts {
        let id = format!("artifact:{}", artifact.path);
        nodes.push(run_plan_node(
            &id,
            "artifact",
            &artifact.path,
            "generated",
            "runtime",
            "low",
            0,
            vec![json!({
                "kind": &artifact.kind,
                "path": &artifact.path,
                "hash": &artifact.hash,
                "validation_status": &artifact.validation.status
            })],
            rerun_decision,
        ));
        edges.push(run_plan_edge("source:program", &id, "emits"));
    }

    let node_ids = run_plan_node_ids(&nodes);
    add_runtime_dependency_edges(
        report,
        runtime_data,
        process_results,
        cache_records,
        db_records,
        output_artifacts,
        &node_ids,
        &mut edges,
    );

    let node_count = nodes.len();
    let edge_count = edges.len();
    let document = json!({
        "format": "eng-run-plan-v1",
        "runtime_version": RUNTIME_VERSION,
        "source_path": path_for_manifest(source_path),
        "source_hash": &report.source_hash,
        "execution_profile": profile.as_str(),
        "status": "completed",
        "rerun_status": rerun_status(rerun_decision),
        "rerun_decision": rerun_decision_json(rerun_decision),
        "artifact_hashes": {
            "static_run_plan": hash_text(static_run_plan_json),
            "result": hash_text(result_json),
            "review": hash_text(review_json)
        },
        "graph": {
            "node_count": node_count,
            "edge_count": edge_count,
            "nodes": nodes,
            "edges": edges
        }
    });
    format!(
        "{}\n",
        serde_json::to_string_pretty(&document).expect("serialize run plan")
    )
}

fn run_plan_node(
    id: &str,
    kind: &str,
    label: &str,
    status: &str,
    phase: &str,
    _risk: &str,
    line: usize,
    outputs: Vec<Value>,
    rerun_decision: &RerunDecision,
) -> Value {
    let risk = classify_workflow_node_review_risk(kind, status);
    json!({
        "id": id,
        "kind": kind,
        "label": label,
        "status": status,
        "phase": phase,
        "risk": risk.level,
        "risk_category": risk.category,
        "risk_severity": risk.severity,
        "rerun_status": rerun_status(rerun_decision),
        "line": line,
        "source_span": {
            "line": line
        },
        "rerun_decision": rerun_decision_json(rerun_decision),
        "outputs": outputs
    })
}

fn rerun_status(decision: &RerunDecision) -> &'static str {
    if decision.decision == "skip" {
        "skipped"
    } else {
        "executed"
    }
}

fn run_plan_edge(from: &str, to: &str, kind: &str) -> Value {
    json!({
        "from": from,
        "to": to,
        "kind": kind
    })
}

fn run_plan_node_ids(nodes: &[Value]) -> HashSet<String> {
    nodes
        .iter()
        .filter_map(|node| node.get("id").and_then(Value::as_str).map(str::to_owned))
        .collect()
}

fn push_run_plan_edge_if_present(
    edges: &mut Vec<Value>,
    node_ids: &HashSet<String>,
    from: &str,
    to: &str,
    kind: &str,
) {
    if !node_ids.contains(from) || !node_ids.contains(to) {
        return;
    }
    if edges.iter().any(|edge| {
        edge.get("from").and_then(Value::as_str) == Some(from)
            && edge.get("to").and_then(Value::as_str) == Some(to)
            && edge.get("kind").and_then(Value::as_str) == Some(kind)
    }) {
        return;
    }
    edges.push(run_plan_edge(from, to, kind));
}

fn source_binding_node_id(binding: &str, node_ids: &HashSet<String>) -> Option<String> {
    [
        format!("table_transform:{binding}"),
        format!("source:csv:{binding}"),
        format!("source:config:{binding}"),
        format!("table_selection:{binding}"),
        format!("timeseries_coverage:{binding}"),
        format!("timeseries_fill:{binding}"),
        format!("timeseries_quality:{binding}"),
        format!("quality_result:{binding}"),
        format!("model:{binding}"),
    ]
    .into_iter()
    .find(|id| node_ids.contains(id))
}

fn cache_owner_node_id(
    owner_kind: &str,
    owner_name: &str,
    node_ids: &HashSet<String>,
    runtime: bool,
) -> Option<String> {
    let candidates = if runtime {
        vec![
            format!("process:{owner_name}"),
            format!("model:{owner_name}"),
            format!("case:{owner_name}"),
            format!("boundary:{owner_kind}:{owner_name}"),
            format!("boundary:{owner_kind}:download"),
        ]
    } else {
        vec![
            format!("process:{owner_name}"),
            format!("{owner_kind}:{owner_name}"),
        ]
    };
    candidates.into_iter().find(|id| node_ids.contains(id))
}

fn add_static_dependency_edges(
    report: &CheckReport,
    node_ids: &HashSet<String>,
    edges: &mut Vec<Value>,
) {
    for transform in &report.semantic_program.table_transforms {
        let id = format!("table_transform:{}", transform.binding);
        if let Some(source_id) = source_binding_node_id(&transform.source_table, node_ids) {
            push_run_plan_edge_if_present(edges, node_ids, &id, &source_id, "depends_on");
        }
        if let Some(secondary_table) = &transform.secondary_table {
            if let Some(source_id) = source_binding_node_id(secondary_table, node_ids) {
                push_run_plan_edge_if_present(edges, node_ids, &id, &source_id, "depends_on");
            }
        }
    }
    for kernel in &report.semantic_program.timeseries_kernels {
        let id = format!("timeseries_kernel:{}", kernel.binding);
        if let Some(source_table) = &kernel.source_table {
            if let Some(source_id) = source_binding_node_id(source_table, node_ids) {
                push_run_plan_edge_if_present(edges, node_ids, &id, &source_id, "depends_on");
            }
        }
    }
    for cache in &report.semantic_program.cache_records {
        let cache_id = format!("cache:{}:{}", cache.owner_kind, cache.owner_name);
        if let Some(owner_id) =
            cache_owner_node_id(&cache.owner_kind, &cache.owner_name, node_ids, false)
        {
            push_run_plan_edge_if_present(edges, node_ids, &owner_id, &cache_id, "uses_cache");
        }
    }
    for export in &report.semantic_program.csv_exports {
        let id = format!("csv_export:{}:{}", export.source, export.line);
        if let Some(source_id) = source_binding_node_id(&export.source, node_ids) {
            push_run_plan_edge_if_present(edges, node_ids, &id, &source_id, "depends_on");
        }
    }
    for ml in &report.semantic_program.ml_infos {
        let id = format!("model:{}", ml.binding);
        if let Some(source) = &ml.source {
            if let Some(source_id) = source_binding_node_id(source, node_ids) {
                push_run_plan_edge_if_present(edges, node_ids, &id, &source_id, "depends_on");
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn add_runtime_dependency_edges(
    report: &CheckReport,
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
    cache_records: &[CacheManifestRecord],
    db_records: &[DbManifestRecord],
    output_artifacts: &[OutputArtifact],
    node_ids: &HashSet<String>,
    edges: &mut Vec<Value>,
) {
    for selection in &runtime_data.table_selections {
        let id = format!("table_selection:{}", selection.binding);
        if let Some(source_id) = source_binding_node_id(&selection.source_table, node_ids) {
            push_run_plan_edge_if_present(edges, node_ids, &id, &source_id, "depends_on");
        }
    }
    for transform in &runtime_data.table_transforms {
        let id = format!("table_transform:{}", transform.binding);
        if let Some(source_id) = source_binding_node_id(&transform.source_table, node_ids) {
            push_run_plan_edge_if_present(edges, node_ids, &id, &source_id, "depends_on");
        }
        if let Some(secondary_table) = &transform.secondary_table {
            if let Some(source_id) = source_binding_node_id(secondary_table, node_ids) {
                push_run_plan_edge_if_present(edges, node_ids, &id, &source_id, "depends_on");
            }
        }
    }
    for coverage in &runtime_data.timeseries_coverage {
        let id = format!("timeseries_coverage:{}", coverage.binding);
        if let Some(source_id) = source_binding_node_id(&coverage.source_table, node_ids) {
            push_run_plan_edge_if_present(edges, node_ids, &id, &source_id, "depends_on");
        }
    }
    for fill in &runtime_data.timeseries_fill {
        let id = format!("timeseries_fill:{}", fill.binding);
        if let Some(source_id) = source_binding_node_id(&fill.source_table, node_ids) {
            push_run_plan_edge_if_present(edges, node_ids, &id, &source_id, "depends_on");
        }
    }
    for quality in &runtime_data.timeseries_quality {
        let id = format!("timeseries_quality:{}", quality.binding);
        let mut has_dependency = false;
        if let Some(fill_binding) = &quality.fill_binding {
            let fill_id = format!("timeseries_fill:{fill_binding}");
            push_run_plan_edge_if_present(edges, node_ids, &id, &fill_id, "depends_on");
            has_dependency |= node_ids.contains(&fill_id);
        }
        if let Some(coverage_binding) = &quality.coverage_binding {
            let coverage_id = format!("timeseries_coverage:{coverage_binding}");
            push_run_plan_edge_if_present(edges, node_ids, &id, &coverage_id, "depends_on");
            has_dependency |= node_ids.contains(&coverage_id);
        }
        if !has_dependency {
            if let Some(source_id) = source_binding_node_id(&quality.source_table, node_ids) {
                push_run_plan_edge_if_present(edges, node_ids, &id, &source_id, "depends_on");
            }
        }
    }
    for suite in &runtime_data.expectation_suites {
        let id = format!("expectation_suite:{}", suite.binding);
        if let Some(source_id) = source_binding_node_id(&suite.target, node_ids) {
            push_run_plan_edge_if_present(edges, node_ids, &id, &source_id, "depends_on");
        }
    }
    for quality in &runtime_data.quality_results {
        let id = format!("quality_result:{}", quality.binding);
        let target_id = format!("timeseries_quality:{}", quality.target);
        if node_ids.contains(&target_id) {
            push_run_plan_edge_if_present(edges, node_ids, &id, &target_id, "depends_on");
        } else {
            let expectation_suite_id = format!("expectation_suite:{}", quality.target);
            if node_ids.contains(&expectation_suite_id) {
                push_run_plan_edge_if_present(
                    edges,
                    node_ids,
                    &id,
                    &expectation_suite_id,
                    "depends_on",
                );
                continue;
            }
            if let Some(source_table) = &quality.source_table {
                if let Some(source_id) = source_binding_node_id(source_table, node_ids) {
                    push_run_plan_edge_if_present(edges, node_ids, &id, &source_id, "depends_on");
                }
            }
        }
    }
    for cache in cache_records {
        let cache_id = format!("cache:{}:{}", cache.owner_kind, cache.owner_name);
        if let Some(owner_id) =
            cache_owner_node_id(&cache.owner_kind, &cache.owner_name, node_ids, true)
        {
            push_run_plan_edge_if_present(edges, node_ids, &owner_id, &cache_id, "uses_cache");
        }
    }
    for case_manifest in &runtime_data.case_manifests {
        let id = format!("case:{}", case_manifest.case_id);
        if let Some(sample_id) = source_binding_node_id(&case_manifest.sample_table, node_ids) {
            push_run_plan_edge_if_present(edges, node_ids, &id, &sample_id, "depends_on");
        }
        for process in &case_manifest.process_bindings {
            let process_id = format!("process:{process}");
            push_run_plan_edge_if_present(edges, node_ids, &id, &process_id, "depends_on");
        }
    }
    for db in db_records {
        let db_id = format!("db:{}", db.binding);
        let process_id = format!("process:{}", db.binding);
        push_run_plan_edge_if_present(edges, node_ids, &db_id, &process_id, "depends_on");
    }
    for artifact in &runtime_data.ml_artifacts {
        let id = format!("model:{}", artifact.binding);
        if let Some(source) = &artifact.source {
            if let Some(source_id) = source_binding_node_id(source, node_ids) {
                push_run_plan_edge_if_present(edges, node_ids, &id, &source_id, "depends_on");
            }
        }
    }
    add_output_artifact_dependency_edges(
        report,
        runtime_data,
        process_results,
        db_records,
        output_artifacts,
        node_ids,
        edges,
    );
}

fn add_output_artifact_dependency_edges(
    report: &CheckReport,
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
    db_records: &[DbManifestRecord],
    output_artifacts: &[OutputArtifact],
    node_ids: &HashSet<String>,
    edges: &mut Vec<Value>,
) {
    for artifact in output_artifacts {
        let artifact_id = format!("artifact:{}", artifact.path);
        for process in process_results {
            if process.expected_outputs.iter().any(|expected| {
                path_for_manifest(&expected.resolved_path) == artifact.path
                    || expected.path == artifact.path
            }) {
                let process_id = format!("process:{}", process.binding);
                push_run_plan_edge_if_present(
                    edges,
                    node_ids,
                    &process_id,
                    &artifact_id,
                    "produces",
                );
            }
        }
        for db in db_records {
            if db.manifest_path == artifact.path || db.resolved_path == artifact.path {
                let db_id = format!("db:{}", db.binding);
                push_run_plan_edge_if_present(edges, node_ids, &db_id, &artifact_id, "produces");
            }
        }
        for case_manifest in &runtime_data.case_manifests {
            if case_manifest
                .output_artifacts
                .iter()
                .any(|path| path == &artifact.path)
            {
                let case_id = format!("case:{}", case_manifest.case_id);
                push_run_plan_edge_if_present(edges, node_ids, &case_id, &artifact_id, "produces");
            }
        }
        for export in &report.semantic_program.csv_exports {
            if export.path == artifact.path {
                let export_id = format!("csv_export:{}:{}", export.source, export.line);
                push_run_plan_edge_if_present(
                    edges,
                    node_ids,
                    &export_id,
                    &artifact_id,
                    "produces",
                );
            }
        }
        for command in report
            .semantic_program
            .command_styles
            .iter()
            .filter(|command| {
                command.verb == "render" && command.target.trim().starts_with("template ")
            })
        {
            let Some(output_expression) = template_command_output_expression(report, command)
            else {
                continue;
            };
            let Some(output_path) =
                evaluate_runtime_output_path_expression(&output_expression, report)
            else {
                continue;
            };
            let output_path = runtime_path_text(output_path);
            let manifest_path = runtime_path_text(format!("{output_path}.render_manifest.json"));
            if artifact.path == output_path || artifact.path == manifest_path {
                let render_id = template_render_node_id(command);
                push_run_plan_edge_if_present(
                    edges,
                    node_ids,
                    &render_id,
                    &artifact_id,
                    "produces",
                );
            }
        }
    }
}

fn source_records_for_registry(registry: &ArtifactRegistryContext<'_>) -> Vec<SourceRecord> {
    let structured_reads = &registry.runtime_data.structured_reads;
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
            .config_promotions
            .iter()
            .map(|promotion| SourceRecord {
                kind: format!("config_{}", promotion.format),
                binding: promotion.binding.clone(),
                path: promotion.resolved_path.clone(),
                hash: promotion.source_hash.clone(),
                schema: Some(promotion.schema_name.clone()),
                row_count: None,
                status: promotion.status.clone(),
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
                status: structured_reads
                    .iter()
                    .find(|read| read.binding == dependency.name)
                    .map(|read| read.parse_status.clone())
                    .unwrap_or_else(|| dependency.status.clone()),
                line: dependency.line,
            }),
    );
    records.extend(network_fixture_source_records(registry));
    records
}

fn network_fixture_source_records(registry: &ArtifactRegistryContext<'_>) -> Vec<SourceRecord> {
    let source_base = registry.report.source_path.parent();
    let mut records = Vec::new();
    for request in &registry.report.semantic_program.net_requests {
        let Some(fixture) = &request.fixture else {
            continue;
        };
        records.push(SourceRecord {
            kind: "source_file".to_owned(),
            binding: request.binding.clone(),
            path: path_for_manifest(&runtime_resolve_source_relative_path(fixture, source_base)),
            hash: request.response_hash.clone(),
            schema: None,
            row_count: None,
            status: request.status.clone(),
            line: request.line,
        });
    }
    for download in &registry.report.semantic_program.net_downloads {
        let Some(fixture) = &download.fixture else {
            continue;
        };
        records.push(SourceRecord {
            kind: "source_file".to_owned(),
            binding: format!("download:{}", download.target_value),
            path: path_for_manifest(&runtime_resolve_source_relative_path(fixture, source_base)),
            hash: download.response_hash.clone(),
            schema: None,
            row_count: None,
            status: download.status.clone(),
            line: download.line,
        });
    }
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
            overwrite_policy: artifact.overwrite_policy.clone(),
            status: "generated".to_owned(),
            validation: artifact.validation.clone(),
        })
        .collect()
}

fn model_artifact_records_for_registry(
    registry: &ArtifactRegistryContext<'_>,
) -> Vec<ModelArtifactRecord> {
    registry
        .runtime_data
        .ml_artifacts
        .iter()
        .filter(|artifact| artifact.model_artifact_hash.is_some() || artifact.model_card.is_some())
        .map(|artifact| {
            let hash = artifact
                .model_artifact_hash
                .clone()
                .or_else(|| artifact.model_card.as_ref().map(|card| hash_text(card)))
                .unwrap_or_else(|| hash_text(&artifact.binding));
            ModelArtifactRecord {
                artifact: ArtifactRecord {
                    kind: "model_artifact".to_owned(),
                    class: "model".to_owned(),
                    path: format!("model://{}", artifact.binding),
                    hash,
                    overwrite_policy: None,
                    status: artifact.status.clone(),
                    validation: artifact_validation(
                        "passed",
                        "model_artifact_hash",
                        "model artifact hash was recorded",
                    ),
                },
                binding: artifact.binding.clone(),
                kind: artifact.kind.clone(),
                source: artifact.source.clone(),
                target: artifact.target.clone(),
                target_quantity: artifact.target_quantity.clone(),
                target_unit: artifact.display_unit.clone(),
                training_data_hash: artifact.training_data_hash.clone(),
                model_artifact_hash: artifact.model_artifact_hash.clone(),
                status: artifact.status.clone(),
                line: artifact.line,
            }
        })
        .collect()
}

fn external_boundary_records_for_run(
    report: &CheckReport,
    processes: &[ProcessExecutionRecord],
    db_records: &[DbManifestRecord],
    cache_records: &[CacheManifestRecord],
) -> Vec<ExternalBoundaryRecord> {
    let mut records = external_boundary_records_for_processes(processes);
    records.extend(external_boundary_records_for_network(report, cache_records));
    records.extend(external_boundary_records_for_db_manifests(db_records));
    records
}

fn external_boundary_records_for_processes(
    processes: &[ProcessExecutionRecord],
) -> Vec<ExternalBoundaryRecord> {
    processes
        .iter()
        .map(|process| ExternalBoundaryRecord {
            kind: "process".to_owned(),
            binding: process.binding.clone(),
            command: process.command.clone(),
            target: process.command.clone(),
            tool_version: process.tool_version.clone(),
            args: process.args.clone(),
            env_keys: process.env_keys.clone(),
            cwd: process.cwd.clone(),
            timeout: process.timeout.clone(),
            retry: process.retry,
            attempt_count: process.attempt_count,
            allow_failure: process.allow_failure,
            timed_out: process.timed_out,
            output_paths: process
                .expected_outputs
                .iter()
                .map(|output| output.path.clone())
                .collect(),
            expected_output_count: process.expected_outputs.len(),
            expected_output_status: process.expected_output_status.clone(),
            response_hash: None,
            expected_hash: None,
            stdout_hash: process.stdout_hash.clone(),
            stderr_hash: process.stderr_hash.clone(),
            success: process.success,
            status: process.status.clone(),
            line: process.line,
        })
        .collect()
}

fn network_cache_hit_hash<'a>(
    cache_records: &'a [CacheManifestRecord],
    owner_kind: &str,
    owner_name: &str,
) -> Option<&'a str> {
    cache_records
        .iter()
        .find(|record| {
            record.owner_kind == owner_kind
                && record.owner_name == owner_name
                && record.lookup_status == "hit"
                && record.status == "hit"
        })
        .and_then(|record| record.observed_hash.as_deref())
}

fn network_status_with_cache(
    status: &str,
    response_hash: Option<&str>,
    cached_hash: Option<&str>,
) -> String {
    if response_hash.is_none() && cached_hash.is_some() {
        "cached".to_owned()
    } else {
        status.to_owned()
    }
}

fn external_boundary_records_for_network(
    report: &CheckReport,
    cache_records: &[CacheManifestRecord],
) -> Vec<ExternalBoundaryRecord> {
    let mut records = Vec::new();
    for request in &report.semantic_program.net_requests {
        let cached_hash =
            network_cache_hit_hash(cache_records, "network_request", &request.binding);
        let response_hash = request
            .response_hash
            .clone()
            .or_else(|| cached_hash.map(ToOwned::to_owned));
        let status = network_status_with_cache(
            &request.status,
            request.response_hash.as_deref(),
            cached_hash,
        );
        records.push(ExternalBoundaryRecord {
            kind: "network_request".to_owned(),
            binding: request.binding.clone(),
            command: request.method.clone(),
            target: request.url_value.clone(),
            tool_version: None,
            args: network_query_args(&request.query),
            env_keys: Vec::new(),
            cwd: String::new(),
            timeout: None,
            retry: 0,
            attempt_count: 1,
            allow_failure: false,
            timed_out: false,
            output_paths: Vec::new(),
            expected_output_count: 0,
            expected_output_status: "not_applicable".to_owned(),
            response_hash,
            expected_hash: request.expected_sha256.clone(),
            stdout_hash: request
                .response_hash
                .as_deref()
                .or(cached_hash)
                .unwrap_or_default()
                .to_owned(),
            stderr_hash: String::new(),
            success: external_boundary_status_success(&status),
            status,
            line: request.line,
        });
    }
    for download in &report.semantic_program.net_downloads {
        let cached_hash =
            network_cache_hit_hash(cache_records, "network_download", &download.target_value);
        let response_hash = download
            .response_hash
            .clone()
            .or_else(|| cached_hash.map(ToOwned::to_owned));
        let status = network_status_with_cache(
            &download.status,
            download.response_hash.as_deref(),
            cached_hash,
        );
        records.push(ExternalBoundaryRecord {
            kind: "network_download".to_owned(),
            binding: "download".to_owned(),
            command: "download".to_owned(),
            target: download.url_value.clone(),
            tool_version: None,
            args: network_query_args(&download.query),
            env_keys: Vec::new(),
            cwd: String::new(),
            timeout: None,
            retry: 0,
            attempt_count: 1,
            allow_failure: false,
            timed_out: false,
            output_paths: vec![download.target_value.clone()],
            expected_output_count: 1,
            expected_output_status: status.clone(),
            response_hash,
            expected_hash: download.expected_sha256.clone(),
            stdout_hash: download
                .response_hash
                .as_deref()
                .or(cached_hash)
                .unwrap_or_default()
                .to_owned(),
            stderr_hash: String::new(),
            success: external_boundary_status_success(&status),
            status,
            line: download.line,
        });
    }
    records
}

fn external_boundary_records_for_db_manifests(
    records: &[DbManifestRecord],
) -> Vec<ExternalBoundaryRecord> {
    records
        .iter()
        .map(|record| ExternalBoundaryRecord {
            kind: "db_write".to_owned(),
            binding: record.binding.clone(),
            command: "db write manifest".to_owned(),
            target: record
                .database
                .clone()
                .unwrap_or_else(|| record.manifest_path.clone()),
            tool_version: None,
            args: Vec::new(),
            env_keys: Vec::new(),
            cwd: String::new(),
            timeout: None,
            retry: 0,
            attempt_count: 1,
            allow_failure: false,
            timed_out: false,
            output_paths: vec![record.manifest_path.clone()],
            expected_output_count: 1,
            expected_output_status: record.status.clone(),
            response_hash: record.hash.clone(),
            expected_hash: None,
            stdout_hash: record.hash.clone().unwrap_or_default(),
            stderr_hash: String::new(),
            success: record.status == "manifest_loaded",
            status: record.status.clone(),
            line: record.line,
        })
        .collect()
}

fn network_query_args(query: &[eng_compiler::NetQueryParam]) -> Vec<String> {
    query
        .iter()
        .map(|param| {
            if param.redacted {
                format!("{}=<redacted>", param.key)
            } else {
                format!("{}={}", param.key, param.value)
            }
        })
        .collect()
}

fn external_boundary_status_success(status: &str) -> bool {
    !matches!(status, "error" | "failed" | "invalid" | "missing")
}

fn push_output_manifest_network_requests_json(
    json: &mut String,
    registry: &ArtifactRegistryContext<'_>,
) {
    for (index, record) in registry
        .external_boundary_records
        .iter()
        .filter(|record| record.kind == "network_request")
        .enumerate()
    {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"http_get\",\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&record.binding)
        ));
        json.push_str(&format!(
            "        \"url\": \"{}\",\n",
            json_escape(&record.target)
        ));
        push_optional_json_string_runtime(
            json,
            "response_hash",
            record.response_hash.as_deref(),
            8,
        );
        push_optional_json_string_runtime(
            json,
            "expected_sha256",
            record.expected_hash.as_deref(),
            8,
        );
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&record.status)
        ));
        json.push_str(&format!("        \"line\": {}\n", record.line));
        json.push_str("      }");
    }
}

fn push_output_manifest_downloads_json(json: &mut String, registry: &ArtifactRegistryContext<'_>) {
    for (index, record) in registry
        .external_boundary_records
        .iter()
        .filter(|record| record.kind == "network_download")
        .enumerate()
    {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"download\",\n");
        json.push_str(&format!(
            "        \"url\": \"{}\",\n",
            json_escape(&record.target)
        ));
        if let Some(target) = record.output_paths.first() {
            json.push_str(&format!("        \"path\": \"{}\",\n", json_escape(target)));
        }
        push_optional_json_string_runtime(json, "hash", record.response_hash.as_deref(), 8);
        push_optional_json_string_runtime(
            json,
            "expected_sha256",
            record.expected_hash.as_deref(),
            8,
        );
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&record.status)
        ));
        json.push_str(&format!("        \"line\": {}\n", record.line));
        json.push_str("      }");
    }
}

fn push_output_manifest_caches_json(json: &mut String, records: &[CacheManifestRecord]) {
    for (index, record) in records.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&record.owner_kind)
        ));
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&record.owner_name)
        ));
        json.push_str(&format!(
            "        \"target\": \"{}\",\n",
            json_escape(&record.cache_path)
        ));
        json.push_str(&format!(
            "        \"cache_dir\": \"{}\",\n",
            json_escape(&record.cache_dir)
        ));
        json.push_str(&format!(
            "        \"cache_key_hash\": \"{}\",\n",
            json_escape(&record.cache_key_hash)
        ));
        json.push_str(&format!(
            "        \"source_hash\": \"{}\",\n",
            json_escape(&record.source_hash)
        ));
        json.push_str(&format!(
            "        \"lookup_status\": \"{}\",\n",
            json_escape(&record.lookup_status)
        ));
        push_optional_json_string_runtime(json, "hash", record.observed_hash.as_deref(), 8);
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&record.status)
        ));
        json.push_str(&format!("        \"line\": {}\n", record.line));
        json.push_str("      }");
    }
}

fn push_artifact_registry_json(
    json: &mut String,
    artifact_records: &[ArtifactRecord],
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
    for (index, record) in artifact_records.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        push_artifact_record_fields_json(json, record, 8);
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");

    json.push_str("    \"external_commands\": [\n");
    for (index, record) in registry
        .external_boundary_records
        .iter()
        .filter(|record| record.kind == "process")
        .enumerate()
    {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&record.kind)
        ));
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&record.binding)
        ));
        json.push_str(&format!(
            "        \"command\": \"{}\",\n",
            json_escape(&record.command)
        ));
        json.push_str(&format!(
            "        \"target\": \"{}\",\n",
            json_escape(&record.target)
        ));
        push_optional_json_string_runtime(json, "tool_version", record.tool_version.as_deref(), 8);
        json.push_str("        \"args\": ");
        push_json_string_array_runtime(json, &record.args);
        json.push_str(",\n");
        json.push_str("        \"outputs\": ");
        push_json_string_array_runtime(json, &record.output_paths);
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

    json.push_str("    \"network_requests\": [\n");
    push_output_manifest_network_requests_json(json, registry);
    json.push_str("\n    ],\n");

    json.push_str("    \"downloads\": [\n");
    push_output_manifest_downloads_json(json, registry);
    json.push_str("\n    ],\n");

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
    for (index, record) in model_artifact_records_for_registry(registry)
        .iter()
        .enumerate()
    {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str("        \"artifact\": {\n");
        push_artifact_record_fields_json(json, &record.artifact, 10);
        json.push_str("        },\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&record.binding)
        ));
        json.push_str(&format!(
            "        \"kind\": \"{}\",\n",
            json_escape(&record.kind)
        ));
        push_optional_json_string_runtime(json, "source", record.source.as_deref(), 8);
        push_optional_json_string_runtime(json, "target", record.target.as_deref(), 8);
        push_optional_json_string_runtime(
            json,
            "target_quantity",
            record.target_quantity.as_deref(),
            8,
        );
        json.push_str(&format!(
            "        \"target_unit\": \"{}\",\n",
            json_escape(&record.target_unit)
        ));
        push_optional_json_string_runtime(
            json,
            "training_data_hash",
            record.training_data_hash.as_deref(),
            8,
        );
        push_optional_json_string_runtime(
            json,
            "model_artifact_hash",
            record.model_artifact_hash.as_deref(),
            8,
        );
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&record.status)
        ));
        json.push_str(&format!("        \"line\": {}\n", record.line));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");

    json.push_str("    \"caches\": [\n");
    push_output_manifest_caches_json(json, registry.cache_manifest_records);
    json.push_str("\n    ],\n");

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

fn push_artifact_record_fields_json(json: &mut String, record: &ArtifactRecord, indent: usize) {
    let padding = " ".repeat(indent);
    json.push_str(&format!(
        "{padding}\"kind\": \"{}\",\n",
        json_escape(&record.kind)
    ));
    json.push_str(&format!(
        "{padding}\"class\": \"{}\",\n",
        json_escape(&record.class)
    ));
    json.push_str(&format!(
        "{padding}\"path\": \"{}\",\n",
        json_escape(&record.path)
    ));
    json.push_str(&format!(
        "{padding}\"hash\": \"{}\",\n",
        json_escape(&record.hash)
    ));
    if let Some(policy) = &record.overwrite_policy {
        json.push_str(&format!(
            "{padding}\"overwrite_policy\": \"{}\",\n",
            json_escape(policy)
        ));
    }
    json.push_str(&format!(
        "{padding}\"status\": \"{}\",\n",
        json_escape(&record.status)
    ));
    push_artifact_validation_json(json, &record.validation, indent);
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
        | "plot_manifest" | "bytecode" | "run_log" | "static_run_plan" | "run_plan"
        | "run_lock" => "review_artifact",
        "process_results" | "process_expected_output" => "external_boundary",
        "cache_manifest" => "cache",
        "case_input" | "case_result" | "case_manifest" | "result_collection" => "case",
        "db_write_manifest" | "sqlite_database" => "db_write",
        "template_render" | "template_render_manifest" => "template",
        "model_artifact"
        | "model_card"
        | "model_metrics"
        | "prediction_result"
        | "prediction_manifest" => "model",
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
    if let Some(value) = evaluate_network_response_field_expression(expression, report) {
        return Some(value);
    }
    if let Some(value) = evaluate_table_row_field_expression(expression, runtime_data) {
        return Some(value);
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
    if let Some(selection) = runtime_data
        .table_selections
        .iter()
        .find(|selection| selection.binding == expression)
    {
        return Some(RuntimeFormatValue::Text(
            selection.selected_value.clone().unwrap_or_default(),
        ));
    }
    if let Some(transform) = runtime_data
        .table_transforms
        .iter()
        .find(|transform| transform.binding == expression)
    {
        return Some(RuntimeFormatValue::Summary(format!(
            "TableTransform {}: {} -> {} rows ({})",
            transform.binding,
            transform.input_row_count,
            transform.output_row_count,
            transform.status
        )));
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
        if let Some(transform) = runtime_data
            .table_transforms
            .iter()
            .find(|transform| transform.binding == declaration.name)
        {
            return Some(RuntimeFormatValue::Summary(format!(
                "TableTransform {}: {} -> {} rows ({})",
                transform.binding,
                transform.input_row_count,
                transform.output_row_count,
                transform.status
            )));
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

fn evaluate_network_response_field_expression(
    expression: &str,
    report: &CheckReport,
) -> Option<RuntimeFormatValue> {
    let (binding, field) = expression.trim().split_once('.')?;
    let request = report
        .semantic_program
        .net_requests
        .iter()
        .find(|request| request.binding == binding.trim())?;
    match field.trim() {
        "body" | "text" => {
            let fixture = request.fixture.as_ref()?;
            let path = runtime_resolve_source_relative_path(fixture, report.source_path.parent());
            fs::read_to_string(path).ok().map(RuntimeFormatValue::Text)
        }
        "status" => Some(RuntimeFormatValue::Text(request.status.clone())),
        "status_class" => Some(RuntimeFormatValue::Text(request.status_class.clone())),
        "status_code" => request
            .status_code
            .map(|status_code| RuntimeFormatValue::Number {
                value: status_code as f64,
                quantity_kind: "DimensionlessNumber".to_owned(),
                unit: String::new(),
            }),
        "response_hash" | "hash" => request.response_hash.clone().map(RuntimeFormatValue::Text),
        "url" => Some(RuntimeFormatValue::Text(request.url_value.clone())),
        _ => None,
    }
}

fn evaluate_table_row_field_expression(
    expression: &str,
    runtime_data: &RuntimeData,
) -> Option<RuntimeFormatValue> {
    let (binding, field) = expression.trim().split_once('.')?;
    let transform = runtime_data.table_transforms.iter().find(|transform| {
        transform.binding == binding.trim()
            && transform.operation == "require_one"
            && transform.status == "selected"
            && transform.output_row_count == 1
    })?;
    let row_index = transform.matched_row_indices.first()?.checked_sub(1)?;
    let table = runtime_table_for_transform_source(runtime_data, &transform.source_table, 0)?;
    let column = table
        .columns
        .iter()
        .find(|column| column.name == field.trim())?;
    match &column.values {
        RuntimeValues::Text(values) => values.get(row_index).cloned().map(RuntimeFormatValue::Text),
        RuntimeValues::Number(values) => {
            values
                .get(row_index)
                .copied()
                .flatten()
                .map(|value| RuntimeFormatValue::Number {
                    value,
                    quantity_kind: column.type_name.clone(),
                    unit: column
                        .unit
                        .clone()
                        .or_else(|| column.canonical_unit.clone())
                        .unwrap_or_default(),
                })
        }
    }
}

fn runtime_table_for_transform_source<'a>(
    runtime_data: &'a RuntimeData,
    source: &str,
    depth: usize,
) -> Option<&'a RuntimeTable> {
    if depth > 16 {
        return None;
    }
    if let Some(table) = runtime_data
        .tables
        .iter()
        .find(|table| table.binding == source.trim())
    {
        return Some(table);
    }
    let transform = runtime_data
        .table_transforms
        .iter()
        .find(|transform| transform.binding == source.trim())?;
    runtime_table_for_transform_source(runtime_data, &transform.source_table, depth + 1)
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
        return Some(runtime_path_text(value));
    }
    if let Some(value) = runtime_strip_call_string_arg(expression, "dir") {
        return Some(runtime_path_text(value));
    }
    if expression.starts_with('"') {
        return Some(runtime_path_text(strip_runtime_string_value(expression)));
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
    canonical_path_text(path.as_ref())
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
    db_manifest_records: &[DbManifestRecord],
    cache_records: &[CacheManifestRecord],
    template_render_records: &[TemplateRenderRecord],
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
                let default_value = if field.redacted {
                    "<redacted>"
                } else {
                    default_value
                };
                args_schema.push_str(&format!(
                    "          \"default\": \"{}\",\n",
                    json_escape(default_value)
                ));
            } else {
                args_schema.push_str("          \"default\": null,\n");
            }
            args_schema.push_str(&format!("          \"redacted\": {},\n", field.redacted));
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
        arg_values.push_str(&format!("      \"redacted\": {},\n", arg.redacted));
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
    let config_promotions = config_promotions_json(report);
    let network_boundaries = network_boundaries_json(report, cache_records);
    let table_selections = table_selections_json(runtime_data, "      ");
    let table_transforms = table_transforms_json(runtime_data, "      ");
    let expectation_suites = expectation_suites_json(runtime_data, "      ");
    let quality_results = quality_results_json(runtime_data, "      ");
    let timeseries_coverage = timeseries_coverage_json(runtime_data, "      ");
    let timeseries_fill = timeseries_fill_json(runtime_data, "      ");
    let timeseries_quality = timeseries_quality_json(runtime_data, "      ");
    let timeseries_fallbacks = timeseries_fallbacks_json(runtime_data, "      ");
    let sample_tables = sample_tables_json(runtime_data);
    let case_manifests = case_manifests_json(runtime_data, process_results, cache_records);
    let case_tables = case_tables_json(runtime_data, process_results, cache_records);
    let case_diagnostics = case_diagnostics_json(runtime_data, process_results, cache_records);
    let db_manifests = db_manifests_json(db_manifest_records);
    let render_manifests = template_render_records_json(template_render_records, "      ");
    let model_cards = model_cards_json(runtime_data, process_results);
    let model_specs = model_specs_json(runtime_data, process_results);
    let prediction_manifests = prediction_manifests_json(runtime_data, process_results);
    let model_diagnostics = model_diagnostics_json(runtime_data, process_results);

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
        push_optional_json_string(
            &mut ml,
            "prediction_input",
            artifact.prediction_input.as_deref(),
            8,
        );
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
            "        \"binding\": \"{}\",\n",
            json_escape(&alignment.binding)
        ));
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
            "        \"strategy\": \"{}\",\n",
            json_escape(&alignment.strategy)
        ));
        time_alignments.push_str(&format!(
            "        \"method\": \"{}\",\n",
            json_escape(&alignment.method)
        ));
        push_optional_json_number(
            &mut time_alignments,
            "resample_step",
            alignment.resample_step,
            8,
        );
        push_optional_json_number(&mut time_alignments, "tolerance", alignment.tolerance, 8);
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
            "        \"status\": \"{}\",\n",
            json_escape(&alignment.status)
        ));
        time_alignments.push_str(&format!("        \"line\": {}\n", alignment.line));
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

    let mut result_json = format!(
        "{{\n  \"format\": \"engres-v1\",\n  \"result_format_version\": 1,\n  \"runtime_version\": \"{RUNTIME_VERSION}\",\n  \"compiler_version\": \"{}\",\n  \"bytecode_version\": {},\n  \"source_path\": \"{}\",\n  \"source_hash\": \"{}\",\n  \"bytecode_hash\": \"{}\",\n  \"numeric_profile\": \"preview-f64\",\n  \"execution_profile\": \"{}\",\n  \"workflow\": {{\n    \"kind\": \"{}\",\n    \"arg_name\": \"{}\",\n    \"arg_type\": \"{}\",\n    \"return_type\": \"{}\"\n  }},\n  \"args_schema\": [\n{}\n  ],\n  \"arg_values\": [\n{}\n  ],\n  \"object_store\": {{\n    \"scalar_count\": {},\n    \"table_count\": {},\n    \"timeseries_count\": {},\n    \"array_count\": {},\n    \"objects\": [\n{}\n    ]\n  }},\n  \"typed_payload\": {{\n    \"kind\": \"{}\",\n    \"status\": \"ok\",\n    \"result_format\": \"{}\",\n    \"vm_steps\": [{}],\n    \"numeric_values\": [\n{}\n    ],\n    \"statistics\": [\n{}\n    ],\n    \"integrations\": [\n{}\n    ],\n    \"table_diagnostics\": [\n{}\n    ],\n    \"structured_reads\": [\n{}\n    ],\n    \"config_promotions\": [\n{}\n    ],\n    \"network_boundaries\": [\n{}\n    ],\n    \"table_selections\": [\n{}\n    ],\n    \"sample_tables\": [\n{}\n    ],\n    \"case_manifests\": [\n{}\n    ],\n    \"db_manifests\": [\n{}\n    ],\n    \"render_manifests\": [\n{}\n    ],\n    \"timeseries_uncertainty_calculations\": [\n{}\n    ],\n    \"metrics\": [\n{}\n    ],\n    \"validations\": [\n{}\n    ],\n    \"time_axes\": [\n{}\n    ],\n    \"timeseries_coverage\": [\n{}\n    ],\n    \"timeseries_fill\": [\n{}\n    ],\n    \"timeseries_fallbacks\": [\n{}\n    ],\n    \"time_alignments\": [\n{}\n    ],\n    \"uncertainties\": [\n{}\n    ],\n    \"ml\": [\n{}\n    ],\n    \"model_cards\": [\n{}\n    ],\n    \"policy_results\": [\n{}\n    ],\n    \"systems\": [\n{}\n    ],\n    \"component_solutions\": [\n{}\n    ],\n    \"solver_boundaries\": [\n{}\n    ],\n    \"system_ir\": [\n{}\n    ]\n  }},\n  \"provenance\": {{\n    \"schema_count\": {},\n    \"csv_promotion_count\": {},\n    \"config_promotion_count\": {},\n    \"network_boundary_count\": {},\n    \"system_count\": {},\n    \"equation_count\": {},\n    \"residual_count\": {},\n    \"component_solution_count\": {},\n    \"environment_dependencies\": [\n{}\n    ],\n    \"profile_diagnostics\": [\n{}\n    ],\n    \"data_hashes\": [\n{}\n    ],\n    \"unit_conversion_history\": [],\n    \"plot_spec_hash\": \"{}\",\n    \"report_spec_hash\": \"{}\",\n    \"schema_hash\": \"preview\"\n  }}\n}}\n",
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
        config_promotions,
        network_boundaries,
        table_selections,
        sample_tables,
        case_manifests,
        db_manifests,
        render_manifests,
        timeseries_uncertainty_calculations,
        metrics,
        validations,
        time_axes,
        timeseries_coverage,
        timeseries_fill,
        timeseries_fallbacks,
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
        report.semantic_program.config_promotions.len(),
        report.semantic_program.net_requests.len() + report.semantic_program.net_downloads.len(),
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
    );
    let quality_results_marker = "    ],\n    \"time_axes\": [\n";
    let quality_results_block = format!(
        "    ],\n    \"expectation_suites\": [\n{}\n    ],\n    \"quality_results\": [\n{}\n    ],\n    \"time_axes\": [\n",
        expectation_suites, quality_results
    );
    result_json = result_json.replace(quality_results_marker, &quality_results_block);
    let timeseries_quality_marker = "    ],\n    \"timeseries_fallbacks\": [\n";
    let timeseries_quality_block = format!(
        "    ],\n    \"timeseries_quality\": [\n{}\n    ],\n    \"timeseries_fallbacks\": [\n",
        timeseries_quality
    );
    result_json = result_json.replace(timeseries_quality_marker, &timeseries_quality_block);
    let table_transform_marker = "    ],\n    \"sample_tables\": [\n";
    let table_transform_block = format!(
        "    ],\n    \"table_transforms\": [\n{}\n    ],\n    \"sample_tables\": [\n",
        table_transforms
    );
    result_json = result_json.replacen(table_transform_marker, &table_transform_block, 1);
    let case_tables_marker = "    ],\n    \"db_manifests\": [\n";
    let case_tables_block = format!(
        "    ],\n    \"case_tables\": [\n{}\n    ],\n    \"case_diagnostics\": [\n{}\n    ],\n    \"db_manifests\": [\n",
        case_tables, case_diagnostics
    );
    result_json = result_json.replacen(case_tables_marker, &case_tables_block, 1);
    let model_specs_marker = "    ],\n    \"policy_results\": [\n";
    let model_specs_block = format!(
        "    ],\n    \"model_specs\": [\n{}\n    ],\n    \"prediction_manifests\": [\n{}\n    ],\n    \"model_diagnostics\": [\n{}\n    ],\n    \"policy_results\": [\n",
        model_specs, prediction_manifests, model_diagnostics
    );
    result_json = result_json.replacen(model_specs_marker, &model_specs_block, 1);
    result_json
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
    external_boundary_records: &[ExternalBoundaryRecord],
    artifacts: &[OutputArtifact],
    cache_records: &[CacheManifestRecord],
    template_render_records: &[TemplateRenderRecord],
    db_manifest_records: &[DbManifestRecord],
) -> String {
    let enriched_boundaries =
        enrich_runtime_review_boundaries(base_review, process_results, external_boundary_records);
    let enriched_side_effects = enrich_runtime_review_side_effects(&enriched_boundaries, artifacts);
    let enriched_caches = enrich_runtime_review_caches(&enriched_side_effects, cache_records);
    let runtime_fallbacks = timeseries_review_fallback_records(runtime_data);
    let enriched_review =
        enrich_runtime_review_fallbacks(&enriched_caches, runtime_fallbacks.as_slice());
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
    json.push_str("\n  ],\n  \"table_selections\": [\n");
    json.push_str(&table_selections_json(runtime_data, "    "));
    json.push_str("\n  ],\n  \"table_transforms\": [\n");
    json.push_str(&table_transforms_json(runtime_data, "    "));
    json.push_str("\n  ],\n  \"expectation_suites\": [\n");
    json.push_str(&expectation_suites_json(runtime_data, "    "));
    json.push_str("\n  ],\n  \"quality_results\": [\n");
    json.push_str(&quality_results_json(runtime_data, "    "));
    json.push_str("\n  ],\n  \"render_manifests\": [\n");
    json.push_str(&template_render_records_json(
        template_render_records,
        "    ",
    ));
    json.push_str("\n  ],\n  \"timeseries_coverage\": [\n");
    json.push_str(&timeseries_coverage_json(runtime_data, "    "));
    json.push_str("\n  ],\n  \"timeseries_fill\": [\n");
    json.push_str(&timeseries_fill_json(runtime_data, "    "));
    json.push_str("\n  ],\n  \"timeseries_quality\": [\n");
    json.push_str(&timeseries_quality_json(runtime_data, "    "));
    json.push_str("\n  ],\n  \"timeseries_fallbacks\": [\n");
    json.push_str(&timeseries_fallbacks_json(runtime_data, "    "));
    json.push_str("\n  ],\n  \"case_manifests\": [\n");
    json.push_str(&case_manifests_json(
        runtime_data,
        process_results,
        cache_records,
    ));
    json.push_str("\n  ],\n  \"case_tables\": [\n");
    json.push_str(&case_tables_json(
        runtime_data,
        process_results,
        cache_records,
    ));
    json.push_str("\n  ],\n  \"case_diagnostics\": [\n");
    json.push_str(&case_diagnostics_json(
        runtime_data,
        process_results,
        cache_records,
    ));
    json.push_str("\n  ],\n  \"db_manifests\": [\n");
    json.push_str(&db_manifests_json(db_manifest_records));
    json.push_str("\n  ],\n  \"model_cards\": [\n");
    json.push_str(&model_cards_json(runtime_data, process_results));
    json.push_str("\n  ],\n  \"model_specs\": [\n");
    json.push_str(&model_specs_json(runtime_data, process_results));
    json.push_str("\n  ],\n  \"prediction_manifests\": [\n");
    json.push_str(&prediction_manifests_json(runtime_data, process_results));
    json.push_str("\n  ],\n  \"model_diagnostics\": [\n");
    json.push_str(&model_diagnostics_json(runtime_data, process_results));
    json.push_str("\n  ]\n}\n");
    json
}

fn enrich_runtime_review_boundaries(
    base_review: &str,
    process_results: &[ProcessExecutionRecord],
    external_boundary_records: &[ExternalBoundaryRecord],
) -> String {
    if external_boundary_records.is_empty() {
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

    for record in external_boundary_records {
        let Some(boundary) = boundaries
            .iter_mut()
            .find(|boundary| review_boundary_matches_record(boundary, record))
        else {
            continue;
        };
        let Some(object) = boundary.as_object_mut() else {
            continue;
        };
        let process = process_results
            .iter()
            .find(|process| record.kind == "process" && process.line == record.line);

        object.insert(
            "provenance".to_owned(),
            Value::String(
                if record.kind == "process" {
                    "runtime_process_result"
                } else {
                    "runtime_external_boundary_record"
                }
                .to_owned(),
            ),
        );
        object.insert("success".to_owned(), Value::Bool(record.success));
        object.insert("status".to_owned(), Value::String(record.status.clone()));
        object.insert("target".to_owned(), Value::String(record.target.clone()));
        object.insert("outputs".to_owned(), json!(record.output_paths));
        object.insert(
            "expected_output_status".to_owned(),
            Value::String(record.expected_output_status.clone()),
        );
        object.insert(
            "stdout_hash".to_owned(),
            Value::String(record.stdout_hash.clone()),
        );
        object.insert(
            "stderr_hash".to_owned(),
            Value::String(record.stderr_hash.clone()),
        );
        if let Some(hash) = &record.response_hash {
            object.insert("response_hash".to_owned(), Value::String(hash.clone()));
        }
        if let Some(hash) = &record.expected_hash {
            object.insert("expected_sha256".to_owned(), Value::String(hash.clone()));
        }
        if let Some(process) = process {
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
            object.insert("env".to_owned(), json!(process.env_keys));
            object.insert("timeout".to_owned(), json!(process.timeout));
            object.insert("retry".to_owned(), json!(process.retry));
            object.insert("attempt_count".to_owned(), json!(process.attempt_count));
            object.insert("allow_failure".to_owned(), json!(process.allow_failure));
            object.insert("timed_out".to_owned(), json!(process.timed_out));
            object.insert("exit_code".to_owned(), json!(process.exit_code));
            object.insert("duration_ms".to_owned(), json!(process.duration_ms));
            object.insert("output_artifacts".to_owned(), json!(output_artifacts));
        }
    }

    serde_json::to_string_pretty(&review)
        .map(|mut json| {
            json.push('\n');
            json
        })
        .unwrap_or_else(|_| base_review.to_owned())
}

fn review_boundary_matches_record(boundary: &Value, record: &ExternalBoundaryRecord) -> bool {
    let name_matches = match boundary.get("name").and_then(Value::as_str) {
        Some(name) => name == record.binding,
        None => true,
    };
    boundary.get("kind").and_then(Value::as_str) == Some(record.kind.as_str())
        && boundary
            .get("line")
            .or_else(|| boundary.get("source_line"))
            .and_then(Value::as_u64)
            == Some(record.line as u64)
        && name_matches
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

fn enrich_runtime_review_caches(base_review: &str, records: &[CacheManifestRecord]) -> String {
    if records.is_empty() {
        return base_review.to_owned();
    }

    let Ok(mut review) = serde_json::from_str::<Value>(base_review) else {
        return base_review.to_owned();
    };
    let Some(caches) = review
        .pointer_mut("/review_document/caches")
        .and_then(Value::as_array_mut)
    else {
        return base_review.to_owned();
    };

    for cache in caches {
        let Some(owner_kind) = cache.get("owner_kind").and_then(Value::as_str) else {
            continue;
        };
        let Some(owner_name) = cache.get("owner_name").and_then(Value::as_str) else {
            continue;
        };
        let Some(record) = records
            .iter()
            .find(|record| record.owner_kind == owner_kind && record.owner_name == owner_name)
        else {
            continue;
        };
        let Some(object) = cache.as_object_mut() else {
            continue;
        };
        object.insert(
            "provenance".to_owned(),
            Value::String("runtime_cache_manifest".to_owned()),
        );
        object.insert(
            "lookup_status".to_owned(),
            Value::String(record.lookup_status.clone()),
        );
        object.insert("status".to_owned(), Value::String(record.status.clone()));
        object.insert(
            "resolved_path".to_owned(),
            Value::String(record.resolved_path.clone()),
        );
    }

    serde_json::to_string_pretty(&review)
        .map(|mut json| {
            json.push('\n');
            json
        })
        .unwrap_or_else(|_| base_review.to_owned())
}

fn enrich_runtime_review_fallbacks(base_review: &str, records: &[ReviewFallbackRecord]) -> String {
    if records.is_empty() {
        return base_review.to_owned();
    }

    let Ok(mut review) = serde_json::from_str::<Value>(base_review) else {
        return base_review.to_owned();
    };
    let fallback_count = {
        let Some(fallbacks) = review
            .pointer_mut("/review_document/fallbacks")
            .and_then(Value::as_array_mut)
        else {
            return base_review.to_owned();
        };

        for record in records {
            fallbacks.push(record.to_json_value());
        }
        fallbacks.len()
    };

    if let Some(contract) = review
        .pointer_mut("/review_document/root_contract")
        .and_then(Value::as_object_mut)
    {
        contract.insert("fallback_count".to_owned(), json!(fallback_count));
    }

    serde_json::to_string_pretty(&review)
        .map(|mut json| {
            json.push('\n');
            json
        })
        .unwrap_or_else(|_| base_review.to_owned())
}

fn enrich_runtime_review_workflow_graph(base_review: &str, run_plan_json: &str) -> String {
    let Ok(mut review) = serde_json::from_str::<Value>(base_review) else {
        return base_review.to_owned();
    };
    let Ok(run_plan) = serde_json::from_str::<Value>(run_plan_json) else {
        return base_review.to_owned();
    };
    let Some(graph) = run_plan.get("graph").and_then(Value::as_object) else {
        return base_review.to_owned();
    };
    let nodes = graph
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let edges = graph
        .get("edges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let risk_by_node = nodes
        .iter()
        .map(|node| {
            json!({
                "id": node.get("id").cloned().unwrap_or(Value::Null),
                "kind": node.get("kind").cloned().unwrap_or(Value::Null),
                "label": node.get("label").cloned().unwrap_or(Value::Null),
                "risk": node.get("risk").cloned().unwrap_or(Value::Null),
                "risk_category": node.get("risk_category").cloned().unwrap_or(Value::Null),
                "risk_severity": node.get("risk_severity").cloned().unwrap_or(Value::Null),
                "status": node.get("status").cloned().unwrap_or(Value::Null),
                "line": node.get("line").cloned().unwrap_or(Value::Null),
                "source_span": node.get("source_span").cloned().unwrap_or(Value::Null)
            })
        })
        .collect::<Vec<_>>();
    let workflow_graph = json!({
        "format": "eng-workflow-graph-review-v1",
        "source": "run_plan",
        "node_count": graph
            .get("node_count")
            .cloned()
            .unwrap_or_else(|| json!(nodes.len())),
        "edge_count": graph
            .get("edge_count")
            .cloned()
            .unwrap_or_else(|| json!(edges.len())),
        "nodes": nodes,
        "edges": edges,
        "risk_by_node": risk_by_node
    });
    let Some(object) = review.as_object_mut() else {
        return base_review.to_owned();
    };
    object.insert("workflow_graph".to_owned(), workflow_graph);

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
    canonical_path_text(&strip_runtime_string_value(path))
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
    cache_records: &[CacheManifestRecord],
) -> String {
    let case_manifests = materialized_case_manifests(runtime_data, process_results, cache_records);
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
        json.push_str(&format!("        \"line\": {},\n", manifest.line));
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

fn case_tables_json(
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
    cache_records: &[CacheManifestRecord],
) -> String {
    let case_tables = materialized_case_tables(runtime_data, process_results, cache_records);
    let mut json = String::new();
    for (index, table) in case_tables.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"sample_table\": \"{}\",\n",
            json_escape(&table.sample_table)
        ));
        json.push_str(&format!(
            "        \"schema_name\": \"{}\",\n",
            json_escape(&table.schema_name)
        ));
        json.push_str(&format!(
            "        \"source\": \"{}\",\n",
            json_escape(&table.source)
        ));
        push_optional_json_string(&mut json, "source_hash", table.source_hash.as_deref(), 8);
        json.push_str(&format!("        \"case_count\": {},\n", table.case_count));
        json.push_str(&format!(
            "        \"pending_count\": {},\n",
            table.pending_count
        ));
        json.push_str(&format!(
            "        \"running_count\": {},\n",
            table.running_count
        ));
        json.push_str(&format!(
            "        \"succeeded_count\": {},\n",
            table.succeeded_count
        ));
        json.push_str(&format!(
            "        \"failed_count\": {},\n",
            table.failed_count
        ));
        json.push_str(&format!(
            "        \"skipped_count\": {},\n",
            table.skipped_count
        ));
        json.push_str("        \"duplicate_case_ids\": [");
        push_json_string_array(&mut json, &table.duplicate_case_ids);
        json.push_str("],\n");
        json.push_str(&format!(
            "        \"case_dir_count\": {},\n",
            table.case_dir_count
        ));
        json.push_str(&format!(
            "        \"generated_input_count\": {},\n",
            table.generated_input_count
        ));
        json.push_str(&format!(
            "        \"output_artifact_count\": {},\n",
            table.output_artifact_count
        ));
        json.push_str(&format!(
            "        \"result_file_count\": {},\n",
            table.result_file_count
        ));
        json.push_str(&format!(
            "        \"metric_count\": {},\n",
            table.metric_count
        ));
        push_optional_json_string(
            &mut json,
            "collection_manifest",
            table.collection_manifest.as_deref(),
            8,
        );
        push_optional_json_string(
            &mut json,
            "collection_status",
            table.collection_status.as_deref(),
            8,
        );
        json.push_str(&format!(
            "        \"collected_case_count\": {},\n",
            table.collected_case_count
        ));
        json.push_str(&format!(
            "        \"missing_case_count\": {},\n",
            table.missing_case_count
        ));
        json.push_str(&format!(
            "        \"failed_case_count\": {},\n",
            table.failed_case_count
        ));
        json.push_str(&format!(
            "        \"runner\": \"{}\",\n",
            json_escape(&table.runner)
        ));
        json.push_str(&format!(
            "        \"scheduler\": \"{}\",\n",
            json_escape(&table.scheduler)
        ));
        json.push_str("        \"scheduler_hooks\": [");
        push_json_string_array(&mut json, &table.scheduler_hooks);
        json.push_str("],\n");
        json.push_str(&format!(
            "        \"parallel_policy\": \"{}\",\n",
            json_escape(&table.parallel_policy)
        ));
        json.push_str(&format!(
            "        \"resume_policy\": \"{}\",\n",
            json_escape(&table.resume_policy)
        ));
        json.push_str(&format!(
            "        \"cache_hit_count\": {},\n",
            table.cache_hit_count
        ));
        json.push_str(&format!(
            "        \"cache_miss_count\": {},\n",
            table.cache_miss_count
        ));
        json.push_str(&format!("        \"line\": {},\n", table.line));
        json.push_str(&format!(
            "        \"status\": \"{}\"\n",
            json_escape(&table.status)
        ));
        json.push_str("      }");
    }
    json
}

fn case_diagnostics_json(
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
    cache_records: &[CacheManifestRecord],
) -> String {
    let diagnostics = materialized_case_diagnostics(runtime_data, process_results, cache_records);
    let mut json = String::new();
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"severity\": \"{}\",\n",
            json_escape(&diagnostic.severity)
        ));
        json.push_str(&format!(
            "        \"code\": \"{}\",\n",
            json_escape(&diagnostic.code)
        ));
        json.push_str(&format!(
            "        \"message\": \"{}\",\n",
            json_escape(&diagnostic.message)
        ));
        push_optional_json_string(&mut json, "case_id", diagnostic.case_id.as_deref(), 8);
        json.push_str(&format!(
            "        \"sample_table\": \"{}\",\n",
            json_escape(&diagnostic.sample_table)
        ));
        json.push_str(&format!("        \"line\": {}\n", diagnostic.line));
        json.push_str("      }");
    }
    json
}

fn materialized_case_tables(
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
    cache_records: &[CacheManifestRecord],
) -> Vec<RuntimeCaseTable> {
    let manifests = materialized_case_manifests(runtime_data, process_results, cache_records);
    let collections = case_collection_records(process_results);
    let mut groups: Vec<(&RuntimeCaseManifest, Vec<&RuntimeCaseManifest>)> = Vec::new();
    for manifest in &manifests {
        if let Some((_, group)) = groups
            .iter_mut()
            .find(|(first, _)| first.sample_table == manifest.sample_table)
        {
            group.push(manifest);
        } else {
            groups.push((manifest, vec![manifest]));
        }
    }

    groups
        .into_iter()
        .map(|(first, group)| {
            let case_ids = group
                .iter()
                .filter(|manifest| !manifest.case_id.trim().is_empty())
                .map(|manifest| manifest.case_id.clone())
                .collect::<HashSet<_>>();
            let (cache_hit_count, cache_miss_count) =
                case_cache_counts_for_cases(&case_ids, cache_records);
            let collection = matching_case_collection(&case_ids, &collections);
            let duplicate_case_ids = duplicate_case_ids_for_manifests(&group);
            let pending_count = group
                .iter()
                .filter(|manifest| manifest.status == "pending")
                .count();
            let running_count = group
                .iter()
                .filter(|manifest| manifest.status == "running")
                .count();
            let succeeded_count = group
                .iter()
                .filter(|manifest| manifest.status == "succeeded")
                .count();
            let failed_count = group
                .iter()
                .filter(|manifest| manifest.status == "failed")
                .count();
            let skipped_count = group
                .iter()
                .filter(|manifest| manifest.status == "skipped")
                .count();
            let case_dir_count = group
                .iter()
                .filter_map(|manifest| manifest.case_dir.clone())
                .collect::<HashSet<_>>()
                .len();
            let generated_input_count = group
                .iter()
                .filter(|manifest| manifest.generated_input_file.is_some())
                .count();
            let output_artifact_count = group
                .iter()
                .map(|manifest| manifest.output_artifacts.len())
                .sum::<usize>();
            let result_file_count = group
                .iter()
                .map(|manifest| manifest.result_files.len())
                .sum::<usize>();
            let metric_count = group
                .iter()
                .map(|manifest| manifest.metrics.len())
                .sum::<usize>();
            let collection_failed_count = collection
                .map(|collection| collection.failed_case_count)
                .unwrap_or(0);
            let has_process = group
                .iter()
                .any(|manifest| !manifest.process_bindings.is_empty());
            let status = if failed_count > 0 || collection_failed_count > 0 {
                "failed"
            } else if running_count > 0 {
                "running"
            } else if pending_count > 0 {
                "pending"
            } else if skipped_count == group.len() && !group.is_empty() {
                "skipped"
            } else {
                "succeeded"
            };
            RuntimeCaseTable {
                sample_table: first.sample_table.clone(),
                schema_name: first.schema_name.clone(),
                source: first.source.clone(),
                source_hash: first.source_hash.clone(),
                case_count: group.len(),
                pending_count,
                running_count,
                succeeded_count,
                failed_count,
                skipped_count,
                duplicate_case_ids,
                case_dir_count,
                generated_input_count,
                output_artifact_count,
                result_file_count,
                metric_count,
                collection_manifest: collection.map(|collection| collection.manifest_path.clone()),
                collection_status: collection.map(|collection| collection.status.clone()),
                collected_case_count: collection
                    .map(|collection| collection.result_case_ids.len())
                    .unwrap_or(0),
                missing_case_count: collection
                    .map(|collection| collection.missing_case_ids.len())
                    .unwrap_or(0),
                failed_case_count: collection_failed_count,
                runner: if has_process {
                    "sequential_process_runner".to_owned()
                } else {
                    "manifest_seed_runner".to_owned()
                },
                scheduler: "sequential".to_owned(),
                scheduler_hooks: vec![
                    "case_id".to_owned(),
                    "case_dir".to_owned(),
                    "sample_row_hash".to_owned(),
                    "case_cache_key".to_owned(),
                ],
                parallel_policy: "not_configured".to_owned(),
                resume_policy: if cache_hit_count + cache_miss_count > 0 {
                    "case_cache_key".to_owned()
                } else {
                    "sample_row_hash".to_owned()
                },
                cache_hit_count,
                cache_miss_count,
                status: status.to_owned(),
                line: first.line,
            }
        })
        .collect()
}

fn materialized_case_diagnostics(
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
    cache_records: &[CacheManifestRecord],
) -> Vec<RuntimeCaseDiagnostic> {
    let manifests = materialized_case_manifests(runtime_data, process_results, cache_records);
    let mut diagnostics = Vec::new();
    push_duplicate_case_diagnostics(&mut diagnostics, &manifests);
    push_case_dir_collision_diagnostics(&mut diagnostics, &manifests);
    for manifest in &manifests {
        for process in &manifest.process_statuses {
            if !case_process_status_is_success(&process.status) {
                diagnostics.push(RuntimeCaseDiagnostic {
                    severity: "error".to_owned(),
                    code: "E-CASE-STEP-FAILED".to_owned(),
                    message: format!(
                        "case `{}` step `{}` reported status `{}`",
                        manifest.case_id, process.name, process.status
                    ),
                    case_id: Some(manifest.case_id.clone()),
                    sample_table: manifest.sample_table.clone(),
                    line: manifest.line,
                });
            }
        }
        if manifest
            .failure_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("did not create expected output"))
        {
            diagnostics.push(RuntimeCaseDiagnostic {
                severity: "error".to_owned(),
                code: "E-CASE-OUTPUT-MISSING".to_owned(),
                message: manifest.failure_reason.clone().unwrap_or_default(),
                case_id: Some(manifest.case_id.clone()),
                sample_table: manifest.sample_table.clone(),
                line: manifest.line,
            });
        }
        if manifest.status == "skipped" {
            diagnostics.push(RuntimeCaseDiagnostic {
                severity: "warning".to_owned(),
                code: "W-CASE-SKIPPED-CACHE".to_owned(),
                message: format!("case `{}` was skipped from case cache", manifest.case_id),
                case_id: Some(manifest.case_id.clone()),
                sample_table: manifest.sample_table.clone(),
                line: manifest.line,
            });
        }
    }
    diagnostics
}

fn materialized_case_manifests(
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
    cache_records: &[CacheManifestRecord],
) -> Vec<RuntimeCaseManifest> {
    let mut manifests = runtime_data.case_manifests.clone();
    if manifests.is_empty() {
        return manifests;
    }

    if !process_results.is_empty() {
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
    }

    apply_case_cache_statuses(&mut manifests, cache_records);
    for manifest in &mut manifests {
        finalize_case_manifest_status(manifest);
    }

    manifests
}

fn apply_case_cache_statuses(
    manifests: &mut [RuntimeCaseManifest],
    cache_records: &[CacheManifestRecord],
) {
    for record in cache_records
        .iter()
        .filter(|record| record.owner_kind == "case" && record.lookup_status == "hit")
    {
        for manifest in manifests
            .iter_mut()
            .filter(|manifest| manifest.case_id == record.owner_name)
        {
            if manifest.failure_reason.is_none() {
                manifest.status = "skipped".to_owned();
            }
        }
    }
}

fn case_cache_counts_for_cases(
    case_ids: &HashSet<String>,
    cache_records: &[CacheManifestRecord],
) -> (usize, usize) {
    let mut hit_count = 0usize;
    let mut miss_count = 0usize;
    for record in cache_records
        .iter()
        .filter(|record| record.owner_kind == "case" && case_ids.contains(&record.owner_name))
    {
        if record.lookup_status == "hit" {
            hit_count += 1;
        } else {
            miss_count += 1;
        }
    }
    (hit_count, miss_count)
}

fn matching_case_collection<'a>(
    case_ids: &HashSet<String>,
    collections: &'a [CaseCollectionRecord],
) -> Option<&'a CaseCollectionRecord> {
    collections.iter().find(|collection| {
        collection
            .expected_case_ids
            .iter()
            .chain(collection.result_case_ids.iter())
            .any(|case_id| case_ids.contains(case_id))
    })
}

fn duplicate_case_ids_for_manifests(manifests: &[&RuntimeCaseManifest]) -> Vec<String> {
    let mut counts = HashMap::new();
    for manifest in manifests {
        if manifest.case_id.trim().is_empty() {
            continue;
        }
        *counts.entry(manifest.case_id.clone()).or_insert(0usize) += 1;
    }
    let mut duplicates = counts
        .into_iter()
        .filter_map(|(case_id, count)| (count > 1).then_some(case_id))
        .collect::<Vec<_>>();
    duplicates.sort();
    duplicates
}

fn push_duplicate_case_diagnostics(
    diagnostics: &mut Vec<RuntimeCaseDiagnostic>,
    manifests: &[RuntimeCaseManifest],
) {
    let mut groups: Vec<(&RuntimeCaseManifest, Vec<&RuntimeCaseManifest>)> = Vec::new();
    for manifest in manifests {
        if let Some((_, group)) = groups
            .iter_mut()
            .find(|(first, _)| first.sample_table == manifest.sample_table)
        {
            group.push(manifest);
        } else {
            groups.push((manifest, vec![manifest]));
        }
    }
    for (first, group) in groups {
        for case_id in duplicate_case_ids_for_manifests(&group) {
            diagnostics.push(RuntimeCaseDiagnostic {
                severity: "error".to_owned(),
                code: "E-CASE-ID-DUPLICATE".to_owned(),
                message: format!(
                    "sample table `{}` contains duplicate case_id `{case_id}`",
                    first.sample_table
                ),
                case_id: Some(case_id),
                sample_table: first.sample_table.clone(),
                line: first.line,
            });
        }
    }
}

fn push_case_dir_collision_diagnostics(
    diagnostics: &mut Vec<RuntimeCaseDiagnostic>,
    manifests: &[RuntimeCaseManifest],
) {
    let mut by_dir: HashMap<(String, String), HashSet<String>> = HashMap::new();
    for manifest in manifests {
        let Some(case_dir) = &manifest.case_dir else {
            continue;
        };
        if manifest.case_id.trim().is_empty() {
            continue;
        }
        by_dir
            .entry((manifest.sample_table.clone(), case_dir.clone()))
            .or_default()
            .insert(manifest.case_id.clone());
    }
    let mut collisions = by_dir
        .into_iter()
        .filter_map(|((sample_table, case_dir), case_ids)| {
            (case_ids.len() > 1).then_some((sample_table, case_dir, case_ids))
        })
        .collect::<Vec<_>>();
    collisions.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    for (sample_table, case_dir, case_ids) in collisions {
        let mut case_ids = case_ids.into_iter().collect::<Vec<_>>();
        case_ids.sort();
        let line = manifests
            .iter()
            .find(|manifest| manifest.sample_table == sample_table)
            .map(|manifest| manifest.line)
            .unwrap_or(0);
        diagnostics.push(RuntimeCaseDiagnostic {
            severity: "error".to_owned(),
            code: "E-CASE-DIR-COLLISION".to_owned(),
            message: format!(
                "case directory `{case_dir}` is shared by case IDs {}",
                case_ids.join(", ")
            ),
            case_id: None,
            sample_table,
            line,
        });
    }
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
    if let Some(case_dir) = infer_case_dir_from_output_path(&output.path, &manifest.case_id) {
        manifest.case_dir = Some(case_dir);
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
        manifest.status = "failed".to_owned();
    } else if manifest.status == "skipped" {
        return;
    } else if manifest.generated_input_file.is_some()
        || !manifest.process_bindings.is_empty()
        || !manifest.process_statuses.is_empty()
        || !manifest.output_artifacts.is_empty()
        || !manifest.result_files.is_empty()
        || !manifest.metrics.is_empty()
    {
        manifest.status = "succeeded".to_owned();
    } else {
        manifest.status = "pending".to_owned();
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

fn model_cards_json(
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
) -> String {
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
    for record in external_model_card_records(process_results) {
        if emitted > 0 {
            json.push_str(",\n");
        }
        emitted += 1;
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&record.binding)
        ));
        push_optional_json_string(&mut json, "source", Some(&record.path), 8);
        json.push_str(&format!(
            "        \"model_kind\": \"{}\",\n",
            json_escape(
                &json_field_string(&record.value, "model_kind")
                    .or_else(|| json_field_string(&record.value, "model"))
                    .unwrap_or_else(|| "external_model_artifact".to_owned())
            )
        ));
        json.push_str("        \"features\": [");
        push_json_string_array(
            &mut json,
            &json_field_string_array(&record.value, "features"),
        );
        json.push_str("],\n");
        push_optional_json_string(
            &mut json,
            "target",
            json_field_string(&record.value, "target").as_deref(),
            8,
        );
        push_optional_json_string(
            &mut json,
            "target_quantity",
            json_field_string(&record.value, "target_quantity").as_deref(),
            8,
        );
        json.push_str(&format!(
            "        \"target_unit\": \"{}\",\n",
            json_escape(
                &json_field_string(&record.value, "target_unit").unwrap_or_else(|| "1".to_owned())
            )
        ));
        push_optional_json_string(
            &mut json,
            "test_fraction",
            external_model_split_fraction(&record.value).as_deref(),
            8,
        );
        push_optional_json_usize(
            &mut json,
            "train_count",
            external_model_train_count(&record.value),
            8,
        );
        push_optional_json_usize(
            &mut json,
            "test_count",
            external_model_test_count(&record.value),
            8,
        );
        json.push_str("        \"metrics\": {\n");
        json.push_str(&format!(
            "          \"rmse\": {},\n",
            optional_json_number(json_number_path(&record.value, &["metrics", "rmse"]))
        ));
        json.push_str(&format!(
            "          \"mae\": {},\n",
            optional_json_number(json_number_path(&record.value, &["metrics", "mae"]))
        ));
        json.push_str(&format!(
            "          \"r2\": {}\n",
            optional_json_number(json_number_path(&record.value, &["metrics", "r2"]))
        ));
        json.push_str("        },\n");
        push_optional_json_string(&mut json, "residual_plot", None, 8);
        json.push_str("        \"residual_point_count\": 0,\n");
        push_optional_json_string(
            &mut json,
            "training_data_hash",
            json_field_string(&record.value, "training_data_hash").as_deref(),
            8,
        );
        push_optional_json_string(
            &mut json,
            "model_artifact_hash",
            json_field_string(&record.value, "model_artifact_hash").as_deref(),
            8,
        );
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(
                &json_field_string(&record.value, "status")
                    .unwrap_or_else(|| "external_model_card".to_owned())
            )
        ));
        json.push_str(&format!("        \"line\": {}\n", record.line));
        json.push_str("      }");
    }
    json
}

fn external_model_card_records(
    process_results: &[ProcessExecutionRecord],
) -> Vec<ExternalModelJsonRecord> {
    external_json_records(process_results, is_model_card_output)
}

fn external_prediction_manifest_records(
    process_results: &[ProcessExecutionRecord],
) -> Vec<ExternalModelJsonRecord> {
    external_json_records(process_results, is_prediction_manifest_output)
}

fn external_json_records(
    process_results: &[ProcessExecutionRecord],
    predicate: fn(&ProcessExpectedOutputRecord) -> bool,
) -> Vec<ExternalModelJsonRecord> {
    let mut records = Vec::new();
    for process in process_results {
        for output in process
            .expected_outputs
            .iter()
            .filter(|output| output.exists && predicate(output))
        {
            let Ok(source) = fs::read_to_string(&output.resolved_path) else {
                continue;
            };
            let Ok(value) = serde_json::from_str::<Value>(&source) else {
                continue;
            };
            records.push(ExternalModelJsonRecord {
                binding: process.binding.clone(),
                path: output.path.clone(),
                value,
                line: process.line,
            });
        }
    }
    records
}

fn is_model_card_output(output: &ProcessExpectedOutputRecord) -> bool {
    let path = output.path.to_ascii_lowercase();
    path.ends_with("model_card.json")
        || path.contains("model-card")
        || (output.artifact_kind == "model_artifact" && path.contains("card"))
}

fn is_prediction_manifest_output(output: &ProcessExpectedOutputRecord) -> bool {
    let path = output.path.to_ascii_lowercase();
    path.ends_with("prediction_manifest.json")
        || path.contains("prediction-manifest")
        || (output.artifact_kind == "prediction_result" && path.contains("manifest"))
}

fn json_number_path(value: &Value, path: &[&str]) -> Option<f64> {
    let mut cursor = value;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor.as_f64()
}

fn external_model_split_fraction(value: &Value) -> Option<String> {
    json_number_path(value, &["train_test_split", "test_fraction"]).map(|fraction| {
        if fraction.fract() == 0.0 {
            format!("{fraction:.0}")
        } else {
            fraction.to_string()
        }
    })
}

fn external_model_train_count(value: &Value) -> Option<usize> {
    value
        .pointer("/train_test_split/train_rows")
        .and_then(Value::as_u64)
        .or_else(|| value.get("training_rows").and_then(Value::as_u64))
        .map(|count| count as usize)
}

fn external_model_test_count(value: &Value) -> Option<usize> {
    value
        .pointer("/train_test_split/test_rows")
        .and_then(Value::as_u64)
        .map(|count| count as usize)
}

fn optional_json_string_literal(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| "null".to_owned())
}

fn push_manifest_file_json(json: &mut String, key: &str, value: Option<&Value>, indent: usize) {
    let indent_text = " ".repeat(indent);
    json.push_str(&format!("{indent_text}\"{key}\": "));
    if let Some(value) = value {
        json.push_str("{");
        json.push_str(&format!(
            "\"path\": {}, ",
            optional_json_string_literal(json_field_string(value, "path").as_deref())
        ));
        json.push_str(&format!(
            "\"sha256\": {}, ",
            optional_json_string_literal(json_field_string(value, "sha256").as_deref())
        ));
        json.push_str(&format!(
            "\"bytes\": {}",
            value.get("bytes").and_then(Value::as_u64).unwrap_or(0)
        ));
        json.push_str("},\n");
    } else {
        json.push_str("null,\n");
    }
}

fn push_prediction_outputs_json(json: &mut String, value: &Value) {
    let Some(outputs) = value.get("outputs").and_then(Value::as_array) else {
        return;
    };
    for (index, output) in outputs.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str("{");
        json.push_str(&format!(
            "\"column\": {}, ",
            optional_json_string_literal(json_field_string(output, "column").as_deref())
        ));
        json.push_str(&format!(
            "\"quantity\": {}, ",
            optional_json_string_literal(json_field_string(output, "quantity").as_deref())
        ));
        json.push_str(&format!(
            "\"unit\": {}",
            optional_json_string_literal(json_field_string(output, "unit").as_deref())
        ));
        json.push_str("}");
    }
}

fn prediction_confidence_column(value: &Value) -> Option<String> {
    value
        .get("outputs")
        .and_then(Value::as_array)
        .and_then(|outputs| {
            outputs.iter().find_map(|output| {
                let column = json_field_string(output, "column")?;
                column
                    .to_ascii_lowercase()
                    .contains("confidence")
                    .then_some(column)
            })
        })
        .or_else(|| {
            json_field_string_array(value, "schema")
                .into_iter()
                .find(|column| column.to_ascii_lowercase().contains("confidence"))
        })
}

fn model_specs_json(
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
) -> String {
    let mut json = String::new();
    let mut emitted = 0usize;
    for artifact in runtime_data.ml_artifacts.iter().filter(|artifact| {
        matches!(
            artifact.kind.as_str(),
            "RegressionModel" | "MlpModel" | "ModelCard"
        )
    }) {
        if emitted > 0 {
            json.push_str(",\n");
        }
        emitted += 1;
        push_model_spec_json(
            &mut json,
            &artifact.binding,
            artifact.source.as_deref(),
            artifact.algorithm.as_deref().unwrap_or(&artifact.kind),
            &artifact.features,
            artifact.target.as_deref(),
            artifact.target_quantity.as_deref(),
            Some(&artifact.display_unit),
            artifact.test_fraction.as_deref(),
            artifact.seed.as_deref(),
            artifact.train_count,
            artifact.test_count,
            artifact.training_data_hash.as_deref(),
            artifact.model_artifact_hash.as_deref(),
            &artifact.status,
            artifact.line,
        );
    }
    for record in external_model_card_records(process_results) {
        if emitted > 0 {
            json.push_str(",\n");
        }
        emitted += 1;
        let features = json_field_string_array(&record.value, "features");
        let model_kind = json_field_string(&record.value, "model_kind")
            .or_else(|| json_field_string(&record.value, "model"))
            .unwrap_or_else(|| "external_model_artifact".to_owned());
        let split_fraction = external_model_split_fraction(&record.value);
        push_model_spec_json(
            &mut json,
            &record.binding,
            Some(&record.path),
            &model_kind,
            &features,
            json_field_string(&record.value, "target").as_deref(),
            json_field_string(&record.value, "target_quantity").as_deref(),
            json_field_string(&record.value, "target_unit").as_deref(),
            split_fraction.as_deref(),
            json_number_path(&record.value, &["train_test_split", "seed"])
                .map(|seed| seed.to_string())
                .as_deref(),
            external_model_train_count(&record.value),
            external_model_test_count(&record.value),
            json_field_string(&record.value, "training_data_hash").as_deref(),
            json_field_string(&record.value, "model_artifact_hash").as_deref(),
            &json_field_string(&record.value, "status")
                .unwrap_or_else(|| "external_model_card".to_owned()),
            record.line,
        );
    }
    json
}

#[allow(clippy::too_many_arguments)]
fn push_model_spec_json(
    json: &mut String,
    binding: &str,
    source: Option<&str>,
    model_kind: &str,
    features: &[String],
    target: Option<&str>,
    target_quantity: Option<&str>,
    target_unit: Option<&str>,
    test_fraction: Option<&str>,
    seed: Option<&str>,
    train_count: Option<usize>,
    test_count: Option<usize>,
    training_data_hash: Option<&str>,
    model_artifact_hash: Option<&str>,
    status: &str,
    line: usize,
) {
    json.push_str("      {\n");
    json.push_str(&format!(
        "        \"binding\": \"{}\",\n",
        json_escape(binding)
    ));
    push_optional_json_string(json, "source", source, 8);
    json.push_str(&format!(
        "        \"model_kind\": \"{}\",\n",
        json_escape(model_kind)
    ));
    json.push_str("        \"features\": [");
    for (index, feature) in features.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!(
            "{{\"name\": \"{}\", \"quantity\": null, \"unit\": null}}",
            json_escape(feature)
        ));
    }
    json.push_str("],\n");
    json.push_str("        \"target\": {");
    json.push_str(&format!(
        "\"name\": {}, ",
        optional_json_string_literal(target)
    ));
    json.push_str(&format!(
        "\"quantity\": {}, ",
        optional_json_string_literal(target_quantity)
    ));
    json.push_str(&format!(
        "\"unit\": {}",
        optional_json_string_literal(target_unit)
    ));
    json.push_str("},\n");
    push_optional_json_string(json, "test_fraction", test_fraction, 8);
    push_optional_json_string(json, "seed", seed, 8);
    push_optional_json_usize(json, "train_count", train_count, 8);
    push_optional_json_usize(json, "test_count", test_count, 8);
    push_optional_json_string(json, "training_data_hash", training_data_hash, 8);
    push_optional_json_string(json, "model_artifact_hash", model_artifact_hash, 8);
    json.push_str(&format!(
        "        \"status\": \"{}\",\n",
        json_escape(status)
    ));
    json.push_str(&format!("        \"line\": {}\n", line));
    json.push_str("      }");
}

fn prediction_manifests_json(
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
) -> String {
    let mut json = String::new();
    let mut emitted = 0usize;
    for artifact in runtime_data
        .ml_artifacts
        .iter()
        .filter(|artifact| artifact.kind == "PredictionResult")
    {
        let Some(table) = runtime_data
            .tables
            .iter()
            .find(|table| table.binding == artifact.binding)
        else {
            continue;
        };
        if emitted > 0 {
            json.push_str(",\n");
        }
        emitted += 1;
        push_native_prediction_manifest_json(&mut json, artifact, table);
    }
    for record in external_prediction_manifest_records(process_results) {
        if emitted > 0 {
            json.push_str(",\n");
        }
        emitted += 1;
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&record.binding)
        ));
        json.push_str(&format!(
            "        \"manifest_path\": \"{}\",\n",
            json_escape(&record.path)
        ));
        json.push_str(&format!(
            "        \"model\": \"{}\",\n",
            json_escape(
                &json_field_string(&record.value, "model").unwrap_or_else(|| "unknown".to_owned())
            )
        ));
        push_manifest_file_json(&mut json, "model_file", record.value.get("model_file"), 8);
        push_manifest_file_json(&mut json, "sample_file", record.value.get("sample_file"), 8);
        push_manifest_file_json(&mut json, "output_file", record.value.get("output_file"), 8);
        json.push_str("        \"schema\": [");
        push_json_string_array(&mut json, &json_field_string_array(&record.value, "schema"));
        json.push_str("],\n");
        json.push_str("        \"outputs\": [");
        push_prediction_outputs_json(&mut json, &record.value);
        json.push_str("],\n");
        json.push_str("        \"case_ids\": [");
        push_json_string_array(
            &mut json,
            &json_field_string_array(&record.value, "case_ids"),
        );
        json.push_str("],\n");
        json.push_str(&format!(
            "        \"row_count\": {},\n",
            record
                .value
                .get("row_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ));
        push_optional_json_string(
            &mut json,
            "confidence_column",
            prediction_confidence_column(&record.value).as_deref(),
            8,
        );
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(
                &json_field_string(&record.value, "status").unwrap_or_else(|| "loaded".to_owned())
            )
        ));
        json.push_str(&format!("        \"line\": {}\n", record.line));
        json.push_str("      }");
    }
    json
}

fn push_native_prediction_manifest_json(
    json: &mut String,
    artifact: &RuntimeMlArtifact,
    table: &runtime_data::RuntimeTable,
) {
    json.push_str("      {\n");
    json.push_str(&format!(
        "        \"binding\": \"{}\",\n",
        json_escape(&artifact.binding)
    ));
    json.push_str(&format!(
        "        \"manifest_path\": \"native:{}\",\n",
        json_escape(&artifact.binding)
    ));
    json.push_str(&format!(
        "        \"model\": \"{}\",\n",
        json_escape(artifact.source.as_deref().unwrap_or("unknown"))
    ));
    push_manifest_file_json(json, "model_file", None, 8);
    push_manifest_file_json(json, "sample_file", None, 8);
    push_manifest_file_json(json, "output_file", None, 8);
    json.push_str("        \"schema\": [");
    push_json_string_array(
        json,
        &table
            .columns
            .iter()
            .map(|column| column.name.clone())
            .collect::<Vec<_>>(),
    );
    json.push_str("],\n");
    json.push_str("        \"outputs\": [");
    push_native_prediction_outputs_json(json, table);
    json.push_str("],\n");
    json.push_str("        \"case_ids\": [");
    push_json_string_array(json, &native_prediction_case_ids(table));
    json.push_str("],\n");
    json.push_str(&format!("        \"row_count\": {},\n", table.row_count));
    push_optional_json_string(
        json,
        "confidence_column",
        native_prediction_confidence_column(table).as_deref(),
        8,
    );
    json.push_str(&format!(
        "        \"status\": \"{}\",\n",
        json_escape(&artifact.status)
    ));
    json.push_str(&format!("        \"line\": {}\n", artifact.line));
    json.push_str("      }");
}

fn push_native_prediction_outputs_json(json: &mut String, table: &runtime_data::RuntimeTable) {
    let output_columns = table
        .columns
        .iter()
        .filter(|column| !column.name.eq_ignore_ascii_case("case_id"))
        .collect::<Vec<_>>();
    for (index, column) in output_columns.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str("{");
        json.push_str(&format!(
            "\"column\": {}, ",
            optional_json_string_literal(Some(column.name.as_str()))
        ));
        json.push_str(&format!(
            "\"quantity\": {}, ",
            optional_json_string_literal(Some(column.type_name.as_str()))
        ));
        json.push_str(&format!(
            "\"unit\": {}",
            optional_json_string_literal(column.unit.as_deref())
        ));
        json.push_str("}");
    }
}

fn native_prediction_case_ids(table: &runtime_data::RuntimeTable) -> Vec<String> {
    let Some(column) = table
        .columns
        .iter()
        .find(|column| column.name.eq_ignore_ascii_case("case_id"))
    else {
        return Vec::new();
    };
    match &column.values {
        RuntimeValues::Text(values) => values.clone(),
        RuntimeValues::Number(values) => values
            .iter()
            .map(|value| value.map(|value| value.to_string()).unwrap_or_default())
            .collect(),
    }
}

fn native_prediction_confidence_column(table: &runtime_data::RuntimeTable) -> Option<String> {
    table.columns.iter().find_map(|column| {
        column
            .name
            .to_ascii_lowercase()
            .contains("confidence")
            .then(|| column.name.clone())
    })
}

fn model_diagnostics_json(
    runtime_data: &RuntimeData,
    process_results: &[ProcessExecutionRecord],
) -> String {
    let mut diagnostics = Vec::new();
    for artifact in &runtime_data.ml_artifacts {
        if matches!(artifact.kind.as_str(), "RegressionModel" | "MlpModel") {
            if artifact.features.is_empty() {
                diagnostics.push((
                    "error",
                    "E-MODEL-FEATURE-MISSING",
                    format!("model `{}` has no feature specification", artifact.binding),
                    artifact.binding.clone(),
                    artifact.line,
                ));
            }
            if artifact.target.is_none() {
                diagnostics.push((
                    "error",
                    "E-MODEL-TARGET-MISSING",
                    format!("model `{}` has no target specification", artifact.binding),
                    artifact.binding.clone(),
                    artifact.line,
                ));
            }
        }
    }
    for record in external_model_card_records(process_results) {
        if json_field_string_array(&record.value, "features").is_empty() {
            diagnostics.push((
                "error",
                "E-MODEL-FEATURE-MISSING",
                format!("model card `{}` has no features", record.path),
                record.binding.clone(),
                record.line,
            ));
        }
        if json_field_string(&record.value, "target").is_none() {
            diagnostics.push((
                "error",
                "E-MODEL-TARGET-MISSING",
                format!("model card `{}` has no target", record.path),
                record.binding.clone(),
                record.line,
            ));
        }
    }
    for record in external_prediction_manifest_records(process_results) {
        for message in json_field_string_array(&record.value, "schema_mismatch_diagnostics") {
            diagnostics.push((
                "warning",
                "W-MODEL-EXTRAPOLATION",
                message,
                record.binding.clone(),
                record.line,
            ));
        }
    }
    for process in process_results {
        let has_model_artifact = process
            .expected_outputs
            .iter()
            .any(|output| output.artifact_kind == "model_artifact");
        let has_model_card = process
            .expected_outputs
            .iter()
            .any(|output| is_model_card_output(output));
        if has_model_artifact && !has_model_card {
            diagnostics.push((
                "error",
                "E-MODEL-CARD-MISSING",
                format!(
                    "model artifact process `{}` does not declare model_card.json",
                    process.binding
                ),
                process.binding.clone(),
                process.line,
            ));
        }
    }
    let mut json = String::new();
    for (index, (severity, code, message, binding, line)) in diagnostics.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!("        \"severity\": \"{}\",\n", severity));
        json.push_str(&format!("        \"code\": \"{}\",\n", code));
        json.push_str(&format!(
            "        \"message\": \"{}\",\n",
            json_escape(message)
        ));
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(binding)
        ));
        json.push_str(&format!("        \"line\": {}\n", line));
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
            "database_hash_before",
            record.database_hash_before.as_deref(),
            8,
        );
        push_optional_json_string(
            &mut json,
            "database_hash_after",
            record.database_hash_after.as_deref(),
            8,
        );
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
        json.push_str("        \"diagnostics\": [\n");
        push_db_manifest_diagnostics_json(&mut json, &record.diagnostics, "          ");
        json.push_str("\n        ],\n");
        json.push_str(&format!(
            "        \"status\": \"{}\"\n",
            json_escape(&record.status)
        ));
        json.push_str("      }");
    }
    json
}

fn push_db_manifest_diagnostics_json(
    json: &mut String,
    diagnostics: &[DbManifestDiagnosticRecord],
    indent: &str,
) {
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{indent}{{\n"));
        json.push_str(&format!(
            "{indent}  \"code\": \"{}\",\n",
            json_escape(&diagnostic.code)
        ));
        json.push_str(&format!(
            "{indent}  \"message\": \"{}\",\n",
            json_escape(&diagnostic.message)
        ));
        push_optional_json_string(json, "table", diagnostic.table.as_deref(), indent.len() + 2);
        json.push_str(&format!("{indent}  \"status\": \"recorded\"\n"));
        json.push_str(&format!("{indent}}}"));
    }
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

fn table_transforms_json(runtime_data: &RuntimeData, indent: &str) -> String {
    let mut json = String::new();
    for (index, transform) in runtime_data.table_transforms.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        push_table_transform_json(&mut json, transform, indent);
    }
    json
}

fn push_table_transform_json(
    json: &mut String,
    transform: &runtime_data::RuntimeTableTransform,
    indent: &str,
) {
    let field_indent = format!("{indent}  ");
    let nested_indent = format!("{indent}    ");
    json.push_str(&format!("{indent}{{\n"));
    json.push_str(&format!(
        "{field_indent}\"binding\": \"{}\",\n",
        json_escape(&transform.binding)
    ));
    json.push_str(&format!(
        "{field_indent}\"operation\": \"{}\",\n",
        json_escape(&transform.operation)
    ));
    json.push_str(&format!(
        "{field_indent}\"source_table\": \"{}\",\n",
        json_escape(&transform.source_table)
    ));
    push_optional_json_string(
        json,
        "secondary_table",
        transform.secondary_table.as_deref(),
        field_indent.len(),
    );
    push_optional_json_string(
        json,
        "schema_name",
        transform.schema_name.as_deref(),
        field_indent.len(),
    );
    json.push_str(&format!(
        "{field_indent}\"input_row_count\": {},\n",
        transform.input_row_count
    ));
    match transform.secondary_input_row_count {
        Some(count) => json.push_str(&format!(
            "{field_indent}\"secondary_input_row_count\": {},\n",
            count
        )),
        None => json.push_str(&format!(
            "{field_indent}\"secondary_input_row_count\": null,\n"
        )),
    }
    json.push_str(&format!(
        "{field_indent}\"output_row_count\": {},\n",
        transform.output_row_count
    ));
    json.push_str(&format!("{field_indent}\"matched_row_indices\": ["));
    for (row_index, row) in transform.matched_row_indices.iter().enumerate() {
        if row_index > 0 {
            json.push_str(", ");
        }
        json.push_str(&row.to_string());
    }
    json.push_str("],\n");
    json.push_str(&format!("{field_indent}\"selected_columns\": [\n"));
    for (column_index, column) in transform.selected_columns.iter().enumerate() {
        if column_index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{nested_indent}{{\n"));
        json.push_str(&format!(
            "{nested_indent}  \"name\": \"{}\",\n",
            json_escape(&column.name)
        ));
        json.push_str(&format!(
            "{nested_indent}  \"status\": \"{}\",\n",
            json_escape(&column.status)
        ));
        json.push_str(&format!("{nested_indent}  \"line\": {}\n", column.line));
        json.push_str(&format!("{nested_indent}}}"));
    }
    json.push_str(&format!("\n{field_indent}],\n"));
    json.push_str(&format!("{field_indent}\"derived_columns\": [\n"));
    for (column_index, column) in transform.derived_columns.iter().enumerate() {
        if column_index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{nested_indent}{{\n"));
        json.push_str(&format!(
            "{nested_indent}  \"name\": \"{}\",\n",
            json_escape(&column.name)
        ));
        json.push_str(&format!(
            "{nested_indent}  \"expression\": \"{}\",\n",
            json_escape(&column.expression)
        ));
        json.push_str(&format!("{nested_indent}  \"source_columns\": ["));
        for (source_index, source_column) in column.source_columns.iter().enumerate() {
            if source_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(source_column)));
        }
        json.push_str("],\n");
        json.push_str(&format!(
            "{nested_indent}  \"status\": \"{}\",\n",
            json_escape(&column.status)
        ));
        json.push_str(&format!("{nested_indent}  \"line\": {}\n", column.line));
        json.push_str(&format!("{nested_indent}}}"));
    }
    json.push_str(&format!("\n{field_indent}],\n"));
    json.push_str(&format!("{field_indent}\"sort_keys\": [\n"));
    for (key_index, key) in transform.sort_keys.iter().enumerate() {
        if key_index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{nested_indent}{{\n"));
        json.push_str(&format!(
            "{nested_indent}  \"column\": \"{}\",\n",
            json_escape(&key.column)
        ));
        json.push_str(&format!(
            "{nested_indent}  \"direction\": \"{}\",\n",
            json_escape(&key.direction)
        ));
        json.push_str(&format!(
            "{nested_indent}  \"status\": \"{}\",\n",
            json_escape(&key.status)
        ));
        json.push_str(&format!("{nested_indent}  \"line\": {}\n", key.line));
        json.push_str(&format!("{nested_indent}}}"));
    }
    json.push_str(&format!("\n{field_indent}],\n"));
    json.push_str(&format!("{field_indent}\"predicates\": [\n"));
    for (predicate_index, predicate) in transform.predicates.iter().enumerate() {
        if predicate_index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{nested_indent}{{\n"));
        json.push_str(&format!(
            "{nested_indent}  \"expression\": \"{}\",\n",
            json_escape(&predicate.expression)
        ));
        push_optional_json_string(
            json,
            "column",
            predicate.column.as_deref(),
            nested_indent.len() + 2,
        );
        json.push_str(&format!(
            "{nested_indent}  \"operator\": \"{}\",\n",
            json_escape(&predicate.operator)
        ));
        push_optional_json_string(
            json,
            "value",
            predicate.value.as_deref(),
            nested_indent.len() + 2,
        );
        push_optional_json_string(
            json,
            "resolved_value",
            predicate.resolved_value.as_deref(),
            nested_indent.len() + 2,
        );
        json.push_str(&format!(
            "{nested_indent}  \"matched_count\": {},\n",
            predicate.matched_count
        ));
        json.push_str(&format!(
            "{nested_indent}  \"status\": \"{}\",\n",
            json_escape(&predicate.status)
        ));
        json.push_str(&format!("{nested_indent}  \"line\": {}\n", predicate.line));
        json.push_str(&format!("{nested_indent}}}"));
    }
    json.push_str(&format!("\n{field_indent}],\n"));
    json.push_str(&format!("{field_indent}\"join_keys\": [\n"));
    for (key_index, key) in transform.join_keys.iter().enumerate() {
        if key_index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{nested_indent}{{\n"));
        json.push_str(&format!(
            "{nested_indent}  \"expression\": \"{}\",\n",
            json_escape(&key.expression)
        ));
        json.push_str(&format!(
            "{nested_indent}  \"left_table\": \"{}\",\n",
            json_escape(&key.left_table)
        ));
        json.push_str(&format!(
            "{nested_indent}  \"left_column\": \"{}\",\n",
            json_escape(&key.left_column)
        ));
        json.push_str(&format!(
            "{nested_indent}  \"right_table\": \"{}\",\n",
            json_escape(&key.right_table)
        ));
        json.push_str(&format!(
            "{nested_indent}  \"right_column\": \"{}\",\n",
            json_escape(&key.right_column)
        ));
        json.push_str(&format!(
            "{nested_indent}  \"matched_pair_count\": {},\n",
            key.matched_pair_count
        ));
        json.push_str(&format!(
            "{nested_indent}  \"status\": \"{}\",\n",
            json_escape(&key.status)
        ));
        json.push_str(&format!("{nested_indent}  \"line\": {}\n", key.line));
        json.push_str(&format!("{nested_indent}}}"));
    }
    json.push_str(&format!("\n{field_indent}],\n"));
    json.push_str(&format!("{field_indent}\"row_diagnostics\": [\n"));
    for (row_index, row) in transform.row_diagnostics.iter().enumerate() {
        if row_index > 0 {
            json.push_str(",\n");
        }
        let row_field_indent = format!("{nested_indent}  ");
        let predicate_indent = format!("{nested_indent}    ");
        let predicate_field_indent = format!("{nested_indent}      ");
        json.push_str(&format!("{nested_indent}{{\n"));
        json.push_str(&format!(
            "{row_field_indent}\"row_index\": {},\n",
            row.row_index
        ));
        json.push_str(&format!("{row_field_indent}\"secondary_row_indices\": ["));
        for (secondary_index, secondary_row) in row.secondary_row_indices.iter().enumerate() {
            if secondary_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&secondary_row.to_string());
        }
        json.push_str("],\n");
        json.push_str(&format!(
            "{row_field_indent}\"status\": \"{}\",\n",
            json_escape(&row.status)
        ));
        json.push_str(&format!(
            "{row_field_indent}\"reason\": \"{}\",\n",
            json_escape(&row.reason)
        ));
        json.push_str(&format!("{row_field_indent}\"predicates\": [\n"));
        for (predicate_index, predicate) in row.predicates.iter().enumerate() {
            if predicate_index > 0 {
                json.push_str(",\n");
            }
            json.push_str(&format!("{predicate_indent}{{\n"));
            json.push_str(&format!(
                "{predicate_field_indent}\"expression\": \"{}\",\n",
                json_escape(&predicate.expression)
            ));
            push_optional_json_string(
                json,
                "column",
                predicate.column.as_deref(),
                predicate_field_indent.len(),
            );
            json.push_str(&format!(
                "{predicate_field_indent}\"operator\": \"{}\",\n",
                json_escape(&predicate.operator)
            ));
            push_optional_json_string(
                json,
                "expected",
                predicate.expected.as_deref(),
                predicate_field_indent.len(),
            );
            push_optional_json_string(
                json,
                "actual",
                predicate.actual.as_deref(),
                predicate_field_indent.len(),
            );
            json.push_str(&format!(
                "{predicate_field_indent}\"matched\": {},\n",
                predicate.matched
            ));
            json.push_str(&format!(
                "{predicate_field_indent}\"status\": \"{}\",\n",
                json_escape(&predicate.status)
            ));
            json.push_str(&format!(
                "{predicate_field_indent}\"line\": {}\n",
                predicate.line
            ));
            json.push_str(&format!("{predicate_indent}}}"));
        }
        json.push_str(&format!("\n{row_field_indent}]\n"));
        json.push_str(&format!("{nested_indent}}}"));
    }
    json.push_str(&format!("\n{field_indent}],\n"));
    json.push_str(&format!(
        "{field_indent}\"status\": \"{}\",\n",
        json_escape(&transform.status)
    ));
    json.push_str(&format!(
        "{field_indent}\"reason\": \"{}\",\n",
        json_escape(&transform.reason)
    ));
    json.push_str(&format!("{field_indent}\"line\": {}\n", transform.line));
    json.push_str(&format!("{indent}}}"));
}

fn quality_results_json(runtime_data: &RuntimeData, indent: &str) -> String {
    let mut json = String::new();
    for (index, quality) in runtime_data.quality_results.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        push_quality_result_json(&mut json, quality, indent);
    }
    json
}

fn expectation_suites_json(runtime_data: &RuntimeData, indent: &str) -> String {
    let mut json = String::new();
    for (index, suite) in runtime_data.expectation_suites.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        push_expectation_suite_json(&mut json, suite, indent);
    }
    json
}

fn push_expectation_suite_json(
    json: &mut String,
    suite: &runtime_data::RuntimeExpectationSuite,
    indent: &str,
) {
    let field_indent = format!("{indent}  ");
    let expectation_indent = format!("{indent}    ");
    let expectation_field_indent = format!("{indent}      ");
    json.push_str(&format!("{indent}{{\n"));
    json.push_str(&format!(
        "{field_indent}\"binding\": \"{}\",\n",
        json_escape(&suite.binding)
    ));
    json.push_str(&format!(
        "{field_indent}\"target\": \"{}\",\n",
        json_escape(&suite.target)
    ));
    json.push_str(&format!(
        "{field_indent}\"expectation_count\": {},\n",
        suite.expectation_count
    ));
    json.push_str(&format!(
        "{field_indent}\"passed_count\": {},\n",
        suite.passed_count
    ));
    json.push_str(&format!(
        "{field_indent}\"warning_count\": {},\n",
        suite.warning_count
    ));
    json.push_str(&format!(
        "{field_indent}\"failed_count\": {},\n",
        suite.failed_count
    ));
    json.push_str(&format!(
        "{field_indent}\"status\": \"{}\",\n",
        json_escape(&suite.status)
    ));
    json.push_str(&format!("{field_indent}\"expectations\": [\n"));
    for (index, expectation) in suite.expectations.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{expectation_indent}{{\n"));
        json.push_str(&format!(
            "{expectation_field_indent}\"kind\": \"{}\",\n",
            json_escape(&expectation.kind)
        ));
        json.push_str(&format!(
            "{expectation_field_indent}\"subject\": \"{}\",\n",
            json_escape(&expectation.subject)
        ));
        json.push_str(&format!(
            "{expectation_field_indent}\"text\": \"{}\",\n",
            json_escape(&expectation.text)
        ));
        push_optional_json_string(
            json,
            "matched_result",
            expectation.matched_result.as_deref(),
            expectation_field_indent.len(),
        );
        json.push_str(&format!(
            "{expectation_field_indent}\"status\": \"{}\",\n",
            json_escape(&expectation.status)
        ));
        json.push_str(&format!(
            "{expectation_field_indent}\"reason\": \"{}\",\n",
            json_escape(&expectation.reason)
        ));
        json.push_str(&format!(
            "{expectation_field_indent}\"line\": {}\n",
            expectation.line
        ));
        json.push_str(&format!("{expectation_indent}}}"));
    }
    json.push_str(&format!("\n{field_indent}],\n"));
    json.push_str(&format!("{field_indent}\"line\": {}\n", suite.line));
    json.push_str(&format!("{indent}}}"));
}

fn push_quality_result_json(
    json: &mut String,
    quality: &runtime_data::RuntimeQualityResult,
    indent: &str,
) {
    let field_indent = format!("{indent}  ");
    json.push_str(&format!("{indent}{{\n"));
    json.push_str(&format!(
        "{field_indent}\"binding\": \"{}\",\n",
        json_escape(&quality.binding)
    ));
    json.push_str(&format!(
        "{field_indent}\"kind\": \"{}\",\n",
        json_escape(&quality.kind)
    ));
    json.push_str(&format!(
        "{field_indent}\"category\": \"{}\",\n",
        json_escape(&quality.category)
    ));
    json.push_str(&format!(
        "{field_indent}\"target\": \"{}\",\n",
        json_escape(&quality.target)
    ));
    json.push_str(&format!(
        "{field_indent}\"subject\": \"{}\",\n",
        json_escape(&quality.subject)
    ));
    push_optional_json_string(
        json,
        "source_table",
        quality.source_table.as_deref(),
        field_indent.len(),
    );
    push_optional_json_string(
        json,
        "source_column",
        quality.source_column.as_deref(),
        field_indent.len(),
    );
    push_optional_json_string(
        json,
        "time_column",
        quality.time_column.as_deref(),
        field_indent.len(),
    );
    push_optional_json_number(json, "score", quality.score, field_indent.len());
    json.push_str(&format!(
        "{field_indent}\"passed_count\": {},\n",
        quality.passed_count
    ));
    json.push_str(&format!(
        "{field_indent}\"warning_count\": {},\n",
        quality.warning_count
    ));
    json.push_str(&format!(
        "{field_indent}\"failed_count\": {},\n",
        quality.failed_count
    ));
    json.push_str(&format!(
        "{field_indent}\"status\": \"{}\",\n",
        json_escape(&quality.status)
    ));
    json.push_str(&format!(
        "{field_indent}\"reason\": \"{}\",\n",
        json_escape(&quality.reason)
    ));
    json.push_str(&format!("{field_indent}\"failures\": [\n"));
    let failure_indent = format!("{indent}    ");
    let failure_field_indent = format!("{indent}      ");
    for (index, failure) in quality.failures.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{failure_indent}{{\n"));
        json.push_str(&format!(
            "{failure_field_indent}\"row\": {},\n",
            failure.row
        ));
        json.push_str(&format!(
            "{failure_field_indent}\"field\": \"{}\",\n",
            json_escape(&failure.field)
        ));
        json.push_str(&format!(
            "{failure_field_indent}\"value\": \"{}\",\n",
            json_escape(&failure.value)
        ));
        json.push_str(&format!(
            "{failure_field_indent}\"message\": \"{}\"\n",
            json_escape(&failure.message)
        ));
        json.push_str(&format!("{failure_indent}}}"));
    }
    json.push_str(&format!("\n{field_indent}],\n"));
    json.push_str(&format!("{field_indent}\"line\": {}\n", quality.line));
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

fn timeseries_fill_json(runtime_data: &RuntimeData, indent: &str) -> String {
    let mut json = String::new();
    let mut first = true;
    for fill in &runtime_data.timeseries_fill {
        if !first {
            json.push_str(",\n");
        }
        first = false;
        push_explicit_timeseries_fill_json(&mut json, fill, indent);
    }
    for coverage in &runtime_data.timeseries_coverage {
        if !first {
            json.push_str(",\n");
        }
        first = false;
        push_timeseries_coverage_fill_json(&mut json, coverage, indent);
    }
    json
}

fn push_explicit_timeseries_fill_json(
    json: &mut String,
    fill: &runtime_data::RuntimeTimeSeriesFill,
    indent: &str,
) {
    let field_indent = format!("{indent}  ");
    json.push_str(&format!("{indent}{{\n"));
    json.push_str(&format!(
        "{field_indent}\"binding\": \"{}\",\n",
        json_escape(&fill.binding)
    ));
    json.push_str(&format!(
        "{field_indent}\"source_table\": \"{}\",\n",
        json_escape(&fill.source_table)
    ));
    json.push_str(&format!(
        "{field_indent}\"source_column\": \"{}\",\n",
        json_escape(&fill.source_column)
    ));
    json.push_str(&format!(
        "{field_indent}\"time_column\": \"{}\",\n",
        json_escape(&fill.time_column)
    ));
    json.push_str(&format!(
        "{field_indent}\"strategy\": \"{}\",\n",
        json_escape(&fill.strategy)
    ));
    json.push_str(&format!(
        "{field_indent}\"method\": \"{}\",\n",
        json_escape(&fill.method)
    ));
    push_optional_json_number(
        json,
        "expected_step",
        fill.expected_step,
        field_indent.len(),
    );
    push_optional_json_number(json, "max_gap", fill.max_gap, field_indent.len());
    json.push_str(&format!(
        "{field_indent}\"fillable_count\": {},\n",
        fill.fillable_count
    ));
    json.push_str(&format!(
        "{field_indent}\"filled_count\": {},\n",
        fill.filled_count
    ));
    json.push_str(&format!(
        "{field_indent}\"missing_count\": {},\n",
        fill.missing_count
    ));
    json.push_str(&format!(
        "{field_indent}\"skipped_count\": {},\n",
        fill.skipped_count
    ));
    json.push_str(&format!(
        "{field_indent}\"fallback_required\": {},\n",
        fill.fallback_required
    ));
    json.push_str(&format!(
        "{field_indent}\"status\": \"{}\",\n",
        json_escape(&fill.status)
    ));
    json.push_str(&format!(
        "{field_indent}\"reason\": \"{}\",\n",
        json_escape(&fill.reason)
    ));
    json.push_str(&format!("{field_indent}\"line\": {}\n", fill.line));
    json.push_str(&format!("{indent}}}"));
}

fn push_timeseries_coverage_fill_json(
    json: &mut String,
    coverage: &runtime_data::RuntimeTimeSeriesCoverage,
    indent: &str,
) {
    let field_indent = format!("{indent}  ");
    let (strategy, status, reason) = timeseries_fill_policy(coverage);
    json.push_str(&format!("{indent}{{\n"));
    json.push_str(&format!(
        "{field_indent}\"binding\": \"{}\",\n",
        json_escape(&coverage.binding)
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
        "{field_indent}\"time_column\": \"{}\",\n",
        json_escape(&coverage.source_column)
    ));
    json.push_str(&format!(
        "{field_indent}\"strategy\": \"{}\",\n",
        json_escape(strategy)
    ));
    json.push_str(&format!(
        "{field_indent}\"method\": \"{}\",\n",
        json_escape(strategy)
    ));
    push_optional_json_number(
        json,
        "expected_step",
        coverage.expected_step,
        field_indent.len(),
    );
    push_optional_json_number(json, "max_gap", coverage.max_gap, field_indent.len());
    json.push_str(&format!("{field_indent}\"fillable_count\": 0,\n"));
    json.push_str(&format!(
        "{field_indent}\"filled_count\": {},\n",
        timeseries_filled_count(coverage)
    ));
    json.push_str(&format!(
        "{field_indent}\"missing_count\": {},\n",
        coverage.missing_count
    ));
    json.push_str(&format!(
        "{field_indent}\"skipped_count\": {},\n",
        coverage.missing_count
    ));
    json.push_str(&format!(
        "{field_indent}\"fallback_required\": {},\n",
        timeseries_fallback_required(coverage)
    ));
    json.push_str(&format!(
        "{field_indent}\"status\": \"{}\",\n",
        json_escape(status)
    ));
    json.push_str(&format!(
        "{field_indent}\"reason\": \"{}\",\n",
        json_escape(reason)
    ));
    json.push_str(&format!("{field_indent}\"line\": {}\n", coverage.line));
    json.push_str(&format!("{indent}}}"));
}

fn timeseries_quality_json(runtime_data: &RuntimeData, indent: &str) -> String {
    let mut json = String::new();
    for (index, quality) in runtime_data.timeseries_quality.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        push_timeseries_quality_json(&mut json, quality, indent);
    }
    json
}

fn push_timeseries_quality_json(
    json: &mut String,
    quality: &runtime_data::RuntimeTimeSeriesQuality,
    indent: &str,
) {
    let field_indent = format!("{indent}  ");
    json.push_str(&format!("{indent}{{\n"));
    json.push_str(&format!(
        "{field_indent}\"binding\": \"{}\",\n",
        json_escape(&quality.binding)
    ));
    json.push_str(&format!(
        "{field_indent}\"source_table\": \"{}\",\n",
        json_escape(&quality.source_table)
    ));
    json.push_str(&format!(
        "{field_indent}\"source_column\": \"{}\",\n",
        json_escape(&quality.source_column)
    ));
    json.push_str(&format!(
        "{field_indent}\"time_column\": \"{}\",\n",
        json_escape(&quality.time_column)
    ));
    push_optional_json_string(
        json,
        "coverage_binding",
        quality.coverage_binding.as_deref(),
        field_indent.len(),
    );
    push_optional_json_string(
        json,
        "fill_binding",
        quality.fill_binding.as_deref(),
        field_indent.len(),
    );
    push_optional_json_number(
        json,
        "expected_step",
        quality.expected_step,
        field_indent.len(),
    );
    push_optional_json_usize(
        json,
        "expected_count",
        quality.expected_count,
        field_indent.len(),
    );
    json.push_str(&format!(
        "{field_indent}\"actual_count\": {},\n",
        quality.actual_count
    ));
    json.push_str(&format!(
        "{field_indent}\"missing_count\": {},\n",
        quality.missing_count
    ));
    json.push_str(&format!(
        "{field_indent}\"filled_count\": {},\n",
        quality.filled_count
    ));
    json.push_str(&format!(
        "{field_indent}\"remaining_missing_count\": {},\n",
        quality.remaining_missing_count
    ));
    json.push_str(&format!(
        "{field_indent}\"coverage_status\": \"{}\",\n",
        json_escape(&quality.coverage_status)
    ));
    push_optional_json_string(
        json,
        "fill_status",
        quality.fill_status.as_deref(),
        field_indent.len(),
    );
    push_optional_json_number(
        json,
        "quality_score",
        quality.quality_score,
        field_indent.len(),
    );
    json.push_str(&format!(
        "{field_indent}\"status\": \"{}\",\n",
        json_escape(&quality.status)
    ));
    json.push_str(&format!(
        "{field_indent}\"reason\": \"{}\",\n",
        json_escape(&quality.reason)
    ));
    json.push_str(&format!("{field_indent}\"line\": {}\n", quality.line));
    json.push_str(&format!("{indent}}}"));
}

fn timeseries_fallbacks_json(runtime_data: &RuntimeData, indent: &str) -> String {
    let mut json = String::new();
    let mut first = true;
    for coverage in &runtime_data.timeseries_coverage {
        if !timeseries_fallback_required(coverage) {
            continue;
        }
        if !first {
            json.push_str(",\n");
        }
        first = false;
        push_timeseries_fallback_json(&mut json, coverage, indent);
    }
    json
}

fn timeseries_review_fallback_records(runtime_data: &RuntimeData) -> Vec<ReviewFallbackRecord> {
    runtime_data
        .timeseries_coverage
        .iter()
        .filter(|coverage| timeseries_fallback_required(coverage))
        .map(|coverage| ReviewFallbackRecord {
            kind: "timeseries_fill_policy".to_owned(),
            category: "data_quality".to_owned(),
            target: coverage.binding.clone(),
            method: "defer_to_explicit_fill_policy".to_owned(),
            fallback_source: timeseries_fallback_source(coverage).to_owned(),
            affected_scope: "timeseries coverage and fill policy".to_owned(),
            assumption: "missing or irregular samples are not automatically filled".to_owned(),
            risk_level: "medium".to_owned(),
            status: "recorded".to_owned(),
            reason: timeseries_fallback_reason(coverage).to_owned(),
            line: coverage.line,
        })
        .collect()
}

fn push_timeseries_fallback_json(
    json: &mut String,
    coverage: &runtime_data::RuntimeTimeSeriesCoverage,
    indent: &str,
) {
    let field_indent = format!("{indent}  ");
    json.push_str(&format!("{indent}{{\n"));
    json.push_str(&format!(
        "{field_indent}\"binding\": \"{}\",\n",
        json_escape(&coverage.binding)
    ));
    json.push_str(&format!(
        "{field_indent}\"kind\": \"timeseries_fill_policy\",\n"
    ));
    json.push_str(&format!(
        "{field_indent}\"fallback_source\": \"{}\",\n",
        json_escape(timeseries_fallback_source(coverage))
    ));
    json.push_str(&format!(
        "{field_indent}\"fallback_strategy\": \"defer_to_explicit_fill_policy\",\n"
    ));
    json.push_str(&format!(
        "{field_indent}\"missing_count\": {},\n",
        coverage.missing_count
    ));
    push_optional_json_number(json, "max_gap", coverage.max_gap, field_indent.len());
    json.push_str(&format!("{field_indent}\"status\": \"recorded\",\n"));
    json.push_str(&format!(
        "{field_indent}\"reason\": \"{}\",\n",
        json_escape(timeseries_fallback_reason(coverage))
    ));
    json.push_str(&format!("{field_indent}\"line\": {}\n", coverage.line));
    json.push_str(&format!("{indent}}}"));
}

fn timeseries_fill_policy(
    coverage: &runtime_data::RuntimeTimeSeriesCoverage,
) -> (&'static str, &'static str, &'static str) {
    if timeseries_fallback_required(coverage) {
        (
            "not_applied",
            "deferred",
            "coverage gaps or irregular axis detected; no automatic fill policy was selected",
        )
    } else {
        ("none_required", "not_required", "coverage is complete")
    }
}

fn timeseries_fallback_required(coverage: &runtime_data::RuntimeTimeSeriesCoverage) -> bool {
    coverage.missing_count > 0 || coverage.status != "complete"
}

fn timeseries_filled_count(_coverage: &runtime_data::RuntimeTimeSeriesCoverage) -> usize {
    0
}

fn timeseries_fallback_source(coverage: &runtime_data::RuntimeTimeSeriesCoverage) -> &'static str {
    if coverage.missing_count > 0 {
        "coverage_gap"
    } else {
        "coverage_status"
    }
}

fn timeseries_fallback_reason(coverage: &runtime_data::RuntimeTimeSeriesCoverage) -> &'static str {
    if coverage.missing_count > 0 {
        "missing samples require an explicit fill or imputation policy"
    } else {
        "coverage status requires an explicit fallback policy"
    }
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

fn config_promotions_json(report: &CheckReport) -> String {
    let mut json = String::new();
    for (index, promotion) in report.semantic_program.config_promotions.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&promotion.binding)
        ));
        json.push_str(&format!(
            "        \"format\": \"{}\",\n",
            json_escape(&promotion.format)
        ));
        json.push_str(&format!(
            "        \"schema_name\": \"{}\",\n",
            json_escape(&promotion.schema_name)
        ));
        json.push_str(&format!(
            "        \"source\": \"{}\",\n",
            json_escape(&promotion.source_literal)
        ));
        json.push_str(&format!(
            "        \"source_value\": \"{}\",\n",
            json_escape(&promotion.source_value)
        ));
        json.push_str(&format!(
            "        \"resolved_path\": \"{}\",\n",
            json_escape(&promotion.resolved_path)
        ));
        push_optional_json_string(
            &mut json,
            "source_hash",
            promotion.source_hash.as_deref(),
            8,
        );
        json.push_str(&format!(
            "        \"field_count\": {},\n",
            promotion.field_count
        ));
        json.push_str("        \"missing_fields\": [");
        push_json_string_array(&mut json, &promotion.missing_fields);
        json.push_str("],\n");
        json.push_str("        \"unknown_fields\": [");
        push_json_string_array(&mut json, &promotion.unknown_fields);
        json.push_str("],\n");
        json.push_str("        \"null_fields\": [");
        push_json_string_array(&mut json, &promotion.null_fields);
        json.push_str("],\n");
        json.push_str("        \"optional_fields\": [");
        push_json_string_array(&mut json, &promotion.optional_fields);
        json.push_str("],\n");
        json.push_str("        \"optional_missing_fields\": [");
        push_json_string_array(&mut json, &promotion.optional_missing_fields);
        json.push_str("],\n");
        json.push_str("        \"optional_null_fields\": [");
        push_json_string_array(&mut json, &promotion.optional_null_fields);
        json.push_str("],\n");
        json.push_str("        \"nested_object_fields\": [");
        push_json_string_array(&mut json, &promotion.nested_object_fields);
        json.push_str("],\n");
        json.push_str("        \"array_fields\": [");
        push_json_string_array(&mut json, &promotion.array_fields);
        json.push_str("],\n");
        json.push_str("        \"default_fields\": [");
        push_json_string_array(&mut json, &promotion.default_fields);
        json.push_str("],\n");
        json.push_str("        \"defaulted_fields\": [");
        push_json_string_array(&mut json, &promotion.defaulted_fields);
        json.push_str("],\n");
        json.push_str(&format!(
            "        \"type_mismatch_count\": {},\n",
            promotion.type_mismatches.len()
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&promotion.status)
        ));
        json.push_str(&format!("        \"line\": {}\n", promotion.line));
        json.push_str("      }");
    }
    json
}

fn network_boundaries_json(report: &CheckReport, cache_records: &[CacheManifestRecord]) -> String {
    let mut json = String::new();
    let mut first = true;
    for request in &report.semantic_program.net_requests {
        let cached_hash =
            network_cache_hit_hash(cache_records, "network_request", &request.binding);
        let response_hash = request.response_hash.as_deref().or(cached_hash);
        let status = network_status_with_cache(
            &request.status,
            request.response_hash.as_deref(),
            cached_hash,
        );
        let status_code = request.status_code.or_else(|| cached_hash.map(|_| 200));
        let status_class = if request.response_hash.is_none() && cached_hash.is_some() {
            "success"
        } else {
            request.status_class.as_str()
        };
        if !first {
            json.push_str(",\n");
        }
        first = false;
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"http_get\",\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&request.binding)
        ));
        json.push_str(&format!(
            "        \"url\": \"{}\",\n",
            json_escape(&request.url_value)
        ));
        push_network_query_json(&mut json, &request.query, "        ");
        push_optional_json_string(&mut json, "response_hash", response_hash, 8);
        push_optional_json_string(
            &mut json,
            "expected_sha256",
            request.expected_sha256.as_deref(),
            8,
        );
        push_optional_json_usize(&mut json, "retry", request.retry, 8);
        push_optional_json_string(&mut json, "timeout", request.timeout.as_deref(), 8);
        push_optional_json_usize(
            &mut json,
            "body_size_limit_bytes",
            request.body_size_limit_bytes,
            8,
        );
        match status_code {
            Some(status_code) => {
                json.push_str(&format!("        \"status_code\": {},\n", status_code))
            }
            None => json.push_str("        \"status_code\": null,\n"),
        }
        json.push_str(&format!(
            "        \"status_class\": \"{}\",\n",
            json_escape(status_class)
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&status)
        ));
        json.push_str(&format!("        \"line\": {}\n", request.line));
        json.push_str("      }");
    }
    for download in &report.semantic_program.net_downloads {
        let cached_hash =
            network_cache_hit_hash(cache_records, "network_download", &download.target_value);
        let response_hash = download.response_hash.as_deref().or(cached_hash);
        let status = network_status_with_cache(
            &download.status,
            download.response_hash.as_deref(),
            cached_hash,
        );
        let status_code = download.status_code.or_else(|| cached_hash.map(|_| 200));
        let status_class = if download.response_hash.is_none() && cached_hash.is_some() {
            "success"
        } else {
            download.status_class.as_str()
        };
        if !first {
            json.push_str(",\n");
        }
        first = false;
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"download\",\n");
        json.push_str(&format!(
            "        \"url\": \"{}\",\n",
            json_escape(&download.url_value)
        ));
        json.push_str(&format!(
            "        \"target\": \"{}\",\n",
            json_escape(&download.target_value)
        ));
        push_network_query_json(&mut json, &download.query, "        ");
        push_optional_json_string(&mut json, "response_hash", response_hash, 8);
        push_optional_json_string(
            &mut json,
            "expected_sha256",
            download.expected_sha256.as_deref(),
            8,
        );
        push_optional_json_usize(&mut json, "retry", download.retry, 8);
        push_optional_json_string(&mut json, "timeout", download.timeout.as_deref(), 8);
        push_optional_json_usize(
            &mut json,
            "body_size_limit_bytes",
            download.body_size_limit_bytes,
            8,
        );
        match status_code {
            Some(status_code) => {
                json.push_str(&format!("        \"status_code\": {},\n", status_code))
            }
            None => json.push_str("        \"status_code\": null,\n"),
        }
        json.push_str(&format!(
            "        \"status_class\": \"{}\",\n",
            json_escape(status_class)
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&status)
        ));
        json.push_str(&format!("        \"line\": {}\n", download.line));
        json.push_str("      }");
    }
    json
}

fn push_network_query_json(json: &mut String, query: &[eng_compiler::NetQueryParam], indent: &str) {
    json.push_str(&format!("{indent}\"query\": [\n"));
    for (index, param) in query.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!(
            "{indent}  {{ \"key\": \"{}\", \"value\": \"{}\", \"redacted\": {} }}",
            json_escape(&param.key),
            json_escape(&param.value),
            param.redacted
        ));
    }
    json.push_str(&format!("\n{indent}],\n"));
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

    fn run_plan_has_node(run_plan: &Value, id: &str) -> bool {
        run_plan
            .pointer("/graph/nodes")
            .and_then(Value::as_array)
            .is_some_and(|nodes| {
                nodes
                    .iter()
                    .any(|node| node.get("id").and_then(Value::as_str) == Some(id))
            })
    }

    fn run_plan_has_edge(run_plan: &Value, from: &str, to: &str, kind: &str) -> bool {
        run_plan
            .pointer("/graph/edges")
            .and_then(Value::as_array)
            .is_some_and(|edges| {
                edges.iter().any(|edge| {
                    edge.get("from").and_then(Value::as_str) == Some(from)
                        && edge.get("to").and_then(Value::as_str) == Some(to)
                        && edge.get("kind").and_then(Value::as_str) == Some(kind)
                })
            })
    }

    fn json_array_item_by_binding<'a>(
        value: &'a Value,
        pointer: &str,
        binding: &str,
    ) -> Option<&'a Value> {
        value
            .pointer(pointer)
            .and_then(Value::as_array)?
            .iter()
            .find(|item| item.get("binding").and_then(Value::as_str) == Some(binding))
    }

    fn json_array_item_by_field<'a>(
        value: &'a Value,
        pointer: &str,
        field: &str,
        expected: &str,
    ) -> Option<&'a Value> {
        value
            .pointer(pointer)
            .and_then(Value::as_array)?
            .iter()
            .find(|item| item.get(field).and_then(Value::as_str) == Some(expected))
    }

    fn sqlite_execute(db_path: &Path, statements: &[&str]) {
        let mut connection = sqlite_open_or_create(db_path).expect("sqlite open");
        for statement in statements {
            connection.execute_batch(statement).expect("sqlite execute");
        }
    }

    fn sqlite_query_scalar(db_path: &Path, sql: &str) -> Value {
        let connection = sqlite_open_existing(db_path).expect("sqlite open");
        let result = connection.query(sql).expect("sqlite query");
        let Some(row) = result.rows.first() else {
            return Value::Null;
        };
        match row.first() {
            Some(GraphiteSqlValue::Null) | None => Value::Null,
            Some(GraphiteSqlValue::Integer(value)) => json!(value),
            Some(GraphiteSqlValue::Real(value)) => json!(value),
            Some(GraphiteSqlValue::Text(value)) => Value::String(value.clone()),
            Some(GraphiteSqlValue::Blob(value)) => {
                Value::String(format!("blob:{} bytes", value.len()))
            }
        }
    }

    fn assert_saved_artifact(path: &Path, label: &str) {
        assert!(path.exists(), "missing {label}: {}", path.display());
    }

    fn assert_common_workflow_artifacts(output: &RunOutput) {
        assert_saved_artifact(&output.review_path, "review.json");
        assert_saved_artifact(&output.output_manifest_path, "output_manifest.json");
        assert_saved_artifact(&output.run_log_path, "run_log.json");
        assert_saved_artifact(&output.run_plan_path, "run_plan.json");
        assert_saved_artifact(&output.run_lock_path, "run_lock.json");
        assert!(output.artifacts_saved);
        assert!(output.run_plan_json.contains("\"graph\""));
        assert!(output
            .output_manifest_json
            .contains("\"artifact_registry\""));
    }

    #[test]
    fn workflow_modules_native_workflow_artifact_contracts() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let weather_source = repo_root
            .join("examples")
            .join("workflows")
            .join("01_weather_api_to_standard_file")
            .join("main.eng");
        let weather_build = repo_root
            .join("build")
            .join("runtime-workflow-modules-weather");
        let _ = fs::remove_dir_all(&weather_build);
        let weather_output = run_file(
            &weather_source,
            &weather_build,
            &RunOptions {
                save_artifacts: true,
                ..RunOptions::default()
            },
        )
        .expect("weather workflow run");
        assert_common_workflow_artifacts(&weather_output);
        assert_saved_artifact(&weather_output.process_results_path, "process_results.json");
        assert_saved_artifact(&weather_output.cache_manifest_path, "cache_manifest.json");
        assert!(weather_output
            .process_results_json
            .contains("\"process_count\": 0"));
        assert!(weather_output
            .result_json
            .contains("\"network_boundaries\""));
        assert!(weather_output
            .cache_manifest_json
            .contains("\"owner_kind\": \"network_request\""));
        assert!(weather_output
            .output_manifest_json
            .contains("outputs/standard_weather_file.txt"));
        assert!(weather_output
            .result_json
            .contains("\"timeseries_coverage\""));
        assert!(weather_output.review_json.contains("\"workflow_modules\""));

        let surrogate_dir = repo_root
            .join("examples")
            .join("workflows")
            .join("02_external_simulation_surrogate");
        let surrogate_source = surrogate_dir.join("main.eng");
        let surrogate_build = repo_root
            .join("build")
            .join("runtime-workflow-modules-surrogate");
        let _ = fs::remove_dir_all(&surrogate_build);
        let surrogate_output = run_file(
            &surrogate_source,
            &surrogate_build,
            &RunOptions {
                save_artifacts: true,
                ..RunOptions::default()
            },
        )
        .expect("surrogate workflow run");
        assert_common_workflow_artifacts(&surrogate_output);
        assert_saved_artifact(
            &surrogate_output.process_results_path,
            "process_results.json",
        );
        assert!(surrogate_output
            .process_results_json
            .contains("\"process_count\": 0"));
        assert!(surrogate_output
            .output_manifest_json
            .contains("outputs/case_001/input.txt"));
        assert!(surrogate_output
            .output_manifest_json
            .contains("outputs/workflow_summary.csv"));
        assert!(surrogate_output.result_json.contains("\"case_manifests\""));
        assert!(surrogate_output.result_json.contains("\"model_cards\""));
        assert!(surrogate_output
            .result_json
            .contains("\"kind\": \"RegressionModel\""));
        assert!(surrogate_output
            .result_json
            .contains("\"schema_name\": \"PredictionResult\""));
        assert!(surrogate_output.result_json.contains("\"db_manifests\""));
        assert!(surrogate_output
            .result_json
            .contains("\"transaction_status\": \"committed\""));
    }

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
        assert!(output.output_manifest_json.contains("\"hash\": \""));
        assert!(output
            .output_manifest_json
            .contains("\"overwrite_policy\": \"allowed\""));
        assert!(output
            .output_manifest_json
            .contains("\"overwrite_policy\": \"not_allowed\""));
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
    fn run_file_renders_template_and_records_manifest() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-template-render");
        let build_root = repo_root
            .join("build")
            .join("runtime-template-render-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("model")).expect("model dir");
        fs::write(
            source_dir.join("model").join("base_template.txt"),
            "CASE={{case_id}}\nLOAD={{load: kW}}\nNOTE={{note}}\n",
        )
        .expect("template");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "input_file = render template file(\"model/base_template.txt\")\nwith {\n    values = { case_id = \"case_001\", load = 12000 W, note = \"ready\" }\n    output = \"outputs/case_001/input.txt\"\n    missing = error\n}\n",
        )
        .expect("source");

        let output = run_file(&source_path, &build_root, &RunOptions::default()).expect("run file");
        let rendered_path = build_root
            .join("result")
            .join("outputs")
            .join("case_001")
            .join("input.txt");
        let rendered = fs::read_to_string(&rendered_path).expect("rendered template");
        assert!(rendered.contains("CASE=case_001"));
        assert!(rendered.contains("LOAD=12 kW"));
        assert!(rendered.contains("NOTE=ready"));
        let render_manifest_path = build_root
            .join("result")
            .join("outputs")
            .join("case_001")
            .join("input.txt.render_manifest.json");
        let render_manifest: Value = serde_json::from_str(
            &fs::read_to_string(&render_manifest_path).expect("render manifest"),
        )
        .expect("render manifest json");
        assert_eq!(
            render_manifest.get("format").and_then(Value::as_str),
            Some("eng-template-render-manifest-v1")
        );
        assert_eq!(
            render_manifest
                .get("placeholder_count")
                .and_then(Value::as_u64),
            Some(3)
        );
        assert!(render_manifest
            .get("output_hash")
            .and_then(Value::as_str)
            .is_some_and(|hash| !hash.is_empty()));
        assert!(render_manifest
            .get("values_hash")
            .and_then(Value::as_str)
            .is_some_and(|hash| !hash.is_empty()));

        let result: Value = serde_json::from_str(&output.result_json).expect("result json");
        let result_render = result
            .pointer("/typed_payload/render_manifests/0")
            .expect("result render manifest");
        assert_eq!(
            result_render.get("binding").and_then(Value::as_str),
            Some("input_file")
        );
        assert_eq!(
            result_render.get("output_path").and_then(Value::as_str),
            Some("outputs/case_001/input.txt")
        );

        let output_manifest: Value =
            serde_json::from_str(&output.output_manifest_json).expect("output manifest json");
        let artifacts = output_manifest
            .get("artifacts")
            .and_then(Value::as_array)
            .expect("artifacts");
        assert!(artifacts.iter().any(|artifact| {
            artifact.get("kind").and_then(Value::as_str) == Some("template_render")
                && artifact.get("class").and_then(Value::as_str) == Some("template")
                && artifact.get("path").and_then(Value::as_str)
                    == Some("outputs/case_001/input.txt")
                && artifact.get("hash").and_then(Value::as_str).is_some()
        }));
        assert!(artifacts.iter().any(|artifact| {
            artifact.get("kind").and_then(Value::as_str) == Some("template_render_manifest")
                && artifact.get("path").and_then(Value::as_str)
                    == Some("outputs/case_001/input.txt.render_manifest.json")
        }));

        let run_plan: Value = serde_json::from_str(&output.run_plan_json).expect("run plan json");
        assert!(run_plan_has_node(&run_plan, "template_render:1"));
        assert!(run_plan_has_edge(
            &run_plan,
            "template_render:1",
            "artifact:outputs/case_001/input.txt",
            "produces"
        ));
        assert!(output.review_json.contains("\"render_manifests\""));
    }

    #[test]
    fn run_file_reports_missing_template_value() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root
            .join("build")
            .join("runtime-template-render-missing");
        let build_root = repo_root
            .join("build")
            .join("runtime-template-render-missing-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("model")).expect("model dir");
        fs::write(
            source_dir.join("model").join("base_template.txt"),
            "MISSING={{required_value}}\n",
        )
        .expect("template");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "render template file(\"model/base_template.txt\") to \"outputs/input.txt\"\n",
        )
        .expect("source");

        let error = run_file(&source_path, &build_root, &RunOptions::default())
            .expect_err("missing template value should fail");
        assert!(error
            .to_string()
            .contains("E-TEMPLATE-MISSING-VALUE: missing template value `required_value`"));
    }

    #[test]
    fn run_file_records_timeseries_coverage_in_review() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root
            .join("build")
            .join("runtime-review-timeseries-coverage");
        let build_root = repo_root
            .join("build")
            .join("runtime-review-timeseries-coverage-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("weather.csv"),
            concat!(
                "time,dry_bulb\n",
                "2024-01-01T00:00:00+09:00,-2.1\n",
                "2024-01-01T01:00:00+09:00,-2.4\n",
            ),
        )
        .expect("write weather csv");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema WeatherHourly {\n",
                "    time: DateTime index\n",
                "    dry_bulb: AbsoluteTemperature [degC]\n",
                "}\n\n",
                "args {\n",
                "    year: Int = 2024\n",
                "    weather_file: CsvFile = file(\"data/weather.csv\")\n",
                "}\n\n",
                "weather = promote csv args.weather_file as WeatherHourly\n",
                "coverage = check coverage weather.time\n",
                "with {\n",
                "    expected_step = 1 h\n",
                "    year = args.year\n",
                "}\n",
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
        .expect("run file");

        assert!(output.result_json.contains("\"timeseries_coverage\""));
        assert!(output.review_json.contains("\"timeseries_coverage\""));
        assert!(output.review_json.contains("\"binding\": \"coverage\""));
        assert!(output.review_json.contains("\"source_table\": \"weather\""));
        assert!(output.review_json.contains("\"expected_count\": 8784"));
        assert!(output.review_json.contains("\"missing_count\": 8782"));
        assert!(output
            .review_json
            .contains("\"leap_year_policy\": \"gregorian_year\""));
        assert!(output.review_json.contains("\"status\": \"gapped\""));
        let result_value: Value =
            serde_json::from_str(&output.result_json).expect("result artifact json");
        let result_fill =
            json_array_item_by_binding(&result_value, "/typed_payload/timeseries_fill", "coverage")
                .expect("result timeseries fill");
        let result_fallback = json_array_item_by_binding(
            &result_value,
            "/typed_payload/timeseries_fallbacks",
            "coverage",
        )
        .expect("result timeseries fallback");
        assert_eq!(
            result_fill.get("status").and_then(Value::as_str),
            Some("deferred")
        );
        assert_eq!(
            result_fill
                .get("fallback_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            result_fallback
                .get("fallback_source")
                .and_then(Value::as_str),
            Some("coverage_gap")
        );
        let review_value: Value =
            serde_json::from_str(&output.review_json).expect("review artifact json");
        let review_fill = json_array_item_by_binding(&review_value, "/timeseries_fill", "coverage")
            .expect("review timeseries fill");
        let review_fallback =
            json_array_item_by_binding(&review_value, "/timeseries_fallbacks", "coverage")
                .expect("review timeseries fallback");
        assert_eq!(
            review_fill.get("strategy").and_then(Value::as_str),
            Some("not_applied")
        );
        assert_eq!(
            review_fallback
                .get("fallback_strategy")
                .and_then(Value::as_str),
            Some("defer_to_explicit_fill_policy")
        );
        let review_document_fallback = json_array_item_by_field(
            &review_value,
            "/review_document/fallbacks",
            "kind",
            "timeseries_fill_policy",
        )
        .expect("review document timeseries fallback");
        assert_eq!(
            review_document_fallback
                .get("category")
                .and_then(Value::as_str),
            Some("data_quality")
        );
        assert_eq!(
            review_document_fallback
                .get("target")
                .and_then(Value::as_str),
            Some("coverage")
        );
        assert_eq!(
            review_document_fallback
                .get("method")
                .and_then(Value::as_str),
            Some("defer_to_explicit_fill_policy")
        );
        assert_eq!(
            review_value
                .pointer("/review_document/root_contract/fallback_count")
                .and_then(Value::as_u64),
            review_value
                .pointer("/review_document/fallbacks")
                .and_then(Value::as_array)
                .map(|fallbacks| fallbacks.len() as u64)
        );
        assert!(output.run_plan_path.exists());
        assert!(output
            .run_plan_json
            .contains("\"format\": \"eng-run-plan-v1\""));
        assert!(output
            .run_plan_json
            .contains("\"id\": \"timeseries_coverage:coverage\""));
        assert!(output.run_plan_json.contains("\"artifact_hashes\""));
        assert!(output.review_json.contains("\"workflow_graph\""));
        assert!(output
            .review_json
            .contains("\"format\": \"eng-workflow-graph-review-v1\""));
        assert!(output.review_json.contains("\"risk_by_node\""));
        assert!(output.review_json.contains("\"risk\": \"medium\""));
        let run_plan: Value = serde_json::from_str(&output.run_plan_json).expect("run plan json");
        let coverage_node = json_array_item_by_field(
            &run_plan,
            "/graph/nodes",
            "id",
            "timeseries_coverage:coverage",
        )
        .expect("run plan timeseries coverage node");
        assert_eq!(
            coverage_node.get("risk").and_then(Value::as_str),
            Some("medium")
        );
        assert_eq!(
            coverage_node.get("risk_category").and_then(Value::as_str),
            Some("data_quality")
        );
        assert_eq!(
            coverage_node.get("risk_severity").and_then(Value::as_str),
            Some("warning")
        );
        let workflow_risk = json_array_item_by_field(
            &review_value,
            "/workflow_graph/risk_by_node",
            "id",
            "timeseries_coverage:coverage",
        )
        .expect("review workflow graph timeseries risk");
        assert_eq!(
            workflow_risk.get("risk_category").and_then(Value::as_str),
            Some("data_quality")
        );
        assert_eq!(
            run_plan
                .pointer("/rerun_decision/decision")
                .and_then(Value::as_str),
            Some("run")
        );
        assert_eq!(
            run_plan.pointer("/rerun_status").and_then(Value::as_str),
            Some("executed")
        );
        assert!(run_plan_has_edge(
            &run_plan,
            "timeseries_coverage:coverage",
            "source:csv:weather",
            "depends_on"
        ));
        let review_hash = hash_text(&output.review_json);
        assert_eq!(
            run_plan
                .pointer("/artifact_hashes/review")
                .and_then(Value::as_str),
            Some(review_hash.as_str())
        );
        assert!(output.static_run_plan_path.exists());
        let static_run_plan: Value =
            serde_json::from_str(&output.static_run_plan_json).expect("static run plan json");
        assert_eq!(
            static_run_plan.get("format").and_then(Value::as_str),
            Some("eng-static-run-plan-v1")
        );
        assert_eq!(
            static_run_plan
                .pointer("/execution_stage")
                .and_then(Value::as_str),
            Some("pre_execution")
        );
        assert_eq!(
            static_run_plan.pointer("/status").and_then(Value::as_str),
            Some("planned")
        );
        let static_run_plan_hash = hash_text(&output.static_run_plan_json);
        assert_eq!(
            run_plan
                .pointer("/artifact_hashes/static_run_plan")
                .and_then(Value::as_str),
            Some(static_run_plan_hash.as_str())
        );
        assert!(output
            .output_manifest_json
            .contains("\"kind\": \"static_run_plan\""));
        assert!(output
            .output_manifest_json
            .contains("\"kind\": \"run_plan\""));
        assert!(output.run_lock_path.exists());
        assert!(output
            .run_lock_json
            .contains("\"format\": \"eng-run-lock-v1\""));
        assert!(output
            .output_manifest_json
            .contains("\"kind\": \"run_lock\""));
        let run_lock: Value = serde_json::from_str(&output.run_lock_json).expect("run lock json");
        assert_eq!(
            run_lock
                .pointer("/artifact_hashes/static_run_plan")
                .and_then(Value::as_str),
            Some(static_run_plan_hash.as_str())
        );
        let first_input_hash = run_lock
            .get("input_hash")
            .and_then(Value::as_str)
            .expect("input hash")
            .to_owned();

        let second_output = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                save_artifacts: true,
                skip_unchanged: true,
                ..RunOptions::default()
            },
        )
        .expect("rerun file");
        assert!(second_output
            .stdout
            .contains("run skipped: unchanged run_lock"));
        let second_run_plan: Value =
            serde_json::from_str(&second_output.run_plan_json).expect("second run plan json");
        assert_eq!(
            second_run_plan
                .pointer("/rerun_decision/decision")
                .and_then(Value::as_str),
            Some("skip")
        );
        assert_eq!(
            second_run_plan
                .pointer("/rerun_decision/reason")
                .and_then(Value::as_str),
            Some("unchanged_run_lock")
        );
        assert_eq!(
            second_run_plan
                .pointer("/graph/nodes/0/rerun_status")
                .and_then(Value::as_str),
            Some("skipped")
        );
        let second_static_run_plan: Value =
            serde_json::from_str(&second_output.static_run_plan_json)
                .expect("second static run plan json");
        assert_eq!(
            second_static_run_plan
                .pointer("/rerun_decision/decision")
                .and_then(Value::as_str),
            Some("skip")
        );
        assert_eq!(
            second_static_run_plan
                .pointer("/graph/nodes/0/rerun_status")
                .and_then(Value::as_str),
            Some("skipped")
        );
        let second_review: Value =
            serde_json::from_str(&second_output.review_json).expect("second review json");
        assert_eq!(
            second_review
                .pointer("/workflow_graph/nodes/0/rerun_status")
                .and_then(Value::as_str),
            Some("skipped")
        );
        assert_eq!(
            second_review
                .pointer("/workflow_graph/nodes/0/rerun_decision/decision")
                .and_then(Value::as_str),
            Some("skip")
        );
        let second_run_lock: Value =
            serde_json::from_str(&second_output.run_lock_json).expect("second run lock json");
        assert_eq!(
            second_run_lock
                .pointer("/rerun_decision/prior_input_hash")
                .and_then(Value::as_str),
            Some(first_input_hash.as_str())
        );

        fs::write(&second_output.result_path, "{\"tampered\":true}\n")
            .expect("tamper saved result");
        let third_output = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                save_artifacts: true,
                skip_unchanged: true,
                ..RunOptions::default()
            },
        )
        .expect("rerun file after artifact hash mismatch");
        assert!(!third_output
            .stdout
            .contains("run skipped: unchanged run_lock"));
        let third_run_plan: Value =
            serde_json::from_str(&third_output.run_plan_json).expect("third run plan json");
        assert_eq!(
            third_run_plan
                .pointer("/rerun_decision/decision")
                .and_then(Value::as_str),
            Some("run")
        );
        assert_eq!(
            third_run_plan
                .pointer("/rerun_decision/reason")
                .and_then(Value::as_str),
            Some("artifact_hash_mismatch:result")
        );
        assert_eq!(
            third_run_plan
                .pointer("/rerun_status")
                .and_then(Value::as_str),
            Some("executed")
        );
        assert!(third_output
            .result_json
            .contains("\"format\": \"engres-v1\""));

        fs::remove_file(&third_output.report_path).expect("remove saved report");
        let fourth_output = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                save_artifacts: true,
                skip_unchanged: true,
                ..RunOptions::default()
            },
        )
        .expect("rerun file after missing artifact");
        assert!(!fourth_output
            .stdout
            .contains("run skipped: unchanged run_lock"));
        let fourth_run_plan: Value =
            serde_json::from_str(&fourth_output.run_plan_json).expect("fourth run plan json");
        assert_eq!(
            fourth_run_plan
                .pointer("/rerun_decision/decision")
                .and_then(Value::as_str),
            Some("run")
        );
        assert_eq!(
            fourth_run_plan
                .pointer("/rerun_decision/reason")
                .and_then(Value::as_str),
            Some("missing_saved_artifact")
        );
        assert_eq!(
            fourth_run_plan
                .pointer("/rerun_status")
                .and_then(Value::as_str),
            Some("executed")
        );
    }

    #[test]
    fn run_file_records_timeseries_fill_missing_in_artifacts() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root
            .join("build")
            .join("runtime-review-timeseries-fill");
        let build_root = repo_root
            .join("build")
            .join("runtime-review-timeseries-fill-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("weather.csv"),
            concat!(
                "time,wind_speed\n",
                "2024-01-01T00:00:00Z,1.0\n",
                "2024-01-01T01:00:00Z,2.0\n",
                "2024-01-01T03:00:00Z,4.0\n",
            ),
        )
        .expect("write weather csv");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema WeatherHourly {\n",
                "    time: DateTime index\n",
                "    wind_speed: DimensionlessNumber [1]\n",
                "}\n\n",
                "args {\n",
                "    weather_file: CsvFile = file(\"data/weather.csv\")\n",
                "}\n\n",
                "weather = promote csv args.weather_file as WeatherHourly\n",
                "filled = fill missing weather.wind_speed\n",
                "with {\n",
                "    method = interpolate\n",
                "    expected_step = 1 h\n",
                "    max_gap = 3 h\n",
                "}\n",
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
        .expect("run file");
        let result: Value = serde_json::from_str(&output.result_json).expect("result json");
        let fill = json_array_item_by_binding(&result, "/typed_payload/timeseries_fill", "filled")
            .expect("result timeseries fill");
        assert_eq!(
            fill.get("method").and_then(Value::as_str),
            Some("interpolate")
        );
        assert_eq!(fill.get("missing_count").and_then(Value::as_u64), Some(1));
        assert_eq!(fill.get("fillable_count").and_then(Value::as_u64), Some(1));
        assert_eq!(fill.get("filled_count").and_then(Value::as_u64), Some(1));
        assert_eq!(fill.get("status").and_then(Value::as_str), Some("applied"));
        let quality =
            json_array_item_by_binding(&result, "/typed_payload/timeseries_quality", "filled")
                .expect("result timeseries quality");
        assert_eq!(
            quality.get("coverage_binding").and_then(Value::as_str),
            Some("weather.Time.coverage")
        );
        assert_eq!(
            quality.get("fill_binding").and_then(Value::as_str),
            Some("filled")
        );
        assert_eq!(
            quality
                .get("remaining_missing_count")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            quality.get("quality_score").and_then(Value::as_f64),
            Some(1.0)
        );
        assert_eq!(
            quality.get("status").and_then(Value::as_str),
            Some("passed")
        );
        let quality_result = json_array_item_by_binding(
            &result,
            "/typed_payload/quality_results",
            "filled.quality_result",
        )
        .expect("result quality result");
        assert_eq!(
            quality_result.get("kind").and_then(Value::as_str),
            Some("timeseries_quality_result")
        );
        assert_eq!(
            quality_result.get("target").and_then(Value::as_str),
            Some("filled")
        );
        assert_eq!(
            quality_result.get("score").and_then(Value::as_f64),
            Some(1.0)
        );
        assert_eq!(
            quality_result.get("status").and_then(Value::as_str),
            Some("passed")
        );
        let filled_object =
            json_array_item_by_field(&result, "/object_store/objects", "name", "filled")
                .expect("filled object");
        assert_eq!(
            filled_object.get("kind").and_then(Value::as_str),
            Some("timeseries")
        );

        let review: Value = serde_json::from_str(&output.review_json).expect("review json");
        let review_fill = json_array_item_by_binding(&review, "/timeseries_fill", "filled")
            .expect("review timeseries fill");
        assert_eq!(
            review_fill.get("source_column").and_then(Value::as_str),
            Some("wind_speed")
        );
        let review_quality = json_array_item_by_binding(&review, "/timeseries_quality", "filled")
            .expect("review timeseries quality");
        assert_eq!(
            review_quality.get("filled_count").and_then(Value::as_u64),
            Some(1)
        );
        let review_quality_result =
            json_array_item_by_binding(&review, "/quality_results", "filled.quality_result")
                .expect("review quality result");
        assert_eq!(
            review_quality_result
                .get("category")
                .and_then(Value::as_str),
            Some("timeseries")
        );
        let run_plan: Value = serde_json::from_str(&output.run_plan_json).expect("run plan json");
        let fill_node =
            json_array_item_by_field(&run_plan, "/graph/nodes", "id", "timeseries_fill:filled")
                .expect("run plan timeseries fill node");
        assert_eq!(
            fill_node.get("risk_category").and_then(Value::as_str),
            Some("data_quality")
        );
        let quality_node =
            json_array_item_by_field(&run_plan, "/graph/nodes", "id", "timeseries_quality:filled")
                .expect("run plan timeseries quality node");
        assert_eq!(
            quality_node.get("risk_category").and_then(Value::as_str),
            Some("data_quality")
        );
        let quality_result_node = json_array_item_by_field(
            &run_plan,
            "/graph/nodes",
            "id",
            "quality_result:filled.quality_result",
        )
        .expect("run plan quality result node");
        assert_eq!(
            quality_result_node
                .get("risk_category")
                .and_then(Value::as_str),
            Some("data_quality")
        );
        assert!(run_plan_has_edge(
            &run_plan,
            "timeseries_fill:filled",
            "source:csv:weather",
            "depends_on"
        ));
        assert!(run_plan_has_edge(
            &run_plan,
            "timeseries_quality:filled",
            "timeseries_fill:filled",
            "depends_on"
        ));
        assert!(run_plan_has_edge(
            &run_plan,
            "timeseries_quality:filled",
            "timeseries_coverage:weather.Time.coverage",
            "depends_on"
        ));
        assert!(run_plan_has_edge(
            &run_plan,
            "quality_result:filled.quality_result",
            "timeseries_quality:filled",
            "depends_on"
        ));
    }

    #[test]
    fn run_file_records_common_quality_results_for_validation_and_schema_constraints() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root
            .join("build")
            .join("runtime-review-common-quality");
        let build_root = repo_root
            .join("build")
            .join("runtime-review-common-quality-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("sensor.csv"),
            concat!(
                "time,m_dot\n",
                "2024-01-01T00:00:00Z,0.10\n",
                "2024-01-01T01:00:00Z,0.30\n",
            ),
        )
        .expect("write sensor csv");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema SensorData {\n",
                "    time: DateTime index\n",
                "    m_dot: MassFlowRate [kg/s]\n",
                "\n",
                "    constraints {\n",
                "        time is monotonic\n",
                "        m_dot <= 0.25 kg/s\n",
                "    }\n",
                "}\n\n",
                "args {\n",
                "    input: CsvFile = file(\"data/sensor.csv\")\n",
                "}\n\n",
                "sensor = promote csv args.input as SensorData\n",
                "expect sensor {\n",
                "    time is continuous with { step = 1 h }\n",
                "    m_dot <= 0.25 kg/s\n",
                "}\n",
                "Q_dist = normal(mean=5 kW, std=0.2 kW, samples=31)\n",
                "validate mean(Q_dist) between 4 kW and 6 kW\n",
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
        .expect("run file");

        let result: Value = serde_json::from_str(&output.result_json).expect("result json");
        let quality_results = result
            .pointer("/typed_payload/quality_results")
            .and_then(Value::as_array)
            .expect("result quality results");
        let validation_quality = quality_results
            .iter()
            .find(|quality| {
                quality.get("kind").and_then(Value::as_str) == Some("validation_result")
            })
            .expect("validation quality result");
        assert_eq!(
            validation_quality.get("category").and_then(Value::as_str),
            Some("validation")
        );
        assert_eq!(
            validation_quality.get("target").and_then(Value::as_str),
            Some("mean(Q_dist) between 4 kW and 6 kW")
        );
        assert_eq!(
            validation_quality.get("status").and_then(Value::as_str),
            Some("passed")
        );
        assert_eq!(
            validation_quality.get("score").and_then(Value::as_f64),
            Some(1.0)
        );
        let schema_quality = quality_results
            .iter()
            .find(|quality| {
                quality.get("kind").and_then(Value::as_str) == Some("schema_constraint_result")
                    && quality.get("subject").and_then(Value::as_str) == Some("SensorData.m_dot")
            })
            .expect("schema constraint quality result");
        assert_eq!(
            schema_quality.get("category").and_then(Value::as_str),
            Some("schema_constraint")
        );
        assert_eq!(
            schema_quality.get("source_table").and_then(Value::as_str),
            Some("sensor")
        );
        assert_eq!(
            schema_quality.get("status").and_then(Value::as_str),
            Some("failed")
        );
        assert_eq!(
            schema_quality.get("failed_count").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            schema_quality
                .pointer("/failures/0/row")
                .and_then(Value::as_u64),
            Some(3)
        );
        assert_eq!(
            schema_quality
                .pointer("/failures/0/field")
                .and_then(Value::as_str),
            Some("m_dot")
        );
        assert_eq!(
            schema_quality.get("score").and_then(Value::as_f64),
            Some(0.5)
        );
        let expectation_suite = json_array_item_by_binding(
            &result,
            "/typed_payload/expectation_suites",
            "sensor.expectations",
        )
        .expect("result expectation suite");
        assert_eq!(
            expectation_suite.get("target").and_then(Value::as_str),
            Some("sensor")
        );
        assert_eq!(
            expectation_suite
                .get("expectation_count")
                .and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            expectation_suite
                .get("passed_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            expectation_suite
                .get("failed_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            expectation_suite.get("status").and_then(Value::as_str),
            Some("failed")
        );
        let suite_quality = json_array_item_by_binding(
            &result,
            "/typed_payload/quality_results",
            "sensor.expectations.quality_result",
        )
        .expect("expectation suite quality result");
        assert_eq!(
            suite_quality.get("kind").and_then(Value::as_str),
            Some("expectation_suite_result")
        );
        assert_eq!(
            suite_quality.get("category").and_then(Value::as_str),
            Some("expectation_suite")
        );
        assert_eq!(
            suite_quality.get("score").and_then(Value::as_f64),
            Some(0.5)
        );
        let report_spec: Value =
            serde_json::from_str(&output.report_spec_json).expect("report spec json");
        let quality_report = report_spec
            .get("quality_report")
            .expect("report spec quality report");
        assert_eq!(
            quality_report.get("status").and_then(Value::as_str),
            Some("failed")
        );
        assert!(quality_report
            .get("total_count")
            .and_then(Value::as_u64)
            .is_some_and(|count| count >= 4));
        assert_eq!(
            quality_report.get("failed_count").and_then(Value::as_u64),
            Some(2)
        );
        assert!(quality_report
            .pointer("/results")
            .and_then(Value::as_array)
            .is_some_and(|results| {
                results.iter().any(|result| {
                    result.get("binding").and_then(Value::as_str)
                        == Some("sensor.expectations.quality_result")
                        && result.get("category").and_then(Value::as_str)
                            == Some("expectation_suite")
                })
            }));
        assert!(output.report_html.contains("Quality Report"));
        assert!(output
            .report_html
            .contains("sensor.expectations.quality_result"));
        assert!(output.report_html.contains("row 3 field m_dot"));

        let review: Value = serde_json::from_str(&output.review_json).expect("review json");
        let review_suite =
            json_array_item_by_binding(&review, "/expectation_suites", "sensor.expectations")
                .expect("review expectation suite");
        assert_eq!(
            review_suite.get("failed_count").and_then(Value::as_u64),
            Some(1)
        );
        let review_quality_results = review
            .pointer("/quality_results")
            .and_then(Value::as_array)
            .expect("review quality results");
        assert!(review_quality_results.iter().any(|quality| {
            quality.get("kind").and_then(Value::as_str) == Some("validation_result")
                && quality.get("status").and_then(Value::as_str) == Some("passed")
        }));
        assert!(review_quality_results.iter().any(|quality| {
            quality.get("kind").and_then(Value::as_str) == Some("schema_constraint_result")
                && quality.get("subject").and_then(Value::as_str) == Some("SensorData.m_dot")
                && quality.get("status").and_then(Value::as_str) == Some("failed")
                && quality.pointer("/failures/0/field").and_then(Value::as_str) == Some("m_dot")
        }));

        let run_plan: Value = serde_json::from_str(&output.run_plan_json).expect("run plan json");
        let suite_node = json_array_item_by_field(
            &run_plan,
            "/graph/nodes",
            "id",
            "expectation_suite:sensor.expectations",
        )
        .expect("expectation suite run plan node");
        assert_eq!(
            suite_node.get("risk_category").and_then(Value::as_str),
            Some("data_quality")
        );
        let schema_quality_binding = schema_quality
            .get("binding")
            .and_then(Value::as_str)
            .expect("schema quality binding");
        let schema_quality_node_id = format!("quality_result:{schema_quality_binding}");
        let schema_quality_node =
            json_array_item_by_field(&run_plan, "/graph/nodes", "id", &schema_quality_node_id)
                .expect("schema quality run plan node");
        assert_eq!(
            schema_quality_node
                .get("risk_category")
                .and_then(Value::as_str),
            Some("data_quality")
        );
        assert!(run_plan_has_edge(
            &run_plan,
            &schema_quality_node_id,
            "source:csv:sensor",
            "depends_on"
        ));
        assert!(run_plan_has_edge(
            &run_plan,
            "expectation_suite:sensor.expectations",
            "source:csv:sensor",
            "depends_on"
        ));
        assert!(run_plan_has_edge(
            &run_plan,
            "quality_result:sensor.expectations.quality_result",
            "expectation_suite:sensor.expectations",
            "depends_on"
        ));
    }

    #[test]
    fn run_file_records_timeseries_alignment_and_resampling_hooks() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root
            .join("build")
            .join("runtime-review-timeseries-align");
        let build_root = repo_root
            .join("build")
            .join("runtime-review-timeseries-align-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("measured.csv"),
            concat!(
                "time,T_zone\n",
                "2024-01-01T00:00:00Z,21.0\n",
                "2024-01-01T01:00:00Z,21.5\n",
                "2024-01-01T02:00:00Z,22.0\n",
            ),
        )
        .expect("write measured csv");
        fs::write(
            source_dir.join("data").join("simulated.csv"),
            concat!(
                "time,T_zone\n",
                "2024-01-01T00:00:00Z,20.9\n",
                "2024-01-01T01:00:00Z,21.4\n",
                "2024-01-01T02:00:00Z,21.9\n",
            ),
        )
        .expect("write simulated csv");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema ZoneTemperature {\n",
                "    time: DateTime index\n",
                "    T_zone: AbsoluteTemperature [degC]\n",
                "}\n\n",
                "args {\n",
                "    measured_file: CsvFile = file(\"data/measured.csv\")\n",
                "    simulated_file: CsvFile = file(\"data/simulated.csv\")\n",
                "}\n\n",
                "measured = promote csv args.measured_file as ZoneTemperature\n",
                "simulated = promote csv args.simulated_file as ZoneTemperature\n",
                "aligned = align measured.T_zone with simulated.T_zone\n",
                "resampled = resample measured.T_zone to simulated.T_zone\n",
                "with {\n",
                "    method = linear\n",
                "    target_step = 1 h\n",
                "    tolerance = 5 min\n",
                "}\n",
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
        .expect("run file");
        let result: Value = serde_json::from_str(&output.result_json).expect("result json");
        let aligned =
            json_array_item_by_binding(&result, "/typed_payload/time_alignments", "aligned")
                .expect("align hook");
        assert_eq!(
            aligned.get("strategy").and_then(Value::as_str),
            Some("align")
        );
        assert_eq!(
            aligned.get("method").and_then(Value::as_str),
            Some("metadata_only")
        );
        assert_eq!(
            aligned.get("status").and_then(Value::as_str),
            Some("matched")
        );
        let resampled =
            json_array_item_by_binding(&result, "/typed_payload/time_alignments", "resampled")
                .expect("resample hook");
        assert_eq!(
            resampled.get("strategy").and_then(Value::as_str),
            Some("resample")
        );
        assert_eq!(
            resampled.get("method").and_then(Value::as_str),
            Some("linear")
        );
        assert_eq!(
            resampled.get("resample_step").and_then(Value::as_f64),
            Some(3600.0)
        );
        assert_eq!(
            resampled.get("tolerance").and_then(Value::as_f64),
            Some(300.0)
        );
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
    fn run_source_materializes_table_transform_artifacts() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-table-transform-run");
        let build_root = repo_root
            .join("build")
            .join("runtime-table-transform-run-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("station_map.csv"),
            concat!(
                "region,station_id,valid_from,valid_to,latitude,longitude\n",
                "demo,STN001,2020-01-01T00:00:00+09:00,,37.5665,126.9780\n",
                "other,STN002,2020-01-01T00:00:00+09:00,,35.1796,129.0756\n",
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
                "candidates = filter stations\n",
                "where {\n",
                "    region == args.region\n",
                "    valid_from <= date(args.year, 1, 1)\n",
                "    valid_to is none or valid_to >= date(args.year, 12, 31)\n",
                "}\n",
                "station = require_one candidates\n",
                "with {\n",
                "    on_none = error \"No station for region/year\"\n",
                "    on_many = error \"Multiple stations for region/year\"\n",
                "}\n",
                "selected_station_id: String = station.station_id\n",
                "write text \"summary.txt\", \"station={selected_station_id}\"\n",
                "with {\n",
                "    overwrite = true\n",
                "}\n",
                "report {\n",
                "    show candidates\n",
                "    show station\n",
                "    show selected_station_id\n",
                "}\n",
            ),
            &build_root,
            &RunOptions::default(),
        )
        .expect("run");

        assert!(output.result_json.contains("\"table_transforms\""));
        assert!(output.result_json.contains("\"binding\": \"candidates\""));
        assert!(output.result_json.contains("\"operation\": \"filter\""));
        assert!(output.result_json.contains("\"input_row_count\": 2"));
        assert!(output.result_json.contains("\"output_row_count\": 1"));
        assert!(output.result_json.contains("\"row_diagnostics\""));
        assert!(output.result_json.contains("\"row_index\": 1"));
        assert!(output.result_json.contains("\"row_index\": 2"));
        assert!(output.result_json.contains("\"status\": \"matched\""));
        assert!(output.result_json.contains("\"status\": \"excluded\""));
        assert!(output
            .result_json
            .contains("\"reason\": \"one or more predicates did not match\""));
        assert!(output.result_json.contains("\"actual\": \"other\""));
        assert!(output.result_json.contains("\"expected\": \"demo\""));
        assert!(output.result_json.contains("\"binding\": \"station\""));
        assert!(output
            .result_json
            .contains("\"operation\": \"require_one\""));
        assert!(output.result_json.contains("\"status\": \"selected\""));
        assert!(output
            .result_json
            .contains("\"reason\": \"require_one selected the only candidate row\""));
        assert!(output.review_json.contains("\"table_transforms\""));
        assert!(output.review_json.contains("\"row_diagnostics\""));
        assert!(output.review_json.contains("\"table_transform_count\": 2"));
        let summary =
            fs::read_to_string(build_root.join("result").join("summary.txt")).expect("summary txt");
        assert_eq!(summary, "station=STN001");
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_source_materializes_table_datetime_comparison_transform_artifacts() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-table-datetime-run");
        let build_root = repo_root
            .join("build")
            .join("runtime-table-datetime-run-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("events.csv"),
            concat!(
                "event_id,timestamp\n",
                "equal,2024-01-01T09:00:00+09:00\n",
                "later,2024-01-01T10:00:00+09:00\n",
            ),
        )
        .expect("events csv");
        let virtual_path = source_dir.join("__ide_terminal__.eng");

        let output = run_source(
            &virtual_path,
            concat!(
                "schema EventLog {\n",
                "    event_id: String\n",
                "    timestamp: DateTime\n",
                "}\n\n",
                "args {\n",
                "    events_path: CsvFile = file(\"data/events.csv\")\n",
                "}\n\n",
                "events = promote csv args.events_path as EventLog\n",
                "exact = filter events\n",
                "where {\n",
                "    timestamp == \"2024-01-01T00:00:00Z\"\n",
                "}\n",
                "report {\n",
                "    show exact\n",
                "}\n",
            ),
            &build_root,
            &RunOptions::default(),
        )
        .expect("run");

        assert!(output.result_json.contains("\"binding\": \"exact\""));
        assert!(output.result_json.contains("\"operation\": \"filter\""));
        assert!(output.result_json.contains("\"input_row_count\": 2"));
        assert!(output.result_json.contains("\"output_row_count\": 1"));
        assert!(output.result_json.contains("\"matched_row_indices\": [1]"));
        assert!(output.result_json.contains("\"row_index\": 1"));
        assert!(output.result_json.contains("\"row_index\": 2"));
        assert!(output.result_json.contains("\"status\": \"matched\""));
        assert!(output.result_json.contains("\"status\": \"excluded\""));
        assert!(output
            .result_json
            .contains("\"actual\": \"2024-01-01T09:00:00+09:00\""));
        assert!(output
            .result_json
            .contains("\"expected\": \"2024-01-01T00:00:00Z\""));
        assert!(output.review_json.contains("\"table_transforms\""));
        assert!(output.review_json.contains("\"table_transform_count\": 1"));
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_source_materializes_table_select_transform_artifacts() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-table-select-run");
        let build_root = repo_root
            .join("build")
            .join("runtime-table-select-run-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("station_map.csv"),
            concat!(
                "region,station_id,latitude\n",
                "demo,STN001,37.5665\n",
                "other,STN002,35.1796\n",
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
                "    latitude: DimensionlessNumber [1]\n",
                "}\n\n",
                "args {\n",
                "    station_map: CsvFile = file(\"data/station_map.csv\")\n",
                "}\n\n",
                "stations = promote csv args.station_map as StationMap\n",
                "station_fields = select stations columns station_id, latitude\n",
                "report {\n",
                "    show station_fields\n",
                "}\n",
            ),
            &build_root,
            &RunOptions::default(),
        )
        .expect("run");

        assert!(output.result_json.contains("\"operation\": \"select\""));
        assert!(output
            .result_json
            .contains("\"status\": \"selected_columns\""));
        assert!(output.result_json.contains("\"input_row_count\": 2"));
        assert!(output.result_json.contains("\"output_row_count\": 2"));
        assert!(output.result_json.contains("\"selected_columns\""));
        assert!(output.result_json.contains("\"name\": \"station_id\""));
        assert!(output.review_json.contains("\"operation\": \"select\""));
        assert!(output.review_json.contains("\"selected_column_count\": 2"));
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_source_materializes_table_sort_transform_artifacts() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-table-sort-run");
        let build_root = repo_root
            .join("build")
            .join("runtime-table-sort-run-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("station_map.csv"),
            concat!(
                "station_id,latitude\n",
                "STN002,35.1796\n",
                "STN001,37.5665\n",
            ),
        )
        .expect("station map csv");
        let virtual_path = source_dir.join("__ide_terminal__.eng");

        let output = run_source(
            &virtual_path,
            concat!(
                "schema StationMap {\n",
                "    station_id: String\n",
                "    latitude: DimensionlessNumber [1]\n",
                "}\n\n",
                "args {\n",
                "    station_map: CsvFile = file(\"data/station_map.csv\")\n",
                "}\n\n",
                "stations = promote csv args.station_map as StationMap\n",
                "ordered = sort stations by latitude desc\n",
                "report {\n",
                "    show ordered\n",
                "}\n",
            ),
            &build_root,
            &RunOptions::default(),
        )
        .expect("run");

        assert!(output.result_json.contains("\"operation\": \"sort\""));
        assert!(output.result_json.contains("\"status\": \"sorted\""));
        assert!(output.result_json.contains("\"sort_keys\""));
        assert!(output.result_json.contains("\"column\": \"latitude\""));
        assert!(output.result_json.contains("\"direction\": \"desc\""));
        assert!(output
            .result_json
            .contains("\"matched_row_indices\": [2, 1]"));
        assert!(output.review_json.contains("\"operation\": \"sort\""));
        assert!(output.review_json.contains("\"sort_keys\""));
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_source_materializes_table_derive_transform_artifacts() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-table-derive-run");
        let build_root = repo_root
            .join("build")
            .join("runtime-table-derive-run-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("station_map.csv"),
            concat!(
                "station_id,longitude\n",
                "STN001,126.9780\n",
                "STN002,129.0756\n",
            ),
        )
        .expect("station map csv");
        let virtual_path = source_dir.join("__ide_terminal__.eng");

        let output = run_source(
            &virtual_path,
            concat!(
                "schema StationMap {\n",
                "    station_id: String\n",
                "    longitude: DimensionlessNumber [1]\n",
                "}\n\n",
                "args {\n",
                "    station_map: CsvFile = file(\"data/station_map.csv\")\n",
                "}\n\n",
                "stations = promote csv args.station_map as StationMap\n",
                "station_plus = derive stations column longitude_copy = longitude\n",
                "report {\n",
                "    show station_plus\n",
                "}\n",
            ),
            &build_root,
            &RunOptions::default(),
        )
        .expect("run");

        assert!(output.result_json.contains("\"operation\": \"derive\""));
        assert!(output
            .result_json
            .contains("\"status\": \"derived_columns\""));
        assert!(output.result_json.contains("\"input_row_count\": 2"));
        assert!(output.result_json.contains("\"output_row_count\": 2"));
        assert!(output.result_json.contains("\"derived_columns\""));
        assert!(output.result_json.contains("\"name\": \"longitude_copy\""));
        assert!(output
            .result_json
            .contains("\"source_columns\": [\"longitude\"]"));
        assert!(output.review_json.contains("\"operation\": \"derive\""));
        assert!(output.review_json.contains("\"derived_columns\""));
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_source_materializes_table_join_transform_artifacts() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-table-join-run");
        let build_root = repo_root
            .join("build")
            .join("runtime-table-join-run-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("samples.csv"),
            concat!("case_id,cooling_cop\n", "case_001,3.2\n", "case_002,3.4\n",),
        )
        .expect("samples csv");
        fs::write(
            source_dir.join("data").join("results.csv"),
            concat!(
                "case_id,unmet_hours\n",
                "case_001,12\n",
                "case_002,8\n",
                "case_003,15\n",
            ),
        )
        .expect("results csv");
        let virtual_path = source_dir.join("__ide_terminal__.eng");

        let output = run_source(
            &virtual_path,
            concat!(
                "schema DesignSample {\n",
                "    case_id: String\n",
                "    cooling_cop: Ratio [1]\n",
                "}\n\n",
                "schema SimulationResult {\n",
                "    case_id: String\n",
                "    unmet_hours: Duration [h]\n",
                "}\n\n",
                "args {\n",
                "    samples_path: CsvFile = file(\"data/samples.csv\")\n",
                "    results_path: CsvFile = file(\"data/results.csv\")\n",
                "}\n\n",
                "samples = promote csv args.samples_path as DesignSample\n",
                "results = promote csv args.results_path as SimulationResult\n",
                "joined = join samples with results\n",
                "on {\n",
                "    samples.case_id == results.case_id\n",
                "}\n",
                "report {\n",
                "    show joined\n",
                "}\n",
            ),
            &build_root,
            &RunOptions::default(),
        )
        .expect("run");

        assert!(output.result_json.contains("\"operation\": \"join\""));
        assert!(output.result_json.contains("\"binding\": \"joined\""));
        assert!(output
            .result_json
            .contains("\"secondary_table\": \"results\""));
        assert!(output.result_json.contains("\"input_row_count\": 2"));
        assert!(output
            .result_json
            .contains("\"secondary_input_row_count\": 3"));
        assert!(output.result_json.contains("\"output_row_count\": 2"));
        assert!(output.result_json.contains("\"matched_pair_count\": 2"));
        assert!(output.result_json.contains("\"row_diagnostics\""));
        assert!(output
            .result_json
            .contains("\"secondary_row_indices\": [1]"));
        assert!(output
            .result_json
            .contains("\"secondary_row_indices\": [2]"));
        assert!(output.result_json.contains("\"operator\": \"join_key\""));
        assert!(output.review_json.contains("\"operation\": \"join\""));
        assert!(output.review_json.contains("\"join_keys\""));
        assert!(output.review_json.contains("\"row_diagnostics\""));
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
    fn run_source_materializes_generated_sample_tables() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-generated-samples");
        let build_root = repo_root
            .join("build")
            .join("runtime-generated-samples-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let virtual_path = source_dir.join("__ide_terminal__.eng");

        let output = run_source(
            &virtual_path,
            concat!(
                "grid_samples = sample grid\n",
                "with {\n",
                "    count = 4\n",
                "    cooling_cop = uniform(2.5, 5.0)\n",
                "    lighting_power_density = uniform(5 W/m2, 15 W/m2)\n",
                "}\n\n",
                "random_samples = sample random\n",
                "with {\n",
                "    count = 3\n",
                "    seed = 7\n",
                "    cooling_cop = uniform(2.5, 5.0)\n",
                "}\n\n",
                "lhs_samples = sample lhs\n",
                "with {\n",
                "    count = 3\n",
                "    seed = 9\n",
                "    cooling_cop = uniform(2.5, 5.0)\n",
                "}\n\n",
                "print \"samples={grid_samples.rows}\"\n",
            ),
            &build_root,
            &RunOptions::default(),
        )
        .expect("generated sample run");

        assert!(output.result_json.contains("\"binding\": \"grid_samples\""));
        assert!(output
            .result_json
            .contains("\"binding\": \"random_samples\""));
        assert!(output.result_json.contains("\"binding\": \"lhs_samples\""));
        assert!(output
            .result_json
            .contains("\"schema_name\": \"GeneratedSample\""));
        assert!(output
            .result_json
            .contains("\"generation\": \"sample_grid\""));
        assert!(output
            .result_json
            .contains("\"generation\": \"sample_random\""));
        assert!(output
            .result_json
            .contains("\"generation\": \"sample_lhs\""));
        assert!(output.result_json.contains("\"seed\": \"7\""));
        assert!(output.result_json.contains("\"seed\": \"9\""));
        assert!(output
            .result_json
            .contains("\"case_id_column\": \"case_id\""));
        assert!(output
            .result_json
            .contains("\"quantity_kind\": \"Irradiance\""));
        assert!(output.result_json.contains("\"display_unit\": \"W/m2\""));
        assert!(output.result_json.contains("\"row_hash_count\": 4"));
        assert!(output
            .result_json
            .contains("\"status\": \"generated_sample_table\""));
        assert!(output.result_json.contains("\"case_id\": \"case_001\""));
        assert!(output
            .result_json
            .contains("\"sample_table\": \"grid_samples\""));
        assert!(output.stdout.contains("samples=4"));
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_file_repro_profile_requires_generated_random_sample_seed() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-sampling-repro-seed");
        let build_root = repo_root
            .join("build")
            .join("runtime-sampling-repro-seed-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "samples = sample random\nwith {\n    count = 2\n    cooling_cop = uniform(2.5, 5.0)\n}\n",
        )
        .expect("write source");

        let error = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                profile: ExecutionProfile::Repro,
                ..RunOptions::default()
            },
        )
        .expect_err("repro profile should require seed");

        assert!(error.to_string().contains("E-SAMPLING-SEED-MISSING"));
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
        assert!(output
            .result_json
            .contains("\"case_dir\": \"outputs/case_001\""));
        assert!(output
            .result_json
            .contains("\"generated_input_file\": null"));
        assert!(output.result_json.contains("\"process_statuses\": []"));
        assert!(output.result_json.contains("\"result_files\": []"));
        assert!(output.result_json.contains("\"metrics\": []"));
        assert!(output.result_json.contains("\"failure_reason\": null"));
        assert!(output.result_json.contains("\"status\": \"pending\""));
        assert!(output.result_json.contains("\"case_tables\""));
        assert!(output.result_json.contains("\"case_count\": 2"));
        assert!(output.result_json.contains("\"pending_count\": 2"));
        assert!(output.result_json.contains("\"case_diagnostics\""));
        assert!(output.review_json.contains("\"case_manifests\""));
        assert!(output.review_json.contains("\"case_id\": \"case_001\""));
        assert!(output.review_json.contains("\"status\": \"pending\""));
        assert!(output.review_json.contains("\"case_tables\""));
        assert!(output
            .cache_manifest_json
            .contains("\"owner_kind\": \"case\""));
        assert!(output
            .cache_manifest_json
            .contains("\"owner_name\": \"case_001\""));
        assert!(output
            .cache_manifest_json
            .contains("\"status\": \"miss_observed\""));
        assert!(output.run_log_json.contains("\"owner_kind\": \"case\""));
        assert!(!virtual_path.exists());
    }

    #[test]
    fn run_source_records_case_table_duplicate_diagnostics() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-case-duplicates");
        let build_root = repo_root
            .join("build")
            .join("runtime-case-duplicates-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("samples")).expect("sample data dir");
        fs::write(
            source_dir.join("samples").join("design_samples.csv"),
            "case_id,cooling_cop\ncase_001,3.2\ncase_001,3.4\n",
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
            ),
            &build_root,
            &RunOptions::default(),
        )
        .expect("duplicate case run");

        assert!(output.result_json.contains("\"case_tables\""));
        assert!(output.result_json.contains("\"failed_count\": 2"));
        assert!(output
            .result_json
            .contains("\"duplicate_case_ids\": [\"case_001\"]"));
        assert!(output.result_json.contains("\"case_diagnostics\""));
        assert!(output.result_json.contains("\"E-CASE-ID-DUPLICATE\""));
        assert!(output.result_json.contains("duplicate case_id `case_001`"));
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
                "    {\"name\": \"patch_input\", \"command\": \"tool patch-input\", \"status\": \"success\"},\n",
                "    {\"name\": \"external_simulation\", \"command\": \"tool run-simulation\", \"status\": \"success\"}\n",
                "  ],\n",
                "  \"result_files\": [{\"path\": \"outputs/case_001/result.json\", \"sha256\": \"result-hash\", \"bytes\": 28}],\n",
                "  \"metrics\": {\"annual_electricity_kwh\": 42.5, \"peak_cooling_kw\": 7.25},\n",
                "  \"failure_reason\": null\n",
                "}\n",
            ),
        )
        .expect("case manifest");
        fs::write(
            source_dir
                .join("outputs")
                .join("result_collection_manifest.json"),
            concat!(
                "{\n",
                "  \"expected_case_ids\": [\"case_001\"],\n",
                "  \"result_case_ids\": [\"case_001\"],\n",
                "  \"missing_case_ids\": [],\n",
                "  \"failed_case_count\": 0,\n",
                "  \"status\": \"complete\"\n",
                "}\n",
            ),
        )
        .expect("case collection manifest");
        let source_path = source_dir.join("main.eng");
        let process_source = if cfg!(windows) {
            "case_manifest_result = run command \"cmd\"\nwith {\n    args = [\"/C\", \"echo\", \"case-manifest\"]\n    expected_outputs = [\"outputs/case_001/case_manifest.json\"]\n    artifact_kind = \"case_manifest\"\n}\n\ncollection_result = run command \"cmd\"\nwith {\n    args = [\"/C\", \"echo\", \"collect\"]\n    expected_outputs = [\"outputs/result_collection_manifest.json\"]\n    artifact_kind = \"result_collection\"\n}\n"
        } else {
            "case_manifest_result = run command \"sh\"\nwith {\n    args = [\"-c\", \"printf case-manifest\"]\n    expected_outputs = [\"outputs/case_001/case_manifest.json\"]\n    artifact_kind = \"case_manifest\"\n}\n\ncollection_result = run command \"sh\"\nwith {\n    args = [\"-c\", \"printf collect\"]\n    expected_outputs = [\"outputs/result_collection_manifest.json\"]\n    artifact_kind = \"result_collection\"\n}\n"
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
            .contains("\"command\": \"tool run-simulation\""));
        assert!(output
            .result_json
            .contains("\"result_files\": [\"outputs/case_001/result.json\"]"));
        assert!(output
            .result_json
            .contains("\"name\": \"annual_electricity_kwh\""));
        assert!(output.result_json.contains("\"value\": 42.5"));
        assert!(output.result_json.contains("\"failure_reason\": null"));
        assert!(output.result_json.contains("\"status\": \"succeeded\""));
        assert!(output.result_json.contains("\"case_tables\""));
        assert!(output.result_json.contains("\"succeeded_count\": 1"));
        assert!(output
            .result_json
            .contains("\"collection_manifest\": \"outputs/result_collection_manifest.json\""));
        assert!(output
            .result_json
            .contains("\"collection_status\": \"complete\""));
        assert!(output.result_json.contains("\"collected_case_count\": 1"));
        assert!(output.result_json.contains("\"failed_case_count\": 0"));
        assert!(output.result_json.contains("\"scheduler_hooks\""));
        assert!(output
            .result_json
            .contains("\"parallel_policy\": \"not_configured\""));
        assert!(output
            .result_json
            .contains("\"runner\": \"sequential_process_runner\""));
        let review: Value = serde_json::from_str(&output.review_json).expect("review json");
        let review_case = review
            .get("case_manifests")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .expect("review case manifest");
        assert_eq!(
            review_case.get("case_id").and_then(Value::as_str),
            Some("case_001")
        );
        assert_eq!(
            review_case
                .get("process_bindings")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(Value::as_str),
            Some("case_manifest_result")
        );
        assert_eq!(
            review_case.get("status").and_then(Value::as_str),
            Some("succeeded")
        );
        assert!(review
            .get("case_tables")
            .and_then(Value::as_array)
            .is_some_and(|items| !items.is_empty()));
        assert!(review_case.get("line").and_then(Value::as_u64).is_some());
        let output_manifest: Value =
            serde_json::from_str(&output.output_manifest_json).expect("output manifest json");
        let case_artifact = json_array_item_by_field(
            &output_manifest,
            "/artifact_registry/generated_files",
            "kind",
            "case_manifest",
        )
        .expect("case manifest artifact record");
        assert_eq!(
            case_artifact.get("class").and_then(Value::as_str),
            Some("case")
        );
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
        let result_json = serde_json::from_str::<Value>(&output.result_json).expect("result json");
        let validations = result_json
            .pointer("/typed_payload/validations")
            .and_then(Value::as_array)
            .expect("validation payload");
        assert_eq!(
            validations
                .iter()
                .filter(|validation| {
                    validation.get("status").and_then(Value::as_str) == Some("passed")
                })
                .count(),
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
        assert!(output.run_log_json.contains("\"kind\": \"process\""));
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
    fn run_file_records_process_cache_manifest() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-process-cache");
        let build_root = repo_root.join("build").join("runtime-process-cache-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        fs::create_dir_all(build_root.join("cache").join("stale-entry")).expect("stale cache");
        let source_path = source_dir.join("main.eng");
        let source = if cfg!(windows) {
            "process_result = run command \"cmd\"\nwith {\n    args = [\"/C\", \"echo\", \"process-cache\"]\n    tool_version = \"cmd-test 1.0\"\n    cache = true\n    cache_key = [\"process\", \"demo\", \"v1\"]\n}\n"
        } else {
            "process_result = run command \"sh\"\nwith {\n    args = [\"-c\", \"echo process-cache\"]\n    tool_version = \"sh-test 1.0\"\n    cache = true\n    cache_key = [\"process\", \"demo\", \"v1\"]\n}\n"
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

        assert!(output.cache_manifest_path.exists());
        assert!(output
            .cache_manifest_json
            .contains("\"format\": \"eng-cache-manifest-v1\""));
        assert!(output
            .cache_manifest_json
            .contains("\"owner_kind\": \"process\""));
        assert!(output
            .cache_manifest_json
            .contains("\"lookup_status\": \"miss\""));
        assert!(output.cache_manifest_json.contains("\"observed_hash\""));
        assert!(output.cache_manifest_json.contains("\"W-CACHE-STALE\""));
        assert!(output.cache_manifest_json.contains("stale-entry"));
        assert!(output.run_log_json.contains("\"cache_events\""));
        assert!(output.run_log_json.contains("\"cache_event_count\": 1"));
        assert!(output.review_json.contains("\"caches\""));
        assert!(output
            .review_json
            .contains("\"provenance\": \"runtime_cache_manifest\""));
        assert!(output
            .output_manifest_json
            .contains("\"kind\": \"cache_manifest\""));
        assert!(output.output_manifest_json.contains("\"class\": \"cache\""));
    }

    #[test]
    fn run_file_repro_profile_rejects_unhashed_cache_record() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-cache-repro-policy");
        let build_root = repo_root
            .join("build")
            .join("runtime-cache-repro-policy-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "cp = 4180 J/kg/K\nQ_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\nsplit = train_test_split(Q_coil, target=Q_coil, features=[T_supply, T_return, m_dot], test=0.5, seed=7)\nwith {\n    cache = true\n    cache_key = [\"split\", \"v1\"]\n}\n",
        )
        .expect("write source");

        let error = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                profile: ExecutionProfile::Repro,
                ..RunOptions::default()
            },
        )
        .expect_err("repro profile should reject unhashed cache record");

        assert!(error.to_string().contains("E-CACHE-UNHASHED-REPRO"));
    }

    #[test]
    fn cache_hash_mismatch_fails_with_cache_diagnostic() {
        let compiler_record = eng_compiler::CacheRecordInfo {
            owner_kind: "network_request".to_owned(),
            owner_name: "weather".to_owned(),
            cache_key: "weather|2026".to_owned(),
            cache_key_parts: vec!["weather".to_owned(), "2026".to_owned()],
            cache_key_hash: "cache-key-hash".to_owned(),
            cache_path: "cache/cache-key-hash".to_owned(),
            cache_dir: "cache".to_owned(),
            source_hash: "source-hash".to_owned(),
            expected_hash: Some("sha256:expected".to_owned()),
            observed_hash: Some("observed".to_owned()),
            status: "fixture_available".to_owned(),
            line: 12,
        };
        let record = cache_manifest_record(
            &compiler_record,
            &RuntimeData::default(),
            &[],
            Path::new("build"),
        );

        assert_eq!(record.status, "hash_mismatch");
        let error =
            ensure_cache_hashes_valid(&[record]).expect_err("hash mismatch should fail run");
        assert!(error.to_string().contains("E-CACHE-HASH-MISMATCH"));
        assert!(error.to_string().contains("network_request"));
        assert!(error.to_string().contains("weather"));
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
    fn run_file_records_process_spec_options() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-process-spec-options");
        let build_root = repo_root
            .join("build")
            .join("runtime-process-spec-options-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let source_path = source_dir.join("main.eng");
        let source = if cfg!(windows) {
            "process_result = run command \"cmd\"\nwith {\n    args = [\"/C\", \"echo %ENG_PROCESS_TEST%\"]\n    env = { ENG_PROCESS_TEST = \"process-env-ok\" }\n    timeout = 5 s\n    retry = 1\n    tool_version = \"cmd-test 1.0\"\n}\n"
        } else {
            "process_result = run command \"sh\"\nwith {\n    args = [\"-c\", \"printf $ENG_PROCESS_TEST\"]\n    env = { ENG_PROCESS_TEST = \"process-env-ok\" }\n    timeout = 5 s\n    retry = 1\n    tool_version = \"sh-test 1.0\"\n}\n"
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
        .expect("process spec option run");

        let process_results: Value =
            serde_json::from_str(&output.process_results_json).expect("process results json");
        let process = process_results
            .pointer("/processes/0")
            .expect("first process record");
        assert_eq!(
            process.pointer("/env/0").and_then(Value::as_str),
            Some("ENG_PROCESS_TEST")
        );
        assert_eq!(process.get("timeout").and_then(Value::as_str), Some("5 s"));
        assert_eq!(process.get("retry").and_then(Value::as_u64), Some(1));
        assert_eq!(
            process.get("attempt_count").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            process.get("allow_failure").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            process.get("timed_out").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(process.get("success").and_then(Value::as_bool), Some(true));
        assert!(process
            .get("stdout")
            .and_then(Value::as_str)
            .is_some_and(|stdout| stdout.contains("process-env-ok")));
        assert!(output.review_json.contains("\"env\""));
        assert!(output.review_json.contains("ENG_PROCESS_TEST"));
        assert!(output.review_json.contains("\"timeout\": \"5 s\""));
        assert!(output.review_json.contains("\"retry\": 1"));
        assert!(output.run_log_json.contains("\"env\""));
        assert!(output.run_log_json.contains("ENG_PROCESS_TEST"));
        assert!(output.run_log_json.contains("\"attempt_count\": 1"));
    }

    #[test]
    fn run_file_records_allowed_process_timeout() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-process-timeout");
        let build_root = repo_root
            .join("build")
            .join("runtime-process-timeout-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let source_path = source_dir.join("main.eng");
        let source = if cfg!(windows) {
            "process_result = run command \"powershell\"\nwith {\n    args = [\"-NoProfile\", \"-Command\", \"Start-Sleep -Seconds 2\"]\n    timeout = 1 s\n    retry = 1\n    allow_failure = true\n}\n"
        } else {
            "process_result = run command \"sh\"\nwith {\n    args = [\"-c\", \"sleep 2\"]\n    timeout = 1 s\n    retry = 1\n    allow_failure = true\n}\n"
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
        .expect("allowed process timeout");

        let process_results: Value =
            serde_json::from_str(&output.process_results_json).expect("process results json");
        let process = process_results
            .pointer("/processes/0")
            .expect("first process record");
        assert_eq!(
            process.get("status").and_then(Value::as_str),
            Some("timed_out_allowed")
        );
        assert_eq!(
            process.get("timed_out").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            process.get("allow_failure").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(process.get("retry").and_then(Value::as_u64), Some(1));
        assert_eq!(
            process.get("attempt_count").and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(process.get("success").and_then(Value::as_bool), Some(false));
        assert!(output.review_json.contains("\"timed_out\": true"));
        assert!(output.review_json.contains("\"attempt_count\": 2"));
        assert!(output
            .run_log_json
            .contains("\"status\": \"timed_out_allowed\""));
        assert!(output.run_log_json.contains("\"timed_out\": true"));
        assert!(output.run_log_json.contains("\"attempt_count\": 2"));
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
        let output_manifest: Value =
            serde_json::from_str(&output.output_manifest_json).expect("output manifest json");
        let db_artifact = json_array_item_by_field(
            &output_manifest,
            "/artifact_registry/generated_files",
            "kind",
            "db_write_manifest",
        )
        .expect("db write artifact record");
        assert_eq!(
            db_artifact.get("class").and_then(Value::as_str),
            Some("db_write")
        );
        assert!(output.run_log_json.contains("\"kind\": \"db_write\""));
        assert!(output
            .run_log_json
            .contains("\"target\": \"outputs/results.sqlite\""));
    }

    #[test]
    fn run_file_writes_sqlite_append_manifest() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-db-sqlite-append");
        let build_root = repo_root
            .join("build")
            .join("runtime-db-sqlite-append-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("data dir");
        fs::write(
            source_dir.join("data").join("results.csv"),
            "case_id,annual_electricity\ncase_001,1200\ncase_002,1350\n",
        )
        .expect("results csv");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema SimulationResult {\n",
                "    case_id: String\n",
                "    annual_electricity: Energy [kWh]\n",
                "}\n\n",
                "results = promote csv file(\"data/results.csv\") as SimulationResult\n",
                "db = open sqlite file(\"outputs/results.sqlite\")\n",
                "write results to db.table(\"simulation_results\")\n",
                "with {\n",
                "    mode = append\n",
                "}\n",
            ),
        )
        .expect("source");

        let output = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                save_artifacts: true,
                ..RunOptions::default()
            },
        )
        .expect("sqlite append run");

        let result: Value = serde_json::from_str(&output.result_json).expect("result json");
        let manifest = result
            .pointer("/typed_payload/db_manifests/0")
            .expect("db manifest");
        assert_eq!(
            manifest.get("binding").and_then(Value::as_str),
            Some("results")
        );
        assert_eq!(
            manifest.get("database").and_then(Value::as_str),
            Some("outputs/results.sqlite")
        );
        assert!(manifest.get("database_hash_before").is_some());
        assert!(manifest
            .get("database_hash_after")
            .and_then(Value::as_str)
            .is_some());
        assert_eq!(
            manifest.get("transaction_status").and_then(Value::as_str),
            Some("committed")
        );
        assert_eq!(
            manifest.get("schema_status").and_then(Value::as_str),
            Some("ok")
        );
        assert_eq!(
            manifest.pointer("/tables/0/mode").and_then(Value::as_str),
            Some("append")
        );
        assert_eq!(
            manifest
                .pointer("/tables/0/row_count")
                .and_then(Value::as_u64),
            Some(2)
        );

        let output_manifest: Value =
            serde_json::from_str(&output.output_manifest_json).expect("output manifest json");
        let db_artifact = json_array_item_by_field(
            &output_manifest,
            "/artifact_registry/generated_files",
            "kind",
            "sqlite_database",
        )
        .expect("sqlite artifact");
        assert_eq!(
            db_artifact.get("class").and_then(Value::as_str),
            Some("db_write")
        );
        assert!(output.output_manifest_json.contains("\"db_writes\""));

        let db_path = build_root
            .join("result")
            .join("outputs")
            .join("results.sqlite");
        let row_count = sqlite_query_scalar(&db_path, "SELECT COUNT(*) FROM simulation_results")
            .as_i64()
            .expect("row count");
        let metadata_count = sqlite_query_scalar(
            &db_path,
            "SELECT COUNT(*) FROM \"_eng_schema_metadata\" WHERE db_table = 'simulation_results'",
        )
        .as_i64()
        .expect("metadata count");
        assert_eq!(row_count, 2);
        assert_eq!(metadata_count, 2);
    }

    #[test]
    fn run_file_writes_sqlite_upsert_with_key() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-db-sqlite-upsert");
        let build_root = repo_root
            .join("build")
            .join("runtime-db-sqlite-upsert-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("data dir");
        fs::write(
            source_dir.join("data").join("results.csv"),
            "case_id,annual_electricity\ncase_001,1200\ncase_002,1350\n",
        )
        .expect("results csv");
        let db_path = build_root
            .join("result")
            .join("outputs")
            .join("results.sqlite");
        fs::create_dir_all(db_path.parent().expect("db parent")).expect("db dir");
        sqlite_execute(
            &db_path,
            &[
                "CREATE TABLE simulation_results (case_id TEXT PRIMARY KEY, annual_electricity REAL)",
                "INSERT INTO simulation_results (case_id, annual_electricity) VALUES ('case_001', 99.0)",
            ],
        );

        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema SimulationResult {\n",
                "    case_id: String\n",
                "    annual_electricity: Energy [kWh]\n",
                "}\n\n",
                "results = promote csv file(\"data/results.csv\") as SimulationResult\n",
                "db = open sqlite file(\"outputs/results.sqlite\")\n",
                "write results to db.table(\"simulation_results\")\n",
                "with {\n",
                "    mode = upsert\n",
                "    key = case_id\n",
                "}\n",
            ),
        )
        .expect("source");

        let output = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                save_artifacts: true,
                ..RunOptions::default()
            },
        )
        .expect("sqlite upsert run");

        let result: Value = serde_json::from_str(&output.result_json).expect("result json");
        let manifest = result
            .pointer("/typed_payload/db_manifests/0")
            .expect("db manifest");
        assert_eq!(
            manifest.pointer("/tables/0/mode").and_then(Value::as_str),
            Some("upsert")
        );
        assert_eq!(
            manifest.pointer("/tables/0/key/0").and_then(Value::as_str),
            Some("case_id")
        );
        assert!(manifest
            .get("database_hash_before")
            .and_then(Value::as_str)
            .is_some());
        assert!(manifest
            .get("database_hash_after")
            .and_then(Value::as_str)
            .is_some());

        let row_count = sqlite_query_scalar(&db_path, "SELECT COUNT(*) FROM simulation_results")
            .as_i64()
            .expect("row count");
        let updated = sqlite_query_scalar(
            &db_path,
            "SELECT annual_electricity FROM simulation_results WHERE case_id = 'case_001'",
        )
        .as_f64()
        .expect("updated row");
        assert_eq!(row_count, 2);
        assert_eq!(updated, 1200.0);
    }

    #[test]
    fn run_file_rolls_back_sqlite_transaction_policy() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-db-sqlite-rollback");
        let build_root = repo_root
            .join("build")
            .join("runtime-db-sqlite-rollback-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("data dir");
        fs::write(
            source_dir.join("data").join("results.csv"),
            "case_id,annual_electricity\ncase_001,1200\n",
        )
        .expect("results csv");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema SimulationResult {\n",
                "    case_id: String\n",
                "    annual_electricity: Energy [kWh]\n",
                "}\n\n",
                "results = promote csv file(\"data/results.csv\") as SimulationResult\n",
                "db = open sqlite file(\"outputs/results.sqlite\")\n",
                "write results to db.table(\"simulation_results\")\n",
                "with {\n",
                "    mode = append\n",
                "    transaction = rollback\n",
                "}\n",
            ),
        )
        .expect("source");

        let output = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                save_artifacts: true,
                ..RunOptions::default()
            },
        )
        .expect("sqlite rollback run");

        let result: Value = serde_json::from_str(&output.result_json).expect("result json");
        let manifest = result
            .pointer("/typed_payload/db_manifests/0")
            .expect("db manifest");
        assert_eq!(
            manifest.get("transaction_status").and_then(Value::as_str),
            Some("rolled_back")
        );
        let db_path = build_root
            .join("result")
            .join("outputs")
            .join("results.sqlite");
        let table_count = sqlite_query_scalar(
            &db_path,
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'simulation_results'",
        )
        .as_i64()
        .expect("table count");
        assert_eq!(table_count, 0);
    }

    #[test]
    fn run_file_records_sqlite_schema_mismatch_diagnostic() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-db-sqlite-mismatch");
        let build_root = repo_root
            .join("build")
            .join("runtime-db-sqlite-mismatch-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("data dir");
        fs::write(
            source_dir.join("data").join("results.csv"),
            "case_id,annual_electricity\ncase_001,1200\n",
        )
        .expect("results csv");
        let db_path = build_root
            .join("result")
            .join("outputs")
            .join("results.sqlite");
        fs::create_dir_all(db_path.parent().expect("db parent")).expect("db dir");
        sqlite_execute(
            &db_path,
            &["CREATE TABLE simulation_results (case_id TEXT, unexpected_column REAL)"],
        );

        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema SimulationResult {\n",
                "    case_id: String\n",
                "    annual_electricity: Energy [kWh]\n",
                "}\n\n",
                "results = promote csv file(\"data/results.csv\") as SimulationResult\n",
                "db = open sqlite file(\"outputs/results.sqlite\")\n",
                "write results to db.table(\"simulation_results\")\n",
                "with {\n",
                "    mode = append\n",
                "}\n",
            ),
        )
        .expect("source");

        let output = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                save_artifacts: true,
                ..RunOptions::default()
            },
        )
        .expect("sqlite mismatch run");

        let result: Value = serde_json::from_str(&output.result_json).expect("result json");
        let manifest = result
            .pointer("/typed_payload/db_manifests/0")
            .expect("db manifest");
        assert_eq!(
            manifest.get("schema_status").and_then(Value::as_str),
            Some("mismatch")
        );
        assert_eq!(
            manifest.get("transaction_status").and_then(Value::as_str),
            Some("rolled_back")
        );
        assert_eq!(
            manifest
                .pointer("/tables/0/row_count")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            manifest
                .pointer("/diagnostics/0/code")
                .and_then(Value::as_str),
            Some("E-DB-SCHEMA-MISMATCH")
        );
    }

    #[test]
    fn run_file_safe_profile_rejects_native_db_write() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-db-sqlite-safe");
        let build_root = repo_root
            .join("build")
            .join("runtime-db-sqlite-safe-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("data dir");
        fs::write(
            source_dir.join("data").join("results.csv"),
            "case_id,annual_electricity\ncase_001,1200\n",
        )
        .expect("results csv");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema SimulationResult {\n",
                "    case_id: String\n",
                "    annual_electricity: Energy [kWh]\n",
                "}\n\n",
                "results = promote csv file(\"data/results.csv\") as SimulationResult\n",
                "db = open sqlite file(\"outputs/results.sqlite\")\n",
                "write results to db.table(\"simulation_results\")\n",
            ),
        )
        .expect("source");

        let error = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                profile: ExecutionProfile::Safe,
                ..RunOptions::default()
            },
        )
        .expect_err("safe profile should reject native db write");

        assert!(error.to_string().contains("profile `safe` rejected"));
        assert!(error.to_string().contains("E-DB-SAFE-PROFILE"));
    }

    #[test]
    fn run_file_repro_profile_records_native_db_write() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-db-sqlite-repro");
        let build_root = repo_root
            .join("build")
            .join("runtime-db-sqlite-repro-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("data dir");
        fs::write(
            source_dir.join("data").join("results.csv"),
            "case_id,annual_electricity\ncase_001,1200\n",
        )
        .expect("results csv");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema SimulationResult {\n",
                "    case_id: String\n",
                "    annual_electricity: Energy [kWh]\n",
                "}\n\n",
                "results = promote csv file(\"data/results.csv\") as SimulationResult\n",
                "db = open sqlite file(\"outputs/results.sqlite\")\n",
                "write results to db.table(\"simulation_results\")\n",
            ),
        )
        .expect("source");

        let output = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                profile: ExecutionProfile::Repro,
                save_artifacts: true,
                ..RunOptions::default()
            },
        )
        .expect("repro DB write run");

        assert!(output.result_json.contains("W-PROFILE-REPRO-DB"));
        assert!(output.result_json.contains("\"database_hash_after\""));
        assert!(output.run_log_json.contains("W-PROFILE-REPRO-DB"));
    }

    #[test]
    fn run_file_classifies_model_expected_output_artifact() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root
            .join("build")
            .join("runtime-model-expected-output");
        let build_root = repo_root
            .join("build")
            .join("runtime-model-expected-output-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let source_path = source_dir.join("main.eng");
        let source = if cfg!(windows) {
            "model_result = run command \"cmd\"\nwith {\n    args = [\"/C\", \"if not exist outputs mkdir outputs && echo model>outputs/model.json\"]\n    expected_outputs = [\"outputs/model.json\"]\n    artifact_kind = \"model_artifact\"\n}\n"
        } else {
            "model_result = run command \"sh\"\nwith {\n    args = [\"-c\", \"mkdir -p outputs && printf model > outputs/model.json\"]\n    expected_outputs = [\"outputs/model.json\"]\n    artifact_kind = \"model_artifact\"\n}\n"
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
        .expect("model process run");

        assert!(output
            .process_results_json
            .contains("\"kind\": \"model_artifact\""));
        let output_manifest: Value =
            serde_json::from_str(&output.output_manifest_json).expect("output manifest json");
        let model_artifact = json_array_item_by_field(
            &output_manifest,
            "/artifact_registry/generated_files",
            "kind",
            "model_artifact",
        )
        .expect("model artifact record");
        assert_eq!(
            model_artifact.get("class").and_then(Value::as_str),
            Some("model")
        );
        assert!(output.result_json.contains("\"model_diagnostics\""));
        assert!(output.result_json.contains("\"E-MODEL-CARD-MISSING\""));
    }

    #[test]
    fn run_file_adapts_external_model_card_and_prediction_manifest() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-external-model-card");
        let build_root = repo_root
            .join("build")
            .join("runtime-external-model-card-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("outputs")).expect("outputs dir");
        fs::write(
            source_dir.join("outputs").join("model_card.json"),
            concat!(
                "{\n",
                "  \"model\": \"linear-fixture\",\n",
                "  \"model_kind\": \"surrogate_regression_fixture\",\n",
                "  \"features\": [\"cooling_cop\", \"lighting_power_density\"],\n",
                "  \"target\": \"annual_electricity\",\n",
                "  \"target_quantity\": \"Energy\",\n",
                "  \"target_unit\": \"kWh\",\n",
                "  \"train_test_split\": {\"test_fraction\": 0.2, \"seed\": 42, \"train_rows\": 2, \"test_rows\": 1},\n",
                "  \"metrics\": {\"rmse\": 0.0, \"mae\": 0.0, \"r2\": 1.0},\n",
                "  \"training_data_hash\": \"training-hash\",\n",
                "  \"model_artifact_hash\": \"model-hash\",\n",
                "  \"status\": \"fixture\"\n",
                "}\n",
            ),
        )
        .expect("model card");
        fs::write(
            source_dir.join("outputs").join("prediction_manifest.json"),
            concat!(
                "{\n",
                "  \"format\": \"prediction-manifest-v1\",\n",
                "  \"status\": \"complete\",\n",
                "  \"model\": \"linear-fixture\",\n",
                "  \"model_file\": {\"path\": \"outputs/surrogate.json\", \"sha256\": \"model-hash\", \"bytes\": 20},\n",
                "  \"sample_file\": {\"path\": \"samples/design_samples.csv\", \"sha256\": \"sample-hash\", \"bytes\": 30},\n",
                "  \"output_file\": {\"path\": \"outputs/predictions.csv\", \"sha256\": \"prediction-hash\", \"bytes\": 40},\n",
                "  \"schema\": [\"case_id\", \"predicted_annual_electricity\", \"prediction_confidence\"],\n",
                "  \"outputs\": [\n",
                "    {\"column\": \"predicted_annual_electricity\", \"quantity\": \"Energy\", \"unit\": \"kWh\"},\n",
                "    {\"column\": \"prediction_confidence\", \"quantity\": \"Ratio\", \"unit\": \"1\"}\n",
                "  ],\n",
                "  \"case_ids\": [\"case_001\", \"case_002\"],\n",
                "  \"row_count\": 2,\n",
                "  \"schema_mismatch_diagnostics\": []\n",
                "}\n",
            ),
        )
        .expect("prediction manifest");
        let source_path = source_dir.join("main.eng");
        let source = if cfg!(windows) {
            "trainer = run command \"cmd\"\nwith {\n    args = [\"/C\", \"echo\", \"trainer\"]\n    expected_outputs = [\"outputs/model_card.json\"]\n    artifact_kind = \"model_artifact\"\n}\n\npredictor = run command \"cmd\"\nwith {\n    args = [\"/C\", \"echo\", \"predictor\"]\n    expected_outputs = [\"outputs/prediction_manifest.json\"]\n    artifact_kind = \"prediction_result\"\n}\n"
        } else {
            "trainer = run command \"sh\"\nwith {\n    args = [\"-c\", \"printf trainer\"]\n    expected_outputs = [\"outputs/model_card.json\"]\n    artifact_kind = \"model_artifact\"\n}\n\npredictor = run command \"sh\"\nwith {\n    args = [\"-c\", \"printf predictor\"]\n    expected_outputs = [\"outputs/prediction_manifest.json\"]\n    artifact_kind = \"prediction_result\"\n}\n"
        };
        fs::write(&source_path, source).expect("write source");

        let output = run_file(&source_path, &build_root, &RunOptions::default())
            .expect("external model artifacts");

        assert!(output.result_json.contains("\"model_cards\""));
        assert!(output
            .result_json
            .contains("\"model_kind\": \"surrogate_regression_fixture\""));
        assert!(output.result_json.contains("\"model_specs\""));
        assert!(output.result_json.contains("\"features\""));
        assert!(output
            .result_json
            .contains("\"target_quantity\": \"Energy\""));
        assert!(output.result_json.contains("\"prediction_manifests\""));
        assert!(output
            .result_json
            .contains("\"confidence_column\": \"prediction_confidence\""));
        assert!(output
            .result_json
            .contains("\"model_artifact_hash\": \"model-hash\""));
        assert!(output.review_json.contains("\"model_specs\""));
        assert!(output.review_json.contains("\"prediction_manifests\""));
    }

    #[test]
    fn run_file_predicts_native_model_into_table_and_sqlite() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-native-model-predict");
        let build_root = repo_root
            .join("build")
            .join("runtime-native-model-predict-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("data dir");
        fs::write(
            source_dir.join("data").join("hvac_training.csv"),
            concat!(
                "time,T_supply,T_return,m_dot\n",
                "2026-01-01T00:00:00Z,7.1,12.4,0.22\n",
                "2026-01-01T00:05:00Z,7.4,12.6,0.23\n",
                "2026-01-01T00:10:00Z,7.7,12.9,0.23\n",
                "2026-01-01T00:15:00Z,7.9,13.3,0.24\n",
                "2026-01-01T00:20:00Z,8.2,13.8,0.25\n",
                "2026-01-01T00:25:00Z,8.5,14.0,0.26\n",
                "2026-01-01T00:30:00Z,8.8,14.4,0.26\n",
                "2026-01-01T00:35:00Z,9.1,14.7,0.27\n",
            ),
        )
        .expect("training csv");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema HvacTrainingData {\n",
                "    time: DateTime index\n",
                "    T_supply: AbsoluteTemperature [degC]\n",
                "    T_return: AbsoluteTemperature [degC]\n",
                "    m_dot: MassFlowRate [kg/s]\n",
                "}\n\n",
                "sensor = promote csv file(\"data/hvac_training.csv\") as HvacTrainingData\n",
                "cp = 4180 J/kg/K\n",
                "Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\n",
                "split = train_test_split(Q_coil, target=Q_coil, features=[T_supply, T_return, m_dot], test=0.25, seed=7)\n",
                "reg_model = regression(split, algorithm=linear)\n",
                "new_samples = sample lhs\n",
                "with {\n",
                "    count = 3\n",
                "    seed = 11\n",
                "    T_supply = uniform(7 degC, 9 degC)\n",
                "    T_return = uniform(12 degC, 15 degC)\n",
                "    m_dot = uniform(0.2 kg/s, 0.3 kg/s)\n",
                "}\n",
                "predictions = predict reg_model using new_samples\n",
                "db = open sqlite file(\"outputs/predictions.sqlite\")\n",
                "write predictions to db.table(\"predictions\")\n",
                "with {\n",
                "    mode = append\n",
                "}\n",
            ),
        )
        .expect("source");

        let output = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                save_artifacts: true,
                ..RunOptions::default()
            },
        )
        .expect("native prediction run");

        let result: Value = serde_json::from_str(&output.result_json).expect("result json");
        let manifest = result
            .pointer("/typed_payload/prediction_manifests/0")
            .expect("native prediction manifest");
        assert_eq!(
            manifest.get("binding").and_then(Value::as_str),
            Some("predictions")
        );
        assert_eq!(
            manifest.get("manifest_path").and_then(Value::as_str),
            Some("native:predictions")
        );
        assert_eq!(
            manifest.get("model").and_then(Value::as_str),
            Some("reg_model")
        );
        assert_eq!(manifest.get("row_count").and_then(Value::as_u64), Some(3));
        assert_eq!(
            manifest.get("confidence_column").and_then(Value::as_str),
            Some("confidence")
        );
        assert_eq!(
            manifest
                .pointer("/outputs/0/column")
                .and_then(Value::as_str),
            Some("predicted_q_coil")
        );
        assert_eq!(
            manifest
                .pointer("/outputs/0/quantity")
                .and_then(Value::as_str),
            Some("HeatRate")
        );
        assert_eq!(
            manifest
                .pointer("/outputs/1/column")
                .and_then(Value::as_str),
            Some("confidence")
        );

        let table_diagnostic = result
            .pointer("/typed_payload/table_diagnostics")
            .and_then(Value::as_array)
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| item.get("binding").and_then(Value::as_str) == Some("predictions"))
            })
            .expect("prediction table diagnostic");
        assert_eq!(
            table_diagnostic.get("schema_name").and_then(Value::as_str),
            Some("PredictionResult")
        );
        assert_eq!(
            table_diagnostic.get("row_count").and_then(Value::as_u64),
            Some(3)
        );
        let prediction_ml = result
            .pointer("/typed_payload/ml")
            .and_then(Value::as_array)
            .and_then(|items| {
                items.iter().find(|item| {
                    item.get("kind").and_then(Value::as_str) == Some("PredictionResult")
                })
            })
            .expect("prediction ml artifact");
        assert_eq!(
            prediction_ml
                .get("prediction_input")
                .and_then(Value::as_str),
            Some("new_samples")
        );
        assert_eq!(
            prediction_ml.get("status").and_then(Value::as_str),
            Some("predicted")
        );

        let db_path = build_root
            .join("result")
            .join("outputs")
            .join("predictions.sqlite");
        let row_count = sqlite_query_scalar(&db_path, "SELECT COUNT(*) FROM predictions")
            .as_i64()
            .expect("prediction row count");
        assert_eq!(row_count, 3);
    }

    #[test]
    fn run_file_records_model_artifact_records() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_path = repo_root.join("examples/internal/05_data_driven_modeling/main.eng");
        let build_root = repo_root
            .join("build")
            .join("runtime-model-artifact-records");
        let _ = fs::remove_dir_all(&build_root);
        let source = fs::read_to_string(&source_path)
            .expect("model source")
            .replace(
                "reg_model = regression(split, algorithm=linear)",
                "reg_model = regression(split, algorithm=linear)\nwith {\n    cache = true\n    cache_key = [\"model\", \"reg\", \"v1\"]\n}",
            );

        let output = run_source(&source_path, &source, &build_root, &RunOptions::default())
            .expect("run file");

        assert!(output.output_manifest_json.contains("\"model_artifacts\""));
        assert!(output.output_manifest_json.contains("\"artifact\""));
        assert!(output
            .output_manifest_json
            .contains("\"kind\": \"model_artifact\""));
        assert!(output.output_manifest_json.contains("\"class\": \"model\""));
        assert!(output
            .output_manifest_json
            .contains("\"model_artifact_hash\""));
        assert!(output.review_json.contains("\"model_cards\""));
        assert!(output.review_json.contains("\"model_kind\": \"linear\""));
        assert!(output.review_json.contains("\"residual_point_count\""));
        assert!(output.review_json.contains("\"model_artifact_hash\""));
        assert!(output.result_json.contains("\"model_specs\""));
        assert!(output.result_json.contains("\"model_diagnostics\""));
        assert!(output
            .cache_manifest_json
            .contains("\"owner_kind\": \"model\""));
        assert!(output
            .cache_manifest_json
            .contains("\"owner_name\": \"reg_model\""));
        assert!(output.cache_manifest_json.contains("\"observed_hash\""));
        assert!(output
            .cache_manifest_json
            .contains("\"status\": \"miss_observed\""));
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
    fn run_file_interpolates_write_text_string_literals() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-write-template");
        let build_root = repo_root
            .join("build")
            .join("runtime-write-template-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("sensor.csv"),
            "time,T_supply\n2024-01-01T00:00:00+09:00,20\n2024-01-01T01:00:00+09:00,21\n",
        )
        .expect("sensor csv");
        fs::write(
            source_dir.join("data").join("stations.csv"),
            "region,station_id,valid_from,valid_to\ndemo,STN001,2020-01-01T00:00:00+09:00,2030-12-31T23:00:00+09:00\n",
        )
        .expect("station csv");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "schema StationMap {\n    region: String\n    station_id: String\n    valid_from: DateTime\n    valid_to: DateTime\n}\n\nschema SensorData {\n    time: DateTime index\n    T_supply: AbsoluteTemperature [degC]\n}\n\nargs {\n    station_map: CsvFile = file(\"data/stations.csv\")\n    input: CsvFile = file(\"data/sensor.csv\")\n}\n\nstations = promote csv args.station_map as StationMap\nsensor = promote csv args.input as SensorData\nselected_station_id: String = select_first_row(stations, return_column=\"station_id\", region=\"demo\", start=date(2024, 1, 1), end=date(2024, 12, 31))\nwrite text \"summary.txt\", \"rows={sensor.rows} station={selected_station_id} input={args.input}\"\nwith {\n    overwrite = true\n}\n",
        )
        .expect("write source");

        run_file(&source_path, &build_root, &RunOptions::default()).expect("run file");

        let text =
            fs::read_to_string(build_root.join("result").join("summary.txt")).expect("summary");
        assert_eq!(text, "rows=2 station=STN001 input=data/sensor.csv");
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
            "args {\n    input: CsvFile = file(\".\\\\data\\\\sensor.csv\")\n    output: DirectoryPath = dir(\".\\\\build\\\\out\")\n}\n\ninput_exists = exists args.input\nsummary_file = join(args.output, \"summary.csv\")\ninput_parent = parent(args.input)\ninput_stem = stem(args.input)\ninput_ext = extension(args.input)\n\nprint \"exists={input_exists} summary={summary_file} parent={input_parent} stem={input_stem} ext={input_ext}\"\n",
        )
        .expect("write source");

        let output = run_file(&source_path, &build_root, &RunOptions::default()).expect("run file");

        assert!(output.stdout.contains("exists=true"));
        assert!(output.stdout.contains("summary=build/out/summary.csv"));
        assert!(output.stdout.contains("parent=data"));
        assert!(output.stdout.contains("stem=sensor"));
        assert!(output.stdout.contains("ext=csv"));
        assert!(output
            .result_json
            .contains("\"value\": \"data/sensor.csv\""));
        assert!(output.result_json.contains("\"value\": \"build/out\""));
        assert!(output.result_json.contains("\"environment_dependencies\""));
        assert!(output.result_json.contains("\"filesystem_exists\""));
        assert!(output.result_json.contains("\"resolved_value\": \"true\""));
        assert!(output
            .report_spec_json
            .contains("\"environment_dependencies\""));
        assert!(output.run_log_json.contains("\"source_path\""));
        assert!(output.run_log_json.contains("\"working_dir\""));
        assert!(output.run_log_json.contains("\"output_dir\""));
        assert!(output.output_manifest_json.contains("\"source_path\""));
        assert!(output.output_manifest_json.contains("\"working_dir\""));
        assert!(output.output_manifest_json.contains("\"output_dir\""));
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
        assert!(output.result_json.contains("\"binding\": \"notes_text\""));
        assert!(output.result_json.contains("\"binding\": \"json_text\""));
        assert!(output.result_json.contains("\"binding\": \"toml_text\""));
        assert!(output.result_json.contains("\"parse_status\": \"parsed\""));
        assert!(output.result_json.contains("\"root_type\": \"text\""));
        assert!(output.result_json.contains("\"root_type\": \"object\""));
        assert!(output.result_json.contains("\"root_type\": \"table\""));
        assert!(output.report_spec_json.contains("\"filesystem_read_text\""));
        assert!(output
            .output_manifest_json
            .contains("\"binding\": \"notes_text\""));
        assert!(output
            .output_manifest_json
            .contains("\"status\": \"parsed\""));
        assert!(output.output_manifest_json.contains("notes.txt"));
        assert!(output.output_manifest_json.contains("\"hash\": \""));
    }

    #[test]
    fn run_file_records_typed_config_promotion_artifacts() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-config-promotion");
        let build_root = repo_root
            .join("build")
            .join("runtime-config-promotion-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("workflow.toml"),
            "year = 2026\nregion = \"KR\"\noutput = \"build/out\"\ncache = true\n",
        )
        .expect("config");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "schema WorkflowConfig {\n    year: Int\n    region: String\n    output: DirectoryPath\n    cache: Bool\n}\n\nconfig = promote toml file(\"data/workflow.toml\") as WorkflowConfig\nx = 1\nprint \"x={x}\"\n",
        )
        .expect("write source");

        let output = run_file(&source_path, &build_root, &RunOptions::default()).expect("run file");
        let result_json = serde_json::from_str::<Value>(&output.result_json).expect("result json");

        assert!(output.stdout.contains("x=1"));
        assert_eq!(
            result_json
                .pointer("/typed_payload/config_promotions/0/status")
                .and_then(Value::as_str),
            Some("validated")
        );
        assert!(output.result_json.contains("\"config_promotions\""));
        assert!(output.result_json.contains("\"binding\": \"config\""));
        assert!(output.result_json.contains("\"format\": \"toml\""));
        assert!(output.result_json.contains("\"status\": \"validated\""));
        assert!(output.result_json.contains("\"config_promotion_count\": 1"));
        assert!(output
            .output_manifest_json
            .contains("\"kind\": \"config_toml\""));
        assert!(output
            .output_manifest_json
            .contains("\"schema\": \"WorkflowConfig\""));
        assert!(output.output_manifest_json.contains("workflow.toml"));
    }

    #[test]
    fn run_file_records_optional_config_field_artifacts() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-config-optional");
        let build_root = repo_root
            .join("build")
            .join("runtime-config-optional-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("workflow.json"),
            "{ \"year\": 2026, \"region\": null }\n",
        )
        .expect("config");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "schema WorkflowConfig {\n    year: Int\n    region: Optional[String]\n    output: DirectoryPath?\n}\n\nconfig = promote json file(\"data/workflow.json\") as WorkflowConfig\nx = 1\nprint \"x={x}\"\n",
        )
        .expect("write source");

        let output = run_file(&source_path, &build_root, &RunOptions::default()).expect("run file");
        let result_json = serde_json::from_str::<Value>(&output.result_json).expect("result json");

        assert!(output.stdout.contains("x=1"));
        assert_eq!(
            result_json
                .pointer("/typed_payload/config_promotions/0/status")
                .and_then(Value::as_str),
            Some("validated")
        );
        assert!(output.result_json.contains("\"optional_fields\""));
        assert!(output
            .result_json
            .contains("\"optional_missing_fields\": [\"output\"]"));
        assert!(output
            .result_json
            .contains("\"optional_null_fields\": [\"region\"]"));
        assert!(output.result_json.contains("\"missing_fields\": []"));
        assert!(output.result_json.contains("\"null_fields\": []"));
        assert!(output
            .output_manifest_json
            .contains("\"kind\": \"config_json\""));
    }

    #[test]
    fn run_file_records_nested_config_object_artifacts() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-config-nested");
        let build_root = repo_root.join("build").join("runtime-config-nested-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("workflow.json"),
            "{ \"year\": 2026, \"database\": { \"path\": \"outputs/results.sqlite\", \"transaction\": \"committed\", \"retry\": 3 } }\n",
        )
        .expect("config");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "schema DbConfig {\n    path: String\n    transaction: String\n    retry: Int\n}\n\nschema WorkflowConfig {\n    year: Int\n    database: DbConfig\n}\n\nconfig = promote json file(\"data/workflow.json\") as WorkflowConfig\nx = 1\nprint \"x={x}\"\n",
        )
        .expect("write source");

        let output = run_file(&source_path, &build_root, &RunOptions::default()).expect("run file");
        let result_json = serde_json::from_str::<Value>(&output.result_json).expect("result json");

        assert!(output.stdout.contains("x=1"));
        assert_eq!(
            result_json
                .pointer("/typed_payload/config_promotions/0/status")
                .and_then(Value::as_str),
            Some("validated")
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/config_promotions/0/nested_object_fields/0")
                .and_then(Value::as_str),
            Some("database")
        );
        assert!(output
            .result_json
            .contains("\"nested_object_fields\": [\"database\"]"));
        assert!(output
            .output_manifest_json
            .contains("\"schema\": \"WorkflowConfig\""));
    }

    #[test]
    fn run_file_records_array_config_artifacts() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-config-array");
        let build_root = repo_root.join("build").join("runtime-config-array-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("workflow.json"),
            "{ \"tags\": [\"alpha\", \"beta\"], \"retries\": [1, 2, 3] }\n",
        )
        .expect("config");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "schema WorkflowConfig {\n    tags: Array[String]\n    retries: List[Int]\n}\n\nconfig = promote json file(\"data/workflow.json\") as WorkflowConfig\nx = 1\nprint \"x={x}\"\n",
        )
        .expect("write source");

        let output = run_file(&source_path, &build_root, &RunOptions::default()).expect("run file");
        let result_json = serde_json::from_str::<Value>(&output.result_json).expect("result json");

        assert!(output.stdout.contains("x=1"));
        assert_eq!(
            result_json
                .pointer("/typed_payload/config_promotions/0/status")
                .and_then(Value::as_str),
            Some("validated")
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/config_promotions/0/array_fields/0")
                .and_then(Value::as_str),
            Some("tags")
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/config_promotions/0/array_fields/1")
                .and_then(Value::as_str),
            Some("retries")
        );
        assert!(output
            .result_json
            .contains("\"array_fields\": [\"tags\", \"retries\"]"));
    }

    #[test]
    fn run_file_records_default_config_field_artifacts() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-config-default");
        let build_root = repo_root
            .join("build")
            .join("runtime-config-default-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("workflow.json"),
            "{ \"year\": 2026 }\n",
        )
        .expect("config");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "schema WorkflowConfig {\n    year: Int\n    output: DirectoryPath = dir(\"build/out\")\n    cache: Bool = true\n}\n\nconfig = promote json file(\"data/workflow.json\") as WorkflowConfig\nx = 1\nprint \"x={x}\"\n",
        )
        .expect("write source");

        let output = run_file(&source_path, &build_root, &RunOptions::default()).expect("run file");
        let result_json = serde_json::from_str::<Value>(&output.result_json).expect("result json");

        assert!(output.stdout.contains("x=1"));
        assert_eq!(
            result_json
                .pointer("/typed_payload/config_promotions/0/status")
                .and_then(Value::as_str),
            Some("validated")
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/config_promotions/0/default_fields/0")
                .and_then(Value::as_str),
            Some("output")
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/config_promotions/0/defaulted_fields/1")
                .and_then(Value::as_str),
            Some("cache")
        );
        assert!(output
            .result_json
            .contains("\"defaulted_fields\": [\"output\", \"cache\"]"));
    }

    #[test]
    fn run_file_records_network_boundary_artifacts() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-net-boundary");
        let build_root = repo_root.join("build").join("runtime-net-boundary-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("response.json"),
            "{\"ok\":true}\n",
        )
        .expect("response");
        fs::write(
            source_dir.join("data").join("download.csv"),
            "id,value\n1,42\n",
        )
        .expect("download");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "response = http get url(\"https://api.example.org/hourly\")\nwith {\n    query = {\n    station = \"108\"\n    serviceKey = secret env(\"API_KEY\")\n    }\n    fixture = file(\"data/response.json\")\n    expected_sha256 = \"e5f1eb4d806641698a35efe20e098efd20d7d57a9b90ee69079d5bb650920726\"\n    retry = 2\n    timeout = 30 s\n    body_size_limit = 2 MB\n    cache = true\n    cache_key = [\"weather\", \"108\", \"2026\"]\n}\n\nresponse_text = response.body\nresponse_status = response.status\nresponse_code = response.status_code\n\ndownload url(\"https://example.org/file.csv\") to file(\"build/raw/file.csv\")\nwith {\n    fixture = file(\"data/download.csv\")\n    expected_sha256 = \"1c70e49dbdaf827d23f5bca1f5c2ec22cc98f102a09ddd4262af97893f101cc7\"\n    retry = 1\n    timeout = 1 min\n    response_body_limit = 512 KiB\n    cache = true\n    cache_key = [\"download\", \"v1\"]\n}\n\nx = 1\nprint \"x={x}\"\nprint \"body={response_text}\"\nprint \"status={response_status} code={response_code} hash={response.hash}\"\n",
        )
        .expect("write source");

        let output = run_file(&source_path, &build_root, &RunOptions::default()).expect("run file");
        let result_json = serde_json::from_str::<Value>(&output.result_json).expect("result json");

        assert!(output.stdout.contains("x=1"));
        assert!(output.stdout.contains("body={\"ok\":true}"));
        assert!(output.stdout.contains("status=fixture code=200"));
        assert_eq!(
            result_json
                .pointer("/typed_payload/network_boundaries/0/status")
                .and_then(Value::as_str),
            Some("fixture")
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/network_boundaries/0/status_code")
                .and_then(Value::as_u64),
            Some(200)
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/network_boundaries/0/status_class")
                .and_then(Value::as_str),
            Some("success")
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/network_boundaries/0/retry")
                .and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/network_boundaries/1/retry")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/network_boundaries/0/timeout")
                .and_then(Value::as_str),
            Some("30 s")
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/network_boundaries/1/timeout")
                .and_then(Value::as_str),
            Some("60 s")
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/network_boundaries/0/body_size_limit_bytes")
                .and_then(Value::as_u64),
            Some(2_000_000)
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/network_boundaries/1/body_size_limit_bytes")
                .and_then(Value::as_u64),
            Some(524_288)
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/network_boundaries/0/response_hash")
                .and_then(Value::as_str),
            Some("e5f1eb4d806641698a35efe20e098efd20d7d57a9b90ee69079d5bb650920726")
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/network_boundaries/1/response_hash")
                .and_then(Value::as_str),
            Some("1c70e49dbdaf827d23f5bca1f5c2ec22cc98f102a09ddd4262af97893f101cc7")
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/network_boundaries/0/expected_sha256")
                .and_then(Value::as_str),
            Some("e5f1eb4d806641698a35efe20e098efd20d7d57a9b90ee69079d5bb650920726")
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/network_boundaries/1/expected_sha256")
                .and_then(Value::as_str),
            Some("1c70e49dbdaf827d23f5bca1f5c2ec22cc98f102a09ddd4262af97893f101cc7")
        );
        assert!(output
            .static_run_plan_json
            .contains("\"body_size_limit_bytes\": 2000000"));
        assert!(output.result_json.contains("\"network_boundaries\""));
        assert!(output.result_json.contains("\"kind\": \"http_get\""));
        assert!(output.result_json.contains("\"kind\": \"download\""));
        assert!(output.result_json.contains("\"value\": \"<redacted>\""));
        assert!(output.result_json.contains("\"network_boundary_count\": 2"));
        assert!(output.run_log_json.contains("\"network_events\""));
        assert!(output
            .run_log_json
            .contains("\"external_boundary_event_count\": 2"));
        assert!(output
            .run_log_json
            .contains("\"kind\": \"network_request\""));
        assert!(output
            .run_log_json
            .contains("\"kind\": \"network_download\""));
        assert!(output.run_log_json.contains("\"network_event_count\": 2"));
        assert!(output.run_log_json.contains("\"cache_events\""));
        assert!(output.run_log_json.contains("\"cache_event_count\": 2"));
        assert!(output
            .cache_manifest_json
            .contains("\"cache_record_count\": 2"));
        assert!(output
            .cache_manifest_json
            .contains("\"cache_dir\": \"cache\""));
        assert!(output
            .cache_manifest_json
            .contains("\"lookup_status\": \"miss\""));
        assert!(output
            .cache_manifest_json
            .contains("\"status\": \"miss_materialized\""));
        assert!(output.output_manifest_json.contains("\"network_requests\""));
        assert!(output.output_manifest_json.contains("\"downloads\""));
        assert!(output.output_manifest_json.contains("\"caches\""));
        assert!(output.output_manifest_json.contains("data/response.json"));
        assert!(output.output_manifest_json.contains("data/download.csv"));
        assert!(output
            .output_manifest_json
            .contains("\"kind\": \"network_request\""));
        assert!(output.output_manifest_json.contains("build/raw/file.csv"));
        assert!(output.review_json.contains("\"external_boundaries\""));
        assert!(output.review_json.contains("\"caches\""));
        assert!(output
            .review_json
            .contains("\"provenance\": \"runtime_external_boundary_record\""));
        assert!(output.review_json.contains("\"lookup_status\": \"miss\""));
        assert!(output.review_json.contains("\"kind\": \"network_request\""));
        assert!(output
            .review_json
            .contains("\"kind\": \"network_download\""));

        fs::remove_file(source_dir.join("data").join("response.json")).expect("remove response");
        fs::remove_file(source_dir.join("data").join("download.csv")).expect("remove download");
        let cached_output =
            run_file(&source_path, &build_root, &RunOptions::default()).expect("cached run");
        let cached_result_json =
            serde_json::from_str::<Value>(&cached_output.result_json).expect("cached result json");

        assert_eq!(
            cached_result_json
                .pointer("/typed_payload/network_boundaries/0/status")
                .and_then(Value::as_str),
            Some("cached")
        );
        assert_eq!(
            cached_result_json
                .pointer("/typed_payload/network_boundaries/1/status")
                .and_then(Value::as_str),
            Some("cached")
        );
        assert_eq!(
            cached_result_json
                .pointer("/typed_payload/network_boundaries/0/response_hash")
                .and_then(Value::as_str),
            Some("e5f1eb4d806641698a35efe20e098efd20d7d57a9b90ee69079d5bb650920726")
        );
        assert!(cached_output
            .cache_manifest_json
            .contains("\"lookup_status\": \"hit\""));
        assert!(cached_output
            .cache_manifest_json
            .contains("\"status\": \"hit\""));
        assert!(cached_output.review_json.contains("\"status\": \"cached\""));
    }

    #[test]
    fn run_file_repro_profile_rejects_unpinned_network_boundaries() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-net-repro-unpinned");
        let build_root = repo_root
            .join("build")
            .join("runtime-net-repro-unpinned-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(&source_dir).expect("source dir");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "response = http get url(\"https://api.example.org/hourly\")\n\ndownload url(\"https://example.org/file.csv\") to file(\"build/raw/file.csv\")\n\nx = 1\nprint \"x={x}\"\n",
        )
        .expect("write source");

        let error = run_file(
            &source_path,
            &build_root,
            &RunOptions {
                profile: ExecutionProfile::Repro,
                ..RunOptions::default()
            },
        )
        .expect_err("repro profile should reject unpinned network boundaries");

        assert!(error.to_string().contains("profile `repro` rejected"));
        assert!(error.to_string().contains("E-NET-UNPINNED-REPRO"));
        assert!(error.to_string().contains("fixture and expected_sha256"));
    }

    #[test]
    fn run_file_redacts_secret_arg_values() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let source_dir = repo_root.join("build").join("runtime-net-secret-arg");
        let build_root = repo_root
            .join("build")
            .join("runtime-net-secret-arg-result");
        let _ = fs::remove_dir_all(&source_dir);
        let _ = fs::remove_dir_all(&build_root);
        fs::create_dir_all(source_dir.join("data")).expect("source data dir");
        fs::write(
            source_dir.join("data").join("response.json"),
            "{\"ok\":true}\n",
        )
        .expect("response");
        let source_path = source_dir.join("main.eng");
        fs::write(
            &source_path,
            "args {\n    api_key: Secret[String] = \"super-secret\"\n}\n\nresponse = http get url(\"https://api.example.org/hourly\")\nwith {\n    query = {\n    serviceKey = args.api_key\n    }\n    fixture = file(\"data/response.json\")\n}\n\nx = 1\nprint \"x={x}\"\n",
        )
        .expect("write source");

        let output = run_file(&source_path, &build_root, &RunOptions::default()).expect("run file");
        let result_json = serde_json::from_str::<Value>(&output.result_json).expect("result json");

        assert!(output.stdout.contains("x=1"));
        assert_eq!(
            result_json
                .pointer("/arg_values/0/type")
                .and_then(Value::as_str),
            Some("Secret[String]")
        );
        assert_eq!(
            result_json
                .pointer("/arg_values/0/value")
                .and_then(Value::as_str),
            Some("<redacted>")
        );
        assert_eq!(
            result_json
                .pointer("/arg_values/0/redacted")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/network_boundaries/0/query/0/value")
                .and_then(Value::as_str),
            Some("<redacted>")
        );
        assert_eq!(
            result_json
                .pointer("/typed_payload/network_boundaries/0/query/0/redacted")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(!output.result_json.contains("super-secret"));
        assert!(!output.review_json.contains("super-secret"));
    }
}
