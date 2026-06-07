use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use eframe::egui;
use eng_compiler::{
    all_quantity_completions, all_unit_infos, check_source, CheckOptions, Severity,
};
use eng_runtime::{run_file, RunOptions, RuntimeError};

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--smoke") {
        smoke()?;
        return Ok(());
    }
    if args
        .iter()
        .any(|arg| arg == "--version" || arg == "version")
    {
        println!("EngLang IDE {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1180.0, 760.0]),
        ..Default::default()
    };
    eframe::run_native(
        "EngLang IDE",
        options,
        Box::new(|_cc| Box::new(EngIdeApp::new())),
    )
}

fn smoke() -> eframe::Result<()> {
    let root = workspace_root();
    let examples = collect_examples(&root);
    if examples.is_empty() {
        eprintln!("EngLang IDE smoke failed: no examples found");
        std::process::exit(1);
    }
    let first = examples
        .iter()
        .find(|path| path.ends_with("examples/official/01_csv_plot/main.eng"))
        .or_else(|| examples.first())
        .expect("examples is not empty");
    let source = match fs::read_to_string(first) {
        Ok(source) => source,
        Err(error) => {
            eprintln!(
                "EngLang IDE smoke failed: could not read {}: {error}",
                first.display()
            );
            std::process::exit(1);
        }
    };
    let report = check_source(first, &source, &CheckOptions::default());
    if report.has_errors() {
        eprintln!("EngLang IDE smoke failed: {} has errors", first.display());
        std::process::exit(2);
    }
    println!(
        "EngLang IDE smoke OK: {} example(s), {} quantity completion(s), {} unit completion(s)",
        examples.len(),
        all_quantity_completions().len(),
        all_unit_infos().len()
    );
    Ok(())
}

struct EngIdeApp {
    root: PathBuf,
    examples: Vec<PathBuf>,
    current_path: PathBuf,
    path_input: String,
    source: String,
    diagnostics: Vec<DiagnosticView>,
    symbols: Vec<SymbolView>,
    status: String,
    run_log: String,
    entry: String,
    dirty: bool,
    last_edit: Option<Instant>,
    cursor_char_index: usize,
    completion_filter: String,
    show_completions: bool,
    last_report_path: Option<PathBuf>,
}

impl EngIdeApp {
    fn new() -> Self {
        let root = workspace_root();
        let examples = collect_examples(&root);
        let current_path = examples
            .iter()
            .find(|path| path.ends_with("examples/official/01_csv_plot/main.eng"))
            .or_else(|| examples.first())
            .cloned()
            .unwrap_or_else(|| root.join("main.eng"));
        let mut app = Self {
            root,
            examples,
            current_path,
            path_input: String::new(),
            source: String::new(),
            diagnostics: Vec::new(),
            symbols: Vec::new(),
            status: "Ready.".to_owned(),
            run_log: String::new(),
            entry: "main".to_owned(),
            dirty: false,
            last_edit: None,
            cursor_char_index: 0,
            completion_filter: String::new(),
            show_completions: true,
            last_report_path: None,
        };
        app.path_input = app.relative_path(&app.current_path);
        app.load_current();
        app.check_current();
        app
    }

    fn load_current(&mut self) {
        match fs::read_to_string(&self.current_path) {
            Ok(source) => {
                self.source = source;
                self.path_input = self.relative_path(&self.current_path);
                self.dirty = false;
                self.last_edit = None;
                self.status = format!("Loaded {}", self.path_input);
            }
            Err(error) => {
                self.source.clear();
                self.diagnostics.clear();
                self.status = format!("Could not load {}: {error}", self.current_path.display());
            }
        }
    }

    fn open_path_input(&mut self) {
        let path = self.resolve_path_input();
        self.current_path = path;
        self.load_current();
        self.check_current();
    }

