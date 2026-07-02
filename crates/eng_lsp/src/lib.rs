use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use eng_compiler::{
    all_quantity_completions, all_unit_infos, bundled_module_registry, check_source,
    classify_diagnostic_review_risk, classify_review_risk, CheckOptions, CheckReport,
    ClassFieldInfo, Diagnostic, DomainTypeParameterInfo, FunctionInfo, SemanticProgram, Severity,
    WithBlockInfo, WithOptionInfo,
};
use serde_json::{json, Value};

pub const LSP_SNAPSHOT_FORMAT: &str = "eng-lsp-snapshot-v1";
pub const LSP_EDITOR_METADATA_FORMAT: &str = "eng-lsp-editor-metadata-v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspSnapshot {
    pub diagnostics: Vec<LspDiagnostic>,
    pub completions: Vec<LspCompletion>,
    pub hovers: Vec<LspHover>,
    pub semantic_tokens: LspSemanticTokens,
    pub document_symbols: Vec<LspDocumentSymbol>,
    pub folding_ranges: Vec<LspFoldingRange>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspDiagnostic {
    pub line: usize,
    pub start_character: usize,
    pub end_character: usize,
    pub severity: String,
    pub code: String,
    pub message: String,
    pub help: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspCompletion {
    pub label: String,
    pub kind: String,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspHover {
    pub name: String,
    pub kind: String,
    pub line: usize,
    pub detail: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub status: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspSemanticTokens {
    pub legend: LspSemanticLegend,
    pub tokens: Vec<LspSemanticToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspSemanticLegend {
    pub token_types: Vec<String>,
    pub token_modifiers: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct LspSemanticToken {
    pub line: usize,
    pub start: usize,
    pub length: usize,
    pub token_type: String,
    pub modifiers: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspDocumentSymbol {
    pub name: String,
    pub detail: String,
    pub kind: u8,
    pub line: usize,
    pub character: usize,
    pub end_line: usize,
    pub end_character: usize,
    pub selection_line: usize,
    pub selection_character: usize,
    pub children: Vec<LspDocumentSymbol>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspFoldingRange {
    pub start_line: usize,
    pub end_line: usize,
    pub kind: Option<String>,
}

impl Default for LspSemanticTokens {
    fn default() -> Self {
        Self {
            legend: semantic_legend(),
            tokens: Vec::new(),
        }
    }
}

pub const SEMANTIC_TOKEN_TYPES: &[&str] = &[
    "namespace",
    "type",
    "class",
    "interface",
    "parameter",
    "variable",
    "property",
    "function",
    "method",
    "keyword",
    "modifier",
    "string",
    "number",
    "operator",
    "comment",
];

pub const SEMANTIC_TOKEN_MODIFIERS: &[&str] = &[
    "declaration",
    "definition",
    "readonly",
    "static",
    "local",
    "imported",
    "defaultLibrary",
    "deprecated",
    "unit",
    "quantity",
    "axis",
    "timeseries",
    "uncertain",
    "sideEffect",
    "external",
    "validation",
    "report",
    "solver",
    "planned",
    "internal",
    "riskHigh",
    "riskMedium",
    "state",
    "input",
    "model",
    "db",
    "cache",
    "workflowStep",
];

const COMPLETION_KEYWORDS: &[&str] = &[
    "across",
    "and",
    "ann",
    "append",
    "apply",
    "args",
    "as",
    "assert",
    "bar",
    "between",
    "by",
    "cases",
    "check",
    "class",
    "command",
    "commit",
    "component",
    "connect",
    "const",
    "constraints",
    "conservation",
    "collect",
    "column",
    "columns",
    "copy",
    "coverage",
    "csv",
    "delete",
    "derive",
    "der",
    "distribution",
    "domain",
    "download",
    "eq",
    "equation",
    "else",
    "evaluate",
    "export",
    "false",
    "fetch",
    "fill",
    "filter",
    "fn",
    "from",
    "golden",
    "grid",
    "head",
    "histogram",
    "get",
    "http",
    "if",
    "import",
    "input",
    "inputs",
    "insert",
    "integrate",
    "interpolate",
    "in",
    "into",
    "is",
    "json",
    "leakage_lint",
    "lhs",
    "line",
    "log",
    "matches",
    "materialize",
    "method",
    "missing",
    "mlp",
    "mode",
    "model",
    "model_card",
    "monotonic",
    "move",
    "not",
    "none",
    "null",
    "on",
    "open",
    "operator",
    "or",
    "of",
    "output",
    "outputs",
    "over",
    "package",
    "patch",
    "parameter",
    "plot",
    "policy",
    "port",
    "post",
    "predict",
    "print",
    "promote",
    "put",
    "random",
    "read",
    "records",
    "regression",
    "render",
    "replace",
    "request",
    "report",
    "return",
    "results",
    "rollback",
    "run",
    "sample",
    "schema",
    "script",
    "select",
    "show",
    "simulate",
    "solve",
    "sort",
    "sqlite",
    "state",
    "states",
    "struct",
    "summary",
    "summarize",
    "system",
    "template",
    "test",
    "text",
    "through",
    "to",
    "toml",
    "train_test_split",
    "true",
    "uniform",
    "upsert",
    "use",
    "using",
    "validate",
    "version",
    "vs",
    "where",
    "with",
    "within",
    "write",
];

const WORKFLOW_BUILTIN_KEYWORDS: &[&str] = &[
    "file",
    "dir",
    "join",
    "parent",
    "stem",
    "extension",
    "exists",
    "url",
    "secret",
    "env",
    "date",
    "datetime",
    "select_first_row",
    "filter",
    "select",
    "derive",
    "sort",
    "require_one",
    "check",
    "coverage",
    "fill",
    "fill_missing",
    "align",
    "resample",
    "sample",
    "uniform",
    "normal",
    "grid",
    "random",
    "lhs",
    "latin_hypercube",
    "latin-hypercube",
    "materialize",
    "apply",
    "collect",
    "run_case",
    "measured",
    "interval",
    "distribution",
    "ensemble",
    "propagate",
    "probability",
    "train",
    "train_test_split",
    "regression",
    "regression_table",
    "train_regression",
    "mlp",
    "ann",
    "evaluate",
    "model_card",
    "leakage_lint",
    "predict",
    "mean",
    "time_weighted_mean",
    "min",
    "max",
    "median",
    "std",
    "p90",
    "p95",
    "rmse",
    "duration_above",
    "integrate",
    "der",
    "delay",
    "sum",
];

const HYPHENATED_WORKFLOW_BUILTIN_KEYWORDS: &[&str] = &["latin-hypercube"];

const LANGUAGE_CONSTANT_KEYWORDS: &[&str] = &[
    "true",
    "false",
    "none",
    "null",
    "info",
    "warn",
    "debug",
    "error",
    "safe",
    "normal",
    "repro",
    "append",
    "insert",
    "upsert",
    "replace",
    "commit",
    "rollback",
    "keep",
    "empty",
    "interpolate",
    "monotonic",
    "linear",
    "pending",
    "running",
    "passed",
    "failed",
    "succeeded",
    "skipped",
    "blocked",
    "completed",
    "cached",
    "stale",
    "hit",
    "miss",
    "created",
    "updated",
    "metadata_ready",
    "warnings_present",
    "diagnostics_present",
    "fixed_step",
    "rk4",
    "adaptive_heun",
    "fixed_point",
    "newton",
    "implicit_euler_dae",
    "dynamic_component_explicit_euler",
    "dynamic_component_semi_implicit_euler",
    "dynamic_component_adaptive_heun",
    "trapezoidal",
];

const PUBLIC_TYPE_COMPLETIONS: &[(&str, &str)] = &[
    ("Bool", "Boolean value"),
    ("CsvFile", "CSV file path"),
    ("Date", "Calendar date"),
    ("DateTime", "Timestamp value"),
    ("DbConnection", "SQLite connection handle"),
    ("DbTableRef", "SQLite table reference"),
    ("DirectoryPath", "Directory path"),
    ("Derivative[T]", "Derivative vector or scalar type"),
    ("Duration", "Time duration"),
    ("FilePath", "Generic file path"),
    ("Float", "Floating-point value"),
    ("InputVector[T]", "State-space input vector"),
    ("Int", "Integer value"),
    ("JsonFile", "JSON file path"),
    ("LinearOperator[From -> To]", "State-space linear operator"),
    ("ModelArtifact", "Trained model artifact"),
    ("ModelCard", "Model-card review artifact"),
    ("Number", "Dimensionless numeric value"),
    ("Optional[T]", "Optional value"),
    ("OutputVector[T]", "State-space output vector"),
    ("Path", "Filesystem path"),
    ("Prediction", "Prediction table row"),
    ("ProcessResult", "External command result metadata"),
    ("Report", "Report artifact request metadata"),
    ("ReportFile", "Report output file path"),
    ("Secret[String]", "Redacted string value"),
    ("StateVector[T]", "State-space state vector"),
    ("String", "String value"),
    ("Table[T]", "Typed table value"),
    ("TextFile", "UTF-8 text file path"),
    ("TimeSeries[Time]", "Time-indexed series value"),
    ("TimeSeries[T]", "Typed time-indexed series value"),
    ("TomlFile", "TOML file path"),
    ("Url", "HTTP or HTTPS URL"),
    ("PlotFile", "Plot output file path"),
];

const WORKFLOW_BUILTIN_COMPLETIONS: &[(&str, &str)] = &[
    ("date", "calendar date constructor"),
    ("datetime", "timestamp constructor"),
    ("dir", "eng.path directory path helper"),
    ("env", "environment variable lookup"),
    ("exists", "eng.path existence check"),
    ("extension", "eng.path extension helper"),
    ("file", "eng.path file path helper"),
    ("join", "eng.path join helper"),
    ("parent", "eng.path parent directory helper"),
    ("secret", "redacted secret constructor"),
    ("stem", "eng.path filename stem helper"),
    ("url", "HTTP or HTTPS URL constructor"),
    ("filter", "Filter table rows with a `where` block"),
    ("select", "Select named columns from a table"),
    ("derive", "Add a derived column to a table"),
    ("sort", "Sort table rows by a column"),
    (
        "require_one",
        "Require exactly one row from a filtered table",
    ),
    ("check", "Start a data-quality check"),
    (
        "coverage",
        "TimeSeries coverage target for `check coverage`",
    ),
    (
        "fill_missing",
        "Fill missing TimeSeries values with an explicit policy",
    ),
    (
        "fill",
        "Record an explicit TimeSeries fill policy with `fill missing`",
    ),
    ("align", "Align TimeSeries values onto a shared axis"),
    ("resample", "Resample a TimeSeries onto a new step"),
    ("sample", "eng.sampling sample set helper"),
    ("uniform", "eng.sampling uniform distribution helper"),
    ("normal", "eng.sampling normal distribution helper"),
    ("grid", "eng.sampling grid construction helper"),
    ("random", "eng.sampling random generator helper"),
    ("lhs", "eng.sampling Latin hypercube helper"),
    ("latin_hypercube", "eng.sampling Latin hypercube helper"),
    (
        "latin-hypercube",
        "eng.sampling Latin hypercube helper alias",
    ),
    (
        "materialize",
        "Create reviewable case rows and case directories",
    ),
    ("apply", "Apply a template or step across case rows"),
    ("collect", "Collect per-case outputs into a table"),
    ("run_case", "Run one case with recorded inputs and outputs"),
    ("measured", "eng.uncertainty measured value helper"),
    ("interval", "eng.uncertainty interval helper"),
    ("distribution", "eng.uncertainty distribution helper"),
    ("ensemble", "eng.uncertainty ensemble helper"),
    ("propagate", "eng.uncertainty propagation helper"),
    ("probability", "eng.uncertainty probability helper"),
    (
        "train_test_split",
        "Create a deterministic train/test split",
    ),
    (
        "train regression",
        "Train a regression model from table columns",
    ),
    ("regression", "Train a small deterministic regression model"),
    (
        "regression_table",
        "Train a regression model from table columns",
    ),
    (
        "train_regression",
        "Train a regression model with explicit options",
    ),
    ("mlp", "Train a small deterministic neural-network model"),
    (
        "ann",
        "Train a small deterministic neural-network model alias",
    ),
    ("evaluate", "Compute model evaluation metrics"),
    ("model_card", "Create a model-card review artifact"),
    ("leakage_lint", "Check model features for leakage risk"),
    ("predict", "Create predictions from a model and input table"),
    ("mean", "eng.timeseries mean"),
    ("time_weighted_mean", "eng.timeseries time-weighted mean"),
    ("min", "eng.timeseries minimum"),
    ("max", "eng.timeseries maximum"),
    ("median", "eng.timeseries median"),
    ("std", "eng.timeseries standard deviation"),
    ("p90", "eng.timeseries 90th percentile"),
    ("p95", "eng.timeseries 95th percentile"),
    ("rmse", "eng.timeseries root mean square error"),
    ("duration_above", "eng.timeseries threshold duration"),
    ("integrate", "eng.timeseries integration helper"),
    ("der", "eng.timeseries derivative helper"),
    ("delay", "eng.timeseries delay helper"),
    ("sum", "domain conservation sum"),
];

const WORKFLOW_OPTION_COMPLETIONS: &[(&str, &str)] = &[
    (
        "algebraic_initialization",
        "solver algebraic initialization policy",
    ),
    ("algorithm", "model training option"),
    (
        "allow_failure",
        "Whether an external command failure can continue",
    ),
    ("artifact_kind", "expected artifact kind"),
    ("args", "external command argument list"),
    ("backend", "execution backend option"),
    ("body", "HTTP request body option"),
    ("body_size_limit", "HTTP response body size limit"),
    ("bias", "uncertainty propagation offset alias"),
    ("cache", "cache behavior option"),
    ("cache_dir", "cache storage directory option"),
    ("cache_key", "cache identity option"),
    ("cache_ttl", "cache entry time-to-live option"),
    ("case_id", "case identifier expression"),
    ("confirm", "explicit filesystem mutation confirmation"),
    ("confidence_band", "plot confidence-band source"),
    ("consistency_tolerance", "solver consistency tolerance"),
    ("count", "sample count option"),
    ("cwd", "external command working directory"),
    ("duration", "solver or simulation duration"),
    ("damping", "solver damping factor"),
    ("display_unit", "display unit option"),
    ("env", "external command environment"),
    ("end", "range end option"),
    ("epochs", "MLP training epoch count"),
    ("expected_sha256", "expected SHA-256 hash"),
    ("expected_outputs", "declared process outputs"),
    ("expected_step", "expected TimeSeries step"),
    ("features", "model feature columns"),
    ("finite_difference_step", "solver finite difference step"),
    (
        "offline_response",
        "Pinned offline HTTP response used instead of live network",
    ),
    ("fixture", "Legacy alias for offline_response"),
    ("headers", "HTTP request headers"),
    ("gain", "uncertainty propagation scale alias"),
    ("hidden", "MLP hidden layer option"),
    ("initial", "solver initial value"),
    ("initial_algebraic", "solver initial algebraic value"),
    ("initial_derivative", "solver initial derivative value"),
    ("inputs", "solver input source"),
    ("jacobian", "solver Jacobian policy"),
    ("kind", "distribution kind option"),
    ("key", "database upsert key"),
    ("layers", "MLP hidden layer option alias"),
    ("line_search_steps", "solver line-search step limit"),
    ("lower", "lower uncertainty or range bound"),
    ("mass_matrix", "solver mass-matrix policy"),
    ("max_gap", "maximum allowed gap option"),
    ("max_iter", "solver maximum iteration count"),
    ("mu", "uncertainty mean alias"),
    ("method", "fill or transform method"),
    ("missing", "missing value policy"),
    ("mode", "write mode"),
    ("n", "uncertainty sample count alias"),
    (
        "on_many",
        "What to do when `require_one` finds multiple rows",
    ),
    ("on_none", "What to do when `require_one` finds no rows"),
    ("offset", "uncertainty propagation offset"),
    ("output", "generated output path"),
    ("output_root", "case output root directory"),
    ("overwrite", "output overwrite policy"),
    ("query", "HTTP query parameters"),
    ("recursive", "filesystem recursion option"),
    ("relaxation", "solver relaxation factor"),
    ("relative_error", "relative uncertainty error option"),
    ("residual_scale", "solver residual scale"),
    ("residual_scales", "solver residual scale list"),
    ("resume", "case resume policy"),
    ("response_body_limit", "HTTP download body size limit"),
    ("retry", "external command retry policy"),
    ("return_column", "projection return column"),
    ("samples", "uncertainty sample count"),
    ("scale", "uncertainty propagation scale"),
    ("seed", "deterministic sampling seed"),
    ("sensor_std", "TimeSeries sensor standard deviation"),
    ("sigma", "uncertainty standard deviation alias"),
    ("split", "Train/test split to evaluate or lint"),
    ("solver", "solver algorithm option"),
    ("start", "range start option"),
    ("status", "case or validation status"),
    ("status_code", "expected HTTP status code"),
    ("step", "case workflow step"),
    ("test", "model train/test split option"),
    ("test_fraction", "model train/test split option alias"),
    ("target", "model target column"),
    ("template", "template source file"),
    ("timeout", "external command timeout"),
    ("timestep", "solver or simulation time step"),
    ("title", "plot or report title"),
    ("tool_version", "external tool version"),
    ("tolerance", "solver convergence tolerance"),
    ("transaction", "SQLite transaction policy"),
    ("type", "workflow display or command subtype option"),
    ("uncertainty", "uncertainty propagation policy"),
    ("upper", "upper uncertainty or range bound"),
    ("values", "template value map"),
    ("variable_scale", "solver variable scale"),
    ("variable_scales", "solver variable scale list"),
    ("x", "model feature column alias"),
    ("y", "model target column alias"),
    ("year", "calendar year option"),
];

const HTTP_RESPONSE_FIELD_COMPLETIONS: &[(&str, &str)] = &[
    ("body", "pinned offline HTTP response body text"),
    ("text", "alias for pinned offline HTTP response body text"),
    ("status", "network boundary status"),
    ("status_code", "HTTP status code"),
    ("status_class", "HTTP status class"),
    ("response_hash", "response SHA-256 hash"),
    ("hash", "alias for response SHA-256 hash"),
    ("url", "resolved request URL"),
];

pub fn snapshot_for_path(path: &Path) -> std::io::Result<LspSnapshot> {
    let source = std::fs::read_to_string(path)?;
    let report = check_source(path, &source, &CheckOptions::default());
    Ok(snapshot_from_report_with_source(&report, Some(&source)))
}

pub fn snapshot_for_source(path: &Path, source: &str) -> LspSnapshot {
    let report = check_source(path, source, &CheckOptions::default());
    snapshot_from_report_with_source(&report, Some(source))
}

pub fn completion_items_for_path_position(
    path: &Path,
    line: usize,
    character: usize,
) -> std::io::Result<Vec<LspCompletion>> {
    let source = std::fs::read_to_string(path)?;
    Ok(completion_items_for_source_position(
        path, &source, line, character,
    ))
}

pub fn completion_items_for_source_position(
    path: &Path,
    source: &str,
    line: usize,
    character: usize,
) -> Vec<LspCompletion> {
    let report = check_source(path, source, &CheckOptions::default());
    completion_items_at(&report, source, line, character)
}

pub fn snapshot_from_report(report: &CheckReport) -> LspSnapshot {
    snapshot_from_report_with_source(report, None)
}

pub fn snapshot_from_report_with_source(report: &CheckReport, source: Option<&str>) -> LspSnapshot {
    LspSnapshot {
        diagnostics: report
            .diagnostics
            .iter()
            .map(|diagnostic| lsp_diagnostic(diagnostic, source))
            .collect(),
        completions: completion_items(report),
        hovers: hover_items(report),
        semantic_tokens: source
            .map(|source| semantic_tokens(report, source))
            .unwrap_or_default(),
        document_symbols: source
            .map(|source| document_symbols(report, source))
            .unwrap_or_default(),
        folding_ranges: source.map(folding_ranges).unwrap_or_default(),
    }
}

fn lsp_diagnostic(diagnostic: &Diagnostic, source: Option<&str>) -> LspDiagnostic {
    let (start_character, end_character) = diagnostic_character_range(diagnostic, source);
    LspDiagnostic {
        line: diagnostic.line,
        start_character,
        end_character,
        severity: diagnostic.severity.as_str().to_owned(),
        code: diagnostic.code.clone(),
        message: diagnostic.message.clone(),
        help: diagnostic.help.clone(),
    }
}

fn diagnostic_character_range(diagnostic: &Diagnostic, source: Option<&str>) -> (usize, usize) {
    let Some(source) = source else {
        return (0, 1);
    };
    let lines = source_lines(source);
    let line_index = line_index_from_one_based(&lines, diagnostic.line);
    let line = lines.get(line_index).copied().unwrap_or_default();
    let Some((start_byte, end_byte)) = diagnostic_byte_range(line, diagnostic) else {
        return (0, 1);
    };
    byte_range_to_utf16(line, start_byte, end_byte)
}

fn diagnostic_byte_range(line: &str, diagnostic: &Diagnostic) -> Option<(usize, usize)> {
    if diagnostic.code.starts_with("E-DIM-ADD-") {
        if let Some(range) = binary_add_sub_operator_range(line) {
            return Some(range);
        }
    }

    match diagnostic.code.as_str() {
        "E-PUBLIC-ANNOTATION-001" => {
            if let Some(range) = find_byte_range(line, "=") {
                return Some(range);
            }
        }
        "E-FS-CONFIRM-001" => {
            if let Some(range) =
                find_byte_range(line, "delete").or_else(|| find_byte_range(line, "move"))
            {
                return Some(range);
            }
        }
        "E-FS-DELETE-001" => {
            if let Some(range) = find_byte_range(line, "delete") {
                return Some(range);
            }
        }
        _ => {}
    }

    diagnostic_backtick_byte_range(line, diagnostic)
        .or_else(|| first_identifier_byte_range(line))
        .or_else(|| first_token_byte_range(line))
}

fn diagnostic_backtick_byte_range(line: &str, diagnostic: &Diagnostic) -> Option<(usize, usize)> {
    if let Some(range) = backtick_payload_byte_range(line, &diagnostic.message) {
        return Some(range);
    }
    diagnostic
        .help
        .as_ref()
        .and_then(|help| backtick_payload_byte_range(line, help))
}

fn backtick_payload_byte_range(line: &str, text: &str) -> Option<(usize, usize)> {
    let mut rest = text;
    loop {
        let Some(open) = rest.find('`') else {
            return None;
        };
        let payload_start = open + '`'.len_utf8();
        let after_open = &rest[payload_start..];
        let Some(close) = after_open.find('`') else {
            return None;
        };
        let payload = &after_open[..close];
        if let Some(range) = find_byte_range(line, payload) {
            return Some(range);
        }
        rest = &after_open[close + '`'.len_utf8()..];
    }
}

fn binary_add_sub_operator_range(line: &str) -> Option<(usize, usize)> {
    let search_start = line
        .find('=')
        .map(|index| index + '='.len_utf8())
        .unwrap_or(0);
    for (relative_index, character) in line[search_start..].char_indices() {
        if character != '+' && character != '-' {
            continue;
        }
        let index = search_start + relative_index;
        let previous = line[..index]
            .chars()
            .rev()
            .find(|candidate| !candidate.is_whitespace());
        let next = line[index + character.len_utf8()..]
            .chars()
            .find(|candidate| !candidate.is_whitespace());
        let Some(previous) = previous else {
            continue;
        };
        let Some(next) = next else {
            continue;
        };
        if matches!(
            previous,
            '=' | '(' | '[' | '{' | ',' | '+' | '-' | '*' | '/' | ':'
        ) {
            continue;
        }
        if matches!(next, ')' | ']' | '}' | ',' | '+' | '-' | '*' | '/') {
            continue;
        }
        return Some((index, index + character.len_utf8()));
    }
    None
}

fn find_byte_range(line: &str, needle: &str) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return None;
    }
    line.find(needle).map(|start| (start, start + needle.len()))
}

fn first_identifier_byte_range(line: &str) -> Option<(usize, usize)> {
    let mut start = None;
    for (index, character) in line.char_indices() {
        let byte = character as u32;
        let is_ascii = byte <= u8::MAX as u32;
        let is_identifier_start = is_ascii && is_ident_start(byte as u8);
        let is_identifier_byte = is_ascii && is_ident_byte(byte as u8);
        match (start, is_identifier_start, is_identifier_byte) {
            (None, true, _) => start = Some(index),
            (Some(start_index), _, false) => return Some((start_index, index)),
            _ => {}
        }
    }
    start.map(|start| (start, line.len()))
}

fn first_token_byte_range(line: &str) -> Option<(usize, usize)> {
    let (start, _) = line
        .char_indices()
        .find(|(_, character)| !character.is_whitespace())?;
    let end = line[start..]
        .char_indices()
        .skip(1)
        .find(|(_, character)| character.is_whitespace())
        .map(|(index, _)| start + index)
        .unwrap_or(line.len());
    Some((start, end))
}

fn byte_range_to_utf16(line: &str, start_byte: usize, end_byte: usize) -> (usize, usize) {
    let start_character = utf16_len(&line[..start_byte]);
    let mut end_character = utf16_len(&line[..end_byte]);
    if end_character <= start_character {
        end_character = start_character + 1;
    }
    (start_character, end_character)
}

pub fn snapshot_json(snapshot: &LspSnapshot) -> Value {
    json!({
        "format": LSP_SNAPSHOT_FORMAT,
        "diagnostics": snapshot.diagnostics.iter().map(diagnostic_json).collect::<Vec<_>>(),
        "completions": snapshot.completions.iter().map(completion_json).collect::<Vec<_>>(),
        "hovers": snapshot.hovers.iter().map(hover_json).collect::<Vec<_>>(),
        "semantic_tokens": semantic_tokens_json(&snapshot.semantic_tokens),
        "document_symbols": snapshot.document_symbols.iter().map(document_symbol_json).collect::<Vec<_>>(),
        "folding_ranges": snapshot.folding_ranges.iter().map(folding_range_json).collect::<Vec<_>>(),
    })
}

pub fn editor_metadata_json() -> Value {
    let completions = editor_completion_seed();
    json!({
        "format": LSP_EDITOR_METADATA_FORMAT,
        "semantic_token_legend": semantic_legend_json(&semantic_legend()),
        "syntax_catalog": editor_syntax_catalog_json(),
        "completion_seed_count": completions.len(),
        "completion_seed": completions.iter().map(editor_completion_json).collect::<Vec<_>>(),
    })
}

pub fn editor_syntax_catalog_json() -> Value {
    json!({
        "keywords": COMPLETION_KEYWORDS,
        "workflow_builtins": WORKFLOW_BUILTIN_KEYWORDS,
        "hyphenated_workflow_builtins": HYPHENATED_WORKFLOW_BUILTIN_KEYWORDS,
        "workflow_options": WORKFLOW_OPTION_COMPLETIONS
            .iter()
            .map(|(label, detail)| json!({
                "label": label,
                "detail": detail,
            }))
            .collect::<Vec<_>>(),
        "public_types": PUBLIC_TYPE_COMPLETIONS
            .iter()
            .map(|(label, detail)| json!({
                "label": label,
                "detail": detail,
                "base": public_type_completion_base(label),
            }))
            .collect::<Vec<_>>(),
        "quantities": all_quantity_completions()
            .iter()
            .map(|quantity| json!({
                "label": quantity.quantity_kind,
                "canonical_unit": quantity.canonical_unit,
                "dimension": quantity.dimension,
                "detail": quantity.description,
            }))
            .collect::<Vec<_>>(),
        "units": all_unit_infos()
            .iter()
            .map(|unit| json!({
                "label": unit.symbol,
                "canonical_unit": unit.canonical_unit,
                "quantity_hint": unit.quantity_hint,
                "dimension": unit.dimension,
            }))
            .collect::<Vec<_>>(),
    })
}

pub fn editor_completion_seed() -> Vec<LspCompletion> {
    let report = check_source(
        Path::new("editor-metadata.eng"),
        "",
        &CheckOptions::default(),
    );
    completion_items(&report)
}

pub fn semantic_legend_json(legend: &LspSemanticLegend) -> Value {
    json!({
        "token_types": legend.token_types,
        "token_modifiers": legend.token_modifiers,
    })
}

pub fn semantic_legend() -> LspSemanticLegend {
    LspSemanticLegend {
        token_types: SEMANTIC_TOKEN_TYPES
            .iter()
            .map(|token_type| (*token_type).to_owned())
            .collect(),
        token_modifiers: SEMANTIC_TOKEN_MODIFIERS
            .iter()
            .map(|modifier| (*modifier).to_owned())
            .collect(),
    }
}

pub fn semantic_tokens_json(tokens: &LspSemanticTokens) -> Value {
    json!({
        "legend": semantic_legend_json(&tokens.legend),
        "tokens": tokens.tokens.iter().map(semantic_token_json).collect::<Vec<_>>(),
    })
}

pub fn semantic_token_json(token: &LspSemanticToken) -> Value {
    json!({
        "line": token.line,
        "start": token.start,
        "length": token.length,
        "type": token.token_type,
        "modifiers": token.modifiers,
    })
}

pub fn semantic_tokens_lsp_json(tokens: &LspSemanticTokens) -> Value {
    let mut data = Vec::new();
    let mut previous_line = 0usize;
    let mut previous_start = 0usize;

    for token in &tokens.tokens {
        let Some(token_type_index) = semantic_token_type_index(&token.token_type) else {
            continue;
        };
        let delta_line = token.line.saturating_sub(previous_line);
        let delta_start = if delta_line == 0 {
            token.start.saturating_sub(previous_start)
        } else {
            token.start
        };
        data.push(delta_line);
        data.push(delta_start);
        data.push(token.length);
        data.push(token_type_index);
        data.push(semantic_token_modifier_bits(&token.modifiers));
        previous_line = token.line;
        previous_start = token.start;
    }

    json!({ "data": data })
}

pub fn document_symbols_lsp_json(symbols: &[LspDocumentSymbol]) -> Value {
    json!(symbols.iter().map(document_symbol_json).collect::<Vec<_>>())
}

pub fn document_symbol_json(symbol: &LspDocumentSymbol) -> Value {
    let selection_end = symbol.selection_character + utf16_len(&symbol.name);
    json!({
        "name": symbol.name,
        "detail": symbol.detail,
        "kind": symbol.kind,
        "range": {
            "start": { "line": symbol.line, "character": symbol.character },
            "end": { "line": symbol.end_line, "character": symbol.end_character }
        },
        "selectionRange": {
            "start": { "line": symbol.selection_line, "character": symbol.selection_character },
            "end": { "line": symbol.selection_line, "character": selection_end }
        },
        "children": symbol.children.iter().map(document_symbol_json).collect::<Vec<_>>(),
    })
}

pub fn folding_ranges_lsp_json(ranges: &[LspFoldingRange]) -> Value {
    json!(ranges.iter().map(folding_range_json).collect::<Vec<_>>())
}

pub fn folding_range_json(range: &LspFoldingRange) -> Value {
    match &range.kind {
        Some(kind) => json!({
            "startLine": range.start_line,
            "endLine": range.end_line,
            "kind": kind,
        }),
        None => json!({
            "startLine": range.start_line,
            "endLine": range.end_line,
        }),
    }
}

fn semantic_token_type_index(token_type: &str) -> Option<usize> {
    SEMANTIC_TOKEN_TYPES
        .iter()
        .position(|candidate| *candidate == token_type)
}

fn semantic_token_modifier_bits(modifiers: &[String]) -> usize {
    let mut bits = 0usize;
    for modifier in modifiers {
        if let Some(index) = SEMANTIC_TOKEN_MODIFIERS
            .iter()
            .position(|candidate| *candidate == modifier)
        {
            bits |= 1usize << index;
        }
    }
    bits
}

fn semantic_tokens(report: &CheckReport, source: &str) -> LspSemanticTokens {
    let mut builder = SemanticTokenBuilder::new(source);
    builder.add_lexical_tokens();
    let program = &report.semantic_program;

    for import in &program.imports {
        let modifiers = stdlib_import_semantic_modifiers(&import.target);
        builder.push_on_line(import.line, &import.target, "namespace", &modifiers);
    }

    for constant in &program.consts {
        builder.push_on_line(
            constant.line,
            &constant.name,
            "variable",
            &["declaration", "readonly"],
        );
    }

    for binding in &program.typed_bindings {
        let modifiers = semantic_modifiers_for_quantity(&binding.semantic_type.quantity_kind);
        builder.push_on_line(binding.line, &binding.name, "variable", &modifiers);
    }

    for hover in &program.hover_hints {
        let mut modifiers = semantic_modifiers_for_quantity(&hover.quantity_kind);
        if hover.detail.starts_with("importable const") {
            modifiers.push("readonly");
            modifiers.push("defaultLibrary");
        }
        builder.push_on_line(hover.line, &hover.name, "variable", &modifiers);
    }

    for function in &program.functions {
        builder.push_on_line(
            function.line,
            &function.name,
            "function",
            &["declaration", "definition"],
        );
        for parameter in &function.parameters {
            builder.push_on_line(
                function.line,
                &parameter.name,
                "parameter",
                &["declaration"],
            );
        }
        for local in &function.locals {
            builder.push_on_line(
                local.line,
                &local.name,
                "variable",
                &["declaration", "local"],
            );
        }
    }
    add_function_scoped_symbol_semantic_tokens(&program.functions, &mut builder);

    for schema in &program.schemas {
        builder.push_on_line(schema.line, &schema.name, "class", &["declaration"]);
        for column in &schema.columns {
            builder.push_on_line(column.line, &column.name, "property", &["declaration"]);
            builder.push_on_line(column.line, &column.type_name, "type", &["quantity"]);
            if let Some(unit) = &column.unit {
                builder.push_on_line(column.line, unit, "type", &["unit"]);
            }
        }
        for constraint in &schema.constraints {
            builder.push_keywords_on_line(
                constraint.line,
                &["between", "monotonic", "is", "none", "and", "or"],
                &["validation"],
            );
        }
        for policy in &schema.missing_policies {
            builder.push_on_line(policy.line, &policy.column, "property", &["validation"]);
            builder.push_on_line(policy.line, "max_gap", "property", &["validation"]);
            builder.push_keywords_on_line(policy.line, &["interpolate", "error"], &["validation"]);
        }
    }

    for promotion in &program.csv_promotions {
        builder.push_on_line(
            promotion.line,
            &promotion.binding,
            "variable",
            &["declaration"],
        );
        builder.push_on_line(
            promotion.line,
            &promotion.schema_name,
            "class",
            &["defaultLibrary"],
        );
        if promotion.source_format == "json_records" {
            builder.push_keywords_on_line(
                promotion.line,
                &["promote", "json", "records"],
                &["workflowStep"],
            );
        }
    }

    for promotion in &program.config_promotions {
        builder.push_on_line(
            promotion.line,
            &promotion.binding,
            "variable",
            &["declaration"],
        );
        builder.push_on_line(
            promotion.line,
            &promotion.schema_name,
            "class",
            &["defaultLibrary"],
        );
    }

    for sample in &program.sample_generations {
        builder.push_on_line(
            sample.line,
            &sample.binding,
            "variable",
            &["declaration", "workflowStep"],
        );
        for distribution in &sample.distributions {
            let modifiers = semantic_modifiers_for_quantity(&distribution.quantity_kind);
            builder.push_on_line(
                distribution.line,
                &distribution.name,
                "property",
                &modifiers,
            );
        }
    }

    for transform in &program.table_transforms {
        builder.push_on_line(
            transform.line,
            &transform.binding,
            "variable",
            &["declaration", "workflowStep"],
        );
        builder.push_on_line(
            transform.line,
            &transform.source_table,
            "variable",
            &["workflowStep"],
        );
        if let Some(secondary_table) = &transform.secondary_table {
            builder.push_on_line(
                transform.line,
                secondary_table,
                "variable",
                &["workflowStep"],
            );
        }
        for column in &transform.selected_columns {
            builder.push_on_line(column.line, &column.name, "property", &["workflowStep"]);
        }
        for key in &transform.sort_keys {
            builder.push_on_line(key.line, &key.column, "property", &["workflowStep"]);
        }
        for column in &transform.derived_columns {
            builder.push_on_line(
                column.line,
                &column.name,
                "property",
                &["declaration", "workflowStep"],
            );
            for source_column in &column.source_columns {
                builder.push_on_line(column.line, source_column, "property", &["workflowStep"]);
            }
        }
        for predicate in &transform.predicates {
            if let Some(column) = &predicate.column {
                builder.push_on_line(predicate.line, column, "property", &["workflowStep"]);
            }
        }
        for join in &transform.join_keys {
            builder.push_on_line(join.line, &join.left_column, "property", &["workflowStep"]);
            builder.push_on_line(join.line, &join.right_column, "property", &["workflowStep"]);
        }
    }

    for request in &program.net_requests {
        builder.push_on_line(
            request.line,
            &request.binding,
            "variable",
            &["declaration", "external"],
        );
        builder.push_member_fields(
            &request.binding,
            HTTP_RESPONSE_FIELD_COMPLETIONS,
            &["external"],
        );
        builder.push_keywords_on_line(
            request.line,
            &["http", http_request_method_keyword(&request.method)],
            &["sideEffect", "external"],
        );
    }

    for download in &program.net_downloads {
        builder.push_keywords_on_line(download.line, &["download"], &["sideEffect", "external"]);
    }

    for axis in &program.axis_infos {
        builder.push_on_line(axis.line, &axis.axis, "type", &["axis"]);
    }

    for uncertainty in &program.uncertainty_infos {
        builder.push_on_line(
            uncertainty.line,
            &uncertainty.binding,
            "variable",
            &["declaration", "uncertain"],
        );
    }

    for ml in &program.ml_infos {
        builder.push_on_line(ml.line, &ml.binding, "variable", &["declaration", "model"]);
        if let Some(source) = &ml.source {
            builder.push_on_line(ml.line, source, "variable", &["model"]);
        }
        if let Some(input) = &ml.prediction_input {
            builder.push_on_line(ml.line, input, "variable", &["model"]);
        }
        if let Some(target) = &ml.target {
            builder.push_on_line(ml.line, "target", "property", &["model"]);
            builder.push_on_line(ml.line, target, "property", &["model"]);
        }
        if !ml.features.is_empty() {
            builder.push_on_line(ml.line, "features", "property", &["model"]);
        }
        for feature in &ml.features {
            builder.push_on_line(ml.line, feature, "property", &["model"]);
        }
        if let Some(algorithm) = &ml.algorithm {
            builder.push_on_line(ml.line, "algorithm", "property", &["model"]);
            if is_simple_identifier_segment(algorithm) {
                builder.push_on_line(ml.line, algorithm, "keyword", &["model"]);
            }
        }
        if ml.test_fraction.is_some() {
            builder.push_on_line(ml.line, "test", "property", &["model"]);
        }
        if ml.seed.is_some() {
            builder.push_on_line(ml.line, "seed", "property", &["model"]);
        }
        if !ml.hidden_layers.is_empty() {
            builder.push_on_line(ml.line, "hidden", "property", &["model"]);
        }
        if ml.epochs.is_some() {
            builder.push_on_line(ml.line, "epochs", "property", &["model"]);
        }
    }

    for cache in &program.cache_records {
        let modifiers = if cache.owner_kind == "model" {
            ["cache", "model"].as_slice()
        } else {
            ["cache"].as_slice()
        };
        builder.push_on_line(cache.line, &cache.owner_name, "variable", modifiers);
    }

    for system in &program.systems {
        builder.push_on_line(system.line, &system.name, "class", &["declaration"]);
        for variable in &system.variables {
            let modifiers = match variable.role.as_str() {
                "state" => ["declaration", "state"].as_slice(),
                "input" => ["declaration", "input"].as_slice(),
                "parameter" => ["declaration", "readonly"].as_slice(),
                _ => ["declaration"].as_slice(),
            };
            builder.push_on_line(variable.line, &variable.name, "variable", modifiers);
        }
    }

    for vector in &program.state_space_vectors {
        builder.push_on_line(vector.line, &vector.name, "variable", &["declaration"]);
    }

    for domain in &program.domains {
        builder.push_on_line(domain.line, &domain.name, "interface", &["declaration"]);
        if let Some(package) = &domain.package {
            builder.push_on_line(
                domain.line,
                package,
                "namespace",
                &["defaultLibrary", "internal"],
            );
        }
        for variable in &domain.variables {
            builder.push_on_line(variable.line, &variable.name, "property", &["declaration"]);
        }
    }

    for component in &program.components {
        builder.push_on_line(component.line, &component.name, "class", &["declaration"]);
        for port in &component.ports {
            builder.push_on_line(port.line, &port.name, "property", &["declaration"]);
            builder.push_on_line(
                port.line,
                &port.domain_name,
                "interface",
                &["defaultLibrary"],
            );
        }
        for parameter in &component.parameters {
            builder.push_on_line(
                parameter.line,
                &parameter.name,
                "parameter",
                &["declaration", "readonly"],
            );
        }
        for input in &component.inputs {
            builder.push_on_line(
                input.line,
                &input.name,
                "parameter",
                &["declaration", "input"],
            );
        }
        for local in &component.local_expressions {
            builder.push_on_line(
                local.line,
                &local.name,
                "variable",
                &["declaration", "local"],
            );
        }
    }

    for class_info in &program.classes {
        builder.push_on_line(class_info.line, &class_info.name, "class", &["declaration"]);
        for field in &class_info.fields {
            builder.push_on_line(field.line, &field.name, "property", &["declaration"]);
            builder.push_on_line(field.line, &field.type_name, "type", &["quantity"]);
        }
        for validation in &class_info.validations {
            builder.push_keywords_on_line(validation.line, &["validate"], &["validation"]);
        }
        for method in &class_info.methods {
            builder.push_on_line(method.line, &method.name, "method", &["declaration"]);
        }
    }

    for object in &program.class_objects {
        builder.push_on_line(object.line, &object.name, "variable", &["declaration"]);
        builder.push_on_line(
            object.line,
            &object.class_name,
            "class",
            &["defaultLibrary"],
        );
        for field in &object.fields {
            builder.push_on_line(field.line, &field.name, "property", &["declaration"]);
        }
        for validation in &object.validations {
            builder.push_keywords_on_line(validation.line, &["validate"], &["validation"]);
        }
    }

    for args_block in &program.args_blocks {
        builder.push_keywords_on_line(args_block.line, &["args"], &["declaration"]);
        for field in &args_block.fields {
            builder.push_on_line(field.line, &field.name, "parameter", &["declaration"]);
            builder.push_on_line(field.line, &field.type_name, "type", &[]);
        }
    }

    for value in &program.arg_values {
        builder.push_on_line(value.line, &value.name, "parameter", &["readonly"]);
    }

    for print in &program.prints {
        builder.push_keywords_on_line(print.line, &["print", "log"], &["report"]);
    }

    for export in &program.csv_exports {
        builder.push_on_line(export.line, &export.source, "variable", &["report"]);
        for field in &export.fields {
            builder.push_on_line(field.line, &field.name, "property", &["report"]);
        }
    }

    for write in &program.writes {
        let mut modifiers = vec!["sideEffect"];
        if write.quantity_kind == "DbWrite" {
            modifiers.push("db");
        } else if write.format == "standard_text" {
            modifiers.push("workflowStep");
        }
        builder.push_on_line(write.line, &write.expression, "variable", &modifiers);
        builder.push_keywords_on_line(write.line, &["write"], &modifiers);
        if write.format == "standard_text" {
            builder.push_on_line(
                write.line,
                &write.format,
                "function",
                &["defaultLibrary", "workflowStep"],
            );
        } else {
            builder.push_on_line(write.line, &write.format, "function", &["defaultLibrary"]);
        }
    }

    for operation in &program.file_operations {
        builder.push_keywords_on_line(
            operation.line,
            &[operation.operation.as_str()],
            &["sideEffect", "external"],
        );
    }

    for process in &program.process_runs {
        builder.push_on_line(
            process.line,
            &process.binding,
            "variable",
            &["declaration", "external"],
        );
        builder.push_keywords_on_line(
            process.line,
            &["run", "command"],
            &["sideEffect", "external"],
        );
    }

    for test in &program.tests {
        builder.push_keywords_on_line(test.line, &["test"], &["validation"]);
        for assert in &test.assertions {
            builder.push_keywords_on_line(assert.line, &["assert"], &["validation"]);
        }
        for golden in &test.goldens {
            builder.push_keywords_on_line(golden.line, &["golden"], &["validation"]);
        }
    }

    for command in &program.command_styles {
        builder.push_on_line(command.line, &command.verb, "function", &["defaultLibrary"]);
        if command.verb == "fill" && command.target.trim().starts_with("missing ") {
            builder.push_keywords_on_line(command.line, &["missing"], &["validation"]);
        }
    }

    for suite in &program.expectation_suites {
        builder.push_on_line(
            suite.line,
            &suite.binding,
            "variable",
            &["declaration", "validation"],
        );
        for expectation in &suite.expectations {
            builder.push_keywords_on_line(expectation.line, &["check"], &["validation"]);
        }
    }

    for block in &program.where_blocks {
        builder.push_keywords_on_line(block.line, &["where"], &["local"]);
        for binding in &block.bindings {
            let modifiers = semantic_modifiers_for_quantity(&binding.quantity_kind);
            let mut modifiers = modifiers;
            modifiers.push("local");
            builder.push_on_line(binding.line, &binding.name, "variable", &modifiers);
        }
    }

    for block in &program.with_blocks {
        builder.push_keywords_on_line(block.line, &["with"], &[]);
        for option in &block.options {
            let modifiers = with_option_semantic_modifiers(program, block, &option.key);
            builder.push_on_line(option.line, &option.key, "property", modifiers);
            add_with_option_value_semantic_token(&mut builder, program, block, option);
        }
    }

    add_review_risk_semantic_tokens(report, &mut builder);

    builder.finish()
}

fn with_option_semantic_modifiers(
    program: &SemanticProgram,
    block: &WithBlockInfo,
    key: &str,
) -> &'static [&'static str] {
    if key == "display_unit" || key.starts_with("unit ") {
        return &["report"];
    }
    match key {
        "cache" | "cache_key" | "cache_dir" | "cache_ttl" => &["cache"],
        "key" | "transaction" => &["db"],
        "on_none" | "on_many" | "expected_step" | "max_gap" | "status" => &["validation"],
        "sensor_std" | "confidence_band" => &["uncertain"],
        "bias" | "gain" | "kind" | "lower" | "mu" | "n" | "offset" | "relative_error"
        | "samples" | "scale" | "sigma" | "uncertainty" | "upper" => &["uncertain"],
        "solver"
        | "timestep"
        | "duration"
        | "tolerance"
        | "finite_difference_step"
        | "damping"
        | "initial"
        | "initial_derivative"
        | "initial_algebraic"
        | "algebraic_initialization"
        | "inputs"
        | "jacobian"
        | "mass_matrix"
        | "max_iter"
        | "line_search_steps"
        | "relaxation"
        | "residual_scale"
        | "residual_scales"
        | "consistency_tolerance"
        | "variable_scale"
        | "variable_scales" => &["solver"],
        "mode" if is_db_write_with_block(program, block.owner_line) => &["db"],
        "overwrite" | "mode" | "confirm" | "recursive" | "output" => &["sideEffect"],
        "args"
        | "query"
        | "headers"
        | "body"
        | "offline_response"
        | "fixture"
        | "expected_sha256"
        | "expected_outputs"
        | "tool_version"
        | "status_code"
        | "body_size_limit"
        | "response_body_limit"
        | "retry"
        | "timeout"
        | "allow_failure"
        | "cwd"
        | "env" => &["external"],
        "algorithm" | "features" | "x" | "hidden" | "layers" | "target" | "y" | "test"
        | "test_fraction" | "epochs" | "split" | "seed"
            if is_model_with_block(program, block.owner_line) =>
        {
            &["model"]
        }
        "count" | "seed" | "start" | "end" | "method"
            if is_sample_with_block(program, block.owner_line) =>
        {
            &["workflowStep"]
        }
        "step" | "case_id" | "output_root" | "resume" | "template" | "values" | "artifact_kind"
        | "year" | "return_column" => &["workflowStep"],
        "title" => &["report"],
        _ if is_db_write_with_block(program, block.owner_line) => &["db"],
        _ if is_net_with_block(program, block.owner_line) => &["external"],
        _ => &[],
    }
}

fn add_with_option_value_semantic_token(
    builder: &mut SemanticTokenBuilder<'_>,
    program: &SemanticProgram,
    block: &WithBlockInfo,
    option: &WithOptionInfo,
) {
    if matches!(option.key.as_str(), "features" | "x")
        && is_model_with_block(program, block.owner_line)
    {
        for value in option_list_value_identifiers(&option.value) {
            builder.push_on_line(option.line, value, "property", &["model"]);
        }
        return;
    }
    let Some(value) = leading_option_value_identifier(&option.value) else {
        return;
    };
    if let Some((token_type, modifiers)) =
        with_option_value_semantic_class(program, block, &option.key, value)
    {
        builder.push_on_line(option.line, value, token_type, modifiers);
    }
}

fn leading_option_value_identifier(value: &str) -> Option<&str> {
    let value = value.trim_start();
    let bytes = value.as_bytes();
    let first = *bytes.first()?;
    if !is_ident_start(first) {
        return None;
    }
    let mut end = 1usize;
    while end < bytes.len() && is_ident_byte(bytes[end]) {
        end += 1;
    }
    Some(&value[..end])
}

fn option_list_value_identifiers(value: &str) -> Vec<&str> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(str::trim)
        .filter(|value| {
            value.bytes().next().is_some_and(is_ident_start) && value.bytes().all(is_ident_byte)
        })
        .collect()
}

fn with_option_value_semantic_class(
    program: &SemanticProgram,
    block: &WithBlockInfo,
    key: &str,
    value: &str,
) -> Option<(&'static str, &'static [&'static str])> {
    match key {
        "transaction" if matches!(value, "commit" | "rollback") => Some(("keyword", &["db"])),
        "mode"
            if is_db_write_with_block(program, block.owner_line)
                && matches!(value, "append" | "insert" | "upsert" | "replace") =>
        {
            Some(("keyword", &["db"]))
        }
        "mode" if matches!(value, "append" | "insert" | "upsert" | "replace") => {
            Some(("keyword", &["sideEffect"]))
        }
        "cache" if matches!(value, "true" | "false") => Some(("keyword", &["cache"])),
        "overwrite" | "confirm" | "recursive" | "allow_failure"
            if matches!(value, "true" | "false") =>
        {
            Some((
                "keyword",
                with_option_semantic_modifiers(program, block, key),
            ))
        }
        "resume" if matches!(value, "true" | "false") => Some(("keyword", &["workflowStep"])),
        "on_none" | "on_many" | "missing" | "status"
            if matches!(
                value,
                "error"
                    | "keep"
                    | "empty"
                    | "interpolate"
                    | "pending"
                    | "running"
                    | "passed"
                    | "failed"
                    | "succeeded"
                    | "skipped"
                    | "blocked"
                    | "completed"
            ) =>
        {
            Some(("keyword", &["validation"]))
        }
        "solver" | "method" | "algebraic_initialization" | "jacobian" | "mass_matrix"
            if matches!(
                value,
                "fixed_step"
                    | "rk4"
                    | "adaptive_heun"
                    | "fixed_point"
                    | "newton"
                    | "implicit_euler_dae"
                    | "dynamic_component_explicit_euler"
                    | "dynamic_component_semi_implicit_euler"
                    | "dynamic_component_adaptive_heun"
                    | "trapezoidal"
                    | "none"
            ) =>
        {
            Some(("keyword", &["solver"]))
        }
        "algorithm" | "split"
            if is_model_with_block(program, block.owner_line)
                && matches!(value, "linear" | "regression" | "mlp") =>
        {
            Some(("keyword", &["model"]))
        }
        "target" | "y" if is_model_with_block(program, block.owner_line) => {
            Some(("property", &["model"]))
        }
        "step" if matches!(value, "run_case") => {
            Some(("function", &["defaultLibrary", "workflowStep"]))
        }
        "method" if is_sample_with_block(program, block.owner_line) => {
            Some(("keyword", &["workflowStep"]))
        }
        _ => None,
    }
}

fn is_db_write_with_block(program: &SemanticProgram, owner_line: Option<usize>) -> bool {
    let Some(owner_line) = owner_line else {
        return false;
    };
    program
        .writes
        .iter()
        .any(|write| write.line == owner_line && write.quantity_kind == "DbWrite")
}

fn is_model_with_block(program: &SemanticProgram, owner_line: Option<usize>) -> bool {
    let Some(owner_line) = owner_line else {
        return false;
    };
    program
        .ml_infos
        .iter()
        .any(|model| model.line == owner_line)
}

fn is_sample_with_block(program: &SemanticProgram, owner_line: Option<usize>) -> bool {
    let Some(owner_line) = owner_line else {
        return false;
    };
    program
        .sample_generations
        .iter()
        .any(|sample| sample.line == owner_line)
}

fn add_function_scoped_symbol_semantic_tokens(
    functions: &[FunctionInfo],
    builder: &mut SemanticTokenBuilder<'_>,
) {
    for function in functions {
        let Some(start_line) = function.line.checked_sub(1) else {
            continue;
        };
        if builder.lines.get(start_line).is_none() {
            continue;
        }
        let end_line = block_end_line(&builder.lines, start_line).unwrap_or(start_line);
        let parameters = function
            .parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>();
        let locals = function
            .locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>();
        for line in start_line..=end_line {
            builder.push_identifiers_on_line(line, &parameters, "parameter", &[]);
            builder.push_identifiers_on_line(line, &locals, "variable", &["local"]);
        }
    }
}

fn is_net_with_block(program: &SemanticProgram, owner_line: Option<usize>) -> bool {
    let Some(owner_line) = owner_line else {
        return false;
    };
    program
        .net_requests
        .iter()
        .any(|request| request.line == owner_line)
        || program
            .net_downloads
            .iter()
            .any(|download| download.line == owner_line)
}

fn add_review_risk_semantic_tokens(report: &CheckReport, builder: &mut SemanticTokenBuilder<'_>) {
    let program = &report.semantic_program;

    for diagnostic in report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
    {
        let classification = classify_diagnostic_review_risk(&diagnostic.code, "warning");
        if let Some(modifier) = review_risk_modifier(classification.level) {
            builder.push_first_identifier_on_line(diagnostic.line, "variable", &[modifier]);
        }
    }

    for schema in &program.schemas {
        for policy in &schema.missing_policies {
            if let Some(modifier) =
                review_risk_modifier(classify_review_risk("data_quality", "info").level)
            {
                builder.push_on_line(policy.line, &policy.column, "property", &[modifier]);
            }
        }
    }

    for process in &program.process_runs {
        let category = if with_option_present(program, process.line, "expected_outputs") {
            "external_boundary"
        } else {
            "reproducibility"
        };
        if let Some(modifier) = review_risk_modifier(classify_review_risk(category, "info").level) {
            builder.push_on_line(process.line, &process.binding, "variable", &[modifier]);
            builder.push_keywords_on_line(process.line, &["run", "command"], &[modifier]);
        }
    }

    for export in &program.csv_exports {
        if let Some(modifier) =
            review_risk_modifier(classify_review_risk("side_effect", "info").level)
        {
            builder.push_on_line(export.line, &export.source, "variable", &[modifier]);
            builder.push_keywords_on_line(export.line, &["export", "csv"], &[modifier]);
        }
    }

    for write in &program.writes {
        if let Some(modifier) =
            review_risk_modifier(classify_review_risk("side_effect", "info").level)
        {
            builder.push_keywords_on_line(write.line, &["write"], &[modifier]);
        }
    }

    for operation in &program.file_operations {
        if let Some(modifier) =
            review_risk_modifier(classify_review_risk("side_effect", "info").level)
        {
            builder.push_keywords_on_line(
                operation.line,
                &[operation.operation.as_str()],
                &[modifier],
            );
        }
    }

    for request in &program.net_requests {
        if let Some(modifier) = review_risk_modifier("medium") {
            builder.push_on_line(request.line, &request.binding, "variable", &[modifier]);
            builder.push_keywords_on_line(
                request.line,
                &["http", http_request_method_keyword(&request.method)],
                &[modifier],
            );
        }
    }

    for download in &program.net_downloads {
        if let Some(modifier) =
            review_risk_modifier(classify_review_risk("external_boundary", "info").level)
        {
            builder.push_keywords_on_line(download.line, &["download"], &[modifier]);
        }
    }

    for dependency in &program.environment_dependencies {
        if let Some(modifier) =
            review_risk_modifier(classify_review_risk("reproducibility", "info").level)
        {
            builder.push_first_identifier_on_line(dependency.line, "variable", &[modifier]);
        }
    }

    for uncertainty in &program.uncertainty_infos {
        if let Some(modifier) =
            review_risk_modifier(classify_review_risk("uncertainty", "info").level)
        {
            builder.push_on_line(
                uncertainty.line,
                &uncertainty.binding,
                "variable",
                &[modifier],
            );
        }
    }

    for system in &program.systems {
        if let Some(modifier) =
            review_risk_modifier(classify_review_risk("solver_or_numeric", "info").level)
        {
            builder.push_on_line(system.line, &system.name, "class", &[modifier]);
        }
    }

    for assembly in &program.component_assemblies {
        if let Some(modifier) =
            review_risk_modifier(classify_review_risk("solver_or_numeric", "info").level)
        {
            builder.push_on_line(assembly.line, &assembly.name, "class", &[modifier]);
        }
    }
}

fn review_risk_modifier(level: &str) -> Option<&'static str> {
    match level {
        "high" => Some("riskHigh"),
        "medium" => Some("riskMedium"),
        _ => None,
    }
}

fn with_option_present(program: &SemanticProgram, owner_line: usize, key: &str) -> bool {
    program.with_blocks.iter().any(|block| {
        block.owner_line == Some(owner_line)
            && block
                .options
                .iter()
                .any(|option| option.key == key && !option.value.trim().is_empty())
    })
}

const SYMBOL_KIND_MODULE: u8 = 2;
const SYMBOL_KIND_CLASS: u8 = 5;
const SYMBOL_KIND_METHOD: u8 = 6;
const SYMBOL_KIND_PROPERTY: u8 = 7;
const SYMBOL_KIND_INTERFACE: u8 = 11;
const SYMBOL_KIND_FUNCTION: u8 = 12;
const SYMBOL_KIND_VARIABLE: u8 = 13;
const SYMBOL_KIND_CONSTANT: u8 = 14;
const SYMBOL_KIND_OBJECT: u8 = 19;
const SYMBOL_KIND_KEY: u8 = 20;
const SYMBOL_KIND_STRUCT: u8 = 23;
const SYMBOL_KIND_OPERATOR: u8 = 25;
const SYMBOL_KIND_TYPE_PARAMETER: u8 = 26;

fn document_symbols(report: &CheckReport, source: &str) -> Vec<LspDocumentSymbol> {
    let lines = source_lines(source);
    let program = &report.semantic_program;
    let mut symbols = Vec::new();
    let mut seen = BTreeSet::<(usize, String)>::new();

    for import in &program.imports {
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            import.target.clone(),
            format!("import {}", import.kind),
            SYMBOL_KIND_MODULE,
            import.line,
            Vec::new(),
        );
    }

    for constant in &program.consts {
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            constant.name.clone(),
            format!(
                "const {} [{}]",
                constant.quantity_kind, constant.display_unit
            ),
            SYMBOL_KIND_CONSTANT,
            constant.line,
            Vec::new(),
        );
    }

    for function in &program.functions {
        let mut children = function
            .parameters
            .iter()
            .map(|parameter| {
                make_document_symbol(
                    &lines,
                    parameter.name.clone(),
                    format!(
                        "parameter {} [{}]",
                        parameter.quantity_kind, parameter.display_unit
                    ),
                    SYMBOL_KIND_TYPE_PARAMETER,
                    function.line,
                    Vec::new(),
                )
            })
            .collect::<Vec<_>>();
        children.extend(function.locals.iter().map(|local| {
            make_document_symbol(
                &lines,
                local.name.clone(),
                "local".to_owned(),
                SYMBOL_KIND_VARIABLE,
                local.line,
                Vec::new(),
            )
        }));
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            function.name.clone(),
            format!(
                "fn -> {} [{}]",
                function.return_quantity_kind, function.return_display_unit
            ),
            SYMBOL_KIND_FUNCTION,
            function.line,
            children,
        );
    }

    for schema in &program.schemas {
        let children = schema
            .columns
            .iter()
            .map(|column| {
                make_document_symbol(
                    &lines,
                    column.name.clone(),
                    match &column.unit {
                        Some(unit) => format!("{} [{}]", column.type_name, unit),
                        None => column.type_name.clone(),
                    },
                    SYMBOL_KIND_PROPERTY,
                    column.line,
                    Vec::new(),
                )
            })
            .collect::<Vec<_>>();
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            schema.name.clone(),
            "schema".to_owned(),
            SYMBOL_KIND_STRUCT,
            schema.line,
            children,
        );
    }

    for promotion in &program.csv_promotions {
        let detail = if promotion.source_format == "json_records" {
            format!("json_records as {}", promotion.schema_name)
        } else {
            format!("csv as {}", promotion.schema_name)
        };
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            promotion.binding.clone(),
            detail,
            SYMBOL_KIND_VARIABLE,
            promotion.line,
            Vec::new(),
        );
    }

    for promotion in &program.config_promotions {
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            promotion.binding.clone(),
            format!("config as {}", promotion.schema_name),
            SYMBOL_KIND_VARIABLE,
            promotion.line,
            Vec::new(),
        );
    }

    for sample in &program.sample_generations {
        let children = sample
            .distributions
            .iter()
            .map(|distribution| {
                make_document_symbol(
                    &lines,
                    distribution.name.clone(),
                    format!(
                        "{} distribution {} [{}]",
                        distribution.kind, distribution.quantity_kind, distribution.display_unit
                    ),
                    SYMBOL_KIND_PROPERTY,
                    distribution.line,
                    Vec::new(),
                )
            })
            .collect::<Vec<_>>();
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            sample.binding.clone(),
            format!("sample {}", sample.method),
            SYMBOL_KIND_VARIABLE,
            sample.line,
            children,
        );
    }

    for transform in &program.table_transforms {
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            transform.binding.clone(),
            "table transform".to_owned(),
            SYMBOL_KIND_VARIABLE,
            transform.line,
            Vec::new(),
        );
    }

    for request in &program.net_requests {
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            request.binding.clone(),
            format!("http {}", http_request_method_keyword(&request.method)),
            SYMBOL_KIND_VARIABLE,
            request.line,
            Vec::new(),
        );
    }

    for system in &program.systems {
        let mut children = system
            .variables
            .iter()
            .map(|variable| {
                make_document_symbol(
                    &lines,
                    variable.name.clone(),
                    format!(
                        "{} {} [{}]",
                        variable.role, variable.quantity_kind, variable.display_unit
                    ),
                    SYMBOL_KIND_VARIABLE,
                    variable.line,
                    Vec::new(),
                )
            })
            .collect::<Vec<_>>();
        children.extend(system.equations.iter().map(|equation| {
            make_document_symbol(
                &lines,
                equation.left.clone(),
                format!("equation {}", equation.relation),
                SYMBOL_KIND_OPERATOR,
                equation.line,
                Vec::new(),
            )
        }));
        children.extend(system.residuals.iter().map(|residual| {
            make_document_symbol(
                &lines,
                residual.name.clone(),
                "residual".to_owned(),
                SYMBOL_KIND_OPERATOR,
                residual.line,
                Vec::new(),
            )
        }));
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            system.name.clone(),
            "system".to_owned(),
            SYMBOL_KIND_CLASS,
            system.line,
            children,
        );
    }

    for domain in &program.domains {
        let mut children = domain
            .variables
            .iter()
            .map(|variable| {
                make_document_symbol(
                    &lines,
                    variable.name.clone(),
                    format!(
                        "{} {} [{}]",
                        variable.role, variable.quantity_kind, variable.display_unit
                    ),
                    SYMBOL_KIND_PROPERTY,
                    variable.line,
                    Vec::new(),
                )
            })
            .collect::<Vec<_>>();
        children.extend(domain.conservations.iter().map(|conservation| {
            make_document_symbol(
                &lines,
                "conservation".to_owned(),
                conservation.status.clone(),
                SYMBOL_KIND_OPERATOR,
                conservation.line,
                Vec::new(),
            )
        }));
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            domain.name.clone(),
            "domain".to_owned(),
            SYMBOL_KIND_INTERFACE,
            domain.line,
            children,
        );
    }

    for component in &program.components {
        let mut children = component
            .ports
            .iter()
            .map(|port| {
                make_document_symbol(
                    &lines,
                    port.name.clone(),
                    format!("port {}", port.domain),
                    SYMBOL_KIND_PROPERTY,
                    port.line,
                    Vec::new(),
                )
            })
            .collect::<Vec<_>>();
        children.extend(component.parameters.iter().map(|parameter| {
            make_document_symbol(
                &lines,
                parameter.name.clone(),
                format!(
                    "parameter {} [{}]",
                    parameter.quantity_kind, parameter.display_unit
                ),
                SYMBOL_KIND_TYPE_PARAMETER,
                parameter.line,
                Vec::new(),
            )
        }));
        children.extend(component.inputs.iter().map(|input| {
            make_document_symbol(
                &lines,
                input.name.clone(),
                format!("input {} [{}]", input.quantity_kind, input.display_unit),
                SYMBOL_KIND_TYPE_PARAMETER,
                input.line,
                Vec::new(),
            )
        }));
        children.extend(component.local_expressions.iter().map(|local| {
            make_document_symbol(
                &lines,
                local.name.clone(),
                format!("local {} [{}]", local.quantity_kind, local.display_unit),
                SYMBOL_KIND_VARIABLE,
                local.line,
                Vec::new(),
            )
        }));
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            component.name.clone(),
            component
                .template_name
                .as_deref()
                .map(|template| format!("component from {template}"))
                .unwrap_or_else(|| "component".to_owned()),
            SYMBOL_KIND_CLASS,
            component.line,
            children,
        );
    }

    for assembly in &program.component_assemblies {
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            assembly.name.clone(),
            format!("assembly {}", assembly.status),
            SYMBOL_KIND_OBJECT,
            assembly.line,
            Vec::new(),
        );
    }

    for class_info in &program.classes {
        let mut children = class_info
            .fields
            .iter()
            .map(|field| {
                make_document_symbol(
                    &lines,
                    field.name.clone(),
                    format!("{} [{}]", field.quantity_kind, field.display_unit),
                    SYMBOL_KIND_PROPERTY,
                    field.line,
                    Vec::new(),
                )
            })
            .collect::<Vec<_>>();
        children.extend(class_info.validations.iter().map(|validation| {
            make_document_symbol(
                &lines,
                "validate".to_owned(),
                validation.status.clone(),
                SYMBOL_KIND_OPERATOR,
                validation.line,
                Vec::new(),
            )
        }));
        children.extend(class_info.methods.iter().map(|method| {
            make_document_symbol(
                &lines,
                method.name.clone(),
                format!(
                    "method -> {} [{}]",
                    method.return_quantity_kind, method.return_display_unit
                ),
                SYMBOL_KIND_METHOD,
                method.line,
                Vec::new(),
            )
        }));
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            class_info.name.clone(),
            "class".to_owned(),
            SYMBOL_KIND_CLASS,
            class_info.line,
            children,
        );
    }

    for object in &program.class_objects {
        let mut children = object
            .fields
            .iter()
            .map(|field| {
                make_document_symbol(
                    &lines,
                    field.name.clone(),
                    format!("{} [{}]", field.quantity_kind, field.display_unit),
                    SYMBOL_KIND_PROPERTY,
                    field.line,
                    Vec::new(),
                )
            })
            .collect::<Vec<_>>();
        children.extend(object.validations.iter().map(|validation| {
            make_document_symbol(
                &lines,
                "validate".to_owned(),
                validation.status.clone(),
                SYMBOL_KIND_OPERATOR,
                validation.line,
                Vec::new(),
            )
        }));
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            object.name.clone(),
            format!("{} object", object.class_name),
            SYMBOL_KIND_OBJECT,
            object.line,
            children,
        );
    }

    for args_block in &program.args_blocks {
        let children = args_block
            .fields
            .iter()
            .map(|field| {
                make_document_symbol(
                    &lines,
                    field.name.clone(),
                    field.type_name.clone(),
                    SYMBOL_KIND_TYPE_PARAMETER,
                    field.line,
                    Vec::new(),
                )
            })
            .collect::<Vec<_>>();
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            args_block.name.clone(),
            "args".to_owned(),
            SYMBOL_KIND_STRUCT,
            args_block.line,
            children,
        );
    }

    for export in &program.csv_exports {
        let children = export
            .fields
            .iter()
            .map(|field| {
                make_document_symbol(
                    &lines,
                    field.name.clone(),
                    format!("{} [{}]", field.quantity_kind, field.display_unit),
                    SYMBOL_KIND_PROPERTY,
                    field.line,
                    Vec::new(),
                )
            })
            .collect::<Vec<_>>();
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            export.source.clone(),
            format!("export {}", export.format),
            SYMBOL_KIND_OBJECT,
            export.line,
            children,
        );
    }

    for process in &program.process_runs {
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            process.binding.clone(),
            "run command".to_owned(),
            SYMBOL_KIND_VARIABLE,
            process.line,
            Vec::new(),
        );
    }

    for test in &program.tests {
        let mut children = test
            .assertions
            .iter()
            .map(|assert| {
                make_document_symbol(
                    &lines,
                    "assert".to_owned(),
                    format!("{} {} {}", assert.left, assert.operator, assert.right),
                    SYMBOL_KIND_OPERATOR,
                    assert.line,
                    Vec::new(),
                )
            })
            .collect::<Vec<_>>();
        children.extend(test.goldens.iter().map(|golden| {
            make_document_symbol(
                &lines,
                golden.artifact.clone(),
                "golden".to_owned(),
                SYMBOL_KIND_KEY,
                golden.line,
                Vec::new(),
            )
        }));
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            test.name.clone(),
            "test".to_owned(),
            SYMBOL_KIND_FUNCTION,
            test.line,
            children,
        );
    }

    for command in &program.command_styles {
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            format!("{} {}", command.verb, command.target),
            command.status.clone(),
            SYMBOL_KIND_FUNCTION,
            command.line,
            Vec::new(),
        );
    }

    for suite in &program.expectation_suites {
        let children = suite
            .expectations
            .iter()
            .map(|expectation| {
                make_document_symbol(
                    &lines,
                    expectation.subject.clone(),
                    expectation.kind.clone(),
                    SYMBOL_KIND_KEY,
                    expectation.line,
                    Vec::new(),
                )
            })
            .collect::<Vec<_>>();
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            suite.binding.clone(),
            format!("expect {}", suite.target),
            SYMBOL_KIND_OBJECT,
            suite.line,
            children,
        );
    }

    for block in &program.where_blocks {
        let children = block
            .bindings
            .iter()
            .map(|binding| {
                make_document_symbol(
                    &lines,
                    binding.name.clone(),
                    format!("{} [{}]", binding.quantity_kind, binding.display_unit),
                    SYMBOL_KIND_VARIABLE,
                    binding.line,
                    Vec::new(),
                )
            })
            .collect::<Vec<_>>();
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            format!("where {}", block.line),
            "where".to_owned(),
            SYMBOL_KIND_OBJECT,
            block.line,
            children,
        );
    }

    for block in &program.with_blocks {
        let children = block
            .options
            .iter()
            .map(|option| {
                make_document_symbol(
                    &lines,
                    option.key.clone(),
                    option.status.clone(),
                    SYMBOL_KIND_KEY,
                    option.line,
                    Vec::new(),
                )
            })
            .collect::<Vec<_>>();
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            format!("with {}", block.line),
            "with".to_owned(),
            SYMBOL_KIND_OBJECT,
            block.line,
            children,
        );
    }

    mark_document_symbols_seen(&symbols, &mut seen);
    for binding in &program.typed_bindings {
        push_document_symbol(
            &mut symbols,
            &mut seen,
            &lines,
            binding.name.clone(),
            format!(
                "{} [{}]",
                binding.semantic_type.quantity_kind, binding.semantic_type.display_unit
            ),
            SYMBOL_KIND_VARIABLE,
            binding.line,
            Vec::new(),
        );
    }

    sort_document_symbols(&mut symbols);
    symbols
}

