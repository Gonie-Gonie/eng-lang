#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use eng_compiler::{
    all_quantity_completions, all_unit_infos, bundled_module_registry, check_source, format_source,
    CheckOptions, CheckReport, Severity,
};
use eng_runtime::{run_file, run_source, ExecutionProfile, RunOptions, RuntimeError};
use serde::Serialize;
use serde_json::{json, Value};
use tauri::State;

#[derive(Default)]
struct IdeState {
    last_output: Mutex<Option<CachedRunOutput>>,
    terminal_session_source: Mutex<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceView {
    root: String,
    file_tree: Vec<FileNodeView>,
    current: FileView,
    current_dir: String,
    check: CheckView,
    completions: Vec<CompletionView>,
    syntax_catalog: Value,
    modules: Vec<ModuleView>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileNodeView {
    name: String,
    path: String,
    kind: String,
    children: Vec<FileNodeView>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileView {
    path: String,
    source: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckView {
    diagnostics: Vec<DiagnosticView>,
    symbols: Vec<SymbolView>,
    status: String,
    semantic_tokens: Value,
    hovers: Value,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticView {
    severity: String,
    code: String,
    line: usize,
    message: String,
    help: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SymbolView {
    name: String,
    line: usize,
    quantity_kind: String,
    display_unit: String,
    canonical_unit: String,
    dimension: String,
    source: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CompletionView {
    label: String,
    insert: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    insert_snippet: Option<String>,
    detail: String,
    kind: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FormatView {
    source: String,
    changed: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModuleView {
    name: String,
    status: String,
    status_label: String,
    status_detail: String,
    backing: String,
    purpose: String,
    artifacts: Vec<String>,
    symbols: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunView {
    ok: bool,
    runtime_updated: bool,
    terminal: String,
    check: CheckView,
    variables: Vec<RuntimeVariableView>,
    args: Vec<RuntimeArgView>,
    artifacts: Vec<ArtifactView>,
    plot_spec: Value,
    report_title: String,
    inspectors: InspectorView,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactView {
    kind: String,
    path: String,
    status: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectorView {
    schemas: Value,
    unit_conversions: Value,
    time_axes: Value,
    time_series: Value,
    time_series_coverage: Value,
    metrics: Value,
    validations: Value,
    quality: Value,
    uncertainty: Value,
    time_alignments: Value,
    table_transforms: Value,
    structured_reads: Value,
    config_promotions: Value,
    systems: Value,
    system_ir: Value,
    linear_operators: Value,
    kernel_plan: Value,
    class_objects: Value,
    assemblies: Value,
    component_graph: Value,
    review_document: Value,
    artifact_outlines: Value,
    effect_records: Value,
    network_cache: Value,
    db_writes: Value,
    model_cards: Value,
    case_manifests: Value,
    output_manifest: Value,
    run_plan: Value,
    run_log: Value,
    process_results: Value,
    test_results: Value,
}

impl Default for InspectorView {
    fn default() -> Self {
        Self {
            schemas: Value::Array(Vec::new()),
            unit_conversions: Value::Array(Vec::new()),
            time_axes: Value::Array(Vec::new()),
            time_series: Value::Array(Vec::new()),
            time_series_coverage: Value::Array(Vec::new()),
            metrics: Value::Array(Vec::new()),
            validations: Value::Array(Vec::new()),
            quality: Value::Null,
            uncertainty: Value::Null,
            time_alignments: Value::Array(Vec::new()),
            table_transforms: Value::Array(Vec::new()),
            structured_reads: Value::Array(Vec::new()),
            config_promotions: Value::Array(Vec::new()),
            systems: Value::Array(Vec::new()),
            system_ir: Value::Array(Vec::new()),
            linear_operators: Value::Array(Vec::new()),
            kernel_plan: Value::Null,
            class_objects: Value::Array(Vec::new()),
            assemblies: Value::Array(Vec::new()),
            component_graph: Value::Null,
            review_document: Value::Null,
            artifact_outlines: Value::Array(Vec::new()),
            effect_records: Value::Null,
            network_cache: Value::Null,
            db_writes: Value::Null,
            model_cards: Value::Null,
            case_manifests: Value::Null,
            output_manifest: Value::Null,
            run_plan: Value::Null,
            run_log: Value::Null,
            process_results: Value::Null,
            test_results: Value::Null,
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeVariableView {
    name: String,
    quantity_kind: String,
    display_unit: String,
    canonical_unit: String,
    dimension: String,
    source: String,
    role: Option<String>,
    value: Option<String>,
    line: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeArgView {
    name: String,
    type_name: String,
    value: String,
    source: String,
    required: bool,
}

#[derive(Clone)]
struct CachedRunOutput {
    bytecode_path: PathBuf,
    result_path: PathBuf,
    review_path: PathBuf,
    static_run_plan_path: PathBuf,
    run_plan_path: PathBuf,
    run_lock_path: PathBuf,
    run_log_path: PathBuf,
    process_results_path: PathBuf,
    test_results_path: PathBuf,
    report_path: PathBuf,
    report_spec_path: PathBuf,
    plot_path: PathBuf,
    plot_spec_path: PathBuf,
    plot_manifest_path: PathBuf,
    output_manifest_path: PathBuf,
    artifacts_saved: bool,
    bytecode: String,
    result_json: String,
    review_json: String,
    static_run_plan_json: String,
    run_plan_json: String,
    run_lock_json: String,
    run_log_json: String,
    process_results_json: String,
    test_results_json: String,
    report_html: String,
    report_spec_json: String,
    plot_svg: String,
    plot_spec_json: String,
    plot_manifest_json: String,
    output_manifest_json: String,
}

impl CachedRunOutput {
    fn from_output(output: eng_runtime::RunOutput) -> Self {
        Self {
            bytecode_path: output.bytecode_path,
            result_path: output.result_path,
            review_path: output.review_path,
            static_run_plan_path: output.static_run_plan_path,
            run_plan_path: output.run_plan_path,
            run_lock_path: output.run_lock_path,
            run_log_path: output.run_log_path,
            process_results_path: output.process_results_path,
            test_results_path: output.test_results_path,
            report_path: output.report_path,
            report_spec_path: output.report_spec_path,
            plot_path: output.plot_path,
            plot_spec_path: output.plot_spec_path,
            plot_manifest_path: output.plot_manifest_path,
            output_manifest_path: output.output_manifest_path,
            artifacts_saved: output.artifacts_saved,
            bytecode: output.bytecode,
            result_json: output.result_json,
            review_json: output.review_json,
            static_run_plan_json: output.static_run_plan_json,
            run_plan_json: output.run_plan_json,
            run_lock_json: output.run_lock_json,
            run_log_json: output.run_log_json,
            process_results_json: output.process_results_json,
            test_results_json: output.test_results_json,
            report_html: output.report_html,
            report_spec_json: output.report_spec_json,
            plot_svg: output.plot_svg,
            plot_spec_json: output.plot_spec_json,
            plot_manifest_json: output.plot_manifest_json,
            output_manifest_json: output.output_manifest_json,
        }
    }

    fn save_artifacts(&mut self) -> Result<(), String> {
        create_parent(&self.bytecode_path)?;
        create_parent(&self.result_path)?;
        create_parent(&self.plot_path)?;
        fs::write(&self.bytecode_path, &self.bytecode).map_err(|error| error.to_string())?;
        fs::write(&self.result_path, &self.result_json).map_err(|error| error.to_string())?;
        fs::write(&self.review_path, &self.review_json).map_err(|error| error.to_string())?;
        fs::write(&self.static_run_plan_path, &self.static_run_plan_json)
            .map_err(|error| error.to_string())?;
        fs::write(&self.run_plan_path, &self.run_plan_json).map_err(|error| error.to_string())?;
        fs::write(&self.run_lock_path, &self.run_lock_json).map_err(|error| error.to_string())?;
        fs::write(&self.run_log_path, &self.run_log_json).map_err(|error| error.to_string())?;
        fs::write(&self.process_results_path, &self.process_results_json)
            .map_err(|error| error.to_string())?;
        fs::write(&self.test_results_path, &self.test_results_json)
            .map_err(|error| error.to_string())?;
        fs::write(&self.report_path, &self.report_html).map_err(|error| error.to_string())?;
        fs::write(&self.report_spec_path, &self.report_spec_json)
            .map_err(|error| error.to_string())?;
        fs::write(&self.plot_path, &self.plot_svg).map_err(|error| error.to_string())?;
        fs::write(&self.plot_spec_path, &self.plot_spec_json).map_err(|error| error.to_string())?;
        fs::write(&self.plot_manifest_path, &self.plot_manifest_json)
            .map_err(|error| error.to_string())?;
        fs::write(&self.output_manifest_path, &self.output_manifest_json)
            .map_err(|error| error.to_string())?;
        self.artifacts_saved = true;
        Ok(())
    }
}

#[tauri::command]
fn ide_bootstrap() -> Result<WorkspaceView, String> {
    let root = workspace_root();
    let current_path = default_file(&root);
    let source = read_utf8(&current_path)?;
    let check = check_view(&current_path, &source);
    Ok(WorkspaceView {
        root: root.display().to_string(),
        file_tree: workspace_tree(&root),
        current: FileView {
            path: relative_to(&root, &current_path),
            source,
        },
        current_dir: relative_to(&root, source_dir(&current_path)),
        check,
        completions: base_completion_items(),
        syntax_catalog: eng_lsp::editor_syntax_catalog_json(),
        modules: module_browser_items(),
    })
}

#[tauri::command]
fn ide_open_file(path: String) -> Result<FileView, String> {
    let root = workspace_root();
    let path = resolve_path(&root, &path);
    Ok(FileView {
        path: relative_to(&root, &path),
        source: read_utf8(&path)?,
    })
}

#[tauri::command]
fn ide_save_file(path: String, source: String) -> Result<FileView, String> {
    let root = workspace_root();
    let path = resolve_path(&root, &path);
    create_parent(&path)?;
    fs::write(&path, source.as_bytes()).map_err(|error| error.to_string())?;
    Ok(FileView {
        path: relative_to(&root, &path),
        source,
    })
}

#[tauri::command]
fn ide_check(path: String, source: String) -> CheckView {
    let root = workspace_root();
    let path = resolve_path(&root, &path);
    check_view(&path, &source)
}

#[tauri::command]
fn ide_format(path: String, source: String) -> FormatView {
    let _ = path;
    let result = format_source(&source);
    FormatView {
        source: result.formatted,
        changed: result.changed,
    }
}

#[tauri::command]
fn ide_run(
    path: String,
    source: String,
    profile: Option<String>,
    state: State<'_, IdeState>,
) -> Result<RunView, String> {
    let root = workspace_root();
    let path = resolve_path(&root, &path);
    let profile = ide_profile(profile.as_deref())?;
    create_parent(&path)?;
    fs::write(&path, source.as_bytes()).map_err(|error| error.to_string())?;
    let check = check_view(&path, &source);
    if check
        .diagnostics
        .iter()
        .any(|item| item.severity == "error")
    {
        return Ok(RunView {
            ok: false,
            runtime_updated: false,
            terminal: "Run blocked by diagnostics. See Problems.".to_owned(),
            check,
            variables: Vec::new(),
            args: Vec::new(),
            artifacts: Vec::new(),
            plot_spec: Value::Null,
            report_title: String::new(),
            inspectors: InspectorView::default(),
        });
    }
    run_source_file(&root, &path, check, profile, state)
}

#[tauri::command]
fn ide_terminal(
    path: String,
    source: String,
    command: String,
    run_dir: Option<String>,
    profile: Option<String>,
    state: State<'_, IdeState>,
) -> Result<RunView, String> {
    let trimmed = command.trim();
    let root = workspace_root();
    let current_path = resolve_path(&root, &path);
    let profile_value = ide_profile(profile.as_deref())?;
    let run_dir_path =
        if let Some(value) = run_dir.as_deref().filter(|value| !value.trim().is_empty()) {
            let path = resolve_path(&root, value);
            if !path.is_dir() {
                return Err(format!("Run directory does not exist: {}", path.display()));
            }
            path
        } else {
            source_dir(&current_path).to_path_buf()
        };
    if trimmed.eq_ignore_ascii_case("clear") || trimmed.eq_ignore_ascii_case("cls") {
        return Ok(RunView::message("Terminal cleared."));
    }
    if trimmed.eq_ignore_ascii_case("reset") {
        *state
            .terminal_session_source
            .lock()
            .map_err(|error| error.to_string())? = String::new();
        return Ok(RunView {
            ok: true,
            runtime_updated: true,
            terminal: "Terminal session reset.".to_owned(),
            check: CheckView {
                diagnostics: Vec::new(),
                symbols: Vec::new(),
                status: "ok".to_owned(),
                semantic_tokens: empty_semantic_tokens_view(),
                hovers: empty_hovers_view(),
            },
            variables: Vec::new(),
            args: Vec::new(),
            artifacts: Vec::new(),
            plot_spec: Value::Null,
            report_title: String::new(),
            inspectors: InspectorView::default(),
        });
    }
    if trimmed.eq_ignore_ascii_case("check") {
        let check = check_view(&current_path, &source);
        return Ok(RunView {
            ok: true,
            runtime_updated: false,
            terminal: diagnostic_summary_text(&check),
            check,
            variables: Vec::new(),
            args: Vec::new(),
            artifacts: Vec::new(),
            plot_spec: Value::Null,
            report_title: String::new(),
            inspectors: InspectorView::default(),
        });
    }
    if trimmed.eq_ignore_ascii_case("run") {
        return ide_run(path, source, Some(profile_value.as_str().to_owned()), state);
    }

    if let Some(check) = terminal_command_error(trimmed)
        .or_else(|| terminal_unrecognized_command_error(trimmed, &run_dir_path))
    {
        return Ok(RunView {
            ok: false,
            runtime_updated: false,
            terminal: diagnostic_summary_text(&check),
            check,
            variables: Vec::new(),
            args: Vec::new(),
            artifacts: Vec::new(),
            plot_spec: Value::Null,
            report_title: String::new(),
            inspectors: InspectorView::default(),
        });
    }

    let (session_source, session_path) = {
        let session = state
            .terminal_session_source
            .lock()
            .map_err(|error| error.to_string())?;
        let mut candidate = session.clone();
        if !candidate.trim().is_empty() {
            candidate.push('\n');
        }
        candidate.push_str(trimmed);
        candidate.push('\n');
        let session_path = run_dir_path.join("__ide_terminal__.eng");
        (candidate, session_path)
    };
    let check = check_view(&session_path, &session_source);
    if check
        .diagnostics
        .iter()
        .any(|item| item.severity == "error")
    {
        return Ok(RunView {
            ok: false,
            runtime_updated: false,
            terminal: diagnostic_summary_text(&check),
            check,
            variables: Vec::new(),
            args: Vec::new(),
            artifacts: Vec::new(),
            plot_spec: Value::Null,
            report_title: String::new(),
            inspectors: InspectorView::default(),
        });
    }
    let mut view = run_virtual_source_file(
        &root,
        &session_path,
        &session_source,
        check,
        profile_value,
        state.clone(),
    )?;
    if view.ok {
        *state
            .terminal_session_source
            .lock()
            .map_err(|error| error.to_string())? = session_source;
        if view.variables.is_empty() && view.args.is_empty() && !has_plot_data(&view.plot_spec) {
            view.runtime_updated = false;
            view.artifacts.clear();
            view.report_title.clear();
        }
    }
    Ok(view)
}

#[tauri::command]
fn ide_open_artifact(kind: String, state: State<'_, IdeState>) -> Result<String, String> {
    let root = workspace_root();
    let mut guard = state
        .last_output
        .lock()
        .map_err(|error| error.to_string())?;
    let Some(output) = guard.as_mut() else {
        return Err("No run output is available.".to_owned());
    };
    if !output.artifacts_saved {
        output.save_artifacts()?;
    }
    let path = match kind.as_str() {
        "result" => output.result_path.clone(),
        "review" => output.review_path.clone(),
        "static_run_plan" => output.static_run_plan_path.clone(),
        "run_plan" => output.run_plan_path.clone(),
        "run_lock" => output.run_lock_path.clone(),
        "run_log" => output.run_log_path.clone(),
        "process_results" => output.process_results_path.clone(),
        "test_results" => output.test_results_path.clone(),
        "output_manifest" => output.output_manifest_path.clone(),
        "report" => output.report_path.clone(),
        "report_spec" => output.report_spec_path.clone(),
        "plot" | "plot_svg" => output.plot_path.clone(),
        "plot_spec" => output.plot_spec_path.clone(),
        "plot_manifest" => output.plot_manifest_path.clone(),
        "output_folder" => output
            .output_manifest_path
            .parent()
            .unwrap_or(&output.output_manifest_path)
            .to_path_buf(),
        _ => output.report_path.clone(),
    };
    let relative = relative_to(&root, &path);
    open_path(&path);
    Ok(relative)
}

#[tauri::command]
fn ide_open_path(path: String, state: State<'_, IdeState>) -> Result<String, String> {
    let root = workspace_root();
    let path = resolve_path(&root, &path);
    if !path.exists() {
        let mut guard = state
            .last_output
            .lock()
            .map_err(|error| error.to_string())?;
        if let Some(output) = guard.as_mut() {
            if !output.artifacts_saved {
                output.save_artifacts()?;
            }
        }
    }
    if !path.exists() {
        return Err(format!(
            "Path does not exist: {}",
            relative_to(&root, &path)
        ));
    }
    let relative = relative_to(&root, &path);
    open_path(&path);
    Ok(relative)
}

impl RunView {
    fn message(text: impl Into<String>) -> Self {
        Self {
            ok: true,
            runtime_updated: false,
            terminal: text.into(),
            check: CheckView {
                diagnostics: Vec::new(),
                symbols: Vec::new(),
                status: "ok".to_owned(),
                semantic_tokens: empty_semantic_tokens_view(),
                hovers: empty_hovers_view(),
            },
            variables: Vec::new(),
            args: Vec::new(),
            artifacts: Vec::new(),
            plot_spec: Value::Null,
            report_title: String::new(),
            inspectors: InspectorView::default(),
        }
    }
}

fn main() {
    if env::args().any(|arg| arg == "--smoke" || arg == "smoke") {
        if let Err(error) = smoke() {
            eprintln!("EngLang IDE smoke failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    tauri::Builder::default()
        .manage(IdeState::default())
        .invoke_handler(tauri::generate_handler![
            ide_bootstrap,
            ide_open_file,
            ide_save_file,
            ide_check,
            ide_format,
            ide_run,
            ide_terminal,
            ide_open_path,
            ide_open_artifact
        ])
        .run(tauri::generate_context!())
        .expect("error while running EngLang IDE");
}

fn run_source_file(
    root: &Path,
    path: &Path,
    check: CheckView,
    profile: ExecutionProfile,
    state: State<'_, IdeState>,
) -> Result<RunView, String> {
    let build_root = root.join("build").join("ide-tauri-run");
    match run_file(
        path,
        &build_root,
        &RunOptions {
            open_report: false,
            save_artifacts: false,
            skip_unchanged: false,
            args: Vec::new(),
            profile,
        },
    ) {
        Ok(output) => {
            let stdout = output.stdout.clone();
            let cached = CachedRunOutput::from_output(output);
            let variables = runtime_variables(&cached);
            let args = runtime_args(&cached.report_spec_json);
            let report_title = report_title(&cached.report_spec_json);
            let plot_spec = serde_json::from_str(&cached.plot_spec_json).unwrap_or(Value::Null);
            let plot_spec = plot_spec_or_null(plot_spec);
            let terminal = terminal_summary(&stdout, &variables, &args, &report_title, &plot_spec);
            let artifacts = runtime_artifacts(root, &cached);
            let inspectors = runtime_inspectors(root, &cached);
            *state
                .last_output
                .lock()
                .map_err(|error| error.to_string())? = Some(cached);
            Ok(RunView {
                ok: true,
                runtime_updated: true,
                terminal,
                check,
                variables,
                args,
                artifacts,
                plot_spec,
                report_title,
                inspectors,
            })
        }
        Err(RuntimeError::Compile(report)) => {
            let check = check_view_from_report(&report, None);
            Ok(RunView {
                ok: false,
                runtime_updated: false,
                terminal: diagnostic_summary_text(&check),
                check,
                variables: Vec::new(),
                args: Vec::new(),
                artifacts: Vec::new(),
                plot_spec: Value::Null,
                report_title: String::new(),
                inspectors: InspectorView::default(),
            })
        }
        Err(error) => Err(error.to_string()),
    }
}

fn run_virtual_source_file(
    root: &Path,
    path: &Path,
    source: &str,
    check: CheckView,
    profile: ExecutionProfile,
    state: State<'_, IdeState>,
) -> Result<RunView, String> {
    let build_root = root.join("build").join("ide-tauri-terminal");
    match run_source(
        path,
        source,
        &build_root,
        &RunOptions {
            open_report: false,
            save_artifacts: false,
            skip_unchanged: false,
            args: Vec::new(),
            profile,
        },
    ) {
        Ok(output) => {
            let stdout = output.stdout.clone();
            let cached = CachedRunOutput::from_output(output);
            let variables = runtime_variables(&cached);
            let args = runtime_args(&cached.report_spec_json);
            let report_title = report_title(&cached.report_spec_json);
            let plot_spec = serde_json::from_str(&cached.plot_spec_json).unwrap_or(Value::Null);
            let plot_spec = plot_spec_or_null(plot_spec);
            let terminal = terminal_summary(&stdout, &variables, &args, &report_title, &plot_spec);
            let artifacts = runtime_artifacts(root, &cached);
            let inspectors = runtime_inspectors(root, &cached);
            *state
                .last_output
                .lock()
                .map_err(|error| error.to_string())? = Some(cached);
            Ok(RunView {
                ok: true,
                runtime_updated: true,
                terminal,
                check,
                variables,
                args,
                artifacts,
                plot_spec,
                report_title,
                inspectors,
            })
        }
        Err(RuntimeError::Compile(report)) => {
            let check = check_view_from_report(&report, None);
            Ok(RunView {
                ok: false,
                runtime_updated: false,
                terminal: diagnostic_summary_text(&check),
                check,
                variables: Vec::new(),
                args: Vec::new(),
                artifacts: Vec::new(),
                plot_spec: Value::Null,
                report_title: String::new(),
                inspectors: InspectorView::default(),
            })
        }
        Err(error) => Err(error.to_string()),
    }
}

fn check_view(path: &Path, source: &str) -> CheckView {
    let report = check_source(path, source, &CheckOptions::default());
    check_view_from_report(&report, Some(source))
}

fn check_view_from_report(report: &CheckReport, source: Option<&str>) -> CheckView {
    let diagnostics: Vec<DiagnosticView> = report
        .diagnostics
        .iter()
        .map(|diagnostic| DiagnosticView {
            severity: diagnostic.severity.as_str().to_owned(),
            code: diagnostic.code.clone(),
            line: diagnostic.line,
            message: diagnostic.message.clone(),
            help: diagnostic.help.clone(),
        })
        .collect();
    let symbols = report
        .semantic_program
        .hover_hints
        .iter()
        .map(|hover| {
            let type_info = report
                .semantic_program
                .type_infos
                .iter()
                .find(|info| info.name == hover.name && info.line == hover.line);
            SymbolView {
                name: hover.name.clone(),
                line: hover.line,
                quantity_kind: hover.quantity_kind.clone(),
                display_unit: hover.display_unit.clone(),
                canonical_unit: type_info
                    .map(|info| info.canonical_unit.clone())
                    .unwrap_or_else(|| hover.display_unit.clone()),
                dimension: type_info
                    .map(|info| info.dimension.clone())
                    .unwrap_or_else(|| "-".to_owned()),
                source: type_info
                    .map(|info| info.source.as_str().to_owned())
                    .unwrap_or_else(|| "symbol".to_owned()),
            }
        })
        .collect();
    let errors = report.diagnostic_count(Severity::Error);
    let warnings = report.diagnostic_count(Severity::Warning);
    let (semantic_tokens, hovers) = editor_payload_view(report, source);
    CheckView {
        diagnostics,
        symbols,
        status: format!("{errors} error(s), {warnings} warning(s)"),
        semantic_tokens,
        hovers,
    }
}

fn editor_payload_view(report: &CheckReport, source: Option<&str>) -> (Value, Value) {
    let Some(source) = source else {
        return (empty_semantic_tokens_view(), empty_hovers_view());
    };
    let snapshot = eng_lsp::snapshot_from_report_with_source(report, Some(source));
    (
        eng_lsp::semantic_tokens_json(&snapshot.semantic_tokens),
        Value::Array(snapshot.hovers.iter().map(eng_lsp::hover_json).collect()),
    )
}

fn empty_semantic_tokens_view() -> Value {
    eng_lsp::semantic_tokens_json(&eng_lsp::LspSemanticTokens::default())
}

fn empty_hovers_view() -> Value {
    Value::Array(Vec::new())
}

fn base_completion_items() -> Vec<CompletionView> {
    let mut items = eng_lsp::editor_completion_items()
        .into_iter()
        .map(CompletionView::from_lsp)
        .collect::<Vec<_>>();
    for snippet in [
        (
            "promote csv",
            "promote csv \"data/sensor.csv\" as SensorData",
            "CSV promotion command",
        ),
        (
            "plot line",
            "plot Q over Time with {\n    type = line\n    title = \"Heat rate\"\n}",
            "PlotSpec line plot block",
        ),
        (
            "log info",
            "log info \"message\"",
            "structured run log message",
        ),
        (
            "class object",
            "class Construction {\n    name: String\n    u_value: Conductance [W/K]\n    validate {\n        u_value > 0 W/K\n    }\n    method summary() -> String = self.name\n}\n\nwall = Construction {\n    name = \"south_wall\"\n    u_value = 120 W/K\n}\n\nbetter_wall = wall with {\n    u_value = 100 W/K\n}",
            "class declaration, validation, method, object literal, and copy-with",
        ),
    ] {
        push_native_completion(&mut items, CompletionView {
            label: snippet.0.to_owned(),
            insert: snippet.1.to_owned(),
            insert_snippet: None,
            detail: snippet.2.to_owned(),
            kind: "snippet".to_owned(),
        });
    }
    items
}

impl CompletionView {
    fn from_lsp(completion: eng_lsp::LspCompletion) -> Self {
        let insert = completion
            .insert
            .clone()
            .unwrap_or_else(|| completion.label.clone());
        Self {
            insert,
            insert_snippet: completion.insert_snippet,
            label: completion.label,
            detail: completion.detail,
            kind: completion.kind,
        }
    }
}

fn push_native_completion(items: &mut Vec<CompletionView>, completion: CompletionView) {
    if !items.iter().any(|item| item.label == completion.label) {
        items.push(completion);
    }
}

fn module_browser_items() -> Vec<ModuleView> {
    bundled_module_registry()
        .map(|registry| {
            registry
                .modules
                .into_iter()
                .map(|module| {
                    let status_label = module.status_label().to_owned();
                    let status_detail = module.status_detail().to_owned();
                    ModuleView {
                        name: module.name,
                        status_label,
                        status_detail,
                        status: module.status,
                        backing: module.backing,
                        purpose: module.purpose,
                        artifacts: module.artifacts,
                        symbols: module.symbols,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn runtime_variables(output: &CachedRunOutput) -> Vec<RuntimeVariableView> {
    let mut variables = Vec::new();
    if let Ok(value) = serde_json::from_str::<Value>(&output.result_json) {
        if let Some(items) = value
            .get("object_store")
            .and_then(|store| store.get("objects"))
            .and_then(Value::as_array)
        {
            for item in items {
                merge_variable(&mut variables, runtime_object_variable(item));
            }
        }
    }
    if let Ok(value) = serde_json::from_str::<Value>(&output.report_spec_json) {
        if let Some(items) = value.get("variable_table").and_then(Value::as_array) {
            for item in items {
                merge_variable(&mut variables, report_variable(item));
            }
        }
        if let Some(items) = value.get("uncertainty").and_then(Value::as_array) {
            for item in items {
                merge_variable(&mut variables, uncertainty_variable(item));
            }
        }
        if let Some(systems) = value.get("system_summary").and_then(Value::as_array) {
            for system in systems {
                let source =
                    json_field_string(system, "name").unwrap_or_else(|| "system".to_owned());
                if let Some(items) = system.get("variables").and_then(Value::as_array) {
                    for item in items {
                        merge_variable(&mut variables, system_variable(item, &source));
                    }
                }
            }
        }
    }
    variables
}

fn runtime_args(text: &str) -> Vec<RuntimeArgView> {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    value
        .get("arg_values")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| RuntimeArgView {
                    name: json_field_string(item, "name").unwrap_or_else(|| "unknown".to_owned()),
                    type_name: json_field_string(item, "type").unwrap_or_default(),
                    value: json_field_string(item, "value").unwrap_or_default(),
                    source: json_field_string(item, "source")
                        .unwrap_or_else(|| "default".to_owned()),
                    required: item
                        .get("required")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn runtime_artifacts(root: &Path, output: &CachedRunOutput) -> Vec<ArtifactView> {
    let status = if output.artifacts_saved {
        "saved"
    } else {
        "memory"
    };
    [
        ("result", &output.result_path),
        ("review", &output.review_path),
        ("static_run_plan", &output.static_run_plan_path),
        ("run_plan", &output.run_plan_path),
        ("run_lock", &output.run_lock_path),
        ("run_log", &output.run_log_path),
        ("process_results", &output.process_results_path),
        ("test_results", &output.test_results_path),
        ("output_manifest", &output.output_manifest_path),
        ("report", &output.report_path),
        ("report_spec", &output.report_spec_path),
        ("plot_svg", &output.plot_path),
        ("plot_spec", &output.plot_spec_path),
        ("plot_manifest", &output.plot_manifest_path),
    ]
    .into_iter()
    .map(|(kind, path)| ArtifactView {
        kind: kind.to_owned(),
        path: relative_to(root, path),
        status: status.to_owned(),
    })
    .collect()
}

fn ide_profile(value: Option<&str>) -> Result<ExecutionProfile, String> {
    let value = value.unwrap_or("normal");
    ExecutionProfile::parse(value).ok_or_else(|| {
        format!("unknown execution profile `{value}`; expected safe, normal, or repro")
    })
}

fn runtime_inspectors(root: &Path, output: &CachedRunOutput) -> InspectorView {
    let report = parse_json_value(&output.report_spec_json);
    let result = parse_json_value(&output.result_json);
    let review = parse_json_value(&output.review_json);
    let output_manifest = output_manifest_inspector(root, output);
    let run_log = parse_json_value(&output.run_log_json);
    let effect_records = effect_records_inspector(&output_manifest, &run_log);
    let network_cache = network_cache_inspector(&result, &output_manifest, &run_log);
    let db_writes = db_writes_inspector(&result, &review, &output_manifest);
    let model_cards = model_cards_inspector(&result);
    let case_manifests = case_manifests_inspector(&result);
    InspectorView {
        schemas: schema_inspector(&report, &result),
        unit_conversions: json_array_clone(&report, "unit_conversion_table"),
        time_axes: json_array_clone(&report, "time_axes"),
        time_series: time_series_inspector(&report, &result),
        time_series_coverage: time_series_coverage_inspector(&result, &review),
        metrics: json_array_clone(&report, "computed_metrics"),
        validations: json_array_clone(&report, "validations"),
        quality: quality_inspector(&report, &result, &review),
        uncertainty: uncertainty_inspector(&report, &review),
        time_alignments: json_array_clone(&report, "time_alignments"),
        table_transforms: table_transform_inspector(&result, &review),
        structured_reads: typed_payload_array_clone(&result, "structured_reads"),
        config_promotions: typed_payload_array_clone(&result, "config_promotions"),
        systems: system_inspector(&report, &result),
        system_ir: json_array_clone(&report, "system_ir"),
        linear_operators: json_array_clone(&report, "linear_operators"),
        kernel_plan: report.get("kernel_plan").cloned().unwrap_or(Value::Null),
        class_objects: json_array_clone(&report, "object_summary"),
        assemblies: json_array_clone(&report, "assembly_summary"),
        component_graph: report
            .get("component_graph")
            .cloned()
            .unwrap_or(Value::Null),
        review_document: review
            .get("review_document")
            .cloned()
            .unwrap_or(Value::Null),
        artifact_outlines: artifact_outlines(root, output),
        effect_records,
        network_cache,
        db_writes,
        model_cards,
        case_manifests,
        output_manifest,
        run_plan: parse_json_value(&output.run_plan_json),
        run_log,
        process_results: parse_json_value(&output.process_results_json),
        test_results: parse_json_value(&output.test_results_json),
    }
}

fn parse_json_value(text: &str) -> Value {
    serde_json::from_str::<Value>(text).unwrap_or(Value::Null)
}

fn effect_records_inspector(output_manifest: &Value, run_log: &Value) -> Value {
    let registry_artifact_records = output_manifest
        .get("artifact_registry")
        .and_then(|registry| registry.get("generated_files"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let manifest_artifact_records = output_manifest
        .get("artifacts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let artifact_records = if registry_artifact_records.is_empty() {
        manifest_artifact_records
    } else {
        registry_artifact_records
    };
    let external_boundary_records = run_log
        .get("external_boundary_events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    json!({
        "format": "eng-ide-effect-records-v1",
        "recordTypes": ["ArtifactRecord", "ExternalBoundaryRecord"],
        "artifactRecords": artifact_records,
        "externalBoundaryRecords": external_boundary_records,
    })
}

fn network_cache_inspector(result: &Value, output_manifest: &Value, run_log: &Value) -> Value {
    let registry = output_manifest
        .get("artifact_registry")
        .cloned()
        .unwrap_or(Value::Null);
    let network_boundaries = typed_payload_array_clone(result, "network_boundaries");
    let network_requests = registry
        .get("network_requests")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let manifest_caches = registry
        .get("caches")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let network_events = run_log
        .get("network_events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let cache_events = run_log
        .get("cache_events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    json!({
        "format": "eng-ide-network-cache-v1",
        "networkBoundaries": network_boundaries,
        "networkRequests": network_requests,
        "networkEvents": network_events,
        "manifestCaches": manifest_caches,
        "cacheEvents": cache_events,
    })
}

fn db_writes_inspector(result: &Value, review: &Value, output_manifest: &Value) -> Value {
    let runtime_manifests = typed_payload_array_clone(result, "db_manifests");
    let manifests = if runtime_manifests
        .as_array()
        .is_some_and(|items| !items.is_empty())
    {
        runtime_manifests
    } else {
        review
            .get("db_manifests")
            .and_then(Value::as_array)
            .map(|items| Value::Array(items.clone()))
            .unwrap_or_else(|| Value::Array(Vec::new()))
    };
    let registry_writes = output_manifest
        .get("artifact_registry")
        .and_then(|registry| registry.get("db_writes"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    json!({
        "format": "eng-ide-db-writes-v1",
        "manifests": manifests,
        "registryWrites": registry_writes,
    })
}

fn model_cards_inspector(result: &Value) -> Value {
    json!({
        "format": "eng-ide-model-cards-v1",
        "cards": typed_payload_array_clone(result, "model_cards"),
        "artifacts": typed_payload_array_clone(result, "ml"),
        "specs": typed_payload_array_clone(result, "model_specs"),
        "predictionManifests": typed_payload_array_clone(result, "prediction_manifests"),
        "diagnostics": typed_payload_array_clone(result, "model_diagnostics"),
    })
}

fn case_manifests_inspector(result: &Value) -> Value {
    let manifests = typed_payload_array_clone(result, "case_manifests");
    let case_tables = typed_payload_array_clone(result, "case_tables");
    let diagnostics = typed_payload_array_clone(result, "case_diagnostics");
    let failed_cases = manifests
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter(|item| {
                    let status = json_field_string(item, "status").unwrap_or_default();
                    let failure_reason = json_field_string(item, "failure_reason");
                    status == "failed" || failure_reason.is_some()
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "format": "eng-ide-case-manifests-v1",
        "manifests": manifests,
        "caseTables": case_tables,
        "diagnostics": diagnostics,
        "failedCases": failed_cases,
    })
}

fn json_array_clone(value: &Value, key: &str) -> Value {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| Value::Array(items.clone()))
        .unwrap_or_else(|| Value::Array(Vec::new()))
}

fn typed_payload_array_clone(value: &Value, key: &str) -> Value {
    value
        .get("typed_payload")
        .and_then(|payload| payload.get(key))
        .and_then(Value::as_array)
        .map(|items| Value::Array(items.clone()))
        .unwrap_or_else(|| Value::Array(Vec::new()))
}

fn quality_inspector(report: &Value, result: &Value, review: &Value) -> Value {
    let report_quality = report.get("quality_report").cloned().unwrap_or(Value::Null);
    let report_results = report_quality
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let result_results = typed_payload_array_clone(result, "quality_results");
    let review_results = json_array_clone(review, "quality_results");
    let result_rows = result_results.as_array().cloned().unwrap_or_default();
    let rows = if report_results.is_empty() {
        result_rows
    } else {
        report_results
    };
    let failure_count = rows
        .iter()
        .map(|row| {
            row.get("failures")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0)
        })
        .sum::<usize>();
    json!({
        "format": "eng-ide-quality-inspector-v1",
        "summary": report_quality,
        "results": rows,
        "reviewResults": review_results,
        "failureCount": failure_count
    })
}

fn time_series_coverage_inspector(result: &Value, review: &Value) -> Value {
    let runtime_items = typed_payload_array_clone(result, "timeseries_coverage");
    if runtime_items
        .as_array()
        .is_some_and(|items| !items.is_empty())
    {
        return runtime_items;
    }
    review
        .get("timeseries_coverage")
        .and_then(Value::as_array)
        .map(|items| Value::Array(items.clone()))
        .unwrap_or_else(|| Value::Array(Vec::new()))
}

fn uncertainty_inspector(report: &Value, review: &Value) -> Value {
    json!({
        "report": json_array_clone(report, "uncertainty"),
        "summary": json_array_clone(review, "uncertainty_summary"),
        "propagation": json_array_clone(review, "uncertainty_propagation"),
        "policies": json_array_clone(review, "uncertainty_policies"),
        "timeseries": json_array_clone(review, "timeseries_uncertainty"),
        "timeseries_calculations": json_array_clone(review, "timeseries_uncertainty_calculations")
    })
}

fn table_transform_inspector(result: &Value, review: &Value) -> Value {
    let runtime_items = result
        .get("typed_payload")
        .and_then(|payload| payload.get("table_transforms"))
        .and_then(Value::as_array)
        .cloned()
        .or_else(|| {
            result
                .get("table_transforms")
                .and_then(Value::as_array)
                .cloned()
        })
        .unwrap_or_default();
    let contracts = review
        .get("review_document")
        .and_then(|document| document.get("table_transforms"))
        .and_then(Value::as_array)
        .cloned()
        .or_else(|| {
            review
                .get("table_transforms")
                .and_then(Value::as_array)
                .cloned()
        })
        .unwrap_or_default();

    let mut rows = Vec::new();
    let mut seen_bindings = Vec::new();
    for item in runtime_items {
        let binding = json_field_string(&item, "binding").unwrap_or_default();
        let contract = contracts.iter().find(|contract| {
            !binding.is_empty()
                && json_field_string(contract, "binding").as_deref() == Some(binding.as_str())
        });
        if !binding.is_empty() {
            seen_bindings.push(binding);
        }
        rows.push(table_transform_inspector_row(&item, contract));
    }
    for contract in contracts {
        let binding = json_field_string(&contract, "binding").unwrap_or_default();
        if binding.is_empty() || seen_bindings.iter().any(|seen| seen == &binding) {
            continue;
        }
        rows.push(table_transform_static_inspector_row(&contract));
    }
    Value::Array(rows)
}

fn table_transform_inspector_row(item: &Value, contract: Option<&Value>) -> Value {
    let row_diagnostics = json_array_field(item, "row_diagnostics").unwrap_or_default();
    let predicates = json_array_field(item, "predicates")
        .or_else(|| contract.and_then(|value| json_array_field(value, "predicates")))
        .unwrap_or_default();
    let selected_columns = json_array_field(item, "selected_columns")
        .or_else(|| contract.and_then(|value| json_array_field(value, "selected_columns")))
        .unwrap_or_default();
    let derived_columns = json_array_field(item, "derived_columns")
        .or_else(|| contract.and_then(|value| json_array_field(value, "derived_columns")))
        .unwrap_or_default();
    let sort_keys = json_array_field(item, "sort_keys")
        .or_else(|| contract.and_then(|value| json_array_field(value, "sort_keys")))
        .unwrap_or_default();
    let join_keys = json_array_field(item, "join_keys")
        .or_else(|| contract.and_then(|value| json_array_field(value, "join_keys")))
        .unwrap_or_default();
    json!({
        "binding": json_field_string(item, "binding").unwrap_or_default(),
        "operation": json_field_string(item, "operation").unwrap_or_default(),
        "source_table": json_field_string(item, "source_table").unwrap_or_default(),
        "secondary_table": json_field_string(item, "secondary_table"),
        "schema_name": json_field_string(item, "schema_name")
            .or_else(|| contract.and_then(|value| json_field_string(value, "schema_name"))),
        "line": json_field_usize(item, "line")
            .or_else(|| contract.and_then(|value| json_field_usize(value, "line")))
            .unwrap_or(0),
        "status": json_field_string(item, "status").unwrap_or_else(|| "runtime".to_owned()),
        "reason": json_field_string(item, "reason").unwrap_or_default(),
        "contract_status": contract
            .and_then(|value| json_field_string(value, "status"))
            .unwrap_or_default(),
        "input_row_count": json_field_usize(item, "input_row_count").unwrap_or(0),
        "secondary_input_row_count": json_field_usize(item, "secondary_input_row_count"),
        "output_row_count": json_field_usize(item, "output_row_count").unwrap_or(0),
        "matched_pair_count": json_field_usize(item, "matched_pair_count"),
        "matched_row_indices": json_array_field(item, "matched_row_indices").unwrap_or_default(),
        "predicate_count": predicates.len(),
        "selected_column_count": selected_columns.len(),
        "derived_column_count": derived_columns.len(),
        "sort_key_count": sort_keys.len(),
        "join_key_count": join_keys.len(),
        "row_diagnostic_count": row_diagnostics.len(),
        "row_diagnostic_summary": table_row_diagnostic_summary(&row_diagnostics),
        "row_diagnostics_preview": row_diagnostics.into_iter().take(20).collect::<Vec<_>>(),
        "predicates": predicates,
        "selected_columns": selected_columns,
        "derived_columns": derived_columns,
        "sort_keys": sort_keys,
        "join_keys": join_keys
    })
}

fn table_transform_static_inspector_row(contract: &Value) -> Value {
    let predicates = json_array_field(contract, "predicates").unwrap_or_default();
    let selected_columns = json_array_field(contract, "selected_columns").unwrap_or_default();
    let derived_columns = json_array_field(contract, "derived_columns").unwrap_or_default();
    let sort_keys = json_array_field(contract, "sort_keys").unwrap_or_default();
    let join_keys = json_array_field(contract, "join_keys").unwrap_or_default();
    json!({
        "binding": json_field_string(contract, "binding").unwrap_or_default(),
        "operation": json_field_string(contract, "operation").unwrap_or_default(),
        "source_table": json_field_string(contract, "source_table").unwrap_or_default(),
        "secondary_table": json_field_string(contract, "secondary_table"),
        "schema_name": json_field_string(contract, "schema_name"),
        "line": json_field_usize(contract, "line").unwrap_or(0),
        "status": "static",
        "reason": "",
        "contract_status": json_field_string(contract, "status").unwrap_or_default(),
        "input_row_count": 0,
        "secondary_input_row_count": Value::Null,
        "output_row_count": 0,
        "matched_pair_count": Value::Null,
        "matched_row_indices": [],
        "predicate_count": predicates.len(),
        "selected_column_count": selected_columns.len(),
        "derived_column_count": derived_columns.len(),
        "sort_key_count": sort_keys.len(),
        "join_key_count": join_keys.len(),
        "row_diagnostic_count": 0,
        "row_diagnostic_summary": [],
        "row_diagnostics_preview": [],
        "predicates": predicates,
        "selected_columns": selected_columns,
        "derived_columns": derived_columns,
        "sort_keys": sort_keys,
        "join_keys": join_keys
    })
}

fn json_array_field(value: &Value, key: &str) -> Option<Vec<Value>> {
    value.get(key).and_then(Value::as_array).cloned()
}

fn table_row_diagnostic_summary(row_diagnostics: &[Value]) -> Value {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for row in row_diagnostics {
        let status = json_field_string(row, "status").unwrap_or_else(|| "unknown".to_owned());
        if let Some((_, count)) = counts
            .iter_mut()
            .find(|(existing_status, _)| existing_status == &status)
        {
            *count += 1;
        } else {
            counts.push((status, 1));
        }
    }
    Value::Array(
        counts
            .into_iter()
            .map(|(status, count)| json!({ "status": status, "count": count }))
            .collect(),
    )
}

fn output_manifest_inspector(root: &Path, output: &CachedRunOutput) -> Value {
    let mut manifest = parse_json_value(&output.output_manifest_json);
    let has_artifacts = manifest
        .get("artifacts")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    if has_artifacts {
        return manifest;
    }

    let artifacts = runtime_artifacts(root, output)
        .into_iter()
        .map(|artifact| {
            let class = runtime_artifact_class(&artifact.kind);
            json!({
                "kind": artifact.kind,
                "class": class,
                "path": artifact.path,
                "hash": "",
                "status": artifact.status,
                "validation": {
                    "status": "available",
                    "rule": "runtime_buffer",
                    "message": "runtime artifact is available through the IDE memory cache"
                }
            })
        })
        .collect::<Vec<_>>();
    if let Some(object) = manifest.as_object_mut() {
        object.insert("artifact_count".to_owned(), json!(artifacts.len()));
        object.insert("artifacts".to_owned(), Value::Array(artifacts));
        return manifest;
    }
    json!({
        "format": "eng-output-manifest-v1",
        "artifact_count": artifacts.len(),
        "artifacts": artifacts
    })
}

fn runtime_artifact_class(kind: &str) -> &'static str {
    match kind {
        "process_results" => "external_boundary",
        "test_results" => "test",
        _ => "review_artifact",
    }
}

fn schema_inspector(report: &Value, result: &Value) -> Value {
    let source_file = json_string(report, &["source_path"]).unwrap_or_default();
    let Some(schemas) = report.get("schema_summary").and_then(Value::as_array) else {
        return Value::Array(Vec::new());
    };
    let objects = result
        .get("object_store")
        .and_then(|store| store.get("objects"))
        .and_then(Value::as_array);
    Value::Array(
        schemas
            .iter()
            .map(|schema| {
                let name = json_field_string(schema, "name").unwrap_or_else(|| "schema".to_owned());
                let table = objects.and_then(|items| {
                    items.iter().find(|item| {
                        json_field_string(item, "type")
                            .and_then(|value| table_schema_name(&value))
                            .as_deref()
                            == Some(name.as_str())
                    })
                });
                let columns = table
                    .and_then(|item| item.get("columns"))
                    .cloned()
                    .unwrap_or_else(|| Value::Array(Vec::new()));
                let parse_failures = table
                    .and_then(|item| item.get("parse_failures"))
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let conversion_failures = columns
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .flat_map(|column| {
                                column
                                    .get("conversion_failures")
                                    .and_then(Value::as_array)
                                    .cloned()
                                    .unwrap_or_default()
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let date_time_index = columns
                    .as_array()
                    .and_then(|items| {
                        items
                            .iter()
                            .find(|column| {
                                column
                                    .get("is_index")
                                    .and_then(Value::as_bool)
                                    .unwrap_or(false)
                            })
                            .and_then(|column| json_field_string(column, "name"))
                    })
                    .unwrap_or_default();
                json!({
                    "name": name,
                    "source_file": source_file,
                    "line": json_field_usize(schema, "line").unwrap_or(0),
                    "row_count": table.and_then(|item| json_field_usize(item, "row_count")).unwrap_or(0),
                    "date_time_index": date_time_index,
                    "columns": columns,
                    "missing_policy_summary": format!("{} policy item(s)", json_field_usize(schema, "missing_policy_count").unwrap_or(0)),
                    "constraint_summary": format!("{} constraint(s)", json_field_usize(schema, "constraint_count").unwrap_or(0)),
                    "parse_failure_count": parse_failures.len(),
                    "conversion_failure_count": conversion_failures.len(),
                    "parse_failures": parse_failures,
                    "conversion_failures": conversion_failures,
                    "source_hash": table.and_then(|item| json_field_string(item, "source_hash")).unwrap_or_default()
                })
            })
            .collect(),
    )
}

fn table_schema_name(value: &str) -> Option<String> {
    value
        .strip_prefix("Table[")
        .and_then(|rest| rest.strip_suffix(']'))
        .map(ToOwned::to_owned)
}

fn time_series_inspector(report: &Value, result: &Value) -> Value {
    let mut rows = Vec::new();
    if let Some(objects) = result
        .get("object_store")
        .and_then(|store| store.get("objects"))
        .and_then(Value::as_array)
    {
        for object in objects {
            if json_field_string(object, "kind").as_deref() != Some("table") {
                continue;
            }
            if json_field_string(object, "axis").as_deref() != Some("Time") {
                continue;
            }
            let table_name =
                json_field_string(object, "name").unwrap_or_else(|| "table".to_owned());
            let source_hash = json_field_string(object, "source_hash").unwrap_or_default();
            let columns = object
                .get("columns")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let index_values = columns
                .iter()
                .find(|column| {
                    column
                        .get("is_index")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                })
                .and_then(|column| column.get("values"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for column in columns.iter().filter(|column| {
                !column
                    .get("is_index")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            }) {
                let values = numeric_values(column.get("values"));
                let summary = numeric_summary(&values);
                rows.push(json!({
                    "name": format!("{}.{}", table_name, json_field_string(column, "name").unwrap_or_else(|| "series".to_owned())),
                    "axis": "Time",
                    "start_time": index_values.first().map(display_json_value).unwrap_or_default(),
                    "end_time": index_values.last().map(display_json_value).unwrap_or_default(),
                    "timestep": interval_label(&index_values),
                    "row_count": json_field_usize(column, "len").or_else(|| json_field_usize(object, "row_count")).unwrap_or(0),
                    "missing_count": json_field_usize(column, "missing_count").unwrap_or(0),
                    "interpolation_policy": "none",
                    "display_unit": json_field_string(column, "unit").unwrap_or_default(),
                    "canonical_unit": json_field_string(column, "canonical_unit").unwrap_or_default(),
                    "mean": summary.get("mean").cloned().unwrap_or(Value::Null),
                    "min": summary.get("min").cloned().unwrap_or(Value::Null),
                    "max": summary.get("max").cloned().unwrap_or(Value::Null),
                    "p95": summary.get("p95").cloned().unwrap_or(Value::Null),
                    "integration_metadata": Value::Null,
                    "source_hash": source_hash.clone()
                }));
            }
        }
    }
    if let Some(systems) = result
        .get("typed_payload")
        .and_then(|payload| payload.get("systems"))
        .and_then(Value::as_array)
    {
        for system in systems {
            for solver_result in solver_results_for_system(system) {
                if solver_result
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    != "computed"
                {
                    continue;
                }
                let points = solver_result_points(solver_result);
                let values = points.iter().map(|(_, y)| *y).collect::<Vec<_>>();
                let summary = numeric_summary(&values);
                let start = points
                    .first()
                    .map(|(value, _)| format!("{} s", format_json_number(*value)))
                    .unwrap_or_default();
                let end = points
                    .last()
                    .map(|(value, _)| format!("{} s", format_json_number(*value)))
                    .unwrap_or_default();
                let series_owner = json_field_string(solver_result, "binding")
                    .or_else(|| json_field_string(system, "name"))
                    .unwrap_or_else(|| "system".to_owned());
                let time_step = json_field_string(solver_result, "time_step")
                    .or_else(|| json_field_string(solver_result, "time_step_s"));
                let step_diagnostics = solver_result
                    .get("step_diagnostics")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let accepted_substeps = step_diagnostics
                    .iter()
                    .filter(|diagnostic| {
                        json_field_string(diagnostic, "status").as_deref() == Some("accepted")
                    })
                    .count();
                let rejected_substeps = step_diagnostics.len().saturating_sub(accepted_substeps);
                rows.push(json!({
                    "name": format!(
                        "{}.{}",
                        series_owner,
                        json_field_string(solver_result, "state").unwrap_or_else(|| "state".to_owned())
                    ),
                    "axis": "Time",
                    "start_time": start,
                    "end_time": end,
                    "timestep": time_step.map(|value| format!("{value} s")).unwrap_or_default(),
                    "row_count": points.len(),
                    "missing_count": 0,
                    "interpolation_policy": "fixed-step",
                    "display_unit": json_field_string(solver_result, "display_unit").unwrap_or_default(),
                    "canonical_unit": json_field_string(solver_result, "canonical_unit").unwrap_or_default(),
                    "mean": summary.get("mean").cloned().unwrap_or(Value::Null),
                    "min": summary.get("min").cloned().unwrap_or(Value::Null),
                    "max": summary.get("max").cloned().unwrap_or(Value::Null),
                    "p95": summary.get("p95").cloned().unwrap_or(Value::Null),
                    "integration_metadata": {
                        "method": json_field_string(solver_result, "method").unwrap_or_default(),
                        "step_count": json_field_usize(solver_result, "step_count").unwrap_or(0),
                        "substep_count": step_diagnostics.len(),
                        "accepted_substep_count": accepted_substeps,
                        "rejected_substep_count": rejected_substeps,
                        "duration": json_field_string(solver_result, "duration").unwrap_or_default(),
                        "final_value": json_field_string(solver_result, "final_value").unwrap_or_default()
                    },
                    "source_hash": ""
                }));
            }
        }
    }
    append_component_solver_time_series(report, &mut rows);
    Value::Array(rows)
}

fn append_component_solver_time_series(report: &Value, rows: &mut Vec<Value>) {
    let Some(assemblies) = report.get("assembly_summary").and_then(Value::as_array) else {
        return;
    };
    for assembly in assemblies {
        let assembly_name =
            json_field_string(assembly, "name").unwrap_or_else(|| "assembly".to_owned());
        let Some(solver_result) = assembly
            .get("solver_result")
            .or_else(|| assembly.get("solverResult"))
        else {
            continue;
        };
        let trajectories = solver_result
            .get("trajectories")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for trajectory in trajectories {
            let points = solver_result_points(&trajectory);
            if points.is_empty() {
                continue;
            }
            let values = points.iter().map(|(_, y)| *y).collect::<Vec<_>>();
            let summary = numeric_summary(&values);
            let start = points
                .first()
                .map(|(value, _)| format!("{} s", format_json_number(*value)))
                .unwrap_or_default();
            let end = points
                .last()
                .map(|(value, _)| format!("{} s", format_json_number(*value)))
                .unwrap_or_default();
            let point_count = json_field_usize(&trajectory, "point_count")
                .or_else(|| json_field_usize(&trajectory, "pointCount"))
                .unwrap_or(points.len());
            rows.push(json!({
                "name": format!(
                    "{}.{}",
                    assembly_name,
                    json_field_string(&trajectory, "name").unwrap_or_else(|| "trajectory".to_owned())
                ),
                "axis": "Time",
                "start_time": start,
                "end_time": end,
                "timestep": fixed_step_label(&points),
                "row_count": point_count,
                "missing_count": 0,
                "interpolation_policy": "fixed-step component-solver",
                "display_unit": json_field_string(&trajectory, "unit").unwrap_or_default(),
                "canonical_unit": json_field_string(&trajectory, "unit").unwrap_or_default(),
                "mean": summary.get("mean").cloned().unwrap_or(Value::Null),
                "min": summary.get("min").cloned().unwrap_or(Value::Null),
                "max": summary.get("max").cloned().unwrap_or(Value::Null),
                "p95": summary.get("p95").cloned().unwrap_or(Value::Null),
                "integration_metadata": {
                    "method": json_field_string(solver_result, "method").unwrap_or_default(),
                    "role": json_field_string(&trajectory, "role").unwrap_or_default(),
                    "status": json_field_string(solver_result, "status").unwrap_or_default(),
                    "convergence_status": json_field_string(solver_result, "convergence_status").unwrap_or_default(),
                    "failure_code": json_field_string(solver_result, "failure_code")
                        .or_else(|| {
                            solver_result
                                .get("failure_artifact")
                                .and_then(|failure| json_field_string(failure, "code"))
                        })
                        .unwrap_or_default(),
                    "failure_reason": json_field_string(solver_result, "failure_reason")
                        .or_else(|| {
                            solver_result
                                .get("failure_artifact")
                                .and_then(|failure| json_field_string(failure, "message"))
                        })
                        .unwrap_or_default(),
                    "final_value": trajectory.get("final_value").cloned().unwrap_or(Value::Null)
                },
                "source_hash": ""
            }));
        }
    }
}

fn solver_results_for_system(system: &Value) -> Vec<&Value> {
    if let Some(results) = system.get("solver_results").and_then(Value::as_array) {
        if !results.is_empty() {
            return results.iter().collect();
        }
    }
    system.get("solver_result").into_iter().collect()
}

fn solver_result_points(solver_result: &Value) -> Vec<(f64, f64)> {
    solver_result
        .get("points")
        .and_then(Value::as_array)
        .map(|points| {
            points
                .iter()
                .filter_map(|point| {
                    if let Some(items) = point.as_array() {
                        return Some((items.first()?.as_f64()?, items.get(1)?.as_f64()?));
                    }
                    Some((point.get("x")?.as_f64()?, point.get("y")?.as_f64()?))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn fixed_step_label(points: &[(f64, f64)]) -> String {
    match (points.first(), points.get(1)) {
        (Some((first, _)), Some((second, _))) => {
            format!("{} s", format_json_number(second - first))
        }
        _ => String::new(),
    }
}

fn numeric_values(value: Option<&Value>) -> Vec<f64> {
    value
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default()
}

fn numeric_summary(values: &[f64]) -> Value {
    if values.is_empty() {
        return json!({
            "mean": Value::Null,
            "min": Value::Null,
            "max": Value::Null,
            "p95": Value::Null
        });
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let min = values
        .iter()
        .copied()
        .fold(f64::INFINITY, |left, right| left.min(right));
    let max = values
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |left, right| left.max(right));
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let index = (((sorted.len() - 1) as f64) * 0.95).round() as usize;
    json!({
        "mean": mean,
        "min": min,
        "max": max,
        "p95": sorted[index]
    })
}

fn display_json_value(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        text.to_owned()
    } else if let Some(number) = value.as_f64() {
        format_json_number(number)
    } else {
        value.to_string()
    }
}

fn interval_label(values: &[Value]) -> String {
    let Some(first) = values.first() else {
        return String::new();
    };
    let Some(second) = values.get(1) else {
        return String::new();
    };
    if let (Some(left), Some(right)) = (value_as_seconds(first), value_as_seconds(second)) {
        return format!("{} s", right - left);
    }
    format!(
        "{} -> {}",
        display_json_value(first),
        display_json_value(second)
    )
}

fn value_as_seconds(value: &Value) -> Option<i64> {
    if let Some(number) = value.as_i64() {
        return Some(number);
    }
    value.as_str().and_then(parse_iso_utc_seconds)
}

fn parse_iso_utc_seconds(value: &str) -> Option<i64> {
    let (date, time) = value.strip_suffix('Z')?.split_once('T')?;
    let mut date_parts = date.split('-').filter_map(|part| part.parse::<i64>().ok());
    let year = date_parts.next()?;
    let month = date_parts.next()?;
    let day = date_parts.next()?;
    let mut time_parts = time.split(':').filter_map(|part| part.parse::<i64>().ok());
    let hour = time_parts.next()?;
    let minute = time_parts.next()?;
    let second = time_parts.next()?;
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second)
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * month_prime + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn system_inspector(report: &Value, result: &Value) -> Value {
    let result_systems = result
        .get("typed_payload")
        .and_then(|payload| payload.get("systems"))
        .and_then(Value::as_array);
    let report_system_ir = report.get("system_ir").and_then(Value::as_array);
    let Some(report_systems) = report.get("system_summary").and_then(Value::as_array) else {
        return result_systems
            .map(|items| Value::Array(items.clone()))
            .unwrap_or_else(|| Value::Array(Vec::new()));
    };
    Value::Array(
        report_systems
            .iter()
            .map(|system| {
                let mut merged = system.as_object().cloned().unwrap_or_default();
                let name = json_field_string(system, "name").unwrap_or_default();
                if let Some(result_system) = result_systems.and_then(|items| {
                    items.iter().find(|item| {
                        json_field_string(item, "name").as_deref() == Some(name.as_str())
                    })
                }) {
                    if let Some(solver_result) = result_system.get("solver_result") {
                        merged.insert("solver_result".to_owned(), solver_result.clone());
                    }
                    if let Some(solver_results) = result_system.get("solver_results") {
                        merged.insert("solver_results".to_owned(), solver_results.clone());
                    }
                } else if let Some(report_ir) = report_system_ir.and_then(|items| {
                    items.iter().find(|item| {
                        json_field_string(item, "name").as_deref() == Some(name.as_str())
                    })
                }) {
                    if let Some(solver_results) = report_ir.get("solver_results") {
                        merged.insert("solver_results".to_owned(), solver_results.clone());
                    }
                }
                Value::Object(merged)
            })
            .collect(),
    )
}

fn artifact_outlines(root: &Path, output: &CachedRunOutput) -> Value {
    let status = if output.artifacts_saved {
        "saved"
    } else {
        "memory"
    };
    let artifacts = [
        ("result", &output.result_path, &output.result_json),
        ("review", &output.review_path, &output.review_json),
        (
            "static_run_plan",
            &output.static_run_plan_path,
            &output.static_run_plan_json,
        ),
        ("run_plan", &output.run_plan_path, &output.run_plan_json),
        ("run_lock", &output.run_lock_path, &output.run_lock_json),
        ("run_log", &output.run_log_path, &output.run_log_json),
        (
            "process_results",
            &output.process_results_path,
            &output.process_results_json,
        ),
        (
            "test_results",
            &output.test_results_path,
            &output.test_results_json,
        ),
        (
            "output_manifest",
            &output.output_manifest_path,
            &output.output_manifest_json,
        ),
        (
            "report_spec",
            &output.report_spec_path,
            &output.report_spec_json,
        ),
        (
            "plot_manifest",
            &output.plot_manifest_path,
            &output.plot_manifest_json,
        ),
        ("plot_spec", &output.plot_spec_path, &output.plot_spec_json),
    ];
    Value::Array(
        artifacts
            .iter()
            .map(|(kind, path, text)| {
                let parsed = parse_json_value(text);
                json!({
                    "kind": kind,
                    "path": relative_to(root, path),
                    "status": status,
                    "sections": artifact_sections(&parsed)
                })
            })
            .collect(),
    )
}

fn artifact_sections(value: &Value) -> Value {
    let Some(object) = value.as_object() else {
        return Value::Array(Vec::new());
    };
    Value::Array(
        object
            .iter()
            .take(18)
            .map(|(name, value)| {
                json!({
                    "name": name,
                    "summary": value_summary(value)
                })
            })
            .collect(),
    )
}

fn value_summary(value: &Value) -> String {
    if let Some(items) = value.as_array() {
        format!("{} item(s)", items.len())
    } else if let Some(object) = value.as_object() {
        format!("{} field(s)", object.len())
    } else if value.is_string() {
        "string".to_owned()
    } else if value.is_number() {
        "number".to_owned()
    } else if value.is_boolean() {
        "bool".to_owned()
    } else if value.is_null() {
        "null".to_owned()
    } else {
        "value".to_owned()
    }
}

fn runtime_object_variable(value: &Value) -> RuntimeVariableView {
    let kind = json_field_string(value, "kind").unwrap_or_else(|| "object".to_owned());
    let object_type = json_field_string(value, "type").unwrap_or_default();
    let display_unit = json_field_string(value, "display_unit").unwrap_or_default();
    let value_text = json_field_usize(value, "row_count")
        .map(|count| format!("{count} rows"))
        .or_else(|| json_field_usize(value, "len").map(|len| format!("{len} items")));
    RuntimeVariableView {
        name: json_field_string(value, "name").unwrap_or_else(|| "unknown".to_owned()),
        quantity_kind: if object_type.is_empty() {
            kind
        } else {
            object_type
        },
        display_unit,
        canonical_unit: String::new(),
        dimension: String::new(),
        source: "runtime".to_owned(),
        role: None,
        value: value_text,
        line: json_field_usize(value, "line").unwrap_or(0),
    }
}

fn report_variable(value: &Value) -> RuntimeVariableView {
    RuntimeVariableView {
        name: json_field_string(value, "name").unwrap_or_else(|| "unknown".to_owned()),
        quantity_kind: json_field_string(value, "quantity_kind").unwrap_or_default(),
        display_unit: json_field_string(value, "display_unit").unwrap_or_default(),
        canonical_unit: json_field_string(value, "canonical_unit").unwrap_or_default(),
        dimension: json_field_string(value, "dimension").unwrap_or_default(),
        source: json_field_string(value, "source").unwrap_or_else(|| "run".to_owned()),
        role: None,
        value: None,
        line: json_field_usize(value, "line").unwrap_or(0),
    }
}

fn uncertainty_variable(value: &Value) -> RuntimeVariableView {
    RuntimeVariableView {
        name: json_field_string(value, "binding").unwrap_or_else(|| "unknown".to_owned()),
        quantity_kind: json_field_string(value, "quantity_kind").unwrap_or_default(),
        display_unit: json_field_string(value, "display_unit").unwrap_or_default(),
        canonical_unit: String::new(),
        dimension: String::new(),
        source: "uncertainty".to_owned(),
        role: Some(
            json_field_string(value, "method")
                .map(|method| format!("uncertainty:{method}"))
                .unwrap_or_else(|| "uncertainty".to_owned()),
        ),
        value: uncertainty_value_label(value),
        line: json_field_usize(value, "line").unwrap_or(0),
    }
}

fn uncertainty_value_label(value: &Value) -> Option<String> {
    let kind = json_field_string(value, "kind")?;
    let mut parts = vec![kind];
    if let Some(distribution) = json_field_string(value, "distribution") {
        parts.push(distribution);
    }
    if let Some(mean) = json_field_string(value, "mean") {
        parts.push(format!("mean={mean}"));
    }
    if let Some(stddev) = json_field_string(value, "stddev") {
        parts.push(format!("std={stddev}"));
    }
    if let (Some(lower), Some(upper)) = (
        json_field_string(value, "lower"),
        json_field_string(value, "upper"),
    ) {
        parts.push(format!("interval=[{lower}, {upper}]"));
    }
    if let Some(p95) = json_field_string(value, "p95") {
        parts.push(format!("p95={p95}"));
    }
    Some(parts.join(" "))
}

fn system_variable(value: &Value, source: &str) -> RuntimeVariableView {
    RuntimeVariableView {
        name: json_field_string(value, "name").unwrap_or_else(|| "unknown".to_owned()),
        quantity_kind: json_field_string(value, "quantity_kind").unwrap_or_default(),
        display_unit: json_field_string(value, "display_unit").unwrap_or_default(),
        canonical_unit: String::new(),
        dimension: json_field_string(value, "dimension").unwrap_or_default(),
        source: source.to_owned(),
        role: json_field_string(value, "role"),
        value: json_field_string(value, "initial_value"),
        line: json_field_usize(value, "line").unwrap_or(0),
    }
}

fn merge_variable(variables: &mut Vec<RuntimeVariableView>, incoming: RuntimeVariableView) {
    if let Some(existing) = variables
        .iter_mut()
        .find(|variable| variable.name == incoming.name && variable.line == incoming.line)
    {
        if existing.value.is_none() {
            existing.value = incoming.value;
        }
        if existing.role.is_none() {
            existing.role = incoming.role;
        }
        if existing.display_unit.is_empty() {
            existing.display_unit = incoming.display_unit;
        }
        if existing.canonical_unit.is_empty() {
            existing.canonical_unit = incoming.canonical_unit;
        }
        if existing.dimension.is_empty() {
            existing.dimension = incoming.dimension;
        }
        return;
    }
    variables.push(incoming);
}

fn terminal_command_error(command: &str) -> Option<CheckView> {
    let name = bare_call_name(command)?;
    let (message, help) = if name.eq_ignore_ascii_case("print") {
        (
            "print is a string-template statement, not a function call.".to_owned(),
            Some("Use: print Q_coil or print Q_coil: .2 kW".to_owned()),
        )
    } else {
        (
            "bare function calls are not executable terminal statements.".to_owned(),
            Some("Bind the value first, for example: x = mean(Q, axis=Time)".to_owned()),
        )
    };
    Some(CheckView {
        diagnostics: vec![DiagnosticView {
            severity: "error".to_owned(),
            code: "E-IDE-TERMINAL-SYNTAX".to_owned(),
            line: 1,
            message,
            help,
        }],
        symbols: Vec::new(),
        status: "1 error(s), 0 warning(s)".to_owned(),
        semantic_tokens: empty_semantic_tokens_view(),
        hovers: empty_hovers_view(),
    })
}

fn terminal_unrecognized_command_error(command: &str, run_dir: &Path) -> Option<CheckView> {
    let report = check_source(
        run_dir.join("__ide_terminal_command__.eng"),
        command,
        &CheckOptions::default(),
    );
    if report.syntax_summary.ast_items > 0 || report.has_errors() {
        return None;
    }
    Some(CheckView {
        diagnostics: vec![DiagnosticView {
            severity: "error".to_owned(),
            code: "E-IDE-TERMINAL-SYNTAX".to_owned(),
            line: 1,
            message: "terminal command was not recognized.".to_owned(),
            help: Some("Use `run`, `check`, `reset`, `clear`, `cd <dir>`, or a one-line EngLang statement such as `x = 3` or `print x`.".to_owned()),
        }],
        symbols: Vec::new(),
        status: "1 error(s), 0 warning(s)".to_owned(),
        semantic_tokens: empty_semantic_tokens_view(),
        hovers: empty_hovers_view(),
    })
}

fn bare_call_name(command: &str) -> Option<&str> {
    if command.contains('=') || command.contains('"') {
        return None;
    }
    let open = command.find('(')?;
    let prefix = command[..open].trim();
    let mut chars = prefix.chars();
    let first = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    if chars.all(|character| character.is_ascii_alphanumeric() || character == '_') {
        Some(prefix)
    } else {
        None
    }
}

fn plot_spec_or_null(value: Value) -> Value {
    if has_plot_data(&value) {
        value
    } else {
        Value::Null
    }
}

fn has_plot_data(value: &Value) -> bool {
    if array_has_items(value, "points") || array_has_items(value, "bins") {
        return true;
    }
    value
        .get("series")
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .any(|item| array_has_items(item, "points") || array_has_items(item, "bins"))
        })
}

fn array_has_items(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
}

fn terminal_summary(
    stdout: &str,
    _variables: &[RuntimeVariableView],
    _args: &[RuntimeArgView],
    _report_title: &str,
    _plot_spec: &Value,
) -> String {
    stdout.trim_end().to_owned()
}

fn diagnostic_summary_text(check: &CheckView) -> String {
    let mut lines = Vec::new();
    for diagnostic in check.diagnostics.iter().take(6) {
        lines.push(format!(
            "{} L{} {}: {}",
            diagnostic.severity, diagnostic.line, diagnostic.code, diagnostic.message
        ));
        if let Some(help) = &diagnostic.help {
            lines.push(format!("  help: {help}"));
        }
    }
    if lines.is_empty() {
        lines.push(check.status.clone());
    }
    if check.diagnostics.len() > 6 {
        lines.push(format!(
            "... {} more diagnostic(s)",
            check.diagnostics.len() - 6
        ));
    }
    lines.join("\n")
}

fn workspace_root() -> PathBuf {
    let current_dir = env::current_dir().ok();
    let exe_dir = env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));

    for candidate in current_dir.iter().chain(exe_dir.iter()) {
        if let Some(root) = find_workspace_root(candidate) {
            return root;
        }
    }

    current_dir.unwrap_or_else(|| PathBuf::from("."))
}

fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut candidate = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        if is_workspace_root(&candidate) {
            return Some(candidate);
        }
        if !candidate.pop() {
            return None;
        }
    }
}

fn is_workspace_root(path: &Path) -> bool {
    path.join("examples").is_dir() && path.join("stdlib").is_dir()
}

fn default_file(root: &Path) -> PathBuf {
    let preferred = root.join("examples/internal/03_integrated_hvac/main.eng");
    if preferred.exists() {
        return preferred;
    }
    collect_examples(root)
        .into_iter()
        .next()
        .unwrap_or_else(|| root.join("main.eng"))
}

fn collect_examples(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_files(&root.join("examples"), &mut paths);
    paths.retain(|path| path.extension().and_then(|value| value.to_str()) == Some("eng"));
    paths.sort();
    paths
}

fn workspace_tree(root: &Path) -> Vec<FileNodeView> {
    ["examples", "stdlib", "docs"]
        .iter()
        .filter_map(|name| {
            let path = root.join(name);
            path.exists().then(|| file_node(root, &path))
        })
        .collect()
}

fn file_node(root: &Path, path: &Path) -> FileNodeView {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("workspace")
        .to_owned();
    let kind = if path.is_dir() { "dir" } else { "file" }.to_owned();
    let mut children = if path.is_dir() {
        sorted_visible_entries(path)
            .into_iter()
            .filter(|child| child.is_dir() || visible_file(child))
            .map(|child| file_node(root, &child))
            .collect()
    } else {
        Vec::new()
    };
    if path.is_dir() {
        children.sort_by(|a: &FileNodeView, b: &FileNodeView| {
            (a.kind.as_str() != "dir", &a.name).cmp(&(b.kind.as_str() != "dir", &b.name))
        });
    }
    FileNodeView {
        name,
        path: relative_to(root, path),
        kind,
        children,
    }
}

fn collect_files(path: &Path, output: &mut Vec<PathBuf>) {
    for entry in sorted_visible_entries(path) {
        if entry.is_dir() {
            collect_files(&entry, output);
        } else {
            output.push(entry);
        }
    }
}

fn sorted_visible_entries(path: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut entries: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|entry| {
            entry
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| !name.starts_with('.') && name != "target" && name != "build")
        })
        .collect();
    entries.sort();
    entries
}

fn visible_file(path: &Path) -> bool {
    let Some(extension) = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|extension| extension.to_ascii_lowercase())
    else {
        return false;
    };
    matches!(
        extension.as_str(),
        "eng" | "csv" | "md" | "txt" | "json" | "yml" | "yaml" | "toml"
    )
}

fn resolve_path(root: &Path, input: &str) -> PathBuf {
    let path = PathBuf::from(input);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn source_dir(path: &Path) -> &Path {
    path.parent().unwrap_or_else(|| Path::new("."))
}

fn read_utf8(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("Could not read {}: {error}", path.display()))
}

fn create_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn relative_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn report_title(text: &str) -> String {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return String::new();
    };
    json_string(&value, &["title"])
        .or_else(|| json_string(&value, &["metadata", "title"]))
        .unwrap_or_default()
}

fn json_string(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(ToOwned::to_owned)
}

fn json_field_string(value: &Value, key: &str) -> Option<String> {
    let field = value.get(key)?;
    if field.is_null() {
        return None;
    }
    if let Some(text) = field.as_str() {
        return Some(text.to_owned());
    }
    if let Some(number) = field.as_f64() {
        return Some(format_json_number(number));
    }
    field.as_bool().map(|value| value.to_string())
}

fn json_field_usize(value: &Value, key: &str) -> Option<usize> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn json_array_non_empty(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
}

fn review_hashes_include(value: &Value, keys: &[&str]) -> bool {
    value
        .get("section_hashes")
        .and_then(Value::as_object)
        .is_some_and(|hashes| keys.iter().all(|key| hashes.contains_key(*key)))
}

fn review_document_has_core_cockpit_sections(value: &Value) -> bool {
    value
        .get("semantic_hash")
        .and_then(Value::as_str)
        .is_some_and(|hash| !hash.is_empty())
        && [
            "symbols",
            "units_quantities",
            "schemas",
            "time_axes",
            "calculations",
            "report_outputs",
            "risks",
        ]
        .iter()
        .all(|key| json_array_non_empty(value, key))
        && review_hashes_include(
            value,
            &[
                "units_quantities",
                "schemas",
                "time_axes",
                "calculations",
                "report_outputs",
                "risks",
            ],
        )
}

fn review_document_has_external_boundary(value: &Value) -> bool {
    json_array_non_empty(value, "external_boundaries")
        && review_hashes_include(value, &["external_boundaries"])
}

fn review_document_has_side_effect(value: &Value) -> bool {
    json_array_non_empty(value, "side_effects") && review_hashes_include(value, &["side_effects"])
}

fn effect_records_has_artifact_and_boundary_records(value: &Value) -> bool {
    let has_record_types = value
        .get("recordTypes")
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .any(|item| item.as_str() == Some("ArtifactRecord"))
                && items
                    .iter()
                    .any(|item| item.as_str() == Some("ExternalBoundaryRecord"))
        });
    let has_artifact_record = value
        .get("artifactRecords")
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(is_artifact_record_value));
    let has_boundary_record = value
        .get("externalBoundaryRecords")
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(is_external_boundary_record_value));
    has_record_types && has_artifact_record && has_boundary_record
}

fn is_artifact_record_value(value: &Value) -> bool {
    ["kind", "class", "path", "hash", "status", "validation"]
        .iter()
        .all(|key| value.get(*key).is_some())
}

fn is_external_boundary_record_value(value: &Value) -> bool {
    ["kind", "target", "status", "success", "line"]
        .iter()
        .all(|key| value.get(*key).is_some())
}

fn json_field_f64(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn format_json_number(value: f64) -> String {
    let text = if value.abs() >= 1000.0 {
        format!("{value:.3}")
    } else if value.abs() >= 10.0 {
        format!("{value:.4}")
    } else {
        format!("{value:.6}")
    };
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
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

fn public_package_smoke(root: &Path) -> Result<(), String> {
    let examples = collect_examples(root);
    let source_path = root.join("examples/official/01_csv_plot/main.eng");
    if !source_path.exists() {
        return Err(format!(
            "public package smoke missing {}",
            source_path.display()
        ));
    }
    let source = read_utf8(&source_path)?;
    let report = check_source(&source_path, &source, &CheckOptions::default());
    if report.has_errors() {
        return Err(format!("{} has diagnostics", source_path.display()));
    }
    let ui_index = root.join("crates/eng_ide/ui/index.html");
    if root.join("crates/eng_ide").exists() && !ui_index.exists() {
        return Err(format!("missing Tauri UI asset {}", ui_index.display()));
    }

    let output = run_file(
        &source_path,
        &root.join("build").join("ide-smoke-public-core"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let cached = CachedRunOutput::from_output(output);
    let inspectors = runtime_inspectors(root, &cached);
    for (label, value) in [
        ("schema", &inspectors.schemas),
        ("timeseries", &inspectors.time_series),
        ("artifact outline", &inspectors.artifact_outlines),
    ] {
        if value.as_array().is_none_or(Vec::is_empty) {
            return Err(format!(
                "{} did not produce IDE {label} inspector metadata",
                source_path.display()
            ));
        }
    }
    let has_kernel_plan = inspectors
        .kernel_plan
        .get("candidates")
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items.iter().any(|item| {
                json_field_string(item, "kind").as_deref() == Some("timeseries_integrate")
                    && item
                        .get("executor")
                        .and_then(|executor| json_field_string(executor, "status"))
                        .as_deref()
                        == Some("interpreter_supported")
            })
        });
    if !has_kernel_plan {
        return Err(format!(
            "{} did not produce IDE kernel plan inspector metadata",
            source_path.display()
        ));
    }
    if !review_document_has_core_cockpit_sections(&inspectors.review_document) {
        return Err(format!(
            "{} did not produce normalized IDE review cockpit metadata",
            source_path.display()
        ));
    }

    let class_example = root.join("examples/official/19_class_object/main.eng");
    let class_output = run_file(
        &class_example,
        &root.join("build").join("ide-smoke-public-class-object"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let class_cached = CachedRunOutput::from_output(class_output);
    let class_inspectors = runtime_inspectors(root, &class_cached);
    let has_class_object = class_inspectors
        .class_objects
        .as_array()
        .is_some_and(|items| {
            items.iter().any(|item| {
                json_field_string(item, "name").as_deref() == Some("building")
                    && json_field_string(item, "class_name").as_deref() == Some("Building")
                    && json_field_usize(item, "field_count").unwrap_or(0) > 0
                    && json_field_usize(item, "validation_count").unwrap_or(0) > 0
            })
        });
    if !has_class_object {
        return Err(format!(
            "{} did not produce IDE class object inspector metadata",
            class_example.display()
        ));
    }

    let effects_example = root.join("examples/official/15_process_result/main.eng");
    let effects_output = run_file(
        &effects_example,
        &root.join("build").join("ide-smoke-public-effects"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let effects_cached = CachedRunOutput::from_output(effects_output);
    let effects_inspectors = runtime_inspectors(root, &effects_cached);
    let has_effect_records =
        effect_records_has_artifact_and_boundary_records(&effects_inspectors.effect_records);
    let has_review_document = effects_inspectors
        .review_document
        .get("external_boundaries")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
        && review_document_has_external_boundary(&effects_inspectors.review_document);
    if !has_effect_records || !has_review_document {
        return Err(format!(
            "{} did not produce IDE side-effect inspector metadata (effect_records={}, review={})",
            effects_example.display(),
            has_effect_records,
            has_review_document
        ));
    }

    let file_effects_example = root.join("examples/official/13_file_operations/main.eng");
    let file_effects_output = run_file(
        &file_effects_example,
        &root.join("build").join("ide-smoke-public-file-effects"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let file_effects_cached = CachedRunOutput::from_output(file_effects_output);
    let file_effects_inspectors = runtime_inspectors(root, &file_effects_cached);
    if !review_document_has_side_effect(&file_effects_inspectors.review_document) {
        return Err(format!(
            "{} did not produce IDE review side-effect metadata",
            file_effects_example.display()
        ));
    }

    let test_example = root.join("examples/official/16_test_assert_golden/main.eng");
    let test_output = run_file(
        &test_example,
        &root.join("build").join("ide-smoke-public-tests"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let test_cached = CachedRunOutput::from_output(test_output);
    let test_inspectors = runtime_inspectors(root, &test_cached);
    let has_tests = test_inspectors
        .test_results
        .get("tests")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    if !has_tests {
        return Err(format!(
            "{} did not produce IDE test-result inspector metadata",
            test_example.display()
        ));
    }

    println!(
        "EngLang IDE public package smoke OK: {} example(s), {} quantity completion(s), {} unit completion(s), schema/TimeSeries/report inspectors, kernel plan inspector, class object inspector, normalized review cockpit, side-effect/review inspectors, test-result inspector",
        examples.len(),
        all_quantity_completions().len(),
        all_unit_infos().len()
    );
    Ok(())
}

fn smoke() -> Result<(), String> {
    let root = workspace_root();
    if !root.join("examples/internal").is_dir() && !root.join("examples/advanced_solver").is_dir() {
        return public_package_smoke(&root);
    }
    let examples = collect_examples(&root);
    let Some(first) = examples.first() else {
        return Err("no .eng examples found".to_owned());
    };
    let source = read_utf8(first)?;
    let report = check_source(first, &source, &CheckOptions::default());
    if report.has_errors() {
        return Err(format!("{} has diagnostics", first.display()));
    }
    let domain_example = root.join("examples/internal/06_domain_port/main.eng");
    let domain_source = read_utf8(&domain_example)?;
    let domain_report = check_source(&domain_example, &domain_source, &CheckOptions::default());
    if domain_report.has_errors()
        || domain_report.semantic_program.domains.is_empty()
        || domain_report.semantic_program.components.is_empty()
        || domain_report.semantic_program.connections.is_empty()
        || domain_report
            .semantic_program
            .component_assemblies
            .is_empty()
    {
        return Err(format!(
            "{} did not produce domain/component/assembly metadata",
            domain_example.display()
        ));
    }
    let ui_index = root.join("crates/eng_ide/ui/index.html");
    if root.join("crates/eng_ide").exists() && !ui_index.exists() {
        return Err(format!("missing Tauri UI asset {}", ui_index.display()));
    }
    assert_native_ide_ui_behavior_status_labels(&root)?;
    let review_example = root.join("examples/official/01_csv_plot/main.eng");
    let review_output = run_file(
        &review_example,
        &root.join("build").join("ide-smoke-review-core"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let review_cached = CachedRunOutput::from_output(review_output);
    let review_inspectors = runtime_inspectors(&root, &review_cached);
    if !review_document_has_core_cockpit_sections(&review_inspectors.review_document) {
        return Err(format!(
            "{} did not produce normalized IDE review cockpit metadata",
            review_example.display()
        ));
    }
    let domain_output = run_file(
        &domain_example,
        &root.join("build").join("ide-smoke-domain"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let domain_cached = CachedRunOutput::from_output(domain_output);
    let domain_inspectors = runtime_inspectors(&root, &domain_cached);
    let component_graph = &domain_inspectors.component_graph;
    let graph_components = component_graph
        .get("components")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let graph_connections = component_graph
        .get("connections")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let has_source_span = component_graph
        .get("connections")
        .and_then(Value::as_array)
        .and_then(|connections| connections.first())
        .and_then(|connection| connection.get("source_span"))
        .is_some();
    if graph_components == 0 || graph_connections == 0 || !has_source_span {
        return Err(format!(
            "{} did not produce IDE component graph inspector metadata",
            domain_example.display()
        ));
    }
    let has_residual_dependency_graph =
        domain_inspectors
            .assemblies
            .as_array()
            .is_some_and(|assemblies| {
                assemblies.iter().any(|assembly| {
                    assembly
                        .get("residual_graph")
                        .and_then(|graph| graph.get("dependencies"))
                        .and_then(Value::as_array)
                        .is_some_and(|dependencies| {
                            dependencies.iter().any(|dependency| {
                                json_field_string(dependency, "residual").is_some()
                                    && json_field_string(dependency, "variable").is_some()
                            })
                        })
                })
            });
    if !has_residual_dependency_graph {
        return Err(format!(
            "{} did not produce IDE residual dependency graph inspector metadata",
            domain_example.display()
        ));
    }
    let behavior_example = root.join("examples/internal/25_component_behavior_nodes/main.eng");
    let behavior_output = run_file(
        &behavior_example,
        &root.join("build").join("ide-smoke-behavior-nodes"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let behavior_cached = CachedRunOutput::from_output(behavior_output);
    let behavior_inspectors = runtime_inspectors(&root, &behavior_cached);
    let behavior_nodes = behavior_inspectors
        .component_graph
        .get("behavior_nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let has_contract_quantity = |node: &Value, key: &str, quantity: &str| {
        node.get(key)
            .and_then(Value::as_array)
            .is_some_and(|contracts| {
                contracts.iter().any(|contract| {
                    json_field_string(contract, "quantity_kind").as_deref() == Some(quantity)
                        && json_field_string(contract, "status").is_some()
                })
            })
    };
    let has_contract_status = |node: &Value, key: &str, status: &str| {
        node.get(key)
            .and_then(Value::as_array)
            .is_some_and(|contracts| {
                contracts.iter().any(|contract| {
                    json_field_string(contract, "status").as_deref() == Some(status)
                })
            })
    };
    let has_diagnostic_channel = |node: &Value, channel: &str| {
        node.get("diagnostic_channels")
            .and_then(Value::as_array)
            .is_some_and(|channels| channels.iter().any(|value| value.as_str() == Some(channel)))
    };
    let has_delay_node = behavior_nodes.iter().any(|node| {
        json_field_string(node, "behavior_kind").as_deref() == Some("delay")
            && json_field_string(node, "signal").as_deref() == Some("temperature_signal")
            && json_field_f64(node, "delay_s").is_some_and(|value| (value - 5.0).abs() <= 1e-9)
            && json_field_string(node, "relationship_status").as_deref()
                == Some("delay_relationship_metadata_only")
            && has_contract_quantity(node, "contract_inputs", "AbsoluteTemperature")
            && has_contract_quantity(node, "contract_outputs", "AbsoluteTemperature")
            && has_diagnostic_channel(node, "delay_history_underflow_failure")
            && node.get("source_span").is_some()
    });
    let has_predictor_node = behavior_nodes.iter().any(|node| {
        json_field_string(node, "behavior_kind").as_deref() == Some("predictor")
            && json_field_string(node, "signal").as_deref() == Some("temperature_signal")
            && json_field_string(node, "status").as_deref()
                == Some("predictor_call_contract_pending_integration")
            && json_field_string(node, "contract_status").as_deref()
                == Some("predictor_contract_metadata")
            && has_contract_quantity(node, "contract_inputs", "AbsoluteTemperature")
            && has_contract_quantity(node, "contract_outputs", "AbsoluteTemperature")
            && has_contract_status(
                node,
                "contract_outputs",
                "predictor_output_typed_identity_contract",
            )
            && has_diagnostic_channel(node, "predictor_valid_range_warning")
            && node.get("source_span").is_some()
    });
    let has_external_node = behavior_nodes.iter().any(|node| {
        json_field_string(node, "behavior_kind").as_deref() == Some("external")
            && json_field_string(node, "signal").as_deref() == Some("out.Q")
            && json_field_string(node, "status").as_deref()
                == Some("external_behavior_wrapper_pending_integration")
            && json_field_string(node, "contract_status").as_deref()
                == Some("external_behavior_contract_metadata")
            && has_contract_quantity(node, "contract_inputs", "HeatRate")
            && has_contract_quantity(node, "contract_outputs", "HeatRate")
            && has_contract_status(
                node,
                "contract_outputs",
                "external_output_typed_identity_contract",
            )
            && has_diagnostic_channel(node, "external_adapter_failure")
            && node.get("source_span").is_some()
    });
    if !has_delay_node || !has_predictor_node || !has_external_node {
        return Err(format!(
            "{} did not produce IDE delay/Predictor/external behavior graph inspector metadata",
            behavior_example.display()
        ));
    }
    let has_component_solver_result =
        domain_inspectors
            .assemblies
            .as_array()
            .is_some_and(|assemblies| {
                assemblies.iter().any(|assembly| {
                    let has_residual_graph_metadata = assembly
                        .get("residual_graph")
                        .and_then(|graph| graph.get("residual_metadata"))
                        .and_then(Value::as_array)
                        .is_some_and(|metadata| {
                            metadata.iter().any(|residual| {
                                json_field_string(residual, "kind").as_deref()
                                    == Some("through_conservation")
                                    && json_field_string(residual, "source_expression")
                                        .is_some_and(|expression| expression.contains("sum("))
                                    && json_field_string(residual, "expression_unit").as_deref()
                                        == Some("kW")
                                    && json_field_string(residual, "expression_quantity_kind")
                                        .as_deref()
                                        == Some("HeatRate")
                                    && residual.get("line").is_some()
                            })
                        });
                    let Some(solver_result) = assembly.get("solver_result") else {
                        return false;
                    };
                    json_field_string(solver_result, "method").as_deref()
                        == Some("linear_residual_graph_shape_check")
                        && json_field_f64(solver_result, "tolerance")
                            .is_some_and(|value| (value - 1e-9).abs() <= 1e-18)
                        && json_field_usize(solver_result, "max_iterations") == Some(1)
                        && solver_result
                            .get("variables")
                            .and_then(Value::as_array)
                            .is_some_and(|variables| !variables.is_empty())
                        && solver_result
                            .get("residuals")
                            .and_then(Value::as_array)
                            .is_some_and(|residuals| {
                                residuals.iter().any(|residual| {
                                    residual.get("normalized_value").is_some()
                                        && residual.get("scale_policy").is_some()
                                })
                            })
                        && solver_result
                            .get("failure_artifact")
                            .and_then(|failure| json_field_string(failure, "code"))
                            .as_deref()
                            == Some("E-ASSEMBLY-UNDERDETERMINED")
                        && solver_result
                            .get("failure_artifact")
                            .and_then(|failure| json_field_string(failure, "message"))
                            .is_some()
                        && json_field_string(solver_result, "failure_reason").is_some()
                        && has_residual_graph_metadata
                })
            });
    if !has_component_solver_result {
        return Err(format!(
            "{} did not produce IDE component solver result inspector metadata",
            domain_example.display()
        ));
    }
    let thermal_assembly_example =
        root.join("examples/internal/21_thermal_component_assembly/main.eng");
    let thermal_assembly_output = run_file(
        &thermal_assembly_example,
        &root.join("build").join("ide-smoke-thermal-assembly"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let thermal_assembly_cached = CachedRunOutput::from_output(thermal_assembly_output);
    let thermal_assembly_inspectors = runtime_inspectors(&root, &thermal_assembly_cached);
    let has_solved_component_solver_result = thermal_assembly_inspectors
        .assemblies
        .as_array()
        .is_some_and(|assemblies| {
            assemblies.iter().any(|assembly| {
                let has_boundary_rhs = assembly
                    .get("equations")
                    .and_then(Value::as_array)
                    .is_some_and(|equations| {
                        equations.iter().any(|equation| {
                            json_field_string(equation, "kind").as_deref()
                                == Some("component_boundary")
                                && json_field_string(equation, "rhs").as_deref() == Some("22 degC")
                        })
                    });
                let Some(solver_result) = assembly.get("solver_result") else {
                    return false;
                };
                let has_solved_variables = solver_result
                    .get("variables")
                    .and_then(Value::as_array)
                    .is_some_and(|variables| {
                        variables.iter().any(|variable| {
                            json_field_string(variable, "name").as_deref()
                                == Some("RoomBoundary.heat.T")
                                && json_field_f64(variable, "value")
                                    .is_some_and(|value| (value - 22.0).abs() <= 1e-9)
                        }) && variables.iter().any(|variable| {
                            json_field_string(variable, "name").as_deref()
                                == Some("AmbientBoundary.heat.Q")
                                && json_field_f64(variable, "value")
                                    .is_some_and(|value| (value + 1.0).abs() <= 1e-9)
                        })
                    });
                let has_satisfied_residuals = solver_result
                    .get("residuals")
                    .and_then(Value::as_array)
                    .is_some_and(|residuals| {
                        residuals.iter().any(|residual| {
                            json_field_string(residual, "status").as_deref() == Some("satisfied")
                                && residual.get("normalized_value").is_some()
                                && residual.get("scale_policy").is_some()
                        })
                    });
                json_field_string(solver_result, "status").as_deref() == Some("solved_linear")
                    && json_field_string(solver_result, "method").as_deref()
                        == Some("dense_linear_residual_graph")
                    && json_field_f64(solver_result, "tolerance")
                        .is_some_and(|value| (value - 1e-9).abs() <= 1e-18)
                    && json_field_usize(solver_result, "max_iterations") == Some(1)
                    && json_field_usize(solver_result, "iteration_count") == Some(1)
                    && json_field_string(solver_result, "convergence_status").as_deref()
                        == Some("linear_converged")
                    && has_boundary_rhs
                    && has_solved_variables
                    && has_satisfied_residuals
            })
        });
    if !has_solved_component_solver_result {
        return Err(format!(
            "{} did not produce IDE solved component assembly inspector metadata",
            thermal_assembly_example.display()
        ));
    }
    let multi_domain_example =
        root.join("examples/internal/22_multi_domain_boundary_solve/main.eng");
    let multi_domain_output = run_file(
        &multi_domain_example,
        &root.join("build").join("ide-smoke-multi-domain-boundary"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let multi_domain_cached = CachedRunOutput::from_output(multi_domain_output);
    let multi_domain_inspectors = runtime_inspectors(&root, &multi_domain_cached);
    let has_multi_domain_solver_result =
        multi_domain_inspectors
            .assemblies
            .as_array()
            .is_some_and(|assemblies| {
                assemblies.iter().any(|assembly| {
                    let Some(solver_result) = assembly.get("solver_result") else {
                        return false;
                    };
                    let has_multi_domain_shape = json_field_usize(assembly, "domain_count")
                        == Some(3)
                        && json_field_usize(assembly, "component_count") == Some(5)
                        && assembly
                            .get("residual_graph")
                            .and_then(|graph| graph.get("dependencies"))
                            .and_then(Value::as_array)
                            .is_some_and(|dependencies| {
                                dependencies.iter().any(|dependency| {
                                    json_field_string(dependency, "variable").as_deref()
                                        == Some("SupplyPipe.outlet.m_dot")
                                }) && dependencies.iter().any(|dependency| {
                                    json_field_string(dependency, "variable").as_deref()
                                        == Some("ShaftB.shaft.P")
                                })
                            });
                    let has_solved_cross_domain_variables = solver_result
                        .get("variables")
                        .and_then(Value::as_array)
                        .is_some_and(|variables| {
                            variables.iter().any(|variable| {
                                json_field_string(variable, "name").as_deref()
                                    == Some("SupplyPipe.outlet.m_dot")
                                    && json_field_f64(variable, "value")
                                        .is_some_and(|value| (value + 0.2).abs() <= 1e-9)
                            }) && variables.iter().any(|variable| {
                                json_field_string(variable, "name").as_deref()
                                    == Some("ShaftB.shaft.P")
                                    && json_field_f64(variable, "value")
                                        .is_some_and(|value| (value + 100.0).abs() <= 1e-9)
                            })
                        });
                    json_field_string(solver_result, "status").as_deref() == Some("solved_linear")
                        && json_field_string(solver_result, "method").as_deref()
                            == Some("dense_linear_residual_graph")
                        && json_field_f64(solver_result, "tolerance")
                            .is_some_and(|value| (value - 1e-9).abs() <= 1e-18)
                        && json_field_usize(solver_result, "max_iterations") == Some(1)
                        && json_field_usize(solver_result, "iteration_count") == Some(1)
                        && json_field_string(solver_result, "convergence_status").as_deref()
                            == Some("linear_converged")
                        && has_multi_domain_shape
                        && has_solved_cross_domain_variables
                        && solver_result
                            .get("largest_residuals")
                            .and_then(Value::as_array)
                            .is_some_and(|residuals| !residuals.is_empty())
                })
            });
    if !has_multi_domain_solver_result {
        return Err(format!(
            "{} did not produce IDE multi-domain boundary solve inspector metadata",
            multi_domain_example.display()
        ));
    }
    let advanced_multi_domain_example =
        root.join("examples/advanced_solver/32_small_thermal_fluid_loop/main.eng");
    let advanced_multi_domain_output = run_file(
        &advanced_multi_domain_example,
        &root.join("build").join("ide-smoke-advanced-thermal-fluid"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let advanced_multi_domain_cached = CachedRunOutput::from_output(advanced_multi_domain_output);
    let advanced_multi_domain_inspectors = runtime_inspectors(&root, &advanced_multi_domain_cached);
    let has_advanced_multi_domain_component_graph = advanced_multi_domain_inspectors
        .component_graph
        .get("connections")
        .and_then(Value::as_array)
        .is_some_and(|connections| {
            connections.iter().any(|connection| {
                json_field_string(connection, "id").as_deref()
                    == Some("pipe.outlet -> return_node.inlet")
                    && connection.get("source_span").is_some()
            })
        });
    let has_advanced_multi_domain_solver_result = advanced_multi_domain_inspectors
        .assemblies
        .as_array()
        .is_some_and(|assemblies| {
            assemblies.iter().any(|assembly| {
                let Some(solver_result) = assembly.get("solver_result") else {
                    return false;
                };
                let has_domain_plans = assembly
                    .get("domain_plans")
                    .and_then(Value::as_array)
                    .is_some_and(|plans| {
                        plans.iter().any(|plan| {
                            json_field_string(plan, "domain").as_deref() == Some("Thermal")
                                && json_field_usize(plan, "equation_count") == Some(4)
                        }) && plans.iter().any(|plan| {
                            json_field_string(plan, "domain").as_deref() == Some("Fluid[Water]")
                                && json_field_usize(plan, "equation_count") == Some(8)
                        })
                    });
                let has_equation_panel_shape = assembly
                    .get("equations")
                    .and_then(Value::as_array)
                    .is_some_and(|equations| {
                        equations.iter().any(|equation| {
                            json_field_string(equation, "name").as_deref()
                                == Some("connection_set_2.across_p_1")
                                && json_field_string(equation, "kind").as_deref()
                                    == Some("across_equality")
                                && json_field_string(equation, "domain").as_deref()
                                    == Some("Fluid[Water]")
                                && json_field_usize(equation, "line") == Some(55)
                                && equation
                                    .get("dependencies")
                                    .and_then(Value::as_array)
                                    .is_some_and(|dependencies| {
                                        dependencies
                                            .iter()
                                            .any(|value| value.as_str() == Some("pump.supply.p"))
                                            && dependencies
                                                .iter()
                                                .any(|value| value.as_str() == Some("pipe.inlet.p"))
                                    })
                        }) && equations.iter().any(|equation| {
                            json_field_string(equation, "name").as_deref()
                                == Some("pipe.equation_1")
                                && json_field_string(equation, "kind").as_deref()
                                    == Some("component_equation")
                                && json_field_string(equation, "domain").as_deref()
                                    == Some("Fluid[Water]")
                                && json_field_usize(equation, "line") == Some(35)
                                && equation
                                    .get("dependencies")
                                    .and_then(Value::as_array)
                                    .is_some_and(|dependencies| {
                                        dependencies
                                            .iter()
                                            .any(|value| value.as_str() == Some("pipe.inlet.p"))
                                            && dependencies.iter().any(|value| {
                                                value.as_str() == Some("pipe.outlet.p")
                                            })
                                    })
                        })
                    });
                let has_dependency_graph = assembly
                    .get("residual_graph")
                    .and_then(|graph| graph.get("dependencies"))
                    .and_then(Value::as_array)
                    .is_some_and(|dependencies| {
                        dependencies.iter().any(|dependency| {
                            json_field_string(dependency, "residual").as_deref()
                                == Some("pipe.equation_1")
                                && json_field_string(dependency, "variable").as_deref()
                                    == Some("pipe.outlet.p")
                        }) && dependencies.iter().any(|dependency| {
                            json_field_string(dependency, "residual").as_deref()
                                == Some("pipe.equation_2")
                                && json_field_string(dependency, "variable").as_deref()
                                    == Some("pipe.inlet.m_dot")
                        })
                    });
                let has_residual_panel = solver_result
                    .get("residuals")
                    .and_then(Value::as_array)
                    .is_some_and(|residuals| {
                        residuals.iter().any(|residual| {
                            json_field_string(residual, "name").as_deref()
                                == Some("pipe.equation_1")
                                && json_field_f64(residual, "value")
                                    .is_some_and(|value| value.abs() <= 1e-12)
                                && json_field_string(residual, "unit").as_deref() == Some("Pa")
                                && json_field_string(residual, "expression_unit").is_some()
                                && json_field_string(residual, "expression_quantity_kind").is_some()
                                && json_field_f64(residual, "normalized_value")
                                    .is_some_and(|value| value.abs() <= 1e-12)
                                && json_field_string(residual, "scale_policy").is_some()
                                && json_field_string(residual, "status").as_deref()
                                    == Some("satisfied")
                        })
                    })
                    && solver_result
                        .get("largest_residuals")
                        .and_then(Value::as_array)
                        .is_some_and(|residuals| !residuals.is_empty());
                let has_solved_variables = solver_result
                    .get("variables")
                    .and_then(Value::as_array)
                    .is_some_and(|variables| {
                        variables.iter().any(|variable| {
                            json_field_string(variable, "name").as_deref() == Some("pump.supply.p")
                                && json_field_f64(variable, "value")
                                    .is_some_and(|value| (value - 220000.0).abs() <= 1e-9)
                        }) && variables.iter().any(|variable| {
                            json_field_string(variable, "name").as_deref()
                                == Some("pipe.outlet.m_dot")
                                && json_field_f64(variable, "value")
                                    .is_some_and(|value| (value - 0.2).abs() <= 1e-9)
                        })
                    });
                json_field_usize(assembly, "domain_count") == Some(2)
                    && json_field_usize(assembly, "component_count") == Some(5)
                    && json_field_usize(assembly, "component_equation_count") == Some(6)
                    && json_field_string(solver_result, "status").as_deref()
                        == Some("solved_linear")
                    && json_field_string(solver_result, "method").as_deref()
                        == Some("dense_linear_residual_graph")
                    && json_field_f64(solver_result, "tolerance")
                        .is_some_and(|value| (value - 1e-9).abs() <= 1e-18)
                    && json_field_usize(solver_result, "max_iterations") == Some(1)
                    && json_field_usize(solver_result, "iteration_count") == Some(1)
                    && json_field_string(solver_result, "convergence_status").as_deref()
                        == Some("linear_converged")
                    && has_domain_plans
                    && has_equation_panel_shape
                    && has_dependency_graph
                    && has_residual_panel
                    && has_solved_variables
            })
        });
    if !has_advanced_multi_domain_component_graph || !has_advanced_multi_domain_solver_result {
        return Err(format!(
            "{} did not produce IDE advanced Thermal/Fluid solver, equation, residual, and graph inspector metadata",
            advanced_multi_domain_example.display()
        ));
    }
    let measured_example = root.join("examples/internal/17_measured_vs_simulated/main.eng");
    let measured_output = run_file(
        &measured_example,
        &root.join("build").join("ide-smoke"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let measured_cached = CachedRunOutput::from_output(measured_output);
    let inspectors = runtime_inspectors(&root, &measured_cached);
    for (label, value) in [
        ("schema", &inspectors.schemas),
        ("timeseries", &inspectors.time_series),
        ("metric", &inspectors.metrics),
        ("validation", &inspectors.validations),
        ("time alignment", &inspectors.time_alignments),
        ("artifact outline", &inspectors.artifact_outlines),
    ] {
        if value.as_array().is_none_or(Vec::is_empty) {
            return Err(format!(
                "{} did not produce IDE {label} inspector metadata",
                measured_example.display()
            ));
        }
    }
    let state_space_example = root.join("examples/internal/18_state_space_metadata/main.eng");
    let state_space_output = run_file(
        &state_space_example,
        &root.join("build").join("ide-smoke-state-space"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let state_space_cached = CachedRunOutput::from_output(state_space_output);
    let state_space_inspectors = runtime_inspectors(&root, &state_space_cached);
    let has_state_space_series =
        state_space_inspectors
            .time_series
            .as_array()
            .is_some_and(|items| {
                items.iter().any(|item| {
                    json_field_string(item, "name").as_deref() == Some("sim.T_zone")
                        && json_field_string(item, "axis").as_deref() == Some("Time")
                        && json_field_usize(item, "row_count").unwrap_or(0) > 0
                })
            });
    let has_state_space_solver = state_space_inspectors
        .systems
        .as_array()
        .is_some_and(|items| {
            items.iter().any(|item| {
                item.get("solver_result")
                    .and_then(|solver| json_field_string(solver, "method"))
                    .as_deref()
                    == Some("state_space_explicit_euler_fixed_step")
            })
        });
    let has_state_space_operator_matrix = state_space_inspectors
        .linear_operators
        .as_array()
        .is_some_and(|items| {
            items.iter().any(|item| {
                json_field_string(item, "name").as_deref() == Some("A")
                    && item
                        .get("canonical_matrix")
                        .and_then(Value::as_array)
                        .and_then(|rows| rows.first())
                        .and_then(Value::as_array)
                        .and_then(|row| row.first())
                        .and_then(Value::as_f64)
                        .is_some_and(|value| (value - (-0.0002)).abs() < 1e-12)
            })
        });
    let has_state_space_operator_entry = state_space_inspectors
        .linear_operators
        .as_array()
        .is_some_and(|items| {
            items.iter().any(|item| {
                json_field_string(item, "name").as_deref() == Some("B")
                    && item
                        .get("canonical_entries")
                        .and_then(Value::as_array)
                        .is_some_and(|entries| {
                            entries.iter().any(|entry| {
                                json_field_string(entry, "row_member").as_deref() == Some("T_zone")
                                    && json_field_string(entry, "column_member").as_deref()
                                        == Some("Q_internal")
                                    && entry
                                        .get("coefficient")
                                        .and_then(Value::as_f64)
                                        .is_some_and(|value| (value - 0.001).abs() < 1e-12)
                            })
                        })
            })
        });
    let has_state_space_source_equations =
        state_space_inspectors
            .systems
            .as_array()
            .is_some_and(|items| {
                items.iter().any(|item| {
                    item.get("solver_results")
                        .and_then(Value::as_array)
                        .is_some_and(|solver_results| {
                            solver_results.iter().any(|solver_result| {
                                solver_result
                                    .get("source_equations")
                                    .and_then(Value::as_array)
                                    .is_some_and(|equations| {
                                        equations.iter().any(|equation| {
                                            json_field_string(equation, "kind")
                                                .as_deref()
                                                .is_some_and(|kind| {
                                                    kind.starts_with("state_space_")
                                                })
                                        })
                                    })
                            })
                        })
                })
            });
    if !has_state_space_series
        || !has_state_space_solver
        || !has_state_space_operator_matrix
        || !has_state_space_source_equations
    {
        return Err(format!(
            "{} did not produce IDE state-space trajectory/operator/source-equation inspector metadata",
            state_space_example.display()
        ));
    }
    if !has_state_space_operator_entry {
        return Err(format!(
            "{} did not produce IDE state-space named operator entry metadata",
            state_space_example.display()
        ));
    }
    let adaptive_example = root.join("examples/internal/27_adaptive_heun_thermal/main.eng");
    let adaptive_output = run_file(
        &adaptive_example,
        &root.join("build").join("ide-smoke-adaptive-heun"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let adaptive_cached = CachedRunOutput::from_output(adaptive_output);
    let adaptive_inspectors = runtime_inspectors(&root, &adaptive_cached);
    let has_adaptive_substeps = adaptive_inspectors.systems.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("solver_results")
                .and_then(Value::as_array)
                .is_some_and(|solver_results| {
                    solver_results.iter().any(|solver_result| {
                        json_field_string(solver_result, "method").as_deref()
                            == Some("adaptive_heun")
                            && solver_result
                                .get("step_diagnostics")
                                .and_then(Value::as_array)
                                .is_some_and(|diagnostics| {
                                    !diagnostics.is_empty()
                                        && diagnostics.iter().any(|diagnostic| {
                                            json_field_string(diagnostic, "status").as_deref()
                                                == Some("accepted")
                                        })
                                })
                    })
                })
        })
    });
    let has_adaptive_timeseries_metadata =
        adaptive_inspectors
            .time_series
            .as_array()
            .is_some_and(|items| {
                items.iter().any(|item| {
                    json_field_string(item, "name").as_deref() == Some("sim.T_zone")
                        && item
                            .get("integration_metadata")
                            .and_then(|metadata| json_field_usize(metadata, "substep_count"))
                            .unwrap_or(0)
                            > 0
                })
            });
    if !has_adaptive_substeps || !has_adaptive_timeseries_metadata {
        return Err(format!(
            "{} did not produce IDE adaptive solver substep inspector metadata",
            adaptive_example.display()
        ));
    }
    let jit_example = root.join("examples/official/01_csv_plot/main.eng");
    let jit_output = run_file(
        &jit_example,
        &root.join("build").join("ide-smoke-jit-kernels"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let jit_cached = CachedRunOutput::from_output(jit_output);
    let jit_inspectors = runtime_inspectors(&root, &jit_cached);
    let has_kernel_plan = jit_inspectors
        .kernel_plan
        .get("candidates")
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items.iter().any(|item| {
                json_field_string(item, "kind").as_deref() == Some("timeseries_integrate")
                    && item
                        .get("executor")
                        .and_then(|executor| json_field_string(executor, "status"))
                        .as_deref()
                        == Some("interpreter_supported")
                    && item
                        .get("executor")
                        .and_then(|executor| json_field_string(executor, "fallback_reason"))
                        .is_some_and(|reason| reason.contains("interpreter kernel IR"))
            })
        });
    if !has_kernel_plan {
        return Err(format!(
            "{} did not produce IDE kernel plan fallback inspector metadata",
            jit_example.display()
        ));
    }
    let class_example = root.join("examples/official/19_class_object/main.eng");
    let class_output = run_file(
        &class_example,
        &root.join("build").join("ide-smoke-class-object"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let class_cached = CachedRunOutput::from_output(class_output);
    let class_inspectors = runtime_inspectors(&root, &class_cached);
    let has_class_object = class_inspectors
        .class_objects
        .as_array()
        .is_some_and(|items| {
            items.iter().any(|item| {
                json_field_string(item, "name").as_deref() == Some("building")
                    && json_field_string(item, "class_name").as_deref() == Some("Building")
                    && json_field_usize(item, "field_count").unwrap_or(0) > 0
                    && json_field_usize(item, "validation_count").unwrap_or(0) > 0
            })
        });
    if !has_class_object {
        return Err(format!(
            "{} did not produce IDE class object inspector metadata",
            class_example.display()
        ));
    }
    let effects_example = root.join("examples/official/15_process_result/main.eng");
    let effects_output = run_file(
        &effects_example,
        &root.join("build").join("ide-smoke-effects"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let effects_cached = CachedRunOutput::from_output(effects_output);
    let effects_inspectors = runtime_inspectors(&root, &effects_cached);
    let has_effect_records =
        effect_records_has_artifact_and_boundary_records(&effects_inspectors.effect_records);
    let has_review_boundary =
        review_document_has_external_boundary(&effects_inspectors.review_document);
    if !has_effect_records || !has_review_boundary {
        return Err(format!(
            "{} did not produce IDE side-effect inspector metadata (effect_records={}, review_boundary={})",
            effects_example.display(),
            has_effect_records,
            has_review_boundary
        ));
    }
    let file_effects_example = root.join("examples/official/13_file_operations/main.eng");
    let file_effects_output = run_file(
        &file_effects_example,
        &root.join("build").join("ide-smoke-file-effects"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let file_effects_cached = CachedRunOutput::from_output(file_effects_output);
    let file_effects_inspectors = runtime_inspectors(&root, &file_effects_cached);
    if !review_document_has_side_effect(&file_effects_inspectors.review_document) {
        return Err(format!(
            "{} did not produce IDE review side-effect metadata",
            file_effects_example.display()
        ));
    }
    let test_example = root.join("examples/official/16_test_assert_golden/main.eng");
    let test_output = run_file(
        &test_example,
        &root.join("build").join("ide-smoke-tests"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let test_cached = CachedRunOutput::from_output(test_output);
    let test_inspectors = runtime_inspectors(&root, &test_cached);
    let has_tests = test_inspectors
        .test_results
        .get("tests")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    if !has_tests {
        return Err(format!(
            "{} did not produce IDE test-result inspector metadata",
            test_example.display()
        ));
    }
    let data_quality_example = root.join("examples/diagnostics/data_quality/bad_numeric_cell.eng");
    let data_quality_output = run_file(
        &data_quality_example,
        &root.join("build").join("ide-smoke-data-quality"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let data_quality_cached = CachedRunOutput::from_output(data_quality_output);
    let data_quality_inspectors = runtime_inspectors(&root, &data_quality_cached);
    let has_schema_failure_counts =
        data_quality_inspectors
            .schemas
            .as_array()
            .is_some_and(|items| {
                items.iter().any(|item| {
                    json_field_usize(item, "parse_failure_count").unwrap_or(0) > 0
                        || json_field_usize(item, "conversion_failure_count").unwrap_or(0) > 0
                })
            });
    if !has_schema_failure_counts {
        return Err(format!(
            "{} did not produce IDE schema failure inspector metadata",
            data_quality_example.display()
        ));
    }

    let table_example = root.join("tests/runtime/table_datetime_comparison.eng");
    if table_example.exists() {
        let table_output = run_file(
            &table_example,
            &root.join("build").join("ide-smoke-table-transforms"),
            &RunOptions::default(),
        )
        .map_err(|error| error.to_string())?;
        let table_cached = CachedRunOutput::from_output(table_output);
        let table_inspectors = runtime_inspectors(&root, &table_cached);
        let has_table_transform =
            table_inspectors
                .table_transforms
                .as_array()
                .is_some_and(|items| {
                    items.iter().any(|item| {
                        json_field_string(item, "binding").as_deref() == Some("exact")
                            && json_field_usize(item, "predicate_count").unwrap_or(0) > 0
                            && json_field_usize(item, "row_diagnostic_count").unwrap_or(0) > 0
                    })
                });
        if !has_table_transform {
            return Err(format!(
                "{} did not produce IDE table transform inspector metadata",
                table_example.display()
            ));
        }
    }
    println!(
        "EngLang IDE smoke OK: {} example(s), {} quantity completion(s), {} unit completion(s), {} domain(s), {} component(s), {} connection(s), {} assembly graph(s), residual dependency inspector, behavior graph inspector, measured workflow inspectors, solved thermal assembly inspector, multi-domain boundary solve inspector, advanced Thermal/Fluid solver inspector, state-space trajectory/operator/source-equation inspector, kernel plan inspector, class object inspector, normalized review cockpit, side-effect inspectors, schema failure inspector, table transform inspector",
        examples.len(),
        all_quantity_completions().len(),
        all_unit_infos().len(),
        domain_report.semantic_program.domains.len(),
        domain_report.semantic_program.components.len(),
        domain_report.semantic_program.connections.len(),
        domain_report.semantic_program.component_assemblies.len()
    );
    Ok(())
}

fn assert_native_ide_ui_behavior_status_labels(root: &Path) -> Result<(), String> {
    let app_js = read_utf8(
        &root
            .join("crates")
            .join("eng_ide")
            .join("ui")
            .join("app.js"),
    )?;
    for required in [
        "function statusLabel(status)",
        "delay runtime buffer not connected to this language-level solve",
        "Predictor contract not connected to this language-level solve",
        "safe/repro profile policy metadata",
        "statusLabel(node.status || \"-\")",
        "relationship=${statusLabel(relationship)}",
        "copyVisibleHighlightsBtn.onclick = copyVisibleHighlights",
        "function highlightTokenCopyText(tokens)",
        "function semanticTokenSelectors(token)",
        "<th>Selectors</th>",
        "function renderSemanticSelectorTable(counts)",
        "Selector ${selector}",
        "selectors=${selectors}",
        "function completionInsertEdit(item)",
        "function snippetInsertEdit(snippet)",
        "item.insertSnippet",
        "placeholderText",
        "editor.selectionStart = before.length + edit.selectionStart",
    ] {
        if !app_js.contains(required) {
            return Err(format!(
                "native IDE app.js should include UI behavior contract `{required}`"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn finds_workspace_root_from_target_release_child() {
        let root = unique_temp_root();
        let release_dir = root.join("target").join("release");
        fs::create_dir_all(root.join("examples")).unwrap();
        fs::create_dir_all(root.join("stdlib")).unwrap();
        fs::create_dir_all(&release_dir).unwrap();

        let found = find_workspace_root(&release_dir).unwrap();
        assert_eq!(found, root);

        fs::remove_dir_all(found).unwrap();
    }

    #[test]
    fn terminal_rejects_function_call_print_syntax() {
        let check = terminal_command_error("print(Q_coil)").expect("terminal error");
        assert_eq!(check.status, "1 error(s), 0 warning(s)");
        assert_eq!(check.diagnostics[0].code, "E-IDE-TERMINAL-SYNTAX");
        assert!(check.diagnostics[0].message.contains("string-template"));
    }

    #[test]
    fn terminal_allows_assignments_with_function_calls() {
        assert!(terminal_command_error("x = mean(Q, axis=Time)").is_none());
        assert!(terminal_command_error("x =3").is_none());
    }

    #[test]
    fn terminal_rejects_unrecognized_commands() {
        let check =
            terminal_unrecognized_command_error("unknown_command", Path::new(".")).expect("error");
        assert_eq!(check.diagnostics[0].code, "E-IDE-TERMINAL-SYNTAX");
        assert!(check.diagnostics[0].message.contains("not recognized"));
        assert!(check.diagnostics[0]
            .help
            .as_deref()
            .unwrap()
            .contains("cd <dir>"));
    }

    fn semantic_token_text<'a>(source: &'a str, token: &Value) -> Option<&'a str> {
        let line = token.get("line")?.as_u64()? as usize;
        let start = token.get("start")?.as_u64()? as usize;
        let length = token.get("length")?.as_u64()? as usize;
        source.lines().nth(line)?.get(start..start + length)
    }

    fn has_semantic_token_text_with_modifier(
        source: &str,
        tokens: &[Value],
        text: &str,
        token_type: &str,
        modifier: &str,
    ) -> bool {
        tokens.iter().any(|token| {
            token.get("type").and_then(Value::as_str) == Some(token_type)
                && semantic_token_text(source, token) == Some(text)
                && token
                    .get("modifiers")
                    .and_then(Value::as_array)
                    .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(modifier)))
        })
    }

    #[test]
    fn check_view_surfaces_lsp_semantic_tokens() {
        let root = workspace_root();
        let path = root
            .join("examples")
            .join("official")
            .join("01_csv_plot")
            .join("main.eng");
        let source = read_utf8(&path).expect("example source");
        let check = check_view(&path, &source);
        let legend_modifiers = check
            .semantic_tokens
            .pointer("/legend/token_modifiers")
            .and_then(Value::as_array)
            .expect("semantic token modifiers");
        assert!(legend_modifiers
            .iter()
            .any(|modifier| modifier.as_str() == Some("unit")));
        assert!(legend_modifiers
            .iter()
            .any(|modifier| modifier.as_str() == Some("riskHigh")));

        let tokens = check
            .semantic_tokens
            .get("tokens")
            .and_then(Value::as_array)
            .expect("semantic tokens");
        assert!(tokens
            .iter()
            .any(|token| { token.get("type").and_then(Value::as_str) == Some("keyword") }));
        assert!(tokens.iter().any(|token| {
            token
                .get("modifiers")
                .and_then(Value::as_array)
                .is_some_and(|items| items.iter().any(|item| item.as_str() == Some("unit")))
        }));
        assert!(check
            .hovers
            .as_array()
            .is_some_and(|hovers| hovers.iter().any(|hover| hover["name"] == "Q_coil")));

        let rich_source = r#"designs = sample lhs
with {
    count = 2
    seed = 7
    people_density = uniform(0.03 person/m2, 0.12 person/m2)
    annual_electricity = uniform(100 kWh, 200 kWh)
}
Q_dist = distribution(kind=normal, mean=5 kW, sigma=0.8 kW, n=31)
with {
    sensor_std = 0.2 kW
}
model = train regression designs
with {
    y = annual_electricity
    x = [people_density]
    seed = 7
}
predictions = predict model using designs
db = open sqlite file("outputs/results.sqlite")
write predictions to db.table("predictions")
with {
    mode = append
}
"#;
        let rich_check = check_view(Path::new("ide_semantic_roles.eng"), rich_source);
        let rich_tokens = rich_check
            .semantic_tokens
            .get("tokens")
            .and_then(Value::as_array)
            .expect("rich semantic tokens");
        for modifier in ["workflowStep", "uncertain", "model", "db", "sideEffect"] {
            assert!(
                has_semantic_token_text_with_modifier(
                    rich_source,
                    rich_tokens,
                    "with",
                    "keyword",
                    modifier
                ),
                "native IDE semantic payload should surface `with` keyword modifier {modifier}"
            );
        }
    }

    #[test]
    fn native_ide_completions_use_lsp_editor_items() {
        let lsp_items = eng_lsp::editor_completion_items();
        let completions = base_completion_items();
        assert!(completions.len() >= lsp_items.len());

        let labels = completions
            .iter()
            .map(|completion| completion.label.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(labels.len(), completions.len());
        for required in [
            "promote json records",
            "eng.table",
            "HeatRate",
            "Array[T]",
            "List[T]",
            "kW",
        ] {
            assert!(
                labels.contains(required),
                "native IDE completions should include LSP label {required}"
            );
        }
        for completion in &lsp_seed {
            assert!(
                completions.iter().any(|item| item.label == completion.label
                    && item.detail == completion.detail
                    && item.kind == completion.kind
                    && item.insert_snippet.as_deref() == completion.insert_snippet.as_deref()),
                "native IDE completion items should mirror LSP completion {}",
                completion.label
            );
        }
        let snippet = completions
            .iter()
            .find(|completion| completion.label == "class object")
            .expect("native IDE snippet");
        assert!(snippet.insert.contains("validate"));
        assert!(
            completions
                .iter()
                .all(|completion| completion.label != "export summary csv"),
            "native IDE completions should not expose the stale export summary csv alias"
        );
        let export_summary = completions
            .iter()
            .find(|completion| completion.label == "export summary to csv")
            .expect("export summary to csv completion");
        assert_eq!(
            export_summary.insert,
            "export summary to csv join(args.output, \"summary.csv\")"
        );
        assert!(export_summary
            .insert_snippet
            .as_deref()
            .is_some_and(|snippet| snippet.contains("export summary to csv join")));
        let read_text = completions
            .iter()
            .find(|completion| completion.label == "read text")
            .expect("read text completion");
        assert_eq!(read_text.insert, "read text args.input");
        let mkdir_dir = completions
            .iter()
            .find(|completion| completion.label == "mkdir dir")
            .expect("mkdir dir completion");
        assert_eq!(mkdir_dir.insert, "mkdir \"outputs/archive\"");
        let file_helper = completions
            .iter()
            .find(|completion| completion.label == "file(...)")
            .expect("file helper completion");
        assert_eq!(file_helper.insert, "file(\"data/input.csv\")");
        assert_eq!(
            file_helper.insert_snippet.as_deref(),
            Some("file(\"${1:data/input.csv}\")")
        );
        let array_type = completions
            .iter()
            .find(|completion| completion.label == "Array[T]")
            .expect("Array[T] completion");
        assert_eq!(array_type.insert_snippet.as_deref(), Some("Array[${1:T}]"));
    }

    #[test]
    fn native_ide_bootstrap_exposes_lsp_syntax_catalog() {
        let workspace = ide_bootstrap().expect("native IDE bootstrap");
        let catalog = workspace.syntax_catalog;
        assert!(catalog["keywords"].as_array().is_some_and(|items| items
            .iter()
            .any(|item| item.as_str() == Some("distribution"))));
        assert!(catalog["constants"].as_array().is_some_and(|items| items
            .iter()
            .any(|item| item.as_str() == Some("source_linear_terms"))));
        assert!(catalog["operator_words"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item.as_str() == Some("within"))));
        assert!(catalog["workflow_builtins"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item.as_str() == Some("train"))));
        assert!(catalog["hyphenated_workflow_builtins"]
            .as_array()
            .is_some_and(|items| items
                .iter()
                .any(|item| item.as_str() == Some("latin-hypercube"))));
        assert!(catalog["workflow_options"]
            .as_array()
            .is_some_and(|items| items
                .iter()
                .any(|item| item["label"].as_str() == Some("offline_response"))));
        assert!(catalog["public_types"].as_array().is_some_and(|items| items
            .iter()
            .any(|item| item["base"].as_str() == Some("Table")
                && item["label"].as_str() == Some("Table[T]"))));
        assert!(catalog["units"].as_array().is_some_and(|items| items
            .iter()
            .any(|item| item["label"].as_str() == Some("W/m^2"))));
    }

    #[test]
    fn native_ide_format_uses_compiler_formatter() {
        let formatted = ide_format(
            "format.eng".to_owned(),
            "report {\nplot Q over Time\nwith {\ntitle = \"Q\"\n}\n}\n".to_owned(),
        );
        assert!(formatted.changed);
        assert_eq!(
            formatted.source,
            "report {\n    plot Q over Time\n    with {\n        title = \"Q\"\n    }\n}\n"
        );

        let clean = ide_format("format.eng".to_owned(), formatted.source.clone());
        assert!(!clean.changed);
        assert_eq!(clean.source, formatted.source);
    }

    #[test]
    fn native_ide_ui_maps_behavior_preview_status_labels() {
        let root = workspace_root();
        assert_native_ide_ui_behavior_status_labels(&root).expect("native IDE UI labels");
    }

    #[test]
    fn terminal_summary_only_returns_stdout() {
        assert_eq!(
            terminal_summary(
                "hello\n",
                &[],
                &[],
                "report",
                &serde_json::json!({ "series": [{ "points": [{ "x": 1, "y": 2 }] }] })
            ),
            "hello"
        );
        assert!(terminal_summary("", &[], &[], "", &Value::Null).is_empty());
    }

    #[test]
    fn time_series_inspector_includes_system_solver_substep_metadata() {
        let result = json!({
            "typed_payload": {
                "systems": [
                    {
                        "name": "AdaptiveThermal",
                        "solver_results": [
                            {
                                "status": "computed",
                                "binding": "sim",
                                "method": "adaptive_heun",
                                "state": "T_zone",
                                "display_unit": "degC",
                                "canonical_unit": "K",
                                "time_step": 1.0,
                                "step_count": 2,
                                "duration": 2.0,
                                "final_value": 22.0,
                                "points": [
                                    { "x": 0.0, "y": 21.0 },
                                    { "x": 1.0, "y": 21.5 },
                                    { "x": 2.0, "y": 22.0 }
                                ],
                                "step_diagnostics": [
                                    {
                                        "output_index": 1,
                                        "start_time_s": 0.0,
                                        "end_time_s": 0.5,
                                        "dt_s": 0.5,
                                        "error_norm": 0.00001,
                                        "status": "accepted"
                                    },
                                    {
                                        "output_index": 1,
                                        "start_time_s": 0.5,
                                        "end_time_s": 0.5,
                                        "dt_s": 0.5,
                                        "error_norm": 0.01,
                                        "status": "rejected_error_above_tolerance"
                                    }
                                ]
                            }
                        ]
                    }
                ]
            }
        });
        let rows = time_series_inspector(&json!({}), &result);
        let rows = rows.as_array().expect("time-series rows");

        assert!(rows.iter().any(|row| {
            json_field_string(row, "name").as_deref() == Some("sim.T_zone")
                && row
                    .get("integration_metadata")
                    .and_then(|metadata| json_field_usize(metadata, "substep_count"))
                    == Some(2)
                && row
                    .get("integration_metadata")
                    .and_then(|metadata| json_field_usize(metadata, "accepted_substep_count"))
                    == Some(1)
                && row
                    .get("integration_metadata")
                    .and_then(|metadata| json_field_usize(metadata, "rejected_substep_count"))
                    == Some(1)
        }));
    }

    #[test]
    fn time_series_inspector_includes_component_solver_trajectories() {
        let report = json!({
            "assembly_summary": [
                {
                    "name": "component_graph",
                    "solver_result": {
                        "status": "computed",
                        "method": "dynamic_component_explicit_euler",
                        "convergence_status": "dynamic_component_fixed_step_completed",
                        "trajectories": [
                            {
                                "name": "x",
                                "role": "state",
                                "unit": "1",
                                "initial_value": 1.0,
                                "final_value": 3.0,
                                "point_count": 3,
                                "points": [
                                    { "x": 0.0, "y": 1.0 },
                                    { "x": 1.0, "y": 2.0 },
                                    { "x": 2.0, "y": 3.0 }
                                ]
                            },
                            {
                                "name": "z",
                                "role": "algebraic",
                                "unit": "1",
                                "initial_value": 2.0,
                                "final_value": 4.0,
                                "point_count": 3,
                                "points": [
                                    { "x": 0.0, "y": 2.0 },
                                    { "x": 1.0, "y": 3.0 },
                                    { "x": 2.0, "y": 4.0 }
                                ]
                            }
                        ]
                    }
                }
            ]
        });
        let rows = time_series_inspector(&report, &json!({}));
        let rows = rows.as_array().expect("time-series rows");

        assert!(rows.iter().any(|row| {
            json_field_string(row, "name").as_deref() == Some("component_graph.x")
                && json_field_string(row, "interpolation_policy").as_deref()
                    == Some("fixed-step component-solver")
                && row
                    .get("integration_metadata")
                    .and_then(|metadata| json_field_string(metadata, "role"))
                    .as_deref()
                    == Some("state")
        }));
        assert!(rows.iter().any(|row| {
            json_field_string(row, "name").as_deref() == Some("component_graph.z")
                && row
                    .get("integration_metadata")
                    .and_then(|metadata| json_field_string(metadata, "role"))
                    .as_deref()
                    == Some("algebraic")
        }));
    }

    #[test]
    fn time_series_inspector_includes_component_solver_failure_metadata() {
        let report = json!({
            "assembly_summary": [
                {
                    "name": "component_graph",
                    "solver_result": {
                        "status": "failed",
                        "method": "dynamic_component_explicit_euler",
                        "convergence_status": "algebraic_solve_failed",
                        "failure_artifact": {
                            "code": "E-FIXED-POINT-NONCONVERGENCE",
                            "message": "fixed-point iteration did not converge"
                        },
                        "trajectories": [
                            {
                                "name": "z",
                                "role": "algebraic",
                                "unit": "1",
                                "initial_value": 3.0,
                                "final_value": 3.0,
                                "point_count": 1,
                                "points": [
                                    { "x": 0.0, "y": 3.0 }
                                ]
                            }
                        ]
                    }
                }
            ]
        });
        let rows = time_series_inspector(&report, &json!({}));
        let rows = rows.as_array().expect("time-series rows");

        assert!(rows.iter().any(|row| {
            json_field_string(row, "name").as_deref() == Some("component_graph.z")
                && row
                    .get("integration_metadata")
                    .and_then(|metadata| json_field_string(metadata, "convergence_status"))
                    .as_deref()
                    == Some("algebraic_solve_failed")
                && row
                    .get("integration_metadata")
                    .and_then(|metadata| json_field_string(metadata, "failure_code"))
                    .as_deref()
                    == Some("E-FIXED-POINT-NONCONVERGENCE")
                && row
                    .get("integration_metadata")
                    .and_then(|metadata| json_field_string(metadata, "failure_reason"))
                    .is_some_and(|reason| reason.contains("fixed-point iteration"))
        }));
    }

    #[test]
    fn empty_plot_specs_are_not_available() {
        assert!(!has_plot_data(&serde_json::json!({
            "plot_type": "line",
            "series": []
        })));
        assert!(has_plot_data(&serde_json::json!({
            "plot_type": "line",
            "series": [{ "points": [{ "x": 1, "y": 2 }] }]
        })));
    }

    #[test]
    fn ide_surfaces_uncertainty_variables_and_inspector() {
        let cached = cached_output_with_report_and_review(
            r#"{
              "variable_table": [],
              "uncertainty": [
                {
                  "binding": "Q_dist",
                  "kind": "Distribution",
                  "quantity_kind": "HeatRate",
                  "display_unit": "kW",
                  "distribution": "normal",
                  "method": "linear",
                  "mean": "5 kW",
                  "stddev": "0.8 kW",
                  "p95": "6.2 kW",
                  "line": 3
                }
              ]
            }"#,
            r#"{
              "uncertainty_summary": [
                { "variable": "Q_dist", "representation": "Distribution" }
              ],
              "uncertainty_propagation": [
                { "output": "Q_total", "status": "metadata_only" }
              ],
              "uncertainty_policies": [],
              "timeseries_uncertainty": [],
              "timeseries_uncertainty_calculations": []
            }"#,
        );

        let variables = runtime_variables(&cached);
        let variable = variables
            .iter()
            .find(|variable| variable.name == "Q_dist")
            .expect("uncertainty variable");
        assert_eq!(variable.source, "uncertainty");
        assert_eq!(variable.role.as_deref(), Some("uncertainty:linear"));
        assert!(variable
            .value
            .as_deref()
            .unwrap_or_default()
            .contains("p95=6.2 kW"));

        let inspectors = runtime_inspectors(Path::new("."), &cached);
        assert!(inspectors
            .uncertainty
            .get("report")
            .and_then(Value::as_array)
            .is_some_and(|items| items.len() == 1));
        assert!(inspectors
            .uncertainty
            .get("propagation")
            .and_then(Value::as_array)
            .is_some_and(|items| items.len() == 1));
    }

    #[test]
    fn ide_surfaces_quality_inspector_failures() {
        let cached = cached_output_with_result_report_and_review(
            r#"{
              "typed_payload": {
                "quality_results": [
                  {
                    "binding": "weather.constraint_1.quality_result",
                    "kind": "schema_constraint_result",
                    "category": "schema_constraint",
                    "target": "weather.constraint_1",
                    "subject": "Weather.dry_bulb",
                    "source_table": "weather",
                    "source_column": "dry_bulb",
                    "time_column": null,
                    "score": 0.5,
                    "passed_count": 1,
                    "warning_count": 0,
                    "failed_count": 1,
                    "status": "failed",
                    "reason": "schema constraint reported row-level field failures",
                    "failures": [
                      { "row": 3, "field": "dry_bulb", "value": "99", "message": "value is outside [-50, 60]" }
                    ],
                    "line": 7
                  }
                ]
              }
            }"#,
            r#"{
              "quality_report": {
                "status": "failed",
                "total_count": 1,
                "passed_count": 0,
                "warning_count": 0,
                "failed_count": 1,
                "unavailable_count": 0,
                "results": [
                  {
                    "binding": "weather.constraint_1.quality_result",
                    "kind": "schema_constraint_result",
                    "category": "schema_constraint",
                    "target": "weather.constraint_1",
                    "subject": "Weather.dry_bulb",
                    "score": 0.5,
                    "passed_count": 1,
                    "warning_count": 0,
                    "failed_count": 1,
                    "status": "failed",
                    "reason": "schema constraint reported row-level field failures",
                    "failures": [
                      { "row": 3, "field": "dry_bulb", "value": "99", "message": "value is outside [-50, 60]" }
                    ],
                    "line": 7
                  }
                ]
              }
            }"#,
            "{}",
        );

        let inspectors = runtime_inspectors(Path::new("."), &cached);
        assert_eq!(
            inspectors
                .quality
                .pointer("/summary/status")
                .and_then(Value::as_str),
            Some("failed")
        );
        assert_eq!(
            inspectors
                .quality
                .get("failureCount")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            inspectors
                .quality
                .pointer("/results/0/failures/0/field")
                .and_then(Value::as_str),
            Some("dry_bulb")
        );
    }

    #[test]
    fn ide_surfaces_table_transform_inspector() {
        let cached = cached_output_with_result_report_and_review(
            r#"{
              "typed_payload": {
                "table_transforms": [
                  {
                    "binding": "exact",
                    "operation": "filter",
                    "source_table": "events",
                    "schema_name": "EventLog",
                    "status": "filtered",
                    "reason": "filter applied predicates",
                    "input_row_count": 2,
                    "output_row_count": 1,
                    "matched_row_indices": [1],
                    "predicates": [
                      {
                        "expression": "timestamp == \"2024-01-01T00:00:00Z\"",
                        "status": "accepted",
                        "resolved_value": "2024-01-01T00:00:00Z"
                      }
                    ],
                    "row_diagnostics": [
                      { "row_index": 1, "status": "matched" },
                      { "row_index": 2, "status": "excluded" }
                    ]
                  }
                ]
              }
            }"#,
            "{}",
            r#"{
              "review_document": {
                "table_transforms": [
                  {
                    "binding": "exact",
                    "operation": "filter",
                    "source_table": "events",
                    "schema_name": "EventLog",
                    "status": "declared",
                    "line": 10,
                    "predicates": [
                      {
                        "expression": "timestamp == \"2024-01-01T00:00:00Z\"",
                        "status": "accepted"
                      }
                    ]
                  }
                ]
              }
            }"#,
        );

        let inspectors = runtime_inspectors(Path::new("."), &cached);
        let transforms = inspectors
            .table_transforms
            .as_array()
            .expect("table transforms");
        let transform = transforms
            .iter()
            .find(|transform| json_field_string(transform, "binding").as_deref() == Some("exact"))
            .expect("exact transform");

        assert_eq!(
            json_field_string(transform, "operation").as_deref(),
            Some("filter")
        );
        assert_eq!(json_field_usize(transform, "predicate_count"), Some(1));
        assert_eq!(json_field_usize(transform, "row_diagnostic_count"), Some(2));
        assert_eq!(
            json_field_string(transform, "contract_status").as_deref(),
            Some("declared")
        );
        let summary = transform
            .get("row_diagnostic_summary")
            .and_then(Value::as_array)
            .expect("row summary");
        assert!(summary.iter().any(|item| {
            json_field_string(item, "status").as_deref() == Some("matched")
                && json_field_usize(item, "count") == Some(1)
        }));
        assert!(summary.iter().any(|item| {
            json_field_string(item, "status").as_deref() == Some("excluded")
                && json_field_usize(item, "count") == Some(1)
        }));
    }

    #[test]
    fn ide_surfaces_network_cache_inspector() {
        let mut cached = cached_output_with_result_report_and_review("{}", "{}", "{}");
        cached.result_json = r#"{
          "typed_payload": {
            "network_boundaries": [
              {
                "kind": "http_get",
                "binding": "weather",
                "url": "https://example.test/weather",
                "status": "fixture",
                "status_code": 200,
                "status_class": "success",
                "response_hash": "net-hash",
                "expected_sha256": "net-hash",
                "retry": 2,
                "timeout": "30 s",
                "body_size_limit_bytes": 2000000,
                "line": 4
              }
            ]
          }
        }"#
        .to_owned();
        cached.run_log_json = r#"{
          "network_events": [
            {
              "kind": "network_request",
              "target": "https://example.test/weather",
              "status": "fixture_ready",
              "status_code": 200,
              "status_class": "success",
              "response_hash": "net-hash",
              "line": 4
            }
          ],
          "cache_events": [
            {
              "owner_kind": "network_request",
              "owner_name": "weather",
              "cache_key": "station=demo",
              "cache_path": "build/cache/weather.json",
              "status": "hit",
              "line": 4
            }
          ]
        }"#
        .to_owned();
        cached.output_manifest_json = r#"{
          "artifact_registry": {
            "network_requests": [
              {
                "kind": "network_request",
                "target": "https://example.test/weather",
                "status": "fixture_ready",
                "response_hash": "net-hash"
              }
            ],
            "caches": [
              {
                "owner_kind": "network_request",
                "owner_name": "weather",
                "cache_key": "station=demo",
                "cache_path": "build/cache/weather.json",
                "status": "hit"
              }
            ]
          }
        }"#
        .to_owned();

        let inspectors = runtime_inspectors(Path::new("."), &cached);
        let network_cache = inspectors
            .network_cache
            .as_object()
            .expect("network cache inspector");
        assert_eq!(
            network_cache
                .get("format")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            "eng-ide-network-cache-v1"
        );
        assert_eq!(
            network_cache
                .get("networkBoundaries")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(
            network_cache
                .get("networkBoundaries")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(|item| json_field_string(item, "expected_sha256"))
                .as_deref(),
            Some("net-hash")
        );
        assert_eq!(
            network_cache
                .get("networkEvents")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(
            network_cache
                .get("cacheEvents")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(
            network_cache
                .get("manifestCaches")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(|item| json_field_string(item, "status"))
                .as_deref(),
            Some("hit")
        );
    }

    #[test]
    fn ide_surfaces_timeseries_coverage_inspector() {
        let cached = cached_output_with_result_report_and_review(
            r#"{
              "typed_payload": {
                "timeseries_coverage": [
                  {
                    "binding": "coverage_weather_time",
                    "name": "weather_time",
                    "source_table": "weather",
                    "source_column": "time",
                    "unit": "h",
                    "start": 0.0,
                    "end": 23.0,
                    "source_start": "2024-01-01T00:00:00Z",
                    "source_end": "2024-01-01T23:00:00Z",
                    "expected_step": 1.0,
                    "expected_count": 24,
                    "actual_count": 23,
                    "missing_count": 1,
                    "missing_intervals": [
                      { "start": 12.0, "end": 12.0, "missing_count": 1 }
                    ],
                    "max_gap": 2.0,
                    "coverage_year": 2024,
                    "leap_year_policy": "gregorian",
                    "status": "gapped",
                    "line": 12
                  }
                ]
              }
            }"#,
            "{}",
            "{}",
        );

        let inspectors = runtime_inspectors(Path::new("."), &cached);
        let coverage = inspectors
            .time_series_coverage
            .as_array()
            .expect("timeseries coverage");
        let item = coverage.first().expect("coverage item");
        assert_eq!(
            json_field_string(item, "binding").as_deref(),
            Some("coverage_weather_time")
        );
        assert_eq!(json_field_string(item, "status").as_deref(), Some("gapped"));
        assert_eq!(json_field_usize(item, "missing_count"), Some(1));
        assert!(item
            .get("missing_intervals")
            .and_then(Value::as_array)
            .is_some_and(|intervals| intervals.len() == 1));
    }

    #[test]
    fn ide_surfaces_db_write_inspector() {
        let mut cached = cached_output_with_result_report_and_review(
            r#"{
              "typed_payload": {
                "db_manifests": [
                  {
                    "binding": "db_result",
                    "manifest_path": "outputs/db_write_manifest.json",
                    "resolved_path": "C:/workspace/outputs/db_write_manifest.json",
                    "hash": "db-hash",
                    "database": "outputs/results.sqlite",
                    "transaction_status": "committed",
                    "schema_status": "matched",
                    "tables": [
                      {
                        "name": "simulation_results",
                        "mode": "upsert",
                        "key": ["case_id"],
                        "schema": ["case_id", "annual_electricity"],
                        "row_count": 3
                      }
                    ],
                    "status": "manifest_loaded"
                  }
                ]
              }
            }"#,
            "{}",
            "{}",
        );
        cached.output_manifest_json = r#"{
          "artifact_registry": {
            "db_writes": [
              {
                "binding": "db_result",
                "manifest_path": "outputs/db_write_manifest.json",
                "hash": "db-hash",
                "database": "outputs/results.sqlite",
                "transaction_status": "committed",
                "table_count": 1,
                "status": "manifest_loaded"
              }
            ]
          }
        }"#
        .to_owned();

        let inspectors = runtime_inspectors(Path::new("."), &cached);
        let db_writes = inspectors
            .db_writes
            .as_object()
            .expect("db write inspector");
        assert_eq!(
            db_writes
                .get("format")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            "eng-ide-db-writes-v1"
        );
        let manifest = db_writes
            .get("manifests")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .expect("db manifest");
        assert_eq!(
            json_field_string(manifest, "transaction_status").as_deref(),
            Some("committed")
        );
        assert_eq!(
            manifest
                .get("tables")
                .and_then(Value::as_array)
                .and_then(|tables| tables.first())
                .and_then(|table| json_field_usize(table, "row_count")),
            Some(3)
        );
        assert_eq!(
            db_writes
                .get("registryWrites")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn ide_surfaces_model_card_inspector() {
        let cached = cached_output_with_result_report_and_review(
            r#"{
              "typed_payload": {
                "model_cards": [
                  {
                    "binding": "reg_card",
                    "source": "reg_model",
                    "model_kind": "linear",
                    "features": ["T_supply", "m_dot"],
                    "target": "Q_coil",
                    "target_quantity": "HeatRate",
                    "target_unit": "kW",
                    "test_fraction": "0.25",
                    "train_count": 12,
                    "test_count": 4,
                    "metrics": { "rmse": 1.2, "mae": 0.8, "r2": 0.94 },
                    "residual_plot": "residual_points",
                    "residual_point_count": 4,
                    "training_data_hash": "training-hash",
                    "model_artifact_hash": "model-hash",
                    "status": "model_card_ready",
                    "line": 17
                  }
                ],
                "model_specs": [
                  {
                    "binding": "reg_card",
                    "source": "reg_model",
                    "model_kind": "linear",
                    "features": [
                      { "name": "T_supply", "quantity": null, "unit": null },
                      { "name": "m_dot", "quantity": null, "unit": null }
                    ],
                    "target": { "name": "Q_coil", "quantity": "HeatRate", "unit": "kW" },
                    "test_fraction": "0.25",
                    "seed": "7",
                    "train_count": 12,
                    "test_count": 4,
                    "training_data_hash": "training-hash",
                    "model_artifact_hash": "model-hash",
                    "status": "model_card_ready",
                    "line": 17
                  }
                ],
                "prediction_manifests": [
                  {
                    "binding": "predictor",
                    "manifest_path": "outputs/prediction_manifest.json",
                    "model": "reg_card",
                    "model_file": { "path": "outputs/model.json", "sha256": "model-hash", "bytes": 12 },
                    "sample_file": { "path": "samples.csv", "sha256": "sample-hash", "bytes": 10 },
                    "output_file": { "path": "outputs/predictions.csv", "sha256": "prediction-hash", "bytes": 20 },
                    "schema": ["case_id", "prediction", "prediction_confidence"],
                    "outputs": [
                      { "column": "prediction", "quantity": "HeatRate", "unit": "kW" },
                      { "column": "prediction_confidence", "quantity": "Ratio", "unit": "1" }
                    ],
                    "case_ids": ["case_001"],
                    "row_count": 1,
                    "confidence_column": "prediction_confidence",
                    "status": "complete",
                    "line": 18
                  }
                ],
                "model_diagnostics": [
                  {
                    "severity": "warning",
                    "code": "W-MODEL-EXTRAPOLATION",
                    "message": "prediction schema mismatch diagnostic",
                    "binding": "predictor",
                    "line": 18
                  }
                ],
                "ml": [
                  {
                    "binding": "reg_model",
                    "kind": "RegressionModel",
                    "source": "split",
                    "target": "Q_coil",
                    "target_quantity": "HeatRate",
                    "target_unit": "kW",
                    "features": ["T_supply", "m_dot"],
                    "algorithm": "linear",
                    "test_fraction": "0.25",
                    "seed": "7",
                    "hidden_layers": [],
                    "epochs": null,
                    "status": "trained",
                    "train_count": 12,
                    "test_count": 4,
                    "rmse": 1.2,
                    "mae": 0.8,
                    "r2": 0.94,
                    "leakage_status": "passed",
                    "leakage_findings": [],
                    "coefficients": [{ "feature": "T_supply", "value": 0.5 }],
                    "intercept": 1.0,
                    "loss_history": [],
                    "training_data_hash": "training-hash",
                    "model_artifact_hash": "model-hash",
                    "model_card": "Linear model card",
                    "parity_points": [{ "x": 1.0, "y": 1.1 }],
                    "residual_points": [{ "x": 1.0, "y": 0.1 }],
                    "expression": "regression(split, algorithm=linear)",
                    "line": 16
                  }
                ]
              }
            }"#,
            "{}",
            "{}",
        );

        let inspectors = runtime_inspectors(Path::new("."), &cached);
        let model_cards = inspectors
            .model_cards
            .as_object()
            .expect("model card inspector");
        assert_eq!(
            model_cards
                .get("format")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            "eng-ide-model-cards-v1"
        );
        let card = model_cards
            .get("cards")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .expect("model card");
        assert_eq!(
            card.get("metrics")
                .and_then(|metrics| json_field_string(metrics, "r2"))
                .as_deref(),
            Some("0.94")
        );
        assert_eq!(json_field_usize(card, "residual_point_count"), Some(4));
        let spec = model_cards
            .get("specs")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .expect("model spec");
        assert_eq!(
            json_field_string(spec, "model_kind").as_deref(),
            Some("linear")
        );
        let prediction = model_cards
            .get("predictionManifests")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .expect("prediction manifest");
        assert_eq!(
            json_field_string(prediction, "confidence_column").as_deref(),
            Some("prediction_confidence")
        );
        let diagnostic = model_cards
            .get("diagnostics")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .expect("model diagnostic");
        assert_eq!(
            json_field_string(diagnostic, "code").as_deref(),
            Some("W-MODEL-EXTRAPOLATION")
        );
        let artifact = model_cards
            .get("artifacts")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .expect("model artifact");
        assert!(artifact
            .get("residual_points")
            .and_then(Value::as_array)
            .is_some_and(|points| points.len() == 1));
    }

    #[test]
    fn ide_surfaces_case_manifest_inspector() {
        let cached = cached_output_with_result_report_and_review(
            r#"{
              "typed_payload": {
                "case_manifests": [
                  {
                    "case_id": "case_001",
                    "sample_table": "designs",
                    "schema_name": "DesignSample",
                    "source": "samples/design_samples.csv",
                    "source_hash": "sample-hash",
                    "sample_row_number": 1,
                    "source_row": 1,
                    "line": 14,
                    "sample_row_hash": "row-hash-1",
                    "case_dir": "outputs/case_001",
                    "generated_input_file": "outputs/case_001/input.txt",
                    "process_bindings": ["materialize_case_001"],
                    "process_statuses": [
                      { "name": "materialize_case_001", "command": "native render template model/native_case_template.txt", "status": "succeeded" }
                    ],
                    "output_artifacts": ["outputs/case_001/result.csv"],
                    "result_files": ["outputs/case_001/result.csv"],
                    "metrics": [
                      { "name": "annual_electricity", "value": 123.4, "unit": "kWh" }
                    ],
                    "failure_reason": null,
                    "status": "succeeded"
                  },
                  {
                    "case_id": "case_002",
                    "sample_table": "designs",
                    "schema_name": "DesignSample",
                    "source": "samples/design_samples.csv",
                    "source_hash": "sample-hash",
                    "sample_row_number": 2,
                    "source_row": 2,
                    "line": 14,
                    "sample_row_hash": "row-hash-2",
                    "case_dir": "outputs/case_002",
                    "generated_input_file": "outputs/case_002/input.txt",
                    "process_bindings": ["materialize_case_002"],
                    "process_statuses": [
                      { "name": "materialize_case_002", "command": "native render template model/native_case_template.txt", "status": "failed" }
                    ],
                    "output_artifacts": [],
                    "result_files": [],
                    "metrics": [],
                    "failure_reason": "case materialize_case_002 failed",
                    "status": "failed"
                  }
                ],
                "case_tables": [
                  {
                    "sample_table": "designs",
                    "schema_name": "DesignSample",
                    "source": "samples/design_samples.csv",
                    "source_hash": "sample-hash",
                    "case_count": 2,
                    "pending_count": 0,
                    "running_count": 0,
                    "succeeded_count": 1,
                    "failed_count": 1,
                    "skipped_count": 0,
                    "duplicate_case_ids": [],
                    "case_dir_count": 2,
                    "generated_input_count": 2,
                    "output_artifact_count": 1,
                    "result_file_count": 1,
                    "metric_count": 1,
                    "runner": "sequential_process_runner",
                    "scheduler": "sequential",
                    "resume_policy": "case_cache_key",
                    "cache_hit_count": 0,
                    "cache_miss_count": 2,
                    "line": 14,
                    "status": "failed"
                  }
                ],
                "case_diagnostics": [
                  {
                    "severity": "error",
                    "code": "E-CASE-STEP-FAILED",
                    "message": "case `case_002` step `materialize_case_002` reported status `failed`",
                    "case_id": "case_002",
                    "sample_table": "designs",
                    "line": 14
                  }
                ]
              }
            }"#,
            "{}",
            "{}",
        );

        let inspectors = runtime_inspectors(Path::new("."), &cached);
        let cases = inspectors
            .case_manifests
            .as_object()
            .expect("case manifest inspector");
        assert_eq!(
            cases
                .get("format")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            "eng-ide-case-manifests-v1"
        );
        assert_eq!(
            cases
                .get("manifests")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(
            cases
                .get("caseTables")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(
            cases
                .get("diagnostics")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(1)
        );
        let failed = cases
            .get("failedCases")
            .and_then(Value::as_array)
            .expect("failed cases");
        assert_eq!(failed.len(), 1);
        assert_eq!(
            failed
                .first()
                .and_then(|item| json_field_string(item, "case_id"))
                .as_deref(),
            Some("case_002")
        );
        assert_eq!(
            failed
                .first()
                .and_then(|item| json_field_usize(item, "line")),
            Some(14)
        );
    }

    #[test]
    fn ide_surfaces_structured_read_inspector() {
        let cached = cached_output_with_result_report_and_review(
            r#"{
              "typed_payload": {
                "structured_reads": [
                  {
                    "binding": "json_text",
                    "kind": "json",
                    "path": "data/case.json",
                    "source_hash": "abc123",
                    "parse_status": "parsed",
                    "root_type": "object",
                    "field_count": 2,
                    "item_count": null,
                    "error": null,
                    "line": 4
                  }
                ],
                "config_promotions": [
                  {
                    "binding": "config",
                    "format": "json",
                    "schema_name": "WorkflowConfig",
                    "source": "payload",
                    "resolved_path": "data/case.json",
                    "source_hash": "abc123",
                    "field_count": 2,
                    "missing_fields": [],
                    "unknown_fields": [],
                    "null_fields": [],
                    "optional_fields": [],
                    "optional_missing_fields": [],
                    "optional_null_fields": [],
                    "type_mismatches": [],
                    "status": "validated",
                    "line": 5
                  }
                ]
              }
            }"#,
            "{}",
            "{}",
        );

        let inspectors = runtime_inspectors(Path::new("."), &cached);
        let reads = inspectors
            .structured_reads
            .as_array()
            .expect("structured reads");
        let read = reads.first().expect("structured read");

        assert_eq!(
            json_field_string(read, "binding").as_deref(),
            Some("json_text")
        );
        assert_eq!(
            json_field_string(read, "parse_status").as_deref(),
            Some("parsed")
        );
        assert_eq!(
            json_field_string(read, "root_type").as_deref(),
            Some("object")
        );
        let configs = inspectors
            .config_promotions
            .as_array()
            .expect("config promotions");
        let config = configs.first().expect("config promotion");
        assert_eq!(
            json_field_string(config, "binding").as_deref(),
            Some("config")
        );
        assert_eq!(
            json_field_string(config, "status").as_deref(),
            Some("validated")
        );
    }

    #[test]
    fn ide_surfaces_normalized_review_cockpit_sections() {
        let cached = cached_output_with_report_and_review(
            "{}",
            r#"{
              "review_document": {
                "semantic_hash": "abc123",
                "section_hashes": {
                  "units_quantities": "u",
                  "schemas": "s",
                  "time_axes": "t",
                  "calculations": "c",
                  "report_outputs": "o",
                  "external_boundaries": "b",
                  "side_effects": "e",
                  "risks": "r"
                },
                "symbols": [{ "name": "Q", "quantity_kind": "HeatRate", "display_unit": "kW", "line": 1 }],
                "units_quantities": [{ "name": "Q", "canonical_unit": "W", "display_unit": "kW", "line": 1 }],
                "schemas": [{ "name": "SensorData", "line": 2 }],
                "time_axes": [{ "axis": "Time", "binding": "sensor", "line": 3 }],
                "calculations": [{ "name": "Q", "expression": "m_dot * cp * dT", "line": 4 }],
                "report_outputs": [{ "kind": "summary", "source": "Q", "line": 5 }],
                "external_boundaries": [{ "kind": "http_fixture", "name": "weather_api", "target": "https://api.example.org/weather/hourly", "line": 6 }],
                "side_effects": [{ "kind": "write_output", "target": "\"out.csv\"", "line": 7 }],
                "risks": [{ "category": "side_effect", "level": "high", "line": 8 }]
              }
            }"#,
        );

        let inspectors = runtime_inspectors(Path::new("."), &cached);

        assert!(review_document_has_core_cockpit_sections(
            &inspectors.review_document
        ));
        assert!(review_document_has_external_boundary(
            &inspectors.review_document
        ));
        assert!(review_document_has_side_effect(&inspectors.review_document));
    }

    #[test]
    fn ide_reads_run_plan_inspector_payload() {
        let mut cached = cached_output_with_report_and_review("{}", "{}");
        cached.run_plan_json = r#"{
          "format": "eng-run-plan-v1",
          "graph": {
            "node_count": 1,
            "edge_count": 0,
            "nodes": [
              { "id": "source:program", "status": "loaded" }
            ],
            "edges": []
          }
        }"#
        .to_owned();

        let inspectors = runtime_inspectors(Path::new("."), &cached);

        assert_eq!(
            inspectors.run_plan.get("format").and_then(Value::as_str),
            Some("eng-run-plan-v1")
        );
        assert_eq!(
            inspectors
                .run_plan
                .pointer("/graph/nodes/0/id")
                .and_then(Value::as_str),
            Some("source:program")
        );
    }

    #[test]
    fn ide_surfaces_output_manifest_artifact_paths() {
        let mut cached = cached_output_with_report_and_review("{}", "{}");
        cached.artifacts_saved = true;
        cached.result_path = PathBuf::from("build/result/result.engres");
        cached.review_path = PathBuf::from("build/result/review.json");
        cached.output_manifest_path = PathBuf::from("build/result/output_manifest.json");
        cached.run_plan_path = PathBuf::from("build/result/run_plan.json");
        cached.run_lock_path = PathBuf::from("build/result/run_lock.json");
        cached.run_log_path = PathBuf::from("build/result/run_log.json");

        let inspectors = runtime_inspectors(Path::new("."), &cached);
        let artifacts = inspectors
            .output_manifest
            .get("artifacts")
            .and_then(Value::as_array)
            .expect("output manifest artifacts");

        assert!(artifacts.iter().any(|artifact| {
            artifact.get("kind").and_then(Value::as_str) == Some("output_manifest")
                && artifact
                    .get("path")
                    .and_then(Value::as_str)
                    .is_some_and(|path| path.ends_with("output_manifest.json"))
        }));
    }

    #[test]
    fn ide_surfaces_memory_runtime_artifacts_as_effect_records() {
        let mut cached = cached_output_with_report_and_review("{}", "{}");
        cached.output_manifest_json = r#"{
          "format": "eng-output-manifest-v1",
          "artifact_count": 0,
          "artifacts": [],
          "artifact_registry": {
            "format": "eng-artifact-registry-v1",
            "generated_files": []
          }
        }"#
        .to_owned();
        cached.run_log_json = r#"{
          "format": "eng-run-log-v1",
          "external_boundary_events": [
            {
              "kind": "process",
              "target": "cmd",
              "status": "completed",
              "success": true,
              "line": 1
            }
          ]
        }"#
        .to_owned();
        cached.process_results_path = PathBuf::from("build/result/process_results.json");

        let inspectors = runtime_inspectors(Path::new("."), &cached);

        assert!(effect_records_has_artifact_and_boundary_records(
            &inspectors.effect_records
        ));
    }

    fn cached_output_with_report_and_review(
        report_spec_json: &str,
        review_json: &str,
    ) -> CachedRunOutput {
        cached_output_with_result_report_and_review("{}", report_spec_json, review_json)
    }

    fn cached_output_with_result_report_and_review(
        result_json: &str,
        report_spec_json: &str,
        review_json: &str,
    ) -> CachedRunOutput {
        CachedRunOutput {
            bytecode_path: PathBuf::new(),
            result_path: PathBuf::new(),
            review_path: PathBuf::new(),
            static_run_plan_path: PathBuf::new(),
            run_plan_path: PathBuf::new(),
            run_lock_path: PathBuf::new(),
            run_log_path: PathBuf::new(),
            process_results_path: PathBuf::new(),
            test_results_path: PathBuf::new(),
            report_path: PathBuf::new(),
            report_spec_path: PathBuf::new(),
            plot_path: PathBuf::new(),
            plot_spec_path: PathBuf::new(),
            plot_manifest_path: PathBuf::new(),
            output_manifest_path: PathBuf::new(),
            artifacts_saved: false,
            bytecode: String::new(),
            result_json: result_json.to_owned(),
            review_json: review_json.to_owned(),
            static_run_plan_json: "{}".to_owned(),
            run_plan_json: "{}".to_owned(),
            run_lock_json: "{}".to_owned(),
            run_log_json: "{}".to_owned(),
            process_results_json: "{}".to_owned(),
            test_results_json: "{}".to_owned(),
            report_html: String::new(),
            report_spec_json: report_spec_json.to_owned(),
            plot_svg: String::new(),
            plot_spec_json: "{}".to_owned(),
            plot_manifest_json: "{}".to_owned(),
            output_manifest_json: "{}".to_owned(),
        }
    }

    fn unique_temp_root() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "eng_ide_workspace_root_test_{}_{}",
            std::process::id(),
            nanos
        ))
    }
}
