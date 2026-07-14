use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use eng_compiler::{
    all_quantity_completions, all_unit_infos, bundled_module_registry, check_source,
    classify_diagnostic_review_risk, classify_review_risk, db_read_expression,
    read_only_io_expression, CheckOptions, CheckReport, ClassFieldInfo, CommandStyleInfo,
    Diagnostic, DomainTypeParameterInfo, FileOperationInfo, FunctionInfo, SemanticProgram,
    Severity, WithBlockInfo, WithOptionInfo,
};
use serde_json::{json, Value};

pub const LSP_SNAPSHOT_FORMAT: &str = "eng-lsp-snapshot-v1";
pub const LSP_EDITOR_METADATA_FORMAT: &str = "eng-lsp-editor-metadata-v2";

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
    pub insert: Option<String>,
    pub insert_snippet: Option<String>,
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
    "documentation",
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
    "output",
    "model",
    "db",
    "cache",
    "workflowStep",
];

const COMPLETION_KEYWORDS: &[&str] = &[
    "across",
    "align",
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
    "index",
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
    "mkdir",
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
    "parity",
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
    "require_one",
    "report",
    "residuals",
    "resample",
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
    "train",
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

const PLOT_COMMAND_STYLE_WORDS: &[&str] = &[
    "line",
    "bar",
    "histogram",
    "distribution",
    "parity",
    "residuals",
];

const PLOT_COMMAND_STYLE_FUNCTIONS: &[&str] = &[
    "line",
    "bar",
    "histogram",
    "distribution",
    "parity",
    "residuals",
];

const WORKFLOW_STATUS_LITERAL_KEYWORDS: &[&str] = &[
    "pending",
    "planned",
    "partial",
    "running",
    "passed",
    "failed",
    "succeeded",
    "skipped",
    "blocked",
    "completed",
    "rendered",
    "collected",
    "missing",
    "empty",
];
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
    "interval",
    "ensemble",
    "monte_carlo",
    "source_linear_terms",
    "finite_difference",
    "asc",
    "desc",
    "pending",
    "planned",
    "partial",
    "running",
    "passed",
    "failed",
    "succeeded",
    "skipped",
    "blocked",
    "completed",
    "rendered",
    "collected",
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

const EDITOR_CONSTANT_EXTRA_KEYWORDS: &[&str] = &[
    "lhs",
    "latin_hypercube",
    "latin-hypercube",
    "grid",
    "random",
    "uniform",
];

const EDITOR_OPERATOR_WORD_KEYWORDS: &[&str] = &[
    "eq", "is", "and", "or", "not", "between", "over", "by", "using", "in", "into", "of", "vs",
    "to", "within", "matches",
];

const EDITOR_LEGACY_UNIT_ALIASES: &[&str] = &[
    "B",
    "byte",
    "bytes",
    "KB",
    "kilobyte",
    "kilobytes",
    "MB",
    "megabyte",
    "megabytes",
    "GB",
    "gigabyte",
    "gigabytes",
    "KiB",
    "kibibyte",
    "kibibytes",
    "MiB",
    "mebibyte",
    "mebibytes",
    "GiB",
    "gibibyte",
    "gibibytes",
    "m2",
    "m3",
    "kJ",
    "%",
];

const EDITOR_IMPORT_KEYWORDS: &[&str] = &["use", "import", "from", "as"];
const EDITOR_DEPRECATED_KEYWORDS: &[&str] = &["script", "struct"];
const EDITOR_DECLARATION_KEYWORDS: &[&str] = &["schema", "class", "system", "domain", "component"];
const EDITOR_FUNCTION_KEYWORDS: &[&str] = &["fn", "method"];
const EDITOR_TEST_KEYWORDS: &[&str] = &["test"];
const EDITOR_BLOCK_KEYWORDS: &[&str] = &["args", "where", "with", "on"];
const EDITOR_MODIFIER_KEYWORDS: &[&str] = &[
    "const",
    "state",
    "input",
    "parameter",
    "output",
    "port",
    "across",
    "through",
    "operator",
    "index",
];
const EDITOR_REPORT_KEYWORDS: &[&str] = &[
    "report",
    "show",
    "plot",
    "line",
    "bar",
    "histogram",
    "summarize",
    "summary",
    "distribution",
];
const EDITOR_VALIDATION_KEYWORDS: &[&str] = &[
    "validate",
    "assert",
    "golden",
    "matches",
    "within",
    "constraints",
    "missing",
    "interpolate",
    "monotonic",
    "fill",
    "align",
    "resample",
    "check",
    "coverage",
];
const EDITOR_SIDE_EFFECT_KEYWORDS: &[&str] = &[
    "write", "export", "copy", "move", "delete", "mkdir", "render", "template", "print", "log",
];
const EDITOR_EXTERNAL_BOUNDARY_KEYWORDS: &[&str] = &[
    "run", "command", "open", "sqlite", "http", "get", "post", "put", "patch", "head", "request",
    "fetch", "download",
];
const EDITOR_SOLVER_KEYWORDS: &[&str] = &[
    "simulate",
    "solve",
    "connect",
    "conservation",
    "equation",
    "operator",
    "states",
    "inputs",
    "outputs",
];
const EDITOR_WORKFLOW_KEYWORDS: &[&str] = &[
    "promote",
    "read",
    "text",
    "json",
    "toml",
    "csv",
    "records",
    "column",
    "columns",
    "to",
    "policy",
    "package",
    "version",
    "model",
    "return",
    "if",
    "else",
    "sample",
    "filter",
    "select",
    "derive",
    "sort",
    "require_one",
    "materialize",
    "cases",
    "collect",
    "results",
    "apply",
    "train",
    "regression",
    "predict",
];

const PUBLIC_TYPE_COMPLETIONS: &[(&str, &str)] = &[
    ("Array[T]", "Schema array value"),
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
    ("List[T]", "Schema list value"),
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

const EDITOR_LEGACY_WORKFLOW_BUILTIN_ALIASES: &[&str] = &["regression_table", "train_regression"];
const EDITOR_LEGACY_WORKFLOW_OPTION_ALIASES: &[&str] =
    &["fixture", "layers", "test_fraction", "x", "y"];

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
    (
        "resample",
        "Record TimeSeries resampling against a target series or explicit step",
    ),
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
    ("target", "model target column"),
    ("template", "template source file"),
    ("timeout", "external command timeout"),
    ("timestep", "solver or simulation time step"),
    ("title", "plot or report title"),
    ("unit x", "plot x-axis display unit option"),
    ("unit y", "plot y-axis display unit option"),
    ("tool_version", "external tool version"),
    ("tolerance", "solver convergence tolerance"),
    ("transaction", "SQLite transaction policy"),
    ("type", "workflow display or command subtype option"),
    ("uncertainty", "uncertainty propagation policy"),
    ("upper", "upper uncertainty or range bound"),
    ("values", "template value map"),
    ("variable_scale", "solver variable scale"),
    ("variable_scales", "solver variable scale list"),
    ("year", "calendar year option"),
];

const HTTP_RESPONSE_FIELD_COMPLETIONS: &[(&str, &str)] = &[
    (
        "body",
        "HTTP response body text from live, cached, or pinned response",
    ),
    ("text", "alias for HTTP response body text"),
    ("method", "HTTP request method"),
    (
        "response_source",
        "response source such as live, cached, or offline_response",
    ),
    ("status_code", "HTTP status code"),
    ("status_class", "HTTP status class"),
    ("response_hash", "response SHA-256 hash"),
    ("query_string", "resolved request query string"),
    (
        "request_url",
        "alias for resolved request URL with query string",
    ),
    ("url", "resolved request URL"),
    ("url_with_query", "resolved request URL with query string"),
];

const DB_CONNECTION_FIELD_COMPLETIONS: &[(&str, &str)] = &[
    (
        "summary",
        "SQLite connection status, table count, row count, and table list",
    ),
    ("tables_written", "written SQLite tables with row counts"),
    ("tables", "alias for written SQLite tables with row counts"),
    ("table_names", "written SQLite table names"),
    ("table_count", "written SQLite table count"),
    ("write_count", "alias for written SQLite table count"),
    ("row_count", "total written SQLite row count"),
    ("rows_written", "alias for total written SQLite row count"),
    ("status", "SQLite connection summary status"),
    ("path", "SQLite database path"),
    ("database", "alias for SQLite database path"),
];
const SAMPLE_TABLE_FIELD_COMPLETIONS: &[(&str, &str)] = &[
    ("sample_count", "generated sample row count"),
    ("method", "sample generation method"),
    ("generation", "sample generation source label"),
    ("seed", "sample generation seed"),
    ("status", "sample table validation status"),
    ("parameter_count", "sample parameter column count"),
    ("row_hash_count", "sample row hash count"),
    ("row_preview", "sample row preview summary"),
    ("source_hash", "sample table source hash"),
    ("case_id_column", "case identifier column"),
];

const CASE_TABLE_FIELD_COMPLETIONS: &[(&str, &str)] = &[
    ("case_count", "case row count"),
    ("pending_count", "pending case count"),
    ("running_count", "running case count"),
    ("succeeded_count", "succeeded case count"),
    ("failed_count", "failed case count"),
    ("skipped_count", "skipped case count"),
    ("status", "case table aggregate status"),
    ("row_count", "table row count"),
    ("column_count", "table column count"),
    ("schema_name", "runtime table schema name"),
    ("source_hash", "runtime table source hash"),
];

const CASE_OUTPUT_TABLE_FIELD_COMPLETIONS: &[(&str, &str)] = &[
    ("case_count", "case output row count"),
    ("expected_count", "expected case output row count"),
    ("rendered_count", "rendered case output count"),
    ("blocked_count", "blocked case output count"),
    ("output_count", "rendered output path count"),
    ("manifest_count", "render manifest path count"),
    ("status", "case output aggregate status"),
    ("row_count", "table row count"),
    ("column_count", "table column count"),
    ("schema_name", "runtime table schema name"),
    ("source_hash", "runtime table source hash"),
];

const CASE_RESULT_COLLECTION_TABLE_FIELD_COMPLETIONS: &[(&str, &str)] = &[
    ("case_count", "case result row count"),
    ("collected_count", "collected case result count"),
    ("missing_count", "missing case result count"),
    ("blocked_count", "blocked case result count"),
    ("output_count", "collected output path count"),
    ("manifest_count", "collected manifest path count"),
    ("status", "case result collection aggregate status"),
    ("row_count", "table row count"),
    ("column_count", "table column count"),
    ("schema_name", "runtime table schema name"),
    ("source_hash", "runtime table source hash"),
];

const MODEL_FIELD_COMPLETIONS: &[(&str, &str)] = &[
    ("status", "model training or prediction readiness status"),
    ("target", "model target column"),
    ("target_quantity", "model target quantity kind"),
    ("target_unit", "model target display unit"),
    ("features", "comma-separated model feature columns"),
    ("feature_count", "model feature column count"),
    ("algorithm", "model training algorithm"),
    ("test_fraction", "model holdout test fraction"),
    ("train_count", "model training row count"),
    ("test_count", "model test or prediction row count"),
    ("rmse", "model root-mean-square error"),
    ("mae", "model mean absolute error"),
    ("r2", "model coefficient of determination"),
    (
        "model_card",
        "model-card text generated for the trained model",
    ),
    ("training_data_hash", "model training data hash"),
    ("model_artifact_hash", "model artifact hash"),
    ("residual_point_count", "model residual point count"),
];

const PREDICTION_TABLE_FIELD_COMPLETIONS: &[(&str, &str)] = &[
    ("case_count", "prediction row count"),
    ("row_count", "prediction table row count"),
    ("column_count", "prediction table column count"),
    ("schema_name", "prediction table schema name"),
    ("source_hash", "prediction table source hash"),
    ("status", "prediction artifact status"),
    ("model", "model binding used for prediction"),
    (
        "prediction_input",
        "input table binding used for prediction",
    ),
    ("target", "predicted target column"),
    ("target_quantity", "predicted target quantity kind"),
    ("target_unit", "predicted target display unit"),
    ("output_column", "prediction output column name"),
    ("confidence_column", "prediction confidence column name"),
    (
        "confidence_count",
        "non-missing prediction confidence count",
    ),
    ("missing_count", "missing prediction output count"),
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
        hovers: hover_items(report, source),
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

    if diagnostic.code == "W-NET-FIXTURE-ALIAS" {
        if let Some(range) = option_key_byte_range(line, "fixture") {
            return Some(range);
        }
    }

    if let Some(option_names) = diagnostic_option_names(diagnostic.code.as_str()) {
        if let Some(range) = option_value_byte_range(line, option_names) {
            return Some(range);
        }
    }

    if let Some(range) = format_interpolation_diagnostic_byte_range(line, diagnostic) {
        return Some(range);
    }

    match diagnostic.code.as_str() {
        "E-IO-JSON-FIELD-ACCESS-001" => {
            if let Some(range) = json_read_field_access_diagnostic_byte_range(line, diagnostic) {
                return Some(range);
            }
        }
        "W-NET-RESPONSE-HASH-ALIAS" => {
            if let Some(range) = member_field_byte_range(line, "hash") {
                return Some(range);
            }
        }
        "W-NET-RESPONSE-STATUS-ALIAS" => {
            if let Some(range) = member_field_byte_range(line, "status") {
                return Some(range);
            }
        }
        "W-STATS-SUM-001" => {
            if let Some(range) = sum_function_name_byte_range(line) {
                return Some(range);
            }
        }
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
        "E-LOG-LEVEL-001" => {
            if let Some(range) = log_level_byte_range(line) {
                return Some(range);
            }
        }
        "E-NET-INVALID-URL" => {
            if let Some(range) = net_url_literal_byte_range(line) {
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

fn format_interpolation_diagnostic_byte_range(
    line: &str,
    diagnostic: &Diagnostic,
) -> Option<(usize, usize)> {
    match diagnostic.code.as_str() {
        "E-PRINT-FMT-003" | "E-WRITE-FMT-003" => last_backtick_payload(&diagnostic.message)
            .and_then(|payload| {
                format_interpolation_payload_byte_range(
                    line,
                    payload,
                    FormatInterpolationPayload::Unit,
                )
            }),
        "E-PRINT-FMT-004" | "E-WRITE-FMT-004" => first_backtick_payload(&diagnostic.message)
            .and_then(|payload| {
                format_interpolation_payload_byte_range(
                    line,
                    payload,
                    FormatInterpolationPayload::Expression,
                )
            }),
        "E-PRINT-FMT-002" | "E-WRITE-FMT-002" => empty_format_interpolation_byte_range(line),
        "E-PRINT-FMT-001" | "E-WRITE-FMT-001" => unterminated_format_interpolation_byte_range(line),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum FormatInterpolationPayload {
    Expression,
    Unit,
}

fn format_interpolation_payload_byte_range(
    line: &str,
    payload: &str,
    payload_kind: FormatInterpolationPayload,
) -> Option<(usize, usize)> {
    for (literal_start, literal_end) in string_literal_byte_ranges(line) {
        let content_start = literal_start + '"'.len_utf8();
        let content_end = literal_end.saturating_sub('"'.len_utf8());
        let content = line.get(content_start..content_end)?;
        let mut cursor = 0usize;
        while let Some(open_offset) = content[cursor..].find('{') {
            let open = cursor + open_offset;
            let field_start = open + '{'.len_utf8();
            let Some(close_offset) = content[field_start..].find('}') else {
                break;
            };
            let close = field_start + close_offset;
            let field = &content[field_start..close];
            let field_line_offset = content_start + field_start;
            let range = match payload_kind {
                FormatInterpolationPayload::Expression => {
                    format_expression_range_in_field(field, field_line_offset, payload)
                }
                FormatInterpolationPayload::Unit => {
                    format_unit_range_in_field(field, field_line_offset, payload)
                }
            };
            if let Some(range) = range {
                return Some(range);
            }
            cursor = close + '}'.len_utf8();
        }
    }
    None
}

fn format_expression_range_in_field(
    field: &str,
    field_line_offset: usize,
    payload: &str,
) -> Option<(usize, usize)> {
    let expression_end = field.find(':').unwrap_or(field.len());
    let expression = &field[..expression_end];
    let (start, end, text) = trimmed_range(expression, field_line_offset)?;
    (text == payload).then_some((start, end))
}

fn format_unit_range_in_field(
    field: &str,
    field_line_offset: usize,
    payload: &str,
) -> Option<(usize, usize)> {
    let colon = field.find(':')?;
    let spec_start = colon + ':'.len_utf8();
    let spec = &field[spec_start..];
    let mut cursor = leading_whitespace_len(spec);
    let after_leading = &spec[cursor..];
    if let Some(after_dot) = after_leading.strip_prefix('.') {
        cursor += '.'.len_utf8();
        cursor += after_dot
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .map(char::len_utf8)
            .sum::<usize>();
    }
    let unit = &spec[cursor..];
    let (start, end, text) = trimmed_range(unit, field_line_offset + spec_start + cursor)?;
    (text == payload).then_some((start, end))
}

fn empty_format_interpolation_byte_range(line: &str) -> Option<(usize, usize)> {
    for (literal_start, literal_end) in string_literal_byte_ranges(line) {
        let content_start = literal_start + '"'.len_utf8();
        let content_end = literal_end.saturating_sub('"'.len_utf8());
        let content = line.get(content_start..content_end)?;
        let mut cursor = 0usize;
        while let Some(open_offset) = content[cursor..].find('{') {
            let open = cursor + open_offset;
            let field_start = open + '{'.len_utf8();
            let Some(close_offset) = content[field_start..].find('}') else {
                break;
            };
            let close = field_start + close_offset;
            if content[field_start..close].trim().is_empty() {
                let start = content_start + open;
                return Some((start, content_start + close + '}'.len_utf8()));
            }
            cursor = close + '}'.len_utf8();
        }
    }
    None
}

fn unterminated_format_interpolation_byte_range(line: &str) -> Option<(usize, usize)> {
    for (literal_start, literal_end) in string_literal_byte_ranges(line) {
        let content_start = literal_start + '"'.len_utf8();
        let content_end = literal_end.saturating_sub('"'.len_utf8());
        let content = line.get(content_start..content_end)?;
        let mut cursor = 0usize;
        while let Some(open_offset) = content[cursor..].find('{') {
            let open = cursor + open_offset;
            let field_start = open + '{'.len_utf8();
            let Some(close_offset) = content[field_start..].find('}') else {
                let start = content_start + open;
                return Some((start, start + '{'.len_utf8()));
            };
            cursor = field_start + close_offset + '}'.len_utf8();
        }
    }
    None
}

fn string_literal_byte_ranges(line: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut cursor = 0usize;
    while cursor < line.len() {
        let Some(relative_quote) = line[cursor..].find('"') else {
            break;
        };
        let quote = cursor + relative_quote;
        let Some((start, end)) = string_literal_byte_range_at(line, quote) else {
            break;
        };
        ranges.push((start, end));
        cursor = end;
    }
    ranges
}

fn trimmed_range<'a>(value: &'a str, line_offset: usize) -> Option<(usize, usize, &'a str)> {
    let leading = leading_whitespace_len(value);
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let start = line_offset + leading;
    Some((start, start + trimmed.len(), trimmed))
}

fn leading_whitespace_len(value: &str) -> usize {
    value
        .chars()
        .take_while(|character| character.is_whitespace())
        .map(char::len_utf8)
        .sum()
}

fn first_backtick_payload(text: &str) -> Option<&str> {
    let open = text.find('`')?;
    let payload_start = open + '`'.len_utf8();
    let after_open = &text[payload_start..];
    let close = after_open.find('`')?;
    Some(&after_open[..close])
}

fn last_backtick_payload(text: &str) -> Option<&str> {
    let mut rest = text;
    let mut last = None;
    loop {
        let Some(open) = rest.find('`') else {
            return last;
        };
        let payload_start = open + '`'.len_utf8();
        let after_open = &rest[payload_start..];
        let Some(close) = after_open.find('`') else {
            return last;
        };
        last = Some(&after_open[..close]);
        rest = &after_open[close + '`'.len_utf8()..];
    }
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

fn json_read_field_access_diagnostic_byte_range(
    line: &str,
    diagnostic: &Diagnostic,
) -> Option<(usize, usize)> {
    let mut rest = diagnostic.message.as_str();
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
        if let Some((binding, field)) = payload.split_once('.') {
            let binding = binding.trim();
            let field = field.trim();
            if is_identifier(binding) && is_identifier(field) {
                let access = format!("{binding}.{field}");
                if let Some(range) = find_member_access_byte_range(line, &access) {
                    return Some(range);
                }
            }
        }
        rest = &after_open[close + '`'.len_utf8()..];
    }
}

fn find_member_access_byte_range(line: &str, access: &str) -> Option<(usize, usize)> {
    let code_end = comment_start(line).unwrap_or(line.len());
    let code = &line[..code_end];
    let mut search_start = 0usize;
    while search_start < code.len() {
        let Some(relative_start) = code[search_start..].find(access) else {
            break;
        };
        let start = search_start + relative_start;
        let end = start + access.len();
        if is_identifier_boundary(code, start, end) {
            return Some((start, end));
        }
        search_start = end;
    }
    None
}

fn member_field_byte_range(line: &str, field: &str) -> Option<(usize, usize)> {
    let code_end = comment_start(line).unwrap_or(line.len());
    let code = &line[..code_end];
    let needle = format!(".{field}");
    let mut search_start = 0usize;
    while search_start < code.len() {
        let Some(relative_start) = code[search_start..].find(&needle) else {
            break;
        };
        let dot_start = search_start + relative_start;
        let field_start = dot_start + '.'.len_utf8();
        let field_end = field_start + field.len();
        let bytes = code.as_bytes();
        if dot_start > 0
            && is_ident_byte(bytes[dot_start - 1])
            && (field_end >= bytes.len() || !is_ident_byte(bytes[field_end]))
        {
            return Some((field_start, field_end));
        }
        search_start = field_end;
    }
    None
}

fn net_url_literal_byte_range(line: &str) -> Option<(usize, usize)> {
    let code_end = comment_start(line).unwrap_or(line.len());
    let code = &line[..code_end];
    call_string_argument_byte_range(code, "url").or_else(|| first_string_literal_byte_range(code))
}

fn call_string_argument_byte_range(line: &str, function_name: &str) -> Option<(usize, usize)> {
    let mut search_start = 0usize;
    while search_start < line.len() {
        let Some(relative_start) = line[search_start..].find(function_name) else {
            break;
        };
        let start = search_start + relative_start;
        let after_name = start + function_name.len();
        if is_identifier_boundary(line, start, after_name) {
            let mut cursor = after_name;
            while cursor < line.len() && line.as_bytes()[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if line.as_bytes().get(cursor) == Some(&b'(') {
                cursor += 1;
                while cursor < line.len() && line.as_bytes()[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if let Some(range) = string_literal_byte_range_at(line, cursor) {
                    return Some(range);
                }
            }
        }
        search_start = after_name;
    }
    None
}

fn first_string_literal_byte_range(line: &str) -> Option<(usize, usize)> {
    let quote = line.find('"')?;
    string_literal_byte_range_at(line, quote)
}

fn string_literal_byte_range_at(line: &str, quote_start: usize) -> Option<(usize, usize)> {
    if line.as_bytes().get(quote_start) != Some(&b'"') {
        return None;
    }
    let mut escaped = false;
    for (relative_index, character) in line[quote_start + 1..].char_indices() {
        let index = quote_start + 1 + relative_index;
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' => return Some((quote_start, index + '"'.len_utf8())),
            _ => {}
        }
    }
    None
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

fn sum_function_name_byte_range(line: &str) -> Option<(usize, usize)> {
    let code_end = comment_start(line).unwrap_or(line.len());
    let code = &line[..code_end];
    let mut search_start = 0usize;
    while search_start < code.len() {
        let Some(relative_start) = code[search_start..].find("sum") else {
            break;
        };
        let start = search_start + relative_start;
        let after_name = start + "sum".len();
        if is_identifier_boundary(code, start, after_name) {
            let whitespace = code[after_name..]
                .chars()
                .take_while(|character| character.is_whitespace())
                .map(char::len_utf8)
                .sum::<usize>();
            if code.as_bytes().get(after_name + whitespace) == Some(&b'(') {
                return Some((start, after_name));
            }
        }
        search_start = after_name;
    }
    None
}

fn diagnostic_option_names(code: &str) -> Option<&'static [&'static str]> {
    match code {
        "E-NET-RETRY-POLICY" | "E-PROCESS-RETRY-POLICY" => Some(&["retry"]),
        "E-NET-TIMEOUT" | "E-PROCESS-TIMEOUT" => Some(&["timeout"]),
        "E-NET-BODY-SIZE-LIMIT" => Some(&["body_size_limit", "response_body_limit"]),
        "E-NET-BODY-METHOD" | "E-NET-BODY-POLICY" => Some(&["body"]),
        "E-PROCESS-ALLOW-FAILURE" => Some(&["allow_failure"]),
        "E-PROCESS-CWD-001" => Some(&["cwd"]),
        "E-PROCESS-ENV-001" => Some(&["env"]),
        "E-SAMPLING-COUNT-INVALID" => Some(&["count"]),
        "E-SAMPLING-SEED-INVALID" => Some(&["seed"]),
        "E-ML-ARGS-001" => Some(&[
            "target",
            "y",
            "features",
            "x",
            "test",
            "test_fraction",
            "hidden",
            "layers",
            "epochs",
        ]),
        "E-ML-ARGS-002" => Some(&[
            "test",
            "test_fraction",
            "seed",
            "hidden",
            "layers",
            "epochs",
        ]),
        "E-ML-ARGS-003" => Some(&["algorithm"]),
        "E-CACHE-KEY-NONDETERMINISTIC" => Some(&["cache_key"]),
        "E-CACHE-DIR" => Some(&["cache_dir"]),
        "E-CACHE-TTL" => Some(&["cache_ttl"]),
        "E-SIM-TIMESTEP-INVALID" | "E-SOLVE-TIMESTEP-INVALID" => Some(&["timestep"]),
        "E-SIM-DURATION-INVALID" | "E-SOLVE-DURATION-INVALID" => Some(&["duration"]),
        "E-SIM-TOLERANCE-INVALID" | "E-SOLVE-TOLERANCE-INVALID" => Some(&["tolerance"]),
        "E-SIM-SOLVER-UNSUPPORTED" | "E-SOLVE-SOLVER-UNSUPPORTED" => Some(&["solver"]),
        "E-SOLVE-RELAXATION-INVALID" => Some(&["relaxation"]),
        "E-SOLVE-FD-STEP-INVALID" => Some(&["finite_difference_step"]),
        "E-SOLVE-DAMPING-INVALID" => Some(&["damping"]),
        "E-SOLVE-CONSISTENCY-TOLERANCE-INVALID" => Some(&["consistency_tolerance"]),
        "E-SOLVE-MAX-ITER-INVALID" => Some(&["max_iter"]),
        "E-SOLVE-LINE-SEARCH-STEPS-INVALID" => Some(&["line_search_steps"]),
        "E-SOLVE-INITIAL-INVALID" => Some(&["initial", "initial_derivative", "initial_algebraic"]),
        "E-SOLVE-VARIABLE-SCALE-INVALID" => Some(&["variable_scale", "variable_scales"]),
        "E-SOLVE-MASS-MATRIX-INVALID" => Some(&["mass_matrix"]),
        "E-SOLVE-JACOBIAN-UNSUPPORTED" => Some(&["jacobian"]),
        "E-SOLVE-ALGEBRAIC-INITIALIZATION-UNSUPPORTED" => Some(&["algebraic_initialization"]),
        _ => None,
    }
}

fn option_key_byte_range(line: &str, option_name: &str) -> Option<(usize, usize)> {
    let indent_len = line_indent_len(line);
    let rest = &line[indent_len..];
    let after_name = rest.strip_prefix(option_name)?;
    if !after_name
        .chars()
        .next()
        .is_some_and(|character| character.is_whitespace() || character == '=')
    {
        return None;
    }
    let equals_offset = after_name.find('=')?;
    if !after_name[..equals_offset].trim().is_empty() {
        return None;
    }
    Some((indent_len, indent_len + option_name.len()))
}

fn option_value_byte_range(line: &str, option_names: &[&str]) -> Option<(usize, usize)> {
    let indent_len = line_indent_len(line);
    let rest = &line[indent_len..];
    for option_name in option_names {
        let Some(after_name) = rest.strip_prefix(option_name) else {
            continue;
        };
        if !after_name
            .chars()
            .next()
            .is_some_and(|character| character.is_whitespace() || character == '=')
        {
            continue;
        }
        let equals_offset = after_name.find('=')?;
        if !after_name[..equals_offset].trim().is_empty() {
            continue;
        }
        let raw_value_start = indent_len + option_name.len() + equals_offset + 1;
        let value_start = raw_value_start
            + line[raw_value_start..]
                .chars()
                .take_while(|character| character.is_whitespace())
                .map(char::len_utf8)
                .sum::<usize>();
        let comment_start = line[value_start..]
            .find('#')
            .map(|offset| value_start + offset)
            .unwrap_or(line.len());
        let value_end = value_start + line[value_start..comment_start].trim_end().len();
        if value_end > value_start {
            return Some((value_start, value_end));
        }
    }
    None
}

fn log_level_byte_range(line: &str) -> Option<(usize, usize)> {
    let start = line_indent_len(line);
    let rest = &line[start..];
    let after_log = rest.strip_prefix("log")?;
    if !after_log.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }
    let after_log_start = start + "log".len();
    let level_start = after_log_start
        + line[after_log_start..]
            .chars()
            .take_while(|character| character.is_whitespace())
            .map(char::len_utf8)
            .sum::<usize>();
    let first = line[level_start..].chars().next()?;
    if first == '"' || first == '#' {
        return Some((start, after_log_start));
    }
    let level_end = level_start
        + line[level_start..]
            .chars()
            .take_while(|character| !character.is_whitespace() && *character != '#')
            .map(char::len_utf8)
            .sum::<usize>();
    (level_end > level_start).then_some((level_start, level_end))
}

fn line_indent_len(line: &str) -> usize {
    line.char_indices()
        .find_map(|(index, character)| (!character.is_whitespace()).then_some(index))
        .unwrap_or(line.len())
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
    let completions = editor_completion_items();
    let completion_values = completions
        .iter()
        .map(editor_completion_json)
        .collect::<Vec<_>>();
    json!({
        "format": LSP_EDITOR_METADATA_FORMAT,
        "semantic_token_legend": semantic_legend_json(&semantic_legend()),
        "syntax_catalog": editor_syntax_catalog_json(),
        "completion_items_count": completions.len(),
        "completion_items": completion_values,
    })
}

pub fn editor_syntax_catalog_json() -> Value {
    let constants = editor_constant_keywords();
    json!({
        "keywords": COMPLETION_KEYWORDS,
        "constants": constants,
        "workflow_status_literals": WORKFLOW_STATUS_LITERAL_KEYWORDS,
        "operator_words": EDITOR_OPERATOR_WORD_KEYWORDS,
        "legacy_unit_aliases": EDITOR_LEGACY_UNIT_ALIASES,
        "keyword_groups": {
            "import": EDITOR_IMPORT_KEYWORDS,
            "deprecated": EDITOR_DEPRECATED_KEYWORDS,
            "declaration": EDITOR_DECLARATION_KEYWORDS,
            "function": EDITOR_FUNCTION_KEYWORDS,
            "test": EDITOR_TEST_KEYWORDS,
            "block": EDITOR_BLOCK_KEYWORDS,
            "modifier": EDITOR_MODIFIER_KEYWORDS,
            "report": EDITOR_REPORT_KEYWORDS,
            "validation": EDITOR_VALIDATION_KEYWORDS,
            "side_effect": EDITOR_SIDE_EFFECT_KEYWORDS,
            "external_boundary": EDITOR_EXTERNAL_BOUNDARY_KEYWORDS,
            "solver": EDITOR_SOLVER_KEYWORDS,
            "workflow": EDITOR_WORKFLOW_KEYWORDS,
        },
        "workflow_builtins": WORKFLOW_BUILTIN_KEYWORDS,
        "hyphenated_workflow_builtins": HYPHENATED_WORKFLOW_BUILTIN_KEYWORDS,
        "legacy_workflow_builtin_aliases": EDITOR_LEGACY_WORKFLOW_BUILTIN_ALIASES,
        "legacy_workflow_option_aliases": EDITOR_LEGACY_WORKFLOW_OPTION_ALIASES,
        "workflow_options": WORKFLOW_OPTION_COMPLETIONS
            .iter()
            .map(|(label, detail)| json!({
                "label": label,
                "detail": detail,
            }))
            .collect::<Vec<_>>(),
        "http_response_fields": HTTP_RESPONSE_FIELD_COMPLETIONS
            .iter()
            .map(|(label, detail)| json!({
                "label": label,
                "detail": detail,
            }))
            .collect::<Vec<_>>(),
        "sample_table_fields": SAMPLE_TABLE_FIELD_COMPLETIONS
            .iter()
            .map(|(label, detail)| json!({
                "label": label,
                "detail": detail,
            }))
            .collect::<Vec<_>>(),
        "db_connection_fields": DB_CONNECTION_FIELD_COMPLETIONS
            .iter()
            .map(|(label, detail)| json!({
                "label": label,
                "detail": detail,
            }))
            .collect::<Vec<_>>(),
        "case_table_fields": CASE_TABLE_FIELD_COMPLETIONS
            .iter()
            .map(|(label, detail)| json!({
                "label": label,
                "detail": detail,
            }))
            .collect::<Vec<_>>(),
        "case_output_table_fields": CASE_OUTPUT_TABLE_FIELD_COMPLETIONS
            .iter()
            .map(|(label, detail)| json!({
                "label": label,
                "detail": detail,
            }))
            .collect::<Vec<_>>(),
        "case_result_collection_table_fields": CASE_RESULT_COLLECTION_TABLE_FIELD_COMPLETIONS
            .iter()
            .map(|(label, detail)| json!({
                "label": label,
                "detail": detail,
            }))
            .collect::<Vec<_>>(),
        "model_fields": MODEL_FIELD_COMPLETIONS
            .iter()
            .map(|(label, detail)| json!({
                "label": label,
                "detail": detail,
            }))
            .collect::<Vec<_>>(),
        "prediction_table_fields": PREDICTION_TABLE_FIELD_COMPLETIONS
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

fn editor_constant_keywords() -> Vec<&'static str> {
    let mut constants = LANGUAGE_CONSTANT_KEYWORDS.to_vec();
    constants.extend(EDITOR_CONSTANT_EXTRA_KEYWORDS.iter().copied());
    constants
}

pub fn editor_completion_items() -> Vec<LspCompletion> {
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

    for stats in &program.stats_infos {
        builder.push_keywords_on_line(stats.line, &["summarize", "by"], &["report"]);
        builder.push_identifier_path_on_line(stats.line, &stats.source, &["report", "timeseries"]);
        for statistic in &stats.statistics {
            for name in summary_statistic_names(statistic) {
                builder.push_on_line(
                    stats.line,
                    name,
                    "function",
                    &["defaultLibrary", "report", "timeseries"],
                );
            }
        }
    }

    for integration in &program.integrations {
        builder.push_keywords_on_line(integration.line, &["integrate", "over"], &["solver"]);
        builder.push_on_line(
            integration.line,
            &integration.source,
            "variable",
            &["solver"],
        );
        builder.push_on_line(integration.line, &integration.over_axis, "type", &["axis"]);
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
        builder.push_keywords_on_line(
            promotion.line,
            promotion_source_format_keywords(&promotion.source_format),
            &["workflowStep"],
        );
        add_promotion_source_semantic_tokens(
            &mut builder,
            promotion.line,
            &promotion.source_literal,
        );
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
        builder.push_keywords_on_line(
            promotion.line,
            config_promotion_format_keywords(&promotion.format),
            &["workflowStep"],
        );
        add_promotion_source_semantic_tokens(
            &mut builder,
            promotion.line,
            &promotion.source_literal,
        );
    }

    for sample in &program.sample_generations {
        builder.push_on_line(
            sample.line,
            &sample.binding,
            "variable",
            &["declaration", "workflowStep"],
        );
        builder.push_member_fields(
            &sample.binding,
            SAMPLE_TABLE_FIELD_COMPLETIONS,
            &["workflowStep"],
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

    for binding in &program.typed_bindings {
        match binding.semantic_type.quantity_kind.as_str() {
            "Table[Case]" => builder.push_member_fields(
                &binding.name,
                CASE_TABLE_FIELD_COMPLETIONS,
                &["workflowStep"],
            ),
            "Table[CaseOutput]" => builder.push_member_fields(
                &binding.name,
                CASE_OUTPUT_TABLE_FIELD_COMPLETIONS,
                &["workflowStep"],
            ),
            "Table[CaseResultCollection]" => builder.push_member_fields(
                &binding.name,
                CASE_RESULT_COLLECTION_TABLE_FIELD_COMPLETIONS,
                &["workflowStep"],
            ),
            "Table[Prediction]" => builder.push_member_fields(
                &binding.name,
                PREDICTION_TABLE_FIELD_COMPLETIONS,
                &["model", "workflowStep"],
            ),
            value if value.starts_with("Model[") => builder.push_member_fields(
                &binding.name,
                MODEL_FIELD_COMPLETIONS,
                &["model", "workflowStep"],
            ),
            "DbConnection" => builder.push_member_fields(
                &binding.name,
                DB_CONNECTION_FIELD_COMPLETIONS,
                &["db", "workflowStep"],
            ),
            _ => {}
        }
    }

    for transform in &program.table_transforms {
        builder.push_on_line(
            transform.line,
            &transform.binding,
            "variable",
            &["declaration", "workflowStep"],
        );
        match transform.operation.as_str() {
            "select" | "derive" => {
                builder.push_keywords_on_line(
                    transform.line,
                    &["column", "columns"],
                    &["workflowStep"],
                );
            }
            "sort" => {
                builder.push_keywords_on_line(transform.line, &["by"], &["workflowStep"]);
            }
            "join" => {
                builder.push_keywords_on_line(transform.line, &["with"], &["workflowStep"]);
            }
            _ => {}
        }
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
        add_http_request_semantic_tokens(&mut builder, request.line, &request.method);
    }

    for download in &program.net_downloads {
        add_download_semantic_tokens(&mut builder, download.line);
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
            match variable.role.as_str() {
                "state" => builder.push_on_line(
                    variable.line,
                    &variable.name,
                    "variable",
                    &["declaration", "state"],
                ),
                "input" => builder.push_on_line(
                    variable.line,
                    &variable.name,
                    "variable",
                    &["declaration", "input"],
                ),
                "parameter" => builder.push_on_line(
                    variable.line,
                    &variable.name,
                    "parameter",
                    &["declaration", "readonly"],
                ),
                "output" => builder.push_on_line(
                    variable.line,
                    &variable.name,
                    "variable",
                    &["declaration", "output"],
                ),
                _ => builder.push_on_line(
                    variable.line,
                    &variable.name,
                    "variable",
                    &["declaration"],
                ),
            }
        }
    }

    for vector in &program.state_space_vectors {
        let modifiers = match vector.role.as_str() {
            "states" => ["declaration", "state"].as_slice(),
            "inputs" => ["declaration", "input"].as_slice(),
            "outputs" => ["declaration", "output"].as_slice(),
            _ => ["declaration"].as_slice(),
        };
        builder.push_on_line(vector.line, &vector.name, "variable", modifiers);
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
        builder.push_keywords_on_line(print.line, &["print", "log"], &["sideEffect"]);
    }

    for export in &program.csv_exports {
        builder.push_on_line(export.line, &export.source, "variable", &["report"]);
        add_csv_export_target_semantic_tokens(&mut builder, export.line, &export.path);
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
        add_write_target_semantic_tokens(&mut builder, write.line, &write.format, &write.path);
    }

    for declaration in &report.inferred_declarations {
        if add_open_sqlite_semantic_tokens(
            &mut builder,
            declaration.line,
            &declaration.name,
            &declaration.expression,
        ) {
            continue;
        }
        if add_read_only_io_semantic_tokens(
            &mut builder,
            declaration.line,
            &declaration.name,
            &declaration.expression,
        ) {
            continue;
        }
        let Some(read) = db_read_expression(&declaration.expression) else {
            continue;
        };
        let modifiers = &["db", "external"];
        builder.push_on_line(
            declaration.line,
            &declaration.name,
            "variable",
            &["declaration", "db", "external"],
        );
        builder.push_keywords_on_line(declaration.line, &["read", "sqlite", "as"], modifiers);
        builder.push_on_line(declaration.line, &read.connection, "variable", modifiers);
        builder.push_on_line(declaration.line, "table", "method", modifiers);
        builder.push_on_line(declaration.line, &read.schema_name, "class", &[]);
    }

    for operation in &program.file_operations {
        add_file_operation_semantic_tokens(&mut builder, operation);
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
        add_command_style_semantic_tokens(&mut builder, command);
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
        builder.push_keywords_on_line(
            block.line,
            &["with"],
            with_block_semantic_modifiers(program, block),
        );
        for option in &block.options {
            let modifiers = with_option_semantic_modifiers(program, block, &option.key);
            builder.push_on_line(option.line, &option.key, "property", modifiers);
            add_with_option_value_semantic_token(&mut builder, program, block, option);
        }
    }

    add_review_risk_semantic_tokens(report, &mut builder);

    builder.finish()
}

fn with_block_semantic_modifiers(
    program: &SemanticProgram,
    block: &WithBlockInfo,
) -> &'static [&'static str] {
    if is_model_with_block(program, block.owner_line) {
        return &["model"];
    }
    if is_coverage_with_block(program, block.owner_line) {
        return &["validation", "workflowStep"];
    }
    if is_sample_with_block(program, block.owner_line) {
        return &["workflowStep"];
    }
    if is_template_workflow_with_block(program, block.owner_line) {
        return &["sideEffect", "workflowStep"];
    }
    if let Some(modifiers) = write_with_block_semantic_modifiers(program, block.owner_line) {
        return modifiers;
    }
    if is_net_with_block(program, block.owner_line) {
        return &["external"];
    }
    if is_process_with_block(program, block.owner_line) {
        return &["sideEffect", "external"];
    }
    if is_report_with_block(program, block.owner_line) {
        return &["report"];
    }
    if is_solver_with_block(program, block.owner_line) {
        return &["solver"];
    }
    if is_workflow_step_with_block(program, block.owner_line) {
        return &["workflowStep"];
    }

    with_block_option_semantic_modifiers(program, block)
}

fn with_block_option_semantic_modifiers(
    program: &SemanticProgram,
    block: &WithBlockInfo,
) -> &'static [&'static str] {
    let mut has_cache = false;
    let mut has_uncertain = false;
    for option in &block.options {
        let modifiers = with_option_semantic_modifiers(program, block, &option.key);
        if modifiers.iter().any(|modifier| *modifier == "db") {
            return &["db"];
        }
        if modifiers.iter().any(|modifier| *modifier == "external") {
            return &["external"];
        }
        if modifiers.iter().any(|modifier| *modifier == "sideEffect") {
            return &["sideEffect"];
        }
        if modifiers.iter().any(|modifier| *modifier == "solver") {
            return &["solver"];
        }
        if modifiers.iter().any(|modifier| *modifier == "report") {
            return &["report"];
        }
        if modifiers.iter().any(|modifier| *modifier == "validation") {
            return &["validation"];
        }
        if modifiers.iter().any(|modifier| *modifier == "workflowStep") {
            return &["workflowStep"];
        }
        if modifiers.iter().any(|modifier| *modifier == "model") {
            return &["model"];
        }
        if modifiers.iter().any(|modifier| *modifier == "cache") {
            has_cache = true;
        }
        if modifiers.iter().any(|modifier| *modifier == "uncertain") {
            has_uncertain = true;
        }
    }
    if has_cache {
        return &["cache"];
    }
    if has_uncertain {
        return &["uncertain"];
    }
    &[]
}

fn with_option_semantic_modifiers(
    program: &SemanticProgram,
    block: &WithBlockInfo,
    key: &str,
) -> &'static [&'static str] {
    if key == "display_unit" || key.starts_with("unit ") {
        return &["report"];
    }
    if is_process_with_block(program, block.owner_line) {
        match key {
            "args" | "cwd" | "env" | "expected_outputs" | "tool_version" | "timeout" | "retry"
            | "allow_failure" => return &["sideEffect", "external"],
            "cache" | "cache_key" | "cache_dir" | "cache_ttl" => {
                return &["cache", "sideEffect", "external"]
            }
            _ => {}
        }
    }
    if is_net_with_block(program, block.owner_line) {
        match key {
            "cache" | "cache_key" | "cache_dir" | "cache_ttl" => return &["cache", "external"],
            _ => {}
        }
    }
    match key {
        "cache" | "cache_key" | "cache_dir" | "cache_ttl" => &["cache"],
        "key" | "transaction" => &["db"],
        "expected_step" | "step" | "year" | "start" | "end" | "max_gap" | "missing" | "status"
            if is_coverage_with_block(program, block.owner_line) =>
        {
            &["validation", "workflowStep"]
        }
        "status" if is_workflow_status_option_block(program, block) => &["workflowStep"],
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
    add_with_option_path_helper_semantic_tokens(builder, program, block, option);
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

fn add_with_option_path_helper_semantic_tokens(
    builder: &mut SemanticTokenBuilder<'_>,
    program: &SemanticProgram,
    block: &WithBlockInfo,
    option: &WithOptionInfo,
) {
    let Some(modifiers) = with_option_path_helper_semantic_modifiers(program, block, &option.key)
    else {
        return;
    };
    let Some(line_index) = option.line.checked_sub(1) else {
        return;
    };
    builder.push_identifiers_on_line(line_index, &["file", "dir", "join"], "function", modifiers);
}

fn with_option_path_helper_semantic_modifiers(
    program: &SemanticProgram,
    block: &WithBlockInfo,
    key: &str,
) -> Option<&'static [&'static str]> {
    match key {
        "cache_dir" if is_process_with_block(program, block.owner_line) => {
            Some(&["cache", "sideEffect", "external"])
        }
        "cache_dir" if is_net_with_block(program, block.owner_line) => Some(&["cache", "external"]),
        "cache_dir" => Some(&["cache"]),
        "expected_outputs" => Some(&["sideEffect", "external"]),
        "offline_response" | "fixture" if is_net_with_block(program, block.owner_line) => {
            Some(&["external"])
        }
        "cwd" if is_net_with_block(program, block.owner_line) => Some(&["external"]),
        "output_root" => Some(&["sideEffect", "workflowStep"]),
        "template" if is_template_workflow_with_block(program, block.owner_line) => {
            Some(&["workflowStep"])
        }
        "output" if is_template_workflow_with_block(program, block.owner_line) => {
            Some(&["sideEffect", "workflowStep"])
        }
        "output" => Some(&["sideEffect"]),
        _ => None,
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
        "cache" if matches!(value, "true" | "false") => Some((
            "keyword",
            with_option_semantic_modifiers(program, block, key),
        )),
        "overwrite" | "confirm" | "recursive" | "allow_failure"
            if matches!(value, "true" | "false") =>
        {
            Some((
                "keyword",
                with_option_semantic_modifiers(program, block, key),
            ))
        }
        "resume" if matches!(value, "true" | "false") => Some(("keyword", &["workflowStep"])),
        "status"
            if is_workflow_status_option_block(program, block)
                && is_workflow_status_literal(value) =>
        {
            Some(("keyword", &["workflowStep"]))
        }
        "on_none" | "on_many" | "missing" | "status" if is_status_or_policy_literal(value) => {
            Some(("keyword", &["validation"]))
        }
        "uncertainty" if matches!(value, "linear" | "interval" | "monte_carlo" | "ensemble") => {
            Some(("keyword", &["uncertain"]))
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
                    | "source_linear_terms"
                    | "finite_difference"
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
        "template" if is_template_workflow_with_block(program, block.owner_line) => {
            if matches!(value, "file" | "dir" | "join") {
                Some(("function", &["defaultLibrary", "workflowStep"]))
            } else {
                Some(("variable", &["workflowStep"]))
            }
        }
        "output" if is_template_workflow_with_block(program, block.owner_line) => {
            if matches!(value, "file" | "dir" | "join") {
                Some((
                    "function",
                    &["defaultLibrary", "sideEffect", "workflowStep"],
                ))
            } else {
                Some(("variable", &["sideEffect", "workflowStep"]))
            }
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

fn is_template_workflow_with_block(program: &SemanticProgram, owner_line: Option<usize>) -> bool {
    let Some(owner_line) = owner_line else {
        return false;
    };
    program.command_styles.iter().any(|command| {
        command.line == owner_line
            && (command.verb == "apply"
                || (command.verb == "render" && command.target.trim().starts_with("template ")))
    })
}

fn is_coverage_with_block(program: &SemanticProgram, owner_line: Option<usize>) -> bool {
    let Some(owner_line) = owner_line else {
        return false;
    };
    program.command_styles.iter().any(|command| {
        command.line == owner_line
            && command.verb == "check"
            && command.target.trim().starts_with("coverage ")
    })
}

fn write_with_block_semantic_modifiers(
    program: &SemanticProgram,
    owner_line: Option<usize>,
) -> Option<&'static [&'static str]> {
    let owner_line = owner_line?;
    let write = program
        .writes
        .iter()
        .find(|write| write.line == owner_line)?;
    if write.quantity_kind == "DbWrite" {
        Some(&["db"])
    } else if write.format == "standard_text" {
        Some(&["sideEffect", "workflowStep"])
    } else {
        Some(&["sideEffect"])
    }
}

fn is_process_with_block(program: &SemanticProgram, owner_line: Option<usize>) -> bool {
    let Some(owner_line) = owner_line else {
        return false;
    };
    program
        .process_runs
        .iter()
        .any(|process| process.line == owner_line)
}

fn is_report_with_block(program: &SemanticProgram, owner_line: Option<usize>) -> bool {
    let Some(owner_line) = owner_line else {
        return false;
    };
    program
        .csv_exports
        .iter()
        .any(|export| export.line == owner_line)
        || program.command_styles.iter().any(|command| {
            command.line == owner_line
                && matches!(
                    command.verb.as_str(),
                    "plot"
                        | "show"
                        | "report"
                        | "summarize"
                        | "summary"
                        | "mean"
                        | "max"
                        | "min"
                        | "duration"
                )
        })
}

fn is_solver_with_block(program: &SemanticProgram, owner_line: Option<usize>) -> bool {
    let Some(owner_line) = owner_line else {
        return false;
    };
    program.command_styles.iter().any(|command| {
        command.line == owner_line
            && matches!(command.verb.as_str(), "simulate" | "solve" | "integrate")
    })
}

fn is_workflow_step_with_block(program: &SemanticProgram, owner_line: Option<usize>) -> bool {
    let Some(owner_line) = owner_line else {
        return false;
    };
    program.command_styles.iter().any(|command| {
        command.line == owner_line
            && matches!(
                command.verb.as_str(),
                "apply"
                    | "materialize"
                    | "collect"
                    | "filter"
                    | "select"
                    | "sort"
                    | "derive"
                    | "fill"
                    | "align"
                    | "resample"
            )
    })
}

fn is_workflow_status_option_block(program: &SemanticProgram, block: &WithBlockInfo) -> bool {
    if is_workflow_step_with_block(program, block.owner_line) {
        return true;
    }
    block
        .options
        .iter()
        .any(|option| is_workflow_step_option_key(&option.key))
}

fn is_workflow_step_option_key(key: &str) -> bool {
    matches!(
        key,
        "step"
            | "case_id"
            | "output_root"
            | "resume"
            | "template"
            | "values"
            | "artifact_kind"
            | "year"
            | "return_column"
    )
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
    if quantity_kind.contains("LinearOperator") {
        modifiers.push("solver");
    }
    if quantity_kind.contains("Table[Case") || quantity_kind.contains("CaseOutput") {
        modifiers.push("workflowStep");
    }
    modifiers
}

fn comment_semantic_modifiers(line: &str, comment_start: usize) -> &'static [&'static str] {
    if line[comment_start..].starts_with("///") {
        &["documentation"]
    } else {
        &[]
    }
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

fn promotion_source_format_keywords(source_format: &str) -> &'static [&'static str] {
    match source_format {
        "json_records" => &["promote", "json", "records", "as"],
        "json" => &["promote", "json", "as"],
        "toml" => &["promote", "toml", "as"],
        _ => &["promote", "csv", "as"],
    }
}

fn config_promotion_format_keywords(format: &str) -> &'static [&'static str] {
    match format {
        "toml" => &["promote", "toml", "as"],
        _ => &["promote", "json", "as"],
    }
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
        let constant_keywords = editor_constant_keywords()
            .into_iter()
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
            let comment_start_index = comment_start(line);
            if let Some(comment_start) = comment_start_index {
                self.push_byte_range(
                    line_index,
                    comment_start,
                    line.len().saturating_sub(comment_start),
                    "comment",
                    comment_semantic_modifiers(line, comment_start),
                );
            }
            self.scan_string_tokens(line_index, comment_start_index.unwrap_or(line.len()));
            self.scan_string_interpolation_tokens(
                line_index,
                comment_start_index.unwrap_or(line.len()),
                &units,
            );

            for (start, end) in code_ranges(line) {
                let unit_ranges = self.scan_unit_tokens(line_index, start, end, &units);
                self.scan_word_tokens(
                    line_index,
                    start,
                    end,
                    &quantity_names,
                    &public_types,
                    &unit_ranges,
                    &units,
                    &constant_keywords,
                );
                self.scan_workflow_status_condition_tokens(line_index, start, end);
                self.scan_legacy_declaration_names(line_index, start, end);
                self.scan_hyphenated_workflow_builtin_tokens(
                    line_index,
                    start,
                    end,
                    &constant_keywords,
                );
                self.scan_generic_type_tokens(line_index, start, end, &generic_type_bases);
                self.scan_number_tokens(line_index, start, end);
                self.scan_symbol_operator_tokens(line_index, start, end, &unit_ranges);
            }
        }
    }

    fn scan_string_tokens(&mut self, line_index: usize, end: usize) {
        let line = self.lines[line_index];
        let bytes = line.as_bytes();
        let end = end.min(bytes.len());
        let mut index = 0usize;
        while index < end {
            if bytes[index] != b'"' {
                index += 1;
                continue;
            }
            let token_start = index;
            index += 1;
            while index < end {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(end);
                    continue;
                }
                if bytes[index] == b'"' {
                    index += 1;
                    break;
                }
                index += 1;
            }
            self.push_byte_range(line_index, token_start, index - token_start, "string", &[]);
        }
    }

    fn scan_string_interpolation_tokens(&mut self, line_index: usize, end: usize, units: &[&str]) {
        let Some(line) = self.lines.get(line_index).copied() else {
            return;
        };
        for (literal_start, literal_end) in string_literal_byte_ranges(line) {
            if literal_start >= end {
                continue;
            }
            let literal_end = literal_end.min(end);
            if literal_end <= literal_start + '"'.len_utf8() {
                continue;
            }
            let content_start = literal_start + '"'.len_utf8();
            let content_end = literal_end.saturating_sub('"'.len_utf8());
            if content_start > content_end {
                continue;
            }
            let Some(content) = line.get(content_start..content_end) else {
                continue;
            };
            let mut cursor = 0usize;
            while let Some(open_offset) = content[cursor..].find('{') {
                let open = cursor + open_offset;
                let field_start = open + '{'.len_utf8();
                if content[field_start..].starts_with('{') {
                    cursor = field_start + '{'.len_utf8();
                    continue;
                }
                let Some(close_offset) = content[field_start..].find('}') else {
                    break;
                };
                let close = field_start + close_offset;
                let field = &content[field_start..close];
                self.scan_string_interpolation_field_tokens(
                    line_index,
                    field,
                    content_start + field_start,
                    units,
                );
                cursor = close + '}'.len_utf8();
            }
        }
    }

    fn scan_string_interpolation_field_tokens(
        &mut self,
        line_index: usize,
        field: &str,
        field_line_offset: usize,
        units: &[&str],
    ) {
        let expression_end = field.find(':').unwrap_or(field.len());
        self.scan_string_interpolation_expression_tokens(
            line_index,
            &field[..expression_end],
            field_line_offset,
        );
        let Some(colon) = field.find(':') else {
            return;
        };
        let spec_start = colon + ':'.len_utf8();
        let spec = &field[spec_start..];
        let mut cursor = leading_whitespace_len(spec);
        if spec[cursor..].starts_with('.') {
            cursor += '.'.len_utf8();
            let digit_start = cursor;
            cursor += spec[cursor..]
                .chars()
                .take_while(|character| character.is_ascii_digit())
                .map(char::len_utf8)
                .sum::<usize>();
            if cursor > digit_start {
                self.push_byte_range(
                    line_index,
                    field_line_offset + spec_start + digit_start,
                    cursor - digit_start,
                    "number",
                    &[],
                );
            }
        }
        self.scan_unit_tokens(
            line_index,
            field_line_offset + spec_start + cursor,
            field_line_offset + field.len(),
            units,
        );
    }

    fn scan_string_interpolation_expression_tokens(
        &mut self,
        line_index: usize,
        expression: &str,
        expression_line_offset: usize,
    ) {
        let bytes = expression.as_bytes();
        let mut index = 0usize;
        while index < bytes.len() {
            if !is_ident_start(bytes[index]) {
                index += 1;
                continue;
            }
            let mut segments = Vec::<(usize, usize)>::new();
            let mut segment_start = index;
            loop {
                let mut segment_end = segment_start + 1;
                while segment_end < bytes.len() && is_ident_byte(bytes[segment_end]) {
                    segment_end += 1;
                }
                segments.push((segment_start, segment_end));
                if segment_end + 1 < bytes.len()
                    && bytes[segment_end] == b'.'
                    && is_ident_start(bytes[segment_end + 1])
                {
                    segment_start = segment_end + 1;
                    continue;
                }
                index = segment_end;
                break;
            }
            for (segment_index, (start, end)) in segments.iter().copied().enumerate() {
                let token_type = if segment_index == 0 && &expression[start..end] == "args" {
                    "parameter"
                } else if segment_index == 0 {
                    "variable"
                } else {
                    "property"
                };
                self.push_byte_range(
                    line_index,
                    expression_line_offset + start,
                    end - start,
                    token_type,
                    &[],
                );
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
        unit_ranges: &[(usize, usize)],
        units: &[&str],
        constant_keywords: &BTreeSet<&str>,
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
                if unit_ranges
                    .iter()
                    .any(|(left, right)| ranges_overlap(token_start, index, *left, *right))
                    || is_unit_denominator_token(line, token_start, token, units)
                {
                    continue;
                }
                if WORKFLOW_BUILTIN_KEYWORDS.contains(&token) {
                    let (token_type, modifiers) = workflow_builtin_semantic_class(
                        line,
                        token,
                        token_start,
                        index,
                        constant_keywords,
                    );
                    self.push_byte_range(
                        line_index,
                        token_start,
                        index - token_start,
                        token_type,
                        modifiers,
                    );
                } else if COMPLETION_KEYWORDS.contains(&token) {
                    self.push_byte_range(
                        line_index,
                        token_start,
                        index - token_start,
                        "keyword",
                        keyword_modifiers(token),
                    );
                } else if constant_keywords.contains(token) {
                    self.push_byte_range(
                        line_index,
                        token_start,
                        index - token_start,
                        "keyword",
                        language_constant_modifiers_for_line(line, token, token_start),
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

    fn scan_workflow_status_condition_tokens(
        &mut self,
        line_index: usize,
        start: usize,
        end: usize,
    ) {
        let line = self.lines[line_index];
        let bytes = line.as_bytes();
        let mut cursor = skip_ascii_whitespace(bytes, start, end);
        let status_end = cursor.saturating_add("status".len());
        if status_end > end
            || line.get(cursor..status_end) != Some("status")
            || !is_identifier_boundary(line, cursor, status_end)
        {
            return;
        }
        cursor = skip_ascii_whitespace(bytes, status_end, end);
        let operator_end = cursor.saturating_add(2);
        let Some(operator) = line.get(cursor..operator_end) else {
            return;
        };
        if !matches!(operator, "==" | "!=") {
            return;
        }
        cursor = skip_ascii_whitespace(bytes, operator_end, end);
        if cursor >= end || cursor >= bytes.len() || !is_ident_start(bytes[cursor]) {
            return;
        }
        let value_start = cursor;
        cursor += 1;
        while cursor < end && cursor < bytes.len() && is_ident_byte(bytes[cursor]) {
            cursor += 1;
        }
        let value = &line[value_start..cursor];
        if !is_workflow_status_literal(value) {
            return;
        }
        self.push_byte_range(
            line_index,
            status_end - "status".len(),
            "status".len(),
            "property",
            &["workflowStep"],
        );
        self.push_byte_range(
            line_index,
            value_start,
            cursor - value_start,
            "keyword",
            &["workflowStep"],
        );
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
        constant_keywords: &BTreeSet<&str>,
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
                    let (token_type, modifiers) = workflow_builtin_semantic_class(
                        line,
                        token,
                        token_start,
                        token_end,
                        constant_keywords,
                    );
                    self.push_byte_range(
                        line_index,
                        token_start,
                        token.len(),
                        token_type,
                        modifiers,
                    );
                }
                search_start = token_end;
            }
        }
    }

    fn scan_unit_tokens(
        &mut self,
        line_index: usize,
        start: usize,
        end: usize,
        units: &[&str],
    ) -> Vec<(usize, usize)> {
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
                let after_unit = skip_ascii_whitespace(line.as_bytes(), unit_end, end);
                if after_unit < end
                    && line
                        .as_bytes()
                        .get(after_unit)
                        .is_some_and(|byte| *byte == b'(')
                {
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
        occupied
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

    fn scan_symbol_operator_tokens(
        &mut self,
        line_index: usize,
        start: usize,
        end: usize,
        excluded_ranges: &[(usize, usize)],
    ) {
        const SYMBOL_OPERATORS: &[&[u8]] = &[
            b"->", b"==", b"!=", b">=", b"<=", b"=", b"+", b"-", b"*", b"/", b">", b"<",
        ];

        let line = self.lines[line_index];
        let bytes = line.as_bytes();
        let mut index = start;
        let end = end.min(bytes.len());
        while index < end {
            let mut matched = false;
            for operator in SYMBOL_OPERATORS {
                let operator_end = index + operator.len();
                if operator_end > end || &bytes[index..operator_end] != *operator {
                    continue;
                }
                matched = true;
                if !excluded_ranges
                    .iter()
                    .any(|(left, right)| ranges_overlap(index, operator_end, *left, *right))
                {
                    self.push_byte_range(line_index, index, operator.len(), "operator", &[]);
                }
                index = operator_end;
                break;
            }
            if !matched {
                index += 1;
            }
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

    fn push_identifier_path_on_line(
        &mut self,
        line_one_based: usize,
        path: &str,
        modifiers: &[&str],
    ) {
        let path = path.trim();
        if !is_simple_identifier_path(path) {
            return;
        }
        let Some(line_index) = line_one_based.checked_sub(1) else {
            return;
        };
        let Some(line) = self.lines.get(line_index).copied() else {
            return;
        };
        let mut search_start = 0usize;
        while search_start <= line.len() {
            let Some(relative) = line[search_start..].find(path) else {
                break;
            };
            let path_start = search_start + relative;
            let path_end = path_start + path.len();
            search_start = path_end;
            if !is_identifier_boundary(line, path_start, path_end) {
                continue;
            }
            let mut segment_start = path_start;
            for (index, segment) in path.split('.').enumerate() {
                let token_type = if index == 0 { "variable" } else { "property" };
                self.push_byte_range(
                    line_index,
                    segment_start,
                    segment.len(),
                    token_type,
                    modifiers,
                );
                segment_start += segment.len() + 1;
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
                        receiver_modifiers,
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
        "write" | "export" | "copy" | "move" | "delete" | "mkdir" | "print" | "log" => {
            &["sideEffect"]
        }
        "render" | "template" => &["sideEffect", "workflowStep"],
        "read" | "filter" | "select" | "derive" | "sort" | "require_one" | "column" | "columns"
        | "materialize" | "apply" | "collect" | "promote" | "records" | "results" | "cases"
        | "text" | "csv" | "json" | "toml" => &["workflowStep"],
        "state" => &["declaration", "state"],
        "input" => &["declaration", "input"],
        "output" => &["declaration", "output"],
        "const" | "parameter" | "port" | "across" | "through" | "index" => &["declaration"],
        "from" | "on" | "using" => &["model"],
        "report" | "show" | "plot" | "line" | "bar" | "histogram" | "summarize" | "summary"
        | "distribution" | "parity" | "residuals" => &["report"],
        "validate" | "check" | "assert" | "golden" | "test" | "matches" | "within"
        | "constraints" | "missing" | "interpolate" | "monotonic" | "between" => &["validation"],
        "simulate" | "solve" | "connect" | "conservation" | "equation" | "operator" | "states"
        | "inputs" | "outputs" => &["solver"],
        "script" | "struct" => &["deprecated"],
        _ => &[],
    }
}

fn is_workflow_status_literal(value: &str) -> bool {
    WORKFLOW_STATUS_LITERAL_KEYWORDS.contains(&value)
}

fn is_status_or_policy_literal(value: &str) -> bool {
    is_workflow_status_literal(value) || matches!(value, "error" | "keep" | "interpolate")
}

fn is_workflow_status_role_literal(value: &str) -> bool {
    is_workflow_status_literal(value)
}

fn language_constant_modifiers_for_line(
    line: &str,
    keyword: &str,
    token_start: usize,
) -> &'static [&'static str] {
    if is_log_level_literal(line, keyword, token_start) {
        &["sideEffect"]
    } else {
        language_constant_modifiers(keyword)
    }
}

fn is_log_level_literal(line: &str, keyword: &str, token_start: usize) -> bool {
    matches!(keyword, "debug" | "info" | "warn" | "error")
        && previous_identifier_before(line, token_start) == Some("log")
}

fn language_constant_modifiers(keyword: &str) -> &'static [&'static str] {
    match keyword {
        _ if is_workflow_status_role_literal(keyword) => &["workflowStep"],
        "cached" | "stale" | "hit" | "miss" => &["cache"],
        "asc" | "desc" => &["workflowStep"],
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
        | "trapezoidal"
        | "source_linear_terms"
        | "finite_difference" => &["solver"],
        "interval" | "ensemble" | "monte_carlo" => &["uncertain"],
        "lhs" | "latin_hypercube" | "latin-hypercube" | "grid" | "random" | "uniform" => {
            &["workflowStep"]
        }
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
        "rmse" => &["defaultLibrary", "report", "timeseries", "validation"],
        "mean" | "time_weighted_mean" | "min" | "max" | "median" | "std" | "p90" | "p95"
        | "duration_above" => &["defaultLibrary", "report", "timeseries"],
        "integrate" | "der" | "delay" | "sum" => &["defaultLibrary", "solver", "timeseries"],
        "fill" | "align" | "resample" => {
            &["defaultLibrary", "validation", "workflowStep", "timeseries"]
        }
        "check" | "coverage" | "fill_missing" => &["defaultLibrary", "validation", "timeseries"],
        "select_first_row" => &["defaultLibrary", "deprecated"],
        _ => &["defaultLibrary"],
    }
}

fn workflow_builtin_semantic_class(
    line: &str,
    keyword: &str,
    token_start: usize,
    token_end: usize,
    constant_keywords: &BTreeSet<&str>,
) -> (&'static str, &'static [&'static str]) {
    let token_type = workflow_builtin_token_type_for_line(line, keyword, token_start);
    if is_distribution_kind_literal(line, keyword, token_start) {
        return ("keyword", &["uncertain"]);
    }
    if token_type == "function"
        && constant_keywords.contains(keyword)
        && next_non_whitespace_after(line, token_end) != Some('(')
    {
        ("keyword", language_constant_modifiers(keyword))
    } else {
        (
            token_type,
            workflow_builtin_modifiers_for_line(line, keyword, token_start),
        )
    }
}

fn is_distribution_kind_literal(line: &str, keyword: &str, token_start: usize) -> bool {
    if !matches!(keyword, "normal" | "uniform") {
        return false;
    }
    let bytes = line.as_bytes();
    let mut cursor = token_start.min(bytes.len());
    while cursor > 0 && bytes[cursor - 1].is_ascii_whitespace() {
        cursor -= 1;
    }
    if cursor == 0 || bytes[cursor - 1] != b'=' {
        return false;
    }
    cursor -= 1;
    while cursor > 0 && bytes[cursor - 1].is_ascii_whitespace() {
        cursor -= 1;
    }
    let key_end = cursor;
    while cursor > 0 && is_ident_byte(bytes[cursor - 1]) {
        cursor -= 1;
    }
    if cursor == key_end || !is_ident_start(bytes[cursor]) || &line[cursor..key_end] != "kind" {
        return false;
    }
    let Some(open_paren) = line[..cursor].rfind('(') else {
        return false;
    };
    let mut name_end = open_paren;
    while name_end > 0 && bytes[name_end - 1].is_ascii_whitespace() {
        name_end -= 1;
    }
    let mut name_start = name_end;
    while name_start > 0 && is_ident_byte(bytes[name_start - 1]) {
        name_start -= 1;
    }
    name_start < name_end
        && is_ident_start(bytes[name_start])
        && &line[name_start..name_end] == "distribution"
}

fn workflow_builtin_token_type_for_line(
    line: &str,
    keyword: &str,
    token_start: usize,
) -> &'static str {
    if next_non_whitespace_after(line, token_start + keyword.len()) == Some('(') {
        return "function";
    }
    if is_workflow_command_keyword(line, keyword, token_start) {
        "keyword"
    } else {
        "function"
    }
}

fn is_workflow_command_keyword(line: &str, keyword: &str, token_start: usize) -> bool {
    match keyword {
        "sample" | "filter" | "select" | "derive" | "sort" | "require_one" | "materialize"
        | "collect" | "fill" | "align" | "resample" => true,
        "apply" => is_apply_step_phrase(line, token_start),
        "join" => is_table_join_phrase(line, token_start),
        "train" => next_identifier_after(line, token_start + keyword.len()) == Some("regression"),
        "regression" => previous_identifier_before(line, token_start) == Some("train"),
        "predict" => is_predict_using_phrase(line, token_start),
        "check" => next_identifier_after(line, token_start + keyword.len()) == Some("coverage"),
        "coverage" => previous_identifier_before(line, token_start) == Some("check"),
        "grid" | "random" | "lhs" | "latin_hypercube" | "latin-hypercube" => {
            previous_identifier_before(line, token_start) == Some("sample")
        }
        "uniform" => previous_identifier_before(line, token_start) == Some("sample"),
        "integrate" | "mean" | "max" | "min" => is_over_command_phrase(line, token_start, keyword),
        _ => false,
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
        return &["defaultLibrary", "workflowStep"];
    }
    if (keyword == "train"
        && next_identifier_after(line, token_start + keyword.len()) == Some("regression"))
        || (keyword == "regression"
            && previous_identifier_before(line, token_start) == Some("train"))
        || (keyword == "predict" && is_predict_using_phrase(line, token_start))
    {
        return &["defaultLibrary", "model", "workflowStep"];
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

fn add_file_operation_semantic_tokens(
    builder: &mut SemanticTokenBuilder<'_>,
    operation: &FileOperationInfo,
) {
    let modifiers = &["sideEffect", "external"];
    builder.push_keywords_on_line(operation.line, &[operation.operation.as_str()], modifiers);
    if matches!(operation.operation.as_str(), "copy" | "move") {
        builder.push_keywords_on_line(operation.line, &["to"], modifiers);
    }
    let Some(line_index) = operation.line.checked_sub(1) else {
        return;
    };
    builder.push_identifiers_on_line(line_index, &["file", "dir", "join"], "function", modifiers);
}

fn add_download_semantic_tokens(builder: &mut SemanticTokenBuilder<'_>, line: usize) {
    let modifiers = &["sideEffect", "external"];
    builder.push_keywords_on_line(line, &["download", "to"], modifiers);
    let Some(line_index) = line.checked_sub(1) else {
        return;
    };
    builder.push_identifiers_on_line(
        line_index,
        &["url", "file", "dir", "join"],
        "function",
        modifiers,
    );
}

fn add_http_request_semantic_tokens(
    builder: &mut SemanticTokenBuilder<'_>,
    line: usize,
    method: &str,
) {
    let modifiers = &["sideEffect", "external"];
    builder.push_keywords_on_line(
        line,
        &["http", http_request_method_keyword(method)],
        modifiers,
    );
    let Some(line_index) = line.checked_sub(1) else {
        return;
    };
    builder.push_identifiers_on_line(line_index, &["url"], "function", modifiers);
}

fn add_csv_export_target_semantic_tokens(
    builder: &mut SemanticTokenBuilder<'_>,
    line: usize,
    path: &str,
) {
    let modifiers = &["sideEffect"];
    builder.push_keywords_on_line(line, &["to", "csv"], modifiers);
    if path.trim().is_empty() {
        return;
    }
    let Some(line_index) = line.checked_sub(1) else {
        return;
    };
    builder.push_identifiers_on_line(line_index, &["file", "dir", "join"], "function", modifiers);
}

fn add_open_sqlite_semantic_tokens(
    builder: &mut SemanticTokenBuilder<'_>,
    line: usize,
    binding: &str,
    expression: &str,
) -> bool {
    let expression = expression.trim();
    if !expression.starts_with("open sqlite ") {
        return false;
    }
    let modifiers = &["sideEffect", "external", "db"];
    builder.push_on_line(
        line,
        binding,
        "variable",
        &["declaration", "external", "db"],
    );
    builder.push_keywords_on_line(line, &["open", "sqlite"], modifiers);
    let Some(line_index) = line.checked_sub(1) else {
        return true;
    };
    builder.push_identifiers_on_line(line_index, &["file", "dir", "join"], "function", modifiers);
    true
}

fn add_read_only_io_semantic_tokens(
    builder: &mut SemanticTokenBuilder<'_>,
    line: usize,
    binding: &str,
    expression: &str,
) -> bool {
    let Some((kind, source_expression)) = read_only_io_expression(expression) else {
        return false;
    };
    let modifiers = &["workflowStep", "external"];
    builder.push_on_line(
        line,
        binding,
        "variable",
        &["declaration", "workflowStep", "external"],
    );
    builder.push_keywords_on_line(line, &["read", kind], modifiers);
    let Some(line_index) = line.checked_sub(1) else {
        return true;
    };
    builder.push_identifiers_on_line(line_index, &["file", "dir", "join"], "function", modifiers);
    let source_expression = source_expression.trim();
    if is_simple_identifier_path(source_expression) {
        builder.push_identifier_path_on_line(line, source_expression, modifiers);
    }
    true
}

fn add_promotion_source_semantic_tokens(
    builder: &mut SemanticTokenBuilder<'_>,
    line: usize,
    source_literal: &str,
) {
    let source_literal = source_literal.trim();
    if source_literal.is_empty() {
        return;
    }
    let modifiers = &["workflowStep", "external"];
    let Some(line_index) = line.checked_sub(1) else {
        return;
    };
    builder.push_identifiers_on_line(
        line_index,
        &["file", "dir", "join", "url"],
        "function",
        modifiers,
    );
    if is_simple_identifier_path(source_literal) {
        builder.push_identifier_path_on_line(line, source_literal, modifiers);
    }
}

fn add_write_target_semantic_tokens(
    builder: &mut SemanticTokenBuilder<'_>,
    line: usize,
    format: &str,
    path: &str,
) {
    if format == "db" || path.trim().is_empty() {
        return;
    }
    let Some(line_index) = line.checked_sub(1) else {
        return;
    };
    if format == "standard_text" {
        builder.push_identifiers_on_line(
            line_index,
            &["file", "dir", "join"],
            "function",
            &["sideEffect", "workflowStep"],
        );
    } else {
        builder.push_identifiers_on_line(
            line_index,
            &["file", "dir", "join"],
            "function",
            &["sideEffect"],
        );
    }
}

fn add_command_style_semantic_tokens(
    builder: &mut SemanticTokenBuilder<'_>,
    command: &CommandStyleInfo,
) {
    match command.verb.as_str() {
        "apply" => {
            if is_simple_identifier_path(&command.target) {
                builder.push_on_line(command.line, &command.target, "function", &["workflowStep"]);
            }
            push_command_clause_keywords(builder, command, &["over"], &["workflowStep"]);
        }
        "integrate" => {
            let modifiers = &["solver", "timeseries"];
            push_command_clause_keywords(builder, command, &["over"], modifiers);
            push_command_style_identifier_paths(
                builder,
                command.line,
                &command.target,
                &[],
                modifiers,
            );
        }
        "mean" | "max" | "min" | "duration" => {
            let modifiers = &["report", "timeseries"];
            push_command_clause_keywords(builder, command, &["over"], modifiers);
            push_command_style_identifier_paths(
                builder,
                command.line,
                &command.target,
                &[],
                modifiers,
            );
        }
        "plot" => {
            builder.push_keywords_on_line(command.line, &["and", "vs"], &["report"]);
            push_command_clause_keywords(builder, command, &["over", "vs", "with"], &["report"]);
            push_plot_command_function_semantic_tokens(builder, command);
            push_command_style_identifier_paths(
                builder,
                command.line,
                &command.target,
                PLOT_COMMAND_STYLE_WORDS,
                &["report"],
            );
            for clause in &command.clauses {
                push_command_style_identifier_paths(
                    builder,
                    command.line,
                    &clause.value,
                    &[],
                    &["report"],
                );
            }
        }
        "show" => {
            push_command_style_identifier_paths(
                builder,
                command.line,
                &command.target,
                &[],
                &["report"],
            );
        }
        "check" => {
            let Some(target) = command.target.trim().strip_prefix("coverage ") else {
                return;
            };
            let modifiers = &["validation", "workflowStep", "timeseries"];
            builder.push_on_line(command.line, "check", "keyword", modifiers);
            builder.push_on_line(command.line, "coverage", "keyword", modifiers);
            push_command_style_identifier_paths(builder, command.line, target, &[], modifiers);
        }
        "fill" => {
            let modifiers = &["validation", "workflowStep", "timeseries"];
            if command.target.trim().starts_with("missing ") {
                builder.push_keywords_on_line(command.line, &["missing"], modifiers);
            }
            push_command_style_identifier_paths(
                builder,
                command.line,
                &command.target,
                &["missing"],
                modifiers,
            );
        }
        "align" | "resample" => {
            let modifiers = &["validation", "workflowStep", "timeseries"];
            push_command_clause_keywords(builder, command, &["with", "to", "by"], modifiers);
            push_command_style_identifier_paths(
                builder,
                command.line,
                &command.target,
                &[],
                modifiers,
            );
            for clause in &command.clauses {
                if clause.name == "by" {
                    continue;
                }
                push_command_style_identifier_paths(
                    builder,
                    command.line,
                    &clause.value,
                    &[],
                    modifiers,
                );
            }
        }
        "render" => {
            if !command.target.trim().starts_with("template ") {
                return;
            }
            builder.push_keywords_on_line(
                command.line,
                &["template"],
                &["sideEffect", "workflowStep"],
            );
            let template_source = command
                .target
                .trim()
                .strip_prefix("template ")
                .unwrap_or("")
                .trim();
            push_command_style_identifier_paths(
                builder,
                command.line,
                template_source,
                &["template", "file", "dir", "join"],
                &["workflowStep"],
            );
            for clause in &command.clauses {
                if clause.name == "to" {
                    builder.push_keywords_on_line(
                        command.line,
                        &["to"],
                        &["sideEffect", "workflowStep"],
                    );
                    push_command_style_identifier_paths(
                        builder,
                        command.line,
                        &clause.value,
                        &["file", "dir", "join"],
                        &["sideEffect", "workflowStep"],
                    );
                }
            }
        }
        _ => {}
    }
}

fn push_plot_command_function_semantic_tokens(
    builder: &mut SemanticTokenBuilder<'_>,
    command: &CommandStyleInfo,
) {
    let target = command.target.trim_start();
    for function in PLOT_COMMAND_STYLE_FUNCTIONS {
        let Some(rest) = target.strip_prefix(function) else {
            continue;
        };
        if rest.trim_start().starts_with('(') {
            builder.push_on_line(command.line, function, "function", &["report"]);
        }
    }
}

fn push_command_clause_keywords(
    builder: &mut SemanticTokenBuilder<'_>,
    command: &CommandStyleInfo,
    names: &[&str],
    modifiers: &[&str],
) {
    for clause in &command.clauses {
        if names.iter().any(|name| clause.name == *name) {
            builder.push_keywords_on_line(command.line, &[clause.name.as_str()], modifiers);
        }
    }
}

fn push_command_style_identifier_paths(
    builder: &mut SemanticTokenBuilder<'_>,
    line_one_based: usize,
    text: &str,
    skip: &[&str],
    modifiers: &[&str],
) {
    for path in command_style_identifier_paths(text, skip) {
        builder.push_identifier_path_on_line(line_one_based, path, modifiers);
    }
}

fn command_style_identifier_paths<'a>(text: &'a str, skip: &[&str]) -> Vec<&'a str> {
    text.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '_' || character == '.')
    })
    .filter_map(|part| {
        let part = part.trim_matches('.');
        if part.is_empty()
            || skip.iter().any(|keyword| *keyword == part)
            || !is_simple_identifier_path(part)
        {
            None
        } else {
            Some(part)
        }
    })
    .collect()
}

fn summary_statistic_names(text: &str) -> Vec<&str> {
    text.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .filter_map(|item| {
            let statistic = item.trim_start();
            let end = statistic
                .find(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
                .unwrap_or(statistic.len());
            let name = &statistic[..end];
            is_simple_identifier_segment(name).then_some(name)
        })
        .collect()
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

fn is_apply_step_phrase(line: &str, token_start: usize) -> bool {
    let Some(after_apply) = line.get(token_start + "apply".len()..) else {
        return false;
    };
    let mut parts = after_apply.split_whitespace();
    let Some(step) = parts.next() else {
        return false;
    };
    let Some(over_keyword) = parts.next() else {
        return false;
    };
    let Some(table) = parts.next() else {
        return false;
    };
    is_simple_identifier_path(step) && over_keyword == "over" && is_simple_identifier_path(table)
}

fn is_predict_using_phrase(line: &str, token_start: usize) -> bool {
    let Some(after_predict) = line.get(token_start + "predict".len()..) else {
        return false;
    };
    let mut parts = after_predict.split_whitespace();
    let Some(model) = parts.next() else {
        return false;
    };
    let Some(using_keyword) = parts.next() else {
        return false;
    };
    let Some(table) = parts.next() else {
        return false;
    };
    is_simple_identifier_path(model) && using_keyword == "using" && is_simple_identifier_path(table)
}

fn is_over_command_phrase(line: &str, token_start: usize, keyword: &str) -> bool {
    let Some(after_keyword) = line.get(token_start + keyword.len()..) else {
        return false;
    };
    let mut parts = after_keyword.split_whitespace();
    let Some(target) = parts.next() else {
        return false;
    };
    let Some(over_keyword) = parts.next() else {
        return false;
    };
    let Some(axis) = parts.next() else {
        return false;
    };
    is_simple_identifier_path(target) && over_keyword == "over" && is_simple_identifier_path(axis)
}

fn next_identifier_after(line: &str, start: usize) -> Option<&str> {
    let bytes = line.as_bytes();
    let mut index = start.min(bytes.len());
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    if index >= bytes.len() || !is_ident_start(bytes[index]) {
        return None;
    }
    let token_start = index;
    index += 1;
    while index < bytes.len() && is_ident_byte(bytes[index]) {
        index += 1;
    }
    Some(&line[token_start..index])
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
        if bytes[index..].starts_with(b"//") {
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
        if !in_string && bytes[index..].starts_with(b"//") {
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

fn is_unit_denominator_token(line: &str, start: usize, token: &str, units: &[&str]) -> bool {
    if !units.contains(&token) {
        return false;
    }
    let bytes = line.as_bytes();
    start
        .checked_sub(1)
        .and_then(|index| bytes.get(index).copied())
        == Some(b'/')
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
    completion_json_with_kind(completion, json!(completion_kind(&completion.kind)))
}

pub fn editor_completion_json(completion: &LspCompletion) -> Value {
    completion_json_with_kind(completion, json!(completion.kind))
}

fn completion_json_with_kind(completion: &LspCompletion, kind: Value) -> Value {
    let mut object = serde_json::Map::new();
    object.insert("label".to_owned(), json!(completion.label));
    object.insert("kind".to_owned(), kind);
    object.insert(
        "lsp_kind".to_owned(),
        json!(completion_kind(&completion.kind)),
    );
    object.insert("detail".to_owned(), json!(completion.detail));
    if let Some(insert) = &completion.insert {
        object.insert("insert".to_owned(), json!(insert));
    }
    if let Some(insert_snippet) = &completion.insert_snippet {
        object.insert("insert_snippet".to_owned(), json!(insert_snippet));
    }
    Value::Object(object)
}

pub fn hover_json(hover: &LspHover) -> Value {
    let mut value = format!(
        "**{}**\n\nKind: {}\n\n{}\n\nQuantity: `{}`",
        hover.name,
        hover_kind_label(&hover.kind),
        hover.detail,
        hover.quantity_kind
    );
    if let Some(display_unit) = hover_display_unit(&hover.display_unit) {
        value.push_str(&format!("\n\nDisplay unit: `{display_unit}`"));
    }
    if let Some(status) = &hover.status {
        value.push_str(&format!("\n\nStatus: {}", hover_status_label(status)));
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

fn hover_kind_label(kind: &str) -> String {
    match kind.trim() {
        "variable" => "Variable".to_owned(),
        "domain" => "Domain".to_owned(),
        "domain_variable" => "Domain variable".to_owned(),
        "domain_conservation" => "Domain conservation".to_owned(),
        "component" => "Component".to_owned(),
        "component_port" => "Component port".to_owned(),
        "connection" => "Connection".to_owned(),
        "component_assembly" => "Component assembly".to_owned(),
        "connection_set" => "Connection set".to_owned(),
        "assembly_equation" => "Assembly equation".to_owned(),
        "function" => "Function".to_owned(),
        "function_local" => "Function local".to_owned(),
        "where_local" => "where local".to_owned(),
        "class" => "Class".to_owned(),
        "class_field" => "Class field".to_owned(),
        "class_validation" => "Class validation".to_owned(),
        "class_method" => "Class method".to_owned(),
        "class_object" => "Class object".to_owned(),
        "object_field" => "Object field".to_owned(),
        "object_validation" => "Object validation".to_owned(),
        "http_response_field" => "HTTP response field".to_owned(),
        "sample_table_field" => "Sample table field".to_owned(),
        "db_connection_field" => "DB connection field".to_owned(),
        "case_table_field" => "Case table field".to_owned(),
        "case_output_table_field" => "Case output field".to_owned(),
        "case_result_collection_table_field" => "Case result collection field".to_owned(),
        "model_field" => "Model field".to_owned(),
        "prediction_table_field" => "Prediction table field".to_owned(),
        value => hover_label_text(value),
    }
}

fn hover_status_label(status: &str) -> String {
    status
        .trim()
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
        .enumerate()
        .map(|(index, part)| hover_status_word(part, index))
        .collect::<Vec<_>>()
        .join(" ")
}

fn hover_status_word(word: &str, index: usize) -> String {
    match word {
        "api" | "db" | "http" | "jit" | "lsp" | "sha" | "ttl" => word.to_uppercase(),
        value if index == 0 => hover_label_word(value),
        value => value.to_owned(),
    }
}

fn hover_label_text(value: &str) -> String {
    value
        .trim()
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(hover_label_word)
        .collect::<Vec<_>>()
        .join(" ")
}

fn hover_label_word(word: &str) -> String {
    match word {
        "db" => "DB".to_owned(),
        "http" => "HTTP".to_owned(),
        value => {
            let mut chars = value.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        }
    }
}

fn hover_display_unit(display_unit: &str) -> Option<&str> {
    let trimmed = display_unit.trim();
    if trimmed.is_empty() || trimmed == "-" {
        None
    } else {
        Some(trimmed)
    }
}
fn push_public_member_field_hovers(hovers: &mut Vec<LspHover>, report: &CheckReport, source: &str) {
    for binding in &report.semantic_program.typed_bindings {
        let Some((fields, kind)) = public_member_hover_fields(&binding.semantic_type.quantity_kind)
        else {
            continue;
        };
        push_member_field_hovers(
            hovers,
            source,
            &binding.name,
            &binding.semantic_type.quantity_kind,
            fields,
            kind,
        );
    }
}

fn public_member_hover_fields(
    quantity_kind: &str,
) -> Option<(&'static [(&'static str, &'static str)], &'static str)> {
    match quantity_kind {
        "HttpResponse" => Some((HTTP_RESPONSE_FIELD_COMPLETIONS, "http_response_field")),
        "Table[Sample]" => Some((SAMPLE_TABLE_FIELD_COMPLETIONS, "sample_table_field")),
        "DbConnection" => Some((DB_CONNECTION_FIELD_COMPLETIONS, "db_connection_field")),
        "Table[Case]" => Some((CASE_TABLE_FIELD_COMPLETIONS, "case_table_field")),
        "Table[CaseOutput]" => Some((
            CASE_OUTPUT_TABLE_FIELD_COMPLETIONS,
            "case_output_table_field",
        )),
        "Table[CaseResultCollection]" => Some((
            CASE_RESULT_COLLECTION_TABLE_FIELD_COMPLETIONS,
            "case_result_collection_table_field",
        )),
        "Table[Prediction]" => Some((PREDICTION_TABLE_FIELD_COMPLETIONS, "prediction_table_field")),
        value if value.starts_with("Model[") => Some((MODEL_FIELD_COMPLETIONS, "model_field")),
        _ => None,
    }
}

fn push_member_field_hovers(
    hovers: &mut Vec<LspHover>,
    source: &str,
    receiver: &str,
    receiver_quantity_kind: &str,
    fields: &[(&str, &str)],
    kind: &str,
) {
    if receiver.trim().is_empty() {
        return;
    }
    let needle = format!("{receiver}.");
    for (line_index, line) in source.lines().enumerate() {
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
                let Some((_, detail)) = fields.iter().find(|(candidate, _)| *candidate == field)
                else {
                    continue;
                };
                let access_start = member_access_path_start(line, receiver_start);
                hovers.push(LspHover {
                    name: line[access_start..field_end].to_owned(),
                    kind: kind.to_owned(),
                    line: line_index + 1,
                    detail: format!("{detail}; member of {receiver_quantity_kind}"),
                    quantity_kind: public_member_field_quantity_kind(field).to_owned(),
                    display_unit: "-".to_owned(),
                    status: Some("metadata".to_owned()),
                });
                search_start = field_end;
            }
        }
    }
}

fn member_access_path_start(line: &str, receiver_start: usize) -> usize {
    let bytes = line.as_bytes();
    let mut start = receiver_start;
    while start > 0 && bytes[start - 1] == b'.' {
        let dot = start - 1;
        let mut segment_start = dot;
        while segment_start > 0 && is_ident_byte(bytes[segment_start - 1]) {
            segment_start -= 1;
        }
        if segment_start == dot {
            break;
        }
        start = segment_start;
    }
    start
}

fn public_member_field_quantity_kind(field: &str) -> &'static str {
    match field {
        "status_code" | "seed" => "Int",
        "rmse" | "mae" => "DimensionlessNumber",
        "r2" => "Ratio",
        value if value.ends_with("_count") => "Int",
        _ => "String",
    }
}

pub fn hover_items(report: &CheckReport, source: Option<&str>) -> Vec<LspHover> {
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

    if let Some(source) = source {
        push_public_member_field_hovers(&mut hovers, report, source);
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
        ("top workflow", "Top-level EngLang workflow starter"),
        ("args block", "Root CLI argument block starter"),
        ("schema csv", "Typed CSV schema starter"),
        ("test block", "Unit-aware test block starter"),
        ("promote csv", "Typed CSV promotion starter"),
        ("plot line", "Line plot command starter"),
        ("log info", "Structured run log starter"),
        ("http get", "eng.net HTTP GET boundary"),
        ("http post", "eng.net HTTP POST boundary"),
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
        ("open sqlite", "eng.db SQLite database handle"),
        ("write sqlite", "eng.db SQLite table write"),
        ("copy file", "eng.fs copy generated output"),
        ("move file", "eng.fs move generated output"),
        ("delete file", "eng.fs delete generated output"),
        ("mkdir dir", "eng.fs create generated output directory"),
        ("run command", "eng.process command boundary"),
        ("promote json config", "eng.config JSON file promotion"),
        (
            "promote json records",
            "eng.table JSON records table promotion",
        ),
        ("promote toml config", "eng.config TOML file promotion"),
        (
            "materialize cases",
            "eng.case materialize native case rows from a table",
        ),
        (
            "apply cases",
            "eng.case apply a template or workflow step over case rows",
        ),
        (
            "collect results",
            "eng.case collect per-case outputs into a table",
        ),
        (
            "predict model using",
            "eng.model create predictions from a model and input table",
        ),
        (
            "sample grid",
            "eng.sampling deterministic grid sample table",
        ),
        (
            "sample random",
            "eng.sampling deterministic random sample table",
        ),
        (
            "sample uniform",
            "eng.sampling deterministic uniform/random sample table alias",
        ),
        ("sample lhs", "eng.sampling Latin hypercube sample table"),
        (
            "sample latin_hypercube",
            "eng.sampling Latin hypercube sample table alias",
        ),
        (
            "sample latin-hypercube",
            "eng.sampling Latin hypercube sample table alias",
        ),
    ] {
        let kind = if is_starter_snippet_label(label) {
            "snippet"
        } else {
            "stdlib"
        };
        push_completion(&mut items, &mut seen, label, kind, detail);
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
        if binding.semantic_type.quantity_kind == "Table[Sample]" {
            for (field, detail) in SAMPLE_TABLE_FIELD_COMPLETIONS {
                push_completion(
                    &mut items,
                    &mut seen,
                    &format!("{}.{}", binding.name, field),
                    "property",
                    detail,
                );
            }
        }
        if binding.semantic_type.quantity_kind == "DbConnection" {
            for (field, detail) in DB_CONNECTION_FIELD_COMPLETIONS {
                push_completion(
                    &mut items,
                    &mut seen,
                    &format!("{}.{}", binding.name, field),
                    "property",
                    detail,
                );
            }
        }
        if binding.semantic_type.quantity_kind == "Table[Case]" {
            for (field, detail) in CASE_TABLE_FIELD_COMPLETIONS {
                push_completion(
                    &mut items,
                    &mut seen,
                    &format!("{}.{}", binding.name, field),
                    "property",
                    detail,
                );
            }
        }
        if binding.semantic_type.quantity_kind == "Table[CaseOutput]" {
            for (field, detail) in CASE_OUTPUT_TABLE_FIELD_COMPLETIONS {
                push_completion(
                    &mut items,
                    &mut seen,
                    &format!("{}.{}", binding.name, field),
                    "property",
                    detail,
                );
            }
        }
        if binding.semantic_type.quantity_kind == "Table[CaseResultCollection]" {
            for (field, detail) in CASE_RESULT_COLLECTION_TABLE_FIELD_COMPLETIONS {
                push_completion(
                    &mut items,
                    &mut seen,
                    &format!("{}.{}", binding.name, field),
                    "property",
                    detail,
                );
            }
        }
        if binding.semantic_type.quantity_kind == "Table[Prediction]" {
            for (field, detail) in PREDICTION_TABLE_FIELD_COMPLETIONS {
                push_completion(
                    &mut items,
                    &mut seen,
                    &format!("{}.{}", binding.name, field),
                    "property",
                    detail,
                );
            }
        }
        if binding.semantic_type.quantity_kind.starts_with("Model[") {
            for (field, detail) in MODEL_FIELD_COMPLETIONS {
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
        .find(|transform| {
            receiver_matches_binding_name(receiver, &transform.binding)
                && transform.operation == "require_one"
        })?;
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
                receiver_matches_binding_name(&receiver, &binding.name)
                    && binding.semantic_type.quantity_kind == "HttpResponse"
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
        if report
            .semantic_program
            .typed_bindings
            .iter()
            .any(|binding| {
                receiver_matches_binding_name(&receiver, &binding.name)
                    && binding.semantic_type.quantity_kind == "Table[Sample]"
            })
        {
            let mut seen = BTreeMap::new();
            let mut items = Vec::new();
            for (field, detail) in SAMPLE_TABLE_FIELD_COMPLETIONS {
                if prefix.is_empty() || field.starts_with(&prefix) {
                    push_completion(&mut items, &mut seen, field, "property", detail);
                }
            }
            return items;
        }
        if report
            .semantic_program
            .typed_bindings
            .iter()
            .any(|binding| {
                receiver_matches_binding_name(&receiver, &binding.name)
                    && binding.semantic_type.quantity_kind == "DbConnection"
            })
        {
            let mut seen = BTreeMap::new();
            let mut items = Vec::new();
            for (field, detail) in DB_CONNECTION_FIELD_COMPLETIONS {
                if prefix.is_empty() || field.starts_with(&prefix) {
                    push_completion(&mut items, &mut seen, field, "property", detail);
                }
            }
            return items;
        }
        if let Some(fields) = report
            .semantic_program
            .typed_bindings
            .iter()
            .find_map(|binding| {
                if !receiver_matches_binding_name(&receiver, &binding.name) {
                    return None;
                }
                match binding.semantic_type.quantity_kind.as_str() {
                    "Table[Case]" => Some(CASE_TABLE_FIELD_COMPLETIONS),
                    "Table[CaseOutput]" => Some(CASE_OUTPUT_TABLE_FIELD_COMPLETIONS),
                    "Table[CaseResultCollection]" => {
                        Some(CASE_RESULT_COLLECTION_TABLE_FIELD_COMPLETIONS)
                    }
                    "Table[Prediction]" => Some(PREDICTION_TABLE_FIELD_COMPLETIONS),
                    value if value.starts_with("Model[") => Some(MODEL_FIELD_COMPLETIONS),
                    _ => None,
                }
            })
        {
            let mut seen = BTreeMap::new();
            let mut items = Vec::new();
            for (field, detail) in fields {
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
            .find(|promotion| receiver_matches_binding_name(&receiver, &promotion.binding))
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
            .find(|object| receiver_matches_binding_name(&receiver, &object.name))
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
            "expected_sha256",
            "retry",
            "timeout",
            "body_size_limit",
            "cache",
            "cache_key",
            "cache_dir",
            "cache_ttl",
            "status_code",
        ]);
    }
    if owner.starts_with("download ") {
        return Some(&[
            "offline_response",
            "expected_sha256",
            "retry",
            "timeout",
            "response_body_limit",
            "cache",
            "cache_key",
            "cache_dir",
            "cache_ttl",
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
            "target",
            "test",
            "hidden",
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

pub fn workflow_option_label_exists(label: &str) -> bool {
    WORKFLOW_OPTION_COMPLETIONS
        .iter()
        .any(|(candidate, _detail)| *candidate == label)
}

fn workflow_option_completion_detail(label: &str) -> Option<&'static str> {
    WORKFLOW_OPTION_COMPLETIONS
        .iter()
        .find(|(candidate, _detail)| *candidate == label)
        .map(|(_candidate, detail)| *detail)
}

fn contextual_workflow_option_completion_detail(label: &str) -> Option<&'static str> {
    workflow_option_completion_detail(label)
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
        "train_test_split" => Some(&["target", "features", "test", "seed"]),
        "regression" | "regression_table" | "train_regression" => {
            Some(&["target", "features", "algorithm", "test", "seed"])
        }
        "mlp" | "ann" => Some(&[
            "target",
            "features",
            "algorithm",
            "hidden",
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
    comment_start(line)
        .map(|comment_start| &line[..comment_start])
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
        insert: completion_insert_for_label(label).map(str::to_owned),
        insert_snippet: completion_insert_snippet_for_label(label),
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

fn is_starter_snippet_label(label: &str) -> bool {
    matches!(
        label,
        "top workflow"
            | "args block"
            | "schema csv"
            | "test block"
            | "promote csv"
            | "plot line"
            | "log info"
    )
}

fn completion_insert_for_label(label: &str) -> Option<&'static str> {
    match label {
        "file(...)" => Some("file(\"data/input.csv\")"),
        "dir(...)" => Some("dir(\"build/result\")"),
        "join(...)" => Some("join(args.output, \"summary.csv\")"),
        "parent(...)" => Some("parent(args.input)"),
        "stem(...)" => Some("stem(args.input)"),
        "extension(...)" => Some("extension(args.input)"),
        "exists path" => Some("exists args.input"),
        "top workflow" => Some("args {\n    input: CsvFile = file(\"data/sensor.csv\")\n}"),
        "args block" => Some("args {\n    input: CsvFile = file(\"data/sensor.csv\")\n}"),
        "schema csv" => Some("schema Sensor {\n    time: DateTime [iso8601]\n}"),
        "test block" => Some("test \"summary values\" {\n    assert mean_Q > 0 kW\n}"),
        "promote csv" => Some("promote csv file(\"data/sensor.csv\") as SensorData"),
        "plot line" => {
            Some("report {\n    plot Q over Time\n    with {\n        unit y = kW\n        title = \"Series over time\"\n    }\n}")
        },
        "log info" => Some("log info \"message\""),
        "http get" => Some("http get args.api_url"),
        "http post" => Some("http post args.api_url"),
        "read text" => Some("read text args.input"),
        "read json" => Some("read json args.config"),
        "read toml" => Some("read toml args.config"),
        "write text" => Some("write text \"outputs/log.txt\", text"),
        "write json" => Some("write json \"outputs/summary.json\", summary"),
        "write standard_text" => Some("write standard_text table"),
        "export summary to csv" => Some("export summary to csv join(args.output, \"summary.csv\")"),
        "open sqlite" => Some("open sqlite args.database_target"),
        "write sqlite" => Some("open sqlite args.database_target"),
        "copy file" => Some("copy file(\"data/template.txt\") to \"outputs/template.txt\""),
        "move file" => Some("move \"outputs/tmp.txt\" to \"outputs/archive/tmp.txt\""),
        "delete file" => Some("delete \"outputs/tmp.txt\""),
        "mkdir dir" => Some("mkdir \"outputs/archive\""),
        "run command" => Some("run command \"tool\""),
        "promote json config" => Some("promote json file(\"workflow.json\") as WorkflowConfig"),
        "promote toml config" => Some("promote toml file(\"workflow.toml\") as WorkflowConfig"),
        "materialize cases" => Some("materialize cases designs"),
        "apply cases" => Some("apply case_input_template over cases"),
        "collect results" => Some("collect results case_results"),
        "predict model using" => Some("predict model using designs"),
        _ => None,
    }
}

fn completion_insert_snippet_for_label(label: &str) -> Option<String> {
    if let Some(base) = label.strip_suffix("[T]") {
        return Some(format!("{base}[${{1:T}}]"));
    }
    if label == "LinearOperator[From -> To]" {
        return Some("LinearOperator[${1:From} -> ${2:To}]".to_owned());
    }
    match label {
        "file(...)" => Some("file(\"${1:data/input.csv}\")".to_owned()),
        "dir(...)" => Some("dir(\"${1:build/result}\")".to_owned()),
        "join(...)" => Some("join(${1:args.output}, \"${2:summary.csv}\")".to_owned()),
        "parent(...)" => Some("parent(${1:args.input})".to_owned()),
        "stem(...)" => Some("stem(${1:args.input})".to_owned()),
        "extension(...)" => Some("extension(${1:args.input})".to_owned()),
        "exists path" => Some("exists ${1:args.input}".to_owned()),
        "top workflow" => Some(
            "args {\n    ${1:input}: CsvFile = file(\"${2:data/sensor.csv}\")\n}\n\n${3:Q}: HeatRate [kW] = ${4:1 kW}\nprint \"${5:case ready}\"\nlog info \"${6:Q = {Q: .2 kW}}\"\n\nreport {\n    show ${3:Q}\n}"
                .to_owned(),
        ),
        "args block" => Some(
            "args {\n    ${1:input}: CsvFile = file(\"${2:data/sensor.csv}\")\n}"
                .to_owned(),
        ),
        "schema csv" => Some(
            "schema ${1:Sensor} {\n    ${2:time}: DateTime [iso8601]\n    ${3:T_supply}: AbsoluteTemperature [degC]\n    ${4:heat}: HeatRate [kW]\n}"
                .to_owned(),
        ),
        "test block" => Some(
            "test \"${1:summary values}\" {\n    assert ${2:mean_Q} > ${3:0 kW}\n    assert ${4:E_coil} == ${5:1.26 kWh} within ${6:0.02 kWh}\n    golden \"${7:summary.csv}\" matches file(\"${8:golden/summary.csv}\")\n}"
                .to_owned(),
        ),
        "promote csv" => Some(
            "promote csv file(\"${1:data/sensor.csv}\") as ${2:SensorData}".to_owned(),
        ),
        "plot line" => Some(
            "report {\n    plot ${1:series} over Time\n    with {\n        unit y = ${2:kW}\n        title = \"${3:Series over time}\"\n    }\n}"
                .to_owned(),
        ),
        "log info" => Some("log info \"${1:message}\"".to_owned()),
        "http get" => Some(
            "http get ${1:args.api_url}\nwith {\n    query = {\n        ${2:station} = ${3:args.station_id}\n    }\n    offline_response = file(\"${4:data/response.json}\")\n    expected_sha256 = \"${5:sha256}\"\n    retry = ${6:2}\n    timeout = ${7:30 s}\n    body_size_limit = ${8:2 MB}\n    cache = true\n    cache_key = [\"${9:http}\", ${10:args.year}]\n}"
                .to_owned(),
        ),
        "http post" => Some(
            "http post ${1:args.api_url}\nwith {\n    headers = {\n        content_type = \"application/json\"\n    }\n    body = ${2:request_body}\n    offline_response = file(\"${3:data/response.json}\")\n    expected_sha256 = \"${4:sha256}\"\n    timeout = ${5:30 s}\n    body_size_limit = ${6:2 MB}\n    cache = true\n    cache_key = [\"${7:post}\", ${8:args.case_id}]\n}"
                .to_owned(),
        ),
        "read text" => Some("read text ${1:args.input}".to_owned()),
        "read json" => Some("read json ${1:args.config}".to_owned()),
        "read toml" => Some("read toml ${1:args.config}".to_owned()),
        "write text" => Some("write text \"${1:outputs/log.txt}\", ${2:text}".to_owned()),
        "write json" => Some("write json \"${1:outputs/summary.json}\", ${2:summary}".to_owned()),
        "write standard_text" => Some(
            "write standard_text ${1:table}\nwith {\n    output = join(${2:args.output}, \"${3:standard_weather_file.txt}\")\n    overwrite = true\n}"
                .to_owned(),
        ),
        "export summary to csv" => Some(
            "export summary to csv join(${1:args.output}, \"${2:summary.csv}\") {\n    ${3:metric} = ${4:value}\n}"
                .to_owned(),
        ),
        "open sqlite" => Some("open sqlite ${1:args.database_target}".to_owned()),
        "write sqlite" => Some(
            "${1:db} = open sqlite ${2:args.database_target}\nwrite ${3:predictions} to ${1:db}.table(\"${4:predictions}\")\nwith {\n    mode = ${5:replace}\n    transaction = commit\n}"
                .to_owned(),
        ),
        "copy file" => Some(
            "copy file(\"${1:data/template.txt}\") to \"${2:outputs/template.txt}\"".to_owned(),
        ),
        "move file" => {
            Some("move \"${1:outputs/tmp.txt}\" to \"${2:outputs/archive/tmp.txt}\"".to_owned())
        }
        "delete file" => Some("delete \"${1:outputs/tmp.txt}\"".to_owned()),
        "mkdir dir" => Some("mkdir \"${1:outputs/archive}\"".to_owned()),
        "run command" => Some("run command \"${1:tool}\"".to_owned()),
        "promote json config" => {
            Some("promote json file(\"${1:workflow.json}\") as ${2:WorkflowConfig}".to_owned())
        }
        "promote toml config" => {
            Some("promote toml file(\"${1:workflow.toml}\") as ${2:WorkflowConfig}".to_owned())
        }
        "materialize cases" => Some("materialize cases ${1:designs}".to_owned()),
        "apply cases" => Some(
            "apply ${1:case_input_template} over ${2:cases}\nwith {\n    template = file(\"${3:model/native_case_template.txt}\")\n    output = \"{case_dir}/${4:input.txt}\"\n    missing = error\n    overwrite = true\n}"
                .to_owned(),
        ),
        "collect results" => Some("collect results ${1:case_results}".to_owned()),
        "train regression" => Some(
            "train regression ${1:training_results}\nwith {\n    target = ${2:annual_electricity}\n    features = [${3:cooling_cop}]\n    test = ${4:0.25}\n    seed = ${5:7}\n}"
                .to_owned(),
        ),
        "predict model using" => Some("predict ${1:model} using ${2:designs}".to_owned()),
        "sample grid" => Some(
            "sample grid\nwith {\n    count = ${1:9}\n    ${2:parameter} = uniform(${3:0.0}, ${4:1.0})\n}"
                .to_owned(),
        ),
        "sample random" => Some(
            "sample random\nwith {\n    count = ${1:8}\n    seed = ${2:42}\n    ${3:parameter} = uniform(${4:0.0}, ${5:1.0})\n}"
                .to_owned(),
        ),
        "sample uniform" => Some(
            "sample uniform\nwith {\n    count = ${1:8}\n    seed = ${2:42}\n    ${3:parameter} = uniform(${4:0.0}, ${5:1.0})\n}"
                .to_owned(),
        ),
        "sample lhs" => Some(
            "sample lhs\nwith {\n    count = ${1:8}\n    seed = ${2:42}\n    ${3:parameter} = uniform(${4:0.0}, ${5:1.0})\n}"
                .to_owned(),
        ),
        "sample latin_hypercube" => Some(
            "sample latin_hypercube\nwith {\n    count = ${1:8}\n    seed = ${2:42}\n    ${3:parameter} = uniform(${4:0.0}, ${5:1.0})\n}"
                .to_owned(),
        ),
        "sample latin-hypercube" => Some(
            "sample latin-hypercube\nwith {\n    count = ${1:8}\n    seed = ${2:42}\n    ${3:parameter} = uniform(${4:0.0}, ${5:1.0})\n}"
                .to_owned(),
        ),
        _ => None,
    }
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
        "snippet" => 15,
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
    let prefix_end = bytes.len();
    let mut prefix_start = prefix_end;
    while prefix_start > 0 && is_ident_byte(bytes[prefix_start - 1]) {
        prefix_start -= 1;
    }
    if prefix_start == 0 || bytes[prefix_start - 1] != b'.' {
        return None;
    }
    let receiver_end = prefix_start - 1;
    let mut receiver_start = receiver_end;
    while receiver_start > 0
        && (is_ident_byte(bytes[receiver_start - 1]) || bytes[receiver_start - 1] == b'.')
    {
        receiver_start -= 1;
    }
    if receiver_start == receiver_end {
        return None;
    }
    let receiver = &before_cursor[receiver_start..receiver_end];
    if !is_simple_identifier_path(receiver) {
        return None;
    }
    Some((
        receiver.to_owned(),
        before_cursor[prefix_start..prefix_end].to_owned(),
    ))
}

fn receiver_matches_binding_name(receiver: &str, binding_name: &str) -> bool {
    receiver == binding_name
        || receiver
            .rsplit('.')
            .next()
            .is_some_and(|segment| segment != receiver && segment == binding_name)
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

    fn assert_semantic_token_on_line(
        snapshot: &LspSnapshot,
        source: &str,
        line_match: &str,
        label: &str,
        token_type: &str,
        modifier: &str,
    ) {
        let line_number = source
            .lines()
            .position(|line| line.contains(line_match))
            .unwrap_or_else(|| panic!("source should contain line matching `{line_match}`"));
        assert!(
            snapshot.semantic_tokens.tokens.iter().any(|token| {
                token.line == line_number
                    && token.token_type == token_type
                    && token.modifiers.iter().any(|item| item == modifier)
                    && source.lines().nth(token.line).is_some_and(|line| {
                        line.get(token.start..token.start + token.length) == Some(label)
                    })
            }),
            "semantic token `{label}` on `{line_match}` should be {token_type} with modifier `{modifier}`"
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

    fn package_semantic_scope_values<'a>(
        package: &'a serde_json::Value,
        selector: &str,
    ) -> BTreeSet<&'a str> {
        package["contributes"]["semanticTokenScopes"]
            .as_array()
            .expect("package should declare semanticTokenScopes")
            .iter()
            .find(|scope| scope["language"] == "englang")
            .and_then(|scope| scope["scopes"].as_object())
            .and_then(|scopes| scopes.get(selector))
            .and_then(|scope| scope.as_array())
            .unwrap_or_else(|| panic!("package should declare semanticTokenScopes for {selector}"))
            .iter()
            .filter_map(|scope| scope.as_str())
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

    fn assert_semantic_token_on_line_type(
        snapshot: &LspSnapshot,
        source: &str,
        line_needle: &str,
        label: &str,
        token_type: &str,
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
                    })
            }),
            "semantic token `{label}` on `{line_needle}` should have type `{token_type}`"
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

    fn assert_semantic_token_on_line_with_modifier(
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
                            && token.modifiers.iter().any(|item| item == modifier)
                    })
            }),
            "semantic token `{label}` on `{line_needle}` should be `{token_type}` with modifier `{modifier}`"
        );
    }

    fn assert_semantic_token_after_line_with_modifier(
        snapshot: &LspSnapshot,
        source: &str,
        owner_line_needle: &str,
        line_needle: &str,
        label: &str,
        token_type: &str,
        modifier: &str,
    ) {
        let owner_line_index = source
            .lines()
            .position(|line| line.contains(owner_line_needle))
            .unwrap_or_else(|| panic!("source line `{owner_line_needle}` should be present"));
        let line_index = source
            .lines()
            .enumerate()
            .skip(owner_line_index + 1)
            .find(|(_, line)| line.contains(line_needle))
            .map(|(index, _)| index)
            .unwrap_or_else(|| {
                panic!("source line `{line_needle}` should be present after `{owner_line_needle}`")
            });
        assert!(
            snapshot.semantic_tokens.tokens.iter().any(|token| {
                token.line == line_index
                    && source.lines().nth(token.line).is_some_and(|line| {
                        line.get(token.start..token.start + token.length) == Some(label)
                            && token.token_type == token_type
                            && token.modifiers.iter().any(|item| item == modifier)
                    })
            }),
            "semantic token `{label}` after `{owner_line_needle}` should be `{token_type}` with modifier `{modifier}`"
        );
    }
    fn assert_first_diagnostic_underlines(source: &str, code: &str, expected_text: &str) {
        let (line, start, end) = first_diagnostic_underline(source, code);

        assert_eq!(
            line.get(start..end),
            Some(expected_text),
            "diagnostic {code} should underline `{expected_text}` on `{line}`"
        );
    }

    fn assert_first_diagnostic_underlines_after(
        source: &str,
        code: &str,
        required_prefix: &str,
        expected_text: &str,
    ) {
        let (line, start, end) = first_diagnostic_underline(source, code);

        assert_eq!(
            line.get(start..end),
            Some(expected_text),
            "diagnostic {code} should underline `{expected_text}` on `{line}`"
        );
        assert!(
            line[..start].contains(required_prefix),
            "diagnostic {code} should underline after `{required_prefix}` on `{line}`"
        );
    }

    fn first_diagnostic_underline<'a>(source: &'a str, code: &str) -> (&'a str, usize, usize) {
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

        (line, start, end)
    }

    #[test]
    fn snapshot_exposes_lsp_diagnostics_hover_and_completion() {
        let source = "/// heat rate smoke\n// write text should stay a comment\nQ: HeatRate [kW] = 2 kW - 1\n}\n";
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
            "mkdir",
            "open",
            "sqlite",
            "predict",
            "check",
            "coverage",
            "records",
            "promote json records",
            "mkdir dir",
            "materialize",
            "materialize cases",
            "apply",
            "apply cases",
            "collect",
            "collect results",
            "predict model using",
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
            completion.label == "eng.net" && completion.detail.starts_with("Native:")
        }));
        assert!(snapshot.completions.iter().any(|completion| {
            completion.label == "eng.cache" && completion.detail.starts_with("Native:")
        }));
        assert!(snapshot.completions.iter().any(|completion| {
            completion.label == "eng.uncertainty" && completion.detail.starts_with("Native:")
        }));
        for (label, detail_part) in [
            ("require_one", "exactly one row"),
            ("uniform", "eng.sampling"),
            ("train regression", "regression model"),
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
        let dim_line_index = source
            .lines()
            .position(|line| line.contains("2 kW - 1"))
            .expect("diagnostic line");
        let dim_line = source.lines().nth(dim_line_index).expect("diagnostic line");
        let minus_character = dim_line.find('-').expect("minus operator");
        assert_eq!(
            dim_diagnostic["range"]["start"]["line"].as_u64(),
            Some(dim_line_index as u64)
        );
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
        let doc_comment_line = source
            .lines()
            .position(|line| line.starts_with("///"))
            .expect("doc comment line");
        assert!(snapshot.semantic_tokens.tokens.iter().any(|token| {
            token.line == doc_comment_line
                && token.token_type == "comment"
                && token
                    .modifiers
                    .iter()
                    .any(|modifier| modifier == "documentation")
        }));
        let slash_comment_line = source
            .lines()
            .position(|line| line.contains("should stay a comment"))
            .expect("slash comment line");
        assert!(snapshot.semantic_tokens.tokens.iter().any(|token| {
            token.line == slash_comment_line
                && token.token_type == "comment"
                && source.lines().nth(token.line).is_some_and(|line| {
                    line.get(token.start..token.start + token.length)
                        == Some("// write text should stay a comment")
                })
        }));
        assert!(!snapshot.semantic_tokens.tokens.iter().any(|token| {
            token.line == slash_comment_line
                && token.token_type == "keyword"
                && source.lines().nth(token.line).is_some_and(|line| {
                    line.get(token.start..token.start + token.length) == Some("write")
                })
        }));
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
    fn editor_metadata_exports_completion_items_and_semantic_legend() {
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
        let syntax_keywords = syntax_catalog["keywords"]
            .as_array()
            .expect("syntax catalog keywords should be an array");
        for label in ["parity", "residuals"] {
            assert!(
                syntax_keywords.iter().any(|keyword| keyword == label),
                "syntax catalog should expose plot report keyword {label}"
            );
        }
        let workflow_status_literals = syntax_catalog["workflow_status_literals"]
            .as_array()
            .expect("syntax catalog workflow status literals should be an array");
        for label in ["partial", "missing", "empty"] {
            assert!(
                workflow_status_literals
                    .iter()
                    .any(|status_literal| status_literal == label),
                "syntax catalog should expose workflow status literal {label}"
            );
        }
        let syntax_constants = syntax_catalog["constants"]
            .as_array()
            .expect("syntax catalog constants should be an array");
        for label in [
            "monte_carlo",
            "source_linear_terms",
            "lhs",
            "latin_hypercube",
            "planned",
            "rendered",
            "collected",
        ] {
            assert!(
                syntax_constants.iter().any(|constant| constant == label),
                "syntax catalog should expose language constant {label}"
            );
        }
        let syntax_operator_words = syntax_catalog["operator_words"]
            .as_array()
            .expect("syntax catalog operator words should be an array");
        for label in ["between", "within", "matches"] {
            assert!(
                syntax_operator_words
                    .iter()
                    .any(|operator_word| operator_word == label),
                "syntax catalog should expose operator word {label}"
            );
        }
        let keyword_groups = &syntax_catalog["keyword_groups"];
        for (group, label) in [
            ("import", "use"),
            ("declaration", "schema"),
            ("block", "args"),
            ("modifier", "state"),
            ("report", "summarize"),
            ("validation", "coverage"),
            ("side_effect", "write"),
            ("external_boundary", "http"),
            ("solver", "solve"),
            ("workflow", "require_one"),
        ] {
            assert!(
                keyword_groups[group]
                    .as_array()
                    .is_some_and(|labels| labels.iter().any(|item| item == label)),
                "syntax catalog should expose keyword group {group} label {label}"
            );
        }
        assert!(
            syntax_catalog["workflow_builtins"]
                .as_array()
                .is_some_and(|labels| labels.iter().any(|label| label == "train")),
            "syntax catalog should expose workflow builtin labels"
        );
        assert!(
            syntax_catalog["hyphenated_workflow_builtins"]
                .as_array()
                .is_some_and(|labels| labels.iter().any(|label| label == "latin-hypercube")),
            "syntax catalog should expose hyphenated workflow builtin labels"
        );
        assert!(
            syntax_catalog["legacy_workflow_builtin_aliases"]
                .as_array()
                .is_some_and(|labels| labels.iter().any(|label| label == "regression_table")),
            "syntax catalog should expose legacy workflow builtin aliases"
        );
        assert!(
            syntax_catalog["legacy_workflow_option_aliases"]
                .as_array()
                .is_some_and(|labels| labels.iter().any(|label| label == "fixture")),
            "syntax catalog should expose legacy workflow option aliases"
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
            syntax_catalog["workflow_options"]
                .as_array()
                .is_some_and(|options| options.iter().any(|option| option["label"] == "unit y")),
            "syntax catalog should expose plot axis with-option labels"
        );
        assert!(
            syntax_catalog["http_response_fields"]
                .as_array()
                .is_some_and(|fields| fields
                    .iter()
                    .any(|field| field["label"] == "response_source")),
            "syntax catalog should expose HTTP response source field label"
        );
        assert!(
            syntax_catalog["http_response_fields"]
                .as_array()
                .is_some_and(|fields| fields
                    .iter()
                    .any(|field| field["label"] == "url_with_query")),
            "syntax catalog should expose HTTP response field labels"
        );
        assert!(
            syntax_catalog["http_response_fields"]
                .as_array()
                .is_some_and(|fields| fields.iter().all(|field| field["label"] != "status")),
            "syntax catalog should not expose response.status as a completion field"
        );
        assert!(
            syntax_catalog["sample_table_fields"]
                .as_array()
                .is_some_and(|fields| fields.iter().any(|field| field["label"] == "sample_count")),
            "syntax catalog should expose sample table field labels"
        );
        assert!(
            syntax_catalog["sample_table_fields"]
                .as_array()
                .is_some_and(|fields| fields.iter().any(|field| field["label"] == "row_preview")),
            "syntax catalog should expose sample row preview field"
        );
        assert!(
            syntax_catalog["case_table_fields"]
                .as_array()
                .is_some_and(|fields| fields.iter().any(|field| field["label"] == "pending_count")),
            "syntax catalog should expose case table field labels"
        );
        assert!(
            syntax_catalog["case_output_table_fields"]
                .as_array()
                .is_some_and(|fields| fields
                    .iter()
                    .any(|field| field["label"] == "expected_count")),
            "syntax catalog should expose expected case output table field labels"
        );
        let case_output_fields = syntax_catalog["case_output_table_fields"]
            .as_array()
            .expect("syntax catalog should expose case output table fields");
        assert!(
            case_output_fields
                .iter()
                .any(|field| field["label"] == "rendered_count"),
            "syntax catalog should expose case output table field labels"
        );
        assert!(
            !case_output_fields
                .iter()
                .any(|field| field["label"] == "planned_count"),
            "syntax catalog should hide compatibility alias planned_count from editor suggestions"
        );
        assert!(
            syntax_catalog["case_result_collection_table_fields"]
                .as_array()
                .is_some_and(|fields| fields
                    .iter()
                    .any(|field| field["label"] == "collected_count")),
            "syntax catalog should expose case result collection table field labels"
        );
        assert!(
            syntax_catalog["model_fields"]
                .as_array()
                .is_some_and(|fields| fields.iter().any(|field| field["label"] == "rmse")),
            "syntax catalog should expose model field labels"
        );
        assert!(
            syntax_catalog["prediction_table_fields"]
                .as_array()
                .is_some_and(|fields| fields.iter().any(|field| field["label"] == "output_column")),
            "syntax catalog should expose prediction table field labels"
        );
        assert!(
            syntax_catalog["units"]
                .as_array()
                .is_some_and(|units| units.iter().any(|unit| unit["label"] == "kW")),
            "syntax catalog should expose compiler unit labels"
        );
        assert!(
            syntax_catalog["legacy_unit_aliases"]
                .as_array()
                .is_some_and(|units| units.iter().any(|unit| unit.as_str() == Some("%"))),
            "syntax catalog should expose editor legacy unit aliases"
        );

        let completions = metadata["completion_items"]
            .as_array()
            .expect("editor completion items should be an array");
        assert_eq!(
            metadata["completion_items_count"].as_u64(),
            Some(completions.len() as u64)
        );
        assert!(metadata.get("completion_seed").is_none());
        assert!(metadata.get("completion_seed_count").is_none());
        for (label, kind) in [
            ("records", "keyword"),
            ("top workflow", "snippet"),
            ("args block", "snippet"),
            ("schema csv", "snippet"),
            ("test block", "snippet"),
            ("promote csv", "snippet"),
            ("plot line", "snippet"),
            ("log info", "snippet"),
            ("promote json records", "stdlib"),
            ("http get", "stdlib"),
            ("http post", "stdlib"),
            ("sample uniform", "stdlib"),
            ("sample latin-hypercube", "stdlib"),
            ("open sqlite", "stdlib"),
            ("write sqlite", "stdlib"),
            ("write standard_text", "stdlib"),
            ("export summary to csv", "stdlib"),
            ("materialize cases", "stdlib"),
            ("apply cases", "stdlib"),
            ("collect results", "stdlib"),
            ("train regression", "function"),
            ("predict model using", "stdlib"),
            ("read json", "stdlib"),
            ("eng.table", "stdlib"),
            ("HeatRate", "class"),
            ("StateVector[T]", "class"),
            ("LinearOperator[From -> To]", "class"),
            ("kW", "unit"),
            ("output", "property"),
            ("inputs", "property"),
            ("index", "keyword"),
            ("split", "property"),
        ] {
            assert!(
                completions
                    .iter()
                    .any(|completion| completion["label"] == label && completion["kind"] == kind),
                "editor metadata should include completion {label} as {kind}"
            );
        }
        let top_workflow_completion = completions
            .iter()
            .find(|completion| completion["label"] == "top workflow")
            .expect("editor metadata should include top workflow completion");
        assert_eq!(top_workflow_completion["lsp_kind"], 15);
        assert_eq!(
            top_workflow_completion["insert_snippet"],
            "args {\n    ${1:input}: CsvFile = file(\"${2:data/sensor.csv}\")\n}\n\n${3:Q}: HeatRate [kW] = ${4:1 kW}\nprint \"${5:case ready}\"\nlog info \"${6:Q = {Q: .2 kW}}\"\n\nreport {\n    show ${3:Q}\n}"
        );
        let schema_csv_completion = completions
            .iter()
            .find(|completion| completion["label"] == "schema csv")
            .expect("editor metadata should include schema csv completion");
        assert_eq!(schema_csv_completion["lsp_kind"], 15);
        assert_eq!(
            schema_csv_completion["insert_snippet"],
            "schema ${1:Sensor} {\n    ${2:time}: DateTime [iso8601]\n    ${3:T_supply}: AbsoluteTemperature [degC]\n    ${4:heat}: HeatRate [kW]\n}"
        );
        let http_get_completion = completions
            .iter()
            .find(|completion| completion["label"] == "http get")
            .expect("editor metadata should include http get completion");
        assert_eq!(http_get_completion["lsp_kind"], 9);
        assert_eq!(
            http_get_completion["insert_snippet"],
            "http get ${1:args.api_url}\nwith {\n    query = {\n        ${2:station} = ${3:args.station_id}\n    }\n    offline_response = file(\"${4:data/response.json}\")\n    expected_sha256 = \"${5:sha256}\"\n    retry = ${6:2}\n    timeout = ${7:30 s}\n    body_size_limit = ${8:2 MB}\n    cache = true\n    cache_key = [\"${9:http}\", ${10:args.year}]\n}"
        );
        let write_sqlite_completion = completions
            .iter()
            .find(|completion| completion["label"] == "write sqlite")
            .expect("editor metadata should include write sqlite completion");
        assert_eq!(
            write_sqlite_completion["insert_snippet"],
            "${1:db} = open sqlite ${2:args.database_target}\nwrite ${3:predictions} to ${1:db}.table(\"${4:predictions}\")\nwith {\n    mode = ${5:replace}\n    transaction = commit\n}"
        );
        let sample_uniform_completion = completions
            .iter()
            .find(|completion| completion["label"] == "sample uniform")
            .expect("editor metadata should include sample uniform completion");
        assert_eq!(sample_uniform_completion["lsp_kind"], 9);
        assert_eq!(
            sample_uniform_completion["insert_snippet"],
            "sample uniform\nwith {\n    count = ${1:8}\n    seed = ${2:42}\n    ${3:parameter} = uniform(${4:0.0}, ${5:1.0})\n}"
        );
        let train_regression_completion = completions
            .iter()
            .find(|completion| completion["label"] == "train regression")
            .expect("editor metadata should include train regression completion");
        assert_eq!(
            train_regression_completion["insert_snippet"],
            "train regression ${1:training_results}\nwith {\n    target = ${2:annual_electricity}\n    features = [${3:cooling_cop}]\n    test = ${4:0.25}\n    seed = ${5:7}\n}"
        );
        let apply_cases_completion = completions
            .iter()
            .find(|completion| completion["label"] == "apply cases")
            .expect("editor metadata should include apply cases completion");
        assert_eq!(
            apply_cases_completion["insert_snippet"],
            "apply ${1:case_input_template} over ${2:cases}\nwith {\n    template = file(\"${3:model/native_case_template.txt}\")\n    output = \"{case_dir}/${4:input.txt}\"\n    missing = error\n    overwrite = true\n}"
        );
        let read_json_completion = completions
            .iter()
            .find(|completion| completion["label"] == "read json")
            .expect("editor metadata should include read json completion");
        assert_eq!(read_json_completion["insert"], "read json args.config");
        assert_eq!(
            read_json_completion["insert_snippet"],
            "read json ${1:args.config}"
        );
        let linear_operator_completion = completions
            .iter()
            .find(|completion| completion["label"] == "LinearOperator[From -> To]")
            .expect("editor metadata should include LinearOperator completion");
        assert_eq!(
            linear_operator_completion["insert_snippet"],
            "LinearOperator[${1:From} -> ${2:To}]"
        );
        let offline_response_completion = completions
            .iter()
            .find(|completion| completion["label"] == "offline_response")
            .expect("editor metadata should include offline_response option completion");
        assert_eq!(
            offline_response_completion["detail"],
            "Pinned offline HTTP response used instead of live network"
        );
        assert!(
            !completions
                .iter()
                .any(|completion| completion["label"] == "fixture"),
            "editor completion items should not suggest legacy fixture option; use offline_response"
        );
        for legacy_model_completion in ["regression_table", "train_regression"] {
            assert!(
                !completions
                    .iter()
                    .any(|completion| completion["label"] == legacy_model_completion),
                "editor completion items should not suggest legacy model completion {legacy_model_completion}; use train regression"
            );
        }
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
        assert!(
            cache_completion["detail"].as_str().is_some_and(
                |detail| !detail.contains("broader") && !detail.contains("remains planned")
            ),
            "eng.cache completion detail should hide planned-scope tail, got {}",
            cache_completion["detail"]
        );
    }

    #[test]
    fn lsp_keyword_completions_cover_compiler_lexer_keywords() {
        let root = repo_root_for_tests();
        let lexer_source = std::fs::read_to_string(
            root.join("crates")
                .join("eng_compiler")
                .join("src")
                .join("lexer.rs"),
        )
        .expect("compiler lexer source should be readable");
        let lexer_keywords = lexer_source
            .lines()
            .filter_map(|line| {
                let rest = line.trim().strip_prefix('"')?;
                let (keyword, tail) = rest.split_once('"')?;
                tail.trim_start()
                    .starts_with("=> Some(Keyword::")
                    .then_some(keyword.to_owned())
            })
            .collect::<BTreeSet<_>>();
        assert!(
            lexer_keywords.len() > 40,
            "compiler lexer keyword scan should find public keywords"
        );

        let completion_keywords = COMPLETION_KEYWORDS.iter().copied().collect::<BTreeSet<_>>();
        let missing = lexer_keywords
            .iter()
            .filter(|keyword| !completion_keywords.contains(keyword.as_str()))
            .cloned()
            .collect::<Vec<_>>();

        assert!(
            missing.is_empty(),
            "compiler lexer keywords missing from LSP keyword completions: {}",
            missing.join(", ")
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
    fn vscode_semantic_scope_mappings_cover_lsp_legend_selectors() {
        let root = repo_root_for_tests();
        let package = read_json_file(
            &root
                .join("tools")
                .join("vscode-englang")
                .join("package.json"),
        );
        let scope_keys = package_semantic_scope_keys(&package);

        let missing_token_types = SEMANTIC_TOKEN_TYPES
            .iter()
            .filter(|token_type| !scope_keys.contains(**token_type))
            .copied()
            .collect::<Vec<_>>();
        assert!(
            missing_token_types.is_empty(),
            "VS Code semanticTokenScopes missing base mappings for LSP token types: {}",
            missing_token_types.join(", ")
        );

        let missing_modifiers = SEMANTIC_TOKEN_MODIFIERS
            .iter()
            .filter(|modifier| {
                let suffix = format!(".{modifier}");
                !scope_keys
                    .iter()
                    .any(|selector| selector.ends_with(&suffix))
            })
            .copied()
            .collect::<Vec<_>>();
        assert!(
            missing_modifiers.is_empty(),
            "VS Code semanticTokenScopes missing fallback mappings for LSP modifiers: {}",
            missing_modifiers.join(", ")
        );

        let mut unknown_selectors = Vec::new();
        for selector in &scope_keys {
            if let Some((token_type, modifier)) = selector.split_once('.') {
                if !SEMANTIC_TOKEN_TYPES.contains(&token_type)
                    || !SEMANTIC_TOKEN_MODIFIERS.contains(&modifier)
                {
                    unknown_selectors.push(*selector);
                }
            } else if !SEMANTIC_TOKEN_TYPES.contains(selector) {
                unknown_selectors.push(*selector);
            }
        }
        assert!(
            unknown_selectors.is_empty(),
            "VS Code semanticTokenScopes contains selectors outside the LSP legend: {}",
            unknown_selectors.join(", ")
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
    fn vscode_type_semantic_scope_mappings_cover_collection_fallbacks() {
        let root = repo_root_for_tests();
        let package = read_json_file(
            &root
                .join("tools")
                .join("vscode-englang")
                .join("package.json"),
        );
        let required: &[(&str, &[&str])] = &[
            (
                "type",
                &[
                    "support.type.englang",
                    "meta.type.generic.englang",
                    "meta.type.array-suffix.englang",
                    "variable.parameter.type.englang",
                ],
            ),
            (
                "type.quantity",
                &[
                    "support.type.englang",
                    "meta.type.generic.englang",
                    "meta.type.array-suffix.englang",
                    "variable.parameter.type.englang",
                ],
            ),
        ];

        for (selector, fallback_scopes) in required {
            let scopes = package_semantic_scope_values(&package, selector);
            for fallback_scope in *fallback_scopes {
                assert!(
                    scopes.contains(fallback_scope),
                    "semantic token scope mapping {selector} should include fallback {fallback_scope}"
                );
            }
        }
    }

    #[test]
    fn vscode_keyword_semantic_scope_mappings_cover_clause_fallbacks() {
        let root = repo_root_for_tests();
        let package = read_json_file(
            &root
                .join("tools")
                .join("vscode-englang")
                .join("package.json"),
        );
        let required: &[(&str, &[&str])] = &[
            (
                "keyword.defaultLibrary",
                &[
                    "keyword.control.workflow.englang",
                    "keyword.operator.word.englang",
                    "keyword.control.validation.englang",
                    "keyword.control.report.englang",
                    "keyword.control.solver.englang",
                    "constant.language.englang",
                ],
            ),
            (
                "keyword.workflowStep",
                &[
                    "keyword.control.workflow.englang",
                    "keyword.operator.word.englang",
                    "keyword.control.validation.englang",
                    "constant.language.englang",
                    "support.function.builtin.englang",
                ],
            ),
            (
                "keyword.model",
                &[
                    "support.function.builtin.englang",
                    "keyword.control.workflow.englang",
                    "keyword.operator.word.englang",
                    "constant.language.englang",
                ],
            ),
            (
                "keyword.timeseries",
                &[
                    "keyword.control.workflow.englang",
                    "keyword.control.validation.englang",
                    "keyword.operator.word.englang",
                ],
            ),
            (
                "keyword.validation",
                &[
                    "keyword.control.validation.englang",
                    "keyword.operator.word.englang",
                    "constant.language.englang",
                ],
            ),
            (
                "keyword.external",
                &[
                    "keyword.control.external-boundary.englang",
                    "keyword.operator.word.englang",
                ],
            ),
            (
                "keyword.sideEffect",
                &[
                    "keyword.control.side-effect.englang",
                    "keyword.operator.word.englang",
                ],
            ),
            (
                "keyword.cache",
                &[
                    "keyword.control.workflow.englang",
                    "keyword.operator.word.englang",
                    "constant.language.englang",
                ],
            ),
            (
                "keyword.db",
                &[
                    "keyword.control.external-boundary.englang",
                    "keyword.operator.word.englang",
                    "constant.language.englang",
                ],
            ),
            (
                "function.deprecated",
                &[
                    "keyword.control.deprecated.englang",
                    "entity.name.function.call.englang",
                ],
            ),
        ];

        for (selector, fallback_scopes) in required {
            let scopes = package_semantic_scope_values(&package, selector);
            for fallback_scope in *fallback_scopes {
                assert!(
                    scopes.contains(fallback_scope),
                    "semantic token scope mapping {selector} should include fallback {fallback_scope}"
                );
            }
        }
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
        let simulate_bad_options = r#"system SimDecay {
    state T: AbsoluteTemperature = 24 degC
    equation {
        der(T) eq 0 K/s
    }
}

sim = simulate SimDecay
with {
    timestep = never
    duration = forever
    solver = adaptive
    tolerance = zero
}
"#;
        let solve_bad_options = r#"domain Scalar {
    across x: DimensionlessNumber [1]
    through balance: DimensionlessNumber [1]
    conservation sum(balance) = 0
}

component LoopNode {
    port source: Scalar
    port target: Scalar
    source.x eq 0.5 * target.x
    source.balance eq 0
}

system FixedPointLoop {
    loop_node = LoopNode()
    connect loop_node.source to loop_node.target
}

fixed_point_result = solve component_graph
with {
    solver = fixed_point
    tolerance = -1
    max_iter = 0
    relaxation = 2
    initial = bad
}
"#;
        let dynamic_solve_bad_options = r#"domain ScalarState {
    across x: DimensionlessNumber [1]
    through balance: DimensionlessNumber [1]
    conservation sum(balance) = 0
}

component DecayNode {
    port node: ScalarState
    der(node.x) eq -0.5 * node.x
}

system DynamicExplicit {
    node = DecayNode()
    connect node.node to node.node
}

explicit_result = solve component_graph
with {
    solver = dynamic_component_explicit_euler
    timestep = never
    duration = none
    initial = bad
}
"#;
        let newton_solve_bad_options = r#"domain Scalar {
    across x: DimensionlessNumber [1]
    through balance: DimensionlessNumber [1]
    conservation sum(balance) = 0
}

component ResidualNode {
    port node: Scalar
    node.x * node.x eq 2
}

system SourceSolves {
    node = ResidualNode()
    connect node.node to node.node
}

newton_result = solve component_graph
with {
    solver = newton
    finite_difference_step = 0
    damping = 2
    line_search_steps = 0
    jacobian = symbolic
    variable_scale = 0
}

dae_result = solve component_graph
with {
    solver = implicit_euler_dae
    timestep = 1 s
    duration = 2 s
    initial_derivative = bad
    consistency_tolerance = 0
    algebraic_initialization = maybe
    mass_matrix = bad
}
"#;
        let regression_bad_test = r#"designs = sample lhs
with {
    count = 4
    seed = 5
    cooling_cop = uniform(2.5, 5.0)
}

results = derive designs column annual_electricity = 10000 kWh - cooling_cop * 500 kWh
model = train regression results
with {
    target = annual_electricity
    features = [cooling_cop]
    test = 1.5
    seed = 7
}
"#;
        let regression_bad_seed = r#"designs = sample lhs
with {
    count = 4
    seed = 5
    cooling_cop = uniform(2.5, 5.0)
}

results = derive designs column annual_electricity = 10000 kWh - cooling_cop * 500 kWh
model = train regression results
with {
    target = annual_electricity
    features = [cooling_cop]
    test = 0.25
    seed = abc
}
"#;
        let regression_bad_algorithm = r#"designs = sample lhs
with {
    count = 4
    seed = 5
    cooling_cop = uniform(2.5, 5.0)
}

results = derive designs column annual_electricity = 10000 kWh - cooling_cop * 500 kWh
model = train regression results
with {
    target = annual_electricity
    features = [cooling_cop]
    test = 0.25
    seed = 7
    algorithm = tree
}
"#;
        let mlp_bad_hidden = r#"cp = 4180 J/kg/K
Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
split = train_test_split(Q_coil, target=Q_coil, features=[T_supply], test=0.25)
mlp_model = mlp(split)
with {
    hidden = [0]
    epochs = 20
    seed = 7
}
"#;
        let mlp_bad_epochs = r#"cp = 4180 J/kg/K
Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
split = train_test_split(Q_coil, target=Q_coil, features=[T_supply], test=0.25)
mlp_model = mlp(split)
with {
    hidden = [4]
    epochs = 0
    seed = 7
}
"#;
        let mlp_bad_seed = r#"cp = 4180 J/kg/K
Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
split = train_test_split(Q_coil, target=Q_coil, features=[T_supply], test=0.25)
mlp_model = mlp(split)
with {
    hidden = [4]
    epochs = 20
    seed = abc
}
"#;
        let write_bad_format_unit =
            "Q: HeatRate [kW] = 10 kW\nwrite text \"m.txt\", \"metric={Q: .2 m}\"\n";
        let write_bad_format_expression =
            "write text \"missing_value.txt\", \"missing_value={missing_value}\"\n";
        let print_bad_format_unit =
            "Q: HeatRate [kW] = 10 kW\nprint \"metric m before {Q: .2 m}\"\n";
        let print_bad_format_expression = "print \"missing_value={missing_value}\"\n";

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
            (
                "E-LOG-LEVEL-001",
                "log trace \"too noisy\"\n",
                "trace",
            ),
            (
                "E-NET-RETRY-POLICY",
                "response = http get url(\"https://example.org/data.json\")\nwith {\n    retry = many\n}\n",
                "many",
            ),
            (
                "E-NET-TIMEOUT",
                "response = http get url(\"https://example.org/data.json\")\nwith {\n    timeout = never\n}\n",
                "never",
            ),
            (
                "E-NET-BODY-SIZE-LIMIT",
                "download url(\"https://example.org/file.csv\") to file(\"build/raw/file.csv\")\nwith {\n    response_body_limit = 0 B\n}\n",
                "0 B",
            ),
            (
                "E-NET-INVALID-URL",
                "response = http get url(\"ftp://example.org/data.json\")\n",
                "\"ftp://example.org/data.json\"",
            ),
            (
                "E-IO-JSON-FIELD-ACCESS-001",
                "payload = read json file(\"payload.json\")\ncase_name = payload.case\n",
                "payload.case",
            ),
            (
                "W-TABLE-LEGACY-SELECT-FIRST-ROW",
                "selected_station_id = select_first_row(stations, return_column=\"station_id\")\n",
                "select_first_row",
            ),
            (
                "W-NET-FIXTURE-ALIAS",
                "response = http get url(\"https://api.example.org/data.json\")\nwith {\n    fixture = file(\"data/response.json\")\n}\n",
                "fixture",
            ),
            (
                "W-NET-RESPONSE-HASH-ALIAS",
                "response = http get url(\"https://api.example.org/data.json\")\nlegacy_hash = response.hash\n",
                "hash",
            ),
            (
                "W-NET-RESPONSE-STATUS-ALIAS",
                "response = http get url(\"https://api.example.org/data.json\")\nsource = response.status\ncode = response.status_code\n",
                "status",
            ),
            (
                "W-STATS-SUM-001",
                "Q_series: TimeSeries[Time] of HeatRate [kW] = 1 kW\nE_sum = sum(Q_series, over=Time)\n",
                "sum",
            ),
            (
                "E-NET-BODY-METHOD",
                "response = http get url(\"https://example.org/submit\")\nwith {\n    body = \"submitted=true\"\n}\n",
                "\"submitted=true\"",
            ),
            (
                "E-PROCESS-TIMEOUT",
                "process_result = run command \"cmd\"\nwith {\n    timeout = never\n}\n",
                "never",
            ),
            (
                "E-PROCESS-RETRY-POLICY",
                "process_result = run command \"cmd\"\nwith {\n    retry = many\n}\n",
                "many",
            ),
            (
                "E-PROCESS-ALLOW-FAILURE",
                "process_result = run command \"cmd\"\nwith {\n    allow_failure = sometimes\n}\n",
                "sometimes",
            ),
            (
                "E-PROCESS-CWD-001",
                "process_result = run command \"cmd\"\nwith {\n    cwd = true\n}\n",
                "true",
            ),
            (
                "E-PROCESS-ENV-001",
                "process_result = run command \"cmd\"\nwith {\n    env = true\n}\n",
                "true",
            ),
            (
                "E-SAMPLING-COUNT-INVALID",
                "samples = sample lhs\nwith {\n    count = 0\n    seed = 42\n    x = uniform(0, 1)\n}\n",
                "0",
            ),
            (
                "E-SAMPLING-SEED-INVALID",
                "samples = sample lhs\nwith {\n    count = 2\n    seed = later\n    x = uniform(0, 1)\n}\n",
                "later",
            ),
            (
                "E-CACHE-KEY-NONDETERMINISTIC",
                "response = http get url(\"https://example.org/data.json\")\nwith {\n    cache = true\n    cache_key = [now(), \"demo\"]\n}\n",
                "[now(), \"demo\"]",
            ),
            (
                "E-CACHE-DIR",
                "process_result = run command \"cmd\"\nwith {\n    cache = true\n    cache_dir = dir(\"../outside\")\n}\n",
                "dir(\"../outside\")",
            ),
            (
                "E-CACHE-TTL",
                "process_result = run command \"cmd\"\nwith {\n    cache = true\n    cache_ttl = forever\n}\n",
                "forever",
            ),
            ("E-SIM-TIMESTEP-INVALID", simulate_bad_options, "never"),
            ("E-SIM-DURATION-INVALID", simulate_bad_options, "forever"),
            ("E-SIM-TOLERANCE-INVALID", simulate_bad_options, "zero"),
            ("E-SIM-SOLVER-UNSUPPORTED", simulate_bad_options, "adaptive"),
            ("E-SOLVE-TOLERANCE-INVALID", solve_bad_options, "-1"),
            ("E-SOLVE-MAX-ITER-INVALID", solve_bad_options, "0"),
            ("E-SOLVE-RELAXATION-INVALID", solve_bad_options, "2"),
            ("E-SOLVE-INITIAL-INVALID", solve_bad_options, "bad"),
            (
                "E-SOLVE-TIMESTEP-INVALID",
                dynamic_solve_bad_options,
                "never",
            ),
            ("E-SOLVE-DURATION-INVALID", dynamic_solve_bad_options, "none"),
            ("E-SOLVE-FD-STEP-INVALID", newton_solve_bad_options, "0"),
            ("E-SOLVE-DAMPING-INVALID", newton_solve_bad_options, "2"),
            (
                "E-SOLVE-LINE-SEARCH-STEPS-INVALID",
                newton_solve_bad_options,
                "0",
            ),
            (
                "E-SOLVE-JACOBIAN-UNSUPPORTED",
                newton_solve_bad_options,
                "symbolic",
            ),
            (
                "E-SOLVE-VARIABLE-SCALE-INVALID",
                newton_solve_bad_options,
                "0",
            ),
            ("E-SOLVE-MASS-MATRIX-INVALID", newton_solve_bad_options, "bad"),
            (
                "E-SOLVE-INITIAL-INVALID",
                newton_solve_bad_options,
                "bad",
            ),
            (
                "E-SOLVE-CONSISTENCY-TOLERANCE-INVALID",
                newton_solve_bad_options,
                "0",
            ),
            (
                "E-SOLVE-ALGEBRAIC-INITIALIZATION-UNSUPPORTED",
                newton_solve_bad_options,
                "maybe",
            ),
            ("E-ML-ARGS-002", regression_bad_test, "1.5"),
            ("E-ML-ARGS-002", regression_bad_seed, "abc"),
            ("E-ML-ARGS-003", regression_bad_algorithm, "tree"),
            ("E-ML-ARGS-002", mlp_bad_hidden, "[0]"),
            ("E-ML-ARGS-002", mlp_bad_epochs, "0"),
            ("E-ML-ARGS-002", mlp_bad_seed, "abc"),
        ] {
            assert_first_diagnostic_underlines(source, code, expected_text);
        }
        assert_first_diagnostic_underlines_after(
            write_bad_format_unit,
            "E-WRITE-FMT-003",
            "{Q: .2 ",
            "m",
        );
        assert_first_diagnostic_underlines_after(
            write_bad_format_expression,
            "E-WRITE-FMT-004",
            "missing_value={",
            "missing_value",
        );
        assert_first_diagnostic_underlines_after(
            print_bad_format_unit,
            "E-PRINT-FMT-003",
            "{Q: .2 ",
            "m",
        );
        assert_first_diagnostic_underlines_after(
            print_bad_format_expression,
            "E-PRINT-FMT-004",
            "missing_value={",
            "missing_value",
        );
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
    expected_outputs = [file("outputs/result.txt")]
}

args {
    output: DirectoryPath = dir("outputs")
}

export summary to csv join(args.output, "summary.csv") {
    T_measured as degC
}
write text join(args.output, "summary.txt"), "ok"

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
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            r#"    expected_outputs = [file("outputs/result.txt")]"#,
            "file",
            "function",
            "external",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            r#"    expected_outputs = [file("outputs/result.txt")]"#,
            "file",
            "function",
            "sideEffect",
        );
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

process_result = run command "cmd"
with {
    args = ["/C", "echo", "ok"]
    expected_outputs = [file("outputs/result.csv")]
    tool_version = "demo 1.0"
    retry = 1
    timeout = 10 s
    allow_failure = false
    cache = true
    cache_key = ["process", "demo"]
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
Q_mc = propagate(Q_dist, method=linear)
with {
    uncertainty = monte_carlo
    uncertainty = interval
    uncertainty = ensemble
    samples = 64
    seed = 7
}

sim = simulate RoomThermal
with {
    timestep = 60 s
    duration = 1 h
    solver = adaptive_heun
    tolerance = 0.001
}
solve_result = solve component_graph
with {
    jacobian = source_linear_terms
    algebraic_initialization = none
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
    status = pending
    status = planned
    status = partial
    status = running
    status = passed
    status = failed
    status = succeeded
    status = skipped
    status = blocked
    status = completed
    status = rendered
    status = collected
    status = missing
    status = empty
}

on {
    status == pending
    status == planned
    status == passed
    status == partial
    status == running
    status == failed
    status == succeeded
    status == skipped
    status == blocked
    status == completed
    status != empty
    status != rendered
    status != collected
    status != missing
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
    cache = true
    cache_key = ["weather", "demo"]
    cache_dir = dir("build/cache")
    cache_ttl = 30 min
}

write text file("outputs/out.txt"), "ok"
with {
    overwrite = true
}
write json join(dir("outputs"), "metrics.json"), Q_coil
write standard_text sensor to file("outputs/sensor_copy.txt")
"#;
        let snapshot = snapshot_for_source(Path::new("roles.eng"), source);

        for (owner_line, modifier) in [
            ("reg_model = regression(split, algorithm=linear)", "model"),
            (r#"write sensor to db.table("sensor")"#, "db"),
            ("write standard_text sensor", "sideEffect"),
            ("write standard_text sensor", "workflowStep"),
            ("selected = require_one sensor", "validation"),
            (
                "Q_dist = distribution(kind=normal, mean=5 kW, sigma=0.8 kW, n=31)",
                "uncertain",
            ),
            ("sim = simulate RoomThermal", "solver"),
            ("    plot distribution(Q_dist)", "report"),
            ("cases = materialize sensor", "workflowStep"),
            (
                r#"upload = http get url("https://example.org/weather")"#,
                "external",
            ),
            (r#"write text file("outputs/out.txt"), "ok""#, "sideEffect"),
        ] {
            assert_semantic_token_after_line_with_modifier(
                &snapshot, source, owner_line, "with {", "with", "keyword", modifier,
            );
        }
        assert_semantic_token_modifier(&snapshot, source, "reg_model", "model");
        assert_semantic_token_modifier(&snapshot, source, "reg_model", "cache");
        assert_semantic_token_modifier(&snapshot, source, "cache_key", "cache");
        assert_semantic_token_modifier(&snapshot, source, "epochs", "model");
        assert_semantic_token_modifier(&snapshot, source, "db", "db");
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            r#"db = open sqlite file("outputs/results.sqlite")"#,
            "db",
            "variable",
            "external",
        );
        for (label, token_type, modifier) in [
            ("open", "keyword", "sideEffect"),
            ("open", "keyword", "external"),
            ("open", "keyword", "db"),
            ("sqlite", "keyword", "db"),
            ("file", "function", "sideEffect"),
            ("file", "function", "external"),
            ("file", "function", "db"),
        ] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                r#"db = open sqlite file("outputs/results.sqlite")"#,
                label,
                token_type,
                modifier,
            );
        }
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
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            r#"    output_root = dir("outputs/cases")"#,
            "dir",
            "function",
            "workflowStep",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            r#"    output_root = dir("outputs/cases")"#,
            "dir",
            "function",
            "sideEffect",
        );
        assert_semantic_token_modifier(&snapshot, source, "case_id", "workflowStep");
        assert_semantic_token_modifier(&snapshot, source, "resume", "workflowStep");
        for status in [
            "pending",
            "planned",
            "partial",
            "running",
            "passed",
            "failed",
            "succeeded",
            "skipped",
            "blocked",
            "completed",
            "rendered",
            "collected",
            "missing",
            "empty",
        ] {
            let line = format!("    status = {status}");
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                &line,
                "status",
                "property",
                "workflowStep",
            );
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                &line,
                status,
                "keyword",
                "workflowStep",
            );
        }
        assert_semantic_token_modifier(&snapshot, source, "on_none", "validation");
        for (line, status) in [
            ("    status == pending", "pending"),
            ("    status == planned", "planned"),
            ("    status == passed", "passed"),
            ("    status == partial", "partial"),
            ("    status == running", "running"),
            ("    status == failed", "failed"),
            ("    status == succeeded", "succeeded"),
            ("    status == skipped", "skipped"),
            ("    status == blocked", "blocked"),
            ("    status == completed", "completed"),
            ("    status != empty", "empty"),
            ("    status != rendered", "rendered"),
            ("    status != collected", "collected"),
            ("    status != missing", "missing"),
        ] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                line,
                "status",
                "property",
                "workflowStep",
            );
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                line,
                status,
                "keyword",
                "workflowStep",
            );
        }
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
        assert_semantic_token_modifier(&snapshot, source, "uncertainty", "uncertain");
        assert_semantic_token_modifier(&snapshot, source, "monte_carlo", "uncertain");
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "    uncertainty = interval",
            "interval",
            "keyword",
            "uncertain",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "    uncertainty = ensemble",
            "ensemble",
            "keyword",
            "uncertain",
        );
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
        assert_semantic_token_modifier(&snapshot, source, "jacobian", "solver");
        assert_semantic_token_modifier(&snapshot, source, "source_linear_terms", "solver");
        assert_semantic_token_modifier(&snapshot, source, "query", "external");
        assert_semantic_token_modifier(&snapshot, source, "station", "external");
        assert_semantic_token_modifier(&snapshot, source, "offline_response", "external");
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            r#"    offline_response = file("data/weather-response.json")"#,
            "file",
            "function",
            "external",
        );
        assert_semantic_token_modifier(&snapshot, source, "expected_sha256", "external");
        assert_semantic_token_modifier(&snapshot, source, "status_code", "external");
        assert_semantic_token_modifier(&snapshot, source, "body_size_limit", "external");
        for label in ["cache", "cache_key", "cache_dir", "cache_ttl"] {
            assert_semantic_token_after_line_with_modifier(
                &snapshot,
                source,
                r#"upload = http get url("https://example.org/weather")"#,
                &format!("    {label} ="),
                label,
                "property",
                "cache",
            );
            assert_semantic_token_after_line_with_modifier(
                &snapshot,
                source,
                r#"upload = http get url("https://example.org/weather")"#,
                &format!("    {label} ="),
                label,
                "property",
                "external",
            );
        }
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            r#"    cache_dir = dir("build/cache")"#,
            "dir",
            "function",
            "cache",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            r#"    cache_dir = dir("build/cache")"#,
            "dir",
            "function",
            "external",
        );
        for label in [
            "args",
            "expected_outputs",
            "tool_version",
            "retry",
            "timeout",
            "allow_failure",
        ] {
            assert_semantic_token_after_line_with_modifier(
                &snapshot,
                source,
                r#"process_result = run command "cmd""#,
                &format!("    {label} ="),
                label,
                "property",
                "sideEffect",
            );
            assert_semantic_token_after_line_with_modifier(
                &snapshot,
                source,
                r#"process_result = run command "cmd""#,
                &format!("    {label} ="),
                label,
                "property",
                "external",
            );
        }
        for label in ["cache", "cache_key"] {
            assert_semantic_token_after_line_with_modifier(
                &snapshot,
                source,
                r#"process_result = run command "cmd""#,
                &format!("    {label} ="),
                label,
                "property",
                "cache",
            );
            assert_semantic_token_after_line_with_modifier(
                &snapshot,
                source,
                r#"process_result = run command "cmd""#,
                &format!("    {label} ="),
                label,
                "property",
                "sideEffect",
            );
            assert_semantic_token_after_line_with_modifier(
                &snapshot,
                source,
                r#"process_result = run command "cmd""#,
                &format!("    {label} ="),
                label,
                "property",
                "external",
            );
        }
        for (owner_line, expected_modifier) in [
            (
                r#"upload = http get url("https://example.org/weather")"#,
                "external",
            ),
            (r#"process_result = run command "cmd""#, "sideEffect"),
        ] {
            assert_semantic_token_after_line_with_modifier(
                &snapshot,
                source,
                owner_line,
                "    cache = true",
                "true",
                "keyword",
                "cache",
            );
            assert_semantic_token_after_line_with_modifier(
                &snapshot,
                source,
                owner_line,
                "    cache = true",
                "true",
                "keyword",
                expected_modifier,
            );
        }
        assert_semantic_token_after_line_with_modifier(
            &snapshot,
            source,
            r#"process_result = run command "cmd""#,
            "    cache = true",
            "true",
            "keyword",
            "external",
        );
        assert_semantic_token_modifier(&snapshot, source, "overwrite", "sideEffect");
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            r#"write text file("outputs/out.txt"), "ok""#,
            "file",
            "function",
            "sideEffect",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            r#"write json join(dir("outputs"), "metrics.json"), Q_coil"#,
            "join",
            "function",
            "sideEffect",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            r#"write json join(dir("outputs"), "metrics.json"), Q_coil"#,
            "dir",
            "function",
            "sideEffect",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            r#"write standard_text sensor to file("outputs/sensor_copy.txt")"#,
            "file",
            "function",
            "sideEffect",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            r#"write standard_text sensor to file("outputs/sensor_copy.txt")"#,
            "file",
            "function",
            "workflowStep",
        );
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
notes = read text file("notes.txt")
choice = if true else false
Q_series: TimeSeries[Time] of HeatRate [kW] = 5 kW
Q_total_unc: TimeSeries[Time] of HeatRate [kW] = 1 kW
E_series = integrate Q_series over Time
mean_Q = mean Q_series over Time
reg_eval = payload
report {
    summarize Q_series by [mean, time_weighted_mean, p90, p95, duration_above(5 kW)]
    plot Q_series over Time
    plot Q_series and Q_total_unc over Time
    plot histogram(Q_series)
    plot line(Q_series)
    plot bar(Q_series)
    plot parity(reg_eval)
    plot residuals(reg_eval)
}
schema JoinRow {
    id: String
}
left = promote csv file("left.csv") as JoinRow
right = promote csv file("right.csv") as JoinRow
joined = join left with right
on {
    left.id == right.id
}
export summary to csv join(dir("exports"), "summary.csv") {
    T as degC
}
write text "summary.txt", "ok"
copy file("data/template.txt") to "ops/copied_note.txt"
mkdir "ops/archive"
move "ops/copied_note.txt" to "ops/archive/copied_note.txt"
delete dir("ops/tmp")