fn folding_ranges(source: &str) -> Vec<LspFoldingRange> {
    let lines = source_lines(source);
    let mut stack = Vec::<usize>::new();
    let mut ranges = Vec::<LspFoldingRange>::new();

    for (line_index, line) in lines.iter().enumerate() {
        for event in brace_events(line) {
            match event {
                '{' => stack.push(line_index),
                '}' => {
                    if let Some(start_line) = stack.pop() {
                        if line_index > start_line {
                            ranges.push(LspFoldingRange {
                                start_line,
                                end_line: line_index,
                                kind: Some("region".to_owned()),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }

    ranges.sort_by_key(|range| (range.start_line, range.end_line));
    ranges.dedup_by(|right, left| {
        right.start_line == left.start_line
            && right.end_line == left.end_line
            && right.kind == left.kind
    });
    ranges
}

fn push_document_symbol(
    symbols: &mut Vec<LspDocumentSymbol>,
    seen: &mut BTreeSet<(usize, String)>,
    lines: &[&str],
    name: String,
    detail: String,
    kind: u8,
    line: usize,
    children: Vec<LspDocumentSymbol>,
) {
    if line == 0 || !seen.insert((line, name.clone())) {
        return;
    }
    symbols.push(make_document_symbol(
        lines, name, detail, kind, line, children,
    ));
}

fn mark_document_symbols_seen(symbols: &[LspDocumentSymbol], seen: &mut BTreeSet<(usize, String)>) {
    for symbol in symbols {
        seen.insert((symbol.line + 1, symbol.name.clone()));
        mark_document_symbols_seen(&symbol.children, seen);
    }
}

fn make_document_symbol(
    lines: &[&str],
    name: String,
    detail: String,
    kind: u8,
    line_one_based: usize,
    mut children: Vec<LspDocumentSymbol>,
) -> LspDocumentSymbol {
    sort_document_symbols(&mut children);
    let line = line_index_from_one_based(lines, line_one_based);
    let character = first_non_whitespace_utf16(lines[line]);
    let selection_character = symbol_start_utf16(lines[line], &name).unwrap_or(character);
    let mut end_line = block_end_line(lines, line).unwrap_or(line);
    for child in &children {
        if child.end_line > end_line {
            end_line = child.end_line;
        }
    }
    let end_character = line_end_character(lines, end_line);

    LspDocumentSymbol {
        name,
        detail,
        kind,
        line,
        character,
        end_line,
        end_character,
        selection_line: line,
        selection_character,
        children,
    }
}

fn sort_document_symbols(symbols: &mut [LspDocumentSymbol]) {
    symbols.sort_by(|left, right| {
        (left.line, left.character, &left.name).cmp(&(right.line, right.character, &right.name))
    });
    for symbol in symbols {
        sort_document_symbols(&mut symbol.children);
    }
}

fn source_lines(source: &str) -> Vec<&str> {
    let mut lines = source.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push("");
    }
    lines
}

fn line_index_from_one_based(lines: &[&str], line: usize) -> usize {
    line.saturating_sub(1).min(lines.len().saturating_sub(1))
}

fn line_end_character(lines: &[&str], line: usize) -> usize {
    lines.get(line).map(|line| utf16_len(line)).unwrap_or(0)
}

fn first_non_whitespace_utf16(line: &str) -> usize {
    line.char_indices()
        .find(|(_, character)| !character.is_whitespace())
        .map(|(byte_index, _)| utf16_len(&line[..byte_index]))
        .unwrap_or(0)
}

fn symbol_start_utf16(line: &str, name: &str) -> Option<usize> {
    let byte_index = find_symbol_start(line, name)?;
    Some(utf16_len(&line[..byte_index]))
}

fn find_symbol_start(line: &str, name: &str) -> Option<usize> {
    if name.is_empty() {
        return None;
    }
    let requires_identifier_boundary = name.as_bytes().iter().all(|byte| is_ident_byte(*byte));
    let mut search_start = 0usize;
    while search_start <= line.len() {
        let Some(offset) = line[search_start..].find(name) else {
            break;
        };
        let start = search_start + offset;
        let end = start + name.len();
        if !requires_identifier_boundary || is_identifier_boundary(line, start, end) {
            return Some(start);
        }
        search_start = end;
    }
    None
}

fn block_end_line(lines: &[&str], start_line: usize) -> Option<usize> {
    if !brace_events(lines.get(start_line)?)
        .iter()
        .any(|event| *event == '{')
    {
        return None;
    }
    let mut depth = 0usize;
    let mut opened = false;
    for (line_index, line) in lines.iter().enumerate().skip(start_line) {
        for event in brace_events(line) {
            match event {
                '{' => {
                    opened = true;
                    depth += 1;
                }
                '}' if opened => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(line_index);
                    }
                }
                _ => {}
            }
        }
    }
    opened.then(|| lines.len().saturating_sub(1))
}

fn brace_events(line: &str) -> Vec<char> {
    let limit = comment_start(line).unwrap_or(line.len());
    let bytes = line.as_bytes();
    let mut events = Vec::new();
    let mut in_string = false;
    let mut index = 0usize;
    while index < limit {
        if bytes[index] == b'\\' && in_string {
            index += 2;
            continue;
        }
        if bytes[index] == b'"' {
            in_string = !in_string;
            index += 1;
            continue;
        }
        if !in_string {
            match bytes[index] {
                b'{' => events.push('{'),
                b'}' => events.push('}'),
                _ => {}
            }
        }
        index += 1;
    }
    events
}

fn semantic_modifiers_for_quantity(quantity_kind: &str) -> Vec<&'static str> {
    let mut modifiers = Vec::new();
    if quantity_kind.contains("TimeSeries") {
        modifiers.push("timeseries");
    }
    if quantity_kind.contains("Uncertain") || quantity_kind.contains("Interval") {
        modifiers.push("uncertain");
    }
    if is_model_quantity_kind(quantity_kind) {
        modifiers.push("model");
    }
    if quantity_kind.contains("Db") {
        modifiers.push("db");
    }
    if quantity_kind.contains("Table[Case") || quantity_kind.contains("CaseOutput") {
        modifiers.push("workflowStep");
    }
    modifiers
}

fn stdlib_import_semantic_modifiers(target: &str) -> Vec<&'static str> {
    let mut modifiers = vec!["declaration", "imported"];
    if !target.starts_with("eng.") {
        return modifiers;
    }
    let Ok(registry) = bundled_module_registry() else {
        return modifiers;
    };
    let Some(module) = registry.modules.iter().find(|module| module.name == target) else {
        return modifiers;
    };
    modifiers.push("defaultLibrary");
    match module.status.as_str() {
        "planned" => modifiers.push("planned"),
        "internal" | "internal_planned" => modifiers.push("internal"),
        _ => {}
    }
    modifiers
}

fn is_model_quantity_kind(quantity_kind: &str) -> bool {
    quantity_kind.contains("Model")
        || quantity_kind.contains("Prediction")
        || quantity_kind.contains("TrainTestSplit")
        || quantity_kind.contains("LeakageLint")
}

struct SemanticTokenBuilder<'a> {
    lines: Vec<&'a str>,
    tokens: Vec<LspSemanticToken>,
    token_keys: BTreeMap<(usize, usize, usize, String), usize>,
}

impl<'a> SemanticTokenBuilder<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            lines: source.lines().collect(),
            tokens: Vec::new(),
            token_keys: BTreeMap::new(),
        }
    }

    fn finish(mut self) -> LspSemanticTokens {
        self.tokens.sort();
        LspSemanticTokens {
            legend: semantic_legend(),
            tokens: self.tokens,
        }
    }

    fn add_lexical_tokens(&mut self) {
        let quantity_names = all_quantity_completions()
            .iter()
            .map(|quantity| quantity.quantity_kind)
            .collect::<BTreeSet<_>>();
        let public_types = PUBLIC_TYPE_COMPLETIONS
            .iter()
            .map(|(type_name, _)| public_type_completion_base(type_name))
            .collect::<BTreeSet<_>>();
        let generic_type_bases = PUBLIC_TYPE_COMPLETIONS
            .iter()
            .filter_map(|(type_name, _)| public_generic_type_base(type_name))
            .collect::<BTreeSet<_>>();
        let units = {
            let mut units = all_unit_infos()
                .iter()
                .map(|unit| unit.symbol)
                .collect::<Vec<_>>();
            units.sort_by_key(|unit| std::cmp::Reverse(unit.len()));
            units
        };

        for line_index in 0..self.lines.len() {
            let line = self.lines[line_index];
            if let Some(comment_start) = comment_start(line) {
                self.push_byte_range(
                    line_index,
                    comment_start,
                    line.len().saturating_sub(comment_start),
                    "comment",
                    &[],
                );
            }

            for (start, end) in code_ranges(line) {
                self.scan_word_tokens(line_index, start, end, &quantity_names, &public_types);
                self.scan_legacy_declaration_names(line_index, start, end);
                self.scan_hyphenated_workflow_builtin_tokens(line_index, start, end);
                self.scan_generic_type_tokens(line_index, start, end, &generic_type_bases);
                self.scan_unit_tokens(line_index, start, end, &units);
                self.scan_number_tokens(line_index, start, end);
            }
        }
    }

    fn scan_word_tokens(
        &mut self,
        line_index: usize,
        start: usize,
        end: usize,
        quantity_names: &BTreeSet<&str>,
        public_types: &BTreeSet<&str>,
    ) {
        let line = self.lines[line_index];
        let bytes = line.as_bytes();
        let mut index = start;
        while index < end {
            if index < bytes.len() && is_ident_start(bytes[index]) {
                let token_start = index;
                index += 1;
                while index < end && index < bytes.len() && is_ident_byte(bytes[index]) {
                    index += 1;
                }
                let token = &line[token_start..index];
                if WORKFLOW_BUILTIN_KEYWORDS.contains(&token) {
                    self.push_byte_range(
                        line_index,
                        token_start,
                        index - token_start,
                        "function",
                        workflow_builtin_modifiers_for_line(line, token, token_start),
                    );
                } else if COMPLETION_KEYWORDS.contains(&token) {
                    self.push_byte_range(
                        line_index,
                        token_start,
                        index - token_start,
                        "keyword",
                        keyword_modifiers(token),
                    );
                } else if LANGUAGE_CONSTANT_KEYWORDS.contains(&token) {
                    self.push_byte_range(
                        line_index,
                        token_start,
                        index - token_start,
                        "keyword",
                        language_constant_modifiers(token),
                    );
                } else if quantity_names.contains(token) {
                    self.push_byte_range(
                        line_index,
                        token_start,
                        index - token_start,
                        "type",
                        &["quantity"],
                    );
                } else if public_types.contains(token) {
                    self.push_byte_range(line_index, token_start, index - token_start, "type", &[]);
                }
                continue;
            }
            index += 1;
        }
    }

    fn scan_legacy_declaration_names(&mut self, line_index: usize, start: usize, end: usize) {
        let line = self.lines[line_index];
        let bytes = line.as_bytes();
        for keyword in ["script", "struct"] {
            let mut search_start = start;
            while search_start < end {
                let Some(relative_start) = line[search_start..end].find(keyword) else {
                    break;
                };
                let keyword_start = search_start + relative_start;
                let keyword_end = keyword_start + keyword.len();
                search_start = keyword_end;
                if !is_identifier_boundary(line, keyword_start, keyword_end) {
                    continue;
                }
                let name_start = skip_ascii_whitespace(bytes, keyword_end, end);
                if name_start >= end
                    || name_start >= bytes.len()
                    || !is_ident_start(bytes[name_start])
                {
                    continue;
                }
                let mut name_end = name_start + 1;
                while name_end < end && name_end < bytes.len() && is_ident_byte(bytes[name_end]) {
                    name_end += 1;
                }
                self.push_byte_range(
                    line_index,
                    name_start,
                    name_end - name_start,
                    "class",
                    &["declaration", "deprecated"],
                );
            }
        }
    }

    fn scan_generic_type_tokens(
        &mut self,
        line_index: usize,
        start: usize,
        end: usize,
        generic_type_bases: &BTreeSet<&str>,
    ) {
        let line = self.lines[line_index];
        let bytes = line.as_bytes();
        let mut index = start;
        while index < end {
            if index >= bytes.len() || !is_ident_start(bytes[index]) {
                index += 1;
                continue;
            }

            let base_start = index;
            index += 1;
            while index < end && index < bytes.len() && is_ident_byte(bytes[index]) {
                index += 1;
            }
            let base = &line[base_start..index];
            if !generic_type_bases.contains(base) {
                continue;
            }

            let mut cursor = skip_ascii_whitespace(bytes, index, end);
            if cursor >= end || cursor >= bytes.len() || bytes[cursor] != b'[' {
                continue;
            }
            cursor += 1;
            let mut bracket_depth = 1usize;
            while cursor < end && cursor < bytes.len() && bracket_depth > 0 {
                match bytes[cursor] {
                    b'[' => {
                        bracket_depth += 1;
                        cursor += 1;
                    }
                    b']' => {
                        bracket_depth -= 1;
                        cursor += 1;
                    }
                    byte if is_ident_start(byte) => {
                        let argument_start = cursor;
                        cursor += 1;
                        while cursor < end && cursor < bytes.len() && is_ident_byte(bytes[cursor]) {
                            cursor += 1;
                        }
                        self.push_byte_range(
                            line_index,
                            argument_start,
                            cursor - argument_start,
                            "type",
                            &[],
                        );
                    }
                    _ => {
                        cursor += 1;
                    }
                }
            }
        }
    }

    fn scan_hyphenated_workflow_builtin_tokens(
        &mut self,
        line_index: usize,
        start: usize,
        end: usize,
    ) {
        let line = self.lines[line_index];
        for token in HYPHENATED_WORKFLOW_BUILTIN_KEYWORDS {
            let mut search_start = start;
            while search_start < end {
                let Some(relative_start) = line[search_start..end].find(token) else {
                    break;
                };
                let token_start = search_start + relative_start;
                let token_end = token_start + token.len();
                if is_identifier_boundary(line, token_start, token_end) {
                    self.push_byte_range(
                        line_index,
                        token_start,
                        token.len(),
                        "function",
                        workflow_builtin_modifiers(token),
                    );
                }
                search_start = token_end;
            }
        }
    }

    fn scan_unit_tokens(&mut self, line_index: usize, start: usize, end: usize, units: &[&str]) {
        let line = self.lines[line_index];
        let mut occupied = Vec::<(usize, usize)>::new();
        for unit in units {
            let mut search_start = start;
            while search_start < end {
                let Some(relative) = line[search_start..end].find(unit) else {
                    break;
                };
                let unit_start = search_start + relative;
                let unit_end = unit_start + unit.len();
                search_start = unit_end;
                if !is_unit_boundary(line, unit_start, unit_end) {
                    continue;
                }
                if occupied
                    .iter()
                    .any(|(left, right)| ranges_overlap(unit_start, unit_end, *left, *right))
                {
                    continue;
                }
                occupied.push((unit_start, unit_end));
                self.push_byte_range(line_index, unit_start, unit.len(), "type", &["unit"]);
            }
        }
    }

    fn scan_number_tokens(&mut self, line_index: usize, start: usize, end: usize) {
        let line = self.lines[line_index];
        let bytes = line.as_bytes();
        let mut index = start;
        while index < end {
            if index < bytes.len() && bytes[index].is_ascii_digit() {
                let token_start = index;
                index += 1;
                while index < end && index < bytes.len() && bytes[index].is_ascii_digit() {
                    index += 1;
                }
                if index < end && index < bytes.len() && bytes[index] == b'.' {
                    index += 1;
                    while index < end && index < bytes.len() && bytes[index].is_ascii_digit() {
                        index += 1;
                    }
                }
                self.push_byte_range(line_index, token_start, index - token_start, "number", &[]);
                continue;
            }
            index += 1;
        }
    }

    fn push_keywords_on_line(
        &mut self,
        line_one_based: usize,
        labels: &[&str],
        modifiers: &[&str],
    ) {
        for label in labels {
            self.push_on_line(line_one_based, label, "keyword", modifiers);
        }
    }

    fn push_on_line(
        &mut self,
        line_one_based: usize,
        label: &str,
        token_type: &str,
        modifiers: &[&str],
    ) {
        if label.trim().is_empty() {
            return;
        }
        let Some(line_index) = line_one_based.checked_sub(1) else {
            return;
        };
        let Some(line) = self.lines.get(line_index).copied() else {
            return;
        };
        let candidates = token_candidates(label);
        for candidate in candidates {
            if let Some(start) = find_token_in_line(line, &candidate) {
                self.push_byte_range(line_index, start, candidate.len(), token_type, modifiers);
                return;
            }
        }
    }

    fn push_identifiers_on_line(
        &mut self,
        line_index: usize,
        identifiers: &[&str],
        token_type: &str,
        modifiers: &[&str],
    ) {
        if identifiers.is_empty() {
            return;
        }
        let Some(line) = self.lines.get(line_index).copied() else {
            return;
        };
        let bytes = line.as_bytes();
        for (range_start, range_end) in code_ranges(line) {
            let mut index = range_start;
            while index < range_end {
                if index < bytes.len() && is_ident_start(bytes[index]) {
                    let token_start = index;
                    index += 1;
                    while index < range_end && index < bytes.len() && is_ident_byte(bytes[index]) {
                        index += 1;
                    }
                    let token = &line[token_start..index];
                    if identifiers.iter().any(|identifier| *identifier == token) {
                        self.push_byte_range(
                            line_index,
                            token_start,
                            index - token_start,
                            token_type,
                            modifiers,
                        );
                    }
                    continue;
                }
                index += 1;
            }
        }
    }

    fn push_member_fields(
        &mut self,
        receiver: &str,
        fields: &[(&str, &str)],
        receiver_modifiers: &[&str],
    ) {
        if receiver.trim().is_empty() {
            return;
        }
        let needle = format!("{receiver}.");
        for line_index in 0..self.lines.len() {
            let line = self.lines[line_index];
            for (range_start, range_end) in code_ranges(line) {
                let mut search_start = range_start;
                while search_start < range_end {
                    let Some(relative) = line[search_start..range_end].find(&needle) else {
                        break;
                    };
                    let receiver_start = search_start + relative;
                    let receiver_end = receiver_start + receiver.len();
                    let field_start = receiver_end + 1;
                    search_start = field_start;
                    if !member_receiver_boundary(line, receiver_start) {
                        continue;
                    }
                    let bytes = line.as_bytes();
                    if field_start >= range_end
                        || field_start >= bytes.len()
                        || !is_ident_start(bytes[field_start])
                    {
                        continue;
                    }
                    let mut field_end = field_start + 1;
                    while field_end < range_end
                        && field_end < bytes.len()
                        && is_ident_byte(bytes[field_end])
                    {
                        field_end += 1;
                    }
                    let field = &line[field_start..field_end];
                    if !fields.iter().any(|(candidate, _)| *candidate == field) {
                        continue;
                    }
                    self.push_byte_range(
                        line_index,
                        receiver_start,
                        receiver.len(),
                        "variable",
                        receiver_modifiers,
                    );
                    self.push_byte_range(
                        line_index,
                        field_start,
                        field_end - field_start,
                        "property",
                        &[],
                    );
                    search_start = field_end;
                }
            }
        }
    }

    fn push_first_identifier_on_line(
        &mut self,
        line_one_based: usize,
        token_type: &str,
        modifiers: &[&str],
    ) {
        let Some(line_index) = line_one_based.checked_sub(1) else {
            return;
        };
        let Some(line) = self.lines.get(line_index).copied() else {
            return;
        };
        let bytes = line.as_bytes();
        for (range_start, range_end) in code_ranges(line) {
            let mut index = range_start;
            while index < range_end {
                if index < bytes.len() && is_ident_start(bytes[index]) {
                    let token_start = index;
                    index += 1;
                    while index < range_end && index < bytes.len() && is_ident_byte(bytes[index]) {
                        index += 1;
                    }
                    let token = &line[token_start..index];
                    if !COMPLETION_KEYWORDS.contains(&token) {
                        self.push_byte_range(
                            line_index,
                            token_start,
                            index - token_start,
                            token_type,
                            modifiers,
                        );
                        return;
                    }
                    continue;
                }
                index += 1;
            }
        }
    }

    fn push_byte_range(
        &mut self,
        line: usize,
        byte_start: usize,
        byte_length: usize,
        token_type: &str,
        modifiers: &[&str],
    ) {
        if byte_length == 0 || !SEMANTIC_TOKEN_TYPES.contains(&token_type) {
            return;
        }
        let Some(line_text) = self.lines.get(line).copied() else {
            return;
        };
        if byte_start >= line_text.len() || byte_start + byte_length > line_text.len() {
            return;
        }
        let start = utf16_len(&line_text[..byte_start]);
        let length = utf16_len(&line_text[byte_start..byte_start + byte_length]);
        let key = (line, start, length, token_type.to_owned());
        let modifiers = modifiers
            .iter()
            .copied()
            .filter(|modifier| SEMANTIC_TOKEN_MODIFIERS.contains(modifier))
            .collect::<Vec<_>>();
        if let Some(index) = self.token_keys.get(&key).copied() {
            for modifier in modifiers {
                if !self.tokens[index]
                    .modifiers
                    .iter()
                    .any(|existing| existing == modifier)
                {
                    self.tokens[index].modifiers.push(modifier.to_owned());
                }
            }
            return;
        }
        self.token_keys.insert(key, self.tokens.len());
        self.tokens.push(LspSemanticToken {
            line,
            start,
            length,
            token_type: token_type.to_owned(),
            modifiers: modifiers
                .into_iter()
                .map(|modifier| modifier.to_owned())
                .collect(),
        });
    }
}

