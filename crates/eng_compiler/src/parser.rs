use crate::ast::{
    ArgsDecl, ArgsFieldDecl, AstItem, CommandClauseDecl, CommandStyleDecl, ComponentDecl,
    ConnectDecl, ConservationDecl, ConstDecl, ConstraintDecl, CsvExportDecl, CsvExportFieldDecl,
    DomainDecl, DomainTypeParameterDecl, DomainVariableDecl, EquationDecl, ExplicitDecl,
    FastBinding, FileOperationDecl, FunctionDecl, FunctionParamDecl, ImportDecl, MissingPolicyDecl,
    PortDecl, PrintDecl, ReturnDecl, SchemaDecl, ScriptDecl, StructDecl, SummaryDecl, SystemDecl,
    SystemVariableDecl, WhereBindingDecl, WhereBlockDecl, WithBlockDecl, WithOptionDecl, WriteDecl,
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
    Function,
    Args,
    Struct,
    System,
    Domain,
    Component,
    Equation,
    Export,
    Where,
    With,
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
    pub imports: usize,
    pub functions: usize,
    pub schemas: usize,
    pub systems: usize,
    pub domains: usize,
    pub domain_variables: usize,
    pub components: usize,
    pub ports: usize,
    pub connections: usize,
    pub structs: usize,
    pub args_blocks: usize,
    pub args_fields: usize,
    pub const_declarations: usize,
    pub equations: usize,
    pub fast_bindings: usize,
    pub explicit_declarations: usize,
    pub command_styles: usize,
    pub where_blocks: usize,
    pub with_blocks: usize,
}

