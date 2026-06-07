mod ast;
mod bytecode;
mod entry;
mod expected;
mod hover;
mod lexer;
mod parser;
mod quantities;
mod schema;
mod semantic;
mod source;
mod stats;
mod type_info;
mod units;

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub use ast::{
    AstItem, EquationDecl, ExplicitDecl, FastBinding, SchemaDecl, ScriptDecl, StructDecl,
    StructFieldDecl, SystemDecl, SystemVariableDecl,
};
pub use bytecode::{
    build_bytecode_program, encode_bytecode, parse_bytecode, BytecodeInstruction, BytecodeObject,
    BytecodeParseError, BytecodeProgram, BYTECODE_FORMAT, BYTECODE_VERSION,
};
pub use entry::{select_entry, EntryPoint};
pub use expected::{ExpectedType, ExpectedTypeSource};
pub use hover::HoverHint;
pub use lexer::{Keyword, Symbol, Token, TokenKind};
pub use parser::{parse_source, ParseContext, ParsedLine, ParsedProgram, SyntaxSummary};
pub use quantities::{all_quantity_completions, QuantityCompletion};
pub use schema::{CsvPromotion, MissingPolicy, SchemaColumn, SchemaConstraint, SchemaInfo};
pub use semantic::{
    ArgValueInfo, ArgsFieldInfo, ArgsStructInfo, EquationDependencyInfo, EquationInfo,
    EquationIrInfo, JacobianSeedInfo, OdeRunnerInfo, ResidualInfo, SemanticProgram, SemanticType,
    SolverPlanInfo, SystemInfo, SystemVariableInfo, TypedBinding,
};
pub use source::SourceSpan;
pub use stats::{AxisInfo, IntegrationInfo, StatsInfo};
pub use type_info::{TypeInfo, TypeInfoSource};
pub use units::{all_unit_infos, UnitDerivation, UnitInfo};

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
    let parsed = parser::parse_source(source);
    let source_path = path.as_ref();
    let mut semantic_output = semantic::analyze(&parsed);
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
    for args_struct in &program.args_structs {
        for field in &args_struct.fields {
            declared.insert(field.name.clone());
            let (value, source) = if let Some(value) = overrides.get(&field.name) {
                (value.clone(), "cli")
            } else if let Some(default_value) = &field.default_value {
                (strip_string_literal(default_value), "default")
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
                Some("Declare the field in struct Args or remove the flag."),
            ));
        }
    }

    (values, diagnostics)
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

