#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use eng_compiler::{
    all_quantity_completions, all_unit_infos, check_source, CheckOptions, CheckReport, Severity,
};
use eng_runtime::{run_file, RunOptions, RuntimeError};
use serde::Serialize;
use serde_json::Value;
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
    terminal: String,
    check: CheckView,
    variables: Vec<RuntimeVariableView>,
    args: Vec<RuntimeArgView>,
    plot_spec: Value,
    report_title: String,
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
    relative_report_path: String,
    relative_plot_path: String,
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
    fn from_output(output: eng_runtime::RunOutput, root: &Path) -> Self {
        Self {
            relative_report_path: relative_to(root, &output.report_path),
            relative_plot_path: relative_to(root, &output.plot_path),
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
fn ide_run(path: String, source: String, state: State<'_, IdeState>) -> Result<RunView, String> {
    let root = workspace_root();
    let path = resolve_path(&root, &path);
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
            terminal: "Run blocked by diagnostics. See Problems.".to_owned(),
            check,
            variables: Vec::new(),
            args: Vec::new(),
            plot_spec: Value::Null,
            report_title: String::new(),
        });
    }
    run_source_file(&root, &path, check, state)
}

#[tauri::command]
fn ide_terminal(
    path: String,
    source: String,
    command: String,
    state: State<'_, IdeState>,
) -> Result<RunView, String> {
    let trimmed = command.trim();
    let root = workspace_root();
    let current_path = resolve_path(&root, &path);
    if trimmed.eq_ignore_ascii_case("clear") || trimmed.eq_ignore_ascii_case("cls") {
        return Ok(RunView::message("Terminal cleared."));
    }
    if trimmed.eq_ignore_ascii_case("reset") {
        *state
            .terminal_session_source
            .lock()
            .map_err(|error| error.to_string())? = String::new();
        return Ok(RunView::message("Terminal session reset."));
    }
    if trimmed.eq_ignore_ascii_case("check") {
        let check = check_view(&current_path, &source);
        return Ok(RunView {
            ok: true,
            terminal: diagnostic_summary_text(&check),
            check,
            variables: Vec::new(),
            args: Vec::new(),
            plot_spec: Value::Null,
            report_title: String::new(),
        });
    }
    if trimmed.eq_ignore_ascii_case("run") {
        return ide_run(path, source, state);
    }

    let session_source = {
        let mut session = state
            .terminal_session_source
            .lock()
            .map_err(|error| error.to_string())?;
        if !session.trim().is_empty() {
            session.push('\n');
        }
        session.push_str(trimmed);
        session.push('\n');
        session.clone()
    };
    let session_path = root
        .join("build")
        .join("ide-tauri-session")
        .join("session.eng");
    let check = check_view(&session_path, &session_source);
    if check
        .diagnostics
        .iter()
        .any(|item| item.severity == "error")
    {
        return Ok(RunView {
            ok: false,
            terminal: diagnostic_summary_text(&check),
            check,
            variables: Vec::new(),
            args: Vec::new(),
            plot_spec: Value::Null,
            report_title: String::new(),
        });
    }
    create_parent(&session_path)?;
    fs::write(&session_path, session_source.as_bytes()).map_err(|error| error.to_string())?;
    run_source_file(&root, &session_path, check, state)
}

#[tauri::command]
fn ide_open_artifact(kind: String, state: State<'_, IdeState>) -> Result<String, String> {
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
    let (path, relative) = match kind.as_str() {
        "plot" => (&output.plot_path, &output.relative_plot_path),
        _ => (&output.report_path, &output.relative_report_path),
    };
    open_path(path);
    Ok(relative.clone())
}

