# Official Example 22: Continuous State-Space

This example is the supported source syntax smoke for a continuous two-state
state-space simulation with a Time-indexed input.

It exercises:

- top-level `states` and `inputs` typed blocks;
- `StateVector[RoomState]` and `InputVector[RoomInput]` system declarations;
- `operator A:` and `operator B:` source declarations;
- `der(x) eq A * x + B * u` lowering to the continuous state-space runtime path;
- CSV-backed `T_out` TimeSeries input materialization;
- fixed-step RK4 output as `sim.T_air` and `sim.T_wall`.

This is a scoped state-space workflow. It is not a nonlinear, DAE, broad
adaptive, or component-coupled solver claim.
