# Official Example 31: External Behavior Solver

This example exercises the scoped source behavior integration path for
`adapter(signal)`. The deterministic identity external adapter wrapper is
evaluated during `solver = dynamic_component_explicit_euler`, and the unitful
behavior signal affects the temperature state derivative through:

```text
der(node.T) + (adapted_T - 300 K) / 1 s eq 0 K/s
```

Scope:

- unitful AbsoluteTemperature component state
- explicit-Euler source behavior RHS only
- deterministic typed external function wrapper path
- not a process backend or broad external co-simulation workflow
