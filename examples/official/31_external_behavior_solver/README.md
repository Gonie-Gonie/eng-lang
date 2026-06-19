# Official Example 31: External Behavior Solver

This example exercises the scoped source behavior integration path for
`adapter(signal)`. The current supported source-level external wrapper is a
typed deterministic identity function seed with safe/repro policy metadata; it
is evaluated during explicit-Euler RHS calculation.

Scope:

- dimensionless scalar component state
- explicit-Euler source behavior RHS only
- deterministic typed external function wrapper seed
- not a process backend or broad external co-simulation workflow
