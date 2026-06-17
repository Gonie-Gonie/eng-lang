use crate::solver::SolverFailure;

#[derive(Clone, Debug, PartialEq)]
pub struct FixedPointOptions {
    pub tolerance: f64,
    pub max_iterations: usize,
    pub relaxation: f64,
}

impl Default for FixedPointOptions {
    fn default() -> Self {
        Self {
            tolerance: 1e-9,
            max_iterations: 50,
            relaxation: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FixedPointResult {
    pub values: Vec<f64>,
    pub residual_history: Vec<f64>,
    pub iteration_count: usize,
    pub convergence_status: String,
    pub failure: Option<SolverFailure>,
}

pub fn solve_fixed_point<F>(
    initial: &[f64],
    options: &FixedPointOptions,
    mut update: F,
) -> Result<FixedPointResult, SolverFailure>
where
    F: FnMut(&[f64]) -> Result<Vec<f64>, SolverFailure>,
{
    if initial.is_empty() {
        return Err(SolverFailure::new(
            "E-FIXED-POINT-SHAPE",
            "fixed-point solver requires at least one variable",
        ));
    }
    if !(0.0..=1.0).contains(&options.relaxation) || options.relaxation == 0.0 {
        return Err(SolverFailure::new(
            "E-FIXED-POINT-RELAXATION",
            "fixed-point relaxation must be in the interval (0, 1]",
        ));
    }
    if options.max_iterations == 0 {
        return Err(SolverFailure::new(
            "E-FIXED-POINT-ITERATIONS",
            "fixed-point solver requires max_iterations greater than zero",
        ));
    }
    if !options.tolerance.is_finite() || options.tolerance <= 0.0 {
        return Err(SolverFailure::new(
            "E-FIXED-POINT-TOLERANCE",
            "fixed-point solver tolerance must be a positive finite number",
        ));
    }

    let mut values = initial.to_vec();
    let mut residual_history = Vec::new();
    for iteration in 1..=options.max_iterations {
        let next = update(&values)?;
        if next.len() != values.len() {
            return Err(SolverFailure::new(
                "E-FIXED-POINT-LAYOUT",
                "fixed-point update vector length changed during iteration",
            ));
        }
        let mut residual_norm = 0.0;
        for (value, next_value) in values.iter_mut().zip(next) {
            let relaxed = *value + options.relaxation * (next_value - *value);
            let residual = relaxed - *value;
            residual_norm += residual * residual;
            *value = relaxed;
        }
        residual_norm = residual_norm.sqrt();
        residual_history.push(residual_norm);
        if residual_norm <= options.tolerance {
            return Ok(FixedPointResult {
                values,
                residual_history,
                iteration_count: iteration,
                convergence_status: "fixed_point_converged".to_owned(),
                failure: None,
            });
        }
    }

    let final_residual = residual_history.last().copied().unwrap_or(f64::INFINITY);
    Ok(FixedPointResult {
        values,
        residual_history,
        iteration_count: options.max_iterations,
        convergence_status: "fixed_point_not_converged".to_owned(),
        failure: Some(SolverFailure::new(
            "E-FIXED-POINT-NONCONVERGENCE",
            format!(
                "fixed-point solver did not converge after {} iteration(s); final residual norm was {}",
                options.max_iterations, final_residual
            ),
        )),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_fixed_point_small_loop() {
        let result = solve_fixed_point(&[0.0], &FixedPointOptions::default(), |values| {
            Ok(vec![0.5 * values[0] + 1.0])
        })
        .unwrap();

        assert_eq!(result.convergence_status, "fixed_point_converged");
        assert!(result.failure.is_none());
        assert!(!result.residual_history.is_empty());
        assert!((result.values[0] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn applies_relaxation_factor_to_iteration_history() {
        let options = FixedPointOptions {
            tolerance: 1e-12,
            max_iterations: 1,
            relaxation: 0.25,
        };
        let result = solve_fixed_point(&[0.0], &options, |_| Ok(vec![4.0])).unwrap();

        assert_eq!(result.values, vec![1.0]);
        assert_eq!(result.residual_history, vec![1.0]);
        assert_eq!(result.convergence_status, "fixed_point_not_converged");
    }

    #[test]
    fn reports_fixed_point_nonconvergence_artifact() {
        let options = FixedPointOptions {
            tolerance: 1e-12,
            max_iterations: 3,
            relaxation: 1.0,
        };
        let result =
            solve_fixed_point(&[0.0], &options, |values| Ok(vec![values[0] + 1.0])).unwrap();

        assert_eq!(result.convergence_status, "fixed_point_not_converged");
        assert_eq!(result.iteration_count, 3);
        assert_eq!(result.residual_history.len(), 3);
        assert_eq!(
            result.failure.as_ref().map(|failure| failure.code.as_str()),
            Some("E-FIXED-POINT-NONCONVERGENCE")
        );
    }

    #[test]
    fn rejects_invalid_fixed_point_options() {
        let mut options = FixedPointOptions {
            tolerance: 1e-9,
            max_iterations: 0,
            relaxation: 1.0,
        };
        let failure =
            solve_fixed_point(&[0.0], &options, |values| Ok(values.to_vec())).unwrap_err();
        assert_eq!(failure.code, "E-FIXED-POINT-ITERATIONS");

        options.max_iterations = 1;
        options.tolerance = 0.0;
        let failure =
            solve_fixed_point(&[0.0], &options, |values| Ok(values.to_vec())).unwrap_err();
        assert_eq!(failure.code, "E-FIXED-POINT-TOLERANCE");

        options.tolerance = 1e-9;
        options.relaxation = 0.0;
        let failure =
            solve_fixed_point(&[0.0], &options, |values| Ok(values.to_vec())).unwrap_err();
        assert_eq!(failure.code, "E-FIXED-POINT-RELAXATION");
    }

    #[test]
    fn rejects_empty_initial_vector() {
        let failure = solve_fixed_point(&[], &FixedPointOptions::default(), |values| {
            Ok(values.to_vec())
        })
        .unwrap_err();

        assert_eq!(failure.code, "E-FIXED-POINT-SHAPE");
    }

    #[test]
    fn rejects_update_layout_changes() {
        let failure = solve_fixed_point(&[0.0], &FixedPointOptions::default(), |_| {
            Ok(vec![0.0, 1.0])
        })
        .unwrap_err();

        assert_eq!(failure.code, "E-FIXED-POINT-LAYOUT");
    }
}
