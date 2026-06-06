use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use eng_compiler::{build_bytecode, check_file, review_json, CheckOptions, CheckReport};

pub const RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Default)]
pub struct RunOptions {
    pub open_report: bool,
}

#[derive(Clone, Debug)]
pub struct RunOutput {
    pub bytecode_path: PathBuf,
    pub result_path: PathBuf,
    pub review_path: PathBuf,
    pub report_path: PathBuf,
    pub plot_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct BuildOutput {
    pub executable_path: PathBuf,
    pub package_path: PathBuf,
    pub lock_path: PathBuf,
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
        }
    }
}

impl Error for RuntimeError {}

impl From<std::io::Error> for RuntimeError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
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
    let check_report = check_file(path, &CheckOptions { review: true })?;
    if check_report.has_errors() {
        return Err(RuntimeError::Compile(Box::new(check_report)));
    }

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
    let report_path = result_dir.join("report.html");

    fs::write(&bytecode_path, build_bytecode(&check_report, &source))?;
    fs::write(&review_path, review_json(&check_report))?;
    fs::write(&plot_path, eng_report::render_svg("EngLang preview plot"))?;
    fs::write(
        &report_path,
        eng_report::render_html(&check_report, "plots/timeseries.svg"),
    )?;
    fs::write(&result_path, result_json(path, &check_report))?;

    if options.open_report {
        open_path(&report_path);
    }

    Ok(RunOutput {
        bytecode_path,
        result_path,
        review_path,
        report_path,
        plot_path,
    })
}

pub fn build_standalone(path: &Path, dist_root: &Path) -> Result<BuildOutput, RuntimeError> {
    let source = fs::read_to_string(path)?;
    let check_report = check_file(path, &CheckOptions { review: true })?;
    if check_report.has_errors() {
        return Err(RuntimeError::Compile(Box::new(check_report)));
    }

    fs::create_dir_all(dist_root)?;
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("model");
    let executable_path = dist_root.join(format!("{stem}.exe"));
    let package_path = dist_root.join(format!("{stem}.engpkg"));
    let lock_path = dist_root.join(format!("{stem}.lock"));
    let review_path = dist_root.join(format!("{stem}.review.html"));

    fs::write(
        &executable_path,
        "EngLang standalone package placeholder. Use eng.exe run for preview execution.\n",
    )?;
    fs::write(
        &package_path,
        format!(
            "format = engpkg-preview-1\nsource_hash = {}\nbytecode_hash = {}\n",
            check_report.source_hash,
            source.len()
        ),
    )?;
    fs::write(
        &lock_path,
        format!(
            "runtime_version = {RUNTIME_VERSION}\ncompiler_version = {}\nprofile = repro\n",
            eng_compiler::COMPILER_VERSION
        ),
    )?;
    fs::write(
        &review_path,
        eng_report::render_html(&check_report, "plots/timeseries.svg"),
    )?;

    Ok(BuildOutput {
        executable_path,
        package_path,
        lock_path,
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

fn result_json(path: &Path, report: &CheckReport) -> String {
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
        if let Some(hash) = &promotion.source_hash {
            data_hashes.push_str(&format!("        \"hash\": \"{}\"\n", json_escape(hash)));
        } else {
            data_hashes.push_str("        \"hash\": null\n");
        }
        data_hashes.push_str("      }");
    }

    format!(
        "{{\n  \"format\": \"engres-preview-1\",\n  \"runtime_version\": \"{RUNTIME_VERSION}\",\n  \"compiler_version\": \"{}\",\n  \"source_path\": \"{}\",\n  \"source_hash\": \"{}\",\n  \"numeric_profile\": \"preview-f64\",\n  \"provenance\": {{\n    \"schema_count\": {},\n    \"csv_promotion_count\": {},\n    \"data_hashes\": [\n{}\n    ],\n    \"unit_conversion_history\": [],\n    \"plot_spec_hash\": \"preview\",\n    \"schema_hash\": \"preview\"\n  }}\n}}\n",
        eng_compiler::COMPILER_VERSION,
        json_escape(&path.display().to_string()),
        report.source_hash,
        report.semantic_program.schemas.len(),
        report.semantic_program.csv_promotions.len(),
        data_hashes
    )
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