test "temperature stays bounded" {
    assert T matches T within 1 K
}

render template file("report.md") to file("report.html")
rendered_template = render template args.template_source to args.rendered_output
with {
    template = file("report.md")
    output = file("report.html")
    missing = error
    artifact_kind = "rendered_report"
}
response = http get url("https://example.org/weather")
payload_from_response = read json response.body
submitted = http post url("https://example.org/weather")
updated = http put url("https://example.org/weather")
patched = http patch url("https://example.org/weather")
probed = http head url("https://example.org/weather")
raw_request = http request url("https://example.org/weather")
fetched = http fetch url("https://example.org/weather")
download url("https://example.org/file.csv") to file("outputs/file.csv")
log debug "debug details"
log info "ready"
log warn "slow"
log error "failed"
print "quick status"
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
uncertainty_policy = monte_carlo
jacobian_policy = source_linear_terms

script LegacyScript
struct LegacyArgs
"#;
        let snapshot = snapshot_for_source(Path::new("keyword_modifiers.eng"), source);

        for label in ["simulate", "equation", "der"] {
            assert_semantic_token_modifier(&snapshot, source, label, "solver");
        }
        for label in ["summarize", "summary", "distribution", "line", "bar"] {
            assert_semantic_token_modifier(&snapshot, source, label, "report");
        }
        for line in [
            "log debug",
            "log info",
            "log warn",
            "log error",
            "print \"quick status\"",
        ] {
            let label = if line.starts_with("print") { "print" } else { "log" };
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                line,
                label,
                "keyword",
                "sideEffect",
            );
            assert_semantic_token_on_line_without_modifier(
                &snapshot,
                source,
                line,
                label,
                "keyword",
                "report",
            );
        }
        for (line, level) in [
            ("log debug", "debug"),
            ("log info", "info"),
            ("log warn", "warn"),
            ("log error", "error"),
        ] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                line,
                level,
                "keyword",
                "sideEffect",
            );
        }
        let summary_line =
            "    summarize Q_series by [mean, time_weighted_mean, p90, p95, duration_above(5 kW)]";
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            summary_line,
            "by",
            "keyword",
            "report",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            summary_line,
            "Q_series",
            "variable",
            "report",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            summary_line,
            "Q_series",
            "variable",
            "timeseries",
        );
        for label in ["mean", "time_weighted_mean", "p90", "p95", "duration_above"] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                "    summarize Q_series by [mean, time_weighted_mean, p90, p95, duration_above(5 kW)]",
                label,
                "function",
                "report",
            );
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                "    summarize Q_series by [mean, time_weighted_mean, p90, p95, duration_above(5 kW)]",
                label,
                "function",
                "timeseries",
            );
        }
        let summary_line_index = source
            .lines()
            .position(|line| line.contains(summary_line))
            .expect("summary source line should be present");
        for modifier in ["report", "timeseries"] {
            assert!(
                !snapshot.semantic_tokens.tokens.iter().any(|token| {
                    token.line == summary_line_index
                        && token.token_type == "function"
                        && source.lines().nth(token.line).is_some_and(|line| {
                            line.get(token.start..token.start + token.length) == Some("kW")
                                && token.modifiers.iter().any(|item| item == modifier)
                        })
                }),
                "duration statistic unit `kW` should not be a report statistic function"
            );
        }

        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "E_series = integrate Q_series over Time",
            "over",
            "keyword",
            "solver",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "E_series = integrate Q_series over Time",
            "over",
            "keyword",
            "timeseries",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "E_series = integrate Q_series over Time",
            "Q_series",
            "variable",
            "timeseries",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "mean_Q = mean Q_series over Time",
            "over",
            "keyword",
            "report",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "mean_Q = mean Q_series over Time",
            "over",
            "keyword",
            "timeseries",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "    plot Q_series over Time",
            "over",
            "keyword",
            "report",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "    plot line T vs Time",
            "vs",
            "keyword",
            "report",
        );
        for label in ["Q_series", "Q_total_unc"] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                "    plot Q_series and Q_total_unc over Time",
                label,
                "variable",
                "report",
            );
        }
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "    plot Q_series and Q_total_unc over Time",
            "and",
            "keyword",
            "report",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "    plot Q_series and Q_total_unc over Time",
            "over",
            "keyword",
            "report",
        );
        for label in ["histogram", "line", "bar"] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                &format!("    plot {label}(Q_series)"),
                label,
                "function",
                "report",
            );
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                &format!("    plot {label}(Q_series)"),
                "Q_series",
                "variable",
                "report",
            );
        }
        for label in ["parity", "residuals"] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                &format!("    plot {label}(reg_eval)"),
                label,
                "function",
                "report",
            );
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                &format!("    plot {label}(reg_eval)"),
                "reg_eval",
                "variable",
                "report",
            );
        }
        for label in ["else", "of", "on", "output", "vs"] {
            assert_semantic_token_type(&snapshot, source, label, "keyword");
        }
        for label in ["read", "csv", "json", "toml", "text"] {
            assert_semantic_token_modifier(&snapshot, source, label, "workflowStep");
        }
        for (line, label) in [
            (r#"payload = read json file("payload.json")"#, "read"),
            (r#"payload = read json file("payload.json")"#, "json"),
            (r#"settings = read toml file("settings.toml")"#, "read"),
            (r#"settings = read toml file("settings.toml")"#, "toml"),
            (r#"notes = read text file("notes.txt")"#, "read"),
            (r#"notes = read text file("notes.txt")"#, "text"),
            ("payload_from_response = read json response.body", "read"),
            ("payload_from_response = read json response.body", "json"),
        ] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot, source, line, label, "keyword", "external",
            );
        }
        for (line, label, token_type) in [
            (
                r#"payload = read json file("payload.json")"#,
                "payload",
                "variable",
            ),
            (
                r#"payload = read json file("payload.json")"#,
                "file",
                "function",
            ),
            (
                r#"settings = read toml file("settings.toml")"#,
                "file",
                "function",
            ),
            (
                "payload_from_response = read json response.body",
                "payload_from_response",
                "variable",
            ),
            (
                "payload_from_response = read json response.body",
                "response",
                "variable",
            ),
            (
                "payload_from_response = read json response.body",
                "body",
                "property",
            ),
        ] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                line,
                label,
                token_type,
                "workflowStep",
            );
            assert_semantic_token_on_line_with_modifier(
                &snapshot, source, line, label, token_type, "external",
            );
        }
        for label in ["assert", "matches", "within"] {
            assert_semantic_token_modifier(&snapshot, source, label, "validation");
        }
        for label in [
            "export", "write", "copy", "mkdir", "move", "delete", "render", "template", "download",
        ] {
            assert_semantic_token_modifier(&snapshot, source, label, "sideEffect");
        }
        for (label, token_type) in [
            ("to", "keyword"),
            ("csv", "keyword"),
            ("join", "function"),
            ("dir", "function"),
        ] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                r#"export summary to csv join(dir("exports"), "summary.csv") {"#,
                label,
                token_type,
                "sideEffect",
            );
        }
        for label in ["render", "template"] {
            assert_semantic_token_modifier(&snapshot, source, label, "workflowStep");
        }
        for label in [
            "copy", "mkdir", "move", "delete", "http", "get", "post", "put", "patch", "head",
            "request", "fetch", "download",
        ] {
            assert_semantic_token_modifier(&snapshot, source, label, "external");
        }
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "render template file(\"report.md\") to file(\"report.html\")",
            "to",
            "keyword",
            "sideEffect",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "render template file(\"report.md\") to file(\"report.html\")",
            "to",
            "keyword",
            "workflowStep",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "rendered_template = render template args.template_source to args.rendered_output",
            "template_source",
            "property",
            "workflowStep",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "rendered_template = render template args.template_source to args.rendered_output",
            "rendered_output",
            "property",
            "sideEffect",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "    template = file(\"report.md\")",
            "file",
            "function",
            "workflowStep",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "    output = file(\"report.html\")",
            "file",
            "function",
            "sideEffect",
        );
        for line in [
            "copy file(\"data/template.txt\") to \"ops/copied_note.txt\"",
            "move \"ops/copied_note.txt\" to \"ops/archive/copied_note.txt\"",
        ] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                line,
                "to",
                "keyword",
                "sideEffect",
            );
            assert_semantic_token_on_line_with_modifier(
                &snapshot, source, line, "to", "keyword", "external",
            );
        }
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "copy file(\"data/template.txt\") to \"ops/copied_note.txt\"",
            "file",
            "function",
            "sideEffect",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "copy file(\"data/template.txt\") to \"ops/copied_note.txt\"",
            "file",
            "function",
            "external",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "delete dir(\"ops/tmp\")",
            "dir",
            "function",
            "sideEffect",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "delete dir(\"ops/tmp\")",
            "dir",
            "function",
            "external",
        );
        for line in [
            "response = http get url(\"https://example.org/weather\")",
            "submitted = http post url(\"https://example.org/weather\")",
            "updated = http put url(\"https://example.org/weather\")",
            "patched = http patch url(\"https://example.org/weather\")",
            "probed = http head url(\"https://example.org/weather\")",
            "raw_request = http request url(\"https://example.org/weather\")",
            "fetched = http fetch url(\"https://example.org/weather\")",
        ] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                line,
                "url",
                "function",
                "sideEffect",
            );
            assert_semantic_token_on_line_with_modifier(
                &snapshot, source, line, "url", "function", "external",
            );
        }
        for (line, method) in [
            (
                "updated = http put url(\"https://example.org/weather\")",
                "put",
            ),
            (
                "patched = http patch url(\"https://example.org/weather\")",
                "patch",
            ),
            (
                "probed = http head url(\"https://example.org/weather\")",
                "head",
            ),
            (
                "raw_request = http request url(\"https://example.org/weather\")",
                "request",
            ),
            (
                "fetched = http fetch url(\"https://example.org/weather\")",
                "fetch",
            ),
        ] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                line,
                method,
                "keyword",
                "sideEffect",
            );
            assert_semantic_token_on_line_with_modifier(
                &snapshot, source, line, method, "keyword", "external",
            );
        }
        for (label, token_type) in [("to", "keyword"), ("url", "function"), ("file", "function")] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                "download url(\"https://example.org/file.csv\") to file(\"outputs/file.csv\")",
                label,
                token_type,
                "sideEffect",
            );
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                "download url(\"https://example.org/file.csv\") to file(\"outputs/file.csv\")",
                label,
                token_type,
                "external",
            );
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
            "monte_carlo",
            "source_linear_terms",
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
        assert_semantic_token_modifier(&snapshot, source, "source_linear_terms", "solver");
        assert_semantic_token_modifier(&snapshot, source, "monte_carlo", "uncertain");
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
    records: Array[WeatherApiRecord]
    tags: List[String]
    flags: Bool[]
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
            "Array",
            "WeatherApiRecord",
            "List",
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
    fn snapshot_marks_lexical_strings_and_symbol_operators_as_semantic_tokens() {
        let source = r#"print "flow {Q: .2 kW} status={coverage.status} arg={args.region}"
ratio = Q / cp_water
specific = 4180 J/kg/K
irradiance = 850 W/m^2
density = 0.12 people/m2
max_gap = 10 min
min_value = min(Q_series)
valid = Q >= 0 kW and Q != 1 kW
operator A: LinearOperator[RoomState -> Derivative[RoomState]] = [[-0.012 1/min]]
"#;
        let snapshot = snapshot_for_source(Path::new("lexical_tokens.eng"), source);

        assert_semantic_token_type(
            &snapshot,
            source,
            "\"flow {Q: .2 kW} status={coverage.status} arg={args.region}\"",
            "string",
        );
        for (label, token_type) in [
            ("Q", "variable"),
            ("2", "number"),
            ("coverage", "variable"),
            ("status", "property"),
            ("args", "parameter"),
            ("region", "property"),
        ] {
            assert_semantic_token_on_line_type(
                &snapshot,
                source,
                "print \"flow",
                label,
                token_type,
            );
        }
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "print \"flow",
            "kW",
            "type",
            "unit",
        );
        for (line, label) in [
            ("ratio = Q / cp_water", "/"),
            ("valid = Q >= 0 kW and Q != 1 kW", "="),
            ("valid = Q >= 0 kW and Q != 1 kW", ">="),
            ("valid = Q >= 0 kW and Q != 1 kW", "!="),
            (
                "operator A: LinearOperator[RoomState -> Derivative[RoomState]]",
                "->",
            ),
            (
                "operator A: LinearOperator[RoomState -> Derivative[RoomState]]",
                "-",
            ),
        ] {
            assert_semantic_token_on_line_type(&snapshot, source, line, label, "operator");
        }

        for (line, label) in [
            ("specific =", "J/kg/K"),
            ("irradiance =", "W/m^2"),
            ("density =", "people/m2"),
            ("max_gap =", "min"),
        ] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot, source, line, label, "type", "unit",
            );
        }

        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "min_value =",
            "min",
            "function",
            "report",
        );
        for line in ["max_gap =", "operator A:"] {
            let line_index = source
                .lines()
                .position(|candidate| candidate.contains(line))
                .unwrap_or_else(|| panic!("source line `{line}` should be present"));
            assert!(
                !snapshot.semantic_tokens.tokens.iter().any(|token| {
                    token.line == line_index
                        && token.token_type == "function"
                        && source.lines().nth(token.line).is_some_and(|line| {
                            line.get(token.start..token.start + token.length) == Some("min")
                        })
                }),
                "unit token `min` on `{line}` must not also be marked as a builtin function"
            );
        }

        let unit_line = source
            .lines()
            .position(|line| line.contains("specific ="))
            .expect("unit line should be present");
        assert!(
            !snapshot.semantic_tokens.tokens.iter().any(|token| {
                token.line == unit_line
                    && token.token_type == "operator"
                    && source.lines().nth(token.line).is_some_and(|line| {
                        line.get(token.start..token.start + token.length) == Some("/")
                    })
            }),
            "semantic operator scan should not split slash-delimited unit tokens"
        );
    }

    #[test]
    fn snapshot_semantic_tokens_cover_generated_syntax_catalog_literals() {
        let metadata = editor_metadata_json();
        let catalog = &metadata["syntax_catalog"];
        let mut keyword_labels = BTreeSet::<String>::new();
        let keyword_groups = catalog["keyword_groups"]
            .as_object()
            .expect("syntax catalog keyword groups should be an object");
        for labels in keyword_groups.values() {
            for label in labels
                .as_array()
                .expect("keyword group should contain labels")
            {
                keyword_labels.insert(
                    label
                        .as_str()
                        .expect("keyword group label should be a string")
                        .to_owned(),
                );
            }
        }
        for label in catalog["operator_words"]
            .as_array()
            .expect("syntax catalog operator words should be an array")
        {
            keyword_labels.insert(
                label
                    .as_str()
                    .expect("operator word label should be a string")
                    .to_owned(),
            );
        }

        let constant_labels = catalog["constants"]
            .as_array()
            .expect("syntax catalog constants should be an array")
            .iter()
            .map(|label| {
                label
                    .as_str()
                    .expect("constant label should be a string")
                    .to_owned()
            })
            .collect::<BTreeSet<_>>();

        let mut rows = Vec::<(String, String, bool)>::new();
        for label in keyword_labels {
            rows.push((
                format!("catalog_keyword_{} = {}", rows.len(), label),
                label,
                false,
            ));
        }
        for label in constant_labels {
            rows.push((
                format!("catalog_constant_{} = {}", rows.len(), label),
                label,
                true,
            ));
        }
        let source = rows
            .iter()
            .map(|(line, _, _)| line.as_str())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        let snapshot =
            snapshot_for_source(Path::new("syntax_catalog_semantic_tokens.eng"), &source);

        for (line, label, is_constant) in rows {
            assert!(
                snapshot.semantic_tokens.tokens.iter().any(|token| {
                    source
                        .lines()
                        .nth(token.line)
                        .is_some_and(|candidate| {
                            candidate.contains(&line)
                                && candidate.get(token.start..token.start + token.length)
                                    == Some(label.as_str())
                        })
                }),
                "generated syntax catalog label `{label}` should produce a semantic token on `{line}`"
            );
            if is_constant {
                assert_semantic_token_on_line_type(&snapshot, &source, &line, &label, "keyword");
            }
        }
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            &source,
            " = empty",
            "empty",
            "keyword",
            "workflowStep",
        );
    }

    #[test]
    fn snapshot_marks_distribution_kind_literals_as_uncertain_semantic_tokens() {
        let source = r#"Q_direct = normal(mean=5 kW, std=0.8 kW, samples=31)
Q_dist = distribution(kind=normal, mean=5 kW, std=0.8 kW, samples=31)
Q_range = distribution(kind=uniform, lower=1 kW, upper=8 kW, samples=11)
designs = sample uniform
with {
    count = 2
    seed = 7
    load = uniform(1 kW, 2 kW)
}
"#;
        let snapshot = snapshot_for_source(Path::new("distribution_kind_literals.eng"), source);

        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "Q_direct = normal",
            "normal",
            "function",
            "uncertain",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "kind=normal",
            "normal",
            "keyword",
            "uncertain",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "kind=uniform",
            "uniform",
            "keyword",
            "uncertain",
        );
        assert_semantic_token_on_line_without_modifier(
            &snapshot,
            source,
            "kind=uniform",
            "uniform",
            "keyword",
            "workflowStep",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "designs = sample uniform",
            "uniform",
            "keyword",
            "workflowStep",
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
    parameter gain: DimensionlessNumber = 1
    output y: OutputVector[RoomOutput]
    operator A: LinearOperator[RoomState -> Derivative[RoomState]] = [[-0.012 1/min]]
}
"#;
        let snapshot = snapshot_for_source(Path::new("state_space_types.eng"), source);

        for label in ["states", "inputs", "outputs", "operator"] {
            assert_semantic_token_modifier(&snapshot, source, label, "solver");
        }
        assert_semantic_token_on_line_with_modifier(
            &snapshot, source, "state x:", "state", "keyword", "state",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot, source, "input u:", "input", "keyword", "input",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "parameter gain:",
            "parameter",
            "keyword",
            "declaration",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "output y:",
            "output",
            "keyword",
            "output",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot, source, "state x:", "x", "variable", "state",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot, source, "input u:", "u", "variable", "input",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "parameter gain:",
            "gain",
            "parameter",
            "readonly",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "output y:",
            "y",
            "variable",
            "output",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "operator A:",
            "A",
            "variable",
            "solver",
        );

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
    fn snapshot_marks_native_workflow_builtins_as_semantic_tokens() {
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
model = train regression from designs
with {
    y = annual_electricity
    x = [people_density]
    test_fraction = 0.25
    seed = 9
}
model_on = train regression on designs
ann_model = ann(split_alias, hidden=[8], epochs=10)
metrics = evaluate(surrogate)
card = model_card(surrogate)
predictions = predict surrogate using designs
filtered = filter designs
selected = select designs columns people_density, cooling_cop
sorted = sort designs by cooling_cop desc
sorted_asc = sort designs by people_density asc
joined = join designs with predictions
derived = derive designs column annual_electricity = people_density * 1 kWh
derived_many = derive designs columns annual_cooling = cooling_cop * 1 kWh
cases = materialize cases designs
case_results = apply run_case over designs
case_inputs = apply case_input_template over cases
collected = collect results case_results
coverage = check coverage designs.cooling_cop
with {
    expected_step = 1 h
    year = 2024
    start = 0 h
    end = 8760 h
    max_gap = 3 h
    missing = error
}
filled = fill missing designs.cooling_cop
aligned = align designs.cooling_cop with predictions.cooling_cop
resampled = resample designs.cooling_cop to predictions.cooling_cop
resampled_by = resample designs.cooling_cop by 30 min
legacy_station = select_first_row(stations, return_column="station_id")
"#;
        let snapshot = snapshot_for_source(Path::new("native_workflow_builtins.eng"), source);

        assert!(
            !snapshot
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-WITH-OPTION-001"),
            "coverage with-block options should match LSP completion labels"
        );
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
            "predict",
            "derive",
            "check",
            "coverage",
            "fill",
            "align",
            "resample",
        ] {
            assert_semantic_token_type(&snapshot, source, label, "keyword");
            assert_semantic_token_modifier(&snapshot, source, label, "defaultLibrary");
        }
        for label in [
            "uniform",
            "ann",
            "evaluate",
            "model_card",
            "select_first_row",
        ] {
            assert_semantic_token_type(&snapshot, source, label, "function");
            assert_semantic_token_modifier(&snapshot, source, label, "defaultLibrary");
        }
        for label in ["run_case", "case_input_template"] {
            assert_semantic_token_type(&snapshot, source, label, "function");
            assert_semantic_token_modifier(&snapshot, source, label, "workflowStep");
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
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "model = train regression from designs",
            "model",
            "variable",
            "model",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "model = train regression from designs",
            "from",
            "keyword",
            "model",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "model_on = train regression on designs",
            "on",
            "keyword",
            "model",
        );
        for (line, label) in [
            ("    y = annual_electricity", "y"),
            ("    x = [people_density]", "x"),
            ("    test_fraction = 0.25", "test_fraction"),
            ("    seed = 9", "seed"),
        ] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot, source, line, label, "property", "model",
            );
        }
        for (line, label) in [
            ("    y = annual_electricity", "annual_electricity"),
            ("    x = [people_density]", "people_density"),
        ] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot, source, line, label, "property", "model",
            );
        }
        assert_semantic_token_modifier(&snapshot, source, "using", "model");
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "sorted = sort designs by cooling_cop desc",
            "by",
            "keyword",
            "workflowStep",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "joined = join designs with predictions",
            "with",
            "keyword",
            "workflowStep",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "case_results = apply run_case over designs",
            "over",
            "keyword",
            "workflowStep",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "aligned = align designs.cooling_cop with predictions.cooling_cop",
            "with",
            "keyword",
            "workflowStep",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "resampled = resample designs.cooling_cop to predictions.cooling_cop",
            "to",
            "keyword",
            "workflowStep",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "resampled_by = resample designs.cooling_cop by 30 min",
            "by",
            "keyword",
            "workflowStep",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "resampled_by = resample designs.cooling_cop by 30 min",
            "by",
            "keyword",
            "timeseries",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "resampled_by = resample designs.cooling_cop by 30 min",
            "by",
            "keyword",
            "validation",
        );
        let resampled_by_line = source
            .lines()
            .position(|line| line.contains("resampled_by = resample designs.cooling_cop by 30 min"))
            .expect("resampled_by source line should be present");
        for modifier in ["validation", "workflowStep", "timeseries"] {
            assert!(
                !snapshot.semantic_tokens.tokens.iter().any(|token| {
                    token.line == resampled_by_line
                        && source.lines().nth(token.line).is_some_and(|line| {
                            line.get(token.start..token.start + token.length) == Some("min")
                                && token.modifiers.iter().any(|item| item == modifier)
                        })
                }),
                "duration unit `min` should not inherit command modifier `{modifier}`"
            );
        }
        assert_eq!(
            semantic_token_modifier_count(&snapshot, source, "surrogate", "variable", "model"),
            4,
            "model declaration plus evaluate/model_card/predict references should be model tokens"
        );
        assert_eq!(
            semantic_token_modifier_count(&snapshot, source, "designs", "variable", "model"),
            4,
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
            "asc",
            "desc",
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
            "predict",
            "derive",
            "check",
            "coverage",
            "fill",
            "align",
            "resample",
        ] {
            assert_semantic_token_modifier(&snapshot, source, label, "workflowStep");
        }
        assert_semantic_token_modifier(&snapshot, source, "fill", "validation");
        assert_semantic_token_modifier(&snapshot, source, "check", "validation");
        assert_semantic_token_modifier(&snapshot, source, "coverage", "validation");
        assert_semantic_token_modifier(&snapshot, source, "missing", "validation");
        for label in ["check", "coverage", "fill", "align", "resample"] {
            assert_semantic_token_modifier(&snapshot, source, label, "timeseries");
        }
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "filled = fill missing designs.cooling_cop",
            "missing",
            "keyword",
            "workflowStep",
        );
        for label in ["asc", "desc"] {
            assert_semantic_token_type(&snapshot, source, label, "keyword");
        }
        for label in ["designs", "case_row", "derived", "derived_many", "coverage"] {
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
        for line in [
            "coverage = check coverage designs.cooling_cop",
            "filled = fill missing designs.cooling_cop",
            "aligned = align designs.cooling_cop with predictions.cooling_cop",
            "resampled = resample designs.cooling_cop to predictions.cooling_cop",
            "resampled_by = resample designs.cooling_cop by 30 min",
        ] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                line,
                "designs",
                "variable",
                "workflowStep",
            );
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                line,
                "cooling_cop",
                "property",
                "workflowStep",
            );
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                line,
                "cooling_cop",
                "property",
                "validation",
            );
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                line,
                "cooling_cop",
                "property",
                "timeseries",
            );
        }
        for (line, label) in [
            ("    expected_step = 1 h", "expected_step"),
            ("    year = 2024", "year"),
            ("    start = 0 h", "start"),
            ("    end = 8760 h", "end"),
            ("    max_gap = 3 h", "max_gap"),
            ("    missing = error", "missing"),
        ] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                line,
                label,
                "property",
                "validation",
            );
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                line,
                label,
                "property",
                "workflowStep",
            );
        }
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "    missing = error",
            "error",
            "keyword",
            "validation",
        );
        for line in [
            "aligned = align designs.cooling_cop with predictions.cooling_cop",
            "resampled = resample designs.cooling_cop to predictions.cooling_cop",
        ] {
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                line,
                "predictions",
                "variable",
                "workflowStep",
            );
            assert_semantic_token_on_line_with_modifier(
                &snapshot,
                source,
                line,
                "predictions",
                "variable",
                "timeseries",
            );
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
rows = promote csv file("data/weather.csv") as WeatherApiRecord
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
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "api_contract = promote json payload as WeatherApiPayload",
            "as",
            "keyword",
            "workflowStep",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "api_contract = promote json payload as WeatherApiPayload",
            "payload",
            "variable",
            "external",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "weather = promote json records payload.records as WeatherApiRecord",
            "as",
            "keyword",
            "workflowStep",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "weather = promote json records payload.records as WeatherApiRecord",
            "payload",
            "variable",
            "external",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "weather = promote json records payload.records as WeatherApiRecord",
            "records",
            "property",
            "external",
        );
        assert_semantic_token_on_line_with_modifier(
            &snapshot,
            source,
            "rows = promote csv file(\"data/weather.csv\") as WeatherApiRecord",
            "file",
            "function",
            "external",
        );
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
            "HTTP response body text from live, cached, or pinned response"
        );
        assert!(member_completions
            .iter()
            .any(|completion| completion.label == "response_source"));
        assert!(!member_completions
            .iter()
            .any(|completion| completion.label == "status"));
        assert!(member_completions
            .iter()
            .any(|completion| completion.label == "status_code"));
        assert!(member_completions
            .iter()
            .any(|completion| completion.label == "method"));
        assert!(member_completions
            .iter()
            .any(|completion| completion.label == "query_string"));
        assert!(member_completions
            .iter()
            .any(|completion| completion.label == "url_with_query"));
        assert!(!member_completions
            .iter()
            .any(|completion| completion.label == "hash"));
        let response_text_hover = snapshot
            .hovers
            .iter()
            .find(|hover| hover.name == "response_text")
            .expect("HTTP response string binding should have hover detail");
        let response_text_markdown = hover_json(response_text_hover)["contents"]["value"]
            .as_str()
            .expect("HTTP response string hover should render markdown")
            .to_owned();
        assert!(response_text_markdown.contains("Quantity: `String`"));
        assert!(!response_text_markdown.contains("Display unit: ``"));
        assert!(!response_text_markdown.contains("Display unit: `-`"));
        assert!(snapshot.semantic_tokens.tokens.iter().any(|token| {
            token.token_type == "property"
                && source
                    .lines()
                    .nth(token.line)
                    .is_some_and(|line| &line[token.start..token.start + token.length] == "body")
        }));
    }

    #[test]
    fn snapshot_exposes_sample_table_member_fields() {
        let source = "samples = sample lhs\nwith {\n    count = 4\n    seed = 42\n    cooling_cop = uniform(2.5, 5.0)\n}\n\nsample_count = samples.sample_count\nrow_preview = samples.row_preview\nnested_row_preview = study.samples.row_preview\n";
        let snapshot = snapshot_for_source(Path::new("sample_members.eng"), source);

        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "samples.sample_count"));
        let line = source
            .lines()
            .position(|line| line.contains("sample_count ="))
            .expect("sample_count line");
        let member_completions = completion_items_for_source_position(
            Path::new("sample_members.eng"),
            source,
            line,
            "sample_count = samples.".len(),
        );
        let count_completion = member_completions
            .iter()
            .find(|completion| completion.label == "sample_count")
            .expect("sample table member completion should include sample_count");
        assert_eq!(count_completion.detail, "generated sample row count");
        assert!(member_completions
            .iter()
            .any(|completion| completion.label == "method"));
        assert!(member_completions
            .iter()
            .any(|completion| completion.label == "seed"));
        let row_preview_completion = member_completions
            .iter()
            .find(|completion| completion.label == "row_preview")
            .expect("sample table member completion should include row_preview");
        assert_eq!(row_preview_completion.detail, "sample row preview summary");
        let nested_line = source
            .lines()
            .position(|line| line.contains("nested_row_preview"))
            .expect("nested_row_preview line");
        let nested_member_completions = completion_items_for_source_position(
            Path::new("sample_members.eng"),
            source,
            nested_line,
            "nested_row_preview = study.samples.".len(),
        );
        assert!(nested_member_completions
            .iter()
            .any(|completion| completion.label == "row_preview"));
        let nested_hover = snapshot
            .hovers
            .iter()
            .find(|hover| hover.name == "study.samples.row_preview")
            .expect("nested sample member field should expose exact hover metadata");
        assert_eq!(nested_hover.kind, "sample_table_field");
        assert_eq!(nested_hover.quantity_kind, "String");
        assert!(nested_hover.detail.contains("sample row preview summary"));
        assert_semantic_token_on_line(
            &snapshot,
            source,
            "sample_count =",
            "sample_count",
            "property",
            "workflowStep",
        );
        assert_semantic_token_on_line(
            &snapshot,
            source,
            "row_preview =",
            "row_preview",
            "property",
            "workflowStep",
        );
        assert_semantic_token_on_line(
            &snapshot,
            source,
            "nested_row_preview =",
            "row_preview",
            "property",
            "workflowStep",
        );
    }

    #[test]
    fn snapshot_exposes_db_connection_member_fields() {
        let source = "db = open sqlite file(\"outputs/results.sqlite\")\ndb_summary = db.summary\ndb_tables = db.tables_written\ndb_count = db.table_count\n";
        let snapshot = snapshot_for_source(Path::new("db_members.eng"), source);

        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "db.summary"));
        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "db.tables_written"));
        let line = source
            .lines()
            .position(|line| line.contains("db_tables ="))
            .expect("db_tables line");
        let member_completions = completion_items_for_source_position(
            Path::new("db_members.eng"),
            source,
            line,
            "db_tables = db.".len(),
        );
        let tables_completion = member_completions
            .iter()
            .find(|completion| completion.label == "tables_written")
            .expect("DB connection member completion should include tables_written");
        assert_eq!(
            tables_completion.detail,
            "written SQLite tables with row counts"
        );
        assert!(member_completions
            .iter()
            .any(|completion| completion.label == "table_count"));
        assert!(member_completions
            .iter()
            .any(|completion| completion.label == "row_count"));
        let tables_written_token = snapshot
            .semantic_tokens
            .tokens
            .iter()
            .find(|token| {
                token.token_type == "property"
                    && source.lines().nth(token.line).is_some_and(|line| {
                        &line[token.start..token.start + token.length] == "tables_written"
                    })
            })
            .expect("DB connection field token should be semantic-highlighted");
        assert!(tables_written_token
            .modifiers
            .iter()
            .any(|modifier| modifier == "db"));
        assert!(tables_written_token
            .modifiers
            .iter()
            .any(|modifier| modifier == "workflowStep"));
    }
    #[test]
    fn snapshot_exposes_case_table_member_fields() {
        let source = "samples = sample lhs\nwith {\n    count = 2\n    seed = 42\n    cooling_cop = uniform(2.5, 5.0)\n}\n\ncases = materialize cases samples\ncase_inputs = apply case_input_template over cases\nwith {\n    template = file(\"model/native_case_template.txt\")\n    output = \"{case_dir}/input.txt\"\n}\ncase_results = collect results case_inputs\n\npending = cases.pending_count\nexpected = case_inputs.expected_count\nrendered = case_inputs.rendered_count\ncollected = case_results.collected_count\n";
        let snapshot = snapshot_for_source(Path::new("case_members.eng"), source);

        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "cases.pending_count"));
        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "case_inputs.expected_count"));
        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "case_inputs.rendered_count"));
        assert!(!snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "case_inputs.planned_count"));
        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "case_results.collected_count"));
        let case_line = source
            .lines()
            .position(|line| line.contains("pending ="))
            .expect("pending line");
        let case_completions = completion_items_for_source_position(
            Path::new("case_members.eng"),
            source,
            case_line,
            "pending = cases.".len(),
        );
        assert!(case_completions
            .iter()
            .any(|completion| completion.label == "pending_count"));
        let expected_line = source
            .lines()
            .position(|line| line.contains("expected ="))
            .expect("expected line");
        let expected_completions = completion_items_for_source_position(
            Path::new("case_members.eng"),
            source,
            expected_line,
            "expected = case_inputs.".len(),
        );
        assert!(expected_completions
            .iter()
            .any(|completion| completion.label == "expected_count"));
        let output_line = source
            .lines()
            .position(|line| line.contains("rendered ="))
            .expect("rendered line");
        let output_completions = completion_items_for_source_position(
            Path::new("case_members.eng"),
            source,
            output_line,
            "rendered = case_inputs.".len(),
        );
        assert!(output_completions
            .iter()
            .any(|completion| completion.label == "rendered_count"));
        assert!(!output_completions
            .iter()
            .any(|completion| completion.label == "planned_count"));
        let collection_line = source
            .lines()
            .position(|line| line.contains("collected ="))
            .expect("collected line");
        let collection_completions = completion_items_for_source_position(
            Path::new("case_members.eng"),
            source,
            collection_line,
            "collected = case_results.".len(),
        );
        assert!(collection_completions
            .iter()
            .any(|completion| completion.label == "collected_count"));
        assert!(snapshot.semantic_tokens.tokens.iter().any(|token| {
            token.token_type == "property"
                && source.lines().nth(token.line).is_some_and(|line| {
                    &line[token.start..token.start + token.length] == "pending_count"
                })
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
        let connection_hover = hovers
            .iter()
            .find(|hover| hover["kind"] == "connection" && hover["status"] == "domain_compatible")
            .expect("connection hover should retain raw kind/status metadata");
        let connection_markdown = connection_hover["contents"]["value"]
            .as_str()
            .expect("connection hover should render markdown");
        assert!(connection_markdown.contains("Kind: Connection"));
        assert!(connection_markdown.contains("Status: Domain compatible"));
        assert!(!connection_markdown.contains("Status: `domain_compatible`"));
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
    fn snapshot_exposes_model_and_prediction_member_fields() {
        let source = concat!(
            "designs = sample lhs\n",
            "with {\n",
            "    count = 4\n",
            "    seed = 5\n",
            "    cooling_cop = uniform(2.5, 5.0)\n",
            "}\n",
            "results = derive designs column annual_electricity = 10000 kWh - cooling_cop * 500 kWh\n",
            "model = train regression results\n",
            "with {\n",
            "    target = annual_electricity\n",
            "    features = [cooling_cop]\n",
            "    test = 0.25\n",
            "    seed = 7\n",
            "}\n",
            "predictions = predict model using designs\n",
            "model_status = model.status\n",
            "model_error = model.rmse\n",
            "prediction_cases = predictions.case_count\n",
            "prediction_output = predictions.output_column\n",
        );
        let snapshot = snapshot_for_source(Path::new("model_prediction_members.eng"), source);

        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "model.rmse"));
        assert!(snapshot
            .completions
            .iter()
            .any(|completion| completion.label == "predictions.output_column"));
        let model_line = source
            .lines()
            .position(|line| line.contains("model_status ="))
            .expect("model status line");
        let model_member_completions = completion_items_for_source_position(
            Path::new("model_prediction_members.eng"),
            source,
            model_line,
            "model_status = model.".len(),
        );
        let rmse_completion = model_member_completions
            .iter()
            .find(|completion| completion.label == "rmse")
            .expect("model member completion should include rmse");
        assert_eq!(rmse_completion.detail, "model root-mean-square error");
        assert!(model_member_completions
            .iter()
            .any(|completion| completion.label == "train_count"));
        let prediction_line = source
            .lines()
            .position(|line| line.contains("prediction_output ="))
            .expect("prediction output line");
        let prediction_member_completions = completion_items_for_source_position(
            Path::new("model_prediction_members.eng"),
            source,
            prediction_line,
            "prediction_output = predictions.".len(),
        );
        let output_completion = prediction_member_completions
            .iter()
            .find(|completion| completion.label == "output_column")
            .expect("prediction member completion should include output_column");
        assert_eq!(output_completion.detail, "prediction output column name");
        assert!(prediction_member_completions
            .iter()
            .any(|completion| completion.label == "confidence_column"));
        let rmse_hover = snapshot
            .hovers
            .iter()
            .find(|hover| hover.name == "model.rmse")
            .expect("model member field should expose hover metadata");
        assert_eq!(rmse_hover.kind, "model_field");
        assert_eq!(rmse_hover.quantity_kind, "DimensionlessNumber");
        let rmse_markdown = hover_json(rmse_hover)["contents"]["value"]
            .as_str()
            .expect("model member hover should render markdown")
            .to_owned();
        assert!(rmse_markdown.contains("Kind: Model field"));
        assert!(!rmse_markdown.contains("Kind: `model_field`"));
        let output_hover = snapshot
            .hovers
            .iter()
            .find(|hover| hover.name == "predictions.output_column")
            .expect("prediction member field should expose hover metadata");
        assert_eq!(output_hover.kind, "prediction_table_field");
        assert_semantic_token_on_line(
            &snapshot,
            source,
            "model_error =",
            "rmse",
            "property",
            "model",
        );
        assert_semantic_token_on_line(
            &snapshot,
            source,
            "prediction_output =",
            "output_column",
            "property",
            "model",
        );
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

        let source = "model = regression(split, ";
        let completions = completion_items_for_source_position(
            Path::new("model_arg_completion.eng"),
            source,
            0,
            source.len(),
        );
        for label in ["features", "target", "test"] {
            assert!(
                completions
                    .iter()
                    .any(|completion| completion.label == label),
                "regression argument completion should include canonical option {label}"
            );
        }
        for alias in ["x", "y", "test_fraction", "layers"] {
            assert!(
                !completions
                    .iter()
                    .any(|completion| completion.label == alias),
                "regression argument completion should hide alias {alias}"
            );
        }

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
    fn with_block_completion_uses_http_cache_context() {
        let source = r#"response = http get url("https://api.example.org/weather")
with {

}
"#;
        let line = source
            .lines()
            .position(|line| line.trim().is_empty())
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("http_with_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        for label in [
            "query",
            "headers",
            "expected_sha256",
            "body_size_limit",
            "cache_ttl",
            "status_code",
        ] {
            assert!(
                completions
                    .iter()
                    .any(|completion| completion.label == label),
                "HTTP with-block completion should include {label}"
            );
        }
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "transaction"));
    }

    #[test]
    fn with_block_completion_uses_download_cache_context() {
        let source = r#"download url("https://example.org/file.csv") to file("outputs/file.csv")
with {

}
"#;
        let line = source
            .lines()
            .position(|line| line.trim().is_empty())
            .unwrap();
        let character = source.lines().nth(line).unwrap().len();
        let report = check_source(
            Path::new("download_with_completion.eng"),
            source,
            &CheckOptions::default(),
        );
        let completions = completion_items_at(&report, source, line, character);

        for label in [
            "offline_response",
            "expected_sha256",
            "response_body_limit",
            "cache_ttl",
        ] {
            assert!(
                completions
                    .iter()
                    .any(|completion| completion.label == label),
                "download with-block completion should include {label}"
            );
        }
        assert!(!completions
            .iter()
            .any(|completion| completion.label == "body_size_limit"));
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

        for label in [
            "algorithm",
            "features",
            "target",
            "test",
            "hidden",
            "epochs",
            "cache_key",
        ] {
            assert!(
                completions
                    .iter()
                    .any(|completion| completion.label == label),
                "model training with-block completion should include {label}"
            );
        }
        for alias in ["x", "y", "test_fraction", "layers"] {
            assert!(
                !completions
                    .iter()
                    .any(|completion| completion.label == alias),
                "model training with-block completion should hide alias {alias}"
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