impl ParsedProgram {
    pub fn summary(&self) -> SyntaxSummary {
        let mut scripts = 0usize;
        let mut imports = 0usize;
        let mut functions = 0usize;
        let mut schemas = 0usize;
        let mut systems = 0usize;
        let mut domains = 0usize;
        let mut domain_variables = 0usize;
        let mut components = 0usize;
        let mut ports = 0usize;
        let mut connections = 0usize;
        let mut structs = 0usize;
        let mut args_blocks = 0usize;
        let mut args_fields = 0usize;
        let mut const_declarations = 0usize;
        let mut equations = 0usize;
        let mut fast_bindings = 0usize;
        let mut explicit_declarations = 0usize;
        let mut command_styles = 0usize;
        let mut where_blocks = 0usize;
        let mut with_blocks = 0usize;

        for item in &self.items {
            match item {
                AstItem::Script(_) => scripts += 1,
                AstItem::Import(_) => imports += 1,
                AstItem::Function(_) => functions += 1,
                AstItem::Schema(_) => schemas += 1,
                AstItem::System(_) => systems += 1,
                AstItem::Domain(_) => domains += 1,
                AstItem::DomainVariable(_) => domain_variables += 1,
                AstItem::Component(_) => components += 1,
                AstItem::Port(_) => ports += 1,
                AstItem::Connect(_) => connections += 1,
                AstItem::Struct(_) => structs += 1,
                AstItem::Args(_) => args_blocks += 1,
                AstItem::ArgsField(_) => args_fields += 1,
                AstItem::Const(_) => const_declarations += 1,
                AstItem::Equation(_) => equations += 1,
                AstItem::FastBinding(_) => fast_bindings += 1,
                AstItem::ExplicitDecl(_) => explicit_declarations += 1,
                AstItem::CommandStyle(_) => command_styles += 1,
                AstItem::WhereBlock(_) => where_blocks += 1,
                AstItem::WithBlock(_) => with_blocks += 1,
                AstItem::SystemVariable(_)
                | AstItem::Return(_)
                | AstItem::Conservation(_)
                | AstItem::Constraint(_)
                | AstItem::MissingPolicy(_)
                | AstItem::Summary(_)
                | AstItem::Print(_)
                | AstItem::CsvExport(_)
                | AstItem::CsvExportField(_)
                | AstItem::Write(_)
                | AstItem::FileOperation(_)
                | AstItem::WhereBinding(_)
                | AstItem::WithOption(_)
                | AstItem::ReservedKeywordUse { .. } => {}
            }
        }

        SyntaxSummary {
            lines: self.lines.len(),
            tokens: self.lines.iter().map(|line| line.tokens.len()).sum(),
            ast_items: self.items.len(),
            scripts,
            imports,
            functions,
            schemas,
            systems,
            domains,
            domain_variables,
            components,
            ports,
            connections,
            structs,
            args_blocks,
            args_fields,
            const_declarations,
            equations,
            fast_bindings,
            explicit_declarations,
            command_styles,
            where_blocks,
            with_blocks,
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
    let mut function_depth = 0i32;
    let mut args_depth = 0i32;
    let mut struct_depth = 0i32;
    let mut system_depth = 0i32;
    let mut domain_depth = 0i32;
    let mut component_depth = 0i32;
    let mut equation_depth = 0i32;
    let mut export_depth = 0i32;
    let mut where_depth = 0i32;
    let mut with_depth = 0i32;
    let mut current_where_owner_line = None;
    let mut current_with_owner_line = None;
    let mut last_attachable_line = None;

    for source_line in source_lines(source) {
        let tokens = lex_line(source_line.line, source_line.start, &source_line.text);
        let context = if equation_depth > 0 {
            ParseContext::Equation
        } else if export_depth > 0 {
            ParseContext::Export
        } else if where_depth > 0 {
            ParseContext::Where
        } else if with_depth > 0 {
            ParseContext::With
        } else if missing_depth > 0 {
            ParseContext::SchemaMissing
        } else if constraints_depth > 0 {
            ParseContext::SchemaConstraints
        } else if schema_depth > 0 {
            ParseContext::Schema
        } else if script_depth > 0 {
            ParseContext::Script
        } else if function_depth > 0 {
            ParseContext::Function
        } else if args_depth > 0 {
            ParseContext::Args
        } else if struct_depth > 0 {
            ParseContext::Struct
        } else if domain_depth > 0 {
            ParseContext::Domain
        } else if component_depth > 0 {
            ParseContext::Component
        } else if system_depth > 0 {
            ParseContext::System
        } else {
            ParseContext::TopLevel
        };

        if !tokens.is_empty() {
            let owner_line = match context {
                ParseContext::Where => current_where_owner_line,
                ParseContext::With => current_with_owner_line,
                _ => last_attachable_line,
            };
            parse_line_items(&mut items, &tokens, &source_line.text, context, owner_line);
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

        if starts_with_keyword(&tokens, Keyword::Fn) {
            let delta = brace_delta(&tokens);
            if delta != 0 {
                function_depth += delta;
            } else if !tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::Symbol(Symbol::Equal)))
            {
                function_depth = 1;
            }
        } else if function_depth > 0 {
            function_depth += brace_delta(&tokens);
            if function_depth <= 0 {
                function_depth = 0;
            }
        }

        if starts_with_identifier(&tokens, "args") {
            args_depth += brace_delta(&tokens);
            if args_depth == 0 {
                args_depth = 1;
            }
        } else if args_depth > 0 {
            args_depth += brace_delta(&tokens);
            if args_depth <= 0 {
                args_depth = 0;
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

        if starts_with_keyword(&tokens, Keyword::Domain) {
            domain_depth += brace_delta(&tokens);
            if domain_depth == 0 {
                domain_depth = 1;
            }
        } else if domain_depth > 0 {
            domain_depth += brace_delta(&tokens);
            if domain_depth <= 0 {
                domain_depth = 0;
            }
        }

        if starts_with_keyword(&tokens, Keyword::Component) {
            component_depth += brace_delta(&tokens);
            if component_depth == 0 {
                component_depth = 1;
            }
        } else if component_depth > 0 {
            component_depth += brace_delta(&tokens);
            if component_depth <= 0 {
                component_depth = 0;
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

        if starts_with_keyword(&tokens, Keyword::Export) {
            export_depth += brace_delta(&tokens);
            if export_depth == 0 {
                export_depth = 1;
            }
        } else if export_depth > 0 {
            export_depth += brace_delta(&tokens);
            if export_depth <= 0 {
                export_depth = 0;
            }
        }

        if starts_with_keyword(&tokens, Keyword::Where) {
            current_where_owner_line = last_attachable_line;
            let delta = brace_delta(&tokens);
            if delta != 0 {
                where_depth += delta;
            } else if !(contains_symbol(&tokens, Symbol::LBrace)
                && contains_symbol(&tokens, Symbol::RBrace))
            {
                where_depth = 1;
            }
        } else if where_depth > 0 {
            where_depth += brace_delta(&tokens);
            if where_depth <= 0 {
                where_depth = 0;
                current_where_owner_line = None;
            }
        }

        if starts_with_keyword(&tokens, Keyword::With) {
            current_with_owner_line = last_attachable_line;
            let delta = brace_delta(&tokens);
            if delta != 0 {
                with_depth += delta;
            } else if !(contains_symbol(&tokens, Symbol::LBrace)
                && contains_symbol(&tokens, Symbol::RBrace))
            {
                with_depth = 1;
            }
        } else if with_depth > 0 {
            with_depth += brace_delta(&tokens);
            if with_depth <= 0 {
                with_depth = 0;
                current_with_owner_line = None;
            }
        }

        if line_is_attachable_owner(&tokens, context) {
            last_attachable_line = Some(source_line.line);
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
    owner_line: Option<usize>,
) {
    if let Some(import) = parse_import_decl(tokens) {
        items.push(AstItem::Import(import));
    }
    if let Some(schema) = parse_schema_decl(tokens) {
        items.push(AstItem::Schema(schema));
    }
    if let Some(script) = parse_script_decl(tokens) {
        items.push(AstItem::Script(script));
    }
    if let Some(function) = parse_function_decl(tokens, line_text) {
        items.push(AstItem::Function(function));
        if let Some(return_decl) = parse_inline_function_return_decl(tokens, line_text) {
            items.push(AstItem::Return(return_decl));
        }
    }
    if let Some(args) = parse_args_decl(tokens) {
        items.push(AstItem::Args(args));
    }
    if let Some(const_decl) = parse_const_decl(tokens, line_text, context) {
        items.push(AstItem::Const(const_decl));
    }
    if let Some(return_decl) = parse_return_decl(tokens, line_text, context) {
        items.push(AstItem::Return(return_decl));
    }
    if let Some(struct_decl) = parse_struct_decl(tokens) {
        items.push(AstItem::Struct(struct_decl));
    }
    if let Some(field) = parse_args_field_decl(tokens, line_text, context) {
        items.push(AstItem::ArgsField(field));
    }
    if let Some(system) = parse_system_decl(tokens) {
        items.push(AstItem::System(system));
    }
    if let Some(domain) = parse_domain_decl(tokens) {
        items.push(AstItem::Domain(domain));
    }
    if let Some(variable) = parse_domain_variable_decl(tokens, line_text, context) {
        items.push(AstItem::DomainVariable(variable));
    }
    if let Some(conservation) = parse_conservation_decl(tokens, line_text, context) {
        items.push(AstItem::Conservation(conservation));
    }
    if let Some(component) = parse_component_decl(tokens) {
        items.push(AstItem::Component(component));
    }
    if let Some(port) = parse_port_decl(tokens, line_text, context) {
        items.push(AstItem::Port(port));
    }
    if let Some(connect) = parse_connect_decl(tokens, line_text) {
        items.push(AstItem::Connect(connect));
    }
    if let Some(variable) = parse_system_variable_decl(tokens, line_text, context) {
        items.push(AstItem::SystemVariable(variable));
    }
    if let Some(equation) = parse_equation_decl(tokens, line_text, context) {
        items.push(AstItem::Equation(equation));
    }
    if let Some(block) = parse_where_block_decl(tokens, owner_line) {
        items.push(AstItem::WhereBlock(block));
    }
    if let Some(block) = parse_with_block_decl(tokens, owner_line) {
        items.push(AstItem::WithBlock(block));
        for option in parse_inline_with_options(tokens, line_text, owner_line) {
            items.push(AstItem::WithOption(option));
        }
    }
    if let Some(binding) = parse_where_binding_decl(tokens, line_text, owner_line, context) {
        items.push(AstItem::WhereBinding(binding));
    }
    if let Some(option) = parse_with_option_decl(tokens, line_text, owner_line, context) {
        items.push(AstItem::WithOption(option));
    }
    if let Some((binding, command)) = parse_fast_binding(tokens, line_text, context) {
        if let Some(command) = command {
            items.push(AstItem::CommandStyle(command));
        }
        items.push(AstItem::FastBinding(binding));
    }
    if !matches!(
        context,
        ParseContext::Args
            | ParseContext::Struct
            | ParseContext::SchemaConstraints
            | ParseContext::SchemaMissing
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
    if let Some(print) = parse_print_decl(tokens, context) {
        items.push(AstItem::Print(print));
    }
    if let Some(export) = parse_csv_export_decl(tokens, context) {
        items.push(AstItem::CsvExport(export));
    }
    if let Some(field) = parse_csv_export_field_decl(tokens, line_text, context) {
        items.push(AstItem::CsvExportField(field));
    }
    if let Some(write) = parse_write_decl(tokens, line_text, context) {
        items.push(AstItem::Write(write));
    }
    if let Some(operation) = parse_file_operation_decl(tokens, line_text, context) {
        items.push(AstItem::FileOperation(operation));
    }
    if let Some(command) = parse_standalone_command_style_decl(tokens, line_text, context) {
        items.push(AstItem::CommandStyle(command));
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

fn parse_import_decl(tokens: &[Token]) -> Option<ImportDecl> {
    let first = tokens.first()?;
    let kind = match first.kind {
        TokenKind::Keyword(Keyword::Use) => "use",
        TokenKind::Keyword(Keyword::Import) => "import",
        _ => return None,
    };
    if let Some(Token {
        kind: TokenKind::StringLiteral(target),
        ..
    }) = tokens.get(1)
    {
        return Some(ImportDecl {
            target: target.clone(),
            kind: "file".to_owned(),
            line: first.span.line,
            span: first.span,
        });
    }

    let mut target = String::new();
    for token in tokens.iter().skip(1) {
        match &token.kind {
            TokenKind::Identifier(value) => target.push_str(value),
            TokenKind::Keyword(_) => target.push_str(&token.lexeme),
            TokenKind::Symbol(Symbol::Dot) => target.push('.'),
            _ => break,
        }
    }
    (!target.is_empty()).then(|| ImportDecl {
        target,
        kind: kind.to_owned(),
        line: first.span.line,
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

fn parse_domain_decl(tokens: &[Token]) -> Option<DomainDecl> {
    let [first, second, ..] = tokens else {
        return None;
    };
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Domain)) {
        return None;
    }
    let TokenKind::Identifier(name) = &second.kind else {
        return None;
    };
    Some(DomainDecl {
        name: name.clone(),
        type_parameters: parse_domain_type_parameters_after(tokens, 2),
        package: parse_metadata_value(tokens, "package"),
        version: parse_metadata_value(tokens, "version"),
        span: first.span,
    })
}

fn parse_domain_type_parameters_after(
    tokens: &[Token],
    start_index: usize,
) -> Vec<DomainTypeParameterDecl> {
    let Some(open_index) =
        tokens
            .iter()
            .enumerate()
            .skip(start_index)
            .find_map(|(index, token)| {
                matches!(token.kind, TokenKind::Symbol(Symbol::LBracket)).then_some(index)
            })
    else {
        return Vec::new();
    };
    let Some(close_index) =
        tokens
            .iter()
            .enumerate()
            .skip(open_index + 1)
            .find_map(|(index, token)| {
                matches!(token.kind, TokenKind::Symbol(Symbol::RBracket)).then_some(index)
            })
    else {
        return Vec::new();
    };

    let mut parameters = Vec::new();
    let mut group = Vec::new();
    for token in &tokens[open_index + 1..close_index] {
        if matches!(token.kind, TokenKind::Symbol(Symbol::Comma)) {
            push_domain_type_parameter(&mut parameters, &group);
            group.clear();
            continue;
        }
        if let TokenKind::Identifier(value) = &token.kind {
            group.push(value.clone());
        }
    }
    push_domain_type_parameter(&mut parameters, &group);
    parameters
}

fn push_domain_type_parameter(parameters: &mut Vec<DomainTypeParameterDecl>, group: &[String]) {
    let Some(kind) = group.first() else {
        return;
    };
    let name = group.get(1).cloned().unwrap_or_else(|| kind.clone());
    parameters.push(DomainTypeParameterDecl {
        kind: kind.clone(),
        name,
    });
}

fn parse_metadata_value(tokens: &[Token], key: &str) -> Option<String> {
    tokens.windows(2).find_map(|window| {
        let [left, right] = window else {
            return None;
        };
        if !metadata_key_matches(left, key) {
            return None;
        }
        match &right.kind {
            TokenKind::StringLiteral(value)
            | TokenKind::Identifier(value)
            | TokenKind::Number(value) => Some(value.clone()),
            _ => None,
        }
    })
}

fn metadata_key_matches(token: &Token, key: &str) -> bool {
    match (&token.kind, key) {
        (TokenKind::Identifier(identifier), expected) => identifier == expected,
        (TokenKind::Keyword(Keyword::Package), "package") => true,
        (TokenKind::Keyword(Keyword::Version), "version") => true,
        _ => false,
    }
}

fn parse_component_decl(tokens: &[Token]) -> Option<ComponentDecl> {
    let [first, second, ..] = tokens else {
        return None;
    };
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Component)) {
        return None;
    }
    let TokenKind::Identifier(name) = &second.kind else {
        return None;
    };
    Some(ComponentDecl {
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

fn parse_args_decl(tokens: &[Token]) -> Option<ArgsDecl> {
    let first = tokens.first()?;
    let TokenKind::Identifier(name) = &first.kind else {
        return None;
    };
    if name != "args" {
        return None;
    }
    if !tokens
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Symbol(Symbol::LBrace)))
    {
        return None;
    }
    Some(ArgsDecl {
        name: "Args".to_owned(),
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

fn parse_const_decl(tokens: &[Token], line_text: &str, context: ParseContext) -> Option<ConstDecl> {
    let [first, second, third, ..] = tokens else {
        return None;
    };
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Const)) {
        return None;
    }
    let TokenKind::Identifier(name) = &second.kind else {
        return None;
    };
    if !matches!(third.kind, TokenKind::Symbol(Symbol::Colon)) {
        return None;
    }
    let raw_after_colon = line_text.split_once(':')?.1.trim();
    let (type_part, expression) = raw_after_colon.split_once('=')?;
    let (type_name, unit) = split_type_and_unit(type_part.trim());
    let expression = expression.trim();
    if type_name.is_empty() || expression.is_empty() {
        return None;
    }
    Some(ConstDecl {
        name: name.clone(),
        type_name,
        unit,
        expression: expression.to_owned(),
        line: first.span.line,
        span: first.span,
        context,
    })
}

fn parse_function_decl(tokens: &[Token], line_text: &str) -> Option<FunctionDecl> {
    let [first, second, ..] = tokens else {
        return None;
    };
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Fn)) {
        return None;
    }
    let TokenKind::Identifier(name) = &second.kind else {
        return None;
    };
    let parameters = parse_function_parameters(line_text);
    let (return_type, return_unit) = parse_function_return(line_text)?;
    Some(FunctionDecl {
        name: name.clone(),
        parameters,
        return_type,
        return_unit,
        span: first.span,
    })
}

fn parse_inline_function_return_decl(tokens: &[Token], line_text: &str) -> Option<ReturnDecl> {
    let first = tokens.first()?;
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Fn)) {
        return None;
    }
    let expression = line_text.split_once('=').map(|(_, right)| right.trim())?;
    if expression.is_empty() || expression.starts_with('{') {
        return None;
    }
    Some(ReturnDecl {
        expression: expression.to_owned(),
        line: first.span.line,
        span: first.span,
        context: ParseContext::Function,
    })
}

fn parse_function_parameters(line_text: &str) -> Vec<FunctionParamDecl> {
    let Some(open) = line_text.find('(') else {
        return Vec::new();
    };
    let Some(close_offset) = line_text[open + 1..].find(')') else {
        return Vec::new();
    };
    let close = open + 1 + close_offset;
    line_text[open + 1..close]
        .split(',')
        .filter_map(parse_function_parameter)
        .collect()
}

fn parse_function_parameter(raw: &str) -> Option<FunctionParamDecl> {
    let (name, type_part) = raw.split_once(':')?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let (type_name, unit) = split_type_and_unit(type_part.trim());
    if type_name.is_empty() {
        return None;
    }
    Some(FunctionParamDecl {
        name: name.to_owned(),
        type_name,
        unit,
    })
}

fn parse_function_return(line_text: &str) -> Option<(String, Option<String>)> {
    let (_, after_arrow) = line_text.split_once("->")?;
    let return_part = after_arrow
        .split_once('{')
        .map(|(left, _)| left)
        .or_else(|| after_arrow.split_once('=').map(|(left, _)| left))
        .unwrap_or(after_arrow)
        .trim();
    if return_part.is_empty() {
        return None;
    }
    Some(split_type_and_unit(return_part))
}

fn parse_return_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<ReturnDecl> {
    if context != ParseContext::Function {
        return None;
    }
    let first = tokens.first()?;
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Return)) {
        return None;
    }
    Some(ReturnDecl {
        expression: line_text
            .trim()
            .strip_prefix("return")
            .unwrap_or(line_text.trim())
            .trim()
            .to_owned(),
        line: first.span.line,
        span: first.span,
        context,
    })
}

fn parse_args_field_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<ArgsFieldDecl> {
    if context != ParseContext::Args {
        return None;
    }
    let [first, second, third, ..] = tokens else {
        return None;
    };
    let name = token_field_name(first)?;
    if !matches!(second.kind, TokenKind::Symbol(Symbol::Colon)) {
        return None;
    }
    let type_name = token_type_name(third)?;
    let default_value = line_text
        .split_once('=')
        .map(|(_, right)| right.trim().to_owned());

    Some(ArgsFieldDecl {
        name,
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

fn token_field_name(token: &Token) -> Option<String> {
    match &token.kind {
        TokenKind::Identifier(value) => Some(value.clone()),
        TokenKind::Keyword(Keyword::Input) => Some("input".to_owned()),
        TokenKind::Keyword(Keyword::Output) => Some("output".to_owned()),
        _ => None,
    }
}

fn parse_domain_variable_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<DomainVariableDecl> {
    if context != ParseContext::Domain {
        return None;
    }
    let [first, second, third, ..] = tokens else {
        return None;
    };
    let role = match first.kind {
        TokenKind::Keyword(Keyword::Across) => "across",
        TokenKind::Keyword(Keyword::Through) => "through",
        _ => return None,
    };
    let TokenKind::Identifier(name) = &second.kind else {
        return None;
    };
    if !matches!(third.kind, TokenKind::Symbol(Symbol::Colon)) {
        return None;
    }
    let raw_after_colon = line_text.split_once(':')?.1.trim();
    let (type_name, unit) = split_type_and_unit(raw_after_colon);

    Some(DomainVariableDecl {
        role: role.to_owned(),
        name: name.clone(),
        type_name,
        unit,
        line: first.span.line,
        span: first.span,
    })
}

fn parse_conservation_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<ConservationDecl> {
    if context != ParseContext::Domain {
        return None;
    }
    let first = tokens.first()?;
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Conservation)) {
        return None;
    }
    Some(ConservationDecl {
        text: line_text
            .trim()
            .strip_prefix("conservation")
            .unwrap_or(line_text.trim())
            .trim()
            .to_owned(),
        line: first.span.line,
        span: first.span,
    })
}

fn parse_port_decl(tokens: &[Token], line_text: &str, context: ParseContext) -> Option<PortDecl> {
    if context != ParseContext::Component {
        return None;
    }
    let [first, second, third, ..] = tokens else {
        return None;
    };
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Port)) {
        return None;
    }
    let TokenKind::Identifier(name) = &second.kind else {
        return None;
    };
    if !matches!(third.kind, TokenKind::Symbol(Symbol::Colon)) {
        return None;
    }
    let domain = line_text.split_once(':')?.1.trim().to_owned();
    Some(PortDecl {
        name: name.clone(),
        domain,
        line: first.span.line,
        span: first.span,
    })
}

fn parse_connect_decl(tokens: &[Token], line_text: &str) -> Option<ConnectDecl> {
    let first = tokens.first()?;
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Connect)) {
        return None;
    }
    let raw = line_text
        .trim()
        .strip_prefix("connect")
        .unwrap_or(line_text.trim())
        .trim();
    let (left, right) = raw.split_once("->")?;
    Some(ConnectDecl {
        left: left.trim().to_owned(),
        right: right.trim().to_owned(),
        line: first.span.line,
        span: first.span,
    })
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
) -> Option<(FastBinding, Option<CommandStyleDecl>)> {
    if matches!(context, ParseContext::Where | ParseContext::With) {
        return None;
    }
    let [first, second, ..] = tokens else {
        return None;
    };
    let TokenKind::Identifier(name) = &first.kind else {
        return None;
    };
    if !matches!(second.kind, TokenKind::Symbol(Symbol::Equal)) {
        return None;
    }
    let expression = expression_after(line_text, '=')?;
    let command = parse_command_style_expression(&expression, first.span, context, Some(name));
    let expression = command
        .as_ref()
        .map(|command| command.canonical.clone())
        .unwrap_or(expression);
    Some((
        FastBinding {
            name: name.clone(),
            expression,
            line: first.span.line,
            span: first.span,
            context,
        },
        command,
    ))
}

fn parse_where_block_decl(tokens: &[Token], owner_line: Option<usize>) -> Option<WhereBlockDecl> {
    let first = tokens.first()?;
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Where)) {
        return None;
    }
    Some(WhereBlockDecl {
        owner_line,
        line: first.span.line,
        span: first.span,
    })
}

