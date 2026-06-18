# Internal 20 Multi-State Thermal

This fixture exercises the internal multi-state state-space simulation path.

- `T_air` and `T_wall` are simulated as separate state trajectories.
- `weather_data.T_out` is bound as a `TimeSeries[Time]` input.
- `A` and `B` operators are checked for vector shape and coefficient units;
  inverse-time entries such as `1/s` and `1/min` are canonicalized to
  per-second coefficients where the target derivative unit is the source
  state/input unit per second, with canonical matrices and named nonzero
  entries surfaced in report/review/IDE inspection.
- Runtime artifacts emit both `sim.T_air` and `sim.T_wall` TimeSeries values.

This is an actual state-space runtime seed. It is not a public-supported
general state-space workflow, nonlinear solver, DAE solver, adaptive solver, or
production component-graph solver.
