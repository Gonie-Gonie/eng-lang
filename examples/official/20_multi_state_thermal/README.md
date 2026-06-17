# Multi-State Thermal

This example exercises the supported multi-state state-space simulation path.

- `T_air` and `T_wall` are simulated as separate state trajectories.
- `weather_data.T_out` is bound as a `TimeSeries[Time]` input.
- `A` and `B` operators are checked for vector shape and executed by the
  fixed-step explicit Euler solver.
- Runtime artifacts emit both `sim.T_air` and `sim.T_wall` TimeSeries values.

This is an actual state-space simulation path. It is not a nonlinear, DAE,
adaptive, or production component-graph solver.
