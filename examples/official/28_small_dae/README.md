# Official 28: Small DAE

This example connects a source-level component residual graph to the existing
implicit-Euler DAE solver. The assembly split classifies `node.x` as state
because it appears inside `der(node.x)`, and `node.z` remains algebraic.

The runtime builds `DaeInput`, applies Newton algebraic initialization by
default, uses an identity mass-matrix fallback, then records state/algebraic
trajectories, step diagnostics, and residual metadata.

Scope limits:

- fixed-step implicit Euler only;
- scalar component equations using `+`, `-`, `*`, `/`, and parentheses;
- one initial state value, derivative value, and algebraic value are broadcast
  across their respective layouts;
- no broad unitful DAE model or production multi-domain simulation claim.
