use std::error::Error;
use std::fmt;

use crate::type_info::TypeInfoSource;
use crate::CheckReport;
use crate::Workflow;

pub const BYTECODE_FORMAT: &str = "engbc-v1";
pub const BYTECODE_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BytecodeProgram {
    pub compiler_version: String,
    pub source_hash: String,
    pub source_bytes: usize,
    pub source_lines: usize,
    pub token_count: usize,
    pub ast_item_count: usize,
    pub typed_binding_count: usize,
    pub schema_count: usize,
    pub csv_promotion_count: usize,
    pub workflow: Workflow,
    pub objects: Vec<BytecodeObject>,
    pub instructions: Vec<BytecodeInstruction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BytecodeObject {
    Scalar {
        name: String,
        quantity_kind: String,
        display_unit: String,
        line: usize,
    },
    Table {
        name: String,
        schema_name: String,
        row_count: usize,
        source_hash: Option<String>,
        line: usize,
    },
    TimeSeries {
        name: String,
        axis: String,
        quantity_kind: String,
        display_unit: String,
        line: usize,
    },
    Array {
        name: String,
        element_type: String,
        len: usize,
        line: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BytecodeInstruction {
    EnterWorkflow { kind: String },
    LoadScalar { name: String },
    LoadTable { name: String },
    LoadTimeSeries { name: String },
    LoadArray { name: String },
    WriteResult { format: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BytecodeParseError {
    message: String,
}

impl fmt::Display for BytecodeParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl Error for BytecodeParseError {}

pub fn build_bytecode_program(report: &CheckReport, source: &str) -> BytecodeProgram {
    let mut objects = Vec::new();

    for promotion in &report.semantic_program.csv_promotions {
        objects.push(BytecodeObject::Table {
            name: promotion.binding.clone(),
            schema_name: promotion.schema_name.clone(),
            row_count: promotion.row_count,
            source_hash: promotion.source_hash.clone(),
            line: promotion.line,
        });
    }

    for binding in &report.semantic_program.typed_bindings {
        if is_public_boundary_binding(report, &binding.name, binding.line) {
            continue;
        }
        if report
            .semantic_program
            .csv_promotions
            .iter()
            .any(|promotion| promotion.binding == binding.name)
        {
            continue;
        }

        if let Some((axis, quantity_kind)) =
            crate::stats::time_series_quantity(&binding.semantic_type.quantity_kind)
        {
            objects.push(BytecodeObject::TimeSeries {
                name: binding.name.clone(),
                axis,
                quantity_kind,
                display_unit: binding.semantic_type.display_unit.clone(),
                line: binding.line,
            });
        } else if let Some(element_type) = binding
            .semantic_type
            .quantity_kind
            .strip_prefix("Array[")
            .and_then(|value| value.strip_suffix(']'))
        {
            objects.push(BytecodeObject::Array {
                name: binding.name.clone(),
                element_type: element_type.to_owned(),
                len: 0,
                line: binding.line,
            });
        } else {
            objects.push(BytecodeObject::Scalar {
                name: binding.name.clone(),
                quantity_kind: binding.semantic_type.quantity_kind.clone(),
                display_unit: binding.semantic_type.display_unit.clone(),
                line: binding.line,
            });
        }
    }

    let mut instructions = Vec::new();
    instructions.push(BytecodeInstruction::EnterWorkflow {
        kind: report.semantic_program.workflow.kind.clone(),
    });
    for object in &objects {
        match object {
            BytecodeObject::Scalar { name, .. } => {
                instructions.push(BytecodeInstruction::LoadScalar { name: name.clone() });
            }
            BytecodeObject::Table { name, .. } => {
                instructions.push(BytecodeInstruction::LoadTable { name: name.clone() });
            }
            BytecodeObject::TimeSeries { name, .. } => {
                instructions.push(BytecodeInstruction::LoadTimeSeries { name: name.clone() });
            }
            BytecodeObject::Array { name, .. } => {
                instructions.push(BytecodeInstruction::LoadArray { name: name.clone() });
            }
        }
    }
    instructions.push(BytecodeInstruction::WriteResult {
        format: "engres-v1".to_owned(),
    });

    BytecodeProgram {
        compiler_version: crate::COMPILER_VERSION.to_owned(),
        source_hash: report.source_hash.clone(),
        source_bytes: source.len(),
        source_lines: report.syntax_summary.lines,
        token_count: report.syntax_summary.tokens,
        ast_item_count: report.syntax_summary.ast_items,
        typed_binding_count: report.semantic_program.typed_bindings.len(),
        schema_count: report.semantic_program.schemas.len(),
        csv_promotion_count: report.semantic_program.csv_promotions.len(),
        workflow: report.semantic_program.workflow.clone(),
        objects,
        instructions,
    }
}

fn is_public_boundary_binding(report: &CheckReport, name: &str, line: usize) -> bool {
    report.semantic_program.type_infos.iter().any(|info| {
        info.name == name
            && info.line == line
            && matches!(
                info.source,
                TypeInfoSource::PublicBoundary | TypeInfoSource::SystemBoundary
            )
    })
}

pub fn encode_bytecode(program: &BytecodeProgram) -> String {
    let mut bytecode = String::new();
    bytecode.push_str(&format!("ENGBYTECODE {BYTECODE_VERSION}\n"));
    bytecode.push_str(&format!("format = {BYTECODE_FORMAT}\n"));
    bytecode.push_str(&format!("bytecode_version = {BYTECODE_VERSION}\n"));
    bytecode.push_str(&format!(
        "compiler_version = {}\n",
        field_escape(&program.compiler_version)
    ));
    bytecode.push_str(&format!("source_hash = {}\n", program.source_hash));
    bytecode.push_str(&format!("source_bytes = {}\n", program.source_bytes));
    bytecode.push_str(&format!("source_lines = {}\n", program.source_lines));
    bytecode.push_str(&format!("tokens = {}\n", program.token_count));
    bytecode.push_str(&format!("ast_items = {}\n", program.ast_item_count));
    bytecode.push_str(&format!(
        "typed_bindings = {}\n",
        program.typed_binding_count
    ));
    bytecode.push_str(&format!("schemas = {}\n", program.schema_count));
    bytecode.push_str(&format!(
        "csv_promotions = {}\n",
        program.csv_promotion_count
    ));
    bytecode.push_str(&format!(
        "workflow = {}\n",
        field_escape(&program.workflow.kind)
    ));
    bytecode.push_str(&format!(
        "workflow_args = {}:{}\n",
        field_escape(program.workflow.arg_name.as_deref().unwrap_or("args")),
        field_escape(program.workflow.arg_type.as_deref().unwrap_or("Args"))
    ));
    bytecode.push_str(&format!(
        "workflow_return = {}\n",
        field_escape(program.workflow.return_type.as_deref().unwrap_or("Report"))
    ));

    bytecode.push_str("objects:\n");
    for object in &program.objects {
        bytecode.push_str(&encode_object(object));
        bytecode.push('\n');
    }

    bytecode.push_str("instructions:\n");
    for (index, instruction) in program.instructions.iter().enumerate() {
        bytecode.push_str(&encode_instruction(index, instruction));
        bytecode.push('\n');
    }

    bytecode
}

pub fn parse_bytecode(source: &str) -> Result<BytecodeProgram, BytecodeParseError> {
    let mut lines = source.lines();
    let Some(magic) = lines.next() else {
        return Err(parse_error("empty bytecode"));
    };
    if magic.trim() != format!("ENGBYTECODE {BYTECODE_VERSION}") {
        return Err(parse_error("unsupported bytecode magic or version"));
    }

    let mut compiler_version = String::new();
    let mut source_hash = String::new();
    let mut source_bytes = 0usize;
    let mut source_lines = 0usize;
    let mut token_count = 0usize;
    let mut ast_item_count = 0usize;
    let mut typed_binding_count = 0usize;
    let mut schema_count = 0usize;
    let mut csv_promotion_count = 0usize;
    let mut workflow_kind = String::new();
    let mut workflow_arg_name = None;
    let mut workflow_arg_type = None;
    let mut workflow_return = None;
    let mut objects = Vec::new();
    let mut instructions = Vec::new();
    let mut section = Section::Header;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match trimmed {
            "objects:" => {
                section = Section::Objects;
                continue;
            }
            "instructions:" => {
                section = Section::Instructions;
                continue;
            }
            _ => {}
        }

        match section {
            Section::Header => {
                let (key, value) = split_assignment(trimmed)?;
                match key {
                    "format" if value != BYTECODE_FORMAT => {
                        return Err(parse_error("unsupported bytecode format"));
                    }
                    "format" | "bytecode_version" => {}
                    "compiler_version" => compiler_version = field_unescape(value),
                    "source_hash" => value.clone_into(&mut source_hash),
                    "source_bytes" => source_bytes = parse_usize(value, key)?,
                    "source_lines" => source_lines = parse_usize(value, key)?,
                    "tokens" => token_count = parse_usize(value, key)?,
                    "ast_items" => ast_item_count = parse_usize(value, key)?,
                    "typed_bindings" => typed_binding_count = parse_usize(value, key)?,
                    "schemas" => schema_count = parse_usize(value, key)?,
                    "csv_promotions" => csv_promotion_count = parse_usize(value, key)?,
                    "workflow" => workflow_kind = field_unescape(value),
                    "workflow_args" => {
                        let (name, arg_type) = value
                            .split_once(':')
                            .ok_or_else(|| parse_error("workflow_args must be `<name>:<type>`"))?;
                        workflow_arg_name = Some(field_unescape(name));
                        workflow_arg_type = Some(field_unescape(arg_type));
                    }
                    "workflow_return" => workflow_return = Some(field_unescape(value)),
                    _ => {}
                }
            }
            Section::Objects => objects.push(parse_object(trimmed)?),
            Section::Instructions => instructions.push(parse_instruction(trimmed)?),
        }
    }

    if workflow_kind.is_empty() {
        return Err(parse_error("bytecode is missing workflow metadata"));
    }

    Ok(BytecodeProgram {
        compiler_version,
        source_hash,
        source_bytes,
        source_lines,
        token_count,
        ast_item_count,
        typed_binding_count,
        schema_count,
        csv_promotion_count,
        workflow: Workflow {
            kind: workflow_kind,
            arg_name: workflow_arg_name,
            arg_type: workflow_arg_type,
            return_type: workflow_return,
            line: 1,
        },
        objects,
        instructions,
    })
}

fn encode_object(object: &BytecodeObject) -> String {
    match object {
        BytecodeObject::Scalar {
            name,
            quantity_kind,
            display_unit,
            line,
        } => format!(
            "scalar|{}|{}|{}|{}",
            field_escape(name),
            field_escape(quantity_kind),
            field_escape(display_unit),
            line
        ),
        BytecodeObject::Table {
            name,
            schema_name,
            row_count,
            source_hash,
            line,
        } => format!(
            "table|{}|{}|{}|{}|{}",
            field_escape(name),
            field_escape(schema_name),
            row_count,
            field_escape(source_hash.as_deref().unwrap_or("null")),
            line
        ),
        BytecodeObject::TimeSeries {
            name,
            axis,
            quantity_kind,
            display_unit,
            line,
        } => format!(
            "timeseries|{}|{}|{}|{}|{}",
            field_escape(name),
            field_escape(axis),
            field_escape(quantity_kind),
            field_escape(display_unit),
            line
        ),
        BytecodeObject::Array {
            name,
            element_type,
            len,
            line,
        } => format!(
            "array|{}|{}|{}|{}",
            field_escape(name),
            field_escape(element_type),
            len,
            line
        ),
    }
}

fn parse_object(line: &str) -> Result<BytecodeObject, BytecodeParseError> {
    let parts = line.split('|').collect::<Vec<_>>();
    match parts.as_slice() {
        ["scalar", name, quantity_kind, display_unit, line] => Ok(BytecodeObject::Scalar {
            name: field_unescape(name),
            quantity_kind: field_unescape(quantity_kind),
            display_unit: field_unescape(display_unit),
            line: parse_usize(line, "scalar line")?,
        }),
        ["table", name, schema_name, row_count, source_hash, line] => Ok(BytecodeObject::Table {
            name: field_unescape(name),
            schema_name: field_unescape(schema_name),
            row_count: parse_usize(row_count, "table row_count")?,
            source_hash: if *source_hash == "null" {
                None
            } else {
                Some(field_unescape(source_hash))
            },
            line: parse_usize(line, "table line")?,
        }),
        ["timeseries", name, axis, quantity_kind, display_unit, line] => {
            Ok(BytecodeObject::TimeSeries {
                name: field_unescape(name),
                axis: field_unescape(axis),
                quantity_kind: field_unescape(quantity_kind),
                display_unit: field_unescape(display_unit),
                line: parse_usize(line, "timeseries line")?,
            })
        }
        ["array", name, element_type, len, line] => Ok(BytecodeObject::Array {
            name: field_unescape(name),
            element_type: field_unescape(element_type),
            len: parse_usize(len, "array len")?,
            line: parse_usize(line, "array line")?,
        }),
        _ => Err(parse_error("invalid object record")),
    }
}

fn encode_instruction(index: usize, instruction: &BytecodeInstruction) -> String {
    match instruction {
        BytecodeInstruction::EnterWorkflow { kind } => {
            format!("{index:04}|enter_workflow|{}", field_escape(kind))
        }
        BytecodeInstruction::LoadScalar { name } => {
            format!("{index:04}|load_scalar|{}", field_escape(name))
        }
        BytecodeInstruction::LoadTable { name } => {
            format!("{index:04}|load_table|{}", field_escape(name))
        }
        BytecodeInstruction::LoadTimeSeries { name } => {
            format!("{index:04}|load_timeseries|{}", field_escape(name))
        }
        BytecodeInstruction::LoadArray { name } => {
            format!("{index:04}|load_array|{}", field_escape(name))
        }
        BytecodeInstruction::WriteResult { format } => {
            format!("{index:04}|write_result|{}", field_escape(format))
        }
    }
}

fn parse_instruction(line: &str) -> Result<BytecodeInstruction, BytecodeParseError> {
    let parts = line.split('|').collect::<Vec<_>>();
    match parts.as_slice() {
        [_index, "enter_workflow", kind] => Ok(BytecodeInstruction::EnterWorkflow {
            kind: field_unescape(kind),
        }),
        [_index, "load_scalar", name] => Ok(BytecodeInstruction::LoadScalar {
            name: field_unescape(name),
        }),
        [_index, "load_table", name] => Ok(BytecodeInstruction::LoadTable {
            name: field_unescape(name),
        }),
        [_index, "load_timeseries", name] => Ok(BytecodeInstruction::LoadTimeSeries {
            name: field_unescape(name),
        }),
        [_index, "load_array", name] => Ok(BytecodeInstruction::LoadArray {
            name: field_unescape(name),
        }),
        [_index, "write_result", format] => Ok(BytecodeInstruction::WriteResult {
            format: field_unescape(format),
        }),
        _ => Err(parse_error("invalid instruction record")),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Section {
    Header,
    Objects,
    Instructions,
}

fn split_assignment(line: &str) -> Result<(&str, &str), BytecodeParseError> {
    line.split_once(" = ")
        .map(|(key, value)| (key.trim(), value.trim()))
        .ok_or_else(|| parse_error("expected `key = value` header"))
}

fn parse_usize(value: &str, name: &str) -> Result<usize, BytecodeParseError> {
    value
        .parse::<usize>()
        .map_err(|_| parse_error(&format!("{name} must be an unsigned integer")))
}

fn parse_error(message: &str) -> BytecodeParseError {
    BytecodeParseError {
        message: message.to_owned(),
    }
}

fn field_escape(value: &str) -> String {
    value.replace('%', "%25").replace('|', "%7C")
}

fn field_unescape(value: &str) -> String {
    value.replace("%7C", "|").replace("%25", "%")
}
