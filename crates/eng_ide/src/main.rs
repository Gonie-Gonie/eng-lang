#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use eng_compiler::{
    all_quantity_completions, all_unit_infos, check_source, CheckOptions, Severity,
};
use eng_runtime::{run_file, RunOptions, RuntimeError};
use serde_json::Value;

const ACCENT: egui::Color32 = egui::Color32::from_rgb(28, 111, 202);
const BG: egui::Color32 = egui::Color32::from_rgb(246, 248, 251);
const PANEL: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
const PANEL_ALT: egui::Color32 = egui::Color32::from_rgb(241, 244, 248);
const BORDER: egui::Color32 = egui::Color32::from_rgb(210, 216, 224);
const TEXT: egui::Color32 = egui::Color32::from_rgb(25, 32, 44);
const MUTED: egui::Color32 = egui::Color32::from_rgb(99, 112, 130);
const ERROR: egui::Color32 = egui::Color32::from_rgb(184, 44, 44);
const WARNING: egui::Color32 = egui::Color32::from_rgb(173, 112, 20);
const OK: egui::Color32 = egui::Color32::from_rgb(43, 131, 91);

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
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 920.0])
            .with_min_inner_size([1120.0, 680.0]),
        ..Default::default()
    };
    eframe::run_native(
        "EngLang IDE",
        options,
        Box::new(|cc| {
            configure_ui(&cc.egui_ctx);
            Box::new(EngIdeApp::new())
        }),
    )
}

