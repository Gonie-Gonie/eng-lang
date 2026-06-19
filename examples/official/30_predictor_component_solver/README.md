# Official Example 30: Predictor Component Solver

This example exercises the scoped source behavior integration path for
`predictor(signal)`. The current supported source-level predictor wrapper is a
typed deterministic identity seed with contract/provenance metadata; it is
evaluated during explicit-Euler RHS calculation.

Scope:

- dimensionless scalar component state
- explicit-Euler source behavior RHS only
- deterministic typed predictor wrapper seed
- not a model-loading or broad black-box solver workflow
