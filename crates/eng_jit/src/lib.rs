use std::collections::{BTreeMap, BTreeSet};

use eng_compiler::{CheckReport, ComponentAssemblyInfo};
use serde_json::{json, Value};

pub const KERNEL_PLAN_FORMAT: &str = "eng-kernel-plan-v1";
pub const KERNEL_IR_FORMAT: &str = "eng-kernel-ir-v1";
pub const DEFAULT_BACKEND_REQUEST: &str = "auto";
pub const INTERPRETER_FALLBACK_BACKEND: &str = "interpreter-fallback";
pub const NATIVE_PREVIEW_BACKEND: &str = "native-preview";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelCandidate {
    pub name: String,
    pub kind: String,
    pub line: usize,
    pub source: String,
    pub reason: String,
    pub lowering_status: String,
    pub operations: Vec<String>,
    pub estimate: KernelEstimate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelEstimate {
    pub estimated_rows: Option<usize>,
    pub input_count: usize,
    pub output_count: usize,
    pub operation_count: usize,
    pub scan_count: usize,
    pub complexity: String,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumericKernelPlan {
    pub format: String,
    pub backend: String,
    pub backend_selection: BackendSelection,
    pub candidates: Vec<KernelCandidate>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendSelection {
    pub requested: String,
    pub selected: String,
    pub status: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KernelIr {
    pub format: String,
    pub name: String,
    pub kind: String,
    pub input_count: usize,
    pub scalar_input_count: usize,
    pub output_count: usize,
    pub instructions: Vec<KernelInstruction>,
}

impl KernelIr {
    pub fn new(
        name: impl Into<String>,
        kind: impl Into<String>,
        input_count: usize,
        output_count: usize,
        instructions: Vec<KernelInstruction>,
    ) -> Self {
        Self {
            format: KERNEL_IR_FORMAT.to_owned(),
            name: name.into(),
            kind: kind.into(),
            input_count,
            scalar_input_count: 0,
            output_count,
            instructions,
        }
    }

    pub fn with_scalar_input_count(mut self, scalar_input_count: usize) -> Self {
        self.scalar_input_count = scalar_input_count;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum KernelInstruction {
    LoadInput {
        input: usize,
        register: usize,
    },
    LoadScalarInput {
        input: usize,
        register: usize,
    },
    LoadConstant {
        value: f64,
        register: usize,
    },
    Binary {
        op: KernelBinaryOp,
        left: usize,
        right: usize,
        target: usize,
    },
    StoreSeries {
        register: usize,
        output: usize,
    },
    StoreScalar {
        register: usize,
        output: usize,
    },
    IntegrateTrapezoid {
        input: usize,
        timestep_s: f64,
        output: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KernelBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct KernelExecutionInput {
    pub series_inputs: Vec<Vec<f64>>,
    pub scalar_inputs: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KernelExecutionOutput {
    pub backend: String,
    pub fallback_reason: Option<String>,
    pub outputs: Vec<KernelOutputValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum KernelOutputValue {
    Series(Vec<f64>),
    Scalar(f64),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelExecutionFailure {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JacobianKernelOutput {
    pub backend: String,
    pub fallback_reason: Option<String>,
    pub values: Vec<Vec<f64>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SolverStepKernelOutput {
    pub backend: String,
    pub fallback_reason: Option<String>,
    pub step: Vec<f64>,
    pub residual_norm: f64,
}

impl KernelExecutionFailure {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanOptions {
    pub requested_backend: String,
}

impl Default for PlanOptions {
    fn default() -> Self {
        Self {
            requested_backend: DEFAULT_BACKEND_REQUEST.to_owned(),
        }
    }
}

pub fn plan_for_report(report: &CheckReport) -> NumericKernelPlan {
    plan_for_report_with_options(report, &PlanOptions::default())
}

pub fn plan_for_report_with_options(
    report: &CheckReport,
    options: &PlanOptions,
) -> NumericKernelPlan {
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();
    let row_estimates = timeseries_row_estimates(report);
    let backend_selection = select_backend(&options.requested_backend);

    for stats in &report.semantic_program.stats_infos {
        let estimated_rows = row_estimates.get(&stats.source).copied();
        push_candidate(
            &mut candidates,
            &mut seen,
            KernelCandidate {
                name: format!("summary:{}", stats.source),
                kind: "statistics_fusion".to_owned(),
                line: stats.line,
                source: stats.source.clone(),
                reason: format!(
                    "{} statistics over {} can share one TimeSeries scan",
                    stats.statistics.len(),
                    stats.axis
                ),
                lowering_status: "lowerable_to_numeric_kernel_plan".to_owned(),
                operations: stats
                    .statistics
                    .iter()
                    .map(|statistic| format!("stat:{statistic}"))
                    .collect(),
                estimate: statistics_estimate(stats.statistics.len(), estimated_rows),
            },
        );
    }

    for integration in &report.semantic_program.integrations {
        let estimated_rows = row_estimates.get(&integration.source).copied();
        push_candidate(
            &mut candidates,
            &mut seen,
            KernelCandidate {
                name: integration.binding.clone(),
                kind: "timeseries_integrate".to_owned(),
                line: integration.line,
                source: integration.source.clone(),
                reason: format!(
                    "{} over {} lowers to a trapezoid-style numeric kernel",
                    integration.input_quantity, integration.over_axis
                ),
                lowering_status: "lowerable_to_numeric_kernel_plan".to_owned(),
                operations: vec![
                    format!("load_timeseries:{}", integration.source),
                    format!("integrate_over:{}", integration.over_axis),
                    format!("store:{}", integration.binding),
                ],
                estimate: integration_estimate(estimated_rows),
            },
        );
    }

    for hover in &report.semantic_program.hover_hints {
        let Some(expression) = &hover.expression else {
            continue;
        };
        if !hover.quantity_kind.starts_with("TimeSeries[") {
            continue;
        }
        if expression.contains('*') || expression.contains('+') || expression.contains('-') {
            let operations = elementwise_operations(expression);
            let estimated_rows = row_estimates.get(&hover.name).copied();
            push_candidate(
                &mut candidates,
                &mut seen,
                KernelCandidate {
                    name: hover.name.clone(),
                    kind: "timeseries_arithmetic".to_owned(),
                    line: hover.line,
                    source: expression.clone(),
                    reason: format!(
                        "{} expression can be lowered to element-wise numeric operations",
                        hover.quantity_kind
                    ),
                    lowering_status: "lowerable_to_numeric_kernel_plan".to_owned(),
                    estimate: elementwise_estimate(expression, &operations, estimated_rows),
                    operations,
                },
            );
        }
    }

    for system in &report.semantic_program.systems {
        for residual in &system.residuals {
            let operations = vec![
                format!("normalize_residual:{}", residual.name),
                "defer_rhs_codegen".to_owned(),
            ];
            push_candidate(
                &mut candidates,
                &mut seen,
                KernelCandidate {
                    name: format!("{}:{}", system.name, residual.name),
                    kind: "system_residual".to_owned(),
                    line: residual.line,
                    source: residual.expression.clone(),
                    reason: "system residual can feed a future RHS/Jacobian kernel".to_owned(),
                    lowering_status: "interface_only".to_owned(),
                    estimate: system_residual_estimate(&residual.expression, &operations),
                    operations,
                },
            );
        }
    }

    for assembly in &report.semantic_program.component_assemblies {
        if component_residual_ir_from_assembly(assembly).is_none() {
            continue;
        }
        push_candidate(
            &mut candidates,
            &mut seen,
            KernelCandidate {
                name: format!("{}:{}", assembly.name, assembly.residual_graph.name),
                kind: "component_residual_graph".to_owned(),
                line: assembly.line,
                source: assembly
                    .equations
                    .iter()
                    .map(|equation| equation.residual.as_str())
                    .collect::<Vec<_>>()
                    .join("; "),
                reason:
                    "component assembly residual graph lowers to a scalar residual evaluator kernel"
                        .to_owned(),
                lowering_status: "lowerable_to_numeric_kernel_plan".to_owned(),
                operations: vec![
                    format!("load_component_variables:{}", assembly.variables.len()),
                    format!("evaluate_component_residuals:{}", assembly.equations.len()),
                    "finite_difference_jacobian_ready".to_owned(),
                ],
                estimate: component_residual_estimate(assembly),
            },
        );
    }

    candidates.sort_by(|left, right| {
        left.line
            .cmp(&right.line)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.name.cmp(&right.name))
    });

    NumericKernelPlan {
        format: KERNEL_PLAN_FORMAT.to_owned(),
        backend: backend_selection.selected.clone(),
        backend_selection,
        candidates,
    }
}

pub fn plan_json(plan: &NumericKernelPlan) -> Value {
    json!({
        "format": plan.format,
        "backend": plan.backend,
        "backend_selection": {
            "requested": plan.backend_selection.requested,
            "selected": plan.backend_selection.selected,
            "status": plan.backend_selection.status,
            "reason": plan.backend_selection.reason,
        },
        "candidate_count": plan.candidates.len(),
        "candidates": plan.candidates.iter().map(candidate_json).collect::<Vec<_>>(),
    })
}

pub fn plan_json_string(plan: &NumericKernelPlan) -> String {
    plan_json(plan).to_string()
}

fn candidate_json(candidate: &KernelCandidate) -> Value {
    let (executor_status, executor_reason) = candidate_executor_status(candidate);
    json!({
        "name": candidate.name,
        "kind": candidate.kind,
        "line": candidate.line,
        "source": candidate.source,
        "reason": candidate.reason,
        "lowering_status": candidate.lowering_status,
        "operations": candidate.operations,
        "estimate": {
            "estimated_rows": candidate.estimate.estimated_rows,
            "input_count": candidate.estimate.input_count,
            "output_count": candidate.estimate.output_count,
            "operation_count": candidate.estimate.operation_count,
            "scan_count": candidate.estimate.scan_count,
            "complexity": candidate.estimate.complexity,
            "notes": candidate.estimate.notes,
        },
        "executor": {
            "backend": INTERPRETER_FALLBACK_BACKEND,
            "status": executor_status,
            "fallback_reason": executor_reason,
        },
    })
}

pub fn execute_interpreter_kernel(
    ir: &KernelIr,
    input: &KernelExecutionInput,
) -> Result<KernelExecutionOutput, KernelExecutionFailure> {
    validate_kernel_ir(ir)?;
    let row_count = validate_kernel_input(ir, input)?;
    let mut outputs = vec![None; ir.output_count];

    for instruction in &ir.instructions {
        if let KernelInstruction::IntegrateTrapezoid {
            input: input_index,
            timestep_s,
            output,
        } = instruction
        {
            if !timestep_s.is_finite() || *timestep_s <= 0.0 {
                return Err(KernelExecutionFailure::new(
                    "E-KERNEL-TIMESTEP",
                    "trapezoid integration timestep must be a positive finite number",
                ));
            }
            let values = &input.series_inputs[*input_index];
            let integral = values
                .windows(2)
                .map(|window| (window[0] + window[1]) * 0.5 * timestep_s)
                .sum::<f64>();
            store_output(&mut outputs, *output, KernelOutputValue::Scalar(integral))?;
        }
    }

    for row in 0..row_count {
        let mut registers = Vec::new();
        for instruction in &ir.instructions {
            execute_row_instruction(instruction, row, input, &mut registers, &mut outputs)?;
        }
    }

    let outputs = outputs
        .into_iter()
        .enumerate()
        .map(|(index, output)| {
            output.ok_or_else(|| {
                KernelExecutionFailure::new(
                    "E-KERNEL-OUTPUT-MISSING",
                    format!("kernel output slot {index} was not written"),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(KernelExecutionOutput {
        backend: INTERPRETER_FALLBACK_BACKEND.to_owned(),
        fallback_reason: None,
        outputs,
    })
}

pub fn execute_finite_difference_jacobian_kernel(
    residual_ir: &KernelIr,
    values: &[f64],
    finite_difference_step: f64,
) -> Result<JacobianKernelOutput, KernelExecutionFailure> {
    if values.is_empty() || residual_ir.scalar_input_count != values.len() {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-JACOBIAN-LAYOUT",
            "Jacobian kernel requires scalar inputs matching the variable vector",
        ));
    }
    if !finite_difference_step.is_finite() || finite_difference_step <= 0.0 {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-JACOBIAN-STEP",
            "Jacobian finite-difference step must be a positive finite number",
        ));
    }
    let baseline = execute_residual_scalar_kernel(residual_ir, values)?;
    if baseline.len() != values.len() {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-JACOBIAN-LAYOUT",
            "Jacobian kernel requires residual output count to match variable count",
        ));
    }

    let mut jacobian = vec![vec![0.0; values.len()]; values.len()];
    for column in 0..values.len() {
        let mut perturbed = values.to_vec();
        let step = finite_difference_step * values[column].abs().max(1.0);
        perturbed[column] += step;
        let residuals = execute_residual_scalar_kernel(residual_ir, &perturbed)?;
        if residuals.len() != baseline.len() {
            return Err(KernelExecutionFailure::new(
                "E-KERNEL-JACOBIAN-LAYOUT",
                "Jacobian residual output count changed during finite differencing",
            ));
        }
        for row in 0..baseline.len() {
            jacobian[row][column] = (residuals[row] - baseline[row]) / step;
        }
    }

    Ok(JacobianKernelOutput {
        backend: INTERPRETER_FALLBACK_BACKEND.to_owned(),
        fallback_reason: None,
        values: jacobian,
    })
}

pub fn execute_newton_step_kernel(
    jacobian: &[Vec<f64>],
    residuals: &[f64],
    tolerance: f64,
) -> Result<SolverStepKernelOutput, KernelExecutionFailure> {
    validate_newton_step_inputs(jacobian, residuals, tolerance)?;
    let rhs = residuals.iter().map(|value| -value).collect::<Vec<_>>();
    let step = solve_dense_kernel_system(jacobian, &rhs, tolerance)?;
    let residual_norm = residuals
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    Ok(SolverStepKernelOutput {
        backend: INTERPRETER_FALLBACK_BACKEND.to_owned(),
        fallback_reason: None,
        step,
        residual_norm,
    })
}

pub fn component_residual_ir_from_assembly(assembly: &ComponentAssemblyInfo) -> Option<KernelIr> {
    if assembly.variables.is_empty() || assembly.equations.is_empty() {
        return None;
    }
    let variable_indices = assembly
        .variables
        .iter()
        .enumerate()
        .map(|(index, variable)| (variable.name.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    let mut instructions = Vec::new();
    let mut next_register = 0;

    for (output, equation) in assembly.equations.iter().enumerate() {
        let mut accumulator = None;
        for (dependency_index, dependency) in equation.dependencies.iter().enumerate() {
            let input = *variable_indices.get(dependency.as_str())?;
            let load_register = next_register;
            next_register += 1;
            instructions.push(KernelInstruction::LoadScalarInput {
                input,
                register: load_register,
            });
            let coefficient = component_residual_coefficient(&equation.kind, dependency_index);
            accumulator = Some(append_signed_term(
                &mut instructions,
                &mut next_register,
                accumulator,
                load_register,
                coefficient,
            )?);
        }
        let mut residual_register = accumulator?;
        if let Some(rhs) = equation.rhs.as_deref().and_then(parse_leading_number) {
            if rhs.abs() > f64::EPSILON {
                let rhs_register = next_register;
                next_register += 1;
                instructions.push(KernelInstruction::LoadConstant {
                    value: rhs,
                    register: rhs_register,
                });
                let target = next_register;
                next_register += 1;
                instructions.push(KernelInstruction::Binary {
                    op: KernelBinaryOp::Sub,
                    left: residual_register,
                    right: rhs_register,
                    target,
                });
                residual_register = target;
            }
        }
        instructions.push(KernelInstruction::StoreScalar {
            register: residual_register,
            output,
        });
    }

    Some(
        KernelIr::new(
            format!("{}:{}", assembly.name, assembly.residual_graph.name),
            "component_residual_graph",
            0,
            assembly.equations.len(),
            instructions,
        )
        .with_scalar_input_count(assembly.variables.len()),
    )
}

fn push_candidate(
    candidates: &mut Vec<KernelCandidate>,
    seen: &mut BTreeSet<String>,
    candidate: KernelCandidate,
) {
    let key = format!("{}:{}:{}", candidate.kind, candidate.name, candidate.line);
    if seen.insert(key) {
        candidates.push(candidate);
    }
}

fn select_backend(requested: &str) -> BackendSelection {
    match requested {
        DEFAULT_BACKEND_REQUEST => BackendSelection {
            requested: requested.to_owned(),
            selected: INTERPRETER_FALLBACK_BACKEND.to_owned(),
            status: "selected".to_owned(),
            reason: "auto currently resolves to the interpreter fallback backend".to_owned(),
        },
        INTERPRETER_FALLBACK_BACKEND => BackendSelection {
            requested: requested.to_owned(),
            selected: INTERPRETER_FALLBACK_BACKEND.to_owned(),
            status: "selected".to_owned(),
            reason: "interpreter fallback is the only executable runtime optimization track path".to_owned(),
        },
        NATIVE_PREVIEW_BACKEND => BackendSelection {
            requested: requested.to_owned(),
            selected: INTERPRETER_FALLBACK_BACKEND.to_owned(),
            status: "not_available".to_owned(),
            reason: "native lowering backend selection is recorded, but no native backend is implemented".to_owned(),
        },
        other => BackendSelection {
            requested: other.to_owned(),
            selected: INTERPRETER_FALLBACK_BACKEND.to_owned(),
            status: "unknown_request".to_owned(),
            reason: "unknown backend request; falling back to interpreter metadata".to_owned(),
        },
    }
}

fn validate_kernel_ir(ir: &KernelIr) -> Result<(), KernelExecutionFailure> {
    if ir.format != KERNEL_IR_FORMAT {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-IR-FORMAT",
            "unsupported kernel IR format",
        ));
    }
    if (ir.input_count + ir.scalar_input_count) == 0
        || ir.output_count == 0
        || ir.instructions.is_empty()
    {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-IR-SHAPE",
            "kernel IR requires at least one input, one output, and one instruction",
        ));
    }
    for instruction in &ir.instructions {
        match instruction {
            KernelInstruction::LoadInput { input, .. } => {
                if *input >= ir.input_count {
                    return Err(KernelExecutionFailure::new(
                        "E-KERNEL-IR-INPUT",
                        "kernel instruction references an out-of-range input",
                    ));
                }
            }
            KernelInstruction::LoadScalarInput { input, .. } => {
                if *input >= ir.scalar_input_count {
                    return Err(KernelExecutionFailure::new(
                        "E-KERNEL-IR-SCALAR-INPUT",
                        "kernel instruction references an out-of-range scalar input",
                    ));
                }
            }
            KernelInstruction::StoreSeries { output, .. } => {
                if *output >= ir.output_count {
                    return Err(KernelExecutionFailure::new(
                        "E-KERNEL-IR-OUTPUT",
                        "kernel instruction references an out-of-range output",
                    ));
                }
            }
            KernelInstruction::StoreScalar { output, .. } => {
                if *output >= ir.output_count {
                    return Err(KernelExecutionFailure::new(
                        "E-KERNEL-IR-OUTPUT",
                        "kernel instruction references an out-of-range output",
                    ));
                }
            }
            KernelInstruction::IntegrateTrapezoid { input, output, .. } => {
                if *input >= ir.input_count {
                    return Err(KernelExecutionFailure::new(
                        "E-KERNEL-IR-INPUT",
                        "kernel instruction references an out-of-range input",
                    ));
                }
                if *output >= ir.output_count {
                    return Err(KernelExecutionFailure::new(
                        "E-KERNEL-IR-OUTPUT",
                        "kernel instruction references an out-of-range output",
                    ));
                }
            }
            KernelInstruction::LoadConstant { .. } | KernelInstruction::Binary { .. } => {}
        }
    }
    Ok(())
}

fn validate_kernel_input(
    ir: &KernelIr,
    input: &KernelExecutionInput,
) -> Result<usize, KernelExecutionFailure> {
    if input.series_inputs.len() != ir.input_count {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-INPUT-LAYOUT",
            "kernel input series count does not match the IR input count",
        ));
    }
    if input.scalar_inputs.len() != ir.scalar_input_count {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-SCALAR-INPUT-LAYOUT",
            "kernel scalar input count does not match the IR scalar input count",
        ));
    }
    let row_count = input
        .series_inputs
        .first()
        .map(|series| series.len())
        .unwrap_or(1);
    if input
        .series_inputs
        .iter()
        .any(|series| series.len() != row_count)
    {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-INPUT-LAYOUT",
            "kernel input series must have equal row counts",
        ));
    }
    if input
        .series_inputs
        .iter()
        .flatten()
        .chain(input.scalar_inputs.iter())
        .any(|value| !value.is_finite())
    {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-INPUT-FINITE",
            "kernel input series contain a non-finite value",
        ));
    }
    Ok(row_count)
}

fn execute_row_instruction(
    instruction: &KernelInstruction,
    row: usize,
    input: &KernelExecutionInput,
    registers: &mut Vec<f64>,
    outputs: &mut [Option<KernelOutputValue>],
) -> Result<(), KernelExecutionFailure> {
    match instruction {
        KernelInstruction::LoadInput {
            input: input_index,
            register,
        } => set_register(registers, *register, input.series_inputs[*input_index][row]),
        KernelInstruction::LoadScalarInput {
            input: input_index,
            register,
        } => set_register(registers, *register, input.scalar_inputs[*input_index]),
        KernelInstruction::LoadConstant { value, register } => {
            if !value.is_finite() {
                return Err(KernelExecutionFailure::new(
                    "E-KERNEL-CONSTANT-FINITE",
                    "kernel constant must be finite",
                ));
            }
            set_register(registers, *register, *value)
        }
        KernelInstruction::Binary {
            op,
            left,
            right,
            target,
        } => {
            let left = get_register(registers, *left)?;
            let right = get_register(registers, *right)?;
            let value = match op {
                KernelBinaryOp::Add => left + right,
                KernelBinaryOp::Sub => left - right,
                KernelBinaryOp::Mul => left * right,
                KernelBinaryOp::Div => {
                    if right.abs() <= f64::EPSILON {
                        return Err(KernelExecutionFailure::new(
                            "E-KERNEL-DIVIDE-BY-ZERO",
                            "kernel division denominator is zero",
                        ));
                    }
                    left / right
                }
            };
            set_register(registers, *target, value)
        }
        KernelInstruction::StoreSeries { register, output } => {
            let value = get_register(registers, *register)?;
            match &mut outputs[*output] {
                None => {
                    let mut values = vec![0.0; input.series_inputs[0].len()];
                    values[row] = value;
                    outputs[*output] = Some(KernelOutputValue::Series(values));
                    Ok(())
                }
                Some(KernelOutputValue::Series(values)) => {
                    values[row] = value;
                    Ok(())
                }
                Some(KernelOutputValue::Scalar(_)) => Err(KernelExecutionFailure::new(
                    "E-KERNEL-OUTPUT-KIND",
                    "kernel attempted to write a series into a scalar output slot",
                )),
            }
        }
        KernelInstruction::StoreScalar { register, output } => {
            let value = get_register(registers, *register)?;
            store_output(outputs, *output, KernelOutputValue::Scalar(value))
        }
        KernelInstruction::IntegrateTrapezoid { .. } => Ok(()),
    }
}

fn execute_residual_scalar_kernel(
    residual_ir: &KernelIr,
    values: &[f64],
) -> Result<Vec<f64>, KernelExecutionFailure> {
    let output = execute_interpreter_kernel(
        residual_ir,
        &KernelExecutionInput {
            series_inputs: Vec::new(),
            scalar_inputs: values.to_vec(),
        },
    )?;
    output
        .outputs
        .into_iter()
        .map(|value| match value {
            KernelOutputValue::Scalar(value) => Ok(value),
            KernelOutputValue::Series(_) => Err(KernelExecutionFailure::new(
                "E-KERNEL-RESIDUAL-OUTPUT",
                "residual kernel outputs must be scalar values",
            )),
        })
        .collect()
}

fn set_register(
    registers: &mut Vec<f64>,
    register: usize,
    value: f64,
) -> Result<(), KernelExecutionFailure> {
    if !value.is_finite() {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-VALUE-FINITE",
            "kernel instruction produced a non-finite value",
        ));
    }
    if register >= registers.len() {
        registers.resize(register + 1, 0.0);
    }
    registers[register] = value;
    Ok(())
}

