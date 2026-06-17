# Internal 26 - Discrete State-Space

This internal fixture exercises the discrete-time state-space runtime path:

- two state trajectories, `T_air` and `T_wall`
- scalar input vector materialization at each fixed step
- `next(x) eq A * x + B * u`
- canonical operator matrices and named nonzero entries in review/report/IDE
  artifacts

This fixture is not a general discrete-control workflow. It exists to keep the
implemented `state_space_discrete_fixed_step` path covered by the development
smoke gate.
