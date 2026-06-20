# Official Example 33: Unit-Parameterized Wall

This example is the supported source-to-solver smoke for a linear Thermal wall
whose component equation uses a unit-parameterized coefficient:

```eng
inside.Q eq UA * (inside.T - outside.T)
```

It exercises:

- component parameter defaults with `Conductance [W/K]`;
- dimension-aware component equation checking for `HeatRate = Conductance * TemperatureDelta`;
- coefficient conversion from `W/K` into a `kW` residual graph;
- generated Thermal connection equations and a square dense linear residual solve;
- named solved variables, residual norm, and largest-residual artifacts.

Current support boundary:

- this is a real numeric solve for a constrained linear Thermal graph;
- it does not claim broad nonlinear component equations, affine-unit expression
  lowering beyond this linear coefficient shape, adaptive component dynamics,
  DAE coupling, or production multi-domain solving.

Useful commands:

```bat
target\debug\eng.exe check examples\official\33_unit_parameterized_wall\main.eng --review
target\debug\eng.exe run examples\official\33_unit_parameterized_wall\main.eng --save-artifacts
```
