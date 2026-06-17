# Internal 24 - Component Boundary Overdetermined

This fixture exercises the component assembly limitation path for a residual
graph with more equations than unknowns.

The generated connection equations plus three component-local boundary
equations overconstrain the two-port Thermal graph. Runtime artifacts must emit:

- `not_solved_overdetermined`
- method `linear_residual_graph_shape_check`
- convergence status `linear_residual_not_attempted_overdetermined`
- failure artifact `W-ASSEMBLY-OVERDETERMINED-SEED`

This is not a production multi-domain component graph solver.
