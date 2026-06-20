# Official 20 Multi-State Thermal

Supported source-equation solver fixture for a two-state thermal model.

This example shows:

- two state declarations, `T_air` and `T_wall`;
- one derivative equation per state using `C * der(state) eq RHS`;
- a promoted CSV outdoor-temperature `TimeSeries` bound through `simulate`;
- fixed-step `rk4` execution through `SolverInput -> SolverResult`;
- scalar `output Q_load` evaluation from simulated state/input sample values;
- generated `sim.T_air`, `sim.T_wall`, and `sim.Q_load` TimeSeries outputs;
- report and plot artifacts for both state trajectories.

Current support boundary:

- explicit fixed-step Euler/RK4 source ODEs with arithmetic RHS expressions and scalar algebraic outputs are supported for this scope;
- adaptive, nonlinear, DAE, and component-graph source solves remain separate internal tracks unless their examples say otherwise.

Useful commands:

```bat
target\debug\eng.exe check examples\official\20_multi_state_thermal\main.eng --review
target\debug\eng.exe run examples\official\20_multi_state_thermal\main.eng --save-artifacts
```