fn keyword_modifiers(keyword: &str) -> &'static [&'static str] {
    match keyword {
        "open" | "sqlite" => &["sideEffect", "external", "db"],
        "commit" | "rollback" => &["db"],
        "run" | "command" | "http" | "get" | "post" | "put" | "patch" | "head" | "request"
        | "fetch" | "download" => &["sideEffect", "external"],
        "write" | "export" | "copy" | "move" | "delete" | "render" | "template" => &["sideEffect"],
        "read" | "filter" | "select" | "derive" | "sort" | "require_one" | "column" | "columns"
        | "materialize" | "apply" | "collect" | "promote" | "records" | "results" | "cases"
        | "text" | "csv" | "json" | "toml" => &["workflowStep"],
        "using" => &["model"],
        "report" | "show" | "plot" | "line" | "bar" | "histogram" | "summarize" | "summary"
        | "distribution" | "print" | "log" => &["report"],
        "validate" | "check" | "assert" | "golden" | "test" | "matches" | "within"
        | "constraints" | "missing" | "interpolate" | "monotonic" | "between" => &["validation"],
        "simulate" | "solve" | "connect" | "conservation" | "equation" | "operator" | "states"
        | "inputs" | "outputs" => &["solver"],
        "script" | "struct" => &["deprecated"],
        _ => &[],
    }
}

