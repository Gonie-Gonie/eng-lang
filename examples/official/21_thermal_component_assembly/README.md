# Official 21 Thermal Component Assembly

Focused thermal component-assembly fixture for the residual graph and dense
linear algebraic solver seed.

This example keeps the model intentionally small:

- `Thermal` declares one across variable (`T`) and one through variable (`Q`).
- `RoomBoundary` contributes component-local boundary equations with
  `name = port.signal = literal`.
- The connection between `RoomBoundary.heat` and `AmbientBoundary.heat`
  generates thermal equality/conservation equations.
- The generated connection equations plus boundary equations form a square
  residual graph.
- Runtime artifacts record the dense linear residual solve, explicit RHS
  literals, solved variables, residual values, residual norm, and convergence
  status.

Current support boundary:

- this is a real numeric solve for a small linear residual graph;
- it is still not a production multi-domain component solver;
- nonlinear, DAE, adaptive, dynamic component, and behavior-node integration
  remain outside this example.

Useful commands:

```bat
target\debug\eng.exe check examples\official\21_thermal_component_assembly\main.eng --review
target\debug\eng.exe run examples\official\21_thermal_component_assembly\main.eng --save-artifacts
```