fn get_register(registers: &[f64], register: usize) -> Result<f64, KernelExecutionFailure> {
    registers.get(register).copied().ok_or_else(|| {
        KernelExecutionFailure::new(
            "E-KERNEL-REGISTER",
            "kernel instruction references an unset register",
        )
    })
}

fn store_output(
    outputs: &mut [Option<KernelOutputValue>],
    output: usize,
    value: KernelOutputValue,
) -> Result<(), KernelExecutionFailure> {
    if outputs[output].is_some() {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-OUTPUT-DUPLICATE",
            "kernel output slot was written more than once",
        ));
    }
    outputs[output] = Some(value);
    Ok(())
}

fn candidate_executor_status(candidate: &KernelCandidate) -> (&'static str, &'static str) {
    if candidate.lowering_status == "lowerable_to_numeric_kernel_plan" {
        (
            "interpreter_supported",
            "candidate can execute through the interpreter kernel IR when runtime inputs are supplied",
        )
    } else {
        (
            "fallback_metadata_only",
            "candidate does not yet have an executable interpreter kernel lowering",
        )
    }
}

fn validate_newton_step_inputs(
    jacobian: &[Vec<f64>],
    residuals: &[f64],
    tolerance: f64,
) -> Result<(), KernelExecutionFailure> {
    let n = jacobian.len();
    if n == 0 || residuals.len() != n || jacobian.iter().any(|row| row.len() != n) {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-NEWTON-STEP-SHAPE",
            "Newton step kernel requires a non-empty square Jacobian and matching residual vector",
        ));
    }
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-NEWTON-STEP-TOLERANCE",
            "Newton step kernel tolerance must be a positive finite number",
        ));
    }
    if jacobian
        .iter()
        .flatten()
        .chain(residuals.iter())
        .any(|value| !value.is_finite())
    {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-NEWTON-STEP-FINITE",
            "Newton step kernel inputs must be finite",
        ));
    }
    Ok(())
}

