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
    ReduceSeries {
        input: usize,
        op: KernelStatisticOp,
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

#[derive(Clone, Debug, PartialEq)]
pub enum KernelStatisticOp {
    Mean,
    TimeWeightedMean,
    Max,
    Min,
    Median,
    PopulationStd,
    NearestRankPercentile(f64),
    DurationAbove(f64),
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
        if assembly.variables.len() == assembly.equations.len() {
            push_candidate(
                &mut candidates,
                &mut seen,
                KernelCandidate {
                    name: format!("{}:{}:jacobian", assembly.name, assembly.residual_graph.name),
                    kind: "component_residual_jacobian".to_owned(),
                    line: assembly.line,
                    source: assembly
                        .equations
                        .iter()
                        .map(|equation| equation.residual.as_str())
                        .collect::<Vec<_>>()
                        .join("; "),
                    reason: "square component residual graph can execute finite-difference Jacobian evaluation"
                        .to_owned(),
                    lowering_status: "lowerable_to_numeric_kernel_plan".to_owned(),
                    operations: vec![
                        format!("load_component_variables:{}", assembly.variables.len()),
                        format!("evaluate_component_residuals:{}", assembly.equations.len()),
                        format!("finite_difference_columns:{}", assembly.variables.len()),
                        format!(
                            "store_dense_jacobian:{}x{}",
                            assembly.equations.len(),
                            assembly.variables.len()
                        ),
                    ],
                    estimate: component_jacobian_estimate(assembly),
                },
            );
        }
    }

    for system in &report.semantic_program.systems {
        let Some(ir) = state_space_rhs_ir_for_system(report, &system.name) else {
            continue;
        };
        let state_count = ir.output_count;
        let input_count = ir.scalar_input_count.saturating_sub(state_count);
        push_candidate(
            &mut candidates,
            &mut seen,
            KernelCandidate {
                name: system.name.clone(),
                kind: "state_space_rhs".to_owned(),
                line: system.line,
                source: state_space_rhs_source(system),
                reason: "continuous state-space A/B operators lower to a scalar RHS kernel"
                    .to_owned(),
                lowering_status: "lowerable_to_numeric_kernel_plan".to_owned(),
                operations: state_space_rhs_operations(report, &system.name),
                estimate: state_space_rhs_estimate(state_count, input_count, &ir.instructions),
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
        match instruction {
            KernelInstruction::IntegrateTrapezoid {
                input: input_index,
                timestep_s,
                output,
            } => {
                let integral = kernel_trapezoid_integral_uniform(
                    &input.series_inputs[*input_index],
                    *timestep_s,
                )?;
                store_output(&mut outputs, *output, KernelOutputValue::Scalar(integral))?;
            }
            KernelInstruction::ReduceSeries {
                input: input_index,
                op,
                timestep_s,
                output,
            } => {
                let value = execute_statistic_reduction(
                    &input.series_inputs[*input_index],
                    op,
                    *timestep_s,
                )?;
                store_output(&mut outputs, *output, KernelOutputValue::Scalar(value))?;
            }
            _ => {}
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

pub fn state_space_rhs_ir_for_system(report: &CheckReport, system_name: &str) -> Option<KernelIr> {
    let system = report
        .semantic_program
        .systems
        .iter()
        .find(|system| system.name == system_name)?;
    if !system
        .equations
        .iter()
        .any(|equation| equation.left.trim().starts_with("der("))
    {
        return None;
    }
    let state_vector = report
        .semantic_program
        .state_space_vectors
        .iter()
        .find(|vector| vector.system == system_name && vector.role == "states")?;
    let input_vector = report
        .semantic_program
        .state_space_vectors
        .iter()
        .find(|vector| vector.system == system_name && vector.role == "inputs")?;
    let operator_a = report
        .semantic_program
        .linear_operators
        .iter()
        .find(|operator| {
            operator.system == system_name
                && operator.from == "StateVector"
                && operator.to == "Derivative[StateVector]"
                && operator.status == "shape_checked"
        })?;
    let operator_b = report
        .semantic_program
        .linear_operators
        .iter()
        .find(|operator| {
            operator.system == system_name
                && operator.from == "InputVector"
                && operator.to == "Derivative[StateVector]"
                && operator.status == "shape_checked"
        })?;
    let matrix_a = parse_numeric_matrix(operator_a.expression.as_deref()?)?;
    let matrix_b = parse_numeric_matrix(operator_b.expression.as_deref()?)?;
    let state_count = state_vector.members.len();
    let input_count = input_vector.members.len();
    if state_count == 0
        || matrix_a.len() != state_count
        || matrix_a.iter().any(|row| row.len() != state_count)
        || matrix_b.len() != state_count
        || matrix_b.iter().any(|row| row.len() != input_count)
    {
        return None;
    }

    let mut instructions = Vec::new();
    let mut next_register = 0;
    for row in 0..state_count {
        let mut accumulator = None;
        for column in 0..state_count {
            accumulator = append_matrix_term(
                &mut instructions,
                &mut next_register,
                accumulator,
                column,
                matrix_a[row][column],
            )?;
        }
        for column in 0..input_count {
            accumulator = append_matrix_term(
                &mut instructions,
                &mut next_register,
                accumulator,
                state_count + column,
                matrix_b[row][column],
            )?;
        }
        let output_register = match accumulator {
            Some(register) => register,
            None => {
                let register = next_register;
                next_register += 1;
                instructions.push(KernelInstruction::LoadConstant {
                    value: 0.0,
                    register,
                });
                register
            }
        };
        instructions.push(KernelInstruction::StoreScalar {
            register: output_register,
            output: row,
        });
    }

    Some(
        KernelIr::new(
            format!("{system_name}:state_space_rhs"),
            "state_space_rhs",
            0,
            state_count,
            instructions,
        )
        .with_scalar_input_count(state_count + input_count),
    )
}

pub fn timeseries_integrate_ir_for_binding(
    report: &CheckReport,
    binding: &str,
    timestep_s: f64,
) -> Option<KernelIr> {
    if !timestep_s.is_finite() || timestep_s <= 0.0 {
        return None;
    }
    let integration = report
        .semantic_program
        .integrations
        .iter()
        .find(|integration| integration.binding == binding)?;
    Some(KernelIr::new(
        integration.binding.clone(),
        "timeseries_integrate",
        1,
        1,
        vec![KernelInstruction::IntegrateTrapezoid {
            input: 0,
            timestep_s,
            output: 0,
        }],
    ))
}

pub fn timeseries_statistics_ir_for_source(
    report: &CheckReport,
    source: &str,
    timestep_s: f64,
) -> Option<KernelIr> {
    if !timestep_s.is_finite() || timestep_s <= 0.0 {
        return None;
    }
    let stats = report
        .semantic_program
        .stats_infos
        .iter()
        .find(|stats| stats.source == source)?;
    let mut instructions = Vec::new();
    for (output, statistic) in stats.statistics.iter().enumerate() {
        instructions.push(KernelInstruction::ReduceSeries {
            input: 0,
            op: statistic_reduction_op(statistic)?,
            timestep_s,
            output,
        });
    }
    Some(KernelIr::new(
        format!("summary:{source}"),
        "statistics_fusion",
        1,
        stats.statistics.len(),
        instructions,
    ))
}

pub fn timeseries_arithmetic_ir_for_binding(
    report: &CheckReport,
    binding: &str,
) -> Option<KernelIr> {
    let expression = report
        .semantic_program
        .hover_hints
        .iter()
        .find(|hover| hover.name == binding)?
        .expression
        .as_deref()?;
    let tokens = tokenize_arithmetic_expression(expression)?;
    let mut builder = ArithmeticIrBuilder::default();
    let mut position = 0;
    let output_register = parse_add_sub(&tokens, &mut position, &mut builder)?;
    if position != tokens.len() {
        return None;
    }
    builder.instructions.push(KernelInstruction::StoreSeries {
        register: output_register,
        output: 0,
    });
    Some(
        KernelIr::new(
            binding.to_owned(),
            "timeseries_arithmetic",
            builder.series_inputs.len(),
            1,
            builder.instructions,
        )
        .with_scalar_input_count(builder.scalar_inputs.len()),
    )
}

fn statistic_reduction_op(name: &str) -> Option<KernelStatisticOp> {
    match name {
        "mean" => Some(KernelStatisticOp::Mean),
        "time_weighted_mean" => Some(KernelStatisticOp::TimeWeightedMean),
        "max" => Some(KernelStatisticOp::Max),
        "min" => Some(KernelStatisticOp::Min),
        "median" => Some(KernelStatisticOp::Median),
        "std" => Some(KernelStatisticOp::PopulationStd),
        percentile if percentile_fraction(percentile).is_some() => Some(
            KernelStatisticOp::NearestRankPercentile(percentile_fraction(percentile)?),
        ),
        duration if duration.starts_with("duration_above(") => Some(
            KernelStatisticOp::DurationAbove(duration_above_threshold(duration)?),
        ),
        _ => None,
    }
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
            KernelInstruction::ReduceSeries {
                input,
                op,
                timestep_s,
                output,
            } => {
                if *input >= ir.input_count {
                    return Err(KernelExecutionFailure::new(
                        "E-KERNEL-IR-INPUT",
                        "kernel instruction references an out-of-range input",
                    ));
                }
                if !timestep_s.is_finite() || *timestep_s <= 0.0 {
                    return Err(KernelExecutionFailure::new(
                        "E-KERNEL-TIMESTEP",
                        "statistics reduction timestep must be a positive finite number",
                    ));
                }
                validate_statistic_op(op)?;
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

fn validate_statistic_op(op: &KernelStatisticOp) -> Result<(), KernelExecutionFailure> {
    match op {
        KernelStatisticOp::NearestRankPercentile(percentile)
            if !percentile.is_finite() || *percentile <= 0.0 || *percentile > 1.0 =>
        {
            Err(KernelExecutionFailure::new(
                "E-KERNEL-STAT-PERCENTILE",
                "percentile reduction requires a percentile in (0, 1]",
            ))
        }
        KernelStatisticOp::DurationAbove(threshold) if !threshold.is_finite() => {
            Err(KernelExecutionFailure::new(
                "E-KERNEL-STAT-THRESHOLD",
                "duration-above threshold must be finite",
            ))
        }
        _ => Ok(()),
    }
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
        KernelInstruction::IntegrateTrapezoid { .. } | KernelInstruction::ReduceSeries { .. } => {
            Ok(())
        }
    }
}

fn execute_statistic_reduction(
    values: &[f64],
    op: &KernelStatisticOp,
    timestep_s: f64,
) -> Result<f64, KernelExecutionFailure> {
    if values.is_empty() {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-STAT-EMPTY",
            "statistics reduction requires at least one value",
        ));
    }
    match op {
        KernelStatisticOp::Mean => Ok(values.iter().sum::<f64>() / values.len() as f64),
        KernelStatisticOp::TimeWeightedMean => {
            let duration = timestep_s * values.len().saturating_sub(1) as f64;
            if duration <= 0.0 {
                return Err(KernelExecutionFailure::new(
                    "E-KERNEL-STAT-DURATION",
                    "time-weighted mean requires at least two samples",
                ));
            }
            Ok(kernel_trapezoid_integral_uniform(values, timestep_s)? / duration)
        }
        KernelStatisticOp::Max => values.iter().copied().reduce(f64::max).ok_or_else(|| {
            KernelExecutionFailure::new(
                "E-KERNEL-STAT-EMPTY",
                "max reduction requires at least one value",
            )
        }),
        KernelStatisticOp::Min => values.iter().copied().reduce(f64::min).ok_or_else(|| {
            KernelExecutionFailure::new(
                "E-KERNEL-STAT-EMPTY",
                "min reduction requires at least one value",
            )
        }),
        KernelStatisticOp::Median => kernel_median(values),
        KernelStatisticOp::PopulationStd => Ok(kernel_population_std(values)),
        KernelStatisticOp::NearestRankPercentile(percentile) => {
            kernel_nearest_rank_percentile(values, *percentile)
        }
        KernelStatisticOp::DurationAbove(threshold) => {
            kernel_duration_above(values, *threshold, timestep_s)
        }
    }
}

fn kernel_trapezoid_integral_uniform(
    values: &[f64],
    timestep_s: f64,
) -> Result<f64, KernelExecutionFailure> {
    if !timestep_s.is_finite() || timestep_s <= 0.0 {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-TIMESTEP",
            "trapezoid integration timestep must be a positive finite number",
        ));
    }
    if values.len() < 2 {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-INTEGRATE-SAMPLES",
            "trapezoid integration requires at least two samples",
        ));
    }
    Ok(values
        .windows(2)
        .map(|window| (window[0] + window[1]) * 0.5 * timestep_s)
        .sum::<f64>())
}

