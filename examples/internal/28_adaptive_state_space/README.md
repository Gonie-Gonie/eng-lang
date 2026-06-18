# Internal 28 - Adaptive State-Space

This internal fixture exercises the continuous state-space adaptive runtime
seed:

- `der(x) eq A * x + B * u`
- promoted `TimeSeries[Time]` input materialization
- `solver = adaptive_heun` with a fixed output TimeGrid
- adaptive Heun/Euler internal substep diagnostics in runtime/report artifacts

It is not a supported general state-space workflow, discrete adaptive solver,
nonlinear solver, DAE solver, or production component-coupled solver.
