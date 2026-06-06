use crate::ast::ExplicitDecl;
use crate::parser::ParseContext;
use crate::source::SourceSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpectedTypeSource {
    ExplicitAnnotation,
    AssignmentTarget,
    PublicBoundary,
    Unknown,
}

impl ExpectedTypeSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ExplicitAnnotation => "explicit_annotation",
            Self::AssignmentTarget => "assignment_target",
            Self::PublicBoundary => "public_boundary",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedType {
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: Option<String>,
    pub source: ExpectedTypeSource,
    pub line: usize,
    pub span: SourceSpan,
}

pub fn expected_type_from_explicit_decl(declaration: &ExplicitDecl) -> ExpectedType {
    ExpectedType {
        name: declaration.name.clone(),
        quantity_kind: declaration.type_name.clone(),
        display_unit: declaration.unit.clone(),
        source: if declaration.context == ParseContext::Schema {
            ExpectedTypeSource::PublicBoundary
        } else {
            ExpectedTypeSource::ExplicitAnnotation
        },
        line: declaration.line,
        span: declaration.span,
    }
}
