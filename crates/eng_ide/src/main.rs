#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use eng_compiler::{
    all_quantity_completions, all_unit_infos, check_source, CheckOptions, CheckReport, Severity,
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
const RESULT_DEFAULT_WIDTH: f32 = 480.0;
const RESULT_MIN_WIDTH: f32 = 320.0;
const CODE_MIN_WIDTH: f32 = 420.0;
const SPLITTER_WIDTH: f32 = 7.0;

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
    configure_fonts(ctx);
    ctx.set_visuals(egui::Visuals::light());
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(9.0, 5.0);
    style.spacing.window_margin = egui::Margin::same(8.0);
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(13.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(12.5, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(16.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(13.5, egui::FontFamily::Monospace),
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

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    install_font_if_present(
        &mut fonts,
        egui::FontFamily::Proportional,
        "segoe_ui",
        &[
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "C:\\Windows\\Fonts\\segoeuisl.ttf",
        ],
    );
    install_font_if_present(
        &mut fonts,
        egui::FontFamily::Monospace,
        "eng_mono",
        &[
            "C:\\Windows\\Fonts\\CascadiaMono.ttf",
            "C:\\Windows\\Fonts\\consola.ttf",
            "C:\\Windows\\Fonts\\cour.ttf",
        ],
    );
    ctx.set_fonts(fonts);
}

fn install_font_if_present(
    fonts: &mut egui::FontDefinitions,
    family: egui::FontFamily,
    name: &str,
    candidates: &[&str],
) {
    for candidate in candidates {
        if let Ok(bytes) = fs::read(candidate) {
            fonts
                .font_data
                .insert(name.to_owned(), egui::FontData::from_owned(bytes));
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, name.to_owned());
            return;
        }
    }
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
    let jit_plan = eng_jit::plan_for_report(&report);
    if jit_plan.candidates.is_empty() {
        eprintln!(
            "EngLang IDE smoke failed: {} produced no kernel plan candidates",
            first.display()
        );
        std::process::exit(3);
    }
    let domain_example = examples
        .iter()
        .find(|path| path.ends_with("examples/official/06_domain_port/main.eng"))
        .expect("official domain example is present");
    let domain_source = match fs::read_to_string(domain_example) {
        Ok(source) => source,
        Err(error) => {
            eprintln!(
                "EngLang IDE smoke failed: could not read {}: {error}",
                domain_example.display()
            );
            std::process::exit(4);
        }
    };
    let domain_report = check_source(domain_example, &domain_source, &CheckOptions::default());
    if domain_report.has_errors()
        || domain_report.semantic_program.domains.is_empty()
        || domain_report.semantic_program.components.is_empty()
        || domain_report.semantic_program.connections.is_empty()
    {
        eprintln!(
            "EngLang IDE smoke failed: {} did not produce domain/component metadata",
            domain_example.display()
        );
        std::process::exit(5);
    }
    println!(
        "EngLang IDE smoke OK: {} example(s), {} quantity completion(s), {} unit completion(s), {} kernel candidate(s), {} domain(s), {} component(s), {} connection(s)",
        examples.len(),
        all_quantity_completions().len(),
        all_unit_infos().len(),
        jit_plan.candidates.len(),
        domain_report.semantic_program.domains.len(),
        domain_report.semantic_program.components.len(),
        domain_report.semantic_program.connections.len()
    );
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RightTab {
    Inspector,
    Completions,
    Runtime,
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
    unit_derivations: Vec<UnitDerivationView>,
    schemas: Vec<SchemaView>,
    csv_promotions: Vec<CsvPromotionView>,
    domains: Vec<DomainView>,
    components: Vec<ComponentView>,
    connections: Vec<ConnectionView>,
    jit_plan: Option<JitPlanView>,
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
    result_width: f32,
    last_output: Option<RunOutputView>,
    plot_preview: Option<PlotPreview>,
    artifact_summary: Option<ArtifactSummary>,
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
            unit_derivations: Vec::new(),
            schemas: Vec::new(),
            csv_promotions: Vec::new(),
            domains: Vec::new(),
            components: Vec::new(),
            connections: Vec::new(),
            jit_plan: None,
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
            result_width: RESULT_DEFAULT_WIDTH,
            last_output: None,
            plot_preview: None,
            artifact_summary: None,
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
                self.last_output = None;
                self.plot_preview = None;
                self.artifact_summary = None;
                self.status = format!("Loaded {}", self.path_input);
            }
            Err(error) => {
                self.source.clear();
                self.diagnostics.clear();
                self.symbols.clear();
                self.unit_derivations.clear();
                self.schemas.clear();
                self.csv_promotions.clear();
                self.domains.clear();
                self.components.clear();
                self.connections.clear();
                self.jit_plan = None;
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
                self.unit_derivations.clear();
                self.schemas.clear();
                self.csv_promotions.clear();
                self.domains.clear();
                self.components.clear();
                self.connections.clear();
                self.jit_plan = None;
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
        self.apply_check_report(&report);
        let errors = report.diagnostic_count(Severity::Error);
        let warnings = report.diagnostic_count(Severity::Warning);
        self.status = format!("Checked: {errors} errors, {warnings} warnings");
    }

    fn apply_check_report(&mut self, report: &CheckReport) {
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

        self.unit_derivations = report
            .semantic_program
            .unit_derivations
            .iter()
            .map(|derivation| UnitDerivationView {
                name: derivation.name.clone(),
                line: derivation.line,
                quantity_kind: derivation.quantity_kind.clone(),
                source_unit: derivation.source_unit.clone(),
                display_unit: derivation.display_unit.clone(),
                canonical_unit: derivation.canonical_unit.clone(),
                expression: derivation.expression.clone(),
                steps: derivation.steps.clone(),
            })
            .collect();

        self.symbols = report
            .semantic_program
            .hover_hints
            .iter()
            .map(|hover| {
                let type_info = report
                    .semantic_program
                    .type_infos
                    .iter()
                    .find(|info| info.name == hover.name && info.line == hover.line);
                let derivation = self
                    .unit_derivations
                    .iter()
                    .find(|item| item.name == hover.name && item.line == hover.line);
                SymbolView {
                    name: hover.name.clone(),
                    line: hover.line,
                    quantity_kind: hover.quantity_kind.clone(),
                    display_unit: hover.display_unit.clone(),
                    canonical_unit: type_info
                        .map(|info| info.canonical_unit.clone())
                        .or_else(|| derivation.map(|item| item.canonical_unit.clone()))
                        .unwrap_or_else(|| hover.display_unit.clone()),
                    dimension: type_info
                        .map(|info| info.dimension.clone())
                        .unwrap_or_else(|| "-".to_owned()),
                    source: type_info
                        .map(|info| info.source.as_str().to_owned())
                        .unwrap_or_else(|| "symbol".to_owned()),
                    source_unit: derivation.and_then(|item| item.source_unit.clone()),
                    expression: derivation.and_then(|item| item.expression.clone()),
                    steps: derivation
                        .map(|item| item.steps.clone())
                        .unwrap_or_default(),
                    detail: hover.detail.clone(),
                }
            })
            .collect();

        self.schemas = report
            .semantic_program
            .schemas
            .iter()
            .map(|schema| SchemaView {
                name: schema.name.clone(),
                line: schema.line,
                columns: schema
                    .columns
                    .iter()
                    .map(|column| SchemaColumnView {
                        name: column.name.clone(),
                        type_name: column.type_name.clone(),
                        unit: column.unit.clone(),
                        is_index: column.is_index,
                        line: column.line,
                    })
                    .collect(),
                constraints: schema
                    .constraints
                    .iter()
                    .map(|constraint| TextLineView {
                        text: constraint.text.clone(),
                        line: constraint.line,
                    })
                    .collect(),
                missing_policies: schema
                    .missing_policies
                    .iter()
                    .map(|policy| MissingPolicyView {
                        column: policy.column.clone(),
                        policy: policy.policy.clone(),
                        line: policy.line,
                    })
                    .collect(),
            })
            .collect();

        self.csv_promotions = report
            .semantic_program
            .csv_promotions
            .iter()
            .map(|promotion| CsvPromotionView {
                binding: promotion.binding.clone(),
                schema_name: promotion.schema_name.clone(),
                source_value: promotion.source_value.clone(),
                resolved_path: promotion.resolved_path.clone(),
                row_count: promotion.row_count,
                headers: promotion.headers.clone(),
                missing_columns: promotion.missing_columns.clone(),
                line: promotion.line,
            })
            .collect();

        self.domains = report
            .semantic_program
            .domains
            .iter()
            .map(|domain| DomainView {
                name: domain.name.clone(),
                type_parameters: domain
                    .type_parameters
                    .iter()
                    .map(|parameter| DomainParameterView {
                        kind: parameter.kind.clone(),
                        name: parameter.name.clone(),
                        display: parameter.display.clone(),
                    })
                    .collect(),
                package: domain.package.clone(),
                version: domain.version.clone(),
                line: domain.line,
                variables: domain
                    .variables
                    .iter()
                    .map(|variable| DomainVariableView {
                        role: variable.role.clone(),
                        name: variable.name.clone(),
                        quantity_kind: variable.quantity_kind.clone(),
                        display_unit: variable.display_unit.clone(),
                        canonical_unit: variable.canonical_unit.clone(),
                        dimension: variable.dimension.clone(),
                        line: variable.line,
                    })
                    .collect(),
                conservations: domain
                    .conservations
                    .iter()
                    .map(|conservation| DomainConservationView {
                        text: conservation.text.clone(),
                        status: conservation.status.clone(),
                        line: conservation.line,
                    })
                    .collect(),
            })
            .collect();

        self.components = report
            .semantic_program
            .components
            .iter()
            .map(|component| ComponentView {
                name: component.name.clone(),
                line: component.line,
                ports: component
                    .ports
                    .iter()
                    .map(|port| PortView {
                        name: port.name.clone(),
                        domain: port.domain.clone(),
                        domain_name: port.domain_name.clone(),
                        type_arguments: port.type_arguments.clone(),
                        status: port.status.clone(),
                        line: port.line,
                    })
                    .collect(),
            })
            .collect();

        self.connections = report
            .semantic_program
            .connections
            .iter()
            .map(|connection| ConnectionView {
                left: connection.left.clone(),
                right: connection.right.clone(),
                domain: connection.domain.clone(),
                status: connection.status.clone(),
                line: connection.line,
            })
            .collect();

        self.jit_plan = Some(JitPlanView::from_plan(&eng_jit::plan_for_report(report)));
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
                self.artifact_summary = ArtifactSummary::from_result(&output.result_path).ok();
                self.run_log = output.summary();
                self.last_output = Some(output);
                self.status = "Run complete".to_owned();
                self.bottom_tab = BottomTab::Artifacts;
                self.right_tab = RightTab::Runtime;
            }
            Err(RuntimeError::Compile(report)) => {
                self.apply_check_report(&report);
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

    fn completion_items_for_filter(&self, filter: &str) -> Vec<CompletionItem> {
        completion_items(filter, &self.symbols, &self.source)
    }

    fn first_completion_for_prefix(&self) -> Option<CompletionItem> {
        let prefix = current_prefix(&self.source, self.cursor_char_index);
        if prefix.is_empty() {
            return None;
        }
        self.completion_items_for_filter(&prefix).into_iter().next()
    }

    fn accept_first_completion(&mut self) -> bool {
        let Some(item) = self.first_completion_for_prefix() else {
            return false;
        };
        self.insert_completion(&item.insert);
        self.completion_filter = current_prefix(&self.source, self.cursor_char_index);
        self.right_tab = RightTab::Completions;
        true
    }

    fn update_completion_filter_from_cursor(&mut self) {
        let prefix = current_prefix(&self.source, self.cursor_char_index);
        if prefix.is_empty() {
            self.completion_filter.clear();
            return;
        }
        self.completion_filter = prefix;
        self.right_tab = RightTab::Completions;
    }

    fn maybe_auto_close_pair(&mut self, before: &str) {
        if before == self.source {
            return;
        }
        let before_chars: Vec<char> = before.chars().collect();
        let after_chars: Vec<char> = self.source.chars().collect();
        if after_chars.len() != before_chars.len() + 1 {
            return;
        }
        let prefix_len = before_chars
            .iter()
            .zip(after_chars.iter())
            .take_while(|(before, after)| before == after)
            .count();
        if before_chars[prefix_len..] != after_chars[prefix_len + 1..] {
            return;
        }
        if self.cursor_char_index != prefix_len + 1 {
            return;
        }
        let inserted = after_chars[prefix_len];
        let Some(closer) = matching_closer(inserted) else {
            return;
        };
        if self
            .source
            .chars()
            .nth(self.cursor_char_index)
            .is_some_and(|next| next == closer)
        {
            return;
        }
        let cursor_byte = char_to_byte_index(&self.source, self.cursor_char_index);
        self.source.insert(cursor_byte, closer);
    }

    fn remove_single_inserted_char(&mut self, before: &str, expected: char) -> bool {
        let before_chars: Vec<char> = before.chars().collect();
        let after_chars: Vec<char> = self.source.chars().collect();
        if after_chars.len() != before_chars.len() + 1 {
            return false;
        }
        let prefix_len = before_chars
            .iter()
            .zip(after_chars.iter())
            .take_while(|(before, after)| before == after)
            .count();
        if after_chars.get(prefix_len).copied() != Some(expected) {
            return false;
        }
        if before_chars[prefix_len..] != after_chars[prefix_len + 1..] {
            return false;
        }
        self.source = before.to_owned();
        self.cursor_char_index = prefix_len;
        true
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
            ui.toggle_value(&mut self.show_inspector_panel, "Sidebar");
            ui.toggle_value(&mut self.show_preview, "Result");
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
        ui.horizontal_wrapped(|ui| {
            if compact_button(ui, "Open File").clicked() {
                self.browse_file();
            }
            if compact_button(ui, "Open Folder").clicked() {
                self.browse_folder();
            }
            if compact_button(ui, "Reveal").clicked() {
                open_path(&self.root);
            }
        });
        ui.label(
            egui::RichText::new(self.root.display().to_string())
                .color(MUTED)
                .monospace()
                .size(11.5),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_sized(
                [(ui.available_width() - 84.0).max(90.0), 23.0],
                egui::TextEdit::singleline(&mut self.new_file_input),
            );
            if compact_button(ui, "New").clicked() {
                self.create_new_file();
            }
        });
        ui.add_space(6.0);
        ui.separator();
        section_label(ui, "Workspace");

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
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
        let label = explorer_directory_label(&self.root, path).unwrap_or(label);
        let default_open = depth < 1 || is_official_examples_dir(&self.root, path);
        egui::CollapsingHeader::new(egui::RichText::new(label).strong().size(12.5).color(TEXT))
            .default_open(default_open)
            .show(ui, |ui| {
                for entry in entries {
                    if entry.is_dir() {
                        self.show_directory(ui, &entry, depth + 1);
                    } else {
                        self.show_file_row(ui, &entry, depth + 1);
                    }
                }
            });
    }

    fn show_file_row(&mut self, ui: &mut egui::Ui, entry: &Path, depth: usize) {
        let display = entry
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("file");
        let extension = entry
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let selected = entry == self.current_path;
        let category = example_category_label(entry);
        let fill = if selected {
            egui::Color32::from_rgb(218, 235, 252)
        } else {
            egui::Color32::TRANSPARENT
        };
        let response = egui::Frame::none()
            .fill(fill)
            .rounding(egui::Rounding::same(4.0))
            .inner_margin(egui::Margin::symmetric(4.0, 2.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.add_space(depth as f32 * 10.0);
                    ui.label(egui::RichText::new(display).size(12.5).color(TEXT));
                    if !extension.is_empty() {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(extension.to_ascii_uppercase())
                                    .size(10.5)
                                    .color(MUTED),
                            );
                            if let Some(category) = category {
                                ui.label(egui::RichText::new(category).size(10.5).color(ACCENT));
                            }
                        });
                    }
                });
            })
            .response
            .interact(egui::Sense::click());
        if response.clicked() {
            self.open_file(entry.to_path_buf());
        }
        let hover = if let Some(category) = category {
            format!("{category}: {}", self.relative_path(entry))
        } else {
            self.relative_path(entry)
        };
        response.on_hover_text(hover);
        ui.add_space(1.0);
    }

    fn show_editor(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(PANEL)
            .stroke(egui::Stroke::new(1.0, BORDER))
            .rounding(egui::Rounding::same(4.0))
            .inner_margin(egui::Margin::same(8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new(self.relative_path(&self.current_path)).size(14.0),
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
                let editor_height = ui.available_height().max(260.0);
                let editor_width = ui.available_width().max(CODE_MIN_WIDTH);
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .max_height(editor_height)
                    .max_width(editor_width)
                    .show(ui, |ui| {
                        let content_width = editor_width.max(estimated_code_width(&self.source));
                        ui.set_min_size(egui::vec2(content_width, editor_height));
                        let line_count = self.source.lines().count().max(32);
                        let source_before = self.source.clone();
                        let completion_before_tab = self.first_completion_for_prefix();
                        let tab_requested = ui.input(|input| {
                            input.key_pressed(egui::Key::Tab)
                                && !input.modifiers.shift
                                && !input.modifiers.ctrl
                                && !input.modifiers.alt
                        });
                        let text_output = egui::TextEdit::multiline(&mut self.source)
                            .code_editor()
                            .desired_width(content_width)
                            .desired_rows(line_count + 2)
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
                            let mut accepted_completion = false;
                            if tab_requested {
                                if let Some(item) = completion_before_tab {
                                    self.remove_single_inserted_char(&source_before, '\t');
                                    self.insert_completion(&item.insert);
                                    self.completion_filter =
                                        current_prefix(&self.source, self.cursor_char_index);
                                    self.right_tab = RightTab::Completions;
                                    accepted_completion = true;
                                } else if self.accept_first_completion() {
                                    accepted_completion = true;
                                }
                            }
                            if text_output.response.changed() && !accepted_completion {
                                self.maybe_auto_close_pair(&source_before);
                            }
                            if ui.input(|input| {
                                input.key_pressed(egui::Key::Space) && input.modifiers.ctrl
                            }) {
                                self.completion_filter =
                                    current_prefix(&self.source, self.cursor_char_index);
                                self.right_tab = RightTab::Completions;
                            }
                            self.update_completion_filter_from_cursor();
                        }
                    });
                let prefix = current_prefix(&self.source, self.cursor_char_index);
                if let Some(item) = self.first_completion_for_prefix() {
                    if !prefix.is_empty() && item.insert != prefix {
                        ui.add_space(4.0);
                        completion_hint(ui, &prefix, &item);
                    }
                }
            });
    }

    fn show_plot_preview(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(PANEL)
            .stroke(egui::Stroke::new(1.0, BORDER))
            .rounding(egui::Rounding::same(4.0))
            .inner_margin(egui::Margin::same(8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("Run Preview").size(14.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(output) = &self.last_output {
                            if compact_button(ui, "Open Folder").clicked() {
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

    fn show_result_panel(&mut self, ui: &mut egui::Ui) {
        panel_header(ui, "Result");
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.show_plot_preview(ui);
                ui.add_space(10.0);
                egui::Frame::none()
                    .fill(PANEL)
                    .stroke(egui::Stroke::new(1.0, BORDER))
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.heading(egui::RichText::new("Runtime").size(14.0));
                            ui.label(egui::RichText::new("result.engres summary").color(MUTED));
                        });
                        ui.separator();
                        self.show_runtime_summary_content(ui);
                    });
                ui.add_space(10.0);
                egui::Frame::none()
                    .fill(PANEL)
                    .stroke(egui::Stroke::new(1.0, BORDER))
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.heading(egui::RichText::new("Artifacts").size(14.0));
                        ui.separator();
                        self.show_artifacts_content(ui);
                    });
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
            tab_button(ui, "Runtime", self.right_tab == RightTab::Runtime)
                .clicked()
                .then(|| self.right_tab = RightTab::Runtime);
        });
        ui.separator();
        match self.right_tab {
            RightTab::Inspector => self.show_inspector(ui),
            RightTab::Completions => self.show_completions(ui),
            RightTab::Runtime => self.show_runtime_inspector(ui),
        }
    }

    fn show_inspector(&mut self, ui: &mut egui::Ui) {
        panel_header(ui, "Inspector");
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                metric_chip(ui, "Variables", &self.symbols.len().to_string(), ACCENT);
                metric_chip(ui, "Schemas", &self.schemas.len().to_string(), ACCENT);
                metric_chip(ui, "CSV", &self.csv_promotions.len().to_string(), ACCENT);
                metric_chip(ui, "Domains", &self.domains.len().to_string(), ACCENT);
                metric_chip(ui, "Components", &self.components.len().to_string(), ACCENT);
                metric_chip(
                    ui,
                    "Connections",
                    &self.connections.len().to_string(),
                    ACCENT,
                );
            });
            ui.add_space(8.0);

            self.show_domain_component_inspector(ui);

            section_label(ui, "Variables");
            if self.symbols.is_empty() {
                ui.label(egui::RichText::new("No symbols").color(MUTED));
            }
            for symbol in &self.symbols {
                runtime_card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&symbol.name).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            status_pill(ui, &format!("L{}", symbol.line), MUTED);
                        });
                    });
                    key_value_row(ui, "quantity", &symbol.quantity_kind);
                    key_value_row(ui, "display", &symbol.display_unit);
                    key_value_row(ui, "canonical", &symbol.canonical_unit);
                    key_value_row(ui, "dimension", &symbol.dimension);
                    key_value_row(ui, "source", &symbol.source);
                    if let Some(source_unit) = &symbol.source_unit {
                        key_value_row(ui, "source unit", source_unit);
                    }
                    if let Some(expression) = &symbol.expression {
                        key_value_row(ui, "expression", expression);
                    }
                    if !symbol.steps.is_empty() {
                        key_value_row(ui, "unit path", &symbol.steps.join(" -> "));
                    }
                })
                .response
                .on_hover_text(&symbol.detail);
                ui.add_space(5.0);
            }

            self.show_unit_derivations(ui);
            self.show_schema_inspector(ui);
            self.show_csv_promotions(ui);
        });
    }

    fn show_domain_component_inspector(&self, ui: &mut egui::Ui) {
        section_label(ui, "Domain Graph");
        if self.domains.is_empty() && self.components.is_empty() && self.connections.is_empty() {
            ui.label(egui::RichText::new("No domain/component declarations").color(MUTED));
            ui.add_space(8.0);
            return;
        }

        if !self.domains.is_empty() {
            ui.label(egui::RichText::new("Domains").color(MUTED).size(12.0));
            for domain in &self.domains {
                runtime_card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&domain.name).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            status_pill(ui, &format!("L{}", domain.line), MUTED);
                        });
                    });
                    key_value_row(ui, "variables", &domain.variables.len().to_string());
                    key_value_row(
                        ui,
                        "parameters",
                        &domain_parameter_list(&domain.type_parameters),
                    );
                    if let Some(package) = &domain.package {
                        key_value_row(ui, "package", package);
                    }
                    if let Some(version) = &domain.version {
                        key_value_row(ui, "version", version);
                    }
                    for variable in &domain.variables {
                        let detail = format!(
                            "{} {}: {} [{}] -> {}",
                            variable.role,
                            variable.name,
                            variable.quantity_kind,
                            variable.display_unit,
                            variable.canonical_unit
                        );
                        key_value_row(ui, &format!("L{}", variable.line), &detail);
                        key_value_row(ui, "dimension", &variable.dimension);
                    }
                    if !domain.conservations.is_empty() {
                        key_value_row(ui, "conservation", &domain.conservations.len().to_string());
                        for conservation in &domain.conservations {
                            ui.horizontal_wrapped(|ui| {
                                ui.add_sized(
                                    [92.0, 18.0],
                                    egui::Label::new(
                                        egui::RichText::new(format!("L{}", conservation.line))
                                            .color(MUTED)
                                            .size(12.0),
                                    ),
                                );
                                ui.label(
                                    egui::RichText::new(&conservation.text)
                                        .monospace()
                                        .size(12.0),
                                );
                                status_pill(
                                    ui,
                                    &conservation.status,
                                    status_color(&conservation.status),
                                );
                            });
                        }
                    }
                });
                ui.add_space(5.0);
            }
        }

        if !self.components.is_empty() {
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Components").color(MUTED).size(12.0));
            for component in &self.components {
                runtime_card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&component.name).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            status_pill(ui, &format!("L{}", component.line), MUTED);
                        });
                    });
                    key_value_row(ui, "ports", &component.ports.len().to_string());
                    for port in &component.ports {
                        ui.horizontal_wrapped(|ui| {
                            ui.add_sized(
                                [92.0, 18.0],
                                egui::Label::new(
                                    egui::RichText::new(format!("L{}", port.line))
                                        .color(MUTED)
                                        .size(12.0),
                                ),
                            );
                            ui.label(
                                egui::RichText::new(format!("{}: {}", port.name, port.domain))
                                    .monospace()
                                    .size(12.0),
                            );
                            status_pill(ui, &port.status, status_color(&port.status));
                        });
                        if !port.type_arguments.is_empty() {
                            key_value_row(ui, "arguments", &compact_list(&port.type_arguments, 4));
                        }
                        key_value_row(ui, "domain base", &port.domain_name);
                    }
                });
                ui.add_space(5.0);
            }
        }

        if !self.connections.is_empty() {
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Connections").color(MUTED).size(12.0));
            for connection in &self.connections {
                runtime_card(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(format!("L{}", connection.line)).color(MUTED));
                        ui.label(
                            egui::RichText::new(format!(
                                "{} -> {}",
                                connection.left, connection.right
                            ))
                            .monospace()
                            .strong(),
                        );
                    });
                    key_value_row(ui, "domain", &connection.domain);
                    ui.horizontal_wrapped(|ui| {
                        ui.add_sized(
                            [92.0, 18.0],
                            egui::Label::new(egui::RichText::new("status").color(MUTED).size(12.0)),
                        );
                        status_pill(ui, &connection.status, status_color(&connection.status));
                    });
                });
                ui.add_space(5.0);
            }
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);
    }

    fn show_unit_derivations(&self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        section_label(ui, "Unit Paths");
        if self.unit_derivations.is_empty() {
            ui.label(egui::RichText::new("No unit derivations").color(MUTED));
            return;
        }
        for derivation in &self.unit_derivations {
            runtime_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&derivation.name).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        status_pill(ui, &format!("L{}", derivation.line), MUTED);
                    });
                });
                key_value_row(ui, "quantity", &derivation.quantity_kind);
                key_value_row(ui, "display", &derivation.display_unit);
                key_value_row(ui, "canonical", &derivation.canonical_unit);
                if let Some(source_unit) = &derivation.source_unit {
                    key_value_row(ui, "source unit", source_unit);
                }
                if let Some(expression) = &derivation.expression {
                    key_value_row(ui, "expression", expression);
                }
                if !derivation.steps.is_empty() {
                    key_value_row(ui, "steps", &derivation.steps.join(" -> "));
                }
            });
            ui.add_space(5.0);
        }
    }

    fn show_schema_inspector(&self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        section_label(ui, "Schemas");
        if self.schemas.is_empty() {
            ui.label(egui::RichText::new("No schema declarations").color(MUTED));
            return;
        }
        for schema in &self.schemas {
            runtime_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&schema.name).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        status_pill(ui, &format!("L{}", schema.line), MUTED);
                    });
                });
                key_value_row(ui, "columns", &schema.columns.len().to_string());
                for column in &schema.columns {
                    let mut detail = format!("{} {}", column.name, column.type_name);
                    if let Some(unit) = &column.unit {
                        detail.push_str(&format!(" [{unit}]"));
                    }
                    if column.is_index {
                        detail.push_str(" index");
                    }
                    key_value_row(ui, &format!("L{}", column.line), &detail);
                }
                if !schema.constraints.is_empty() {
                    ui.add_space(4.0);
                    key_value_row(ui, "constraints", &schema.constraints.len().to_string());
                    for constraint in &schema.constraints {
                        key_value_row(ui, &format!("L{}", constraint.line), &constraint.text);
                    }
                }
                if !schema.missing_policies.is_empty() {
                    ui.add_space(4.0);
                    key_value_row(ui, "missing", &schema.missing_policies.len().to_string());
                    for policy in &schema.missing_policies {
                        key_value_row(
                            ui,
                            &format!("L{}", policy.line),
                            &format!("{}: {}", policy.column, policy.policy),
                        );
                    }
                }
            });
            ui.add_space(5.0);
        }
    }

    fn show_csv_promotions(&self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        section_label(ui, "CSV Promotions");
        if self.csv_promotions.is_empty() {
            ui.label(egui::RichText::new("No CSV promotions").color(MUTED));
            return;
        }
        for promotion in &self.csv_promotions {
            runtime_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&promotion.binding).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        status_pill(ui, &format!("L{}", promotion.line), MUTED);
                    });
                });
                key_value_row(ui, "schema", &promotion.schema_name);
                key_value_row(ui, "rows", &promotion.row_count.to_string());
                key_value_row(ui, "source", &promotion.source_value);
                if !promotion.resolved_path.is_empty() {
                    key_value_row(ui, "resolved", &promotion.resolved_path);
                }
                key_value_row(ui, "headers", &compact_list(&promotion.headers, 8));
                if promotion.missing_columns.is_empty() {
                    key_value_row(ui, "missing", "none");
                } else {
                    key_value_row(ui, "missing", &promotion.missing_columns.join(", "));
                }
            });
            ui.add_space(5.0);
        }
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
            let items = self.completion_items_for_filter(&self.completion_filter);
            for (index, item) in items.iter().enumerate() {
                let fill = if index == 0 {
                    egui::Color32::from_rgb(222, 235, 249)
                } else {
                    PANEL_ALT
                };
                let response = egui::Frame::none()
                    .fill(fill)
                    .rounding(egui::Rounding::same(5.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&item.label).strong());
                            if index == 0 {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        status_pill(ui, "Tab", ACCENT);
                                    },
                                );
                            }
                        });
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

    fn show_runtime_inspector(&self, ui: &mut egui::Ui) {
        panel_header(ui, "Runtime Summary");
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.show_runtime_summary_content(ui);
        });
    }

    fn show_runtime_summary_content(&self, ui: &mut egui::Ui) {
        let Some(summary) = &self.artifact_summary else {
            ui.label(
                egui::RichText::new("Run the current file to inspect result artifacts.")
                    .color(MUTED),
            );
            ui.add_space(8.0);
            self.show_jit_plan(ui);
            return;
        };
        ui.horizontal_wrapped(|ui| {
            metric_chip(ui, "Status", &summary.status, OK);
            metric_chip(
                ui,
                "Uncertainty",
                &summary.uncertainties.len().to_string(),
                ACCENT,
            );
            metric_chip(ui, "ML", &summary.ml.len().to_string(), ACCENT);
            metric_chip(ui, "Policies", &summary.policy_count.to_string(), ACCENT);
            metric_chip(ui, "Systems", &summary.system_count.to_string(), ACCENT);
            if let Some(plan) = &self.jit_plan {
                metric_chip(
                    ui,
                    "Kernel Plan",
                    &plan.candidates.len().to_string(),
                    ACCENT,
                );
            }
        });
        ui.add_space(8.0);

        section_label(ui, "Uncertainty");
        if summary.uncertainties.is_empty() {
            ui.label(egui::RichText::new("No uncertainty artifacts in this run.").color(MUTED));
        }
        for item in &summary.uncertainties {
            runtime_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&item.binding).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        status_pill(ui, &item.status, OK);
                    });
                });
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {} [{}]",
                        item.kind, item.quantity_kind, item.display_unit
                    ))
                    .color(MUTED),
                );
                key_value_row(
                    ui,
                    "distribution",
                    item.distribution.as_deref().unwrap_or(""),
                );
                key_value_row(ui, "method", item.method.as_deref().unwrap_or(""));
                let transform =
                    runtime_transform_label(item.scale.as_deref(), item.offset.as_deref());
                if !transform.is_empty() {
                    key_value_row(ui, "transform", &transform);
                }
                if !item.propagation.is_empty() {
                    key_value_row(ui, "propagation", &item.propagation.join(", "));
                }
                key_value_row(
                    ui,
                    "mean",
                    &item.mean.clone().unwrap_or_else(|| "-".to_owned()),
                );
                key_value_row(
                    ui,
                    "stddev",
                    &item.stddev.clone().unwrap_or_else(|| "-".to_owned()),
                );
                key_value_row(
                    ui,
                    "p05/p50/p95",
                    &format!(
                        "{} / {} / {}",
                        item.p05.as_deref().unwrap_or("-"),
                        item.p50.as_deref().unwrap_or("-"),
                        item.p95.as_deref().unwrap_or("-")
                    ),
                );
                key_value_row(ui, "samples", &item.sample_count.to_string());
            });
            ui.add_space(6.0);
        }

        ui.add_space(8.0);
        section_label(ui, "ML Models");
        if summary.ml.is_empty() {
            ui.label(egui::RichText::new("No ML artifacts in this run.").color(MUTED));
        }
        for item in &summary.ml {
            runtime_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&item.binding).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        status_pill(ui, &item.status, OK);
                    });
                });
                ui.label(egui::RichText::new(&item.kind).color(MUTED));
                if let Some(target) = &item.target {
                    key_value_row(ui, "target", target);
                }
                if !item.features.is_empty() {
                    key_value_row(ui, "features", &item.features.join(", "));
                }
                key_value_row(
                    ui,
                    "train/test",
                    &format!(
                        "{} / {}",
                        item.train_count
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "-".to_owned()),
                        item.test_count
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "-".to_owned())
                    ),
                );
                key_value_row(
                    ui,
                    "rmse/mae/r2",
                    &format!(
                        "{} / {} / {}",
                        item.rmse.as_deref().unwrap_or("-"),
                        item.mae.as_deref().unwrap_or("-"),
                        item.r2.as_deref().unwrap_or("-")
                    ),
                );
                if let Some(leakage) = &item.leakage_status {
                    key_value_row(ui, "leakage", leakage);
                }
                if !item.coefficients.is_empty() {
                    key_value_row(ui, "coefficients", &item.coefficients.join(", "));
                }
                if let Some(loss) = &item.loss_summary {
                    key_value_row(ui, "loss", loss);
                }
            });
            ui.add_space(6.0);
        }

        ui.add_space(8.0);
        self.show_jit_plan(ui);
    }

    fn show_jit_plan(&self, ui: &mut egui::Ui) {
        section_label(ui, "Kernel Plan");
        let Some(plan) = &self.jit_plan else {
            ui.label(
                egui::RichText::new("Check the current file to inspect kernel candidates.")
                    .color(MUTED),
            );
            return;
        };

        ui.horizontal_wrapped(|ui| {
            metric_chip(ui, "Format", &plan.format, MUTED);
            metric_chip(ui, "Backend", &plan.backend, WARNING);
            metric_chip(ui, "Backend Status", &plan.backend_status, WARNING);
            metric_chip(ui, "Candidates", &plan.candidates.len().to_string(), ACCENT);
        });
        key_value_row(ui, "requested", &plan.backend_requested);
        key_value_row(ui, "selection", &plan.backend_reason);
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("Planning metadata only; execution still uses the normal runtime.")
                .color(MUTED)
                .size(12.0),
        );
        ui.add_space(6.0);

        if plan.candidates.is_empty() {
            ui.label(egui::RichText::new("No kernel candidates in this file.").color(MUTED));
            return;
        }

        for candidate in &plan.candidates {
            runtime_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&candidate.name).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        status_pill(ui, &format!("L{}", candidate.line), MUTED);
                    });
                });
                ui.horizontal_wrapped(|ui| {
                    status_pill(ui, &candidate.kind, ACCENT);
                    let status_color = if candidate.lowering_status == "interface_only" {
                        WARNING
                    } else {
                        OK
                    };
                    status_pill(ui, &candidate.lowering_status, status_color);
                });
                key_value_row(ui, "source", &candidate.source);
                key_value_row(ui, "reason", &candidate.reason);
                key_value_row(
                    ui,
                    "estimate",
                    &format!(
                        "rows={}, inputs={}, outputs={}, ops={}, scans={}",
                        candidate
                            .estimate
                            .estimated_rows
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "-".to_owned()),
                        candidate.estimate.input_count,
                        candidate.estimate.output_count,
                        candidate.estimate.operation_count,
                        candidate.estimate.scan_count
                    ),
                );
                key_value_row(ui, "complexity", &candidate.estimate.complexity);
                key_value_row(ui, "ops", &candidate.operations.join(" -> "));
                if !candidate.estimate.notes.is_empty() {
                    key_value_row(ui, "notes", &candidate.estimate.notes.join("; "));
                }
            });
            ui.add_space(6.0);
        }
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
            self.show_artifacts_content(ui);
        });
    }

    fn show_artifacts_content(&self, ui: &mut egui::Ui) {
        if let Some(output) = &self.last_output {
            if let Some(summary) = &self.artifact_summary {
                ui.horizontal_wrapped(|ui| {
                    metric_chip(ui, "Run", &summary.status, OK);
                    metric_chip(
                        ui,
                        "Uncertainty",
                        &summary.uncertainties.len().to_string(),
                        ACCENT,
                    );
                    metric_chip(ui, "ML", &summary.ml.len().to_string(), ACCENT);
                    metric_chip(ui, "Systems", &summary.system_count.to_string(), ACCENT);
                });
                ui.add_space(8.0);
            }
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
    }

    fn show_workspace(&mut self, ui: &mut egui::Ui) {
        if !self.show_preview {
            self.show_editor(ui);
            return;
        }

        let available = ui.available_size_before_wrap();
        if available.x < 520.0 {
            self.show_editor(ui);
            return;
        }

        let gap = 8.0;
        let usable_width = (available.x - SPLITTER_WIDTH - gap).max(1.0);
        let min_result = RESULT_MIN_WIDTH.min((usable_width * 0.45).max(220.0));
        let min_code = CODE_MIN_WIDTH.min((usable_width - min_result).max(260.0));
        let max_result = (usable_width - min_code).max(min_result);
        self.result_width = self.result_width.clamp(min_result, max_result);
        let code_width = (usable_width - self.result_width).max(120.0);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            ui.allocate_ui_with_layout(
                egui::vec2(code_width, available.y),
                egui::Layout::top_down(egui::Align::Min),
                |ui| self.show_editor(ui),
            );
            ui.add_space(gap * 0.5);
            let (splitter_rect, splitter_response) = ui.allocate_exact_size(
                egui::vec2(SPLITTER_WIDTH, available.y),
                egui::Sense::click_and_drag(),
            );
            if splitter_response.dragged() {
                let delta_x = ui.input(|input| input.pointer.delta().x);
                self.result_width = (self.result_width - delta_x).clamp(min_result, max_result);
            }
            let splitter_color = if splitter_response.hovered() || splitter_response.dragged() {
                ACCENT
            } else {
                BORDER
            };
            ui.painter().rect_filled(
                splitter_rect.shrink2(egui::vec2(2.5, 0.0)),
                egui::Rounding::same(3.0),
                splitter_color,
            );
            ui.add_space(gap * 0.5);
            ui.allocate_ui_with_layout(
                egui::vec2(self.result_width, available.y),
                egui::Layout::top_down(egui::Align::Min),
                |ui| self.show_result_panel(ui),
            );
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
            .default_height(190.0)
            .frame(panel_frame())
            .show(ctx, |ui| self.show_bottom_panel(ui));

        if self.show_explorer {
            egui::SidePanel::left("explorer")
                .resizable(true)
                .default_width(230.0)
                .width_range(180.0..=420.0)
                .frame(panel_frame())
                .show(ctx, |ui| self.show_explorer(ui));
        }

        if self.show_inspector_panel {
            egui::SidePanel::right("inspector")
                .resizable(true)
                .default_width(320.0)
                .width_range(260.0..=520.0)
                .frame(panel_frame())
                .show(ctx, |ui| self.show_right_panel(ui));
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(BG)
                    .inner_margin(egui::Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                self.show_workspace(ui);
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
    canonical_unit: String,
    dimension: String,
    source: String,
    source_unit: Option<String>,
    expression: Option<String>,
    steps: Vec<String>,
    detail: String,
}

#[derive(Clone)]
struct UnitDerivationView {
    name: String,
    line: usize,
    quantity_kind: String,
    source_unit: Option<String>,
    display_unit: String,
    canonical_unit: String,
    expression: Option<String>,
    steps: Vec<String>,
}

#[derive(Clone)]
struct SchemaView {
    name: String,
    line: usize,
    columns: Vec<SchemaColumnView>,
    constraints: Vec<TextLineView>,
    missing_policies: Vec<MissingPolicyView>,
}

#[derive(Clone)]
struct SchemaColumnView {
    name: String,
    type_name: String,
    unit: Option<String>,
    is_index: bool,
    line: usize,
}

#[derive(Clone)]
struct TextLineView {
    text: String,
    line: usize,
}

#[derive(Clone)]
struct MissingPolicyView {
    column: String,
    policy: String,
    line: usize,
}

#[derive(Clone)]
struct CsvPromotionView {
    binding: String,
    schema_name: String,
    source_value: String,
    resolved_path: String,
    row_count: usize,
    headers: Vec<String>,
    missing_columns: Vec<String>,
    line: usize,
}

#[derive(Clone)]
struct DomainView {
    name: String,
    type_parameters: Vec<DomainParameterView>,
    package: Option<String>,
    version: Option<String>,
    line: usize,
    variables: Vec<DomainVariableView>,
    conservations: Vec<DomainConservationView>,
}

#[derive(Clone)]
struct DomainParameterView {
    kind: String,
    name: String,
    display: String,
}

#[derive(Clone)]
struct DomainVariableView {
    role: String,
    name: String,
    quantity_kind: String,
    display_unit: String,
    canonical_unit: String,
    dimension: String,
    line: usize,
}

#[derive(Clone)]
struct DomainConservationView {
    text: String,
    status: String,
    line: usize,
}

#[derive(Clone)]
struct ComponentView {
    name: String,
    line: usize,
    ports: Vec<PortView>,
}

#[derive(Clone)]
struct PortView {
    name: String,
    domain: String,
    domain_name: String,
    type_arguments: Vec<String>,
    status: String,
    line: usize,
}

#[derive(Clone)]
struct ConnectionView {
    left: String,
    right: String,
    domain: String,
    status: String,
    line: usize,
}

#[derive(Clone)]
struct JitPlanView {
    format: String,
    backend: String,
    backend_requested: String,
    backend_status: String,
    backend_reason: String,
    candidates: Vec<JitCandidateView>,
}

impl JitPlanView {
    fn from_plan(plan: &eng_jit::NumericKernelPlan) -> Self {
        Self {
            format: plan.format.clone(),
            backend: plan.backend.clone(),
            backend_requested: plan.backend_selection.requested.clone(),
            backend_status: plan.backend_selection.status.clone(),
            backend_reason: plan.backend_selection.reason.clone(),
            candidates: plan
                .candidates
                .iter()
                .map(JitCandidateView::from_candidate)
                .collect(),
        }
    }
}

#[derive(Clone)]
struct JitCandidateView {
    name: String,
    kind: String,
    line: usize,
    source: String,
    reason: String,
    lowering_status: String,
    operations: Vec<String>,
    estimate: JitEstimateView,
}

impl JitCandidateView {
    fn from_candidate(candidate: &eng_jit::KernelCandidate) -> Self {
        Self {
            name: candidate.name.clone(),
            kind: candidate.kind.clone(),
            line: candidate.line,
            source: candidate.source.clone(),
            reason: candidate.reason.clone(),
            lowering_status: candidate.lowering_status.clone(),
            operations: candidate.operations.clone(),
            estimate: JitEstimateView::from_estimate(&candidate.estimate),
        }
    }
}

#[derive(Clone)]
struct JitEstimateView {
    estimated_rows: Option<usize>,
    input_count: usize,
    output_count: usize,
    operation_count: usize,
    scan_count: usize,
    complexity: String,
    notes: Vec<String>,
}

impl JitEstimateView {
    fn from_estimate(estimate: &eng_jit::KernelEstimate) -> Self {
        Self {
            estimated_rows: estimate.estimated_rows,
            input_count: estimate.input_count,
            output_count: estimate.output_count,
            operation_count: estimate.operation_count,
            scan_count: estimate.scan_count,
            complexity: estimate.complexity.clone(),
            notes: estimate.notes.clone(),
        }
    }
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

struct ArtifactSummary {
    status: String,
    uncertainties: Vec<UncertaintyArtifactView>,
    ml: Vec<MlArtifactView>,
    policy_count: usize,
    system_count: usize,
}

impl ArtifactSummary {
    fn from_result(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
        let value: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
        let payload = value
            .get("typed_payload")
            .ok_or_else(|| "missing typed_payload".to_owned())?;
        let status = json_string(payload, &["status"]).unwrap_or_else(|| "unknown".to_owned());
        let uncertainties = payload
            .get("uncertainties")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(UncertaintyArtifactView::from_json)
                    .collect()
            })
            .unwrap_or_default();
        let ml = payload
            .get("ml")
            .and_then(Value::as_array)
            .map(|items| items.iter().map(MlArtifactView::from_json).collect())
            .unwrap_or_default();
        let policy_count = payload
            .get("policy_results")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        let system_count = payload
            .get("systems")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        Ok(Self {
            status,
            uncertainties,
            ml,
            policy_count,
            system_count,
        })
    }
}

struct UncertaintyArtifactView {
    binding: String,
    kind: String,
    quantity_kind: String,
    display_unit: String,
    distribution: Option<String>,
    method: Option<String>,
    scale: Option<String>,
    offset: Option<String>,
    mean: Option<String>,
    stddev: Option<String>,
    p05: Option<String>,
    p50: Option<String>,
    p95: Option<String>,
    sample_count: usize,
    propagation: Vec<String>,
    status: String,
}

impl UncertaintyArtifactView {
    fn from_json(value: &Value) -> Self {
        let propagation = value
            .get("propagation")
            .and_then(Value::as_array)
            .map(|terms| {
                terms
                    .iter()
                    .filter_map(|term| {
                        let source = term.get("source")?.as_str()?;
                        let role = term.get("role")?.as_str()?;
                        let quantity_kind = term.get("quantity_kind")?.as_str()?;
                        Some(format!("{source}:{role}[{quantity_kind}]"))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            binding: json_field_string(value, "binding").unwrap_or_else(|| "unknown".to_owned()),
            kind: json_field_string(value, "kind").unwrap_or_else(|| "Uncertainty".to_owned()),
            quantity_kind: json_field_string(value, "quantity_kind").unwrap_or_default(),
            display_unit: json_field_string(value, "display_unit").unwrap_or_default(),
            distribution: json_field_string(value, "distribution"),
            method: json_field_string(value, "method"),
            scale: json_field_string(value, "scale"),
            offset: json_field_string(value, "offset"),
            mean: json_field_string(value, "mean"),
            stddev: json_field_string(value, "stddev"),
            p05: json_field_string(value, "p05"),
            p50: json_field_string(value, "p50"),
            p95: json_field_string(value, "p95"),
            sample_count: json_field_usize(value, "sample_count").unwrap_or(0),
            propagation,
            status: json_field_string(value, "status").unwrap_or_else(|| "unknown".to_owned()),
        }
    }
}

struct MlArtifactView {
    binding: String,
    kind: String,
    target: Option<String>,
    features: Vec<String>,
    train_count: Option<usize>,
    test_count: Option<usize>,
    rmse: Option<String>,
    mae: Option<String>,
    r2: Option<String>,
    leakage_status: Option<String>,
    coefficients: Vec<String>,
    loss_summary: Option<String>,
    status: String,
}

impl MlArtifactView {
    fn from_json(value: &Value) -> Self {
        let loss_values = value
            .get("loss_history")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_f64).collect::<Vec<f64>>())
            .unwrap_or_default();
        let loss_summary = match (loss_values.first(), loss_values.last()) {
            (Some(first), Some(last)) => Some(format!(
                "{} -> {}",
                format_json_number(*first),
                format_json_number(*last)
            )),
            _ => None,
        };
        let coefficients = value
            .get("coefficients")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        let feature = json_field_string(item, "feature")?;
                        let value = item.get("value")?.as_f64()?;
                        Some(format!("{feature}={}", format_json_number(value)))
                    })
                    .collect()
            })
            .unwrap_or_default();

        Self {
            binding: json_field_string(value, "binding").unwrap_or_else(|| "unknown".to_owned()),
            kind: json_field_string(value, "kind").unwrap_or_else(|| "ML".to_owned()),
            target: json_field_string(value, "target"),
            features: json_field_string_array(value, "features"),
            train_count: json_field_usize(value, "train_count"),
            test_count: json_field_usize(value, "test_count"),
            rmse: json_field_string(value, "rmse"),
            mae: json_field_string(value, "mae"),
            r2: json_field_string(value, "r2"),
            leakage_status: json_field_string(value, "leakage_status"),
            coefficients,
            loss_summary,
            status: json_field_string(value, "status").unwrap_or_else(|| "unknown".to_owned()),
        }
    }
}

