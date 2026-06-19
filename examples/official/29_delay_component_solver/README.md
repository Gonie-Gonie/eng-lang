# Official Example 29: Delay Component Solver

This example exercises the scoped source behavior integration path. A
component-local `delay(signal, duration)` expression is evaluated by the runtime
behavior graph during `solver = dynamic_component_explicit_euler`, and the
delayed value affects the state derivative.

Scope:

- dimensionless scalar component state
- explicit-Euler source behavior RHS only
- linear interpolation with hold-initial delay history
- not a broad behavior-graph or production component simulator
