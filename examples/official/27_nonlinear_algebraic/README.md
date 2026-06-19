# Official 27: Nonlinear Algebraic Residual

This example solves a source-level component residual graph with the Newton
solver. The component equation is intentionally small:

```text
node.x * node.x eq 2
```

The runtime evaluates that residual directly, applies the residual scale policy,
uses finite-difference Jacobian estimation by default, and records convergence
history plus largest residual metadata in the generated artifacts.

Scope limits:

- dimensionless scalar component equations only;
- one initial guess value is applied to all unknowns;
- source-provided Jacobian support is limited to `jacobian = source_linear_terms`;
- this is not a broad physical nonlinear component simulator.
