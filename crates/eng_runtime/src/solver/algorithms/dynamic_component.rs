use std::collections::HashMap;

use crate::solver::algorithms::fixed_point::{solve_fixed_point, FixedPointOptions};
use crate::solver::{
    ResidualGraph, SolverDiagnostics, SolverFailure, SolverInput, SolverOutput, SolverResult,
    SolverScalar, StateLayout, StateTrajectory,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DynamicComponentOptions {
    pub algebraic: FixedPointOptions,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DynamicComponentStepDiagnostic {
    pub step_index: usize,
    pub time_s: f64,
    pub algebraic_iteration_count: usize,
    pub residual_norm: f64,
    pub convergence_status: String,
    pub failure: Option<SolverFailure>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DynamicComponentResult {
    pub solver_result: SolverResult,
    pub algebraic_layout: StateLayout,
    pub algebraic_trajectories: Vec<StateTrajectory>,
    pub step_diagnostics: Vec<DynamicComponentStepDiagnostic>,
}

pub struct AlgebraicStepInput<'a> {
    pub time_s: f64,
    pub step_index: usize,
    pub state: &'a [f64],
    pub algebraic: &'a [f64],
    pub inputs: &'a [SolverScalar],
    pub parameters: &'a [SolverScalar],
}

pub struct DynamicStepInput<'a> {
    pub time_s: f64,
    pub step_index: usize,
    pub state: &'a [f64],
    pub algebraic: &'a [f64],
    pub inputs: &'a [SolverScalar],
    pub parameters: &'a [SolverScalar],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResidualGraphRhsEvaluator {
    equations: Vec<ResidualRhsEquation>,
    state_count: usize,
    algebraic_count: usize,
    input_count: usize,
    parameter_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct ResidualRhsEquation {
    residual_name: String,
    derivative_variable: String,
    derivative_coefficient: f64,
    rhs_value: f64,
    terms: Vec<ResidualRhsTerm>,
}

#[derive(Clone, Debug, PartialEq)]
struct ResidualRhsTerm {
    role: ResidualRhsRole,
    local_index: usize,
    coefficient: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResidualRhsRole {
    State,
    Algebraic,
    Input,
    Parameter,
    Time,
}

#[derive(Clone, Debug, Default)]
struct ResidualRhsRoleCounts {
    state: usize,
    algebraic: usize,
    input: usize,
    parameter: usize,
}

impl ResidualGraphRhsEvaluator {
    pub fn new(graph: &ResidualGraph) -> Result<Self, SolverFailure> {
        let mut counts = ResidualRhsRoleCounts::default();
        let mut variable_slots = HashMap::new();
        let mut derivative_variables = HashMap::new();

        for variable in &graph.variables {
            if is_derivative_role(&variable.role) {
                if derivative_variables
                    .insert(variable.index, variable.name.clone())
                    .is_some()
                {
                    return Err(SolverFailure::new(
                        "E-DYNAMIC-COMPONENT-RHS-VARIABLE",
                        format!("duplicate derivative variable index {}", variable.index),
                    ));
                }
                continue;
            }

            let role = residual_rhs_role(&variable.role).ok_or_else(|| {
                SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-RHS-VARIABLE",
                    format!(
                        "residual RHS graph variable `{}` has unsupported role `{}`",
                        variable.name, variable.role
                    ),
                )
            })?;
            let local_index = counts.allocate(role);
            if variable_slots
                .insert(variable.index, (role, local_index))
                .is_some()
            {
                return Err(SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-RHS-VARIABLE",
                    format!("duplicate residual RHS variable index {}", variable.index),
                ));
            }
        }

        if counts.state == 0 {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-RHS-SHAPE",
                "residual RHS graph requires at least one state variable",
            ));
        }
        if derivative_variables.len() != counts.state {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-RHS-SHAPE",
                "residual RHS graph requires one derivative variable per state",
            ));
        }

        let mut equations_by_derivative = Vec::new();
        for residual in &graph.residuals {
            if !residual.rhs_value.is_finite() {
                return Err(SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-RHS-VALUE",
                    format!("residual `{}` has a non-finite RHS value", residual.name),
                ));
            }
            let derivative_terms = residual
                .terms
                .iter()
                .filter(|term| derivative_variables.contains_key(&term.variable_index))
                .collect::<Vec<_>>();
            if derivative_terms.len() != 1 {
                return Err(SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-RHS-SHAPE",
                    format!(
                        "residual `{}` must contain exactly one derivative variable",
                        residual.name
                    ),
                ));
            }
            let derivative_term = derivative_terms[0];
            if !derivative_term.coefficient.is_finite() || derivative_term.coefficient == 0.0 {
                return Err(SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-RHS-VALUE",
                    format!(
                        "residual `{}` derivative coefficient must be a finite non-zero value",
                        residual.name
                    ),
                ));
            }

            let mut terms = Vec::new();
            for term in &residual.terms {
                if !term.coefficient.is_finite() {
                    return Err(SolverFailure::new(
                        "E-DYNAMIC-COMPONENT-RHS-VALUE",
                        format!(
                            "residual `{}` term for `{}` has a non-finite coefficient",
                            residual.name, term.variable
                        ),
                    ));
                }
                if term.variable_index == derivative_term.variable_index {
                    continue;
                }
                let (role, local_index) =
                    variable_slots.get(&term.variable_index).ok_or_else(|| {
                        SolverFailure::new(
                            "E-DYNAMIC-COMPONENT-RHS-VARIABLE",
                            format!(
                                "residual `{}` references unsupported RHS variable `{}`",
                                residual.name, term.variable
                            ),
                        )
                    })?;
                terms.push(ResidualRhsTerm {
                    role: *role,
                    local_index: *local_index,
                    coefficient: term.coefficient,
                });
            }

            equations_by_derivative.push((
                derivative_term.variable_index,
                ResidualRhsEquation {
                    residual_name: residual.name.clone(),
                    derivative_variable: derivative_variables
                        .get(&derivative_term.variable_index)
                        .cloned()
                        .unwrap_or_else(|| term_variable_name(derivative_term.variable_index)),
                    derivative_coefficient: derivative_term.coefficient,
                    rhs_value: residual.rhs_value,
                    terms,
                },
            ));
        }

        equations_by_derivative.sort_by_key(|(variable_index, _)| *variable_index);
        if equations_by_derivative
            .windows(2)
            .any(|pair| pair[0].0 == pair[1].0)
        {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-RHS-SHAPE",
                "residual RHS graph contains multiple residuals for the same derivative variable",
            ));
        }
        if equations_by_derivative.len() != counts.state {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-RHS-SHAPE",
                "residual RHS graph requires one derivative residual per state",
            ));
        }

        Ok(Self {
            equations: equations_by_derivative
                .into_iter()
                .map(|(_, equation)| equation)
                .collect(),
            state_count: counts.state,
            algebraic_count: counts.algebraic,
            input_count: counts.input,
            parameter_count: counts.parameter,
        })
    }

    pub fn evaluate(&self, sample: &DynamicStepInput<'_>) -> Result<Vec<f64>, SolverFailure> {
        if sample.state.len() != self.state_count
            || sample.algebraic.len() != self.algebraic_count
            || sample.inputs.len() != self.input_count
            || sample.parameters.len() != self.parameter_count
        {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-RHS-LAYOUT",
                "residual RHS sample layout does not match residual graph variables",
            ));
        }
        if !sample.time_s.is_finite() {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-RHS-VALUE",
                "residual RHS sample time must be finite",
            ));
        }
        ensure_finite_values(
            "E-DYNAMIC-COMPONENT-RHS-VALUE",
            "residual RHS state",
            sample.state,
        )?;
        ensure_finite_values(
            "E-DYNAMIC-COMPONENT-RHS-VALUE",
            "residual RHS algebraic",
            sample.algebraic,
        )?;
        ensure_finite_scalars(
            "E-DYNAMIC-COMPONENT-RHS-VALUE",
            "residual RHS input",
            sample.inputs,
        )?;
        ensure_finite_scalars(
            "E-DYNAMIC-COMPONENT-RHS-VALUE",
            "residual RHS parameter",
            sample.parameters,
        )?;

        let mut derivatives = Vec::with_capacity(self.equations.len());
        for equation in &self.equations {
            let mut remaining = equation.rhs_value;
            for term in &equation.terms {
                let value = match term.role {
                    ResidualRhsRole::State => sample.state[term.local_index],
                    ResidualRhsRole::Algebraic => sample.algebraic[term.local_index],
                    ResidualRhsRole::Input => sample.inputs[term.local_index].value,
                    ResidualRhsRole::Parameter => sample.parameters[term.local_index].value,
                    ResidualRhsRole::Time => sample.time_s,
                };
                remaining -= term.coefficient * value;
                if !remaining.is_finite() {
                    return Err(SolverFailure::new(
                        "E-DYNAMIC-COMPONENT-RHS-VALUE",
                        format!(
                            "residual `{}` produced a non-finite RHS accumulator",
                            equation.residual_name
                        ),
                    ));
                }
            }

            let derivative = remaining / equation.derivative_coefficient;
            if !derivative.is_finite() {
                return Err(SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-RHS-VALUE",
                    format!(
                        "residual `{}` produced a non-finite derivative for `{}`",
                        equation.residual_name, equation.derivative_variable
                    ),
                ));
            }
            derivatives.push(derivative);
        }

        Ok(derivatives)
    }
}