fn kernel_median(values: &[f64]) -> Result<f64, KernelExecutionFailure> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let midpoint = sorted.len() / 2;
    if sorted.len() & 1 == 0 {
        Ok((sorted[midpoint - 1] + sorted[midpoint]) * 0.5)
    } else {
        sorted.get(midpoint).copied().ok_or_else(|| {
            KernelExecutionFailure::new(
                "E-KERNEL-STAT-EMPTY",
                "median reduction requires at least one value",
            )
        })
    }
}

fn kernel_population_std(values: &[f64]) -> f64 {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

fn kernel_nearest_rank_percentile(
    values: &[f64],
    percentile: f64,
) -> Result<f64, KernelExecutionFailure> {
    if !percentile.is_finite() || percentile <= 0.0 || percentile > 1.0 {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-STAT-PERCENTILE",
            "percentile reduction requires a percentile in (0, 1]",
        ));
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = (percentile * sorted.len() as f64).ceil() as usize;
    sorted.get(rank.saturating_sub(1)).copied().ok_or_else(|| {
        KernelExecutionFailure::new(
            "E-KERNEL-STAT-EMPTY",
            "percentile reduction requires at least one value",
        )
    })
}

fn kernel_duration_above(
    values: &[f64],
    threshold: f64,
    timestep_s: f64,
) -> Result<f64, KernelExecutionFailure> {
    if !threshold.is_finite() {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-STAT-THRESHOLD",
            "duration-above threshold must be finite",
        ));
    }
    if values.len() < 2 {
        return Err(KernelExecutionFailure::new(
            "E-KERNEL-STAT-DURATION",
            "duration-above reduction requires at least two samples",
        ));
    }
    let mut duration = 0.0;
    for window in values.windows(2) {
        let y0 = window[0];
        let y1 = window[1];
        let y0_above = y0 > threshold;
        let y1_above = y1 > threshold;
        if y0_above && y1_above {
            duration += timestep_s;
        } else if y0_above != y1_above {
            let dy = y1 - y0;
            if dy.abs() <= f64::EPSILON {
                continue;
            }
            let fraction = ((threshold - y0) / dy).clamp(0.0, 1.0);
            duration += if y0_above {
                fraction * timestep_s
            } else {
                (1.0 - fraction) * timestep_s
            };
        }
    }
    Ok(duration)
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

