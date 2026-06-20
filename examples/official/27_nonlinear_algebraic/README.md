# Official 27: Nonlinear Algebraic Residual

This example solves a source-level component residual graph with the Newton
solver. The component equations are still small, but they now form a coupled
multi-variable unitful residual system:

```text
source.q * source.q / 1 kW + target.q eq 6 kW
target.q * target.q / 1 kW + source.q eq 6 kW
```

The runtime evaluates those residuals directly, applies the HeatRate residual
scale policy, uses finite-difference Jacobian estimation by default, and records
convergence history plus largest-residual metadata in the generated artifacts.
The generated connection equations are solved in the same Newton vector, so the
artifact has multiple unknowns and residual rows rather than a scalar-only smoke.

Scope limits:

- coupled multi-variable nonlinear component equations over unitful HeatRate signals;
- one numeric initial guess value is applied to all unknowns in their display units;
- source-provided Jacobian support is limited to `jacobian = source_linear_terms` for linear residual graphs;
- this is not a broad physical nonlinear component simulator.