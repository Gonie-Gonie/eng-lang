# Official Simple System Example

This is the primary v1.0 physical-system example. It covers:

```text
- system block
- parameter, state, and input declarations
- equation block with `eq`
- derivative dimension metadata through `der(T)`
- residual metadata in review/report/result artifacts
- fixed-step ODE preview in report_spec/result artifacts
```

Run from the repository root:

```bat
target\debug\eng.exe run examples\official\02_simple_system\main.eng --entry main
```
