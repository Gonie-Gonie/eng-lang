use std::collections::HashSet;

use crate::ast::AstItem;
use crate::lexer::{Symbol, TokenKind};
use crate::parser::ParsedProgram;
use crate::semantic::{SemanticProgram, WithOptionInfo};
use crate::{Diagnostic, InferredDeclaration};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseRunOutputInfo {
    pub name: String,
    pub expression: String,
    pub source_columns: Vec<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseRunInfo {
    pub binding: String,
    pub source_table: String,
    pub outputs: Vec<CaseRunOutputInfo>,
    pub result_path: String,
    pub manifest_path: String,
    pub on_error: String,
    pub resume: bool,
    pub overwrite: bool,
    pub scheduler: String,
    pub runner: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CaseAnalysis {
    pub runs: Vec<CaseRunInfo>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn analyze_case_runs(
    parsed: &ParsedProgram,
    program: &SemanticProgram,
    declarations: &[InferredDeclaration],
) -> CaseAnalysis {
    let mut analysis = CaseAnalysis::default();
    for item in &parsed.items {
        let AstItem::FastBinding(binding) = item else {
            continue;
        };
        let Some((step, source_table)) = parse_case_apply_expression(&binding.expression) else {
            continue;
        };
        if step != "run_case" {
            continue;
        }

        let mut status = "declared".to_owned();
        let source_type = program
            .typed_bindings
            .iter()
            .find(|candidate| candidate.name == source_table)
            .map(|candidate| candidate.semantic_type.quantity_kind.as_str());
        if !matches!(source_type, Some("Table[Case]") | Some("Table[CaseOutput]")) {
            analysis.diagnostics.push(Diagnostic::error(
                "E-CASE-RUN-SOURCE",
                binding.line,
                &format!(
                    "`apply run_case` requires a Case or CaseOutput table, but `{source_table}` is not one."
                ),
                Some("Use the result of `materialize cases ...` or `apply case_input_template over ...`."),
            ));
            status = "invalid_source".to_owned();
        }

        let options = options_for_owner(program, binding.line);
        let result_range = result_map_range(parsed, &options);
        validate_top_level_options(
            &options,
            result_range,
            binding.line,
            &mut analysis.diagnostics,
            &mut status,
        );
        let mut outputs = case_run_outputs(parsed, &options, result_range);
        validate_case_run_outputs(
            &mut outputs,
            binding.line,
            &mut analysis.diagnostics,
            &mut status,
        );

        let on_error = option_text(&options, "on_error").unwrap_or_else(|| "fail".to_owned());
        if !matches!(on_error.as_str(), "fail" | "continue") {
            analysis.diagnostics.push(Diagnostic::error(
                "E-CASE-RUN-POLICY",
                option_line(&options, "on_error").unwrap_or(binding.line),
                &format!("Unknown run-case `on_error` policy `{on_error}`."),
                Some("Use `on_error = fail` or `on_error = continue`."),
            ));
            status = "invalid_policy".to_owned();
        }

        let resume = bool_option(
            &options,
            "resume",
            false,
            &mut analysis.diagnostics,
            &mut status,
        );
        let overwrite = bool_option(
            &options,
            "overwrite",
            false,
            &mut analysis.diagnostics,
            &mut status,
        );
        let result_path = case_run_path_option(
            parsed,
            &options,
            "result",
            "{case_dir}/result.json",
            &mut analysis.diagnostics,
            &mut status,
        );
        let manifest_path = case_run_path_option(
            parsed,
            &options,
            "manifest",
            "{case_dir}/case_run_manifest.json",
            &mut analysis.diagnostics,
            &mut status,
        );
        if result_path == manifest_path {
            analysis.diagnostics.push(Diagnostic::error(
                "E-CASE-RUN-PATH",
                option_line(&options, "manifest").unwrap_or(binding.line),
                "Run-case `result` and `manifest` paths must be different.",
                Some("Write the calculated result and scheduler manifest to separate per-case files."),
            ));
            status = "invalid_path".to_owned();
        }

        if declarations
            .iter()
            .all(|declaration| declaration.name != source_table)
        {
            analysis.diagnostics.push(Diagnostic::error(
                "E-CASE-RUN-SOURCE",
                binding.line,
                &format!("Run-case source `{source_table}` is not a prior materialized binding."),
                Some("Declare the case table before applying `run_case`."),
            ));
            status = "missing_source".to_owned();
        }

        analysis.runs.push(CaseRunInfo {
            binding: binding.name.clone(),
            source_table,
            outputs,
            result_path,
            manifest_path,
            on_error,
            resume,
            overwrite,
            scheduler: "sequential".to_owned(),
            runner: "native_expression".to_owned(),
            status,
            line: binding.line,
        });
    }
    analysis
}

fn options_for_owner(program: &SemanticProgram, owner_line: usize) -> Vec<WithOptionInfo> {
    program
        .with_blocks
        .iter()
        .filter(|block| block.owner_line == Some(owner_line))
        .flat_map(|block| block.options.iter().cloned())
        .filter(|option| option.status == "accepted")
        .collect()
}

fn result_map_range(parsed: &ParsedProgram, options: &[WithOptionInfo]) -> Option<(usize, usize)> {
    let option = options
        .iter()
        .find(|option| option.key == "results" && option.value.trim_start().starts_with('{'))?;
    option_map_range(parsed, option.line)
}

fn option_map_range(parsed: &ParsedProgram, start_line: usize) -> Option<(usize, usize)> {
    let mut depth = 0i32;
    let mut seen_start = false;
    for line in parsed.lines.iter().filter(|line| line.line >= start_line) {
        seen_start |= line.line == start_line;
        if !seen_start {
            continue;
        }
        depth += line
            .tokens
            .iter()
            .map(|token| match token.kind {
                TokenKind::Symbol(Symbol::LBrace) => 1,
                TokenKind::Symbol(Symbol::RBrace) => -1,
                _ => 0,
            })
            .sum::<i32>();
        if depth <= 0 {
            return Some((start_line, line.line));
        }
    }
    seen_start.then_some((start_line, usize::MAX))
}

fn validate_top_level_options(
    options: &[WithOptionInfo],
    result_range: Option<(usize, usize)>,
    owner_line: usize,
    diagnostics: &mut Vec<Diagnostic>,
    status: &mut String,
) {
    for option in options {
        if option.key == "}" || line_in_range(option.line, result_range) {
            continue;
        }
        if case_run_control_option(&option.key) {
            continue;
        }
        diagnostics.push(Diagnostic::error(
            "E-CASE-RUN-OPTION",
            option.line,
            &format!("Unknown run-case option `{}`.", option.key),
            Some("Put calculated output fields inside `results = { ... }`; keep scheduler policy options at the top level."),
        ));
        *status = "invalid_option".to_owned();
    }
    if result_range.is_none() {
        diagnostics.push(Diagnostic::error(
            "E-CASE-RUN-RESULTS-MISSING",
            owner_line,
            "`apply run_case` requires a `results = { ... }` map.",
            Some("Declare one or more native result expressions, for example `results = { annual_energy = load * hours }`."),
        ));
        *status = "missing_results".to_owned();
    }
}

fn case_run_outputs(
    _parsed: &ParsedProgram,
    options: &[WithOptionInfo],
    result_range: Option<(usize, usize)>,
) -> Vec<CaseRunOutputInfo> {
    let Some(range) = result_range else {
        return Vec::new();
    };
    options
        .iter()
        .filter(|option| line_in_range(option.line, Some(range)))
        .filter(|option| option.key != "}")
        .map(|option| CaseRunOutputInfo {
            name: option.key.clone(),
            expression: option.value.clone(),
            source_columns: crate::table::expression_columns(&option.value),
            status: "accepted".to_owned(),
            line: option.line,
        })
        .collect()
}

fn validate_case_run_outputs(
    outputs: &mut [CaseRunOutputInfo],
    owner_line: usize,
    diagnostics: &mut Vec<Diagnostic>,
    run_status: &mut String,
) {
    if outputs.is_empty() {
        diagnostics.push(Diagnostic::error(
            "E-CASE-RUN-RESULTS-MISSING",
            owner_line,
            "Run-case `results` must declare at least one calculated output.",
            Some("Add `name = expression` entries inside the `results` map."),
        ));
        *run_status = "missing_results".to_owned();
        return;
    }
    let mut names = HashSet::new();
    for output in outputs {
        if reserved_case_run_column(&output.name) {
            diagnostics.push(Diagnostic::error(
                "E-CASE-RUN-OUTPUT-RESERVED",
                output.line,
                &format!("Run-case output `{}` conflicts with scheduler metadata.", output.name),
                Some("Choose a domain result name that is not a case status, path, hash, or runner field."),
            ));
            output.status = "reserved_name".to_owned();
            *run_status = "invalid_output".to_owned();
        } else if !names.insert(output.name.clone()) {
            diagnostics.push(Diagnostic::error(
                "E-CASE-RUN-OUTPUT-DUPLICATE",
                output.line,
                &format!(
                    "Run-case output `{}` is declared more than once.",
                    output.name
                ),
                Some("Keep one expression for each result field."),
            ));
            output.status = "duplicate".to_owned();
            *run_status = "invalid_output".to_owned();
        } else if output.expression.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "E-CASE-RUN-OUTPUT-EXPRESSION",
                output.line,
                &format!("Run-case output `{}` has an empty expression.", output.name),
                Some("Assign a numeric expression based on case input columns."),
            ));
            output.status = "empty_expression".to_owned();
            *run_status = "invalid_output".to_owned();
        }
    }
}

