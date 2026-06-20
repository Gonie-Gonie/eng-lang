# Official 27: Nonlinear Algebraic Residual

This example solves a source-level component residual graph with the Newton
solver. The component equation is intentionally small, but it now carries a real
unitful residual:

```text
node.q * node.q / 1 kW eq 4 kW
```

The runtime evaluates that residual directly, applies the HeatRate residual
scale policy, uses finite-difference Jacobian estimation by default, and records
convergence history plus largest residual metadata in the generated artifacts.

Scope limits:

- one scalar nonlinear component equation over a unitful HeatRate signal;
- one numeric initial guess value is applied to all unknowns in their display units;
- source-provided Jacobian support is limited to `jacobian = source_linear_terms` for linear residual graphs;
- this is not a broad multi-variable physical nonlinear component simulator.
