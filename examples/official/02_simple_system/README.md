# Official Simple System Example

This is the primary supported physical-system example. It covers:

```text
- system block
- parameter, state, and input declarations
- equation block with `eq`
- derivative dimension metadata through `der(T)`
- residual metadata in review/report/result artifacts
- fixed-step one-state ODE result in report_spec/result artifacts
```

Limitations:

```text
- one-state thermal system only
- fixed-step one-state ODE runner only
- solver metadata and solver plan are review artifacts, not a general solver
- no DAE, adaptive, nonlinear, or multi-state solving claim
```

Run from the repository root:

```bat
target\debug\eng.exe run examples\official\02_simple_system\main.eng
```
