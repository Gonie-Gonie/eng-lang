#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use eng_compiler::{
    all_quantity_completions, all_unit_infos, check_source, CheckOptions, CheckReport, Severity,
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
    detail: String,
    kind: String,
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
    metrics: Value,
    validations: Value,
    time_alignments: Value,
    systems: Value,
    system_ir: Value,
    linear_operators: Value,
    kernel_plan: Value,
    class_objects: Value,
    assemblies: Value,
    component_graph: Value,
    artifact_outlines: Value,
    output_manifest: Value,
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
            metrics: Value::Array(Vec::new()),
            validations: Value::Array(Vec::new()),
            time_alignments: Value::Array(Vec::new()),
            systems: Value::Array(Vec::new()),
            system_ir: Value::Array(Vec::new()),
            linear_operators: Value::Array(Vec::new()),
            kernel_plan: Value::Null,
            class_objects: Value::Array(Vec::new()),
            assemblies: Value::Array(Vec::new()),
            component_graph: Value::Null,
            artifact_outlines: Value::Array(Vec::new()),
            output_manifest: Value::Null,
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
            ide_run,
            ide_terminal,
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
            let check = check_view_from_report(&report);
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
            let check = check_view_from_report(&report);
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
    check_view_from_report(&report)
}

fn check_view_from_report(report: &CheckReport) -> CheckView {
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
    CheckView {
        diagnostics,
        symbols,
        status: format!("{errors} error(s), {warnings} warning(s)"),
    }
}

