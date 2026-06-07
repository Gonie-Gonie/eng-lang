use std::collections::BTreeSet;
use std::path::Path;

use eng_compiler::{
    all_quantity_completions, all_unit_infos, check_file, check_source, CheckOptions, CheckReport,
    DomainTypeParameterInfo, Severity,
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
    pub kind: String,
    pub line: usize,
    pub detail: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub status: Option<String>,
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
        hovers: hover_items(report),
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
                    "port {} on component {} references domain {} (base {}, arguments {})",
                    port.name,
                    component.name,
                    port.domain,
                    port.domain_name,
                    string_list(&port.type_arguments)
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

    hovers.sort_by(|left, right| {
        left.line
            .cmp(&right.line)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.name.cmp(&right.name))
    });
    hovers
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
        "package",
        "version",
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
    fn snapshot_exposes_domain_component_hover_and_completion() {
        let source = "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ncomponent RoomBoundary {\n    port heat: Thermal\n}\n\ncomponent AmbientBoundary {\n    port heat: Thermal\n}\n\nconnect RoomBoundary.heat -> AmbientBoundary.heat\n";
        let snapshot = snapshot_for_source(Path::new("domain.eng"), source);

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
