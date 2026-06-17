use crate::solver::{
    SolverFailure, SolverInput, SolverOutput, SolverResult, SolverScalar, StateTrajectory,
};

pub struct RhsSample<'a> {
    pub time_s: f64,
    pub state: &'a [f64],
    pub inputs: &'a [SolverScalar],
    pub parameters: &'a [SolverScalar],
}

pub fn solve_explicit_euler<F>(
    input: &SolverInput,
    mut rhs: F,
) -> Result<SolverResult, SolverFailure>
where
    F: FnMut(RhsSample<'_>) -> Result<Vec<f64>, SolverFailure>,
{
    if input.state_layout.is_empty() {
        return Err(SolverFailure::new(
            "E-SIM-SYSTEM-SHAPE-UNSUPPORTED",
            "solver input has no state variables",
        ));
    }
    if input.initial_state.len() != input.state_layout.len() {
        return Err(SolverFailure::new(
            "E-SOLVER-STATE-LAYOUT-MISMATCH",
            "initial state vector length does not match the state layout",
        ));
    }

    let mut state = input.initial_state.clone();
    let mut values_by_state = vec![Vec::with_capacity(input.time_grid.step_count + 1); state.len()];
    for (index, value) in state.iter().copied().enumerate() {
        values_by_state[index].push(value);
    }

    for step in 1..=input.time_grid.step_count {
        let time_s = input.time_grid.step_time_s(step);
        let derivative = rhs(RhsSample {
            time_s,
            state: &state,
            inputs: &input.inputs,
            parameters: &input.parameters,
        })?;
        if derivative.len() != state.len() {
            return Err(SolverFailure::new(
                "E-SOLVER-RHS-LAYOUT-MISMATCH",
                "RHS vector length does not match the state layout",
            ));
        }
        for (state_value, derivative_value) in state.iter_mut().zip(derivative) {
            *state_value += derivative_value * input.time_grid.timestep_s;
        }
        for (index, value) in state.iter().copied().enumerate() {
            values_by_state[index].push(value);
        }
    }

    let state_trajectories = input
        .state_layout
        .entries
        .iter()
        .zip(values_by_state)
        .map(|(entry, values)| StateTrajectory {
            name: entry.name.clone(),
            quantity_kind: entry.quantity_kind.clone(),
            canonical_unit: entry.canonical_unit.clone(),
            values,
        })
        .collect();

    Ok(SolverResult::computed(
        input.plan.clone(),
        input.time_grid.clone(),
        input.state_layout.clone(),
        input.output_layout.clone(),
        SolverOutput {
            state_trajectories,
            algebraic_trajectories: Vec::new(),
        },
        input.time_grid.step_count,
    ))
}

pub fn solve_rk4<F>(input: &SolverInput, mut rhs: F) -> Result<SolverResult, SolverFailure>
where
    F: FnMut(RhsSample<'_>) -> Result<Vec<f64>, SolverFailure>,
{
    if input.state_layout.is_empty() {
        return Err(SolverFailure::new(
            "E-SIM-SYSTEM-SHAPE-UNSUPPORTED",
            "solver input has no state variables",
        ));
    }
    if input.initial_state.len() != input.state_layout.len() {
        return Err(SolverFailure::new(
            "E-SOLVER-STATE-LAYOUT-MISMATCH",
            "initial state vector length does not match the state layout",
        ));
    }

    let mut state = input.initial_state.clone();
    let mut values_by_state = vec![Vec::with_capacity(input.time_grid.step_count + 1); state.len()];
    for (index, value) in state.iter().copied().enumerate() {
        values_by_state[index].push(value);
    }

    for step in 1..=input.time_grid.step_count {
        let t0 = input.time_grid.step_time_s(step - 1);
        let dt = input.time_grid.timestep_s;
        let half_dt = dt / 2.0;
        let t_half = t0 + half_dt;
        let t1 = input.time_grid.step_time_s(step);

        let k1 = rhs(RhsSample {
            time_s: t0,
            state: &state,
            inputs: &input.inputs,
            parameters: &input.parameters,
        })?;
        ensure_derivative_len(&k1, state.len())?;

        let state_k2 = offset_state(&state, &k1, half_dt);
        let k2 = rhs(RhsSample {
            time_s: t_half,
            state: &state_k2,
            inputs: &input.inputs,
            parameters: &input.parameters,
        })?;
        ensure_derivative_len(&k2, state.len())?;

        let state_k3 = offset_state(&state, &k2, half_dt);
        let k3 = rhs(RhsSample {
            time_s: t_half,
            state: &state_k3,
            inputs: &input.inputs,
            parameters: &input.parameters,
        })?;
        ensure_derivative_len(&k3, state.len())?;

        let state_k4 = offset_state(&state, &k3, dt);
        let k4 = rhs(RhsSample {
            time_s: t1,
            state: &state_k4,
            inputs: &input.inputs,
            parameters: &input.parameters,
        })?;
        ensure_derivative_len(&k4, state.len())?;

        for index in 0..state.len() {
            state[index] += dt / 6.0 * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index]);
        }
        for (index, value) in state.iter().copied().enumerate() {
            values_by_state[index].push(value);
        }
    }

    let state_trajectories = input
        .state_layout
        .entries
        .iter()
        .zip(values_by_state)
        .map(|(entry, values)| StateTrajectory {
            name: entry.name.clone(),
            quantity_kind: entry.quantity_kind.clone(),
            canonical_unit: entry.canonical_unit.clone(),
            values,
        })
        .collect();

    Ok(SolverResult::computed(
        input.plan.clone(),
        input.time_grid.clone(),
        input.state_layout.clone(),
        input.output_layout.clone(),
        SolverOutput {
            state_trajectories,
            algebraic_trajectories: Vec::new(),
        },
        input.time_grid.step_count,
    ))
}

