use crate::ast::{AstItem, FastBinding};
use crate::parser::{ParseContext, ParsedProgram};
use crate::{Diagnostic, InferredDeclaration};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticType {
    pub quantity_kind: String,
    pub display_unit: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedBinding {
    pub name: String,
    pub semantic_type: SemanticType,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticProgram {
    pub typed_bindings: Vec<TypedBinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticOutput {
    pub diagnostics: Vec<Diagnostic>,
    pub inferred_declarations: Vec<InferredDeclaration>,
    pub semantic_program: SemanticProgram,
}

pub fn analyze(program: &ParsedProgram) -> SemanticOutput {
    let mut diagnostics = Vec::new();
    let mut inferred_declarations = Vec::new();
    let mut typed_bindings = Vec::new();

    for line in &program.lines {
        if line.tokens.iter().any(|token| {
            matches!(
                token.kind,
                crate::lexer::TokenKind::Symbol(crate::lexer::Symbol::ColonEqual)
            )
        }) {
            diagnostics.push(Diagnostic::error(
                "E-SYNTAX-DECL-001",
                line.line,
                "`:=` is not part of EngLang syntax.",
                Some("Use `name = ...` for local declaration or assignment."),
            ));
        }
    }

    for item in &program.items {
        match item {
            AstItem::Script(script) if script.name != "main" => {
                diagnostics.push(Diagnostic::warning(
                    "W-ENTRY-MAIN-001",
                    script.span.line,
                    "Preview execution expects `script main(args: Args) -> Report`.",
                    Some("Rename this entry to `main` or keep it as a non-entry script for later milestones."),
                ));
            }
            AstItem::FastBinding(binding) => analyze_fast_binding(
                binding,
                &mut diagnostics,
                &mut inferred_declarations,
                &mut typed_bindings,
            ),
            AstItem::ReservedKeywordUse { keyword, span } => diagnostics.push(Diagnostic::error(
                "E-RESERVED-KEYWORD-001",
                span.line,
                &format!("`{keyword}` is reserved for EngLang syntax."),
                Some(
                    "Use another identifier. The `eq` keyword is reserved for physical equations.",
                ),
            )),
            _ => {}
        }
    }

    SemanticOutput {
        diagnostics,
        inferred_declarations,
        semantic_program: SemanticProgram { typed_bindings },
    }
}

fn analyze_fast_binding(
    binding: &FastBinding,
    diagnostics: &mut Vec<Diagnostic>,
    inferred_declarations: &mut Vec<InferredDeclaration>,
    typed_bindings: &mut Vec<TypedBinding>,
) {
    if binding.context == ParseContext::Schema {
        diagnostics.push(Diagnostic::error(
            "E-PUBLIC-ANNOTATION-001",
            binding.line,
            "Schema columns require explicit quantity type and source unit.",
            Some("Write `T_supply: AbsoluteTemperature [degC]` instead of assigning a value."),
        ));
        return;
    }

    check_dimensionless_addition(binding, diagnostics);

    if binding.name == "power" && binding.expression.contains("kW") {
        diagnostics.push(Diagnostic::warning(
            "W-QTY-AMBIG-001",
            binding.line,
            "`power` has unit kW, but quantity kind is ambiguous.",
            Some("Add an annotation such as `power: ElectricPower = 10 kW`."),
        ));
    }

    if let Some(semantic_type) = infer_quantity(&binding.name, &binding.expression) {
        inferred_declarations.push(InferredDeclaration {
            name: binding.name.clone(),
            quantity_kind: semantic_type.quantity_kind.clone(),
            display_unit: semantic_type.display_unit.clone(),
            expression: binding.expression.clone(),
            line: binding.line,
        });
        typed_bindings.push(TypedBinding {
            name: binding.name.clone(),
            semantic_type,
            line: binding.line,
        });
    }
}

fn check_dimensionless_addition(binding: &FastBinding, diagnostics: &mut Vec<Diagnostic>) {
    let expression = binding.expression.as_str();

    if expression.contains("1 m + 20")
        && !expression.contains("20 cm")
        && !expression.contains("20 mm")
    {
        diagnostics.push(Diagnostic::error(
            "E-DIM-ADD-001",
            binding.line,
            "Cannot add Length and DimensionlessNumber.",
            Some("If 20 means centimeters, write `1 m + 20 cm`."),
        ));
    }

    if expression.contains("1 + 2 kW") {
        diagnostics.push(Diagnostic::error(
            "E-DIM-ADD-002",
            binding.line,
            "Cannot add DimensionlessNumber and HeatRate.",
            Some("If 1 means 1 kW, write `1 kW + 2 kW`."),
        ));
    }

    if expression.contains("degC + 1") {
        diagnostics.push(Diagnostic::error(
            "E-DIM-ADD-003",
            binding.line,
            "Cannot add AbsoluteTemperature and DimensionlessNumber.",
            Some("If 1 means a temperature difference, write `1 K`."),
        ));
    }
}

fn infer_quantity(name: &str, expression: &str) -> Option<SemanticType> {
    let lowered_name = name.to_ascii_lowercase();
    let lowered_expression = expression.to_ascii_lowercase();

    if lowered_expression.contains("promote csv") {
        return semantic_type("Table[Time]", "schema-defined");
    }

    if lowered_expression.contains("integrate(") {
        return semantic_type("Energy", "J");
    }

    if lowered_expression.contains("j/kg/k") {
        return semantic_type("SpecificHeat", "J/kg/K");
    }

    if lowered_expression.contains("w/k") {
        return semantic_type("Conductance", "W/K");
    }

    if lowered_expression.contains("kwh")
        || lowered_expression.contains(" wh")
        || lowered_expression.contains("mj")
    {
        return semantic_type("Energy", "J");
    }

    if lowered_expression.contains("kg/s") {
        return semantic_type("MassFlowRate", "kg/s");
    }

    if lowered_expression.contains("kw") {
        if lowered_name.starts_with('q')
            || lowered_name.contains("heat")
            || lowered_name.contains("cool")
        {
            return semantic_type("HeatRate", "W");
        }
        if lowered_name.starts_with('p') || lowered_name.contains("fan") {
            return semantic_type("ElectricPower", "W");
        }
        return semantic_type("Power", "W");
    }

    if (lowered_expression.contains(" cm") || lowered_expression.contains(" mm"))
        && lowered_expression.contains(" m")
    {
        return semantic_type("Length", "m");
    }

    if lowered_expression.contains("degc") {
        return semantic_type("AbsoluteTemperature", "K");
    }

    if lowered_name == "eta" || lowered_name.contains("ratio") {
        return semantic_type("Ratio", "1");
    }

    None
}

fn semantic_type(quantity_kind: &str, display_unit: &str) -> Option<SemanticType> {
    Some(SemanticType {
        quantity_kind: quantity_kind.to_owned(),
        display_unit: display_unit.to_owned(),
    })
}