fn solve_dense_kernel_system(
    matrix: &[Vec<f64>],
    rhs: &[f64],
    tolerance: f64,
) -> Result<Vec<f64>, KernelExecutionFailure> {
    let n = matrix.len();
    let mut a = matrix.to_vec();
    let mut b = rhs.to_vec();
    for pivot_index in 0..n {
        let mut best_row = pivot_index;
        let mut best_abs = a[pivot_index][pivot_index].abs();
        for (row_index, row) in a.iter().enumerate().skip(pivot_index + 1) {
            let value_abs = row[pivot_index].abs();
            if value_abs > best_abs {
                best_row = row_index;
                best_abs = value_abs;
            }
        }
        if best_abs <= tolerance {
            return Err(KernelExecutionFailure::new(
                "E-KERNEL-NEWTON-STEP-SINGULAR",
                "Newton step kernel Jacobian is singular or ill-conditioned",
            ));
        }
        if best_row != pivot_index {
            a.swap(best_row, pivot_index);
            b.swap(best_row, pivot_index);
        }

        let pivot = a[pivot_index][pivot_index];
        for column_index in pivot_index..n {
            a[pivot_index][column_index] /= pivot;
        }
        b[pivot_index] /= pivot;

        for row_index in 0..n {
            if row_index == pivot_index {
                continue;
            }
            let factor = a[row_index][pivot_index];
            if factor.abs() <= f64::EPSILON {
                continue;
            }
            for column_index in pivot_index..n {
                a[row_index][column_index] -= factor * a[pivot_index][column_index];
            }
            b[row_index] -= factor * b[pivot_index];
        }
    }
    Ok(b)
}

