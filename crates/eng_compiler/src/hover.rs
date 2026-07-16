use crate::source::SourceSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoverHint {
    pub name: String,
    pub line: usize,
    pub column: usize,
    pub span: SourceSpan,
    pub quantity_kind: String,
    pub display_unit: String,
    pub expression: Option<String>,
    pub detail: String,
    pub quick_fixes: Vec<String>,
}

impl HoverHint {
    pub fn inferred(
        name: String,
        quantity_kind: String,
        display_unit: String,
        expression: String,
        span: SourceSpan,
    ) -> Self {
        Self {
            name,
            line: span.line,
            column: span.column,
            span,
            detail: format!("inferred as {quantity_kind} [{display_unit}]"),
            quantity_kind,
            display_unit,
            expression: Some(expression),
            quick_fixes: vec!["Expand declaration".to_owned()],
        }
    }

    pub fn explicit(
        name: String,
        quantity_kind: String,
        display_unit: String,
        expression: Option<String>,
        span: SourceSpan,
    ) -> Self {
        Self {
            name,
            line: span.line,
            column: span.column,
            span,
            detail: format!("declared as {quantity_kind} [{display_unit}]"),
            quantity_kind,
            display_unit,
            expression,
            quick_fixes: Vec::new(),
        }
    }

    pub fn importable_const(
        name: String,
        quantity_kind: String,
        display_unit: String,
        expression: String,
        span: SourceSpan,
    ) -> Self {
        Self {
            name,
            line: span.line,
            column: span.column,
            span,
            detail: format!("importable const {quantity_kind} [{display_unit}]"),
            quantity_kind,
            display_unit,
            expression: Some(expression),
            quick_fixes: Vec::new(),
        }
    }
}
