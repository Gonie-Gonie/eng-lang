# Official 32 Small Thermal/Fluid Loop

This example is a constrained multi-domain algebraic residual solve. It uses:

- `Thermal` across/through connection equations;
- `Fluid[Water]` pressure/flow across-through connection equations;
- system-local component instances with named constructor arguments;
- component-local boundary seeds materialized from those arguments;
- a simple linear pipe pressure-drop equation;
- dense linear residual graph solve artifacts.

Current support boundary:

- this is a real numeric solve for a square Thermal/Fluid[Water] residual graph;
- the Fluid seed uses the public `Pressure [Pa]` quantity and a fixed pipe
  pressure-drop equation;
- it is not a production hydraulic simulation, nonlinear pump/pipe model, DAE,
  adaptive component solve, or broad multi-domain solver.
