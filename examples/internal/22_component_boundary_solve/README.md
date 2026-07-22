# Component Boundary Solve

Internal fixture that executes the native component-assembly linear residual
solve.

- `RoomBoundary` adds component-local boundary equations with
  `name = port.signal = literal`.
- Thermal connection equations supply `T` equality and `Q` conservation.
- The runtime residual graph converts the boundary RHS literals into canonical
  units and solves the square linear system.
- Artifacts must show `solved_linear`, residuals, variables, and the explicit
  boundary RHS values.

The supported scope is a small algebraic assembly, not a production
multi-domain component graph solver.
