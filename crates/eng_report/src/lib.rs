use eng_compiler::{CheckReport, Severity};

pub const REPORT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REPORT_SPEC_VERSION: u32 = 1;
pub const PLOT_SPEC_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq)]
pub struct PlotSpec {
    pub title: String,
    pub plot_type: String,
    pub x_axis: PlotAxis,
    pub y_axis: PlotAxis,
    pub series: Vec<PlotSeries>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlotAxis {
    pub name: String,
    pub label: String,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlotSeries {
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub points: Vec<PlotPoint>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlotPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportSpec {
    pub source_path: String,
    pub source_hash: String,
    pub compiler_version: String,
    pub report_version: String,
    pub variables: Vec<ReportVariable>,
    pub inferred_declarations: Vec<ReportInferredDeclaration>,
    pub unit_conversions: Vec<ReportUnitConversion>,
    pub schemas: Vec<ReportSchemaSummary>,
    pub args: Vec<ReportArgsStruct>,
    pub computed_statistics: Vec<ReportComputedStatistics>,
    pub computed_integrations: Vec<ReportComputedIntegration>,
    pub policy_results: Vec<ReportPolicyResult>,
    pub systems: Vec<ReportSystemSummary>,
    pub plot_manifest: ReportPlotManifest,
    pub warnings: Vec<ReportWarning>,
    pub provenance: ReportProvenance,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportVariable {
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub dimension: String,
    pub source: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportInferredDeclaration {
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub expression: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportUnitConversion {
    pub name: String,
    pub quantity_kind: String,
    pub source_unit: Option<String>,
    pub display_unit: String,
    pub canonical_unit: String,
    pub steps: Vec<String>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportSchemaSummary {
    pub name: String,
    pub columns: Vec<String>,
    pub column_count: usize,
    pub constraint_count: usize,
    pub missing_policy_count: usize,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportArgsStruct {
    pub name: String,
    pub fields: Vec<ReportArgsField>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportArgsField {
    pub name: String,
    pub type_name: String,
    pub default_value: Option<String>,
    pub required: bool,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComputedStatistics {
    pub source: String,
    pub quantity_kind: String,
    pub axis: String,
    pub status: String,
    pub values: Vec<ReportComputedStatisticValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComputedStatisticValue {
    pub name: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportComputedIntegration {
    pub binding: String,
    pub source: String,
    pub input_quantity: String,
    pub over_axis: String,
    pub result_quantity: String,
    pub value: f64,
    pub unit: String,
    pub method: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportPolicyResult {
    pub schema: String,
    pub binding: String,
    pub kind: String,
    pub target: String,
    pub policy: String,
    pub status: String,
    pub checked_rows: usize,
    pub violation_count: usize,
    pub violations: Vec<ReportPolicyViolation>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportPolicyViolation {
    pub row: usize,
    pub column: String,
    pub value: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportSystemSummary {
    pub name: String,
    pub variables: Vec<ReportSystemVariable>,
    pub equations: Vec<ReportEquation>,
    pub residuals: Vec<ReportResidual>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportSystemVariable {
    pub role: String,
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub dimension: String,
    pub initial_value: Option<String>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportEquation {
    pub left: String,
    pub relation: String,
    pub right: String,
    pub left_dimension: String,
    pub right_dimension: String,
    pub residual: String,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportResidual {
    pub name: String,
    pub expression: String,
    pub dimension: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportPlotManifest {
    pub path: String,
    pub hash: String,
    pub format: String,
    pub plot_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportWarning {
    pub code: String,
    pub message: String,
    pub help: Option<String>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportProvenance {
    pub syntax_items: usize,
    pub schema_count: usize,
    pub csv_promotion_count: usize,
    pub system_count: usize,
    pub equation_count: usize,
    pub residual_count: usize,
    pub plot_spec_version: u32,
}

pub fn report_spec_from_report(
    report: &CheckReport,
    plot_manifest_relative_path: &str,
    plot_manifest_hash: &str,
) -> ReportSpec {
    let variables = report
        .semantic_program
        .typed_bindings
        .iter()
        .map(|binding| {
            let type_info = report
                .semantic_program
                .type_infos
                .iter()
                .find(|info| info.name == binding.name && info.line == binding.line);
            ReportVariable {
                name: binding.name.clone(),
                quantity_kind: binding.semantic_type.quantity_kind.clone(),
                display_unit: binding.semantic_type.display_unit.clone(),
                canonical_unit: type_info
                    .map(|info| info.canonical_unit.clone())
                    .unwrap_or_else(|| "unknown".to_owned()),
                dimension: type_info
                    .map(|info| info.dimension.clone())
                    .unwrap_or_else(|| "unknown".to_owned()),
                source: type_info
                    .map(|info| info.source.as_str().to_owned())
                    .unwrap_or_else(|| "runtime".to_owned()),
                line: binding.line,
            }
        })
        .collect();

    let inferred_declarations = report
        .inferred_declarations
        .iter()
        .map(|declaration| ReportInferredDeclaration {
            name: declaration.name.clone(),
            quantity_kind: declaration.quantity_kind.clone(),
            display_unit: declaration.display_unit.clone(),
            expression: declaration.expression.clone(),
            line: declaration.line,
        })
        .collect();

    let unit_conversions = report
        .semantic_program
        .unit_derivations
        .iter()
        .map(|derivation| ReportUnitConversion {
            name: derivation.name.clone(),
            quantity_kind: derivation.quantity_kind.clone(),
            source_unit: derivation.source_unit.clone(),
            display_unit: derivation.display_unit.clone(),
            canonical_unit: derivation.canonical_unit.clone(),
            steps: derivation.steps.clone(),
            line: derivation.line,
        })
        .collect();

    let schemas = report
        .semantic_program
        .schemas
        .iter()
        .map(|schema| ReportSchemaSummary {
            name: schema.name.clone(),
            columns: schema
                .columns
                .iter()
                .map(|column| column.name.clone())
                .collect(),
            column_count: schema.columns.len(),
            constraint_count: schema.constraints.len(),
            missing_policy_count: schema.missing_policies.len(),
            line: schema.line,
        })
        .collect();

    let args = report
        .semantic_program
        .args_structs
        .iter()
        .map(|args_struct| ReportArgsStruct {
            name: args_struct.name.clone(),
            fields: args_struct
                .fields
                .iter()
                .map(|field| ReportArgsField {
                    name: field.name.clone(),
                    type_name: field.type_name.clone(),
                    default_value: field.default_value.clone(),
                    required: field.required,
                    line: field.line,
                })
                .collect(),
            line: args_struct.line,
        })
        .collect();

    let warnings = report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .map(|diagnostic| ReportWarning {
            code: diagnostic.code.clone(),
            message: diagnostic.message.clone(),
            help: diagnostic.help.clone(),
            line: diagnostic.line,
        })
        .collect();

    let systems = report
        .semantic_program
        .systems
        .iter()
        .map(|system| ReportSystemSummary {
            name: system.name.clone(),
            variables: system
                .variables
                .iter()
                .map(|variable| ReportSystemVariable {
                    role: variable.role.clone(),
                    name: variable.name.clone(),
                    quantity_kind: variable.quantity_kind.clone(),
                    display_unit: variable.display_unit.clone(),
                    dimension: variable.dimension.clone(),
                    initial_value: variable.initial_value.clone(),
                    line: variable.line,
                })
                .collect(),
            equations: system
                .equations
                .iter()
                .map(|equation| ReportEquation {
                    left: equation.left.clone(),
                    relation: equation.relation.clone(),
                    right: equation.right.clone(),
                    left_dimension: equation.left_dimension.clone(),
                    right_dimension: equation.right_dimension.clone(),
                    residual: equation.residual.clone(),
                    status: equation.status.clone(),
                    line: equation.line,
                })
                .collect(),
            residuals: system
                .residuals
                .iter()
                .map(|residual| ReportResidual {
                    name: residual.name.clone(),
                    expression: residual.expression.clone(),
                    dimension: residual.dimension.clone(),
                    line: residual.line,
                })
                .collect(),
            line: system.line,
        })
        .collect::<Vec<_>>();
    let equation_count = systems
        .iter()
        .map(|system| system.equations.len())
        .sum::<usize>();
    let residual_count = systems
        .iter()
        .map(|system| system.residuals.len())
        .sum::<usize>();

    ReportSpec {
        source_path: report.source_path.display().to_string(),
        source_hash: report.source_hash.clone(),
        compiler_version: eng_compiler::COMPILER_VERSION.to_owned(),
        report_version: REPORT_VERSION.to_owned(),
        variables,
        inferred_declarations,
        unit_conversions,
        schemas,
        args,
        computed_statistics: Vec::new(),
        computed_integrations: Vec::new(),
        policy_results: Vec::new(),
        systems,
        plot_manifest: ReportPlotManifest {
            path: plot_manifest_relative_path.to_owned(),
            hash: plot_manifest_hash.to_owned(),
            format: "eng-plot-manifest-v1".to_owned(),
            plot_count: 1,
        },
        warnings,
        provenance: ReportProvenance {
            syntax_items: report.syntax_summary.ast_items,
            schema_count: report.semantic_program.schemas.len(),
            csv_promotion_count: report.semantic_program.csv_promotions.len(),
            system_count: report.semantic_program.systems.len(),
            equation_count,
            residual_count,
            plot_spec_version: PLOT_SPEC_VERSION,
        },
    }
}

pub fn report_spec_json(spec: &ReportSpec) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"format\": \"eng-report-spec-v1\",\n");
    json.push_str(&format!(
        "  \"report_schema_version\": {REPORT_SPEC_VERSION},\n"
    ));
    json.push_str(&format!(
        "  \"compiler_version\": \"{}\",\n",
        json_escape(&spec.compiler_version)
    ));
    json.push_str(&format!(
        "  \"report_version\": \"{}\",\n",
        json_escape(&spec.report_version)
    ));
    json.push_str(&format!(
        "  \"source_path\": \"{}\",\n",
        json_escape(&spec.source_path)
    ));
    json.push_str(&format!(
        "  \"source_hash\": \"{}\",\n",
        json_escape(&spec.source_hash)
    ));
    json.push_str("  \"provenance\": {\n");
    json.push_str(&format!(
        "    \"syntax_items\": {},\n",
        spec.provenance.syntax_items
    ));
    json.push_str(&format!(
        "    \"schema_count\": {},\n",
        spec.provenance.schema_count
    ));
    json.push_str(&format!(
        "    \"csv_promotion_count\": {},\n",
        spec.provenance.csv_promotion_count
    ));
    json.push_str(&format!(
        "    \"system_count\": {},\n",
        spec.provenance.system_count
    ));
    json.push_str(&format!(
        "    \"equation_count\": {},\n",
        spec.provenance.equation_count
    ));
    json.push_str(&format!(
        "    \"residual_count\": {},\n",
        spec.provenance.residual_count
    ));
    json.push_str(&format!(
        "    \"plot_spec_version\": {}\n",
        spec.provenance.plot_spec_version
    ));
    json.push_str("  },\n");

    json.push_str("  \"variable_table\": [\n");
    for (index, variable) in spec.variables.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&variable.name)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&variable.quantity_kind)
        ));
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&variable.display_unit)
        ));
        json.push_str(&format!(
            "      \"canonical_unit\": \"{}\",\n",
            json_escape(&variable.canonical_unit)
        ));
        json.push_str(&format!(
            "      \"dimension\": \"{}\",\n",
            json_escape(&variable.dimension)
        ));
        json.push_str(&format!(
            "      \"source\": \"{}\",\n",
            json_escape(&variable.source)
        ));
        json.push_str(&format!("      \"line\": {}\n", variable.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"inferred_declaration_table\": [\n");
    for (index, declaration) in spec.inferred_declarations.iter().enumerate() {
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
        json.push_str(&format!(
            "      \"expression\": \"{}\",\n",
            json_escape(&declaration.expression)
        ));
        json.push_str(&format!("      \"line\": {}\n", declaration.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"unit_conversion_table\": [\n");
    for (index, conversion) in spec.unit_conversions.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&conversion.name)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&conversion.quantity_kind)
        ));
        if let Some(source_unit) = &conversion.source_unit {
            json.push_str(&format!(
                "      \"source_unit\": \"{}\",\n",
                json_escape(source_unit)
            ));
        } else {
            json.push_str("      \"source_unit\": null,\n");
        }
        json.push_str(&format!(
            "      \"display_unit\": \"{}\",\n",
            json_escape(&conversion.display_unit)
        ));
        json.push_str(&format!(
            "      \"canonical_unit\": \"{}\",\n",
            json_escape(&conversion.canonical_unit)
        ));
        json.push_str(&format!("      \"line\": {},\n", conversion.line));
        json.push_str("      \"steps\": [");
        push_json_string_array(&mut json, &conversion.steps);
        json.push_str("]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"schema_summary\": [\n");
    for (index, schema) in spec.schemas.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&schema.name)
        ));
        json.push_str(&format!("      \"line\": {},\n", schema.line));
        json.push_str("      \"columns\": [");
        push_json_string_array(&mut json, &schema.columns);
        json.push_str("],\n");
        json.push_str(&format!(
            "      \"column_count\": {},\n",
            schema.column_count
        ));
        json.push_str(&format!(
            "      \"constraint_count\": {},\n",
            schema.constraint_count
        ));
        json.push_str(&format!(
            "      \"missing_policy_count\": {}\n",
            schema.missing_policy_count
        ));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"args_summary\": [\n");
    for (index, args_struct) in spec.args.iter().enumerate() {
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

    json.push_str("  \"computed_statistics\": [\n");
    for (index, summary) in spec.computed_statistics.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"source\": \"{}\",\n",
            json_escape(&summary.source)
        ));
        json.push_str(&format!(
            "      \"quantity_kind\": \"{}\",\n",
            json_escape(&summary.quantity_kind)
        ));
        json.push_str(&format!(
            "      \"axis\": \"{}\",\n",
            json_escape(&summary.axis)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&summary.status)
        ));
        json.push_str("      \"values\": [\n");
        for (value_index, value) in summary.values.iter().enumerate() {
            if value_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"name\": \"{}\",\n",
                json_escape(&value.name)
            ));
            json.push_str(&format!("          \"value\": {},\n", value.value));
            json.push_str(&format!(
                "          \"unit\": \"{}\"\n",
                json_escape(&value.unit)
            ));
            json.push_str("        }");
        }
        json.push_str("\n      ]\n");
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"computed_integrations\": [\n");
    for (index, integration) in spec.computed_integrations.iter().enumerate() {
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
        json.push_str(&format!("      \"value\": {},\n", integration.value));
        json.push_str(&format!(
            "      \"unit\": \"{}\",\n",
            json_escape(&integration.unit)
        ));
        json.push_str(&format!(
            "      \"method\": \"{}\",\n",
            json_escape(&integration.method)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\"\n",
            json_escape(&integration.status)
        ));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"policy_results\": [\n");
    for (index, policy) in spec.policy_results.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"schema\": \"{}\",\n",
            json_escape(&policy.schema)
        ));
        json.push_str(&format!(
            "      \"binding\": \"{}\",\n",
            json_escape(&policy.binding)
        ));
        json.push_str(&format!(
            "      \"kind\": \"{}\",\n",
            json_escape(&policy.kind)
        ));
        json.push_str(&format!(
            "      \"target\": \"{}\",\n",
            json_escape(&policy.target)
        ));
        json.push_str(&format!(
            "      \"policy\": \"{}\",\n",
            json_escape(&policy.policy)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&policy.status)
        ));
        json.push_str(&format!(
            "      \"checked_rows\": {},\n",
            policy.checked_rows
        ));
        json.push_str(&format!(
            "      \"violation_count\": {},\n",
            policy.violation_count
        ));
        json.push_str("      \"violations\": [\n");
        for (violation_index, violation) in policy.violations.iter().enumerate() {
            if violation_index > 0 {
                json.push_str(",\n");
            }
            json.push_str("        {\n");
            json.push_str(&format!("          \"row\": {},\n", violation.row));
            json.push_str(&format!(
                "          \"column\": \"{}\",\n",
                json_escape(&violation.column)
            ));
            json.push_str(&format!(
                "          \"value\": \"{}\",\n",
                json_escape(&violation.value)
            ));
            json.push_str(&format!(
                "          \"message\": \"{}\"\n",
                json_escape(&violation.message)
            ));
            json.push_str("        }");
        }
        json.push_str("\n      ],\n");
        json.push_str(&format!("      \"line\": {}\n", policy.line));
        json.push_str("    }");
    }
    json.push_str("\n  ],\n");

    json.push_str("  \"system_summary\": [\n");
    for (index, system) in spec.systems.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&system.name)
        ));
        json.push_str(&format!("      \"line\": {},\n", system.line));
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
            if let Some(initial_value) = &variable.initial_value {
                json.push_str(&format!(
                    "          \"initial_value\": \"{}\",\n",
                    json_escape(initial_value)
                ));
            } else {
                json.push_str("          \"initial_value\": null,\n");
            }
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

    json.push_str("  \"plot_manifest\": {\n");
    json.push_str(&format!(
        "    \"path\": \"{}\",\n",
        json_escape(&spec.plot_manifest.path)
    ));
    json.push_str(&format!(
        "    \"hash\": \"{}\",\n",
        json_escape(&spec.plot_manifest.hash)
    ));
    json.push_str(&format!(
        "    \"format\": \"{}\",\n",
        json_escape(&spec.plot_manifest.format)
    ));
    json.push_str(&format!(
        "    \"plot_count\": {}\n",
        spec.plot_manifest.plot_count
    ));
    json.push_str("  },\n");

    json.push_str("  \"warning_list\": [\n");
    for (index, warning) in spec.warnings.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"code\": \"{}\",\n",
            json_escape(&warning.code)
        ));
        json.push_str(&format!(
            "      \"message\": \"{}\",\n",
            json_escape(&warning.message)
        ));
        if let Some(help) = &warning.help {
            json.push_str(&format!("      \"help\": \"{}\",\n", json_escape(help)));
        } else {
            json.push_str("      \"help\": null,\n");
        }
        json.push_str(&format!("      \"line\": {}\n", warning.line));
        json.push_str("    }");
    }
    json.push_str("\n  ]\n");
    json.push_str("}\n");
    json
}