fn language_constant_modifiers(keyword: &str) -> &'static [&'static str] {
    match keyword {
        "cached" | "stale" | "hit" | "miss" => &["cache"],
        "created" | "updated" | "metadata_ready" | "warnings_present" | "diagnostics_present" => {
            &["workflowStep"]
        }
        "fixed_step"
        | "rk4"
        | "adaptive_heun"
        | "fixed_point"
        | "newton"
        | "implicit_euler_dae"
        | "dynamic_component_explicit_euler"
        | "dynamic_component_semi_implicit_euler"
        | "dynamic_component_adaptive_heun"
        | "trapezoidal" => &["solver"],
        _ => &[],
    }
}

fn http_request_method_keyword(method: &str) -> &'static str {
    match method.to_ascii_uppercase().as_str() {
        "POST" => "post",
        "PUT" => "put",
        "PATCH" => "patch",
        "HEAD" => "head",
        "REQUEST" => "request",
        "FETCH" => "fetch",
        _ => "get",
    }
}

fn workflow_builtin_modifiers(keyword: &str) -> &'static [&'static str] {
    match keyword {
        "sample" | "grid" | "random" | "lhs" | "latin_hypercube" | "latin-hypercube" => {
            &["defaultLibrary", "workflowStep"]
        }
        "filter" | "select" | "derive" | "sort" | "require_one" => {
            &["defaultLibrary", "workflowStep"]
        }
        "normal" | "uniform" | "distribution" => &["defaultLibrary", "uncertain"],
        "materialize" | "apply" | "collect" | "run_case" => &["defaultLibrary", "workflowStep"],
        "measured" | "interval" | "ensemble" | "propagate" | "probability" => {
            &["defaultLibrary", "uncertain"]
        }
        "train" | "train_test_split" | "regression" | "regression_table" | "train_regression"
        | "mlp" | "ann" | "evaluate" | "model_card" | "leakage_lint" | "predict" => {
            &["defaultLibrary", "model"]
        }
        "integrate" | "der" | "delay" | "sum" => &["defaultLibrary", "solver"],
        "fill" => &["defaultLibrary", "validation", "workflowStep"],
        "check" | "coverage" | "fill_missing" | "align" | "resample" | "rmse" => {
            &["defaultLibrary", "validation"]
        }
        "select_first_row" => &["defaultLibrary", "deprecated"],
        _ => &["defaultLibrary"],
    }
}

fn workflow_builtin_modifiers_for_line(
    line: &str,
    keyword: &str,
    token_start: usize,
) -> &'static [&'static str] {
    if keyword == "uniform"
        && previous_identifier_before(line, token_start)
            .is_some_and(|previous| previous == "sample")
    {
        return &["defaultLibrary", "uncertain", "workflowStep"];
    }
    if keyword == "distribution"
        && previous_identifier_before(line, token_start).is_some_and(|previous| previous == "plot")
    {
        return &["defaultLibrary", "report"];
    }
    if keyword == "distribution"
        && next_non_whitespace_after(line, token_start + keyword.len()) != Some('(')
    {
        return &["defaultLibrary", "report"];
    }
    if keyword == "join" && is_table_join_phrase(line, token_start) {
        return &["defaultLibrary", "workflowStep"];
    }
    workflow_builtin_modifiers(keyword)
}

fn is_table_join_phrase(line: &str, token_start: usize) -> bool {
    let Some(after_join) = line.get(token_start + "join".len()..) else {
        return false;
    };
    let mut parts = after_join.split_whitespace();
    let Some(left_table) = parts.next() else {
        return false;
    };
    let Some(with_keyword) = parts.next() else {
        return false;
    };
    let Some(right_table) = parts.next() else {
        return false;
    };
    is_simple_identifier_path(left_table)
        && with_keyword == "with"
        && is_simple_identifier_path(right_table)
}

fn is_simple_identifier_path(value: &str) -> bool {
    let mut segments = value.split('.');
    segments.next().is_some_and(is_simple_identifier_segment)
        && segments.all(is_simple_identifier_segment)
}

fn is_simple_identifier_segment(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    is_ident_start(first) && bytes.all(is_ident_byte)
}

fn previous_identifier_before(line: &str, token_start: usize) -> Option<&str> {
    let bytes = line.as_bytes();
    let mut index = token_start.min(bytes.len());
    while index > 0 && bytes[index - 1].is_ascii_whitespace() {
        index -= 1;
    }
    let end = index;
    while index > 0 && is_ident_byte(bytes[index - 1]) {
        index -= 1;
    }
    (index < end && is_ident_start(bytes[index])).then_some(&line[index..end])
}

fn next_non_whitespace_after(line: &str, start: usize) -> Option<char> {
    line.get(start..)?
        .chars()
        .find(|character| !character.is_whitespace())
}

fn token_candidates(label: &str) -> Vec<String> {
    let trimmed = label.trim().trim_end_matches("()");
    let mut candidates = Vec::new();
    candidates.push(trimmed.to_owned());
    if let Some(last) = trimmed.rsplit('.').next() {
        if last != trimmed {
            candidates.push(last.to_owned());
        }
    }
    candidates
}

fn find_token_in_line(line: &str, token: &str) -> Option<usize> {
    let mut search_start = 0usize;
    while search_start <= line.len() {
        let relative = line[search_start..].find(token)?;
        let start = search_start + relative;
        let end = start + token.len();
        if is_identifier_boundary(line, start, end) || is_unit_boundary(line, start, end) {
            return Some(start);
        }
        search_start = end;
    }
    None
}

fn code_ranges(line: &str) -> Vec<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut ranges = Vec::new();
    let mut start = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"///") {
            if start < index {
                ranges.push((start, index));
            }
            return ranges;
        }
        if bytes[index] == b'#' {
            if start < index {
                ranges.push((start, index));
            }
            return ranges;
        }
        if bytes[index] == b'"' {
            if start < index {
                ranges.push((start, index));
            }
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index += 2;
                    continue;
                }
                if bytes[index] == b'"' {
                    index += 1;
                    break;
                }
                index += 1;
            }
            start = index;
            continue;
        }
        index += 1;
    }
    if start < bytes.len() {
        ranges.push((start, bytes.len()));
    }
    ranges
}

