mod ast;
mod lexer;
mod parser;
mod semantic;
mod source;

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub use ast::{AstItem, ExplicitDecl, FastBinding, SchemaDecl, ScriptDecl};
pub use lexer::{Keyword, Symbol, Token, TokenKind};
pub use parser::{parse_source, ParseContext, ParsedLine, ParsedProgram, SyntaxSummary};
pub use semantic::{SemanticProgram, SemanticType, TypedBinding};
pub use source::SourceSpan;

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
}

pub fn check_file(path: impl AsRef<Path>, options: &CheckOptions) -> std::io::Result<CheckReport> {
    let path = path.as_ref();
    let source = fs::read_to_string(path)?;
    Ok(check_source(path, &source, options))
}

pub fn check_source(path: impl AsRef<Path>, source: &str, _options: &CheckOptions) -> CheckReport {
    let parsed = parser::parse_source(source);
    let semantic_output = semantic::analyze(&parsed);

    CheckReport {
        source_path: path.as_ref().to_path_buf(),
        source_hash: hash_text(source),
        diagnostics: semantic_output.diagnostics,
        inferred_declarations: semantic_output.inferred_declarations,
        syntax_summary: parsed.summary(),
        semantic_program: semantic_output.semantic_program,
    }
}

pub fn build_bytecode(report: &CheckReport, source: &str) -> String {
    let mut bytecode = String::new();
    bytecode.push_str("ENGBYTECODE 0.1\n");
    bytecode.push_str("format = engbc-preview-1\n");
    bytecode.push_str(&format!("compiler_version = {}\n", COMPILER_VERSION));
    bytecode.push_str(&format!("source_hash = {}\n", report.source_hash));
    bytecode.push_str(&format!("source_bytes = {}\n", source.len()));
    bytecode.push_str(&format!("source_lines = {}\n", report.syntax_summary.lines));
    bytecode.push_str(&format!("tokens = {}\n", report.syntax_summary.tokens));
    bytecode.push_str(&format!(
        "ast_items = {}\n",
        report.syntax_summary.ast_items
    ));
    bytecode.push_str(&format!(
        "typed_bindings = {}\n",
        report.semantic_program.typed_bindings.len()
    ));
    bytecode.push_str("entry = script main\n");
    bytecode.push_str("instructions:\n");
    bytecode.push_str("  0000 LOAD_TYPED_SOURCE\n");
    bytecode.push_str("  0001 VALIDATE_SEMANTICS\n");
    bytecode.push_str("  0002 EMIT_RESULT\n");
    bytecode
}

pub fn review_json(report: &CheckReport) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"format\": \"eng-review-preview-1\",\n");
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
        "    \"fast_bindings\": {},\n",
        report.syntax_summary.fast_bindings
    ));
    json.push_str(&format!(
        "    \"explicit_declarations\": {}\n",
        report.syntax_summary.explicit_declarations
    ));
    json.push_str("  },\n");

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
    json.push_str("\n  ]\n");
    json.push_str("}\n");
    json
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
        assert_eq!(report.syntax_summary.fast_bindings, 1);
        assert_eq!(report.inferred_declarations[0].quantity_kind, "Length");
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
    fn rejects_reserved_eq_binding() {
        let report = check_source("bad.eng", "eq = 1", &CheckOptions::default());

        assert!(report.has_errors());
        assert_eq!(report.diagnostics[0].code, "E-RESERVED-KEYWORD-001");
    }
}
