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
mod type_info;
mod units;

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub use ast::{AstItem, ExplicitDecl, FastBinding, SchemaDecl, ScriptDecl};
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
pub use semantic::{SemanticProgram, SemanticType, TypedBinding};
pub use source::SourceSpan;
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
}

pub fn check_file(path: impl AsRef<Path>, options: &CheckOptions) -> std::io::Result<CheckReport> {
    let path = path.as_ref();
    let source = fs::read_to_string(path)?;
    Ok(check_source(path, &source, options))
}

pub fn check_source(path: impl AsRef<Path>, source: &str, _options: &CheckOptions) -> CheckReport {
    let parsed = parser::parse_source(source);
    let source_path = path.as_ref();
    let schema_analysis = schema::analyze_schema(&parsed, source_path.parent());
    let mut semantic_output = semantic::analyze(&parsed);
    semantic_output
        .diagnostics
        .extend(schema_analysis.diagnostics);
    semantic_output.semantic_program.schemas = schema_analysis.schemas;
    semantic_output.semantic_program.csv_promotions = schema_analysis.csv_promotions;

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

pub fn build_bytecode(report: &CheckReport, source: &str, entry: &EntryPoint) -> String {
    encode_bytecode(&build_bytecode_program(report, source, entry))
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
    json.push_str(&format!(
        "  \"quantity_completion_count\": {},\n",
        report.quantity_completion_count
    ));
    json.push_str(&format!(
        "  \"unit_info_count\": {},\n",
        report.unit_info_count
    ));

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
