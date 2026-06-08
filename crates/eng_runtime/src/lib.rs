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

use runtime_data::{
    materialize_runtime_data, RuntimeData, RuntimeStatisticValue, RuntimeTimeSeries, RuntimeValues,
};
pub use vm::{execute_bytecode, VmExecution, VmObject, VmObjectKind};

pub const RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Default)]
pub struct RunOptions {
    pub open_report: bool,
    pub save_artifacts: bool,
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
    pub csv_export_paths: Vec<PathBuf>,
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
    let bytecode_program = parse_bytecode(&bytecode)?;
    let mut execution = execute_bytecode(&bytecode_program)?;
    let runtime_data = materialize_runtime_data(&check_report, &source);
    apply_runtime_lengths(&mut execution, &runtime_data);
    let stdout = render_stdout(&check_report, &runtime_data);
    let csv_export_paths = write_csv_exports(&check_report, &runtime_data, &result_dir)?;
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
    report_spec.uncertainty = runtime_data.report_uncertainty();
    report_spec.ml = runtime_data.report_ml();
    report_spec.policy_results = runtime_data.report_policy_results();
    runtime_data.apply_system_solutions(&mut report_spec);
    let report_spec_json = eng_report::report_spec_json(&report_spec);
    let report_spec_hash = hash_text(&report_spec_json);
    let review_json = review_json(&check_report);
    let report_html = eng_report::render_html(&check_report, "plots/timeseries.svg");
    let result_json = result_json(
        path,
        &check_report,
        &execution,
        &runtime_data,
        &bytecode_hash,
        &plot_spec_hash,
        &report_spec_hash,
    );