fn elementwise_operations(expression: &str) -> Vec<String> {
    let mut operations = Vec::new();
    if expression.contains('*') {
        operations.push("elementwise_mul".to_owned());
    }
    if expression.contains('/') {
        operations.push("elementwise_div".to_owned());
    }
    if expression.contains('+') {
        operations.push("elementwise_add".to_owned());
    }
    if expression.contains('-') {
        operations.push("elementwise_sub".to_owned());
    }
    if operations.is_empty() {
        operations.push("elementwise_eval".to_owned());
    }
    operations
}

fn timeseries_row_estimates(report: &CheckReport) -> BTreeMap<String, usize> {
    let csv_rows = report
        .semantic_program
        .csv_promotions
        .iter()
        .map(|promotion| (promotion.binding.clone(), promotion.row_count))
        .collect::<BTreeMap<_, _>>();
    let mut rows = csv_rows.clone();

    for hover in &report.semantic_program.hover_hints {
        if !hover.quantity_kind.starts_with("TimeSeries[") {
            continue;
        }
        let Some(expression) = &hover.expression else {
            continue;
        };
        if let Some(row_count) =
            expression_rows(expression, &rows).or_else(|| expression_rows(expression, &csv_rows))
        {
            rows.insert(hover.name.clone(), row_count);
        }
    }

    rows
}