pub fn build_bytecode(report: &CheckReport, source: &str, entry: &EntryPoint) -> String {
    encode_bytecode(&build_bytecode_program(report, source, entry))
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
        "    \"schemas\": {},\n",
        report.syntax_summary.schemas
    ));
    json.push_str(&format!(
        "    \"systems\": {},\n",
        report.syntax_summary.systems
    ));
    json.push_str(&format!(
        "    \"structs\": {},\n",
        report.syntax_summary.structs
    ));
    json.push_str(&format!(
        "    \"struct_fields\": {},\n",
        report.syntax_summary.struct_fields
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
        "    \"explicit_declarations\": {}\n",
        report.syntax_summary.explicit_declarations
    ));
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

    json.push_str("  \"entry_points\": [\n");
    for (index, entry) in report.semantic_program.entry_points.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"kind\": \"{}\",\n",
            json_escape(&entry.kind)
        ));
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&entry.name)
        ));
        if let Some(arg_name) = &entry.arg_name {
            json.push_str(&format!(
                "      \"arg_name\": \"{}\",\n",
                json_escape(arg_name)
            ));
        } else {
            json.push_str("      \"arg_name\": null,\n");
        }
        if let Some(arg_type) = &entry.arg_type {
            json.push_str(&format!(
                "      \"arg_type\": \"{}\",\n",
                json_escape(arg_type)
            ));
        } else {
            json.push_str("      \"arg_type\": null,\n");
        }
        if let Some(return_type) = &entry.return_type {
            json.push_str(&format!(
                "      \"return_type\": \"{}\",\n",
                json_escape(return_type)
            ));
        } else {
            json.push_str("      \"return_type\": null,\n");
        }
        json.push_str(&format!("      \"line\": {}\n", entry.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"args_summary\": [\n");
    for (index, args_struct) in report.semantic_program.args_structs.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&args_struct.name)
        ));
        json.push_str(&format!("      \"line\": {},\n", args_struct.line));
        json.push_str(&format!(
            "      \"field_count\": {},\n",
            args_struct.fields.len()
        ));
        json.push_str("      \"fields\": [\n");
        for (field_index, field) in args_struct.fields.iter().enumerate() {
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
    fn parser_records_script_and_binding_items() {
        let report = check_source(
            "ok.eng",
            "script main(args: Args) -> Report {\n    L = 1 m + 20 cm\n}\n",
            &CheckOptions::default(),
        );

        assert_eq!(report.syntax_summary.scripts, 1);
        assert_eq!(report.semantic_program.entry_points[0].name, "main");
        assert_eq!(
            report.semantic_program.entry_points[0].arg_type.as_deref(),
            Some("Args")
        );
        assert_eq!(
            report.semantic_program.entry_points[0]
                .return_type
                .as_deref(),
            Some("Report")
        );
        assert_eq!(report.syntax_summary.fast_bindings, 1);
        assert_eq!(report.inferred_declarations[0].quantity_kind, "Length");
    }

    #[test]
    fn records_args_struct_metadata() {
        let report = check_source(
            "ok.eng",
            "struct Args {\n    case_name: String = \"baseline\"\n}\n\nscript main(args: Args) -> Report {\n    L = 1 m\n}\n",
            &CheckOptions::default(),
        );

        assert_eq!(report.syntax_summary.structs, 1);
        assert_eq!(report.syntax_summary.struct_fields, 1);
        assert_eq!(report.semantic_program.args_structs[0].name, "Args");
        assert_eq!(
            report.semantic_program.args_structs[0].fields[0].name,
            "case_name"
        );
        assert_eq!(
            report.semantic_program.args_structs[0].fields[0]
                .default_value
                .as_deref(),
            Some("\"baseline\"")
        );

        let review = review_json(&report);
        assert!(review.contains("\"args_summary\""));
        assert!(review.contains("\"case_name\""));
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
    fn selects_default_script_main_entry() {
        let report = check_source(
            "ok.eng",
            "script main(args: Args) -> Report {\n    L = 1 m\n}\n",
            &CheckOptions::default(),
        );

        let entry = select_entry(&report.semantic_program.entry_points, None).unwrap();

        assert_eq!(entry.signature(), "script main(args: Args) -> Report");
    }

    #[test]
    fn reports_missing_run_entry() {
        let report = check_source("bad.eng", "L = 1 m\n", &CheckOptions::default());

        let diagnostic = select_entry(&report.semantic_program.entry_points, None).unwrap_err();

        assert_eq!(diagnostic.code, "E-ENTRY-NOT-FOUND-001");
    }

    #[test]
    fn bytecode_v1_round_trips_entry_and_instructions() {
        let source = "script main(args: Args) -> Report {\n    L = 1 m\n}\n";
        let report = check_source("ok.eng", source, &CheckOptions::default());
        let entry = select_entry(&report.semantic_program.entry_points, None).unwrap();

        let bytecode = build_bytecode(&report, source, &entry);
        let decoded = parse_bytecode(&bytecode).unwrap();

        assert!(bytecode.starts_with("ENGBYTECODE 1\nformat = engbc-v1\n"));
        assert!(bytecode.contains("entry = script main\n"));
        assert!(bytecode.contains("0000|enter_entry|script|main\n"));
        assert_eq!(decoded.entry.name, "main");
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
            "script main(args: Args) -> Report {\n    sensor = promote csv \"data/sensor.csv\" as SensorData\n    cp = 4180 J/kg/K\n    Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\n    E_coil = integrate(Q_coil, over=Time)\n\n    return report {\n        summarize Q_coil by [mean, max, p95]\n    }\n}\n",
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
            "script main(args: Args) -> Report {\n    sensor = promote csv \"data/sensor.csv\" as SensorData\n    cp = 4180 J/kg/K\n    Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\n    E_bad = sum(Q_coil, axis=Time)\n}\n",
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
    fn review_json_exposes_v07_review_contract_sections() {
        let report = check_source(
            "ok.eng",
            "schema SensorData {\n    time: DateTime index\n    T_supply: AbsoluteTemperature [degC]\n}\n\nscript main(args: Args) -> Report {\n    power = 10 kW\n    L = 1 m + 20 cm\n}\n",
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
            "script main(args: Args) -> Report {\n    sensor = promote csv \"data/sensor.csv\" as MissingSchema\n}\n",
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
