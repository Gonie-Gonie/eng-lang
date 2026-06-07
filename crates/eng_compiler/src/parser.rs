use crate::ast::{
    AstItem, ConstraintDecl, EquationDecl, ExplicitDecl, FastBinding, MissingPolicyDecl,
    SchemaDecl, ScriptDecl, StructDecl, StructFieldDecl, SummaryDecl, SystemDecl,
    SystemVariableDecl,
};
use crate::lexer::{lex_line, Keyword, Symbol, Token, TokenKind};
use crate::source::{source_lines, SourceSpan};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseContext {
    TopLevel,
    Schema,
    SchemaConstraints,
    SchemaMissing,
    Script,
    Struct,
    System,
    Equation,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedLine {
    pub line: usize,
    pub text: String,
    pub tokens: Vec<Token>,
    pub context: ParseContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedProgram {
    pub lines: Vec<ParsedLine>,
    pub items: Vec<AstItem>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyntaxSummary {
    pub lines: usize,
    pub tokens: usize,
    pub ast_items: usize,
    pub scripts: usize,
    pub schemas: usize,
    pub systems: usize,
    pub structs: usize,
    pub struct_fields: usize,
    pub equations: usize,
    pub fast_bindings: usize,
    pub explicit_declarations: usize,
}

impl ParsedProgram {
    pub fn summary(&self) -> SyntaxSummary {
        let mut scripts = 0usize;
        let mut schemas = 0usize;
        let mut systems = 0usize;
        let mut structs = 0usize;
        let mut struct_fields = 0usize;
        let mut equations = 0usize;
        let mut fast_bindings = 0usize;
        let mut explicit_declarations = 0usize;

        for item in &self.items {
            match item {
                AstItem::Script(_) => scripts += 1,
                AstItem::Schema(_) => schemas += 1,
                AstItem::System(_) => systems += 1,
                AstItem::Struct(_) => structs += 1,
                AstItem::StructField(_) => struct_fields += 1,
                AstItem::Equation(_) => equations += 1,
                AstItem::FastBinding(_) => fast_bindings += 1,
                AstItem::ExplicitDecl(_) => explicit_declarations += 1,
                AstItem::SystemVariable(_)
                | AstItem::Constraint(_)
                | AstItem::MissingPolicy(_)
                | AstItem::Summary(_)
                | AstItem::ReservedKeywordUse { .. } => {}
            }
        }

        SyntaxSummary {
            lines: self.lines.len(),
            tokens: self.lines.iter().map(|line| line.tokens.len()).sum(),
            ast_items: self.items.len(),
            scripts,
            schemas,
            systems,
            structs,
            struct_fields,
            equations,
            fast_bindings,
            explicit_declarations,
        }
    }
}

pub fn parse_source(source: &str) -> ParsedProgram {
    let mut parsed_lines = Vec::new();
    let mut items = Vec::new();
    let mut schema_depth = 0i32;
    let mut constraints_depth = 0i32;
    let mut missing_depth = 0i32;
    let mut script_depth = 0i32;
    let mut struct_depth = 0i32;
    let mut system_depth = 0i32;
    let mut equation_depth = 0i32;

    for source_line in source_lines(source) {
        let tokens = lex_line(source_line.line, source_line.start, &source_line.text);
        let context = if equation_depth > 0 {
            ParseContext::Equation
        } else if missing_depth > 0 {
            ParseContext::SchemaMissing
        } else if constraints_depth > 0 {
            ParseContext::SchemaConstraints
        } else if schema_depth > 0 {
            ParseContext::Schema
        } else if script_depth > 0 {
            ParseContext::Script
        } else if struct_depth > 0 {
            ParseContext::Struct
        } else if system_depth > 0 {
            ParseContext::System
        } else {
            ParseContext::TopLevel
        };

        if !tokens.is_empty() {
            parse_line_items(&mut items, &tokens, &source_line.text, context);
        }

        if starts_with_keyword(&tokens, Keyword::Schema) {
            schema_depth += brace_delta(&tokens);
            if schema_depth == 0 {
                schema_depth = 1;
            }
        } else if schema_depth > 0 {
            schema_depth += brace_delta(&tokens);
            if schema_depth <= 0 {
                schema_depth = 0;
            }
        }

        if schema_depth > 0 && starts_with_keyword(&tokens, Keyword::Constraints) {
            constraints_depth += brace_delta(&tokens);
            if constraints_depth == 0 {
                constraints_depth = 1;
            }
        } else if constraints_depth > 0 {
            constraints_depth += brace_delta(&tokens);
            if constraints_depth <= 0 {
                constraints_depth = 0;
            }
        }

        if schema_depth > 0 && starts_with_keyword(&tokens, Keyword::Missing) {
            missing_depth += brace_delta(&tokens);
            if missing_depth == 0 {
                missing_depth = 1;
            }
        } else if missing_depth > 0 {
            missing_depth += brace_delta(&tokens);
            if missing_depth <= 0 {
                missing_depth = 0;
            }
        }

        if starts_with_keyword(&tokens, Keyword::Script) {
            script_depth += brace_delta(&tokens);
            if script_depth == 0 {
                script_depth = 1;
            }
        } else if script_depth > 0 {
            script_depth += brace_delta(&tokens);
            if script_depth <= 0 {
                script_depth = 0;
            }
        }

        if starts_with_keyword(&tokens, Keyword::Struct) {
            struct_depth += brace_delta(&tokens);
            if struct_depth == 0 {
                struct_depth = 1;
            }
        } else if struct_depth > 0 {
            struct_depth += brace_delta(&tokens);
            if struct_depth <= 0 {
                struct_depth = 0;
            }
        }

        if starts_with_keyword(&tokens, Keyword::System) {
            system_depth += brace_delta(&tokens);
            if system_depth == 0 {
                system_depth = 1;
            }
        } else if system_depth > 0 {
            system_depth += brace_delta(&tokens);
            if system_depth <= 0 {
                system_depth = 0;
            }
        }

        if system_depth > 0 && starts_with_keyword(&tokens, Keyword::Equation) {
            equation_depth += brace_delta(&tokens);
            if equation_depth == 0 {
                equation_depth = 1;
            }
        } else if equation_depth > 0 {
            equation_depth += brace_delta(&tokens);
            if equation_depth <= 0 {
                equation_depth = 0;
            }
        }

        parsed_lines.push(ParsedLine {
            line: source_line.line,
            text: source_line.text,
            tokens,
            context,
        });
    }

    ParsedProgram {
        lines: parsed_lines,
        items,
    }
}

fn parse_line_items(
    items: &mut Vec<AstItem>,
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) {
    if let Some(schema) = parse_schema_decl(tokens) {
        items.push(AstItem::Schema(schema));
    }
    if let Some(script) = parse_script_decl(tokens) {
        items.push(AstItem::Script(script));
    }
    if let Some(struct_decl) = parse_struct_decl(tokens) {
        items.push(AstItem::Struct(struct_decl));
    }
    if let Some(field) = parse_struct_field_decl(tokens, line_text, context) {
        items.push(AstItem::StructField(field));
    }
    if let Some(system) = parse_system_decl(tokens) {
        items.push(AstItem::System(system));
    }
    if let Some(variable) = parse_system_variable_decl(tokens, line_text, context) {
        items.push(AstItem::SystemVariable(variable));
    }
    if let Some(equation) = parse_equation_decl(tokens, line_text, context) {
        items.push(AstItem::Equation(equation));
    }
    if let Some(binding) = parse_fast_binding(tokens, line_text, context) {
        items.push(AstItem::FastBinding(binding));
    }
    if !matches!(
        context,
        ParseContext::Struct | ParseContext::SchemaConstraints | ParseContext::SchemaMissing
    ) {
        if let Some(declaration) = parse_explicit_decl(tokens, line_text, context) {
            items.push(AstItem::ExplicitDecl(declaration));
        }
    }
    if let Some(constraint) = parse_constraint_decl(tokens, line_text, context) {
        items.push(AstItem::Constraint(constraint));
    }
    if let Some(policy) = parse_missing_policy(tokens, line_text, context) {
        items.push(AstItem::MissingPolicy(policy));
    }
    if let Some(summary) = parse_summary_decl(tokens, line_text) {
        items.push(AstItem::Summary(summary));
    }
    if let Some(keyword) = parse_reserved_keyword_use(tokens) {
        items.push(keyword);
    }
}

fn parse_schema_decl(tokens: &[Token]) -> Option<SchemaDecl> {
    let [first, second, ..] = tokens else {
        return None;
    };
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Schema)) {
        return None;
    }
    let TokenKind::Identifier(name) = &second.kind else {
        return None;
    };
    Some(SchemaDecl {
        name: name.clone(),
        span: first.span,
    })
}

fn parse_system_decl(tokens: &[Token]) -> Option<SystemDecl> {
    let [first, second, ..] = tokens else {
        return None;
    };
    if !matches!(first.kind, TokenKind::Keyword(Keyword::System)) {
        return None;
    }
    let TokenKind::Identifier(name) = &second.kind else {
        return None;
    };
    Some(SystemDecl {
        name: name.clone(),
        span: first.span,
    })
}

fn parse_struct_decl(tokens: &[Token]) -> Option<StructDecl> {
    let [first, second, ..] = tokens else {
        return None;
    };
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Struct)) {
        return None;
    }
    let TokenKind::Identifier(name) = &second.kind else {
        return None;
    };
    Some(StructDecl {
        name: name.clone(),
        span: first.span,
    })
}

