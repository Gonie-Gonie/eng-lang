use std::collections::BTreeSet;
use std::path::Path;

use eng_compiler::{
    all_quantity_completions, all_unit_infos, check_file, check_source, CheckOptions, CheckReport,
    Severity,
};
use serde_json::{json, Value};

pub const LSP_SNAPSHOT_FORMAT: &str = "eng-lsp-snapshot-v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspSnapshot {
    pub diagnostics: Vec<LspDiagnostic>,
    pub completions: Vec<LspCompletion>,
    pub hovers: Vec<LspHover>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspDiagnostic {
    pub line: usize,
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
    pub line: usize,
    pub detail: String,
    pub quantity_kind: String,
    pub display_unit: String,
}

pub fn snapshot_for_path(path: &Path) -> std::io::Result<LspSnapshot> {
    let report = check_file(path, &CheckOptions::default())?;
    Ok(snapshot_from_report(&report))
}

pub fn snapshot_for_source(path: &Path, source: &str) -> LspSnapshot {
    let report = check_source(path, source, &CheckOptions::default());
    snapshot_from_report(&report)
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
    LspSnapshot {
        diagnostics: report
            .diagnostics
            .iter()
            .map(|diagnostic| LspDiagnostic {
                line: diagnostic.line,
                severity: diagnostic.severity.as_str().to_owned(),
                code: diagnostic.code.clone(),
                message: diagnostic.message.clone(),
                help: diagnostic.help.clone(),
            })
            .collect(),
        completions: completion_items(report),
        hovers: report
            .semantic_program
            .hover_hints
            .iter()
            .map(|hover| LspHover {
                name: hover.name.clone(),
                line: hover.line,
                detail: hover.detail.clone(),
                quantity_kind: hover.quantity_kind.clone(),
                display_unit: hover.display_unit.clone(),
            })
            .collect(),
    }
}

pub fn snapshot_json(snapshot: &LspSnapshot) -> Value {
    json!({
        "format": LSP_SNAPSHOT_FORMAT,
        "diagnostics": snapshot.diagnostics.iter().map(diagnostic_json).collect::<Vec<_>>(),
        "completions": snapshot.completions.iter().map(completion_json).collect::<Vec<_>>(),
        "hovers": snapshot.hovers.iter().map(hover_json).collect::<Vec<_>>(),
    })
}

pub fn diagnostic_json(diagnostic: &LspDiagnostic) -> Value {
    json!({
        "range": {
            "start": { "line": diagnostic.line.saturating_sub(1), "character": 0 },
            "end": { "line": diagnostic.line.saturating_sub(1), "character": 1 }
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

pub fn hover_json(hover: &LspHover) -> Value {
    json!({
        "name": hover.name,
        "line": hover.line,
        "quantity_kind": hover.quantity_kind,
        "display_unit": hover.display_unit,
        "contents": {
            "kind": "markdown",
            "value": format!(
                "**{}**\n\n{}\n\nQuantity: `{}`\n\nDisplay unit: `{}`",
                hover.name, hover.detail, hover.quantity_kind, hover.display_unit
            )
        }
    })
}

pub fn completion_items(report: &CheckReport) -> Vec<LspCompletion> {
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();

    for keyword in [
        "schema",
        "script",
        "struct",
        "system",
        "domain",
        "across",
        "through",
        "conservation",
        "component",
        "port",
        "connect",
        "state",
        "parameter",
        "input",
        "equation",
        "promote",
        "return",
        "plot",
        "integrate",
        "train_test_split",
        "regression",
        "mlp",
        "evaluate",
        "model_card",
        "leakage_lint",
    ] {
        push_completion(&mut items, &mut seen, keyword, "keyword", "EngLang keyword");
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

pub fn completion_items_at(
    report: &CheckReport,
    source: &str,
    line: usize,
    character: usize,
) -> Vec<LspCompletion> {
    if let Some((receiver, prefix)) = member_completion_context(source, line, character) {
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
                let mut seen = BTreeSet::new();
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
    }

    completion_items(report)
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
    seen: &mut BTreeSet<String>,
    label: &str,
    kind: &str,
    detail: &str,
) {
    if seen.insert(label.to_owned()) {
        items.push(LspCompletion {
            label: label.to_owned(),
            kind: kind.to_owned(),
            detail: detail.to_owned(),
        });
    }
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
        "unit" => 3,
        _ => 1,
    }
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

    #[test]
    fn snapshot_exposes_lsp_diagnostics_hover_and_completion() {
        let source = "script main(args: Args) -> Report {\n    Q = 2 kW - 1\n}\n";
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

        let json = snapshot_json(&snapshot);
        assert_eq!(json["format"], LSP_SNAPSHOT_FORMAT);
        assert!(!json["diagnostics"].as_array().unwrap().is_empty());
    }

    #[test]
    fn member_completion_uses_csv_promotion_schema_columns() {
        let source = r#"schema SensorData {
    time: DateTime index
    T_supply: AbsoluteTemperature [degC]
    T_return: AbsoluteTemperature [degC]
    m_dot: MassFlowRate [kg/s]
}

script main() -> Report {
    sensor = promote csv "missing.csv" as SensorData
    Q = sensor.T
}
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
}
