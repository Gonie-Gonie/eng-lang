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
        SolverOutput { state_trajectories },
        input.time_grid.step_count,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{
        InputLayout, LayoutEntry, ParameterLayout, SimulationPlan, SolverOptions, SolverPlan,
        StateLayout, TimeGrid,
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
        assert_eq!(result.diagnostics.status, "computed");
    }
}
