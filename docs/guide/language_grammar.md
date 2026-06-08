# EngLang Language Grammar Guide

This guide documents the current user-facing grammar shape for the preview
language. It is intentionally practical: it describes the forms the compiler
accepts today, the policy behind parenthesis-light commands, and the places
where syntax is recorded as metadata for review/runtime artifacts.

## Execution Model

EngLang executes the top-level workflow of one source file. There is no public
`entry` selector and no `script main` execution root. Put ordinary workflow
statements at top level and declare CLI inputs with one root `args` block.

```eng partial
args {
    input: CsvFile = file("data/sensor.csv")
}

sensor = promote csv args.input as SensorData
Q = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
print "Q = {Q}"
```

Top-level executable items are evaluated in source order for compiler metadata
and runtime artifacts. Imported files may contribute functions and importable
constants, but their executable top-level bodies are not imported into the
caller workflow.

## Top-Level Forms

The supported top-level declaration families are:

```text
use "relative/file.eng"
import package.name

args { ... }
const name: Quantity = expression
name: Quantity [unit] = expression
name = expression

schema Name { ... }
fn name(param: Quantity [unit], ...) -> Quantity [unit] { ... }
system Name { ... }
domain Name<...> package "..." version "..." { ... }
component Name { ... }

print "template {expression: .2 unit}"
export summary to csv "summary.csv" { ... }
report { ... }
```

`struct Args` and `script` blocks are rejected as compatibility syntax. The
only execution argument form is root `args { ... }`.

## Expressions And Function Calls

General function calls stay parenthesized:

```eng partial
Q_wall = heat_loss(UA_wall, dT_wall)
Q_mean = mean(Q_coil, axis=Time)
E_coil = integrate(Q_coil, over=Time)
```

User-defined functions require typed parameters and an explicit return:

```eng partial
fn heat_loss(UA: ThermalConductance [W/K], dT: TemperatureDelta [K]) -> HeatRate [W] {
    Q = UA * dT
    return Q
}
```

Function locals are scoped to the function body. Imported functions are usable
from the caller, but imported executable bindings are not.

## Parenthesis-Light Commands

Parenthesis-light syntax is reserved for built-in workflow verbs. It is not a
general replacement for function-call parentheses.

Supported command-style verbs:

```text
integrate
mean
max
min
duration
plot
show
validate
```

Initial command clauses:

```text
over
by
as
above
below
between
from
to
with
```

The compiler lowers command-style expressions to canonical call strings in the
AST/review metadata:

```eng partial
E = integrate Q_coil over Time
mean_Q = mean Q_coil over Time
peak_Q = max Q_coil over Time
plot Q_coil over Time
```

Canonical lowering:

```text
integrate Q_coil over Time -> integrate(Q_coil, over=Time)
mean Q_coil over Time      -> mean(Q_coil, axis=Time)
max Q_coil over Time       -> max(Q_coil, axis=Time)
plot Q_coil over Time      -> plot(Q_coil, over=Time)
```

Complex command targets must be parenthesized. This is rejected:

```eng error
Q1 = 1 kW
Q2 = 2 kW
E = integrate Q1 + Q2 over Time
```

Use:

```eng partial
E = integrate (Q1 + Q2) over Time
```

This rule keeps command syntax readable without making expression parsing
surprising. General calls such as `heat_loss(UA, dT)` remain parenthesized.

## `where` Blocks

`where` introduces a local calculation context for the immediately preceding
owner expression or command. Names defined in the block are visible only to that
owner and to later bindings in the same `where` block.

```eng partial
E_coil = integrate Q_for_energy over Time
where {
    Q_for_energy = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
}
```

`where` locals are not exported into the top-level variable table. Reusing a
where-local outside its owner is rejected with `E-NAME-LOCAL-001`.

Forward references inside the same `where` block are rejected with
`E-WHERE-FWD-001`:

```eng error
E = integrate Q_local over Time
where {
    Q_local = Q_late
    Q_late = 1 kW
}
```

The compiler records `where_blocks` in review metadata. Runtime time-series
materialization can consume typed where-local heat-rate series when the owner
integration uses the local as its source.

## `with` Blocks

`with` introduces options for the immediately preceding owner expression or
command. It is for configuration/display/backend choices, not for local
calculation.

```eng partial
E_coil = integrate Q_for_energy over Time
where {
    Q_for_energy = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
}
with {
    method = trapezoidal
}
```

Common accepted options:

```text
method
backend
title
type
unit x
unit y
display_unit
solver
tolerance
max_iter
seed
output
```

Unknown options are rejected with `E-WITH-OPTION-001`. Display unit options are
checked against the owner quantity when the owner type is known; incompatible
units are rejected with `E-WITH-UNIT-001`.

Plot display options may be written with a following `with` block:

```eng partial
report {
    plot Q_coil over Time
    with {
        unit y = kW
        title = "Command-style coil heat rate"
    }
}
```

## Unit-Aware Printing And CSV Export

`print` is for debugging and CLI output. Quantity values print with units by
default, and requested display units are type-checked:

```eng partial
print "Q mean = {mean_Q: .2 kW}"
print "E total = {E_coil: .2 kWh}"
```

`export summary to csv` writes reproducible scalar summary artifacts:

```eng partial
export summary to csv "summary.csv" {
    mean_Q as kW with ".2"
    peak_Q as kW with ".2"
    E_coil as kWh with ".2"
}
```

TimeSeries and Table values print as summaries by default. Use report/show/plot
and export commands for durable artifacts.

## Review Metadata

`eng check --review` exposes these grammar-policy sections:

```text
syntax_summary.command_styles
syntax_summary.where_blocks
syntax_summary.with_blocks
command_styles[]
where_blocks[]
with_blocks[]
```

These sections make surface syntax reviewable while preserving canonical
function-call expressions for downstream compiler and runtime paths.

## Official Example

See:

```text
examples/official/09_command_where_with/main.eng
```

That file combines top-level execution, command-style integration/statistics,
scoped `where` locals, `with` options, unit-aware print/export, and plot/report
output.