fn parse_script_decl(tokens: &[Token]) -> Option<ScriptDecl> {
    let [first, second, ..] = tokens else {
        return None;
    };
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Script)) {
        return None;
    }
    let TokenKind::Identifier(name) = &second.kind else {
        return None;
    };
    let (arg_name, arg_type) = parse_script_arg(tokens);
    let return_type = parse_script_return(tokens);
    Some(ScriptDecl {
        name: name.clone(),
        arg_name,
        arg_type,
        return_type,
        span: first.span,
    })
}

fn parse_struct_field_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<StructFieldDecl> {
    if context != ParseContext::Struct {
        return None;
    }
    let [first, second, third, ..] = tokens else {
        return None;
    };
    let TokenKind::Identifier(name) = &first.kind else {
        return None;
    };
    if !matches!(second.kind, TokenKind::Symbol(Symbol::Colon)) {
        return None;
    }
    let type_name = token_type_name(third)?;
    let default_value = line_text
        .split_once('=')
        .map(|(_, right)| right.trim().to_owned());

    Some(StructFieldDecl {
        name: name.clone(),
        type_name,
        default_value,
        line: first.span.line,
        span: first.span,
    })
}

fn parse_script_arg(tokens: &[Token]) -> (Option<String>, Option<String>) {
    for window in tokens.windows(5) {
        let [open, arg_name, colon, arg_type, close] = window else {
            continue;
        };
        if !matches!(open.kind, TokenKind::Symbol(Symbol::LParen))
            || !matches!(colon.kind, TokenKind::Symbol(Symbol::Colon))
            || !matches!(close.kind, TokenKind::Symbol(Symbol::RParen))
        {
            continue;
        }
        let Some(arg_name) = token_type_name(arg_name) else {
            continue;
        };
        let Some(arg_type) = token_type_name(arg_type) else {
            continue;
        };
        return (Some(arg_name), Some(arg_type));
    }

    (None, None)
}