fn expression_rows(expression: &str, rows: &BTreeMap<String, usize>) -> Option<usize> {
    rows.iter()
        .filter_map(|(binding, row_count)| {
            expression_references_binding(expression, binding).then_some(*row_count)
        })
        .max()
}

fn expression_references_binding(expression: &str, binding: &str) -> bool {
    if expression.contains(&format!("{binding}.")) {
        return true;
    }
    expression_tokens(expression).any(|token| token == binding)
}

fn statistics_estimate(statistic_count: usize, estimated_rows: Option<usize>) -> KernelEstimate {
    KernelEstimate {
        estimated_rows,
        input_count: 1,
        output_count: statistic_count.max(1),
        operation_count: statistic_count.max(1),
        scan_count: 1,
        complexity: "O(n) fused TimeSeries reduction".to_owned(),
        notes: vec![
            "single input series".to_owned(),
            "statistics can share one scan".to_owned(),
        ],
    }
}

fn integration_estimate(estimated_rows: Option<usize>) -> KernelEstimate {
    KernelEstimate {
        estimated_rows,
        input_count: 1,
        output_count: 1,
        operation_count: 2,
        scan_count: 1,
        complexity: "O(n) TimeSeries integration".to_owned(),
        notes: vec![
            "adjacent samples form trapezoid intervals".to_owned(),
            "stores one integrated quantity".to_owned(),
        ],
    }
}