impl ResidualRhsRoleCounts {
    fn allocate(&mut self, role: ResidualRhsRole) -> usize {
        match role {
            ResidualRhsRole::State => {
                let index = self.state;
                self.state += 1;
                index
            }
            ResidualRhsRole::Algebraic => {
                let index = self.algebraic;
                self.algebraic += 1;
                index
            }
            ResidualRhsRole::Input => {
                let index = self.input;
                self.input += 1;
                index
            }
            ResidualRhsRole::Parameter => {
                let index = self.parameter;
                self.parameter += 1;
                index
            }
            ResidualRhsRole::Time => 0,
        }
    }
}

fn is_derivative_role(role: &str) -> bool {
    matches!(role, "derivative" | "state_derivative" | "xdot")
}

fn residual_rhs_role(role: &str) -> Option<ResidualRhsRole> {
    match role {
        "state" => Some(ResidualRhsRole::State),
        "algebraic" => Some(ResidualRhsRole::Algebraic),
        "input" => Some(ResidualRhsRole::Input),
        "parameter" => Some(ResidualRhsRole::Parameter),
        "time" => Some(ResidualRhsRole::Time),
        _ => None,
    }
}

fn term_variable_name(variable_index: usize) -> String {
    format!("variable#{variable_index}")
}

pub fn solve_explicit_euler_with_algebraic<A, R>(
    input: &SolverInput,
    algebraic_layout: StateLayout,
    initial_algebraic: Vec<f64>,
    options: DynamicComponentOptions,
    mut algebraic_update: A,
    mut rhs: R,
) -> Result<DynamicComponentResult, SolverFailure>
where
    A: FnMut(AlgebraicStepInput<'_>) -> Result<Vec<f64>, SolverFailure>,
    R: FnMut(DynamicStepInput<'_>) -> Result<Vec<f64>, SolverFailure>,
{
    if input.state_layout.is_empty() {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-STATE-SHAPE",
            "dynamic component solver requires at least one state variable",
        ));
    }
    if input.initial_state.len() != input.state_layout.len() {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-STATE-LAYOUT",
            "initial state vector length does not match the state layout",
        ));
    }
    input.validate_layouts()?;
    if initial_algebraic.len() != algebraic_layout.len() {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-ALGEBRAIC-LAYOUT",
            "initial algebraic vector length does not match the algebraic layout",
        ));
    }
    ensure_finite_values(
        "E-DYNAMIC-COMPONENT-ALGEBRAIC-VALUE",
        "initial algebraic",
        &initial_algebraic,
    )?;

    let mut state = input.initial_state.clone();
    let mut algebraic = initial_algebraic;
    let mut state_values_by_state =
        vec![Vec::with_capacity(input.time_grid.step_count + 1); state.len()];
    let mut algebraic_values_by_variable =
        vec![Vec::with_capacity(input.time_grid.step_count + 1); algebraic.len()];
    for (index, value) in state.iter().copied().enumerate() {
        state_values_by_state[index].push(value);
    }

    let mut step_diagnostics = Vec::with_capacity(input.time_grid.step_count + 1);
    let mut total_iterations = 0usize;

    for step_index in 0..=input.time_grid.step_count {
        let time_s = input.time_grid.step_time_s(step_index);
        let (algebraic_iteration_count, residual_norm, convergence_status, failure) =
            if algebraic.is_empty() {
                (0, 0.0, "algebraic_not_required".to_owned(), None)
            } else {
                let fixed_point = solve_fixed_point(&algebraic, &options.algebraic, |guess| {
                    algebraic_update(AlgebraicStepInput {
                        time_s,
                        step_index,
                        state: &state,
                        algebraic: guess,
                        inputs: &input.inputs,
                        parameters: &input.parameters,
                    })
                })?;
                total_iterations += fixed_point.iteration_count;
                algebraic = fixed_point.values;
                (
                    fixed_point.iteration_count,
                    fixed_point.residual_history.last().copied().unwrap_or(0.0),
                    fixed_point.convergence_status,
                    fixed_point.failure.clone(),
                )
            };

        for (index, value) in algebraic.iter().copied().enumerate() {
            algebraic_values_by_variable[index].push(value);
        }
        step_diagnostics.push(DynamicComponentStepDiagnostic {
            step_index,
            time_s,
            algebraic_iteration_count,
            residual_norm,
            convergence_status,
            failure: failure.clone(),
        });
        if let Some(failure) = failure {
            return Ok(dynamic_component_result(
                input,
                algebraic_layout,
                state_values_by_state,
                algebraic_values_by_variable,
                step_diagnostics,
                SolverDiagnostics {
                    status: "failed".to_owned(),
                    convergence_status: "algebraic_solve_failed".to_owned(),
                    failure: Some(failure),
                    iteration_count: total_iterations,
                    tolerance: options.algebraic.tolerance,
                    max_iterations: options.algebraic.max_iterations,
                },
            ));
        }

        if step_index == input.time_grid.step_count {
            break;
        }

        let dt = input.time_grid.step_dt_s(step_index + 1);
        let derivative = rhs(DynamicStepInput {
            time_s,
            step_index,
            state: &state,
            algebraic: &algebraic,
            inputs: &input.inputs,
            parameters: &input.parameters,
        })?;
        if derivative.len() != state.len() {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-RHS-LAYOUT",
                "dynamic component RHS vector length does not match the state layout",
            ));
        }
        ensure_finite_values(
            "E-DYNAMIC-COMPONENT-RHS-VALUE",
            "dynamic component RHS",
            &derivative,
        )?;
        for (state_value, derivative_value) in state.iter_mut().zip(derivative) {
            *state_value += derivative_value * dt;
        }
        ensure_finite_values(
            "E-DYNAMIC-COMPONENT-STATE-VALUE",
            "dynamic component state",
            &state,
        )?;
        for (index, value) in state.iter().copied().enumerate() {
            state_values_by_state[index].push(value);
        }
    }

    Ok(dynamic_component_result(
        input,
        algebraic_layout,
        state_values_by_state,
        algebraic_values_by_variable,
        step_diagnostics,
        SolverDiagnostics {
            status: "computed".to_owned(),
            convergence_status: "dynamic_component_fixed_step_completed".to_owned(),
            failure: None,
            iteration_count: total_iterations,
            tolerance: options.algebraic.tolerance,
            max_iterations: options.algebraic.max_iterations,
        },
    ))
}

