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
pub struct ImportDecl {
    pub target: String,
    pub kind: String,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionDecl {
    pub name: String,
    pub parameters: Vec<FunctionParamDecl>,
    pub return_type: String,
    pub return_unit: Option<String>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionParamDecl {
    pub name: String,
    pub type_name: String,
    pub unit: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArgsDecl {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstDecl {
    pub name: String,
    pub type_name: String,
    pub unit: Option<String>,
    pub expression: String,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReturnDecl {
    pub expression: String,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructDecl {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassDecl {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassFieldDecl {
    pub name: String,
    pub type_name: String,
    pub unit: Option<String>,
    pub default_value: Option<String>,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassValidationDecl {
    pub expression: String,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassMethodDecl {
    pub name: String,
    pub return_type: String,
    pub return_unit: Option<String>,
    pub expression: String,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassObjectDecl {
    pub name: String,
    pub class_name: String,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassObjectCopyDecl {
    pub name: String,
    pub source_name: String,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassObjectFieldDecl {
    pub owner_line: Option<usize>,
    pub name: String,
    pub expression: String,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArgsFieldDecl {
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
pub struct StateSpaceTypeBlockDecl {
    pub role: String,
    pub name: String,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateSpaceTypeMemberDecl {
    pub name: String,
    pub type_name: String,
    pub unit: Option<String>,
    pub line: usize,
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
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateSpaceVectorDecl {
    pub role: String,
    pub name: String,
    pub members: Vec<String>,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EquationDecl {
    pub left: String,
    pub right: String,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
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
pub struct PrintDecl {
    pub level: String,
    pub template: String,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CsvExportDecl {
    pub source: String,
    pub format: String,
    pub path: String,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CsvExportFieldDecl {
    pub expression: String,
    pub display_unit: Option<String>,
    pub format: Option<String>,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteDecl {
    pub format: String,
    pub path: String,
    pub expression: String,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileOperationDecl {
    pub operation: String,
    pub source: String,
    pub destination: Option<String>,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetDownloadDecl {
    pub url: String,
    pub target: String,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessRunDecl {
    pub binding: Option<String>,
    pub command: String,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestDecl {
    pub name: String,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssertDecl {
    pub left: String,
    pub operator: String,
    pub right: String,
    pub tolerance: Option<String>,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoldenDecl {
    pub artifact: String,
    pub expected: String,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandClauseDecl {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandStyleDecl {
    pub verb: String,
    pub target: String,
    pub clauses: Vec<CommandClauseDecl>,
    pub canonical: String,
    pub status: String,
    pub owner: Option<String>,
    pub line: usize,
    pub span: SourceSpan,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhereBlockDecl {
    pub owner_line: Option<usize>,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhereBindingDecl {
    pub owner_line: Option<usize>,
    pub name: String,
    pub expression: String,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WherePredicateDecl {
    pub owner_line: Option<usize>,
    pub expression: String,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OnBlockDecl {
    pub owner_line: Option<usize>,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OnPredicateDecl {
    pub owner_line: Option<usize>,
    pub expression: String,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithBlockDecl {
    pub owner_line: Option<usize>,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithOptionDecl {
    pub owner_line: Option<usize>,
    pub key: String,
    pub value: String,
    pub line: usize,
    pub span: SourceSpan,
    pub key_span: SourceSpan,
    pub value_span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectationSuiteDecl {
    pub target: String,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectationDecl {
    pub suite_line: Option<usize>,
    pub text: String,
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
    Import(ImportDecl),
    Function(FunctionDecl),
    Args(ArgsDecl),
    Const(ConstDecl),
    Return(ReturnDecl),
    Struct(StructDecl),
    Class(ClassDecl),
    ClassField(ClassFieldDecl),
    ClassValidation(ClassValidationDecl),
    ClassMethod(ClassMethodDecl),
    ClassObject(ClassObjectDecl),
    ClassObjectCopy(ClassObjectCopyDecl),
    ClassObjectField(ClassObjectFieldDecl),
    ArgsField(ArgsFieldDecl),
    System(SystemDecl),
    StateSpaceTypeBlock(StateSpaceTypeBlockDecl),
    StateSpaceTypeMember(StateSpaceTypeMemberDecl),
    Domain(DomainDecl),
    DomainVariable(DomainVariableDecl),
    Conservation(ConservationDecl),
    Component(ComponentDecl),
    Port(PortDecl),
    Connect(ConnectDecl),
    SystemVariable(SystemVariableDecl),
    StateSpaceVector(StateSpaceVectorDecl),
    Equation(EquationDecl),
    Constraint(ConstraintDecl),
    FastBinding(FastBinding),
    ExplicitDecl(ExplicitDecl),
    MissingPolicy(MissingPolicyDecl),
    Summary(SummaryDecl),
    Print(PrintDecl),
    CsvExport(CsvExportDecl),
    CsvExportField(CsvExportFieldDecl),
    Write(WriteDecl),
    FileOperation(FileOperationDecl),
    NetDownload(NetDownloadDecl),
    ProcessRun(ProcessRunDecl),
    Test(TestDecl),
    Assert(AssertDecl),
    Golden(GoldenDecl),
    CommandStyle(CommandStyleDecl),
    WhereBlock(WhereBlockDecl),
    WhereBinding(WhereBindingDecl),
    WherePredicate(WherePredicateDecl),
    OnBlock(OnBlockDecl),
    OnPredicate(OnPredicateDecl),
    WithBlock(WithBlockDecl),
    WithOption(WithOptionDecl),
    ExpectationSuite(ExpectationSuiteDecl),
    Expectation(ExpectationDecl),
    ReservedKeywordUse { keyword: String, span: SourceSpan },
}