pub fn plot_spec_from_report(report: &CheckReport) -> PlotSpec {
    let series_binding = report
        .semantic_program
        .typed_bindings
        .iter()
        .find_map(|binding| {
            time_series_quantity(&binding.semantic_type.quantity_kind).map(|(axis, quantity)| {
                (
                    binding.name.clone(),
                    axis,
                    quantity,
                    binding.semantic_type.display_unit.clone(),
                )
            })
        });

    let (name, axis, quantity, unit) = series_binding.unwrap_or_else(|| {
        (
            "preview".to_owned(),
            "Time".to_owned(),
            "Value".to_owned(),
            "unit".to_owned(),
        )
    });

    PlotSpec {
        title: if name == "preview" {
            "EngLang preview plot".to_owned()
        } else {
            format!("{name} over {axis}")
        },
        plot_type: "line".to_owned(),
        x_axis: PlotAxis {
            name: axis.clone(),
            label: axis,
            unit: "sample".to_owned(),
        },
        y_axis: PlotAxis {
            name: quantity.clone(),
            label: quantity,
            unit: unit.clone(),
        },
        series: vec![PlotSeries {
            name,
            quantity_kind: "TimeSeries".to_owned(),
            display_unit: unit,
            points: preview_points(),
        }],
    }
}

pub fn render_svg(title: &str) -> String {
    render_svg_from_spec(&default_plot_spec(title))
}

