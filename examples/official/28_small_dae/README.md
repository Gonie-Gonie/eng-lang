# Official 28: Small DAE

This example connects a source-level component residual graph to the existing
implicit-Euler DAE solver. The assembly split classifies `node.T` as state
because it appears inside `der(node.T)`, and `node.T_ref` remains algebraic.

The runtime builds `DaeInput`, applies Newton algebraic initialization by
default, uses an identity mass-matrix fallback, then records state/algebraic
trajectories, step diagnostics, and residual metadata for a unitful temperature
DAE residual:

```text
der(node.T) + (node.T - node.T_ref) / 1 s eq 0 K/s
node.T_ref eq 300 K
```

Scope limits:

- fixed-step implicit Euler only;
- scalar component equations using `+`, `-`, `*`, `/`, and parentheses;
- one initial state value, derivative value, and algebraic value are broadcast
  across their respective layouts;
- this is a small unitful DAE smoke, not a broad physical DAE model or
  production multi-domain simulation claim.