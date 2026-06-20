# Official Example 21: Discrete State-Space

This example is the supported source syntax smoke for a discrete two-state
state-space simulation.

It exercises:

- top-level `states` and `inputs` typed blocks;
- `StateVector[RoomState]` and `InputVector[RoomInput]` system declarations;
- `operator A:` and `operator B:` source declarations;
- `next(x) eq A * x + B * u` lowering to the discrete state-space runtime path;
- scalar `output Q_total` evaluation after each state update;
- `sim.T_air`, `sim.T_wall`, and `sim.Q_total` TimeSeries materialization.

This is a scoped state-space workflow. It is not a nonlinear, DAE, broad
adaptive, or component-coupled solver claim.