pub fn render_svg_from_spec(spec: &PlotSpec) -> String {
    let title = xml_escape(&spec.title);
    let x_label = xml_escape(&axis_label(&spec.x_axis));
    let y_label = xml_escape(&axis_label(&spec.y_axis));
    let points = spec
        .series
        .first()
        .map(|series| svg_points(&series.points))
        .unwrap_or_default();
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="720" height="320" viewBox="0 0 720 320" role="img" aria-label="{title}">
  <rect width="720" height="320" fill="#f7f8fb"/>
  <line x1="72" y1="250" x2="660" y2="250" stroke="#222" stroke-width="2"/>
  <line x1="72" y1="40" x2="72" y2="250" stroke="#222" stroke-width="2"/>
  <polyline points="{points}" fill="none" stroke="#0b6bcb" stroke-width="4"/>
  <text x="72" y="26" font-family="Segoe UI, Arial, sans-serif" font-size="20" fill="#111">{title}</text>
  <text x="328" y="294" font-family="Segoe UI, Arial, sans-serif" font-size="14" fill="#333">{x_label}</text>
  <text x="18" y="156" transform="rotate(-90 18 156)" font-family="Segoe UI, Arial, sans-serif" font-size="14" fill="#333">{y_label}</text>
</svg>
"##
    )
}