fn parse_where_binding_decl(
    tokens: &[Token],
    line_text: &str,
    owner_line: Option<usize>,
    context: ParseContext,
) -> Option<WhereBindingDecl> {
    if context != ParseContext::Where {
        return None;
    }
    let [first, second, ..] = tokens else {
        return None;
    };
    let TokenKind::Identifier(name) = &first.kind else {
        return None;
    };
    if !matches!(second.kind, TokenKind::Symbol(Symbol::Equal)) {
        return None;
    }
    let expression = expression_after(line_text, '=')?;
    let expression = parse_command_style_expression(&expression, first.span, context, Some(name))
        .map(|command| command.canonical)
        .unwrap_or(expression);
    Some(WhereBindingDecl {
        owner_line,
        name: name.clone(),
        expression,
        line: first.span.line,
        span: first.span,
    })
}

fn parse_with_block_decl(tokens: &[Token], owner_line: Option<usize>) -> Option<WithBlockDecl> {
    let first = tokens.first()?;
    if !matches!(first.kind, TokenKind::Keyword(Keyword::With)) {
        return None;
    }
    Some(WithBlockDecl {
        owner_line,
        line: first.span.line,
        span: first.span,
    })
}

fn parse_with_option_decl(
    tokens: &[Token],
    line_text: &str,
    owner_line: Option<usize>,
    context: ParseContext,
) -> Option<WithOptionDecl> {
    if context != ParseContext::With {
        return None;
    }
    let first = tokens.first()?;
    if matches!(
        &first.kind,
        TokenKind::Symbol(Symbol::LBrace | Symbol::RBrace)
    ) {
        return None;
    }
    parse_with_option_text(
        line_text.trim().trim_end_matches(','),
        first.span,
        owner_line,
    )
}

