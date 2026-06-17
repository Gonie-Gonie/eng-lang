# Component Boundary Singular Solve

Internal fixture for dense linear residual solver failure reporting.

The graph is square, but both component-local boundary equations constrain the
same Thermal across variable with different RHS values. That leaves the heat
flow variables without enough independent constraints, so the dense linear
solve must report a singular/ill-conditioned failure instead of fabricating a
solution.

Artifacts must show:

- `linear_solve_failed`
- method `dense_linear_residual_graph`
- failure code `E-LINEAR-SINGULAR`
- residual and variable metadata preserved for inspection

This is a failure-artifact fixture, not a production component graph solver.
