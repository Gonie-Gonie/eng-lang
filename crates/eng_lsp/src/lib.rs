use std::collections::BTreeSet;
use std::path::Path;

use eng_compiler::{
    all_quantity_completions, all_unit_infos, bundled_module_registry, check_file, check_source,
    CheckOptions, CheckReport, ClassFieldInfo, DomainTypeParameterInfo, FunctionInfo, Severity,
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

const COMPLETION_KEYWORDS: &[&str] = &[
    "across",
    "and",
    "append",
    "args",
    "as",
    "assert",
    "bar",
    "between",
    "by",
    "check",
    "class",
    "command",
    "component",
    "connect",
    "const",
    "constraints",
    "conservation",
    "copy",
    "coverage",
    "csv",
    "delete",
    "derive",
    "der",
    "domain",
    "eq",
    "equation",
    "evaluate",
    "export",
    "false",
    "filter",
    "fn",
    "from",
    "golden",
    "grid",
    "histogram",
    "if",
    "import",
    "input",
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
    "method",
    "missing",
    "mlp",
    "mode",
    "model_card",
    "monotonic",
    "move",
    "not",
    "none",
    "null",
    "open",
    "or",
    "over",
    "package",
    "parameter",
    "plot",
    "policy",
    "port",
    "predict",
    "print",
    "promote",
    "random",
    "read",
    "regression",
    "render",
    "report",
    "return",
    "run",
    "sample",
    "schema",
    "script",
    "select",
    "select_first_row",
    "show",
    "sort",
    "sqlite",
    "state",
    "struct",
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
    "where",
    "with",
    "within",
    "write",
];

const PUBLIC_TYPE_COMPLETIONS: &[(&str, &str)] = &[
    ("Bool", "Boolean value"),
    ("CsvFile", "CSV file path"),
    ("Date", "Calendar date"),
    ("DateTime", "Timestamp value"),
    ("DbConnection", "SQLite connection handle"),
    ("DbTableRef", "SQLite table reference"),
    ("DirectoryPath", "Directory path"),
    ("Duration", "Time duration"),
    ("FilePath", "Generic file path"),
    ("Float", "Floating-point value"),
    ("Int", "Integer value"),
    ("JsonFile", "JSON file path"),
    ("ModelArtifact", "Trained model artifact"),
    ("ModelCard", "Model-card review artifact"),
    ("Number", "Dimensionless numeric value"),
    ("Optional[T]", "Optional value"),
    ("Path", "Filesystem path"),
    ("Prediction", "Prediction table row"),
    ("ProcessResult", "External command result metadata"),
    ("Report", "Report artifact request metadata"),
    ("Secret[String]", "Redacted string value"),
    ("String", "String value"),
    ("Table[T]", "Typed table value"),
    ("TextFile", "UTF-8 text file path"),
    ("TimeSeries[Time]", "Time-indexed series value"),
    ("TimeSeries[T]", "Typed time-indexed series value"),
    ("TomlFile", "TOML file path"),
    ("Url", "HTTP or HTTPS URL"),
];

const WORKFLOW_BUILTIN_COMPLETIONS: &[(&str, &str)] = &[
    ("coverage", "eng.timeseries coverage check"),
    ("duration_above", "TimeSeries threshold duration"),
    ("max", "TimeSeries maximum"),
    ("mean", "TimeSeries mean"),
    ("median", "TimeSeries median"),
    ("min", "TimeSeries minimum"),
    ("normal", "normal distribution sampling helper"),
    ("std", "TimeSeries standard deviation"),
    ("sum", "domain conservation sum"),
];

const WORKFLOW_OPTION_COMPLETIONS: &[(&str, &str)] = &[
    ("algorithm", "model training option"),
    ("allow_failure", "external command failure policy"),
    ("artifact_kind", "expected artifact kind"),
    ("cache", "cache behavior option"),
    ("cache_key", "cache identity option"),
    ("count", "sample count option"),
    ("cwd", "external command working directory"),
    ("env", "external command environment"),
    ("expected_outputs", "declared process outputs"),
    ("features", "model feature columns"),
    ("hidden", "MLP hidden layer option"),
    ("method", "fill or transform method"),
    ("recursive", "filesystem recursion option"),
    ("retry", "external command retry policy"),
    ("return_column", "projection return column"),
    ("seed", "deterministic sampling seed"),
    ("status", "case or validation status"),
    ("target", "model target column"),
    ("timeout", "external command timeout"),
    ("tool_version", "external tool version"),
];

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
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();

    for keyword in COMPLETION_KEYWORDS.iter().copied() {
        push_completion(&mut items, &mut seen, keyword, "keyword", "EngLang keyword");
    }

    for (type_name, detail) in PUBLIC_TYPE_COMPLETIONS.iter().copied() {
        push_completion(&mut items, &mut seen, type_name, "class", detail);
    }

    for (label, detail) in WORKFLOW_BUILTIN_COMPLETIONS.iter().copied() {
        push_completion(&mut items, &mut seen, label, "function", detail);
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
        ("copy file", "eng.fs copy generated output"),
        ("move file", "eng.fs move generated output"),
        ("delete file", "eng.fs delete generated output"),
        ("run command", "eng.process command boundary"),
        ("promote json config", "eng.config JSON file promotion"),
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

pub fn completion_items_at(
    report: &CheckReport,
    source: &str,
    line: usize,
    character: usize,
) -> Vec<LspCompletion> {
    if let Some(context) = object_field_completion_context(report, source, line, character) {
        if let Some(class_info) = report
            .semantic_program
            .classes
            .iter()
            .find(|class_info| class_info.name == context.class_name)
        {
            let mut seen = BTreeSet::new();
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
        if let Some(object) = report
            .semantic_program
            .class_objects
            .iter()
            .find(|object| object.name == receiver)
        {
            let mut seen = BTreeSet::new();
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

    #[test]
    fn snapshot_exposes_lsp_diagnostics_hover_and_completion() {
        let source = "Q = 2 kW - 1\n}\n";
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
            "render",
            "template",
            "open",
            "sqlite",
            "predict",
            "check",
            "coverage",
            "expected_outputs",
            "artifact_kind",
            "allow_failure",
            "cache_key",
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
            completion.label == "eng.net" && completion.detail.contains("supported_seed")
        }));
        assert!(snapshot.completions.iter().any(|completion| {
            completion.label == "eng.cache" && completion.detail.contains("supported_seed")
        }));

        let json = snapshot_json(&snapshot);
        assert_eq!(json["format"], LSP_SNAPSHOT_FORMAT);
        assert!(!json["diagnostics"].as_array().unwrap().is_empty());
        let completion_json = json["completions"].as_array().unwrap();
        assert!(completion_json
            .iter()
            .any(|completion| { completion["label"] == "kW" && completion["kind"] == 11 }));
        assert!(completion_json
            .iter()
            .any(|completion| { completion["label"] == "eng.path" && completion["kind"] == 9 }));
    }

    #[test]
    fn snapshot_exposes_domain_component_hover_and_completion() {
        let source = "domain Thermal {\n    across T: AbsoluteTemperature [degC]\n    through Q: HeatRate [kW]\n    conservation sum(Q) = 0\n}\n\ndomain Fluid[Medium M] {\n    across height: Length [m]\n    through m_dot: MassFlowRate [kg/s]\n    conservation sum(m_dot) = 0\n}\n\ncomponent RoomBoundary {\n    port heat: Thermal\n}\n\ncomponent AmbientBoundary {\n    port heat: Thermal\n}\n\ncomponent SupplyPipe {\n    port inlet: Fluid[Water]\n    port outlet: Fluid[Water]\n}\n\nconnect RoomBoundary.heat -> AmbientBoundary.heat\nconnect SupplyPipe.inlet -> SupplyPipe.outlet\n";
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
