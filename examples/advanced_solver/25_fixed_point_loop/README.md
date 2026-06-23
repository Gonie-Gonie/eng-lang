# Official Example 25: Fixed-Point Loop

This example is the supported source-to-solver smoke for explicit fixed-point
iteration over a small algebraic component residual graph.

It exercises:

- a unitful HeatRate component graph with generated connection equations;
- component-local linear equations with a unitful affine constant that can be rearranged into `g(x)`;
- `result = solve component_graph` with `solver = fixed_point`;
- source options for tolerance, max iterations, relaxation, and initial guess;
- runtime fixed-point convergence, named variables, residual norm, and
  largest-residual artifacts.

Current support boundary:

- this is a scoped fixed-point source path for linear residual graphs whose
  equations can each be assigned to one variable update;
- unitful affine constants are covered for simple linear component equations;
- nonlinear expressions, broad solver selection, and production nonlinear
  algebraic loops remain outside this official example.

Useful commands:

```bat
target\debug\eng.exe check examples\advanced_solver\25_fixed_point_loop\main.eng --review
target\debug\eng.exe run examples\advanced_solver\25_fixed_point_loop\main.eng --save-artifacts
```