fn ensure_derivative_len(derivative: &[f64], state_len: usize) -> Result<(), SolverFailure> {
    if derivative.len() == state_len {
        Ok(())
    } else {
        Err(SolverFailure::new(
            "E-SOLVER-RHS-LAYOUT-MISMATCH",
            "RHS vector length does not match the state layout",
        ))
    }
}

fn offset_state(state: &[f64], derivative: &[f64], scale: f64) -> Vec<f64> {
    state
        .iter()
        .zip(derivative.iter())
        .map(|(value, derivative)| value + derivative * scale)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{
        InputLayout, LayoutEntry, OutputLayout, ParameterLayout, SimulationPlan, SolverOptions,
        SolverPlan, StateLayout, TimeGrid,
    };

    #[test]
    fn explicit_euler_solves_vector_state() {
        let options = SolverOptions::fixed_step("explicit_euler_fixed_step", 1.0);
        let input = SolverInput {
            plan: SolverPlan::new("TwoState", SimulationPlan::default(), options),
            time_grid: TimeGrid::fixed_step(2.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![
                LayoutEntry::new(0, "x", "Dimensionless", "1", "1"),
                LayoutEntry::new(1, "y", "Dimensionless", "1", "1"),
            ]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout {
                entries: vec![
                    LayoutEntry::new(0, "x", "Dimensionless", "1", "1"),
                    LayoutEntry::new(1, "y", "Dimensionless", "1", "1"),
                ],
            },
            initial_state: vec![1.0, 10.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };

        let result = solve_explicit_euler(&input, |sample| {
            Ok(vec![sample.state[0], -sample.state[1] / 2.0])
        })
        .unwrap();

        assert_eq!(
            result.output.state_trajectories[0].values,
            vec![1.0, 2.0, 4.0]
        );
        assert_eq!(
            result.output.state_trajectories[1].values,
            vec![10.0, 5.0, 2.5]
        );
        assert_eq!(result.output_layout.entries.len(), 2);
        assert_eq!(result.output_layout.entries[0].name, "x");
        assert!(result.output.algebraic_trajectories.is_empty());
        assert_eq!(result.diagnostics.status, "computed");
    }

    #[test]
    fn rk4_solves_vector_state() {
        let options = SolverOptions::fixed_step("rk4_fixed_step", 1.0);
        let input = SolverInput {
            plan: SolverPlan::new("TwoState", SimulationPlan::default(), options),
            time_grid: TimeGrid::fixed_step(2.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![
                LayoutEntry::new(0, "x", "Dimensionless", "1", "1"),
                LayoutEntry::new(1, "y", "Dimensionless", "1", "1"),
            ]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0, 10.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };

        let result = solve_rk4(&input, |sample| {
            Ok(vec![sample.state[0], -sample.state[1] / 2.0])
        })
        .unwrap();

        let x_final = result.output.state_trajectories[0].final_value().unwrap();
        let y_final = result.output.state_trajectories[1].final_value().unwrap();
        assert!((x_final - 7.335069444444444).abs() < 1e-9);
        assert!((y_final - 3.681708441840278).abs() < 1e-9);
        assert!(result.output.algebraic_trajectories.is_empty());
        assert_eq!(result.diagnostics.status, "computed");
    }
}
