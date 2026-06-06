use crate::source::SourceSpan;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Keyword {
    As,
    Component,
    Constraints,
    Csv,
    Eq,
    Equation,
    Fn,
    Input,
    Missing,
    Model,
    Output,
    Parameter,
    Plot,
    Promote,
    Report,
    Return,
    Schema,
    Script,
    Show,
    State,
    Summarize,
    System,
    Where,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Symbol {
    Arrow,
    Colon,
    ColonEqual,
    Comma,
    Dot,
    Equal,
    EqualEqual,
    FatArrow,
    GreaterEqual,
    LBrace,
    LBracket,
    LParen,
    LessEqual,
    Minus,
    NotEqual,
    Plus,
    RBrace,
    RBracket,
    RParen,
    Slash,
    Star,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Keyword(Keyword),
    Number(String),
    StringLiteral(String),
    Symbol(Symbol),
    Unknown(char),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub span: SourceSpan,
}

pub fn lex_line(line_number: usize, line_start: usize, text: &str) -> Vec<Token> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut tokens = Vec::new();
    let mut cursor = 0usize;

    while cursor < chars.len() {
        let (byte_index, character) = chars[cursor];

        if character.is_whitespace() {
            cursor += 1;
            continue;
        }

        if character == '/' && peek_char(&chars, cursor + 1) == Some('/') {
            break;
        }

        if character.is_ascii_alphabetic() || character == '_' {
            let start = cursor;
            cursor += 1;
            while cursor < chars.len() {
                let next = chars[cursor].1;
                if next.is_ascii_alphanumeric() || next == '_' {
                    cursor += 1;
                } else {
                    break;
                }
            }
            let lexeme = slice_lexeme(text, &chars, start, cursor);
            let kind = keyword(&lexeme)
                .map(TokenKind::Keyword)
                .unwrap_or_else(|| TokenKind::Identifier(lexeme.clone()));
            tokens.push(token(
                kind,
                lexeme,
                line_start,
                line_number,
                byte_index,
                text,
                &chars,
                start,
                cursor,
            ));
            continue;
        }

        if character.is_ascii_digit() {
            let start = cursor;
            cursor += 1;
            while cursor < chars.len() {
                let next = chars[cursor].1;
                if next.is_ascii_digit() || next == '.' {
                    cursor += 1;
                } else {
                    break;
                }
            }
            let lexeme = slice_lexeme(text, &chars, start, cursor);
            tokens.push(token(
                TokenKind::Number(lexeme.clone()),
                lexeme,
                line_start,
                line_number,
                byte_index,
                text,
                &chars,
                start,
                cursor,
            ));
            continue;
        }

        if character == '"' {
            let start = cursor;
            cursor += 1;
            while cursor < chars.len() && chars[cursor].1 != '"' {
                cursor += 1;
            }
            if cursor < chars.len() {
                cursor += 1;
            }
            let lexeme = slice_lexeme(text, &chars, start, cursor);
            let value = lexeme.trim_matches('"').to_owned();
            tokens.push(token(
                TokenKind::StringLiteral(value),
                lexeme,
                line_start,
                line_number,
                byte_index,
                text,
                &chars,
                start,
                cursor,
            ));
            continue;
        }

        let start = cursor;
        let symbol = match (character, peek_char(&chars, cursor + 1)) {
            (':', Some('=')) => {
                cursor += 2;
                Some(Symbol::ColonEqual)
            }
            ('=', Some('=')) => {
                cursor += 2;
                Some(Symbol::EqualEqual)
            }
            ('=', Some('>')) => {
                cursor += 2;
                Some(Symbol::FatArrow)
            }
            ('!', Some('=')) => {
                cursor += 2;
                Some(Symbol::NotEqual)
            }
            ('>', Some('=')) => {
                cursor += 2;
                Some(Symbol::GreaterEqual)
            }
            ('<', Some('=')) => {
                cursor += 2;
                Some(Symbol::LessEqual)
            }
            ('-', Some('>')) => {
                cursor += 2;
                Some(Symbol::Arrow)
            }
            ('{', _) => advance_symbol(&mut cursor, Symbol::LBrace),
            ('}', _) => advance_symbol(&mut cursor, Symbol::RBrace),
            ('(', _) => advance_symbol(&mut cursor, Symbol::LParen),
            (')', _) => advance_symbol(&mut cursor, Symbol::RParen),
            ('[', _) => advance_symbol(&mut cursor, Symbol::LBracket),
            (']', _) => advance_symbol(&mut cursor, Symbol::RBracket),
            (':', _) => advance_symbol(&mut cursor, Symbol::Colon),
            ('=', _) => advance_symbol(&mut cursor, Symbol::Equal),
            (',', _) => advance_symbol(&mut cursor, Symbol::Comma),
            ('.', _) => advance_symbol(&mut cursor, Symbol::Dot),
            ('+', _) => advance_symbol(&mut cursor, Symbol::Plus),
            ('-', _) => advance_symbol(&mut cursor, Symbol::Minus),
            ('*', _) => advance_symbol(&mut cursor, Symbol::Star),
            ('/', _) => advance_symbol(&mut cursor, Symbol::Slash),
            _ => {
                cursor += 1;
                None
            }
        };

        let lexeme = slice_lexeme(text, &chars, start, cursor);
        let kind = symbol
            .map(TokenKind::Symbol)
            .unwrap_or(TokenKind::Unknown(character));
        tokens.push(token(
            kind,
            lexeme,
            line_start,
            line_number,
            byte_index,
            text,
            &chars,
            start,
            cursor,
        ));
    }

    tokens
}

