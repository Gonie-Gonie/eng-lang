# Internal 18 - State-Space Metadata

This internal fixture exercises the typed matrix/state-space metadata surface
without claiming a general dynamic solver.

It demonstrates:

- `states`, `inputs`, and `outputs` declarations inside a system
- `StateVector`, `InputVector`, and `OutputVector` metadata
- `Derivative[StateVector]` and `LinearOperator[From -> To]` type strings
- inverse-time coefficient entries where source and derivative units are
  compatible
- report/review/IDE canonical operator matrix and named-entry summaries in
  per-second coefficients
- a vector equation shape, `der(x) eq A * x + B * u`, recorded for review
- a narrow fixed-step explicit-Euler runtime path with `T_out` materialized
  from a promoted TimeSeries input

Current limitation:

```text
- this fixture remains a one-state metadata/runtime smoke for the state-space
  surface
- multi-state state-space runtime coverage lives in examples/official/20_multi_state_thermal
- no nonlinear, DAE, adaptive, component-coupled, or stable general matrix
  simulation solver claim
- no broad unit-compatible operator algebra support beyond the checked
  inverse-time derivative-coupling path
```
