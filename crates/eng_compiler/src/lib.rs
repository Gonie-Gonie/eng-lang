mod ast;
mod bytecode;
mod cache;
mod expected;
mod formatter;
mod hover;
mod lexer;
mod ml;
mod module_registry;
mod net;
mod parser;
mod quantities;
mod schema;
mod semantic;
mod source;
mod stats;
mod table;
mod type_info;
mod uncertainty;
mod units;
mod workflow;

use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub use ast::{
    ArgsDecl, ArgsFieldDecl, AssertDecl, AstItem, ClassDecl, ClassFieldDecl, ClassMethodDecl,
    ClassObjectCopyDecl, ClassObjectDecl, ClassObjectFieldDecl, ClassValidationDecl,
    CommandClauseDecl, CommandStyleDecl, ComponentDecl, ConnectDecl, ConservationDecl, ConstDecl,
    CsvExportDecl, CsvExportFieldDecl, DomainDecl, DomainVariableDecl, EquationDecl, ExplicitDecl,
    FastBinding, FileOperationDecl, FunctionDecl, FunctionParamDecl, GoldenDecl, ImportDecl,
    NetDownloadDecl, OnBlockDecl, OnPredicateDecl, PortDecl, PrintDecl, ReturnDecl, SchemaDecl,
    ScriptDecl, StateSpaceTypeBlockDecl, StateSpaceTypeMemberDecl, StructDecl, SystemDecl,
    SystemVariableDecl, TestDecl, WhereBindingDecl, WhereBlockDecl, WherePredicateDecl,
    WithBlockDecl, WithOptionDecl, WriteDecl,
};
pub use bytecode::{
    build_bytecode_program, encode_bytecode, parse_bytecode, BytecodeInstruction, BytecodeObject,
    BytecodeParseError, BytecodeProgram, BYTECODE_FORMAT, BYTECODE_VERSION,
};
pub use cache::CacheRecordInfo;
pub use expected::{ExpectedType, ExpectedTypeSource};
pub use formatter::{format_source, format_source_with_options, FormatOptions, FormatResult};
pub use hover::HoverHint;
pub use lexer::{Keyword, Symbol, Token, TokenKind};
pub use ml::MlInfo;
pub use module_registry::{
    bundled_module_registry, load_module_registry, parse_module_registry, ModuleRegistry,
    ModuleRegistryEntry, ModuleRegistryError,
};
pub use net::{NetDownloadInfo, NetQueryParam, NetRequestInfo};
pub use parser::{parse_source, ParseContext, ParsedLine, ParsedProgram, SyntaxSummary};
pub use quantities::{all_quantity_completions, normalize_unit, QuantityCompletion};
pub use schema::{
    ConfigPromotion, ConfigTypeMismatch, CsvPromotion, MissingPolicy, SchemaColumn,
    SchemaConstraint, SchemaInfo,
};
pub use semantic::read_only_io_expression;
pub use semantic::{
    ArgValueInfo, ArgsBlockInfo, ArgsFieldInfo, AssertInfo, ClassFieldInfo, ClassInfo,
    ClassMethodInfo, ClassObjectFieldInfo, ClassObjectInfo, ClassObjectValidationInfo,
    ClassValidationInfo, CommandClauseInfo, CommandStyleInfo, ComponentAssemblyBoundaryInfo,
    ComponentAssemblyEquationInfo, ComponentAssemblyInfo, ComponentAssemblyVariableInfo,
    ComponentConnectionSetInfo, ComponentConstructorArgumentInfo, ComponentDomainPlanInfo,
    ComponentInfo, ComponentJacobianSparsityInfo, ComponentLocalExpressionInfo,
    ComponentResidualDependencyInfo, ComponentResidualGraphInfo,
    ComponentResidualGraphResidualInfo, ComponentSolverPreviewInfo, ConnectionInfo,
    ConservationInfo, ConstInfo, CsvExportFieldInfo, CsvExportInfo, DomainInfo,
    DomainTypeParameterInfo, DomainVariableInfo, EnvironmentDependencyInfo, EquationDependencyInfo,
    EquationInfo, EquationIrInfo, FileOperationInfo, FormatExpressionInfo, FunctionInfo,
    FunctionLocalInfo, FunctionParamInfo, GoldenInfo, ImportInfo, JacobianSeedInfo,
    LinearOperatorEntryInfo, LinearOperatorInfo, OdeRunnerInfo, PortInfo, PrintInfo, ResidualInfo,
    SemanticProgram, SemanticType, SolverPlanInfo, StateSpaceVectorInfo, SystemInfo,
    SystemVariableInfo, TestInfo, TimeSeriesKernelInfo, TypedBinding, WhereBindingInfo,
    WhereBlockInfo, WithBlockInfo, WithOptionInfo, WriteInfo,
};
pub use source::SourceSpan;
pub use stats::{AxisInfo, IntegrationInfo, StatsInfo};
pub use table::{
    TableColumnInfo, TableDerivedColumnInfo, TableJoinKeyInfo, TablePredicateInfo,
    TableSortKeyInfo, TableTransformInfo,
};
pub use type_info::{TypeInfo, TypeInfoSource};
pub use uncertainty::{UncertaintyInfo, UncertaintyPropagationTerm};
pub use units::{all_unit_infos, UnitDerivation, UnitInfo};
pub use workflow::Workflow;

pub const COMPILER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub line: usize,
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn error(code: &str, line: usize, message: &str, help: Option<&str>) -> Self {
        Self {
            severity: Severity::Error,
            code: code.to_owned(),
            message: message.to_owned(),
            line,
            help: help.map(str::to_owned),
        }
    }

    pub fn warning(code: &str, line: usize, message: &str, help: Option<&str>) -> Self {
        Self {
            severity: Severity::Warning,
            code: code.to_owned(),
            message: message.to_owned(),
            line,
            help: help.map(str::to_owned),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewFallbackRecord {
    pub kind: String,
    pub category: String,
    pub target: String,
    pub method: String,
    pub fallback_source: String,
    pub affected_scope: String,
    pub assumption: String,
    pub risk_level: String,
    pub status: String,
    pub reason: String,
    pub line: usize,
}

impl ReviewFallbackRecord {
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": &self.kind,
            "category": &self.category,
            "target": &self.target,
            "method": &self.method,
            "fallback_source": &self.fallback_source,
            "affected_scope": &self.affected_scope,
            "assumption": &self.assumption,
            "risk_level": &self.risk_level,
            "status": &self.status,
            "reason": &self.reason,
            "line": self.line
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReviewRiskClassification {
    pub category: &'static str,
    pub severity: &'static str,
    pub level: &'static str,
}

pub fn classify_review_risk(category: &str, severity: &str) -> ReviewRiskClassification {
    let category = normalize_review_risk_category(category);
    let severity = normalize_review_risk_severity(severity);
    let level = match category {
        "external_boundary" | "side_effect" => "high",
        "data_quality" | "unit_or_quantity" | "uncertainty" | "solver_or_numeric"
        | "reproducibility" => "medium",
        _ if severity == "warning" || severity == "error" => "medium",
        _ => "low",
    };
    ReviewRiskClassification {
        category,
        severity,
        level,
    }
}

pub fn classify_diagnostic_review_risk(code: &str, severity: &str) -> ReviewRiskClassification {
    classify_review_risk(diagnostic_review_risk_category(code), severity)
}

pub fn classify_workflow_node_review_risk(kind: &str, status: &str) -> ReviewRiskClassification {
    classify_review_risk(
        workflow_node_review_risk_category(kind),
        workflow_risk_severity(status),
    )
}

fn normalize_review_risk_category(category: &str) -> &'static str {
    match category {
        "data_quality" => "data_quality",
        "unit_or_quantity" => "unit_or_quantity",
        "external_boundary" => "external_boundary",
        "reproducibility" => "reproducibility",
        "uncertainty" => "uncertainty",
        "solver_or_numeric" => "solver_or_numeric",
        "side_effect" => "side_effect",
        _ => "claim_boundary",
    }
}

fn normalize_review_risk_severity(severity: &str) -> &'static str {
    match severity {
        "warning" => "warning",
        "error" => "error",
        _ => "info",
    }
}

fn diagnostic_review_risk_category(code: &str) -> &'static str {
    if code.contains("UNIT") || code.contains("QTY") {
        "unit_or_quantity"
    } else if code.contains("UNC") {
        "uncertainty"
    } else if code.contains("SOLVER") || code.contains("NEWTON") || code.contains("DAE") {
        "solver_or_numeric"
    } else if code.contains("SCHEMA")
        || code.contains("CSV")
        || code.contains("DATA")
        || code.contains("TABLE")
    {
        "data_quality"
    } else if code.contains("SIDE") || code.contains("PROCESS") || code.contains("FILE") {
        "side_effect"
    } else {
        "claim_boundary"
    }
}

fn workflow_node_review_risk_category(kind: &str) -> &'static str {
    match kind {
        "network_request" | "network_download" | "process" | "db_write" => "external_boundary",
        "file_operation" => "side_effect",
        "cache" | "environment_dependency" => "reproducibility",
        "csv_promotion"
        | "config_promotion"
        | "timeseries_kernel"
        | "timeseries_coverage"
        | "case"
        | "model" => "data_quality",
        "system" | "component_solution" | "solver_boundary" => "solver_or_numeric",
        _ => "claim_boundary",
    }
}

fn workflow_risk_severity(status: &str) -> &'static str {
    let status = status.to_ascii_lowercase();
    if status.contains("fail")
        || status.contains("error")
        || status.contains("gapped")
        || status.contains("mismatch")
        || status.contains("missing")
    {
        "warning"
    } else {
        "info"
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InferredDeclaration {
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub expression: String,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub struct CheckReport {
    pub source_path: PathBuf,
    pub source_hash: String,
    pub diagnostics: Vec<Diagnostic>,
    pub inferred_declarations: Vec<InferredDeclaration>,
    pub syntax_summary: SyntaxSummary,
    pub semantic_program: SemanticProgram,
    pub quantity_completion_count: usize,
    pub unit_info_count: usize,
}

impl CheckReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    }

    pub fn diagnostic_count(&self, severity: Severity) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == severity)
            .count()
    }
}

#[derive(Clone, Debug, Default)]
pub struct CheckOptions {
    pub review: bool,
    pub args: Vec<ArgOverride>,
    pub require_args: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArgOverride {
    pub name: String,
    pub value: String,
}

pub fn check_file(path: impl AsRef<Path>, options: &CheckOptions) -> std::io::Result<CheckReport> {
    let path = path.as_ref();
    let source = fs::read_to_string(path)?;
    Ok(check_source(path, &source, options))
}

pub fn check_source(path: impl AsRef<Path>, source: &str, options: &CheckOptions) -> CheckReport {
    let source_path = path.as_ref();
    let source_hash = hash_text(source);
    let mut parsed = parser::parse_source(source);
    let mut import_diagnostics = Vec::new();
    if let Some(base_dir) = source_path.parent() {
        let mut visited = HashSet::new();
        let imported_items =
            resolve_file_imports(&parsed, base_dir, &mut visited, &mut import_diagnostics);
        if !imported_items.is_empty() {
            parsed.items.splice(0..0, imported_items);
        }
    }
    let mut semantic_output = semantic::analyze(&parsed);
    semantic_output.diagnostics.extend(import_diagnostics);
    let (arg_values, arg_diagnostics) =
        resolve_arg_values(&semantic_output.semantic_program, options);
    let schema_analysis = schema::analyze_schema(&parsed, source_path.parent(), &arg_values);
    semantic_output.diagnostics.extend(arg_diagnostics);
    semantic_output
        .diagnostics
        .extend(schema_analysis.diagnostics);
    semantic_output.semantic_program.schemas = schema_analysis.schemas;
    semantic_output.semantic_program.csv_promotions = schema_analysis.csv_promotions;
    semantic_output.semantic_program.config_promotions = schema_analysis.config_promotions;
    semantic_output.semantic_program.arg_values = arg_values;
    let table_analysis =
        table::analyze_table_transforms(&parsed, &semantic_output.semantic_program);
    semantic_output
        .diagnostics
        .extend(table_analysis.diagnostics);
    semantic_output.semantic_program.table_transforms = table_analysis.transforms;
    let net_analysis = net::analyze_net_boundaries(
        &parsed,
        source_path.parent(),
        &semantic_output.semantic_program,
    );
    semantic_output.diagnostics.extend(net_analysis.diagnostics);
    semantic_output.semantic_program.net_requests = net_analysis.requests;
    semantic_output.semantic_program.net_downloads = net_analysis.downloads;
    let cache_analysis =
        cache::analyze_cache_records(&semantic_output.semantic_program, &source_hash);
    semantic_output
        .diagnostics
        .extend(cache_analysis.diagnostics);
    semantic_output.semantic_program.cache_records = cache_analysis.records;
    semantic_output.semantic_program.environment_dependencies = collect_environment_dependencies(
        &parsed,
        source_path.parent(),
        &semantic_output.semantic_program,
    );
    semantic_output
        .diagnostics
        .extend(validate_structured_read_parse_diagnostics(
            &semantic_output.semantic_program.environment_dependencies,
        ));
    semantic_output
        .diagnostics
        .extend(semantic::validate_simulation_contracts(
            &semantic_output.semantic_program,
            &semantic_output.inferred_declarations,
        ));
    semantic_output
        .diagnostics
        .extend(validate_generated_output_path_policies(
            &semantic_output.semantic_program,
        ));

    CheckReport {
        source_path: source_path.to_path_buf(),
        source_hash,
        diagnostics: semantic_output.diagnostics,
        inferred_declarations: semantic_output.inferred_declarations,
        syntax_summary: parsed.summary(),
        semantic_program: semantic_output.semantic_program,
        quantity_completion_count: quantities::completion_count(),
        unit_info_count: units::unit_info_count(),
    }
}

fn resolve_file_imports(
    parsed: &ParsedProgram,
    base_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<AstItem> {
    let mut imported_items = Vec::new();
    for item in &parsed.items {
        let AstItem::Import(import) = item else {
            continue;
        };
        if import_target_is_dynamic(&import.target) {
            diagnostics.push(Diagnostic::error(
                "E-IMPORT-DYNAMIC-001",
                import.line,
                "import path cannot depend on args/runtime values.",
                Some("Use a static file import such as `use \"./defaults.eng\"`."),
            ));
            continue;
        }
        if import.kind != "file" {
            diagnostics.push(Diagnostic::error(
                "E-IMPORT-001",
                import.line,
                &format!(
                    "`use {}` is not supported by the current import resolver.",
                    import.target
                ),
                Some("Use a file import such as `use \"thermal.eng\"`."),
            ));
            continue;
        }
        let Some(import_path) =
            resolve_import_path(base_dir, &import.target, import.line, diagnostics)
        else {
            continue;
        };
        if !visited.insert(import_path.clone()) {
            diagnostics.push(Diagnostic::error(
                "E-IMPORT-002",
                import.line,
                &format!("Import cycle detected at `{}`.", import_path.display()),
                Some("Remove the recursive import or split shared functions into a third file."),
            ));
            continue;
        }
        let source = match fs::read_to_string(&import_path) {
            Ok(source) => source,
            Err(error) => {
                diagnostics.push(Diagnostic::error(
                    "E-IMPORT-003",
                    import.line,
                    &format!("Could not read import `{}`: {error}.", import.target),
                    Some("Check the relative path and file encoding. EngLang sources should be UTF-8."),
                ));
                visited.remove(&import_path);
                continue;
            }
        };
        let imported = parser::parse_source(&source);
        if imported_has_args_block(&imported) {
            diagnostics.push(Diagnostic::warning(
                "W-MODULE-ARGS-NOT-IMPORTED-001",
                import.line,
                &format!(
                    "Imported module `{}` has an args block, but args are not imported.",
                    import.target
                ),
                Some("Args belong to the root execution context only."),
            ));
        }
        diagnose_non_importable_symbol_uses(&imported, parsed, &import.target, diagnostics);
        if let Some(import_base_dir) = import_path.parent() {
            imported_items.extend(resolve_file_imports(
                &imported,
                import_base_dir,
                visited,
                diagnostics,
            ));
        }
        imported_items.extend(
            imported
                .items
                .into_iter()
                .filter(importable_definition_item),
        );
        visited.remove(&import_path);
    }
    imported_items
}

fn import_target_is_dynamic(target: &str) -> bool {
    let compact = target.replace(char::is_whitespace, "");
    compact.contains('{')
        || compact == "args"
        || compact.contains("args.")
        || compact.contains("(args")
        || compact.contains(",args")
}

fn resolve_import_path(
    base_dir: &Path,
    target: &str,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<PathBuf> {
    let raw = Path::new(target);
    let path = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        base_dir.join(raw)
    };
    match path.canonicalize() {
        Ok(path) => Some(path),
        Err(_) if path.exists() => Some(path),
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                "E-IMPORT-004",
                line,
                &format!("Could not resolve import `{target}`: {error}."),
                Some("Imports are resolved relative to the importing source file."),
            ));
            None
        }
    }
}

fn importable_definition_item(item: &AstItem) -> bool {
    match item {
        AstItem::Function(_) | AstItem::Return(_) => true,
        AstItem::FastBinding(binding) => binding.context == ParseContext::Function,
        AstItem::Const(declaration) => declaration.context == ParseContext::TopLevel,
        AstItem::Schema(_)
        | AstItem::Constraint(_)
        | AstItem::MissingPolicy(_)
        | AstItem::System(_)
        | AstItem::StateSpaceTypeBlock(_)
        | AstItem::StateSpaceTypeMember(_)
        | AstItem::SystemVariable(_)
        | AstItem::Equation(_)
        | AstItem::Domain(_)
        | AstItem::DomainVariable(_)
        | AstItem::Conservation(_)
        | AstItem::Component(_)
        | AstItem::Port(_)
        | AstItem::Class(_)
        | AstItem::ClassField(_)
        | AstItem::ClassValidation(_)
        | AstItem::ClassMethod(_) => true,
        AstItem::ExplicitDecl(declaration) => declaration.context == ParseContext::Schema,
        _ => false,
    }
}

fn imported_has_args_block(program: &ParsedProgram) -> bool {
    program
        .items
        .iter()
        .any(|item| matches!(item, AstItem::Args(_)))
}

fn diagnose_non_importable_symbol_uses(
    imported: &ParsedProgram,
    importer: &ParsedProgram,
    target: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let local_definitions = importer
        .items
        .iter()
        .filter_map(importer_defined_symbol)
        .collect::<HashSet<_>>();
    for item in &imported.items {
        let Some(binding) = non_importable_top_level_binding(item) else {
            continue;
        };
        if local_definitions.contains(&binding.name) {
            continue;
        }
        if let Some(line) = first_symbol_use_line(importer, &binding.name) {
            diagnostics.push(Diagnostic::error(
                "E-IMPORT-SYMBOL-001",
                line,
                &format!("`{}` is not importable from {}.", binding.name, target),
                Some(
                    "Top-level `name = expr` bindings are executable locals. Use `const name: Type = expr` for reusable module values.",
                ),
            ));
        }
    }
}

fn importer_defined_symbol(item: &AstItem) -> Option<String> {
    match item {
        AstItem::Const(declaration) => Some(declaration.name.clone()),
        AstItem::Function(function) => Some(function.name.clone()),
        AstItem::FastBinding(binding) if binding.context == ParseContext::TopLevel => {
            Some(binding.name.clone())
        }
        AstItem::ExplicitDecl(declaration) if declaration.context == ParseContext::TopLevel => {
            Some(declaration.name.clone())
        }
        _ => None,
    }
}

fn non_importable_top_level_binding(item: &AstItem) -> Option<&FastBinding> {
    match item {
        AstItem::FastBinding(binding) if binding.context == ParseContext::TopLevel => Some(binding),
        _ => None,
    }
}

fn first_symbol_use_line(program: &ParsedProgram, symbol: &str) -> Option<usize> {
    program.items.iter().find_map(|item| match item {
        AstItem::FastBinding(binding)
            if binding.context == ParseContext::TopLevel
                && expression_mentions_identifier(&binding.expression, symbol) =>
        {
            Some(binding.line)
        }
        AstItem::ExplicitDecl(declaration)
            if declaration.context == ParseContext::TopLevel
                && declaration.expression.as_deref().is_some_and(|expression| {
                    expression_mentions_identifier(expression, symbol)
                }) =>
        {
            Some(declaration.line)
        }
        AstItem::Const(declaration)
            if expression_mentions_identifier(&declaration.expression, symbol) =>
        {
            Some(declaration.line)
        }
        AstItem::Print(print) if print.template.contains(symbol) => Some(print.line),
        AstItem::CsvExportField(field)
            if expression_mentions_identifier(&field.expression, symbol) =>
        {
            Some(field.line)
        }
        _ => None,
    })
}

fn expression_mentions_identifier(expression: &str, identifier: &str) -> bool {
    expression
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .any(|token| token == identifier)
}

fn is_identifier_text(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn resolve_arg_values(
    program: &SemanticProgram,
    options: &CheckOptions,
) -> (Vec<ArgValueInfo>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut overrides = HashMap::new();
    for arg in &options.args {
        overrides.insert(normalize_arg_name(&arg.name), arg.value.clone());
    }

    let mut declared = HashSet::new();
    let mut values = Vec::new();
    for args_block in &program.args_blocks {
        for field in &args_block.fields {
            declared.insert(field.name.clone());
            let (raw_value, source) = if let Some(value) = overrides.get(&field.name) {
                (value.clone(), "cli")
            } else if let Some(default_value) = &field.default_value {
                if arg_default_has_side_effect(default_value) {
                    diagnostics.push(Diagnostic::error(
                        "E-ARGS-SIDE-EFFECT-001",
                        field.line,
                        "Args default expressions must not perform side-effecting operations.",
                        Some("Move this operation into the executable body."),
                    ));
                    continue;
                }
                if arg_default_depends_on_runtime(default_value) {
                    diagnostics.push(Diagnostic::warning(
                        "W-ARGS-RUNTIME-DEFAULT-001",
                        field.line,
                        &format!(
                            "Args default for `{}` depends on environment/time/current directory.",
                            field.name
                        ),
                        Some("The resolved value is recorded in arg_values for provenance."),
                    ));
                }
                (
                    evaluate_arg_default(default_value, program)
                        .unwrap_or_else(|| strip_string_literal(default_value)),
                    "default",
                )
            } else {
                if options.require_args {
                    diagnostics.push(Diagnostic::error(
                        "E-ARGS-REQUIRED-001",
                        field.line,
                        &format!("Required Args field `{}` was not provided.", field.name),
                        Some("Pass it as `--<name> <value>` when running the script."),
                    ));
                }
                continue;
            };
            let value = match normalize_arg_value(&field.type_name, &raw_value) {
                Ok(value) => value,
                Err(message) => {
                    diagnostics.push(Diagnostic::error(
                        "E-ARGS-TYPE-001",
                        field.line,
                        &format!(
                            "Args field `{}` expects {}, but got `{}`.",
                            field.name, field.type_name, raw_value
                        ),
                        Some(&message),
                    ));
                    continue;
                }
            };
            values.push(ArgValueInfo {
                name: field.name.clone(),
                type_name: field.type_name.clone(),
                value,
                source: source.to_owned(),
                required: field.required,
                line: field.line,
            });
        }
    }

    for name in overrides.keys() {
        if !declared.contains(name) {
            diagnostics.push(Diagnostic::error(
                "E-ARGS-UNKNOWN-001",
                1,
                &format!("Unknown Args field `{name}`."),
                Some("Declare the field in `args { ... }` or remove the flag."),
            ));
        }
    }

    (values, diagnostics)
}

fn normalize_arg_value(type_name: &str, value: &str) -> Result<String, String> {
    let stripped = strip_string_literal(value);
    let normalized_type = type_name.trim().to_ascii_lowercase();
    match normalized_type.as_str() {
        "string" => Ok(stripped),
        "path" | "filepath" | "csvfile" | "jsonfile" | "tomlfile" | "textfile" | "reportfile"
        | "plotfile" | "directorypath" => Ok(canonical_path_text(&stripped)),
        "bool" | "boolean" => parse_bool_arg(&stripped).ok_or_else(|| {
            "Use true/false, yes/no, on/off, or 1/0 for boolean Args fields.".to_owned()
        }),
        "int" | "integer" | "i32" | "i64" => stripped
            .trim()
            .parse::<i64>()
            .map(|value| value.to_string())
            .map_err(|_| "Use a whole-number integer value.".to_owned()),
        "count" | "usize" | "u32" | "u64" => stripped
            .trim()
            .parse::<u64>()
            .map(|value| value.to_string())
            .map_err(|_| "Use a non-negative whole-number count.".to_owned()),
        "float" | "number" | "f32" | "f64" => {
            let parsed = stripped
                .trim()
                .parse::<f64>()
                .map_err(|_| "Use a finite numeric value.".to_owned())?;
            if parsed.is_finite() {
                Ok(format_arg_number(parsed))
            } else {
                Err("Use a finite numeric value.".to_owned())
            }
        }
        "duration" => normalize_duration_arg(&stripped),
        _ => Ok(stripped),
    }
}

fn evaluate_arg_default(expression: &str, program: &SemanticProgram) -> Option<String> {
    let expression = expression.trim();
    if let Some(value) = evaluate_path_expression(expression, &[]) {
        return Some(value);
    }
    if let Some(value) = evaluate_env_default(expression) {
        return Some(value);
    }
    if let Some(const_info) = program.consts.iter().find(|const_info| {
        const_info.importable
            && const_info.name == expression
            && !arg_default_has_side_effect(&const_info.expression)
    }) {
        return evaluate_arg_default(&const_info.expression, program)
            .or_else(|| Some(strip_string_literal(&const_info.expression)));
    }
    if let Some(call_name) = zero_arg_call_name(expression) {
        if let Some(function) = program
            .functions
            .iter()
            .find(|function| function.name == call_name && function.parameters.is_empty())
        {
            if let Some(return_expression) = &function.return_expression {
                return evaluate_arg_default(return_expression, program)
                    .or_else(|| Some(strip_string_literal(return_expression)));
            }
        }
    }
    if expression.starts_with('"') {
        return Some(strip_string_literal(expression));
    }
    None
}

fn collect_environment_dependencies(
    parsed: &ParsedProgram,
    source_base: Option<&Path>,
    program: &SemanticProgram,
) -> Vec<EnvironmentDependencyInfo> {
    let mut dependencies = Vec::new();
    for args_block in &program.args_blocks {
        for field in &args_block.fields {
            let Some(default_value) = &field.default_value else {
                continue;
            };
            if !arg_default_depends_on_runtime(default_value) {
                continue;
            }
            let resolved_value = program
                .arg_values
                .iter()
                .find(|arg| arg.name == field.name)
                .map(|arg| arg.value.clone())
                .unwrap_or_else(|| "<unresolved>".to_owned());
            dependencies.push(EnvironmentDependencyInfo {
                name: field.name.clone(),
                kind: environment_dependency_kind(default_value).to_owned(),
                expression: default_value.clone(),
                resolved_value,
                source_hash: None,
                status: "recorded".to_owned(),
                line: field.line,
            });
        }
    }

    for item in &parsed.items {
        let AstItem::FastBinding(binding) = item else {
            continue;
        };
        if binding.context != ParseContext::TopLevel {
            continue;
        }
        if let Some(observation) =
            evaluate_exists_expression(&binding.expression, source_base, &program.arg_values)
        {
            dependencies.push(EnvironmentDependencyInfo {
                name: binding.name.clone(),
                kind: "filesystem_exists".to_owned(),
                expression: binding.expression.clone(),
                resolved_value: observation.value,
                source_hash: None,
                status: observation.status,
                line: binding.line,
            });
            continue;
        }
        if let Some(observation) =
            evaluate_read_expression(&binding.expression, source_base, &program.arg_values)
        {
            dependencies.push(EnvironmentDependencyInfo {
                name: binding.name.clone(),
                kind: format!("filesystem_read_{}", observation.kind),
                expression: binding.expression.clone(),
                resolved_value: observation.resolved_path,
                source_hash: observation.source_hash,
                status: observation.status,
                line: binding.line,
            });
        }
    }
    dependencies
}

fn environment_dependency_kind(expression: &str) -> &'static str {
    let lowered = expression.to_ascii_lowercase();
    if lowered.contains("env(") {
        "env"
    } else if lowered.contains("today(") || lowered.contains("now(") {
        "time"
    } else if lowered.contains("current_dir(") || lowered.contains("cwd(") {
        "current_directory"
    } else {
        "runtime"
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExistsObservation {
    value: String,
    status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReadObservation {
    kind: String,
    resolved_path: String,
    source_hash: Option<String>,
    status: String,
}

fn evaluate_exists_expression(
    expression: &str,
    source_base: Option<&Path>,
    arg_values: &[ArgValueInfo],
) -> Option<ExistsObservation> {
    let expression = expression.trim();
    let inner = if let Some(inner) = expression.strip_prefix("exists ") {
        inner.trim()
    } else {
        strip_call_inner(expression, "exists")?
    };
    let path_text = evaluate_path_expression(inner, arg_values)?;
    let path = resolve_source_relative_path(&path_text, source_base);
    let exists = path.exists();
    Some(ExistsObservation {
        value: exists.to_string(),
        status: if exists { "exists" } else { "missing" }.to_owned(),
    })
}

fn evaluate_read_expression(
    expression: &str,
    source_base: Option<&Path>,
    arg_values: &[ArgValueInfo],
) -> Option<ReadObservation> {
    let (kind, path_expression) = semantic::read_only_io_expression(expression)?;
    let path_text = evaluate_path_expression(path_expression, arg_values)?;
    let path = resolve_source_relative_path(&path_text, source_base);
    match fs::read_to_string(&path) {
        Ok(source) => Some(ReadObservation {
            kind: kind.to_owned(),
            resolved_path: canonical_path_text(&path.display().to_string()),
            source_hash: Some(hash_text(&source)),
            status: "read".to_owned(),
        }),
        Err(_) => Some(ReadObservation {
            kind: kind.to_owned(),
            resolved_path: canonical_path_text(&path.display().to_string()),
            source_hash: None,
            status: "missing".to_owned(),
        }),
    }
}

fn validate_structured_read_parse_diagnostics(
    dependencies: &[EnvironmentDependencyInfo],
) -> Vec<Diagnostic> {
    dependencies
        .iter()
        .filter_map(|dependency| {
            let kind = dependency.kind.strip_prefix("filesystem_read_")?;
            if dependency.status != "read" || !matches!(kind, "json" | "toml") {
                return None;
            }
            let source = fs::read_to_string(&dependency.resolved_value).ok()?;
            structured_read_parse_error(kind, &source).map(|error| {
                let (code, label) = match kind {
                    "json" => ("E-IO-JSON-PARSE", "JSON"),
                    "toml" => ("E-IO-TOML-PARSE", "TOML"),
                    _ => unreachable!(),
                };
                Diagnostic::error(
                    code,
                    dependency.line,
                    &format!(
                        "read {kind} source `{}` is not valid {label}: {error}",
                        dependency.resolved_value
                    ),
                    Some("Fix the source file syntax, or use `read text` if raw UTF-8 content is intended."),
                )
            })
        })
        .collect()
}

fn structured_read_parse_error(kind: &str, source: &str) -> Option<String> {
    match kind {
        "json" => serde_json::from_str::<serde_json::Value>(source)
            .err()
            .map(|error| error.to_string()),
        "toml" => source
            .parse::<toml::Value>()
            .err()
            .map(|error| error.to_string()),
        _ => None,
    }
}

fn validate_generated_output_path_policies(program: &SemanticProgram) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for export in &program.csv_exports {
        push_generated_output_path_diagnostic(
            &mut diagnostics,
            "export",
            &export.path,
            export.line,
            program,
        );
    }
    for write in &program.writes {
        push_generated_output_path_diagnostic(
            &mut diagnostics,
            "write output",
            &write.path,
            write.line,
            program,
        );
    }
    for operation in &program.file_operations {
        match operation.operation.as_str() {
            "copy" => {
                if let Some(destination) = &operation.destination {
                    push_generated_output_path_diagnostic(
                        &mut diagnostics,
                        "copy destination",
                        destination,
                        operation.line,
                        program,
                    );
                }
            }
            "move" => {
                push_generated_output_path_diagnostic(
                    &mut diagnostics,
                    "move source",
                    &operation.source,
                    operation.line,
                    program,
                );
                if let Some(destination) = &operation.destination {
                    push_generated_output_path_diagnostic(
                        &mut diagnostics,
                        "move destination",
                        destination,
                        operation.line,
                        program,
                    );
                }
            }
            "delete" => {
                push_generated_output_path_diagnostic(
                    &mut diagnostics,
                    "delete target",
                    &operation.source,
                    operation.line,
                    program,
                );
            }
            _ => {}
        }
    }
    diagnostics
}

fn push_generated_output_path_diagnostic(
    diagnostics: &mut Vec<Diagnostic>,
    label: &str,
    expression: &str,
    line: usize,
    program: &SemanticProgram,
) {
    let Some(path) = generated_output_path_value(expression, program) else {
        return;
    };
    let Some((code, reason)) = generated_output_path_policy_violation(&path) else {
        return;
    };
    diagnostics.push(Diagnostic::error(
        code,
        line,
        &format!("{label} path `{path}` {reason}."),
        Some("Generated outputs must stay under the run result directory; remove absolute roots and `..` segments."),
    ));
}

fn generated_output_path_value(expression: &str, program: &SemanticProgram) -> Option<String> {
    evaluate_path_expression(expression, &program.arg_values).or_else(|| {
        let trimmed = expression.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("args.")
            || trimmed.contains('(')
            || trimmed.contains(',')
        {
            None
        } else {
            Some(strip_string_literal(trimmed))
        }
    })
}

fn generated_output_path_policy_violation(path: &str) -> Option<(&'static str, &'static str)> {
    let normalized = path.replace('\\', "/");
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return Some(("E-PATH-INVALID", "is empty"));
    }
    if trimmed.starts_with('/') || trimmed.starts_with("//") || has_windows_drive_prefix(trimmed) {
        return Some((
            "E-PATH-OUTSIDE-OUTPUT-ROOT",
            "escapes the generated output root",
        ));
    }
    if trimmed.split('/').any(|segment| segment == "..") {
        return Some(("E-PATH-TRAVERSAL", "contains a parent-directory segment"));
    }
    None
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic()
}

fn evaluate_path_expression(expression: &str, arg_values: &[ArgValueInfo]) -> Option<String> {
    let expression = expression.trim();
    if let Some(arg_name) = expression.strip_prefix("args.") {
        return arg_values
            .iter()
            .find(|arg| arg.name == arg_name.trim())
            .map(|arg| arg.value.clone());
    }
    if let Some(value) = strip_call_string_arg(expression, "file") {
        return Some(canonical_path_text(&value));
    }
    if let Some(value) = strip_call_string_arg(expression, "dir") {
        return Some(canonical_path_text(&value));
    }
    if expression.starts_with('"') {
        return Some(canonical_path_text(&strip_string_literal(expression)));
    }
    if let Some(inner) = strip_call_inner(expression, "join") {
        let parts = split_call_args(inner)
            .into_iter()
            .map(|part| evaluate_path_expression(&part, arg_values))
            .collect::<Option<Vec<_>>>()?;
        if parts.is_empty() {
            return None;
        }
        return Some(join_path_text(&parts));
    }
    if let Some(inner) = strip_call_inner(expression, "parent") {
        let path = evaluate_path_expression(inner, arg_values)?;
        return Some(parent_path_text(&path));
    }
    if let Some(inner) = strip_call_inner(expression, "stem") {
        let path = evaluate_path_expression(inner, arg_values)?;
        return Some(
            Path::new(&path)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_owned(),
        );
    }
    if let Some(inner) = strip_call_inner(expression, "extension") {
        let path = evaluate_path_expression(inner, arg_values)?;
        return Some(
            Path::new(&path)
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_owned(),
        );
    }
    None
}

fn strip_call_inner<'a>(expression: &'a str, function_name: &str) -> Option<&'a str> {
    let trimmed = expression.trim();
    let prefix = format!("{function_name}(");
    trimmed
        .strip_prefix(&prefix)?
        .strip_suffix(')')
        .map(str::trim)
}

fn join_path_text(parts: &[String]) -> String {
    let mut joined = String::new();
    for part in parts {
        let normalized = path_text(part);
        let trimmed = normalized.trim_matches('/');
        if trimmed.is_empty() {
            continue;
        }
        if !joined.is_empty() {
            joined.push('/');
        }
        joined.push_str(trimmed);
    }
    joined
}

fn parent_path_text(path: &str) -> String {
    Path::new(path)
        .parent()
        .and_then(|value| value.to_str())
        .map(path_text)
        .unwrap_or_default()
}

fn path_text(path: impl AsRef<str>) -> String {
    canonical_path_text(path.as_ref())
}

pub fn canonical_path_text(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let preserve_unc_prefix = normalized.starts_with("//");
    let mut collapsed = String::new();
    let mut previous_was_slash = false;
    for ch in normalized.chars() {
        if ch == '/' {
            if previous_was_slash && !(preserve_unc_prefix && collapsed == "/") {
                continue;
            }
            previous_was_slash = true;
        } else {
            previous_was_slash = false;
        }
        collapsed.push(ch);
    }
    while let Some(stripped) = collapsed.strip_prefix("./") {
        collapsed = stripped.to_owned();
    }
    if collapsed.is_empty() {
        return ".".to_owned();
    }
    collapsed
}

fn resolve_source_relative_path(path: &str, source_base: Option<&Path>) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    source_base.unwrap_or_else(|| Path::new(".")).join(path)
}

fn strip_call_string_arg(expression: &str, function_name: &str) -> Option<String> {
    let trimmed = expression.trim();
    let prefix = format!("{function_name}(");
    let inner = trimmed.strip_prefix(&prefix)?.strip_suffix(')')?.trim();
    Some(strip_string_literal(inner))
}

fn evaluate_env_default(expression: &str) -> Option<String> {
    let inner = expression
        .trim()
        .strip_prefix("env(")?
        .strip_suffix(')')?
        .trim();
    let parts = split_call_args(inner);
    let name = parts.first().map(|value| strip_string_literal(value))?;
    let fallback = parts.get(1).map(|value| strip_string_literal(value));
    env::var(&name).ok().or(fallback)
}

fn zero_arg_call_name(expression: &str) -> Option<&str> {
    let trimmed = expression.trim();
    let name = trimmed.strip_suffix("()")?;
    if is_identifier_text(name) {
        Some(name)
    } else {
        None
    }
}

fn split_call_args(args: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in args.char_indices() {
        if in_string {
            escaped = character == '\\' && !escaped;
            if character == '"' && !escaped {
                in_string = false;
            }
            if character != '\\' {
                escaped = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                let part = args[start..index].trim();
                if !part.is_empty() {
                    parts.push(part.to_owned());
                }
                start = index + 1;
            }
            _ => {}
        }
    }
    let tail = args[start..].trim();
    if !tail.is_empty() {
        parts.push(tail.to_owned());
    }
    parts
}

fn arg_default_has_side_effect(expression: &str) -> bool {
    let lowered = expression.to_ascii_lowercase();
    [
        "download(",
        "read text ",
        "read json ",
        "read toml ",
        "read_text(",
        "read_json(",
        "read_toml(",
        "read_csv(",
        "write_file(",
        "save(",
        "create_temp_dir(",
        "promote ",
        "promote(",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn arg_default_depends_on_runtime(expression: &str) -> bool {
    let lowered = expression.to_ascii_lowercase();
    ["env(", "today(", "now(", "current_dir(", "cwd("]
        .iter()
        .any(|needle| lowered.contains(needle))
}

fn parse_bool_arg(value: &str) -> Option<String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "y" | "on" => Some("true".to_owned()),
        "false" | "0" | "no" | "n" | "off" => Some("false".to_owned()),
        _ => None,
    }
}

fn normalize_duration_arg(value: &str) -> Result<String, String> {
    let (amount, unit) = parse_number_with_suffix(value)
        .ok_or_else(|| "Use a duration such as `30 s`, `10 min`, or `1 h`.".to_owned())?;
    if !amount.is_finite() || amount < 0.0 {
        return Err("Use a non-negative finite duration.".to_owned());
    }
    let seconds = match unit.unwrap_or("s") {
        "s" | "sec" | "secs" | "second" | "seconds" => amount,
        "m" | "min" | "mins" | "minute" | "minutes" => amount * 60.0,
        "h" | "hr" | "hrs" | "hour" | "hours" => amount * 3600.0,
        _ => return Err("Supported duration units are s, min, and h.".to_owned()),
    };
    Ok(format!("{} s", format_arg_number(seconds)))
}

fn parse_number_with_suffix(value: &str) -> Option<(f64, Option<&str>)> {
    let trimmed = value.trim();
    let mut split_at = 0usize;
    let mut saw_digit = false;
    let mut previous = '\0';
    for (index, character) in trimmed.char_indices() {
        let allowed = character.is_ascii_digit()
            || character == '.'
            || ((character == '-' || character == '+')
                && (index == 0 || previous == 'e' || previous == 'E'))
            || ((character == 'e' || character == 'E') && saw_digit);
        if !allowed {
            break;
        }
        if character.is_ascii_digit() {
            saw_digit = true;
        }
        split_at = index + character.len_utf8();
        previous = character;
    }
    if !saw_digit {
        return None;
    }
    let amount = trimmed[..split_at].parse::<f64>().ok()?;
    let unit = trimmed[split_at..].trim();
    Some((amount, (!unit.is_empty()).then_some(unit)))
}

fn format_arg_number(value: f64) -> String {
    let mut text = format!("{value:.6}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn normalize_arg_name(name: &str) -> String {
    name.trim_start_matches("--").replace('-', "_")
}

fn strip_string_literal(value: &str) -> String {
    let trimmed = value.trim();
    if let Some(inner) = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        inner.to_owned()
    } else {
        trimmed.to_owned()
    }
}

pub fn build_bytecode(report: &CheckReport, source: &str) -> String {
    encode_bytecode(&build_bytecode_program(report, source))
}

pub fn review_json(report: &CheckReport) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"format\": \"eng-review-preview-1\",\n");
    json.push_str("  \"review_schema_version\": 1,\n");
    json.push_str(&format!(
        "  \"compiler_version\": \"{}\",\n",
        json_escape(COMPILER_VERSION)
    ));
    json.push_str(&format!(
        "  \"source_path\": \"{}\",\n",
        json_escape(&report.source_path.display().to_string())
    ));
    json.push_str(&format!(
        "  \"source_hash\": \"{}\",\n",
        json_escape(&report.source_hash)
    ));
    json.push_str("  \"syntax_summary\": {\n");
    json.push_str(&format!(
        "    \"lines\": {},\n",
        report.syntax_summary.lines
    ));
    json.push_str(&format!(
        "    \"tokens\": {},\n",
        report.syntax_summary.tokens
    ));
    json.push_str(&format!(
        "    \"ast_items\": {},\n",
        report.syntax_summary.ast_items
    ));
    json.push_str(&format!(
        "    \"scripts\": {},\n",
        report.syntax_summary.scripts
    ));
    json.push_str(&format!(
        "    \"imports\": {},\n",
        report.syntax_summary.imports
    ));
    json.push_str(&format!(
        "    \"functions\": {},\n",
        report.syntax_summary.functions
    ));
    json.push_str(&format!(
        "    \"schemas\": {},\n",
        report.syntax_summary.schemas
    ));
    json.push_str(&format!(
        "    \"systems\": {},\n",
        report.syntax_summary.systems
    ));
    json.push_str(&format!(
        "    \"domains\": {},\n",
        report.syntax_summary.domains
    ));
    json.push_str(&format!(
        "    \"domain_variables\": {},\n",
        report.syntax_summary.domain_variables
    ));
    json.push_str(&format!(
        "    \"components\": {},\n",
        report.syntax_summary.components
    ));
    json.push_str(&format!(
        "    \"ports\": {},\n",
        report.syntax_summary.ports
    ));
    json.push_str(&format!(
        "    \"connections\": {},\n",
        report.syntax_summary.connections
    ));
    json.push_str(&format!(
        "    \"structs\": {},\n",
        report.syntax_summary.structs
    ));
    json.push_str(&format!(
        "    \"classes\": {},\n",
        report.syntax_summary.classes
    ));
    json.push_str(&format!(
        "    \"class_fields\": {},\n",
        report.syntax_summary.class_fields
    ));
    json.push_str(&format!(
        "    \"class_validations\": {},\n",
        report.syntax_summary.class_validations
    ));
    json.push_str(&format!(
        "    \"class_methods\": {},\n",
        report.syntax_summary.class_methods
    ));
    json.push_str(&format!(
        "    \"class_objects\": {},\n",
        report.syntax_summary.class_objects
    ));
    json.push_str(&format!(
        "    \"class_object_copies\": {},\n",
        report.syntax_summary.class_object_copies
    ));
    json.push_str(&format!(
        "    \"class_object_fields\": {},\n",
        report.syntax_summary.class_object_fields
    ));
    json.push_str(&format!(
        "    \"args_blocks\": {},\n",
        report.syntax_summary.args_blocks
    ));
    json.push_str(&format!(
        "    \"args_fields\": {},\n",
        report.syntax_summary.args_fields
    ));
    json.push_str(&format!(
        "    \"const_declarations\": {},\n",
        report.syntax_summary.const_declarations
    ));
    json.push_str(&format!(
        "    \"equations\": {},\n",
        report.syntax_summary.equations
    ));
    json.push_str(&format!(
        "    \"fast_bindings\": {},\n",
        report.syntax_summary.fast_bindings
    ));
    json.push_str(&format!(
        "    \"explicit_declarations\": {},\n",
        report.syntax_summary.explicit_declarations
    ));
    json.push_str(&format!(
        "    \"command_styles\": {},\n",
        report.syntax_summary.command_styles
    ));
    json.push_str(&format!(
        "    \"where_blocks\": {},\n",
        report.syntax_summary.where_blocks
    ));
    json.push_str(&format!(
        "    \"with_blocks\": {},\n",
        report.syntax_summary.with_blocks
    ));
    json.push_str(&format!("    \"tests\": {}\n", report.syntax_summary.tests));
    json.push_str("  },\n");
    json.push_str(&format!(
        "  \"quantity_completion_count\": {},\n",
        report.quantity_completion_count
    ));
    json.push_str(&format!(
        "  \"unit_info_count\": {},\n",
        report.unit_info_count
    ));
    push_review_document_json(&mut json, report);

    json.push_str("  \"variable_table\": [\n");
    for (index, binding) in report.semantic_program.typed_bindings.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        let type_info = report
            .semantic_program
            .type_infos
            .iter()
            .find(|info| info.name == binding.name && info.line == binding.line);
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&binding.name)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&binding.semantic_type.quantity_kind)
        ));
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&binding.semantic_type.display_unit)
        ));
        json.push_str(&format!(
            "      \"canonical_unit\": \"{}\",\n",
            json_escape(
                type_info
                    .map(|info| info.canonical_unit.as_str())
                    .unwrap_or("unknown")
            )
        ));
        json.push_str(&format!(
            "      \"dimension\": \"{}\",\n",
            json_escape(
                type_info
                    .map(|info| info.dimension.as_str())
                    .unwrap_or("unknown")
            )
        ));
        json.push_str(&format!(
            "      \"source\": \"{}\",\n",
            type_info
                .map(|info| info.source.as_str())
                .unwrap_or("runtime")
        ));
        json.push_str(&format!("      \"line\": {}\n", binding.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"diagnostics\": [\n");
    for (index, diagnostic) in report.diagnostics.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"severity\": \"{}\",\n",
            diagnostic.severity.as_str()
        ));
        json.push_str(&format!(
            "      \"code\": \"{}\",\n",
            json_escape(&diagnostic.code)
        ));
        json.push_str(&format!("      \"line\": {},\n", diagnostic.line));
        json.push_str(&format!(
            "      \"message\": \"{}\"",
            json_escape(&diagnostic.message)
        ));
        if let Some(help) = &diagnostic.help {
            json.push_str(&format!(",\n      \"help\": \"{}\"\n", json_escape(help)));
        } else {
            json.push('\n');
        }
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"warning_list\": [\n");
    for (warning_index, diagnostic) in report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .enumerate()
    {
        if warning_index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"code\": \"{}\",\n",
            json_escape(&diagnostic.code)
        ));
        json.push_str(&format!("      \"line\": {},\n", diagnostic.line));
        json.push_str(&format!(
            "      \"message\": \"{}\"",
            json_escape(&diagnostic.message)
        ));
        if let Some(help) = &diagnostic.help {
            json.push_str(&format!(",\n      \"help\": \"{}\"\n", json_escape(help)));
        } else {
            json.push('\n');
        }
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"plot_manifest\": {\n");
    json.push_str("    \"path\": \"plots/plot_manifest.json\",\n");
    json.push_str("    \"producer\": \"eng run\",\n");
    json.push_str("    \"available_in_check\": false\n");
    json.push_str("  },\n");

    let workflow = &report.semantic_program.workflow;
    json.push_str("  \"workflow\": {\n");
    json.push_str(&format!(
        "    \"kind\": \"{}\",\n",
        json_escape(&workflow.kind)
    ));
    json.push_str(&format!(
        "    \"arg_name\": \"{}\",\n",
        json_escape(workflow.arg_name.as_deref().unwrap_or("args"))
    ));
    json.push_str(&format!(
        "    \"arg_type\": \"{}\",\n",
        json_escape(workflow.arg_type.as_deref().unwrap_or("Args"))
    ));
    json.push_str(&format!(
        "    \"return_type\": \"{}\",\n",
        json_escape(workflow.return_type.as_deref().unwrap_or("Report"))
    ));
    json.push_str(&format!("    \"line\": {}\n", workflow.line));
    json.push_str("  },\n");

    json.push_str("  \"imports\": [\n");
    for (index, import) in report.semantic_program.imports.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"target\": \"{}\",\n",
            json_escape(&import.target)
        ));
        json.push_str(&format!(
            "      \"kind\": \"{}\",\n",
            json_escape(&import.kind)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&import.status)
        ));
        json.push_str(&format!("      \"line\": {}\n", import.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"const_summary\": [\n");
    for (index, const_info) in report.semantic_program.consts.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&const_info.name)
        ));
        json.push_str(&format!(
            "      \"type_name\": \"{}\",\n",
            json_escape(&const_info.type_name)
        ));
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&const_info.display_unit)
        ));
        json.push_str(&format!(
            "      \"dimension\": \"{}\",\n",
            json_escape(&const_info.dimension)
        ));
        json.push_str(&format!(
            "      \"expression\": \"{}\",\n",
            json_escape(&const_info.expression)
        ));
        json.push_str(&format!(
            "      \"importable\": {},\n",
            const_info.importable
        ));
        json.push_str(&format!("      \"line\": {}\n", const_info.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"function_summary\": [\n");
    for (index, function) in report.semantic_program.functions.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&function.name)
        ));
        json.push_str("      \"parameters\": [\n");
        for (param_index, parameter) in function.parameters.iter().enumerate() {
            if param_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&parameter.name)
            ));
            json.push_str(&format!(
                "          \"quantity_kind\": \"{}\",\n",
                json_escape(&parameter.quantity_kind)
            ));
            json.push_str(&format!(
                "          \"display_unit\": \"{}\",\n",
                json_escape(&parameter.display_unit)
            ));
            json.push_str(&format!(
                "          \"dimension\": \"{}\"\n",
                json_escape(&parameter.dimension)
            ));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"locals\": [\n");
        for (local_index, local) in function.locals.iter().enumerate() {
            if local_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&local.name)
            ));
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&local.expression)
            ));
            json.push_str(&format!("          \"line\": {}\n", local.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str(&format!(
            "      \"return_quantity_kind\": \"{}\",\n",
            json_escape(&function.return_quantity_kind)
        ));
        json.push_str(&format!(
            "      \"return_display_unit\": \"{}\",\n",
            json_escape(&function.return_display_unit)
        ));
        json.push_str(&format!(
            "      \"return_dimension\": \"{}\",\n",
            json_escape(&function.return_dimension)
        ));
        if let Some(expression) = &function.return_expression {
            json.push_str(&format!(
                "      \"return_expression\": \"{}\",\n",
                json_escape(expression)
            ));
        } else {
            json.push_str("      \"return_expression\": null,\n");
        }
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&function.status)
        ));
        json.push_str(&format!("      \"line\": {}\n", function.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"args_summary\": [\n");
    for (index, args_block) in report.semantic_program.args_blocks.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&args_block.name)
        ));
        json.push_str(&format!("      \"line\": {},\n", args_block.line));
        json.push_str(&format!(
            "      \"field_count\": {},\n",
            args_block.fields.len()
        ));
        json.push_str("      \"fields\": [\n");
        for (field_index, field) in args_block.fields.iter().enumerate() {
            if field_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&field.name)
            ));
            json.push_str(&format!(
                "          \"type\": \"{}\",\n",
                json_escape(&field.type_name)
            ));
            if let Some(default_value) = &field.default_value {
                json.push_str(&format!(
                    "          \"default\": \"{}\",\n",
                    json_escape(default_value)
                ));
            } else {
                json.push_str("          \"default\": null,\n");
            }
            json.push_str(&format!("          \"required\": {},\n", field.required));
            json.push_str(&format!("          \"line\": {}\n", field.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"arg_values\": [\n");
    for (index, arg) in report.semantic_program.arg_values.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&arg.name)
        ));
        json.push_str(&format!(
            "      \"type\": \"{}\",\n",
            json_escape(&arg.type_name)
        ));
        json.push_str(&format!(
            "      \"value\": \"{}\",\n",
            json_escape(&arg.value)
        ));
        json.push_str(&format!(
            "      \"source\": \"{}\",\n",
            json_escape(&arg.source)
        ));
        json.push_str(&format!("      \"required\": {},\n", arg.required));
        json.push_str(&format!("      \"line\": {}\n", arg.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"environment_dependencies\": [\n");
    for (index, dependency) in report
        .semantic_program
        .environment_dependencies
        .iter()
        .enumerate()
    {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&dependency.name)
        ));
        json.push_str(&format!(
            "      \"kind\": \"{}\",\n",
            json_escape(&dependency.kind)
        ));
        json.push_str(&format!(
            "      \"expression\": \"{}\",\n",
            json_escape(&dependency.expression)
        ));
        json.push_str(&format!(
            "      \"resolved_value\": \"{}\",\n",
            json_escape(&dependency.resolved_value)
        ));
        match &dependency.source_hash {
            Some(source_hash) => json.push_str(&format!(
                "      \"source_hash\": \"{}\",\n",
                json_escape(source_hash)
            )),
            None => json.push_str("      \"source_hash\": null,\n"),
        }
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&dependency.status)
        ));
        json.push_str(&format!("      \"line\": {}\n", dependency.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    push_net_requests_json(&mut json, report, 2);
    push_net_downloads_json(&mut json, report, 2);
    push_cache_records_json(&mut json, report, 2);

    json.push_str("  \"inferred_declarations\": [\n");
    for (index, declaration) in report.inferred_declarations.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&declaration.name)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&declaration.quantity_kind)
        ));
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&declaration.display_unit)
        ));
        json.push_str(&format!("      \"line\": {},\n", declaration.line));
        json.push_str(&format!(
            "      \"expression\": \"{}\"\n",
            json_escape(&declaration.expression)
        ));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"expected_types\": [\n");
    for (index, expected_type) in report.semantic_program.expected_types.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&expected_type.name)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&expected_type.quantity_kind)
        ));
        if let Some(unit) = &expected_type.display_unit {
            json.push_str(&format!(
                "      \"display_unit\": \"{}\",\n",
                json_escape(unit)
            ));
        } else {
            json.push_str("      \"display_unit\": null,\n");
        }
        json.push_str(&format!(
            "      \"source\": \"{}\",\n",
            expected_type.source.as_str()
        ));
        json.push_str(&format!("      \"line\": {}\n", expected_type.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"hover_hints\": [\n");
    for (index, hover) in report.semantic_program.hover_hints.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&hover.name)
        ));
        json.push_str(&format!("      \"line\": {},\n", hover.line));
        json.push_str(&format!("      \"column\": {},\n", hover.column));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&hover.quantity_kind)
        ));
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&hover.display_unit)
        ));
        json.push_str(&format!(
            "      \"detail\": \"{}\"\n",
            json_escape(&hover.detail)
        ));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"type_info\": [\n");
    for (index, info) in report.semantic_program.type_infos.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&info.name)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&info.quantity_kind)
        ));
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&info.display_unit)
        ));
        json.push_str(&format!(
            "      \"canonical_unit\": \"{}\",\n",
            json_escape(&info.canonical_unit)
        ));
        json.push_str(&format!(
            "      \"dimension\": \"{}\",\n",
            json_escape(&info.dimension)
        ));
        json.push_str(&format!(
            "      \"source\": \"{}\",\n",
            info.source.as_str()
        ));
        json.push_str(&format!("      \"line\": {}\n", info.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"state_space_vectors\": [\n");
    for (index, vector) in report
        .semantic_program
        .state_space_vectors
        .iter()
        .enumerate()
    {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"system\": \"{}\",\n",
            json_escape(&vector.system)
        ));
        json.push_str(&format!(
            "      \"role\": \"{}\",\n",
            json_escape(&vector.role)
        ));
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&vector.name)
        ));
        json.push_str(&format!(
            "      \"vector_type\": \"{}\",\n",
            json_escape(&vector.vector_type)
        ));
        json.push_str("      \"members\": [");
        for (member_index, member) in vector.members.iter().enumerate() {
            if member_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(member)));
        }
        json.push_str("],\n");
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&vector.status)
        ));
        json.push_str(&format!("      \"line\": {}\n", vector.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"linear_operators\": [\n");
    for (index, operator) in report.semantic_program.linear_operators.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"system\": \"{}\",\n",
            json_escape(&operator.system)
        ));
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&operator.name)
        ));
        json.push_str(&format!(
            "      \"from\": \"{}\",\n",
            json_escape(&operator.from)
        ));
        json.push_str(&format!(
            "      \"to\": \"{}\",\n",
            json_escape(&operator.to)
        ));
        push_optional_json_string(&mut json, "expression", operator.expression.as_deref(), 6);
        json.push_str("      \"canonical_matrix\": ");
        push_optional_json_matrix(&mut json, operator.canonical_matrix.as_deref());
        json.push_str(",\n");
        push_linear_operator_entries_json(&mut json, &operator.canonical_entries, 6);
        json.push_str(&format!("      \"row_count\": {},\n", operator.row_count));
        json.push_str(&format!(
            "      \"column_count\": {},\n",
            operator.column_count
        ));
        push_named_json_string_array(&mut json, "row_members", &operator.row_members, 6);
        push_named_json_string_array(&mut json, "column_members", &operator.column_members, 6);
        push_named_json_string_array(
            &mut json,
            "row_quantity_kinds",
            &operator.row_quantity_kinds,
            6,
        );
        push_named_json_string_array(
            &mut json,
            "column_quantity_kinds",
            &operator.column_quantity_kinds,
            6,
        );
        push_named_json_string_array(&mut json, "row_units", &operator.row_units, 6);
        push_named_json_string_array(&mut json, "column_units", &operator.column_units, 6);
        json.push_str(&format!(
            "      \"compatibility_status\": \"{}\",\n",
            json_escape(&operator.compatibility_status)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&operator.status)
        ));
        json.push_str(&format!("      \"line\": {}\n", operator.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"unit_derivations\": [\n");
    for (index, derivation) in report.semantic_program.unit_derivations.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&derivation.name)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&derivation.quantity_kind)
        ));
        if let Some(source_unit) = &derivation.source_unit {
            json.push_str(&format!(
                "      \"source_unit\": \"{}\",\n",
                json_escape(source_unit)
            ));
        } else {
            json.push_str("      \"source_unit\": null,\n");
        }
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&derivation.display_unit)
        ));
        json.push_str(&format!(
            "      \"canonical_unit\": \"{}\",\n",
            json_escape(&derivation.canonical_unit)
        ));
        json.push_str(&format!("      \"line\": {},\n", derivation.line));
        json.push_str("      \"steps\": [");
        for (step_index, step) in derivation.steps.iter().enumerate() {
            if step_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(step)));
        }
        json.push_str("]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"unit_conversion_table\": [\n");
    for (index, derivation) in report.semantic_program.unit_derivations.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&derivation.name)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&derivation.quantity_kind)
        ));
        if let Some(source_unit) = &derivation.source_unit {
            json.push_str(&format!(
                "      \"source_unit\": \"{}\",\n",
                json_escape(source_unit)
            ));
        } else {
            json.push_str("      \"source_unit\": null,\n");
        }
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&derivation.display_unit)
        ));
        json.push_str(&format!(
            "      \"canonical_unit\": \"{}\",\n",
            json_escape(&derivation.canonical_unit)
        ));
        json.push_str(&format!("      \"line\": {},\n", derivation.line));
        json.push_str("      \"steps\": [");
        for (step_index, step) in derivation.steps.iter().enumerate() {
            if step_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(step)));
        }
        json.push_str("]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"axis_info\": [\n");
    for (index, axis) in report.semantic_program.axis_infos.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&axis.binding)
        ));
        json.push_str(&format!(
            "      \"axis\": \"{}\",\n",
            json_escape(&axis.axis)
        ));
        json.push_str(&format!(
            "      \"role\": \"{}\",\n",
            json_escape(&axis.role)
        ));
        json.push_str(&format!(
            "      \"source\": \"{}\",\n",
            json_escape(&axis.source)
        ));
        json.push_str(&format!("      \"line\": {}\n", axis.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"stats_info\": [\n");
    for (index, stats) in report.semantic_program.stats_infos.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"source\": \"{}\",\n",
            json_escape(&stats.source)
        ));
        json.push_str(&format!(
            "      \"source_type\": \"{}\",\n",
            json_escape(&stats.source_type)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&stats.quantity_kind)
        ));
        json.push_str(&format!(
            "      \"axis\": \"{}\",\n",
            json_escape(&stats.axis)
        ));
        json.push_str("      \"statistics\": [");
        for (stat_index, statistic) in stats.statistics.iter().enumerate() {
            if stat_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(statistic)));
        }
        json.push_str("],\n");
        json.push_str(&format!(
            "      \"cache_key\": \"{}\",\n",
            json_escape(&stats.cache_key)
        ));
        json.push_str(&format!("      \"line\": {}\n", stats.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"integrations\": [\n");
    for (index, integration) in report.semantic_program.integrations.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&integration.binding)
        ));
        json.push_str(&format!(
            "      \"source\": \"{}\",\n",
            json_escape(&integration.source)
        ));
        json.push_str(&format!(
            "      \"input_quantity\": \"{}\",\n",
            json_escape(&integration.input_quantity)
        ));
        json.push_str(&format!(
            "      \"over_axis\": \"{}\",\n",
            json_escape(&integration.over_axis)
        ));
        json.push_str(&format!(
            "      \"result_quantity\": \"{}\",\n",
            json_escape(&integration.result_quantity)
        ));
        json.push_str(&format!("      \"line\": {}\n", integration.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"prints\": [\n");
    for (index, print) in report.semantic_program.prints.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"level\": \"{}\",\n",
            json_escape(&print.level)
        ));
        json.push_str(&format!(
            "      \"template\": \"{}\",\n",
            json_escape(&print.template)
        ));
        json.push_str(&format!("      \"line\": {},\n", print.line));
        json.push_str("      \"fields\": [\n");
        for (field_index, field) in print.fields.iter().enumerate() {
            if field_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&field.expression)
            ));
            json.push_str(&format!(
                "          \"quantity_kind\": \"{}\",\n",
                json_escape(&field.quantity_kind)
            ));
            json.push_str(&format!(
                "          \"display_unit\": \"{}\",\n",
                json_escape(&field.display_unit)
            ));
            push_optional_json_string(
                &mut json,
                "requested_unit",
                field.requested_unit.as_deref(),
                10,
            );
            if let Some(precision) = field.precision {
                json.push_str(&format!("          \"precision\": {},\n", precision));
            } else {
                json.push_str("          \"precision\": null,\n");
            }
            json.push_str(&format!("          \"line\": {}\n", field.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"csv_exports\": [\n");
    for (index, export) in report.semantic_program.csv_exports.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"source\": \"{}\",\n",
            json_escape(&export.source)
        ));
        json.push_str(&format!(
            "      \"format\": \"{}\",\n",
            json_escape(&export.format)
        ));
        json.push_str(&format!(
            "      \"path\": \"{}\",\n",
            json_escape(&export.path)
        ));
        json.push_str(&format!("      \"line\": {},\n", export.line));
        json.push_str("      \"fields\": [\n");
        for (field_index, field) in export.fields.iter().enumerate() {
            if field_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&field.name)
            ));
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&field.expression)
            ));
            json.push_str(&format!(
                "          \"quantity_kind\": \"{}\",\n",
                json_escape(&field.quantity_kind)
            ));
            json.push_str(&format!(
                "          \"display_unit\": \"{}\",\n",
                json_escape(&field.display_unit)
            ));
            push_optional_json_string(
                &mut json,
                "requested_unit",
                field.requested_unit.as_deref(),
                10,
            );
            if let Some(precision) = field.precision {
                json.push_str(&format!("          \"precision\": {},\n", precision));
            } else {
                json.push_str("          \"precision\": null,\n");
            }
            json.push_str(&format!("          \"line\": {}\n", field.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"writes\": [\n");
    for (index, write) in report.semantic_program.writes.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"format\": \"{}\",\n",
            json_escape(&write.format)
        ));
        json.push_str(&format!(
            "      \"path\": \"{}\",\n",
            json_escape(&write.path)
        ));
        json.push_str(&format!(
            "      \"expression\": \"{}\",\n",
            json_escape(&write.expression)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&write.quantity_kind)
        ));
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&write.display_unit)
        ));
        json.push_str(&format!("      \"line\": {}\n", write.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"file_operations\": [\n");
    for (index, operation) in report.semantic_program.file_operations.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"operation\": \"{}\",\n",
            json_escape(&operation.operation)
        ));
        json.push_str(&format!(
            "      \"source\": \"{}\",\n",
            json_escape(&operation.source)
        ));
        push_optional_json_string(
            &mut json,
            "destination",
            operation.destination.as_deref(),
            6,
        );
        json.push_str(&format!("      \"line\": {}\n", operation.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"process_runs\": [\n");
    for (index, process) in report.semantic_program.process_runs.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&process.binding)
        ));
        json.push_str(&format!(
            "      \"command\": \"{}\",\n",
            json_escape(&process.command)
        ));
        let tool_version = review_option_values(report, process.line, "tool_version")
            .into_iter()
            .next();
        push_optional_json_string(&mut json, "tool_version", tool_version.as_deref(), 6);
        let expected_outputs = review_option_values(report, process.line, "expected_outputs");
        json.push_str("      \"expected_outputs\": [");
        push_json_string_array(&mut json, &expected_outputs);
        json.push_str("],\n");
        json.push_str(&format!("      \"line\": {}\n", process.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"tests\": [\n");
    for (index, test) in report.semantic_program.tests.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&test.name)
        ));
        json.push_str(&format!("      \"line\": {},\n", test.line));
        json.push_str("      \"assertions\": [\n");
        for (assert_index, assertion) in test.assertions.iter().enumerate() {
            if assert_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"left\": \"{}\",\n",
                json_escape(&assertion.left)
            ));
            json.push_str(&format!(
                "          \"operator\": \"{}\",\n",
                json_escape(&assertion.operator)
            ));
            json.push_str(&format!(
                "          \"right\": \"{}\",\n",
                json_escape(&assertion.right)
            ));
            push_optional_json_string(&mut json, "tolerance", assertion.tolerance.as_deref(), 10);
            json.push_str(&format!("          \"line\": {}\n", assertion.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"goldens\": [\n");
        for (golden_index, golden) in test.goldens.iter().enumerate() {
            if golden_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"artifact\": \"{}\",\n",
                json_escape(&golden.artifact)
            ));
            json.push_str(&format!(
                "          \"expected\": \"{}\",\n",
                json_escape(&golden.expected)
            ));
            json.push_str(&format!("          \"line\": {}\n", golden.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"command_styles\": [\n");
    for (index, command) in report.semantic_program.command_styles.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"verb\": \"{}\",\n",
            json_escape(&command.verb)
        ));
        json.push_str(&format!(
            "      \"target\": \"{}\",\n",
            json_escape(&command.target)
        ));
        json.push_str("      \"clauses\": [");
        for (clause_index, clause) in command.clauses.iter().enumerate() {
            if clause_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!(
                "{{\"name\": \"{}\", \"value\": \"{}\"}}",
                json_escape(&clause.name),
                json_escape(&clause.value)
            ));
        }
        json.push_str("],\n");
        json.push_str(&format!(
            "      \"canonical\": \"{}\",\n",
            json_escape(&command.canonical)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&command.status)
        ));
        push_optional_json_string(&mut json, "owner", command.owner.as_deref(), 6);
        json.push_str(&format!("      \"line\": {}\n", command.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"where_blocks\": [\n");
    for (index, block) in report.semantic_program.where_blocks.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        match block.owner_line {
            Some(owner_line) => json.push_str(&format!("      \"owner_line\": {},\n", owner_line)),
            None => json.push_str("      \"owner_line\": null,\n"),
        }
        json.push_str(&format!("      \"line\": {},\n", block.line));
        json.push_str("      \"bindings\": [\n");
        for (binding_index, binding) in block.bindings.iter().enumerate() {
            if binding_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&binding.name)
            ));
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&binding.expression)
            ));
            json.push_str(&format!(
                "          \"quantity_kind\": \"{}\",\n",
                json_escape(&binding.quantity_kind)
            ));
            json.push_str(&format!(
                "          \"display_unit\": \"{}\",\n",
                json_escape(&binding.display_unit)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&binding.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", binding.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"with_blocks\": [\n");
    for (index, block) in report.semantic_program.with_blocks.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        match block.owner_line {
            Some(owner_line) => json.push_str(&format!("      \"owner_line\": {},\n", owner_line)),
            None => json.push_str("      \"owner_line\": null,\n"),
        }
        json.push_str(&format!("      \"line\": {},\n", block.line));
        json.push_str("      \"options\": [\n");
        for (option_index, option) in block.options.iter().enumerate() {
            if option_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"key\": \"{}\",\n",
                json_escape(&option.key)
            ));
            json.push_str(&format!(
                "          \"value\": \"{}\",\n",
                json_escape(&option.value)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&option.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", option.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    push_uncertainty_policies_json(&mut json, report);
    push_timeseries_uncertainty_json(&mut json, report);
    push_timeseries_uncertainty_calculations_json(&mut json, report);
    push_simulation_requests_json(&mut json, report);
    json.push_str("  \"timeseries_kernels\": [\n");
    for (index, kernel) in report
        .semantic_program
        .timeseries_kernels
        .iter()
        .enumerate()
    {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&kernel.binding)
        ));
        json.push_str(&format!(
            "      \"kind\": \"{}\",\n",
            json_escape(&kernel.kind)
        ));
        push_optional_json_string(&mut json, "source_table", kernel.source_table.as_deref(), 6);
        json.push_str(&format!(
            "      \"axis\": \"{}\",\n",
            json_escape(&kernel.axis)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&kernel.quantity_kind)
        ));
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&kernel.display_unit)
        ));
        json.push_str(&format!(
            "      \"expression\": \"{}\",\n",
            json_escape(&kernel.expression)
        ));
        json.push_str("      \"operations\": [");
        push_json_string_array(&mut json, &kernel.operations);
        json.push_str("],\n");
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&kernel.status)
        ));
        json.push_str(&format!("      \"line\": {}\n", kernel.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"uncertainty_info\": [\n");
    for (index, uncertainty) in report.semantic_program.uncertainty_infos.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&uncertainty.binding)
        ));
        json.push_str(&format!(
            "      \"kind\": \"{}\",\n",
            json_escape(&uncertainty.kind)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&uncertainty.quantity_kind)
        ));
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&uncertainty.display_unit)
        ));
        json.push_str(&format!(
            "      \"expression\": \"{}\",\n",
            json_escape(&uncertainty.expression)
        ));
        push_optional_json_string(&mut json, "source", uncertainty.source.as_deref(), 6);
        push_optional_json_string(
            &mut json,
            "distribution",
            uncertainty.distribution.as_deref(),
            6,
        );
        push_optional_json_string(&mut json, "method", uncertainty.method.as_deref(), 6);
        push_optional_json_string(&mut json, "scale", uncertainty.scale.as_deref(), 6);
        push_optional_json_string(&mut json, "offset", uncertainty.offset.as_deref(), 6);
        push_optional_json_string(&mut json, "mean", uncertainty.mean.as_deref(), 6);
        push_optional_json_string(&mut json, "stddev", uncertainty.stddev.as_deref(), 6);
        push_optional_json_string(&mut json, "error", uncertainty.error.as_deref(), 6);
        push_optional_json_string(&mut json, "lower", uncertainty.lower.as_deref(), 6);
        push_optional_json_string(&mut json, "upper", uncertainty.upper.as_deref(), 6);
        json.push_str(&format!(
            "      \"sample_count\": {},\n",
            uncertainty.sample_count
        ));
        json.push_str("      \"propagation\": [");
        for (term_index, term) in uncertainty.propagation.iter().enumerate() {
            if term_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!(
                "{{ \"source\": \"{}\", \"role\": \"{}\", \"quantity_kind\": \"{}\" }}",
                json_escape(&term.source),
                json_escape(&term.role),
                json_escape(&term.quantity_kind)
            ));
        }
        json.push_str("],\n");
        json.push_str(&format!("      \"line\": {}\n", uncertainty.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    push_uncertainty_summary_json(&mut json, report);
    push_uncertainty_propagation_json(&mut json, report);
    json.push_str("  \"ml_info\": [\n");
    for (index, ml) in report.semantic_program.ml_infos.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&ml.binding)
        ));
        json.push_str(&format!("      \"kind\": \"{}\",\n", json_escape(&ml.kind)));
        push_optional_json_string(&mut json, "source", ml.source.as_deref(), 6);
        push_optional_json_string(&mut json, "target", ml.target.as_deref(), 6);
        json.push_str("      \"features\": [");
        for (feature_index, feature) in ml.features.iter().enumerate() {
            if feature_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(feature)));
        }
        json.push_str("],\n");
        push_optional_json_string(&mut json, "algorithm", ml.algorithm.as_deref(), 6);
        push_optional_json_string(&mut json, "test_fraction", ml.test_fraction.as_deref(), 6);
        push_optional_json_string(&mut json, "seed", ml.seed.as_deref(), 6);
        json.push_str("      \"hidden_layers\": [");
        for (layer_index, layer) in ml.hidden_layers.iter().enumerate() {
            if layer_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&layer.to_string());
        }
        json.push_str("],\n");
        match ml.epochs {
            Some(epochs) => json.push_str(&format!("      \"epochs\": {},\n", epochs)),
            None => json.push_str("      \"epochs\": null,\n"),
        }
        json.push_str(&format!(
            "      \"expression\": \"{}\",\n",
            json_escape(&ml.expression)
        ));
        json.push_str(&format!("      \"line\": {}\n", ml.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"domain_summary\": [\n");
    for (index, domain) in report.semantic_program.domains.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&domain.name)
        ));
        json.push_str("      \"type_parameters\": [");
        for (parameter_index, parameter) in domain.type_parameters.iter().enumerate() {
            if parameter_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!(
                "{{\"kind\": \"{}\", \"name\": \"{}\", \"display\": \"{}\"}}",
                json_escape(&parameter.kind),
                json_escape(&parameter.name),
                json_escape(&parameter.display)
            ));
        }
        json.push_str("],\n");
        match &domain.package {
            Some(package) => json.push_str(&format!(
                "      \"package\": \"{}\",\n",
                json_escape(package)
            )),
            None => json.push_str("      \"package\": null,\n"),
        }
        match &domain.version {
            Some(version) => json.push_str(&format!(
                "      \"version\": \"{}\",\n",
                json_escape(version)
            )),
            None => json.push_str("      \"version\": null,\n"),
        }
        json.push_str(&format!("      \"line\": {},\n", domain.line));
        json.push_str(&format!(
            "      \"variable_count\": {},\n",
            domain.variables.len()
        ));
        json.push_str(&format!(
            "      \"conservation_count\": {},\n",
            domain.conservations.len()
        ));
        json.push_str("      \"variables\": [\n");
        for (variable_index, variable) in domain.variables.iter().enumerate() {
            if variable_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"role\": \"{}\",\n",
                json_escape(&variable.role)
            ));
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&variable.name)
            ));
            json.push_str(&format!(
                "          \"quantity_kind\": \"{}\",\n",
                json_escape(&variable.quantity_kind)
            ));
            json.push_str(&format!(
                "          \"display_unit\": \"{}\",\n",
                json_escape(&variable.display_unit)
            ));
            json.push_str(&format!(
                "          \"canonical_unit\": \"{}\",\n",
                json_escape(&variable.canonical_unit)
            ));
            json.push_str(&format!(
                "          \"dimension\": \"{}\",\n",
                json_escape(&variable.dimension)
            ));
            json.push_str(&format!("          \"line\": {}\n", variable.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"conservations\": [\n");
        for (conservation_index, conservation) in domain.conservations.iter().enumerate() {
            if conservation_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"text\": \"{}\",\n",
                json_escape(&conservation.text)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&conservation.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", conservation.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"component_summary\": [\n");
    for (index, component) in report.semantic_program.components.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&component.name)
        ));
        push_optional_json_string(
            &mut json,
            "template_name",
            component.template_name.as_deref(),
            6,
        );
        json.push_str("      \"constructor_arguments\": [\n");
        for (argument_index, argument) in component.constructor_arguments.iter().enumerate() {
            if argument_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&argument.name)
            ));
            json.push_str(&format!(
                "          \"value\": \"{}\"\n",
                json_escape(&argument.value)
            ));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"parameters\": [\n");
        for (parameter_index, parameter) in component.parameters.iter().enumerate() {
            if parameter_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&parameter.name)
            ));
            json.push_str(&format!(
                "          \"quantity_kind\": \"{}\",\n",
                json_escape(&parameter.quantity_kind)
            ));
            json.push_str(&format!(
                "          \"display_unit\": \"{}\",\n",
                json_escape(&parameter.display_unit)
            ));
            json.push_str(&format!(
                "          \"canonical_unit\": \"{}\",\n",
                json_escape(&parameter.canonical_unit)
            ));
            push_optional_json_string(
                &mut json,
                "default_value",
                parameter.default_value.as_deref(),
                10,
            );
            push_optional_json_string(&mut json, "value", parameter.value.as_deref(), 10);
            json.push_str(&format!(
                "          \"source\": \"{}\",\n",
                json_escape(&parameter.source)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&parameter.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", parameter.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str(&format!("      \"line\": {},\n", component.line));
        json.push_str(&format!(
            "      \"parameter_count\": {},\n",
            component.parameters.len()
        ));
        json.push_str(&format!(
            "      \"port_count\": {},\n",
            component.ports.len()
        ));
        json.push_str(&format!(
            "      \"local_expression_count\": {},\n",
            component.local_expressions.len()
        ));
        json.push_str("      \"ports\": [\n");
        for (port_index, port) in component.ports.iter().enumerate() {
            if port_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&port.name)
            ));
            json.push_str(&format!(
                "          \"domain\": \"{}\",\n",
                json_escape(&port.domain)
            ));
            json.push_str(&format!(
                "          \"domain_name\": \"{}\",\n",
                json_escape(&port.domain_name)
            ));
            json.push_str("          \"type_arguments\": [");
            for (argument_index, argument) in port.type_arguments.iter().enumerate() {
                if argument_index > 0 {
                    json.push_str(", ");
                }
                json.push_str(&format!("\"{}\"", json_escape(argument)));
            }
            json.push_str("],\n");
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&port.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", port.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"local_expressions\": [\n");
        for (local_index, local) in component.local_expressions.iter().enumerate() {
            if local_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&local.name)
            ));
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&local.expression)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&local.status)
            ));
            json.push_str(&format!(
                "          \"quantity_kind\": \"{}\",\n",
                json_escape(&local.quantity_kind)
            ));
            json.push_str(&format!(
                "          \"display_unit\": \"{}\",\n",
                json_escape(&local.display_unit)
            ));
            json.push_str(&format!(
                "          \"canonical_unit\": \"{}\",\n",
                json_escape(&local.canonical_unit)
            ));
            json.push_str(&format!(
                "          \"type_status\": \"{}\",\n",
                json_escape(&local.type_status)
            ));
            json.push_str(&format!("          \"line\": {}\n", local.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"connection_summary\": [\n");
    for (index, connection) in report.semantic_program.connections.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"left\": \"{}\",\n",
            json_escape(&connection.left)
        ));
        json.push_str(&format!(
            "      \"right\": \"{}\",\n",
            json_escape(&connection.right)
        ));
        json.push_str(&format!(
            "      \"domain\": \"{}\",\n",
            json_escape(&connection.domain)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&connection.status)
        ));
        json.push_str(&format!("      \"line\": {}\n", connection.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"assembly_summary\": [\n");
    for (index, assembly) in report
        .semantic_program
        .component_assemblies
        .iter()
        .enumerate()
    {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&assembly.name)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&assembly.status)
        ));
        json.push_str(&format!("      \"line\": {},\n", assembly.line));
        json.push_str(&format!(
            "      \"component_count\": {},\n",
            assembly.component_count
        ));
        json.push_str(&format!("      \"port_count\": {},\n", assembly.port_count));
        json.push_str(&format!(
            "      \"connection_count\": {},\n",
            assembly.connection_count
        ));
        json.push_str(&format!(
            "      \"component_equation_count\": {},\n",
            assembly.component_equation_count
        ));
        json.push_str(&format!(
            "      \"local_expression_count\": {},\n",
            assembly.local_expression_count
        ));
        json.push_str(&format!(
            "      \"operator_call_count\": {},\n",
            assembly.operator_call_count
        ));
        json.push_str(&format!(
            "      \"predictor_call_count\": {},\n",
            assembly.predictor_call_count
        ));
        json.push_str(&format!(
            "      \"domain_count\": {},\n",
            assembly.domain_count
        ));
        json.push_str("      \"domain_plans\": [\n");
        for (plan_index, plan) in assembly.domain_plans.iter().enumerate() {
            if plan_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"domain\": \"{}\",\n",
                json_escape(&plan.domain)
            ));
            json.push_str(&format!(
                "          \"connection_set_count\": {},\n",
                plan.connection_set_count
            ));
            json.push_str(&format!(
                "          \"equation_count\": {},\n",
                plan.equation_count
            ));
            json.push_str(&format!(
                "          \"variable_count\": {},\n",
                plan.variable_count
            ));
            json.push_str(&format!(
                "          \"conservation_status\": \"{}\",\n",
                json_escape(&plan.conservation_status)
            ));
            json.push_str(&format!(
                "          \"solver_role\": \"{}\"\n",
                json_escape(&plan.solver_role)
            ));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"solver_preview\": {\n");
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&assembly.solver_preview.status)
        ));
        json.push_str(&format!(
            "        \"method\": \"{}\",\n",
            json_escape(&assembly.solver_preview.method)
        ));
        json.push_str(&format!(
            "        \"mixed_algebraic_dynamic\": \"{}\",\n",
            json_escape(&assembly.solver_preview.mixed_algebraic_dynamic)
        ));
        json.push_str(&format!(
            "        \"nonlinear_residual\": \"{}\",\n",
            json_escape(&assembly.solver_preview.nonlinear_residual)
        ));
        json.push_str(&format!(
            "        \"dae_split\": \"{}\",\n",
            json_escape(&assembly.solver_preview.dae_split)
        ));
        json.push_str(&format!(
            "        \"delay_history\": \"{}\",\n",
            json_escape(&assembly.solver_preview.delay_history)
        ));
        json.push_str(&format!(
            "        \"predictor\": \"{}\",\n",
            json_escape(&assembly.solver_preview.predictor)
        ));
        json.push_str(&format!(
            "        \"external_adapter\": \"{}\",\n",
            json_escape(&assembly.solver_preview.external_adapter)
        ));
        json.push_str("        \"limitations\": [");
        for (limitation_index, limitation) in assembly.solver_preview.limitations.iter().enumerate()
        {
            if limitation_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(limitation)));
        }
        json.push_str("]\n");
        json.push_str("      },\n");
        json.push_str("      \"connection_sets\": [\n");
        for (set_index, connection_set) in assembly.connection_sets.iter().enumerate() {
            if set_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&connection_set.name)
            ));
            json.push_str(&format!(
                "          \"domain\": \"{}\",\n",
                json_escape(&connection_set.domain)
            ));
            json.push_str(&format!(
                "          \"connection_count\": {},\n",
                connection_set.connection_count
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&connection_set.status)
            ));
            json.push_str(&format!("          \"line\": {},\n", connection_set.line));
            json.push_str("          \"ports\": [");
            for (port_index, port) in connection_set.ports.iter().enumerate() {
                if port_index > 0 {
                    json.push_str(", ");
                }
                json.push_str(&format!("\"{}\"", json_escape(port)));
            }
            json.push_str("]\n");
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"equations\": [\n");
        for (equation_index, equation) in assembly.equations.iter().enumerate() {
            if equation_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&equation.name)
            ));
            json.push_str(&format!(
                "          \"kind\": \"{}\",\n",
                json_escape(&equation.kind)
            ));
            json.push_str(&format!(
                "          \"domain\": \"{}\",\n",
                json_escape(&equation.domain)
            ));
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&equation.expression)
            ));
            json.push_str(&format!(
                "          \"residual\": \"{}\",\n",
                json_escape(&equation.residual)
            ));
            json.push_str(&format!(
                "          \"reason\": \"{}\",\n",
                json_escape(&equation.reason)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&equation.status)
            ));
            json.push_str(&format!("          \"line\": {},\n", equation.line));
            json.push_str("          \"dependencies\": [");
            for (dependency_index, dependency) in equation.dependencies.iter().enumerate() {
                if dependency_index > 0 {
                    json.push_str(", ");
                }
                json.push_str(&format!("\"{}\"", json_escape(dependency)));
            }
            json.push_str("]\n");
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"variables\": [\n");
        for (variable_index, variable) in assembly.variables.iter().enumerate() {
            if variable_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&variable.name)
            ));
            json.push_str(&format!(
                "          \"role\": \"{}\",\n",
                json_escape(&variable.role)
            ));
            json.push_str(&format!(
                "          \"domain\": \"{}\",\n",
                json_escape(&variable.domain)
            ));
            json.push_str(&format!(
                "          \"source\": \"{}\",\n",
                json_escape(&variable.source)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\"\n",
                json_escape(&variable.status)
            ));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"boundary\": {\n");
        json.push_str(&format!(
            "        \"state_count\": {},\n",
            assembly.boundary.state_count
        ));
        json.push_str(&format!(
            "        \"algebraic_count\": {},\n",
            assembly.boundary.algebraic_count
        ));
        json.push_str(&format!(
            "        \"input_count\": {},\n",
            assembly.boundary.input_count
        ));
        json.push_str(&format!(
            "        \"output_count\": {},\n",
            assembly.boundary.output_count
        ));
        json.push_str(&format!(
            "        \"parameter_count\": {},\n",
            assembly.boundary.parameter_count
        ));
        json.push_str(&format!(
            "        \"equation_count\": {},\n",
            assembly.boundary.equation_count
        ));
        json.push_str(&format!(
            "        \"unknown_count\": {},\n",
            assembly.boundary.unknown_count
        ));
        json.push_str(&format!(
            "        \"balance_status\": \"{}\",\n",
            json_escape(&assembly.boundary.balance_status)
        ));
        match &assembly.boundary.diagnostic_code {
            Some(code) => json.push_str(&format!(
                "        \"diagnostic_code\": \"{}\"\n",
                json_escape(code)
            )),
            None => json.push_str("        \"diagnostic_code\": null\n"),
        }
        json.push_str("      },\n");
        json.push_str("      \"residual_graph\": {\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&assembly.residual_graph.name)
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&assembly.residual_graph.status)
        ));
        json.push_str(&format!(
            "        \"solver_plan\": \"{}\",\n",
            json_escape(&assembly.residual_graph.solver_plan)
        ));
        json.push_str("        \"residuals\": [");
        for (residual_index, residual) in assembly.residual_graph.residuals.iter().enumerate() {
            if residual_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(residual)));
        }
        json.push_str("],\n");
        json.push_str("        \"residual_metadata\": [\n");
        for (metadata_index, metadata) in
            assembly.residual_graph.residual_metadata.iter().enumerate()
        {
            if metadata_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"name\": \"{}\",\n",
                json_escape(&metadata.name)
            ));
            json.push_str(&format!(
                "            \"kind\": \"{}\",\n",
                json_escape(&metadata.kind)
            ));
            json.push_str(&format!(
                "            \"domain\": \"{}\",\n",
                json_escape(&metadata.domain)
            ));
            json.push_str(&format!(
                "            \"source_expression\": \"{}\",\n",
                json_escape(&metadata.source_expression)
            ));
            json.push_str(&format!(
                "            \"residual_expression\": \"{}\",\n",
                json_escape(&metadata.residual_expression)
            ));
            match &metadata.rhs {
                Some(rhs) => {
                    json.push_str(&format!("            \"rhs\": \"{}\",\n", json_escape(rhs)))
                }
                None => json.push_str("            \"rhs\": null,\n"),
            }
            json.push_str("            \"dependencies\": [");
            for (dependency_index, dependency) in metadata.dependencies.iter().enumerate() {
                if dependency_index > 0 {
                    json.push_str(", ");
                }
                json.push_str(&format!("\"{}\"", json_escape(dependency)));
            }
            json.push_str("],\n");
            json.push_str(&format!(
                "            \"status\": \"{}\",\n",
                json_escape(&metadata.status)
            ));
            json.push_str(&format!("            \"line\": {}\n", metadata.line));
            json.push_str("          }");
        }
        json.push_str("\n        ],\n");
        json.push_str("        \"dependencies\": [\n");
        for (dependency_index, dependency) in
            assembly.residual_graph.dependencies.iter().enumerate()
        {
            if dependency_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"residual\": \"{}\",\n",
                json_escape(&dependency.residual)
            ));
            json.push_str(&format!(
                "            \"variable\": \"{}\"\n",
                json_escape(&dependency.variable)
            ));
            json.push_str("          }");
        }
        json.push_str("\n        ],\n");
        json.push_str("        \"algebraic_loops\": [\n");
        for (loop_index, algebraic_loop) in
            assembly.residual_graph.algebraic_loops.iter().enumerate()
        {
            if loop_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          [");
            for (variable_index, variable) in algebraic_loop.iter().enumerate() {
                if variable_index > 0 {
                    json.push_str(", ");
                }
                json.push_str(&format!("\"{}\"", json_escape(variable)));
            }
            json.push(']');
        }
        json.push_str("\n        ],\n");
        json.push_str("        \"jacobian_sparsity\": [\n");
        for (seed_index, seed) in assembly.residual_graph.jacobian_sparsity.iter().enumerate() {
            if seed_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"residual\": \"{}\",\n",
                json_escape(&seed.residual)
            ));
            json.push_str(&format!(
                "            \"status\": \"{}\",\n",
                json_escape(&seed.status)
            ));
            json.push_str("            \"with_respect_to\": [");
            for (variable_index, variable) in seed.with_respect_to.iter().enumerate() {
                if variable_index > 0 {
                    json.push_str(", ");
                }
                json.push_str(&format!("\"{}\"", json_escape(variable)));
            }
            json.push_str("]\n");
            json.push_str("          }");
        }
        json.push_str("\n        ]\n");
        json.push_str("      }\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    write_component_graph_json(&mut json, &report.semantic_program);
    json.push_str(",\n");
    json.push_str("  \"class_summary\": [\n");
    for (index, class_info) in report.semantic_program.classes.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&class_info.name)
        ));
        json.push_str(&format!("      \"line\": {},\n", class_info.line));
        json.push_str(&format!(
            "      \"field_count\": {},\n",
            class_info.fields.len()
        ));
        json.push_str(&format!(
            "      \"validation_count\": {},\n",
            class_info.validations.len()
        ));
        json.push_str(&format!(
            "      \"method_count\": {},\n",
            class_info.methods.len()
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&class_info.status)
        ));
        json.push_str("      \"fields\": [\n");
        for (field_index, field) in class_info.fields.iter().enumerate() {
            if field_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&field.name)
            ));
            json.push_str(&format!(
                "          \"type_name\": \"{}\",\n",
                json_escape(&field.type_name)
            ));
            json.push_str(&format!(
                "          \"quantity_kind\": \"{}\",\n",
                json_escape(&field.quantity_kind)
            ));
            json.push_str(&format!(
                "          \"display_unit\": \"{}\",\n",
                json_escape(&field.display_unit)
            ));
            json.push_str(&format!(
                "          \"canonical_unit\": \"{}\",\n",
                json_escape(&field.canonical_unit)
            ));
            json.push_str(&format!(
                "          \"dimension\": \"{}\",\n",
                json_escape(&field.dimension)
            ));
            match &field.default_value {
                Some(default_value) => json.push_str(&format!(
                    "          \"default\": \"{}\",\n",
                    json_escape(default_value)
                )),
                None => json.push_str("          \"default\": null,\n"),
            }
            json.push_str(&format!("          \"required\": {},\n", field.required));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&field.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", field.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"validations\": [\n");
        for (validation_index, validation) in class_info.validations.iter().enumerate() {
            if validation_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&validation.expression)
            ));
            json.push_str(&format!(
                "          \"left\": \"{}\",\n",
                json_escape(&validation.left)
            ));
            json.push_str(&format!(
                "          \"operator\": \"{}\",\n",
                json_escape(&validation.operator)
            ));
            json.push_str(&format!(
                "          \"right\": \"{}\",\n",
                json_escape(&validation.right)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&validation.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", validation.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"methods\": [\n");
        for (method_index, method) in class_info.methods.iter().enumerate() {
            if method_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&method.name)
            ));
            json.push_str(&format!(
                "          \"return_type\": \"{}\",\n",
                json_escape(&method.return_type)
            ));
            json.push_str(&format!(
                "          \"return_quantity_kind\": \"{}\",\n",
                json_escape(&method.return_quantity_kind)
            ));
            json.push_str(&format!(
                "          \"return_display_unit\": \"{}\",\n",
                json_escape(&method.return_display_unit)
            ));
            json.push_str(&format!(
                "          \"return_canonical_unit\": \"{}\",\n",
                json_escape(&method.return_canonical_unit)
            ));
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&method.expression)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&method.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", method.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"object_summary\": [\n");
    for (index, object) in report.semantic_program.class_objects.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&object.name)
        ));
        json.push_str(&format!(
            "      \"class_name\": \"{}\",\n",
            json_escape(&object.class_name)
        ));
        match &object.source_object {
            Some(source_object) => json.push_str(&format!(
                "      \"source_object\": \"{}\",\n",
                json_escape(source_object)
            )),
            None => json.push_str("      \"source_object\": null,\n"),
        }
        json.push_str(&format!(
            "      \"construction\": \"{}\",\n",
            json_escape(&object.construction)
        ));
        json.push_str(&format!("      \"line\": {},\n", object.line));
        json.push_str(&format!(
            "      \"field_count\": {},\n",
            object.fields.len()
        ));
        json.push_str(&format!(
            "      \"validation_count\": {},\n",
            object.validations.len()
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&object.status)
        ));
        json.push_str("      \"fields\": [\n");
        for (field_index, field) in object.fields.iter().enumerate() {
            if field_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&field.name)
            ));
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&field.expression)
            ));
            json.push_str(&format!(
                "          \"quantity_kind\": \"{}\",\n",
                json_escape(&field.quantity_kind)
            ));
            json.push_str(&format!(
                "          \"display_unit\": \"{}\",\n",
                json_escape(&field.display_unit)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&field.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", field.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"validations\": [\n");
        for (validation_index, validation) in object.validations.iter().enumerate() {
            if validation_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&validation.expression)
            ));
            json.push_str(&format!(
                "          \"left\": \"{}\",\n",
                json_escape(&validation.left)
            ));
            json.push_str(&format!(
                "          \"operator\": \"{}\",\n",
                json_escape(&validation.operator)
            ));
            json.push_str(&format!(
                "          \"right\": \"{}\",\n",
                json_escape(&validation.right)
            ));
            push_optional_json_string(
                &mut json,
                "left_value",
                validation.left_value.as_deref(),
                10,
            );
            push_optional_json_string(
                &mut json,
                "right_value",
                validation.right_value.as_deref(),
                10,
            );
            json.push_str(&format!(
                "          \"unit\": \"{}\",\n",
                json_escape(&validation.unit)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&validation.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", validation.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"system_summary\": [\n");
    for (index, system) in report.semantic_program.systems.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&system.name)
        ));
        json.push_str(&format!("      \"line\": {},\n", system.line));
        json.push_str(&format!(
            "      \"variable_count\": {},\n",
            system.variables.len()
        ));
        json.push_str(&format!(
            "      \"equation_count\": {},\n",
            system.equations.len()
        ));
        json.push_str(&format!(
            "      \"residual_count\": {},\n",
            system.residuals.len()
        ));
        json.push_str("      \"variables\": [\n");
        for (variable_index, variable) in system.variables.iter().enumerate() {
            if variable_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"role\": \"{}\",\n",
                json_escape(&variable.role)
            ));
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&variable.name)
            ));
            json.push_str(&format!(
                "          \"quantity_kind\": \"{}\",\n",
                json_escape(&variable.quantity_kind)
            ));
            json.push_str(&format!(
                "          \"display_unit\": \"{}\",\n",
                json_escape(&variable.display_unit)
            ));
            json.push_str(&format!(
                "          \"dimension\": \"{}\",\n",
                json_escape(&variable.dimension)
            ));
            json.push_str(&format!("          \"line\": {}\n", variable.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"equations\": [\n");
        for (equation_index, equation) in system.equations.iter().enumerate() {
            if equation_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"left\": \"{}\",\n",
                json_escape(&equation.left)
            ));
            json.push_str(&format!(
                "          \"relation\": \"{}\",\n",
                json_escape(&equation.relation)
            ));
            json.push_str(&format!(
                "          \"right\": \"{}\",\n",
                json_escape(&equation.right)
            ));
            json.push_str(&format!(
                "          \"left_dimension\": \"{}\",\n",
                json_escape(&equation.left_dimension)
            ));
            json.push_str(&format!(
                "          \"right_dimension\": \"{}\",\n",
                json_escape(&equation.right_dimension)
            ));
            json.push_str(&format!(
                "          \"residual\": \"{}\",\n",
                json_escape(&equation.residual)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&equation.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", equation.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"residuals\": [\n");
        for (residual_index, residual) in system.residuals.iter().enumerate() {
            if residual_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&residual.name)
            ));
            json.push_str(&format!(
                "          \"expression\": \"{}\",\n",
                json_escape(&residual.expression)
            ));
            json.push_str(&format!(
                "          \"dimension\": \"{}\",\n",
                json_escape(&residual.dimension)
            ));
            json.push_str(&format!("          \"line\": {}\n", residual.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    push_system_ir_json(&mut json, &report.semantic_program.systems);
    json.push_str("  \"schema_summary\": [\n");
    for (index, schema) in report.semantic_program.schemas.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&schema.name)
        ));
        json.push_str(&format!("      \"line\": {},\n", schema.line));
        json.push_str(&format!(
            "      \"column_count\": {},\n",
            schema.columns.len()
        ));
        json.push_str(&format!(
            "      \"constraint_count\": {},\n",
            schema.constraints.len()
        ));
        json.push_str(&format!(
            "      \"missing_policy_count\": {}\n",
            schema.missing_policies.len()
        ));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"schemas\": [\n");
    for (index, schema) in report.semantic_program.schemas.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&schema.name)
        ));
        json.push_str(&format!("      \"line\": {},\n", schema.line));
        json.push_str("      \"columns\": [\n");
        for (column_index, column) in schema.columns.iter().enumerate() {
            if column_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&column.name)
            ));
            json.push_str(&format!(
                "          \"type_name\": \"{}\",\n",
                json_escape(&column.type_name)
            ));
            if let Some(unit) = &column.unit {
                json.push_str(&format!("          \"unit\": \"{}\",\n", json_escape(unit)));
            } else {
                json.push_str("          \"unit\": null,\n");
            }
            json.push_str(&format!("          \"is_index\": {},\n", column.is_index));
            json.push_str(&format!("          \"optional\": {},\n", column.optional));
            json.push_str(&format!("          \"line\": {}\n", column.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"constraints\": [\n");
        for (constraint_index, constraint) in schema.constraints.iter().enumerate() {
            if constraint_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"text\": \"{}\",\n",
                json_escape(&constraint.text)
            ));
            json.push_str(&format!("          \"line\": {}\n", constraint.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str("      \"missing_policies\": [\n");
        for (policy_index, policy) in schema.missing_policies.iter().enumerate() {
            if policy_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"column\": \"{}\",\n",
                json_escape(&policy.column)
            ));
            json.push_str(&format!(
                "          \"policy\": \"{}\",\n",
                json_escape(&policy.policy)
            ));
            json.push_str(&format!("          \"line\": {}\n", policy.line));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"csv_promotions\": [\n");
    for (index, promotion) in report.semantic_program.csv_promotions.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&promotion.binding)
        ));
        json.push_str(&format!(
            "      \"schema_name\": \"{}\",\n",
            json_escape(&promotion.schema_name)
        ));
        json.push_str(&format!(
            "      \"source_literal\": \"{}\",\n",
            json_escape(&promotion.source_literal)
        ));
        json.push_str(&format!(
            "      \"source_value\": \"{}\",\n",
            json_escape(&promotion.source_value)
        ));
        json.push_str(&format!(
            "      \"resolved_path\": \"{}\",\n",
            json_escape(&promotion.resolved_path)
        ));
        if let Some(hash) = &promotion.source_hash {
            json.push_str(&format!(
                "      \"source_hash\": \"{}\",\n",
                json_escape(hash)
            ));
        } else {
            json.push_str("      \"source_hash\": null,\n");
        }
        json.push_str(&format!("      \"row_count\": {},\n", promotion.row_count));
        json.push_str("      \"headers\": [");
        for (header_index, header) in promotion.headers.iter().enumerate() {
            if header_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(header)));
        }
        json.push_str("],\n");
        json.push_str("      \"missing_columns\": [");
        for (missing_index, column) in promotion.missing_columns.iter().enumerate() {
            if missing_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(column)));
        }
        json.push_str("],\n");
        json.push_str("      \"optional_missing_columns\": [");
        for (missing_index, column) in promotion.optional_missing_columns.iter().enumerate() {
            if missing_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!("\"{}\"", json_escape(column)));
        }
        json.push_str("],\n");
        json.push_str(&format!("      \"line\": {}\n", promotion.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
    json.push_str("  \"config_promotions\": [\n");
    for (index, promotion) in report.semantic_program.config_promotions.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&promotion.binding)
        ));
        json.push_str(&format!(
            "      \"format\": \"{}\",\n",
            json_escape(&promotion.format)
        ));
        json.push_str(&format!(
            "      \"schema_name\": \"{}\",\n",
            json_escape(&promotion.schema_name)
        ));
        json.push_str(&format!(
            "      \"source_literal\": \"{}\",\n",
            json_escape(&promotion.source_literal)
        ));
        json.push_str(&format!(
            "      \"source_value\": \"{}\",\n",
            json_escape(&promotion.source_value)
        ));
        json.push_str(&format!(
            "      \"resolved_path\": \"{}\",\n",
            json_escape(&promotion.resolved_path)
        ));
        if let Some(hash) = &promotion.source_hash {
            json.push_str(&format!(
                "      \"source_hash\": \"{}\",\n",
                json_escape(hash)
            ));
        } else {
            json.push_str("      \"source_hash\": null,\n");
        }
        json.push_str(&format!(
            "      \"field_count\": {},\n",
            promotion.field_count
        ));
        json.push_str("      \"missing_fields\": [");
        push_json_string_array(&mut json, &promotion.missing_fields);
        json.push_str("],\n");
        json.push_str("      \"unknown_fields\": [");
        push_json_string_array(&mut json, &promotion.unknown_fields);
        json.push_str("],\n");
        json.push_str("      \"null_fields\": [");
        push_json_string_array(&mut json, &promotion.null_fields);
        json.push_str("],\n");
        json.push_str("      \"optional_fields\": [");
        push_json_string_array(&mut json, &promotion.optional_fields);
        json.push_str("],\n");
        json.push_str("      \"optional_missing_fields\": [");
        push_json_string_array(&mut json, &promotion.optional_missing_fields);
        json.push_str("],\n");
        json.push_str("      \"optional_null_fields\": [");
        push_json_string_array(&mut json, &promotion.optional_null_fields);
        json.push_str("],\n");
        json.push_str("      \"type_mismatches\": [\n");
        for (mismatch_index, mismatch) in promotion.type_mismatches.iter().enumerate() {
            if mismatch_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"field\": \"{}\",\n",
                json_escape(&mismatch.field)
            ));
            json.push_str(&format!(
                "          \"expected\": \"{}\",\n",
                json_escape(&mismatch.expected)
            ));
            json.push_str(&format!(
                "          \"actual\": \"{}\"\n",
                json_escape(&mismatch.actual)
            ));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&promotion.status)
        ));
        json.push_str(&format!("      \"line\": {}\n", promotion.line));
        json.push_str("    }");
    }
    json.push_str("\n  ]\n");
    json.push_str("}\n");
    json
}

fn push_system_ir_json(json: &mut String, systems: &[SystemInfo]) {
    json.push_str("  \"system_ir\": [\n");
    for (system_index, system) in systems.iter().enumerate() {
        if system_index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&system.name)
        ));
        json.push_str("      \"solver_boundary\": {\n");
        json.push_str("        \"status\": \"unsolved\",\n");
        json.push_str(
            "        \"reason\": \"numeric solver deferred until the solver milestone\",\n",
        );
        json.push_str(&format!(
            "        \"parameter_count\": {},\n",
            system
                .variables
                .iter()
                .filter(|variable| variable.role == "parameter")
                .count()
        ));
        json.push_str(&format!(
            "        \"state_count\": {},\n",
            system
                .variables
                .iter()
                .filter(|variable| variable.role == "state")
                .count()
        ));
        json.push_str(&format!(
            "        \"input_count\": {},\n",
            system
                .variables
                .iter()
                .filter(|variable| variable.role == "input")
                .count()
        ));
        json.push_str(&format!(
            "        \"equation_count\": {},\n",
            system.equations.len()
        ));
        json.push_str(&format!(
            "        \"residual_count\": {}\n",
            system.residuals.len()
        ));
        json.push_str("      },\n");
        push_solver_plan_json(json, &system.solver_plan, "      ");
        json.push_str(",\n");
        json.push_str("      \"equations\": [\n");
        for (equation_index, equation) in system.equation_ir.iter().enumerate() {
            if equation_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"residual\": \"{}\",\n",
                json_escape(&equation.residual)
            ));
            json.push_str(&format!(
                "          \"relation\": \"{}\",\n",
                json_escape(&equation.relation)
            ));
            json.push_str(&format!(
                "          \"normalized_residual\": \"{}\",\n",
                json_escape(&equation.normalized_residual)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&equation.status)
            ));
            json.push_str("          \"dependencies\": [\n");
            for (dependency_index, dependency) in equation.dependencies.iter().enumerate() {
                if dependency_index > 0 {
                    json.push_str(",\n");
                }
                json.push_str("            {\n");
                json.push_str(&format!(
                    "              \"name\": \"{}\",\n",
                    json_escape(&dependency.name)
                ));
                json.push_str(&format!(
                    "              \"role\": \"{}\",\n",
                    json_escape(&dependency.role)
                ));
                json.push_str(&format!(
                    "              \"quantity_kind\": \"{}\"\n",
                    json_escape(&dependency.quantity_kind)
                ));
                json.push_str("            }");
            }
            json.push_str("\n          ],\n");
            json.push_str("          \"derivative_states\": [");
            for (state_index, state) in equation.derivative_states.iter().enumerate() {
                if state_index > 0 {
                    json.push_str(", ");
                }
                json.push_str(&format!("\"{}\"", json_escape(state)));
            }
            json.push_str("],\n");
            json.push_str(&format!("          \"line\": {}\n", equation.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str(&format!("      \"line\": {}\n", system.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
}

fn push_solver_plan_json(json: &mut String, plan: &SolverPlanInfo, indent: &str) {
    json.push_str(&format!("{indent}\"solver_plan\": {{\n"));
    json.push_str(&format!(
        "{indent}  \"status\": \"{}\",\n",
        json_escape(&plan.status)
    ));
    json.push_str(&format!(
        "{indent}  \"method\": \"{}\",\n",
        json_escape(&plan.method)
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
        json_escape(&plan.ode_runner.status)
    ));
    json.push_str(&format!(
        "{indent}    \"reason\": \"{}\"\n",
        json_escape(&plan.ode_runner.reason)
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

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}: {}: {}",
            self.line,
            self.code,
            self.severity.as_str(),
            self.message
        )?;
        if let Some(help) = &self.help {
            write!(formatter, "\n  help: {help}")?;
        }
        Ok(())
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

fn push_optional_json_string(json: &mut String, key: &str, value: Option<&str>, indent: usize) {
    let spaces = " ".repeat(indent);
    match value {
        Some(value) => json.push_str(&format!("{spaces}\"{key}\": \"{}\",\n", json_escape(value))),
        None => json.push_str(&format!("{spaces}\"{key}\": null,\n")),
    }
}

fn push_optional_json_number(json: &mut String, key: &str, value: Option<f64>, indent: usize) {
    let spaces = " ".repeat(indent);
    match value {
        Some(value) => json.push_str(&format!(
            "{spaces}\"{key}\": {},\n",
            format_arg_number(value)
        )),
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

fn push_optional_json_matrix(json: &mut String, matrix: Option<&[Vec<f64>]>) {
    let Some(matrix) = matrix else {
        json.push_str("null");
        return;
    };
    json.push('[');
    for (row_index, row) in matrix.iter().enumerate() {
        if row_index > 0 {
            json.push_str(", ");
        }
        json.push('[');
        for (column_index, value) in row.iter().enumerate() {
            if column_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format_json_number(*value));
        }
        json.push(']');
    }
    json.push(']');
}

fn format_json_number(value: f64) -> String {
    if value.is_finite() {
        value.to_string()
    } else {
        "null".to_owned()
    }
}

fn push_linear_operator_entries_json(
    json: &mut String,
    entries: &[LinearOperatorEntryInfo],
    indent: usize,
) {
    let spaces = " ".repeat(indent);
    json.push_str(&format!("{spaces}\"canonical_entries\": [\n"));
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{spaces}  {{\n"));
        json.push_str(&format!(
            "{spaces}    \"row_index\": {},\n",
            entry.row_index
        ));
        json.push_str(&format!(
            "{spaces}    \"column_index\": {},\n",
            entry.column_index
        ));
        json.push_str(&format!(
            "{spaces}    \"row_member\": \"{}\",\n",
            json_escape(&entry.row_member)
        ));
        json.push_str(&format!(
            "{spaces}    \"column_member\": \"{}\",\n",
            json_escape(&entry.column_member)
        ));
        json.push_str(&format!(
            "{spaces}    \"coefficient\": {}\n",
            format_json_number(entry.coefficient)
        ));
        json.push_str(&format!("{spaces}  }}"));
    }
    json.push_str(&format!("\n{spaces}],\n"));
}

fn push_json_string_array(json: &mut String, values: &[String]) {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!("\"{}\"", json_escape(value)));
    }
}

fn push_net_requests_json(json: &mut String, report: &CheckReport, indent: usize) {
    let spaces = " ".repeat(indent);
    json.push_str(&format!("{spaces}\"net_requests\": [\n"));
    for (index, request) in report.semantic_program.net_requests.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{spaces}  {{\n"));
        json.push_str(&format!(
            "{spaces}    \"binding\": \"{}\",\n",
            json_escape(&request.binding)
        ));
        json.push_str(&format!(
            "{spaces}    \"method\": \"{}\",\n",
            json_escape(&request.method)
        ));
        json.push_str(&format!(
            "{spaces}    \"url\": \"{}\",\n",
            json_escape(&request.url_value)
        ));
        push_net_query_json(json, &request.query, indent + 4);
        push_optional_json_string(
            json,
            "expected_sha256",
            request.expected_sha256.as_deref(),
            indent + 4,
        );
        push_optional_json_string(json, "fixture", request.fixture.as_deref(), indent + 4);
        push_optional_json_string(
            json,
            "response_hash",
            request.response_hash.as_deref(),
            indent + 4,
        );
        push_optional_json_usize(json, "retry", request.retry, indent + 4);
        json.push_str(&format!("{spaces}    \"cache\": {},\n", request.cache));
        match request.status_code {
            Some(status_code) => {
                json.push_str(&format!("{spaces}    \"status_code\": {},\n", status_code))
            }
            None => json.push_str(&format!("{spaces}    \"status_code\": null,\n")),
        }
        json.push_str(&format!(
            "{spaces}    \"status_class\": \"{}\",\n",
            json_escape(&request.status_class)
        ));
        json.push_str(&format!(
            "{spaces}    \"status\": \"{}\",\n",
            json_escape(&request.status)
        ));
        json.push_str(&format!("{spaces}    \"line\": {}\n", request.line));
        json.push_str(&format!("{spaces}  }}"));
    }
    json.push_str(&format!("\n{spaces}],\n"));
}

fn push_net_downloads_json(json: &mut String, report: &CheckReport, indent: usize) {
    let spaces = " ".repeat(indent);
    json.push_str(&format!("{spaces}\"net_downloads\": [\n"));
    for (index, download) in report.semantic_program.net_downloads.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{spaces}  {{\n"));
        json.push_str(&format!(
            "{spaces}    \"url\": \"{}\",\n",
            json_escape(&download.url_value)
        ));
        json.push_str(&format!(
            "{spaces}    \"target\": \"{}\",\n",
            json_escape(&download.target_value)
        ));
        push_net_query_json(json, &download.query, indent + 4);
        push_optional_json_string(
            json,
            "expected_sha256",
            download.expected_sha256.as_deref(),
            indent + 4,
        );
        push_optional_json_string(json, "fixture", download.fixture.as_deref(), indent + 4);
        push_optional_json_string(
            json,
            "response_hash",
            download.response_hash.as_deref(),
            indent + 4,
        );
        push_optional_json_usize(json, "retry", download.retry, indent + 4);
        json.push_str(&format!("{spaces}    \"cache\": {},\n", download.cache));
        match download.status_code {
            Some(status_code) => {
                json.push_str(&format!("{spaces}    \"status_code\": {},\n", status_code))
            }
            None => json.push_str(&format!("{spaces}    \"status_code\": null,\n")),
        }
        json.push_str(&format!(
            "{spaces}    \"status_class\": \"{}\",\n",
            json_escape(&download.status_class)
        ));
        json.push_str(&format!(
            "{spaces}    \"status\": \"{}\",\n",
            json_escape(&download.status)
        ));
        json.push_str(&format!("{spaces}    \"line\": {}\n", download.line));
        json.push_str(&format!("{spaces}  }}"));
    }
    json.push_str(&format!("\n{spaces}],\n"));
}

fn push_net_query_json(json: &mut String, query: &[net::NetQueryParam], indent: usize) {
    let spaces = " ".repeat(indent);
    json.push_str(&format!("{spaces}\"query\": [\n"));
    for (index, param) in query.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!(
            "{spaces}  {{ \"key\": \"{}\", \"value\": \"{}\", \"redacted\": {} }}",
            json_escape(&param.key),
            json_escape(&param.value),
            param.redacted
        ));
    }
    json.push_str(&format!("\n{spaces}],\n"));
}

fn push_cache_records_json(json: &mut String, report: &CheckReport, indent: usize) {
    let spaces = " ".repeat(indent);
    json.push_str(&format!("{spaces}\"cache_records\": [\n"));
    for (index, record) in report.semantic_program.cache_records.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!("{spaces}  {{\n"));
        json.push_str(&format!(
            "{spaces}    \"owner_kind\": \"{}\",\n",
            json_escape(&record.owner_kind)
        ));
        json.push_str(&format!(
            "{spaces}    \"owner_name\": \"{}\",\n",
            json_escape(&record.owner_name)
        ));
        json.push_str(&format!(
            "{spaces}    \"cache_key\": \"{}\",\n",
            json_escape(&record.cache_key)
        ));
        json.push_str(&format!("{spaces}    \"cache_key_parts\": ["));
        push_json_string_array(json, &record.cache_key_parts);
        json.push_str("],\n");
        json.push_str(&format!(
            "{spaces}    \"cache_key_hash\": \"{}\",\n",
            json_escape(&record.cache_key_hash)
        ));
        json.push_str(&format!(
            "{spaces}    \"cache_path\": \"{}\",\n",
            json_escape(&record.cache_path)
        ));
        json.push_str(&format!(
            "{spaces}    \"cache_dir\": \"{}\",\n",
            json_escape(&record.cache_dir)
        ));
        json.push_str(&format!(
            "{spaces}    \"source_hash\": \"{}\",\n",
            json_escape(&record.source_hash)
        ));
        push_optional_json_string(
            json,
            "expected_hash",
            record.expected_hash.as_deref(),
            indent + 4,
        );
        push_optional_json_string(
            json,
            "observed_hash",
            record.observed_hash.as_deref(),
            indent + 4,
        );
        json.push_str(&format!(
            "{spaces}    \"status\": \"{}\",\n",
            json_escape(&record.status)
        ));
        json.push_str(&format!("{spaces}    \"line\": {}\n", record.line));
        json.push_str(&format!("{spaces}  }}"));
    }
    json.push_str(&format!("\n{spaces}],\n"));
}

fn review_option_values(report: &CheckReport, owner_line: usize, key: &str) -> Vec<String> {
    let Some(raw) = report
        .semantic_program
        .with_blocks
        .iter()
        .filter(|block| block.owner_line == Some(owner_line))
        .flat_map(|block| block.options.iter())
        .find(|option| option.key == key && option.status == "accepted")
        .map(|option| option.value.as_str())
    else {
        return Vec::new();
    };
    parse_review_option_list(raw)
}

fn parse_review_option_list(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if let Some(inner) = trimmed
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        if inner.trim().is_empty() {
            return Vec::new();
        }
        return split_review_top_level(inner)
            .into_iter()
            .map(|part| strip_string_literal(part.trim()))
            .collect();
    }
    vec![strip_string_literal(trimmed)]
}

fn split_review_top_level(expression: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut previous = '\0';
    for (index, character) in expression.char_indices() {
        if character == '"' && previous != '\\' {
            in_string = !in_string;
        } else if !in_string {
            match character {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                ',' if depth == 0 => {
                    parts.push(expression[start..index].trim());
                    start = index + character.len_utf8();
                }
                _ => {}
            }
        }
        previous = character;
    }
    parts.push(expression[start..].trim());
    parts
}

fn push_named_json_string_array(json: &mut String, key: &str, values: &[String], indent: usize) {
    let spaces = " ".repeat(indent);
    json.push_str(&format!("{spaces}\"{key}\": ["));
    push_json_string_array(json, values);
    json.push_str("],\n");
}

fn push_review_document_json(json: &mut String, report: &CheckReport) {
    let program = &report.semantic_program;
    let status = if report.has_errors() {
        "diagnostics_present"
    } else if report.diagnostic_count(Severity::Warning) > 0 {
        "warnings_present"
    } else {
        "metadata_ready"
    };
    let validation_count = program
        .command_styles
        .iter()
        .filter(|command| command.verb == "validate")
        .count()
        + program
            .classes
            .iter()
            .map(|class| class.validations.len())
            .sum::<usize>()
        + program
            .class_objects
            .iter()
            .map(|object| object.validations.len())
            .sum::<usize>();
    let side_effect_count = program.writes.len()
        + program.file_operations.len()
        + program.csv_exports.len()
        + program.net_downloads.len();
    let external_boundary_count = program.process_runs.len()
        + program.environment_dependencies.len()
        + program.net_requests.len()
        + program.net_downloads.len();
    let cache_count = program.cache_records.len();
    let table_transform_count = program.table_transforms.len();
    let workflow_module_count = review_workflow_module_count();
    let fallback_count = review_fallback_count(report);
    let risk_count = review_risk_count(report);

    json.push_str("  \"review_document\": {\n");
    json.push_str("    \"format\": \"eng-review-document-preview-1\",\n");
    json.push_str(&format!("    \"status\": \"{}\",\n", json_escape(status)));
    json.push_str(&format!(
        "    \"workflow_signature\": \"{}\",\n",
        json_escape(&program.workflow.signature())
    ));
    json.push_str(&format!(
        "    \"semantic_hash\": \"{}\",\n",
        json_escape(&review_semantic_hash(report))
    ));
    push_review_section_hashes_json(json, report);
    json.push_str("    \"root_contract\": {\n");
    json.push_str(&format!(
        "      \"input_count\": {},\n",
        review_input_count(report)
    ));
    json.push_str(&format!(
        "      \"symbol_count\": {},\n",
        program.typed_bindings.len()
    ));
    json.push_str(&format!(
        "      \"schema_count\": {},\n",
        program.schemas.len()
    ));
    json.push_str(&format!(
        "      \"config_promotion_count\": {},\n",
        program.config_promotions.len()
    ));
    json.push_str(&format!(
        "      \"unit_quantity_count\": {},\n",
        program.typed_bindings.len()
    ));
    json.push_str(&format!(
        "      \"time_axis_count\": {},\n",
        program.axis_infos.len()
    ));
    json.push_str(&format!(
        "      \"calculation_count\": {},\n",
        review_calculation_count(report)
    ));
    json.push_str(&format!(
        "      \"derived_value_count\": {},\n",
        report.inferred_declarations.len()
    ));
    json.push_str(&format!(
        "      \"report_output_count\": {},\n",
        review_report_output_count(report)
    ));
    json.push_str(&format!(
        "      \"table_transform_count\": {},\n",
        table_transform_count
    ));
    json.push_str(&format!(
        "      \"validation_count\": {},\n",
        validation_count
    ));
    json.push_str(&format!(
        "      \"side_effect_count\": {},\n",
        side_effect_count
    ));
    json.push_str(&format!(
        "      \"external_boundary_count\": {},\n",
        external_boundary_count
    ));
    json.push_str(&format!("      \"cache_count\": {},\n", cache_count));
    json.push_str(&format!(
        "      \"workflow_module_count\": {},\n",
        workflow_module_count
    ));
    json.push_str(&format!("      \"fallback_count\": {},\n", fallback_count));
    json.push_str(&format!("      \"risk_count\": {}\n", risk_count));
    json.push_str("    },\n");
    push_review_workflow_modules_json(json);
    push_review_inputs_json(json, report);
    push_review_schemas_json(json, report);
    push_review_config_promotions_json(json, report);
    push_review_units_quantities_json(json, report);
    push_review_time_axes_json(json, report);
    push_review_symbols_json(json, report);
    push_review_derived_values_json(json, report);
    push_review_calculations_json(json, report);
    push_review_table_transforms_json(json, report);
    push_review_report_outputs_json(json, report);
    push_review_validations_json(json, report);
    push_review_side_effects_json(json, report);
    push_review_external_boundaries_json(json, report);
    push_review_caches_json(json, report);
    push_review_fallbacks_json(json, report);
    push_review_risks_json(json, report);
    json.push_str("  },\n");
}

fn push_uncertainty_summary_json(json: &mut String, report: &CheckReport) {
    json.push_str("  \"uncertainty_summary\": [\n");
    for (index, uncertainty) in report.semantic_program.uncertainty_infos.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"variable\": \"{}\",\n",
            json_escape(&uncertainty.binding)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&uncertainty.quantity_kind)
        ));
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&uncertainty.display_unit)
        ));
        json.push_str(&format!(
            "      \"representation\": \"{}\",\n",
            json_escape(&uncertainty.kind)
        ));
        push_optional_json_string(json, "source", uncertainty.source.as_deref(), 6);
        push_optional_json_string(json, "distribution", uncertainty.distribution.as_deref(), 6);
        push_optional_json_string(json, "mean", uncertainty.mean.as_deref(), 6);
        push_optional_json_string(json, "stddev", uncertainty.stddev.as_deref(), 6);
        push_optional_json_string(json, "error", uncertainty.error.as_deref(), 6);
        push_optional_json_string(json, "interval_lower", uncertainty.lower.as_deref(), 6);
        push_optional_json_string(json, "interval_upper", uncertainty.upper.as_deref(), 6);
        push_optional_json_string(json, "propagation_method", uncertainty.method.as_deref(), 6);
        json.push_str(&format!(
            "      \"samples\": {},\n",
            uncertainty.sample_count
        ));
        json.push_str("      \"assumptions\": [");
        push_json_string_array(json, &uncertainty_assumptions(uncertainty));
        json.push_str("],\n");
        json.push_str("      \"warnings\": [");
        push_json_string_array(json, &uncertainty_warnings(uncertainty));
        json.push_str("],\n");
        json.push_str(&format!("      \"line\": {}\n", uncertainty.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
}

fn push_uncertainty_propagation_json(json: &mut String, report: &CheckReport) {
    json.push_str("  \"uncertainty_propagation\": [\n");
    let mut first = true;
    for uncertainty in &report.semantic_program.uncertainty_infos {
        if uncertainty.propagation.is_empty() {
            continue;
        }
        if !first {
            json.push_str(",\n");
        }
        first = false;
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"output\": \"{}\",\n",
            json_escape(&uncertainty.binding)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&uncertainty.quantity_kind)
        ));
        push_optional_json_string(json, "method", uncertainty.method.as_deref(), 6);
        json.push_str("      \"source_terms\": [");
        for (term_index, term) in uncertainty.propagation.iter().enumerate() {
            if term_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!(
                "{{ \"source\": \"{}\", \"role\": \"{}\", \"quantity_kind\": \"{}\" }}",
                json_escape(&term.source),
                json_escape(&term.role),
                json_escape(&term.quantity_kind)
            ));
        }
        json.push_str("],\n");
        json.push_str("      \"assumptions\": [");
        push_json_string_array(json, &uncertainty_assumptions(uncertainty));
        json.push_str("],\n");
        json.push_str("      \"warnings\": [");
        push_json_string_array(json, &uncertainty_warnings(uncertainty));
        json.push_str("],\n");
        json.push_str("      \"status\": \"metadata_only\",\n");
        json.push_str(&format!("      \"line\": {}\n", uncertainty.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
}

fn uncertainty_assumptions(uncertainty: &UncertaintyInfo) -> Vec<String> {
    let mut assumptions = Vec::new();
    match uncertainty.kind.as_str() {
        "Measured" if uncertainty.stddev.is_some() => {
            assumptions.push("measured_standard_deviation".to_owned())
        }
        "Measured" if uncertainty.error.is_some() => {
            assumptions.push("measured_relative_error".to_owned())
        }
        "Interval" => assumptions.push("bounded_interval".to_owned()),
        "Distribution" => assumptions.push(
            uncertainty
                .distribution
                .as_deref()
                .map(|distribution| format!("{distribution}_distribution"))
                .unwrap_or_else(|| "distribution".to_owned()),
        ),
        "Ensemble" => assumptions.push("deterministic_ensemble_samples".to_owned()),
        _ => {}
    }
    if uncertainty.method.as_deref() == Some("linear") {
        assumptions.push("linearized_propagation".to_owned());
    }
    if !uncertainty.propagation.is_empty() {
        assumptions.push("source_terms_recorded".to_owned());
    }
    assumptions
}

fn uncertainty_warnings(uncertainty: &UncertaintyInfo) -> Vec<String> {
    let mut warnings = Vec::new();
    if uncertainty.method.as_deref() == Some("linear") && uncertainty.propagation.len() > 1 {
        warnings.push("W-UNC-INDEPENDENCE-ASSUMED".to_owned());
    }
    warnings
}

fn push_uncertainty_policies_json(json: &mut String, report: &CheckReport) {
    json.push_str("  \"uncertainty_policies\": [\n");
    let mut first_policy = true;
    for block in &report.semantic_program.with_blocks {
        let Some(policy) = block
            .options
            .iter()
            .find(|option| option.key == "uncertainty")
        else {
            continue;
        };
        if !first_policy {
            json.push_str(",\n");
        }
        first_policy = false;
        let samples = review_option_any(&block.options, "samples")
            .and_then(|option| option.value.trim().parse::<usize>().ok())
            .filter(|count| *count > 0);
        let seed = review_option_any(&block.options, "seed")
            .and_then(|option| option.value.trim().parse::<u64>().ok());
        let status = review_uncertainty_policy_status(policy, &block.options);
        json.push_str("    {\n");
        match block.owner_line {
            Some(owner_line) => json.push_str(&format!("      \"owner_line\": {},\n", owner_line)),
            None => json.push_str("      \"owner_line\": null,\n"),
        }
        json.push_str(&format!(
            "      \"method\": \"{}\",\n",
            json_escape(&policy.value.trim().to_ascii_lowercase())
        ));
        match samples {
            Some(samples) => json.push_str(&format!("      \"samples\": {},\n", samples)),
            None => json.push_str("      \"samples\": null,\n"),
        }
        match seed {
            Some(seed) => json.push_str(&format!("      \"seed\": {},\n", seed)),
            None => json.push_str("      \"seed\": null,\n"),
        }
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&status)
        ));
        json.push_str(&format!("      \"line\": {}\n", policy.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
}

fn review_uncertainty_policy_status(
    policy: &semantic::WithOptionInfo,
    options: &[semantic::WithOptionInfo],
) -> String {
    if policy.status != "accepted" {
        return policy.status.clone();
    }
    for key in ["samples", "seed"] {
        if let Some(option) = review_option_any(options, key) {
            if option.status != "accepted" {
                return option.status.clone();
            }
        }
    }
    let seed_present =
        review_option_any(options, "seed").is_some_and(|option| option.status == "accepted");
    if policy.value.trim().eq_ignore_ascii_case("monte_carlo") && !seed_present {
        return "missing_seed_warning".to_owned();
    }
    "accepted".to_owned()
}

fn review_option_any<'a>(
    options: &'a [semantic::WithOptionInfo],
    key: &str,
) -> Option<&'a semantic::WithOptionInfo> {
    options.iter().find(|option| option.key == key)
}

fn push_timeseries_uncertainty_json(json: &mut String, report: &CheckReport) {
    json.push_str("  \"timeseries_uncertainty\": [\n");
    let mut first_entry = true;
    for block in &report.semantic_program.with_blocks {
        let Some(sensor_std) = review_option_any(&block.options, "sensor_std") else {
            continue;
        };
        let Some(owner_line) = block.owner_line else {
            continue;
        };
        let Some(binding) = report
            .semantic_program
            .typed_bindings
            .iter()
            .find(|binding| binding.line == owner_line)
        else {
            continue;
        };
        let Some((axis, quantity_kind)) =
            crate::stats::time_series_quantity(&binding.semantic_type.quantity_kind)
        else {
            continue;
        };
        if !first_entry {
            json.push_str(",\n");
        }
        first_entry = false;
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&binding.name)
        ));
        json.push_str(&format!("      \"axis\": \"{}\",\n", json_escape(&axis)));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&quantity_kind)
        ));
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&binding.semantic_type.display_unit)
        ));
        json.push_str("      \"method\": \"pointwise_measured_std\",\n");
        json.push_str(&format!(
            "      \"sensor_std\": \"{}\",\n",
            json_escape(&sensor_std.value)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&sensor_std.status)
        ));
        json.push_str(&format!("      \"line\": {}\n", sensor_std.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
}

fn push_timeseries_uncertainty_calculations_json(json: &mut String, report: &CheckReport) {
    json.push_str("  \"timeseries_uncertainty_calculations\": [\n");
    let mut first_entry = true;
    for stats in &report.semantic_program.stats_infos {
        let Some(sensor_std) = timeseries_sensor_std_option(report, &stats.source) else {
            continue;
        };
        let duration_statistics = stats
            .statistics
            .iter()
            .filter(|statistic| is_duration_above_statistic(statistic))
            .cloned()
            .collect::<Vec<_>>();
        let summary_statistics = stats
            .statistics
            .iter()
            .filter(|statistic| !is_duration_above_statistic(statistic))
            .cloned()
            .collect::<Vec<_>>();
        if !summary_statistics.is_empty() {
            push_timeseries_uncertainty_calculation_entry(
                json,
                &mut first_entry,
                "timeseries_statistics",
                None,
                &stats.source,
                &summary_statistics,
                "statistics",
                &sensor_std.value,
                stats.line,
            );
        }
        if !duration_statistics.is_empty() {
            push_timeseries_uncertainty_calculation_entry(
                json,
                &mut first_entry,
                "timeseries_duration_above",
                None,
                &stats.source,
                &duration_statistics,
                "duration_above",
                &sensor_std.value,
                stats.line,
            );
        }
    }
    for integration in &report.semantic_program.integrations {
        let Some(sensor_std) = timeseries_sensor_std_option(report, &integration.source) else {
            continue;
        };
        push_timeseries_uncertainty_calculation_entry(
            json,
            &mut first_entry,
            "timeseries_integrate",
            Some(&integration.binding),
            &integration.source,
            &[],
            "integrate",
            &sensor_std.value,
            integration.line,
        );
    }
    json.push_str("\n  ],\n");
}

fn push_timeseries_uncertainty_calculation_entry(
    json: &mut String,
    first_entry: &mut bool,
    kind: &str,
    binding: Option<&str>,
    source: &str,
    statistics: &[String],
    operation: &str,
    sensor_std: &str,
    line: usize,
) {
    if !*first_entry {
        json.push_str(",\n");
    }
    *first_entry = false;
    json.push_str("    {\n");
    json.push_str(&format!("      \"kind\": \"{}\",\n", json_escape(kind)));
    if let Some(binding) = binding {
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(binding)
        ));
    } else {
        json.push_str("      \"binding\": null,\n");
    }
    json.push_str(&format!("      \"source\": \"{}\",\n", json_escape(source)));
    json.push_str("      \"statistics\": [");
    push_json_string_array(json, statistics);
    json.push_str("],\n");
    json.push_str(&format!(
        "      \"operation\": \"{}\",\n",
        json_escape(operation)
    ));
    json.push_str("      \"method\": \"pointwise_measured_std_metadata\",\n");
    json.push_str(&format!(
        "      \"sensor_std\": \"{}\",\n",
        json_escape(sensor_std)
    ));
    json.push_str("      \"status\": \"metadata_only\",\n");
    json.push_str(&format!("      \"line\": {}\n", line));
    json.push_str("    }");
}

fn is_duration_above_statistic(statistic: &str) -> bool {
    statistic.trim().starts_with("duration_above(") && statistic.trim().ends_with(')')
}

fn timeseries_sensor_std_option<'a>(
    report: &'a CheckReport,
    binding_name: &str,
) -> Option<&'a semantic::WithOptionInfo> {
    let binding = report
        .semantic_program
        .typed_bindings
        .iter()
        .find(|binding| binding.name == binding_name)?;
    report
        .semantic_program
        .with_blocks
        .iter()
        .filter(|block| block.owner_line == Some(binding.line))
        .flat_map(|block| block.options.iter())
        .find(|option| option.key == "sensor_std" && option.status == "accepted")
}

fn review_calculation_count(report: &CheckReport) -> usize {
    let program = &report.semantic_program;
    report.inferred_declarations.len()
        + program.stats_infos.len()
        + program.integrations.len()
        + program.timeseries_kernels.len()
        + program.uncertainty_infos.len()
        + program.ml_infos.len()
        + program
            .systems
            .iter()
            .map(|system| system.equations.len())
            .sum::<usize>()
        + program
            .component_assemblies
            .iter()
            .map(|assembly| assembly.equations.len())
            .sum::<usize>()
}

fn review_report_output_count(report: &CheckReport) -> usize {
    report.semantic_program.stats_infos.len()
        + report.semantic_program.integrations.len()
        + report.semantic_program.timeseries_kernels.len()
}

fn review_semantic_hash(report: &CheckReport) -> String {
    let program = &report.semantic_program;
    let mut digest = String::new();
    digest.push_str(&program.workflow.signature());
    digest.push('|');
    digest.push_str(&review_section_digest(report, "workflow_modules"));
    digest.push('|');
    digest.push_str(&review_section_digest(report, "inputs"));
    digest.push('|');
    digest.push_str(&review_section_digest(report, "schemas"));
    digest.push('|');
    digest.push_str(&review_section_digest(report, "units_quantities"));
    digest.push('|');
    digest.push_str(&review_section_digest(report, "time_axes"));
    digest.push('|');
    digest.push_str(&review_section_digest(report, "derived_values"));
    digest.push('|');
    digest.push_str(&review_section_digest(report, "calculations"));
    digest.push('|');
    digest.push_str(&review_section_digest(report, "table_transforms"));
    digest.push('|');
    digest.push_str(&review_section_digest(report, "report_outputs"));
    digest.push('|');
    digest.push_str(&review_section_digest(report, "validations"));
    digest.push('|');
    digest.push_str(&review_section_digest(report, "side_effects"));
    digest.push('|');
    digest.push_str(&review_section_digest(report, "external_boundaries"));
    digest.push('|');
    digest.push_str(&review_section_digest(report, "caches"));
    digest.push('|');
    digest.push_str(&review_section_digest(report, "fallbacks"));
    digest.push('|');
    digest.push_str(&review_section_digest(report, "risks"));
    hash_text(&digest)
}

fn review_section_digest(report: &CheckReport, section: &str) -> String {
    let program = &report.semantic_program;
    match section {
        "workflow_modules" => format!("{:?}", review_workflow_module_entries()),
        "inputs" => format!(
            "{:?}|{:?}|{:?}",
            program.args_blocks, program.arg_values, program.environment_dependencies
        ),
        "schemas" => format!(
            "{:?}|{:?}|{:?}",
            program.schemas, program.csv_promotions, program.config_promotions
        ),
        "units_quantities" => {
            format!(
                "{:?}|{:?}",
                program.typed_bindings, program.unit_derivations
            )
        }
        "time_axes" => format!("{:?}", program.axis_infos),
        "derived_values" => format!("{:?}", report.inferred_declarations),
        "calculations" => format!(
            "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
            report.inferred_declarations,
            program.stats_infos,
            program.integrations,
            program.timeseries_kernels,
            program.uncertainty_infos,
            program.ml_infos,
            program.systems,
            program.component_assemblies
        ),
        "table_transforms" => format!("{:?}", program.table_transforms),
        "report_outputs" => format!(
            "{:?}|{:?}|{:?}",
            program.stats_infos, program.integrations, program.timeseries_kernels
        ),
        "validations" => format!(
            "{:?}|{:?}|{:?}",
            program.command_styles, program.classes, program.class_objects
        ),
        "side_effects" => format!(
            "{:?}|{:?}|{:?}|{:?}",
            program.csv_exports, program.writes, program.file_operations, program.net_downloads
        ),
        "external_boundaries" => format!(
            "{:?}|{:?}|{:?}|{:?}",
            program.process_runs,
            program.environment_dependencies,
            program.net_requests,
            program.net_downloads
        ),
        "caches" => format!("{:?}", program.cache_records),
        "fallbacks" => format!(
            "{:?}|{:?}",
            program.with_blocks, program.component_assemblies
        ),
        "risks" => format!(
            "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
            report.diagnostics,
            program.schemas,
            program.table_transforms,
            program.process_runs,
            program.file_operations,
            program.environment_dependencies,
            program.net_requests,
            program.net_downloads,
            program.cache_records,
            program.uncertainty_infos,
            program.systems,
            program.component_assemblies
        ),
        _ => String::new(),
    }
}

fn push_review_section_hashes_json(json: &mut String, report: &CheckReport) {
    json.push_str("    \"section_hashes\": {\n");
    let sections = [
        "workflow_modules",
        "inputs",
        "schemas",
        "units_quantities",
        "time_axes",
        "derived_values",
        "calculations",
        "table_transforms",
        "report_outputs",
        "validations",
        "side_effects",
        "external_boundaries",
        "caches",
        "fallbacks",
        "risks",
    ];
    for (index, section) in sections.iter().enumerate() {
        let comma = if index + 1 == sections.len() { "" } else { "," };
        json.push_str(&format!(
            "      \"{}\": \"{}\"{}\n",
            json_escape(section),
            json_escape(&hash_text(&review_section_digest(report, section))),
            comma
        ));
    }
    json.push_str("    },\n");
}

fn push_review_workflow_modules_json(json: &mut String) {
    json.push_str("    \"workflow_modules\": [\n");
    for (index, module) in review_workflow_module_entries().iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"native_module\",\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&module.name)
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&module.status)
        ));
        json.push_str(&format!(
            "        \"backing\": \"{}\",\n",
            json_escape(&module.backing)
        ));
        json.push_str(&format!(
            "        \"purpose\": \"{}\",\n",
            json_escape(&module.purpose)
        ));
        json.push_str("        \"artifacts\": [");
        push_json_string_array(json, &module.artifacts);
        json.push_str("],\n");
        json.push_str("        \"diagnostics\": [");
        push_json_string_array(json, &module.diagnostics);
        json.push_str("],\n");
        json.push_str("        \"examples\": [");
        push_json_string_array(json, &module.examples);
        json.push_str("],\n");
        json.push_str("        \"tests\": [");
        push_json_string_array(json, &module.tests);
        json.push_str("],\n");
        json.push_str("        \"symbols\": [");
        push_json_string_array(json, &module.symbols);
        json.push_str("],\n");
        json.push_str(&format!(
            "        \"artifact_count\": {},\n",
            module.artifacts.len()
        ));
        json.push_str(&format!(
            "        \"diagnostic_count\": {},\n",
            module.diagnostics.len()
        ));
        json.push_str(&format!(
            "        \"example_count\": {},\n",
            module.examples.len()
        ));
        json.push_str(&format!(
            "        \"test_count\": {},\n",
            module.tests.len()
        ));
        json.push_str(&format!(
            "        \"symbol_count\": {}\n",
            module.symbols.len()
        ));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");
}

fn review_input_count(report: &CheckReport) -> usize {
    report
        .semantic_program
        .args_blocks
        .iter()
        .map(|args| args.fields.len())
        .sum::<usize>()
        + report.semantic_program.schemas.len()
        + report.semantic_program.environment_dependencies.len()
}

fn review_workflow_module_entries() -> Vec<ModuleRegistryEntry> {
    bundled_module_registry()
        .map(|registry| registry.modules)
        .unwrap_or_default()
}

fn review_workflow_module_count() -> usize {
    review_workflow_module_entries().len()
}

fn review_fallback_count(report: &CheckReport) -> usize {
    review_fallback_records(report).len()
}

fn review_fallback_records(report: &CheckReport) -> Vec<ReviewFallbackRecord> {
    let mut records = Vec::new();
    for block in &report.semantic_program.with_blocks {
        let Some(owner_line) = block.owner_line else {
            continue;
        };
        for option in block
            .options
            .iter()
            .filter(|option| option.key == "allow_failure" && option.value.trim() == "true")
        {
            records.push(ReviewFallbackRecord {
                kind: "allowed_failure".to_owned(),
                category: "external_boundary".to_owned(),
                target: format!("owner_line:{owner_line}"),
                method: "allow_failure".to_owned(),
                fallback_source: "external_operation".to_owned(),
                affected_scope: "external boundary status".to_owned(),
                assumption: "failure is acceptable for this workflow boundary".to_owned(),
                risk_level: "high".to_owned(),
                status: "declared".to_owned(),
                reason: format!("owner line {owner_line} allows an external operation to fail"),
                line: option.line,
            });
        }
    }
    for assembly in &report.semantic_program.component_assemblies {
        if assembly.solver_preview.limitations.is_empty() {
            continue;
        }
        records.push(ReviewFallbackRecord {
            kind: "solver_preview_limitation".to_owned(),
            category: "solver_or_numeric".to_owned(),
            target: assembly.name.clone(),
            method: "solver_preview".to_owned(),
            fallback_source: "metadata_only_solver_preview".to_owned(),
            affected_scope: "component assembly solve interpretation".to_owned(),
            assumption: "solver preview limitations must be reviewed before using the result as a physical solve".to_owned(),
            risk_level: "medium".to_owned(),
            status: "metadata_only".to_owned(),
            reason: assembly.solver_preview.limitations.join("; "),
            line: assembly.line,
        });
    }
    records
}

fn review_risk_count(report: &CheckReport) -> usize {
    report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .count()
        + report.semantic_program.process_runs.len()
        + report.semantic_program.file_operations.len()
        + report.semantic_program.environment_dependencies.len()
        + report
            .semantic_program
            .schemas
            .iter()
            .map(|schema| schema.missing_policies.len())
            .sum::<usize>()
        + report.semantic_program.uncertainty_infos.len()
        + report.semantic_program.component_assemblies.len()
        + report.semantic_program.systems.len()
}

fn push_review_inputs_json(json: &mut String, report: &CheckReport) {
    json.push_str("    \"inputs\": [\n");
    let mut first = true;
    for args in &report.semantic_program.args_blocks {
        for field in &args.fields {
            push_review_comma(json, &mut first);
            json.push_str("      {\n");
            json.push_str("        \"kind\": \"arg\",\n");
            json.push_str(&format!(
                "        \"name\": \"{}\",\n",
                json_escape(&field.name)
            ));
            json.push_str(&format!(
                "        \"type\": \"{}\",\n",
                json_escape(&field.type_name)
            ));
            match &field.default_value {
                Some(value) => json.push_str(&format!(
                    "        \"default\": \"{}\",\n",
                    json_escape(value)
                )),
                None => json.push_str("        \"default\": null,\n"),
            }
            json.push_str(&format!("        \"required\": {},\n", field.required));
            json.push_str(&format!("        \"line\": {}\n", field.line));
            json.push_str("      }");
        }
    }
    for schema in &report.semantic_program.schemas {
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"schema\",\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&schema.name)
        ));
        json.push_str(&format!(
            "        \"column_count\": {},\n",
            schema.columns.len()
        ));
        json.push_str(&format!(
            "        \"constraint_count\": {},\n",
            schema.constraints.len()
        ));
        json.push_str(&format!(
            "        \"missing_policy_count\": {},\n",
            schema.missing_policies.len()
        ));
        json.push_str(&format!("        \"line\": {}\n", schema.line));
        json.push_str("      }");
    }
    for dependency in &report.semantic_program.environment_dependencies {
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"environment_dependency\",\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&dependency.name)
        ));
        json.push_str(&format!(
            "        \"type\": \"{}\",\n",
            json_escape(&dependency.kind)
        ));
        if dependency.kind.starts_with("filesystem_read_") {
            json.push_str(&format!(
                "        \"path\": \"{}\",\n",
                json_escape(&dependency.resolved_value)
            ));
            push_optional_json_string(json, "source_hash", dependency.source_hash.as_deref(), 8);
        }
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&dependency.status)
        ));
        json.push_str(&format!("        \"line\": {}\n", dependency.line));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");
}

fn push_review_schemas_json(json: &mut String, report: &CheckReport) {
    json.push_str("    \"schemas\": [\n");
    for (index, schema) in report.semantic_program.schemas.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&schema.name)
        ));
        json.push_str("        \"columns\": [");
        for (column_index, column) in schema.columns.iter().enumerate() {
            if column_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!(
                "{{ \"name\": \"{}\", \"type\": \"{}\", \"unit\": {}, \"is_index\": {}, \"optional\": {}, \"line\": {} }}",
                json_escape(&column.name),
                json_escape(&column.type_name),
                match &column.unit {
                    Some(unit) => format!("\"{}\"", json_escape(unit)),
                    None => "null".to_owned(),
                },
                column.is_index,
                column.optional,
                column.line
            ));
        }
        json.push_str("],\n");
        json.push_str("        \"constraints\": [");
        for (constraint_index, constraint) in schema.constraints.iter().enumerate() {
            if constraint_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!(
                "{{ \"text\": \"{}\", \"line\": {} }}",
                json_escape(&constraint.text),
                constraint.line
            ));
        }
        json.push_str("],\n");
        json.push_str("        \"missing_policies\": [");
        for (policy_index, policy) in schema.missing_policies.iter().enumerate() {
            if policy_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!(
                "{{ \"column\": \"{}\", \"policy\": \"{}\", \"line\": {} }}",
                json_escape(&policy.column),
                json_escape(&policy.policy),
                policy.line
            ));
        }
        json.push_str("],\n");
        json.push_str(&format!("        \"line\": {}\n", schema.line));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");
}

fn push_review_config_promotions_json(json: &mut String, report: &CheckReport) {
    json.push_str("    \"config_promotions\": [\n");
    for (index, promotion) in report.semantic_program.config_promotions.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&promotion.binding)
        ));
        json.push_str(&format!(
            "        \"format\": \"{}\",\n",
            json_escape(&promotion.format)
        ));
        json.push_str(&format!(
            "        \"schema_name\": \"{}\",\n",
            json_escape(&promotion.schema_name)
        ));
        json.push_str(&format!(
            "        \"source\": \"{}\",\n",
            json_escape(&promotion.source_literal)
        ));
        json.push_str(&format!(
            "        \"resolved_path\": \"{}\",\n",
            json_escape(&promotion.resolved_path)
        ));
        push_optional_json_string(json, "source_hash", promotion.source_hash.as_deref(), 8);
        json.push_str(&format!(
            "        \"field_count\": {},\n",
            promotion.field_count
        ));
        json.push_str("        \"missing_fields\": [");
        push_json_string_array(json, &promotion.missing_fields);
        json.push_str("],\n");
        json.push_str("        \"unknown_fields\": [");
        push_json_string_array(json, &promotion.unknown_fields);
        json.push_str("],\n");
        json.push_str("        \"null_fields\": [");
        push_json_string_array(json, &promotion.null_fields);
        json.push_str("],\n");
        json.push_str("        \"optional_fields\": [");
        push_json_string_array(json, &promotion.optional_fields);
        json.push_str("],\n");
        json.push_str("        \"optional_missing_fields\": [");
        push_json_string_array(json, &promotion.optional_missing_fields);
        json.push_str("],\n");
        json.push_str("        \"optional_null_fields\": [");
        push_json_string_array(json, &promotion.optional_null_fields);
        json.push_str("],\n");
        json.push_str(&format!(
            "        \"type_mismatch_count\": {},\n",
            promotion.type_mismatches.len()
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&promotion.status)
        ));
        json.push_str(&format!("        \"line\": {}\n", promotion.line));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");
}

fn push_review_units_quantities_json(json: &mut String, report: &CheckReport) {
    json.push_str("    \"units_quantities\": [\n");
    for (index, binding) in report.semantic_program.typed_bindings.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        let derivation = report
            .semantic_program
            .unit_derivations
            .iter()
            .find(|candidate| candidate.name == binding.name);
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&binding.name)
        ));
        json.push_str(&format!(
            "        \"quantity_kind\": \"{}\",\n",
            json_escape(&binding.semantic_type.quantity_kind)
        ));
        json.push_str(&format!(
            "        \"display_unit\": \"{}\",\n",
            json_escape(&binding.semantic_type.display_unit)
        ));
        json.push_str(&format!(
            "        \"canonical_unit\": \"{}\",\n",
            json_escape(
                derivation
                    .map(|item| item.canonical_unit.as_str())
                    .unwrap_or(&binding.semantic_type.display_unit)
            )
        ));
        json.push_str(&format!(
            "        \"source_unit\": {},\n",
            derivation
                .and_then(|item| item.source_unit.as_ref())
                .map(|unit| format!("\"{}\"", json_escape(unit)))
                .unwrap_or_else(|| "null".to_owned())
        ));
        json.push_str("        \"derivation_steps\": [");
        if let Some(derivation) = derivation {
            push_json_string_array(json, &derivation.steps);
        }
        json.push_str("],\n");
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            if derivation.is_some() {
                "derived"
            } else {
                "declared_or_inferred"
            }
        ));
        json.push_str(&format!("        \"line\": {}\n", binding.line));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");
}

fn push_review_time_axes_json(json: &mut String, report: &CheckReport) {
    json.push_str("    \"time_axes\": [\n");
    for (index, axis) in report.semantic_program.axis_infos.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&axis.binding)
        ));
        json.push_str(&format!(
            "        \"axis\": \"{}\",\n",
            json_escape(&axis.axis)
        ));
        json.push_str(&format!(
            "        \"role\": \"{}\",\n",
            json_escape(&axis.role)
        ));
        json.push_str(&format!(
            "        \"source\": \"{}\",\n",
            json_escape(&axis.source)
        ));
        json.push_str(&format!("        \"line\": {}\n", axis.line));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");
}

fn push_review_symbols_json(json: &mut String, report: &CheckReport) {
    json.push_str("    \"symbols\": [\n");
    for (index, binding) in report.semantic_program.typed_bindings.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        let type_info = report
            .semantic_program
            .type_infos
            .iter()
            .find(|info| info.name == binding.name && info.line == binding.line);
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&binding.name)
        ));
        json.push_str(&format!(
            "        \"quantity_kind\": \"{}\",\n",
            json_escape(&binding.semantic_type.quantity_kind)
        ));
        json.push_str(&format!(
            "        \"display_unit\": \"{}\",\n",
            json_escape(&binding.semantic_type.display_unit)
        ));
        json.push_str(&format!(
            "        \"source\": \"{}\",\n",
            json_escape(
                type_info
                    .map(|info| info.source.as_str())
                    .unwrap_or("runtime")
            )
        ));
        json.push_str(&format!("        \"line\": {}\n", binding.line));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");
}

fn push_review_derived_values_json(json: &mut String, report: &CheckReport) {
    json.push_str("    \"derived_values\": [\n");
    for (index, declaration) in report.inferred_declarations.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&declaration.name)
        ));
        json.push_str(&format!(
            "        \"expression\": \"{}\",\n",
            json_escape(&declaration.expression)
        ));
        json.push_str(&format!(
            "        \"quantity_kind\": \"{}\",\n",
            json_escape(&declaration.quantity_kind)
        ));
        json.push_str(&format!(
            "        \"display_unit\": \"{}\",\n",
            json_escape(&declaration.display_unit)
        ));
        json.push_str(&format!("        \"line\": {}\n", declaration.line));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");
}

fn review_input_symbols(report: &CheckReport, expression: &str, output: &str) -> Vec<String> {
    report
        .semantic_program
        .typed_bindings
        .iter()
        .filter(|binding| binding.name != output)
        .filter(|binding| expression_mentions_symbol(expression, &binding.name))
        .map(|binding| binding.name.clone())
        .collect()
}

fn expression_mentions_symbol(expression: &str, symbol: &str) -> bool {
    expression
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .any(|token| token == symbol)
}

fn review_unit_derivation_steps(report: &CheckReport, name: &str) -> Vec<String> {
    report
        .semantic_program
        .unit_derivations
        .iter()
        .find(|derivation| derivation.name == name)
        .map(|derivation| derivation.steps.clone())
        .unwrap_or_default()
}

fn review_where_expansions(report: &CheckReport, owner_line: usize) -> Vec<String> {
    report
        .semantic_program
        .where_blocks
        .iter()
        .filter(|block| block.owner_line == Some(owner_line))
        .flat_map(|block| block.bindings.iter())
        .map(|binding| format!("{} = {}", binding.name, binding.expression))
        .collect()
}

fn review_function_calls(report: &CheckReport, expression: &str) -> Vec<String> {
    report
        .semantic_program
        .functions
        .iter()
        .filter(|function| expression.contains(&format!("{}(", function.name)))
        .map(|function| function.name.clone())
        .collect()
}

fn push_review_calculation_trace_fields(
    json: &mut String,
    report: &CheckReport,
    output: &str,
    expression: &str,
    output_quantity: &str,
    owner_line: usize,
) {
    push_named_json_string_array(
        json,
        "input_symbols",
        &review_input_symbols(report, expression, output),
        8,
    );
    json.push_str(&format!(
        "        \"output_quantity\": \"{}\",\n",
        json_escape(output_quantity)
    ));
    push_named_json_string_array(
        json,
        "unit_derivation",
        &review_unit_derivation_steps(report, output),
        8,
    );
    push_named_json_string_array(
        json,
        "where_expansions",
        &review_where_expansions(report, owner_line),
        8,
    );
    push_named_json_string_array(
        json,
        "function_calls",
        &review_function_calls(report, expression),
        8,
    );
    json.push_str("        \"warnings\": [],\n");
}

fn push_review_calculations_json(json: &mut String, report: &CheckReport) {
    json.push_str("    \"calculations\": [\n");
    let mut first = true;
    for declaration in &report.inferred_declarations {
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"binding\",\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&declaration.name)
        ));
        json.push_str(&format!(
            "        \"expression\": \"{}\",\n",
            json_escape(&declaration.expression)
        ));
        json.push_str(&format!(
            "        \"quantity_kind\": \"{}\",\n",
            json_escape(&declaration.quantity_kind)
        ));
        push_review_calculation_trace_fields(
            json,
            report,
            &declaration.name,
            &declaration.expression,
            &declaration.quantity_kind,
            declaration.line,
        );
        json.push_str(&format!("        \"line\": {}\n", declaration.line));
        json.push_str("      }");
    }
    for statistic in &report.semantic_program.stats_infos {
        let expression = format!("summary {} over {}", statistic.source, statistic.axis);
        let output = format!("summary:{}", statistic.source);
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"timeseries_statistics\",\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&statistic.source)
        ));
        json.push_str(&format!(
            "        \"expression\": \"{}\",\n",
            json_escape(&expression)
        ));
        json.push_str(&format!(
            "        \"quantity_kind\": \"{}\",\n",
            json_escape(&statistic.quantity_kind)
        ));
        push_review_calculation_trace_fields(
            json,
            report,
            &output,
            &expression,
            &statistic.quantity_kind,
            statistic.line,
        );
        json.push_str(&format!("        \"line\": {}\n", statistic.line));
        json.push_str("      }");
    }
    for integration in &report.semantic_program.integrations {
        let expression = format!(
            "integrate {} over {}",
            integration.source, integration.over_axis
        );
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"timeseries_integration\",\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&integration.binding)
        ));
        json.push_str(&format!(
            "        \"expression\": \"integrate {} over {}\",\n",
            json_escape(&integration.source),
            json_escape(&integration.over_axis)
        ));
        json.push_str(&format!(
            "        \"quantity_kind\": \"{}\",\n",
            json_escape(&integration.result_quantity)
        ));
        push_review_calculation_trace_fields(
            json,
            report,
            &integration.binding,
            &expression,
            &integration.result_quantity,
            integration.line,
        );
        json.push_str(&format!("        \"line\": {}\n", integration.line));
        json.push_str("      }");
    }
    for kernel in &report.semantic_program.timeseries_kernels {
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"timeseries_kernel\",\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&kernel.binding)
        ));
        json.push_str(&format!(
            "        \"expression\": \"{}\",\n",
            json_escape(&kernel.expression)
        ));
        json.push_str(&format!(
            "        \"quantity_kind\": \"{}\",\n",
            json_escape(&kernel.quantity_kind)
        ));
        push_review_calculation_trace_fields(
            json,
            report,
            &kernel.binding,
            &kernel.expression,
            &kernel.quantity_kind,
            kernel.line,
        );
        json.push_str(&format!("        \"line\": {}\n", kernel.line));
        json.push_str("      }");
    }
    for uncertainty in &report.semantic_program.uncertainty_infos {
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"uncertainty\",\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&uncertainty.binding)
        ));
        json.push_str(&format!(
            "        \"expression\": \"{}\",\n",
            json_escape(&uncertainty.expression)
        ));
        json.push_str(&format!(
            "        \"quantity_kind\": \"{}\",\n",
            json_escape(&uncertainty.quantity_kind)
        ));
        push_review_calculation_trace_fields(
            json,
            report,
            &uncertainty.binding,
            &uncertainty.expression,
            &uncertainty.quantity_kind,
            uncertainty.line,
        );
        json.push_str(&format!("        \"line\": {}\n", uncertainty.line));
        json.push_str("      }");
    }
    for ml in &report.semantic_program.ml_infos {
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"modeling\",\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&ml.binding)
        ));
        json.push_str(&format!(
            "        \"expression\": \"{}\",\n",
            json_escape(&ml.expression)
        ));
        json.push_str(&format!(
            "        \"quantity_kind\": \"{}\",\n",
            json_escape(&ml.kind)
        ));
        push_review_calculation_trace_fields(
            json,
            report,
            &ml.binding,
            &ml.expression,
            &ml.kind,
            ml.line,
        );
        json.push_str(&format!("        \"line\": {}\n", ml.line));
        json.push_str("      }");
    }
    for system in &report.semantic_program.systems {
        for equation in &system.equations {
            let expression = format!("{} {} {}", equation.left, equation.relation, equation.right);
            push_review_comma(json, &mut first);
            json.push_str("      {\n");
            json.push_str("        \"kind\": \"system_equation\",\n");
            json.push_str(&format!(
                "        \"name\": \"{}\",\n",
                json_escape(&system.name)
            ));
            json.push_str(&format!(
                "        \"expression\": \"{} {} {}\",\n",
                json_escape(&equation.left),
                json_escape(&equation.relation),
                json_escape(&equation.right)
            ));
            json.push_str(&format!(
                "        \"quantity_kind\": \"{}\",\n",
                json_escape(&equation.left_dimension)
            ));
            push_review_calculation_trace_fields(
                json,
                report,
                &system.name,
                &expression,
                &equation.left_dimension,
                equation.line,
            );
            json.push_str(&format!("        \"line\": {}\n", equation.line));
            json.push_str("      }");
        }
    }
    json.push_str("\n    ],\n");
}

fn push_review_table_transforms_json(json: &mut String, report: &CheckReport) {
    json.push_str("    \"table_transforms\": [\n");
    for (index, transform) in report.semantic_program.table_transforms.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&transform.binding)
        ));
        json.push_str(&format!(
            "        \"operation\": \"{}\",\n",
            json_escape(&transform.operation)
        ));
        json.push_str(&format!(
            "        \"source_table\": \"{}\",\n",
            json_escape(&transform.source_table)
        ));
        push_optional_json_string(
            json,
            "secondary_table",
            transform.secondary_table.as_deref(),
            8,
        );
        push_optional_json_string(json, "schema_name", transform.schema_name.as_deref(), 8);
        json.push_str(&format!(
            "        \"selected_column_count\": {},\n",
            transform.selected_columns.len()
        ));
        json.push_str("        \"selected_columns\": [\n");
        for (column_index, column) in transform.selected_columns.iter().enumerate() {
            if column_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"name\": \"{}\",\n",
                json_escape(&column.name)
            ));
            json.push_str(&format!(
                "            \"status\": \"{}\",\n",
                json_escape(&column.status)
            ));
            json.push_str(&format!("            \"line\": {}\n", column.line));
            json.push_str("          }");
        }
        json.push_str("\n        ],\n");
        json.push_str("        \"derived_columns\": [\n");
        for (column_index, column) in transform.derived_columns.iter().enumerate() {
            if column_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"name\": \"{}\",\n",
                json_escape(&column.name)
            ));
            json.push_str(&format!(
                "            \"expression\": \"{}\",\n",
                json_escape(&column.expression)
            ));
            json.push_str("            \"source_columns\": [");
            for (source_index, source_column) in column.source_columns.iter().enumerate() {
                if source_index > 0 {
                    json.push_str(", ");
                }
                json.push_str(&format!("\"{}\"", json_escape(source_column)));
            }
            json.push_str("],\n");
            json.push_str(&format!(
                "            \"status\": \"{}\",\n",
                json_escape(&column.status)
            ));
            json.push_str(&format!("            \"line\": {}\n", column.line));
            json.push_str("          }");
        }
        json.push_str("\n        ],\n");
        json.push_str("        \"sort_keys\": [\n");
        for (key_index, key) in transform.sort_keys.iter().enumerate() {
            if key_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"column\": \"{}\",\n",
                json_escape(&key.column)
            ));
            json.push_str(&format!(
                "            \"direction\": \"{}\",\n",
                json_escape(&key.direction)
            ));
            json.push_str(&format!(
                "            \"status\": \"{}\",\n",
                json_escape(&key.status)
            ));
            json.push_str(&format!("            \"line\": {}\n", key.line));
            json.push_str("          }");
        }
        json.push_str("\n        ],\n");
        json.push_str(&format!(
            "        \"predicate_count\": {},\n",
            transform.predicates.len()
        ));
        json.push_str("        \"predicates\": [\n");
        for (predicate_index, predicate) in transform.predicates.iter().enumerate() {
            if predicate_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"expression\": \"{}\",\n",
                json_escape(&predicate.expression)
            ));
            push_optional_json_string(json, "column", predicate.column.as_deref(), 12);
            json.push_str(&format!(
                "            \"operator\": \"{}\",\n",
                json_escape(&predicate.operator)
            ));
            push_optional_json_string(json, "value", predicate.value.as_deref(), 12);
            json.push_str(&format!(
                "            \"status\": \"{}\",\n",
                json_escape(&predicate.status)
            ));
            json.push_str(&format!("            \"line\": {}\n", predicate.line));
            json.push_str("          }");
        }
        json.push_str("\n        ],\n");
        json.push_str("        \"join_keys\": [\n");
        for (key_index, key) in transform.join_keys.iter().enumerate() {
            if key_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("          {\n");
            json.push_str(&format!(
                "            \"expression\": \"{}\",\n",
                json_escape(&key.expression)
            ));
            json.push_str(&format!(
                "            \"left_table\": \"{}\",\n",
                json_escape(&key.left_table)
            ));
            json.push_str(&format!(
                "            \"left_column\": \"{}\",\n",
                json_escape(&key.left_column)
            ));
            json.push_str(&format!(
                "            \"right_table\": \"{}\",\n",
                json_escape(&key.right_table)
            ));
            json.push_str(&format!(
                "            \"right_column\": \"{}\",\n",
                json_escape(&key.right_column)
            ));
            json.push_str(&format!(
                "            \"status\": \"{}\",\n",
                json_escape(&key.status)
            ));
            json.push_str(&format!("            \"line\": {}\n", key.line));
            json.push_str("          }");
        }
        json.push_str("\n        ],\n");
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&transform.status)
        ));
        json.push_str(&format!("        \"line\": {}\n", transform.line));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");
}

fn push_review_report_outputs_json(json: &mut String, report: &CheckReport) {
    json.push_str("    \"report_outputs\": [\n");
    let mut first = true;
    for statistic in &report.semantic_program.stats_infos {
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"summary\",\n");
        json.push_str(&format!(
            "        \"source\": \"{}\",\n",
            json_escape(&statistic.source)
        ));
        json.push_str(&format!(
            "        \"axis\": \"{}\",\n",
            json_escape(&statistic.axis)
        ));
        json.push_str("        \"statistics\": [");
        push_json_string_array(json, &statistic.statistics);
        json.push_str("],\n");
        json.push_str(&format!(
            "        \"quantity_kind\": \"{}\",\n",
            json_escape(&statistic.quantity_kind)
        ));
        json.push_str("        \"status\": \"declared\",\n");
        json.push_str(&format!("        \"line\": {}\n", statistic.line));
        json.push_str("      }");
    }
    for integration in &report.semantic_program.integrations {
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"derived_quantity\",\n");
        json.push_str(&format!(
            "        \"source\": \"{}\",\n",
            json_escape(&integration.source)
        ));
        json.push_str(&format!(
            "        \"binding\": \"{}\",\n",
            json_escape(&integration.binding)
        ));
        json.push_str(&format!(
            "        \"quantity_kind\": \"{}\",\n",
            json_escape(&integration.result_quantity)
        ));
        json.push_str("        \"status\": \"declared\",\n");
        json.push_str(&format!("        \"line\": {}\n", integration.line));
        json.push_str("      }");
    }
    for kernel in &report.semantic_program.timeseries_kernels {
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"plot_candidate\",\n");
        json.push_str(&format!(
            "        \"source\": \"{}\",\n",
            json_escape(&kernel.binding)
        ));
        json.push_str(&format!(
            "        \"axis\": \"{}\",\n",
            json_escape(&kernel.axis)
        ));
        json.push_str(&format!(
            "        \"quantity_kind\": \"{}\",\n",
            json_escape(&kernel.quantity_kind)
        ));
        json.push_str("        \"status\": \"metadata_only\",\n");
        json.push_str(&format!("        \"line\": {}\n", kernel.line));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");
}

fn push_review_validations_json(json: &mut String, report: &CheckReport) {
    json.push_str("    \"validations\": [\n");
    let mut first = true;
    for command in report
        .semantic_program
        .command_styles
        .iter()
        .filter(|command| command.verb == "validate")
    {
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"command_validation\",\n");
        json.push_str(&format!(
            "        \"expression\": \"{}\",\n",
            json_escape(&command.target)
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&command.status)
        ));
        json.push_str(&format!("        \"line\": {}\n", command.line));
        json.push_str("      }");
    }
    for class in &report.semantic_program.classes {
        for validation in &class.validations {
            push_review_comma(json, &mut first);
            json.push_str("      {\n");
            json.push_str("        \"kind\": \"class_validation\",\n");
            json.push_str(&format!(
                "        \"expression\": \"{}\",\n",
                json_escape(&validation.expression)
            ));
            json.push_str(&format!(
                "        \"status\": \"{}\",\n",
                json_escape(&validation.status)
            ));
            json.push_str(&format!("        \"line\": {}\n", validation.line));
            json.push_str("      }");
        }
    }
    json.push_str("\n    ],\n");
}

fn push_review_side_effects_json(json: &mut String, report: &CheckReport) {
    json.push_str("    \"side_effects\": [\n");
    let mut first = true;
    for export in &report.semantic_program.csv_exports {
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"csv_export\",\n");
        json.push_str(&format!(
            "        \"target\": \"{}\",\n",
            json_escape(&export.path)
        ));
        json.push_str("        \"status\": \"declared\",\n");
        json.push_str(&format!("        \"line\": {}\n", export.line));
        json.push_str("      }");
    }
    for write in &report.semantic_program.writes {
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"write_output\",\n");
        json.push_str(&format!(
            "        \"target\": \"{}\",\n",
            json_escape(&write.path)
        ));
        json.push_str("        \"status\": \"declared\",\n");
        json.push_str(&format!("        \"line\": {}\n", write.line));
        json.push_str("      }");
    }
    for operation in &report.semantic_program.file_operations {
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"kind\": \"file_{}\",\n",
            json_escape(&operation.operation)
        ));
        json.push_str(&format!(
            "        \"target\": \"{}\",\n",
            json_escape(
                operation
                    .destination
                    .as_deref()
                    .unwrap_or(&operation.source)
            )
        ));
        json.push_str("        \"status\": \"declared\",\n");
        json.push_str(&format!("        \"line\": {}\n", operation.line));
        json.push_str("      }");
    }
    for download in &report.semantic_program.net_downloads {
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"network_download\",\n");
        json.push_str(&format!(
            "        \"target\": \"{}\",\n",
            json_escape(&download.target_value)
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&download.status)
        ));
        json.push_str(&format!("        \"line\": {}\n", download.line));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");
}

fn push_review_external_boundaries_json(json: &mut String, report: &CheckReport) {
    json.push_str("    \"external_boundaries\": [\n");
    let mut first = true;
    for process in &report.semantic_program.process_runs {
        let expected_outputs = review_option_values(report, process.line, "expected_outputs");
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"process\",\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&process.binding)
        ));
        json.push_str(&format!(
            "        \"target\": \"{}\",\n",
            json_escape(&process.command)
        ));
        let tool_version = review_option_values(report, process.line, "tool_version")
            .into_iter()
            .next();
        push_optional_json_string(json, "tool_version", tool_version.as_deref(), 8);
        json.push_str("        \"inputs\": [");
        push_json_string_array(json, &review_option_values(report, process.line, "args"));
        json.push_str("],\n");
        json.push_str("        \"outputs\": [");
        push_json_string_array(json, &expected_outputs);
        json.push_str("],\n");
        json.push_str("        \"side_effects\": [\"process_execution\"],\n");
        json.push_str("        \"provenance\": \"static_review\",\n");
        json.push_str("        \"success\": null,\n");
        json.push_str("        \"risk_level\": \"high\",\n");
        json.push_str("        \"expected_outputs\": [");
        push_json_string_array(json, &expected_outputs);
        json.push_str("],\n");
        json.push_str("        \"status\": \"declared\",\n");
        json.push_str(&format!("        \"source_line\": {},\n", process.line));
        json.push_str(&format!("        \"line\": {}\n", process.line));
        json.push_str("      }");
    }
    for dependency in &report.semantic_program.environment_dependencies {
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"environment_dependency\",\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&dependency.name)
        ));
        json.push_str(&format!(
            "        \"target\": \"{}\",\n",
            json_escape(&dependency.expression)
        ));
        json.push_str("        \"inputs\": [");
        push_json_string_array(json, std::slice::from_ref(&dependency.expression));
        json.push_str("],\n");
        json.push_str("        \"outputs\": [],\n");
        json.push_str("        \"side_effects\": [\"environment_read\"],\n");
        json.push_str(&format!(
            "        \"provenance\": {},\n",
            dependency
                .source_hash
                .as_ref()
                .map(|hash| format!("\"{}\"", json_escape(hash)))
                .unwrap_or_else(|| "null".to_owned())
        ));
        json.push_str("        \"success\": null,\n");
        json.push_str("        \"risk_level\": \"medium\",\n");
        json.push_str("        \"expected_outputs\": [],\n");
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&dependency.status)
        ));
        json.push_str(&format!("        \"source_line\": {},\n", dependency.line));
        json.push_str(&format!("        \"line\": {}\n", dependency.line));
        json.push_str("      }");
    }
    for request in &report.semantic_program.net_requests {
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"network_request\",\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&request.binding)
        ));
        json.push_str(&format!(
            "        \"target\": \"{}\",\n",
            json_escape(&request.url_value)
        ));
        json.push_str("        \"inputs\": [],\n");
        json.push_str("        \"outputs\": [],\n");
        json.push_str("        \"side_effects\": [\"http_get\"],\n");
        push_optional_json_string(json, "provenance", request.response_hash.as_deref(), 8);
        json.push_str("        \"success\": null,\n");
        json.push_str("        \"risk_level\": \"medium\",\n");
        json.push_str("        \"expected_outputs\": [],\n");
        match request.status_code {
            Some(status_code) => {
                json.push_str(&format!("        \"status_code\": {},\n", status_code))
            }
            None => json.push_str("        \"status_code\": null,\n"),
        }
        json.push_str(&format!(
            "        \"status_class\": \"{}\",\n",
            json_escape(&request.status_class)
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&request.status)
        ));
        json.push_str(&format!("        \"source_line\": {},\n", request.line));
        json.push_str(&format!("        \"line\": {}\n", request.line));
        json.push_str("      }");
    }
    for download in &report.semantic_program.net_downloads {
        push_review_comma(json, &mut first);
        json.push_str("      {\n");
        json.push_str("        \"kind\": \"network_download\",\n");
        json.push_str("        \"name\": \"download\",\n");
        json.push_str(&format!(
            "        \"target\": \"{}\",\n",
            json_escape(&download.url_value)
        ));
        json.push_str("        \"inputs\": [],\n");
        json.push_str("        \"outputs\": [");
        push_json_string_array(json, std::slice::from_ref(&download.target_value));
        json.push_str("],\n");
        json.push_str("        \"side_effects\": [\"download_file\"],\n");
        push_optional_json_string(json, "provenance", download.response_hash.as_deref(), 8);
        json.push_str("        \"success\": null,\n");
        json.push_str("        \"risk_level\": \"high\",\n");
        json.push_str("        \"expected_outputs\": [");
        push_json_string_array(json, std::slice::from_ref(&download.target_value));
        json.push_str("],\n");
        match download.status_code {
            Some(status_code) => {
                json.push_str(&format!("        \"status_code\": {},\n", status_code))
            }
            None => json.push_str("        \"status_code\": null,\n"),
        }
        json.push_str(&format!(
            "        \"status_class\": \"{}\",\n",
            json_escape(&download.status_class)
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&download.status)
        ));
        json.push_str(&format!("        \"source_line\": {},\n", download.line));
        json.push_str(&format!("        \"line\": {}\n", download.line));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");
}

fn push_review_caches_json(json: &mut String, report: &CheckReport) {
    json.push_str("    \"caches\": [\n");
    for (index, record) in report.semantic_program.cache_records.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"owner_kind\": \"{}\",\n",
            json_escape(&record.owner_kind)
        ));
        json.push_str(&format!(
            "        \"owner_name\": \"{}\",\n",
            json_escape(&record.owner_name)
        ));
        json.push_str(&format!(
            "        \"cache_key\": \"{}\",\n",
            json_escape(&record.cache_key)
        ));
        json.push_str(&format!(
            "        \"cache_key_hash\": \"{}\",\n",
            json_escape(&record.cache_key_hash)
        ));
        json.push_str(&format!(
            "        \"cache_path\": \"{}\",\n",
            json_escape(&record.cache_path)
        ));
        json.push_str(&format!(
            "        \"cache_dir\": \"{}\",\n",
            json_escape(&record.cache_dir)
        ));
        json.push_str(&format!(
            "        \"source_hash\": \"{}\",\n",
            json_escape(&record.source_hash)
        ));
        push_optional_json_string(json, "expected_hash", record.expected_hash.as_deref(), 8);
        push_optional_json_string(json, "observed_hash", record.observed_hash.as_deref(), 8);
        json.push_str(
            "        \"policy\": \"explicit_cache_key_or_declared_boundary_fingerprint\",\n",
        );
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&record.status)
        ));
        json.push_str(&format!("        \"line\": {}\n", record.line));
        json.push_str("      }");
    }
    json.push_str("\n    ],\n");
}

fn push_review_fallbacks_json(json: &mut String, report: &CheckReport) {
    json.push_str("    \"fallbacks\": [\n");
    let mut first = true;
    for record in review_fallback_records(report) {
        push_review_comma(json, &mut first);
        push_review_fallback_record_json(json, &record, 6);
    }
    json.push_str("\n    ],\n");
}

fn push_review_fallback_record_json(
    json: &mut String,
    record: &ReviewFallbackRecord,
    indent: usize,
) {
    let spaces = " ".repeat(indent);
    json.push_str(&format!("{spaces}{{\n"));
    json.push_str(&format!(
        "{spaces}  \"kind\": \"{}\",\n",
        json_escape(&record.kind)
    ));
    json.push_str(&format!(
        "{spaces}  \"category\": \"{}\",\n",
        json_escape(&record.category)
    ));
    json.push_str(&format!(
        "{spaces}  \"target\": \"{}\",\n",
        json_escape(&record.target)
    ));
    json.push_str(&format!(
        "{spaces}  \"method\": \"{}\",\n",
        json_escape(&record.method)
    ));
    json.push_str(&format!(
        "{spaces}  \"fallback_source\": \"{}\",\n",
        json_escape(&record.fallback_source)
    ));
    json.push_str(&format!(
        "{spaces}  \"affected_scope\": \"{}\",\n",
        json_escape(&record.affected_scope)
    ));
    json.push_str(&format!(
        "{spaces}  \"assumption\": \"{}\",\n",
        json_escape(&record.assumption)
    ));
    json.push_str(&format!(
        "{spaces}  \"risk_level\": \"{}\",\n",
        json_escape(&record.risk_level)
    ));
    json.push_str(&format!(
        "{spaces}  \"status\": \"{}\",\n",
        json_escape(&record.status)
    ));
    json.push_str(&format!(
        "{spaces}  \"reason\": \"{}\",\n",
        json_escape(&record.reason)
    ));
    json.push_str(&format!("{spaces}  \"line\": {}\n", record.line));
    json.push_str(&format!("{spaces}}}"));
}

fn push_review_risks_json(json: &mut String, report: &CheckReport) {
    json.push_str("    \"risks\": [\n");
    let mut first = true;
    for diagnostic in report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
    {
        push_review_comma(json, &mut first);
        let classification = classify_diagnostic_review_risk(&diagnostic.code, "warning");
        push_review_risk_json(
            json,
            classification.category,
            classification.severity,
            classification.level,
            &diagnostic.message,
            diagnostic.line,
        );
    }
    for schema in &report.semantic_program.schemas {
        for policy in &schema.missing_policies {
            push_review_comma(json, &mut first);
            let classification = classify_review_risk("data_quality", "info");
            push_review_risk_json(
                json,
                classification.category,
                classification.severity,
                classification.level,
                &format!(
                    "schema `{}` uses missing-data policy `{}` for `{}`",
                    schema.name, policy.policy, policy.column
                ),
                policy.line,
            );
        }
    }
    for process in &report.semantic_program.process_runs {
        let category = if review_option_values(report, process.line, "expected_outputs").is_empty()
        {
            "reproducibility"
        } else {
            "external_boundary"
        };
        push_review_comma(json, &mut first);
        let classification = classify_review_risk(category, "info");
        push_review_risk_json(
            json,
            classification.category,
            classification.severity,
            classification.level,
            &format!(
                "external process `{}` is opaque to EngLang",
                process.binding
            ),
            process.line,
        );
    }
    for operation in &report.semantic_program.file_operations {
        push_review_comma(json, &mut first);
        let classification = classify_review_risk("side_effect", "info");
        push_review_risk_json(
            json,
            classification.category,
            classification.severity,
            classification.level,
            &format!(
                "file operation `{}` mutates filesystem state",
                operation.operation
            ),
            operation.line,
        );
    }
    for dependency in &report.semantic_program.environment_dependencies {
        push_review_comma(json, &mut first);
        let classification = classify_review_risk("reproducibility", "info");
        push_review_risk_json(
            json,
            classification.category,
            classification.severity,
            classification.level,
            &format!(
                "environment dependency `{}` affects reproducibility",
                dependency.name
            ),
            dependency.line,
        );
    }
    for uncertainty in &report.semantic_program.uncertainty_infos {
        push_review_comma(json, &mut first);
        let classification = classify_review_risk("uncertainty", "info");
        push_review_risk_json(
            json,
            classification.category,
            classification.severity,
            classification.level,
            &format!(
                "uncertainty representation `{}` requires assumption review",
                uncertainty.kind
            ),
            uncertainty.line,
        );
    }
    for system in &report.semantic_program.systems {
        push_review_comma(json, &mut first);
        let classification = classify_review_risk("solver_or_numeric", "info");
        push_review_risk_json(
            json,
            classification.category,
            classification.severity,
            classification.level,
            &format!("system `{}` has solver metadata boundary", system.name),
            system.line,
        );
    }
    for assembly in &report.semantic_program.component_assemblies {
        push_review_comma(json, &mut first);
        let classification = classify_review_risk("solver_or_numeric", "info");
        push_review_risk_json(
            json,
            classification.category,
            classification.severity,
            classification.level,
            &format!(
                "component assembly `{}` has {} unknown(s) and {} equation(s)",
                assembly.name, assembly.boundary.unknown_count, assembly.boundary.equation_count
            ),
            assembly.line,
        );
    }
    json.push_str("\n    ]\n");
}

fn push_review_risk_json(
    json: &mut String,
    category: &str,
    severity: &str,
    level: &str,
    summary: &str,
    line: usize,
) {
    json.push_str("      {\n");
    json.push_str(&format!(
        "        \"category\": \"{}\",\n",
        json_escape(category)
    ));
    json.push_str(&format!(
        "        \"severity\": \"{}\",\n",
        json_escape(severity)
    ));
    json.push_str(&format!("        \"level\": \"{}\",\n", json_escape(level)));
    json.push_str(&format!(
        "        \"summary\": \"{}\",\n",
        json_escape(summary)
    ));
    json.push_str(&format!("        \"line\": {}\n", line));
    json.push_str("      }");
}

fn push_review_comma(json: &mut String, first: &mut bool) {
    if *first {
        *first = false;
    } else {
        json.push_str(",\n");
    }
}

fn push_simulation_requests_json(json: &mut String, report: &CheckReport) {
    json.push_str("  \"simulation_requests\": [\n");
    let mut first_request = true;
    for declaration in &report.inferred_declarations {
        let Some(system) = declaration
            .expression
            .trim()
            .strip_prefix("simulate ")
            .map(str::trim)
        else {
            continue;
        };
        if !first_request {
            json.push_str(",\n");
        }
        first_request = false;
        let options = report
            .semantic_program
            .with_blocks
            .iter()
            .find(|block| block.owner_line == Some(declaration.line))
            .map(|block| block.options.as_slice())
            .unwrap_or(&[]);
        let solver = review_option_value(options, "solver").unwrap_or("missing");
        let timestep = review_option_value(options, "timestep");
        let duration = review_option_value(options, "duration");
        let timestep_s = timestep.and_then(review_duration_seconds);
        let duration_s = duration.and_then(review_duration_seconds);
        let step_count = timestep_s.zip(duration_s).map(|(timestep_s, duration_s)| {
            if timestep_s > 0.0 {
                (duration_s / timestep_s).ceil() as usize
            } else {
                0
            }
        });
        let time_grid_status = match (timestep_s, duration_s) {
            (Some(_), Some(_)) => "declared_fixed_step",
            (Some(_), None) => "runtime_from_timeseries",
            _ => "missing_timestep",
        };
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&declaration.name)
        ));
        json.push_str(&format!("      \"system\": \"{}\",\n", json_escape(system)));
        json.push_str(&format!("      \"solver\": \"{}\",\n", json_escape(solver)));
        json.push_str("      \"time_grid\": {\n");
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(time_grid_status)
        ));
        push_optional_json_number(json, "timestep_s", timestep_s, 8);
        push_optional_json_number(json, "duration_s", duration_s, 8);
        match step_count {
            Some(step_count) => json.push_str(&format!("        \"step_count\": {}\n", step_count)),
            None => json.push_str("        \"step_count\": null\n"),
        }
        json.push_str("      },\n");
        json.push_str("      \"inputs\": [\n");
        let mut first_input = true;
        for option in options
            .iter()
            .filter(|option| !matches!(option.key.as_str(), "solver" | "timestep" | "duration"))
        {
            if !first_input {
                json.push_str(",\n");
            }
            first_input = false;
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&option.key)
            ));
            json.push_str(&format!(
                "          \"source\": \"{}\",\n",
                json_escape(&option.value)
            ));
            json.push_str(&format!(
                "          \"status\": \"{}\",\n",
                json_escape(&option.status)
            ));
            json.push_str(&format!("          \"line\": {}\n", option.line));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str(&format!("      \"line\": {}\n", declaration.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");
}

fn review_option_value<'a>(options: &'a [semantic::WithOptionInfo], key: &str) -> Option<&'a str> {
    options
        .iter()
        .find(|option| option.key == key && option.status == "accepted")
        .map(|option| option.value.as_str())
}

fn review_duration_seconds(value: &str) -> Option<f64> {
    let mut parts = value.split_whitespace();
    let number = parts.next()?.parse::<f64>().ok()?;
    let unit = parts.next().unwrap_or("s");
    match unit {
        "s" | "sec" | "second" | "seconds" => Some(number),
        "min" | "minute" | "minutes" => Some(number * 60.0),
        "h" | "hr" | "hour" | "hours" => Some(number * 3600.0),
        _ => None,
    }
}

fn write_component_graph_json(json: &mut String, program: &semantic::SemanticProgram) {
    let port_count = program
        .components
        .iter()
        .map(|component| component.ports.len())
        .sum::<usize>();
    let behavior_nodes = component_behavior_nodes(program);
    let node_count = program.components.len() + port_count + behavior_nodes.len();
    let status = if program.components.is_empty() {
        "empty"
    } else if program
        .connections
        .iter()
        .any(|connection| connection.status != "domain_compatible")
    {
        "diagnostics_present"
    } else {
        "metadata_ready"
    };
    let mut port_lookup = HashMap::new();
    for component in &program.components {
        for port in &component.ports {
            port_lookup.insert(format!("{}.{}", component.name, port.name), port);
        }
    }

    json.push_str("  \"component_graph\": {\n");
    json.push_str("    \"format\": \"eng-component-graph-v1\",\n");
    json.push_str(&format!("    \"status\": \"{}\",\n", status));
    json.push_str(&format!("    \"node_count\": {},\n", node_count));
    json.push_str(&format!(
        "    \"edge_count\": {},\n",
        program.connections.len()
    ));
    json.push_str("    \"components\": [\n");
    for (component_index, component) in program.components.iter().enumerate() {
        if component_index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"id\": \"{}\",\n",
            json_escape(&component.name)
        ));
        json.push_str("        \"kind\": \"component\",\n");
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&component.name)
        ));
        json.push_str(&format!(
            "        \"port_count\": {},\n",
            component.ports.len()
        ));
        json.push_str("        \"ports\": [");
        for (port_index, port) in component.ports.iter().enumerate() {
            if port_index > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!(
                "\"{}.{}\"",
                json_escape(&component.name),
                json_escape(&port.name)
            ));
        }
        json.push_str("],\n");
        json.push_str(&format!("        \"line\": {},\n", component.line));
        write_source_span_json(json, "        ", component.line, false);
        json.push_str("\n      }");
    }
    json.push_str("\n    ],\n");

    json.push_str("    \"ports\": [\n");
    let mut first_port = true;
    for component in &program.components {
        for port in &component.ports {
            if !first_port {
                json.push_str(",\n");
            }
            first_port = false;
            let (medium_label, frame_label, axis_label) =
                domain_argument_labels(&program.domains, &port.domain_name, &port.type_arguments);
            json.push_str("      {\n");
            json.push_str(&format!(
                "        \"id\": \"{}.{}\",\n",
                json_escape(&component.name),
                json_escape(&port.name)
            ));
            json.push_str("        \"kind\": \"port\",\n");
            json.push_str(&format!(
                "        \"component\": \"{}\",\n",
                json_escape(&component.name)
            ));
            json.push_str(&format!(
                "        \"name\": \"{}\",\n",
                json_escape(&port.name)
            ));
            json.push_str(&format!(
                "        \"domain_label\": \"{}\",\n",
                json_escape(&port.domain)
            ));
            json.push_str(&format!(
                "        \"domain_name\": \"{}\",\n",
                json_escape(&port.domain_name)
            ));
            json.push_str("        \"type_arguments\": [");
            push_json_string_array(json, &port.type_arguments);
            json.push_str("],\n");
            push_optional_json_string(json, "medium_label", medium_label.as_deref(), 8);
            push_optional_json_string(json, "frame_label", frame_label.as_deref(), 8);
            push_optional_json_string(json, "axis_label", axis_label.as_deref(), 8);
            json.push_str(&format!(
                "        \"status\": \"{}\",\n",
                json_escape(&port.status)
            ));
            json.push_str(&format!("        \"line\": {},\n", port.line));
            write_source_span_json(json, "        ", port.line, false);
            json.push_str("\n      }");
        }
    }
    json.push_str("\n    ],\n");

    json.push_str("    \"connections\": [\n");
    for (connection_index, connection) in program.connections.iter().enumerate() {
        if connection_index > 0 {
            json.push_str(",\n");
        }
        let port = port_lookup
            .get(&connection.left)
            .or_else(|| port_lookup.get(&connection.right));
        let (medium_label, frame_label, axis_label) = port
            .map(|port| {
                domain_argument_labels(&program.domains, &port.domain_name, &port.type_arguments)
            })
            .unwrap_or((None, None, None));
        json.push_str("      {\n");
        json.push_str(&format!(
            "        \"id\": \"{} -> {}\",\n",
            json_escape(&connection.left),
            json_escape(&connection.right)
        ));
        json.push_str("        \"kind\": \"connection\",\n");
        json.push_str(&format!(
            "        \"left\": \"{}\",\n",
            json_escape(&connection.left)
        ));
        json.push_str(&format!(
            "        \"right\": \"{}\",\n",
            json_escape(&connection.right)
        ));
        json.push_str(&format!(
            "        \"left_component\": \"{}\",\n",
            json_escape(&connection.left_component)
        ));
        json.push_str(&format!(
            "        \"left_port\": \"{}\",\n",
            json_escape(&connection.left_port)
        ));
        json.push_str(&format!(
            "        \"right_component\": \"{}\",\n",
            json_escape(&connection.right_component)
        ));
        json.push_str(&format!(
            "        \"right_port\": \"{}\",\n",
            json_escape(&connection.right_port)
        ));
        json.push_str(&format!(
            "        \"domain_label\": \"{}\",\n",
            json_escape(&connection.domain)
        ));
        push_optional_json_string(json, "medium_label", medium_label.as_deref(), 8);
        push_optional_json_string(json, "frame_label", frame_label.as_deref(), 8);
        push_optional_json_string(json, "axis_label", axis_label.as_deref(), 8);
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&connection.status)
        ));
        json.push_str(&format!("        \"line\": {},\n", connection.line));
        write_source_span_json(json, "        ", connection.line, false);
        json.push_str("\n      }");
    }
    json.push_str("\n    ],\n");

    json.push_str("    \"connection_sets\": [\n");
    let mut first_set = true;
    for assembly in &program.component_assemblies {
        for connection_set in &assembly.connection_sets {
            if !first_set {
                json.push_str(",\n");
            }
            first_set = false;
            json.push_str("      {\n");
            json.push_str(&format!(
                "        \"assembly\": \"{}\",\n",
                json_escape(&assembly.name)
            ));
            json.push_str(&format!(
                "        \"name\": \"{}\",\n",
                json_escape(&connection_set.name)
            ));
            json.push_str(&format!(
                "        \"domain_label\": \"{}\",\n",
                json_escape(&connection_set.domain)
            ));
            json.push_str(&format!(
                "        \"status\": \"{}\",\n",
                json_escape(&connection_set.status)
            ));
            json.push_str(&format!(
                "        \"connection_count\": {},\n",
                connection_set.connection_count
            ));
            json.push_str("        \"ports\": [");
            push_json_string_array(json, &connection_set.ports);
            json.push_str("],\n");
            json.push_str(&format!("        \"line\": {},\n", connection_set.line));
            write_source_span_json(json, "        ", connection_set.line, false);
            json.push_str("\n      }");
        }
    }
    json.push_str("\n    ],\n");

    json.push_str("    \"behavior_nodes\": [\n");
    for (node_index, node) in behavior_nodes.iter().enumerate() {
        if node_index > 0 {
            json.push_str(",\n");
        }
        json.push_str("      {\n");
        json.push_str(&format!("        \"id\": \"{}\",\n", json_escape(&node.id)));
        json.push_str("        \"kind\": \"behavior\",\n");
        json.push_str(&format!(
            "        \"behavior_kind\": \"{}\",\n",
            json_escape(&node.behavior_kind)
        ));
        json.push_str(&format!(
            "        \"component\": \"{}\",\n",
            json_escape(&node.component)
        ));
        json.push_str(&format!(
            "        \"name\": \"{}\",\n",
            json_escape(&node.name)
        ));
        json.push_str(&format!(
            "        \"expression\": \"{}\",\n",
            json_escape(&node.expression)
        ));
        json.push_str(&format!(
            "        \"status\": \"{}\",\n",
            json_escape(&node.status)
        ));
        push_optional_json_string(json, "signal", node.signal.as_deref(), 8);
        push_optional_json_number(json, "delay_s", node.delay_s, 8);
        push_optional_json_string(
            json,
            "relationship_status",
            node.relationship_status.as_deref(),
            8,
        );
        push_optional_json_string(json, "contract_status", node.contract_status.as_deref(), 8);
        push_optional_json_string(json, "jacobian_policy", node.jacobian_policy.as_deref(), 8);
        push_optional_json_string(json, "profile_policy", node.profile_policy.as_deref(), 8);
        json.push_str(&format!("        \"line\": {},\n", node.line));
        write_source_span_json(json, "        ", node.line, false);
        json.push_str("\n      }");
    }
    json.push_str("\n    ]\n");
    json.push_str("  }");
}

#[derive(Clone, Debug, PartialEq)]
struct ComponentBehaviorNode {
    id: String,
    behavior_kind: String,
    component: String,
    name: String,
    expression: String,
    status: String,
    signal: Option<String>,
    delay_s: Option<f64>,
    relationship_status: Option<String>,
    contract_status: Option<String>,
    jacobian_policy: Option<String>,
    profile_policy: Option<String>,
    line: usize,
}

fn component_behavior_nodes(program: &semantic::SemanticProgram) -> Vec<ComponentBehaviorNode> {
    program
        .components
        .iter()
        .flat_map(|component| {
            component.local_expressions.iter().flat_map(move |local| {
                behavior_node_seeds(&local.expression)
                    .into_iter()
                    .map(move |seed| ComponentBehaviorNode {
                        id: format!("{}.{}:{}", component.name, local.name, seed.behavior_kind),
                        behavior_kind: seed.behavior_kind,
                        component: component.name.clone(),
                        name: local.name.clone(),
                        expression: local.expression.clone(),
                        status: seed.status,
                        signal: seed.signal,
                        delay_s: seed.delay_s,
                        relationship_status: seed.relationship_status,
                        contract_status: seed.contract_status,
                        jacobian_policy: seed.jacobian_policy,
                        profile_policy: seed.profile_policy,
                        line: local.line,
                    })
            })
        })
        .collect()
}

struct ComponentBehaviorSeed {
    behavior_kind: String,
    status: String,
    signal: Option<String>,
    delay_s: Option<f64>,
    relationship_status: Option<String>,
    contract_status: Option<String>,
    jacobian_policy: Option<String>,
    profile_policy: Option<String>,
}

fn behavior_node_seeds(expression: &str) -> Vec<ComponentBehaviorSeed> {
    let normalized = expression.to_ascii_lowercase();
    let mut nodes = Vec::new();
    if normalized.contains("delay(") {
        let arguments = first_behavior_call_arguments(expression, "delay").unwrap_or_default();
        nodes.push(ComponentBehaviorSeed {
            behavior_kind: "delay".to_owned(),
            status: "delay_call_runtime_buffer_seed_not_integrated".to_owned(),
            signal: arguments.first().cloned(),
            delay_s: arguments
                .get(1)
                .and_then(|duration| review_duration_seconds(duration.trim())),
            relationship_status: Some("delay_relationship_metadata_only".to_owned()),
            contract_status: None,
            jacobian_policy: None,
            profile_policy: None,
        });
    }
    if normalized.contains("predict(") || normalized.contains("predictor(") {
        nodes.push(ComponentBehaviorSeed {
            behavior_kind: "predictor".to_owned(),
            status: "predictor_call_contract_seed_not_integrated".to_owned(),
            signal: first_behavior_call_arguments(expression, "predictor")
                .or_else(|| first_behavior_call_arguments(expression, "predict"))
                .and_then(|arguments| arguments.first().cloned()),
            delay_s: None,
            relationship_status: None,
            contract_status: Some("predictor_contract_metadata_seed".to_owned()),
            jacobian_policy: Some("solver_policy_not_integrated".to_owned()),
            profile_policy: None,
        });
    }
    if normalized.contains("external(") || normalized.contains("adapter(") {
        nodes.push(ComponentBehaviorSeed {
            behavior_kind: "external".to_owned(),
            status: "external_behavior_wrapper_seed_not_integrated".to_owned(),
            signal: first_behavior_call_arguments(expression, "external")
                .or_else(|| first_behavior_call_arguments(expression, "adapter"))
                .and_then(|arguments| arguments.first().cloned()),
            delay_s: None,
            relationship_status: None,
            contract_status: Some("external_behavior_contract_metadata_seed".to_owned()),
            jacobian_policy: None,
            profile_policy: Some("safe_repro_profile_policy_seed".to_owned()),
        });
    }
    nodes
}

fn first_behavior_call_arguments(expression: &str, call_name: &str) -> Option<Vec<String>> {
    let lowered = expression.to_ascii_lowercase();
    let needle = format!("{call_name}(");
    let start = lowered.find(&needle)?;
    let open_index = start + call_name.len();
    let mut depth = 0i32;
    let mut close_index = None;
    for (index, character) in expression[open_index..].char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close_index = Some(open_index + index);
                    break;
                }
            }
            _ => {}
        }
    }
    let close_index = close_index?;
    Some(
        split_behavior_arguments(&expression[open_index + 1..close_index])
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect(),
    )
}

fn split_behavior_arguments(arguments: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (index, character) in arguments.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(arguments[start..index].trim().to_owned());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(arguments[start..].trim().to_owned());
    parts
}

fn domain_argument_labels(
    domains: &[semantic::DomainInfo],
    domain_name: &str,
    type_arguments: &[String],
) -> (Option<String>, Option<String>, Option<String>) {
    let mut medium_label = None;
    let mut frame_label = None;
    let mut axis_label = None;
    if let Some(domain) = domains.iter().find(|domain| domain.name == domain_name) {
        for (index, parameter) in domain.type_parameters.iter().enumerate() {
            let Some(argument) = type_arguments.get(index) else {
                continue;
            };
            match parameter.kind.as_str() {
                "Medium" => medium_label = Some(argument.clone()),
                "Frame" => frame_label = Some(argument.clone()),
                "Axis" => axis_label = Some(argument.clone()),
                _ => {}
            }
        }
    }
    (medium_label, frame_label, axis_label)
}

fn write_source_span_json(json: &mut String, indent: &str, line: usize, trailing_comma: bool) {
    json.push_str(&format!(
        "{}\"source_span\": {{ \"line\": {}, \"column\": 1 }}{}",
        indent,
        line,
        if trailing_comma { "," } else { "" }
    ));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_review_risk_classifier_covers_diagnostics_and_workflow_nodes() {
        let diagnostic = classify_diagnostic_review_risk("W-QTY-AMBIG-001", "warning");
        assert_eq!(diagnostic.category, "unit_or_quantity");
        assert_eq!(diagnostic.severity, "warning");
        assert_eq!(diagnostic.level, "medium");

        let process = classify_workflow_node_review_risk("process", "process-ok");
        assert_eq!(process.category, "external_boundary");
        assert_eq!(process.severity, "info");
        assert_eq!(process.level, "high");

        let coverage = classify_workflow_node_review_risk("timeseries_coverage", "gapped");
        assert_eq!(coverage.category, "data_quality");
        assert_eq!(coverage.severity, "warning");
        assert_eq!(coverage.level, "medium");
    }

    #[test]
    fn lexer_records_source_span() {
        let program = parse_source("L = 1 m + 20 cm");
        let first_token = &program.lines[0].tokens[0];

        assert_eq!(first_token.span.line, 1);
        assert_eq!(first_token.span.column, 1);
    }

    #[test]
    fn parser_records_top_level_workflow_and_binding_items() {
        let report = check_source("ok.eng", "L = 1 m + 20 cm\n", &CheckOptions::default());

        assert_eq!(report.syntax_summary.scripts, 0);
        assert_eq!(report.semantic_program.workflow.kind, "top_level");
        assert_eq!(
            report.semantic_program.workflow.arg_type.as_deref(),
            Some("Args")
        );
        assert_eq!(
            report.semantic_program.workflow.return_type.as_deref(),
            Some("Report")
        );
        assert_eq!(report.syntax_summary.fast_bindings, 1);
        assert_eq!(report.inferred_declarations[0].quantity_kind, "Length");
    }

    #[test]
    fn records_args_block_metadata() {
        let report = check_source(
            "ok.eng",
            "args {\n    case_name: String = \"baseline\"\n}\n\nL = 1 m\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert_eq!(report.syntax_summary.args_blocks, 1);
        assert_eq!(report.syntax_summary.structs, 0);
        assert_eq!(report.semantic_program.args_blocks[0].name, "Args");
        assert_eq!(
            report.semantic_program.args_blocks[0].fields[0].name,
            "case_name"
        );
        assert_eq!(
            report.semantic_program.args_blocks[0].fields[0]
                .default_value
                .as_deref(),
            Some("\"baseline\"")
        );

        let review = review_json(&report);
        assert!(review.contains("\"args_summary\""));
        assert!(review.contains("\"case_name\""));
    }

    #[test]
    fn records_top_level_args_block_and_dynamic_defaults() {
        let report = check_source(
            "ok.eng",
            "const default_input: CsvFile = file(\"sensor.csv\")\n\nfn default_output_dir() -> DirectoryPath = dir(\"build/result\")\n\nargs {\n    input: CsvFile = default_input\n    output: DirectoryPath = default_output_dir()\n}\n\nQ = 5 kW\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert_eq!(report.syntax_summary.args_blocks, 1);
        assert_eq!(report.syntax_summary.const_declarations, 1);
        assert_eq!(report.semantic_program.args_blocks[0].name, "Args");
        let value = |name: &str| {
            report
                .semantic_program
                .arg_values
                .iter()
                .find(|value| value.name == name)
                .map(|value| value.value.as_str())
        };
        assert_eq!(value("input"), Some("sensor.csv"));
        assert_eq!(value("output"), Some("build/result"));
        assert_eq!(report.semantic_program.workflow.kind, "top_level");
        assert_eq!(report.semantic_program.consts[0].name, "default_input");
    }

    #[test]
    fn path_helpers_typecheck_and_record_exists_provenance() {
        let root = std::env::temp_dir().join("englang-path-helper-test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(root.join("data").join("sensor.csv"), "time,T\n0,20\n").expect("sensor csv");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            "args {\n    input: CsvFile = file(\".\\\\data\\\\sensor.csv\")\n    output: DirectoryPath = dir(\".\\\\build\\\\out\")\n}\n\ninput_exists = exists args.input\nsummary_file = join(args.output, \"summary.csv\")\ninput_parent = parent(args.input)\ninput_stem = stem(args.input)\ninput_ext = extension(args.input)\n\nprint \"exists={input_exists} summary={summary_file} parent={input_parent} stem={input_stem} ext={input_ext}\"\n",
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(!report.has_errors());
        let input_arg = report
            .semantic_program
            .arg_values
            .iter()
            .find(|arg| arg.name == "input")
            .expect("input arg");
        assert_eq!(input_arg.value, "data/sensor.csv");
        let output_arg = report
            .semantic_program
            .arg_values
            .iter()
            .find(|arg| arg.name == "output")
            .expect("output arg");
        assert_eq!(output_arg.value, "build/out");
        let input_exists = report
            .semantic_program
            .typed_bindings
            .iter()
            .find(|binding| binding.name == "input_exists")
            .expect("input_exists binding");
        assert_eq!(input_exists.semantic_type.quantity_kind, "Bool");
        let summary_file = report
            .semantic_program
            .typed_bindings
            .iter()
            .find(|binding| binding.name == "summary_file")
            .expect("summary_file binding");
        assert_eq!(summary_file.semantic_type.quantity_kind, "FilePath");
        let dependency = report
            .semantic_program
            .environment_dependencies
            .iter()
            .find(|dependency| dependency.name == "input_exists")
            .expect("exists dependency");
        assert_eq!(dependency.kind, "filesystem_exists");
        assert_eq!(dependency.resolved_value, "true");
        assert_eq!(dependency.status, "exists");

        let review = review_json(&report);
        assert!(review.contains("\"environment_dependencies\""));
        assert!(review.contains("\"filesystem_exists\""));
        assert!(review.contains("\"resolved_value\": \"true\""));
    }

    #[test]
    fn rejects_generated_output_path_traversal() {
        let source = "Q = 10 kW\nexport summary to csv \"../summary.csv\" {\n    Q as kW\n}\nwrite text \"../summary.txt\", Q\ncopy file(\"template.txt\") to \"../copied.txt\"\nwith {\n    confirm = true\n}\ndelete \"/tmp/scratch.txt\"\nwith {\n    confirm = true\n}\n";

        let report = check_source(
            std::path::Path::new("path_policy.eng"),
            source,
            &CheckOptions::default(),
        );

        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-PATH-TRAVERSAL" && diagnostic.line == 2));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-PATH-TRAVERSAL" && diagnostic.line == 5));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-PATH-TRAVERSAL" && diagnostic.line == 6));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-PATH-OUTSIDE-OUTPUT-ROOT" && diagnostic.line == 10
        }));
    }

    #[test]
    fn read_only_io_typechecks_and_records_source_hash() {
        let root = std::env::temp_dir().join("englang-read-only-io-test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(root.join("data").join("notes.txt"), "calibrated run\n").expect("notes");
        fs::write(
            root.join("data").join("case.json"),
            "{ \"case\": \"baseline\" }\n",
        )
        .expect("json");
        fs::write(root.join("data").join("case.toml"), "case = \"baseline\"\n").expect("toml");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            "args {\n    notes: TextFile = file(\"data/notes.txt\")\n    config_json: JsonFile = file(\"data/case.json\")\n    config_toml: TomlFile = file(\"data/case.toml\")\n}\n\nnotes_text = read text args.notes\njson_text = read json args.config_json\ntoml_text = read toml args.config_toml\n\nprint \"notes={notes_text} json={json_text} toml={toml_text}\"\n",
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(!report.has_errors());
        let notes = report
            .semantic_program
            .typed_bindings
            .iter()
            .find(|binding| binding.name == "notes_text")
            .expect("notes binding");
        assert_eq!(notes.semantic_type.quantity_kind, "String");
        let reads = report
            .semantic_program
            .environment_dependencies
            .iter()
            .filter(|dependency| dependency.kind.starts_with("filesystem_read_"))
            .collect::<Vec<_>>();
        assert_eq!(reads.len(), 3);
        assert!(reads.iter().all(|dependency| dependency.status == "read"));
        assert!(reads
            .iter()
            .all(|dependency| dependency.source_hash.is_some()));

        let review = review_json(&report);
        assert!(review.contains("\"filesystem_read_text\""));
        assert!(review.contains("\"filesystem_read_json\""));
        assert!(review.contains("\"filesystem_read_toml\""));
        assert!(review.contains("\"path\": \""));
        assert!(review.contains("case.json"));
        assert!(review.contains("case.toml"));
        assert!(review.contains("\"source_hash\""));
    }

    #[test]
    fn rejects_invalid_structured_read_sources() {
        let root = std::env::temp_dir().join("englang-structured-read-parse-test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(root.join("data").join("bad.json"), "{ \"case\": ").expect("json");
        fs::write(root.join("data").join("bad.toml"), "case = ").expect("toml");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            "json_text = read json file(\"data/bad.json\")\ntoml_text = read toml file(\"data/bad.toml\")\n",
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "E-IO-JSON-PARSE" && diagnostic.line == 1 }));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "E-IO-TOML-PARSE" && diagnostic.line == 2 }));
    }

    #[test]
    fn resolves_typed_args_values() {
        let report = check_source(
            "ok.eng",
            "args {\n    enabled: Bool = false\n    count: Count = 3\n    gain: Float = 1.0\n    window: Duration = 5 min\n}\n\nL = 1 m\n",
            &CheckOptions {
                args: vec![
                    ArgOverride {
                        name: "enabled".to_owned(),
                        value: "yes".to_owned(),
                    },
                    ArgOverride {
                        name: "count".to_owned(),
                        value: "12".to_owned(),
                    },
                    ArgOverride {
                        name: "gain".to_owned(),
                        value: "1.25".to_owned(),
                    },
                    ArgOverride {
                        name: "window".to_owned(),
                        value: "10 min".to_owned(),
                    },
                ],
                ..CheckOptions::default()
            },
        );

        assert!(!report.has_errors());
        let value = |name: &str| {
            report
                .semantic_program
                .arg_values
                .iter()
                .find(|value| value.name == name)
                .map(|value| value.value.as_str())
        };
        assert_eq!(value("enabled"), Some("true"));
        assert_eq!(value("count"), Some("12"));
        assert_eq!(value("gain"), Some("1.25"));
        assert_eq!(value("window"), Some("600 s"));
    }

    #[test]
    fn rejects_invalid_typed_args_values() {
        let report = check_source(
            "bad.eng",
            "args {\n    enabled: Bool = maybe\n    count: Count = -1\n    window: Duration = 2 weeks\n}\n\nL = 1 m\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert_eq!(
            report
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == "E-ARGS-TYPE-001")
                .count(),
            3
        );
    }

    #[test]
    fn rejects_struct_args_compatibility_syntax() {
        let report = check_source(
            "bad.eng",
            "struct Args {\n    input: String = \"sensor.csv\"\n}\n\nL = 1 m\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-STRUCT-ARGS-001"));
        assert!(report.semantic_program.args_blocks.is_empty());
    }

    #[test]
    fn parser_records_system_and_equation_items() {
        let report = check_source(
            "ok.eng",
            "system RoomThermal {\n    parameter C: HeatCapacity = 500 kJ/K\n    state T: AbsoluteTemperature = 24 degC\n    input T_out: AbsoluteTemperature\n    equation {\n        C * der(T) eq T_out\n    }\n}\n",
            &CheckOptions::default(),
        );

        assert_eq!(report.syntax_summary.systems, 1);
        assert_eq!(report.syntax_summary.equations, 1);
        assert_eq!(report.semantic_program.systems[0].name, "RoomThermal");
        assert_eq!(report.semantic_program.systems[0].variables.len(), 3);
    }

    #[test]
    fn records_domain_component_and_connection_metadata() {
        let report = check_source(
            "ok.eng",
            "domain Fluid[Medium M] package \"eng.std.domains.fluid\" version \"0.1.0\" {\n    across height: Length [m]\n    through m_dot: MassFlowRate [kg/s]\n    conservation sum(m_dot) = 0\n}\n\ncomponent Supply {\n    port outlet: Fluid[Water]\n    pressure_seed = delay(outlet.m_dot, 5 s)\n}\n\ncomponent Return {\n    port inlet: Fluid[Water]\n}\n\nconnect Supply.outlet -> Return.inlet\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-ASSEMBLY-UNDERDETERMINED"));
        assert_eq!(report.syntax_summary.domains, 1);
        assert_eq!(report.syntax_summary.domain_variables, 2);
        assert_eq!(report.syntax_summary.components, 2);
        assert_eq!(report.syntax_summary.ports, 2);
        assert_eq!(report.syntax_summary.connections, 1);
        assert_eq!(report.semantic_program.domains[0].name, "Fluid");
        assert_eq!(
            report.semantic_program.domains[0].type_parameters[0].kind,
            "Medium"
        );
        assert_eq!(
            report.semantic_program.domains[0].type_parameters[0].name,
            "M"
        );
        assert_eq!(
            report.semantic_program.domains[0].type_parameters[0].display,
            "Medium M"
        );
        assert_eq!(
            report.semantic_program.domains[0].package.as_deref(),
            Some("eng.std.domains.fluid")
        );
        assert_eq!(
            report.semantic_program.domains[0].version.as_deref(),
            Some("0.1.0")
        );
        assert_eq!(
            report.semantic_program.domains[0].variables[0].role,
            "across"
        );
        assert_eq!(
            report.semantic_program.domains[0].conservations[0].status,
            "recorded"
        );
        assert_eq!(
            report.semantic_program.components[0].ports[0].status,
            "domain_resolved"
        );
        assert_eq!(
            report.semantic_program.components[0].ports[0].domain,
            "Fluid[Water]"
        );
        assert_eq!(
            report.semantic_program.components[0].ports[0].type_arguments,
            vec!["Water".to_owned()]
        );
        assert_eq!(
            report.semantic_program.components[0].local_expressions[0].name,
            "pressure_seed"
        );
        assert!(report
            .inferred_declarations
            .iter()
            .all(|declaration| declaration.name != "pressure_seed"));
        assert_eq!(
            report.semantic_program.connections[0].status,
            "domain_compatible"
        );
        assert_eq!(report.semantic_program.component_assemblies.len(), 1);
        let assembly = &report.semantic_program.component_assemblies[0];
        assert_eq!(assembly.status, "assembly_seed");
        assert_eq!(assembly.local_expression_count, 1);
        assert_eq!(
            assembly.solver_preview.delay_history,
            "delay_call_runtime_buffer_seed_not_integrated"
        );
        assert_eq!(assembly.connection_sets.len(), 1);
        assert_eq!(assembly.connection_sets[0].ports.len(), 2);
        assert_eq!(assembly.equations.len(), 2);
        assert!(assembly
            .equations
            .iter()
            .any(|equation| equation.kind == "across_equality"));
        assert!(assembly
            .equations
            .iter()
            .any(|equation| equation.kind == "through_conservation"));
        assert!(assembly.equations.iter().any(|equation| equation.reason
            == "generated from through variable conservation within a connection set"));
        assert_eq!(assembly.boundary.algebraic_count, 4);
        assert_eq!(assembly.boundary.equation_count, 2);
        assert_eq!(assembly.boundary.balance_status, "underdetermined_seed");
        assert_eq!(
            assembly.boundary.diagnostic_code.as_deref(),
            Some("E-ASSEMBLY-UNDERDETERMINED")
        );
        assert_eq!(assembly.domain_count, 1);
        assert_eq!(assembly.domain_plans[0].domain, "Fluid[Water]");
        assert_eq!(
            assembly.domain_plans[0].conservation_status,
            "conservation_recorded"
        );
        assert_eq!(
            assembly.domain_plans[0].solver_role,
            "homogeneous_connection_constraints"
        );
        assert_eq!(assembly.solver_preview.status, "single_domain_preview");
        assert_eq!(
            assembly.solver_preview.nonlinear_residual,
            "symbolic_residual_seed_no_nonlinear_iteration"
        );
        assert_eq!(assembly.residual_graph.status, "metadata_only");
        assert_eq!(assembly.residual_graph.jacobian_sparsity.len(), 2);
        assert_eq!(
            assembly.residual_graph.residual_metadata.len(),
            assembly.equations.len()
        );
        let through_metadata = assembly
            .residual_graph
            .residual_metadata
            .iter()
            .find(|metadata| metadata.name == "connection_set_1.through_m_dot_conservation")
            .expect("through conservation residual metadata");
        assert_eq!(through_metadata.kind, "through_conservation");
        assert_eq!(through_metadata.domain, "Fluid[Water]");
        assert_eq!(through_metadata.dependencies.len(), 2);
        assert!(through_metadata.source_expression.contains("sum("));
        assert!(through_metadata.line > 0);

        let review = review_json(&report);
        assert!(review.contains("\"domain_summary\""));
        assert!(review.contains("\"component_summary\""));
        assert!(review.contains("\"local_expression_count\": 1"));
        assert!(review.contains("\"pressure_seed\""));
        assert!(review.contains("\"delay_call_runtime_buffer_seed_not_integrated\""));
        assert!(review.contains("\"connection_summary\""));
        assert!(review.contains("\"assembly_summary\""));
        assert!(review.contains("\"component_graph\""));
        assert!(review.contains("\"format\": \"eng-component-graph-v1\""));
        assert!(review.contains("\"node_count\": 5"));
        assert!(review.contains("\"edge_count\": 1"));
        assert!(review.contains("\"behavior_nodes\""));
        assert!(review.contains("\"behavior_kind\": \"delay\""));
        assert!(review.contains("\"signal\": \"outlet.m_dot\""));
        assert!(review.contains("\"delay_s\": 5"));
        assert!(review.contains("\"relationship_status\": \"delay_relationship_metadata_only\""));
        assert!(review.contains("\"connection_set_1\""));
        assert!(review.contains("\"through_conservation\""));
        assert!(
            review.contains("generated from through variable conservation within a connection set")
        );
        assert!(review.contains("\"component_residual_graph\""));
        assert!(review.contains("\"residual_metadata\""));
        assert!(review.contains("\"source_expression\""));
        assert!(review.contains("\"connection_set_1.through_m_dot_conservation\""));
        assert!(review.contains("\"type_parameters\""));
        assert!(review.contains("\"kind\": \"Medium\""));
        assert!(review.contains("\"name\": \"M\""));
        assert!(review.contains("\"package\": \"eng.std.domains.fluid\""));
        assert!(review.contains("\"Fluid[Water]\""));
        assert!(review.contains("\"medium_label\": \"Water\""));
        assert!(review.contains("\"source_span\""));
        assert!(review.contains("\"domain_count\": 1"));
        assert!(review.contains("\"single_domain_preview\""));
        assert!(review.contains("\"not_production_multi_domain\""));
        assert!(review.contains("\"domain_compatible\""));
    }

    #[test]
    fn component_behavior_calls_accept_prior_local_signal_contracts() {
        let report = check_source(
            "ok.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent Source {\n    port out: Thermal\n    temperature_signal = out.T\n    delayed_temperature = delay(temperature_signal, 5 s)\n    nested_delayed_temperature = delay(delay(out.T, 1 s), 5 s)\n    predicted_temperature = predictor(delay(out.T, 1 s))\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        let locals = &report.semantic_program.components[0].local_expressions;
        assert_eq!(locals.len(), 4);
        assert_eq!(locals[0].name, "temperature_signal");
        assert_eq!(locals[0].quantity_kind, "AbsoluteTemperature");
        assert_eq!(locals[0].display_unit, "degC");
        assert_eq!(locals[0].canonical_unit, "K");
        assert_eq!(locals[0].type_status, "domain_signal_resolved");
        assert_eq!(locals[1].name, "delayed_temperature");
        assert_eq!(locals[1].quantity_kind, "AbsoluteTemperature");
        assert_eq!(locals[1].display_unit, "degC");
        assert_eq!(locals[1].canonical_unit, "K");
        assert_eq!(locals[1].type_status, "delay_output_matches_signal");
        assert_eq!(locals[2].name, "nested_delayed_temperature");
        assert_eq!(locals[2].quantity_kind, "AbsoluteTemperature");
        assert_eq!(locals[2].display_unit, "degC");
        assert_eq!(locals[2].canonical_unit, "K");
        assert_eq!(locals[2].type_status, "delay_output_matches_signal");

        let review = review_json(&report);
        assert!(review.contains("\"signal\": \"temperature_signal\""));
        assert!(review.contains("\"signal\": \"delay(out.T, 1 s)\""));
        assert!(review.contains("\"quantity_kind\": \"AbsoluteTemperature\""));
        assert!(review.contains("\"type_status\": \"delay_output_matches_signal\""));
    }

    #[test]
    fn records_square_component_boundary_residual_candidate() {
        let report = check_source(
            "ok.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent RoomBoundary {\n    port heat: Thermal\n    boundary_T = heat.T = 22 degC\n    boundary_Q = heat.Q = 1 kW\n}\n\ncomponent AmbientBoundary {\n    port heat: Thermal\n}\n\nconnect RoomBoundary.heat -> AmbientBoundary.heat\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        let assembly = &report.semantic_program.component_assemblies[0];
        assert_eq!(assembly.boundary.balance_status, "balanced_metadata_seed");
        assert_eq!(assembly.boundary.equation_count, 4);
        assert_eq!(assembly.boundary.unknown_count, 4);
        assert_eq!(assembly.component_equation_count, 2);
        assert!(assembly
            .equations
            .iter()
            .any(|equation| equation.kind == "component_boundary"
                && equation.rhs.as_deref() == Some("22 degC")));
        assert_eq!(
            assembly.residual_graph.status,
            "linear_residual_graph_candidate"
        );
        assert_eq!(
            assembly.residual_graph.solver_plan,
            "dense_linear_residual_graph_candidate"
        );
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "W-ASSEMBLY-ALGEBRAIC-LOOP"
                && diagnostic.severity == Severity::Warning
        }));

        let review = review_json(&report);
        assert!(review.contains("\"linear_residual_graph_candidate\""));
        assert!(review.contains("\"dense_linear_residual_graph_candidate\""));
        assert!(review.contains("\"algebraic_loops\""));
    }

    #[test]
    fn lowers_system_component_instances_into_component_assembly() {
        let report = check_source(
            "ok.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent RoomBoundary {\n    port heat: Thermal\n    boundary_T = heat.T = 22 degC\n    boundary_Q = heat.Q = 1 kW\n}\n\ncomponent AmbientBoundary {\n    port heat: Thermal\n}\n\nsystem Envelope {\n    room = RoomBoundary()\n    ambient = AmbientBoundary()\n    connect room.heat to ambient.heat\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert_eq!(report.syntax_summary.components, 2);
        assert_eq!(report.syntax_summary.connections, 1);
        assert_eq!(
            report
                .semantic_program
                .components
                .iter()
                .map(|component| component.name.as_str())
                .collect::<Vec<_>>(),
            vec!["room", "ambient"]
        );
        assert_eq!(
            report.semantic_program.connections[0].status,
            "domain_compatible"
        );
        assert_eq!(report.semantic_program.component_assemblies.len(), 1);
        let assembly = &report.semantic_program.component_assemblies[0];
        assert_eq!(assembly.component_count, 2);
        assert_eq!(assembly.connection_count, 1);
        assert_eq!(assembly.local_expression_count, 2);
        assert_eq!(assembly.component_equation_count, 2);
        assert_eq!(assembly.boundary.balance_status, "balanced_metadata_seed");
        assert_eq!(assembly.boundary.equation_count, 4);
        assert!(assembly
            .equations
            .iter()
            .any(|equation| equation.kind == "component_boundary"
                && equation.expression == "room.heat.T eq 22 degC"));

        let review = review_json(&report);
        assert!(review.contains("\"left\": \"room.heat\""));
        assert!(review.contains("\"right\": \"ambient.heat\""));
        assert!(review.contains("\"linear_residual_graph_candidate\""));
    }

    #[test]
    fn accepts_named_component_constructor_arguments() {
        let report = check_source(
            "ok.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent RoomBoundary {\n    port heat: Thermal\n    boundary_T = heat.T = T_room\n    boundary_Q = heat.Q = Q_room\n}\n\ncomponent AmbientBoundary {\n    port heat: Thermal\n}\n\nsystem Envelope {\n    room = RoomBoundary(T_room=22 degC, Q_room=1 kW)\n    ambient = AmbientBoundary()\n    connect room.heat to ambient.heat\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let assembly = &report.semantic_program.component_assemblies[0];
        assert!(assembly.equations.iter().any(|equation| {
            equation.kind == "component_boundary" && equation.expression == "room.heat.T eq 22 degC"
        }));
        assert!(assembly.equations.iter().any(|equation| {
            equation.kind == "component_boundary" && equation.expression == "room.heat.Q eq 1 kW"
        }));
        let room = report
            .semantic_program
            .components
            .iter()
            .find(|component| component.name == "room")
            .expect("room instance");
        assert_eq!(room.template_name.as_deref(), Some("RoomBoundary"));
        assert_eq!(room.constructor_arguments.len(), 2);
        assert_eq!(room.constructor_arguments[0].name, "T_room");
        assert_eq!(room.constructor_arguments[0].value, "22 degC");
        let review = review_json(&report);
        assert!(review.contains("\"template_name\": \"RoomBoundary\""));
        assert!(review.contains("\"constructor_arguments\""));
        assert!(review.contains("\"value\": \"1 kW\""));
    }

    #[test]
    fn accepts_declared_component_parameter_defaults_and_overrides() {
        let report = check_source(
            "ok.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent RoomBoundary {\n    port heat: Thermal\n    parameter T_room: AbsoluteTemperature [degC] = 21 degC\n    parameter Q_room: HeatRate [kW] = 1 kW\n    boundary_T = heat.T = T_room\n    boundary_Q = heat.Q = Q_room\n}\n\ncomponent AmbientBoundary {\n    port heat: Thermal\n}\n\nsystem Envelope {\n    room = RoomBoundary(T_room=22 degC)\n    ambient = AmbientBoundary()\n    connect room.heat to ambient.heat\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let room = report
            .semantic_program
            .components
            .iter()
            .find(|component| component.name == "room")
            .expect("room instance");
        assert_eq!(room.parameters.len(), 2);
        assert_eq!(room.parameters[0].name, "T_room");
        assert_eq!(room.parameters[0].value.as_deref(), Some("22 degC"));
        assert_eq!(room.parameters[0].status, "constructor_override");
        assert_eq!(room.parameters[1].name, "Q_room");
        assert_eq!(room.parameters[1].value.as_deref(), Some("1 kW"));
        assert_eq!(room.parameters[1].status, "defaulted");
        let assembly = &report.semantic_program.component_assemblies[0];
        assert_eq!(assembly.boundary.parameter_count, 2);
        assert!(assembly.equations.iter().any(|equation| {
            equation.kind == "component_boundary"
                && equation.expression == "room.heat.T eq room.T_room"
                && equation.dependencies == vec!["room.heat.T".to_owned(), "room.T_room".to_owned()]
        }));
        assert!(assembly.equations.iter().any(|equation| {
            equation.kind == "component_boundary"
                && equation.expression == "room.heat.Q eq room.Q_room"
                && equation.dependencies == vec!["room.heat.Q".to_owned(), "room.Q_room".to_owned()]
        }));
        let review = review_json(&report);
        assert!(review.contains("\"parameters\""));
        assert!(review.contains("\"status\": \"constructor_override\""));
        assert!(review.contains("\"status\": \"defaulted\""));
    }

    #[test]
    fn accepts_component_input_declarations_in_assembly() {
        let report = check_source(
            "ok.eng",
            "domain ScalarInputState {\n    across x: DimensionlessNumber [1]\n    through balance: DimensionlessNumber [1]\n    conservation sum(balance) = 0\n}\n\ncomponent DrivenNode {\n    port node: ScalarInputState\n    input drive: DimensionlessNumber [1] = 0.25\n    der(node.x) + sin(node.x) - drive eq 0\n}\n\nsystem DrivenSystem {\n    node = DrivenNode()\n    connect node.node to node.node\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let node = report
            .semantic_program
            .components
            .iter()
            .find(|component| component.name == "node")
            .expect("node instance");
        assert_eq!(node.inputs.len(), 1);
        assert_eq!(node.inputs[0].name, "drive");
        assert_eq!(node.inputs[0].value.as_deref(), Some("0.25"));
        assert_eq!(node.inputs[0].status, "defaulted");
        let assembly = &report.semantic_program.component_assemblies[0];
        assert_eq!(assembly.boundary.input_count, 1);
        assert!(assembly.variables.iter().any(|variable| {
            variable.name == "node.drive"
                && variable.role == "input"
                && variable.source == "component_input.DimensionlessNumber"
        }));
        assert!(assembly.equations.iter().any(|equation| {
            equation.expression == "der(node.node.x) + sin(node.node.x) - node.drive eq 0"
                && equation.dependencies.contains(&"node.drive".to_owned())
        }));
    }

    #[test]
    fn accepts_const_component_parameter_defaults_and_constructor_overrides() {
        let report = check_source(
            "ok.eng",
            "const DEFAULT_T: AbsoluteTemperature [degC] = 21 degC\nconst ROOM_Q: HeatRate [kW] = 2 kW\n\ndomain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent RoomBoundary {\n    port heat: Thermal\n    parameter T_room: AbsoluteTemperature [degC] = DEFAULT_T\n    parameter Q_room: HeatRate [kW] = 1 kW\n    boundary_T = heat.T = T_room\n    boundary_Q = heat.Q = Q_room\n}\n\ncomponent AmbientBoundary {\n    port heat: Thermal\n}\n\nsystem Envelope {\n    room = RoomBoundary(Q_room=ROOM_Q)\n    ambient = AmbientBoundary()\n    connect room.heat to ambient.heat\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let room = report
            .semantic_program
            .components
            .iter()
            .find(|component| component.name == "room")
            .expect("room instance");
        assert_eq!(room.constructor_arguments[0].name, "Q_room");
        assert_eq!(room.constructor_arguments[0].value, "ROOM_Q");
        assert_eq!(
            room.parameters[0].default_value.as_deref(),
            Some("DEFAULT_T")
        );
        assert_eq!(room.parameters[0].value.as_deref(), Some("21 degC"));
        assert_eq!(room.parameters[0].status, "defaulted");
        assert_eq!(room.parameters[1].value.as_deref(), Some("2 kW"));
        assert_eq!(room.parameters[1].status, "constructor_override");

        let review = review_json(&report);
        assert!(review.contains("\"default_value\": \"DEFAULT_T\""));
        assert!(review.contains("\"value\": \"21 degC\""));
        assert!(review.contains("\"value\": \"ROOM_Q\""));
    }
    #[test]
    fn accepts_arithmetic_component_parameter_defaults_and_constructor_overrides() {
        let source = r#"const BASE_T: AbsoluteTemperature [degC] = 20 degC
const DT_ROOM: TemperatureDelta [K] = 2 K
const BASE_Q: HeatRate [kW] = 1 kW

domain Thermal {
    across T: AbsoluteTemperature [degC]
    through Q: HeatRate [kW]
    conservation sum(Q) = 0
}

component RoomBoundary {
    port heat: Thermal
    parameter T_room: AbsoluteTemperature [degC] = BASE_T + DT_ROOM
    parameter Q_room: HeatRate [W] = BASE_Q * 2 + 500 W
    boundary_T = heat.T = T_room
    boundary_Q = heat.Q = Q_room
}

component AmbientBoundary {
    port heat: Thermal
}

system Envelope {
    room = RoomBoundary(Q_room=(BASE_Q * 3) / 2)
    ambient = AmbientBoundary()
    connect room.heat to ambient.heat
}
"#;
        let report = check_source("ok.eng", source, &CheckOptions::default());

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let room = report
            .semantic_program
            .components
            .iter()
            .find(|component| component.name == "room")
            .expect("room instance");
        assert_eq!(room.constructor_arguments[0].name, "Q_room");
        assert_eq!(room.constructor_arguments[0].value, "(BASE_Q * 3) / 2");
        assert_eq!(
            room.parameters[0].default_value.as_deref(),
            Some("BASE_T + DT_ROOM")
        );
        assert_eq!(room.parameters[0].value.as_deref(), Some("295.15 K"));
        assert_eq!(room.parameters[0].status, "defaulted");
        assert_eq!(room.parameters[1].value.as_deref(), Some("1500 W"));
        assert_eq!(room.parameters[1].status, "constructor_override");

        let review = review_json(&report);
        assert!(review.contains("\"default_value\": \"BASE_T + DT_ROOM\""));
        assert!(review.contains("\"value\": \"295.15 K\""));
        assert!(review.contains("\"value\": \"(BASE_Q * 3) / 2\""));
    }

    #[test]
    fn rejects_incompatible_component_parameter_expressions() {
        let source = r#"const BASE_Q: HeatRate [kW] = 2 kW

domain Fluid[Medium M] {
    across p: Pressure [Pa]
    through m_dot: MassFlowRate [kg/s]
    conservation sum(m_dot) = 0
}

component PumpBoundary {
    port supply: Fluid[Water]
    parameter p_supply: Pressure [Pa]
    supply_pressure = supply.p = p_supply
}

system Loop {
    pump = PumpBoundary(BASE_Q + 1 kW)
}
"#;
        let report = check_source("bad.eng", source, &CheckOptions::default());

        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-COMPONENT-PARAM-UNIT-001"
                && diagnostic.message.contains("p_supply")
                && diagnostic.message.contains("Power")
        }));
    }
    #[test]
    fn rejects_incompatible_const_component_parameter_values() {
        let report = check_source(
            "bad.eng",
            "const WRONG_Q: HeatRate [kW] = 2 kW\n\ndomain Fluid[Medium M] {\n    across p: Pressure [Pa]\n    through m_dot: MassFlowRate [kg/s]\n    conservation sum(m_dot) = 0\n}\n\ncomponent PumpBoundary {\n    port supply: Fluid[Water]\n    parameter p_supply: Pressure [Pa]\n    supply_pressure = supply.p = p_supply\n}\n\nsystem Loop {\n    pump = PumpBoundary(WRONG_Q)\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-COMPONENT-PARAM-UNIT-001"
                && diagnostic.message.contains("WRONG_Q")
        }));
    }
    #[test]
    fn accepts_positional_component_constructor_arguments_for_declared_parameters() {
        let report = check_source(
            "ok.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent RoomBoundary {\n    port heat: Thermal\n    parameter T_room: AbsoluteTemperature [degC]\n    parameter Q_room: HeatRate [kW] = 1 kW\n    boundary_T = heat.T = T_room\n    boundary_Q = heat.Q = Q_room\n}\n\ncomponent AmbientBoundary {\n    port heat: Thermal\n}\n\nsystem Envelope {\n    room = RoomBoundary(22 degC, Q_room=2 kW)\n    ambient = AmbientBoundary()\n    connect room.heat to ambient.heat\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let room = report
            .semantic_program
            .components
            .iter()
            .find(|component| component.name == "room")
            .expect("room instance");
        assert_eq!(room.constructor_arguments.len(), 2);
        assert_eq!(room.constructor_arguments[0].name, "T_room");
        assert_eq!(room.constructor_arguments[0].value, "22 degC");
        assert_eq!(room.constructor_arguments[1].name, "Q_room");
        assert_eq!(room.constructor_arguments[1].value, "2 kW");
        assert_eq!(room.parameters[0].value.as_deref(), Some("22 degC"));
        assert_eq!(room.parameters[0].status, "constructor_override");
        assert_eq!(room.parameters[1].value.as_deref(), Some("2 kW"));
        assert_eq!(room.parameters[1].status, "constructor_override");

        let review = review_json(&report);
        assert!(review.contains("\"name\": \"T_room\""));
        assert!(review.contains("\"value\": \"22 degC\""));
    }
    #[test]
    fn rejects_unsupported_system_component_constructor_shapes() {
        let unknown = check_source(
            "bad.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\nsystem Envelope {\n    room = MissingComponent()\n}\n",
            &CheckOptions::default(),
        );
        assert!(unknown.has_errors());
        assert!(unknown
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-COMPONENT-INSTANCE-UNKNOWN"));

        let with_args = check_source(
            "bad.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent RoomBoundary {\n    port heat: Thermal\n}\n\nsystem Envelope {\n    room = RoomBoundary(22 degC)\n}\n",
            &CheckOptions::default(),
        );
        assert!(with_args.has_errors());
        assert!(with_args
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-COMPONENT-INSTANCE-ARGS"));

        let unused_arg = check_source(
            "bad.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent RoomBoundary {\n    port heat: Thermal\n    boundary_T = heat.T = 22 degC\n}\n\nsystem Envelope {\n    room = RoomBoundary(unused=1 kW)\n}\n",
            &CheckOptions::default(),
        );
        assert!(unused_arg.has_errors());
        assert!(unused_arg
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-COMPONENT-INSTANCE-ARGS"));

        let duplicate_arg = check_source(
            "bad.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent RoomBoundary {\n    port heat: Thermal\n    boundary_T = heat.T = T_room\n}\n\nsystem Envelope {\n    room = RoomBoundary(T_room=22 degC, T_room=23 degC)\n}\n",
            &CheckOptions::default(),
        );
        assert!(duplicate_arg.has_errors());
        assert!(duplicate_arg
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-COMPONENT-INSTANCE-ARGS"));

        let positional_after_named = check_source(
            "bad.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent RoomBoundary {\n    port heat: Thermal\n    parameter T_room: AbsoluteTemperature [degC]\n    parameter Q_room: HeatRate [kW]\n}\n\nsystem Envelope {\n    room = RoomBoundary(T_room=22 degC, 1 kW)\n}\n",
            &CheckOptions::default(),
        );
        assert!(positional_after_named.has_errors());
        assert!(positional_after_named.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-COMPONENT-INSTANCE-ARGS"
                && diagnostic.message.contains("after named")
        }));

        let too_many_positional = check_source(
            "bad.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent RoomBoundary {\n    port heat: Thermal\n    parameter T_room: AbsoluteTemperature [degC]\n}\n\nsystem Envelope {\n    room = RoomBoundary(22 degC, 1 kW)\n}\n",
            &CheckOptions::default(),
        );
        assert!(too_many_positional.has_errors());
        assert!(too_many_positional.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-COMPONENT-INSTANCE-ARGS"
                && diagnostic.message.contains("too many positional")
        }));
    }

    #[test]
    fn lowers_component_local_equations_into_assembly_residuals() {
        let report = check_source(
            "ok.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent RoomBoundary {\n    port heat: Thermal\n    boundary_T = heat.T = 22 degC\n    balance_heat: heat.Q eq 0 kW\n}\n\ncomponent AmbientBoundary {\n    port heat: Thermal\n}\n\nsystem Envelope {\n    room = RoomBoundary()\n    ambient = AmbientBoundary()\n    connect room.heat to ambient.heat\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        let assembly = &report.semantic_program.component_assemblies[0];
        assert_eq!(assembly.boundary.balance_status, "balanced_metadata_seed");
        assert_eq!(assembly.boundary.equation_count, 4);
        assert_eq!(assembly.component_equation_count, 2);
        assert!(assembly
            .equations
            .iter()
            .any(|equation| equation.kind == "component_equation"
                && equation.expression == "room.heat.Q eq 0 kW"
                && equation.residual == "room.heat.Q"
                && equation.rhs.as_deref() == Some("0 kW")));
        assert_eq!(
            assembly.residual_graph.status,
            "linear_residual_graph_candidate"
        );
    }

    #[test]
    fn accepts_unit_parameterized_component_equations() {
        let source = r#"domain Thermal {
    across T: AbsoluteTemperature [degC]
    through Q: HeatRate [kW]
    conservation sum(Q) = 0
}

component ZoneBoundary {
    port heat: Thermal
    boundary_T = heat.T = 22 degC
}

component OutdoorBoundary {
    port heat: Thermal
    boundary_T = heat.T = 12 degC
}

component WallConductance {
    port inside: Thermal
    port outside: Thermal
    parameter UA: Conductance [W/K] = 500 W/K
    inside.Q eq UA * (inside.T - outside.T)
    outside.Q + inside.Q eq 0 kW
}

system Envelope {
    zone = ZoneBoundary()
    outdoor = OutdoorBoundary()
    wall = WallConductance()
    connect zone.heat to wall.inside
    connect wall.outside to outdoor.heat
}
"#;
        let report = check_source("ok.eng", source, &CheckOptions::default());

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let assembly = &report.semantic_program.component_assemblies[0];
        assert!(assembly.equations.iter().any(|equation| {
            equation.kind == "component_equation"
                && equation.expression
                    == "wall.inside.Q eq wall.UA * (wall.inside.T - wall.outside.T)"
                && equation.residual
                    == "wall.inside.Q - (wall.UA * (wall.inside.T - wall.outside.T))"
                && equation.dependencies.first().map(String::as_str) == Some("wall.inside.Q")
                && equation.dependencies.contains(&"wall.UA".to_owned())
        }));
    }

    #[test]
    fn parenthesizes_compound_component_equation_rhs_in_residuals() {
        let report = check_source(
            "ok.eng",
            "domain ScalarState {\n    across x: DimensionlessNumber [1]\n    through balance: DimensionlessNumber [1]\n    conservation sum(balance) = 0\n}\n\ncomponent DynamicNode {\n    port node: ScalarState\n    der(node.x) + node.balance eq 0\n}\n\ncomponent DrivenBoundary {\n    port node: ScalarState\n    input drive: DimensionlessNumber [1] = 0.25\n    node.balance * node.balance eq node.x + drive\n}\n\nsystem DrivenSystem {\n    node = DynamicNode()\n    boundary = DrivenBoundary()\n    connect node.node to boundary.node\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let assembly = &report.semantic_program.component_assemblies[0];
        let equation = assembly
            .equations
            .iter()
            .find(|equation| {
                equation.expression
                    == "boundary.node.balance * boundary.node.balance eq boundary.node.x + boundary.drive"
            })
            .expect("compound RHS component equation");
        assert_eq!(
            equation.residual,
            "boundary.node.balance * boundary.node.balance - (boundary.node.x + boundary.drive)"
        );
        assert!(equation.dependencies.contains(&"boundary.drive".to_owned()));
    }

    #[test]
    fn rejects_incompatible_unitful_component_equation_constants() {
        let report = check_source(
            "bad.eng",
            "domain Fluid[Medium M] {\n    across p: Pressure [Pa]\n    through m_dot: MassFlowRate [kg/s]\n    conservation sum(m_dot) = 0\n}\n\ncomponent Source {\n    port outlet: Fluid[Water]\n}\n\ncomponent Sink {\n    port inlet: Fluid[Water]\n}\n\ncomponent PipeRun {\n    port inlet: Fluid[Water]\n    port outlet: Fluid[Water]\n    outlet.p + 2 kg/s eq inlet.p\n    outlet.m_dot + inlet.m_dot eq 0\n}\n\nsystem Loop {\n    source = Source()\n    sink = Sink()\n    pipe = PipeRun()\n    connect source.outlet to pipe.inlet\n    connect pipe.outlet to sink.inlet\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-COMPONENT-EQUATION-UNIT-001"));
    }

    #[test]
    fn accepts_fixed_point_algebraic_solve_request() {
        let report = check_source(
            "ok.eng",
            "domain Scalar {\n    across x: DimensionlessNumber [1]\n    through balance: DimensionlessNumber [1]\n    conservation sum(balance) = 0\n}\n\ncomponent LoopNode {\n    port source: Scalar\n    port target: Scalar\n    source.x eq 0.5 * target.x\n    source.balance eq 0\n}\n\nsystem FixedPointLoop {\n    loop_node = LoopNode()\n    connect loop_node.source to loop_node.target\n}\n\nfixed_point_result = solve component_graph\nwith {\n    solver = fixed_point\n    tolerance = 0.000001\n    max_iter = 60\n    relaxation = 0.5\n    initial = 4\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert!(report.inferred_declarations.iter().any(|declaration| {
            declaration.name == "fixed_point_result"
                && declaration.quantity_kind == "ComponentSolveResult"
                && declaration.expression == "solve component_graph"
        }));
        assert!(report
            .semantic_program
            .with_blocks
            .iter()
            .flat_map(|block| block.options.iter())
            .any(|option| option.key == "relaxation" && option.status == "accepted"));
    }

    #[test]
    fn rejects_invalid_fixed_point_algebraic_solve_request() {
        let report = check_source(
            "bad.eng",
            "domain Scalar {\n    across x: DimensionlessNumber [1]\n    through balance: DimensionlessNumber [1]\n    conservation sum(balance) = 0\n}\n\ncomponent LoopNode {\n    port source: Scalar\n    port target: Scalar\n    source.x eq 0.5 * target.x\n    source.balance eq 0\n}\n\nsystem FixedPointLoop {\n    loop_node = LoopNode()\n    connect loop_node.source to loop_node.target\n}\n\nfixed_point_result = solve component_graph\nwith {\n    solver = fixed_point\n    tolerance = -1\n    max_iter = 0\n    relaxation = 2\n    initial = bad\n}\n\nmissing_result = solve missing_graph\nwith {\n    solver = fixed_point\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        for code in [
            "E-SOLVE-TOLERANCE-INVALID",
            "E-SOLVE-MAX-ITER-INVALID",
            "E-SOLVE-RELAXATION-INVALID",
            "E-SOLVE-INITIAL-INVALID",
            "E-SOLVE-ASSEMBLY-001",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code),
                "missing {code}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn accepts_dynamic_component_solve_request() {
        let report = check_source(
            "ok.eng",
            "domain ScalarState {\n    across x: DimensionlessNumber [1]\n    through balance: DimensionlessNumber [1]\n    conservation sum(balance) = 0\n}\n\ncomponent DecayNode {\n    port node: ScalarState\n    der(node.x) eq -0.5 * node.x\n}\n\nsystem DynamicExplicit {\n    node = DecayNode()\n    connect node.node to node.node\n}\n\nexplicit_result = solve component_graph\nwith {\n    solver = dynamic_component_explicit_euler\n    timestep = 1 s\n    duration = 3 s\n    initial = 4\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert!(report.inferred_declarations.iter().any(|declaration| {
            declaration.name == "explicit_result"
                && declaration.quantity_kind == "ComponentSolveResult"
                && declaration.expression == "solve component_graph"
        }));
        let assembly = &report.semantic_program.component_assemblies[0];
        assert_eq!(assembly.boundary.state_count, 1);
        assert!(assembly.equations.iter().any(|equation| equation
            .dependencies
            .contains(&"der(node.node.x)".to_owned())));
    }

    #[test]
    fn rejects_invalid_dynamic_component_solve_request() {
        let report = check_source(
            "bad.eng",
            "domain ScalarState {\n    across x: DimensionlessNumber [1]\n    through balance: DimensionlessNumber [1]\n    conservation sum(balance) = 0\n}\n\ncomponent DecayNode {\n    port node: ScalarState\n    der(node.x) eq -0.5 * node.x\n}\n\nsystem DynamicExplicit {\n    node = DecayNode()\n    connect node.node to node.node\n}\n\nexplicit_result = solve component_graph\nwith {\n    solver = dynamic_component_explicit_euler\n    timestep = never\n    initial = bad\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        for code in [
            "E-SOLVE-TIMESTEP-INVALID",
            "E-SOLVE-DURATION-INVALID",
            "E-SOLVE-INITIAL-INVALID",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code),
                "missing {code}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn accepts_newton_and_dae_component_solve_requests() {
        let report = check_source(
            "ok.eng",
            "domain Scalar {\n    across x: DimensionlessNumber [1]\n    across z: DimensionlessNumber [1]\n    through balance: DimensionlessNumber [1]\n    conservation sum(balance) = 0\n}\n\ncomponent ResidualNode {\n    port node: Scalar\n    node.x * node.x eq 2\n    der(node.z) + node.x eq 0\n}\n\nsystem SourceSolves {\n    node = ResidualNode()\n    connect node.node to node.node\n}\n\nnewton_result = solve component_graph\nwith {\n    solver = newton\n    initial = 1\n    finite_difference_step = 0.000001\n    damping = 1\n    line_search_steps = 8\n    jacobian = finite_difference\n}\n\ndae_result = solve component_graph\nwith {\n    solver = implicit_euler_dae\n    timestep = 1 s\n    duration = 2 s\n    initial = 1\n    initial_derivative = -1\n    initial_algebraic = 0\n    consistency_tolerance = 0.000001\n    algebraic_initialization = newton\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        for name in ["newton_result", "dae_result"] {
            assert!(report.inferred_declarations.iter().any(|declaration| {
                declaration.name == name
                    && declaration.quantity_kind == "ComponentSolveResult"
                    && declaration.expression == "solve component_graph"
            }));
        }
    }

    #[test]
    fn rejects_invalid_newton_and_dae_component_solve_options() {
        let report = check_source(
            "bad.eng",
            "domain Scalar {\n    across x: DimensionlessNumber [1]\n    through balance: DimensionlessNumber [1]\n    conservation sum(balance) = 0\n}\n\ncomponent ResidualNode {\n    port node: Scalar\n    node.x * node.x eq 2\n}\n\nsystem SourceSolves {\n    node = ResidualNode()\n    connect node.node to node.node\n}\n\nnewton_result = solve component_graph\nwith {\n    solver = newton\n    finite_difference_step = 0\n    damping = 2\n    line_search_steps = 0\n    jacobian = symbolic\n}\n\ndae_result = solve component_graph\nwith {\n    solver = implicit_euler_dae\n    timestep = never\n    duration = none\n    initial_derivative = bad\n    consistency_tolerance = 0\n    algebraic_initialization = maybe\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        for code in [
            "E-SOLVE-FD-STEP-INVALID",
            "E-SOLVE-DAMPING-INVALID",
            "E-SOLVE-LINE-SEARCH-STEPS-INVALID",
            "E-SOLVE-JACOBIAN-UNSUPPORTED",
            "E-SOLVE-TIMESTEP-INVALID",
            "E-SOLVE-DURATION-INVALID",
            "E-SOLVE-INITIAL-INVALID",
            "E-SOLVE-CONSISTENCY-TOLERANCE-INVALID",
            "E-SOLVE-ALGEBRAIC-INITIALIZATION-UNSUPPORTED",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code),
                "missing {code}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn rejects_invalid_component_local_equations() {
        let unknown_signal = check_source(
            "bad.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent Source {\n    port heat: Thermal\n    heat.unknown eq 0 kW\n}\n\ncomponent Sink {\n    port heat: Thermal\n}\n\nconnect Source.heat -> Sink.heat\n",
            &CheckOptions::default(),
        );
        assert!(unknown_signal.has_errors());
        assert!(unknown_signal
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-COMPONENT-EQUATION-SIGNAL-001"));

        let bad_unit = check_source(
            "bad.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent Source {\n    port heat: Thermal\n    heat.Q eq 1 m\n}\n\ncomponent Sink {\n    port heat: Thermal\n}\n\nconnect Source.heat -> Sink.heat\n",
            &CheckOptions::default(),
        );
        assert!(bad_unit.has_errors());
        assert!(bad_unit
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-COMPONENT-EQUATION-UNIT-001"));
    }

    #[test]
    fn rejects_invalid_component_delay_calls() {
        let missing_duration = check_source(
            "bad.eng",
            "domain Fluid {\n    across height: Length [m]\n    through m_dot: MassFlowRate [kg/s]\n    conservation sum(m_dot) = 0\n}\n\ncomponent Supply {\n    port outlet: Fluid\n    pressure_seed = delay(outlet.m_dot)\n}\n",
            &CheckOptions::default(),
        );
        assert!(missing_duration.has_errors());
        assert!(missing_duration
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-DELAY-CALL-001"));

        let bad_duration = check_source(
            "bad.eng",
            "domain Fluid {\n    across height: Length [m]\n    through m_dot: MassFlowRate [kg/s]\n    conservation sum(m_dot) = 0\n}\n\ncomponent Supply {\n    port outlet: Fluid\n    pressure_seed = delay(outlet.m_dot, 5 kg)\n}\n",
            &CheckOptions::default(),
        );
        assert!(bad_duration.has_errors());
        assert!(bad_duration
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-DELAY-DURATION-001"));

        let unknown_signal = check_source(
            "bad.eng",
            "domain Fluid {\n    across height: Length [m]\n    through m_dot: MassFlowRate [kg/s]\n    conservation sum(m_dot) = 0\n}\n\ncomponent Supply {\n    port outlet: Fluid\n    pressure_seed = delay(outlet.unknown, 5 s)\n}\n",
            &CheckOptions::default(),
        );
        assert!(unknown_signal.has_errors());
        assert!(unknown_signal
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-DELAY-SIGNAL-001"));
    }

    #[test]
    fn rejects_invalid_component_predictor_and_external_calls() {
        let predictor_extra_arg = check_source(
            "bad.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent Source {\n    port out: Thermal\n    prediction = predictor(out.T, out.Q)\n}\n",
            &CheckOptions::default(),
        );
        assert!(predictor_extra_arg.has_errors());
        assert!(predictor_extra_arg
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-PREDICTOR-CALL-001"));

        let predictor_unknown_signal = check_source(
            "bad.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent Source {\n    port out: Thermal\n    prediction = predict(out.unknown)\n}\n",
            &CheckOptions::default(),
        );
        assert!(predictor_unknown_signal.has_errors());
        assert!(predictor_unknown_signal
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-PREDICTOR-SIGNAL-001"));

        let external_extra_arg = check_source(
            "bad.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent Source {\n    port out: Thermal\n    adapter_value = external(out.T, out.Q)\n}\n",
            &CheckOptions::default(),
        );
        assert!(external_extra_arg.has_errors());
        assert!(external_extra_arg
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-EXTERNAL-BEHAVIOR-CALL-001"));

        let external_unknown_signal = check_source(
            "bad.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent Source {\n    port out: Thermal\n    adapter_value = adapter(out.unknown)\n}\n",
            &CheckOptions::default(),
        );
        assert!(external_unknown_signal.has_errors());
        assert!(external_unknown_signal
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-EXTERNAL-BEHAVIOR-SIGNAL-001"));
    }

    #[test]
    fn records_predictor_contract_seed_status_in_component_preview() {
        let report = check_source(
            "ok.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent Source {\n    port out: Thermal\n    prediction = predictor(out.T)\n}\n\ncomponent Sink {\n    port inlet: Thermal\n}\n\nconnect Source.out -> Sink.inlet\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        let assembly = &report.semantic_program.component_assemblies[0];
        assert_eq!(assembly.predictor_call_count, 1);
        assert_eq!(
            assembly.solver_preview.predictor,
            "predictor_call_contract_seed_not_integrated"
        );
    }

    #[test]
    fn records_external_behavior_seed_status_in_component_preview() {
        let report = check_source(
            "ok.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent Source {\n    port out: Thermal\n    adapter_value = adapter(out.T)\n}\n\ncomponent Sink {\n    port inlet: Thermal\n}\n\nconnect Source.out -> Sink.inlet\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        let assembly = &report.semantic_program.component_assemblies[0];
        assert_eq!(
            assembly.solver_preview.external_adapter,
            "external_behavior_wrapper_seed_not_integrated"
        );
    }

    #[test]
    fn diagnoses_duplicate_connections_and_warns_unconnected_ports() {
        let report = check_source(
            "bad.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent A {\n    port heat: Thermal\n}\n\ncomponent B {\n    port heat: Thermal\n}\n\ncomponent C {\n    port heat: Thermal\n}\n\nconnect A.heat -> B.heat\nconnect B.heat -> A.heat\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-CONNECT-DUPLICATE-001"));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "W-CONNECT-UNCONNECTED-PORT"
                && diagnostic.severity == Severity::Warning
        }));
    }

    #[test]
    fn records_class_object_metadata_and_field_access() {
        let report = check_source(
            "class_object.eng",
            "class Construction {\n    name: String\n    u_value: Conductance [W/K]\n    thickness: Length [m] = 0.2 m\n    validate {\n        u_value > 0 W/K\n        thickness > 0 m\n    }\n    method summary() -> String = self.name\n}\n\nclass Zone {\n    name: String\n    capacity: HeatCapacity [J/K]\n}\n\nwall = Construction {\n    name = \"South\"\n    u_value = 120 W/K\n}\n\nbetter_wall = wall with {\n    u_value = 100 W/K\n}\n\nzone = Zone {\n    name = \"Office\"\n    capacity = 120000 J/K\n}\n\nwall_u = better_wall.u_value\nwall_summary = better_wall.summary()\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert_eq!(report.syntax_summary.classes, 2);
        assert_eq!(report.syntax_summary.class_validations, 2);
        assert_eq!(report.syntax_summary.class_methods, 1);
        assert_eq!(report.syntax_summary.class_objects, 2);
        assert_eq!(report.syntax_summary.class_object_copies, 1);
        assert_eq!(report.semantic_program.classes.len(), 2);
        assert_eq!(report.semantic_program.class_objects.len(), 3);
        assert_eq!(report.semantic_program.classes[0].validations.len(), 2);
        assert_eq!(report.semantic_program.classes[0].methods.len(), 1);
        assert_eq!(
            report.semantic_program.class_objects[0].validations.len(),
            2
        );
        assert_eq!(
            report.semantic_program.class_objects[1]
                .source_object
                .as_deref(),
            Some("wall")
        );
        assert_eq!(
            report.semantic_program.class_objects[1].construction,
            "copy_with"
        );
        assert!(report.semantic_program.class_objects[0]
            .validations
            .iter()
            .all(|validation| validation.status == "pass"));
        assert!(report
            .semantic_program
            .typed_bindings
            .iter()
            .any(|binding| {
                binding.name == "wall_u" && binding.semantic_type.quantity_kind == "Conductance"
            }));
        assert!(report
            .semantic_program
            .typed_bindings
            .iter()
            .any(|binding| {
                binding.name == "wall_summary" && binding.semantic_type.quantity_kind == "String"
            }));
        let review = review_json(&report);
        assert!(review.contains("\"class_summary\""));
        assert!(review.contains("\"object_summary\""));
        assert!(review.contains("\"validation_count\": 2"));
        assert!(review.contains("\"method_count\": 1"));
        assert!(review.contains("\"construction\": \"copy_with\""));
    }

    #[test]
    fn rejects_failed_class_validation() {
        let report = check_source(
            "bad_class_validation.eng",
            "class Construction {\n    u_value: Conductance [W/K]\n    validate {\n        u_value > 0 W/K\n    }\n}\n\nbad = Construction {\n    u_value = 0 W/K\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-CLASS-VALIDATION-002"));
        assert_eq!(
            report.semantic_program.class_objects[0].validations[0].status,
            "fail"
        );
    }

    #[test]
    fn rejects_invalid_class_methods_and_copy_with() {
        let report = check_source(
            "bad_class_methods.eng",
            "class Construction {\n    name: String\n    u_value: Conductance [W/K]\n    method bad() -> Length [m] = self.u_value\n}\n\nwall = Construction {\n    name = \"South\"\n    u_value = 120 W/K\n}\n\ncopy_missing = nope with {\n    u_value = 100 W/K\n}\n\nbad_call = wall.missing()\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        for expected_code in [
            "E-CLASS-METHOD-RETURN-001",
            "E-CLASS-COPY-001",
            "E-CLASS-METHOD-CALL-002",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == expected_code),
                "expected {expected_code}"
            );
        }
    }

    #[test]
    fn rejects_invalid_class_object_fields() {
        let report = check_source(
            "bad_class.eng",
            "class Construction {\n    u_value: Conductance [W/K]\n    thickness: Length [m]\n}\n\nbad = Construction {\n    u_value = 2 m\n    unknown = 1 m\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        for expected_code in [
            "E-CLASS-FIELD-TYPE-001",
            "E-CLASS-FIELD-UNKNOWN-001",
            "E-CLASS-FIELD-MISSING-001",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == expected_code),
                "expected {expected_code}"
            );
        }
    }

    #[test]
    fn rejects_incompatible_port_connection_domains() {
        let report = check_source(
            "bad.eng",
            "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ndomain Fluid {\n    across height: Length [m]\n    through m_dot: MassFlowRate [kg/s]\n    conservation sum(m_dot) = 0\n}\n\ncomponent Heater {\n    port heat: Thermal\n}\n\ncomponent Pipe {\n    port inlet: Fluid\n}\n\nconnect Heater.heat -> Pipe.inlet\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-CONNECT-DOMAIN-MISMATCH"));
        assert_eq!(
            report.semantic_program.connections[0].status,
            "domain_mismatch"
        );
    }

    #[test]
    fn rejects_generic_domain_parameter_mismatches() {
        for (parameter, left, right, expected_code, expected_status) in [
            (
                "Medium",
                "Water",
                "Air",
                "E-CONNECT-MEDIUM-MISMATCH",
                "medium_mismatch",
            ),
            (
                "Frame",
                "World",
                "Body",
                "E-CONNECT-FRAME-001",
                "frame_mismatch",
            ),
            ("Axis", "X", "Y", "E-CONNECT-AXIS-001", "axis_mismatch"),
        ] {
            let source = format!(
                "domain Generic[{parameter} P] {{\n    across x: Length [m]\n    through m_dot: MassFlowRate [kg/s]\n    conservation sum(m_dot) = 0\n}}\n\ncomponent Left {{\n    port p: Generic[{left}]\n}}\n\ncomponent Right {{\n    port p: Generic[{right}]\n}}\n\nconnect Left.p -> Right.p\n"
            );
            let report = check_source("bad.eng", &source, &CheckOptions::default());

            assert!(report.has_errors());
            assert!(report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == expected_code));
            assert_eq!(
                report.semantic_program.connections[0].status,
                expected_status
            );
        }
    }

    #[test]
    fn rejects_generic_domain_arity_mismatch() {
        let report = check_source(
            "bad.eng",
            "domain Fluid[Medium M] {\n    across height: Length [m]\n    through m_dot: MassFlowRate [kg/s]\n    conservation sum(m_dot) = 0\n}\n\ncomponent Pipe {\n    port inlet: Fluid\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-PORT-DOMAIN-002"));
        assert_eq!(
            report.semantic_program.components[0].ports[0].status,
            "generic_arity_mismatch"
        );
    }

    #[test]
    fn rejects_incomplete_domain_contracts() {
        let report = check_source(
            "bad.eng",
            "domain Incomplete {\n    across x: Length [m]\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-DOMAIN-CONTRACT-002"));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-DOMAIN-CONTRACT-003"));
    }

    #[test]
    fn rejects_script_workflow_syntax() {
        let report = check_source(
            "ok.eng",
            "script main(args: Args) -> Report {\n    L = 1 m\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SCRIPT-001"));
    }

    #[test]
    fn records_top_level_workflow() {
        let report = check_source("ok.eng", "L = 1 m\n", &CheckOptions::default());

        assert_eq!(report.semantic_program.workflow.kind, "top_level");
        assert_eq!(
            report.semantic_program.workflow.signature(),
            "top-level workflow(args: Args) -> Report"
        );
    }

    #[test]
    fn bytecode_v1_round_trips_workflow_and_instructions() {
        let source = "L = 1 m\n";
        let report = check_source("ok.eng", source, &CheckOptions::default());

        let bytecode = build_bytecode(&report, source);
        let decoded = parse_bytecode(&bytecode).unwrap();

        assert!(bytecode.starts_with("ENGBYTECODE 1\nformat = engbc-v1\n"));
        assert!(bytecode.contains("workflow = top_level\n"));
        assert!(bytecode.contains("0000|enter_workflow|top_level\n"));
        assert_eq!(decoded.workflow.kind, "top_level");
        assert_eq!(
            decoded.instructions.last(),
            Some(&BytecodeInstruction::WriteResult {
                format: "engres-v1".to_owned()
            })
        );
    }

    #[test]
    fn records_timeseries_axis_summary_and_integrate_metadata() {
        let report = check_source(
            "ok.eng",
            "sensor = promote csv \"data/sensor.csv\" as SensorData\ncp = 4180 J/kg/K\nQ_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\nE_coil = integrate(Q_coil, over=Time)\n\nreport {\n    summarize Q_coil by [mean, max, p95]\n}\n",
            &CheckOptions::default(),
        );

        let q_type = report
            .semantic_program
            .typed_bindings
            .iter()
            .find(|binding| binding.name == "Q_coil")
            .unwrap();

        assert_eq!(
            q_type.semantic_type.quantity_kind,
            "TimeSeries[Time] of HeatRate"
        );
        assert!(report
            .semantic_program
            .axis_infos
            .iter()
            .any(|axis| axis.binding == "Q_coil" && axis.axis == "Time"));
        assert_eq!(report.semantic_program.stats_infos[0].source, "Q_coil");
        assert_eq!(
            report.semantic_program.stats_infos[0].statistics,
            vec!["mean", "max", "p95"]
        );
        assert_eq!(report.semantic_program.integrations[0].binding, "E_coil");
        assert_eq!(
            report.semantic_program.integrations[0].input_quantity,
            "HeatRate"
        );
        assert_eq!(report.semantic_program.timeseries_kernels.len(), 1);
        let kernel = &report.semantic_program.timeseries_kernels[0];
        assert_eq!(kernel.binding, "Q_coil");
        assert_eq!(kernel.kind, "table_heat_rate_from_mass_flow_cp_delta_t");
        assert_eq!(kernel.source_table.as_deref(), Some("sensor"));
        assert_eq!(kernel.status, "supported");
        assert!(kernel
            .operations
            .iter()
            .any(|operation| operation == "temperature_delta:return_minus_supply"));
        let review = review_json(&report);
        assert!(review.contains("\"timeseries_kernels\""));
        assert!(review.contains("\"table_heat_rate_from_mass_flow_cp_delta_t\""));
    }

    #[test]
    fn records_timeseries_sensor_std_uncertainty_metadata() {
        let report = check_source(
            "ok.eng",
            "T_zone: TimeSeries[Time] of AbsoluteTemperature [degC] = 24 degC\nwith {\n    sensor_std = 0.2 K\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert!(report.semantic_program.with_blocks[0]
            .options
            .iter()
            .any(|option| option.key == "sensor_std"
                && option.value == "0.2 K"
                && option.status == "accepted"));
        let review = review_json(&report);
        assert!(review.contains("\"timeseries_uncertainty\""));
        assert!(review.contains("\"binding\": \"T_zone\""));
        assert!(review.contains("\"method\": \"pointwise_measured_std\""));
        assert!(review.contains("\"sensor_std\": \"0.2 K\""));
    }

    #[test]
    fn rejects_invalid_timeseries_sensor_std_metadata() {
        let report = check_source(
            "bad.eng",
            "Q: HeatRate [kW] = 1 kW\nwith {\n    sensor_std = 0.2 K\n}\nT_zone: TimeSeries[Time] of AbsoluteTemperature [degC] = 24 degC\nwith {\n    sensor_std = 1 kW\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert_eq!(
            report
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == "E-UNC-TS-STD-001")
                .count(),
            2
        );
    }

    #[test]
    fn records_timeseries_uncertainty_calculation_metadata() {
        let report = check_source(
            "ok.eng",
            "Q_series: TimeSeries[Time] of HeatRate [kW] = 5 kW\nwith {\n    sensor_std = 0.2 kW\n}\nE = integrate(Q_series, over=Time)\n\nreport {\n    summarize Q_series by [mean, p95, duration_above(4 kW)]\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let review = review_json(&report);
        assert!(review.contains("\"timeseries_uncertainty_calculations\""));
        assert!(review.contains("\"kind\": \"timeseries_statistics\""));
        assert!(review.contains("\"kind\": \"timeseries_duration_above\""));
        assert!(review.contains("\"kind\": \"timeseries_integrate\""));
        assert!(review.contains("\"statistics\": [\"mean\", \"p95\"]"));
        assert!(review.contains("\"operation\": \"duration_above\""));
        assert!(review.contains("\"statistics\": [\"duration_above(4 kW)\"]"));
        assert!(review.contains("\"sensor_std\": \"0.2 kW\""));
        assert!(review.contains("\"status\": \"metadata_only\""));
    }

    #[test]
    fn records_unit_aware_print_and_csv_export_metadata() {
        let report = check_source(
            "ok.eng",
            "cp = 4180 J/kg/K\nQ_series = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\nmean_Q = mean(Q_series, axis=Time)\nQ = 10 kW\nE: Energy [J] = 3600 J\nprint \"Q={Q: .2 kW} E={E: .3 kWh}\"\nlog info \"run started for {Q: .1 kW}\"\nlog warn \"check energy {E: .3 kWh}\"\nlog debug \"debug detail\"\nlog error \"review required\"\nprocess_result = run command \"cmd\"\nwith {\n    args = [\"/C\", \"echo\", \"ok\"]\n}\nexport summary to csv \"summary.csv\" {\n    Q as kW with \".2\"\n    E as kWh with \".3\"\n    mean_Q as kW with \".2\"\n}\nwith {\n    overwrite = true\n}\nwrite text \"summary.txt\", Q\nwrite json \"summary.json\", E\ncopy file(\"source.txt\") to \"copied.txt\"\nmove \"copied.txt\" to \"moved.txt\"\nwith {\n    confirm = true\n    overwrite = true\n}\ndelete \"moved.txt\"\nwith {\n    confirm = true\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert_eq!(report.semantic_program.prints.len(), 5);
        assert_eq!(report.semantic_program.prints[0].level, "print");
        assert_eq!(report.semantic_program.prints[1].level, "info");
        assert_eq!(report.semantic_program.prints[2].level, "warn");
        assert_eq!(report.semantic_program.prints[3].level, "debug");
        assert_eq!(report.semantic_program.prints[4].level, "error");
        assert_eq!(report.semantic_program.prints[0].fields.len(), 2);
        assert_eq!(
            report.semantic_program.prints[0].fields[0]
                .requested_unit
                .as_deref(),
            Some("kW")
        );
        assert_eq!(report.semantic_program.csv_exports.len(), 1);
        assert_eq!(report.semantic_program.csv_exports[0].source, "summary");
        assert_eq!(report.semantic_program.csv_exports[0].fields.len(), 3);
        assert_eq!(
            report.semantic_program.csv_exports[0].fields[1]
                .requested_unit
                .as_deref(),
            Some("kWh")
        );
        assert_eq!(report.semantic_program.writes.len(), 2);
        assert_eq!(report.semantic_program.writes[0].format, "text");
        assert_eq!(report.semantic_program.writes[1].format, "json");
        assert_eq!(report.semantic_program.file_operations.len(), 3);
        assert_eq!(report.semantic_program.file_operations[0].operation, "copy");
        assert_eq!(report.semantic_program.file_operations[1].operation, "move");
        assert_eq!(
            report.semantic_program.file_operations[2].operation,
            "delete"
        );
        assert_eq!(report.semantic_program.process_runs.len(), 1);
        assert_eq!(
            report.semantic_program.process_runs[0].binding,
            "process_result"
        );
        assert!(report
            .semantic_program
            .typed_bindings
            .iter()
            .any(|binding| binding.name == "process_result"
                && binding.semantic_type.quantity_kind == "ProcessResult"));
        assert_eq!(report.semantic_program.with_blocks.len(), 4);
        let review = review_json(&report);
        assert!(review.contains("\"prints\""));
        assert!(review.contains("\"level\": \"warn\""));
        assert!(review.contains("\"csv_exports\""));
        assert!(review.contains("\"writes\""));
        assert!(review.contains("\"file_operations\""));
        assert!(review.contains("\"process_runs\""));
        assert!(review.contains("\"overwrite\""));
        assert!(review.contains("\"confirm\""));
    }

    #[test]
    fn supports_expression_print_without_string_template_quotes() {
        let report = check_source(
            "ok.eng",
            "Q = 10 kW\nprint Q: .1 kW\nprint Q = {Q: .2 kW}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert_eq!(report.semantic_program.prints.len(), 2);
        assert_eq!(report.semantic_program.prints[0].template, "{Q: .1 kW}");
        assert_eq!(report.semantic_program.prints[0].fields.len(), 1);
        assert_eq!(
            report.semantic_program.prints[0].fields[0]
                .requested_unit
                .as_deref(),
            Some("kW")
        );
        assert_eq!(report.semantic_program.prints[1].template, "Q = {Q: .2 kW}");
    }

    #[test]
    fn rejects_unknown_expression_print_variable() {
        let report = check_source("bad.eng", "print missing_value", &CheckOptions::default());

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-PRINT-FMT-004"));
    }

    #[test]
    fn rejects_invalid_log_levels() {
        let report = check_source(
            "bad.eng",
            "log trace \"too noisy\"\nlog \"missing level\"\n",
            &CheckOptions::default(),
        );

        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-LOG-LEVEL-001" && diagnostic.line == 1));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-LOG-LEVEL-001" && diagnostic.line == 2));
    }

    #[test]
    fn rejects_invalid_process_runs() {
        let report = check_source(
            "bad.eng",
            "run command \"cmd\"\nprocess_result = run command \"\"\nother_result = run command\n",
            &CheckOptions::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-PROCESS-BINDING-001" && diagnostic.line == 1
        }));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-PROCESS-CMD-001" && diagnostic.line == 2));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-PROCESS-CMD-001" && diagnostic.line == 3));
    }

    #[test]
    fn records_test_assert_and_golden_metadata() {
        let report = check_source(
            "test.eng",
            "Q = 10 kW\nexport summary to csv \"summary.csv\" {\n    Q as kW with \".1\"\n}\n\ntest \"summary values\" {\n    assert Q == 10 kW within 0.01 kW\n    golden \"summary.csv\" matches file(\"golden/summary.csv\")\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert_eq!(report.syntax_summary.tests, 1);
        assert_eq!(report.semantic_program.tests.len(), 1);
        assert_eq!(report.semantic_program.tests[0].name, "summary values");
        assert_eq!(report.semantic_program.tests[0].assertions.len(), 1);
        assert_eq!(report.semantic_program.tests[0].goldens.len(), 1);
        let review = review_json(&report);
        assert!(review.contains("\"tests\""));
        assert!(review.contains("\"goldens\""));
    }

    #[test]
    fn rejects_invalid_test_assertions() {
        let report = check_source(
            "bad_test.eng",
            "assert Q == 1 kW\n\ntest \"bad\" {\n    assert Q\n    assert 1 m == 1 kW\n    golden \"summary.csv\"\n}\n",
            &CheckOptions::default(),
        );

        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "E-ASSERT-001" && diagnostic.line == 1 }));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-ASSERT-002" && diagnostic.line == 4));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-ASSERT-UNIT-001" && diagnostic.line == 5));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-GOLDEN-002" && diagnostic.line == 6));
    }

    #[test]
    fn rejects_unconfirmed_file_mutations() {
        let report = check_source(
            "bad.eng",
            "move \"a.txt\" to \"b.txt\"\ndelete dir(\"old\")\nwith {\n    confirm = true\n}\n",
            &CheckOptions::default(),
        );

        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-FS-CONFIRM-001" && diagnostic.line == 1));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-FS-DELETE-001" && diagnostic.line == 2));
    }

    #[test]
    fn lowers_command_style_statistics_and_integration() {
        let report = check_source(
            "ok.eng",
            "cp = 4180 J/kg/K\nQ_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\nE_coil = integrate Q_coil over Time\nmean_Q = mean Q_coil over Time\npeak_Q = max Q_coil over Time\nprint \"mean={mean_Q: .2 kW} peak={peak_Q: .2 kW} E={E_coil: .2 kWh}\"\nexport summary to csv \"summary.csv\" {\n    mean_Q as kW with \".2\"\n    peak_Q as kW with \".2\"\n    E_coil as kWh with \".2\"\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert_eq!(report.semantic_program.command_styles.len(), 3);
        assert!(report
            .semantic_program
            .command_styles
            .iter()
            .any(|command| command.canonical == "integrate(Q_coil, over=Time)"));
        assert_eq!(report.semantic_program.integrations[0].binding, "E_coil");
        assert_eq!(
            report
                .inferred_declarations
                .iter()
                .find(|declaration| declaration.name == "mean_Q")
                .unwrap()
                .expression,
            "mean(Q_coil, axis=Time)"
        );
        let review = review_json(&report);
        assert!(review.contains("\"command_styles\""));
        assert!(review.contains("\"canonical\": \"max(Q_coil, axis=Time)\""));
    }

    #[test]
    fn records_where_and_with_context_for_command_owner() {
        let report = check_source(
            "ok.eng",
            "cp = 4180 J/kg/K\nE_from_local = integrate Q_local over Time\nwhere {\n    Q_local = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\n}\nwith {\n    method = trapezoidal\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert_eq!(report.semantic_program.integrations[0].source, "Q_local");
        assert_eq!(report.semantic_program.where_blocks.len(), 1);
        assert_eq!(
            report.semantic_program.where_blocks[0].bindings[0].quantity_kind,
            "TimeSeries[Time] of HeatRate"
        );
        assert_eq!(report.semantic_program.with_blocks.len(), 1);
        assert_eq!(
            report.semantic_program.with_blocks[0].options[0].key,
            "method"
        );
        let review = review_json(&report);
        assert!(review.contains("\"where_blocks\""));
        assert!(review.contains("\"with_blocks\""));
    }

    #[test]
    fn reports_command_where_and_with_policy_diagnostics() {
        let command_report = check_source(
            "bad.eng",
            "Q1 = 1 kW\nQ2 = 2 kW\nE = integrate Q1 + Q2 over Time\n",
            &CheckOptions::default(),
        );
        assert!(command_report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-CMD-AMBIG-001"));

        let where_report = check_source(
            "bad.eng",
            "E = integrate Q_local over Time\nwhere {\n    Q_local = Q_late\n    Q_late = 1 kW\n}\nprint \"local={Q_local: .2 kW}\"\n",
            &CheckOptions::default(),
        );
        assert!(where_report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-WHERE-FWD-001"));
        assert!(where_report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-NAME-LOCAL-001"));

        let with_report = check_source(
            "bad.eng",
            "Q = 1 kW\nwith { unit y = m; banana = x }\n",
            &CheckOptions::default(),
        );
        assert!(with_report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-WITH-UNIT-001"));
        assert!(with_report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-WITH-OPTION-001"));
    }

    #[test]
    fn rejects_unknown_command_style_verb() {
        let report = check_source(
            "bad.eng",
            "Q = 1 kW\nmedian_Q = median Q over Time\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-CMD-UNKNOWN-VERB"));
        assert_eq!(report.semantic_program.command_styles.len(), 1);
        assert_eq!(report.semantic_program.command_styles[0].verb, "median");
        assert_eq!(
            report.semantic_program.command_styles[0].status,
            "unknown_verb"
        );
    }

    #[test]
    fn validates_command_style_validate_comparisons() {
        let report = check_source(
            "ok.eng",
            "rmse_T = rmse measured.T_zone vs sim.T_zone\nvalidate rmse_T < 5 K\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert!(report
            .semantic_program
            .command_styles
            .iter()
            .any(|command| command.canonical == "validate(rmse_T < 5 K)"));
    }

    #[test]
    fn rejects_invalid_validate_command_expressions() {
        let non_bool = check_source(
            "bad.eng",
            "rmse_T = rmse measured.T_zone vs sim.T_zone\nvalidate rmse_T\n",
            &CheckOptions::default(),
        );
        assert!(non_bool.has_errors());
        assert!(non_bool
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-VALIDATE-BOOL-001"));

        let unit_mismatch = check_source(
            "bad.eng",
            "rmse_T = rmse measured.T_zone vs sim.T_zone\nvalidate rmse_T < 5 m\n",
            &CheckOptions::default(),
        );
        assert!(unit_mismatch.has_errors());
        assert!(unit_mismatch
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-VALIDATE-UNIT-001"));

        let unresolved = check_source(
            "bad.eng",
            "validate missing_metric < 5 K\n",
            &CheckOptions::default(),
        );
        assert!(unresolved.has_errors());
        assert!(unresolved
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-VALIDATE-EXPR-001"));
    }

    #[test]
    fn rejects_incompatible_print_and_csv_export_units() {
        let report = check_source(
            "bad.eng",
            "L: Length [m] = 1 m\nprint \"bad {L: .2 kW}\"\nexport summary to csv \"bad.csv\" {\n    L as kW with \".2\"\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-PRINT-FMT-003"));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-EXPORT-CSV-004"));
    }

    #[test]
    fn records_uncertainty_core_metadata() {
        let report = check_source(
            "ok.eng",
            "T_supply_meas = measured(12 degC, std=0.2 K)\nT_return_band = interval(20 degC, 24 degC)\nL_sensor_meas = measured(10 m, error=1 %)\nQ_coil_dist = normal(mean=5 kW, std=0.8 kW, samples=31)\nQ_uniform = uniform(4 kW, 6 kW, samples=11)\nQ_coil_ensemble = ensemble(Q_coil_dist, samples=31)\nQ_total_unc = propagate(Q_coil_dist, method=linear, scale=1.08, offset=0.4 kW)\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert_eq!(report.semantic_program.uncertainty_infos.len(), 7);
        assert_eq!(
            report.semantic_program.uncertainty_infos[0].kind,
            "Measured"
        );
        assert_eq!(
            report.semantic_program.uncertainty_infos[3].sample_count,
            31
        );
        assert_eq!(
            report.semantic_program.uncertainty_infos[4]
                .distribution
                .as_deref(),
            Some("uniform")
        );
        assert_eq!(
            report.semantic_program.uncertainty_infos[2]
                .error
                .as_deref(),
            Some("1 %")
        );
        assert_eq!(
            report.semantic_program.uncertainty_infos[3].display_unit,
            "kW"
        );
        assert_eq!(
            report.semantic_program.uncertainty_infos[6]
                .source
                .as_deref(),
            Some("Q_coil_dist")
        );
        assert_eq!(
            report.semantic_program.uncertainty_infos[6].display_unit,
            "kW"
        );
        let ensemble_type = report
            .semantic_program
            .typed_bindings
            .iter()
            .find(|binding| binding.name == "Q_coil_ensemble")
            .expect("Q_coil_ensemble type");
        assert_eq!(ensemble_type.semantic_type.display_unit, "kW");
        let propagated_type = report
            .semantic_program
            .typed_bindings
            .iter()
            .find(|binding| binding.name == "Q_total_unc")
            .expect("Q_total_unc type");
        assert_eq!(propagated_type.semantic_type.display_unit, "kW");
        assert_eq!(
            report.semantic_program.uncertainty_infos[6]
                .method
                .as_deref(),
            Some("linear")
        );
        assert_eq!(
            report.semantic_program.uncertainty_infos[6]
                .scale
                .as_deref(),
            Some("1.08")
        );
        assert_eq!(
            report.semantic_program.uncertainty_infos[6]
                .offset
                .as_deref(),
            Some("0.4 kW")
        );

        let review = review_json(&report);
        assert!(review.contains("\"uncertainty_info\""));
        assert!(review.contains("\"uncertainty_summary\""));
        assert!(review.contains("\"uncertainty_propagation\""));
        assert!(review.contains("\"variable\": \"Q_coil_dist\""));
        assert!(review.contains("\"representation\": \"Distribution\""));
        assert!(review.contains("\"normal_distribution\""));
        assert!(review.contains("\"output\": \"Q_total_unc\""));
        assert!(review.contains("\"source_terms_recorded\""));
        assert!(review.contains("\"distribution\": \"uniform\""));
        assert!(review.contains("\"error\": \"1 %\""));
        assert!(review.contains("\"scale\": \"1.08\""));
        assert!(review.contains("\"offset\": \"0.4 kW\""));
        assert!(review.contains("\"Measured[AbsoluteTemperature]\""));
        assert!(review.contains("\"Distribution[HeatRate]\""));
    }

    #[test]
    fn records_uncertainty_arithmetic_metadata() {
        let report = check_source(
            "ok.eng",
            "Q_meas = measured(10 kW, std=1 kW)\nQ_total = Q_meas + 2 kW\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert_eq!(report.semantic_program.uncertainty_infos.len(), 2);
        let derived = &report.semantic_program.uncertainty_infos[1];
        assert_eq!(derived.binding, "Q_total");
        assert_eq!(derived.kind, "Measured");
        assert_eq!(derived.quantity_kind, "HeatRate");
        assert_eq!(derived.display_unit, "kW");
        assert_eq!(derived.source.as_deref(), Some("Q_meas"));
        assert_eq!(derived.distribution.as_deref(), Some("arithmetic"));
        assert_eq!(derived.method.as_deref(), Some("linear"));
        assert_eq!(derived.propagation.len(), 1);
        assert_eq!(derived.propagation[0].source, "Q_meas");
        let derived_type = report
            .semantic_program
            .typed_bindings
            .iter()
            .find(|binding| binding.name == "Q_total")
            .expect("Q_total type");
        assert_eq!(
            derived_type.semantic_type.quantity_kind,
            "Measured[HeatRate]"
        );
    }

    #[test]
    fn records_uncertainty_with_policy_metadata() {
        let report = check_source(
            "ok.eng",
            "Q_meas = measured(10 kW, std=1 kW)\nQ_total = Q_meas + 2 kW\nwith {\n    uncertainty = linear\n    samples = 64\n    seed = 42\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert_eq!(report.semantic_program.with_blocks.len(), 1);
        assert!(report.semantic_program.with_blocks[0]
            .options
            .iter()
            .any(|option| option.key == "uncertainty"
                && option.value == "linear"
                && option.status == "accepted"));
        let review = review_json(&report);
        assert!(review.contains("\"uncertainty_policies\""));
        assert!(review.contains("\"method\": \"linear\""));
        assert!(review.contains("\"samples\": 64"));
        assert!(review.contains("\"seed\": 42"));
        assert!(review.contains("\"status\": \"accepted\""));
    }

    #[test]
    fn validates_uncertainty_with_policy_options() {
        let warning_report = check_source(
            "warn.eng",
            "Q_meas = measured(10 kW, std=1 kW)\nQ_mc = Q_meas + 2 kW\nwith {\n    uncertainty = monte_carlo\n    samples = 64\n}\n",
            &CheckOptions::default(),
        );

        assert!(
            !warning_report.has_errors(),
            "{:?}",
            warning_report.diagnostics
        );
        assert!(warning_report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "W-WITH-UNCERTAINTY-SEED-001"
                && diagnostic.severity == Severity::Warning
        }));
        let warning_review = review_json(&warning_report);
        assert!(warning_review.contains("\"status\": \"missing_seed_warning\""));

        let error_report = check_source(
            "bad.eng",
            "Q_meas = measured(10 kW, std=1 kW)\nQ_bad = Q_meas + 2 kW\nwith {\n    uncertainty = quadratic\n    samples = 0\n    seed = abc\n}\n",
            &CheckOptions::default(),
        );

        assert!(error_report.has_errors());
        assert!(error_report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-WITH-UNCERTAINTY-POLICY-001"));
        assert!(error_report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-WITH-UNCERTAINTY-SAMPLES-001"));
        assert!(error_report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-WITH-UNCERTAINTY-SEED-001"));
    }

    #[test]
    fn rejects_direct_uncertainty_validation_and_assertion() {
        let report = check_source(
            "bad.eng",
            "Q = normal(mean=5 kW, std=0.8 kW, samples=31)\nvalidate Q < 10 kW\n\ntest \"uncertain\" {\n    assert Q < 10 kW\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert_eq!(
            report
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == "E-UNC-DIRECT-COMPARE")
                .count(),
            2
        );
    }

    #[test]
    fn accepts_explicit_uncertainty_validation_statistics() {
        let report = check_source(
            "ok.eng",
            "Q = normal(mean=5 kW, std=0.8 kW, samples=31)\nvalidate p95(Q) < 10 kW\nvalidate probability(Q < 10 kW) > 0.95\nvalidate mean(Q) between 4 kW and 6 kW\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert!(report
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "E-UNC-DIRECT-COMPARE"));
    }

    #[test]
    fn rejects_invalid_uncertainty_probability_and_percentile_units() {
        let report = check_source(
            "bad.eng",
            "Q = normal(mean=5 kW, std=0.8 kW, samples=31)\nvalidate p95(Q) < 10 m\nvalidate probability(Q < 10 m) > 0.95\nvalidate probability(5 kW < 10 kW) > 0.95\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-UNC-PERCENTILE-UNIT-MISMATCH"));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-UNC-PROBABILITY-EXPR-INVALID"));
    }

    #[test]
    fn rejects_unresolved_uncertainty_source() {
        let report = check_source(
            "bad.eng",
            "Q_total_unc = propagate(Q_missing, method=linear)\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-UNC-SOURCE-001"));
    }

    #[test]
    fn rejects_non_uncertainty_source() {
        let report = check_source(
            "bad.eng",
            "Q_coil = 5 kW\nQ_total_unc = ensemble(Q_coil, samples=16)\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-UNC-SOURCE-002"));
    }

    #[test]
    fn rejects_invalid_uncertainty_arguments() {
        let report = check_source(
            "bad.eng",
            "T_bad = measured(sensor_value, std=abc)\nQ_bad_dist = normal(mean=5 kW, std=-0.8 kW, samples=0)\nQ_bad_uniform = uniform(0.7 kW, 0.3 kW, samples=abc)\nQ_source = normal(mean=4 kW, std=0.4 kW, samples=9)\nQ_bad_prop = propagate(Q_source, method=quadratic, scale=abc)\nQ_bad_distribution = distribution(kind=triangular, mean=5 kW, std=0.2 kW)\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        let codes = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>();
        assert!(codes.contains(&"E-UNC-ARGS-001"));
        assert!(codes.contains(&"E-UNC-ARGS-002"));
        assert!(codes.contains(&"E-UNC-ARGS-003"));
    }

    #[test]
    fn records_data_driven_modeling_metadata() {
        let report = check_source(
            "ok.eng",
            "cp = 4180 J/kg/K\nQ_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\nsplit = train_test_split(Q_coil, target=Q_coil, features=[T_supply, T_return, m_dot], test=0.5, seed=7)\nreg_model = regression(split, algorithm=linear)\nmlp_model = mlp(split, hidden=[4], epochs=20, seed=7)\nreg_eval = evaluate(reg_model, split=split)\nreg_card = model_card(reg_model)\nleakage = leakage_lint(split)\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert_eq!(report.semantic_program.ml_infos.len(), 6);
        assert_eq!(report.semantic_program.ml_infos[0].kind, "TrainTestSplit");
        assert_eq!(
            report.semantic_program.ml_infos[0].features,
            vec![
                "T_supply".to_owned(),
                "T_return".to_owned(),
                "m_dot".to_owned()
            ]
        );
        assert_eq!(report.semantic_program.ml_infos[2].kind, "MlpModel");
        assert_eq!(report.semantic_program.ml_infos[2].hidden_layers, vec![4]);
        assert_eq!(
            report
                .semantic_program
                .typed_bindings
                .iter()
                .find(|binding| binding.name == "reg_model")
                .unwrap()
                .semantic_type
                .quantity_kind,
            "Model[Regression]"
        );

        let review = review_json(&report);
        assert!(review.contains("\"ml_info\""));
        assert!(review.contains("\"Model[MLP]\""));
        assert!(review.contains("\"LeakageLint\""));
    }

    #[test]
    fn rejects_unresolved_ml_source() {
        let report = check_source(
            "bad.eng",
            "split = train_test_split(Q_missing, target=Q_missing, features=[T_supply], test=0.25)\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-ML-SOURCE-001"));
    }

    #[test]
    fn rejects_invalid_ml_source_kind() {
        let report = check_source(
            "bad.eng",
            "Q_coil = 5 kW\nsplit = train_test_split(Q_coil, target=Q_coil, features=[T_supply], test=0.25)\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-ML-SOURCE-002"));
    }

    #[test]
    fn rejects_ml_model_without_split_source() {
        let report = check_source(
            "bad.eng",
            "cp = 4180 J/kg/K\nQ_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\nreg_model = regression(Q_coil, algorithm=linear)\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-ML-SOURCE-002"));
    }

    #[test]
    fn rejects_evaluate_with_unresolved_split_reference() {
        let report = check_source(
            "bad.eng",
            "cp = 4180 J/kg/K\nQ_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\nsplit = train_test_split(Q_coil, target=Q_coil, features=[T_supply], test=0.25)\nreg_model = regression(split, algorithm=linear)\nreg_eval = evaluate(reg_model, split=missing_split)\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-ML-SOURCE-001"));
    }

    #[test]
    fn rejects_missing_ml_split_arguments() {
        let report = check_source(
            "bad.eng",
            "cp = 4180 J/kg/K\nQ_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\nsplit = train_test_split(Q_coil, features=[], test=1.5)\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-ML-ARGS-001"));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-ML-ARGS-002"));
    }

    #[test]
    fn rejects_unsupported_ml_algorithm() {
        let report = check_source(
            "bad.eng",
            "cp = 4180 J/kg/K\nQ_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\nsplit = train_test_split(Q_coil, target=Q_coil, features=[T_supply], test=0.25)\nreg_model = regression(split, algorithm=tree)\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-ML-ARGS-003"));
    }

    #[test]
    fn rejects_invalid_mlp_arguments() {
        let report = check_source(
            "bad.eng",
            "cp = 4180 J/kg/K\nQ_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\nsplit = train_test_split(Q_coil, target=Q_coil, features=[T_supply], test=0.25)\nmlp_model = mlp(split, hidden=[0], epochs=0, seed=abc)\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        let ml_arg_errors = report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "E-ML-ARGS-002")
            .count();
        assert!(ml_arg_errors >= 3);
    }

    #[test]
    fn records_unit_consistent_system_equation_and_residual() {
        let report = check_source(
            "ok.eng",
            "system RoomThermal {\n    parameter C: HeatCapacity = 500 kJ/K\n    parameter UA: Conductance = 150 W/K\n    state T: AbsoluteTemperature = 24 degC\n    input T_out: AbsoluteTemperature\n    input Q_internal: HeatRate\n    equation {\n        C * der(T) eq UA * (T_out - T) + Q_internal\n    }\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        let system = &report.semantic_program.systems[0];
        assert_eq!(system.equations[0].left_dimension, "Power");
        assert_eq!(system.equations[0].right_dimension, "Power");
        assert_eq!(system.equations[0].status, "unit_consistent");
        assert_eq!(system.residuals[0].dimension, "Power");
        assert_eq!(system.equation_ir[0].dependencies.len(), 5);
        assert_eq!(system.solver_plan.status, "metadata_only");
        assert_eq!(
            system.solver_plan.solve_order,
            vec!["RoomThermal.residual_1".to_owned()]
        );
        assert_eq!(
            system.solver_plan.jacobian_seed[0].with_respect_to,
            vec!["T".to_owned()]
        );
        assert_eq!(system.solver_plan.ode_runner.status, "deferred");
        assert_eq!(
            system.equation_ir[0].derivative_states,
            vec!["T".to_owned()]
        );
    }

    #[test]
    fn parses_timeseries_system_input_type() {
        let report = check_source(
            "ok.eng",
            "system SolarRoom {\n    input solar: TimeSeries[Time] of Irradiance [W/m2]\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        let solar = &report.semantic_program.systems[0].variables[0];
        assert_eq!(solar.quantity_kind, "TimeSeries[Time] of Irradiance");
        assert_eq!(solar.display_unit, "W/m2");
        assert_eq!(solar.canonical_unit, "W/m2");
    }

    #[test]
    fn records_state_space_vectors_and_linear_operators() {
        let report = check_source(
            "ok.eng",
            "system ThermalStateSpaceMetadata {\n    state T_zone: AbsoluteTemperature = 22 degC\n    input T_out: AbsoluteTemperature = 8 degC\n    input Q_internal: HeatRate = 500 W\n    states x = [T_zone]\n    inputs u = [T_out, Q_internal]\n    outputs y = [T_zone]\n    A: LinearOperator[StateVector -> Derivative[StateVector]] = [[-0.012 1/min]]\n    B: LinearOperator[InputVector -> Derivative[StateVector]] = [[0.012 1/min, 0.001]]\n    equation {\n        der(x) eq A * x + B * u\n    }\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert_eq!(report.semantic_program.state_space_vectors.len(), 3);
        assert_eq!(
            report.semantic_program.state_space_vectors[0].vector_type,
            "StateVector"
        );
        assert_eq!(report.semantic_program.linear_operators.len(), 2);
        assert_eq!(
            report.semantic_program.linear_operators[1].from,
            "InputVector"
        );
        assert_eq!(
            report.semantic_program.linear_operators[1].to,
            "Derivative[StateVector]"
        );
        assert_eq!(
            report.semantic_program.linear_operators[0].status,
            "shape_checked"
        );
        assert_eq!(
            report.semantic_program.state_space_vectors[0].status,
            "members_checked"
        );
        assert_eq!(
            report.semantic_program.linear_operators[1].row_members,
            vec!["T_zone".to_owned()]
        );
        assert_eq!(
            report.semantic_program.linear_operators[1].column_members,
            vec!["T_out".to_owned(), "Q_internal".to_owned()]
        );
        assert_eq!(
            report.semantic_program.linear_operators[1].row_quantity_kinds,
            vec!["Derivative[AbsoluteTemperature]".to_owned()]
        );
        assert_eq!(
            report.semantic_program.linear_operators[1].column_units,
            vec!["K".to_owned(), "W".to_owned()]
        );
        assert_eq!(
            report.semantic_program.linear_operators[1].compatibility_status,
            "coefficient_units_checked"
        );
        assert_eq!(
            report.semantic_program.linear_operators[0]
                .canonical_matrix
                .as_ref()
                .unwrap()[0][0],
            -0.0002
        );
        assert_eq!(
            report.semantic_program.linear_operators[1].canonical_entries[0].row_member,
            "T_zone"
        );
        assert_eq!(
            report.semantic_program.linear_operators[1].canonical_entries[1].column_member,
            "Q_internal"
        );
        assert_eq!(report.semantic_program.linear_operators[1].row_count, 1);
        assert_eq!(report.semantic_program.linear_operators[1].column_count, 2);
        let json = review_json(&report);
        assert!(json.contains("\"state_space_vectors\""));
        assert!(json.contains("\"linear_operators\""));
        assert!(json.contains("\"canonical_matrix\": [[-0.0002]]"));
        assert!(json.contains("\"canonical_entries\""));
        assert!(json.contains("\"column_member\": \"Q_internal\""));
        assert!(json.contains("\"row_quantity_kinds\""));
    }

    #[test]
    fn lowers_typed_state_space_blocks_to_solver_layouts() {
        let report = check_source(
            "ok.eng",
            "states ZoneState {\n    T_air: AbsoluteTemperature [degC]\n    T_wall: AbsoluteTemperature [degC]\n}\n\ninputs ZoneInput {\n    T_out: AbsoluteTemperature [degC]\n    Q_hvac: HeatRate [W]\n}\n\nsystem ZoneSS {\n    state x: StateVector[ZoneState] = [20 degC, 19 degC]\n    input u: InputVector[ZoneInput] = [8 degC, 1000 W]\n\n    operator A: LinearOperator[ZoneState -> Derivative[ZoneState]] = [[-0.01 1/min, 0.01 1/min]; [0.02 1/min, -0.02 1/min]]\n    operator B: LinearOperator[ZoneInput -> Derivative[ZoneState]] = [[0.01 1/min, 0.000001]; [0.0 1/min, 0.0]]\n\n    equation {\n        der(x) eq A * x + B * u\n    }\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let system = &report.semantic_program.systems[0];
        assert!(system
            .variables
            .iter()
            .any(|variable| variable.role == "state" && variable.name == "T_air"));
        assert!(system
            .variables
            .iter()
            .any(|variable| variable.role == "input" && variable.name == "Q_hvac"));
        assert_eq!(report.semantic_program.state_space_vectors.len(), 2);
        assert_eq!(
            report.semantic_program.state_space_vectors[0].members,
            vec!["T_air".to_owned(), "T_wall".to_owned()]
        );
        assert_eq!(report.semantic_program.linear_operators.len(), 2);
        assert_eq!(
            report.semantic_program.linear_operators[0].from,
            "StateVector"
        );
        assert_eq!(
            report.semantic_program.linear_operators[0].to,
            "Derivative[StateVector]"
        );
        assert_eq!(
            report.semantic_program.linear_operators[1].from,
            "InputVector"
        );
        assert_eq!(
            report.semantic_program.linear_operators[1].column_members,
            vec!["T_out".to_owned(), "Q_hvac".to_owned()]
        );
        assert_eq!(
            system
                .variables
                .iter()
                .find(|variable| variable.name == "T_wall")
                .unwrap()
                .initial_value
                .as_deref(),
            Some("19 degC")
        );
    }

    #[test]
    fn rejects_missing_state_derivative_equation() {
        let report = check_source(
            "bad.eng",
            "system TwoStateThermal {\n    parameter C_air: HeatCapacity = 500 kJ/K\n    state T_air: AbsoluteTemperature = 22 degC\n    state T_wall: AbsoluteTemperature = 20 degC\n    input Q_zone: HeatRate = 0 W\n    equation {\n        C_air * der(T_air) eq Q_zone\n    }\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SYS-DER-MISSING"));
    }

    #[test]
    fn rejects_duplicate_state_derivative_equation() {
        let report = check_source(
            "bad.eng",
            "system DuplicateThermal {\n    parameter C: HeatCapacity = 500 kJ/K\n    state T: AbsoluteTemperature = 22 degC\n    input Q_a: HeatRate = 0 W\n    input Q_b: HeatRate = 0 W\n    equation {\n        C * der(T) eq Q_a\n        C * der(T) eq Q_b\n    }\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SYS-DER-DUPLICATE"));
    }

    #[test]
    fn rejects_unsupported_state_quantity() {
        let report = check_source(
            "bad.eng",
            "system BadState {\n    state T_history: TimeSeries[Time] of AbsoluteTemperature [degC] = 22 degC\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SYS-STATE-UNSUPPORTED"));
    }

    #[test]
    fn accepts_state_space_vector_derivative_without_scalar_derivatives() {
        let report = check_source(
            "ok.eng",
            "system ThermalStateSpace {\n    state T_air: AbsoluteTemperature = 22 degC\n    state T_wall: AbsoluteTemperature = 20 degC\n    input T_out: AbsoluteTemperature = 8 degC\n    states x = [T_air, T_wall]\n    inputs u = [T_out]\n    A: LinearOperator[StateVector -> Derivative[StateVector]] = [[-0.01 1/min, 0.01 1/min]; [0.02 1/min, -0.02 1/min]]\n    B: LinearOperator[InputVector -> Derivative[StateVector]] = [[0.01 1/min]; [0.0 1/min]]\n    equation {\n        der(x) eq A * x + B * u\n    }\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert!(report.diagnostics.iter().all(|diagnostic| {
            diagnostic.code != "E-SYS-DER-MISSING" && diagnostic.code != "E-SYS-DER-DUPLICATE"
        }));
    }

    #[test]
    fn rejects_state_space_operator_shape_mismatch() {
        let report = check_source(
            "bad.eng",
            "system BadStateSpace {\n    state T_zone: AbsoluteTemperature = 22 degC\n    input T_out: AbsoluteTemperature = 8 degC\n    states x = [T_zone]\n    inputs u = [T_out]\n    B: LinearOperator[InputVector -> Derivative[StateVector]] = [[0.1, 0.2]]\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-STATE-SPACE-OP-SHAPE-001"));
        assert_eq!(
            report.semantic_program.linear_operators[0].status,
            "shape_mismatch"
        );
    }

    #[test]
    fn rejects_state_space_operator_missing_matrix_entry() {
        let report = check_source(
            "bad.eng",
            "system BadStateSpace {\n    state T_air: AbsoluteTemperature = 22 degC\n    state T_wall: AbsoluteTemperature = 20 degC\n    states x = [T_air, T_wall]\n    A: LinearOperator[StateVector -> Derivative[StateVector]] = [[0.1, 0.2]; [0.3]]\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-STATE-SPACE-OP-SHAPE-001"));
        assert_eq!(
            report.semantic_program.linear_operators[0].status,
            "shape_mismatch"
        );
    }

    #[test]
    fn rejects_unsupported_state_space_operator_coefficient_units() {
        let report = check_source(
            "bad.eng",
            "system BadStateSpace {\n    state T_zone: AbsoluteTemperature = 22 degC\n    states x = [T_zone]\n    A: LinearOperator[StateVector -> Derivative[StateVector]] = [[0.1 s]]\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-STATE-SPACE-OP-ENTRY-UNIT-001"));
        assert_eq!(
            report.semantic_program.linear_operators[0].compatibility_status,
            "entry_unit_unsupported"
        );
    }

    #[test]
    fn rejects_non_numeric_state_space_operator_coefficients() {
        let report = check_source(
            "bad.eng",
            "system BadStateSpace {\n    state T_zone: AbsoluteTemperature = 22 degC\n    states x = [T_zone]\n    A: LinearOperator[StateVector -> Derivative[StateVector]] = [[oops]]\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-STATE-SPACE-OP-ENTRY-VALUE-001"));
        assert_eq!(
            report.semantic_program.linear_operators[0].compatibility_status,
            "entry_value_invalid"
        );
        assert!(report.semantic_program.linear_operators[0]
            .canonical_matrix
            .is_none());
    }

    #[test]
    fn rejects_nonfinite_state_space_operator_coefficients() {
        let report = check_source(
            "bad.eng",
            "system BadStateSpace {\n    state T_zone: AbsoluteTemperature = 22 degC\n    states x = [T_zone]\n    A: LinearOperator[StateVector -> Derivative[StateVector]] = [[NaN]]\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-STATE-SPACE-OP-ENTRY-VALUE-001"));
        assert_eq!(
            report.semantic_program.linear_operators[0].compatibility_status,
            "entry_value_invalid"
        );
        assert!(report.semantic_program.linear_operators[0]
            .canonical_matrix
            .is_none());
    }

    #[test]
    fn rejects_incompatible_state_space_operator_coefficient_units() {
        let report = check_source(
            "bad.eng",
            "system BadStateSpace {\n    state T_zone: AbsoluteTemperature = 22 degC\n    input Q_internal: HeatRate = 500 W\n    states x = [T_zone]\n    inputs u = [Q_internal]\n    B: LinearOperator[InputVector -> Derivative[StateVector]] = [[0.1 1/min]]\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert_eq!(
            report.semantic_program.linear_operators[0].compatibility_status,
            "entry_unit_unsupported"
        );
    }

    #[test]
    fn rejects_state_space_vector_unknown_member() {
        let report = check_source(
            "bad.eng",
            "system BadStateSpace {\n    state T_zone: AbsoluteTemperature = 22 degC\n    states x = [MissingState]\n    A: LinearOperator[StateVector -> Derivative[StateVector]] = [[0.1]]\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-STATE-SPACE-VECTOR-MEMBER-001"));
        assert_eq!(
            report.semantic_program.state_space_vectors[0].status,
            "member_unresolved"
        );
        assert_eq!(
            report.semantic_program.linear_operators[0].compatibility_status,
            "member_unresolved"
        );
    }

    #[test]
    fn rejects_state_space_vector_member_role_mismatch() {
        let report = check_source(
            "bad.eng",
            "system BadStateSpaceRoles {\n    state T_zone: AbsoluteTemperature = 22 degC\n    input Q_internal: HeatRate = 500 W\n    output Q_total: HeatRate\n    states x = [T_zone, Q_internal]\n    inputs u = [T_zone]\n    outputs y = [Q_internal, Q_total]\n    A: LinearOperator[StateVector -> Derivative[StateVector]] = [[0.1, 0.0]; [0.0, 0.1]]\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-STATE-SPACE-VECTOR-MEMBER-ROLE"));
        assert_eq!(
            report
                .semantic_program
                .state_space_vectors
                .iter()
                .filter(|vector| vector.status == "member_role_mismatch")
                .count(),
            3
        );
        assert_eq!(
            report.semantic_program.linear_operators[0].compatibility_status,
            "member_role_mismatch"
        );
    }

    #[test]
    fn rejects_simulate_missing_required_options() {
        let report = check_source(
            "bad.eng",
            "system RoomThermal {\n    parameter C: HeatCapacity = 500 kJ/K\n    parameter UA: Conductance = 150 W/K\n    state T: AbsoluteTemperature = 24 degC\n    input T_out: AbsoluteTemperature\n    input Q_internal: HeatRate\n    equation {\n        C * der(T) eq UA * (T_out - T) + Q_internal\n    }\n}\n\nsim = simulate RoomThermal\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SIM-TIMESTEP-INVALID"));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SIM-SOLVER-UNSUPPORTED"));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SIM-MISSING-INPUT"));
    }

    #[test]
    fn review_json_exposes_simulation_request_time_grid() {
        let report = check_source(
            "ok.eng",
            "system Decay {\n    state T: AbsoluteTemperature = 24 degC\n}\n\nsim = simulate Decay\nwith {\n    timestep = 10 min\n    duration = 30 min\n    solver = fixed_step\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        let json = review_json(&report);
        assert!(json.contains("\"simulation_requests\""));
        assert!(json.contains("\"binding\": \"sim\""));
        assert!(json.contains("\"status\": \"declared_fixed_step\""));
        assert!(json.contains("\"timestep_s\": 600"));
        assert!(json.contains("\"duration_s\": 1800"));
        assert!(json.contains("\"step_count\": 3"));
    }

    #[test]
    fn rejects_simulate_unknown_system() {
        let report = check_source(
            "bad.eng",
            "sim = simulate MissingSystem\nwith {\n    timestep = 10 min\n    solver = fixed_step\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SIM-SYSTEM-001"));
    }

    #[test]
    fn accepts_one_state_adaptive_heun_solver_option() {
        let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/internal/27_adaptive_heun_thermal/main.eng");
        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(!report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .all(|diagnostic| { diagnostic.code != "E-SIM-SOLVER-UNSUPPORTED" }));
    }

    #[test]
    fn accepts_continuous_state_space_adaptive_heun_solver_option() {
        let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/internal/28_adaptive_state_space/main.eng");
        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(!report.has_errors());
        assert!(report.diagnostics.iter().all(|diagnostic| {
            diagnostic.code != "E-SIM-SOLVER-UNSUPPORTED"
                && diagnostic.code != "E-SIM-SYSTEM-SHAPE-UNSUPPORTED"
        }));
    }

    #[test]
    fn rejects_adaptive_heun_for_discrete_state_space_shape() {
        let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/diagnostics/error_messages/simulate_adaptive_discrete_state_space.eng");
        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SIM-SYSTEM-SHAPE-UNSUPPORTED"));
    }

    #[test]
    fn accepts_declared_simulation_input_option_names() {
        let report = check_source(
            "source_input.eng",
            "drive_signal: TimeSeries[Time] of DimensionlessNumber [1] = 0.5\n\nsystem DrivenSource {\n    input drive: TimeSeries[Time] of DimensionlessNumber [1]\n    state x: DimensionlessNumber = 0.1\n    equation {\n        der(x) eq (drive - x) / 1 s\n    }\n}\n\nsim = simulate DrivenSource\nwith {\n    drive = drive_signal\n    timestep = 1 s\n    solver = adaptive_heun\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert!(report.diagnostics.iter().all(|diagnostic| {
            diagnostic.code != "E-WITH-OPTION-001"
                && diagnostic.code != "E-SIM-MISSING-INPUT"
                && diagnostic.code != "E-SIM-INPUT-QTY-MISMATCH"
        }));
    }

    #[test]
    fn accepts_declared_simulation_parameter_option_names() {
        let report = check_source(
            "sim_parameter.eng",
            "system ParamOde {\n    parameter gain: DimensionlessNumber [1] = 1\n    state x: DimensionlessNumber = 0\n    equation {\n        der(x) eq gain / 1 s\n    }\n}\n\nsim = simulate ParamOde\nwith {\n    gain = 2\n    timestep = 1 s\n    solver = rk4\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert!(report
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "E-WITH-OPTION-001"));
    }

    #[test]
    fn rejects_simulation_parameter_option_unit_mismatch() {
        let report = check_source(
            "bad_sim_parameter.eng",
            "system ParamOde {\n    parameter gain: DimensionlessNumber [1] = 1\n    state x: DimensionlessNumber = 0\n    equation {\n        der(x) eq gain / 1 s\n    }\n}\n\nsim = simulate ParamOde\nwith {\n    gain = 2 kW\n    timestep = 1 s\n    solver = rk4\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SIM-PARAMETER-QTY-MISMATCH"));
    }
    #[test]
    fn rejects_invalid_simulate_duration_and_tolerance() {
        let report = check_source(
            "bad.eng",
            "T_out_signal: TimeSeries[Time] of AbsoluteTemperature [degC] = 8 degC\n\nsystem RoomThermal {\n    parameter C: HeatCapacity = 500 kJ/K\n    parameter UA: Conductance = 150 W/K\n    state T: AbsoluteTemperature = 24 degC\n    input T_out: TimeSeries[Time] of AbsoluteTemperature [degC]\n    input Q_internal: HeatRate = 500 W\n    equation {\n        C * der(T) eq UA * (T_out - T) + Q_internal\n    }\n}\n\nsim = simulate RoomThermal\nwith {\n    T_out = T_out_signal\n    timestep = 10 min\n    duration = forever\n    solver = adaptive_heun\n    tolerance = -0.1\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SIM-DURATION-INVALID"));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SIM-TOLERANCE-INVALID"));
    }

    #[test]
    fn accepts_source_ode_adaptive_heun_for_non_thermal_shape() {
        let report = check_source(
            "source_adaptive.eng",
            "system Cooling {\n    state T: AbsoluteTemperature = 300 K\n    equation {\n        der(T) eq 0 K/s\n    }\n}\n\nsim = simulate Cooling\nwith {\n    timestep = 1 s\n    solver = adaptive_heun\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert!(report
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "E-SIM-SYSTEM-SHAPE-UNSUPPORTED"));
    }

    #[test]
    fn rejects_simulate_unsupported_solver() {
        let report = check_source(
            "bad.eng",
            "system Decay {\n    parameter C: HeatCapacity = 500 kJ/K\n    state T: AbsoluteTemperature = 24 degC\n    equation {\n        C * der(T) eq 0 W\n    }\n}\n\nsim = simulate Decay\nwith {\n    timestep = 10 min\n    solver = adaptive\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SIM-SOLVER-UNSUPPORTED"));
    }

    #[test]
    fn rejects_simulate_wrong_input_quantity() {
        let report = check_source(
            "bad.eng",
            "Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\n\nsystem RoomThermal {\n    parameter C: HeatCapacity = 500 kJ/K\n    parameter UA: Conductance = 150 W/K\n    state T: AbsoluteTemperature = 24 degC\n    input T_out: AbsoluteTemperature\n    input Q_internal: HeatRate\n    equation {\n        C * der(T) eq UA * (T_out - T) + Q_internal\n    }\n}\n\nsim = simulate RoomThermal\nwith {\n    T_out = Q_coil\n    timestep = 10 min\n    solver = fixed_step\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SIM-INPUT-QTY-MISMATCH"));
    }

    #[test]
    fn accepts_declared_scalar_simulation_input_option_values() {
        let report = check_source(
            "scalar_input.eng",
            "system DrivenScalar {\n    input drive: DimensionlessNumber [1] = 1\n    state x: DimensionlessNumber = 0\n    equation {\n        der(x) eq drive / 1 s\n    }\n}\n\nsim = simulate DrivenScalar\nwith {\n    drive = 2\n    timestep = 1 s\n    duration = 2 s\n    solver = rk4\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert!(report.diagnostics.iter().all(|diagnostic| {
            diagnostic.code != "E-SIM-INPUT-VALUE" && diagnostic.code != "E-SIM-INPUT-QTY-MISMATCH"
        }));
    }

    #[test]
    fn rejects_declared_scalar_simulation_input_option_unit_mismatch() {
        let report = check_source(
            "scalar_input_bad.eng",
            "system DrivenScalar {\n    input drive: DimensionlessNumber [1] = 1\n    state x: DimensionlessNumber = 0\n    equation {\n        der(x) eq drive / 1 s\n    }\n}\n\nsim = simulate DrivenScalar\nwith {\n    drive = 2 kW\n    timestep = 1 s\n    duration = 2 s\n    solver = rk4\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SIM-INPUT-QTY-MISMATCH"));
    }
    #[test]
    fn rejects_simulate_wrong_axis_and_timestep_unit() {
        let report = check_source(
            "bad.eng",
            "bad_weather: TimeSeries[Sample] of AbsoluteTemperature [degC] = 8 degC\n\nsystem RoomThermal {\n    parameter C: HeatCapacity = 500 kJ/K\n    parameter UA: Conductance = 150 W/K\n    state T: AbsoluteTemperature = 24 degC\n    input T_out: AbsoluteTemperature\n    input Q_internal: HeatRate\n    equation {\n        C * der(T) eq UA * (T_out - T) + Q_internal\n    }\n}\n\nsim = simulate RoomThermal\nwith {\n    T_out = bad_weather\n    timestep = 10 samples\n    solver = fixed_step\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SIM-INPUT-AXIS-MISMATCH"));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SIM-TIMESTEP-INVALID"));
    }

    #[test]
    fn rejects_boolean_equality_in_equation_block() {
        let report = check_source(
            "bad.eng",
            "system RoomThermal {\n    parameter C: HeatCapacity = 500 kJ/K\n    state T: AbsoluteTemperature = 24 degC\n    input T_out: AbsoluteTemperature\n    equation {\n        C * der(T) == T_out\n    }\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert_eq!(report.diagnostics[0].code, "E-EQ-BOOL-001");
    }

    #[test]
    fn rejects_unit_mismatched_system_equation() {
        let report = check_source(
            "bad.eng",
            "system RoomThermal {\n    parameter C: HeatCapacity = 500 kJ/K\n    state T: AbsoluteTemperature = 24 degC\n    input T_out: AbsoluteTemperature\n    equation {\n        C * der(T) eq T_out\n    }\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-EQ-UNIT-001"));
    }

    #[test]
    fn warns_when_summing_heat_rate_over_time() {
        let report = check_source(
            "warn.eng",
            "sensor = promote csv \"data/sensor.csv\" as SensorData\n    cp = 4180 J/kg/K\n    Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\n    E_bad = sum(Q_coil, axis=Time)\n}\n",
            &CheckOptions::default(),
        );

        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "W-STATS-SUM-001"));
    }

    #[test]
    fn rejects_short_declaration_operator() {
        let report = check_source("bad.eng", "Q := 10 kW", &CheckOptions::default());

        assert!(report.has_errors());
        assert_eq!(report.diagnostics[0].code, "E-SYNTAX-DECL-001");
    }

    #[test]
    fn rejects_dimensionless_addition_to_length() {
        let report = check_source("bad.eng", "X = 1 m + 20", &CheckOptions::default());

        assert!(report.has_errors());
        assert_eq!(report.diagnostics[0].code, "E-DIM-ADD-001");
    }

    #[test]
    fn rejects_dimensionless_subtraction_from_power() {
        let report = check_source("bad.eng", "Q = 2 kW - 1", &CheckOptions::default());

        assert!(report.has_errors());
        assert_eq!(report.diagnostics[0].code, "E-DIM-ADD-002");
    }

    #[test]
    fn infers_dimensionless_number_for_plain_numeric_binding() {
        let report = check_source("ok.eng", "x =3", &CheckOptions::default());

        assert!(!report.has_errors());
        let binding = report
            .semantic_program
            .typed_bindings
            .iter()
            .find(|binding| binding.name == "x")
            .expect("x binding");
        assert_eq!(binding.semantic_type.quantity_kind, "DimensionlessNumber");
        assert_eq!(binding.semantic_type.display_unit, "1");
    }

    #[test]
    fn records_expected_type_for_explicit_declaration() {
        let report = check_source(
            "ok.eng",
            "Q: HeatRate [kW] = 1 kW + 2 kW",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert_eq!(
            report.semantic_program.expected_types[0].quantity_kind,
            "HeatRate"
        );
    }

    #[test]
    fn imports_function_definitions_without_importing_executable_body() {
        let root = std::env::temp_dir().join("englang-function-import-test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp dir");
        fs::write(
            root.join("thermal.eng"),
            "const UA_wall_default: Conductance [W/K] = 150 W/K\n\nfn heat_loss(UA: Conductance [W/K], dT: TemperatureDelta [K]) -> HeatRate [W] {\n    UA_local = UA\n    dT_local = dT\n    return UA_local * dT_local\n}\n\nQ_unused = 1 kW\n",
        )
        .expect("thermal source");
        let main_path = root.join("main.eng");
        fs::write(
            &main_path,
            "use \"thermal.eng\"\n\nUA_wall = UA_wall_default\ndT_wall = 8 K\nQ_wall = heat_loss(UA_wall, dT_wall)\nprint \"Q wall = {Q_wall: .2 kW}\"\n",
        )
        .expect("main source");

        let report = check_file(&main_path, &CheckOptions::default()).expect("check file");

        assert!(!report.has_errors());
        assert_eq!(report.semantic_program.imports.len(), 1);
        assert_eq!(report.semantic_program.consts.len(), 1);
        assert_eq!(report.semantic_program.functions.len(), 1);
        assert_eq!(report.semantic_program.functions[0].locals.len(), 2);
        let q_wall = report
            .semantic_program
            .typed_bindings
            .iter()
            .find(|binding| binding.name == "Q_wall")
            .expect("Q_wall binding");
        assert_eq!(q_wall.semantic_type.quantity_kind, "HeatRate");
        assert_eq!(
            report.semantic_program.functions[0].status,
            "unit_consistent"
        );
        let review = review_json(&report);
        assert!(review.contains("\"function_summary\""));
        assert!(review.contains("\"heat_loss\""));
    }

    #[test]
    fn imported_module_args_are_not_imported_and_top_level_bindings_are_not_importable() {
        let root = std::env::temp_dir().join("englang-import-policy-test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp dir");
        fs::write(
            root.join("defaults.eng"),
            "args {\n    input: CsvFile = file(\"module.csv\")\n}\n\ncp_water = 4180 J/kg/K\n",
        )
        .expect("defaults source");
        let main_path = root.join("main.eng");
        fs::write(&main_path, "use \"defaults.eng\"\n\ncp = cp_water\n").expect("main source");

        let report = check_file(&main_path, &CheckOptions::default()).expect("check file");

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "W-MODULE-ARGS-NOT-IMPORTED-001"));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-IMPORT-SYMBOL-001"));
        assert!(report.semantic_program.args_blocks.is_empty());
    }

    #[test]
    fn rejects_dynamic_import_paths_from_args_expressions() {
        let report = check_source(
            "bad.eng",
            "args {\n    input: CsvFile = file(\"defaults.eng\")\n    dir: DirectoryPath = dir(\".\")\n}\n\nuse args.input\nuse join(args.dir, \"defaults.eng\")\nuse \"cases/{args.case}.eng\"\n\nQ = 1 kW\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert_eq!(
            report
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == "E-IMPORT-DYNAMIC-001")
                .count(),
            3
        );
    }

    #[test]
    fn rejects_args_and_const_side_effect_policy_violations() {
        let report = check_source(
            "bad.eng",
            "args {\n    input: CsvFile = download(\"https://example.com/data.csv\")\n}\n\nconst selected_input: CsvFile = args.input\nconst generated: CsvFile = download(\"https://example.com/data.csv\")\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-ARGS-SIDE-EFFECT-001"));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-CONST-ARGS-001"));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-CONST-SIDE-EFFECT-001"));
    }

    #[test]
    fn rejects_function_call_dimension_mismatch() {
        let report = check_source(
            "bad.eng",
            "fn heat_loss(UA: Conductance [W/K], dT: TemperatureDelta [K]) -> HeatRate [W] {\n    return UA * dT\n}\n\nL_wall = 2 m\n    dT_wall = 8 K\n    Q_wall = heat_loss(L_wall, dT_wall)\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-FN-CALL-004"));
    }

    #[test]
    fn rejects_side_effects_inside_functions() {
        let report = check_source(
            "bad.eng",
            "fn noisy(Q: HeatRate [kW], notes: TextFile) -> HeatRate [W] {\n    print \"Q={Q: .1 kW}\"\n    text = read text notes\n    return Q\n}\n\nQ = 1 kW\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert_eq!(
            report
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == "E-FN-SIDE-EFFECT-001")
                .count(),
            2
        );
        assert!(report.semantic_program.prints.is_empty());
        assert_eq!(
            report.semantic_program.functions[0].status,
            "side_effect_rejected"
        );
    }

    #[test]
    fn records_hover_hint_for_inferred_declaration() {
        let report = check_source("ok.eng", "L = 1 m + 20 cm", &CheckOptions::default());

        assert_eq!(report.semantic_program.hover_hints[0].name, "L");
        assert_eq!(
            report.semantic_program.hover_hints[0].quick_fixes[0],
            "Expand declaration"
        );
    }

    #[test]
    fn records_type_info_and_unit_derivation() {
        let report = check_source("ok.eng", "L = 1 m + 20 cm", &CheckOptions::default());

        assert_eq!(
            report.semantic_program.type_infos[0].quantity_kind,
            "Length"
        );
        assert_eq!(
            report.semantic_program.unit_derivations[0].canonical_unit,
            "m"
        );
        assert!(report.unit_info_count > 0);
    }

    #[test]
    fn records_pressure_quantity_and_unit_derivation() {
        let report = check_source("ok.eng", "p_supply = 220 kPa", &CheckOptions::default());

        assert!(!report.has_errors());
        assert_eq!(report.inferred_declarations[0].quantity_kind, "Pressure");
        assert_eq!(report.inferred_declarations[0].display_unit, "Pa");
        assert_eq!(
            report.semantic_program.unit_derivations[0]
                .source_unit
                .as_deref(),
            Some("kPa")
        );
        assert_eq!(
            report.semantic_program.unit_derivations[0].canonical_unit,
            "Pa"
        );
    }

    #[test]
    fn records_people_density_quantity_and_unit_derivation() {
        let report = check_source(
            "ok.eng",
            "occupant_density = 0.08 person/m2",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert_eq!(
            report.inferred_declarations[0].quantity_kind,
            "PeopleDensity"
        );
        assert_eq!(report.inferred_declarations[0].display_unit, "person/m2");
        assert_eq!(
            report.semantic_program.unit_derivations[0]
                .source_unit
                .as_deref(),
            Some("person/m2")
        );
        assert_eq!(
            report.semantic_program.unit_derivations[0].canonical_unit,
            "person/m2"
        );
    }

    #[test]
    fn records_duration_quantity_and_unit_derivation() {
        let report = check_source("ok.eng", "unmet = 2 h", &CheckOptions::default());

        assert!(!report.has_errors());
        assert_eq!(report.inferred_declarations[0].quantity_kind, "Duration");
        assert_eq!(report.inferred_declarations[0].display_unit, "s");
        assert_eq!(
            report.semantic_program.unit_derivations[0]
                .source_unit
                .as_deref(),
            Some("h")
        );
        assert_eq!(
            report.semantic_program.unit_derivations[0].canonical_unit,
            "s"
        );
    }

    #[test]
    fn accepts_celsius_symbol_alias_for_absolute_temperature() {
        let report = check_source(
            "ok.eng",
            "schema SensorData {\n    T_supply: AbsoluteTemperature [°C]\n}\n\nT_room = 24 °C\n}\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert_eq!(
            report.semantic_program.schemas[0].columns[0]
                .unit
                .as_deref(),
            Some("°C")
        );
        assert_eq!(
            report.inferred_declarations[0].quantity_kind,
            "AbsoluteTemperature"
        );
        assert_eq!(report.inferred_declarations[0].display_unit, "K");
        let room_derivation = report
            .semantic_program
            .unit_derivations
            .iter()
            .find(|derivation| derivation.name == "T_room")
            .expect("T_room derivation");
        assert_eq!(room_derivation.source_unit.as_deref(), Some("°C"));
    }

    #[test]
    fn review_json_exposes_v07_review_contract_sections() {
        let report = check_source(
            "ok.eng",
            "schema SensorData {\n    time: DateTime index\n    T_supply: AbsoluteTemperature [degC]\n}\n\npower = 10 kW\n    L = 1 m + 20 cm\n}\n",
            &CheckOptions::default(),
        );

        let json = review_json(&report);

        assert!(json.contains("\"review_schema_version\": 1"));
        assert!(json.contains("\"variable_table\""));
        assert!(json.contains("\"unit_conversion_table\""));
        assert!(json.contains("\"schema_summary\""));
        assert!(json.contains("\"plot_manifest\""));
        assert!(json.contains("\"warning_list\""));
        assert!(json.contains("\"W-QTY-AMBIG-001\""));
    }

    #[test]
    fn review_json_exposes_normalized_review_document() {
        let report = check_source(
            "ok.eng",
            "x = 1 m\nvalidate x > 0 m\nprocess_result = run command \"python\"\nwith {\n    args = [\"--version\"]\n    expected_outputs = [\"outputs/tool.txt\"]\n    allow_failure = true\n}\n",
            &CheckOptions::default(),
        );

        let json = review_json(&report);

        assert!(json.contains("\"review_document\""));
        assert!(json.contains("\"format\": \"eng-review-document-preview-1\""));
        assert!(json.contains("\"semantic_hash\""));
        assert!(json.contains("\"section_hashes\""));
        assert!(json.contains("\"root_contract\""));
        assert!(json.contains("\"workflow_module_count\""));
        assert!(json.contains("\"workflow_modules\""));
        assert!(json.contains("\"unit_quantity_count\""));
        assert!(json.contains("\"time_axis_count\""));
        assert!(json.contains("\"report_output_count\""));
        assert!(json.contains("\"schemas\""));
        assert!(json.contains("\"units_quantities\""));
        assert!(json.contains("\"time_axes\""));
        assert!(json.contains("\"derived_values\""));
        assert!(json.contains("\"calculations\""));
        assert!(json.contains("\"input_symbols\""));
        assert!(json.contains("\"output_quantity\""));
        assert!(json.contains("\"unit_derivation\""));
        assert!(json.contains("\"report_outputs\""));
        assert!(json.contains("\"validations\""));
        assert!(json.contains("\"external_boundaries\""));
        assert!(json.contains("\"fallbacks\""));
        assert!(json.contains("\"risks\""));
        assert!(json.contains("\"kind\": \"process\""));
        assert!(json.contains("\"risk_level\": \"high\""));
        assert!(json.contains("\"kind\": \"allowed_failure\""));
        assert!(json.contains("\"fallback_source\": \"external_operation\""));
        assert!(json.contains("\"category\": \"external_boundary\""));
        assert!(json.contains("\"level\": \"high\""));
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("normalized review document json");
        let fallback = value
            .pointer("/review_document/fallbacks/0")
            .expect("shared fallback record");
        assert_eq!(
            fallback.get("target").and_then(serde_json::Value::as_str),
            Some("owner_line:3")
        );
        assert_eq!(
            fallback.get("method").and_then(serde_json::Value::as_str),
            Some("allow_failure")
        );
        assert_eq!(
            fallback
                .get("affected_scope")
                .and_then(serde_json::Value::as_str),
            Some("external boundary status")
        );
        assert_eq!(
            value
                .pointer("/review_document/root_contract/fallback_count")
                .and_then(serde_json::Value::as_u64),
            Some(1)
        );
        assert_eq!(
            value
                .pointer("/review_document/workflow_modules/0/kind")
                .and_then(serde_json::Value::as_str),
            Some("native_module")
        );
        assert!(value
            .pointer("/review_document/workflow_modules")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|modules| modules.iter().any(|module| {
                module.get("name").and_then(serde_json::Value::as_str) == Some("eng.review")
            })));
        assert!(value
            .pointer("/review_document/section_hashes/workflow_modules")
            .and_then(serde_json::Value::as_str)
            .is_some());
    }

    #[test]
    fn review_json_exposes_v08_system_sections() {
        let report = check_source(
            "ok.eng",
            "system RoomThermal {\n    parameter C: HeatCapacity = 500 kJ/K\n    parameter UA: Conductance = 150 W/K\n    state T: AbsoluteTemperature = 24 degC\n    input T_out: AbsoluteTemperature\n    input Q_internal: HeatRate\n    equation {\n        C * der(T) eq UA * (T_out - T) + Q_internal\n    }\n}\n",
            &CheckOptions::default(),
        );

        let json = review_json(&report);

        assert!(json.contains("\"systems\": 1"));
        assert!(json.contains("\"equations\": 1"));
        assert!(json.contains("\"system_summary\""));
        assert!(json.contains("\"system_ir\""));
        assert!(json.contains("\"solver_boundary\""));
        assert!(json.contains("\"solver_plan\""));
        assert!(json.contains("\"solve_order\": [\"RoomThermal.residual_1\"]"));
        assert!(json.contains("\"jacobian_seed\""));
        assert!(json.contains("\"ode_runner\""));
        assert!(json.contains("\"status\": \"unsolved\""));
        assert!(json.contains("\"derivative_states\": [\"T\"]"));
        assert!(json.contains("\"RoomThermal\""));
        assert!(json.contains("\"unit_consistent\""));
        assert!(json.contains("\"RoomThermal.residual_1\""));
    }

    #[test]
    fn refines_ambiguous_power_warning() {
        let report = check_source("warn.eng", "power = 10 kW", &CheckOptions::default());

        assert_eq!(report.diagnostics[0].code, "W-QTY-AMBIG-001");
        assert!(report.diagnostics[0]
            .help
            .as_ref()
            .is_some_and(|help| help.contains("HeatRate")));
    }

    #[test]
    fn rejects_schema_fast_assignment() {
        let report = check_source(
            "bad.eng",
            "schema SensorData {\n    T_supply = 24 degC\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert_eq!(report.diagnostics[0].code, "E-PUBLIC-ANNOTATION-001");
    }

    #[test]
    fn records_schema_symbol_table() {
        let report = check_source(
            "ok.eng",
            "schema SensorData {\n    time: DateTime index\n    T_supply: AbsoluteTemperature [degC]\n    missing {\n        T_supply: interpolate max_gap=10 min\n    }\n}\n",
            &CheckOptions::default(),
        );

        assert_eq!(report.semantic_program.schemas[0].name, "SensorData");
        assert_eq!(report.semantic_program.schemas[0].columns.len(), 2);
        assert!(report.semantic_program.schemas[0].columns[0].is_index);
        assert_eq!(
            report.semantic_program.schemas[0].missing_policies[0].column,
            "T_supply"
        );
        assert!(!report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "W-SCHEMA-POLICY-001"));
        assert_eq!(
            report
                .semantic_program
                .type_infos
                .iter()
                .filter(|info| info.name == "T_supply")
                .count(),
            1
        );
    }

    #[test]
    fn records_table_filter_require_one_transforms() {
        let root =
            env::temp_dir().join(format!("englang-table-transform-ok-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(
            root.join("data").join("station_map.csv"),
            concat!(
                "region,station_id,valid_from,valid_to,latitude,longitude\n",
                "demo,STN001,2020-01-01T00:00:00+09:00,,37.5665,126.9780\n",
                "other,STN002,2020-01-01T00:00:00+09:00,,35.1796,129.0756\n",
            ),
        )
        .expect("station map csv");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema StationMap {\n",
                "    region: String\n",
                "    station_id: String\n",
                "    valid_from: DateTime\n",
                "    valid_to: DateTime\n",
                "    latitude: DimensionlessNumber [1]\n",
                "    longitude: DimensionlessNumber [1]\n",
                "}\n\n",
                "args {\n",
                "    year: Int = 2024\n",
                "    region: String = \"demo\"\n",
                "    station_map: CsvFile = file(\"data/station_map.csv\")\n",
                "}\n\n",
                "stations = promote csv args.station_map as StationMap\n",
                "candidates = filter stations\n",
                "where {\n",
                "    region == args.region\n",
                "    valid_from <= date(args.year, 1, 1)\n",
                "    valid_to is none or valid_to >= date(args.year, 12, 31)\n",
                "}\n",
                "station = require_one candidates\n",
                "with {\n",
                "    on_none = error \"No station for region/year\"\n",
                "    on_many = error \"Multiple stations for region/year\"\n",
                "}\n",
            ),
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert_eq!(report.semantic_program.table_transforms.len(), 2);
        let filter = &report.semantic_program.table_transforms[0];
        assert_eq!(filter.binding, "candidates");
        assert_eq!(filter.operation, "filter");
        assert_eq!(filter.schema_name.as_deref(), Some("StationMap"));
        assert_eq!(filter.predicates.len(), 3);
        assert!(filter
            .predicates
            .iter()
            .any(|predicate| predicate.operator == "or"));
        let require_one = &report.semantic_program.table_transforms[1];
        assert_eq!(require_one.binding, "station");
        assert_eq!(require_one.operation, "require_one");
        assert_eq!(require_one.source_table, "candidates");
        assert_eq!(require_one.schema_name.as_deref(), Some("StationMap"));

        let review = review_json(&report);
        assert!(review.contains("\"table_transforms\""));
        assert!(review.contains("\"table_transform_count\": 2"));
        assert!(review.contains("\"operation\": \"filter\""));
        assert!(review.contains("\"operation\": \"require_one\""));
    }

    #[test]
    fn diagnoses_unknown_table_filter_column() {
        let root = env::temp_dir().join(format!(
            "englang-table-transform-bad-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(
            root.join("data").join("station_map.csv"),
            "region,station_id\ndemo,STN001\n",
        )
        .expect("station map csv");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema StationMap {\n",
                "    region: String\n",
                "    station_id: String\n",
                "}\n\n",
                "args {\n",
                "    region: String = \"demo\"\n",
                "    station_map: CsvFile = file(\"data/station_map.csv\")\n",
                "}\n\n",
                "stations = promote csv args.station_map as StationMap\n",
                "candidates = filter stations\n",
                "where {\n",
                "    missing_column == args.region\n",
                "}\n",
            ),
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-TABLE-UNKNOWN-COLUMN"));
    }

    #[test]
    fn diagnoses_table_datetime_predicate_type_mismatch() {
        let root = env::temp_dir().join(format!(
            "englang-table-datetime-predicate-type-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(
            root.join("data").join("events.csv"),
            "timestamp,name\n2024-01-01T00:00:00Z,start\n",
        )
        .expect("events csv");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema EventLog {\n",
                "    timestamp: DateTime\n",
                "    name: String\n",
                "}\n\n",
                "args {\n",
                "    events_path: CsvFile = file(\"data/events.csv\")\n",
                "}\n\n",
                "events = promote csv args.events_path as EventLog\n",
                "bad_time = filter events\n",
                "where {\n",
                "    timestamp >= 42\n",
                "    name <= date(2024, 1, 1)\n",
                "}\n",
            ),
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(report.has_errors());
        assert_eq!(
            report
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == "E-TABLE-PREDICATE-TYPE")
                .count(),
            2
        );
        let transform = report
            .semantic_program
            .table_transforms
            .iter()
            .find(|transform| transform.binding == "bad_time")
            .expect("filter transform");
        assert_eq!(transform.predicates.len(), 2);
        assert!(transform
            .predicates
            .iter()
            .all(|predicate| predicate.status == "type_mismatch"));
    }

    #[test]
    fn records_table_select_columns_transform() {
        let root = env::temp_dir().join(format!("englang-table-select-ok-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(
            root.join("data").join("station_map.csv"),
            "region,station_id,latitude\ndemo,STN001,37.5\n",
        )
        .expect("station map csv");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema StationMap {\n",
                "    region: String\n",
                "    station_id: String\n",
                "    latitude: DimensionlessNumber [1]\n",
                "}\n\n",
                "args {\n",
                "    station_map: CsvFile = file(\"data/station_map.csv\")\n",
                "}\n\n",
                "stations = promote csv args.station_map as StationMap\n",
                "station_fields = select stations columns station_id, latitude\n",
            ),
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let transform = report
            .semantic_program
            .table_transforms
            .iter()
            .find(|transform| transform.binding == "station_fields")
            .expect("select transform");
        assert_eq!(transform.operation, "select");
        assert_eq!(transform.source_table, "stations");
        assert_eq!(transform.selected_columns.len(), 2);
        assert_eq!(transform.selected_columns[0].name, "station_id");
        assert_eq!(transform.selected_columns[1].name, "latitude");

        let review = review_json(&report);
        assert!(review.contains("\"operation\": \"select\""));
        assert!(review.contains("\"selected_column_count\": 2"));
        assert!(review.contains("\"name\": \"station_id\""));
    }

    #[test]
    fn diagnoses_unknown_table_select_column() {
        let root = env::temp_dir().join(format!("englang-table-select-bad-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(root.join("data").join("station_map.csv"), "region\ndemo\n")
            .expect("station map csv");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema StationMap {\n",
                "    region: String\n",
                "}\n\n",
                "args {\n",
                "    station_map: CsvFile = file(\"data/station_map.csv\")\n",
                "}\n\n",
                "stations = promote csv args.station_map as StationMap\n",
                "station_fields = select stations columns station_id\n",
            ),
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-TABLE-UNKNOWN-COLUMN"));
    }

    #[test]
    fn records_table_sort_transform_keys() {
        let root = env::temp_dir().join(format!("englang-table-sort-ok-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(
            root.join("data").join("station_map.csv"),
            "station_id,latitude\nSTN002,35.1\nSTN001,37.5\n",
        )
        .expect("station map csv");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema StationMap {\n",
                "    station_id: String\n",
                "    latitude: DimensionlessNumber [1]\n",
                "}\n\n",
                "args {\n",
                "    station_map: CsvFile = file(\"data/station_map.csv\")\n",
                "}\n\n",
                "stations = promote csv args.station_map as StationMap\n",
                "ordered = sort stations by station_id desc\n",
            ),
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let transform = report
            .semantic_program
            .table_transforms
            .iter()
            .find(|transform| transform.binding == "ordered")
            .expect("sort transform");
        assert_eq!(transform.operation, "sort");
        assert_eq!(transform.source_table, "stations");
        assert_eq!(transform.sort_keys.len(), 1);
        assert_eq!(transform.sort_keys[0].column, "station_id");
        assert_eq!(transform.sort_keys[0].direction, "desc");

        let review = review_json(&report);
        assert!(review.contains("\"operation\": \"sort\""));
        assert!(review.contains("\"sort_keys\""));
        assert!(review.contains("\"direction\": \"desc\""));
    }

    #[test]
    fn diagnoses_unknown_table_sort_column() {
        let root = env::temp_dir().join(format!("englang-table-sort-bad-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(
            root.join("data").join("station_map.csv"),
            "station_id\nSTN001\n",
        )
        .expect("station map csv");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema StationMap {\n",
                "    station_id: String\n",
                "}\n\n",
                "args {\n",
                "    station_map: CsvFile = file(\"data/station_map.csv\")\n",
                "}\n\n",
                "stations = promote csv args.station_map as StationMap\n",
                "ordered = sort stations by missing_column\n",
            ),
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-TABLE-UNKNOWN-COLUMN"));
    }

    #[test]
    fn records_table_derive_columns_transform() {
        let root = env::temp_dir().join(format!("englang-table-derive-ok-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(
            root.join("data").join("station_map.csv"),
            "station_id,longitude\nSTN001,126.9\n",
        )
        .expect("station map csv");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema StationMap {\n",
                "    station_id: String\n",
                "    longitude: DimensionlessNumber [1]\n",
                "}\n\n",
                "args {\n",
                "    station_map: CsvFile = file(\"data/station_map.csv\")\n",
                "}\n\n",
                "stations = promote csv args.station_map as StationMap\n",
                "station_plus = derive stations column longitude_copy = longitude\n",
                "station_fields = select station_plus columns station_id, longitude_copy\n",
            ),
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let transform = report
            .semantic_program
            .table_transforms
            .iter()
            .find(|transform| transform.binding == "station_plus")
            .expect("derive transform");
        assert_eq!(transform.operation, "derive");
        assert_eq!(transform.source_table, "stations");
        assert_eq!(transform.derived_columns.len(), 1);
        assert_eq!(transform.derived_columns[0].name, "longitude_copy");
        assert_eq!(transform.derived_columns[0].expression, "longitude");
        assert_eq!(
            transform.derived_columns[0].source_columns,
            vec!["longitude".to_owned()]
        );

        let review = review_json(&report);
        assert!(review.contains("\"operation\": \"derive\""));
        assert!(review.contains("\"derived_columns\""));
        assert!(review.contains("\"name\": \"longitude_copy\""));
    }

    #[test]
    fn diagnoses_unknown_table_derive_source_column() {
        let root = env::temp_dir().join(format!("englang-table-derive-bad-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(
            root.join("data").join("station_map.csv"),
            "station_id\nSTN001\n",
        )
        .expect("station map csv");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema StationMap {\n",
                "    station_id: String\n",
                "}\n\n",
                "args {\n",
                "    station_map: CsvFile = file(\"data/station_map.csv\")\n",
                "}\n\n",
                "stations = promote csv args.station_map as StationMap\n",
                "station_plus = derive stations column longitude_copy = longitude\n",
            ),
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-TABLE-UNKNOWN-COLUMN"));
    }

    #[test]
    fn records_table_join_transform_keys() {
        let root = env::temp_dir().join(format!("englang-table-join-ok-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(
            root.join("data").join("samples.csv"),
            "case_id,cooling_cop\ncase_001,3.2\ncase_002,3.4\n",
        )
        .expect("samples csv");
        fs::write(
            root.join("data").join("results.csv"),
            "case_id,unmet_hours\ncase_001,12\ncase_002,8\n",
        )
        .expect("results csv");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema DesignSample {\n",
                "    case_id: String\n",
                "    cooling_cop: Ratio [1]\n",
                "}\n\n",
                "schema SimulationResult {\n",
                "    case_id: String\n",
                "    unmet_hours: Duration [h]\n",
                "}\n\n",
                "args {\n",
                "    samples: CsvFile = file(\"data/samples.csv\")\n",
                "    results: CsvFile = file(\"data/results.csv\")\n",
                "}\n\n",
                "samples = promote csv args.samples as DesignSample\n",
                "results = promote csv args.results as SimulationResult\n",
                "joined = join samples with results\n",
                "on { samples.case_id == results.case_id }\n",
            ),
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let join = report
            .semantic_program
            .table_transforms
            .iter()
            .find(|transform| transform.binding == "joined")
            .expect("join transform");
        assert_eq!(join.operation, "join");
        assert_eq!(join.source_table, "samples");
        assert_eq!(join.secondary_table.as_deref(), Some("results"));
        assert_eq!(join.join_keys.len(), 1);
        assert_eq!(join.join_keys[0].left_column, "case_id");
        assert_eq!(join.join_keys[0].right_column, "case_id");

        let review = review_json(&report);
        assert!(review.contains("\"operation\": \"join\""));
        assert!(review.contains("\"secondary_table\": \"results\""));
        assert!(review.contains("\"join_keys\""));
    }

    #[test]
    fn diagnoses_table_join_key_mismatch() {
        let root = env::temp_dir().join(format!("englang-table-join-bad-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(root.join("data").join("samples.csv"), "case_id\ncase_001\n")
            .expect("samples csv");
        fs::write(root.join("data").join("results.csv"), "case_id\ncase_001\n")
            .expect("results csv");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            concat!(
                "schema DesignSample {\n",
                "    case_id: String\n",
                "}\n\n",
                "schema SimulationResult {\n",
                "    case_id: String\n",
                "}\n\n",
                "args {\n",
                "    samples: CsvFile = file(\"data/samples.csv\")\n",
                "    results: CsvFile = file(\"data/results.csv\")\n",
                "}\n\n",
                "samples = promote csv args.samples as DesignSample\n",
                "results = promote csv args.results as SimulationResult\n",
                "joined = join samples with results\n",
                "on {\n",
                "    samples.case_id == missing.case_id\n",
                "}\n",
            ),
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-TABLE-JOIN-KEY-MISMATCH"));
    }

    #[test]
    fn reports_unknown_promote_schema() {
        let report = check_source(
            "bad.eng",
            "sensor = promote csv \"data/sensor.csv\" as MissingSchema\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert_eq!(report.diagnostics[0].code, "E-SCHEMA-PROMOTE-001");
    }

    #[test]
    fn records_typed_config_promotions() {
        let root = env::temp_dir().join(format!(
            "englang-config-promotion-ok-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(
            root.join("data").join("workflow.toml"),
            "year = 2026\nregion = \"KR\"\noutput = \"build/out\"\ncache = true\n",
        )
        .expect("toml config");
        fs::write(
            root.join("data").join("workflow.json"),
            "{ \"year\": 2026, \"region\": \"KR\", \"output\": \"build/out\", \"cache\": true }\n",
        )
        .expect("json config");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            "schema WorkflowConfig {\n    year: Int\n    region: String\n    output: DirectoryPath\n    cache: Bool\n}\n\ntoml_config = promote toml file(\"data/workflow.toml\") as WorkflowConfig\njson_config = promote json file(\"data/workflow.json\") as WorkflowConfig\n",
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert_eq!(report.semantic_program.config_promotions.len(), 2);
        assert!(report
            .semantic_program
            .config_promotions
            .iter()
            .all(|promotion| promotion.status == "validated"
                && promotion.schema_name == "WorkflowConfig"
                && promotion.field_count == 4
                && promotion.source_hash.is_some()));
        let toml_binding = report
            .semantic_program
            .typed_bindings
            .iter()
            .find(|binding| binding.name == "toml_config")
            .expect("toml config binding");
        assert_eq!(toml_binding.semantic_type.quantity_kind, "ConfigObject");
        let review = review_json(&report);
        assert!(review.contains("\"config_promotions\""));
        assert!(review.contains("\"config_promotion_count\": 2"));
        assert!(review.contains("\"source_hash\": \""));
    }

    #[test]
    fn promotes_raw_structured_read_config_binding() {
        let root = env::temp_dir().join(format!(
            "englang-config-raw-read-promotion-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(
            root.join("data").join("workflow.json"),
            "{ \"year\": 2026, \"region\": \"KR\", \"output\": \"build/out\" }\n",
        )
        .expect("json config");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            "schema WorkflowConfig {\n    year: Int\n    region: String\n    output: DirectoryPath\n}\n\npayload = read json file(\"data/workflow.json\")\nconfig = promote json payload as WorkflowConfig\n",
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert_eq!(report.semantic_program.config_promotions.len(), 1);
        let promotion = &report.semantic_program.config_promotions[0];
        assert_eq!(promotion.source_literal, "payload");
        assert_eq!(promotion.status, "validated");
        assert!(promotion.resolved_path.contains("workflow.json"));
        assert!(promotion.source_hash.is_some());
        assert!(report.semantic_program.environment_dependencies.iter().any(
            |dependency| dependency.name == "payload"
                && dependency.kind == "filesystem_read_json"
                && dependency.source_hash.is_some()
        ));
        let review = review_json(&report);
        assert!(review.contains("\"source\": \"payload\""));
        assert!(review.contains("\"resolved_path\""));
        assert!(review.contains("\"source_hash\": \""));
    }

    #[test]
    fn diagnoses_invalid_typed_config_promotions() {
        let root = env::temp_dir().join(format!(
            "englang-config-promotion-bad-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(
            root.join("data").join("workflow.json"),
            "{ \"year\": \"2026\", \"region\": null, \"extra\": true }\n",
        )
        .expect("json config");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            "schema WorkflowConfig {\n    year: Int\n    region: String\n    output: DirectoryPath\n}\n\nconfig = promote json file(\"data/workflow.json\") as WorkflowConfig\n",
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(report.has_errors());
        for code in [
            "E-CONFIG-MISSING-FIELD",
            "E-CONFIG-UNKNOWN-FIELD",
            "E-CONFIG-NULL-NOT-OPTIONAL",
            "E-CONFIG-TYPE-MISMATCH",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code),
                "{code}: {:?}",
                report.diagnostics
            );
        }
        let promotion = report
            .semantic_program
            .config_promotions
            .first()
            .expect("config promotion");
        assert_eq!(promotion.status, "invalid");
        assert_eq!(promotion.missing_fields, vec!["output".to_owned()]);
        assert_eq!(promotion.unknown_fields, vec!["extra".to_owned()]);
        assert_eq!(promotion.null_fields, vec!["region".to_owned()]);
        assert_eq!(promotion.type_mismatches[0].field, "year");
    }

    #[test]
    fn accepts_optional_typed_config_fields() {
        let root = env::temp_dir().join(format!(
            "englang-config-promotion-optional-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(
            root.join("data").join("workflow.json"),
            "{ \"year\": 2026, \"region\": null }\n",
        )
        .expect("json config");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            "schema WorkflowConfig {\n    year: Int\n    region: Optional[String]\n    output: DirectoryPath?\n}\n\nconfig = promote json file(\"data/workflow.json\") as WorkflowConfig\n",
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let schema = report
            .semantic_program
            .schemas
            .iter()
            .find(|schema| schema.name == "WorkflowConfig")
            .expect("schema");
        let region = schema
            .columns
            .iter()
            .find(|column| column.name == "region")
            .expect("region column");
        assert_eq!(region.type_name, "String");
        assert!(region.optional);
        let promotion = report
            .semantic_program
            .config_promotions
            .first()
            .expect("config promotion");
        assert_eq!(promotion.status, "validated");
        assert!(promotion.missing_fields.is_empty());
        assert!(promotion.null_fields.is_empty());
        assert_eq!(
            promotion.optional_fields,
            vec!["region".to_owned(), "output".to_owned()]
        );
        assert_eq!(promotion.optional_missing_fields, vec!["output".to_owned()]);
        assert_eq!(promotion.optional_null_fields, vec!["region".to_owned()]);

        let review = review_json(&report);
        assert!(review.contains("\"optional\": true"));
        assert!(review.contains("\"optional_missing_fields\": [\"output\"]"));
        assert!(review.contains("\"optional_null_fields\": [\"region\"]"));
    }

    #[test]
    fn permits_optional_csv_column_missing_but_not_transform_reference() {
        let root = env::temp_dir().join(format!(
            "englang-csv-optional-column-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(root.join("data").join("stations.csv"), "id\nSTN001\n").expect("csv");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            "schema Station {\n    id: String\n    note: Optional[String]\n}\n\nstations = promote csv file(\"data/stations.csv\") as Station\nfiltered = filter stations\nwhere {\n    note is none\n}\n",
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-TABLE-UNKNOWN-COLUMN"));
        assert!(report
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "E-SCHEMA-CSV-002"));
        let promotion = report
            .semantic_program
            .csv_promotions
            .first()
            .expect("csv promotion");
        assert!(promotion.missing_columns.is_empty());
        assert_eq!(promotion.optional_missing_columns, vec!["note".to_owned()]);
    }

    #[test]
    fn records_net_http_get_and_download_boundaries() {
        let root = env::temp_dir().join(format!("englang-net-boundary-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("data dir");
        fs::write(root.join("data").join("response.json"), "{\"ok\":true}\n")
            .expect("response fixture");
        fs::write(root.join("data").join("file.csv"), "id,value\n1,42\n")
            .expect("download fixture");
        let source_path = root.join("main.eng");
        fs::write(
            &source_path,
            "response = http get url(\"https://api.example.org/hourly\")\nwith {\n    query = {\n    station = \"108\"\n    serviceKey = secret env(\"API_KEY\")\n    }\n    retry = 2\n    cache = true\n    cache_key = [\"weather\", \"108\", \"2026\"]\n    fixture = file(\"data/response.json\")\n}\n\ndownload url(\"https://example.org/file.csv\") to file(\"build/raw/file.csv\")\nwith {\n    fixture = file(\"data/file.csv\")\n    expected_sha256 = \"fixture-hash\"\n    cache = true\n    cache_key = [\"file\", \"v1\"]\n}\n",
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert_eq!(report.semantic_program.net_requests.len(), 1);
        assert_eq!(report.semantic_program.net_downloads.len(), 1);
        assert_eq!(report.semantic_program.cache_records.len(), 2);
        let request = &report.semantic_program.net_requests[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.status, "fixture");
        assert_eq!(request.retry, Some(2));
        assert!(request.cache);
        assert_eq!(request.status_code, Some(200));
        assert_eq!(request.status_class, "success");
        assert!(request.response_hash.is_some());
        assert!(request.query.iter().any(|param| param.key == "serviceKey"
            && param.value == "<redacted>"
            && param.redacted));
        let cache_record = &report.semantic_program.cache_records[0];
        assert_eq!(cache_record.owner_kind, "network_request");
        assert_eq!(cache_record.cache_dir, "cache");
        assert!(cache_record.cache_key_parts.starts_with(&[
            "weather".to_owned(),
            "108".to_owned(),
            "2026".to_owned()
        ]));
        assert!(cache_record
            .cache_key_parts
            .iter()
            .any(|part| part.starts_with("source_hash=")));
        assert_eq!(cache_record.status, "fixture_available");
        assert!(cache_record.observed_hash.is_some());
        let binding = report
            .semantic_program
            .typed_bindings
            .iter()
            .find(|binding| binding.name == "response")
            .expect("response binding");
        assert_eq!(binding.semantic_type.quantity_kind, "HttpResponse");
        let review = review_json(&report);
        assert!(review.contains("\"net_requests\""));
        assert!(review.contains("\"net_downloads\""));
        assert!(review.contains("\"status_code\": 200"));
        assert!(review.contains("\"status_class\": \"success\""));
        assert!(review.contains("\"cache_records\""));
        assert!(review.contains("\"cache_dir\": \"cache\""));
        assert!(review.contains("\"caches\""));
        assert!(review.contains("\"cache_count\": 2"));
        assert!(review.contains("\"kind\": \"network_request\""));
        assert!(review.contains("\"kind\": \"network_download\""));
    }

    #[test]
    fn rejects_nondeterministic_cache_key() {
        let report = check_source(
            "bad.eng",
            "response = http get url(\"https://example.org/data.json\")\nwith {\n    cache = true\n    cache_key = [now(), \"demo\"]\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-CACHE-KEY-NONDETERMINISTIC"));
    }

    #[test]
    fn rejects_invalid_net_url() {
        let report = check_source(
            "bad.eng",
            "response = http get url(\"ftp://example.org/file.csv\")\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-NET-INVALID-URL"));
    }

    #[test]
    fn rejects_missing_policy_for_unknown_column() {
        let report = check_source(
            "bad.eng",
            "schema SensorData {\n    time: DateTime index\n    missing {\n        T_supply: interpolate max_gap=10 min\n    }\n}\n",
            &CheckOptions::default(),
        );

        assert!(report.has_errors());
        assert_eq!(report.diagnostics[0].code, "E-SCHEMA-MISSING-001");
    }

    #[test]
    fn rejects_reserved_eq_binding() {
        let report = check_source("bad.eng", "eq = 1", &CheckOptions::default());

        assert!(report.has_errors());
        assert_eq!(report.diagnostics[0].code, "E-RESERVED-KEYWORD-001");
    }
}
