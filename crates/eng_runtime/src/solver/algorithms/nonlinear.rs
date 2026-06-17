use crate::solver::{algorithms::linear::solve_dense_linear_system, SolverFailure};

#[derive(Clone, Debug, PartialEq)]
pub struct NewtonOptions {
    pub tolerance: f64,
    pub max_iterations: usize,
    pub finite_difference_step: f64,
    pub damping: f64,
    pub line_search_steps: usize,
}

impl Default for NewtonOptions {
    fn default() -> Self {
        Self {
            tolerance: 1e-9,
            max_iterations: 25,
            finite_difference_step: 1e-6,
            damping: 1.0,
            line_search_steps: 8,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewtonResult {
    pub values: Vec<f64>,
    pub residual_history: Vec<f64>,
    pub iteration_count: usize,
    pub convergence_status: String,
    pub failure: Option<SolverFailure>,
}

pub fn solve_newton<F>(
    initial: &[f64],
    options: &NewtonOptions,
    mut residual: F,
) -> Result<NewtonResult, SolverFailure>
where
    F: FnMut(&[f64]) -> Result<Vec<f64>, SolverFailure>,
{
    validate_newton_options(initial, options)?;

    let mut values = initial.to_vec();
    let mut residual_values = residual(&values)?;
    validate_residual_layout(values.len(), &residual_values)?;
    let mut residual_norm = norm(&residual_values);
    let mut residual_history = vec![residual_norm];
    if residual_norm <= options.tolerance {
        return Ok(NewtonResult {
            values,
            residual_history,
            iteration_count: 0,
            convergence_status: "newton_converged".to_owned(),
            failure: None,
        });
    }

    for iteration in 1..=options.max_iterations {
        let jacobian =
            finite_difference_jacobian(&values, &residual_values, options, &mut residual)?;
        let rhs = residual_values
            .iter()
            .map(|value| -value)
            .collect::<Vec<_>>();
        let step = solve_dense_linear_system(&jacobian, &rhs, options.tolerance)?.values;
        let accepted = damped_step(&values, &step, residual_norm, options, &mut residual)?;
        values = accepted.values;
        residual_values = accepted.residuals;
        residual_norm = accepted.residual_norm;
        residual_history.push(residual_norm);

        if residual_norm <= options.tolerance {
            return Ok(NewtonResult {
                values,
                residual_history,
                iteration_count: iteration,
                convergence_status: "newton_converged".to_owned(),
                failure: None,
            });
        }
    }

    Ok(NewtonResult {
        values,
        residual_history,
        iteration_count: options.max_iterations,
        convergence_status: "newton_not_converged".to_owned(),
        failure: Some(SolverFailure::new(
            "E-NEWTON-NONCONVERGENCE",
            format!(
                "Newton solver did not converge after {} iteration(s); final residual norm was {}",
                options.max_iterations, residual_norm
            ),
        )),
    })
}

fn validate_newton_options(initial: &[f64], options: &NewtonOptions) -> Result<(), SolverFailure> {
    if initial.is_empty() {
        return Err(SolverFailure::new(
            "E-NEWTON-SHAPE",
            "Newton solver requires at least one variable",
        ));
    }
    if options.max_iterations == 0 {
        return Err(SolverFailure::new(
            "E-NEWTON-ITERATIONS",
            "Newton solver requires max_iterations greater than zero",
        ));
    }
    if !options.tolerance.is_finite() || options.tolerance <= 0.0 {
        return Err(SolverFailure::new(
            "E-NEWTON-TOLERANCE",
            "Newton solver tolerance must be a positive finite number",
        ));
    }
    if !options.finite_difference_step.is_finite() || options.finite_difference_step <= 0.0 {
        return Err(SolverFailure::new(
            "E-NEWTON-FD-STEP",
            "Newton solver finite-difference step must be a positive finite number",
        ));
    }
    if !options.damping.is_finite() || options.damping <= 0.0 || options.damping > 1.0 {
        return Err(SolverFailure::new(
            "E-NEWTON-DAMPING",
            "Newton solver damping must be in the interval (0, 1]",
        ));
    }
    Ok(())
}

fn validate_residual_layout(expected: usize, residuals: &[f64]) -> Result<(), SolverFailure> {
    if residuals.len() != expected {
        return Err(SolverFailure::new(
            "E-NEWTON-RESIDUAL-LAYOUT",
            format!(
                "Newton residual vector length {} does not match variable count {}",
                residuals.len(),
                expected
            ),
        ));
    }
    if residuals.iter().any(|value| !value.is_finite()) {
        return Err(SolverFailure::new(
            "E-NEWTON-RESIDUAL-FINITE",
            "Newton residual evaluator returned a non-finite value",
        ));
    }
    Ok(())
}

fn finite_difference_jacobian<F>(
    values: &[f64],
    baseline_residuals: &[f64],
    options: &NewtonOptions,
    residual: &mut F,
) -> Result<Vec<Vec<f64>>, SolverFailure>
where
    F: FnMut(&[f64]) -> Result<Vec<f64>, SolverFailure>,
{
    let n = values.len();
    let mut jacobian = vec![vec![0.0; n]; n];
    for column in 0..n {
        let mut perturbed = values.to_vec();
        let step = options.finite_difference_step * values[column].abs().max(1.0);
        perturbed[column] += step;
        let perturbed_residuals = residual(&perturbed)?;
        validate_residual_layout(n, &perturbed_residuals)?;
        for row in 0..n {
            jacobian[row][column] = (perturbed_residuals[row] - baseline_residuals[row]) / step;
        }
    }
    Ok(jacobian)
}

#[derive(Clone, Debug, PartialEq)]
struct AcceptedStep {
    values: Vec<f64>,
    residuals: Vec<f64>,
    residual_norm: f64,
}

fn damped_step<F>(
    values: &[f64],
    step: &[f64],
    current_residual_norm: f64,
    options: &NewtonOptions,
    residual: &mut F,
) -> Result<AcceptedStep, SolverFailure>
where
    F: FnMut(&[f64]) -> Result<Vec<f64>, SolverFailure>,
{
    let mut scale = options.damping;
    let attempts = options.line_search_steps.max(1);
    let mut best: Option<AcceptedStep> = None;
    for _ in 0..attempts {
        let candidate = values
            .iter()
            .zip(step.iter())
            .map(|(value, delta)| value + scale * delta)
            .collect::<Vec<_>>();
        let candidate_residuals = residual(&candidate)?;
        validate_residual_layout(values.len(), &candidate_residuals)?;
        let candidate_norm = norm(&candidate_residuals);
        let accepted = AcceptedStep {
            values: candidate,
            residuals: candidate_residuals,
            residual_norm: candidate_norm,
        };
        if candidate_norm <= current_residual_norm {
            return Ok(accepted);
        }
        if best
            .as_ref()
            .is_none_or(|best| candidate_norm < best.residual_norm)
        {
            best = Some(accepted);
        }
        scale *= 0.5;
    }
    best.ok_or_else(|| {
        SolverFailure::new(
            "E-NEWTON-LINE-SEARCH",
            "Newton solver could not evaluate any damped step candidate",
        )
    })
}

fn norm(values: &[f64]) -> f64 {
    values.iter().map(|value| value * value).sum::<f64>().sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_scalar_nonlinear_system() {
        let result = solve_newton(&[1.0], &NewtonOptions::default(), |values| {
            Ok(vec![values[0] * values[0] - 2.0])
        })
        .unwrap();

        assert_eq!(result.convergence_status, "newton_converged");
        assert!(result.failure.is_none());
        assert!((result.values[0] - 2.0_f64.sqrt()).abs() < 1e-7);
        assert!(result.residual_history.last().copied().unwrap() <= 1e-9);
    }

    #[test]
    fn solves_two_variable_nonlinear_system() {
        let result = solve_newton(&[0.8, 2.1], &NewtonOptions::default(), |values| {
            let x = values[0];
            let y = values[1];
            Ok(vec![x + y - 3.0, x * x + y * y - 5.0])
        })
        .unwrap();

        assert_eq!(result.convergence_status, "newton_converged");
        assert!((result.values[0] - 1.0).abs() < 1e-7);
        assert!((result.values[1] - 2.0).abs() < 1e-7);
    }

    #[test]
    fn reports_singular_jacobian_failure() {
        let failure =
            solve_newton(&[1.0], &NewtonOptions::default(), |_| Ok(vec![1.0])).unwrap_err();

        assert_eq!(failure.code, "E-LINEAR-SINGULAR");
    }

    #[test]
    fn reports_nonconvergence_with_failure_artifact() {
        let options = NewtonOptions {
            tolerance: 1e-15,
            max_iterations: 1,
            finite_difference_step: 1e-6,
            damping: 1.0,
            line_search_steps: 1,
        };
        let result = solve_newton(&[10.0], &options, |values| {
            Ok(vec![values[0] * values[0] - 2.0])
        })
        .unwrap();

        assert_eq!(result.convergence_status, "newton_not_converged");
        assert_eq!(
            result.failure.as_ref().map(|failure| failure.code.as_str()),
            Some("E-NEWTON-NONCONVERGENCE")
        );
    }

    #[test]
    fn rejects_invalid_newton_options() {
        let options = NewtonOptions {
            max_iterations: 0,
            ..Default::default()
        };
        let failure = solve_newton(&[1.0], &options, |values| Ok(values.to_vec())).unwrap_err();

        assert_eq!(failure.code, "E-NEWTON-ITERATIONS");

        let options = NewtonOptions {
            damping: 0.0,
            ..Default::default()
        };
        let failure = solve_newton(&[1.0], &options, |values| Ok(values.to_vec())).unwrap_err();

        assert_eq!(failure.code, "E-NEWTON-DAMPING");
    }
}