pub fn plot_spec_json(spec: &PlotSpec) -> String {
    let mut points = String::new();
    for (index, point) in spec
        .series
        .first()
        .map(|series| series.points.as_slice())
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        if index > 0 {
            points.push_str(", ");
        }
        points.push_str(&format!("[{}, {}]", point.x, point.y));
    }

    let series = spec.series.first();
    format!(
        "{{\n  \"format\": \"eng-plotspec-v1\",\n  \"plot_spec_version\": {PLOT_SPEC_VERSION},\n  \"plot_type\": \"{}\",\n  \"title\": \"{}\",\n  \"x_axis\": {{ \"name\": \"{}\", \"label\": \"{}\", \"unit\": \"{}\" }},\n  \"y_axis\": {{ \"name\": \"{}\", \"label\": \"{}\", \"unit\": \"{}\" }},\n  \"series\": [\n    {{\n      \"name\": \"{}\",\n      \"quantity_kind\": \"{}\",\n      \"display_unit\": \"{}\",\n      \"points\": [{}]\n    }}\n  ]\n}}\n",
        json_escape(&spec.plot_type),
        json_escape(&spec.title),
        json_escape(&spec.x_axis.name),
        json_escape(&spec.x_axis.label),
        json_escape(&spec.x_axis.unit),
        json_escape(&spec.y_axis.name),
        json_escape(&spec.y_axis.label),
        json_escape(&spec.y_axis.unit),
        json_escape(series.map(|series| series.name.as_str()).unwrap_or("preview")),
        json_escape(
            series
                .map(|series| series.quantity_kind.as_str())
                .unwrap_or("Value")
        ),
        json_escape(
            series
                .map(|series| series.display_unit.as_str())
                .unwrap_or("unit")
        ),
        points
    )
}