struct PlotPreview {
    title: String,
    plot_type: String,
    x_label: String,
    y_label: String,
    series_name: String,
    bins: Vec<PlotBinPreview>,
    points: Vec<(f64, f64)>,
}

#[derive(Clone, Copy)]
struct PlotBinPreview {
    lower: f64,
    upper: f64,
    center: f64,
    count: f64,
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
        let bins = series
            .and_then(|item| item.get("bins"))
            .and_then(Value::as_array)
            .map(|bins| {
                bins.iter()
                    .filter_map(|bin| {
                        Some(PlotBinPreview {
                            lower: bin.get("lower")?.as_f64()?,
                            upper: bin.get("upper")?.as_f64()?,
                            center: bin.get("center")?.as_f64()?,
                            count: bin.get("count")?.as_f64()?,
                        })
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
            bins,
            points,
        })
    }
}

fn completion_items(filter: &str, symbols: &[SymbolView], source: &str) -> Vec<CompletionItem> {
    let normalized = filter.trim().to_ascii_lowercase();
    let mut items = Vec::new();
    let mut seen = HashSet::new();

    for symbol in symbols {
        push_completion(
            &mut items,
            &mut seen,
            symbol.name.clone(),
            symbol.name.clone(),
            format!(
                "variable, line {}, {} [{}]",
                symbol.line, symbol.quantity_kind, symbol.display_unit
            ),
        );
    }

    let units = all_unit_infos();
    let unit_symbols: HashSet<String> = units
        .iter()
        .map(|unit| unit.symbol.to_ascii_lowercase())
        .collect();
    for identifier in source_identifiers(source) {
        if unit_symbols.contains(&identifier.to_ascii_lowercase()) {
            continue;
        }
        push_completion(
            &mut items,
            &mut seen,
            identifier.clone(),
            identifier,
            "source identifier".to_owned(),
        );
    }

    for keyword in [
        "schema",
        "script",
        "struct",
        "system",
        "domain",
        "across",
        "through",
        "conservation",
        "component",
        "port",
        "connect",
        "package",
        "version",
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
        "measured",
        "interval",
        "normal",
        "uniform",
        "ensemble",
        "propagate",
        "samples",
        "sigma",
        "scale",
        "offset",
    ] {
        push_completion(
            &mut items,
            &mut seen,
            keyword.to_owned(),
            keyword.to_owned(),
            "language keyword".to_owned(),
        );
    }

    for quantity in all_quantity_completions() {
        push_completion(
            &mut items,
            &mut seen,
            quantity.quantity_kind.to_owned(),
            quantity.quantity_kind.to_owned(),
            format!(
                "quantity, canonical unit {}, {}",
                quantity.canonical_unit, quantity.description
            ),
        );
    }

    for unit in units {
        push_completion(
            &mut items,
            &mut seen,
            unit.symbol.to_owned(),
            unit.symbol.to_owned(),
            format!(
                "unit for {}, canonical {}",
                unit.quantity_hint, unit.canonical_unit
            ),
        );
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
            "snippet: domain ports",
            "domain Thermal package \"eng.std.domains.thermal\" version \"0.1.0\" {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ndomain Fluid[Medium M] package \"eng.std.domains.fluid\" version \"0.1.0\" {\n    across height: Length [m]\n    through m_dot: MassFlowRate [kg/s]\n    conservation sum(m_dot) = 0\n}\n\ndomain MechanicalNode[Frame F, Axis DOF] package \"eng.std.domains.mechanical\" version \"0.1.0\" {\n    across x: Length [m]\n    through P: MechanicalPower [W]\n    conservation sum(P) = 0\n}\n\ncomponent RoomBoundary {\n    port heat: Thermal\n}\n\ncomponent SupplyPipe {\n    port inlet: Fluid[Water]\n    port outlet: Fluid[Water]\n}\n\ncomponent ShaftA {\n    port shaft: MechanicalNode[World, X]\n}\n\ncomponent ShaftB {\n    port shaft: MechanicalNode[World, X]\n}\n\nconnect SupplyPipe.inlet -> SupplyPipe.outlet\nconnect ShaftA.shaft -> ShaftB.shaft",
            "domain package/version, generic ports, and connection",
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
        (
            "snippet: uncertainty",
            "Q_dist = normal(mean=5 kW, std=0.8 kW, samples=31)\nQ_total = propagate(Q_dist, method=linear, scale=1.08, offset=0.4 kW)\n\nreturn report {\n    show Q_total\n    plot distribution(Q_dist) {\n        title = \"Uncertainty histogram\"\n    }\n}",
            "uncertainty distribution and histogram",
        ),
    ] {
        push_completion(
            &mut items,
            &mut seen,
            label.to_owned(),
            insert.to_owned(),
            detail.to_owned(),
        );
    }

    if normalized.is_empty() {
        return items.into_iter().take(120).collect();
    }

    let mut starts = Vec::new();
    let mut contains = Vec::new();
    for item in items {
        let label = item.label.to_ascii_lowercase();
        let insert = item.insert.to_ascii_lowercase();
        if label.starts_with(&normalized) || insert.starts_with(&normalized) {
            starts.push(item);
        } else if label.contains(&normalized) || insert.contains(&normalized) {
            contains.push(item);
        }
    }
    starts.extend(contains);
    starts.into_iter().take(120).collect()
}

fn push_completion(
    items: &mut Vec<CompletionItem>,
    seen: &mut HashSet<String>,
    label: String,
    insert: String,
    detail: String,
) {
    let key = insert.to_ascii_lowercase();
    if seen.insert(key) {
        items.push(CompletionItem {
            label,
            insert,
            detail,
        });
    }
}

fn source_identifiers(source: &str) -> Vec<String> {
    let mut identifiers = Vec::new();
    let mut seen = HashSet::new();
    let chars: Vec<char> = source.chars().collect();
    let mut index = 0usize;
    while index < chars.len() {
        if chars[index].is_ascii_alphabetic() || chars[index] == '_' {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric() || chars[index] == '_')
            {
                index += 1;
            }
            let token: String = chars[start..index].iter().collect();
            if !is_keyword(&token) && token.len() > 1 && seen.insert(token.to_ascii_lowercase()) {
                identifiers.push(token);
            }
        } else {
            index += 1;
        }
    }
    identifiers
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
        font_id: egui::FontId::monospace(13.5),
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
            | "domain"
            | "across"
            | "through"
            | "conservation"
            | "component"
            | "port"
            | "connect"
            | "package"
            | "version"
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
            | "measured"
            | "interval"
            | "normal"
            | "uniform"
            | "ensemble"
            | "propagate"
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

fn estimated_code_width(source: &str) -> f32 {
    let longest_line = source
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    if longest_line <= 92 {
        0.0
    } else {
        (longest_line as f32 * 7.4 + 36.0).min(1800.0)
    }
}

fn char_to_byte_index(source: &str, char_index: usize) -> usize {
    source
        .char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or_else(|| source.len())
}

fn matching_closer(opener: char) -> Option<char> {
    match opener {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' => Some('"'),
        '\'' => Some('\''),
        _ => None,
    }
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
    sort_example_paths(&mut examples);
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
    entries.sort_by_key(|path| explorer_entry_sort_key(path));
    entries
}

fn sort_example_paths(paths: &mut [PathBuf]) {
    paths.sort_by_key(|path| example_file_sort_key(path));
}

fn example_file_sort_key(path: &Path) -> (u8, String) {
    (example_category_rank(path), normalized_path(path))
}

fn explorer_entry_sort_key(path: &Path) -> (u8, u8, String) {
    let kind_rank = if path.is_dir() { 0 } else { 1 };
    (
        kind_rank,
        example_category_rank(path),
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase(),
    )
}

fn example_category_rank(path: &Path) -> u8 {
    let normalized = normalized_path(path);
    if path_matches_segment(&normalized, "examples/official") {
        0
    } else if path_matches_segment(&normalized, "examples/05_error_messages") {
        2
    } else if path_matches_segment(&normalized, "examples/07_data_quality") {
        3
    } else if path_matches_segment(&normalized, "examples/scratch") {
        4
    } else if path_matches_segment(&normalized, "examples") {
        1
    } else {
        5
    }
}

fn example_category_label(path: &Path) -> Option<&'static str> {
    let normalized = normalized_path(path);
    if path_matches_segment(&normalized, "examples/official") {
        Some("Official")
    } else if path_matches_segment(&normalized, "examples/05_error_messages") {
        Some("Diagnostic")
    } else if path_matches_segment(&normalized, "examples/07_data_quality") {
        Some("Data")
    } else if path_matches_segment(&normalized, "examples/scratch") {
        Some("Scratch")
    } else if path_matches_segment(&normalized, "examples") {
        Some("Regression")
    } else {
        None
    }
}