fn dynamic_component_result(
    input: &SolverInput,
    algebraic_layout: StateLayout,
    state_values_by_state: Vec<Vec<f64>>,
    algebraic_values_by_variable: Vec<Vec<f64>>,
    step_diagnostics: Vec<DynamicComponentStepDiagnostic>,
    diagnostics: SolverDiagnostics,
) -> DynamicComponentResult {
    let state_trajectories = trajectories_from_layout(&input.state_layout, state_values_by_state);
    let algebraic_trajectories =
        trajectories_from_layout(&algebraic_layout, algebraic_values_by_variable);
    DynamicComponentResult {
        solver_result: SolverResult {
            plan: input.plan.clone(),
            time_grid: input.time_grid.clone(),
            state_layout: input.state_layout.clone(),
            output_layout: input.output_layout.clone(),
            output: SolverOutput {
                state_trajectories,
                algebraic_trajectories: algebraic_trajectories.clone(),
            },
            diagnostics,
        },
        algebraic_layout,
        algebraic_trajectories,
        step_diagnostics,
    }
}

fn trajectories_from_layout(
    layout: &StateLayout,
    values_by_variable: Vec<Vec<f64>>,
) -> Vec<StateTrajectory> {
    layout
        .entries
        .iter()
        .zip(values_by_variable)
        .map(|(entry, values)| StateTrajectory {
            name: entry.name.clone(),
            quantity_kind: entry.quantity_kind.clone(),
            canonical_unit: entry.canonical_unit.clone(),
            values,
        })
        .collect()
}