pub fn plot_manifest_json(
    spec: &PlotSpec,
    svg_relative_path: &str,
    plot_spec_hash: &str,
    svg_hash: &str,
) -> String {
    format!(
        "{{\n  \"format\": \"eng-plot-manifest-v1\",\n  \"plot_spec_version\": {PLOT_SPEC_VERSION},\n  \"plots\": [\n    {{\n      \"title\": \"{}\",\n      \"plot_type\": \"{}\",\n      \"plot_spec\": \"plot_spec.json\",\n      \"plot_spec_hash\": \"{}\",\n      \"svg\": \"{}\",\n      \"svg_hash\": \"{}\",\n      \"x_axis_label\": \"{}\",\n      \"y_axis_label\": \"{}\"\n    }}\n  ]\n}}\n",
        json_escape(&spec.title),
        json_escape(&spec.plot_type),
        json_escape(plot_spec_hash),
        json_escape(svg_relative_path),
        json_escape(svg_hash),
        json_escape(&axis_label(&spec.x_axis)),
        json_escape(&axis_label(&spec.y_axis))
    )
}

pub fn render_html(report: &CheckReport, plot_relative_path: &str) -> String {
    let title = html_escape(&format!(
        "EngLang Review - {}",
        report
            .source_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("source.eng")
    ));
    let mut diagnostics = String::new();
    for diagnostic in &report.diagnostics {
        diagnostics.push_str("<tr>");
        diagnostics.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            diagnostic.line,
            html_escape(diagnostic.severity.as_str()),
            html_escape(&diagnostic.code),
            html_escape(&diagnostic.message)
        ));
        diagnostics.push_str("</tr>");
    }
    if diagnostics.is_empty() {
        diagnostics.push_str("<tr><td colspan=\"4\">No diagnostics.</td></tr>");
    }

    let mut inferred = String::new();
    for declaration in &report.inferred_declarations {
        inferred.push_str("<tr>");
        inferred.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td>",
            declaration.line,
            html_escape(&declaration.name),
            html_escape(&declaration.quantity_kind),
            html_escape(&declaration.display_unit),
            html_escape(&declaration.expression)
        ));
        inferred.push_str("</tr>");
    }
    if inferred.is_empty() {
        inferred.push_str("<tr><td colspan=\"5\">No inferred local declarations.</td></tr>");
    }

    let mut hover_hints = String::new();
    for hover in &report.semantic_program.hover_hints {
        hover_hints.push_str("<tr>");
        hover_hints.push_str(&format!(
            "<td>{}:{}</td><td>{}</td><td>{}</td><td>{}</td>",
            hover.line,
            hover.column,
            html_escape(&hover.name),
            html_escape(&hover.quantity_kind),
            html_escape(&hover.detail)
        ));
        hover_hints.push_str("</tr>");
    }
    if hover_hints.is_empty() {
        hover_hints.push_str("<tr><td colspan=\"4\">No hover hints.</td></tr>");
    }

    let mut type_info = String::new();
    for info in &report.semantic_program.type_infos {
        type_info.push_str("<tr>");
        type_info.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            info.line,
            html_escape(&info.name),
            html_escape(&info.quantity_kind),
            html_escape(&info.display_unit),
            html_escape(&info.canonical_unit),
            html_escape(&info.dimension)
        ));
        type_info.push_str("</tr>");
    }
    if type_info.is_empty() {
        type_info.push_str("<tr><td colspan=\"6\">No type info.</td></tr>");
    }

    let mut unit_derivations = String::new();
    for derivation in &report.semantic_program.unit_derivations {
        unit_derivations.push_str("<tr>");
        unit_derivations.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            derivation.line,
            html_escape(&derivation.name),
            html_escape(derivation.source_unit.as_deref().unwrap_or("not detected")),
            html_escape(&derivation.display_unit),
            html_escape(&derivation.canonical_unit)
        ));
        unit_derivations.push_str("</tr>");
    }
    if unit_derivations.is_empty() {
        unit_derivations.push_str("<tr><td colspan=\"5\">No unit derivations.</td></tr>");
    }

    let mut axis_info = String::new();
    for axis in &report.semantic_program.axis_infos {
        axis_info.push_str("<tr>");
        axis_info.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            axis.line,
            html_escape(&axis.binding),
            html_escape(&axis.axis),
            html_escape(&axis.role),
            html_escape(&axis.source)
        ));
        axis_info.push_str("</tr>");
    }
    if axis_info.is_empty() {
        axis_info.push_str("<tr><td colspan=\"5\">No axis metadata.</td></tr>");
    }

    let mut stats_info = String::new();
    for stats in &report.semantic_program.stats_infos {
        stats_info.push_str("<tr>");
        stats_info.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            stats.line,
            html_escape(&stats.source),
            html_escape(&stats.quantity_kind),
            html_escape(&stats.axis),
            html_escape(&stats.statistics.join(", ")),
            html_escape(&stats.cache_key)
        ));
        stats_info.push_str("</tr>");
    }
    if stats_info.is_empty() {
        stats_info.push_str("<tr><td colspan=\"6\">No statistics summaries.</td></tr>");
    }

    let mut integrations = String::new();
    for integration in &report.semantic_program.integrations {
        integrations.push_str("<tr>");
        integrations.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            integration.line,
            html_escape(&integration.binding),
            html_escape(&integration.source),
            html_escape(&integration.input_quantity),
            html_escape(&integration.over_axis),
            html_escape(&integration.result_quantity)
        ));
        integrations.push_str("</tr>");
    }
    if integrations.is_empty() {
        integrations.push_str("<tr><td colspan=\"6\">No integrations.</td></tr>");
    }

    let mut system_equations = String::new();
    for system in &report.semantic_program.systems {
        for equation in &system.equations {
            system_equations.push_str("<tr>");
            system_equations.push_str(&format!(
                "<td>{}</td><td>{}</td><td><code>{} {} {}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
                equation.line,
                html_escape(&system.name),
                html_escape(&equation.left),
                html_escape(&equation.relation),
                html_escape(&equation.right),
                html_escape(&equation.left_dimension),
                html_escape(&equation.right_dimension),
                html_escape(&equation.residual),
                html_escape(&equation.status)
            ));
            system_equations.push_str("</tr>");
        }
    }
    if system_equations.is_empty() {
        system_equations.push_str("<tr><td colspan=\"7\">No system equations.</td></tr>");
    }

    let mut schemas = String::new();
    for schema in &report.semantic_program.schemas {
        schemas.push_str("<tr>");
        schemas.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            schema.line,
            html_escape(&schema.name),
            schema.columns.len(),
            schema.constraints.len(),
            schema.missing_policies.len()
        ));
        schemas.push_str("</tr>");
    }
    if schemas.is_empty() {
        schemas.push_str("<tr><td colspan=\"5\">No schemas.</td></tr>");
    }

    let mut csv_promotions = String::new();
    for promotion in &report.semantic_program.csv_promotions {
        csv_promotions.push_str("<tr>");
        csv_promotions.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            promotion.line,
            html_escape(&promotion.binding),
            html_escape(&promotion.schema_name),
            html_escape(&promotion.source_literal),
            promotion.row_count,
            html_escape(promotion.source_hash.as_deref().unwrap_or("not available"))
        ));
        csv_promotions.push_str("</tr>");
    }
    if csv_promotions.is_empty() {
        csv_promotions.push_str("<tr><td colspan=\"6\">No CSV promotions.</td></tr>");
    }

    let mut entry_points = String::new();
    for entry in &report.semantic_program.entry_points {
        entry_points.push_str("<tr>");
        entry_points.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            entry.line,
            html_escape(&entry.kind),
            html_escape(&entry.name),
            html_escape(entry.arg_type.as_deref().unwrap_or("Args")),
            html_escape(entry.return_type.as_deref().unwrap_or("Report"))
        ));
        entry_points.push_str("</tr>");
    }
    if entry_points.is_empty() {
        entry_points.push_str("<tr><td colspan=\"5\">No entry points.</td></tr>");
    }

    let mut args_metadata = String::new();
    for args_struct in &report.semantic_program.args_structs {
        if args_struct.fields.is_empty() {
            args_metadata.push_str("<tr>");
            args_metadata.push_str(&format!(
                "<td>{}</td><td>{}</td><td colspan=\"4\">No fields.</td>",
                args_struct.line,
                html_escape(&args_struct.name)
            ));
            args_metadata.push_str("</tr>");
            continue;
        }
        for field in &args_struct.fields {
            args_metadata.push_str("<tr>");
            args_metadata.push_str(&format!(
                "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
                field.line,
                html_escape(&args_struct.name),
                html_escape(&field.name),
                html_escape(&field.type_name),
                html_escape(field.default_value.as_deref().unwrap_or("")),
                if field.required { "yes" } else { "no" }
            ));
            args_metadata.push_str("</tr>");
        }
    }
    if args_metadata.is_empty() {
        args_metadata.push_str("<tr><td colspan=\"6\">No Args metadata.</td></tr>");
    }

    let error_count = report.diagnostic_count(Severity::Error);
    let warning_count = report.diagnostic_count(Severity::Warning);
    let syntax_items = report.syntax_summary.ast_items;
    let typed_bindings = report.semantic_program.typed_bindings.len();
    let expected_types = report.semantic_program.expected_types.len();
    let hover_count = report.semantic_program.hover_hints.len();
    let quantity_completion_count = report.quantity_completion_count;
    let unit_info_count = report.unit_info_count;
    let type_info_count = report.semantic_program.type_infos.len();
    let unit_derivation_count = report.semantic_program.unit_derivations.len();
    let axis_info_count = report.semantic_program.axis_infos.len();
    let stats_info_count = report.semantic_program.stats_infos.len();
    let integration_count = report.semantic_program.integrations.len();
    let system_count = report.semantic_program.systems.len();
    let equation_count = report
        .semantic_program
        .systems
        .iter()
        .map(|system| system.equations.len())
        .sum::<usize>();
    let residual_count = report
        .semantic_program
        .systems
        .iter()
        .map(|system| system.residuals.len())
        .sum::<usize>();
    let schema_count = report.semantic_program.schemas.len();
    let csv_promotion_count = report.semantic_program.csv_promotions.len();
    let entry_point_count = report.semantic_program.entry_points.len();
    let plot_relative_path = html_escape(plot_relative_path);

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <style>
    :root {{
      color-scheme: light;
      font-family: "Segoe UI", Arial, sans-serif;
      background: #f5f6f8;
      color: #20242a;
    }}
    body {{
      margin: 0;
      padding: 32px;
    }}
    main {{
      max-width: 1040px;
      margin: 0 auto;
    }}
    h1, h2 {{
      letter-spacing: 0;
    }}
    h1 {{
      margin: 0 0 8px;
      font-size: 28px;
    }}
    h2 {{
      margin-top: 28px;
      font-size: 20px;
    }}
    .summary {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: 12px;
      margin: 24px 0;
    }}
    .metric {{
      border: 1px solid #d9dee7;
      border-radius: 8px;
      padding: 14px;
      background: #fff;
    }}
    .metric strong {{
      display: block;
      font-size: 24px;
    }}
    table {{
      width: 100%;
      border-collapse: collapse;
      background: #fff;
      border: 1px solid #d9dee7;
    }}
    th, td {{
      text-align: left;
      border-bottom: 1px solid #e7ebf0;
      padding: 10px 12px;
      vertical-align: top;
    }}
    th {{
      background: #eef2f7;
      font-weight: 600;
    }}
    code {{
      font-family: Consolas, "SFMono-Regular", monospace;
    }}
    .plot {{
      width: 100%;
      min-height: 320px;
      border: 1px solid #d9dee7;
      border-radius: 8px;
      background: #fff;
    }}
  </style>