fn comment_start(line: &str) -> Option<usize> {
    let mut in_string = false;
    let bytes = line.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'\\' && in_string {
            index += 2;
            continue;
        }
        if bytes[index] == b'"' {
            in_string = !in_string;
            index += 1;
            continue;
        }
        if !in_string && bytes[index..].starts_with(b"///") {
            return Some(index);
        }
        if !in_string && bytes[index] == b'#' {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn is_identifier_boundary(line: &str, start: usize, end: usize) -> bool {
    let bytes = line.as_bytes();
    let before = start
        .checked_sub(1)
        .and_then(|index| bytes.get(index).copied());
    let after = bytes.get(end).copied();
    before.is_none_or(|byte| !is_ident_byte(byte)) && after.is_none_or(|byte| !is_ident_byte(byte))
}

fn member_receiver_boundary(line: &str, start: usize) -> bool {
    let bytes = line.as_bytes();
    start
        .checked_sub(1)
        .and_then(|index| bytes.get(index).copied())
        .is_none_or(|byte| !is_ident_byte(byte))
}

fn is_unit_boundary(line: &str, start: usize, end: usize) -> bool {
    let bytes = line.as_bytes();
    let before = start
        .checked_sub(1)
        .and_then(|index| bytes.get(index).copied());
    let after = bytes.get(end).copied();
    before.is_none_or(|byte| !is_unit_byte(byte)) && after.is_none_or(|byte| !is_unit_byte(byte))
}

fn is_unit_byte(byte: u8) -> bool {
    byte == b'_' || byte == b'/' || byte == b'^' || byte.is_ascii_alphanumeric()
}

fn is_ident_start(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphabetic()
}

fn public_type_completion_base(type_name: &str) -> &str {
    type_name
        .split_once('[')
        .map_or(type_name, |(base, _)| base)
}

fn public_generic_type_base(type_name: &str) -> Option<&str> {
    type_name.split_once('[').map(|(base, _)| base)
}

fn skip_ascii_whitespace(bytes: &[u8], mut index: usize, end: usize) -> usize {
    while index < end && index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    index
}

fn ranges_overlap(
    left_start: usize,
    left_end: usize,
    right_start: usize,
    right_end: usize,
) -> bool {
    left_start < right_end && right_start < left_end
}

fn utf16_len(value: &str) -> usize {
    value.encode_utf16().count()
}

pub fn diagnostic_json(diagnostic: &LspDiagnostic) -> Value {
    json!({
        "range": {
            "start": {
                "line": diagnostic.line.saturating_sub(1),
                "character": diagnostic.start_character
            },
            "end": {
                "line": diagnostic.line.saturating_sub(1),
                "character": diagnostic.end_character
            }
        },
        "severity": lsp_severity(&diagnostic.severity),
        "source": "eng",
        "code": diagnostic.code,
        "message": match &diagnostic.help {
            Some(help) => format!("{}\n{}", diagnostic.message, help),
            None => diagnostic.message.clone(),
        }
    })
}

pub fn completion_json(completion: &LspCompletion) -> Value {
    json!({
        "label": completion.label,
        "kind": completion_kind(&completion.kind),
        "detail": completion.detail,
    })
}

pub fn editor_completion_json(completion: &LspCompletion) -> Value {
    json!({
        "label": completion.label,
        "kind": completion.kind,
        "lsp_kind": completion_kind(&completion.kind),
        "detail": completion.detail,
    })
}

pub fn hover_json(hover: &LspHover) -> Value {
    let mut value = format!(
        "**{}**\n\nKind: `{}`\n\n{}\n\nQuantity: `{}`\n\nDisplay unit: `{}`",
        hover.name, hover.kind, hover.detail, hover.quantity_kind, hover.display_unit
    );
    if let Some(status) = &hover.status {
        value.push_str(&format!("\n\nStatus: `{status}`"));
    }
    json!({
        "name": hover.name,
        "kind": hover.kind,
        "line": hover.line,
        "quantity_kind": hover.quantity_kind,
        "display_unit": hover.display_unit,
        "status": hover.status,
        "contents": {
            "kind": "markdown",
            "value": value
        }
    })
}

pub fn hover_items(report: &CheckReport) -> Vec<LspHover> {
    let mut hovers = report
        .semantic_program
        .hover_hints
        .iter()
        .map(|hover| LspHover {
            name: hover.name.clone(),
            kind: "variable".to_owned(),
            line: hover.line,
            detail: hover.detail.clone(),
            quantity_kind: hover.quantity_kind.clone(),
            display_unit: hover.display_unit.clone(),
            status: None,
        })
        .collect::<Vec<_>>();

    for domain in &report.semantic_program.domains {
        hovers.push(LspHover {
            name: domain.name.clone(),
            kind: "domain".to_owned(),
            line: domain.line,
            detail: format!(
                "{}, {} variable(s), {} conservation contract(s), package {}, version {}",
                domain_signature(&domain.name, &domain.type_parameters),
                domain.variables.len(),
                domain.conservations.len(),
                domain.package.as_deref().unwrap_or("-"),
                domain.version.as_deref().unwrap_or("-")
            ),
            quantity_kind: "domain".to_owned(),
            display_unit: "-".to_owned(),
            status: Some("metadata".to_owned()),
        });
        for variable in &domain.variables {
            hovers.push(LspHover {
                name: format!("{}.{}", domain.name, variable.name),
                kind: "domain_variable".to_owned(),
                line: variable.line,
                detail: format!(
                    "{} variable in domain {}; canonical unit {}; dimension {}",
                    variable.role, domain.name, variable.canonical_unit, variable.dimension
                ),
                quantity_kind: variable.quantity_kind.clone(),
                display_unit: variable.display_unit.clone(),
                status: None,
            });
        }
        for conservation in &domain.conservations {
            hovers.push(LspHover {
                name: format!("{}.conservation", domain.name),
                kind: "domain_conservation".to_owned(),
                line: conservation.line,
                detail: conservation.text.clone(),
                quantity_kind: "conservation".to_owned(),
                display_unit: "-".to_owned(),
                status: Some(conservation.status.clone()),
            });
        }
    }

    for component in &report.semantic_program.components {
        hovers.push(LspHover {
            name: component.name.clone(),
            kind: "component".to_owned(),
            line: component.line,
            detail: format!("{} port(s)", component.ports.len()),
            quantity_kind: "component".to_owned(),
            display_unit: "-".to_owned(),
            status: Some("metadata".to_owned()),
        });
        for port in &component.ports {
            hovers.push(LspHover {
                name: format!("{}.{}", component.name, port.name),
                kind: "component_port".to_owned(),
                line: port.line,
                detail: format!(
                    "port {} on component {}; {}",
                    port.name,
                    component.name,
                    port_metadata_detail(port, &report.semantic_program.domains)
                ),
                quantity_kind: "port".to_owned(),
                display_unit: port.domain.clone(),
                status: Some(port.status.clone()),
            });
        }
    }

    for connection in &report.semantic_program.connections {
        hovers.push(LspHover {
            name: format!("{} -> {}", connection.left, connection.right),
            kind: "connection".to_owned(),
            line: connection.line,
            detail: format!(
                "connects {} to {} in domain {}",
                connection.left, connection.right, connection.domain
            ),
            quantity_kind: connection.domain.clone(),
            display_unit: "-".to_owned(),
            status: Some(connection.status.clone()),
        });
    }

    for assembly in &report.semantic_program.component_assemblies {
        hovers.push(LspHover {
            name: assembly.name.clone(),
            kind: "component_assembly".to_owned(),
            line: assembly.line,
            detail: format!(
                "{} connection set(s), {} generated equation(s), {} unknown(s), balance {}",
                assembly.connection_sets.len(),
                assembly.equations.len(),
                assembly.boundary.unknown_count,
                assembly.boundary.balance_status
            ),
            quantity_kind: "assembly".to_owned(),
            display_unit: "-".to_owned(),
            status: Some(assembly.status.clone()),
        });
        for connection_set in &assembly.connection_sets {
            hovers.push(LspHover {
                name: connection_set.name.clone(),
                kind: "connection_set".to_owned(),
                line: connection_set.line,
                detail: format!(
                    "{} port(s) in domain {}: {}",
                    connection_set.ports.len(),
                    connection_set.domain,
                    string_list(&connection_set.ports)
                ),
                quantity_kind: connection_set.domain.clone(),
                display_unit: "-".to_owned(),
                status: Some(connection_set.status.clone()),
            });
        }
        for equation in &assembly.equations {
            hovers.push(LspHover {
                name: equation.name.clone(),
                kind: "assembly_equation".to_owned(),
                line: equation.line,
                detail: format!(
                    "{}; residual {}; dependencies {}",
                    equation.expression,
                    equation.residual,
                    string_list(&equation.dependencies)
                ),
                quantity_kind: equation.domain.clone(),
                display_unit: "-".to_owned(),
                status: Some(equation.status.clone()),
            });
        }
    }

    for function in &report.semantic_program.functions {
        hovers.push(LspHover {
            name: function.name.clone(),
            kind: "function".to_owned(),
            line: function.line,
            detail: function_signature_detail(function),
            quantity_kind: function.return_quantity_kind.clone(),
            display_unit: function.return_display_unit.clone(),
            status: Some(function.status.clone()),
        });
        for local in &function.locals {
            hovers.push(LspHover {
                name: format!("{}.{}", function.name, local.name),
                kind: "function_local".to_owned(),
                line: local.line,
                detail: format!(
                    "local `{}` in function `{}` = {}",
                    local.name, function.name, local.expression
                ),
                quantity_kind: "local".to_owned(),
                display_unit: "-".to_owned(),
                status: Some("function_scope".to_owned()),
            });
        }
    }

    for block in &report.semantic_program.where_blocks {
        for binding in &block.bindings {
            hovers.push(LspHover {
                name: format!("where.{}", binding.name),
                kind: "where_local".to_owned(),
                line: binding.line,
                detail: format!(
                    "where local `{}` = {}; owner line {}; status {}",
                    binding.name,
                    binding.expression,
                    block
                        .owner_line
                        .map(|line| line.to_string())
                        .unwrap_or_else(|| "-".to_owned()),
                    binding.status
                ),
                quantity_kind: binding.quantity_kind.clone(),
                display_unit: binding.display_unit.clone(),
                status: Some(binding.status.clone()),
            });
        }
    }

    for class_info in &report.semantic_program.classes {
        hovers.push(LspHover {
            name: class_info.name.clone(),
            kind: "class".to_owned(),
            line: class_info.line,
            detail: format!("class with {} field(s)", class_info.fields.len()),
            quantity_kind: "class".to_owned(),
            display_unit: "-".to_owned(),
            status: Some(class_info.status.clone()),
        });
        for field in &class_info.fields {
            hovers.push(LspHover {
                name: format!("{}.{}", class_info.name, field.name),
                kind: "class_field".to_owned(),
                line: field.line,
                detail: format!(
                    "field {}: {} [{}], {}",
                    field.name,
                    field.type_name,
                    display_unit_label(&field.display_unit),
                    class_field_requirement(field)
                ),
                quantity_kind: field.quantity_kind.clone(),
                display_unit: field.display_unit.clone(),
                status: Some(field.status.clone()),
            });
        }
        for validation in &class_info.validations {
            hovers.push(LspHover {
                name: format!("{}.validate", class_info.name),
                kind: "class_validation".to_owned(),
                line: validation.line,
                detail: format!("validates {}", validation.expression),
                quantity_kind: "Bool".to_owned(),
                display_unit: "1".to_owned(),
                status: Some(validation.status.clone()),
            });
        }
        for method in &class_info.methods {
            hovers.push(LspHover {
                name: format!("{}.{}()", class_info.name, method.name),
                kind: "class_method".to_owned(),
                line: method.line,
                detail: format!("method {}() -> {}", method.name, method.return_type),
                quantity_kind: method.return_quantity_kind.clone(),
                display_unit: method.return_display_unit.clone(),
                status: Some(method.status.clone()),
            });
        }
    }

    for object in &report.semantic_program.class_objects {
        hovers.push(LspHover {
            name: object.name.clone(),
            kind: "class_object".to_owned(),
            line: object.line,
            detail: format!(
                "{} object with {} explicit field(s)",
                object.class_name,
                object.fields.len()
            ),
            quantity_kind: format!("Object[{}]", object.class_name),
            display_unit: "object".to_owned(),
            status: Some(object.status.clone()),
        });
        for field in &object.fields {
            hovers.push(LspHover {
                name: format!("{}.{}", object.name, field.name),
                kind: "object_field".to_owned(),
                line: field.line,
                detail: format!("{} = {}", field.name, field.expression),
                quantity_kind: field.quantity_kind.clone(),
                display_unit: field.display_unit.clone(),
                status: Some(field.status.clone()),
            });
        }
        for validation in &object.validations {
            hovers.push(LspHover {
                name: format!("{}.validate", object.name),
                kind: "object_validation".to_owned(),
                line: validation.line,
                detail: format!("{} => {}", validation.expression, validation.status),
                quantity_kind: "Bool".to_owned(),
                display_unit: validation.unit.clone(),
                status: Some(validation.status.clone()),
            });
        }
    }

    hovers.sort_by(|left, right| {
        left.line
            .cmp(&right.line)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.name.cmp(&right.name))
    });
    hovers
}

pub fn completion_items(report: &CheckReport) -> Vec<LspCompletion> {
    let mut seen = BTreeMap::new();
    let mut items = Vec::new();

    for (type_name, detail) in PUBLIC_TYPE_COMPLETIONS.iter().copied() {
        push_completion(&mut items, &mut seen, type_name, "class", detail);
    }

    for (label, detail) in WORKFLOW_BUILTIN_COMPLETIONS.iter().copied() {
        push_completion(&mut items, &mut seen, label, "function", detail);
    }

    for keyword in COMPLETION_KEYWORDS.iter().copied() {
        push_completion(&mut items, &mut seen, keyword, "keyword", "EngLang keyword");
    }

    for (label, detail) in WORKFLOW_OPTION_COMPLETIONS.iter().copied() {
        push_completion(&mut items, &mut seen, label, "property", detail);
    }

    for module in bundled_module_registry()
        .map(|registry| registry.modules)
        .unwrap_or_default()
    {
        push_completion(
            &mut items,
            &mut seen,
            &module.name,
            "stdlib",
            &module.completion_detail(),
        );
        for symbol in &module.symbols {
            push_completion(
                &mut items,
                &mut seen,
                &module_symbol_label(symbol),
                "stdlib",
                &format!("{} {}", module.name, symbol),
            );
        }
    }

    for (label, detail) in [
        ("read text", "eng.io raw text read"),
        ("read json", "eng.io raw JSON read"),
        ("read toml", "eng.io raw TOML read"),
        ("write text", "eng.io text output"),
        ("write json", "eng.io JSON output"),
        (
            "write standard_text",
            "Write a stable text artifact from table rows",
        ),
        ("export summary to csv", "eng.io one-row summary CSV export"),
        ("copy file", "eng.fs copy generated output"),
        ("move file", "eng.fs move generated output"),
        ("delete file", "eng.fs delete generated output"),
        ("run command", "eng.process command boundary"),
        ("promote json config", "eng.config JSON file promotion"),
        (
            "promote json records",
            "eng.table JSON records table promotion",
        ),
        ("promote toml config", "eng.config TOML file promotion"),
    ] {
        push_completion(&mut items, &mut seen, label, "stdlib", detail);
    }

    for binding in &report.semantic_program.typed_bindings {
        push_completion(
            &mut items,
            &mut seen,
            &binding.name,
            "variable",
            &format!(
                "{} [{}]",
                binding.semantic_type.quantity_kind, binding.semantic_type.display_unit
            ),
        );
        if binding.semantic_type.quantity_kind == "HttpResponse" {
            for (field, detail) in HTTP_RESPONSE_FIELD_COMPLETIONS {
                push_completion(
                    &mut items,
                    &mut seen,
                    &format!("{}.{}", binding.name, field),
                    "property",
                    detail,
                );
            }
        }
        if binding.semantic_type.quantity_kind == "TableRow" {
            if let Some(schema_name) = table_row_schema_name(report, &binding.name) {
                if let Some(schema) = report
                    .semantic_program
                    .schemas
                    .iter()
                    .find(|schema| schema.name == schema_name)
                {
                    for column in &schema.columns {
                        push_completion(
                            &mut items,
                            &mut seen,
                            &format!("{}.{}", binding.name, column.name),
                            "property",
                            &format!(
                                "{} [{}] from {}: {}",
                                column.type_name,
                                column.unit.as_deref().unwrap_or("schema-defined"),
                                binding.name,
                                schema.name
                            ),
                        );
                    }
                }
            }
        }
    }

    for schema in &report.semantic_program.schemas {
        for column in &schema.columns {
            push_completion(
                &mut items,
                &mut seen,
                &column.name,
                "property",
                &format!(
                    "{} [{}]",
                    column.type_name,
                    column.unit.as_deref().unwrap_or("schema-defined")
                ),
            );
        }
    }

    for domain in &report.semantic_program.domains {
        push_completion(
            &mut items,
            &mut seen,
            &domain.name,
            "class",
            &format!(
                "domain {}, {} variable(s), {} conservation(s)",
                domain_signature(&domain.name, &domain.type_parameters),
                domain.variables.len(),
                domain.conservations.len()
            ),
        );
        for variable in &domain.variables {
            push_completion(
                &mut items,
                &mut seen,
                &format!("{}.{}", domain.name, variable.name),
                "property",
                &format!(
                    "{} {} [{}]",
                    variable.role, variable.quantity_kind, variable.display_unit
                ),
            );
        }
    }

    for component in &report.semantic_program.components {
        push_completion(
            &mut items,
            &mut seen,
            &component.name,
            "class",
            &format!("component, {} port(s)", component.ports.len()),
        );
        for port in &component.ports {
            push_completion(
                &mut items,
                &mut seen,
                &format!("{}.{}", component.name, port.name),
                "property",
                &format!("port domain {} ({})", port.domain, port.status),
            );
        }
    }

    for assembly in &report.semantic_program.component_assemblies {
        push_completion(
            &mut items,
            &mut seen,
            &assembly.name,
            "value",
            &format!(
                "component assembly, {} equation(s), {} unknown(s)",
                assembly.equations.len(),
                assembly.boundary.unknown_count
            ),
        );
        for equation in &assembly.equations {
            push_completion(
                &mut items,
                &mut seen,
                &equation.name,
                "function",
                &format!("{} generated equation ({})", equation.kind, equation.status),
            );
        }
    }

    for function in &report.semantic_program.functions {
        push_completion(
            &mut items,
            &mut seen,
            &function.name,
            "function",
            &function_signature_detail(function),
        );
    }

    for class_info in &report.semantic_program.classes {
        push_completion(
            &mut items,
            &mut seen,
            &class_info.name,
            "class",
            &format!("class with {} field(s)", class_info.fields.len()),
        );
        for field in &class_info.fields {
            push_completion(
                &mut items,
                &mut seen,
                &format!("{}.{}", class_info.name, field.name),
                "property",
                &class_field_completion_detail(field, &class_info.name),
            );
        }
        for method in &class_info.methods {
            push_completion(
                &mut items,
                &mut seen,
                &format!("{}.{}()", class_info.name, method.name),
                "method",
                &format!(
                    "method returns {} [{}]",
                    method.return_type,
                    display_unit_label(&method.return_display_unit)
                ),
            );
        }
    }

    for object in &report.semantic_program.class_objects {
        for field in &object.fields {
            push_completion(
                &mut items,
                &mut seen,
                &format!("{}.{}", object.name, field.name),
                "property",
                &format!("{} [{}]", field.quantity_kind, field.display_unit),
            );
        }
    }

    for quantity in all_quantity_completions() {
        push_completion(
            &mut items,
            &mut seen,
            quantity.quantity_kind,
            "class",
            &format!("canonical unit {}", quantity.canonical_unit),
        );
    }

    for unit in all_unit_infos() {
        push_completion(
            &mut items,
            &mut seen,
            unit.symbol,
            "unit",
            &format!("{} unit", unit.quantity_hint),
        );
    }

    items
}

fn module_symbol_label(symbol: &str) -> String {
    let name = symbol
        .split_once('(')
        .map(|(name, _)| name)
        .unwrap_or(symbol)
        .trim();
    if name == "exists" {
        "exists path".to_owned()
    } else {
        format!("{name}(...)")
    }
}

fn table_row_schema_name<'a>(report: &'a CheckReport, receiver: &str) -> Option<&'a str> {
    let transform = report
        .semantic_program
        .table_transforms
        .iter()
        .find(|transform| transform.binding == receiver && transform.operation == "require_one")?;
    transform
        .schema_name
        .as_deref()
        .or_else(|| table_transform_source_schema_name(report, &transform.source_table, 0))
}

fn table_transform_source_schema_name<'a>(
    report: &'a CheckReport,
    source: &str,
    depth: usize,
) -> Option<&'a str> {
    if depth > 16 {
        return None;
    }
    if let Some(promotion) = report
        .semantic_program
        .csv_promotions
        .iter()
        .find(|promotion| promotion.binding == source)
    {
        return Some(promotion.schema_name.as_str());
    }
    let transform = report
        .semantic_program
        .table_transforms
        .iter()
        .find(|transform| transform.binding == source)?;
    transform
        .schema_name
        .as_deref()
        .or_else(|| table_transform_source_schema_name(report, &transform.source_table, depth + 1))
}

pub fn completion_items_at(
    report: &CheckReport,
    source: &str,
    line: usize,
    character: usize,
) -> Vec<LspCompletion> {
    if let Some(context) = with_block_completion_context(source, line) {
        let mut seen = BTreeMap::new();
        let mut items = Vec::new();
        if let Some(labels) = with_block_option_labels(&context.owner_text) {
            for label in labels {
                if context.assigned_options.contains(*label) {
                    continue;
                }
                if let Some(detail) = contextual_workflow_option_completion_detail(label) {
                    push_completion(&mut items, &mut seen, label, "property", detail);
                }
            }
        }
        if !items.is_empty() {
            return items;
        }
    }

    if let Some(context) = object_field_completion_context(report, source, line, character) {
        if let Some(class_info) = report
            .semantic_program
            .classes
            .iter()
            .find(|class_info| class_info.name == context.class_name)
        {
            let mut seen = BTreeMap::new();
            let mut items = Vec::new();
            for field in &class_info.fields {
                if context.assigned_fields.contains(&field.name) {
                    continue;
                }
                if context.prefix.is_empty() || field.name.starts_with(&context.prefix) {
                    push_completion(
                        &mut items,
                        &mut seen,
                        &field.name,
                        "property",
                        &class_field_completion_detail(field, &class_info.name),
                    );
                }
            }
            if !items.is_empty() {
                return items;
            }
        }
    }

    if let Some((receiver, prefix)) = member_completion_context(source, line, character) {
        if report
            .semantic_program
            .typed_bindings
            .iter()
            .any(|binding| {
                binding.name == receiver && binding.semantic_type.quantity_kind == "HttpResponse"
            })
        {
            let mut seen = BTreeMap::new();
            let mut items = Vec::new();
            for (field, detail) in HTTP_RESPONSE_FIELD_COMPLETIONS {
                if prefix.is_empty() || field.starts_with(&prefix) {
                    push_completion(&mut items, &mut seen, field, "property", detail);
                }
            }
            return items;
        }
        if let Some(schema_name) = table_row_schema_name(report, &receiver) {
            if let Some(schema) = report
                .semantic_program
                .schemas
                .iter()
                .find(|schema| schema.name == schema_name)
            {
                let mut seen = BTreeMap::new();
                let mut items = Vec::new();
                for column in &schema.columns {
                    if prefix.is_empty() || column.name.starts_with(&prefix) {
                        push_completion(
                            &mut items,
                            &mut seen,
                            &column.name,
                            "property",
                            &format!(
                                "{} [{}] from {}: {}",
                                column.type_name,
                                column.unit.as_deref().unwrap_or("schema-defined"),
                                receiver,
                                schema.name
                            ),
                        );
                    }
                }
                return items;
            }
        }
        if let Some(schema_name) = report
            .semantic_program
            .csv_promotions
            .iter()
            .find(|promotion| promotion.binding == receiver)
            .map(|promotion| promotion.schema_name.as_str())
        {
            if let Some(schema) = report
                .semantic_program
                .schemas
                .iter()
                .find(|schema| schema.name == schema_name)
            {
                let mut seen = BTreeMap::new();
                let mut items = Vec::new();
                for column in &schema.columns {
                    if prefix.is_empty() || column.name.starts_with(&prefix) {
                        push_completion(
                            &mut items,
                            &mut seen,
                            &column.name,
                            "property",
                            &format!(
                                "{} [{}] from {}: {}",
                                column.type_name,
                                column.unit.as_deref().unwrap_or("schema-defined"),
                                receiver,
                                schema.name
                            ),
                        );
                    }
                }
                return items;
            }
        }
        if let Some(object) = report
            .semantic_program
            .class_objects
            .iter()
            .find(|object| object.name == receiver)
        {
            let mut seen = BTreeMap::new();
            let mut items = Vec::new();
            if let Some(class_info) = report
                .semantic_program
                .classes
                .iter()
                .find(|class_info| class_info.name == object.class_name)
            {
                for field in &class_info.fields {
                    if prefix.is_empty() || field.name.starts_with(&prefix) {
                        push_completion(
                            &mut items,
                            &mut seen,
                            &field.name,
                            "property",
                            &class_field_completion_detail(field, &object.class_name),
                        );
                    }
                }
                for method in &class_info.methods {
                    if prefix.is_empty() || method.name.starts_with(&prefix) {
                        push_completion(
                            &mut items,
                            &mut seen,
                            &format!("{}()", method.name),
                            "method",
                            &format!(
                                "{} [{}] from {}",
                                method.return_type,
                                display_unit_label(&method.return_display_unit),
                                object.class_name
                            ),
                        );
                    }
                }
            }
            if !items.is_empty() {
                return items;
            }
        }
    }

    if let Some(context) = function_argument_completion_context(source, line, character) {
        if let Some(labels) = function_argument_option_labels(&context.function_name) {
            let mut seen = BTreeMap::new();
            let mut items = Vec::new();
            for label in labels {
                if context.assigned_options.contains(*label) {
                    continue;
                }
                if !context.prefix.is_empty() && !label.starts_with(&context.prefix) {
                    continue;
                }
                if let Some(detail) = function_argument_option_completion_detail(label) {
                    push_completion(&mut items, &mut seen, label, "property", detail);
                }
            }
            if !items.is_empty() {
                return items;
            }
        }
    }

    completion_items(report)
}

struct WithBlockCompletionContext {
    owner_text: String,
    assigned_options: BTreeSet<String>,
}

fn with_block_completion_context(source: &str, line: usize) -> Option<WithBlockCompletionContext> {
    let lines = source.lines().collect::<Vec<_>>();
    if line >= lines.len() {
        return None;
    }
    for start in 0..=line {
        if !is_with_block_start(lines[start]) {
            continue;
        }
        let Some(end) = source_block_end(&lines, start) else {
            continue;
        };
        if line <= start || line >= end {
            continue;
        }
        let owner_line = previous_non_empty_line(&lines, start)?;
        return Some(WithBlockCompletionContext {
            owner_text: lines[owner_line].trim().to_owned(),
            assigned_options: assigned_with_options(&lines, start, end),
        });
    }
    None
}

fn with_block_option_labels(owner_text: &str) -> Option<&'static [&'static str]> {
    let owner = owner_text.trim();
    if is_http_request_owner_text(owner) {
        return Some(&[
            "query",
            "headers",
            "body",
            "offline_response",
            "fixture",
            "expected_sha256",
            "retry",
            "timeout",
            "body_size_limit",
            "cache",
            "cache_key",
            "cache_dir",
            "status_code",
        ]);
    }
    if owner.starts_with("download ") {
        return Some(&[
            "offline_response",
            "fixture",
            "expected_sha256",
            "retry",
            "timeout",
            "response_body_limit",
            "cache",
            "cache_key",
            "cache_dir",
        ]);
    }
    if owner.contains("run command") {
        return Some(&[
            "args",
            "cwd",
            "env",
            "timeout",
            "retry",
            "expected_outputs",
            "allow_failure",
            "tool_version",
            "cache",
            "cache_key",
            "cache_dir",
            "cache_ttl",
        ]);
    }
    if owner.starts_with("delete ") {
        return Some(&["confirm", "recursive"]);
    }
    if owner.starts_with("move ") {
        return Some(&["confirm", "overwrite"]);
    }
    if owner.starts_with("write ") && owner.contains(" to ") && owner.contains(".table(") {
        return Some(&["mode", "key", "transaction", "overwrite"]);
    }
    if owner.starts_with("copy ") || owner.starts_with("write ") || owner.starts_with("export ") {
        return Some(&["overwrite", "mode"]);
    }
    if owner.starts_with("plot ") {
        return Some(&["unit y", "unit x", "title", "confidence_band"]);
    }
    if owner.contains("require_one ") || owner.starts_with("require_one(") {
        return Some(&["on_none", "on_many"]);
    }
    if owner.contains("TimeSeries[") {
        return Some(&["sensor_std"]);
    }
    if owner.starts_with("simulate ") {
        return Some(&[
            "timestep",
            "duration",
            "solver",
            "tolerance",
            "inputs",
            "initial",
        ]);
    }
    if owner.starts_with("solve ") || owner.contains("solve component_graph") {
        return Some(&[
            "solver",
            "timestep",
            "duration",
            "initial",
            "initial_derivative",
            "initial_algebraic",
            "algebraic_initialization",
            "inputs",
            "jacobian",
            "mass_matrix",
            "tolerance",
            "max_iter",
            "finite_difference_step",
            "damping",
            "line_search_steps",
            "relaxation",
            "residual_scale",
            "residual_scales",
            "consistency_tolerance",
            "variable_scale",
            "variable_scales",
        ]);
    }
    if owner.contains("render template") {
        return Some(&[
            "values",
            "output",
            "missing",
            "overwrite",
            "artifact_kind",
            "cache",
            "cache_key",
            "cache_dir",
        ]);
    }
    if owner.contains("apply ") || owner.contains("apply(") {
        return Some(&[
            "template",
            "values",
            "output",
            "missing",
            "overwrite",
            "artifact_kind",
        ]);
    }
    if owner.contains("materialize ") {
        return Some(&[
            "step",
            "output_root",
            "resume",
            "case_id",
            "cache",
            "cache_key",
        ]);
    }
    if owner.contains("check coverage") || owner.contains("coverage ") {
        return Some(&[
            "expected_step",
            "year",
            "start",
            "end",
            "max_gap",
            "missing",
        ]);
    }
    if owner.contains("train_test_split") {
        return Some(&["target", "features", "test", "seed", "cache", "cache_key"]);
    }
    if owner.contains("train regression")
        || owner.contains("regression(")
        || owner.contains("train_regression")
        || owner.contains("mlp(")
        || owner.contains("ann(")
    {
        return Some(&[
            "algorithm",
            "features",
            "x",
            "target",
            "y",
            "test",
            "test_fraction",
            "hidden",
            "layers",
            "epochs",
            "cache",
            "cache_key",
        ]);
    }
    if owner.contains("evaluate(") || owner.contains("leakage_lint(") {
        return Some(&["split", "cache", "cache_key"]);
    }
    if owner.contains("predict ") || owner.contains("predict(") {
        return Some(&["output", "cache", "cache_key"]);
    }
    if owner.contains("sample ") || owner.contains("lhs(") || owner.contains("uniform(") {
        return Some(&["count", "seed", "start", "end", "method"]);
    }
    None
}