fn configure_ui(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals::light());
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.spacing.window_margin = egui::Margin::same(10.0);
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(14.5, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(14.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(19.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(15.0, egui::FontFamily::Monospace),
    );
    style.visuals.window_fill = BG;
    style.visuals.panel_fill = BG;
    style.visuals.extreme_bg_color = PANEL_ALT;
    style.visuals.widgets.noninteractive.fg_stroke.color = TEXT;
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(234, 238, 244);
    style.visuals.widgets.inactive.fg_stroke.color = TEXT;
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(222, 235, 249);
    style.visuals.widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(12, 74, 130);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(204, 224, 246);
    style.visuals.widgets.active.fg_stroke.color = egui::Color32::from_rgb(9, 61, 108);
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(185, 220, 252);
    style.visuals.selection.stroke.color = ACCENT;
    ctx.set_style(style);
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum RightTab {
    Inspector,
    Completions,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BottomTab {
    Problems,
    Output,
    Artifacts,
}

struct EngIdeApp {
    root: PathBuf,
    examples: Vec<PathBuf>,
    current_path: PathBuf,
    path_input: String,
    new_file_input: String,
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
    right_tab: RightTab,
    bottom_tab: BottomTab,
    show_explorer: bool,
    show_inspector_panel: bool,
    show_preview: bool,
    last_output: Option<RunOutputView>,
    plot_preview: Option<PlotPreview>,
}

impl EngIdeApp {
    fn new() -> Self {
        let root = workspace_root();
        let examples = collect_examples(&root);
        let current_path = examples
            .iter()
            .find(|path| path.ends_with("examples/official/03_integrated_hvac/main.eng"))
            .or_else(|| examples.first())
            .cloned()
            .unwrap_or_else(|| root.join("main.eng"));
        let mut app = Self {
            root,
            examples,
            current_path,
            path_input: String::new(),
            new_file_input: "examples/scratch/main.eng".to_owned(),
            source: String::new(),
            diagnostics: Vec::new(),
            symbols: Vec::new(),
            status: "Ready".to_owned(),
            run_log: String::new(),
            entry: "main".to_owned(),
            dirty: false,
            last_edit: None,
            cursor_char_index: 0,
            completion_filter: String::new(),
            right_tab: RightTab::Inspector,
            bottom_tab: BottomTab::Problems,
            show_explorer: true,
            show_inspector_panel: true,
            show_preview: true,
            last_output: None,
            plot_preview: None,
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
                self.run_log.clear();
                self.status = format!("Loaded {}", self.path_input);
            }
            Err(error) => {
                self.source.clear();
                self.diagnostics.clear();
                self.symbols.clear();
                self.status = format!("Could not load {}: {error}", self.current_path.display());
            }
        }
    }

    fn open_file(&mut self, path: PathBuf) {
        if self.dirty {
            self.save_current();
        }
        self.current_path = path;
        self.load_current();
        self.check_current();
    }

    fn open_path_input(&mut self) {
        let path = self.resolve_path_input();
        self.open_file(path);
    }

    fn browse_file(&mut self) {
        let start_dir = self
            .current_path
            .parent()
            .unwrap_or(self.root.as_path())
            .to_path_buf();
        if let Some(path) = rfd::FileDialog::new()
            .set_directory(start_dir)
            .add_filter("EngLang", &["eng"])
            .add_filter("Markdown", &["md"])
            .pick_file()
        {
            self.open_file(path);
        }
    }

    fn browse_folder(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_directory(&self.root)
            .pick_folder()
        {
            if self.dirty {
                self.save_current();
            }
            self.root = path;
            self.examples = collect_examples(&self.root);
            if let Some(first) = self.examples.first().cloned() {
                self.open_file(first);
            } else {
                self.current_path = self.root.join("main.eng");
                self.path_input = self.relative_path(&self.current_path);
                self.source.clear();
                self.diagnostics.clear();
                self.symbols.clear();
                self.dirty = false;
            }
            self.status = format!("Workspace: {}", self.root.display());
        }
    }

    fn create_new_file(&mut self) {
        let mut path = self.resolve_relative_or_absolute(self.new_file_input.trim());
        if path.extension().and_then(|value| value.to_str()).is_none() {
            path.set_extension("eng");
        }
        if path.exists() {
            self.status = format!("File already exists: {}", self.relative_path(&path));
            self.open_file(path);
            return;
        }
        if let Some(parent) = path.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                self.status = format!("Could not create {}: {error}", parent.display());
                return;
            }
        }
        let template = r#"script main() -> Report {
    value = 1 kW

    return report {
        show value
        plot value over Time {
            unit y = kW
            title = "EngLang preview"
        }
    }
}
"#;
        match fs::write(&path, template) {
            Ok(()) => {
                self.examples = collect_examples(&self.root);
                self.open_file(path);
            }
            Err(error) => {
                self.status = format!("New file failed: {error}");
            }
        }
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
        self.status = format!("Checked: {errors} errors, {warnings} warnings");
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
                let output = RunOutputView::from_output(output, &self.root);
                self.plot_preview = PlotPreview::from_plot_spec(&output.plot_spec_path).ok();
                self.run_log = output.summary();
                self.last_output = Some(output);
                self.status = "Run complete".to_owned();
                self.bottom_tab = BottomTab::Artifacts;
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
                self.run_log = "Run failed during compile. See Problems.".to_owned();
                self.status = "Run failed".to_owned();
                self.bottom_tab = BottomTab::Problems;
            }
            Err(error) => {
                self.run_log = format!("Run failed: {error}");
                self.status = "Run failed".to_owned();
                self.bottom_tab = BottomTab::Output;
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
        self.resolve_relative_or_absolute(self.path_input.trim())
    }

    fn resolve_relative_or_absolute(&self, input: &str) -> PathBuf {
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

    fn status_counts(&self) -> (usize, usize) {
        let errors = self
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == "error")
            .count();
        let warnings = self
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == "warning")
            .count();
        (errors, warnings)
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
            if primary_button(ui, "Check").clicked() {
                self.check_current();
                self.bottom_tab = BottomTab::Problems;
            }
            if ui.button("Save").clicked() {
                self.save_current();
                self.check_current();
            }
            if primary_button(ui, "Run").clicked() {
                self.run_current();
            }
            if ui.button("Report").clicked() {
                if let Some(output) = &self.last_output {
                    open_path(&output.report_path);
                } else {
                    self.status = "No report yet".to_owned();
                }
            }
            if ui.button("Plot SVG").clicked() {
                if let Some(output) = &self.last_output {
                    open_path(&output.plot_path);
                } else {
                    self.status = "No plot yet".to_owned();
                }
            }
            ui.separator();
            ui.toggle_value(&mut self.show_explorer, "Explorer");
            ui.toggle_value(&mut self.show_inspector_panel, "Inspector");
            ui.toggle_value(&mut self.show_preview, "Preview");
            ui.separator();
            ui.label("Entry");
            ui.add_sized([110.0, 28.0], egui::TextEdit::singleline(&mut self.entry));
            ui.separator();
            let (errors, warnings) = self.status_counts();
            status_badge(ui, "Errors", errors, if errors > 0 { ERROR } else { OK });
            status_badge(
                ui,
                "Warnings",
                warnings,
                if warnings > 0 { WARNING } else { OK },
            );
            if self.dirty {
                status_pill(ui, "Unsaved", WARNING);
            }
            ui.label(egui::RichText::new(&self.status).color(MUTED));
        });
        ui.add_space(3.0);
        ui.horizontal(|ui| {
            ui.label("File");
            let path_width = (ui.available_width() - 210.0).max(180.0);
            let response = ui.add_sized(
                [path_width, 26.0],
                egui::TextEdit::singleline(&mut self.path_input),
            );
            if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                self.open_path_input();
            }
            if ui.button("Open").clicked() {
                self.open_path_input();
            }
            if ui.button("Browse...").clicked() {
                self.browse_file();
            }
            if ui.button("Folder...").clicked() {
                self.browse_folder();
            }
        });
    }

    fn show_explorer(&mut self, ui: &mut egui::Ui) {
        panel_header(ui, "Explorer");
        ui.horizontal(|ui| {
            if ui.button("Open File...").clicked() {
                self.browse_file();
            }
            if ui.button("Open Folder...").clicked() {
                self.browse_folder();
            }
            if ui.button("Explorer").clicked() {
                open_path(&self.root);
            }
        });
        ui.label(egui::RichText::new(self.root.display().to_string()).color(MUTED));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_sized(
                [ui.available_width() - 62.0, 25.0],
                egui::TextEdit::singleline(&mut self.new_file_input),
            );
            if ui.button("New").clicked() {
                self.create_new_file();
            }
        });
        ui.add_space(8.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            for path in explorer_roots(&self.root) {
                if path.exists() {
                    self.show_directory(ui, &path, 0);
                }
            }
        });
    }

    fn show_directory(&mut self, ui: &mut egui::Ui, path: &Path, depth: usize) {
        let label = path
            .file_name()
            .and_then(|value| value.to_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        let entries = sorted_visible_entries(path);
        egui::CollapsingHeader::new(egui::RichText::new(label).strong())
            .default_open(depth < 2)
            .show(ui, |ui| {
                for entry in entries {
                    if entry.is_dir() {
                        self.show_directory(ui, &entry, depth + 1);
                    } else {
                        let display = entry
                            .file_name()
                            .and_then(|value| value.to_str())
                            .unwrap_or("file");
                        let selected = entry == self.current_path;
                        let response = ui.selectable_label(selected, display);
                        if response.clicked() {
                            self.open_file(entry.clone());
                        }
                        response.on_hover_text(self.relative_path(&entry));
                    }
                }
            });
    }

    fn show_editor(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(PANEL)
            .stroke(egui::Stroke::new(1.0, BORDER))
            .rounding(egui::Rounding::same(6.0))
            .inner_margin(egui::Margin::same(10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new(self.relative_path(&self.current_path)).size(16.0),
                    );
                    if self.dirty {
                        ui.label(egui::RichText::new("modified").color(WARNING));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{} lines", self.source.lines().count()))
                                .color(MUTED),
                        );
                    });
                });
                ui.separator();
                let diagnostics = self.diagnostics.clone();
                let mut layouter = move |ui: &egui::Ui, text: &str, wrap_width: f32| {
                    let mut job = highlight_eng(text, &diagnostics);
                    job.wrap.max_width = wrap_width;
                    ui.fonts(|fonts| fonts.layout_job(job))
                };
                let text_output = egui::TextEdit::multiline(&mut self.source)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(30)
                    .lock_focus(true)
                    .layouter(&mut layouter)
                    .show(ui);
                if text_output.response.changed() {
                    self.dirty = true;
                    self.last_edit = Some(Instant::now());
                }
                if text_output.response.has_focus() {
                    if let Some(cursor_range) = text_output.cursor_range {
                        self.cursor_char_index = cursor_range.primary.ccursor.index;
                    }
                    if ui.input(|input| input.key_pressed(egui::Key::Space) && input.modifiers.ctrl)
                    {
                        self.completion_filter =
                            current_prefix(&self.source, self.cursor_char_index);
                        self.right_tab = RightTab::Completions;
                    }
                }
            });
    }

    fn show_plot_preview(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(PANEL)
            .stroke(egui::Stroke::new(1.0, BORDER))
            .rounding(egui::Rounding::same(6.0))
            .inner_margin(egui::Margin::same(10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("Run Preview").size(16.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(output) = &self.last_output {
                            if ui.button("Open Result Folder").clicked() {
                                if let Some(parent) = output.result_path.parent() {
                                    open_path(parent);
                                }
                            }
                        }
                    });
                });
                ui.separator();
                if let Some(plot) = &self.plot_preview {
                    draw_plot(ui, plot);
                } else if self.last_output.is_some() {
                    ui.label(
                        egui::RichText::new("Run succeeded, but no PlotSpec preview was found.")
                            .color(MUTED),
                    );
                } else {
                    ui.label(
                        egui::RichText::new(
                            "Run the current file to see generated plots and artifacts here.",
                        )
                        .color(MUTED),
                    );
                }
            });
    }

    fn show_right_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            tab_button(ui, "Inspector", self.right_tab == RightTab::Inspector)
                .clicked()
                .then(|| self.right_tab = RightTab::Inspector);
            tab_button(ui, "Completions", self.right_tab == RightTab::Completions)
                .clicked()
                .then(|| self.right_tab = RightTab::Completions);
        });
        ui.separator();
        match self.right_tab {
            RightTab::Inspector => self.show_inspector(ui),
            RightTab::Completions => self.show_completions(ui),
        }
    }

    fn show_inspector(&mut self, ui: &mut egui::Ui) {
        panel_header(ui, "Symbols");
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.symbols.is_empty() {
                ui.label(egui::RichText::new("No symbols").color(MUTED));
            }
            for symbol in &self.symbols {
                egui::Frame::none()
                    .fill(PANEL_ALT)
                    .rounding(egui::Rounding::same(5.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&symbol.name).strong());
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(format!("L{}", symbol.line))
                                            .color(MUTED),
                                    );
                                },
                            );
                        });
                        ui.label(
                            egui::RichText::new(format!(
                                "{} [{}]",
                                symbol.quantity_kind, symbol.display_unit
                            ))
                            .color(MUTED),
                        );
                    })
                    .response
                    .on_hover_text(&symbol.detail);
                ui.add_space(5.0);
            }
        });
    }

    fn show_completions(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Filter");
            ui.add_sized(
                [ui.available_width(), 25.0],
                egui::TextEdit::singleline(&mut self.completion_filter),
            );
        });
        ui.add_space(6.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            for item in completion_items(&self.completion_filter) {
                let response = egui::Frame::none()
                    .fill(PANEL_ALT)
                    .rounding(egui::Rounding::same(5.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(&item.label).strong());
                        ui.label(egui::RichText::new(&item.detail).color(MUTED).size(12.0));
                    })
                    .response;
                if response.interact(egui::Sense::click()).clicked() {
                    self.insert_completion(&item.insert);
                }
                ui.add_space(5.0);
            }
        });
    }

    fn show_bottom_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            tab_button(ui, "Problems", self.bottom_tab == BottomTab::Problems)
                .clicked()
                .then(|| self.bottom_tab = BottomTab::Problems);
            tab_button(ui, "Output", self.bottom_tab == BottomTab::Output)
                .clicked()
                .then(|| self.bottom_tab = BottomTab::Output);
            tab_button(ui, "Artifacts", self.bottom_tab == BottomTab::Artifacts)
                .clicked()
                .then(|| self.bottom_tab = BottomTab::Artifacts);
        });
        ui.separator();
        match self.bottom_tab {
            BottomTab::Problems => self.show_problems(ui),
            BottomTab::Output => self.show_output(ui),
            BottomTab::Artifacts => self.show_artifacts(ui),
        }
    }

    fn show_problems(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.diagnostics.is_empty() {
                ui.label(egui::RichText::new("No diagnostics").color(OK));
            }
            for diagnostic in &self.diagnostics {
                let color = if diagnostic.severity == "error" {
                    ERROR
                } else {
                    WARNING
                };
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(&diagnostic.severity)
                            .color(color)
                            .strong(),
                    );
                    ui.label(egui::RichText::new(format!("L{}", diagnostic.line)).color(MUTED));
                    ui.label(egui::RichText::new(&diagnostic.code).monospace());
                    ui.label(&diagnostic.message);
                });
                if let Some(help) = &diagnostic.help {
                    ui.label(egui::RichText::new(format!("help: {help}")).color(MUTED));
                }
                ui.add_space(6.0);
            }
        });
    }

    fn show_output(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.run_log.is_empty() {
                ui.label(egui::RichText::new("No run output").color(MUTED));
            } else {
                ui.monospace(&self.run_log);
            }
        });
    }

    fn show_artifacts(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(output) = &self.last_output {
                artifact_row(ui, "Report HTML", &output.report_path);
                artifact_row(ui, "ReportSpec JSON", &output.report_spec_path);
                artifact_row(ui, "Plot SVG", &output.plot_path);
                artifact_row(ui, "PlotSpec JSON", &output.plot_spec_path);
                artifact_row(ui, "Plot Manifest", &output.plot_manifest_path);
                artifact_row(ui, "Result", &output.result_path);
                artifact_row(ui, "Review", &output.review_path);
                artifact_row(ui, "Bytecode", &output.bytecode_path);
            } else {
                ui.label(egui::RichText::new("No artifacts yet").color(MUTED));
            }
        });
    }
}

