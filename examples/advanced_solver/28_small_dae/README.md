# Official 28: Multi-State DAE

This example connects a source-level component residual graph to the existing
implicit-Euler DAE solver. The assembly split classifies `hot.T` and `cold.T`
as states because they appear inside `der(...)`, while each reference
temperature and generated balance variable remains algebraic.

The runtime builds `DaeInput`, accepts bracketed unitful initial vectors for
state values, state derivatives, and algebraic guesses, applies Newton
algebraic initialization by default, uses an explicit dense identity
mass_matrix, then records state/algebraic trajectories, step diagnostics, and
residual metadata
for coupled unitful temperature DAE residuals:

```text
der(hot.T) + (hot.T - hot.T_ref) / 1 s eq 0 K/s
der(cold.T) + (cold.T - cold.T_ref) / 2 s eq 0 K/s
hot.T_ref eq 300 K
cold.T_ref eq 295 K
```

Scope limits:

- fixed-step implicit Euler only;
- multi-state component equations using `+`, `-`, `*`, `/`, and parentheses;
- bracketed initial vectors must match the generated state/algebraic layouts;
- optional `mass_matrix` coefficients may be scalar, diagonal-vector, or dense
  square dimensionless values matching the generated state layout;
- this is a compact unitful DAE smoke, not a broad physical DAE model or
  production multi-domain simulation claim.
