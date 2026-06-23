# Official 32 Small Thermal/Fluid Loop

This example is a constrained multi-domain algebraic residual solve. It uses:

- `Thermal` across/through connection equations;
- `Fluid[Water]` pressure/flow across-through connection equations;
- system-local component instances with declared numeric/importable-const/pure-arithmetic parameter defaults plus named or declaration-order positional constructor overrides;
- component-local boundary and pressure-drop seeds materialized from typed component parameters;
- a simple linear pipe pressure-drop equation using the `dp` component parameter default;
- dense linear residual graph solve artifacts.

Current support boundary:

- this is a real numeric solve for a square Thermal/Fluid[Water] residual graph;
- the Fluid seed uses the public `Pressure [Pa]` quantity and a fixed pipe
  pressure-drop equation;
- it is not a production hydraulic simulation, nonlinear pump/pipe model, DAE,
  adaptive component solve, or broad multi-domain solver.