fn parse_inline_with_options(
    tokens: &[Token],
    line_text: &str,
    owner_line: Option<usize>,
) -> Vec<WithOptionDecl> {
    let Some(first) = tokens.first() else {
        return Vec::new();
    };
    if !matches!(first.kind, TokenKind::Keyword(Keyword::With)) {
        return Vec::new();
    }
    let Some((_, after_open)) = line_text.split_once('{') else {
        return Vec::new();
    };
    let Some((inside, _)) = after_open.rsplit_once('}') else {
        return Vec::new();
    };
    inside
        .split([';', ','])
        .filter_map(|part| parse_with_option_text(part.trim(), first.span, owner_line))
        .collect()
}

fn parse_with_option_text(
    text: &str,
    span: SourceSpan,
    owner_line: Option<usize>,
) -> Option<WithOptionDecl> {
    if text.is_empty() {
        return None;
    }
    let (key, value) = if let Some(rest) = text.strip_prefix("unit ") {
        let (axis, value) = rest.split_once('=')?;
        (format!("unit {}", axis.trim()), value.trim().to_owned())
    } else {
        let (key, value) = text.split_once('=')?;
        (key.trim().to_owned(), value.trim().to_owned())
    };
    if key.is_empty() || value.is_empty() {
        return None;
    }
    Some(WithOptionDecl {
        owner_line,
        key,
        value: strip_wrapping_quotes(&value),
        line: span.line,
        span,
    })
}