</head>
<body>
  <main>
    <h1>{title}</h1>
    <p>Reviewable EngLang preview artifact with source hash <code>{source_hash}</code>.</p>
    <section class="summary" aria-label="Run summary">
      <div class="metric"><span>Errors</span><strong>{error_count}</strong></div>
      <div class="metric"><span>Warnings</span><strong>{warning_count}</strong></div>
      <div class="metric"><span>AST Items</span><strong>{syntax_items}</strong></div>
      <div class="metric"><span>Typed Bindings</span><strong>{typed_bindings}</strong></div>
      <div class="metric"><span>Expected Types</span><strong>{expected_types}</strong></div>
      <div class="metric"><span>Hover Hints</span><strong>{hover_count}</strong></div>
      <div class="metric"><span>Quantity Completions</span><strong>{quantity_completion_count}</strong></div>
      <div class="metric"><span>Unit Infos</span><strong>{unit_info_count}</strong></div>
      <div class="metric"><span>Type Info</span><strong>{type_info_count}</strong></div>
      <div class="metric"><span>Unit Derivations</span><strong>{unit_derivation_count}</strong></div>
      <div class="metric"><span>Axis Info</span><strong>{axis_info_count}</strong></div>
      <div class="metric"><span>Stats Info</span><strong>{stats_info_count}</strong></div>
      <div class="metric"><span>Integrations</span><strong>{integration_count}</strong></div>
      <div class="metric"><span>Systems</span><strong>{system_count}</strong></div>
      <div class="metric"><span>Equations</span><strong>{equation_count}</strong></div>
      <div class="metric"><span>Residuals</span><strong>{residual_count}</strong></div>
      <div class="metric"><span>Schemas</span><strong>{schema_count}</strong></div>
      <div class="metric"><span>CSV Promotions</span><strong>{csv_promotion_count}</strong></div>
      <div class="metric"><span>Entry Points</span><strong>{entry_point_count}</strong></div>
      <div class="metric"><span>Compiler</span><strong>{compiler_version}</strong></div>
      <div class="metric"><span>Report</span><strong>{report_version}</strong></div>
    </section>
    <h2>Entry Points</h2>
    <table>
      <thead><tr><th>Line</th><th>Kind</th><th>Name</th><th>Args</th><th>Returns</th></tr></thead>
      <tbody>{entry_points}</tbody>
    </table>
    <h2>Args Metadata</h2>
    <table>
      <thead><tr><th>Line</th><th>Struct</th><th>Field</th><th>Type</th><th>Default</th><th>Required</th></tr></thead>
      <tbody>{args_metadata}</tbody>
    </table>
    <h2>Inferred Declarations</h2>
    <table>
      <thead><tr><th>Line</th><th>Name</th><th>Quantity</th><th>Display Unit</th><th>Expression</th></tr></thead>
      <tbody>{inferred}</tbody>
    </table>
    <h2>Hover Hints</h2>
    <table>
      <thead><tr><th>Position</th><th>Name</th><th>Quantity</th><th>Detail</th></tr></thead>
      <tbody>{hover_hints}</tbody>
    </table>
    <h2>Type Info</h2>
    <table>
      <thead><tr><th>Line</th><th>Name</th><th>Quantity</th><th>Display Unit</th><th>Canonical Unit</th><th>Dimension</th></tr></thead>
      <tbody>{type_info}</tbody>
    </table>
    <h2>Unit Derivations</h2>
    <table>
      <thead><tr><th>Line</th><th>Name</th><th>Source Unit</th><th>Display Unit</th><th>Canonical Unit</th></tr></thead>
      <tbody>{unit_derivations}</tbody>
    </table>
    <h2>Axis Info</h2>
    <table>
      <thead><tr><th>Line</th><th>Binding</th><th>Axis</th><th>Role</th><th>Source</th></tr></thead>
      <tbody>{axis_info}</tbody>
    </table>
    <h2>Statistics</h2>
    <table>
      <thead><tr><th>Line</th><th>Source</th><th>Quantity</th><th>Axis</th><th>Statistics</th><th>Cache Key</th></tr></thead>
      <tbody>{stats_info}</tbody>
    </table>
    <h2>Integrations</h2>
    <table>
      <thead><tr><th>Line</th><th>Binding</th><th>Source</th><th>Input</th><th>Axis</th><th>Result</th></tr></thead>
      <tbody>{integrations}</tbody>
    </table>
    <h2>System Equations</h2>
    <table>
      <thead><tr><th>Line</th><th>System</th><th>Equation</th><th>Left Dimension</th><th>Right Dimension</th><th>Residual</th><th>Status</th></tr></thead>
      <tbody>{system_equations}</tbody>
    </table>
    <h2>Schemas</h2>
    <table>
      <thead><tr><th>Line</th><th>Name</th><th>Columns</th><th>Constraints</th><th>Missing Policies</th></tr></thead>
      <tbody>{schemas}</tbody>
    </table>
    <h2>CSV Promotions</h2>
    <table>
      <thead><tr><th>Line</th><th>Binding</th><th>Schema</th><th>Source</th><th>Rows</th><th>Source Hash</th></tr></thead>
      <tbody>{csv_promotions}</tbody>
    </table>
    <h2>Diagnostics</h2>
    <table>
      <thead><tr><th>Line</th><th>Severity</th><th>Code</th><th>Message</th></tr></thead>
      <tbody>{diagnostics}</tbody>
    </table>
    <h2>Plot</h2>
    <iframe class="plot" src="{plot_relative_path}" title="Generated plot"></iframe>
  </main>
