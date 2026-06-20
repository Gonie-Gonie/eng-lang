# Official Example 24: Linear Algebraic Thermal Node

This example is the supported source-to-solver smoke for a small steady
Thermal algebraic graph. It exists separately from example 23 so the solver
path has an official example focused on ResidualGraph lowering and dense
linear solve artifacts.

It exercises:

- `domain` declarations with across/through variables and conservation
  metadata;
- system-local component instances using `name = Component()`;
- `connect instance.port to instance.port` connection equations;
- component-local literal boundary seeds on both across and through signals;
- a square four-equation/four-unknown residual graph assembled from source;
- dense linear residual solving, named solution variables, residual norm, and
  largest-residual report artifacts.

Current support boundary:

- this is a real numeric solve for a constrained linear Thermal graph;
- nonlinear algebraic loops, fixed-point iteration, broad args/object/non-arithmetic constructor bindings,
  behavior-node solving, DAE coupling, dynamic components, and production
  multi-domain solving are outside this official example.

Useful commands:

```bat
target\debug\eng.exe check examples\official\24_linear_algebraic_thermal_node\main.eng --review
target\debug\eng.exe run examples\official\24_linear_algebraic_thermal_node\main.eng --save-artifacts
```