fn parse_standalone_command_style_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<CommandStyleDecl> {
    if matches!(
        context,
        ParseContext::Where | ParseContext::With | ParseContext::Export
    ) {
        return None;
    }
    let first = tokens.first()?;
    parse_command_style_expression(line_text.trim(), first.span, context, None)
}

fn parse_command_style_expression(
    expression: &str,
    span: SourceSpan,
    context: ParseContext,
    owner: Option<&String>,
) -> Option<CommandStyleDecl> {
    let trimmed = expression.trim().trim_end_matches('{').trim();
    let (verb, rest) = split_first_word(trimmed)?;
    if !is_command_style_verb(verb) {
        return None;
    }
    if trimmed.starts_with(&format!("{verb}(")) {
        return None;
    }

    let (target, clauses) = split_command_target_and_clauses(rest);
    let target = target.trim();
    let status = if target.is_empty() {
        "missing_target"
    } else if command_target_is_ambiguous(target) {
        "ambiguous_target"
    } else {
        "lowered"
    };
    let canonical_target = target.trim();
    let canonical = canonical_command_call(verb, canonical_target, &clauses);
    Some(CommandStyleDecl {
        verb: verb.to_owned(),
        target: canonical_target.to_owned(),
        clauses: clauses
            .iter()
            .map(|(name, value)| CommandClauseDecl {
                name: name.clone(),
                value: value.clone(),
            })
            .collect(),
        canonical,
        status: status.to_owned(),
        owner: owner.cloned(),
        line: span.line,
        span,
        context,
    })
}

