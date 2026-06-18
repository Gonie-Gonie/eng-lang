# Simple System Tutorial

The current supported surface includes a minimal physical `system` form. It validates
equation dimensions, writes residual metadata, records a small system IR, makes
the solver boundary explicit, records solve_order and Jacobian seed columns,
then runs a fixed-step ODE path for the official one-state thermal system in
`eng run` artifacts. It does not run a full multi-state or nonlinear ODE solver
yet.

## Example

Open:

```text
examples/official/02_simple_system/main.eng
```

The supported shape is:

```eng
system RoomThermal {
    parameter C: HeatCapacity = 500 kJ/K
    parameter UA: Conductance = 150 W/K

    state T: AbsoluteTemperature = 24 degC

    input T_out: AbsoluteTemperature = 10 degC
    input Q_internal: HeatRate = 500 W

    equation {
        C * der(T) eq UA * (T_out - T) + Q_internal
    }
}

report {
    show T
}
```

Run it:

```bat
target\debug\eng.exe run examples\official\02_simple_system\main.eng
```

Generated files include:

```text
build/result/review.json
build/result/report_spec.json
build/result/report.html
build/result/result.engres
```

## What Is Checked

The equation:

```eng partial
C * der(T) eq UA * (T_out - T) + Q_internal
```

is accepted because:

```text
C                  HeatCapacity        Energy/Temperature
der(T)             Temperature/Time
C * der(T)         Power
UA                 Conductance         Power/Temperature
T_out - T          Temperature
UA * (T_out - T)   Power
Q_internal         HeatRate            Power
```

Both sides are `Power`, so the equation status is:

```text
unit_consistent
```

## Review Artifacts

`review.json` includes:

```text
syntax_summary.systems
syntax_summary.equations
system_summary
system_ir
system_ir.solver_plan
```

`report_spec.json` includes:

```text
provenance.system_count
provenance.equation_count
provenance.residual_count
system_summary
system_ir
system_ir.solver_plan
```

`result.engres` includes:

```text
typed_payload.systems
typed_payload.solver_boundaries
typed_payload.system_ir
typed_payload.system_ir[].solver_plan
provenance.system_count
provenance.equation_count
provenance.residual_count
```

`report.html` includes a `System Equations` section.

## Residual Representation

v0.8 lowers each accepted equation to report-facing residual metadata:

```text
RoomThermal.residual_1
C * der(T) - (UA * (T_out - T) + Q_internal)
dimension = Power
```

The hardened artifact path also emits system IR:

```text
relation = C * der(T) eq UA * (T_out - T) + Q_internal
normalized_residual = C * der(T) - (UA * (T_out - T) + Q_internal)
dependencies = C(parameter), UA(parameter), T(state), T_out(input), Q_internal(input)
derivative_states = T
solver_boundary.status = unsolved
```

`review.json` keeps this compiler-only unsolved boundary. During `eng run`,
`report_spec.json` and `result.engres` mark the official one-state thermal ODE
as computed and record:

```text
solver_plan.method = explicit_euler_fixed_step
solver_result.state = T
solver_result.step_count = 12
solver_result.time_step = 300 s
solver_result.final_value = 16.773071865745123 degC
```

If a `simulate` command targets a different system shape, runtime artifacts
record `solver_result.status = skipped_unsupported_shape` with the skip reason
and do not create a simulated state TimeSeries.

## Error: Use `eq`, Not `==`

This is intentionally invalid:

```eng partial
C * der(T) == UA * (T_out - T) + Q_internal
```

Check:

```bat
target\debug\eng.exe check examples\diagnostics\error_messages\eq_boolean.eng --review
```

Expected diagnostic:

```text
E-EQ-BOOL-001
Use `eq` for physical equations. `==` returns Bool.
```

## Error: Unit Mismatch

This is intentionally invalid:

```eng partial
C * der(T) eq T_out
```

Check:

```bat
target\debug\eng.exe check examples\diagnostics\error_messages\equation_unit_mismatch.eng --review
```

Expected diagnostic:

```text
E-EQ-UNIT-001
```

## Current Limits

Deferred beyond the current hardening path:

```text
- Jacobian generation
- connection/component graph
- multi-equation system solving
- adaptive or implicit ODE solving
```
