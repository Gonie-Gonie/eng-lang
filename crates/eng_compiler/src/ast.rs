use crate::parser::ParseContext;
use crate::source::SourceSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaDecl {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptDecl {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FastBinding {
    pub name: String,
    pub expression: String,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplicitDecl {
    pub name: String,
    pub type_name: String,
    pub unit: Option<String>,
    pub expression: Option<String>,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AstItem {
    Schema(SchemaDecl),
    Script(ScriptDecl),
    FastBinding(FastBinding),
    ExplicitDecl(ExplicitDecl),
    ReservedKeywordUse { keyword: String, span: SourceSpan },
}