fn split_first_word(value: &str) -> Option<(&str, &str)> {
    let trimmed = value.trim_start();
    let end = trimmed
        .char_indices()
        .find_map(|(index, character)| character.is_whitespace().then_some(index))
        .unwrap_or(trimmed.len());
    if end == 0 {
        return None;
    }
    Some((&trimmed[..end], trimmed[end..].trim_start()))
}

fn is_command_style_verb(verb: &str) -> bool {
    matches!(
        verb,
        "integrate" | "mean" | "max" | "min" | "duration" | "plot" | "show" | "validate"
    )
}

fn split_command_target_and_clauses(rest: &str) -> (String, Vec<(String, String)>) {
    let positions = top_level_clause_positions(
        rest,
        &[
            "over", "by", "as", "above", "below", "between", "from", "to", "with",
        ],
    );
    if positions.is_empty() {
        return (rest.trim().to_owned(), Vec::new());
    }

    let target = rest[..positions[0].0].trim().to_owned();
    let mut clauses = Vec::new();
    for (index, (start, name)) in positions.iter().enumerate() {
        let value_start = start + name.len();
        let value_end = positions
            .get(index + 1)
            .map(|(next_start, _)| *next_start)
            .unwrap_or(rest.len());
        let value = rest[value_start..value_end].trim();
        if !value.is_empty() {
            clauses.push(((*name).to_owned(), value.to_owned()));
        }
    }
    (target, clauses)
}

