use std::error::Error;
use std::fmt;

use eng_compiler::{BytecodeInstruction, BytecodeObject, BytecodeProgram, EntryPoint};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VmExecution {
    pub entry: EntryPoint,
    pub result_format: String,
    pub objects: Vec<VmObject>,
    pub steps: Vec<String>,
}

impl VmExecution {
    pub fn scalar_count(&self) -> usize {
        self.objects
            .iter()
            .filter(|object| object.kind == VmObjectKind::Scalar)
            .count()
    }

    pub fn table_count(&self) -> usize {
        self.objects
            .iter()
            .filter(|object| object.kind == VmObjectKind::Table)
            .count()
    }

    pub fn array_count(&self) -> usize {
        self.objects
            .iter()
            .filter(|object| object.kind == VmObjectKind::Array)
            .count()
    }

    pub fn timeseries_count(&self) -> usize {
        self.objects
            .iter()
            .filter(|object| object.kind == VmObjectKind::TimeSeries)
            .count()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VmObject {
    pub name: String,
    pub kind: VmObjectKind,
    pub type_name: String,
    pub axis: Option<String>,
    pub display_unit: Option<String>,
    pub row_count: Option<usize>,
    pub len: Option<usize>,
    pub source_hash: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VmObjectKind {
    Scalar,
    Table,
    TimeSeries,
    Array,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VmError {
    message: String,
}

impl fmt::Display for VmError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl Error for VmError {}

pub fn execute_bytecode(program: &BytecodeProgram) -> Result<VmExecution, VmError> {
    let mut objects = Vec::new();
    let mut steps = Vec::new();
    let mut result_format = None;

    for instruction in &program.instructions {
        match instruction {
            BytecodeInstruction::EnterEntry { kind, name } => {
                steps.push(format!("enter {kind} {name}"));
            }
            BytecodeInstruction::LoadScalar { name } => {
                let object = find_object(program, name)?;
                let BytecodeObject::Scalar {
                    quantity_kind,
                    display_unit,
                    ..
                } = object
                else {
                    return Err(vm_error(&format!("`{name}` is not a scalar object")));
                };
                objects.push(VmObject {
                    name: name.clone(),
                    kind: VmObjectKind::Scalar,
                    type_name: quantity_kind.clone(),
                    axis: None,
                    display_unit: Some(display_unit.clone()),
                    row_count: None,
                    len: None,
                    source_hash: None,
                });
                steps.push(format!("load scalar {name}"));
            }
            BytecodeInstruction::LoadTable { name } => {
                let object = find_object(program, name)?;
                let BytecodeObject::Table {
                    schema_name,
                    row_count,
                    source_hash,
                    ..
                } = object
                else {
                    return Err(vm_error(&format!("`{name}` is not a table object")));
                };
                objects.push(VmObject {
                    name: name.clone(),
                    kind: VmObjectKind::Table,
                    type_name: format!("Table[{schema_name}]"),
                    axis: Some("Time".to_owned()),
                    display_unit: None,
                    row_count: Some(*row_count),
                    len: None,
                    source_hash: source_hash.clone(),
                });
                steps.push(format!("load table {name}"));
            }
            BytecodeInstruction::LoadTimeSeries { name } => {
                let object = find_object(program, name)?;
                let BytecodeObject::TimeSeries {
                    axis,
                    quantity_kind,
                    display_unit,
                    ..
                } = object
                else {
                    return Err(vm_error(&format!("`{name}` is not a TimeSeries object")));
                };
                objects.push(VmObject {
                    name: name.clone(),
                    kind: VmObjectKind::TimeSeries,
                    type_name: format!("TimeSeries[{axis}] of {quantity_kind}"),
                    axis: Some(axis.clone()),
                    display_unit: Some(display_unit.clone()),
                    row_count: None,
                    len: None,
                    source_hash: None,
                });
                steps.push(format!("load timeseries {name}"));
            }
            BytecodeInstruction::LoadArray { name } => {
                let object = find_object(program, name)?;
                let BytecodeObject::Array {
                    element_type, len, ..
                } = object
                else {
                    return Err(vm_error(&format!("`{name}` is not an array object")));
                };
                objects.push(VmObject {
                    name: name.clone(),
                    kind: VmObjectKind::Array,
                    type_name: format!("Array[{element_type}]"),
                    axis: None,
                    display_unit: None,
                    row_count: None,
                    len: Some(*len),
                    source_hash: None,
                });
                steps.push(format!("load array {name}"));
            }
            BytecodeInstruction::WriteResult { format } => {
                result_format = Some(format.clone());
                steps.push(format!("write result {format}"));
            }
        }
    }

    let result_format =
        result_format.ok_or_else(|| vm_error("bytecode did not contain a result write"))?;

    Ok(VmExecution {
        entry: program.entry.clone(),
        result_format,
        objects,
        steps,
    })
}

fn find_object<'a>(
    program: &'a BytecodeProgram,
    name: &str,
) -> Result<&'a BytecodeObject, VmError> {
    program
        .objects
        .iter()
        .find(|object| match object {
            BytecodeObject::Scalar {
                name: object_name, ..
            }
            | BytecodeObject::Table {
                name: object_name, ..
            }
            | BytecodeObject::TimeSeries {
                name: object_name, ..
            }
            | BytecodeObject::Array {
                name: object_name, ..
            } => object_name == name,
        })
        .ok_or_else(|| vm_error(&format!("bytecode references unknown object `{name}`")))
}

fn vm_error(message: &str) -> VmError {
    VmError {
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eng_compiler::{
        build_bytecode_program, check_source, select_entry, BytecodeInstruction, CheckOptions,
    };

    #[test]
    fn executes_scalar_bytecode() {
        let source = "script main(args: Args) -> Report {\n    L = 1 m\n}\n";
        let report = check_source("ok.eng", source, &CheckOptions::default());
        let entry = select_entry(&report.semantic_program.entry_points, None).unwrap();
        let program = build_bytecode_program(&report, source, &entry);

        let execution = execute_bytecode(&program).unwrap();

        assert_eq!(execution.entry.name, "main");
        assert_eq!(execution.result_format, "engres-v1");
        assert_eq!(execution.scalar_count(), 1);
    }

    #[test]
    fn executes_timeseries_bytecode() {
        let source = "script main(args: Args) -> Report {\n    sensor = promote csv \"data/sensor.csv\" as SensorData\n    cp = 4180 J/kg/K\n    Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\n}\n";
        let report = check_source("ok.eng", source, &CheckOptions::default());
        let entry = select_entry(&report.semantic_program.entry_points, None).unwrap();
        let program = build_bytecode_program(&report, source, &entry);

        let execution = execute_bytecode(&program).unwrap();
        let timeseries = execution
            .objects
            .iter()
            .find(|object| object.kind == VmObjectKind::TimeSeries)
            .unwrap();

        assert_eq!(execution.timeseries_count(), 1);
        assert_eq!(timeseries.axis.as_deref(), Some("Time"));
    }

    #[test]
    fn supports_array_value_seed() {
        let program = BytecodeProgram {
            compiler_version: "test".to_owned(),
            source_hash: "hash".to_owned(),
            source_bytes: 0,
            source_lines: 0,
            token_count: 0,
            ast_item_count: 0,
            typed_binding_count: 1,
            schema_count: 0,
            csv_promotion_count: 0,
            entry: EntryPoint {
                kind: "script".to_owned(),
                name: "main".to_owned(),
                arg_name: Some("args".to_owned()),
                arg_type: Some("Args".to_owned()),
                return_type: Some("Report".to_owned()),
                line: 1,
            },
            objects: vec![BytecodeObject::Array {
                name: "samples".to_owned(),
                element_type: "Length".to_owned(),
                len: 3,
                line: 2,
            }],
            instructions: vec![
                BytecodeInstruction::EnterEntry {
                    kind: "script".to_owned(),
                    name: "main".to_owned(),
                },
                BytecodeInstruction::LoadArray {
                    name: "samples".to_owned(),
                },
                BytecodeInstruction::WriteResult {
                    format: "engres-v1".to_owned(),
                },
            ],
        };

        let execution = execute_bytecode(&program).unwrap();

        assert_eq!(execution.array_count(), 1);
        assert_eq!(execution.objects[0].type_name, "Array[Length]");
    }
}