fn is_http_request_owner_text(owner: &str) -> bool {
    let Some(index) = owner.find("http ") else {
        return false;
    };
    let rest = owner[index + "http ".len()..].trim_start();
    let Some(method) = rest.split_whitespace().next() else {
        return false;
    };
    matches!(
        method,
        "get" | "post" | "put" | "patch" | "head" | "request" | "fetch"
    )
}

fn workflow_option_completion_detail(label: &str) -> Option<&'static str> {
    WORKFLOW_OPTION_COMPLETIONS
        .iter()
        .find(|(candidate, _detail)| *candidate == label)
        .map(|(_candidate, detail)| *detail)
}

fn contextual_workflow_option_completion_detail(label: &str) -> Option<&'static str> {
    workflow_option_completion_detail(label).or_else(|| match label {
        "unit x" => Some("plot x-axis display unit option"),
        "unit y" => Some("plot y-axis display unit option"),
        _ => None,
    })
}

struct FunctionArgumentCompletionContext {
    function_name: String,
    prefix: String,
    assigned_options: BTreeSet<String>,
}

fn function_argument_completion_context(
    source: &str,
    line: usize,
    character: usize,
) -> Option<FunctionArgumentCompletionContext> {
    let before_cursor = source_prefix_at_position(source, line, character)?;
    let open_paren = last_unmatched_open_paren(&before_cursor)?;
    let function_name = function_name_before_open_paren(&before_cursor, open_paren)?;
    let arguments = &before_cursor[open_paren + 1..];
    if current_argument_has_assignment(arguments) {
        return None;
    }
    Some(FunctionArgumentCompletionContext {
        function_name,
        prefix: current_argument_prefix(arguments),
        assigned_options: assigned_function_argument_options(arguments),
    })
}

fn source_prefix_at_position(source: &str, line: usize, character: usize) -> Option<String> {
    let mut prefix = String::new();
    for (line_index, line_text) in source.lines().enumerate() {
        if line_index < line {
            prefix.push_str(line_text);
            prefix.push('\n');
            continue;
        }
        if line_index == line {
            prefix.extend(line_text.chars().take(character));
            return Some(prefix);
        }
        break;
    }
    None
}

fn function_argument_option_labels(function_name: &str) -> Option<&'static [&'static str]> {
    match function_name {
        "train_test_split" => Some(&["target", "features", "x", "test", "test_fraction", "seed"]),
        "regression" | "regression_table" | "train_regression" => Some(&[
            "target",
            "y",
            "features",
            "x",
            "algorithm",
            "test",
            "test_fraction",
            "seed",
        ]),
        "mlp" | "ann" => Some(&[
            "target",
            "y",
            "features",
            "x",
            "algorithm",
            "hidden",
            "layers",
            "epochs",
            "seed",
        ]),
        "evaluate" | "metrics" | "leakage_lint" => Some(&["split"]),
        "measured" => Some(&["std", "sigma", "uncertainty", "error", "relative_error"]),
        "interval" | "uniform" => Some(&["lower", "min", "upper", "max", "samples", "n"]),
        "normal" => Some(&["mean", "mu", "std", "sigma", "samples", "n"]),
        "distribution" => Some(&[
            "kind",
            "distribution",
            "mean",
            "mu",
            "std",
            "sigma",
            "lower",
            "min",
            "upper",
            "max",
            "samples",
            "n",
        ]),
        "propagate" => Some(&["method", "scale", "gain", "offset", "bias", "samples", "n"]),
        "ensemble" => Some(&["method", "samples", "n"]),
        _ => None,
    }
}

fn function_argument_option_completion_detail(label: &str) -> Option<&'static str> {
    contextual_workflow_option_completion_detail(label).or_else(|| match label {
        "distribution" => Some("uncertainty distribution kind option alias"),
        "error" => Some("measurement absolute error option"),
        "layers" => Some("MLP hidden layer alias"),
        "max" => Some("upper uncertainty or range bound alias"),
        "mean" => Some("uncertainty mean option"),
        "min" => Some("lower uncertainty or range bound alias"),
        "std" => Some("uncertainty standard deviation option"),
        "test_fraction" => Some("train/test split fraction alias"),
        _ => None,
    })
}

fn last_unmatched_open_paren(value: &str) -> Option<usize> {
    let mut stack = Vec::new();
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in value.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' => stack.push(index),
            ')' => {
                stack.pop();
            }
            _ => {}
        }
    }
    stack.last().copied()
}

fn function_name_before_open_paren(value: &str, open_paren: usize) -> Option<String> {
    let bytes = value.as_bytes();
    let mut end = open_paren.min(bytes.len());
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && is_ident_byte(bytes[start - 1]) {
        start -= 1;
    }
    (start < end && is_ident_start(bytes[start])).then(|| value[start..end].to_ascii_lowercase())
}

fn current_argument_has_assignment(arguments: &str) -> bool {
    top_level_current_argument(arguments).contains('=')
}

fn current_argument_prefix(arguments: &str) -> String {
    let current = top_level_current_argument(arguments).trim_end();
    let bytes = current.as_bytes();
    let mut start = bytes.len();
    while start > 0 && is_ident_byte(bytes[start - 1]) {
        start -= 1;
    }
    current[start..].to_owned()
}

fn top_level_current_argument(arguments: &str) -> &str {
    let mut segment_start = 0usize;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in arguments.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => segment_start = index + character.len_utf8(),
            _ => {}
        }
    }
    &arguments[segment_start..]
}

fn assigned_function_argument_options(arguments: &str) -> BTreeSet<String> {
    let mut options = BTreeSet::new();
    let bytes = arguments.as_bytes();
    let mut index = 0usize;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    while index < bytes.len() {
        let character = arguments[index..].chars().next().unwrap_or_default();
        let width = character.len_utf8();
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            index += width;
            continue;
        }
        match character {
            '"' => {
                in_string = true;
                index += width;
            }
            '(' | '[' | '{' => {
                depth += 1;
                index += width;
            }
            ')' | ']' | '}' => {
                depth = depth.saturating_sub(1);
                index += width;
            }
            _ if depth == 0 && is_ident_start(bytes[index]) => {
                let start = index;
                index += 1;
                while index < bytes.len() && is_ident_byte(bytes[index]) {
                    index += 1;
                }
                let end = index;
                let after_key = skip_ascii_whitespace(bytes, index, bytes.len());
                if after_key < bytes.len() && bytes[after_key] == b'=' {
                    options.insert(arguments[start..end].to_owned());
                }
            }
            _ => {
                index += width;
            }
        }
    }
    options
}

fn is_with_block_start(line: &str) -> bool {
    line.trim() == "with {"
}

fn previous_non_empty_line(lines: &[&str], before: usize) -> Option<usize> {
    (0..before)
        .rev()
        .find(|line| !lines[*line].trim().is_empty())
}

