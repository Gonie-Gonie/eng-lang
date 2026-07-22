use crate::ast::{
    ArgsDecl, ArgsFieldDecl, AssertDecl, AstItem, ClassDecl, ClassFieldDecl, ClassMethodDecl,
    ClassObjectCopyDecl, ClassObjectDecl, ClassObjectFieldDecl, ClassValidationDecl,
    CommandClauseDecl, CommandStyleDecl, ComponentDecl, ConnectDecl, ConservationDecl, ConstDecl,
    ConstraintDecl, CsvExportDecl, CsvExportFieldDecl, DbReadDecl, DbTableTargetDecl, DomainDecl,
    DomainTypeParameterDecl, DomainVariableDecl, EquationDecl, ExpectationDecl,
    ExpectationSuiteDecl, ExplicitDecl, FastBinding, FileOperationDecl, FunctionDecl,
    FunctionParamDecl, GoldenDecl, ImportDecl, MissingPolicyDecl, NetDownloadDecl, OnBlockDecl,
    OnPredicateDecl, PortDecl, PrintDecl, ProcessRunDecl, PromotionDecl, PromotionKind, ReturnDecl,
    SchemaDecl, ScriptDecl, StateSpaceTypeBlockDecl, StateSpaceTypeMemberDecl,
    StateSpaceVectorDecl, StructDecl, SummaryDecl, SystemDecl, SystemVariableDecl, TestDecl,
    WhereBindingDecl, WhereBlockDecl, WherePredicateDecl, WithBlockDecl, WithOptionDecl, WriteDecl,
};
use crate::lexer::{lex_line, lex_line_in_source, Keyword, Symbol, Token, TokenKind};
use crate::source::{source_lines, SourceLine, SourceSpan};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseContext {
    TopLevel,
    Schema,
    SchemaConstraints,
    SchemaMissing,
    Script,
    Function,
    Args,
    StateSpaceTypeBlock,
    Struct,
    Class,
    ClassValidation,
    Object,
    System,
    Domain,
    Component,
    Equation,
    Export,
    Test,
    Where,
    On,
    With,
    Expect,
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
    pub classes: usize,
    pub class_fields: usize,
    pub class_validations: usize,
    pub class_methods: usize,
    pub class_objects: usize,
    pub class_object_copies: usize,
    pub class_object_fields: usize,
    pub args_blocks: usize,
    pub args_fields: usize,
    pub const_declarations: usize,
    pub equations: usize,
    pub fast_bindings: usize,
    pub explicit_declarations: usize,
    pub command_styles: usize,
    pub expectation_suites: usize,
    pub expectations: usize,
    pub where_blocks: usize,
    pub with_blocks: usize,
    pub tests: usize,
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
        let mut classes = 0usize;
        let mut class_fields = 0usize;
        let mut class_validations = 0usize;
        let mut class_methods = 0usize;
        let mut class_objects = 0usize;
        let mut class_object_copies = 0usize;
        let mut class_object_fields = 0usize;
        let mut args_blocks = 0usize;
        let mut args_fields = 0usize;
        let mut const_declarations = 0usize;
        let mut equations = 0usize;
        let mut fast_bindings = 0usize;
        let mut explicit_declarations = 0usize;
        let mut command_styles = 0usize;
        let mut expectation_suites = 0usize;
        let mut expectations = 0usize;
        let mut where_blocks = 0usize;
        let mut with_blocks = 0usize;
        let mut tests = 0usize;

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
                AstItem::Class(_) => classes += 1,
                AstItem::ClassField(_) => class_fields += 1,
                AstItem::ClassValidation(_) => class_validations += 1,
                AstItem::ClassMethod(_) => class_methods += 1,
                AstItem::ClassObject(_) => class_objects += 1,
                AstItem::ClassObjectCopy(_) => class_object_copies += 1,
                AstItem::ClassObjectField(_) => class_object_fields += 1,
                AstItem::Args(_) => args_blocks += 1,
                AstItem::ArgsField(_) => args_fields += 1,
                AstItem::Const(_) => const_declarations += 1,
                AstItem::Equation(_) => equations += 1,
                AstItem::FastBinding(_) => fast_bindings += 1,
                AstItem::ExplicitDecl(_) => explicit_declarations += 1,
                AstItem::CommandStyle(_) => command_styles += 1,
                AstItem::ExpectationSuite(_) => expectation_suites += 1,
                AstItem::Expectation(_) => expectations += 1,
                AstItem::WhereBlock(_) => where_blocks += 1,
                AstItem::WithBlock(_) => with_blocks += 1,
                AstItem::Test(_) => tests += 1,
                AstItem::SystemVariable(_)
                | AstItem::StateSpaceTypeBlock(_)
                | AstItem::StateSpaceTypeMember(_)
                | AstItem::StateSpaceVector(_)
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
                | AstItem::NetDownload(_)
                | AstItem::ProcessRun(_)
                | AstItem::Assert(_)
                | AstItem::Golden(_)
                | AstItem::WhereBinding(_)
                | AstItem::WherePredicate(_)
                | AstItem::OnBlock(_)
                | AstItem::OnPredicate(_)
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
            classes,
            class_fields,
            class_validations,
            class_methods,
            class_objects,
            class_object_copies,
            class_object_fields,
            args_blocks,
            args_fields,
            const_declarations,
            equations,
            fast_bindings,
            explicit_declarations,
            command_styles,
            expectation_suites,
            expectations,
            where_blocks,
            with_blocks,
            tests,
        }
    }
}

pub fn parse_source(source: &str) -> ParsedProgram {
    parse_source_in_source(source, SourceSpan::ROOT_SOURCE_ID)
}

pub(crate) fn parse_top_level_source_line(source_line: &SourceLine) -> ParsedProgram {
    let tokens = lex_line(source_line.line, source_line.start, &source_line.text);
    let mut items = Vec::new();
    if !tokens.is_empty() {
        parse_line_items(
            &mut items,
            &tokens,
            &source_line.text,
            ParseContext::TopLevel,
            None,
        );
    }
    ParsedProgram {
        lines: vec![ParsedLine {
            line: source_line.line,
            text: source_line.text.clone(),
            tokens,
            context: ParseContext::TopLevel,
        }],
        items,
    }
}

