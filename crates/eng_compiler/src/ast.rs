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
    pub arg_name: Option<String>,
    pub arg_type: Option<String>,
    pub return_type: Option<String>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructDecl {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructFieldDecl {
    pub name: String,
    pub type_name: String,
    pub default_value: Option<String>,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemDecl {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomainDecl {
    pub name: String,
    pub type_parameters: Vec<DomainTypeParameterDecl>,
    pub package: Option<String>,
    pub version: Option<String>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomainTypeParameterDecl {
    pub kind: String,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomainVariableDecl {
    pub role: String,
    pub name: String,
    pub type_name: String,
    pub unit: Option<String>,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConservationDecl {
    pub text: String,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentDecl {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortDecl {
    pub name: String,
    pub domain: String,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectDecl {
    pub left: String,
    pub right: String,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemVariableDecl {
    pub role: String,
    pub name: String,
    pub type_name: String,
    pub unit: Option<String>,
    pub expression: Option<String>,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EquationDecl {
    pub left: String,
    pub right: String,
    pub line: usize,
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
pub struct ConstraintDecl {
    pub text: String,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissingPolicyDecl {
    pub column: String,
    pub policy: String,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SummaryDecl {
    pub source: String,
    pub statistics: Vec<String>,
    pub line: usize,
    pub span: SourceSpan,
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
    Struct(StructDecl),
    StructField(StructFieldDecl),
    System(SystemDecl),
    Domain(DomainDecl),
    DomainVariable(DomainVariableDecl),
    Conservation(ConservationDecl),
    Component(ComponentDecl),
    Port(PortDecl),
    Connect(ConnectDecl),
    SystemVariable(SystemVariableDecl),
    Equation(EquationDecl),
    Constraint(ConstraintDecl),
    FastBinding(FastBinding),
    ExplicitDecl(ExplicitDecl),
    MissingPolicy(MissingPolicyDecl),
    Summary(SummaryDecl),
    ReservedKeywordUse { keyword: String, span: SourceSpan },
}
