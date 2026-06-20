# Official Example 30: Predictor Component Solver

This example exercises the scoped source behavior integration path for
`predictor(signal)`. The deterministic identity Predictor wrapper is evaluated
during `solver = dynamic_component_explicit_euler`, and the unitful behavior
signal affects the temperature state derivative through:

```text
der(node.T) + (predicted_T - 300 K) / 1 s eq 0 K/s
```

Scope:

- unitful AbsoluteTemperature component state
- explicit-Euler source behavior RHS only
- deterministic typed predictor wrapper seed
- not a model-loading or broad black-box solver workflow