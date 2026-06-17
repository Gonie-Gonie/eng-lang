use super::{SolverFailure, SolverInput, SolverOutput, SolverResult, StateTrajectory};

pub fn solve_discrete_state_space<F>(
    input: &SolverInput,
    matrix_a: &[Vec<f64>],
    matrix_b: &[Vec<f64>],
    mut input_values_at: F,
) -> Result<SolverResult, SolverFailure>
where
    F: FnMut(f64) -> Result<Vec<f64>, SolverFailure>,
{
    input.validate_layouts()?;
    validate_discrete_state_space_layout(input, matrix_a, matrix_b)?;

    let mut state = input.initial_state.clone();
    let mut values_by_state = vec![Vec::with_capacity(input.time_grid.step_count + 1); state.len()];
    for (index, value) in state.iter().copied().enumerate() {
        values_by_state[index].push(value);
    }

    for step in 1..=input.time_grid.step_count {
        let sample_time_s = input.time_grid.step_time_s(step - 1);
        let input_values = input_values_at(sample_time_s)?;
        if input_values.len() != input.input_layout.entries.len() {
            return Err(SolverFailure::new(
                "E-RHS-INPUT-LAYOUT",
                "discrete state-space input vector length does not match input layout",
            ));
        }

        state = matrix_a
            .iter()
            .zip(matrix_b.iter())
            .map(|(a_row, b_row)| {
                let state_term = a_row
                    .iter()
                    .zip(state.iter())
                    .map(|(coefficient, value)| coefficient * value)
                    .sum::<f64>();
                let input_term = b_row
                    .iter()
                    .zip(input_values.iter())
                    .map(|(coefficient, value)| coefficient * value)
                    .sum::<f64>();
                state_term + input_term
            })
            .collect::<Vec<_>>();
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

fn validate_discrete_state_space_layout(
    input: &SolverInput,
    matrix_a: &[Vec<f64>],
    matrix_b: &[Vec<f64>],
) -> Result<(), SolverFailure> {
    let state_count = input.state_layout.entries.len();
    let input_count = input.input_layout.entries.len();
    if matrix_a.len() != state_count
        || matrix_a.iter().any(|row| row.len() != state_count)
        || matrix_b.len() != state_count
        || matrix_b.iter().any(|row| row.len() != input_count)
    {
        return Err(SolverFailure::new(
            "E-RHS-MATRIX-SHAPE",
            "discrete state-space A/B matrix dimensions do not match state/input layouts",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{
        InputLayout, LayoutEntry, OutputLayout, ParameterLayout, SimulationPlan, SolverOptions,
        SolverPlan, SolverScalar, TimeGrid,
    };

    #[test]
    fn solves_discrete_state_space_with_sampled_input() {
        let input = SolverInput {
            plan: SolverPlan::new(
                "discrete_state_space",
                SimulationPlan {
                    states: vec!["x".to_owned(), "y".to_owned()],
                    inputs: vec!["u".to_owned()],
                    outputs: vec!["x".to_owned(), "y".to_owned()],
                    parameters: Vec::new(),
                },
                SolverOptions::fixed_step("state_space_discrete_fixed_step", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(2.0, 1.0).unwrap(),
            state_layout: crate::solver::StateLayout::new(vec![
                LayoutEntry::new(0, "x", "Dimensionless", "1", "1"),
                LayoutEntry::new(1, "y", "Dimensionless", "1", "1"),
            ]),
            input_layout: InputLayout {
                entries: vec![LayoutEntry::new(0, "u", "Dimensionless", "1", "1")],
            },
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0, 0.0],
            inputs: vec![SolverScalar::new("u", "Dimensionless", "1", 0.0)],
            parameters: Vec::new(),
        };

        let result = solve_discrete_state_space(
            &input,
            &[vec![1.0, 1.0], vec![0.0, 1.0]],
            &[vec![1.0], vec![2.0]],
            |time_s| Ok(vec![time_s + 1.0]),
        )
        .unwrap();

        assert_eq!(result.output.state_trajectories.len(), 2);
        assert_eq!(
            result.output.state_trajectories[0].values,
            vec![1.0, 2.0, 6.0]
        );
        assert_eq!(
            result.output.state_trajectories[1].values,
            vec![0.0, 2.0, 6.0]
        );
        assert_eq!(result.diagnostics.iteration_count, 2);
    }

    #[test]
    fn rejects_discrete_state_space_matrix_shape_mismatch() {
        let input = SolverInput {
            plan: SolverPlan::new(
                "bad_discrete_state_space",
                SimulationPlan {
                    states: vec!["x".to_owned()],
                    inputs: vec!["u".to_owned()],
                    outputs: vec!["x".to_owned()],
                    parameters: Vec::new(),
                },
                SolverOptions::fixed_step("state_space_discrete_fixed_step", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(1.0, 1.0).unwrap(),
            state_layout: crate::solver::StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout {
                entries: vec![LayoutEntry::new(0, "u", "Dimensionless", "1", "1")],
            },
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0],
            inputs: vec![SolverScalar::new("u", "Dimensionless", "1", 0.0)],
            parameters: Vec::new(),
        };

        let failure =
            solve_discrete_state_space(&input, &[vec![1.0, 0.0]], &[vec![1.0]], |_| Ok(vec![1.0]))
                .unwrap_err();

        assert_eq!(failure.code, "E-RHS-MATRIX-SHAPE");
    }
}