fn bool_option(
    options: &[WithOptionInfo],
    key: &str,
    default: bool,
    diagnostics: &mut Vec<Diagnostic>,
    status: &mut String,
) -> bool {
    let Some(option) = options.iter().find(|option| option.key == key) else {
        return default;
    };
    match option.value.trim().to_ascii_lowercase().as_str() {
        "true" => true,
        "false" => false,
        _ => {
            diagnostics.push(Diagnostic::error(
                "E-CASE-RUN-POLICY",
                option.line,
                &format!("Run-case option `{key}` expects `true` or `false`."),
                Some(&format!("Use `{key} = true` or `{key} = false`.")),
            ));
            *status = "invalid_policy".to_owned();
            default
        }
    }
}

fn case_run_path_option(
    parsed: &ParsedProgram,
    options: &[WithOptionInfo],
    key: &str,
    default: &str,
    diagnostics: &mut Vec<Diagnostic>,
    status: &mut String,
) -> String {
    let Some(option) = options.iter().find(|option| option.key == key) else {
        return default.to_owned();
    };
    let path = option.value.trim();
    let is_string_literal = parsed
        .lines
        .iter()
        .find(|line| line.line == option.line)
        .is_some_and(|line| {
            line.tokens.iter().any(
                |token| matches!(&token.kind, TokenKind::StringLiteral(value) if value == path),
            )
        });
    if !is_string_literal {
        diagnostics.push(Diagnostic::error(
            "E-CASE-RUN-PATH",
            option.line,
            &format!("Run-case `{key}` expects a quoted per-case path."),
            Some(&format!(
                "Use `{key} = \"{{case_dir}}/{}.json\"`.",
                if key == "result" {
                    "result"
                } else {
                    "case_run_manifest"
                }
            )),
        ));
        *status = "invalid_path".to_owned();
        return path.to_owned();
    }
    if path.trim().is_empty()
        || !["{case_dir}", "{case_id}", "{row}"]
            .iter()
            .any(|placeholder| path.contains(placeholder))
    {
        diagnostics.push(Diagnostic::error(
            "E-CASE-RUN-PATH",
            option.line,
            &format!("Run-case `{key}` must identify each case separately."),
            Some("Include `{case_dir}`, `{case_id}`, or `{row}` in the path to prevent case outputs from colliding."),
        ));
        *status = "invalid_path".to_owned();
    }
    path.to_owned()
}