fn explorer_directory_label(root: &Path, path: &Path) -> Option<String> {
    let normalized = relative_to(root, path);
    match normalized.as_str() {
        "examples" => Some("Examples".to_owned()),
        "examples/official" => Some("Official Examples".to_owned()),
        "examples/05_error_messages" => Some("Diagnostic Fixtures".to_owned()),
        "examples/07_data_quality" => Some("Data Quality Fixtures".to_owned()),
        "examples/scratch" => Some("Scratch Files".to_owned()),
        value if value.starts_with("examples/") && path.is_dir() => path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| format!("Regression {name}")),
        _ => None,
    }
}

fn is_official_examples_dir(root: &Path, path: &Path) -> bool {
    relative_to(root, path) == "examples/official"
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase()
}

fn path_matches_segment(normalized_path: &str, segment: &str) -> bool {
    normalized_path == segment
        || normalized_path.starts_with(&format!("{segment}/"))
        || normalized_path.ends_with(&format!("/{segment}"))
        || normalized_path.contains(&format!("/{segment}/"))
}

fn compact_list(values: &[String], limit: usize) -> String {
    if values.is_empty() {
        return "-".to_owned();
    }
    if values.len() <= limit {
        return values.join(", ");
    }
    let mut head = values
        .iter()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    head.push_str(&format!(" ... +{}", values.len() - limit));
    head
}

