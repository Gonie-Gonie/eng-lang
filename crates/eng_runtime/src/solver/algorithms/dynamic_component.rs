use crate::solver::algorithms::fixed_point::{solve_fixed_point, FixedPointOptions};
use crate::solver::{
    SolverDiagnostics, SolverFailure, SolverInput, SolverOutput, SolverResult, SolverScalar,
    StateLayout, StateTrajectory,
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
        for (state_value, derivative_value) in state.iter_mut().zip(derivative) {
            *state_value += derivative_value * dt;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{
        InputLayout, LayoutEntry, OutputLayout, ParameterLayout, SimulationPlan, SolverOptions,
        SolverPlan, TimeGrid,
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
}