fn ensure_finite_values(code: &str, label: &str, values: &[f64]) -> Result<(), SolverFailure> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(SolverFailure::new(
            code,
            format!("{label} vector contains a non-finite value"),
        ))
    }
}

fn ensure_finite_scalars(
    code: &str,
    label: &str,
    values: &[SolverScalar],
) -> Result<(), SolverFailure> {
    if values.iter().all(|value| value.value.is_finite()) {
        Ok(())
    } else {
        Err(SolverFailure::new(
            code,
            format!("{label} vector contains a non-finite value"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{
        InputLayout, LayoutEntry, OutputLayout, ParameterLayout, ResidualEquation,
        ResidualExpression, ResidualScale, ResidualSource, ResidualTerm, ResidualUnit,
        ResidualVariableRef, SimulationPlan, SolverOptions, SolverPlan, TimeGrid,
    };

    #[test]
    fn solves_dynamic_component_two_state_without_algebraic_node() {
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(2.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![
                LayoutEntry::new(0, "x", "Dimensionless", "1", "1"),
                LayoutEntry::new(1, "y", "Dimensionless", "1", "1"),
            ]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![0.0, 10.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };
        let mut algebraic_update_called = false;

        let result = solve_explicit_euler_with_algebraic(
            &input,
            StateLayout::default(),
            Vec::new(),
            DynamicComponentOptions::default(),
            |_| {
                algebraic_update_called = true;
                Ok(Vec::new())
            },
            |sample| {
                assert!(sample.algebraic.is_empty());
                Ok(vec![1.0, -2.0])
            },
        )
        .unwrap();

        assert!(!algebraic_update_called);
        assert_eq!(result.solver_result.diagnostics.status, "computed");
        assert_eq!(result.solver_result.diagnostics.iteration_count, 0);
        assert_eq!(
            result.solver_result.output.state_trajectories[0].values,
            vec![0.0, 1.0, 2.0]
        );
        assert_eq!(
            result.solver_result.output.state_trajectories[1].values,
            vec![10.0, 8.0, 6.0]
        );
        assert!(result.algebraic_trajectories.is_empty());
        assert!(result
            .solver_result
            .output
            .algebraic_trajectories
            .is_empty());
        assert_eq!(result.step_diagnostics.len(), 3);
        assert!(result.step_diagnostics.iter().all(|diagnostic| {
            diagnostic.algebraic_iteration_count == 0
                && diagnostic.residual_norm == 0.0
                && diagnostic.convergence_status == "algebraic_not_required"
                && diagnostic.failure.is_none()
        }));
    }

    #[test]
    fn dynamic_component_uses_partial_final_step() {
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(2.5, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![0.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };

        let result = solve_explicit_euler_with_algebraic(
            &input,
            StateLayout::default(),
            Vec::new(),
            DynamicComponentOptions::default(),
            |_| Ok(Vec::new()),
            |_sample| Ok(vec![2.0]),
        )
        .unwrap();

        assert_eq!(
            result.solver_result.output.state_trajectories[0].values,
            vec![0.0, 2.0, 4.0, 5.0]
        );
        assert_eq!(result.step_diagnostics.len(), 4);
        assert_eq!(result.step_diagnostics[3].time_s, 2.5);
    }

    #[test]
    fn dynamic_component_rejects_nonfinite_values() {
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(1.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![0.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };
        let algebraic_layout =
            StateLayout::new(vec![LayoutEntry::new(0, "z", "Dimensionless", "1", "1")]);

        let failure = solve_explicit_euler_with_algebraic(
            &input,
            algebraic_layout.clone(),
            vec![f64::NAN],
            DynamicComponentOptions::default(),
            |_| Ok(vec![0.0]),
            |_sample| Ok(vec![0.0]),
        )
        .unwrap_err();
        assert_eq!(failure.code, "E-DYNAMIC-COMPONENT-ALGEBRAIC-VALUE");

        let failure = solve_explicit_euler_with_algebraic(
            &input,
            StateLayout::default(),
            Vec::new(),
            DynamicComponentOptions::default(),
            |_| Ok(Vec::new()),
            |_sample| Ok(vec![f64::INFINITY]),
        )
        .unwrap_err();
        assert_eq!(failure.code, "E-DYNAMIC-COMPONENT-RHS-VALUE");

        let failure = solve_explicit_euler_with_algebraic(
            &input,
            algebraic_layout,
            vec![0.0],
            DynamicComponentOptions::default(),
            |_| Ok(vec![f64::INFINITY]),
            |_sample| Ok(vec![0.0]),
        )
        .unwrap_err();
        assert_eq!(failure.code, "E-FIXED-POINT-VALUE");
    }

    #[test]
    fn residual_graph_rhs_evaluator_lowers_derivative_residuals() {
        let graph = residual_rhs_graph();
        let evaluator = ResidualGraphRhsEvaluator::new(&graph).unwrap();
        let state = vec![1.0, 2.0];
        let inputs = vec![SolverScalar::new("u", "Dimensionless", "1", 3.0)];
        let parameters = Vec::new();

        let derivative = evaluator
            .evaluate(&DynamicStepInput {
                time_s: 0.0,
                step_index: 0,
                state: &state,
                algebraic: &[],
                inputs: &inputs,
                parameters: &parameters,
            })
            .unwrap();

        assert_eq!(derivative, vec![4.0, -2.0]);
    }

    #[test]
    fn residual_graph_rhs_evaluator_rejects_invalid_dynamic_graphs() {
        let mut graph = residual_rhs_graph();
        graph.residuals.pop();

        let failure = ResidualGraphRhsEvaluator::new(&graph).unwrap_err();

        assert_eq!(failure.code, "E-DYNAMIC-COMPONENT-RHS-SHAPE");
    }

    #[test]
    fn dynamic_component_solver_uses_residual_graph_rhs() {
        let graph = residual_rhs_graph();
        let evaluator = ResidualGraphRhsEvaluator::new(&graph).unwrap();
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(1.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![
                LayoutEntry::new(0, "x", "Dimensionless", "1", "1"),
                LayoutEntry::new(1, "y", "Dimensionless", "1", "1"),
            ]),
            input_layout: InputLayout {
                entries: vec![LayoutEntry::new(0, "u", "Dimensionless", "1", "1")],
            },
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0, 2.0],
            inputs: vec![SolverScalar::new("u", "Dimensionless", "1", 3.0)],
            parameters: Vec::new(),
        };

        let result = solve_explicit_euler_with_algebraic(
            &input,
            StateLayout::default(),
            Vec::new(),
            DynamicComponentOptions::default(),
            |_| Ok(Vec::new()),
            |sample| evaluator.evaluate(&sample),
        )
        .unwrap();

        assert_eq!(
            result.solver_result.output.state_trajectories[0].values,
            vec![1.0, 5.0]
        );
        assert_eq!(
            result.solver_result.output.state_trajectories[1].values,
            vec![2.0, 0.0]
        );
        assert_eq!(
            result
                .step_diagnostics
                .iter()
                .map(|diagnostic| diagnostic.convergence_status.as_str())
                .collect::<Vec<_>>(),
            vec!["algebraic_not_required", "algebraic_not_required"]
        );
    }

    #[test]
    fn solves_dynamic_component_with_algebraic_node() {
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(2.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![0.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };
        let algebraic_layout =
            StateLayout::new(vec![LayoutEntry::new(0, "z", "Dimensionless", "1", "1")]);

        let result = solve_explicit_euler_with_algebraic(
            &input,
            algebraic_layout,
            vec![0.0],
            DynamicComponentOptions::default(),
            |sample| Ok(vec![0.5 * sample.state[0] + 1.0]),
            |sample| Ok(vec![sample.algebraic[0]]),
        )
        .unwrap();

        assert_eq!(result.solver_result.diagnostics.status, "computed");
        assert_eq!(
            result.solver_result.diagnostics.convergence_status,
            "dynamic_component_fixed_step_completed"
        );
        assert_eq!(
            result.solver_result.output.state_trajectories[0].values,
            vec![0.0, 1.0, 2.5]
        );
        assert_eq!(
            result.algebraic_trajectories[0].values,
            vec![1.0, 1.5, 2.25]
        );
        assert_eq!(
            result.solver_result.output.algebraic_trajectories[0].values,
            vec![1.0, 1.5, 2.25]
        );
        assert_eq!(result.step_diagnostics.len(), 3);
        assert!(result
            .step_diagnostics
            .iter()
            .all(|diagnostic| diagnostic.failure.is_none()
                && diagnostic.convergence_status == "fixed_point_converged"));
    }

    #[test]
    fn reports_dynamic_component_algebraic_nonconvergence() {
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(2.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![0.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };
        let algebraic_layout =
            StateLayout::new(vec![LayoutEntry::new(0, "z", "Dimensionless", "1", "1")]);
        let options = DynamicComponentOptions {
            algebraic: FixedPointOptions {
                tolerance: 1e-12,
                max_iterations: 3,
                relaxation: 1.0,
            },
        };

        let result = solve_explicit_euler_with_algebraic(
            &input,
            algebraic_layout,
            vec![0.0],
            options,
            |sample| Ok(vec![sample.algebraic[0] + 1.0]),
            |_sample| Ok(vec![0.0]),
        )
        .unwrap();

        assert_eq!(result.solver_result.diagnostics.status, "failed");
        assert_eq!(
            result.solver_result.diagnostics.convergence_status,
            "algebraic_solve_failed"
        );
        assert_eq!(
            result
                .solver_result
                .diagnostics
                .failure
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("E-FIXED-POINT-NONCONVERGENCE")
        );
        assert_eq!(result.step_diagnostics.len(), 1);
        assert_eq!(
            result.step_diagnostics[0].convergence_status,
            "fixed_point_not_converged"
        );
        assert_eq!(
            result.solver_result.output.state_trajectories[0].values,
            vec![0.0]
        );
        assert_eq!(result.algebraic_trajectories[0].values, vec![3.0]);
        assert_eq!(
            result.solver_result.output.algebraic_trajectories[0].values,
            vec![3.0]
        );
    }

    fn residual_rhs_graph() -> ResidualGraph {
        ResidualGraph {
            name: "component.rhs".to_owned(),
            variables: vec![
                variable(0, "x", "state"),
                variable(1, "y", "state"),
                variable(2, "u", "input"),
                variable(3, "der_x", "state_derivative"),
                variable(4, "der_y", "state_derivative"),
            ],
            residuals: vec![
                residual(
                    "x_rhs",
                    &[(3, "der_x", 1.0), (0, "x", -1.0), (2, "u", -1.0)],
                    0.0,
                ),
                residual("y_rhs", &[(4, "der_y", 1.0), (1, "y", 1.0)], 0.0),
            ],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    fn variable(index: usize, name: &str, role: &str) -> ResidualVariableRef {
        ResidualVariableRef {
            index,
            name: name.to_owned(),
            role: role.to_owned(),
            unit: "1".to_owned(),
        }
    }

    fn residual(name: &str, terms: &[(usize, &str, f64)], rhs_value: f64) -> ResidualEquation {
        ResidualEquation {
            name: name.to_owned(),
            expression: ResidualExpression {
                text: name.to_owned(),
            },
            rhs_value,
            unit: ResidualUnit {
                unit: "1".to_owned(),
                quantity_kind: "Dimensionless".to_owned(),
            },
            scale: ResidualScale::default(),
            source: ResidualSource::default(),
            variable_indices: terms.iter().map(|(index, _, _)| *index).collect(),
            terms: terms
                .iter()
                .map(|(index, variable, coefficient)| ResidualTerm {
                    variable_index: *index,
                    variable: (*variable).to_owned(),
                    coefficient: *coefficient,
                })
                .collect(),
        }
    }
}