fn option_text(options: &[WithOptionInfo], key: &str) -> Option<String> {
    options
        .iter()
        .find(|option| option.key == key)
        .map(|option| strip_string_literal(&option.value))
}

fn option_line(options: &[WithOptionInfo], key: &str) -> Option<usize> {
    options
        .iter()
        .find(|option| option.key == key)
        .map(|option| option.line)
}

fn strip_string_literal(value: &str) -> String {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(value)
        .to_owned()
}

fn line_in_range(line: usize, range: Option<(usize, usize)>) -> bool {
    range.is_some_and(|(start, end)| line > start && line < end)
}

fn case_run_control_option(key: &str) -> bool {
    matches!(
        key,
        "results" | "result" | "manifest" | "on_error" | "resume" | "overwrite"
    )
}

fn reserved_case_run_column(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "case_id"
            | "case_dir"
            | "case_status"
            | "input_status"
            | "input_path"
            | "input_manifest_path"
            | "result_path"
            | "manifest_path"
            | "result_hash"
            | "failure_reason"
            | "runner"
            | "scheduler"
            | "status"
    )
}

fn parse_case_apply_expression(expression: &str) -> Option<(String, String)> {
    let expression = expression.trim();
    if let Some(inner) = expression
        .strip_prefix("apply(")
        .and_then(|value| value.strip_suffix(')'))
    {
        let mut parts = inner.split(',').map(str::trim);
        let step = parts.next().filter(|value| is_identifier(value))?;
        let source = parts.find_map(|part| {
            part.strip_prefix("over=")
                .map(str::trim)
                .filter(|value| is_identifier(value))
        })?;
        return Some((step.to_owned(), source.to_owned()));
    }

    let mut parts = expression.split_whitespace();
    if parts.next()? != "apply" {
        return None;
    }
    let step = parts.next()?;
    if parts.next()? != "over" {
        return None;
    }
    let source = parts.next()?;
    if parts.next().is_some() || !is_identifier(step) || !is_identifier(source) {
        return None;
    }
    Some((step.to_owned(), source.to_owned()))
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}
