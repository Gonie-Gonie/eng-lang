# Official Example 29: Delay Component Solver

This example exercises the scoped native source behavior execution path for
`delay(signal, duration)`. The behavior node is evaluated during
`solver = dynamic_component_explicit_euler`, and the unitful behavior signal
affects the temperature state derivative through:

```text
der(node.T) + (delayed_T - 300 K) / 1 s eq 0 K/s
```

Scope:

- unitful AbsoluteTemperature component state
- explicit-Euler source behavior RHS only
- linear interpolation with hold-initial delay history
- not a broad behavior-graph or production component simulator