impl eframe::App for EngIdeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.maybe_auto_check(ctx);

        egui::TopBottomPanel::top("toolbar")
            .frame(panel_frame())
            .show(ctx, |ui| self.show_toolbar(ui));

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .default_height(210.0)
            .frame(panel_frame())
            .show(ctx, |ui| self.show_bottom_panel(ui));

        if self.show_explorer {
            egui::SidePanel::left("explorer")
                .resizable(true)
                .default_width(250.0)
                .width_range(170.0..=520.0)
                .frame(panel_frame())
                .show(ctx, |ui| self.show_explorer(ui));
        }

        if self.show_inspector_panel {
            egui::SidePanel::right("inspector")
                .resizable(true)
                .default_width(300.0)
                .width_range(220.0..=560.0)
                .frame(panel_frame())
                .show(ctx, |ui| self.show_right_panel(ui));
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(BG)
                    .inner_margin(egui::Margin::same(10.0)),
            )
            .show(ctx, |ui| {
                self.show_editor(ui);
                if self.show_preview {
                    ui.add_space(10.0);
                    self.show_plot_preview(ui);
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

struct RunOutputView {
    bytecode_path: PathBuf,
    result_path: PathBuf,
    review_path: PathBuf,
    report_path: PathBuf,
    report_spec_path: PathBuf,
    plot_path: PathBuf,
    plot_spec_path: PathBuf,
    plot_manifest_path: PathBuf,
    relative_report_path: String,
    relative_plot_path: String,
}

impl RunOutputView {
    fn from_output(output: eng_runtime::RunOutput, root: &Path) -> Self {
        Self {
            relative_report_path: relative_to(root, &output.report_path),
            relative_plot_path: relative_to(root, &output.plot_path),
            bytecode_path: output.bytecode_path,
            result_path: output.result_path,
            review_path: output.review_path,
            report_path: output.report_path,
            report_spec_path: output.report_spec_path,
            plot_path: output.plot_path,
            plot_spec_path: output.plot_spec_path,
            plot_manifest_path: output.plot_manifest_path,
        }
    }

    fn summary(&self) -> String {
        format!(
            "Run OK\nreport: {}\nplot:   {}\nresult: {}\nreview: {}\nreport spec: {}\nplotspec: {}\nmanifest: {}\nbytecode: {}",
            self.relative_report_path,
            self.relative_plot_path,
            self.result_path.display(),
            self.review_path.display(),
            self.report_spec_path.display(),
            self.plot_spec_path.display(),
            self.plot_manifest_path.display(),
            self.bytecode_path.display()
        )
    }
}

struct PlotPreview {
    title: String,
    plot_type: String,
    x_label: String,
    y_label: String,
    series_name: String,
    points: Vec<(f64, f64)>,
}

impl PlotPreview {
    fn from_plot_spec(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
        let value: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
        let title = json_string(&value, &["title"]).unwrap_or_else(|| "Plot".to_owned());
        let plot_type = json_string(&value, &["plot_type"]).unwrap_or_else(|| "line".to_owned());
        let x_label = axis_label(&value, "x_axis");
        let y_label = axis_label(&value, "y_axis");
        let series = value
            .get("series")
            .and_then(Value::as_array)
            .and_then(|items| items.first());
        let series_name = series
            .and_then(|item| item.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("series")
            .to_owned();
        let points = series
            .and_then(|item| item.get("points"))
            .and_then(Value::as_array)
            .map(|points| {
                points
                    .iter()
                    .filter_map(|point| {
                        let values = point.as_array()?;
                        let x = values.first()?.as_f64()?;
                        let y = values.get(1)?.as_f64()?;
                        Some((x, y))
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            title,
            plot_type,
            x_label,
            y_label,
            series_name,
            points,
        })
    }
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
        "report",
        "summarize",
        "show",
        "plot",
        "line",
        "bar",
        "histogram",
        "scatter",
        "parity",
        "residuals",
        "der",
        "eq",
        "integrate",
        "train_test_split",
        "regression",
        "mlp",
        "ann",
        "evaluate",
        "metrics",
        "model_card",
        "leakage_lint",
        "mean",
        "max",
        "median",
        "std",
        "duration_above",
    ] {
        items.push(CompletionItem {
            label: keyword.to_owned(),
            insert: keyword.to_owned(),
            detail: "language keyword".to_owned(),
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
            "script main() -> Report {\n    value = 1 kW\n\n    return report {\n        show value\n    }\n}",
            "main report script",
        ),
        (
            "snippet: csv schema",
            "schema Sensor {\n    time: DateTime index\n    heat: HeatRate [kW]\n}",
            "typed CSV schema",
        ),
        (
            "snippet: thermal system",
            "system Room {\n    state T: AbsoluteTemperature = 20 degC\n    parameter C: HeatCapacity = 1200 kJ/K\n    parameter UA: Conductance = 250 W/K\n    input T_out: AbsoluteTemperature = 10 degC\n    input Q_internal: HeatRate = 500 W\n    equation energy_balance:\n        C * der(T) eq UA * (T_out - T) + Q_internal\n}",
            "first-order thermal system",
        ),
        (
            "snippet: plot report",
            "return report {\n    summarize value by [mean, max, median, std]\n    plot value over Time {\n        unit y = kW\n        title = \"Preview\"\n    }\n}",
            "report with plot",
        ),
        (
            "snippet: ML model",
            "split = train_test_split(Q_coil, target=Q_coil, features=[T_supply, T_return, m_dot], test=0.5, seed=7)\nreg_model = regression(split, algorithm=linear)\nreg_eval = evaluate(reg_model, split=split)\n\nreturn report {\n    show reg_eval\n    plot parity(reg_eval) {\n        title = \"Regression parity\"\n    }\n}",
            "data-driven regression seed",
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
        .take(120)
        .collect()
}

fn highlight_eng(source: &str, diagnostics: &[DiagnosticView]) -> LayoutJob {
    let mut job = LayoutJob::default();
    for (line_index, line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let background = if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.line == line_number && diagnostic.severity == "error")
        {
            egui::Color32::from_rgb(255, 239, 239)
        } else if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.line == line_number && diagnostic.severity == "warning")
        {
            egui::Color32::from_rgb(255, 248, 226)
        } else {
            egui::Color32::TRANSPARENT
        };
        append_highlighted_line(&mut job, line, background);
        job.append("\n", 0.0, code_format(TEXT, background));
    }
    if source.is_empty() {
        job.append("", 0.0, code_format(TEXT, egui::Color32::TRANSPARENT));
    }
    job
}

fn append_highlighted_line(job: &mut LayoutJob, line: &str, background: egui::Color32) {
    let chars: Vec<char> = line.chars().collect();
    let mut index = 0usize;
    while index < chars.len() {
        let character = chars[index];
        if character == '"' {
            let start = index;
            index += 1;
            while index < chars.len() {
                let next = chars[index];
                index += 1;
                if next == '"' {
                    break;
                }
            }
            append_chars(
                job,
                &chars[start..index],
                egui::Color32::from_rgb(138, 78, 23),
                background,
            );
        } else if character == '#' {
            append_chars(job, &chars[index..], MUTED, background);
            break;
        } else if character.is_ascii_digit() {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index].is_ascii_digit() || chars[index] == '.' || chars[index] == '_')
            {
                index += 1;
            }
            append_chars(
                job,
                &chars[start..index],
                egui::Color32::from_rgb(126, 76, 175),
                background,
            );
        } else if character.is_ascii_alphabetic() || character == '_' {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric() || chars[index] == '_')
            {
                index += 1;
            }
            let token: String = chars[start..index].iter().collect();
            let color = if is_keyword(&token) {
                ACCENT
            } else if is_quantity_like(&token) {
                egui::Color32::from_rgb(120, 72, 156)
            } else {
                TEXT
            };
            append_chars(job, &chars[start..index], color, background);
        } else {
            append_chars(job, &chars[index..index + 1], TEXT, background);
            index += 1;
        }
    }
}

fn append_chars(
    job: &mut LayoutJob,
    chars: &[char],
    color: egui::Color32,
    background: egui::Color32,
) {
    let text: String = chars.iter().collect();
    job.append(&text, 0.0, code_format(color, background));
}

fn code_format(color: egui::Color32, background: egui::Color32) -> TextFormat {
    TextFormat {
        font_id: egui::FontId::monospace(15.0),
        color,
        background,
        ..Default::default()
    }
}

fn is_keyword(token: &str) -> bool {
    matches!(
        token,
        "schema"
            | "script"
            | "struct"
            | "system"
            | "state"
            | "parameter"
            | "input"
            | "equation"
            | "constraints"
            | "missing"
            | "policy"
            | "promote"
            | "csv"
            | "as"
            | "return"
            | "report"
            | "summarize"
            | "by"
            | "show"
            | "plot"
            | "over"
            | "unit"
            | "title"
            | "where"
            | "between"
            | "and"
            | "is"
            | "interpolate"
            | "error"
            | "index"
            | "from"
            | "eq"
            | "der"
            | "integrate"
            | "train_test_split"
            | "regression"
            | "mlp"
            | "ann"
            | "evaluate"
            | "metrics"
            | "model_card"
            | "leakage_lint"
            | "parity"
            | "residuals"
    )
}

fn is_quantity_like(token: &str) -> bool {
    token
        .chars()
        .next()
        .map(|character| character.is_ascii_uppercase())
        .unwrap_or(false)
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
    let examples_root = root.join("examples");
    if examples_root.exists() {
        collect_eng_files(&examples_root, &mut examples);
    } else {
        collect_eng_files(root, &mut examples);
    }
    examples.sort();
    examples
}

fn explorer_roots(root: &Path) -> Vec<PathBuf> {
    let preferred = [
        root.join("examples"),
        root.join("stdlib"),
        root.join("docs").join("tutorials"),
    ];
    let roots = preferred
        .into_iter()
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    if roots.is_empty() {
        vec![root.to_path_buf()]
    } else {
        roots
    }
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

fn sorted_visible_entries(path: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut entries: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            if path.is_dir() {
                true
            } else {
                matches!(
                    path.extension().and_then(|value| value.to_str()),
                    Some("eng") | Some("md")
                )
            }
        })
        .collect();
    entries.sort_by(|a, b| {
        b.is_dir()
            .cmp(&a.is_dir())
            .then_with(|| a.file_name().cmp(&b.file_name()))
    });
    entries
}

fn relative_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(PANEL)
        .stroke(egui::Stroke::new(1.0, BORDER))
        .inner_margin(egui::Margin::same(10.0))
}

fn panel_header(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(16.0).strong().color(TEXT));
    ui.add_space(4.0);
}

fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(text)
                .color(egui::Color32::WHITE)
                .strong(),
        )
        .fill(ACCENT),
    )
}

fn tab_button(ui: &mut egui::Ui, text: &str, selected: bool) -> egui::Response {
    let fill = if selected { ACCENT } else { PANEL_ALT };
    let color = if selected { egui::Color32::WHITE } else { TEXT };
    ui.add(egui::Button::new(egui::RichText::new(text).color(color)).fill(fill))
}

fn status_badge(ui: &mut egui::Ui, label: &str, count: usize, color: egui::Color32) {
    status_pill(ui, &format!("{label} {count}"), color);
}

fn status_pill(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    egui::Frame::none()
        .fill(color.linear_multiply(0.10))
        .stroke(egui::Stroke::new(1.0, color.linear_multiply(0.55)))
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::symmetric(8.0, 4.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).color(color).size(12.0));
        });
}

fn artifact_row(ui: &mut egui::Ui, label: &str, path: &Path) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).strong());
        ui.monospace(path.display().to_string());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Open").clicked() {
                open_path(path);
            }
        });
    });
    ui.add_space(5.0);
}

fn draw_plot(ui: &mut egui::Ui, plot: &PlotPreview) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(&plot.title).strong());
        ui.label(
            egui::RichText::new(format!("{} / {}", plot.plot_type, plot.series_name)).color(MUTED),
        );
    });
    let desired = egui::vec2(ui.available_width(), 260.0);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(
        rect,
        egui::Rounding::same(6.0),
        egui::Color32::from_rgb(250, 252, 255),
    );
    painter.rect_stroke(
        rect,
        egui::Rounding::same(6.0),
        egui::Stroke::new(1.0, BORDER),
    );

    let plot_rect = egui::Rect::from_min_max(
        rect.min + egui::vec2(58.0, 28.0),
        rect.max - egui::vec2(22.0, 42.0),
    );
    painter.line_segment(
        [plot_rect.left_bottom(), plot_rect.right_bottom()],
        egui::Stroke::new(1.5, egui::Color32::from_rgb(65, 76, 90)),
    );
    painter.line_segment(
        [plot_rect.left_bottom(), plot_rect.left_top()],
        egui::Stroke::new(1.5, egui::Color32::from_rgb(65, 76, 90)),
    );

    if plot.points.is_empty() {
        painter.text(
            plot_rect.center(),
            egui::Align2::CENTER_CENTER,
            "No points",
            egui::FontId::proportional(14.0),
            MUTED,
        );
    } else {
        let (min_x, max_x, min_y, max_y) = point_bounds(&plot.points);
        let to_screen = |point: (f64, f64)| -> egui::Pos2 {
            let x_t = normalized(point.0, min_x, max_x);
            let y_t = normalized(point.1, min_y, max_y);
            egui::pos2(
                plot_rect.left() + (x_t as f32) * plot_rect.width(),
                plot_rect.bottom() - (y_t as f32) * plot_rect.height(),
            )
        };
        if plot.plot_type == "bar" || plot.plot_type == "histogram" {
            let bar_width = (plot_rect.width() / plot.points.len().max(1) as f32) * 0.68;
            for point in &plot.points {
                let screen = to_screen(*point);
                let base = egui::pos2(screen.x, plot_rect.bottom());
                let bar_rect = egui::Rect::from_center_size(
                    egui::pos2(screen.x, (screen.y + base.y) * 0.5),
                    egui::vec2(bar_width.max(2.0), (base.y - screen.y).abs().max(1.0)),
                );
                painter.rect_filled(bar_rect, egui::Rounding::same(2.0), ACCENT);
            }
        } else {
            let points: Vec<egui::Pos2> = plot.points.iter().copied().map(to_screen).collect();
            painter.add(egui::Shape::line(
                points.clone(),
                egui::Stroke::new(2.5, ACCENT),
            ));
            for point in points.iter().step_by((points.len() / 24).max(1)) {
                painter.circle_filled(*point, 2.8, egui::Color32::from_rgb(255, 255, 255));
                painter.circle_stroke(*point, 2.8, egui::Stroke::new(1.2, ACCENT));
            }
        }
        painter.text(
            plot_rect.left_top() + egui::vec2(0.0, -18.0),
            egui::Align2::LEFT_CENTER,
            format!("y min {:.3}, max {:.3}", min_y, max_y),
            egui::FontId::proportional(12.0),
            MUTED,
        );
        painter.text(
            plot_rect.right_bottom() + egui::vec2(0.0, 18.0),
            egui::Align2::RIGHT_CENTER,
            format!("x {:.3}..{:.3}", min_x, max_x),
            egui::FontId::proportional(12.0),
            MUTED,
        );
    }
    painter.text(
        rect.center_bottom() - egui::vec2(0.0, 14.0),
        egui::Align2::CENTER_CENTER,
        &plot.x_label,
        egui::FontId::proportional(13.0),
        TEXT,
    );
    painter.text(
        rect.left_center() + egui::vec2(18.0, 0.0),
        egui::Align2::CENTER_CENTER,
        &plot.y_label,
        egui::FontId::proportional(13.0),
        TEXT,
    );
}

fn point_bounds(points: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (x, y) in points {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
    }
    (min_x, max_x, min_y, max_y)
}

fn normalized(value: f64, min: f64, max: f64) -> f64 {
    if (max - min).abs() < f64::EPSILON {
        0.5
    } else {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    }
}

fn json_string(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(ToOwned::to_owned)
}

fn axis_label(value: &Value, axis: &str) -> String {
    let label = json_string(value, &[axis, "label"])
        .or_else(|| json_string(value, &[axis, "name"]))
        .unwrap_or_else(|| axis.to_owned());
    let unit = json_string(value, &[axis, "unit"]).unwrap_or_default();
    if unit.is_empty() {
        label
    } else {
        format!("{label} [{unit}]")
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