fn source_block_end(lines: &[&str], start: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (line_number, line) in lines.iter().enumerate().skip(start) {
        for character in strip_source_comment(line).chars() {
            match character {
                '{' => depth += 1,
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(line_number);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn assigned_with_options(lines: &[&str], start: usize, end: usize) -> BTreeSet<String> {
    let mut options = BTreeSet::new();
    for line in lines.iter().take(end).skip(start + 1) {
        let Some((key, _value)) = strip_source_comment(line).trim().split_once('=') else {
            continue;
        };
        let key = key.trim();
        if is_with_option_assignment_key(key) {
            options.insert(key.to_owned());
        }
    }
    options
}

fn is_with_option_assignment_key(key: &str) -> bool {
    if key
        .chars()
        .all(|character| character == '_' || character.is_ascii_alphanumeric())
    {
        return true;
    }
    key.strip_prefix("unit ").is_some_and(|axis| {
        !axis.is_empty()
            && axis
                .chars()
                .all(|character| character == '_' || character.is_ascii_alphanumeric())
    })
}

fn strip_source_comment(line: &str) -> &str {
    line.split_once('#')
        .map(|(before_comment, _comment)| before_comment)
        .unwrap_or(line)
}

pub fn severity_to_lsp(severity: &Severity) -> u8 {
    match severity {
        Severity::Error => 1,
        Severity::Warning => 2,
        Severity::Info => 3,
    }
}

fn push_completion(
    items: &mut Vec<LspCompletion>,
    seen: &mut BTreeMap<String, usize>,
    label: &str,
    kind: &str,
    detail: &str,
) {
    let completion = LspCompletion {
        label: label.to_owned(),
        kind: kind.to_owned(),
        detail: detail.to_owned(),
    };
    if let Some(index) = seen.get(label).copied() {
        if should_replace_completion(label, &items[index].kind, kind) {
            items[index] = completion;
        }
    } else {
        seen.insert(label.to_owned(), items.len());
        items.push(completion);
    }
}

fn should_replace_completion(label: &str, existing_kind: &str, candidate_kind: &str) -> bool {
    matches!(label, "output" | "inputs")
        && existing_kind == "keyword"
        && candidate_kind == "property"
}

fn domain_signature(name: &str, parameters: &[DomainTypeParameterInfo]) -> String {
    if parameters.is_empty() {
        name.to_owned()
    } else {
        format!(
            "{name}[{}]",
            parameters
                .iter()
                .map(|parameter| parameter.display.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn string_list(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_owned()
    } else {
        values.join(", ")
    }
}

fn port_metadata_detail(
    port: &eng_compiler::PortInfo,
    domains: &[eng_compiler::DomainInfo],
) -> String {
    let mut labels = vec![
        format!("type {}", port.domain),
        format!("domain {}", port.domain_name),
    ];
    if let Some(domain) = domains
        .iter()
        .find(|domain| domain.name == port.domain_name)
    {
        let mut saw_medium = false;
        for (parameter, argument) in domain.type_parameters.iter().zip(&port.type_arguments) {
            let label = parameter.kind.to_ascii_lowercase();
            if label == "medium" {
                saw_medium = true;
            }
            labels.push(format!("{label} {argument}"));
        }
        if !saw_medium {
            labels.push("medium -".to_owned());
        }
    } else if port.type_arguments.is_empty() {
        labels.push("medium -".to_owned());
    } else {
        labels.push(format!("arguments {}", string_list(&port.type_arguments)));
    }
    labels.join("; ")
}

fn lsp_severity(severity: &str) -> u8 {
    match severity {
        "error" => 1,
        "warning" => 2,
        _ => 3,
    }
}

fn completion_kind(kind: &str) -> u8 {
    match kind {
        "keyword" => 14,
        "variable" => 6,
        "property" => 10,
        "class" => 7,
        "function" => 3,
        "method" => 2,
        "stdlib" => 9,
        "unit" => 11,
        "value" => 12,
        _ => 1,
    }
}

#[derive(Debug)]
struct ObjectFieldCompletionContext {
    class_name: String,
    prefix: String,
    assigned_fields: BTreeSet<String>,
}

fn object_field_completion_context(
    report: &CheckReport,
    source: &str,
    line: usize,
    character: usize,
) -> Option<ObjectFieldCompletionContext> {
    let lines = source.lines().collect::<Vec<_>>();
    let current_line = lines.get(line)?;
    let before_cursor = current_line.chars().take(character).collect::<String>();
    let prefix = object_field_prefix(&before_cursor)?;
    let mut stack = Vec::<ObjectContext>::new();

    for (index, full_line) in lines.iter().enumerate().take(line + 1) {
        let line_text = if index == line {
            before_cursor.as_str()
        } else {
            full_line
        };
        let trimmed = line_text.trim();
        if trimmed.starts_with('}') {
            stack.pop();
            continue;
        }
        if let Some(class_name) = object_context_class_name(report, trimmed) {
            stack.push(ObjectContext {
                class_name,
                start_line: index,
            });
            continue;
        }
        if trimmed.contains('}') {
            stack.pop();
        }
    }

    let context = stack.last()?;
    Some(ObjectFieldCompletionContext {
        class_name: context.class_name.clone(),
        prefix,
        assigned_fields: assigned_object_fields(&lines, context.start_line, line),
    })
}

#[derive(Debug)]
struct ObjectContext {
    class_name: String,
    start_line: usize,
}

fn object_context_class_name(report: &CheckReport, trimmed_line: &str) -> Option<String> {
    if !trimmed_line.ends_with('{') {
        return None;
    }
    let (left, right) = trimmed_line.split_once('=')?;
    if !is_identifier(left.trim()) {
        return None;
    }
    let body = right.trim_end_matches('{').trim();
    let parts = body.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [class_name] if class_exists(report, class_name) => Some((*class_name).to_owned()),
        [source_object, "with"] => report
            .semantic_program
            .class_objects
            .iter()
            .find(|object| object.name == *source_object)
            .map(|object| object.class_name.clone()),
        _ => None,
    }
}

fn object_field_prefix(before_cursor: &str) -> Option<String> {
    let content = before_cursor.trim_end().trim_start();
    if content.contains('=')
        || content.contains('.')
        || content.contains('{')
        || content.contains('}')
        || content.split_whitespace().count() > 1
    {
        return None;
    }
    if !content.is_empty() && !is_identifier(content) {
        return None;
    }
    Some(content.to_owned())
}

fn assigned_object_fields(
    lines: &[&str],
    start_line: usize,
    current_line: usize,
) -> BTreeSet<String> {
    let mut assigned = BTreeSet::new();
    for line in lines
        .iter()
        .enumerate()
        .skip(start_line + 1)
        .take(current_line.saturating_sub(start_line))
        .map(|(_, line)| *line)
    {
        let Some((name, _)) = line.trim().split_once('=') else {
            continue;
        };
        let name = name.trim();
        if is_identifier(name) {
            assigned.insert(name.to_owned());
        }
    }
    assigned
}

fn class_exists(report: &CheckReport, class_name: &str) -> bool {
    report
        .semantic_program
        .classes
        .iter()
        .any(|class_info| class_info.name == class_name)
}

fn is_identifier(value: &str) -> bool {
    let mut bytes = value.as_bytes().iter();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !(*first == b'_' || first.is_ascii_alphabetic()) {
        return false;
    }
    bytes.all(|byte| is_ident_byte(*byte))
}

fn class_field_requirement(field: &ClassFieldInfo) -> String {
    match (&field.default_value, field.required) {
        (_, true) => "required".to_owned(),
        (Some(default_value), false) => format!("default = {default_value}"),
        (None, false) => "optional".to_owned(),
    }
}

fn class_field_completion_detail(field: &ClassFieldInfo, class_name: &str) -> String {
    format!(
        "{} {} [{}] from {}",
        class_field_requirement(field),
        field.type_name,
        display_unit_label(&field.display_unit),
        class_name
    )
}

fn display_unit_label(unit: &str) -> &str {
    if unit.is_empty() {
        "-"
    } else {
        unit
    }
}

fn function_signature_detail(function: &FunctionInfo) -> String {
    let params = function
        .parameters
        .iter()
        .map(|parameter| {
            format!(
                "{}: {} [{}]",
                parameter.name,
                parameter.quantity_kind,
                display_unit_label(&parameter.display_unit)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut detail = format!(
        "fn {}({}) -> {} [{}]",
        function.name,
        params,
        function.return_quantity_kind,
        display_unit_label(&function.return_display_unit)
    );
    if let Some(return_expression) = &function.return_expression {
        detail.push_str(&format!(" returns `{return_expression}`"));
    }
    if !function.locals.is_empty() {
        detail.push_str(&format!(
            "; locals {}",
            function
                .locals
                .iter()
                .map(|local| local.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    detail
}

fn member_completion_context(
    source: &str,
    line: usize,
    character: usize,
) -> Option<(String, String)> {
    let line_text = source.lines().nth(line)?;
    let before_cursor = line_text
        .chars()
        .take(character)
        .collect::<String>()
        .trim_end()
        .to_owned();
    let bytes = before_cursor.as_bytes();
    let mut prefix_end = bytes.len();
    let mut prefix_start = prefix_end;
    while prefix_start > 0 && is_ident_byte(bytes[prefix_start - 1]) {
        prefix_start -= 1;
    }
    if prefix_start == 0 || bytes[prefix_start - 1] != b'.' {
        return None;
    }
    let receiver_end = prefix_start - 1;
    let mut receiver_start = receiver_end;
    while receiver_start > 0 && is_ident_byte(bytes[receiver_start - 1]) {
        receiver_start -= 1;
    }
    if receiver_start == receiver_end {
        return None;
    }
    prefix_end = prefix_end.max(prefix_start);
    Some((
        before_cursor[receiver_start..receiver_end].to_owned(),
        before_cursor[prefix_start..prefix_end].to_owned(),
    ))
}

fn is_ident_byte(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn assert_semantic_token_modifier(
        snapshot: &LspSnapshot,
        source: &str,
        label: &str,
        modifier: &str,
    ) {
        assert!(
            snapshot.semantic_tokens.tokens.iter().any(|token| {
                source.lines().nth(token.line).is_some_and(|line| {
                    line.get(token.start..token.start + token.length) == Some(label)
                        && token.modifiers.iter().any(|item| item == modifier)
                })
            }),
            "semantic token `{label}` should include modifier `{modifier}`"
        );
    }

    fn repo_root_for_tests() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("eng_lsp crate should live under crates/eng_lsp")
            .to_path_buf()
    }

    fn read_json_file(path: &Path) -> serde_json::Value {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        serde_json::from_str(&content)
            .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
    }

    fn package_semantic_scope_keys(package: &serde_json::Value) -> BTreeSet<&str> {
        package["contributes"]["semanticTokenScopes"]
            .as_array()
            .expect("package should declare semanticTokenScopes")
            .iter()
            .find(|scope| scope["language"] == "englang")
            .and_then(|scope| scope["scopes"].as_object())
            .expect("package should declare englang semanticTokenScopes")
            .keys()
            .map(String::as_str)
            .collect()
    }

    fn collect_textmate_keyword_words(value: &serde_json::Value, words: &mut BTreeSet<String>) {
        match value {
            serde_json::Value::Array(items) => {
                for item in items {
                    collect_textmate_keyword_words(item, words);
                }
            }
            serde_json::Value::Object(object) => {
                if object
                    .get("name")
                    .and_then(|name| name.as_str())
                    .is_some_and(|name| name.starts_with("keyword."))
                {
                    if let Some(pattern) = object.get("match").and_then(|pattern| pattern.as_str())
                    {
                        for word in textmate_word_alternatives(pattern) {
                            words.insert(word);
                        }
                    }
                }
                for child in object.values() {
                    collect_textmate_keyword_words(child, words);
                }
            }
            _ => {}
        }
    }

    fn textmate_word_alternatives(pattern: &str) -> Vec<String> {
        let mut words = Vec::new();
        let mut rest = pattern;
        while let Some(start) = rest.find("\\b(") {
            let after_start = &rest[start + 3..];
            let Some(end) = after_start.find(")\\b") else {
                break;
            };
            for alternative in after_start[..end].split('|') {
                if alternative
                    .chars()
                    .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
                {
                    words.push(alternative.to_owned());
                }
            }
            rest = &after_start[end + 3..];
        }
        words
    }

    fn assert_semantic_token_type(
        snapshot: &LspSnapshot,
        source: &str,
        label: &str,
        token_type: &str,
    ) {
        assert!(
            snapshot.semantic_tokens.tokens.iter().any(|token| {
                source.lines().nth(token.line).is_some_and(|line| {
                    line.get(token.start..token.start + token.length) == Some(label)
                        && token.token_type == token_type
                })
            }),
            "semantic token `{label}` should have type `{token_type}`"
        );
    }

    fn semantic_token_count(
        snapshot: &LspSnapshot,
        source: &str,
        label: &str,
        token_type: &str,
    ) -> usize {
        snapshot
            .semantic_tokens
            .tokens
            .iter()
            .filter(|token| {
                source.lines().nth(token.line).is_some_and(|line| {
                    line.get(token.start..token.start + token.length) == Some(label)
                        && token.token_type == token_type
                })
            })
            .count()
    }

    fn semantic_token_modifier_count(
        snapshot: &LspSnapshot,
        source: &str,
        label: &str,
        token_type: &str,
        modifier: &str,
    ) -> usize {
        snapshot
            .semantic_tokens
            .tokens
            .iter()
            .filter(|token| {
                source.lines().nth(token.line).is_some_and(|line| {
                    line.get(token.start..token.start + token.length) == Some(label)
                        && token.token_type == token_type
                        && token.modifiers.iter().any(|item| item == modifier)
                })
            })
            .count()
    }

    fn assert_semantic_token_on_line_without_modifier(
        snapshot: &LspSnapshot,
        source: &str,
        line_needle: &str,
        label: &str,
        token_type: &str,
        modifier: &str,
    ) {
        let line_index = source
            .lines()
            .position(|line| line.contains(line_needle))
            .unwrap_or_else(|| panic!("source line `{line_needle}` should be present"));
        assert!(
            snapshot.semantic_tokens.tokens.iter().any(|token| {
                token.line == line_index
                    && source.lines().nth(token.line).is_some_and(|line| {
                        line.get(token.start..token.start + token.length) == Some(label)
                            && token.token_type == token_type
                            && !token.modifiers.iter().any(|item| item == modifier)
                    })
            }),
            "semantic token `{label}` on `{line_needle}` should be `{token_type}` without modifier `{modifier}`"
        );
    }

    fn assert_first_diagnostic_underlines(source: &str, code: &str, expected_text: &str) {
        let snapshot = snapshot_for_source(Path::new("diagnostic_ranges.eng"), source);
        let json = snapshot_json(&snapshot);
        let diagnostic = json["diagnostics"]
            .as_array()
            .and_then(|diagnostics| {
                diagnostics
                    .iter()
                    .find(|diagnostic| diagnostic["code"] == code)
            })
            .unwrap_or_else(|| panic!("diagnostic {code} should be present"));
        let line_index = diagnostic["range"]["start"]["line"]
            .as_u64()
            .expect("diagnostic should have a start line") as usize;
        let start = diagnostic["range"]["start"]["character"]
            .as_u64()
            .expect("diagnostic should have a start character") as usize;
        let end = diagnostic["range"]["end"]["character"]
            .as_u64()
            .expect("diagnostic should have an end character") as usize;
        let line = source.lines().nth(line_index).expect("diagnostic line");

        assert_eq!(
            line.get(start..end),
            Some(expected_text),
            "diagnostic {code} should underline `{expected_text}` on `{line}`"
        );
    }

    #[test]
    fn snapshot_exposes_lsp_diagnostics_hover_and_completion() {
        let source = "/// heat rate smoke\nQ: HeatRate [kW] = 2 kW - 1\n}\n";
        let snapshot = snapshot_for_source(Path::new("bad.eng"), source);

        assert!(snapshot
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-DIM-ADD-002"));
        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "HeatRate"));
        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "kW"));
        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "eng.path"));
        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "read text"));
        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "eng.process"));
        for required in [
            "CsvFile",
            "DirectoryPath",
            "JsonFile",
            "TimeSeries[Time]",
            "ProcessResult",
            "file",
            "dir",
            "join",
            "url",
            "secret",
            "http",
            "get",
            "download",
            "render",
            "template",
            "open",
            "sqlite",
            "predict",
            "check",
            "coverage",
            "records",
            "promote json records",
            "materialize",
            "apply",
            "collect",
            "case_id",
            "output_root",
            "resume",
            "step",
            "args",
            "output",
            "values",
            "offline_response",
            "expected_sha256",
            "expected_outputs",
            "artifact_kind",
            "allow_failure",
            "cache_key",
            "body_size_limit",
            "mean",
            "min",
            "sum",
        ] {
            assert!(
                snapshot
                    .completions
                    .iter()
                    .any(|completion| completion.label == required),
                "LSP completion should include {required}"
            );
        }
        assert!(snapshot.completions.iter().any(|completion| {
            completion.label == "eng.net" && completion.detail.contains("Native workflow support")
        }));
        assert!(snapshot.completions.iter().any(|completion| {
            completion.label == "eng.cache" && completion.detail.contains("Native workflow support")
        }));
        assert!(snapshot.completions.iter().any(|completion| {
            completion.label == "eng.uncertainty"
                && completion.detail.contains("Native workflow support")
        }));
        for (label, detail_part) in [
            ("require_one", "exactly one row"),
            ("uniform", "eng.sampling"),
            ("regression_table", "regression model"),
            ("predict", "predictions"),
            ("time_weighted_mean", "eng.timeseries"),
        ] {
            let completion = snapshot
                .completions
                .iter()
                .find(|completion| completion.label == label)
                .unwrap_or_else(|| panic!("LSP completion should include {label}"));
            assert_eq!(completion.kind, "function");
            assert!(
                completion.detail.contains(detail_part),
                "completion {label} detail should contain {detail_part}, got {}",
                completion.detail
            );
        }

        let json = snapshot_json(&snapshot);
        assert_eq!(json["format"], LSP_SNAPSHOT_FORMAT);
        assert!(!json["diagnostics"].as_array().unwrap().is_empty());
        let dim_diagnostic = json["diagnostics"]
            .as_array()
            .and_then(|diagnostics| {
                diagnostics
                    .iter()
                    .find(|diagnostic| diagnostic["code"] == "E-DIM-ADD-002")
            })
            .expect("dimensionless diagnostic");
        let dim_line = source.lines().nth(1).expect("diagnostic line");
        let minus_character = dim_line.find('-').expect("minus operator");
        assert_eq!(
            dim_diagnostic["range"]["start"]["character"].as_u64(),
            Some(minus_character as u64)
        );
        assert_eq!(
            dim_diagnostic["range"]["end"]["character"].as_u64(),
            Some((minus_character + 1) as u64)
        );
        assert!(snapshot
            .semantic_tokens
            .legend
            .token_types
            .iter()
            .any(|token_type| token_type == "variable"));
        assert!(snapshot.semantic_tokens.tokens.iter().any(|token| {
            token.token_type == "type" && token.modifiers.iter().any(|modifier| modifier == "unit")
        }));
        assert!(snapshot.semantic_tokens.tokens.iter().any(|token| {
            token.token_type == "type"
                && token
                    .modifiers
                    .iter()
                    .any(|modifier| modifier == "quantity")
        }));
        assert!(snapshot
            .semantic_tokens
            .tokens
            .iter()
            .any(|token| token.token_type == "comment"));
        assert!(snapshot
            .document_symbols
            .iter()
            .any(|symbol| symbol.name == "Q" && symbol.kind == SYMBOL_KIND_VARIABLE));
        let completion_json = json["completions"].as_array().unwrap();
        assert!(completion_json
            .iter()
            .any(|completion| { completion["label"] == "kW" && completion["kind"] == 11 }));
        assert!(completion_json
            .iter()
            .any(|completion| { completion["label"] == "eng.path" && completion["kind"] == 9 }));
        assert!(!json["semantic_tokens"]["tokens"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(json["document_symbols"]
            .as_array()
            .unwrap()
            .iter()
            .any(|symbol| symbol["name"] == "Q"));
    }

    #[test]
    fn editor_metadata_exports_completion_seed_and_semantic_legend() {
        let metadata = editor_metadata_json();
        assert_eq!(metadata["format"], LSP_EDITOR_METADATA_FORMAT);
        assert_eq!(
            metadata["semantic_token_legend"]["token_types"][0],
            SEMANTIC_TOKEN_TYPES[0]
        );
        assert_eq!(
            metadata["semantic_token_legend"]["token_modifiers"][0],
            SEMANTIC_TOKEN_MODIFIERS[0]
        );
        let syntax_catalog = &metadata["syntax_catalog"];
        assert_eq!(syntax_catalog["keywords"][0], COMPLETION_KEYWORDS[0]);
        assert!(
            syntax_catalog["workflow_builtins"]
                .as_array()
                .is_some_and(|labels| labels.iter().any(|label| label == "train")),
            "syntax catalog should expose workflow builtin labels"
        );
        assert!(
            syntax_catalog["workflow_options"]
                .as_array()
                .is_some_and(|options| options
                    .iter()
                    .any(|option| option["label"] == "offline_response")),
            "syntax catalog should expose with-block option labels"
        );
        assert!(
            syntax_catalog["units"]
                .as_array()
                .is_some_and(|units| units.iter().any(|unit| unit["label"] == "kW")),
            "syntax catalog should expose compiler unit labels"
        );

        let completions = metadata["completion_seed"]
            .as_array()
            .expect("editor completion seed should be an array");
        assert_eq!(
            metadata["completion_seed_count"].as_u64(),
            Some(completions.len() as u64)
        );
        for (label, kind) in [
            ("records", "keyword"),
            ("promote json records", "stdlib"),
            ("read json", "stdlib"),
            ("eng.table", "stdlib"),
            ("HeatRate", "class"),
            ("StateVector[T]", "class"),
            ("LinearOperator[From -> To]", "class"),
            ("kW", "unit"),
            ("output", "property"),
            ("inputs", "property"),
            ("split", "property"),
        ] {
            assert!(
                completions
                    .iter()
                    .any(|completion| completion["label"] == label && completion["kind"] == kind),
                "editor metadata should include completion {label} as {kind}"
            );
        }
        let offline_response_completion = completions
            .iter()
            .find(|completion| completion["label"] == "offline_response")
            .expect("editor metadata should include offline_response option completion");
        assert_eq!(
            offline_response_completion["detail"],
            "Pinned offline HTTP response used instead of live network"
        );
        let fixture_completion = completions
            .iter()
            .find(|completion| completion["label"] == "fixture")
            .expect("editor metadata should include fixture legacy option completion");
        assert_eq!(
            fixture_completion["detail"],
            "Legacy alias for offline_response"
        );
        let net_completion = completions
            .iter()
            .find(|completion| completion["label"] == "eng.net")
            .expect("editor metadata should include eng.net module completion");
        assert!(
            net_completion["detail"]
                .as_str()
                .is_some_and(|detail| detail.contains("pinned offline/cache HTTP")),
            "eng.net completion detail should describe pinned offline/cache HTTP, got {}",
            net_completion["detail"]
        );
        let cache_completion = completions
            .iter()
            .find(|completion| completion["label"] == "eng.cache")
            .expect("editor metadata should include eng.cache module completion");
        assert!(
            cache_completion["detail"]
                .as_str()
                .is_some_and(|detail| detail.contains("pinned network response cache")),
            "eng.cache completion detail should describe pinned network response cache, got {}",
            cache_completion["detail"]
        );
    }

    #[test]
    fn lsp_keyword_completions_cover_textmate_keyword_fallback_words() {
        let root = repo_root_for_tests();
        let grammar = read_json_file(
            &root
                .join("tools")
                .join("vscode-englang")
                .join("syntaxes")
                .join("eng.tmLanguage.json"),
        );
        let mut textmate_words = BTreeSet::new();
        collect_textmate_keyword_words(&grammar, &mut textmate_words);

        let completion_keywords = COMPLETION_KEYWORDS.iter().copied().collect::<BTreeSet<_>>();
        let missing = textmate_words
            .iter()
            .filter(|word| !completion_keywords.contains(word.as_str()))
            .cloned()
            .collect::<Vec<_>>();

        assert!(
            missing.is_empty(),
            "TextMate keyword fallback words missing from LSP keyword completions: {}",
            missing.join(", ")
        );
    }

    #[test]
    fn vscode_scope_mapping_covers_fixture_semantic_token_pairs() {
        let root = repo_root_for_tests();
        let package = read_json_file(
            &root
                .join("tools")
                .join("vscode-englang")
                .join("package.json"),
        );
        let scope_keys = package_semantic_scope_keys(&package);
        let fixture_dir = root
            .join("tools")
            .join("vscode-englang")
            .join("test")
            .join("grammar-fixtures");
        let mut fixtures = std::fs::read_dir(&fixture_dir)
            .unwrap_or_else(|error| {
                panic!(
                    "failed to read grammar fixture dir {}: {error}",
                    fixture_dir.display()
                )
            })
            .map(|entry| {
                entry
                    .expect("grammar fixture entry should be readable")
                    .path()
            })
            .filter(|path| path.extension().is_some_and(|extension| extension == "eng"))
            .collect::<Vec<_>>();
        fixtures.sort();

        let mut missing = BTreeSet::new();
        for fixture in fixtures {
            let source = std::fs::read_to_string(&fixture).unwrap_or_else(|error| {
                panic!(
                    "failed to read grammar fixture {}: {error}",
                    fixture.display()
                )
            });
            let snapshot = snapshot_for_source(&fixture, &source);
            let fixture_name = fixture
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("<fixture>");
            for token in &snapshot.semantic_tokens.tokens {
                if token.modifiers.is_empty() {
                    if !scope_keys.contains(token.token_type.as_str()) {
                        missing.insert(format!("{fixture_name}: {}", token.token_type));
                    }
                } else {
                    for modifier in &token.modifiers {
                        let scope_key = format!("{}.{}", token.token_type, modifier);
                        if !scope_keys.contains(scope_key.as_str()) {
                            missing.insert(format!("{fixture_name}: {scope_key}"));
                        }
                    }
                }
            }
        }

        assert!(
            missing.is_empty(),
            "VS Code semanticTokenScopes missing fallback mappings for fixture token pairs: {}",
            missing.into_iter().collect::<Vec<_>>().join(", ")
        );
    }

    #[test]
    fn diagnostic_json_uses_source_token_ranges() {
        let source = "schema SensorData {\n    m_dot = 1 kg/s\n}\n";
        let snapshot = snapshot_for_source(Path::new("schema.eng"), source);
        let json = snapshot_json(&snapshot);
        let diagnostic = json["diagnostics"]
            .as_array()
            .and_then(|diagnostics| {
                diagnostics
                    .iter()
                    .find(|diagnostic| diagnostic["code"] == "E-PUBLIC-ANNOTATION-001")
            })
            .expect("schema annotation diagnostic");
        let line = source.lines().nth(1).expect("schema column line");
        let equals_character = line.find('=').expect("assignment operator");

        assert_eq!(diagnostic["range"]["start"]["line"].as_u64(), Some(1));
        assert_eq!(
            diagnostic["range"]["start"]["character"].as_u64(),
            Some(equals_character as u64)
        );
        assert_eq!(
            diagnostic["range"]["end"]["character"].as_u64(),
            Some((equals_character + 1) as u64)
        );
    }

    #[test]
    fn diagnostic_json_pins_editor_underline_targets() {
        let component_unknown_signal = r#"domain Thermal {
    across T: AbsoluteTemperature [degC]
    through Q: HeatRate [kW]
    conservation sum(Q) = 0
}

component Source {
    port heat: Thermal
    heat.unknown eq 0 kW
}

component Sink {
    port heat: Thermal
}

connect Source.heat -> Sink.heat
"#;
        let where_local_escape = "E = integrate Q_local over Time\nwhere {\n    Q_local = Q_late\n    Q_late = 1 kW\n}\nprint \"local={Q_local: .2 kW}\"\n";

        for (code, source, expected_text) in [
            ("E-DIM-ADD-002", "Q: HeatRate [kW] = 2 kW - 1\n", "-"),
            (
                "E-COMPONENT-EQUATION-SIGNAL-001",
                component_unknown_signal,
                "heat.unknown",
            ),
            ("E-NAME-LOCAL-001", where_local_escape, "Q_local"),
            (
                "E-SCRIPT-001",
                "script main(args: Args) -> Report {\n    L = 1 m\n}\n",
                "script",
            ),
            (
                "E-STRUCT-ARGS-001",
                "struct Args {\n    input: String = \"sensor.csv\"\n}\n",
                "struct Args",
            ),
            (
                "E-UNC-DIRECT-COMPARE",
                "Q = normal(mean=5 kW, std=0.8 kW, samples=31)\nvalidate 10 kW < Q\n",
                "Q",
            ),
        ] {
            assert_first_diagnostic_underlines(source, code, expected_text);
        }
    }

    #[test]
    fn snapshot_marks_review_risk_semantic_tokens() {
        let source = r#"schema SensorData {
    reading: HeatRate [kW]

    missing {
        reading: interpolate max_gap=10 min
    }
}

process_result = run command "cmd"
with {
    expected_outputs = ["outputs/result.txt"]
}

export summary to csv "summary.csv" {
    T_measured as degC
}
write text "summary.txt", "ok"

response = http get url("https://example.org/weather")
download url("https://example.org/file.csv") to file("build/raw/file.csv")

T_measured = measured(12 degC, std=0.2 K)

system RoomThermal {
    state T_zone: AbsoluteTemperature = 22 degC
}
"#;
        let snapshot = snapshot_for_source(Path::new("risk.eng"), source);

        assert_semantic_token_modifier(&snapshot, source, "process_result", "riskHigh");
        assert_semantic_token_modifier(&snapshot, source, "run", "riskHigh");
        assert_semantic_token_modifier(&snapshot, source, "export", "riskHigh");
        assert_semantic_token_modifier(&snapshot, source, "write", "riskHigh");
        assert_semantic_token_modifier(&snapshot, source, "download", "riskHigh");
        assert_semantic_token_modifier(&snapshot, source, "response", "riskMedium");
        assert_semantic_token_modifier(&snapshot, source, "http", "riskMedium");
        assert_semantic_token_modifier(&snapshot, source, "reading", "riskMedium");
        assert_semantic_token_modifier(&snapshot, source, "T_measured", "riskMedium");
        assert_semantic_token_modifier(&snapshot, source, "RoomThermal", "riskMedium");
    }

    #[test]
    fn snapshot_marks_model_db_cache_and_workflow_step_semantic_tokens() {
        let source = r#"schema SensorData {
    time: DateTime index
    T_supply: AbsoluteTemperature [degC]
    T_return: AbsoluteTemperature [degC]
    m_dot: MassFlowRate [kg/s]

    constraints {
        time is monotonic
        T_supply between 0 degC and 60 degC
    }

    missing {
        T_supply: interpolate max_gap=10 min
        T_return: error
    }
}

sensor = promote csv file("data/sensor.csv") as SensorData
cp = 4180 J/kg/K
Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
split = train_test_split(Q_coil, target=Q_coil, features=[T_supply], test=0.5, seed=7)
reg_model = regression(split, algorithm=linear)
with {
    cache = true
    cache_key = ["model", "reg", "v1"]
    epochs = 20
}

db = open sqlite file("outputs/results.sqlite")
write sensor to db.table("sensor")
with {
    mode = upsert
    mode = replace
    key = time
    transaction = commit
    transaction = rollback
}

write standard_text sensor
with {
    output = "outputs/sensor_standard.txt"
}

selected = require_one sensor
with {
    on_none = error "No sensor row"
    on_many = error "Multiple sensor rows"
}

Q_series: TimeSeries[Time] of HeatRate [kW] = 5 kW
Q_dist = distribution(kind=normal, mean=5 kW, sigma=0.8 kW, n=31)
with {
    sensor_std = 0.2 kW
}

sim = simulate RoomThermal
with {
    timestep = 60 s
    duration = 1 h
    solver = adaptive_heun
    tolerance = 0.001
}

report {
    plot Q_series over Time
    plot distribution(Q_dist)
    with {
        unit y = kW
        title = "Coil heat"
        confidence_band = sensor_std
    }
}

cases = materialize sensor
with {
    step = "prepare"
    output_root = dir("outputs/cases")
    case_id = time
    resume = true
}

upload = http get url("https://example.org/weather")
with {
    query = {
        station = "demo"
    }
    offline_response = file("data/weather-response.json")
    expected_sha256 = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    status_code = 200
    body_size_limit = 2 MB
}

write text "outputs/out.txt", "ok"
with {
    overwrite = true
}
"#;
        let snapshot = snapshot_for_source(Path::new("roles.eng"), source);

        assert_semantic_token_modifier(&snapshot, source, "reg_model", "model");
        assert_semantic_token_modifier(&snapshot, source, "reg_model", "cache");
        assert_semantic_token_modifier(&snapshot, source, "cache_key", "cache");
        assert_semantic_token_modifier(&snapshot, source, "epochs", "model");
        assert_semantic_token_modifier(&snapshot, source, "db", "db");
        assert_semantic_token_modifier(&snapshot, source, "write", "db");
        assert_semantic_token_modifier(&snapshot, source, "mode", "db");
        assert_semantic_token_modifier(&snapshot, source, "upsert", "db");
        assert_semantic_token_modifier(&snapshot, source, "replace", "db");
        assert_semantic_token_modifier(&snapshot, source, "key", "db");
        assert_semantic_token_modifier(&snapshot, source, "transaction", "db");
        assert_semantic_token_modifier(&snapshot, source, "commit", "db");
        assert_semantic_token_modifier(&snapshot, source, "rollback", "db");
        assert_semantic_token_modifier(&snapshot, source, "linear", "model");
        assert_semantic_token_modifier(&snapshot, source, "standard_text", "workflowStep");
        assert_semantic_token_modifier(&snapshot, source, "output", "sideEffect");
        assert_semantic_token_modifier(&snapshot, source, "materialize", "workflowStep");
        assert_semantic_token_modifier(&snapshot, source, "step", "workflowStep");
        assert_semantic_token_modifier(&snapshot, source, "output_root", "workflowStep");
        assert_semantic_token_modifier(&snapshot, source, "case_id", "workflowStep");
        assert_semantic_token_modifier(&snapshot, source, "resume", "workflowStep");
        assert_semantic_token_modifier(&snapshot, source, "on_none", "validation");
        assert_semantic_token_modifier(&snapshot, source, "on_many", "validation");
        assert_semantic_token_modifier(&snapshot, source, "constraints", "validation");
        assert_semantic_token_modifier(&snapshot, source, "missing", "validation");
        assert_semantic_token_modifier(&snapshot, source, "monotonic", "validation");
        assert_semantic_token_modifier(&snapshot, source, "between", "validation");
        assert_semantic_token_modifier(&snapshot, source, "interpolate", "validation");
        assert_semantic_token_modifier(&snapshot, source, "max_gap", "validation");
        assert_semantic_token_modifier(&snapshot, source, "error", "validation");
        assert_semantic_token_modifier(&snapshot, source, "sensor_std", "uncertain");
        assert_semantic_token_modifier(&snapshot, source, "confidence_band", "uncertain");
        assert_semantic_token_modifier(&snapshot, source, "Q_dist", "uncertain");
        assert_eq!(
            semantic_token_modifier_count(
                &snapshot,
                source,
                "distribution",
                "function",
                "uncertain"
            ),
            1,
            "`distribution(...)` should be an uncertainty builtin"
        );
        assert_eq!(
            semantic_token_modifier_count(&snapshot, source, "distribution", "function", "report"),
            1,
            "`plot distribution(...)` should keep report context"
        );
        assert_semantic_token_modifier(&snapshot, source, "unit y", "report");
        assert_semantic_token_modifier(&snapshot, source, "title", "report");
        assert_semantic_token_modifier(&snapshot, source, "timestep", "solver");
        assert_semantic_token_modifier(&snapshot, source, "duration", "solver");
        assert_semantic_token_modifier(&snapshot, source, "solver", "solver");
        assert_semantic_token_modifier(&snapshot, source, "adaptive_heun", "solver");
        assert_semantic_token_modifier(&snapshot, source, "tolerance", "solver");
        assert_semantic_token_modifier(&snapshot, source, "query", "external");
        assert_semantic_token_modifier(&snapshot, source, "station", "external");
        assert_semantic_token_modifier(&snapshot, source, "offline_response", "external");
        assert_semantic_token_modifier(&snapshot, source, "expected_sha256", "external");
        assert_semantic_token_modifier(&snapshot, source, "status_code", "external");
        assert_semantic_token_modifier(&snapshot, source, "body_size_limit", "external");
        assert_semantic_token_modifier(&snapshot, source, "overwrite", "sideEffect");
    }

    #[test]
    fn snapshot_marks_richer_keyword_semantic_modifiers() {
        let source = r#"system RoomThermal {
    state T: AbsoluteTemperature = 20 degC
    output Q_load: HeatRate = 1 kW
    equation balance:
        der(T) eq 0 K/s
}

simulate RoomThermal
with {
    timestep = 60 s
    duration = 1 h
    solver = fixed_step
}

report {
    summarize T
    distribution T
    plot line T vs Time
}

payload = read json file("payload.json")
settings = read toml file("settings.toml")
choice = if true else false
Q_series: TimeSeries[Time] of HeatRate [kW] = 5 kW
schema JoinRow {
    id: String
}
left = promote csv file("left.csv") as JoinRow
right = promote csv file("right.csv") as JoinRow
joined = join left with right
on {
    left.id == right.id
}
export summary to csv "summary.csv" {
    T as degC
}
write text "summary.txt", "ok"

test "temperature stays bounded" {
    assert T matches T within 1 K
}

render template file("report.md") to file("report.html")
response = http get url("https://example.org/weather")
submitted = http post url("https://example.org/weather")
log debug "debug details"
log info "ready"
log warn "slow"
profile_safe = safe
profile_repro = repro
cache_status = cached
cache_freshness = stale
cache_lookup = hit
cache_miss = miss
case_created = created
case_updated = updated
case_metadata = metadata_ready
case_warning = warnings_present
case_diagnostics = diagnostics_present
solver_mode = rk4

script LegacyScript
struct LegacyArgs
"#;
        let snapshot = snapshot_for_source(Path::new("keyword_modifiers.eng"), source);

        for label in ["simulate", "equation", "der"] {
            assert_semantic_token_modifier(&snapshot, source, label, "solver");
        }
        for label in ["summarize", "summary", "distribution", "line"] {
            assert_semantic_token_modifier(&snapshot, source, label, "report");
        }
        for label in ["else", "of", "on", "output", "vs"] {
            assert_semantic_token_type(&snapshot, source, label, "keyword");
        }
        for label in ["read", "csv", "json", "toml", "text"] {
            assert_semantic_token_modifier(&snapshot, source, label, "workflowStep");
        }
        for label in ["assert", "matches", "within"] {
            assert_semantic_token_modifier(&snapshot, source, label, "validation");
        }
        for label in ["export", "write", "render", "template"] {
            assert_semantic_token_modifier(&snapshot, source, label, "sideEffect");
        }
        for label in ["http", "get", "post"] {
            assert_semantic_token_modifier(&snapshot, source, label, "external");
        }
        for label in [
            "debug",
            "info",
            "warn",
            "safe",
            "repro",
            "cached",
            "stale",
            "hit",
            "miss",
            "created",
            "updated",
            "metadata_ready",
            "warnings_present",
            "diagnostics_present",
            "rk4",
        ] {
            assert_semantic_token_type(&snapshot, source, label, "keyword");
        }
        for label in ["cached", "stale", "hit", "miss"] {
            assert_semantic_token_modifier(&snapshot, source, label, "cache");
        }
        for label in [
            "created",
            "updated",
            "metadata_ready",
            "warnings_present",
            "diagnostics_present",
        ] {
            assert_semantic_token_modifier(&snapshot, source, label, "workflowStep");
        }
        assert_semantic_token_modifier(&snapshot, source, "rk4", "solver");
        for label in ["script", "struct"] {
            assert_semantic_token_modifier(&snapshot, source, label, "deprecated");
        }
        for label in ["LegacyScript", "LegacyArgs"] {
            assert_semantic_token_type(&snapshot, source, label, "class");
            assert_semantic_token_modifier(&snapshot, source, label, "declaration");
            assert_semantic_token_modifier(&snapshot, source, label, "deprecated");
        }
    }

    #[test]
    fn snapshot_marks_core_symbol_roles_as_semantic_tokens() {
        let source = r#"import eng.table
use eng.stats
use eng.system

const cp_water: SpecificHeat [J/kg/K] = 4180 J/kg/K

schema SensorData {
    time: DateTime [iso8601]
    T_supply: AbsoluteTemperature [degC]
}

schema WorkflowArtifactRefs {
    api_key: Secret[String]
    predictions: Table[T]
    maybe_output: Optional[DirectoryPath]
    heat_series: TimeSeries[Time]
}

args {
    input: CsvFile = file("data/sensor.csv")
}

sensor = 1 kg/s

fn coil_heat(m_dot: MassFlowRate, dT: TemperatureDelta) -> HeatRate {
    Q = m_dot * cp_water * dT
    return Q
}
"#;
        let snapshot = snapshot_for_source(Path::new("core_symbol_roles.eng"), source);

        assert_semantic_token_type(&snapshot, source, "eng.table", "namespace");
        assert_semantic_token_modifier(&snapshot, source, "eng.table", "imported");
        assert_semantic_token_modifier(&snapshot, source, "eng.table", "declaration");
        assert_semantic_token_modifier(&snapshot, source, "eng.table", "defaultLibrary");
        assert_semantic_token_type(&snapshot, source, "eng.stats", "namespace");
        assert_semantic_token_modifier(&snapshot, source, "eng.stats", "planned");
        assert_semantic_token_modifier(&snapshot, source, "eng.stats", "defaultLibrary");
        assert_semantic_token_type(&snapshot, source, "eng.system", "namespace");
        assert_semantic_token_modifier(&snapshot, source, "eng.system", "internal");
        assert_semantic_token_modifier(&snapshot, source, "eng.system", "defaultLibrary");
        assert_semantic_token_type(&snapshot, source, "cp_water", "variable");
        assert_semantic_token_modifier(&snapshot, source, "cp_water", "readonly");
        assert_semantic_token_modifier(&snapshot, source, "cp_water", "declaration");
        assert_semantic_token_type(&snapshot, source, "SensorData", "class");
        assert_semantic_token_type(&snapshot, source, "time", "property");
        assert_semantic_token_modifier(&snapshot, source, "time", "declaration");
        for label in [
            "Secret",
            "String",
            "Table",
            "T",
            "Optional",
            "DirectoryPath",
            "TimeSeries",
            "Time",
        ] {
            assert_semantic_token_type(&snapshot, source, label, "type");
        }
        assert_semantic_token_type(&snapshot, source, "input", "parameter");
        assert_semantic_token_modifier(&snapshot, source, "input", "declaration");
        assert_semantic_token_type(&snapshot, source, "sensor", "variable");
        assert_semantic_token_type(&snapshot, source, "m_dot", "parameter");
        assert_semantic_token_modifier(&snapshot, source, "m_dot", "declaration");
        assert_semantic_token_type(&snapshot, source, "Q", "variable");
        assert_semantic_token_modifier(&snapshot, source, "Q", "local");
        assert_eq!(
            semantic_token_count(&snapshot, source, "m_dot", "parameter"),
            2,
            "function parameter declaration and body reference should both be semantic tokens"
        );
        assert_eq!(
            semantic_token_count(&snapshot, source, "dT", "parameter"),
            2,
            "second function parameter declaration and body reference should both be semantic tokens"
        );
        assert_eq!(
            semantic_token_count(&snapshot, source, "Q", "variable"),
            2,
            "function local declaration and return reference should both be semantic tokens"
        );
        assert_semantic_token_on_line_without_modifier(
            &snapshot,
            source,
            "Q = m_dot",
            "m_dot",
            "parameter",
            "declaration",
        );
        assert_semantic_token_on_line_without_modifier(
            &snapshot,
            source,
            "return Q",
            "Q",
            "variable",
            "declaration",
        );
    }

    #[test]
    fn snapshot_marks_state_space_type_arguments_as_semantic_tokens() {
        let source = r#"states RoomState {
    T_air: AbsoluteTemperature [degC]
}

inputs RoomInput {
    T_out: AbsoluteTemperature [degC]
}

outputs RoomOutput {
    T_zone: AbsoluteTemperature [degC]
}

system StateSpaceFixture {
    state x: StateVector[RoomState] = [22 degC]
    input u: InputVector[RoomInput] = [8 degC]
    output y: OutputVector[RoomOutput]
    operator A: LinearOperator[RoomState -> Derivative[RoomState]] = [[-0.012 1/min]]
}
"#;
        let snapshot = snapshot_for_source(Path::new("state_space_types.eng"), source);

        for label in ["states", "inputs", "outputs", "operator"] {
            assert_semantic_token_modifier(&snapshot, source, label, "solver");
        }
        for label in [
            "StateVector",
            "InputVector",
            "OutputVector",
            "LinearOperator",
            "Derivative",
            "RoomState",
            "RoomInput",
            "RoomOutput",
        ] {
            assert_semantic_token_type(&snapshot, source, label, "type");
        }
    }

    #[test]
    fn snapshot_marks_native_workflow_builtins_as_semantic_functions() {
        let source = r#"designs = sample lhs
with {
    count = 2
    seed = 7
    people_density = uniform(0.03 person/m2, 0.12 person/m2)
}

alias_designs = sample uniform
with {
    count = 2
    seed = 11
    cooling_cop = uniform(2.5, 5.0)
}

latin_designs = sample latin_hypercube
with {
    count = 2
    seed = 13
    cooling_cop = uniform(2.5, 5.0)
}

latin_hyphen_designs = sample latin-hypercube
with {
    count = 2
    seed = 17
    cooling_cop = uniform(2.5, 5.0)
}

case_row = require_one designs
surrogate = train regression designs
with {
    target = annual_electricity
    features = [people_density]
    test = 0.25
    seed = 7
}
ann_model = ann(split_alias, hidden=[8], epochs=10)
metrics = evaluate(surrogate)
card = model_card(surrogate)
predictions = predict surrogate using designs
filtered = filter designs
selected = select designs columns people_density, cooling_cop
sorted = sort designs by cooling_cop desc
joined = join designs with predictions
derived = derive designs column annual_electricity = people_density * 1 kWh
derived_many = derive designs columns annual_cooling = cooling_cop * 1 kWh
cases = materialize cases designs
case_results = apply run_case over designs
case_inputs = apply case_input_template over cases
collected = collect results case_results
filled = fill missing designs.cooling_cop
legacy_station = select_first_row(stations, return_column="station_id")
"#;
        let snapshot = snapshot_for_source(Path::new("native_workflow_builtins.eng"), source);

        for label in [
            "sample",
            "lhs",
            "uniform",
            "latin_hypercube",
            "latin-hypercube",
            "filter",
            "select",
            "sort",
            "join",
            "require_one",
            "materialize",
            "apply",
            "collect",
            "train",
            "regression",
            "ann",
            "evaluate",
            "model_card",
            "predict",
            "derive",
            "fill",
            "select_first_row",
        ] {
            assert_semantic_token_type(&snapshot, source, label, "function");
            assert_semantic_token_modifier(&snapshot, source, label, "defaultLibrary");
        }
        for label in [
            "train",
            "regression",
            "ann",
            "evaluate",
            "model_card",
            "predict",
        ] {
            assert_semantic_token_modifier(&snapshot, source, label, "model");
        }
        assert_semantic_token_modifier(&snapshot, source, "using", "model");
        assert_eq!(
            semantic_token_modifier_count(&snapshot, source, "surrogate", "variable", "model"),
            4,
            "model declaration plus evaluate/model_card/predict references should be model tokens"
        );
        assert_eq!(
            semantic_token_modifier_count(&snapshot, source, "designs", "variable", "model"),
            2,
            "regression and prediction table operands should be model tokens"
        );
        for label in ["annual_electricity", "people_density"] {
            assert_semantic_token_modifier(&snapshot, source, label, "model");
        }
        for label in [
            "sample",
            "lhs",
            "uniform",
            "latin_hypercube",
            "latin-hypercube",
            "filter",
            "select",
            "sort",
            "join",
            "require_one",
            "materialize",
            "apply",
            "collect",
            "derive",
            "fill",
        ] {
            assert_semantic_token_modifier(&snapshot, source, label, "workflowStep");
        }
        assert_semantic_token_modifier(&snapshot, source, "fill", "validation");
        assert_semantic_token_modifier(&snapshot, source, "missing", "validation");
        for label in ["designs", "case_row", "derived", "derived_many"] {
            assert_semantic_token_modifier(&snapshot, source, label, "workflowStep");
        }
        assert_eq!(
            semantic_token_modifier_count(&snapshot, source, "cases", "variable", "workflowStep"),
            1,
            "materialized case table binding should be a workflow-step semantic token"
        );
        assert_eq!(
            semantic_token_modifier_count(
                &snapshot,
                source,
                "case_inputs",
                "variable",
                "workflowStep"
            ),
            1,
            "case apply output binding should be a workflow-step semantic token"
        );
        for label in ["column", "columns", "results"] {
            assert_semantic_token_modifier(&snapshot, source, label, "workflowStep");
        }
        for label in [
            "annual_electricity",
            "annual_cooling",
            "people_density",
            "cooling_cop",
        ] {
            assert_semantic_token_modifier(&snapshot, source, label, "workflowStep");
        }
        assert_semantic_token_modifier(&snapshot, source, "uniform", "uncertain");
        assert_semantic_token_type(&snapshot, source, "column", "keyword");
        assert_semantic_token_type(&snapshot, source, "columns", "keyword");
        assert_semantic_token_modifier(&snapshot, source, "select_first_row", "deprecated");
        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "column"));
        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "columns"));
        assert!(!snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "select_first_row"));
    }

    #[test]
    fn snapshot_marks_json_records_promotion_as_workflow_metadata() {
        let source = r#"schema WeatherApiRecord {
    time: DateTime index
    value: Float
}