</body>
</html>
"#,
        source_hash = html_escape(&report.source_hash),
        compiler_version = html_escape(eng_compiler::COMPILER_VERSION),
        report_version = html_escape(REPORT_VERSION)
    )
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn xml_escape(value: &str) -> String {
    html_escape(value)
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

fn push_json_string_array(json: &mut String, values: &[String]) {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!("\"{}\"", json_escape(value)));
    }
}

fn default_plot_spec(title: &str) -> PlotSpec {
    PlotSpec {
        title: title.to_owned(),
        plot_type: "line".to_owned(),
        x_axis: PlotAxis {
            name: "Time".to_owned(),
            label: "Time".to_owned(),
            unit: "sample".to_owned(),
        },
        y_axis: PlotAxis {
            name: "Value".to_owned(),
            label: "unit-aware value".to_owned(),
            unit: "preview".to_owned(),
        },
        series: vec![PlotSeries {
            name: "preview".to_owned(),
            quantity_kind: "Value".to_owned(),
            display_unit: "preview".to_owned(),
            points: preview_points(),
        }],
    }
}

fn preview_points() -> Vec<PlotPoint> {
    vec![
        PlotPoint { x: 0.0, y: 20.0 },
        PlotPoint { x: 1.0, y: 32.0 },
        PlotPoint { x: 2.0, y: 36.0 },
        PlotPoint { x: 3.0, y: 54.0 },
        PlotPoint { x: 4.0, y: 61.0 },
        PlotPoint { x: 5.0, y: 78.0 },
        PlotPoint { x: 6.0, y: 74.0 },
        PlotPoint { x: 7.0, y: 96.0 },
    ]
}