fn parse_script_return(tokens: &[Token]) -> Option<String> {
    for (index, token) in tokens.iter().enumerate() {
        if !matches!(token.kind, TokenKind::Symbol(Symbol::Arrow)) {
            continue;
        }
        return tokens.get(index + 1).and_then(token_type_name);
    }
    None
}

fn token_type_name(token: &Token) -> Option<String> {
    match &token.kind {
        TokenKind::Identifier(value) => Some(value.clone()),
        TokenKind::Keyword(Keyword::Report) => Some("Report".to_owned()),
        _ => None,
    }
}

fn parse_system_variable_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<SystemVariableDecl> {
    if context != ParseContext::System {
        return None;
    }
    let [first, second, third, ..] = tokens else {
        return None;
    };
    let role = match first.kind {
        TokenKind::Keyword(Keyword::Parameter) => "parameter",
        TokenKind::Keyword(Keyword::State) => "state",
        TokenKind::Keyword(Keyword::Input) => "input",
        _ => return None,
    };
    let TokenKind::Identifier(name) = &second.kind else {
        return None;
    };
    if !matches!(third.kind, TokenKind::Symbol(Symbol::Colon)) {
        return None;
    }

    let raw_after_colon = line_text.split_once(':')?.1.trim();
    let (type_part, expression) = raw_after_colon
        .split_once('=')
        .map(|(left, right)| (left.trim(), Some(right.trim().to_owned())))
        .unwrap_or((raw_after_colon, None));
    let (type_name, unit) = split_type_and_unit(type_part);

    Some(SystemVariableDecl {
        role: role.to_owned(),
        name: name.clone(),
        type_name,
        unit,
        expression,
        line: first.span.line,
        span: first.span,
    })
}

fn parse_equation_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<EquationDecl> {
    if context != ParseContext::Equation {
        return None;
    }
    let first = tokens.first()?;
    if matches!(
        &first.kind,
        TokenKind::Symbol(Symbol::LBrace | Symbol::RBrace)
    ) {
        return None;
    }

    let eq_token = tokens
        .iter()
        .find(|token| matches!(token.kind, TokenKind::Keyword(Keyword::Eq)))?;
    let (left, right) = line_text.split_once(" eq ")?;
    Some(EquationDecl {
        left: left.trim().to_owned(),
        right: right.trim().to_owned(),
        line: eq_token.span.line,
        span: eq_token.span,
    })
}