    let artifacts_saved = options.save_artifacts || options.open_report;
    if artifacts_saved {
        fs::create_dir_all(&plots_dir)?;
        fs::write(&bytecode_path, &bytecode)?;
        fs::write(&review_path, &review_json)?;
        fs::write(&plot_spec_path, &plot_spec_json)?;
        fs::write(&plot_path, &plot_svg)?;
        fs::write(&plot_manifest_path, &plot_manifest_json)?;
        fs::write(&report_spec_path, &report_spec_json)?;
        fs::write(&report_path, &report_html)?;
        fs::write(&result_path, &result_json)?;
    }

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
        csv_export_paths,
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
            "format = engpkg-stable-1\npackage_format_version = 1\nruntime_abi = eng-runtime-cli-v1\nprofile = repro\nrunner = run.bat\nengine = eng.exe\nsource_root = source\nartifact_root = build/result\nsource = {}\nbytecode = {}\nsource_hash = {}\nbytecode_hash = {}\nentry_name = {}\nentry = {}\nargs_schema = {}\nargs_field_count = {}\nargs_help = ARGS_HELP.txt\ndependency_count = {}\ndependencies = {}\ndependency_hashes = {}\n",
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
            args_field_count(&check_report, &entry),
            bundled_dependencies.len(),
            dependency_paths(&bundled_dependencies),
            dependency_hashes(&bundled_dependencies)
        ),
    )?;
    fs::write(
        &lock_path,
        format!(
            "runtime_version = {RUNTIME_VERSION}\ncompiler_version = {}\npackage_format_version = 1\nruntime_abi = eng-runtime-cli-v1\nbytecode_version = {}\nresult_format_version = 1\nreport_schema_version = {}\nplot_spec_version = {}\nprofile = repro\nsource_hash = {}\nbytecode_hash = {}\nentry_name = {}\ndependency_count = {}\ndependency_hashes = {}\n",
            eng_compiler::COMPILER_VERSION,
            eng_compiler::BYTECODE_VERSION,
            eng_report::REPORT_SPEC_VERSION,
            eng_report::PLOT_SPEC_VERSION,
            check_report.source_hash,
            bytecode_hash,
            entry.name,
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
        "@echo off\r\nsetlocal\r\ncd /d \"%~dp0\"\r\nif \"%~1\"==\"--help\" goto help\r\nif \"%~1\"==\"-h\" goto help\r\nif \"%~1\"==\"/?\" goto help\r\n\"%~dp0eng.exe\" run \"%~dp0source\\{}\" --entry {} --save-artifacts %*\r\nexit /b %ERRORLEVEL%\r\n:help\r\ntype \"%~dp0ARGS_HELP.txt\"\r\nexit /b 0\r\n",
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
    for print in &report.semantic_program.prints {
        output.push_str(&render_print_template(print, report, runtime_data));
        output.push('\n');
    }
    output
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

fn write_csv_exports(
    report: &CheckReport,
    runtime_data: &RuntimeData,
    result_dir: &Path,
) -> Result<Vec<PathBuf>, RuntimeError> {
    let mut paths = Vec::new();
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
        fs::write(&path, csv)?;
        paths.push(path);
    }
    Ok(paths)
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
    if value.contains(|character| matches!(character, ',' | '"' | '\n' | '\r')) {
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
    if let Some(value) = evaluate_statistic_expression(expression, runtime_data) {
        return Some(value);
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
        if let Some(solution) = runtime_data
            .system_solutions
            .iter()
            .find(|solution| solution.system == system.name)
        {
            systems.push_str(",\n        \"solver_result\": ");
            push_system_solution_json(&mut systems, solution, "        ");
        }
        systems.push('\n');
        systems.push_str("      }");
    }
    let solver_boundaries = solver_boundaries_json(report, runtime_data);
    let system_ir = system_ir_json(report, runtime_data);

    format!(
        "{{\n  \"format\": \"engres-v1\",\n  \"result_format_version\": 1,\n  \"runtime_version\": \"{RUNTIME_VERSION}\",\n  \"compiler_version\": \"{}\",\n  \"bytecode_version\": {},\n  \"source_path\": \"{}\",\n  \"source_hash\": \"{}\",\n  \"bytecode_hash\": \"{}\",\n  \"numeric_profile\": \"preview-f64\",\n  \"entry\": {{\n    \"kind\": \"{}\",\n    \"name\": \"{}\",\n    \"arg_name\": \"{}\",\n    \"arg_type\": \"{}\",\n    \"return_type\": \"{}\"\n  }},\n  \"args_schema\": [\n{}\n  ],\n  \"arg_values\": [\n{}\n  ],\n  \"object_store\": {{\n    \"scalar_count\": {},\n    \"table_count\": {},\n    \"timeseries_count\": {},\n    \"array_count\": {},\n    \"objects\": [\n{}\n    ]\n  }},\n  \"typed_payload\": {{\n    \"kind\": \"{}\",\n    \"status\": \"ok\",\n    \"result_format\": \"{}\",\n    \"vm_steps\": [{}],\n    \"statistics\": [\n{}\n    ],\n    \"integrations\": [\n{}\n    ],\n    \"uncertainties\": [\n{}\n    ],\n    \"ml\": [\n{}\n    ],\n    \"policy_results\": [\n{}\n    ],\n    \"systems\": [\n{}\n    ],\n    \"solver_boundaries\": [\n{}\n    ],\n    \"system_ir\": [\n{}\n    ]\n  }},\n  \"provenance\": {{\n    \"schema_count\": {},\n    \"csv_promotion_count\": {},\n    \"system_count\": {},\n    \"equation_count\": {},\n    \"residual_count\": {},\n    \"data_hashes\": [\n{}\n    ],\n    \"unit_conversion_history\": [],\n    \"plot_spec_hash\": \"{}\",\n    \"report_spec_hash\": \"{}\",\n    \"schema_hash\": \"preview\"\n  }}\n}}\n",
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
        uncertainties,
        ml,
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
    json.push_str(&format!(
        "{indent}  \"method\": \"{}\",\n",
        json_escape(&solution.method)
    ));
    json.push_str(&format!(
        "{indent}  \"reason\": \"{}\",\n",
        json_escape(&solution.reason)
    ));
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
    json.push_str(&format!("{indent}  \"points\": ["));
    push_runtime_points(json, &solution.points);
    json.push_str("]\n");
    json.push_str(&format!("{indent}}}"));
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

fn push_optional_json_number(json: &mut String, key: &str, value: Option<f64>, indent: usize) {
    let spaces = " ".repeat(indent);
    match value {
        Some(value) => json.push_str(&format!("{spaces}\"{key}\": {value},\n")),
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
            "schema SensorData {\n    time: DateTime index\n    T_supply: AbsoluteTemperature [degC]\n    T_return: AbsoluteTemperature [degC]\n    m_dot: MassFlowRate [kg/s]\n}\n\nstruct Args {\n    input: String = \"../../examples/official/01_csv_plot/data/sensor.csv\"\n}\n\nscript main(args: Args) -> Report {\n    sensor = promote csv args.input as SensorData\n    cp = 4180 J/kg/K\n    Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\n    E_coil = integrate(Q_coil, over=Time)\n    mean_Q = mean(Q_coil, axis=Time)\n\n    print \"Loaded {sensor.rows} rows from {args.input}\"\n    print \"Q mean = {mean(Q_coil, axis=Time): .2 kW}\"\n    print \"E total = {E_coil: .2 kWh}\"\n\n    export summary to csv \"summary.csv\" {\n        E_coil as kWh with \".2\"\n        mean_Q as kW with \".2\"\n    }\n}\n",
        )
        .expect("write source");

        let output = run_file(&source_path, &build_root, &RunOptions::default()).expect("run file");

        assert!(output.stdout.contains("Loaded 4 rows"));
        assert!(output.stdout.contains("Q mean = "));
        assert!(output.stdout.contains(" kW"));
        assert!(output.stdout.contains("E total = "));
        assert_eq!(output.csv_export_paths.len(), 1);
        assert!(!output.artifacts_saved);
        let csv =
            fs::read_to_string(build_root.join("result").join("summary.csv")).expect("summary csv");
        assert!(csv.contains("E_coil [kWh]"));
        assert!(csv.contains("mean_Q [kW]"));
        assert_eq!(csv.lines().count(), 2);
    }
}