fn axis_label(axis: &PlotAxis) -> String {
    if axis.unit.is_empty() {
        axis.label.clone()
    } else {
        format!("{} ({})", axis.label, axis.unit)
    }
}

fn svg_points(points: &[PlotPoint]) -> String {
    if points.is_empty() {
        return String::new();
    }

    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let x_span = (max_x - min_x).max(1.0);
    let y_span = (max_y - min_y).max(1.0);

    points
        .iter()
        .map(|point| {
            let x = 72.0 + ((point.x - min_x) / x_span) * 588.0;
            let y = 250.0 - ((point.y - min_y) / y_span) * 210.0;
            format!("{x:.0},{y:.0}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn time_series_quantity(quantity_kind: &str) -> Option<(String, String)> {
    let rest = quantity_kind.strip_prefix("TimeSeries[")?;
    let (axis, after_axis) = rest.split_once(']')?;
    let quantity = after_axis.trim().strip_prefix("of ")?;
    Some((axis.trim().to_owned(), quantity.trim().to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use eng_compiler::{check_source, CheckOptions};

    #[test]
    fn plotspec_uses_timeseries_axis_unit_labels() {
        let report = check_source(
            "ok.eng",
            "script main(args: Args) -> Report {\n    sensor = promote csv \"data/sensor.csv\" as SensorData\n    cp = 4180 J/kg/K\n    Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\n}\n",
            &CheckOptions::default(),
        );

        let spec = plot_spec_from_report(&report);
        let json = plot_spec_json(&spec);
        let svg = render_svg_from_spec(&spec);

        assert_eq!(spec.plot_type, "line");
        assert_eq!(spec.x_axis.label, "Time");
        assert_eq!(spec.y_axis.unit, "W");
        assert!(json.contains("\"format\": \"eng-plotspec-v1\""));
        assert!(svg.contains("HeatRate (W)"));
    }

    #[test]
    fn report_spec_collects_v07_review_tables() {
        let report = check_source(
            "ok.eng",
            "schema SensorData {\n    time: DateTime index\n    T_supply: AbsoluteTemperature [degC]\n}\n\nscript main(args: Args) -> Report {\n    power = 10 kW\n    L = 1 m + 20 cm\n}\n",
            &CheckOptions::default(),
        );

        let spec = report_spec_from_report(&report, "plots/plot_manifest.json", "abc123");
        let json = report_spec_json(&spec);

        assert_eq!(spec.plot_manifest.path, "plots/plot_manifest.json");
        assert_eq!(spec.plot_manifest.hash, "abc123");
        assert!(spec.variables.iter().any(|variable| variable.name == "L"));
        assert_eq!(spec.schemas[0].name, "SensorData");
        assert!(spec
            .warnings
            .iter()
            .any(|warning| warning.code == "W-QTY-AMBIG-001"));
        assert!(json.contains("\"format\": \"eng-report-spec-v1\""));
        assert!(json.contains("\"variable_table\""));
        assert!(json.contains("\"inferred_declaration_table\""));
        assert!(json.contains("\"unit_conversion_table\""));
        assert!(json.contains("\"schema_summary\""));
        assert!(json.contains("\"plot_manifest\""));
        assert!(json.contains("\"warning_list\""));
    }

    #[test]
    fn report_spec_and_html_include_system_equation_summary() {
        let report = check_source(
            "ok.eng",
            "system RoomThermal {\n    parameter C: HeatCapacity = 500 kJ/K\n    parameter UA: Conductance = 150 W/K\n    state T: AbsoluteTemperature = 24 degC\n    input T_out: AbsoluteTemperature\n    input Q_internal: HeatRate\n    equation {\n        C * der(T) eq UA * (T_out - T) + Q_internal\n    }\n}\n",
            &CheckOptions::default(),
        );

        let spec = report_spec_from_report(&report, "plots/plot_manifest.json", "abc123");
        let json = report_spec_json(&spec);
        let html = render_html(&report, "plots/timeseries.svg");

        assert_eq!(spec.provenance.system_count, 1);
        assert_eq!(spec.provenance.equation_count, 1);
        assert_eq!(spec.systems[0].equations[0].status, "unit_consistent");
        assert!(json.contains("\"system_summary\""));
        assert!(json.contains("\"RoomThermal.residual_1\""));
        assert!(html.contains("System Equations"));
        assert!(html.contains("unit_consistent"));
    }
}