fn advance_symbol(cursor: &mut usize, symbol: Symbol) -> Option<Symbol> {
    *cursor += 1;
    Some(symbol)
}

fn peek_char(chars: &[(usize, char)], cursor: usize) -> Option<char> {
    chars.get(cursor).map(|(_, character)| *character)
}

fn slice_lexeme(text: &str, chars: &[(usize, char)], start: usize, end: usize) -> String {
    let byte_start = chars[start].0;
    let byte_end = chars
        .get(end)
        .map(|(byte_index, _)| *byte_index)
        .unwrap_or(text.len());
    text[byte_start..byte_end].to_owned()
}

#[allow(clippy::too_many_arguments)]
fn token(
    kind: TokenKind,
    lexeme: String,
    line_start: usize,
    line_number: usize,
    byte_index: usize,
    text: &str,
    chars: &[(usize, char)],
    _start: usize,
    end: usize,
) -> Token {
    let byte_end = chars
        .get(end)
        .map(|(next_byte, _)| *next_byte)
        .unwrap_or(text.len());
    Token {
        kind,
        lexeme,
        span: SourceSpan::new(
            line_start + byte_index,
            line_start + byte_end,
            line_number,
            byte_index + 1,
        ),
    }
}

fn keyword(value: &str) -> Option<Keyword> {
    match value {
        "as" => Some(Keyword::As),
        "component" => Some(Keyword::Component),
        "constraints" => Some(Keyword::Constraints),
        "csv" => Some(Keyword::Csv),
        "eq" => Some(Keyword::Eq),
        "equation" => Some(Keyword::Equation),
        "fn" => Some(Keyword::Fn),
        "input" => Some(Keyword::Input),
        "missing" => Some(Keyword::Missing),
        "model" => Some(Keyword::Model),
        "output" => Some(Keyword::Output),
        "parameter" => Some(Keyword::Parameter),
        "plot" => Some(Keyword::Plot),
        "promote" => Some(Keyword::Promote),
        "report" => Some(Keyword::Report),
        "return" => Some(Keyword::Return),
        "schema" => Some(Keyword::Schema),
        "script" => Some(Keyword::Script),
        "show" => Some(Keyword::Show),
        "state" => Some(Keyword::State),
        "summarize" => Some(Keyword::Summarize),
        "system" => Some(Keyword::System),
        "where" => Some(Keyword::Where),
        _ => None,
    }
}