    fn save_current(&mut self) {
        if let Some(parent) = self.current_path.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                self.status = format!("Could not create {}: {error}", parent.display());
                return;
            }
        }
        match fs::write(&self.current_path, &self.source) {
            Ok(()) => {
                self.dirty = false;
                self.status = format!("Saved {}", self.relative_path(&self.current_path));
            }
            Err(error) => {
                self.status = format!("Save failed: {error}");
            }
        }
    }

    fn check_current(&mut self) {
        let report = check_source(&self.current_path, &self.source, &CheckOptions::default());
        self.diagnostics = report
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
        self.symbols = report
            .semantic_program
            .hover_hints
            .iter()
            .map(|hover| SymbolView {
                name: hover.name.clone(),
                line: hover.line,
                quantity_kind: hover.quantity_kind.clone(),
                display_unit: hover.display_unit.clone(),
                detail: hover.detail.clone(),
            })
            .collect();

        let errors = report.diagnostic_count(Severity::Error);
        let warnings = report.diagnostic_count(Severity::Warning);
        self.status = format!("Check complete: {errors} error(s), {warnings} warning(s)");
    }

    fn run_current(&mut self) {
        if self.dirty {
            self.save_current();
        }
        let build_root = self.root.join("build").join("ide-run");
        match run_file(
            &self.current_path,
            &build_root,
            &RunOptions {
                open_report: false,
                entry: Some(self.entry.clone()),
                args: Vec::new(),
            },
        ) {
            Ok(output) => {
                self.last_report_path = Some(output.report_path.clone());
                self.run_log = format!(
                    "Run OK\nbytecode: {}\nresult: {}\nreview: {}\nreport_spec: {}\nreport: {}",
                    output.bytecode_path.display(),
                    output.result_path.display(),
                    output.review_path.display(),
                    output.report_spec_path.display(),
                    output.report_path.display()
                );
                self.status = "Run complete.".to_owned();
            }
            Err(RuntimeError::Compile(report)) => {
                self.diagnostics = report
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
                self.run_log = "Run failed during compile. See diagnostics.".to_owned();
                self.status = "Run failed.".to_owned();
            }
            Err(error) => {
                self.run_log = format!("Run failed: {error}");
                self.status = "Run failed.".to_owned();
            }
        }
    }

    fn maybe_auto_check(&mut self, ctx: &egui::Context) {
        if let Some(last_edit) = self.last_edit {
            if last_edit.elapsed() >= Duration::from_millis(650) {
                self.check_current();
                self.last_edit = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(150));
            }
        }
    }

    fn insert_completion(&mut self, insertion: &str) {
        let prefix = current_prefix(&self.source, self.cursor_char_index);
        let cursor_byte = char_to_byte_index(&self.source, self.cursor_char_index);
        let prefix_chars = prefix.chars().count();
        let start_char = self.cursor_char_index.saturating_sub(prefix_chars);
        let start_byte = char_to_byte_index(&self.source, start_char);
        self.source
            .replace_range(start_byte..cursor_byte, insertion);
        self.cursor_char_index = start_char + insertion.chars().count();
        self.dirty = true;
        self.last_edit = Some(Instant::now());
    }

    fn resolve_path_input(&self) -> PathBuf {
        let input = self.path_input.trim();
        let path = PathBuf::from(input);
        if path.is_absolute() {
            path
        } else {
            self.root.join(path)
        }
    }

    fn relative_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }
}

impl eframe::App for EngIdeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.maybe_auto_check(ctx);

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Check").clicked() {
                    self.check_current();
                }
                if ui.button("Save").clicked() {
                    self.save_current();
                    self.check_current();
                }
                if ui.button("Run").clicked() {
                    self.run_current();
                }
                if ui.button("Open Report").clicked() {
                    if let Some(path) = &self.last_report_path {
                        open_path(path);
                    } else {
                        self.status = "No report yet. Run the current file first.".to_owned();
                    }
                }
                ui.separator();
                ui.label("Entry");
                ui.text_edit_singleline(&mut self.entry);
                ui.separator();
                ui.label(&self.status);
            });
            ui.horizontal(|ui| {
                ui.label("File");
                let response = ui.text_edit_singleline(&mut self.path_input);
                if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                    self.open_path_input();
                }
                if ui.button("Open").clicked() {
                    self.open_path_input();
                }
            });
        });

        egui::SidePanel::left("examples")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.heading("Examples");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let examples = self.examples.clone();
                    for path in examples {
                        let label = self.relative_path(&path);
                        let selected = path == self.current_path;
                        if ui.selectable_label(selected, label).clicked() {
                            self.current_path = path;
                            self.load_current();
                            self.check_current();
                        }
                    }
                });
            });

        egui::SidePanel::right("intelligence")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Completions");
                ui.horizontal(|ui| {
                    ui.label("Filter");
                    ui.text_edit_singleline(&mut self.completion_filter);
                });
                ui.small("Ctrl+Space updates the filter from the cursor prefix.");
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(250.0)
                    .show(ui, |ui| {
                        for item in completion_items(&self.completion_filter) {
                            if ui.button(&item.label).on_hover_text(&item.detail).clicked() {
                                self.insert_completion(&item.insert);
                            }
                        }
                    });
                ui.separator();
                ui.heading("Symbols");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for symbol in &self.symbols {
                        ui.label(format!(
                            "{}:{}  {} [{}]",
                            symbol.line, symbol.name, symbol.quantity_kind, symbol.display_unit
                        ))
                        .on_hover_text(&symbol.detail);
                    }
                });
            });

        egui::TopBottomPanel::bottom("diagnostics")
            .resizable(true)
            .default_height(170.0)
            .show(ctx, |ui| {
                ui.heading("Diagnostics");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.diagnostics.is_empty() {
                        ui.label("No diagnostics.");
                    }
                    for diagnostic in &self.diagnostics {
                        let text = format!(
                            "{}:{}:{} {}",
                            diagnostic.line,
                            diagnostic.severity,
                            diagnostic.code,
                            diagnostic.message
                        );
                        ui.label(text);
                        if let Some(help) = &diagnostic.help {
                            ui.small(format!("help: {help}"));
                        }
                    }
                    if !self.run_log.is_empty() {
                        ui.separator();
                        ui.label(&self.run_log);
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let text_output = egui::TextEdit::multiline(&mut self.source)
                .code_editor()
                .desired_rows(28)
                .lock_focus(true)
                .show(ui);
            if text_output.response.changed() {
                self.dirty = true;
                self.last_edit = Some(Instant::now());
            }
            if text_output.response.has_focus() {
                if let Some(cursor_range) = text_output.cursor_range {
                    self.cursor_char_index = cursor_range.primary.ccursor.index;
                }
                if ui.input(|input| input.key_pressed(egui::Key::Space) && input.modifiers.ctrl) {
                    self.completion_filter = current_prefix(&self.source, self.cursor_char_index);
                    self.show_completions = true;
                }
            }
        });
    }
}