pub(crate) fn parse_source_in_source(source: &str, source_id: usize) -> ParsedProgram {
    let mut parsed_lines = Vec::new();
    let mut items = Vec::new();
    let mut schema_depth = 0i32;
    let mut constraints_depth = 0i32;
    let mut missing_depth = 0i32;
    let mut script_depth = 0i32;
    let mut function_depth = 0i32;
    let mut args_depth = 0i32;
    let mut state_space_type_block_depth = 0i32;
    let mut struct_depth = 0i32;
    let mut class_depth = 0i32;
    let mut class_validation_depth = 0i32;
    let mut object_depth = 0i32;
    let mut system_depth = 0i32;
    let mut domain_depth = 0i32;
    let mut component_depth = 0i32;
    let mut equation_depth = 0i32;
    let mut export_depth = 0i32;
    let mut test_depth = 0i32;
    let mut where_depth = 0i32;
    let mut on_depth = 0i32;
    let mut with_depth = 0i32;
    let mut expect_depth = 0i32;
    let mut current_where_owner_line = None;
    let mut current_on_owner_line = None;
    let mut current_with_owner_line = None;
    let mut current_expect_owner_line = None;
    let mut current_object_owner_line = None;
    let mut last_attachable_line = None;

    for source_line in source_lines(source) {
        let tokens = if source_id == SourceSpan::ROOT_SOURCE_ID {
            lex_line(source_line.line, source_line.start, &source_line.text)
        } else {
            lex_line_in_source(
                source_id,
                source_line.line,
                source_line.start,
                &source_line.text,
            )
        };
        let context = if equation_depth > 0 {
            ParseContext::Equation
        } else if export_depth > 0 {
            ParseContext::Export
        } else if test_depth > 0 {
            ParseContext::Test
        } else if where_depth > 0 {
            ParseContext::Where
        } else if on_depth > 0 {
            ParseContext::On
        } else if with_depth > 0 {
            ParseContext::With
        } else if expect_depth > 0 {
            ParseContext::Expect
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
        } else if state_space_type_block_depth > 0 {
            ParseContext::StateSpaceTypeBlock
        } else if struct_depth > 0 {
            ParseContext::Struct
        } else if class_validation_depth > 0 {
            ParseContext::ClassValidation
        } else if class_depth > 0 {
            ParseContext::Class
        } else if object_depth > 0 {
            ParseContext::Object
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
                ParseContext::On => current_on_owner_line,
                ParseContext::With => current_with_owner_line,
                ParseContext::Expect => current_expect_owner_line,
                ParseContext::Object => current_object_owner_line,
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

        if context == ParseContext::TopLevel && starts_state_space_type_block(&tokens) {
            state_space_type_block_depth += brace_delta(&tokens);
            if state_space_type_block_depth == 0 {
                state_space_type_block_depth = 1;
            }
        } else if state_space_type_block_depth > 0 {
            state_space_type_block_depth += brace_delta(&tokens);
            if state_space_type_block_depth <= 0 {
                state_space_type_block_depth = 0;
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

        if starts_with_keyword(&tokens, Keyword::Class) {
            class_depth += brace_delta(&tokens);
            if class_depth == 0 {
                class_depth = 1;
            }
        } else if class_depth > 0 {
            class_depth += brace_delta(&tokens);
            if class_depth <= 0 {
                class_depth = 0;
            }
        }

        if class_depth > 0 && starts_with_identifier(&tokens, "validate") {
            let delta = brace_delta(&tokens);
            if delta > 0 {
                class_validation_depth += delta;
            }
        } else if class_validation_depth > 0 {
            class_validation_depth += brace_delta(&tokens);
            if class_validation_depth <= 0 {
                class_validation_depth = 0;
            }
        }

        if starts_object_literal(&tokens) || starts_class_object_copy_literal(&tokens) {
            current_object_owner_line = tokens.first().map(|token| token.span.line);
            object_depth += brace_delta(&tokens);
            if object_depth == 0 {
                object_depth = 1;
            }
        } else if object_depth > 0 {
            object_depth += brace_delta(&tokens);
            if object_depth <= 0 {
                object_depth = 0;
                current_object_owner_line = None;
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

        if starts_with_keyword(&tokens, Keyword::Test) && !contains_symbol(&tokens, Symbol::Equal) {
            test_depth += brace_delta(&tokens);
            if test_depth == 0 {
                test_depth = 1;
            }
        } else if test_depth > 0 {
            test_depth += brace_delta(&tokens);
            if test_depth <= 0 {
                test_depth = 0;
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

        if starts_with_keyword(&tokens, Keyword::On) {
            current_on_owner_line = last_attachable_line;
            let delta = brace_delta(&tokens);
            if delta != 0 {
                on_depth += delta;
            } else if !(contains_symbol(&tokens, Symbol::LBrace)
                && contains_symbol(&tokens, Symbol::RBrace))
            {
                on_depth = 1;
            }
        } else if on_depth > 0 {
            on_depth += brace_delta(&tokens);
            if on_depth <= 0 {
                on_depth = 0;
                current_on_owner_line = None;
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

        if context == ParseContext::TopLevel && starts_with_identifier(&tokens, "expect") {
            current_expect_owner_line = tokens.first().map(|token| token.span.line);
            let delta = brace_delta(&tokens);
            if delta != 0 {
                expect_depth += delta;
            } else {
                expect_depth = 1;
            }
        } else if expect_depth > 0 {
            expect_depth += brace_delta(&tokens);
            if expect_depth <= 0 {
                expect_depth = 0;
                current_expect_owner_line = None;
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
    if let Some(class_decl) = parse_class_decl(tokens) {
        items.push(AstItem::Class(class_decl));
    }
    if let Some(validation) = parse_class_validation_decl(tokens, line_text, context) {
        items.push(AstItem::ClassValidation(validation));
    }
    if let Some(method) = parse_class_method_decl(tokens, line_text, context) {
        items.push(AstItem::ClassMethod(method));
    }
    if let Some(field) = parse_class_field_decl(tokens, line_text, context) {
        items.push(AstItem::ClassField(field));
    }
    if let Some(object) = parse_class_object_decl(tokens, context) {
        items.push(AstItem::ClassObject(object));
    }
    if let Some(object) = parse_class_object_copy_decl(tokens, context) {
        items.push(AstItem::ClassObjectCopy(object));
    }
    if let Some(field) = parse_class_object_field_decl(tokens, line_text, owner_line, context) {
        items.push(AstItem::ClassObjectField(field));
    }
    if let Some(field) = parse_args_field_decl(tokens, line_text, context) {
        items.push(AstItem::ArgsField(field));
    }
    if let Some(system) = parse_system_decl(tokens) {
        items.push(AstItem::System(system));
    }
    if let Some(block) = parse_state_space_type_block_decl(tokens, context) {
        items.push(AstItem::StateSpaceTypeBlock(block));
    }
    if let Some(member) = parse_state_space_type_member_decl(tokens, line_text, context) {
        items.push(AstItem::StateSpaceTypeMember(member));
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
    if let Some(connect) = parse_connect_decl(tokens, line_text, context) {
        items.push(AstItem::Connect(connect));
    }
    if let Some(variable) = parse_system_variable_decl(tokens, line_text, context) {
        items.push(AstItem::SystemVariable(variable));
    }
    if let Some(vector) = parse_state_space_vector_decl(tokens, line_text, context) {
        items.push(AstItem::StateSpaceVector(vector));
    }
    if let Some(operator) = parse_operator_decl(tokens, line_text, context) {
        items.push(AstItem::ExplicitDecl(operator));
    }
    if let Some(equation) = parse_equation_decl(tokens, line_text, context) {
        items.push(AstItem::Equation(equation));
    }
    if let Some(block) = parse_where_block_decl(tokens, owner_line) {
        items.push(AstItem::WhereBlock(block));
    }
    if let Some(block) = parse_on_block_decl(tokens, owner_line) {
        items.push(AstItem::OnBlock(block));
        for predicate in parse_inline_on_predicates(tokens, line_text, owner_line) {
            items.push(AstItem::OnPredicate(predicate));
        }
    }
    if let Some(block) = parse_with_block_decl(tokens, owner_line) {
        items.push(AstItem::WithBlock(block));
        for option in parse_inline_with_options(tokens, line_text, owner_line) {
            items.push(AstItem::WithOption(option));
        }
    }
    if let Some(suite) = parse_expectation_suite_decl(tokens, line_text, context) {
        items.push(AstItem::ExpectationSuite(suite));
    }
    if let Some(expectation) = parse_expectation_decl(tokens, line_text, owner_line, context) {
        items.push(AstItem::Expectation(expectation));
    }
    if let Some(predicate) = parse_where_predicate_decl(tokens, line_text, owner_line, context) {
        items.push(AstItem::WherePredicate(predicate));
    } else if let Some(binding) = parse_where_binding_decl(tokens, line_text, owner_line, context) {
        items.push(AstItem::WhereBinding(binding));
    }
    if let Some(predicate) = parse_on_predicate_decl(tokens, line_text, owner_line, context) {
        items.push(AstItem::OnPredicate(predicate));
    }
    if let Some(option) = parse_with_option_decl(tokens, line_text, owner_line, context) {
        items.push(AstItem::WithOption(option));
    }
    if let Some(process) = parse_process_run_decl(tokens, line_text, context) {
        items.push(AstItem::ProcessRun(process));
    }
    if let Some(test) = parse_test_decl(tokens, context) {
        items.push(AstItem::Test(test));
    }
    if let Some(assertion) = parse_assert_decl(tokens, line_text, context) {
        items.push(AstItem::Assert(assertion));
    }
    if let Some(golden) = parse_golden_decl(tokens, line_text, context) {
        items.push(AstItem::Golden(golden));
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
            | ParseContext::StateSpaceTypeBlock
            | ParseContext::Class
            | ParseContext::ClassValidation
            | ParseContext::Object
            | ParseContext::SchemaConstraints
            | ParseContext::SchemaMissing
            | ParseContext::Expect
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
    if let Some(print) = parse_print_decl(tokens, line_text, context) {
        items.push(AstItem::Print(print));
    }
    if let Some(export) = parse_csv_export_decl(tokens, line_text, context) {
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
    if let Some(download) = parse_net_download_decl(tokens, line_text, context) {
        items.push(AstItem::NetDownload(download));
    }
    if let Some(command) = parse_standalone_command_style_decl(tokens, line_text, context) {
        items.push(AstItem::CommandStyle(command));
    }
    if let Some(keyword) = parse_reserved_keyword_use(tokens) {
        items.push(keyword);
    }
}

fn parse_expectation_suite_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<ExpectationSuiteDecl> {
    if context != ParseContext::TopLevel {
        return None;
    }
    let first = tokens.first()?;
    if !token_is_identifier(first, "expect") {
        return None;
    }
    if !contains_symbol(tokens, Symbol::LBrace) {
        return None;
    }
    let target = line_text
        .trim_start()
        .strip_prefix("expect")?
        .trim()
        .trim_end_matches('{')
        .trim();
    if target.is_empty() {
        return None;
    }
    Some(ExpectationSuiteDecl {
        target: target.to_owned(),
        line: first.span.line,
        span: first.span,
    })
}

fn parse_expectation_decl(
    tokens: &[Token],
    line_text: &str,
    owner_line: Option<usize>,
    context: ParseContext,
) -> Option<ExpectationDecl> {
    if context != ParseContext::Expect {
        return None;
    }
    let first = tokens.first()?;
    if matches!(first.kind, TokenKind::Symbol(Symbol::RBrace)) {
        return None;
    }
    let text = line_text.trim();
    if text.is_empty() || text == "}" {
        return None;
    }
    Some(ExpectationDecl {
        suite_line: owner_line,
        text: text.to_owned(),
        line: first.span.line,
        span: first.span,
    })
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
        name_span: second.span,
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
    if let Some(target_token) = tokens.get(1) {
        if let TokenKind::StringLiteral(target) = &target_token.kind {
            return Some(ImportDecl {
                target: target.clone(),
                target_span: string_literal_content_span(target_token),
                kind: "file".to_owned(),
                line: first.span.line,
                span: first.span,
            });
        }
    }

    let target_tokens = tokens.get(1..)?;
    let target = target_tokens
        .iter()
        .map(|token| token.lexeme.as_str())
        .collect::<Vec<_>>()
        .join("");
    let target_start = target_tokens.first()?.span;
    let target_end = target_tokens.last()?.span;
    (!target.is_empty()).then(|| ImportDecl {
        target,
        target_span: SourceSpan::new_in_source(
            target_start.source_id,
            target_start.start,
            target_end.end,
            target_start.line,
            target_start.column,
        ),
        kind: kind.to_owned(),
        line: first.span.line,
        span: first.span,
    })
}

fn string_literal_content_span(token: &Token) -> SourceSpan {
    let leading_quote = usize::from(token.lexeme.starts_with('"'));
    let trailing_quote =
        usize::from(token.lexeme.len() > leading_quote && token.lexeme.ends_with('"'));
    SourceSpan::new_in_source(
        token.span.source_id,
        token.span.start + leading_quote,
        token.span.end.saturating_sub(trailing_quote),
        token.span.line,
        token.span.column + leading_quote,
    )
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
        name_span: second.span,
        span: first.span,
    })
}

fn parse_state_space_type_block_decl(
    tokens: &[Token],
    context: ParseContext,
) -> Option<StateSpaceTypeBlockDecl> {
    if context != ParseContext::TopLevel {
        return None;
    }
    let [first, second, ..] = tokens else {
        return None;
    };
    let role = state_space_type_block_role(first)?;
    let TokenKind::Identifier(name) = &second.kind else {
        return None;
    };
    if !tokens
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Symbol(Symbol::LBrace)))
    {
        return None;
    }
    Some(StateSpaceTypeBlockDecl {
        role: role.to_owned(),
        name: name.clone(),
        name_span: second.span,
        line: first.span.line,
        span: first.span,
    })
}

fn parse_state_space_type_member_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<StateSpaceTypeMemberDecl> {
    if context != ParseContext::StateSpaceTypeBlock {
        return None;
    }
    let [first, second, ..] = tokens else {
        return None;
    };
    if matches!(
        &first.kind,
        TokenKind::Symbol(Symbol::LBrace | Symbol::RBrace)
    ) {
        return None;
    }
    let TokenKind::Identifier(name) = &first.kind else {
        return None;
    };
    if !matches!(second.kind, TokenKind::Symbol(Symbol::Colon)) {
        return None;
    }
    let type_part = line_text
        .split_once(':')?
        .1
        .trim()
        .trim_end_matches(',')
        .trim_end();
    let (type_name, unit) = split_type_and_unit(type_part);
    if type_name.is_empty() {
        return None;
    }
    let (type_span, unit_span) = type_and_unit_source_spans(first.span, line_text, type_part)?;
    Some(StateSpaceTypeMemberDecl {
        name: name.clone(),
        type_name,
        type_span,
        unit,
        unit_span,
        line: first.span.line,
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
        name_span: second.span,
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
        if matches!(token.kind, TokenKind::Identifier(_)) {
            group.push(token);
        }
    }
    push_domain_type_parameter(&mut parameters, &group);
    parameters
}

fn push_domain_type_parameter(parameters: &mut Vec<DomainTypeParameterDecl>, group: &[&Token]) {
    let Some(kind_token) = group.first().copied() else {
        return;
    };
    let TokenKind::Identifier(kind) = &kind_token.kind else {
        return;
    };
    let name_token = group.get(1).copied().unwrap_or(kind_token);
    let TokenKind::Identifier(name) = &name_token.kind else {
        return;
    };
    parameters.push(DomainTypeParameterDecl {
        kind: kind.clone(),
        kind_span: kind_token.span,
        name: name.clone(),
        name_span: name_token.span,
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
        name_span: second.span,
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
        name_span: second.span,
        span: first.span,
    })
}

fn parse_class_decl(tokens: &[Token]) -> Option<ClassDecl> {
    let [first, second, ..] = tokens else {
        return None;
    };
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Class)) {
        return None;
    }
    let TokenKind::Identifier(name) = &second.kind else {
        return None;
    };
    Some(ClassDecl {
        name: name.clone(),
        name_span: second.span,
        span: first.span,
    })
}

fn parse_class_field_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<ClassFieldDecl> {
    if context != ParseContext::Class {
        return None;
    }
    let [first, second, ..] = tokens else {
        return None;
    };
    if token_is_identifier(first, "validate") {
        return None;
    }
    if token_is_identifier(first, "method") {
        return None;
    }
    let name = token_field_name(first)?;
    if !matches!(second.kind, TokenKind::Symbol(Symbol::Colon)) {
        return None;
    }
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let code_end = tokens.last()?.span.end.checked_sub(line_start)?;
    let code = line_text.get(..code_end)?.trim_end();
    let code = code.strip_suffix(',').unwrap_or(code).trim_end();
    let raw_after_colon = code.split_once(':')?.1.trim();
    let (type_part, default_value_source) = raw_after_colon
        .split_once('=')
        .map(|(left, right)| (left.trim(), Some(right.trim())))
        .unwrap_or((raw_after_colon, None));
    let (type_name, unit) = split_type_and_unit(type_part);
    let (type_span, unit_span) = type_and_unit_source_spans(first.span, line_text, type_part)?;
    if type_name.is_empty() {
        return None;
    }
    Some(ClassFieldDecl {
        name,
        type_name,
        type_span,
        unit,
        unit_span,
        default_value: default_value_source.map(str::to_owned),
        default_value_span: default_value_source
            .and_then(|value| source_span_for_subslice(first.span, line_text, value)),
        line: first.span.line,
        span: first.span,
    })
}

fn parse_class_validation_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<ClassValidationDecl> {
    let first = tokens.first()?;
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let code_end = tokens.last()?.span.end.checked_sub(line_start)?;
    let code = line_text.get(..code_end)?;
    let expression = match context {
        ParseContext::Class => {
            if !token_is_identifier(first, "validate") {
                return None;
            }
            let rest = code.trim_start().strip_prefix("validate")?.trim();
            class_validation_expression_from_validate_line(rest)?
        }
        ParseContext::ClassValidation => class_validation_expression_from_block_line(code)?,
        _ => return None,
    };
    Some(ClassValidationDecl {
        expression: expression.to_owned(),
        expression_span: source_span_for_subslice(first.span, line_text, expression)?,
        line: first.span.line,
        span: first.span,
    })
}

fn class_validation_expression_from_validate_line(rest: &str) -> Option<&str> {
    let trimmed = rest.trim().trim_end_matches(',').trim();
    if trimmed.is_empty() || trimmed == "{" {
        return None;
    }
    if let Some(after_open) = trimmed.strip_prefix('{') {
        let expression = after_open.trim().trim_end_matches('}').trim();
        return (!expression.is_empty()).then_some(expression);
    }
    let expression = trimmed.trim_end_matches('{').trim();
    (!expression.is_empty()).then_some(expression)
}

fn class_validation_expression_from_block_line(line_text: &str) -> Option<&str> {
    let trimmed = line_text.trim().trim_end_matches(',').trim();
    if trimmed.is_empty() || trimmed == "}" {
        return None;
    }
    let expression = trimmed.trim_end_matches('}').trim();
    (!expression.is_empty()).then_some(expression)
}

fn parse_class_method_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<ClassMethodDecl> {
    if context != ParseContext::Class {
        return None;
    }
    let [first, second, ..] = tokens else {
        return None;
    };
    if !token_is_identifier(first, "method") {
        return None;
    }
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let code_end = tokens.last()?.span.end.checked_sub(line_start)?;
    let text = line_text.get(..code_end)?.trim().trim_end_matches(',');
    let rest = text.strip_prefix("method")?.trim();
    let open = rest.find('(')?;
    let name = rest[..open].trim();
    if name.is_empty() || !is_identifier_text(name) || name != second.lexeme {
        return None;
    }
    let after_open = &rest[open + 1..];
    let close = after_open.find(')')?;
    let after_signature = after_open[close + 1..].trim();
    let after_arrow = after_signature.strip_prefix("->")?.trim();
    let (return_part, expression) = after_arrow.split_once('=')?;
    let return_part = return_part.trim();
    let (return_type, return_unit) = split_type_and_unit(return_part);
    let expression = expression.trim();
    if return_type.is_empty() || expression.is_empty() {
        return None;
    }
    let return_span = source_span_for_subslice(first.span, line_text, return_part)?;
    let (return_type_span, return_unit_span) =
        type_and_unit_source_spans_at(first.span, return_span.column.checked_sub(1)?, return_part);
    Some(ClassMethodDecl {
        name: name.to_owned(),
        name_span: second.span,
        return_type,
        return_type_span,
        return_unit,
        return_unit_span,
        expression: expression.to_owned(),
        expression_span: source_span_for_subslice(first.span, line_text, expression)?,
        line: first.span.line,
        span: first.span,
    })
}

fn parse_class_object_decl(tokens: &[Token], context: ParseContext) -> Option<ClassObjectDecl> {
    if context != ParseContext::TopLevel && context != ParseContext::Other {
        return None;
    }
    let [first, second, third, ..] = tokens else {
        return None;
    };
    let TokenKind::Identifier(name) = &first.kind else {
        return None;
    };
    if !matches!(second.kind, TokenKind::Symbol(Symbol::Equal)) {
        return None;
    }
    let TokenKind::Identifier(class_name) = &third.kind else {
        return None;
    };
    if tokens
        .get(3)
        .is_some_and(|token| matches!(token.kind, TokenKind::Keyword(Keyword::With)))
    {
        return None;
    }
    if !tokens
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Symbol(Symbol::LBrace)))
    {
        return None;
    }
    Some(ClassObjectDecl {
        name: name.clone(),
        class_name: class_name.clone(),
        class_name_span: third.span,
        line: first.span.line,
        span: first.span,
        context,
    })
}

fn parse_class_object_copy_decl(
    tokens: &[Token],
    context: ParseContext,
) -> Option<ClassObjectCopyDecl> {
    if context != ParseContext::TopLevel && context != ParseContext::Other {
        return None;
    }
    let [first, second, third, fourth, ..] = tokens else {
        return None;
    };
    let TokenKind::Identifier(name) = &first.kind else {
        return None;
    };
    if !matches!(second.kind, TokenKind::Symbol(Symbol::Equal)) {
        return None;
    }
    let TokenKind::Identifier(source_name) = &third.kind else {
        return None;
    };
    if !matches!(fourth.kind, TokenKind::Keyword(Keyword::With)) {
        return None;
    }
    if !tokens
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Symbol(Symbol::LBrace)))
    {
        return None;
    }
    Some(ClassObjectCopyDecl {
        name: name.clone(),
        source_name: source_name.clone(),
        source_name_span: third.span,
        line: first.span.line,
        span: first.span,
        context,
    })
}

fn parse_class_object_field_decl(
    tokens: &[Token],
    line_text: &str,
    owner_line: Option<usize>,
    context: ParseContext,
) -> Option<ClassObjectFieldDecl> {
    if context != ParseContext::Object {
        return None;
    }
    let first = tokens.first()?;
    if matches!(
        &first.kind,
        TokenKind::Symbol(Symbol::LBrace | Symbol::RBrace)
    ) {
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
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let code_end = tokens.last()?.span.end.checked_sub(line_start)?;
    let code = line_text.get(..code_end)?.trim_end();
    let code = code.strip_suffix(',').unwrap_or(code).trim_end();
    let expression = code.split_once('=')?.1.trim();
    if expression.is_empty() {
        return None;
    }
    Some(ClassObjectFieldDecl {
        owner_line,
        name: name.clone(),
        expression: expression.to_owned(),
        expression_span: source_span_for_subslice(first.span, line_text, expression)?,
        line: first.span.line,
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
    let parts = typed_declaration_parts(first.span, line_text)?;
    let expression = parts.expression?;
    let expression_span = parts.expression_span?;
    Some(ConstDecl {
        name: name.clone(),
        name_span: second.span,
        type_name: parts.type_name,
        type_span: parts.type_span,
        unit: parts.unit,
        unit_span: parts.unit_span,
        expression,
        expression_span,
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
    let parameters = parse_function_parameters(tokens, line_text);
    let return_parts = parse_function_return(tokens, line_text)?;
    Some(FunctionDecl {
        name: name.clone(),
        name_span: second.span,
        parameters,
        return_type: return_parts.type_name,
        return_type_span: return_parts.type_span,
        return_unit: return_parts.unit,
        return_unit_span: return_parts.unit_span,
        span: first.span,
    })
}

fn parse_inline_function_return_decl(tokens: &[Token], line_text: &str) -> Option<ReturnDecl> {
    let first = tokens.first()?;
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Fn)) {
        return None;
    }
    let equal = tokens
        .iter()
        .find(|token| matches!(token.kind, TokenKind::Symbol(Symbol::Equal)))?;
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let expression_start = equal.span.end.checked_sub(line_start)?;
    let (expression, expression_span) =
        trimmed_source_parts(first.span, line_text, expression_start, line_text.len())?;
    if expression.starts_with('{') {
        return None;
    }
    Some(ReturnDecl {
        expression,
        expression_span,
        line: first.span.line,
        span: first.span,
        context: ParseContext::Function,
    })
}

fn parse_function_parameters(tokens: &[Token], line_text: &str) -> Vec<FunctionParamDecl> {
    let Some(first) = tokens.first() else {
        return Vec::new();
    };
    let Some(column_offset) = first.span.column.checked_sub(1) else {
        return Vec::new();
    };
    let Some(line_start) = first.span.start.checked_sub(column_offset) else {
        return Vec::new();
    };
    let Some(open_index) = tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::Symbol(Symbol::LParen)))
    else {
        return Vec::new();
    };
    let Some(close_offset) = tokens[open_index + 1..]
        .iter()
        .position(|token| matches!(token.kind, TokenKind::Symbol(Symbol::RParen)))
    else {
        return Vec::new();
    };
    let close_index = open_index + 1 + close_offset;
    tokens[open_index + 1..close_index]
        .split(|token| matches!(token.kind, TokenKind::Symbol(Symbol::Comma)))
        .filter_map(|parameter_tokens| {
            parse_function_parameter(parameter_tokens, line_text, line_start)
        })
        .collect()
}

fn parse_function_parameter(
    tokens: &[Token],
    line_text: &str,
    line_start: usize,
) -> Option<FunctionParamDecl> {
    let [name_token, colon_token, type_tokens @ ..] = tokens else {
        return None;
    };
    let TokenKind::Identifier(name) = &name_token.kind else {
        return None;
    };
    if !matches!(colon_token.kind, TokenKind::Symbol(Symbol::Colon)) {
        return None;
    }
    let type_end = type_tokens.last()?.span.end.checked_sub(line_start)?;
    let type_start = colon_token.span.end.checked_sub(line_start)?;
    let parts = typed_annotation_parts(name_token.span, line_text, type_start, type_end)?;
    Some(FunctionParamDecl {
        name: name.clone(),
        name_span: name_token.span,
        type_name: parts.type_name,
        type_span: parts.type_span,
        unit: parts.unit,
        unit_span: parts.unit_span,
    })
}

fn parse_function_return(tokens: &[Token], line_text: &str) -> Option<TypedAnnotationParts> {
    let first = tokens.first()?;
    let column_offset = first.span.column.checked_sub(1)?;
    let line_start = first.span.start.checked_sub(column_offset)?;
    let arrow_index = tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::Symbol(Symbol::Arrow)))?;
    let arrow = &tokens[arrow_index];
    let return_end = tokens[arrow_index + 1..]
        .iter()
        .take_while(|token| {
            !matches!(
                token.kind,
                TokenKind::Symbol(Symbol::LBrace | Symbol::Equal)
            )
        })
        .last()?;
    let type_start = arrow.span.end.checked_sub(line_start)?;
    let type_end = return_end.span.end.checked_sub(line_start)?;
    typed_annotation_parts(first.span, line_text, type_start, type_end)
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
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let expression_start = first.span.end.checked_sub(line_start)?;
    let (expression, expression_span) =
        trimmed_source_parts(first.span, line_text, expression_start, line_text.len())?;
    Some(ReturnDecl {
        expression,
        expression_span,
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
    let [first, second, ..] = tokens else {
        return None;
    };
    let name = token_field_name(first)?;
    if !matches!(second.kind, TokenKind::Symbol(Symbol::Colon)) {
        return None;
    }
    let raw_after_colon = line_text.split_once(':')?.1.trim().trim_end_matches(',');
    let (type_part, default_value) = raw_after_colon
        .split_once('=')
        .map(|(left, right)| (left.trim(), Some(right.trim().to_owned())))
        .unwrap_or((raw_after_colon, None));
    let (type_name, unit) = split_type_and_unit(type_part);
    let (type_span, unit_span) = type_and_unit_source_spans(first.span, line_text, type_part)?;
    if type_name.is_empty() {
        return None;
    }
    let default_value_span = default_value
        .as_ref()
        .and_then(|_| source_span_after_equals_without_trailing_comma(first.span, line_text));

    Some(ArgsFieldDecl {
        name,
        type_name,
        type_span,
        unit,
        unit_span,
        default_value,
        default_value_span,
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

fn token_is_identifier(token: &Token, expected: &str) -> bool {
    matches!(&token.kind, TokenKind::Identifier(value) if value == expected)
}

fn token_field_name(token: &Token) -> Option<String> {
    match &token.kind {
        TokenKind::Identifier(value) => Some(value.clone()),
        TokenKind::Keyword(Keyword::Input) => Some("input".to_owned()),
        TokenKind::Keyword(Keyword::Output) => Some("output".to_owned()),
        _ => None,
    }
}

fn state_space_type_block_role(token: &Token) -> Option<&str> {
    match &token.kind {
        TokenKind::Identifier(value)
            if matches!(value.as_str(), "states" | "inputs" | "outputs") =>
        {
            Some(value.as_str())
        }
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
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let type_start = third.span.end.checked_sub(line_start)?;
    let type_end = tokens.last()?.span.end.checked_sub(line_start)?;
    let annotation = typed_annotation_parts(first.span, line_text, type_start, type_end)?;

    Some(DomainVariableDecl {
        role: role.to_owned(),
        name: name.clone(),
        name_span: second.span,
        type_name: annotation.type_name,
        type_span: annotation.type_span,
        unit: annotation.unit,
        unit_span: annotation.unit_span,
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
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let text_start = first.span.end.checked_sub(line_start)?;
    let text_end = tokens.last()?.span.end.checked_sub(line_start)?;
    let (text, text_span) = trimmed_source_parts(first.span, line_text, text_start, text_end)
        .unwrap_or_else(|| (String::new(), first.span));
    Some(ConservationDecl {
        text,
        text_span,
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
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let domain_start = third.span.end.checked_sub(line_start)?;
    let domain_end = tokens.last()?.span.end.checked_sub(line_start)?;
    let raw_domain = line_text.get(domain_start..domain_end)?;
    let leading = raw_domain
        .len()
        .checked_sub(raw_domain.trim_start().len())?;
    let trailing = raw_domain.len().checked_sub(raw_domain.trim_end().len())?;
    let domain_start = domain_start.checked_add(leading)?;
    let domain_end = domain_end.checked_sub(trailing)?;
    let domain = line_text.get(domain_start..domain_end)?;
    let domain_span = source_span_for_line_range(first.span, domain_start, domain_end);
    Some(PortDecl {
        name: name.clone(),
        name_span: second.span,
        domain: domain.to_owned(),
        domain_span,
        line: first.span.line,
        span: first.span,
    })
}

fn parse_connect_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<ConnectDecl> {
    let first = tokens.first()?;
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Connect)) {
        return None;
    }
    let separator = tokens.iter().skip(1).find(|token| {
        matches!(
            token.kind,
            TokenKind::Symbol(Symbol::Arrow) | TokenKind::Keyword(Keyword::To)
        )
    })?;
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let left_start = first.span.end.checked_sub(line_start)?;
    let left_end = separator.span.start.checked_sub(line_start)?;
    let right_start = separator.span.end.checked_sub(line_start)?;
    let right_end = tokens.last()?.span.end.checked_sub(line_start)?;
    let (left, left_span) = trimmed_source_parts(first.span, line_text, left_start, left_end)?;
    let (right, right_span) = trimmed_source_parts(first.span, line_text, right_start, right_end)?;
    Some(ConnectDecl {
        left,
        left_span,
        right,
        right_span,
        line: first.span.line,
        span: first.span,
        context,
    })
}

fn parse_system_variable_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<SystemVariableDecl> {
    if !matches!(context, ParseContext::System | ParseContext::Component) {
        return None;
    }
    let [first, second, third, ..] = tokens else {
        return None;
    };
    let role = match (&first.kind, context) {
        (TokenKind::Keyword(Keyword::Parameter), ParseContext::System) => "parameter",
        (TokenKind::Keyword(Keyword::State), ParseContext::System) => "state",
        (TokenKind::Keyword(Keyword::Input), ParseContext::System) => "input",
        (TokenKind::Keyword(Keyword::Output), ParseContext::System) => "output",
        (TokenKind::Keyword(Keyword::Parameter), ParseContext::Component) => "parameter",
        (TokenKind::Keyword(Keyword::Input), ParseContext::Component) => "input",
        _ => return None,
    };
    let TokenKind::Identifier(name) = &second.kind else {
        return None;
    };
    if !matches!(third.kind, TokenKind::Symbol(Symbol::Colon)) {
        return None;
    }

    let parts = typed_declaration_parts(first.span, line_text)?;

    Some(SystemVariableDecl {
        role: role.to_owned(),
        name: name.clone(),
        name_span: second.span,
        type_name: parts.type_name,
        type_span: parts.type_span,
        unit: parts.unit,
        unit_span: parts.unit_span,
        expression: parts.expression,
        expression_span: parts.expression_span,
        line: first.span.line,
        span: first.span,
        context,
    })
}
fn parse_state_space_vector_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<StateSpaceVectorDecl> {
    if context != ParseContext::System {
        return None;
    }
    let [first, second, third, ..] = tokens else {
        return None;
    };
    let role = match &first.kind {
        TokenKind::Identifier(value)
            if matches!(value.as_str(), "states" | "inputs" | "outputs") =>
        {
            value.as_str()
        }
        _ => return None,
    };
    let TokenKind::Identifier(name) = &second.kind else {
        return None;
    };
    if !matches!(third.kind, TokenKind::Symbol(Symbol::Equal)) {
        return None;
    }
    let members = line_text
        .split_once('=')
        .map(|(_, right)| vector_members(right))
        .unwrap_or_default();
    Some(StateSpaceVectorDecl {
        role: role.to_owned(),
        name: name.clone(),
        name_span: second.span,
        declared_type: None,
        type_span: None,
        members,
        expression_span: source_span_after_equals(first.span, line_text),
        line: first.span.line,
        span: first.span,
        context,
    })
}

fn vector_members(text: &str) -> Vec<String> {
    text.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(str::trim)
        .filter(|member| !member.is_empty())
        .map(str::to_owned)
        .collect()
}

fn parse_operator_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<ExplicitDecl> {
    if context != ParseContext::System {
        return None;
    }
    let [first, second, third, ..] = tokens else {
        return None;
    };
    if !token_is_identifier(first, "operator") {
        return None;
    }
    let TokenKind::Identifier(name) = &second.kind else {
        return None;
    };
    if !matches!(third.kind, TokenKind::Symbol(Symbol::Colon)) {
        return None;
    }
    let parts = typed_declaration_parts(first.span, line_text)?;
    Some(ExplicitDecl {
        name: name.clone(),
        name_span: second.span,
        type_name: parts.type_name,
        type_span: parts.type_span,
        unit: parts.unit,
        unit_span: parts.unit_span,
        expression: parts.expression,
        expression_span: parts.expression_span,
        line: first.span.line,
        span: first.span,
        context,
    })
}

fn parse_equation_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<EquationDecl> {
    if !matches!(context, ParseContext::Equation | ParseContext::Component) {
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
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let left_start = first.span.start.checked_sub(line_start)?;
    let left_end = eq_token.span.start.checked_sub(line_start)?;
    let right_start = eq_token.span.end.checked_sub(line_start)?;
    let right_end = tokens.last()?.span.end.checked_sub(line_start)?;
    let (left, left_span) = trimmed_source_parts(first.span, line_text, left_start, left_end)?;
    let (right, right_span) = trimmed_source_parts(first.span, line_text, right_start, right_end)?;
    Some(EquationDecl {
        left,
        left_span,
        right,
        right_span,
        line: eq_token.span.line,
        span: eq_token.span,
        context,
    })
}

fn parse_fast_binding(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<(FastBinding, Option<CommandStyleDecl>)> {
    if matches!(
        context,
        ParseContext::Where
            | ParseContext::With
            | ParseContext::Class
            | ParseContext::ClassValidation
            | ParseContext::Object
    ) {
        return None;
    }
    if starts_object_literal(tokens) || starts_class_object_copy_literal(tokens) {
        return None;
    }
    let [first, second, ..] = tokens else {
        return None;
    };
    let name = fast_binding_name(first)?;
    if !matches!(second.kind, TokenKind::Symbol(Symbol::Equal)) {
        return None;
    }
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let expression_start = second.span.end.checked_sub(line_start)?;
    let expression_end = tokens.last()?.span.end.checked_sub(line_start)?;
    let raw_expression = line_text.get(expression_start..expression_end)?;
    let leading = raw_expression
        .len()
        .checked_sub(raw_expression.trim_start().len())?;
    let trailing = raw_expression
        .len()
        .checked_sub(raw_expression.trim_end().len())?;
    let expression_start = expression_start.checked_add(leading)?;
    let expression_end = expression_end.checked_sub(trailing)?;
    let expression_source = line_text.get(expression_start..expression_end)?;
    let expression_span = source_span_for_line_range(first.span, expression_start, expression_end);
    let expression = expression_source.to_owned();
    if is_process_run_rhs(&expression) {
        return None;
    }
    let promotion = parse_promotion_decl(tokens, line_text, line_start);
    let db_read = parse_db_read_decl(tokens, line_text, line_start).map(Box::new);
    let command = parse_command_style_expression(
        &expression,
        expression_span,
        first.span,
        context,
        Some(&name),
    );
    let is_command_style = command.is_some();
    let expression = command
        .as_ref()
        .map(|command| command.canonical.clone())
        .unwrap_or(expression);
    Some((
        FastBinding {
            name: name.clone(),
            expression,
            expression_span,
            is_command_style,
            promotion,
            db_read,
            line: first.span.line,
            span: first.span,
            context,
        },
        command,
    ))
}

fn fast_binding_name(token: &Token) -> Option<String> {
    match &token.kind {
        TokenKind::Identifier(name) => Some(name.clone()),
        TokenKind::Keyword(Keyword::Model) => Some("model".to_owned()),
        _ => None,
    }
}

fn parse_promotion_decl(
    tokens: &[Token],
    line_text: &str,
    line_start: usize,
) -> Option<PromotionDecl> {
    let expression_tokens = tokens.get(2..)?;
    let promote = expression_tokens.first()?;
    if promote.lexeme != "promote" {
        return None;
    }
    let format = expression_tokens.get(1)?;
    let (kind, source_start, records_span) = match format.lexeme.as_str() {
        "csv" => (PromotionKind::Csv, 2, None),
        "json" if expression_tokens.get(2)?.lexeme == "records" => (
            PromotionKind::JsonRecords,
            3,
            Some(expression_tokens.get(2)?.span),
        ),
        "json" => (PromotionKind::Json, 2, None),
        "toml" => (PromotionKind::Toml, 2, None),
        _ => return None,
    };
    let as_index = top_level_as_index(expression_tokens, source_start)?;
    if as_index <= source_start {
        return None;
    }
    let source_tokens = expression_tokens.get(source_start..as_index)?;
    let source_first = source_tokens.first()?;
    let source_last = source_tokens.last()?;
    let source_span = span_covering_tokens(source_first, source_last);
    let source_start_in_line = source_span.start.checked_sub(line_start)?;
    let source_end_in_line = source_span.end.checked_sub(line_start)?;
    let source_expression = line_text
        .get(source_start_in_line..source_end_in_line)?
        .to_owned();
    let as_token = expression_tokens.get(as_index)?;
    let schema = expression_tokens.get(as_index + 1)?;
    if !is_identifier_text(&schema.lexeme) {
        return None;
    }

    let (source_binding, source_binding_span, records_field, records_field_span) =
        if kind == PromotionKind::JsonRecords {
            let dot_index = source_tokens
                .iter()
                .position(|token| matches!(token.kind, TokenKind::Symbol(Symbol::Dot)))?;
            let binding_tokens = source_tokens.get(..dot_index)?;
            let field_tokens = source_tokens.get(dot_index + 1..)?;
            let binding = identifier_path_from_tokens(binding_tokens)?;
            let field = identifier_path_from_tokens(field_tokens)?;
            (
                Some(binding),
                Some(span_covering_tokens(
                    binding_tokens.first()?,
                    binding_tokens.last()?,
                )),
                Some(field),
                Some(span_covering_tokens(
                    field_tokens.first()?,
                    field_tokens.last()?,
                )),
            )
        } else {
            (None, None, None, None)
        };

    Some(PromotionDecl {
        kind,
        promote_span: promote.span,
        format_span: format.span,
        records_span,
        source_expression,
        source_span,
        source_binding,
        source_binding_span,
        records_field,
        records_field_span,
        as_span: as_token.span,
        schema_name: schema.lexeme.clone(),
        schema_span: schema.span,
    })
}

fn parse_db_read_decl(tokens: &[Token], line_text: &str, line_start: usize) -> Option<DbReadDecl> {
    let expression_tokens = tokens.get(2..)?;
    let read = expression_tokens.first()?;
    let sqlite = expression_tokens.get(1)?;
    if read.lexeme != "read" || sqlite.lexeme != "sqlite" {
        return None;
    }
    let as_index = top_level_as_index(expression_tokens, 2)?;
    let target_tokens = expression_tokens.get(2..as_index)?;
    let target = parse_db_table_target_decl(target_tokens, line_text, line_start)?;
    let as_token = expression_tokens.get(as_index)?;
    let schema = expression_tokens.get(as_index + 1)?;
    if as_index + 2 != expression_tokens.len() || !is_identifier_text(&schema.lexeme) {
        return None;
    }
    Some(DbReadDecl {
        read_span: read.span,
        sqlite_span: sqlite.span,
        target,
        as_span: as_token.span,
        schema_name: schema.lexeme.clone(),
        schema_span: schema.span,
    })
}

fn parse_db_table_target_decl(
    tokens: &[Token],
    line_text: &str,
    line_start: usize,
) -> Option<DbTableTargetDecl> {
    if tokens.len() < 6 {
        return None;
    }
    let table_index = tokens.len().checked_sub(4)?;
    let dot_index = table_index.checked_sub(1)?;
    let connection_tokens = tokens.get(..dot_index)?;
    let dot = tokens.get(dot_index)?;
    let table_method = tokens.get(table_index)?;
    let lparen = tokens.get(table_index + 1)?;
    let table_value = tokens.get(table_index + 2)?;
    let rparen = tokens.get(table_index + 3)?;
    if !matches!(dot.kind, TokenKind::Symbol(Symbol::Dot))
        || table_method.lexeme != "table"
        || !matches!(lparen.kind, TokenKind::Symbol(Symbol::LParen))
        || !matches!(rparen.kind, TokenKind::Symbol(Symbol::RParen))
    {
        return None;
    }
    let TokenKind::StringLiteral(table) = &table_value.kind else {
        return None;
    };
    let connection = identifier_path_from_tokens(connection_tokens)?;
    let expression_span = span_covering_tokens(tokens.first()?, tokens.last()?);
    Some(DbTableTargetDecl {
        expression: source_text_for_span(line_text, line_start, expression_span)?.to_owned(),
        expression_span,
        connection,
        connection_span: span_covering_tokens(
            connection_tokens.first()?,
            connection_tokens.last()?,
        ),
        table: table.clone(),
        table_method_span: table_method.span,
        table_span: string_literal_content_span(table_value),
    })
}

fn top_level_as_index(tokens: &[Token], start: usize) -> Option<usize> {
    top_level_keyword_index(tokens, start, Keyword::As)
}

fn top_level_keyword_index(tokens: &[Token], start: usize, keyword: Keyword) -> Option<usize> {
    top_level_token_index(
        tokens,
        start,
        |token| matches!(token.kind, TokenKind::Keyword(found) if found == keyword),
    )
}

fn top_level_symbol_index(tokens: &[Token], start: usize, symbol: Symbol) -> Option<usize> {
    top_level_token_index(
        tokens,
        start,
        |token| matches!(token.kind, TokenKind::Symbol(found) if found == symbol),
    )
}

fn top_level_token_index(
    tokens: &[Token],
    start: usize,
    predicate: impl Fn(&Token) -> bool,
) -> Option<usize> {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 && predicate(token) {
            return Some(index);
        }
        match token.kind {
            TokenKind::Symbol(Symbol::LParen) => paren_depth += 1,
            TokenKind::Symbol(Symbol::RParen) => paren_depth = paren_depth.saturating_sub(1),
            TokenKind::Symbol(Symbol::LBracket) => bracket_depth += 1,
            TokenKind::Symbol(Symbol::RBracket) => bracket_depth = bracket_depth.saturating_sub(1),
            TokenKind::Symbol(Symbol::LBrace) => brace_depth += 1,
            TokenKind::Symbol(Symbol::RBrace) => brace_depth = brace_depth.saturating_sub(1),
            _ => {}
        }
    }
    None
}

fn identifier_path_from_tokens(tokens: &[Token]) -> Option<String> {
    let mut segments = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if index % 2 == 0 {
            if !is_identifier_text(&token.lexeme) {
                return None;
            }
            segments.push(token.lexeme.as_str());
        } else if !matches!(token.kind, TokenKind::Symbol(Symbol::Dot)) {
            return None;
        }
    }
    (!segments.is_empty()).then(|| segments.join("."))
}

fn span_covering_tokens(first: &Token, last: &Token) -> SourceSpan {
    SourceSpan::new_in_source(
        first.span.source_id,
        first.span.start,
        last.span.end,
        first.span.line,
        first.span.column,
    )
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
    let expression_span = source_span_after_equals(first.span, line_text)?;
    let expression = parse_command_style_expression(
        &expression,
        expression_span,
        first.span,
        context,
        Some(name),
    )
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

fn parse_where_predicate_decl(
    tokens: &[Token],
    line_text: &str,
    owner_line: Option<usize>,
    context: ParseContext,
) -> Option<WherePredicateDecl> {
    if context != ParseContext::Where {
        return None;
    }
    let first = tokens.first()?;
    if matches!(
        &first.kind,
        TokenKind::Symbol(Symbol::LBrace | Symbol::RBrace)
    ) {
        return None;
    }
    let expression = line_text.trim().trim_end_matches(',').to_owned();
    if expression.is_empty() || !looks_like_where_predicate(&expression) {
        return None;
    }
    Some(WherePredicateDecl {
        owner_line,
        expression,
        line: first.span.line,
        span: first.span,
    })
}

fn looks_like_where_predicate(expression: &str) -> bool {
    let lowered = expression.to_ascii_lowercase();
    lowered.contains("==")
        || lowered.contains("!=")
        || lowered.contains("<=")
        || lowered.contains(">=")
        || lowered.contains('<')
        || lowered.contains('>')
        || lowered.contains(" is none")
        || lowered.contains(" is not none")
}

fn parse_on_block_decl(tokens: &[Token], owner_line: Option<usize>) -> Option<OnBlockDecl> {
    let first = tokens.first()?;
    if !matches!(first.kind, TokenKind::Keyword(Keyword::On)) {
        return None;
    }
    Some(OnBlockDecl {
        owner_line,
        line: first.span.line,
        span: first.span,
    })
}

fn parse_inline_on_predicates(
    tokens: &[Token],
    line_text: &str,
    owner_line: Option<usize>,
) -> Vec<OnPredicateDecl> {
    let Some(first) = tokens.first() else {
        return Vec::new();
    };
    if !matches!(first.kind, TokenKind::Keyword(Keyword::On)) {
        return Vec::new();
    }
    let Some(start) = line_text.find('{') else {
        return Vec::new();
    };
    let Some(end) = line_text.rfind('}') else {
        return Vec::new();
    };
    if end <= start {
        return Vec::new();
    }

    line_text[start + 1..end]
        .split(',')
        .map(str::trim)
        .filter(|expression| expression.contains("=="))
        .map(|expression| OnPredicateDecl {
            owner_line,
            expression: expression.to_owned(),
            line: first.span.line,
            span: first.span,
        })
        .collect()
}

fn parse_on_predicate_decl(
    tokens: &[Token],
    line_text: &str,
    owner_line: Option<usize>,
    context: ParseContext,
) -> Option<OnPredicateDecl> {
    if context != ParseContext::On {
        return None;
    }
    let first = tokens.first()?;
    if matches!(
        &first.kind,
        TokenKind::Symbol(Symbol::LBrace | Symbol::RBrace)
    ) {
        return None;
    }
    let expression = line_text.trim().trim_end_matches(',').to_owned();
    if expression.is_empty() || !expression.contains("==") {
        return None;
    }
    Some(OnPredicateDecl {
        owner_line,
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
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let code_end = tokens.last()?.span.end.checked_sub(line_start)?;
    let (option_start, mut option_end) = trimmed_byte_range(line_text, 0, code_end)?;
    if line_text.as_bytes().get(option_end.saturating_sub(1)) == Some(&b',') {
        option_end = option_end.saturating_sub(1);
    }
    let (option_start, option_end) = trimmed_byte_range(line_text, option_start, option_end)?;
    let span = source_span_for_line_range(first.span, option_start, option_end);
    parse_with_option_text(&line_text[option_start..option_end], span, owner_line)
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
    let Some(open) = tokens
        .iter()
        .find(|token| matches!(token.kind, TokenKind::Symbol(Symbol::LBrace)))
    else {
        return Vec::new();
    };
    let Some(close) = tokens
        .iter()
        .rfind(|token| matches!(token.kind, TokenKind::Symbol(Symbol::RBrace)))
    else {
        return Vec::new();
    };
    let Some(line_start) = first
        .span
        .start
        .checked_sub(first.span.column.saturating_sub(1))
    else {
        return Vec::new();
    };
    let Some(open_index) = open.span.start.checked_sub(line_start) else {
        return Vec::new();
    };
    let Some(close_index) = close.span.start.checked_sub(line_start) else {
        return Vec::new();
    };
    if close_index <= open_index {
        return Vec::new();
    }
    let inside_start = open_index + '{'.len_utf8();
    let inside = &line_text[inside_start..close_index];
    split_top_level_ranges(inside, &[';', ','])
        .into_iter()
        .filter_map(|(part_start, part_end)| {
            let (part_start, part_end) = trimmed_byte_range(inside, part_start, part_end)?;
            let line_start = inside_start + part_start;
            let line_end = inside_start + part_end;
            let span = source_span_for_line_range(first.span, line_start, line_end);
            parse_with_option_text(&line_text[line_start..line_end], span, owner_line)
        })
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
    let equals_index = text.find('=')?;
    let (key_start, key_end) = trimmed_byte_range(text, 0, equals_index)?;
    let (value_start, value_end) =
        trimmed_byte_range(text, equals_index + '='.len_utf8(), text.len())?;
    let key_text = &text[key_start..key_end];
    let value_text = &text[value_start..value_end];
    let key = if let Some(axis) = key_text.strip_prefix("unit ") {
        let axis = axis.trim();
        if axis.is_empty() {
            return None;
        }
        format!("unit {axis}")
    } else {
        key_text.to_owned()
    };
    Some(WithOptionDecl {
        owner_line,
        key,
        value: strip_wrapping_quotes(value_text),
        line: span.line,
        span,
        key_span: source_span_for_parent_range(span, key_start, key_end),
        value_span: source_span_for_parent_range(span, value_start, value_end),
    })
}

fn trimmed_byte_range(text: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let slice = text.get(start..end)?;
    let trimmed = slice.trim();
    if trimmed.is_empty() {
        return None;
    }
    let leading = slice.len() - slice.trim_start().len();
    let trimmed_start = start + leading;
    Some((trimmed_start, trimmed_start + trimmed.len()))
}

fn trimmed_source_parts(
    line_anchor: SourceSpan,
    line_text: &str,
    start: usize,
    end: usize,
) -> Option<(String, SourceSpan)> {
    let (trimmed_start, trimmed_end) = trimmed_byte_range(line_text, start, end)?;
    Some((
        line_text.get(trimmed_start..trimmed_end)?.to_owned(),
        source_span_for_line_range(line_anchor, trimmed_start, trimmed_end),
    ))
}

fn source_span_for_line_range(line_anchor: SourceSpan, start: usize, end: usize) -> SourceSpan {
    let line_start = line_anchor
        .start
        .saturating_sub(line_anchor.column.saturating_sub(1));
    SourceSpan::new_in_source(
        line_anchor.source_id,
        line_start + start,
        line_start + end,
        line_anchor.line,
        start + 1,
    )
}

fn source_text_for_span(line_text: &str, line_start: usize, span: SourceSpan) -> Option<&str> {
    let start = span.start.checked_sub(line_start)?;
    let end = span.end.checked_sub(line_start)?;
    line_text.get(start..end)
}

fn source_parts_for_tokens(
    tokens: &[Token],
    line_text: &str,
    line_start: usize,
) -> Option<(String, SourceSpan)> {
    let span = span_covering_tokens(tokens.first()?, tokens.last()?);
    Some((
        source_text_for_span(line_text, line_start, span)?.to_owned(),
        span,
    ))
}

fn source_span_for_parent_range(parent: SourceSpan, start: usize, end: usize) -> SourceSpan {
    SourceSpan::new_in_source(
        parent.source_id,
        parent.start + start,
        parent.start + end,
        parent.line,
        parent.column + start,
    )
}

fn source_span_for_subslice(
    line_anchor: SourceSpan,
    line_text: &str,
    subslice: &str,
) -> Option<SourceSpan> {
    let start = (subslice.as_ptr() as usize).checked_sub(line_text.as_ptr() as usize)?;
    let end = start.checked_add(subslice.len())?;
    if end > line_text.len()
        || !line_text.is_char_boundary(start)
        || !line_text.is_char_boundary(end)
    {
        return None;
    }
    Some(source_span_for_line_range(line_anchor, start, end))
}

fn source_span_for_parent_subslice(
    parent: SourceSpan,
    parent_text: &str,
    subslice: &str,
) -> Option<SourceSpan> {
    let start = (subslice.as_ptr() as usize).checked_sub(parent_text.as_ptr() as usize)?;
    let end = start.checked_add(subslice.len())?;
    if end > parent_text.len()
        || !parent_text.is_char_boundary(start)
        || !parent_text.is_char_boundary(end)
    {
        return None;
    }
    Some(source_span_for_parent_range(parent, start, end))
}

fn split_top_level_ranges(text: &str, separators: &[char]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            separator
                if paren_depth == 0
                    && bracket_depth == 0
                    && brace_depth == 0
                    && separators.contains(&separator) =>
            {
                ranges.push((start, index));
                start = index + separator.len_utf8();
            }
            _ => {}
        }
    }
    ranges.push((start, text.len()));
    ranges
}

fn parse_standalone_command_style_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<CommandStyleDecl> {
    if matches!(
        context,
        ParseContext::Args
            | ParseContext::Where
            | ParseContext::With
            | ParseContext::Schema
            | ParseContext::SchemaConstraints
            | ParseContext::SchemaMissing
            | ParseContext::Struct
            | ParseContext::Export
            | ParseContext::Class
            | ParseContext::ClassValidation
            | ParseContext::Object
            | ParseContext::System
            | ParseContext::Domain
            | ParseContext::Component
            | ParseContext::Equation
            | ParseContext::Expect
            | ParseContext::Test
    ) {
        return None;
    }
    if contains_symbol(tokens, Symbol::Equal) {
        return None;
    }
    let first = tokens.first()?;
    let last = tokens.last()?;
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let expression_start = first.span.start.checked_sub(line_start)?;
    let expression_end = last.span.end.checked_sub(line_start)?;
    let expression = line_text.get(expression_start..expression_end)?;
    let expression_span = source_span_for_line_range(first.span, expression_start, expression_end);
    parse_command_style_expression(expression, expression_span, first.span, context, None)
}

fn parse_command_style_expression(
    expression: &str,
    expression_span: SourceSpan,
    declaration_span: SourceSpan,
    context: ParseContext,
    owner: Option<&String>,
) -> Option<CommandStyleDecl> {
    let trimmed = expression.trim().trim_end_matches('{').trim();
    let trimmed_span = source_span_for_parent_subslice(expression_span, expression, trimmed)?;
    let (verb, rest) = split_first_word(trimmed)?;
    let rest_span = source_span_for_parent_subslice(trimmed_span, trimmed, rest)?;
    if !is_command_style_verb(verb) {
        if looks_like_unknown_command_style(verb, rest) {
            let parts = split_command_target_and_clauses(rest);
            return Some(CommandStyleDecl {
                verb: verb.to_owned(),
                target: parts.target,
                target_span: parts
                    .target_range
                    .map(|(start, end)| source_span_for_parent_range(rest_span, start, end)),
                clauses: command_clause_decls(&parts.clauses, rest_span),
                canonical: trimmed.to_owned(),
                status: "unknown_verb".to_owned(),
                owner: owner.cloned(),
                line: declaration_span.line,
                span: declaration_span,
                expression_span: trimmed_span,
                context,
            });
        }
        return None;
    }
    if trimmed.starts_with(&format!("{verb}("))
        || (verb == "rmse" && rest.trim_start().starts_with('('))
    {
        return None;
    }

    let parts = split_command_target_and_clauses_for_verb(verb, rest);
    let target = parts.target.as_str();
    let status = if verb == "rmse" && !valid_rmse_command_parts(&parts) {
        "invalid_rmse"
    } else if target.is_empty() {
        "missing_target"
    } else if command_target_is_ambiguous(verb, target) {
        "ambiguous_target"
    } else {
        "lowered"
    };
    let canonical = canonical_command_call(verb, target, &parts.clauses);
    Some(CommandStyleDecl {
        verb: verb.to_owned(),
        target: parts.target,
        target_span: parts
            .target_range
            .map(|(start, end)| source_span_for_parent_range(rest_span, start, end)),
        clauses: command_clause_decls(&parts.clauses, rest_span),
        canonical,
        status: status.to_owned(),
        owner: owner.cloned(),
        line: declaration_span.line,
        span: declaration_span,
        expression_span: trimmed_span,
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
        "integrate"
            | "mean"
            | "max"
            | "min"
            | "duration"
            | "plot"
            | "show"
            | "validate"
            | "check"
            | "fill"
            | "align"
            | "resample"
            | "render"
            | "apply"
            | "rmse"
    )
}

fn looks_like_unknown_command_style(verb: &str, rest: &str) -> bool {
    is_identifier_text(verb)
        && !is_non_command_style_statement_verb(verb)
        && !rest.trim().is_empty()
        && !top_level_clause_positions(
            rest,
            &[
                "over", "by", "as", "above", "below", "between", "from", "to", "with",
            ],
        )
        .is_empty()
}

fn is_non_command_style_statement_verb(verb: &str) -> bool {
    matches!(
        verb,
        "args"
            | "assert"
            | "class"
            | "component"
            | "const"
            | "copy"
            | "delete"
            | "derive"
            | "domain"
            | "download"
            | "export"
            | "filter"
            | "fn"
            | "golden"
            | "http"
            | "import"
            | "join"
            | "log"
            | "mkdir"
            | "move"
            | "print"
            | "promote"
            | "require_one"
            | "read"
            | "run"
            | "schema"
            | "script"
            | "select"
            | "sort"
            | "struct"
            | "summarize"
            | "system"
            | "test"
            | "train"
            | "use"
            | "write"
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CommandTargetParts {
    target: String,
    target_range: Option<(usize, usize)>,
    clauses: Vec<CommandClauseParts>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CommandClauseParts {
    name: String,
    name_range: (usize, usize),
    value: String,
    value_range: (usize, usize),
}

fn split_command_target_and_clauses(rest: &str) -> CommandTargetParts {
    split_command_target_and_clauses_with_keywords(
        rest,
        &[
            "over", "by", "as", "above", "below", "between", "from", "to", "with",
        ],
    )
}

fn split_command_target_and_clauses_for_verb(verb: &str, rest: &str) -> CommandTargetParts {
    if verb == "rmse" {
        split_command_target_and_clauses_with_keywords(rest, &["vs"])
    } else {
        split_command_target_and_clauses(rest)
    }
}

fn split_command_target_and_clauses_with_keywords(
    rest: &str,
    keywords: &[&str],
) -> CommandTargetParts {
    let positions = top_level_clause_positions(rest, keywords);
    let target_end = positions
        .first()
        .map(|(start, _)| *start)
        .unwrap_or(rest.len());
    let target_range = trimmed_byte_range(rest, 0, target_end);
    let target = target_range
        .and_then(|(start, end)| rest.get(start..end))
        .unwrap_or_default()
        .to_owned();
    if positions.is_empty() {
        return CommandTargetParts {
            target,
            target_range,
            clauses: Vec::new(),
        };
    }

    let mut clauses = Vec::new();
    for (index, (start, name)) in positions.iter().enumerate() {
        let value_start = start + name.len();
        let value_end = positions
            .get(index + 1)
            .map(|(next_start, _)| *next_start)
            .unwrap_or(rest.len());
        let Some(value_range) = trimmed_byte_range(rest, value_start, value_end) else {
            continue;
        };
        let value = rest[value_range.0..value_range.1].to_owned();
        clauses.push(CommandClauseParts {
            name: (*name).to_owned(),
            name_range: (*start, *start + name.len()),
            value,
            value_range,
        });
    }
    CommandTargetParts {
        target,
        target_range,
        clauses,
    }
}

fn valid_rmse_command_parts(parts: &CommandTargetParts) -> bool {
    parts.target.split('.').all(is_identifier_text)
        && parts.clauses.len() == 1
        && parts.clauses[0].name == "vs"
        && parts.clauses[0].value.split('.').all(is_identifier_text)
}

fn command_clause_decls(
    clauses: &[CommandClauseParts],
    rest_span: SourceSpan,
) -> Vec<CommandClauseDecl> {
    clauses
        .iter()
        .map(|clause| CommandClauseDecl {
            name: clause.name.clone(),
            name_span: source_span_for_parent_range(
                rest_span,
                clause.name_range.0,
                clause.name_range.1,
            ),
            value: clause.value.clone(),
            value_span: source_span_for_parent_range(
                rest_span,
                clause.value_range.0,
                clause.value_range.1,
            ),
        })
        .collect()
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

fn is_identifier_text(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn command_target_is_ambiguous(verb: &str, target: &str) -> bool {
    let target = target.trim();
    if target.starts_with('(') && target.ends_with(')') && balanced_delimiters(target) {
        return false;
    }
    if verb == "validate" {
        return false;
    }
    if verb == "check"
        && target
            .strip_prefix("coverage ")
            .is_some_and(|source| is_simple_dotted_identifier(source.trim()))
    {
        return false;
    }
    if verb == "fill"
        && target
            .strip_prefix("missing ")
            .is_some_and(|source| is_simple_dotted_identifier(source.trim()))
    {
        return false;
    }
    if matches!(verb, "align" | "resample") && is_simple_dotted_identifier(target) {
        return false;
    }
    if verb == "render"
        && target
            .strip_prefix("template ")
            .is_some_and(|source| !source.trim().is_empty())
    {
        return false;
    }
    if verb == "plot" && target.split(" and ").all(is_simple_dotted_identifier) {
        return false;
    }
    if target.split_whitespace().count() > 1 {
        return true;
    }
    target
        .chars()
        .any(|character| matches!(character, '+' | '-' | '*' | '/'))
}

fn is_simple_dotted_identifier(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && trimmed
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(is_word_character))
}

fn canonical_command_call(verb: &str, target: &str, clauses: &[CommandClauseParts]) -> String {
    if verb == "rmse" {
        let mut operands = vec![target.to_owned()];
        operands.extend(clauses.iter().map(|clause| clause.value.clone()));
        return format!("rmse({})", operands.join(", "));
    }
    let mut args = vec![target.to_owned()];
    for clause in clauses {
        let canonical_name = match (verb, clause.name.as_str()) {
            ("mean" | "max" | "min", "over") => "axis",
            _ => &clause.name,
        };
        args.push(format!("{canonical_name}={}", clause.value));
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

fn parse_print_decl(tokens: &[Token], line_text: &str, context: ParseContext) -> Option<PrintDecl> {
    let first = tokens.first()?;
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let (level, level_span, template, template_span, template_is_expression) = match first.kind {
        TokenKind::Keyword(Keyword::Print) => {
            if let Some(token) = tokens
                .iter()
                .skip(1)
                .find(|token| matches!(token.kind, TokenKind::StringLiteral(_)))
            {
                let TokenKind::StringLiteral(template) = &token.kind else {
                    unreachable!("filtered string literal")
                };
                (
                    "print".to_owned(),
                    None,
                    template.clone(),
                    string_literal_content_span(token),
                    false,
                )
            } else {
                let expression_tokens = tokens.get(1..)?;
                let template_span =
                    span_covering_tokens(expression_tokens.first()?, expression_tokens.last()?);
                let expression = source_text_for_span(line_text, line_start, template_span)?;
                (
                    "print".to_owned(),
                    None,
                    print_expression_template(expression)?,
                    template_span,
                    true,
                )
            }
        }
        TokenKind::Keyword(Keyword::Log) => {
            let level_token = tokens.get(1)?;
            let level = match &level_token.kind {
                TokenKind::Identifier(value) => value.clone(),
                TokenKind::Keyword(_) => level_token.lexeme.clone(),
                TokenKind::StringLiteral(_) => String::new(),
                _ => level_token.lexeme.clone(),
            };
            let template_start = if matches!(level_token.kind, TokenKind::StringLiteral(_)) {
                1
            } else {
                2
            };
            let template_token = tokens
                .iter()
                .skip(template_start)
                .find(|token| matches!(token.kind, TokenKind::StringLiteral(_)))?;
            let TokenKind::StringLiteral(template) = &template_token.kind else {
                unreachable!("filtered string literal")
            };
            (
                level,
                Some(level_token.span),
                template.clone(),
                string_literal_content_span(template_token),
                false,
            )
        }
        _ => return None,
    };
    Some(PrintDecl {
        level,
        level_span,
        template,
        template_span,
        template_is_expression,
        line: first.span.line,
        span: first.span,
        context,
    })
}

fn print_expression_template(expression: &str) -> Option<String> {
    let expression = expression.trim();
    if expression.is_empty() {
        return None;
    }
    if expression.contains('{') || expression.contains('}') {
        Some(expression.to_owned())
    } else {
        Some(format!("{{{expression}}}"))
    }
}

fn parse_csv_export_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<CsvExportDecl> {
    let [first, second, third, fourth, ..] = tokens else {
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
    let path_end = top_level_symbol_index(tokens, 4, Symbol::LBrace).unwrap_or(tokens.len());
    let path_tokens = tokens.get(4..path_end)?;
    if path_tokens.is_empty() {
        return None;
    }
    let path_span = span_covering_tokens(path_tokens.first()?, path_tokens.last()?);
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let path = source_text_for_span(line_text, line_start, path_span)?.to_owned();

    Some(CsvExportDecl {
        source: source.clone(),
        source_span: second.span,
        format: "csv".to_owned(),
        format_span: fourth.span,
        path,
        path_span,
        to_span: third.span,
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
    let content_end = tokens
        .last()
        .filter(|token| matches!(token.kind, TokenKind::Symbol(Symbol::Comma)))
        .map_or(tokens.len(), |_| tokens.len().saturating_sub(1));
    let content_tokens = tokens.get(..content_end)?;
    let as_index = top_level_as_index(content_tokens, 0)?;
    let with_index = top_level_keyword_index(content_tokens, as_index + 1, Keyword::With);
    let expression_tokens = content_tokens.get(..as_index)?;
    let unit_end = with_index.unwrap_or(content_tokens.len());
    let unit_tokens = content_tokens.get(as_index + 1..unit_end)?;
    if expression_tokens.is_empty() || unit_tokens.is_empty() {
        return None;
    }
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let expression_span =
        span_covering_tokens(expression_tokens.first()?, expression_tokens.last()?);
    let display_unit_span = span_covering_tokens(unit_tokens.first()?, unit_tokens.last()?);
    let expression_source = source_text_for_span(line_text, line_start, expression_span)?;
    let display_unit = source_text_for_span(line_text, line_start, display_unit_span)?.to_owned();
    let expression = parse_command_style_expression(
        expression_source,
        expression_span,
        first.span,
        context,
        None,
    )
    .map(|command| command.canonical)
    .unwrap_or_else(|| expression_source.to_owned());
    let (format, format_span, with_span) = if let Some(with_index) = with_index {
        let format_tokens = content_tokens.get(with_index + 1..)?;
        let format_span = (!format_tokens.is_empty()).then(|| {
            span_covering_tokens(
                format_tokens.first().expect("non-empty format tokens"),
                format_tokens.last().expect("non-empty format tokens"),
            )
        });
        let format = format_span.and_then(|span| {
            source_text_for_span(line_text, line_start, span).and_then(extract_quoted)
        });
        (format, format_span, Some(content_tokens[with_index].span))
    } else {
        (None, None, None)
    };

    Some(CsvExportFieldDecl {
        expression,
        expression_span,
        display_unit: Some(display_unit),
        display_unit_span: Some(display_unit_span),
        format,
        format_span,
        as_span: content_tokens[as_index].span,
        with_span,
        line: first.span.line,
        span: first.span,
    })
}

fn parse_write_decl(tokens: &[Token], line_text: &str, context: ParseContext) -> Option<WriteDecl> {
    let first = tokens.first()?;
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Write)) {
        return None;
    }
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let body = tokens.get(1..)?;
    if let Some(to_index) = top_level_keyword_index(body, 0, Keyword::To) {
        let expression_tokens = body.get(..to_index)?;
        let target_tokens = body.get(to_index + 1..)?;
        if !expression_tokens.is_empty() {
            if let Some(db_target) =
                parse_db_table_target_decl(target_tokens, line_text, line_start)
            {
                let (expression, expression_span) =
                    source_parts_for_tokens(expression_tokens, line_text, line_start)?;
                let path = db_target.expression.clone();
                let path_span = db_target.expression_span;
                return Some(WriteDecl {
                    format: "db".to_owned(),
                    format_span: None,
                    path,
                    path_span: Some(path_span),
                    db_target: Some(db_target),
                    expression,
                    expression_span,
                    to_span: Some(body[to_index].span),
                    line: first.span.line,
                    span: first.span,
                    context,
                });
            }
        }
    }
    let format_token = body.first()?;
    let format = format_token.lexeme.as_str();
    let rest = body.get(1..)?;
    if format == "standard_text" {
        let (path, path_span, expression, expression_span, to_span) =
            if let Some(comma_index) = top_level_symbol_index(rest, 0, Symbol::Comma) {
                let (path, path_span) =
                    source_parts_for_tokens(rest.get(..comma_index)?, line_text, line_start)?;
                let (expression, expression_span) =
                    source_parts_for_tokens(rest.get(comma_index + 1..)?, line_text, line_start)?;
                (path, Some(path_span), expression, expression_span, None)
            } else if let Some(to_index) = top_level_keyword_index(rest, 0, Keyword::To) {
                let (expression, expression_span) =
                    source_parts_for_tokens(rest.get(..to_index)?, line_text, line_start)?;
                let (path, path_span) =
                    source_parts_for_tokens(rest.get(to_index + 1..)?, line_text, line_start)?;
                (
                    path,
                    Some(path_span),
                    expression,
                    expression_span,
                    Some(rest[to_index].span),
                )
            } else {
                let (expression, expression_span) =
                    source_parts_for_tokens(rest, line_text, line_start)?;
                (String::new(), None, expression, expression_span, None)
            };
        return Some(WriteDecl {
            format: format.to_owned(),
            format_span: Some(format_token.span),
            path,
            path_span,
            db_target: None,
            expression,
            expression_span,
            to_span,
            line: first.span.line,
            span: first.span,
            context,
        });
    }
    let comma_index = top_level_symbol_index(rest, 0, Symbol::Comma)?;
    let (path, path_span) =
        source_parts_for_tokens(rest.get(..comma_index)?, line_text, line_start)?;
    let (expression, expression_span) =
        source_parts_for_tokens(rest.get(comma_index + 1..)?, line_text, line_start)?;
    Some(WriteDecl {
        format: format.to_owned(),
        format_span: Some(format_token.span),
        path,
        path_span: Some(path_span),
        db_target: None,
        expression,
        expression_span,
        to_span: None,
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
        TokenKind::Keyword(Keyword::Mkdir) => "mkdir",
        _ => return None,
    };
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let body = tokens.get(1..)?;
    let (source, source_span, destination, destination_span, to_span) =
        if matches!(operation, "copy" | "move") {
            let to_index = top_level_keyword_index(body, 0, Keyword::To)?;
            let (source, source_span) =
                source_parts_for_tokens(body.get(..to_index)?, line_text, line_start)?;
            let (destination, destination_span) =
                source_parts_for_tokens(body.get(to_index + 1..)?, line_text, line_start)?;
            (
                source,
                source_span,
                Some(destination),
                Some(destination_span),
                Some(body[to_index].span),
            )
        } else {
            let (source, source_span) = source_parts_for_tokens(body, line_text, line_start)?;
            (source, source_span, None, None, None)
        };
    Some(FileOperationDecl {
        operation: operation.to_owned(),
        source,
        source_span,
        destination,
        destination_span,
        to_span,
        line: first.span.line,
        span: first.span,
        context,
    })
}

fn parse_net_download_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<NetDownloadDecl> {
    let first = tokens.first()?;
    if !matches!(&first.kind, TokenKind::Identifier(value) if value == "download") {
        return None;
    }
    if contains_symbol(tokens, Symbol::Equal) {
        return None;
    }
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let rest_start = first.span.end.checked_sub(line_start)?;
    let code_end = tokens.last()?.span.end.checked_sub(line_start)?;
    let raw_rest = line_text.get(rest_start..code_end)?;
    let leading = raw_rest.len().checked_sub(raw_rest.trim_start().len())?;
    let trailing = raw_rest.len().checked_sub(raw_rest.trim_end().len())?;
    let rest_start = rest_start.checked_add(leading)?;
    let rest_end = code_end.checked_sub(trailing)?;
    let rest = line_text.get(rest_start..rest_end)?;
    let (url, target) = split_file_operation_to(rest)?;
    let url = url.trim();
    let target = target.trim();
    if url.is_empty() || target.is_empty() {
        return None;
    }
    let url_start = rest.find(url)?;
    let target_search_start = url_start.checked_add(url.len())?;
    let target_start =
        target_search_start.checked_add(rest.get(target_search_start..)?.find(target)?)?;
    let url_start = rest_start.checked_add(url_start)?;
    let target_start = rest_start.checked_add(target_start)?;
    Some(NetDownloadDecl {
        url: url.to_owned(),
        url_span: source_span_for_line_range(first.span, url_start, url_start + url.len()),
        target: target.to_owned(),
        target_span: source_span_for_line_range(
            first.span,
            target_start,
            target_start + target.len(),
        ),
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

fn parse_process_run_decl(
    tokens: &[Token],
    _line_text: &str,
    context: ParseContext,
) -> Option<ProcessRunDecl> {
    let first = tokens.first()?;
    let (binding, binding_span, rhs_start) = if tokens
        .get(1)
        .is_some_and(|token| matches!(token.kind, TokenKind::Symbol(Symbol::Equal)))
    {
        (Some(fast_binding_name(first)?), Some(first.span), 2)
    } else {
        (None, None, 0)
    };
    let rhs = tokens.get(rhs_start..)?;
    let run = rhs.first()?;
    let command_keyword = rhs.get(1)?;
    if run.lexeme != "run" || command_keyword.lexeme != "command" {
        return None;
    }
    let command_tokens = rhs.get(2..)?;
    let command_span = (!command_tokens.is_empty()).then(|| {
        span_covering_tokens(
            command_tokens.first().expect("non-empty command tokens"),
            command_tokens.last().expect("non-empty command tokens"),
        )
    });
    let command = command_tokens
        .iter()
        .find_map(|token| match &token.kind {
            TokenKind::StringLiteral(value) => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_default();
    Some(ProcessRunDecl {
        binding,
        binding_span,
        command,
        command_span,
        run_span: run.span,
        command_keyword_span: command_keyword.span,
        line: run.span.line,
        span: first.span,
        context,
    })
}

fn is_process_run_rhs(rhs: &str) -> bool {
    let mut parts = rhs.split_whitespace();
    matches!(parts.next(), Some("run")) && matches!(parts.next(), Some("command"))
}

fn parse_test_decl(tokens: &[Token], context: ParseContext) -> Option<TestDecl> {
    if context == ParseContext::With {
        return None;
    }
    let first = tokens.first()?;
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Test)) {
        return None;
    }
    let (name, name_span) = tokens
        .iter()
        .skip(1)
        .find_map(|token| match &token.kind {
            TokenKind::StringLiteral(value) => {
                Some((value.clone(), Some(string_literal_content_span(token))))
            }
            TokenKind::Identifier(value) => Some((value.clone(), Some(token.span))),
            _ => None,
        })
        .unwrap_or_else(|| (String::new(), None));
    Some(TestDecl {
        name,
        name_span,
        line: first.span.line,
        span: first.span,
        context,
    })
}

fn parse_assert_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<AssertDecl> {
    let first = tokens.first()?;
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Assert)) {
        return None;
    }
    let last = tokens.last()?;
    let line_start = first
        .span
        .start
        .checked_sub(first.span.column.checked_sub(1)?)?;
    let expression_start = first.span.end.checked_sub(line_start)?;
    let expression_end = last.span.end.checked_sub(line_start)?;
    let parts = trimmed_byte_range(line_text, expression_start, expression_end)
        .and_then(|(start, end)| {
            let expression = line_text.get(start..end)?;
            let expression_span = source_span_for_line_range(first.span, start, end);
            Some(split_assert_expression(expression, expression_span))
        })
        .unwrap_or_default();
    Some(AssertDecl {
        left: parts.left,
        left_span: parts.left_span,
        operator: parts.operator,
        operator_span: parts.operator_span,
        right: parts.right,
        right_span: parts.right_span,
        tolerance: parts.tolerance,
        tolerance_span: parts.tolerance_span,
        line: first.span.line,
        span: first.span,
        context,
    })
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct AssertExpressionParts {
    left: String,
    left_span: Option<SourceSpan>,
    operator: String,
    operator_span: Option<SourceSpan>,
    right: String,
    right_span: Option<SourceSpan>,
    tolerance: Option<String>,
    tolerance_span: Option<SourceSpan>,
}

fn split_assert_expression(expression: &str, expression_span: SourceSpan) -> AssertExpressionParts {
    let (comparison, tolerance) = expression
        .split_once(" within ")
        .map(|(comparison, tolerance)| (comparison.trim(), Some(tolerance.trim())))
        .unwrap_or((expression.trim(), None));
    for operator in ["==", "!=", ">=", "<=", ">", "<"] {
        if let Some((left, right)) = comparison.split_once(operator) {
            let left = left.trim();
            let right = right.trim();
            let operator_start = comparison.find(operator).unwrap_or(0);
            return AssertExpressionParts {
                left: left.to_owned(),
                left_span: source_span_for_parent_subslice(expression_span, expression, left),
                operator: operator.to_owned(),
                operator_span: Some(source_span_for_parent_range(
                    source_span_for_parent_subslice(expression_span, expression, comparison)
                        .unwrap_or(expression_span),
                    operator_start,
                    operator_start + operator.len(),
                )),
                right: right.to_owned(),
                right_span: source_span_for_parent_subslice(expression_span, expression, right),
                tolerance: tolerance.map(str::to_owned),
                tolerance_span: tolerance.and_then(|value| {
                    source_span_for_parent_subslice(expression_span, expression, value)
                }),
            };
        }
    }
    AssertExpressionParts {
        tolerance: tolerance.map(str::to_owned),
        tolerance_span: tolerance
            .and_then(|value| source_span_for_parent_subslice(expression_span, expression, value)),
        ..AssertExpressionParts::default()
    }
}

fn parse_golden_decl(
    tokens: &[Token],
    line_text: &str,
    context: ParseContext,
) -> Option<GoldenDecl> {
    let first = tokens.first()?;
    if !matches!(first.kind, TokenKind::Keyword(Keyword::Golden)) {
        return None;
    }
    let (artifact, artifact_span) = tokens
        .iter()
        .find_map(|token| match &token.kind {
            TokenKind::StringLiteral(value) => {
                Some((value.clone(), Some(string_literal_content_span(token))))
            }
            _ => None,
        })
        .unwrap_or_else(|| (String::new(), None));
    let expected = line_text
        .split_once(" matches ")
        .map(|(_, expected)| expected.trim().to_owned())
        .unwrap_or_default();
    Some(GoldenDecl {
        artifact,
        artifact_span,
        expected,
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
    let name = match &first.kind {
        TokenKind::Identifier(name) => name.clone(),
        TokenKind::Keyword(Keyword::Input | Keyword::Output)
            if matches!(
                context,
                ParseContext::Schema
                    | ParseContext::Args
                    | ParseContext::Struct
                    | ParseContext::Class
            ) =>
        {
            first.lexeme.clone()
        }
        _ => return None,
    };
    if !matches!(second.kind, TokenKind::Symbol(Symbol::Colon)) {
        return None;
    }

    let parts = typed_declaration_parts(first.span, line_text)?;

    Some(ExplicitDecl {
        name,
        name_span: first.span,
        type_name: parts.type_name,
        type_span: parts.type_span,
        unit: parts.unit,
        unit_span: parts.unit_span,
        expression: parts.expression,
        expression_span: parts.expression_span,
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

struct TypedAnnotationParts {
    type_name: String,
    type_span: SourceSpan,
    unit: Option<String>,
    unit_span: Option<SourceSpan>,
}

struct TypedDeclarationParts {
    type_name: String,
    type_span: SourceSpan,
    unit: Option<String>,
    unit_span: Option<SourceSpan>,
    expression: Option<String>,
    expression_span: Option<SourceSpan>,
}

fn typed_annotation_parts(
    line_anchor: SourceSpan,
    line_text: &str,
    start: usize,
    end: usize,
) -> Option<TypedAnnotationParts> {
    let (trimmed_start, trimmed_end) = trimmed_byte_range(line_text, start, end)?;
    let type_part = line_text.get(trimmed_start..trimmed_end)?;
    let (type_name, unit) = split_type_and_unit(type_part);
    if type_name.is_empty() {
        return None;
    }
    let (type_span, unit_span) =
        type_and_unit_source_spans_at(line_anchor, trimmed_start, type_part);
    Some(TypedAnnotationParts {
        type_name,
        type_span,
        unit,
        unit_span,
    })
}

fn typed_declaration_parts(
    line_anchor: SourceSpan,
    line_text: &str,
) -> Option<TypedDeclarationParts> {
    let raw_after_colon = line_text.split_once(':')?.1.trim();
    let (type_part, expression) = raw_after_colon
        .split_once('=')
        .map(|(left, right)| (left.trim(), Some(right.trim().to_owned())))
        .unwrap_or((raw_after_colon, None));
    let (type_name, unit) = split_type_and_unit(type_part);
    if type_name.is_empty() {
        return None;
    }
    let (type_span, unit_span) = type_and_unit_source_spans(line_anchor, line_text, type_part)?;
    Some(TypedDeclarationParts {
        type_name,
        type_span,
        unit,
        unit_span,
        expression,
        expression_span: source_span_after_equals(line_anchor, line_text),
    })
}

fn source_span_after_equals(line_anchor: SourceSpan, line_text: &str) -> Option<SourceSpan> {
    let expression_start = line_text.find('=')? + '='.len_utf8();
    let (start, end) = trimmed_byte_range(line_text, expression_start, line_text.len())?;
    Some(source_span_for_line_range(line_anchor, start, end))
}

fn source_span_after_equals_without_trailing_comma(
    line_anchor: SourceSpan,
    line_text: &str,
) -> Option<SourceSpan> {
    let expression_start = line_text.find('=')? + '='.len_utf8();
    let trimmed_end = line_text.trim_end().len();
    let expression_end = if line_text[..trimmed_end].ends_with(',') {
        trimmed_end.saturating_sub(','.len_utf8())
    } else {
        trimmed_end
    };
    let (start, end) = trimmed_byte_range(line_text, expression_start, expression_end)?;
    Some(source_span_for_line_range(line_anchor, start, end))
}

fn split_type_and_unit(type_part: &str) -> (String, Option<String>) {
    let trimmed = type_part.trim();
    if let Some(rest) = trimmed.strip_prefix("TimeSeries[") {
        if let Some((axis, after_axis)) = rest.split_once(']') {
            let after_axis = after_axis.trim();
            if let Some(quantity_part) = after_axis.strip_prefix("of ") {
                let (quantity, unit) = split_trailing_unit(quantity_part);
                return (
                    format!("TimeSeries[{}] of {}", axis.trim(), quantity.trim()),
                    unit,
                );
            }
        }
    }
    split_trailing_unit(trimmed)
}

fn type_and_unit_source_spans(
    line_anchor: SourceSpan,
    line_text: &str,
    type_part: &str,
) -> Option<(SourceSpan, Option<SourceSpan>)> {
    let trimmed = type_part.trim();
    let trimmed_start = subslice_start_after_colon(line_text, trimmed)?;
    Some(type_and_unit_source_spans_at(
        line_anchor,
        trimmed_start,
        trimmed,
    ))
}

fn type_and_unit_source_spans_at(
    line_anchor: SourceSpan,
    trimmed_start: usize,
    trimmed: &str,
) -> (SourceSpan, Option<SourceSpan>) {
    let (type_end, unit_range) = trailing_unit_source_range(trimmed)
        .map(|(type_end, unit_start, unit_end)| (type_end, Some((unit_start, unit_end))))
        .unwrap_or((trimmed.len(), None));
    let type_span = source_span_for_line_range(
        line_anchor,
        trimmed_start,
        trimmed_start.saturating_add(type_end),
    );
    let unit_span = unit_range.map(|(unit_start, unit_end)| {
        source_span_for_line_range(
            line_anchor,
            trimmed_start.saturating_add(unit_start),
            trimmed_start.saturating_add(unit_end),
        )
    });
    (type_span, unit_span)
}

fn trailing_unit_source_range(trimmed: &str) -> Option<(usize, usize, usize)> {
    if !trimmed.ends_with(']') {
        return None;
    }
    let unit_open = trimmed.rfind('[')?;
    if unit_open > 0
        && !trimmed[..unit_open]
            .chars()
            .last()
            .is_some_and(char::is_whitespace)
    {
        return None;
    }
    let unit_source = &trimmed[unit_open + '['.len_utf8()..trimmed.len() - ']'.len_utf8()];
    let unit = unit_source.trim();
    if unit.is_empty() {
        return None;
    }
    let unit_leading = unit_source.len() - unit_source.trim_start().len();
    let unit_start = unit_open + '['.len_utf8() + unit_leading;
    let type_end = trimmed[..unit_open].trim_end().len();
    Some((type_end, unit_start, unit_start + unit.len()))
}

fn subslice_start_after_colon(line_text: &str, subslice: &str) -> Option<usize> {
    let search_start = line_text.find(':')? + ':'.len_utf8();
    line_text
        .get(search_start..)?
        .find(subslice)
        .map(|relative| search_start + relative)
}

fn split_trailing_unit(type_part: &str) -> (String, Option<String>) {
    let trimmed = type_part.trim();
    if !trimmed.ends_with(']') {
        return (trimmed.to_owned(), None);
    }
    let Some(unit_start) = trimmed.rfind('[') else {
        return (trimmed.to_owned(), None);
    };
    if unit_start > 0
        && !trimmed[..unit_start]
            .chars()
            .last()
            .is_some_and(char::is_whitespace)
    {
        return (trimmed.to_owned(), None);
    }
    let unit = trimmed[unit_start + 1..trimmed.len() - 1].trim();
    if unit.is_empty() {
        return (trimmed.to_owned(), None);
    }
    (
        trimmed[..unit_start].trim().to_owned(),
        Some(unit.to_owned()),
    )
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

fn starts_state_space_type_block(tokens: &[Token]) -> bool {
    let Some(first) = tokens.first() else {
        return false;
    };
    if state_space_type_block_role(first).is_none() {
        return false;
    }
    tokens
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Symbol(Symbol::LBrace)))
}

fn starts_object_literal(tokens: &[Token]) -> bool {
    let [first, second, third, ..] = tokens else {
        return false;
    };
    matches!(first.kind, TokenKind::Identifier(_))
        && matches!(second.kind, TokenKind::Symbol(Symbol::Equal))
        && matches!(third.kind, TokenKind::Identifier(_))
        && tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::Symbol(Symbol::LBrace)))
}

fn starts_class_object_copy_literal(tokens: &[Token]) -> bool {
    let [first, second, third, fourth, ..] = tokens else {
        return false;
    };
    matches!(first.kind, TokenKind::Identifier(_))
        && matches!(second.kind, TokenKind::Symbol(Symbol::Equal))
        && matches!(third.kind, TokenKind::Identifier(_))
        && matches!(fourth.kind, TokenKind::Keyword(Keyword::With))
        && tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::Symbol(Symbol::LBrace)))
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
        TokenKind::Keyword(Keyword::Where | Keyword::On | Keyword::With)
            | TokenKind::Symbol(Symbol::LBrace | Symbol::RBrace)
    ) {
        return false;
    }
    if fast_binding_name(first).is_some()
        && tokens
            .get(1)
            .is_some_and(|token| matches!(token.kind, TokenKind::Symbol(Symbol::Equal)))
    {
        return true;
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
                    | Keyword::Log
                    | Keyword::Write
                    | Keyword::Copy
                    | Keyword::Move
                    | Keyword::Delete
                    | Keyword::Mkdir
                    | Keyword::Run
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