fn domain_parameter_list(parameters: &[DomainParameterView]) -> String {
    if parameters.is_empty() {
        "-".to_owned()
    } else {
        parameters
            .iter()
            .map(|parameter| {
                if parameter.kind == parameter.name {
                    parameter.display.clone()
                } else {
                    format!(
                        "{} ({} {})",
                        parameter.display, parameter.kind, parameter.name
                    )
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn runtime_transform_label(scale: Option<&str>, offset: Option<&str>) -> String {
    match (scale, offset) {
        (Some(scale), Some(offset)) => format!("scale={scale}, offset={offset}"),
        (Some(scale), None) => format!("scale={scale}"),
        (None, Some(offset)) => format!("offset={offset}"),
        (None, None) => String::new(),
    }
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
        .inner_margin(egui::Margin::same(8.0))
}

fn panel_header(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(14.0).strong().color(TEXT));
    ui.add_space(2.0);
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

fn compact_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add_sized(
        [76.0, 23.0],
        egui::Button::new(egui::RichText::new(text).size(12.0)),
    )
}

fn completion_hint(ui: &mut egui::Ui, prefix: &str, item: &CompletionItem) {
    let display_insert = if item.insert.chars().count() > 48 {
        item.label.as_str()
    } else {
        item.insert.as_str()
    };
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(245, 249, 255))
        .stroke(egui::Stroke::new(
            1.0,
            egui::Color32::from_rgb(191, 219, 254),
        ))
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::symmetric(8.0, 5.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                status_pill(ui, "Tab", ACCENT);
                ui.label(egui::RichText::new("complete").color(MUTED).size(12.0));
                ui.label(egui::RichText::new(prefix).monospace().size(12.0));
                ui.label(egui::RichText::new("to").color(MUTED).size(12.0));
                ui.label(
                    egui::RichText::new(display_insert)
                        .monospace()
                        .strong()
                        .size(12.0),
                );
                ui.label(egui::RichText::new(&item.detail).color(MUTED).size(12.0));
            });
        });
}

