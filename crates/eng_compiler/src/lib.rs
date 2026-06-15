mod ast;
mod bytecode;
mod expected;
mod hover;
mod lexer;
mod ml;
mod parser;
mod quantities;
mod schema;
mod semantic;
mod source;
mod stats;
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
    PortDecl, PrintDecl, ReturnDecl, SchemaDecl, ScriptDecl, StructDecl, SystemDecl,
    SystemVariableDecl, TestDecl, WhereBindingDecl, WhereBlockDecl, WithBlockDecl, WithOptionDecl,
    WriteDecl,
};
pub use bytecode::{
    build_bytecode_program, encode_bytecode, parse_bytecode, BytecodeInstruction, BytecodeObject,
    BytecodeParseError, BytecodeProgram, BYTECODE_FORMAT, BYTECODE_VERSION,
};
pub use expected::{ExpectedType, ExpectedTypeSource};
pub use hover::HoverHint;
pub use lexer::{Keyword, Symbol, Token, TokenKind};
pub use ml::MlInfo;
pub use parser::{parse_source, ParseContext, ParsedLine, ParsedProgram, SyntaxSummary};
pub use quantities::{all_quantity_completions, normalize_unit, QuantityCompletion};
pub use schema::{CsvPromotion, MissingPolicy, SchemaColumn, SchemaConstraint, SchemaInfo};
pub use semantic::read_only_io_expression;
pub use semantic::{
    ArgValueInfo, ArgsBlockInfo, ArgsFieldInfo, AssertInfo, ClassFieldInfo, ClassInfo,
    ClassMethodInfo, ClassObjectFieldInfo, ClassObjectInfo, ClassObjectValidationInfo,
    ClassValidationInfo, CommandClauseInfo, CommandStyleInfo, ComponentAssemblyBoundaryInfo,
    ComponentAssemblyEquationInfo, ComponentAssemblyInfo, ComponentAssemblyVariableInfo,
    ComponentConnectionSetInfo, ComponentInfo, ComponentJacobianSparsityInfo,
    ComponentResidualDependencyInfo, ComponentResidualGraphInfo, ConnectionInfo, ConservationInfo,
    ConstInfo, CsvExportFieldInfo, CsvExportInfo, DomainInfo, DomainTypeParameterInfo,
    DomainVariableInfo, EnvironmentDependencyInfo, EquationDependencyInfo, EquationInfo,
    EquationIrInfo, FileOperationInfo, FormatExpressionInfo, FunctionInfo, FunctionLocalInfo,
    FunctionParamInfo, GoldenInfo, ImportInfo, JacobianSeedInfo, OdeRunnerInfo, PortInfo,
    PrintInfo, ResidualInfo, SemanticProgram, SemanticType, SolverPlanInfo, SystemInfo,
    SystemVariableInfo, TestInfo, TimeSeriesKernelInfo, TypedBinding, WhereBindingInfo,
    WhereBlockInfo, WithBlockInfo, WithOptionInfo, WriteInfo,
};
pub use source::SourceSpan;
pub use stats::{AxisInfo, IntegrationInfo, StatsInfo};
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
    semantic_output.semantic_program.arg_values = arg_values;
    semantic_output.semantic_program.environment_dependencies = collect_environment_dependencies(
        &parsed,
        source_path.parent(),
        &semantic_output.semantic_program,
    );
    semantic_output
        .diagnostics
        .extend(semantic::validate_simulation_contracts(
            &semantic_output.semantic_program,
            &semantic_output.inferred_declarations,
        ));

    CheckReport {
        source_path: source_path.to_path_buf(),
        source_hash: hash_text(source),
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
        if import.kind != "file" {
            diagnostics.push(Diagnostic::error(
                "E-IMPORT-001",
                import.line,
                &format!(
                    "`use {}` is not supported by the preview import resolver.",
                    import.target
                ),
                Some("Use a file import such as `use \"thermal.eng\"`."),
            ));
            continue;
        }
        if import.target.contains('{') || import.target.contains("args.") {
            diagnostics.push(Diagnostic::error(
                "E-IMPORT-DYNAMIC-001",
                import.line,
                "import path cannot depend on args/runtime values.",
                Some("Use a static file import such as `use \"./defaults.eng\"`."),
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
        "string" | "path" | "filepath" | "csvfile" | "jsonfile" | "tomlfile" | "textfile"
        | "reportfile" | "plotfile" | "directorypath" => Ok(stripped),
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
            resolved_path: path.display().to_string(),
            source_hash: Some(hash_text(&source)),
            status: "read".to_owned(),
        }),
        Err(_) => Some(ReadObservation {
            kind: kind.to_owned(),
            resolved_path: path.display().to_string(),
            source_hash: None,
            status: "missing".to_owned(),
        }),
    }
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
        return Some(value);
    }
    if let Some(value) = strip_call_string_arg(expression, "dir") {
        return Some(value);
    }
    if expression.starts_with('"') {
        return Some(strip_string_literal(expression));
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
    path.as_ref().replace('\\', "/")
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
        json.push_str(&format!("      \"row_count\": {},\n", operator.row_count));
        json.push_str(&format!(
            "      \"column_count\": {},\n",
            operator.column_count
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
        json.push_str(&format!("      \"line\": {},\n", component.line));
        json.push_str(&format!(
            "      \"port_count\": {},\n",
            component.ports.len()
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

fn push_json_string_array(json: &mut String, values: &[String]) {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!("\"{}\"", json_escape(value)));
    }
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
            "args {\n    input: CsvFile = file(\"data/sensor.csv\")\n    output: DirectoryPath = dir(\"build/out\")\n}\n\ninput_exists = exists args.input\nsummary_file = join(args.output, \"summary.csv\")\ninput_parent = parent(args.input)\ninput_stem = stem(args.input)\ninput_ext = extension(args.input)\n\nprint \"exists={input_exists} summary={summary_file} parent={input_parent} stem={input_stem} ext={input_ext}\"\n",
        )
        .expect("source");

        let report = check_file(&source_path, &CheckOptions::default()).expect("check file");

        assert!(!report.has_errors());
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
        assert!(review.contains("\"source_hash\""));
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
            "domain Fluid[Medium M] package \"eng.std.domains.fluid\" version \"0.1.0\" {\n    across height: Length [m]\n    through m_dot: MassFlowRate [kg/s]\n    conservation sum(m_dot) = 0\n}\n\ncomponent Supply {\n    port outlet: Fluid[Water]\n}\n\ncomponent Return {\n    port inlet: Fluid[Water]\n}\n\nconnect Supply.outlet -> Return.inlet\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
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
            report.semantic_program.connections[0].status,
            "domain_compatible"
        );
        assert_eq!(report.semantic_program.component_assemblies.len(), 1);
        let assembly = &report.semantic_program.component_assemblies[0];
        assert_eq!(assembly.status, "assembly_seed");
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
        assert_eq!(assembly.boundary.algebraic_count, 4);
        assert_eq!(assembly.boundary.equation_count, 2);
        assert_eq!(assembly.boundary.balance_status, "underdetermined_seed");
        assert_eq!(
            assembly.boundary.diagnostic_code.as_deref(),
            Some("W-ASSEMBLY-UNDERDETERMINED-SEED")
        );
        assert_eq!(assembly.residual_graph.status, "metadata_only");
        assert_eq!(assembly.residual_graph.jacobian_sparsity.len(), 2);

        let review = review_json(&report);
        assert!(review.contains("\"domain_summary\""));
        assert!(review.contains("\"component_summary\""));
        assert!(review.contains("\"connection_summary\""));
        assert!(review.contains("\"assembly_summary\""));
        assert!(review.contains("\"connection_set_1\""));
        assert!(review.contains("\"through_conservation\""));
        assert!(review.contains("\"component_residual_graph\""));
        assert!(review.contains("\"type_parameters\""));
        assert!(review.contains("\"kind\": \"Medium\""));
        assert!(review.contains("\"name\": \"M\""));
        assert!(review.contains("\"package\": \"eng.std.domains.fluid\""));
        assert!(review.contains("\"Fluid[Water]\""));
        assert!(review.contains("\"domain_compatible\""));
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
            diagnostic.code == "W-PORT-UNCONNECTED-001" && diagnostic.severity == Severity::Warning
        }));
    }

    #[test]
    fn records_class_object_metadata_and_field_access() {
        let report = check_source(
            "class_preview.eng",
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
            .any(|diagnostic| diagnostic.code == "E-CONNECT-DOMAIN-001"));
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
                "E-CONNECT-MEDIUM-001",
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
        assert_eq!(kernel.status, "preview_supported");
        assert!(kernel
            .operations
            .iter()
            .any(|operation| operation == "temperature_delta:return_minus_supply"));
        let review = review_json(&report);
        assert!(review.contains("\"timeseries_kernels\""));
        assert!(review.contains("\"table_heat_rate_from_mass_flow_cp_delta_t\""));
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
            "T_supply_meas = measured(12 degC, std=0.2 K)\nT_return_band = interval(20 degC, 24 degC)\nQ_coil_dist = normal(mean=5 kW, std=0.8 kW, samples=31)\nQ_uniform = uniform(4 kW, 6 kW, samples=11)\nQ_coil_ensemble = ensemble(Q_coil_dist, samples=31)\nQ_total_unc = propagate(Q_coil_dist, method=linear, scale=1.08, offset=0.4 kW)\n",
            &CheckOptions::default(),
        );

        assert!(!report.has_errors());
        assert_eq!(report.semantic_program.uncertainty_infos.len(), 6);
        assert_eq!(
            report.semantic_program.uncertainty_infos[0].kind,
            "Measured"
        );
        assert_eq!(
            report.semantic_program.uncertainty_infos[2].sample_count,
            31
        );
        assert_eq!(
            report.semantic_program.uncertainty_infos[3]
                .distribution
                .as_deref(),
            Some("uniform")
        );
        assert_eq!(
            report.semantic_program.uncertainty_infos[2].display_unit,
            "kW"
        );
        assert_eq!(
            report.semantic_program.uncertainty_infos[5]
                .source
                .as_deref(),
            Some("Q_coil_dist")
        );
        assert_eq!(
            report.semantic_program.uncertainty_infos[5].display_unit,
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
            report.semantic_program.uncertainty_infos[5]
                .method
                .as_deref(),
            Some("linear")
        );
        assert_eq!(
            report.semantic_program.uncertainty_infos[5]
                .scale
                .as_deref(),
            Some("1.08")
        );
        assert_eq!(
            report.semantic_program.uncertainty_infos[5]
                .offset
                .as_deref(),
            Some("0.4 kW")
        );

        let review = review_json(&report);
        assert!(review.contains("\"uncertainty_info\""));
        assert!(review.contains("\"distribution\": \"uniform\""));
        assert!(review.contains("\"scale\": \"1.08\""));
        assert!(review.contains("\"offset\": \"0.4 kW\""));
        assert!(review.contains("\"Measured[AbsoluteTemperature]\""));
        assert!(review.contains("\"Distribution[HeatRate]\""));
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
            "system ThermalStateSpacePreview {\n    state T_zone: AbsoluteTemperature = 22 degC\n    input T_out: AbsoluteTemperature = 8 degC\n    input Q_internal: HeatRate = 500 W\n    states x = [T_zone]\n    inputs u = [T_out, Q_internal]\n    outputs y = [T_zone]\n    A: LinearOperator[StateVector -> Derivative[StateVector]] = [[-0.0002]]\n    B: LinearOperator[InputVector -> Derivative[StateVector]] = [[0.0002, 0.001]]\n    equation {\n        der(x) eq A * x + B * u\n    }\n}\n",
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
        assert_eq!(report.semantic_program.linear_operators[1].row_count, 1);
        assert_eq!(report.semantic_program.linear_operators[1].column_count, 2);
        let json = review_json(&report);
        assert!(json.contains("\"state_space_vectors\""));
        assert!(json.contains("\"linear_operators\""));
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
            .any(|diagnostic| diagnostic.code == "E-SIM-OPTION-MISSING-001"));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SIM-OPTION-MISSING-002"));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SIM-INPUT-MISSING-001"));
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
            .any(|diagnostic| diagnostic.code == "E-SIM-INPUT-QTY-001"));
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
            .any(|diagnostic| diagnostic.code == "E-SIM-INPUT-AXIS-001"));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SIM-OPTION-TYPE-001"));
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