#[derive(Clone)]
struct DiagnosticView {
    severity: String,
    code: String,
    line: usize,
    message: String,
    help: Option<String>,
}

#[derive(Clone)]
struct SymbolView {
    name: String,
    line: usize,
    quantity_kind: String,
    display_unit: String,
    detail: String,
}

#[derive(Clone)]
struct CompletionItem {
    label: String,
    insert: String,
    detail: String,
}

fn completion_items(filter: &str) -> Vec<CompletionItem> {
    let normalized = filter.trim().to_ascii_lowercase();
    let mut items = Vec::new();

    for keyword in [
        "schema",
        "script",
        "struct",
        "system",
        "state",
        "parameter",
        "input",
        "equation",
        "promote",
        "from",
        "policy",
        "missing",
        "where",
        "return",
        "plot",
        "line",
        "bar",
        "histogram",
        "der",
        "eq",
        "integrate",
        "mean",
        "max",
        "median",
        "std",
        "duration_above",
    ] {
        items.push(CompletionItem {
            label: keyword.to_owned(),
            insert: keyword.to_owned(),
            detail: "keyword".to_owned(),
        });
    }

    for quantity in all_quantity_completions() {
        items.push(CompletionItem {
            label: quantity.quantity_kind.to_owned(),
            insert: quantity.quantity_kind.to_owned(),
            detail: format!(
                "quantity, canonical unit {}, {}",
                quantity.canonical_unit, quantity.description
            ),
        });
    }

    for unit in all_unit_infos() {
        items.push(CompletionItem {
            label: unit.symbol.to_owned(),
            insert: unit.symbol.to_owned(),
            detail: format!(
                "unit for {}, canonical {}",
                unit.quantity_hint, unit.canonical_unit
            ),
        });
    }

    for (label, insert, detail) in [
        (
            "snippet: script main",
            "script main() -> Report {\n    value = 1 kW\n    return plot line value\n}",
            "main report script",
        ),
        (
            "snippet: csv schema",
            "schema Sensor {\n    time: DateTime [iso8601]\n    heat: HeatRate [kW]\n}",
            "typed CSV schema",
        ),
        (
            "snippet: thermal system",
            "system Room {\n    state T: AbsoluteTemperature = 20 degC\n    parameter C: HeatCapacity = 1200 kJ/K\n    parameter UA: Conductance = 250 W/K\n    input T_out: AbsoluteTemperature = 10 degC\n    input Q_internal: HeatRate = 500 W\n    equation energy_balance:\n        C * der(T) eq UA * (T_out - T) + Q_internal\n}",
            "first-order thermal system",
        ),
    ] {
        items.push(CompletionItem {
            label: label.to_owned(),
            insert: insert.to_owned(),
            detail: detail.to_owned(),
        });
    }

    items
        .into_iter()
        .filter(|item| {
            normalized.is_empty() || item.label.to_ascii_lowercase().contains(&normalized)
        })
        .take(80)
        .collect()
}

fn current_prefix(source: &str, cursor_char_index: usize) -> String {
    let before_cursor: String = source.chars().take(cursor_char_index).collect();
    before_cursor
        .chars()
        .rev()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

fn char_to_byte_index(source: &str, char_index: usize) -> usize {
    source
        .char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or_else(|| source.len())
}

fn workspace_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn collect_examples(root: &Path) -> Vec<PathBuf> {
    let mut examples = Vec::new();
    collect_eng_files(&root.join("examples"), &mut examples);
    examples.sort();
    examples
}

fn collect_eng_files(path: &Path, output: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_eng_files(&path, output);
        } else if path.extension().and_then(|value| value.to_str()) == Some("eng") {
            output.push(path);
        }
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
