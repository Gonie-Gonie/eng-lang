# Official 32 Small Thermal/Fluid Loop

This example is a constrained multi-domain algebraic residual solve. It uses:

- `Thermal` across/through connection equations;
- `Fluid[Water]` across/through connection equations;
- system-local component instances;
- component-local boundary seeds;
- simple linear component equations for a head/flow loop;
- dense linear residual graph solve artifacts.

Current support boundary:

- this is a real numeric solve for a square Thermal/Fluid[Water] residual graph;
- the Fluid seed uses hydraulic head (`height`) because `Pressure`/`Pa` are not
  in the public quantity registry yet;
- it is not a production hydraulic simulation, pressure-drop package, nonlinear
  pump/pipe model, DAE, adaptive component solve, or broad multi-domain solver.