fn tab_button(ui: &mut egui::Ui, text: &str, selected: bool) -> egui::Response {
    let fill = if selected { ACCENT } else { PANEL_ALT };
    let color = if selected { egui::Color32::WHITE } else { TEXT };
    ui.add(
        egui::Button::new(egui::RichText::new(text).color(color).size(12.0))
            .fill(fill)
            .min_size(egui::vec2(0.0, 23.0)),
    )
}

fn status_badge(ui: &mut egui::Ui, label: &str, count: usize, color: egui::Color32) {
    status_pill(ui, &format!("{label} {count}"), color);
}

fn status_pill(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    egui::Frame::none()
        .fill(color.linear_multiply(0.10))
        .stroke(egui::Stroke::new(1.0, color.linear_multiply(0.55)))
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::symmetric(7.0, 3.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).color(color).size(11.5));
        });
}

fn status_color(status: &str) -> egui::Color32 {
    match status {
        "recorded" | "domain_resolved" | "domain_compatible" | "unit_consistent" => OK,
        "unknown_domain"
        | "generic_arity_mismatch"
        | "domain_mismatch"
        | "medium_mismatch"
        | "frame_mismatch"
        | "axis_mismatch"
        | "domain_parameter_mismatch"
        | "unresolved_endpoint" => ERROR,
        "unvalidated" | "metadata_only" | "deferred" | "unsolved" => WARNING,
        _ => MUTED,
    }
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(12.5).strong().color(TEXT));
    ui.add_space(3.0);
}