schema WeatherApiPayload {
    records: Array[WeatherApiRecord]
}

payload = read json file("data/weather.json")
api_contract = promote json payload as WeatherApiPayload
weather = promote json records payload.records as WeatherApiRecord
"#;
        let snapshot = snapshot_for_source(Path::new("json_records.eng"), source);

        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "promote json records"));
        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "records"));
        assert_semantic_token_type(&snapshot, source, "records", "keyword");
        assert_semantic_token_modifier(&snapshot, source, "records", "workflowStep");
        assert_semantic_token_type(&snapshot, source, "weather", "variable");
        let weather_symbol = snapshot
            .document_symbols
            .iter()
            .find(|symbol| symbol.name == "weather")
            .expect("weather document symbol");
        assert_eq!(weather_symbol.detail, "json_records as WeatherApiRecord");
    }

    #[test]
    fn snapshot_exposes_http_response_member_fields() {
        let source = "response = http get url(\"https://api.example.org/hourly\")\nwith {\n    offline_response = file(\"data/response.json\")\n}\n\nresponse_text = response.body\n";
        let snapshot = snapshot_for_source(Path::new("net.eng"), source);

        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "response.body"));
        let line = source
            .lines()
            .position(|line| line.contains("response_text"))
            .expect("response_text line");
        let member_completions = completion_items_for_source_position(
            Path::new("net.eng"),
            source,
            line,
            "response_text = response.".len(),
        );
        let body_completion = member_completions
            .iter()
            .find(|completion| completion.label == "body")
            .expect("HTTP response member completion should include body");
        assert_eq!(
            body_completion.detail,
            "pinned offline HTTP response body text"
        );
        assert!(member_completions
            .iter()
            .any(|completion| completion.label == "status_code"));
        assert!(snapshot.semantic_tokens.tokens.iter().any(|token| {
            token.token_type == "property"
                && source
                    .lines()
                    .nth(token.line)
                    .is_some_and(|line| &line[token.start..token.start + token.length] == "body")
        }));
    }

    #[test]
    fn table_row_member_completion_uses_require_one_schema() {
        let source = "schema StationMap {\n    region: String\n    station_id: String\n    latitude: DimensionlessNumber [1]\n}\n\nstations = promote csv file(\"data/stations.csv\") as StationMap\ncandidates = filter stations\nwhere {\n    region == \"demo\"\n}\nstation = require_one candidates\nselected_station_id: String = station.\n";
        let snapshot = snapshot_for_source(Path::new("table_row.eng"), source);

        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "station.station_id"));
        let line = source
            .lines()
            .position(|line| line.contains("selected_station_id"))
            .expect("selected_station_id line");
        let member_completions = completion_items_for_source_position(
            Path::new("table_row.eng"),
            source,
            line,
            "selected_station_id: String = station.".len(),
        );
        assert!(member_completions
            .iter()
            .any(|completion| completion.label == "station_id"));
        assert!(member_completions
            .iter()
            .any(|completion| completion.label == "latitude"));
    }

    #[test]
    fn snapshot_exposes_domain_component_hover_and_completion() {
        let source = "domain Thermal package \"eng.std.domains.thermal\" version \"0.1.0\" {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ndomain Fluid[Medium M] {\n    across height: Length [m]\n    through m_dot: MassFlowRate [kg/s]\n    conservation sum(m_dot) = 0\n}\n\ncomponent RoomBoundary {\n    port heat: Thermal\n}\n\ncomponent AmbientBoundary {\n    port heat: Thermal\n}\n\ncomponent SupplyPipe {\n    port inlet: Fluid[Water]\n    port outlet: Fluid[Water]\n}\n\nconnect RoomBoundary.heat -> AmbientBoundary.heat\nconnect SupplyPipe.inlet -> SupplyPipe.outlet\n";
        let snapshot = snapshot_for_source(Path::new("domain.eng"), source);

        assert_semantic_token_type(&snapshot, source, "eng.std.domains.thermal", "namespace");
        assert_semantic_token_modifier(
            &snapshot,
            source,
            "eng.std.domains.thermal",
            "defaultLibrary",
        );
        assert_semantic_token_modifier(&snapshot, source, "eng.std.domains.thermal", "internal");
        assert!(snapshot
            .hovers
            .iter()
            .any(|hover| hover.kind == "domain" && hover.name == "Thermal"));
        assert!(snapshot.hovers.iter().any(|hover| {
            hover.kind == "domain_variable"
                && hover.name == "Thermal.T"
                && hover.quantity_kind == "AbsoluteTemperature"
                && hover.display_unit == "degC"
        }));
        assert!(snapshot.hovers.iter().any(|hover| {
            hover.kind == "component_port"
                && hover.name == "RoomBoundary.heat"
                && hover.status.as_deref() == Some("domain_resolved")
        }));
        assert!(snapshot.hovers.iter().any(|hover| {
            hover.kind == "component_port"
                && hover.name == "SupplyPipe.inlet"
                && hover.detail.contains("type Fluid[Water]")
                && hover.detail.contains("domain Fluid")
                && hover.detail.contains("medium Water")
        }));
        assert!(snapshot.hovers.iter().any(|hover| {
            hover.kind == "connection" && hover.status.as_deref() == Some("domain_compatible")
        }));
        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "Thermal"));
        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "RoomBoundary.heat"));

        let json = snapshot_json(&snapshot);
        let hovers = json["hovers"].as_array().unwrap();
        assert!(hovers
            .iter()
            .any(|hover| hover["kind"] == "connection" && hover["status"] == "domain_compatible"));
        assert!(snapshot.document_symbols.iter().any(|symbol| {
            symbol.name == "Thermal"
                && symbol.children.iter().any(|child| {
                    child.name == "T"
                        && child.kind == SYMBOL_KIND_PROPERTY
                        && child.line == 1
                        && child.end_line == 1
                })
        }));
        assert!(snapshot
            .folding_ranges
            .iter()
            .any(|range| range.start_line == 0 && range.end_line == 4));
        assert!(json["folding_ranges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|range| range["startLine"] == 0 && range["endLine"] == 4));
    }

    #[test]
    fn member_completion_uses_csv_promotion_schema_columns() {
        let source = r#"schema SensorData {
    time: DateTime index
    T_supply: AbsoluteTemperature [degC]
    T_return: AbsoluteTemperature [degC]
    m_dot: MassFlowRate [kg/s]
}

sensor = promote csv "missing.csv" as SensorData
Q = sensor.T
"#;
        let line = source
            .lines()
            .position(|line| line.contains("sensor.T"))
            .unwrap();
        let character =
            source.lines().nth(line).unwrap().find("sensor.T").unwrap() + "sensor.T".len();
        let report = check_source(
            Path::new("completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        assert!(completions
            .iter()
            .any(|completion| completion.label == "T_supply"));
        assert!(completions
            .iter()
            .any(|completion| completion.label == "T_return"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "schema"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "m_dot"));
    }

    #[test]
    fn with_block_completion_uses_owner_context() {
        let source = r#"response = http post url("https://api.example.org/hourly")
with {
    
}
"#;
        let line = source
            .lines()
            .position(|line| line.trim().is_empty())
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("network_with_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        assert!(completions
            .iter()
            .any(|completion| completion.label == "expected_sha256"));
        assert!(completions
            .iter()
            .any(|completion| completion.label == "body_size_limit"));
        assert!(completions
            .iter()
            .any(|completion| completion.label == "cache_key"));
        assert!(completions
            .iter()
            .any(|completion| completion.label == "body"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "response_hash"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "recursive"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "HeatRate"));
    }

    #[test]
    fn with_block_completion_skips_existing_options() {
        let source = r#"delete dir("old")
with {
    confirm = true
    
}
"#;
        let line = source
            .lines()
            .position(|line| line.trim().is_empty())
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("delete_with_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        assert!(completions
            .iter()
            .any(|completion| completion.label == "recursive"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "confirm"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "expected_sha256"));
    }

    #[test]
    fn function_argument_completion_uses_uncertainty_context() {
        let source = "Q = distribution(kind=normal, sig";
        let line = 0;
        let character = source.len();
        let report = check_source(
            Path::new("uncertainty_arg_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        let sigma = completions
            .iter()
            .find(|completion| completion.label == "sigma")
            .expect("distribution argument completion should include sigma");
        assert_eq!(sigma.kind, "property");
        assert_eq!(sigma.detail, "uncertainty standard deviation alias");
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "kind"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "target"));

        let source = "Q = distribution(";
        let report = check_source(
            Path::new("distribution_arg_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, source.len());
        for label in [
            "kind",
            "distribution",
            "mean",
            "sigma",
            "lower",
            "upper",
            "n",
        ] {
            assert!(
                completions
                    .iter()
                    .any(|completion| completion.label == label && completion.kind == "property"),
                "distribution argument completion should include property {label}"
            );
        }
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "cache_key"));

        let source = r#"Q = distribution(
    kind=normal,
    sig"#;
        let line = 2;
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("multiline_distribution_arg_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);
        assert!(completions
            .iter()
            .any(|completion| completion.label == "sigma" && completion.kind == "property"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "kind"));
    }

    #[test]
    fn function_argument_completion_uses_model_context() {
        let source = "model = regression(split, fe";
        let line = 0;
        let character = source.len();
        let report = check_source(
            Path::new("model_arg_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        let features = completions
            .iter()
            .find(|completion| completion.label == "features")
            .expect("regression argument completion should include features");
        assert_eq!(features.kind, "property");
        assert_eq!(features.detail, "model feature columns");
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "sigma"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "cache_key"));

        let source = r#"model = regression(
    split,
    fe"#;
        let line = 2;
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("multiline_model_arg_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);
        assert!(completions
            .iter()
            .any(|completion| completion.label == "features" && completion.kind == "property"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "sigma"));
    }

    #[test]
    fn with_block_completion_uses_plot_context() {
        let source = r#"plot Q_sensor over Time
with {

}
"#;
        let line = source
            .lines()
            .position(|line| line.trim().is_empty())
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("plot_with_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        assert!(completions
            .iter()
            .any(|completion| completion.label == "confidence_band"));
        assert!(completions
            .iter()
            .any(|completion| completion.label == "unit y"));
        assert!(completions
            .iter()
            .any(|completion| completion.label == "unit x"));
        assert!(completions
            .iter()
            .any(|completion| completion.label == "title"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "unit"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "expected_sha256"));

        let source = r#"plot Q_sensor over Time
with {
    unit y = kW

}
"#;
        let line = source
            .lines()
            .position(|line| line.trim().is_empty())
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("plot_with_existing_unit_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        assert!(completions
            .iter()
            .any(|completion| completion.label == "unit x"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "unit y"));
    }

    #[test]
    fn with_block_completion_uses_require_one_context() {
        let source = r#"selected = require_one rows
with {

}
"#;
        let line = source
            .lines()
            .position(|line| line.trim().is_empty())
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("require_one_with_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        assert!(completions
            .iter()
            .any(|completion| completion.label == "on_none"));
        assert!(completions
            .iter()
            .any(|completion| completion.label == "on_many"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "expected_sha256"));
    }

    #[test]
    fn with_block_completion_uses_timeseries_context() {
        let source = r#"Q_series: TimeSeries[Time] of HeatRate [kW] = 5 kW
with {

}
"#;
        let line = source
            .lines()
            .position(|line| line.trim().is_empty())
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("timeseries_with_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        assert!(completions
            .iter()
            .any(|completion| completion.label == "sensor_std"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "expected_sha256"));
    }

    #[test]
    fn with_block_completion_uses_write_context() {
        let source = r#"write text "out.txt", "ok"
with {

}
"#;
        let line = source
            .lines()
            .position(|line| line.trim().is_empty())
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("write_with_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        assert!(completions
            .iter()
            .any(|completion| completion.label == "overwrite"));
        assert!(completions
            .iter()
            .any(|completion| completion.label == "mode"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "expected_sha256"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "transaction"));
    }

    #[test]
    fn with_block_completion_uses_db_write_context() {
        let source = r#"write sensor to db.table("sensor")
with {

}
"#;
        let line = source
            .lines()
            .position(|line| line.trim().is_empty())
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("db_write_with_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        for label in ["mode", "key", "transaction", "overwrite"] {
            assert!(
                completions
                    .iter()
                    .any(|completion| completion.label == label),
                "DB write with-block completion should include {label}"
            );
        }
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "expected_sha256"));
    }

    #[test]
    fn with_block_completion_uses_process_cache_context() {
        let source = r#"process_result = run command "tool"
with {

}
"#;
        let line = source
            .lines()
            .position(|line| line.trim().is_empty())
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("process_with_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        for label in [
            "expected_outputs",
            "tool_version",
            "allow_failure",
            "cache_ttl",
        ] {
            assert!(
                completions
                    .iter()
                    .any(|completion| completion.label == label),
                "process with-block completion should include {label}"
            );
        }
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "transaction"));
    }

    #[test]
    fn with_block_completion_uses_render_template_context() {
        let source = r#"render template file("model/base.txt")
with {

}
"#;
        let line = source
            .lines()
            .position(|line| line.trim().is_empty())
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("render_template_with_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        for label in ["values", "output", "missing", "overwrite", "artifact_kind"] {
            assert!(
                completions
                    .iter()
                    .any(|completion| completion.label == label),
                "render template with-block completion should include {label}"
            );
        }
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "expected_sha256"));
    }

    #[test]
    fn with_block_completion_uses_apply_template_context() {
        let source = r#"case_inputs = apply case_input_template over cases
with {

}
"#;
        let line = source
            .lines()
            .position(|line| line.trim().is_empty())
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("apply_template_with_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        for label in [
            "template",
            "values",
            "output",
            "missing",
            "overwrite",
            "artifact_kind",
        ] {
            assert!(
                completions
                    .iter()
                    .any(|completion| completion.label == label),
                "apply with-block completion should include {label}"
            );
        }
        let template_completion = completions
            .iter()
            .find(|completion| completion.label == "template")
            .expect("apply with-block completion should include template");
        assert_eq!(template_completion.kind, "property");
        assert_eq!(template_completion.detail, "template source file");
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "expected_sha256"));
    }

    #[test]
    fn with_block_completion_uses_model_training_context() {
        let source = r#"model = mlp(split)
with {

}
"#;
        let line = source
            .lines()
            .position(|line| line.trim().is_empty())
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("model_with_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        for label in ["algorithm", "hidden", "epochs", "cache_key"] {
            assert!(
                completions
                    .iter()
                    .any(|completion| completion.label == label),
                "model training with-block completion should include {label}"
            );
        }
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "expected_sha256"));
    }

    #[test]
    fn with_block_completion_uses_model_evaluation_context() {
        let source = r#"metrics = evaluate(model)
with {

}
"#;
        let line = source
            .lines()
            .position(|line| line.trim().is_empty())
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("model_evaluation_with_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        for label in ["split", "cache_key"] {
            assert!(
                completions
                    .iter()
                    .any(|completion| completion.label == label),
                "model evaluation with-block completion should include {label}"
            );
        }
        let split_completion = completions
            .iter()
            .find(|completion| completion.label == "split")
            .expect("model evaluation with-block completion should include split");
        assert_eq!(split_completion.kind, "property");
        assert_eq!(
            split_completion.detail,
            "Train/test split to evaluate or lint"
        );
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "expected_sha256"));
    }

    #[test]
    fn with_block_completion_uses_simulate_context() {
        let source = r#"simulate RoomThermal
with {

}
"#;
        let line = source
            .lines()
            .position(|line| line.trim().is_empty())
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("simulate_with_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        for label in [
            "timestep",
            "duration",
            "solver",
            "tolerance",
            "inputs",
            "initial",
        ] {
            assert!(
                completions
                    .iter()
                    .any(|completion| completion.label == label),
                "simulate with-block completion should include {label}"
            );
        }
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "expected_sha256"));
    }

    #[test]
    fn with_block_completion_uses_solve_context() {
        let source = r#"solve component_graph
with {

}
"#;
        let line = source
            .lines()
            .position(|line| line.trim().is_empty())
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("solve_with_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        for label in [
            "initial",
            "initial_derivative",
            "initial_algebraic",
            "algebraic_initialization",
            "inputs",
            "jacobian",
            "mass_matrix",
            "max_iter",
            "finite_difference_step",
            "damping",
            "line_search_steps",
            "relaxation",
            "residual_scale",
            "residual_scales",
            "consistency_tolerance",
            "variable_scale",
            "variable_scales",
        ] {
            assert!(
                completions
                    .iter()
                    .any(|completion| completion.label == label),
                "solve with-block completion should include {label}"
            );
        }
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "expected_sha256"));
    }

    #[test]
    fn snapshot_exposes_function_signature_hover_and_completion() {
        let source = r#"fn heat_loss(UA: Conductance [W/K], dT: TemperatureDelta [K]) -> HeatRate [W] {
    UA_local = UA
    dT_local = dT
    return UA_local * dT_local
}

Q = heat_loss(150 W/K, 8 K)
"#;
        let snapshot = snapshot_for_source(Path::new("functions.eng"), source);

        let function_hover = snapshot
            .hovers
            .iter()
            .find(|hover| hover.kind == "function" && hover.name == "heat_loss")
            .expect("function signature hover should be present");
        assert!(function_hover
            .detail
            .contains("fn heat_loss(UA: Conductance [W/K], dT: TemperatureDelta [K])"));
        assert!(function_hover.detail.contains("-> HeatRate [W]"));
        assert!(function_hover.detail.contains("locals UA_local, dT_local"));
        assert!(snapshot
            .hovers
            .iter()
            .any(|hover| { hover.kind == "function_local" && hover.name == "heat_loss.UA_local" }));
        assert!(snapshot.completions.iter().any(|completion| {
            completion.label == "heat_loss"
                && completion.kind == "function"
                && completion.detail.contains("-> HeatRate [W]")
        }));
    }

    #[test]
    fn snapshot_exposes_where_local_hover() {
        let source = r#"Q_coil = 5 kW
E_coil = integrate Q_for_energy over Time
where {
    Q_for_energy = Q_coil
}
"#;
        let snapshot = snapshot_for_source(Path::new("where.eng"), source);

        let hover = snapshot
            .hovers
            .iter()
            .find(|hover| hover.kind == "where_local" && hover.name == "where.Q_for_energy")
            .expect("where local hover should be present");
        assert_eq!(hover.quantity_kind, "HeatRate");
        assert_eq!(hover.display_unit, "W");
        assert!(hover.detail.contains("owner line 2"));
        assert!(hover.detail.contains("Q_for_energy"));
        assert!(hover.detail.contains("= Q_coil"));
    }

    #[test]
    fn object_literal_completion_marks_required_and_default_fields() {
        let source = r#"class Construction {
    name: String
    u_value: Conductance [W/K]
    thickness: Length [m] = 0.2 m
}

wall = Construction {

}
"#;
        let object_start_line = source
            .lines()
            .position(|line| line.contains("wall = Construction {"))
            .unwrap();
        let line = object_start_line + 1;
        let character = 0;
        let report = check_source(
            Path::new("class_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        let name = completions
            .iter()
            .find(|completion| completion.label == "name")
            .expect("object literal completion should include required name field");
        assert!(name
            .detail
            .contains("required String [-] from Construction"));
        let thickness = completions
            .iter()
            .find(|completion| completion.label == "thickness")
            .expect("object literal completion should include defaulted thickness field");
        assert!(thickness
            .detail
            .contains("default = 0.2 m Length [m] from Construction"));
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "schema"));
    }

    #[test]
    fn member_completion_marks_class_field_requirements() {
        let source = r#"class Construction {
    name: String
    thickness: Length [m] = 0.2 m
    method summary() -> String = self.name
}

wall = Construction {
    name = "south_wall"
}

wall_value = wall.
"#;
        let line = source
            .lines()
            .position(|line| line.contains("wall_value"))
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("class_member_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        assert!(completions.iter().any(|completion| {
            completion.label == "name"
                && completion
                    .detail
                    .contains("required String [-] from Construction")
        }));
        assert!(completions.iter().any(|completion| {
            completion.label == "thickness"
                && completion
                    .detail
                    .contains("default = 0.2 m Length [m] from Construction")
        }));
        assert!(completions.iter().any(|completion| {
            completion.label == "summary()"
                && completion.detail.contains("String [-] from Construction")
        }));
    }
}