fn top_level_clause_positions<'a>(text: &str, keywords: &[&'a str]) -> Vec<(usize, &'a str)> {
    let mut positions = Vec::new();
    let mut depth = 0i32;
    let mut bracket_depth = 0i32;
    let mut in_string = false;
    for (index, character) in text.char_indices() {
        match character {
            '"' => in_string = !in_string,
            '(' if !in_string => depth += 1,
            ')' if !in_string => depth -= 1,
            '[' if !in_string => bracket_depth += 1,
            ']' if !in_string => bracket_depth -= 1,
            _ => {}
        }
        if in_string || depth != 0 || bracket_depth != 0 {
            continue;
        }
        for keyword in keywords {
            if starts_with_word_at(text, index, keyword) {
                positions.push((index, *keyword));
            }
        }
    }
    positions.sort_by_key(|(index, _)| *index);
    positions.dedup_by_key(|(index, _)| *index);
    positions
}

fn starts_with_word_at(text: &str, index: usize, word: &str) -> bool {
    if !text[index..].starts_with(word) {
        return false;
    }
    let before_ok = index == 0
        || text[..index]
            .chars()
            .next_back()
            .is_some_and(|character| !is_word_character(character));
    let after_index = index + word.len();
    let after_ok = after_index >= text.len()
        || text[after_index..]
            .chars()
            .next()
            .is_some_and(|character| !is_word_character(character));
    before_ok && after_ok
}

fn is_word_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn command_target_is_ambiguous(target: &str) -> bool {
    let target = target.trim();
    if target.starts_with('(') && target.ends_with(')') && balanced_delimiters(target) {
        return false;
    }
    if target.split_whitespace().count() > 1 {
        return true;
    }
    target
        .chars()
        .any(|character| matches!(character, '+' | '-' | '*' | '/'))
}

fn canonical_command_call(verb: &str, target: &str, clauses: &[(String, String)]) -> String {
    let mut args = vec![target.to_owned()];
    for (name, value) in clauses {
        let canonical_name = match (verb, name.as_str()) {
            ("mean" | "max" | "min", "over") => "axis",
            _ => name,
        };
        args.push(format!("{canonical_name}={value}"));
    }
    format!("{verb}({})", args.join(", "))
}

fn balanced_delimiters(value: &str) -> bool {
    let mut parens = 0i32;
    let mut brackets = 0i32;
    let mut in_string = false;
    for character in value.chars() {
        match character {
            '"' => in_string = !in_string,
            '(' if !in_string => parens += 1,
            ')' if !in_string => {
                parens -= 1;
                if parens < 0 {
                    return false;
                }
            }
            '[' if !in_string => brackets += 1,
            ']' if !in_string => {
                brackets -= 1;
                if brackets < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    parens == 0 && brackets == 0 && !in_string
}

fn strip_wrapping_quotes(value: &str) -> String {
    let trimmed = value.trim();
    trimmed
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(trimmed)
        .to_owned()
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

fn parse_print_decl(tokens: &[Token], context: ParseContext) -> Option<PrintDecl> {
    let first = tokens.first()?;
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Print)) {
        return None;
    }
    let template = tokens.iter().skip(1).find_map(|token| match &token.kind {
        TokenKind::StringLiteral(value) => Some(value.clone()),
        _ => None,
    })?;
    Some(PrintDecl {
        template,
        line: first.span.line,
        span: first.span,
        context,
    })
}

fn parse_csv_export_decl(tokens: &[Token], context: ParseContext) -> Option<CsvExportDecl> {
    let [first, second, third, fourth, fifth, ..] = tokens else {
        return None;
    };
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Export)) {
        return None;
    }
    let TokenKind::Identifier(source) = &second.kind else {
        return None;
    };
    if !matches!(third.kind, TokenKind::Keyword(Keyword::To)) {
        return None;
    }
    if !matches!(fourth.kind, TokenKind::Keyword(Keyword::Csv)) {
        return None;
    }
    let TokenKind::StringLiteral(path) = &fifth.kind else {
        return None;
    };

    Some(CsvExportDecl {
        source: source.clone(),
        format: "csv".to_owned(),
        path: path.clone(),
        line: first.span.line,
        span: first.span,
        context,
    })
}