impl RunView {
    fn message(text: impl Into<String>) -> Self {
        Self {
            ok: true,
            terminal: text.into(),
            check: CheckView {
                diagnostics: Vec::new(),
                symbols: Vec::new(),
                status: "ok".to_owned(),
            },
            variables: Vec::new(),
            args: Vec::new(),
            plot_spec: Value::Null,
            report_title: String::new(),
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
            ..RunOptions::default()
        },
    ) {
        Ok(output) => {
            let stdout = output.stdout.clone();
            let cached = CachedRunOutput::from_output(output, root);
            let variables = runtime_variables(&cached);
            let args = runtime_args(&cached.report_spec_json);
            let report_title = report_title(&cached.report_spec_json);
            let plot_spec = serde_json::from_str(&cached.plot_spec_json).unwrap_or(Value::Null);
            let terminal = terminal_summary(&stdout, &variables, &args, &report_title, &plot_spec);
            *state
                .last_output
                .lock()
                .map_err(|error| error.to_string())? = Some(cached);
            Ok(RunView {
                ok: true,
                terminal,
                check,
                variables,
                args,
                plot_spec,
                report_title,
            })
        }
        Err(RuntimeError::Compile(report)) => {
            let check = check_view_from_report(&report);
            Ok(RunView {
                ok: false,
                terminal: diagnostic_summary_text(&check),
                check,
                variables: Vec::new(),
                args: Vec::new(),
                plot_spec: Value::Null,
                report_title: String::new(),
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
        "args", "const", "export", "fn", "if", "import", "log", "plot", "print", "promote", "read",
        "report", "return", "schema", "system", "test", "where", "with", "write",
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

fn terminal_summary(
    stdout: &str,
    variables: &[RuntimeVariableView],
    args: &[RuntimeArgView],
    report_title: &str,
    plot_spec: &Value,
) -> String {
    let mut lines = Vec::new();
    if !stdout.trim().is_empty() {
        lines.push(stdout.trim_end().to_owned());
    }
    lines.push("Run OK".to_owned());
    lines.push(format!(
        "variables: {}, args: {}",
        variables.len(),
        args.len()
    ));
    if !report_title.is_empty() {
        lines.push(format!("report: {report_title}"));
    }
    if !plot_spec.is_null() {
        lines.push("plot: available".to_owned());
    }
    lines.join("\n")
}

fn diagnostic_summary_text(check: &CheckView) -> String {
    let mut lines = vec![format!("diagnostics: {}", check.status)];
    for diagnostic in check.diagnostics.iter().take(6) {
        lines.push(format!(
            "L{} {}: {}",
            diagnostic.line, diagnostic.code, diagnostic.message
        ));
        if let Some(help) = &diagnostic.help {
            lines.push(format!("  help: {help}"));
        }
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
    let preferred = root.join("examples/official/03_integrated_hvac/main.eng");
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

fn format_json_number(value: f64) -> String {
    if value.abs() >= 1000.0 {
        format!("{value:.3}")
    } else if value.abs() >= 10.0 {
        format!("{value:.4}")
    } else {
        format!("{value:.6}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
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
    let domain_example = root.join("examples/official/06_domain_port/main.eng");
    let domain_source = read_utf8(&domain_example)?;
    let domain_report = check_source(&domain_example, &domain_source, &CheckOptions::default());
    if domain_report.has_errors()
        || domain_report.semantic_program.domains.is_empty()
        || domain_report.semantic_program.components.is_empty()
        || domain_report.semantic_program.connections.is_empty()
    {
        return Err(format!(
            "{} did not produce domain/component metadata",
            domain_example.display()
        ));
    }
    let ui_index = root.join("crates/eng_ide/ui/index.html");
    if root.join("crates/eng_ide").exists() && !ui_index.exists() {
        return Err(format!("missing Tauri UI asset {}", ui_index.display()));
    }
    println!(
        "EngLang IDE smoke OK: {} example(s), {} quantity completion(s), {} unit completion(s), {} domain(s), {} component(s), {} connection(s)",
        examples.len(),
        all_quantity_completions().len(),
        all_unit_infos().len(),
        domain_report.semantic_program.domains.len(),
        domain_report.semantic_program.components.len(),
        domain_report.semantic_program.connections.len()
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