fn parse_fast_binding(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<FastBinding> {
    let [first, second, ..] = tokens else {
        return None;
    };
    let TokenKind::Identifier(name) = &first.kind else {
        return None;
    };
    if !matches!(second.kind, TokenKind::Symbol(Symbol::Equal)) {
        return None;
    }
    Some(FastBinding {
        name: name.clone(),
        expression: expression_after(line_text, '=')?,
        line: first.span.line,
        span: first.span,
        context,
    })
}

fn parse_constraint_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<ConstraintDecl> {
    if context != ParseContext::SchemaConstraints {
        return None;
    }
    let first = tokens.first()?;
    if matches!(
        &first.kind,
        TokenKind::Symbol(Symbol::LBrace | Symbol::RBrace)
    ) {
        return None;
    }
    Some(ConstraintDecl {
        text: line_text.trim().to_owned(),
        line: first.span.line,
        span: first.span,
    })
}

fn parse_missing_policy(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<MissingPolicyDecl> {
    if context != ParseContext::SchemaMissing {
        return None;
    }
    let [first, second, ..] = tokens else {
        return None;
    };
    let TokenKind::Identifier(column) = &first.kind else {
        return None;
    };
    if !matches!(second.kind, TokenKind::Symbol(Symbol::Colon)) {
        return None;
    }
    let policy = line_text.split_once(':')?.1.trim().to_owned();
    Some(MissingPolicyDecl {
        column: column.clone(),
        policy,
        line: first.span.line,
        span: first.span,
    })
}

fn parse_summary_decl(tokens: &[Token], line_text: &str) -> Option<SummaryDecl> {
    let [first, second, ..] = tokens else {
        return None;
    };
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Summarize)) {
        return None;
    }
    let TokenKind::Identifier(source) = &second.kind else {
        return None;
    };

    let statistics = line_text
        .split_once('[')
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(inside, _)| {
            inside
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Some(SummaryDecl {
        source: source.clone(),
        statistics,
        line: first.span.line,
        span: first.span,
    })
}

fn parse_explicit_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<ExplicitDecl> {
    let [first, second, ..] = tokens else {
        return None;
    };
    let TokenKind::Identifier(name) = &first.kind else {
        return None;
    };
    if !matches!(second.kind, TokenKind::Symbol(Symbol::Colon)) {
        return None;
    }

    let raw_after_colon = line_text.split_once(':')?.1.trim();
    let (type_part, expression) = raw_after_colon
        .split_once('=')
        .map(|(left, right)| (left.trim(), Some(right.trim().to_owned())))
        .unwrap_or((raw_after_colon, None));
    let (type_name, unit) = split_type_and_unit(type_part);

    Some(ExplicitDecl {
        name: name.clone(),
        type_name,
        unit,
        expression,
        line: first.span.line,
        span: first.span,
        context,
    })
}

fn parse_reserved_keyword_use(tokens: &[Token]) -> Option<AstItem> {
    let [first, second, ..] = tokens else {
        return None;
    };
    if matches!(first.kind, TokenKind::Keyword(Keyword::Eq))
        && matches!(second.kind, TokenKind::Symbol(Symbol::Equal))
    {
        return Some(AstItem::ReservedKeywordUse {
            keyword: "eq".to_owned(),
            span: first.span,
        });
    }
    None
}

fn split_type_and_unit(type_part: &str) -> (String, Option<String>) {
    let Some((before_unit, after_unit)) = type_part.split_once('[') else {
        return (type_part.trim().to_owned(), None);
    };
    let unit = after_unit
        .split_once(']')
        .map(|(unit, _)| unit.trim().to_owned());
    (before_unit.trim().to_owned(), unit)
}

fn expression_after(line_text: &str, marker: char) -> Option<String> {
    line_text
        .split_once(marker)
        .map(|(_, expression)| expression.trim().to_owned())
}

fn starts_with_keyword(tokens: &[Token], keyword: Keyword) -> bool {
    tokens
        .first()
        .is_some_and(|token| matches!(token.kind, TokenKind::Keyword(found) if found == keyword))
}

fn brace_delta(tokens: &[Token]) -> i32 {
    let mut delta = 0i32;
    for token in tokens {
        match token.kind {
            TokenKind::Symbol(Symbol::LBrace) => delta += 1,
            TokenKind::Symbol(Symbol::RBrace) => delta -= 1,
            _ => {}
        }
    }
    delta
}

#[allow(dead_code)]
fn span_of(tokens: &[Token]) -> Option<SourceSpan> {
    tokens.first().map(|token| token.span)
}
