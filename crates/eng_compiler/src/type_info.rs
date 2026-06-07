use crate::source::SourceSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeInfoSource {
    Explicit,
    Inferred,
    PublicBoundary,
    SystemBoundary,
}

impl TypeInfoSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::Inferred => "inferred",
            Self::PublicBoundary => "public_boundary",
            Self::SystemBoundary => "system_boundary",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeInfo {
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub canonical_unit: String,
    pub dimension: String,
    pub source: TypeInfoSource,
    pub line: usize,
    pub span: SourceSpan,
}
