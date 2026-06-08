# Integrated Language Philosophy

This page is the active short-form philosophy for EngLang. It condenses the
integrated redesign direction into rules that should guide implementation,
examples, documentation, and release claims.

## Product Statement

EngLang is a unit-safe engineering programming language for typed data
analysis, system simulation workflows, plotting, and reproducible review.

The language must carry two workflows without splitting into two unrelated
languages:

```text
1. SQL/MATLAB/R/pandas-like typed Table and TimeSeries analysis
2. system/component/port-based engineering simulation modeling
```

The integration point is:

```text
System modeling produces typed TimeSeries.
Data analysis validates, calibrates, summarizes, and explains those TimeSeries.
```

The default workflow therefore moves in this direction:

```text
schema/promote
-> typed Table/TimeSeries
-> system/component simulation input
-> typed simulation output TimeSeries
-> metrics/validation/calibration
-> PlotSpec/report/review artifacts
-> IDE visual inspection
-> standalone package
```

## Surface Syntax Rule

Keep the surface language small and consistent. New syntax should earn its
place by making engineering intent clearer than a typed function, contract, or
library object would.

The core surface is:

```text
use
args
const
schema
fn
system
component
connect
report
where
with
=
eq
```

Rules:

```text
1. Local calculations use `name = expr`.
2. Module-shared values use `const name: Type = expr`.
3. Reusable calculations use `fn`.
4. Physical equations use `eq`.
5. Port/component assembly uses `connect`.
6. Temporary calculation context uses `where`.
7. Options, methods, solver settings, display settings, and backend choices use `with`.
8. ANN, delay, solver, transport, and adapters should normally be typed
   function/component/library objects rather than new syntax families.
```

## Execution And Imports

Root files execute top-level workflows from top to bottom. Imported files are
static dependency declarations and must not execute workflow bodies.

Importable declarations:

```text
const
fn
schema
system
domain
component
type/unit/quantity declarations when they exist
```

Not imported and not executed during import:

```text
args
top-level `name = expr`
promote
plot
report
print
export
runtime-local workflow statements
```

## Parentheses, Commands, `where`, And `with`

Parenthesis-light command syntax is for built-in workflow verbs only:

```eng partial
E = integrate Q over Time
avg = mean T_zone over Time
plot Q over Time
```

General user functions keep normal call syntax:

```eng partial
Q = coil_heat(m_dot, cp_water, dT)
```

`where` introduces owner-local calculations that do not escape. `with`
introduces option/config/display/method/backend context. The recommended order
is:

```eng partial
command
where {
    local definitions
}
with {
    options
}
```

## Complexity Boundary

Complexity belongs in types, contracts, artifacts, and libraries before it
belongs in new syntax.

Examples:

```text
ANN/data-driven model     -> typed Predictor value or function-like object
delay                     -> delay(x, tau) or explicit delay component
transport                 -> component/function library
solver                    -> simulate ... with { solver = ... }
validation                -> validate/report artifact semantics
component behavior        -> `=` and `eq` first, typed adapters only when needed
```

## General Programming Boundary

EngLang should support enough general programming to build real engineering
workflows, but hidden side effects are not acceptable.

Principle:

```text
General manipulation is allowed.
Hidden side effects are not.
```

Therefore:

```text
- file/path/process/network values are typed
- file writes and destructive operations are explicit
- external commands are recorded
- environment/time dependencies are visible
- reproducibility profiles can warn or reject unsafe effects
- report/review captures relevant effects
- IDE surfaces side effects and generated artifacts
```

Use [side effect policy](../reference/side_effect_policy.md) for the detailed
general programming support plan.

## Completion Rule

A feature is not complete because an example parses. A feature is complete for
its stated preview scope only when these agree:

```text
language rule
compiler check
runtime/check behavior
diagnostic
artifact/review metadata
IDE visibility when relevant
official example or fixture
documentation
release note or maturity entry
```

Public claims must stay narrower than implementation seeds on `main`.
