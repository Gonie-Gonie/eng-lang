# Simple System Tutorial

v0.8-alpha introduces a minimal physical `system` surface. It validates equation dimensions and writes residual-only metadata into review/report artifacts. It does not run a full ODE solver yet.

## Example

Open:

```text
examples/06_simple_system/main.eng
```

The supported v0.8 shape is:

```eng
system RoomThermal {
    parameter C: HeatCapacity = 500 kJ/K
    parameter UA: Conductance = 150 W/K

    state T: AbsoluteTemperature = 24 degC

    input T_out: AbsoluteTemperature
    input Q_internal: HeatRate

    equation {
        C * der(T) eq UA * (T_out - T) + Q_internal
    }
}

script main(args: Args) -> Report {
    return report {
        show T
    }
}
```

Run it:

```bat
target\debug\eng.exe run examples\06_simple_system\main.eng --entry main
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
```

`report_spec.json` includes:

```text
provenance.system_count
provenance.equation_count
provenance.residual_count
system_summary
```

`result.engres` includes:

```text
typed_payload.systems
provenance.system_count
provenance.equation_count
provenance.residual_count
```

`report.html` includes a `System Equations` section.

## Residual Representation

v0.8 lowers each accepted equation to residual metadata:

```text
RoomThermal.residual_1
C * der(T) - (UA * (T_out - T) + Q_internal)
dimension = Power
```

This is a report/review representation only. Numeric solving is deferred.

## Error: Use `eq`, Not `==`

This is intentionally invalid:

```eng partial
C * der(T) == UA * (T_out - T) + Q_internal
```

Check:

```bat
target\debug\eng.exe check examples\05_error_messages\eq_boolean.eng --review
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
target\debug\eng.exe check examples\05_error_messages\equation_unit_mismatch.eng --review
```

Expected diagnostic:

```text
E-EQ-UNIT-001
```

## Current Limits

Deferred beyond v0.8:

```text
- full symbolic IR
- solver runtime
- time stepping
- Jacobian generation
- connection/component graph
- multi-equation system solving
```