fn elementwise_estimate(
    expression: &str,
    operations: &[String],
    estimated_rows: Option<usize>,
) -> KernelEstimate {
    let input_count = expression_source_count(expression).max(1);
    KernelEstimate {
        estimated_rows,
        input_count,
        output_count: 1,
        operation_count: operations.len().max(1),
        scan_count: 1,
        complexity: "O(n) element-wise TimeSeries expression".to_owned(),
        notes: vec![
            format!("{input_count} referenced source term(s)"),
            "one output series".to_owned(),
        ],
    }
}

fn system_residual_estimate(expression: &str, operations: &[String]) -> KernelEstimate {
    let input_count = expression_source_count(expression).max(1);
    let arithmetic_count = expression
        .chars()
        .filter(|value| matches!(value, '+' | '-' | '*' | '/' | '^'))
        .count()
        .max(1);
    KernelEstimate {
        estimated_rows: None,
        input_count,
        output_count: 1,
        operation_count: operations.len().max(arithmetic_count),
        scan_count: 0,
        complexity: "O(1) per residual evaluation before solver iteration scaling".to_owned(),
        notes: vec![
            format!("{input_count} referenced source term(s)"),
            "interface-only RHS/Jacobian boundary".to_owned(),
        ],
    }
}

fn component_residual_estimate(assembly: &ComponentAssemblyInfo) -> KernelEstimate {
    let dependency_count = assembly
        .equations
        .iter()
        .map(|equation| equation.dependencies.len())
        .sum::<usize>();
    let rhs_count = assembly
        .equations
        .iter()
        .filter(|equation| equation.rhs.is_some())
        .count();
    KernelEstimate {
        estimated_rows: None,
        input_count: assembly.variables.len(),
        output_count: assembly.equations.len(),
        operation_count: (dependency_count + rhs_count).max(1),
        scan_count: 0,
        complexity: "O(equations + dependencies) per residual evaluation".to_owned(),
        notes: vec![
            format!("{} component variable input(s)", assembly.variables.len()),
            format!("{} residual output(s)", assembly.equations.len()),
            format!("residual graph status: {}", assembly.residual_graph.status),
            format!("solver plan: {}", assembly.residual_graph.solver_plan),
        ],
    }
}

fn component_residual_coefficient(kind: &str, dependency_index: usize) -> f64 {
    match kind {
        "across_equality" if dependency_index == 1 => -1.0,
        _ => 1.0,
    }
}

fn append_signed_term(
    instructions: &mut Vec<KernelInstruction>,
    next_register: &mut usize,
    accumulator: Option<usize>,
    value_register: usize,
    coefficient: f64,
) -> Option<usize> {
    if !coefficient.is_finite() {
        return None;
    }
    if coefficient.abs() <= f64::EPSILON {
        return accumulator;
    }

    let sign = if coefficient < 0.0 { -1.0 } else { 1.0 };
    let magnitude = coefficient.abs();
    let term_register = if (magnitude - 1.0).abs() <= f64::EPSILON {
        value_register
    } else {
        let coefficient_register = *next_register;
        *next_register += 1;
        instructions.push(KernelInstruction::LoadConstant {
            value: magnitude,
            register: coefficient_register,
        });
        let target = *next_register;
        *next_register += 1;
        instructions.push(KernelInstruction::Binary {
            op: KernelBinaryOp::Mul,
            left: value_register,
            right: coefficient_register,
            target,
        });
        target
    };

    match accumulator {
        Some(accumulator) => {
            let target = *next_register;
            *next_register += 1;
            instructions.push(KernelInstruction::Binary {
                op: if sign > 0.0 {
                    KernelBinaryOp::Add
                } else {
                    KernelBinaryOp::Sub
                },
                left: accumulator,
                right: term_register,
                target,
            });
            Some(target)
        }
        None if sign > 0.0 => Some(term_register),
        None => {
            let zero_register = *next_register;
            *next_register += 1;
            instructions.push(KernelInstruction::LoadConstant {
                value: 0.0,
                register: zero_register,
            });
            let target = *next_register;
            *next_register += 1;
            instructions.push(KernelInstruction::Binary {
                op: KernelBinaryOp::Sub,
                left: zero_register,
                right: term_register,
                target,
            });
            Some(target)
        }
    }
}

fn parse_leading_number(text: &str) -> Option<f64> {
    text.split_whitespace().next()?.parse::<f64>().ok()
}

fn expression_source_count(expression: &str) -> usize {
    expression_tokens(expression)
        .filter(|token| {
            !token
                .chars()
                .all(|value| value.is_ascii_digit() || value == '.')
        })
        .filter(|token| !matches!(*token, "over" | "integrate" | "return" | "report"))
        .collect::<BTreeSet<_>>()
        .len()
}