pub fn candidate_executor_status(candidate: &KernelCandidate) -> (&'static str, &'static str) {
    if candidate.lowering_status == "lowerable_to_numeric_kernel_plan"
        && candidate_has_interpreter_lowering(candidate)
    {
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

fn candidate_has_interpreter_lowering(candidate: &KernelCandidate) -> bool {
    match candidate.kind.as_str() {
        "timeseries_arithmetic"
        | "timeseries_integrate"
        | "component_residual_graph"
        | "component_residual_jacobian"
        | "state_space_rhs" => true,
        "statistics_fusion" => candidate.operations.iter().all(|operation| {
            operation
                .strip_prefix("stat:")
                .is_some_and(|statistic| statistic_reduction_op(statistic).is_some())
        }),
        _ => false,
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

fn component_jacobian_estimate(assembly: &ComponentAssemblyInfo) -> KernelEstimate {
    let variable_count = assembly.variables.len();
    let equation_count = assembly.equations.len();
    let dependency_count = assembly
        .equations
        .iter()
        .map(|equation| equation.dependencies.len())
        .sum::<usize>();
    KernelEstimate {
        estimated_rows: None,
        input_count: variable_count,
        output_count: equation_count * variable_count,
        operation_count: ((variable_count + 1) * dependency_count.max(1)).max(1),
        scan_count: 0,
        complexity: "O(variables * residual-evaluation) finite-difference Jacobian".to_owned(),
        notes: vec![
            format!("{variable_count} component variable input(s)"),
            format!("{equation_count} residual equation(s)"),
            format!("{equation_count}x{variable_count} dense Jacobian output"),
            "uses the scalar residual interpreter kernel at perturbed variable values".to_owned(),
        ],
    }
}

fn state_space_rhs_estimate(
    state_count: usize,
    input_count: usize,
    instructions: &[KernelInstruction],
) -> KernelEstimate {
    let binary_count = instructions
        .iter()
        .filter(|instruction| matches!(instruction, KernelInstruction::Binary { .. }))
        .count();
    KernelEstimate {
        estimated_rows: None,
        input_count: state_count + input_count,
        output_count: state_count,
        operation_count: binary_count.max(1),
        scan_count: 0,
        complexity: "O(states * (states + inputs)) per RHS evaluation".to_owned(),
        notes: vec![
            format!("{state_count} state input(s)"),
            format!("{input_count} external input(s)"),
            "continuous A/B RHS kernel; fixed-step loop remains in the runtime solver".to_owned(),
        ],
    }
}

fn state_space_rhs_source(system: &eng_compiler::SystemInfo) -> String {
    system
        .equations
        .iter()
        .find(|equation| equation.left.trim().starts_with("der("))
        .map(|equation| format!("{} eq {}", equation.left, equation.right))
        .unwrap_or_else(|| "der(x) eq A * x + B * u".to_owned())
}

fn state_space_rhs_operations(report: &CheckReport, system_name: &str) -> Vec<String> {
    let mut operations = Vec::new();
    operations.extend(
        report
            .semantic_program
            .state_space_vectors
            .iter()
            .filter(|vector| vector.system == system_name && vector.role == "states")
            .map(|vector| format!("load_state_vector:{}", vector.name)),
    );
    operations.extend(
        report
            .semantic_program
            .state_space_vectors
            .iter()
            .filter(|vector| vector.system == system_name && vector.role == "inputs")
            .map(|vector| format!("load_input_vector:{}", vector.name)),
    );
    operations.extend(
        report
            .semantic_program
            .linear_operators
            .iter()
            .filter(|operator| {
                operator.system == system_name
                    && operator.to == "Derivative[StateVector]"
                    && (operator.from == "StateVector" || operator.from == "InputVector")
            })
            .map(|operator| format!("apply_linear_operator:{}", operator.name)),
    );
    operations.push("store_derivative_vector:Derivative[StateVector]".to_owned());
    operations
}

fn component_residual_coefficient(kind: &str, dependency_index: usize) -> f64 {
    match kind {
        "across_equality" if dependency_index == 1 => -1.0,
        _ => 1.0,
    }
}

fn append_matrix_term(
    instructions: &mut Vec<KernelInstruction>,
    next_register: &mut usize,
    accumulator: Option<usize>,
    input: usize,
    coefficient: f64,
) -> Option<Option<usize>> {
    if !coefficient.is_finite() {
        return None;
    }
    if coefficient.abs() <= f64::EPSILON {
        return Some(accumulator);
    }
    let input_register = *next_register;
    *next_register += 1;
    instructions.push(KernelInstruction::LoadScalarInput {
        input,
        register: input_register,
    });
    Some(Some(append_signed_term(
        instructions,
        next_register,
        accumulator,
        input_register,
        coefficient,
    )?))
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

fn parse_numeric_matrix(expression: &str) -> Option<Vec<Vec<f64>>> {
    let trimmed = expression
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');
    let rows = trimmed
        .split(';')
        .map(str::trim)
        .filter(|row| !row.is_empty())
        .map(|row| {
            row.trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::parse::<f64>)
                .collect::<Result<Vec<_>, _>>()
                .ok()
        })
        .collect::<Option<Vec<_>>>()?;
    (!rows.is_empty()).then_some(rows)
}

#[derive(Clone, Debug, PartialEq)]
enum ArithmeticToken {
    Identifier(String),
    Number(f64),
    Operator(char),
    LeftParen,
    RightParen,
}

#[derive(Default)]
struct ArithmeticIrBuilder {
    instructions: Vec<KernelInstruction>,
    next_register: usize,
    series_inputs: BTreeMap<String, usize>,
    scalar_inputs: BTreeMap<String, usize>,
}

impl ArithmeticIrBuilder {
    fn load_identifier(&mut self, name: &str) -> usize {
        let register = self.take_register();
        if name.contains('.') {
            let input = intern_index(&mut self.series_inputs, name);
            self.instructions
                .push(KernelInstruction::LoadInput { input, register });
        } else {
            let input = intern_index(&mut self.scalar_inputs, name);
            self.instructions
                .push(KernelInstruction::LoadScalarInput { input, register });
        }
        register
    }

    fn load_number(&mut self, value: f64) -> usize {
        let register = self.take_register();
        self.instructions
            .push(KernelInstruction::LoadConstant { value, register });
        register
    }

    fn binary(&mut self, op: KernelBinaryOp, left: usize, right: usize) -> usize {
        let target = self.take_register();
        self.instructions.push(KernelInstruction::Binary {
            op,
            left,
            right,
            target,
        });
        target
    }

    fn take_register(&mut self) -> usize {
        let register = self.next_register;
        self.next_register += 1;
        register
    }
}

fn intern_index(map: &mut BTreeMap<String, usize>, name: &str) -> usize {
    if let Some(index) = map.get(name) {
        *index
    } else {
        let index = map.len();
        map.insert(name.to_owned(), index);
        index
    }
}

fn tokenize_arithmetic_expression(expression: &str) -> Option<Vec<ArithmeticToken>> {
    let chars = expression.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            index += 1;
            continue;
        }
        match ch {
            '+' | '-' | '*' | '/' => {
                tokens.push(ArithmeticToken::Operator(ch));
                index += 1;
            }
            '(' => {
                tokens.push(ArithmeticToken::LeftParen);
                index += 1;
            }
            ')' => {
                tokens.push(ArithmeticToken::RightParen);
                index += 1;
            }
            _ if ch.is_ascii_digit() || ch == '.' => {
                let start = index;
                index += 1;
                while index < chars.len()
                    && (chars[index].is_ascii_digit()
                        || chars[index] == '.'
                        || chars[index] == 'e'
                        || chars[index] == 'E'
                        || ((chars[index] == '+' || chars[index] == '-')
                            && matches!(chars.get(index.wrapping_sub(1)), Some('e' | 'E'))))
                {
                    index += 1;
                }
                let text = chars[start..index].iter().collect::<String>();
                tokens.push(ArithmeticToken::Number(text.parse::<f64>().ok()?));
            }
            _ if ch.is_ascii_alphabetic() || ch == '_' => {
                let start = index;
                index += 1;
                while index < chars.len()
                    && (chars[index].is_ascii_alphanumeric()
                        || chars[index] == '_'
                        || chars[index] == '.')
                {
                    index += 1;
                }
                tokens.push(ArithmeticToken::Identifier(
                    chars[start..index].iter().collect(),
                ));
            }
            _ => return None,
        }
    }
    Some(tokens)
}

fn parse_add_sub(
    tokens: &[ArithmeticToken],
    position: &mut usize,
    builder: &mut ArithmeticIrBuilder,
) -> Option<usize> {
    let mut left = parse_mul_div(tokens, position, builder)?;
    while let Some(ArithmeticToken::Operator(op @ ('+' | '-'))) = tokens.get(*position) {
        let op = *op;
        *position += 1;
        let right = parse_mul_div(tokens, position, builder)?;
        left = builder.binary(
            if op == '+' {
                KernelBinaryOp::Add
            } else {
                KernelBinaryOp::Sub
            },
            left,
            right,
        );
    }
    Some(left)
}

fn parse_mul_div(
    tokens: &[ArithmeticToken],
    position: &mut usize,
    builder: &mut ArithmeticIrBuilder,
) -> Option<usize> {
    let mut left = parse_primary(tokens, position, builder)?;
    while let Some(ArithmeticToken::Operator(op @ ('*' | '/'))) = tokens.get(*position) {
        let op = *op;
        *position += 1;
        let right = parse_primary(tokens, position, builder)?;
        left = builder.binary(
            if op == '*' {
                KernelBinaryOp::Mul
            } else {
                KernelBinaryOp::Div
            },
            left,
            right,
        );
    }
    Some(left)
}

fn parse_primary(
    tokens: &[ArithmeticToken],
    position: &mut usize,
    builder: &mut ArithmeticIrBuilder,
) -> Option<usize> {
    match tokens.get(*position)?.clone() {
        ArithmeticToken::Identifier(name) => {
            *position += 1;
            Some(builder.load_identifier(&name))
        }
        ArithmeticToken::Number(value) => {
            *position += 1;
            Some(builder.load_number(value))
        }
        ArithmeticToken::LeftParen => {
            *position += 1;
            let register = parse_add_sub(tokens, position, builder)?;
            if !matches!(tokens.get(*position), Some(ArithmeticToken::RightParen)) {
                return None;
            }
            *position += 1;
            Some(register)
        }
        ArithmeticToken::Operator(_) | ArithmeticToken::RightParen => None,
    }
}

fn percentile_fraction(name: &str) -> Option<f64> {
    let percentile = name.strip_prefix('p')?.parse::<u32>().ok()?;
    (1..=100)
        .contains(&percentile)
        .then_some(percentile as f64 / 100.0)
}

fn duration_above_threshold(name: &str) -> Option<f64> {
    let inside = name
        .trim()
        .strip_prefix("duration_above(")?
        .strip_suffix(')')?;
    let mut parts = inside.split_whitespace();
    let value = parts.next()?.parse::<f64>().ok()?;
    let unit = parts.next().map(|unit| {
        unit.trim_matches(|character| matches!(character, '(' | ')' | ','))
            .to_owned()
    });
    if parts.next().is_some() {
        return None;
    }
    match unit.as_deref() {
        None | Some("") | Some("W") => Some(value),
        Some("kW") => Some(value * 1000.0),
        Some("MW") => Some(value * 1_000_000.0),
        _ => None,
    }
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
        assert!(json["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| {
                candidate["kind"] == "statistics_fusion"
                    && candidate["executor"]["status"] == "interpreter_supported"
            }));

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
    fn lowers_official_integration_candidate_to_ir() {
        let report = check_file(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/official/01_csv_plot/main.eng"),
            &CheckOptions::default(),
        )
        .expect("official CSV example should check");
        let ir = timeseries_integrate_ir_for_binding(&report, "E_coil", 300.0)
            .expect("E_coil integration should lower to IR");

        assert_eq!(ir.name, "E_coil");
        assert_eq!(ir.kind, "timeseries_integrate");
        assert_eq!(ir.input_count, 1);
        assert_eq!(ir.output_count, 1);

        let output = execute_interpreter_kernel(
            &ir,
            &KernelExecutionInput {
                series_inputs: vec![vec![4873.88, 4999.28, 4999.28, 5417.28]],
                scalar_inputs: Vec::new(),
            },
        )
        .unwrap();

        assert_eq!(output.outputs, vec![KernelOutputValue::Scalar(4543242.0)]);
    }

    #[test]
    fn lowers_official_statistics_candidate_to_ir() {
        let report = check_file(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/official/01_csv_plot/main.eng"),
            &CheckOptions::default(),
        )
        .expect("official CSV example should check");
        let ir = timeseries_statistics_ir_for_source(&report, "Q_coil", 300.0)
            .expect("Q_coil statistics should lower to IR");

        assert_eq!(ir.name, "summary:Q_coil");
        assert_eq!(ir.kind, "statistics_fusion");
        assert_eq!(ir.input_count, 1);
        assert_eq!(ir.output_count, 8);

        let output = execute_interpreter_kernel(
            &ir,
            &KernelExecutionInput {
                series_inputs: vec![vec![4873.88, 4999.28, 4999.28, 5417.28]],
                scalar_inputs: Vec::new(),
            },
        )
        .unwrap();
        let values = output
            .outputs
            .iter()
            .map(|output| match output {
                KernelOutputValue::Scalar(value) => *value,
                KernelOutputValue::Series(_) => panic!("statistics outputs should be scalar"),
            })
            .collect::<Vec<_>>();

        assert_eq!(values.len(), 8);
        assert!((values[0] - 5072.43).abs() < 1e-9);
        assert!((values[1] - 5048.046666666667).abs() < 1e-9);
        assert!((values[2] - 5417.28).abs() < 1e-9);
        assert!((values[3] - 4999.28).abs() < 1e-9);
        assert!((values[5] - 5417.28).abs() < 1e-9);
        assert!((values[6] - 5417.28).abs() < 1e-9);
        assert!((values[7] - 299.4832535885168).abs() < 1e-9);
    }

    #[test]
    fn lowers_official_timeseries_arithmetic_candidate_to_ir() {
        let report = check_file(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/official/01_csv_plot/main.eng"),
            &CheckOptions::default(),
        )
        .expect("official CSV example should check");
        let ir = timeseries_arithmetic_ir_for_binding(&report, "Q_coil")
            .expect("Q_coil arithmetic should lower to IR");

        assert_eq!(ir.name, "Q_coil");
        assert_eq!(ir.kind, "timeseries_arithmetic");
        assert_eq!(ir.input_count, 3);
        assert_eq!(ir.scalar_input_count, 1);
        assert_eq!(ir.output_count, 1);

        let output = execute_interpreter_kernel(
            &ir,
            &KernelExecutionInput {
                series_inputs: vec![
                    vec![0.22, 0.23, 0.23, 0.24],
                    vec![12.4, 12.6, 12.9, 13.3],
                    vec![7.1, 7.4, 7.7, 7.9],
                ],
                scalar_inputs: vec![4180.0],
            },
        )
        .unwrap();

        let KernelOutputValue::Series(values) = &output.outputs[0] else {
            panic!("Q_coil output should be a TimeSeries");
        };
        assert_eq!(values.len(), 4);
        for (actual, expected) in values.iter().zip([4873.88, 4999.28, 4999.28, 5417.28]) {
            assert!((actual - expected).abs() < 1e-9);
        }
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
        let jacobian_candidate = candidates
            .iter()
            .find(|candidate| candidate["kind"] == "component_residual_jacobian")
            .expect("component residual Jacobian candidate JSON should exist");
        assert_eq!(
            jacobian_candidate["executor"]["status"],
            "interpreter_supported"
        );
        assert_eq!(jacobian_candidate["estimate"]["input_count"], 4);
        assert_eq!(jacobian_candidate["estimate"]["output_count"], 16);
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
    fn detects_state_space_rhs_kernel_candidate() {
        let report = check_file(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/official/20_multi_state_thermal/main.eng"),
            &CheckOptions::default(),
        )
        .expect("official multi-state thermal example should check");
        let plan = plan_for_report(&report);
        let candidate = plan
            .candidates
            .iter()
            .find(|candidate| candidate.kind == "state_space_rhs")
            .expect("state-space RHS candidate should exist");

        assert_eq!(candidate.name, "MultiStateThermal");
        assert_eq!(
            candidate.lowering_status,
            "lowerable_to_numeric_kernel_plan"
        );
        assert_eq!(candidate.estimate.input_count, 4);
        assert_eq!(candidate.estimate.output_count, 2);
        assert!(candidate
            .operations
            .contains(&"apply_linear_operator:A".to_owned()));

        let json = plan_json(&plan);
        let candidates = json["candidates"].as_array().unwrap();
        let state_space_candidate = candidates
            .iter()
            .find(|candidate| candidate["kind"] == "state_space_rhs")
            .expect("state-space RHS candidate JSON should exist");
        assert_eq!(
            state_space_candidate["executor"]["status"],
            "interpreter_supported"
        );
    }

    #[test]
    fn lowers_state_space_rhs_kernel_ir() {
        let report = check_file(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/internal/18_state_space_metadata/main.eng"),
            &CheckOptions::default(),
        )
        .expect("internal state-space metadata example should check");
        let ir = state_space_rhs_ir_for_system(&report, "ThermalStateSpaceMetadata")
            .expect("state-space RHS should lower to IR");

        assert_eq!(ir.name, "ThermalStateSpaceMetadata:state_space_rhs");
        assert_eq!(ir.kind, "state_space_rhs");
        assert_eq!(ir.input_count, 0);
        assert_eq!(ir.scalar_input_count, 3);
        assert_eq!(ir.output_count, 1);

        let output = execute_interpreter_kernel(
            &ir,
            &KernelExecutionInput {
                series_inputs: Vec::new(),
                scalar_inputs: vec![22.0, 8.0, 500.0],
            },
        )
        .unwrap();

        assert_eq!(output.outputs.len(), 1);
        let KernelOutputValue::Scalar(value) = &output.outputs[0] else {
            panic!("state-space RHS output should be scalar");
        };
        assert!((*value - 0.4972).abs() < 1e-12);
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
