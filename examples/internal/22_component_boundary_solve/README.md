# Component Boundary Solve

Internal fixture for the component-assembly linear residual solver seed.

- `RoomBoundary` adds component-local boundary equations with
  `name = port.signal = literal`.
- Thermal connection equations supply `T` equality and `Q` conservation.
- The runtime residual graph converts the boundary RHS literals into canonical
  units and solves the square linear system.
- Artifacts must show `solved_linear`, residuals, variables, and the explicit
  boundary RHS values.

This is still a small algebraic assembly seed, not a production multi-domain
component graph solver.
