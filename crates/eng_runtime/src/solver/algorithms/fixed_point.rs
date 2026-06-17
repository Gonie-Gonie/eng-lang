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
            });
        }
    }

    Ok(FixedPointResult {
        values,
        residual_history,
        iteration_count: options.max_iterations,
        convergence_status: "fixed_point_not_converged".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_contracting_map() {
        let result = solve_fixed_point(&[0.0], &FixedPointOptions::default(), |values| {
            Ok(vec![0.5 * values[0] + 1.0])
        })
        .unwrap();

        assert_eq!(result.convergence_status, "fixed_point_converged");
        assert!((result.values[0] - 2.0).abs() < 1e-6);
    }
}
