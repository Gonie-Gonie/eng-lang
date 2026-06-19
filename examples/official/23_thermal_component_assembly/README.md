# Official Example 23: Thermal Component Assembly

This example is the supported source syntax smoke for a small component
assembly that reaches the current dense linear residual solve path.

It exercises:

- `domain` declarations with across/through variables and conservation
  metadata;
- component templates with `port` declarations;
- system-local component instances using `name = Component()`;
- `connect instance.port to instance.port` component graph assembly syntax;
- a component-local boundary seed written as `name = port.signal = literal`;
- a component-local equation seed written as `port.signal eq literal`;
- generated across equality and through conservation equations;
- a square four-equation/four-unknown linear residual graph solved by the
  runtime artifact path.

Current support boundary:

- this is a real numeric solve for a constrained linear boundary graph;
- constructor arguments, general non-linear component-local equations, nonlinear
  iteration, DAE coupling, adaptive component dynamics, and production
  multi-domain solving are outside this official example.

Useful commands:

```bat
target\debug\eng.exe check examples\official\23_thermal_component_assembly\main.eng --review
target\debug\eng.exe run examples\official\23_thermal_component_assembly\main.eng --save-artifacts
```
