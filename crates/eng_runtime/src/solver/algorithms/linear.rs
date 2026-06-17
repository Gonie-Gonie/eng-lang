use crate::solver::{euclidean_norm, SolverFailure};

#[derive(Clone, Debug, PartialEq)]
pub struct LinearSolveResult {
    pub values: Vec<f64>,
    pub residual_norm: f64,
    pub status: String,
}

pub fn solve_dense_linear_system(
    matrix: &[Vec<f64>],
    rhs: &[f64],
    tolerance: f64,
) -> Result<LinearSolveResult, SolverFailure> {
    let n = matrix.len();
    if n == 0 || rhs.len() != n || matrix.iter().any(|row| row.len() != n) {
        return Err(SolverFailure::new(
            "E-LINEAR-SHAPE",
            "linear solver requires a non-empty square matrix and matching RHS vector",
        ));
    }
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(SolverFailure::new(
            "E-LINEAR-TOLERANCE",
            "linear solver tolerance must be a positive finite number",
        ));
    }
    if matrix
        .iter()
        .flatten()
        .chain(rhs.iter())
        .any(|value| !value.is_finite())
    {
        return Err(SolverFailure::new(
            "E-LINEAR-FINITE",
            "linear solver matrix and RHS values must be finite",
        ));
    }

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
            return Err(SolverFailure::new(
                "E-LINEAR-SINGULAR",
                "linear system is singular or ill-conditioned at the requested tolerance",
            ));
        }
        if best_row != pivot_index {
            a.swap(best_row, pivot_index);
            b.swap(best_row, pivot_index);
        }

        let pivot = a[pivot_index][pivot_index];
        for value in a[pivot_index].iter_mut().take(n).skip(pivot_index) {
            *value /= pivot;
        }
        b[pivot_index] /= pivot;
        let pivot_row = a[pivot_index].clone();
        let pivot_rhs = b[pivot_index];

        for (row_index, row) in a.iter_mut().enumerate().take(n) {
            if row_index == pivot_index {
                continue;
            }
            let factor = row[pivot_index];
            if factor.abs() <= f64::EPSILON {
                continue;
            }
            for (value, pivot_value) in row.iter_mut().zip(pivot_row.iter()).skip(pivot_index) {
                *value -= factor * *pivot_value;
            }
            b[row_index] -= factor * pivot_rhs;
        }
    }

    let residuals = matrix
        .iter()
        .zip(rhs.iter())
        .map(|(row, expected)| {
            let actual = row
                .iter()
                .zip(b.iter())
                .map(|(coefficient, value)| coefficient * value)
                .sum::<f64>();
            actual - expected
        })
        .collect::<Vec<_>>();
    let residual_norm = euclidean_norm(&residuals);

    Ok(LinearSolveResult {
        values: b,
        residual_norm,
        status: if residual_norm <= tolerance {
            "converged".to_owned()
        } else {
            "residual_above_tolerance".to_owned()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_small_dense_system() {
        let result =
            solve_dense_linear_system(&[vec![2.0, 1.0], vec![1.0, -1.0]], &[5.0, 1.0], 1e-9)
                .unwrap();

        assert!((result.values[0] - 2.0).abs() <= 1e-9);
        assert!((result.values[1] - 1.0).abs() <= 1e-9);
        assert_eq!(result.status, "converged");
    }

    #[test]
    fn reports_singular_system() {
        let failure =
            solve_dense_linear_system(&[vec![1.0, 2.0], vec![2.0, 4.0]], &[3.0, 6.0], 1e-9)
                .unwrap_err();

        assert_eq!(failure.code, "E-LINEAR-SINGULAR");
    }

    #[test]
    fn rejects_nonfinite_linear_inputs() {
        let matrix_failure =
            solve_dense_linear_system(&[vec![f64::NAN]], &[1.0], 1e-9).unwrap_err();
        assert_eq!(matrix_failure.code, "E-LINEAR-FINITE");

        let rhs_failure =
            solve_dense_linear_system(&[vec![1.0]], &[f64::INFINITY], 1e-9).unwrap_err();
        assert_eq!(rhs_failure.code, "E-LINEAR-FINITE");

        let tolerance_failure =
            solve_dense_linear_system(&[vec![1.0]], &[1.0], f64::NAN).unwrap_err();
        assert_eq!(tolerance_failure.code, "E-LINEAR-TOLERANCE");
    }
}
