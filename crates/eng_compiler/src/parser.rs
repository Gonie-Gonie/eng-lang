use crate::ast::{AstItem, ExplicitDecl, FastBinding, SchemaDecl, ScriptDecl};
use crate::lexer::{lex_line, Keyword, Symbol, Token, TokenKind};
use crate::source::{source_lines, SourceSpan};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseContext {
    TopLevel,
    Schema,
    Script,
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
    pub fast_bindings: usize,
    pub explicit_declarations: usize,
}

impl ParsedProgram {
    pub fn summary(&self) -> SyntaxSummary {
        let mut scripts = 0usize;
        let mut schemas = 0usize;
        let mut fast_bindings = 0usize;
        let mut explicit_declarations = 0usize;

        for item in &self.items {
            match item {
                AstItem::Script(_) => scripts += 1,
                AstItem::Schema(_) => schemas += 1,
                AstItem::FastBinding(_) => fast_bindings += 1,
                AstItem::ExplicitDecl(_) => explicit_declarations += 1,
                AstItem::ReservedKeywordUse { .. } => {}
            }
        }

        SyntaxSummary {
            lines: self.lines.len(),
            tokens: self.lines.iter().map(|line| line.tokens.len()).sum(),
            ast_items: self.items.len(),
            scripts,
            schemas,
            fast_bindings,
            explicit_declarations,
        }
    }
}

pub fn parse_source(source: &str) -> ParsedProgram {
    let mut parsed_lines = Vec::new();
    let mut items = Vec::new();
    let mut schema_depth = 0i32;
    let mut script_depth = 0i32;

    for source_line in source_lines(source) {
        let tokens = lex_line(source_line.line, source_line.start, &source_line.text);
        let context = if schema_depth > 0 {
            ParseContext::Schema
        } else if script_depth > 0 {
            ParseContext::Script
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
    if let Some(binding) = parse_fast_binding(tokens, line_text, context) {
        items.push(AstItem::FastBinding(binding));
    }
    if let Some(declaration) = parse_explicit_decl(tokens, line_text, context) {
        items.push(AstItem::ExplicitDecl(declaration));
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
    Some(ScriptDecl {
        name: name.clone(),
        span: first.span,
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