fn base_completion_items() -> Vec<CompletionView> {
    let mut items = Vec::new();
    for keyword in [
        "args", "class", "const", "export", "fn", "if", "import", "log", "method", "plot", "print",
        "promote", "read", "report", "return", "schema", "system", "test", "validate", "where",
        "with", "write",
    ] {
        items.push(CompletionView {
            label: keyword.to_owned(),
            insert: keyword.to_owned(),
            detail: "keyword".to_owned(),
            kind: "keyword".to_owned(),
        });
    }
    for snippet in [
        (
            "promote csv",
            "promote csv \"data/sensor.csv\" as SensorData",
            "CSV promotion command",
        ),
        (
            "export summary csv",
            "export summary to csv \"summary.csv\" {\n    E as kWh with \".2\"\n}",
            "unit-aware CSV export block",
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
        items.push(CompletionView {
            label: snippet.0.to_owned(),
            insert: snippet.1.to_owned(),
            detail: snippet.2.to_owned(),
            kind: "snippet".to_owned(),
        });
    }
    for quantity in all_quantity_completions() {
        items.push(CompletionView {
            label: quantity.quantity_kind.to_owned(),
            insert: quantity.quantity_kind.to_owned(),
            detail: format!("{} [{}]", quantity.description, quantity.canonical_unit),
            kind: "quantity".to_owned(),
        });
    }
    for unit in all_unit_infos() {
        items.push(CompletionView {
            label: unit.symbol.to_owned(),
            insert: unit.symbol.to_owned(),
            detail: format!("{} -> {}", unit.quantity_hint, unit.canonical_unit),
            kind: "unit".to_owned(),
        });
    }
    items
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
    InspectorView {
        schemas: schema_inspector(&report, &result),
        unit_conversions: json_array_clone(&report, "unit_conversion_table"),
        time_axes: json_array_clone(&report, "time_axes"),
        time_series: time_series_inspector(&report, &result),
        metrics: json_array_clone(&report, "computed_metrics"),
        validations: json_array_clone(&report, "validations"),
        time_alignments: json_array_clone(&report, "time_alignments"),
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
        artifact_outlines: artifact_outlines(root, output),
        output_manifest: output_manifest_inspector(root, output),
        run_log: parse_json_value(&output.run_log_json),
        process_results: parse_json_value(&output.process_results_json),
        test_results: parse_json_value(&output.test_results_json),
    }
}

fn parse_json_value(text: &str) -> Value {
    serde_json::from_str::<Value>(text).unwrap_or(Value::Null)
}

fn json_array_clone(value: &Value, key: &str) -> Value {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| Value::Array(items.clone()))
        .unwrap_or_else(|| Value::Array(Vec::new()))
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
            json!({
                "kind": artifact.kind,
                "path": artifact.path,
                "hash": "",
                "status": artifact.status
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
            help: Some("Use a binding like `x = 3`, an expression print like `print x`, `run`, `check`, `reset`, or `clear`.".to_owned()),
        }],
        symbols: Vec::new(),
        status: "1 error(s), 0 warning(s)".to_owned(),
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

fn smoke() -> Result<(), String> {
    let root = workspace_root();
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
                == Some("predictor_call_contract_seed_not_integrated")
            && json_field_string(node, "contract_status").as_deref()
                == Some("predictor_contract_metadata_seed")
            && has_contract_quantity(node, "contract_inputs", "AbsoluteTemperature")
            && has_diagnostic_channel(node, "predictor_valid_range_warning")
            && node.get("source_span").is_some()
    });
    let has_external_node = behavior_nodes.iter().any(|node| {
        json_field_string(node, "behavior_kind").as_deref() == Some("external")
            && json_field_string(node, "signal").as_deref() == Some("out.Q")
            && json_field_string(node, "status").as_deref()
                == Some("external_behavior_wrapper_seed_not_integrated")
            && json_field_string(node, "contract_status").as_deref()
                == Some("external_behavior_contract_metadata_seed")
            && has_contract_quantity(node, "contract_inputs", "HeatRate")
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
    let official_multi_domain_example =
        root.join("examples/official/32_small_thermal_fluid_loop/main.eng");
    let official_multi_domain_output = run_file(
        &official_multi_domain_example,
        &root.join("build").join("ide-smoke-official-thermal-fluid"),
        &RunOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let official_multi_domain_cached = CachedRunOutput::from_output(official_multi_domain_output);
    let official_multi_domain_inspectors = runtime_inspectors(&root, &official_multi_domain_cached);
    let has_official_multi_domain_component_graph = official_multi_domain_inspectors
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
    let has_official_multi_domain_solver_result = official_multi_domain_inspectors
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
                                == Some("connection_set_2.across_height_1")
                                && json_field_string(equation, "kind").as_deref()
                                    == Some("across_equality")
                                && json_field_string(equation, "domain").as_deref()
                                    == Some("Fluid[Water]")
                                && json_field_usize(equation, "line") == Some(48)
                                && equation
                                    .get("dependencies")
                                    .and_then(Value::as_array)
                                    .is_some_and(|dependencies| {
                                        dependencies.iter().any(|value| {
                                            value.as_str() == Some("pump.supply.height")
                                        }) && dependencies.iter().any(|value| {
                                            value.as_str() == Some("pipe.inlet.height")
                                        })
                                    })
                        }) && equations.iter().any(|equation| {
                            json_field_string(equation, "name").as_deref()
                                == Some("pipe.equation_1")
                                && json_field_string(equation, "kind").as_deref()
                                    == Some("component_equation")
                                && json_field_string(equation, "domain").as_deref()
                                    == Some("Fluid[Water]")
                                && json_field_usize(equation, "line") == Some(32)
                                && equation
                                    .get("dependencies")
                                    .and_then(Value::as_array)
                                    .is_some_and(|dependencies| {
                                        dependencies.iter().any(|value| {
                                            value.as_str() == Some("pipe.inlet.height")
                                        }) && dependencies.iter().any(|value| {
                                            value.as_str() == Some("pipe.outlet.height")
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
                                    == Some("pipe.outlet.height")
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
                                && json_field_string(residual, "unit").as_deref() == Some("m")
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
                            json_field_string(variable, "name").as_deref()
                                == Some("pump.supply.height")
                                && json_field_f64(variable, "value")
                                    .is_some_and(|value| (value - 12.0).abs() <= 1e-9)
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
    if !has_official_multi_domain_component_graph || !has_official_multi_domain_solver_result {
        return Err(format!(
            "{} did not produce IDE official Thermal/Fluid solver, equation, residual, and graph inspector metadata",
            official_multi_domain_example.display()
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
    if !has_state_space_series || !has_state_space_solver || !has_state_space_operator_matrix {
        return Err(format!(
            "{} did not produce IDE state-space trajectory/operator inspector metadata",
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
    let has_manifest = effects_inspectors
        .output_manifest
        .get("artifacts")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    let has_run_log = effects_inspectors
        .run_log
        .get("messages")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    let has_process = effects_inspectors
        .process_results
        .get("processes")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    if !has_manifest || !has_run_log || !has_process {
        return Err(format!(
            "{} did not produce IDE side-effect inspector metadata (manifest={}, run_log={}, process={})",
            effects_example.display(),
            has_manifest,
            has_run_log,
            has_process
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
    println!(
        "EngLang IDE smoke OK: {} example(s), {} quantity completion(s), {} unit completion(s), {} domain(s), {} component(s), {} connection(s), {} assembly graph(s), residual dependency inspector, behavior graph inspector, measured workflow inspectors, solved thermal assembly inspector, multi-domain boundary solve inspector, official Thermal/Fluid solver inspector, state-space trajectory/operator inspector, kernel plan inspector, class object inspector, side-effect inspectors, schema failure inspector",
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

#[cfg(test)]
mod tests {
    use super::*;
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