fn parse_csv_export_field_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<CsvExportFieldDecl> {
    if context != ParseContext::Export {
        return None;
    }
    let first = tokens.first()?;
    if matches!(
        &first.kind,
        TokenKind::Symbol(Symbol::LBrace | Symbol::RBrace)
    ) {
        return None;
    }

    let raw = line_text.trim().trim_end_matches(',');
    let (expression, rest) = raw.split_once(" as ")?;
    let (display_unit, format) = rest
        .split_once(" with ")
        .map(|(unit, format)| (unit.trim().to_owned(), extract_quoted(format)))
        .unwrap_or_else(|| (rest.trim().to_owned(), None));
    let expression = expression.trim();
    if expression.is_empty() || display_unit.trim().is_empty() {
        return None;
    }
    let expression = parse_command_style_expression(expression, first.span, context, None)
        .map(|command| command.canonical)
        .unwrap_or_else(|| expression.to_owned());

    Some(CsvExportFieldDecl {
        expression,
        display_unit: Some(display_unit),
        format,
        line: first.span.line,
        span: first.span,
    })
}

fn parse_write_decl(tokens: &[Token], line_text: &str, context: ParseContext) -> Option<WriteDecl> {
    let first = tokens.first()?;
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Write)) {
        return None;
    }
    let raw = line_text.trim();
    let rest = raw.strip_prefix("write ")?.trim();
    let (format, rest) = rest.split_once(char::is_whitespace)?;
    let format = format.trim();
    if !matches!(format, "text" | "json") {
        return None;
    }
    let (path, expression) = rest.trim().split_once(',')?;
    let path = path.trim();
    let expression = expression.trim();
    if path.is_empty() || expression.is_empty() {
        return None;
    }
    Some(WriteDecl {
        format: format.to_owned(),
        path: path.to_owned(),
        expression: expression.to_owned(),
        line: first.span.line,
        span: first.span,
        context,
    })
}

fn parse_file_operation_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<FileOperationDecl> {
    let first = tokens.first()?;
    let operation = match first.kind {
        TokenKind::Keyword(Keyword::Copy) => "copy",
        TokenKind::Keyword(Keyword::Move) => "move",
        TokenKind::Keyword(Keyword::Delete) => "delete",
        _ => return None,
    };
    let rest = line_text.trim().strip_prefix(operation)?.trim();
    if rest.is_empty() {
        return None;
    }
    let (source, destination) = if matches!(operation, "copy" | "move") {
        let (source, destination) = split_file_operation_to(rest)?;
        (
            source.trim().to_owned(),
            Some(destination.trim().to_owned()),
        )
    } else {
        (rest.to_owned(), None)
    };
    if source.is_empty() || destination.as_deref().is_some_and(str::is_empty) {
        return None;
    }
    Some(FileOperationDecl {
        operation: operation.to_owned(),
        source,
        destination,
        line: first.span.line,
        span: first.span,
        context,
    })
}

fn split_file_operation_to(rest: &str) -> Option<(&str, &str)> {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in rest.char_indices() {
        if in_string {
            escaped = character == '\\' && !escaped;
            if character == '"' && !escaped {
                in_string = false;
            }
            if character != '\\' {
                escaped = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            't' if depth == 0 && rest[index..].starts_with("to") => {
                let before_raw = &rest[..index];
                let before = before_raw.trim_end();
                let after_index = index + 2;
                let after = rest[after_index..].trim_start();
                let valid_before = before_raw
                    .chars()
                    .last()
                    .is_some_and(|value| value.is_whitespace());
                let valid_after = rest[after_index..]
                    .chars()
                    .next()
                    .is_some_and(|value| value.is_whitespace());
                if valid_before && valid_after && !before.is_empty() && !after.is_empty() {
                    return Some((before, after));
                }
            }
            _ => {}
        }
    }
    None
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

fn extract_quoted(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if let Some(inner) = trimmed
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        return Some(inner.to_owned());
    }
    let start = trimmed.find('"')?;
    let tail = &trimmed[start + 1..];
    let end = tail.find('"')?;
    Some(tail[..end].to_owned())
}

fn starts_with_keyword(tokens: &[Token], keyword: Keyword) -> bool {
    tokens
        .first()
        .is_some_and(|token| matches!(token.kind, TokenKind::Keyword(found) if found == keyword))
}

fn starts_with_identifier(tokens: &[Token], expected: &str) -> bool {
    tokens.first().is_some_and(
        |token| matches!(&token.kind, TokenKind::Identifier(found) if found == expected),
    )
}

fn line_is_attachable_owner(tokens: &[Token], context: ParseContext) -> bool {
    if !matches!(context, ParseContext::TopLevel | ParseContext::Other) {
        return false;
    }
    let Some(first) = tokens.first() else {
        return false;
    };
    if matches!(
        first.kind,
        TokenKind::Keyword(Keyword::Where | Keyword::With)
            | TokenKind::Symbol(Symbol::LBrace | Symbol::RBrace)
    ) {
        return false;
    }
    matches!(
        first.kind,
        TokenKind::Identifier(_)
            | TokenKind::Keyword(
                Keyword::Plot
                    | Keyword::Show
                    | Keyword::Summarize
                    | Keyword::Export
                    | Keyword::Print
                    | Keyword::Write
                    | Keyword::Copy
                    | Keyword::Move
                    | Keyword::Delete
            )
    )
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

fn contains_symbol(tokens: &[Token], symbol: Symbol) -> bool {
    tokens
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Symbol(found) if found == symbol))
}

#[allow(dead_code)]
fn span_of(tokens: &[Token]) -> Option<SourceSpan> {
    tokens.first().map(|token| token.span)
}
