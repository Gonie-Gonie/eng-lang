# Internal 18 - State-Space Metadata

This internal fixture exercises the typed matrix/state-space metadata surface
without claiming a general dynamic solver.

It demonstrates:

- `states`, `inputs`, and `outputs` declarations inside a system
- `StateVector`, `InputVector`, and `OutputVector` metadata
- `Derivative[StateVector]` and `LinearOperator[From -> To]` type strings
- a vector equation shape, `der(x) eq A * x + B * u`, recorded for review

Current limitation:

```text
- metadata and diagnostics only for the state-space surface
- no nonlinear, DAE, adaptive, or full matrix simulation solver claim
- numeric state trajectories still come from the supported one-state thermal
  fixed-step path
```