fn metric_chip(ui: &mut egui::Ui, label: &str, value: &str, color: egui::Color32) {
    egui::Frame::none()
        .fill(color.linear_multiply(0.08))
        .stroke(egui::Stroke::new(1.0, color.linear_multiply(0.35)))
        .rounding(egui::Rounding::same(5.0))
        .inner_margin(egui::Margin::symmetric(8.0, 5.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(label).color(MUTED).size(12.0));
                ui.label(egui::RichText::new(value).color(color).strong().size(12.0));
            });
        });
}

fn runtime_card(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> egui::InnerResponse<()> {
    egui::Frame::none()
        .fill(PANEL_ALT)
        .stroke(egui::Stroke::new(1.0, BORDER))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(9.0, 7.0))
        .show(ui, add_contents)
}

fn key_value_row(ui: &mut egui::Ui, key: &str, value: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.add_sized(
            [92.0, 18.0],
            egui::Label::new(egui::RichText::new(key).color(MUTED).size(12.0)),
        );
        ui.label(egui::RichText::new(value).monospace().size(12.0));
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
        rect.min + egui::vec2(66.0, 30.0),
        rect.max - egui::vec2(28.0, 48.0),
    );

    if plot.points.is_empty() {
        painter.line_segment(
            [plot_rect.left_bottom(), plot_rect.right_bottom()],
            egui::Stroke::new(1.4, egui::Color32::from_rgb(65, 76, 90)),
        );
        painter.line_segment(
            [plot_rect.left_bottom(), plot_rect.left_top()],
            egui::Stroke::new(1.4, egui::Color32::from_rgb(65, 76, 90)),
        );
        painter.text(
            plot_rect.center(),
            egui::Align2::CENTER_CENTER,
            "No points",
            egui::FontId::proportional(14.0),
            MUTED,
        );
    } else {
        let (min_x, max_x, min_y, max_y) = plot_bounds(plot);
        let x_ticks = tick_values(min_x, max_x, 5);
        let y_ticks = tick_values(min_y, max_y, 5);
        let to_screen = |point: (f64, f64)| -> egui::Pos2 {
            let x_t = normalized(point.0, min_x, max_x);
            let y_t = normalized(point.1, min_y, max_y);
            egui::pos2(
                plot_rect.left() + (x_t as f32) * plot_rect.width(),
                plot_rect.bottom() - (y_t as f32) * plot_rect.height(),
            )
        };

        for tick in &x_ticks {
            let x = to_screen((*tick, min_y)).x;
            painter.line_segment(
                [
                    egui::pos2(x, plot_rect.top()),
                    egui::pos2(x, plot_rect.bottom()),
                ],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(226, 232, 240)),
            );
            painter.text(
                egui::pos2(x, plot_rect.bottom() + 7.0),
                egui::Align2::CENTER_TOP,
                format_tick(*tick),
                egui::FontId::proportional(10.5),
                MUTED,
            );
        }
        for tick in &y_ticks {
            let y = to_screen((min_x, *tick)).y;
            painter.line_segment(
                [
                    egui::pos2(plot_rect.left(), y),
                    egui::pos2(plot_rect.right(), y),
                ],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(226, 232, 240)),
            );
            painter.text(
                egui::pos2(plot_rect.left() - 8.0, y),
                egui::Align2::RIGHT_CENTER,
                format_tick(*tick),
                egui::FontId::proportional(10.5),
                MUTED,
            );
        }

        painter.line_segment(
            [plot_rect.left_bottom(), plot_rect.right_bottom()],
            egui::Stroke::new(1.5, egui::Color32::from_rgb(65, 76, 90)),
        );
        painter.line_segment(
            [plot_rect.left_bottom(), plot_rect.left_top()],
            egui::Stroke::new(1.5, egui::Color32::from_rgb(65, 76, 90)),
        );
        if min_y < 0.0 && max_y > 0.0 {
            let zero_y = to_screen((min_x, 0.0)).y;
            painter.line_segment(
                [
                    egui::pos2(plot_rect.left(), zero_y),
                    egui::pos2(plot_rect.right(), zero_y),
                ],
                egui::Stroke::new(1.5, egui::Color32::from_rgb(148, 163, 184)),
            );
        }

        if plot.plot_type == "histogram" && !plot.bins.is_empty() {
            let baseline_y = to_screen((min_x, 0.0)).y;
            for bin in &plot.bins {
                let left = to_screen((bin.lower, 0.0)).x;
                let right = to_screen((bin.upper, 0.0)).x;
                let value_y = to_screen((bin.center, bin.count)).y;
                let bar_rect = egui::Rect::from_min_max(
                    egui::pos2(left.min(right), value_y.min(baseline_y)),
                    egui::pos2(
                        left.max(right).max(left.min(right) + 2.0),
                        value_y.max(baseline_y),
                    ),
                );
                painter.rect_filled(bar_rect, egui::Rounding::same(1.5), ACCENT);
            }
        } else if plot.plot_type == "bar" || plot.plot_type == "histogram" {
            let bar_width = (plot_rect.width() / plot.points.len().max(1) as f32) * 0.68;
            let baseline_value = if min_y <= 0.0 && max_y >= 0.0 {
                0.0
            } else if min_y > 0.0 {
                min_y
            } else {
                max_y
            };
            let baseline_y = to_screen((min_x, baseline_value)).y;
            for point in &plot.points {
                let screen = to_screen(*point);
                let bar_rect = egui::Rect::from_center_size(
                    egui::pos2(screen.x, (screen.y + baseline_y) * 0.5),
                    egui::vec2(bar_width.max(2.0), (baseline_y - screen.y).abs().max(1.0)),
                );
                painter.rect_filled(bar_rect, egui::Rounding::same(2.0), ACCENT);
            }
        } else if plot.plot_type == "scatter" {
            for point in plot.points.iter().copied().map(to_screen) {
                painter.circle_filled(point, 3.2, egui::Color32::from_rgb(255, 255, 255));
                painter.circle_stroke(point, 3.2, egui::Stroke::new(1.5, ACCENT));
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

fn plot_bounds(plot: &PlotPreview) -> (f64, f64, f64, f64) {
    if plot.plot_type == "histogram" && !plot.bins.is_empty() {
        let min_x = plot
            .bins
            .iter()
            .map(|bin| bin.lower.min(bin.upper))
            .fold(f64::INFINITY, f64::min);
        let max_x = plot
            .bins
            .iter()
            .map(|bin| bin.lower.max(bin.upper))
            .fold(f64::NEG_INFINITY, f64::max);
        let max_y = plot.bins.iter().map(|bin| bin.count).fold(0.0, f64::max);
        return (min_x, max_x, 0.0, max_y.max(1.0));
    }
    point_bounds(&plot.points)
}

fn normalized(value: f64, min: f64, max: f64) -> f64 {
    if (max - min).abs() < f64::EPSILON {
        0.5
    } else {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    }
}

fn tick_values(min: f64, max: f64, count: usize) -> Vec<f64> {
    let count = count.max(2);
    if (max - min).abs() < f64::EPSILON {
        return vec![min];
    }
    let step = (max - min) / (count - 1) as f64;
    (0..count).map(|index| min + step * index as f64).collect()
}

fn format_tick(value: f64) -> String {
    if value.abs() >= 1000.0 {
        format!("{value:.0}")
    } else if value.abs() >= 10.0 {
        format!("{value:.1}")
    } else if value.abs() >= 1.0 {
        format!("{value:.2}")
    } else {
        format!("{value:.3}")
    }
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

fn json_field_string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
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
