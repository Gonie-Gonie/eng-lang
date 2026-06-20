# Official Example 26: Dynamic Component Room

This example is the supported source-to-solver smoke for timestep execution of
an assembled component graph.

It exercises:

- a Thermal zone node with a parameterized `C * der(port.T)` component-local equation;
- generated Thermal connection equations for zone/wall and wall/outdoor ports;
- a semi-implicit dynamic component solve from `solve component_graph`;
- state and algebraic trajectories in the component solver artifact;
- per-step algebraic diagnostics and failure-artifact plumbing.

Current support boundary:

- the dynamic source path supports simple linear residual terms over assembled
  component port signals and materialized component-parameter coefficients;
- the wall heat flow is still a fixed linear boundary seed in this example, not
  a unit-parameterized conductance model;
- nonlinear component equations, broad args/object/non-arithmetic constructor bindings,
  adaptive component timestepping, and full DAE solving remain
  outside this official example.

Useful commands:

```bat
target\debug\eng.exe check examples\official\26_dynamic_component_room\main.eng --review
target\debug\eng.exe run examples\official\26_dynamic_component_room\main.eng --save-artifacts
```
