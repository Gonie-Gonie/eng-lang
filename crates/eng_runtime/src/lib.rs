use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use eng_compiler::{
    build_bytecode, check_file, parse_bytecode, review_json, select_entry, ArgOverride,
    CheckOptions, CheckReport, EntryPoint,
};

mod runtime_data;
mod vm;

use runtime_data::{materialize_runtime_data, RuntimeData, RuntimeValues};
pub use vm::{execute_bytecode, VmExecution, VmObject, VmObjectKind};

pub const RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Default)]
pub struct RunOptions {
    pub open_report: bool,
    pub entry: Option<String>,
    pub args: Vec<ArgOverride>,
}

#[derive(Clone, Debug, Default)]
pub struct BuildOptions {
    pub entry: Option<String>,
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
        &repo_root.join("examples").join("01_units").join("main.eng"),
    ));

    DoctorReport { checks }
}

pub fn run_file(
    path: &Path,
    build_root: &Path,
    options: &RunOptions,
) -> Result<RunOutput, RuntimeError> {
    let source = fs::read_to_string(path)?;
    let check_report = check_file(
        path,
        &CheckOptions {
            review: true,
            args: options.args.clone(),
            require_args: true,
        },
    )?;
    if check_report.has_errors() {
        return Err(RuntimeError::Compile(Box::new(check_report)));
    }
    let entry = match select_entry(
        &check_report.semantic_program.entry_points,
        options.entry.as_deref(),
    ) {
        Ok(entry) => entry,
        Err(diagnostic) => {
            return Err(RuntimeError::Compile(with_diagnostic(
                check_report,
                diagnostic,
            )))
        }
    };

    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("main");
    let result_dir = build_root.join("result");
    let plots_dir = result_dir.join("plots");
    fs::create_dir_all(&plots_dir)?;

    let bytecode_path = build_root.join(format!("{stem}.engbc"));
    let result_path = result_dir.join("result.engres");
    let review_path = result_dir.join("review.json");
    let plot_path = plots_dir.join("timeseries.svg");
    let plot_spec_path = plots_dir.join("plot_spec.json");
    let plot_manifest_path = plots_dir.join("plot_manifest.json");
    let report_spec_path = result_dir.join("report_spec.json");
    let report_path = result_dir.join("report.html");

    let bytecode = build_bytecode(&check_report, &source, &entry);
    let bytecode_hash = hash_text(&bytecode);
    fs::write(&bytecode_path, &bytecode)?;
    let bytecode_program = parse_bytecode(&bytecode)?;
    let mut execution = execute_bytecode(&bytecode_program)?;
    let runtime_data = materialize_runtime_data(&check_report, &source);
    apply_runtime_lengths(&mut execution, &runtime_data);
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
    report_spec.policy_results = runtime_data.report_policy_results();
    let report_spec_json = eng_report::report_spec_json(&report_spec);
    let report_spec_hash = hash_text(&report_spec_json);
    fs::write(&review_path, review_json(&check_report))?;
    fs::write(&plot_spec_path, plot_spec_json)?;
    fs::write(&plot_path, plot_svg)?;
    fs::write(&plot_manifest_path, plot_manifest_json)?;
    fs::write(&report_spec_path, report_spec_json)?;
    fs::write(
        &report_path,
        eng_report::render_html(&check_report, "plots/timeseries.svg"),
    )?;
    fs::write(
        &result_path,
        result_json(
            path,
            &check_report,
            &execution,
            &runtime_data,
            &bytecode_hash,
            &plot_spec_hash,
            &report_spec_hash,
        ),
    )?;

    if options.open_report {
        open_path(&report_path);
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
    let entry = match select_entry(
        &check_report.semantic_program.entry_points,
        options.entry.as_deref(),
    ) {
        Ok(entry) => entry,
        Err(diagnostic) => {
            return Err(RuntimeError::Compile(with_diagnostic(
                check_report,
                diagnostic,
            )))
        }
    };
    let bytecode = build_bytecode(&check_report, &source, &entry);
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
        fs::copy(Path::new(&promotion.resolved_path), destination)?;
    }

    let executable_path = bundle_path.join("eng.exe");
    fs::copy(env::current_exe()?, &executable_path)?;

    let runner_path = bundle_path.join("run.bat");
    fs::write(
        &runner_path,
        standalone_runner_script(source_file_name, &entry.name),
    )?;
    let args_help_path = bundle_path.join("ARGS_HELP.txt");
    fs::write(args_help_path, args_help_text(&check_report, &entry))?;

    let bytecode_path = bundle_path.join(format!("{stem}.engbc"));
    let package_path = bundle_path.join(format!("{stem}.engpkg"));
    let lock_path = bundle_path.join(format!("{stem}.lock"));
    let review_path = bundle_path.join(format!("{stem}.review.html"));

    fs::write(&bytecode_path, &bytecode)?;
    fs::write(
        &package_path,
        format!(
            "format = engpkg-stable-1\npackage_format_version = 1\nrunner = run.bat\nengine = eng.exe\nsource = {}\nbytecode = {}\nsource_hash = {}\nbytecode_hash = {}\nentry_name = {}\nentry = {}\nargs_schema = {}\nargs_field_count = {}\nargs_help = ARGS_HELP.txt\n",
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
            entry.name,
            entry.signature(),
            entry.arg_type.as_deref().unwrap_or("Args"),
            args_field_count(&check_report, &entry)
        ),
    )?;
    fs::write(
        &lock_path,
        format!(
            "runtime_version = {RUNTIME_VERSION}\ncompiler_version = {}\nbytecode_version = {}\nresult_format_version = 1\nreport_schema_version = {}\nplot_spec_version = {}\nprofile = repro\n",
            eng_compiler::COMPILER_VERSION,
            eng_compiler::BYTECODE_VERSION,
            eng_report::REPORT_SPEC_VERSION,
            eng_report::PLOT_SPEC_VERSION
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

script main(args: Args) -> Report {
    sensor = promote csv "data/sensor.csv" as SensorData
    cp = 4180 J/kg/K
    Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
    E_coil = integrate(Q_coil, over=Time)

    return report {
        summarize Q_coil by [mean, max, p95]
        show E_coil
        plot Q_coil over Time
    }
}
"#,
    )?;
    fs::write(
        path.join("data").join("sensor.csv"),
        "time,T_supply,T_return,m_dot\n2026-01-01T00:00:00Z,7.0,12.0,0.21\n",
    )?;
    Ok(())
}

fn with_diagnostic(
    mut report: CheckReport,
    diagnostic: eng_compiler::Diagnostic,
) -> Box<CheckReport> {
    report.diagnostics.push(diagnostic);
    Box::new(report)
}

fn file_check(name: &'static str, path: &Path) -> DoctorCheck {
    DoctorCheck {
        name,
        ok: path.exists(),
        detail: path.display().to_string(),
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

fn standalone_runner_script(source_file_name: &str, entry_name: &str) -> String {
    format!(
        "@echo off\r\nsetlocal\r\ncd /d \"%~dp0\"\r\nif \"%~1\"==\"--help\" goto help\r\nif \"%~1\"==\"-h\" goto help\r\nif \"%~1\"==\"/?\" goto help\r\n\"%~dp0eng.exe\" run \"%~dp0source\\{}\" --entry {} %*\r\nexit /b %ERRORLEVEL%\r\n:help\r\ntype \"%~dp0ARGS_HELP.txt\"\r\nexit /b 0\r\n",
        source_file_name, entry_name
    )
}

fn args_field_count(report: &CheckReport, entry: &EntryPoint) -> usize {
    let arg_type = entry.arg_type.as_deref().unwrap_or("Args");
    report
        .semantic_program
        .args_structs
        .iter()
        .find(|args_struct| args_struct.name == arg_type)
        .map(|args_struct| args_struct.fields.len())
        .unwrap_or(0)
}

fn args_help_text(report: &CheckReport, entry: &EntryPoint) -> String {
    let arg_type = entry.arg_type.as_deref().unwrap_or("Args");
    let mut text = String::new();
    text.push_str("EngLang standalone package\n\n");
    text.push_str("Entry:\n");
    text.push_str(&format!("  {}\n\n", entry.signature()));
    text.push_str("Args metadata:\n");

    match report
        .semantic_program
        .args_structs
        .iter()
        .find(|args_struct| args_struct.name == arg_type)
    {
        Some(args_struct) if args_struct.fields.is_empty() => {
            text.push_str(&format!("  struct {} has no fields.\n", args_struct.name));
        }
        Some(args_struct) => {
            text.push_str(&format!("  struct {}\n", args_struct.name));
            for field in &args_struct.fields {
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
                "  struct {arg_type} is not declared in this source.\n"
            ));
        }
    }

    text.push_str("\nFlags are forwarded to eng.exe run and recorded in arg_values.\n");
    text
}

fn path_for_manifest(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
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

fn result_json(
    path: &Path,
    report: &CheckReport,
    execution: &VmExecution,
    runtime_data: &RuntimeData,
    bytecode_hash: &str,
    plot_spec_hash: &str,
    report_spec_hash: &str,
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
    for (args_index, args_struct) in report.semantic_program.args_structs.iter().enumerate() {
        if args_index > 0 {
            args_schema.push_str(",\n");
        }
        args_schema.push_str("    {\n");
        args_schema.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&args_struct.name)
        ));
        args_schema.push_str(&format!("      \"line\": {},\n", args_struct.line));
        args_schema.push_str(&format!(
            "      \"field_count\": {},\n",
            args_struct.fields.len()
        ));
        args_schema.push_str("      \"fields\": [\n");
        for (field_index, field) in args_struct.fields.iter().enumerate() {
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
        systems.push_str("\n        ]\n");
        systems.push_str("      }");
    }
    let solver_boundaries = solver_boundaries_json(report);
    let system_ir = system_ir_json(report);

    format!(
        "{{\n  \"format\": \"engres-v1\",\n  \"result_format_version\": 1,\n  \"runtime_version\": \"{RUNTIME_VERSION}\",\n  \"compiler_version\": \"{}\",\n  \"bytecode_version\": {},\n  \"source_path\": \"{}\",\n  \"source_hash\": \"{}\",\n  \"bytecode_hash\": \"{}\",\n  \"numeric_profile\": \"preview-f64\",\n  \"entry\": {{\n    \"kind\": \"{}\",\n    \"name\": \"{}\",\n    \"arg_name\": \"{}\",\n    \"arg_type\": \"{}\",\n    \"return_type\": \"{}\"\n  }},\n  \"args_schema\": [\n{}\n  ],\n  \"arg_values\": [\n{}\n  ],\n  \"object_store\": {{\n    \"scalar_count\": {},\n    \"table_count\": {},\n    \"timeseries_count\": {},\n    \"array_count\": {},\n    \"objects\": [\n{}\n    ]\n  }},\n  \"typed_payload\": {{\n    \"kind\": \"{}\",\n    \"status\": \"ok\",\n    \"result_format\": \"{}\",\n    \"vm_steps\": [{}],\n    \"statistics\": [\n{}\n    ],\n    \"integrations\": [\n{}\n    ],\n    \"policy_results\": [\n{}\n    ],\n    \"systems\": [\n{}\n    ],\n    \"solver_boundaries\": [\n{}\n    ],\n    \"system_ir\": [\n{}\n    ]\n  }},\n  \"provenance\": {{\n    \"schema_count\": {},\n    \"csv_promotion_count\": {},\n    \"system_count\": {},\n    \"equation_count\": {},\n    \"residual_count\": {},\n    \"data_hashes\": [\n{}\n    ],\n    \"unit_conversion_history\": [],\n    \"plot_spec_hash\": \"{}\",\n    \"report_spec_hash\": \"{}\",\n    \"schema_hash\": \"preview\"\n  }}\n}}\n",
        eng_compiler::COMPILER_VERSION,
        eng_compiler::BYTECODE_VERSION,
        json_escape(&path.display().to_string()),
        report.source_hash,
        bytecode_hash,
        json_escape(&execution.entry.kind),
        json_escape(&execution.entry.name),
        json_escape(execution.entry.arg_name.as_deref().unwrap_or("args")),
        json_escape(execution.entry.arg_type.as_deref().unwrap_or("Args")),
        json_escape(execution.entry.return_type.as_deref().unwrap_or("Report")),
        args_schema,
        arg_values,
        execution.scalar_count(),
        execution.table_count(),
        execution.timeseries_count(),
        execution.array_count(),
        objects,
        json_escape(execution.entry.return_type.as_deref().unwrap_or("Report")),
        json_escape(&execution.result_format),
        steps,
        statistics,
        integrations,
        policy_results,
        systems,
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
        data_hashes,
        plot_spec_hash,
        report_spec_hash
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

fn solver_boundaries_json(report: &CheckReport) -> String {
    let mut json = String::new();
    for (index, system) in report.semantic_program.systems.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"system\": \"{}\",\n",
            json_escape(&system.name)
        ));
        json.push_str("        \"status\": \"unsolved\",\n");
        json.push_str(
            "        \"reason\": \"numeric solver deferred until the solver milestone\",\n",
        );
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

fn system_ir_json(report: &CheckReport) -> String {
    let mut json = String::new();
    for (index, system) in report.semantic_program.systems.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&system.name)
        ));
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

fn role_count(system: &eng_compiler::SystemInfo, role: &str) -> usize {
    system
        .variables
        .iter()
        .filter(|variable| variable.role == role)
        .count()
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
        json.push_str("]\n");
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
    let mut hash = 0xcbf29ce484222325u64;
    for byte in source.as_bytes() {
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