fn expression_tokens(expression: &str) -> impl Iterator<Item = &str> {
    expression
        .split(|value: char| !(value.is_ascii_alphanumeric() || value == '_' || value == '.'))
        .filter(|token| !token.is_empty())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use eng_compiler::{check_file, CheckOptions};

    use super::*;

    #[test]
    fn detects_official_csv_hot_kernels() {
        let report = check_file(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/official/01_csv_plot/main.eng"),
            &CheckOptions::default(),
        )
        .expect("official CSV example should check");
        let plan = plan_for_report(&report);

        assert!(plan.candidates.iter().any(
            |candidate| candidate.kind == "timeseries_arithmetic" && candidate.name == "Q_coil"
        ));
        assert!(plan.candidates.iter().any(
            |candidate| candidate.kind == "timeseries_integrate" && candidate.name == "E_coil"
        ));
        assert!(plan
            .candidates
            .iter()
            .any(|candidate| candidate.kind == "statistics_fusion"
                && candidate.name == "summary:Q_coil"));
        let q_coil = plan
            .candidates
            .iter()
            .find(|candidate| candidate.name == "Q_coil")
            .expect("Q_coil candidate should exist");
        assert_eq!(q_coil.estimate.estimated_rows, Some(4));
        assert_eq!(q_coil.estimate.output_count, 1);
        assert!(q_coil.estimate.input_count >= 3);

        let json = plan_json(&plan);
        assert_eq!(json["format"], KERNEL_PLAN_FORMAT);
        assert_eq!(json["backend"], INTERPRETER_FALLBACK_BACKEND);
        assert_eq!(
            json["backend_selection"]["requested"],
            DEFAULT_BACKEND_REQUEST
        );
        assert!(json["candidate_count"].as_u64().unwrap() >= 3);
        assert_eq!(json["candidates"][0]["estimate"]["estimated_rows"], 4);

        let native_plan = plan_for_report_with_options(
            &report,
            &PlanOptions {
                requested_backend: NATIVE_PREVIEW_BACKEND.to_owned(),
            },
        );
        assert_eq!(native_plan.backend, INTERPRETER_FALLBACK_BACKEND);
        assert_eq!(
            native_plan.backend_selection.requested,
            NATIVE_PREVIEW_BACKEND
        );
        assert_eq!(native_plan.backend_selection.status, "not_available");
    }

    #[test]
    fn detects_component_assembly_residual_kernel_candidate() {
        let report = check_file(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/official/21_thermal_component_assembly/main.eng"),
            &CheckOptions::default(),
        )
        .expect("official thermal component assembly example should check");
        let plan = plan_for_report(&report);
        let candidate = plan
            .candidates
            .iter()
            .find(|candidate| candidate.kind == "component_residual_graph")
            .expect("component residual graph candidate should exist");

        assert_eq!(candidate.name, "component_graph:component_residual_graph");
        assert_eq!(
            candidate.lowering_status,
            "lowerable_to_numeric_kernel_plan"
        );
        assert_eq!(candidate.estimate.input_count, 4);
        assert_eq!(candidate.estimate.output_count, 4);
        assert!(candidate
            .operations
            .contains(&"finite_difference_jacobian_ready".to_owned()));

        let json = plan_json(&plan);
        let candidates = json["candidates"].as_array().unwrap();
        let component_candidate = candidates
            .iter()
            .find(|candidate| candidate["kind"] == "component_residual_graph")
            .expect("component residual graph candidate JSON should exist");
        assert_eq!(
            component_candidate["executor"]["status"],
            "interpreter_supported"
        );
    }

    #[test]
    fn lowers_component_assembly_residual_kernel_ir() {
        let report = check_file(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/official/21_thermal_component_assembly/main.eng"),
            &CheckOptions::default(),
        )
        .expect("official thermal component assembly example should check");
        let assembly = report
            .semantic_program
            .component_assemblies
            .first()
            .expect("component assembly should exist");
        let ir = component_residual_ir_from_assembly(assembly)
            .expect("component residual graph should lower to IR");

        assert_eq!(ir.kind, "component_residual_graph");
        assert_eq!(ir.input_count, 0);
        assert_eq!(ir.scalar_input_count, 4);
        assert_eq!(ir.output_count, 4);

        let output = execute_interpreter_kernel(
            &ir,
            &KernelExecutionInput {
                series_inputs: Vec::new(),
                scalar_inputs: vec![22.0, 1.0, 22.0, -1.0],
            },
        )
        .unwrap();
        assert_eq!(
            output.outputs,
            vec![
                KernelOutputValue::Scalar(0.0),
                KernelOutputValue::Scalar(0.0),
                KernelOutputValue::Scalar(0.0),
                KernelOutputValue::Scalar(0.0)
            ]
        );

        let jacobian =
            execute_finite_difference_jacobian_kernel(&ir, &[22.0, 1.0, 22.0, -1.0], 1e-6).unwrap();
        assert!((jacobian.values[0][0] - 1.0).abs() < 1e-8);
        assert!((jacobian.values[0][2] + 1.0).abs() < 1e-8);
        assert!((jacobian.values[1][1] - 1.0).abs() < 1e-8);
        assert!((jacobian.values[1][3] - 1.0).abs() < 1e-8);
        assert!((jacobian.values[3][1] - 1.0).abs() < 1e-8);
    }

    #[test]
    fn interpreter_executes_elementwise_kernel_ir() {
        let ir = KernelIr::new(
            "mul_add",
            "timeseries_arithmetic",
            2,
            1,
            vec![
                KernelInstruction::LoadInput {
                    input: 0,
                    register: 0,
                },
                KernelInstruction::LoadInput {
                    input: 1,
                    register: 1,
                },
                KernelInstruction::Binary {
                    op: KernelBinaryOp::Mul,
                    left: 0,
                    right: 1,
                    target: 2,
                },
                KernelInstruction::LoadConstant {
                    value: 1.0,
                    register: 3,
                },
                KernelInstruction::Binary {
                    op: KernelBinaryOp::Add,
                    left: 2,
                    right: 3,
                    target: 4,
                },
                KernelInstruction::StoreSeries {
                    register: 4,
                    output: 0,
                },
            ],
        );
        let output = execute_interpreter_kernel(
            &ir,
            &KernelExecutionInput {
                series_inputs: vec![vec![1.0, 2.0], vec![10.0, 20.0]],
                scalar_inputs: Vec::new(),
            },
        )
        .unwrap();

        assert_eq!(output.backend, INTERPRETER_FALLBACK_BACKEND);
        assert_eq!(output.fallback_reason, None);
        assert_eq!(
            output.outputs,
            vec![KernelOutputValue::Series(vec![11.0, 41.0])]
        );
    }

    #[test]
    fn interpreter_executes_trapezoid_integral_kernel_ir() {
        let ir = KernelIr::new(
            "integrate_q",
            "timeseries_integrate",
            1,
            1,
            vec![KernelInstruction::IntegrateTrapezoid {
                input: 0,
                timestep_s: 0.5,
                output: 0,
            }],
        );
        let output = execute_interpreter_kernel(
            &ir,
            &KernelExecutionInput {
                series_inputs: vec![vec![0.0, 2.0, 4.0]],
                scalar_inputs: Vec::new(),
            },
        )
        .unwrap();

        assert_eq!(output.outputs, vec![KernelOutputValue::Scalar(2.0)]);
    }

    #[test]
    fn interpreter_reports_input_shape_failure() {
        let ir = KernelIr::new(
            "bad_shape",
            "timeseries_arithmetic",
            2,
            1,
            vec![KernelInstruction::LoadInput {
                input: 0,
                register: 0,
            }],
        );
        let failure = execute_interpreter_kernel(
            &ir,
            &KernelExecutionInput {
                series_inputs: vec![vec![1.0], vec![1.0, 2.0]],
                scalar_inputs: Vec::new(),
            },
        )
        .unwrap_err();

        assert_eq!(failure.code, "E-KERNEL-INPUT-LAYOUT");
    }

    #[test]
    fn interpreter_executes_scalar_residual_kernel_ir() {
        let ir = small_linear_residual_ir();
        let output = execute_interpreter_kernel(
            &ir,
            &KernelExecutionInput {
                series_inputs: Vec::new(),
                scalar_inputs: vec![2.0, 1.0],
            },
        )
        .unwrap();

        assert_eq!(
            output.outputs,
            vec![
                KernelOutputValue::Scalar(0.0),
                KernelOutputValue::Scalar(0.0)
            ]
        );
    }

    #[test]
    fn interpreter_executes_finite_difference_jacobian_kernel() {
        let ir = small_linear_residual_ir();
        let output = execute_finite_difference_jacobian_kernel(&ir, &[2.0, 1.0], 1e-6).unwrap();

        assert_eq!(output.backend, INTERPRETER_FALLBACK_BACKEND);
        assert!((output.values[0][0] - 1.0).abs() < 1e-8);
        assert!((output.values[0][1] - 1.0).abs() < 1e-8);
        assert!((output.values[1][0] - 1.0).abs() < 1e-8);
        assert!((output.values[1][1] + 1.0).abs() < 1e-8);
    }

    #[test]
    fn interpreter_executes_newton_step_kernel() {
        let output = execute_newton_step_kernel(&[vec![2.0]], &[-1.0], 1e-9).unwrap();

        assert_eq!(output.backend, INTERPRETER_FALLBACK_BACKEND);
        assert_eq!(output.fallback_reason, None);
        assert_eq!(output.step, vec![0.5]);
        assert!((output.residual_norm - 1.0).abs() < 1e-9);
    }

    #[test]
    fn interpreter_reports_singular_newton_step_kernel() {
        let failure =
            execute_newton_step_kernel(&[vec![1.0, 2.0], vec![2.0, 4.0]], &[1.0, 2.0], 1e-9)
                .unwrap_err();

        assert_eq!(failure.code, "E-KERNEL-NEWTON-STEP-SINGULAR");
    }

    #[test]
    fn plan_json_reports_executor_status_and_fallback_reason() {
        let lowerable = KernelCandidate {
            name: "candidate".to_owned(),
            kind: "timeseries_arithmetic".to_owned(),
            line: 1,
            source: "a + b".to_owned(),
            reason: "test".to_owned(),
            lowering_status: "lowerable_to_numeric_kernel_plan".to_owned(),
            operations: vec!["elementwise_add".to_owned()],
            estimate: elementwise_estimate("a + b", &["elementwise_add".to_owned()], Some(2)),
        };
        let interface_only = KernelCandidate {
            lowering_status: "interface_only".to_owned(),
            ..lowerable.clone()
        };

        assert_eq!(
            candidate_json(&lowerable)["executor"]["status"],
            "interpreter_supported"
        );
        assert_eq!(
            candidate_json(&interface_only)["executor"]["status"],
            "fallback_metadata_only"
        );
        assert!(
            candidate_json(&interface_only)["executor"]["fallback_reason"]
                .as_str()
                .unwrap()
                .contains("does not yet have")
        );
    }

    fn small_linear_residual_ir() -> KernelIr {
        KernelIr::new(
            "linear_residual",
            "residual_evaluator",
            0,
            2,
            vec![
                KernelInstruction::LoadScalarInput {
                    input: 0,
                    register: 0,
                },
                KernelInstruction::LoadScalarInput {
                    input: 1,
                    register: 1,
                },
                KernelInstruction::Binary {
                    op: KernelBinaryOp::Add,
                    left: 0,
                    right: 1,
                    target: 2,
                },
                KernelInstruction::LoadConstant {
                    value: 3.0,
                    register: 3,
                },
                KernelInstruction::Binary {
                    op: KernelBinaryOp::Sub,
                    left: 2,
                    right: 3,
                    target: 4,
                },
                KernelInstruction::StoreScalar {
                    register: 4,
                    output: 0,
                },
                KernelInstruction::Binary {
                    op: KernelBinaryOp::Sub,
                    left: 0,
                    right: 1,
                    target: 5,
                },
                KernelInstruction::LoadConstant {
                    value: 1.0,
                    register: 6,
                },
                KernelInstruction::Binary {
                    op: KernelBinaryOp::Sub,
                    left: 5,
                    right: 6,
                    target: 7,
                },
                KernelInstruction::StoreScalar {
                    register: 7,
                    output: 1,
                },
            ],
        )
        .with_scalar_input_count(2)
    }
}
