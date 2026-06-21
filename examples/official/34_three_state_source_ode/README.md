# Official Example 34: Three-State Source ODE

This example is the supported non-thermal source-equation ODE smoke for a system
with three named dimensionless states, a promoted CSV TimeSeries input, adaptive
Heun integration, generated `sim.<state>` TimeSeries, and a scalar output
trajectory.

It exercises:

- arbitrary declared `simulate <System>` input option names such as `drive`;
- one `der(state)` equation per state for three source states;
- Time-indexed `DimensionlessNumber` CSV input materialization;
- adaptive Heun substep diagnostics on a fixed output TimeGrid;
- scalar `output` materialization through `sim.total`.

Current support boundary:

- this is a real numeric solve for the supported source-equation ODE shape;
- it does not claim arbitrary algebraic loops, broad nonlinear/DAE simulation,
  component-coupled adaptive solving, events, or a production general equation
  solver.

Useful commands:

```bat
target\debug\eng.exe check examples\official\34_three_state_source_ode\main.eng --review
target\debug\eng.exe run examples\official\34_three_state_source_ode\main.eng --save-artifacts
```
