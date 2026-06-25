# Integrated Language Philosophy

This page is the active short-form philosophy for EngLang. It should guide
implementation, examples, documentation, and release claims.

## Product Statement

EngLang is a semantic engineering workflow language.
It helps engineers and LLM-generated code preserve units, quantities, schemas,
axes, provenance, plots, and review artifacts across typed data analysis and
simulation-result validation.

The primary goal is not to become a solver-first language. Solvers are
producers of typed TimeSeries and reviewable residual/convergence artifacts;
they are supporting capability, not the primary identity of EngLang.

## What It Is / What It Is Not

| EngLang is | EngLang is not |
|---|---|
| Unit-safe typed data analysis | A general nonlinear solver platform |
| Schema/promote data boundary | A Modelica or Simulink replacement |
| TimeSeries semantics with explicit axes | An EnergyPlus replacement |
| Plot/report/review artifact generation | A production multi-domain simulator |
| IDE inspection for engineering meaning | A solver benchmark project |
| LLM-reviewable engineering computation | A hidden code-generation runtime |

## LLM-Reviewable Engineering Computation

EngLang assumes that engineering code may be written, modified, or reviewed by
LLM-assisted tools. The language should make the meaning of the computation
visible enough that a human reviewer can inspect it without trusting the code
blindly.

That means public workflows should surface:

- variable, quantity, and unit tables
- schema and promote boundaries
- TimeSeries axes and alignment metadata
- metric and validation artifacts
- provenance for data, files, commands, and generated outputs
- report/review artifacts that explain what happened
- IDE panels that expose the same information without raw JSON spelunking

## Workflow Integration

The language must carry two workflows without splitting into unrelated
languages:

```text
1. SQL/MATLAB/R/pandas-like typed Table and TimeSeries analysis
2. scoped system/component simulation that produces typed TimeSeries
```

The integration point is:

```text
System modeling produces typed TimeSeries.
Data analysis validates, summarizes, and explains those TimeSeries.
```

The default workflow therefore moves in this direction:

```text
schema/promote
-> typed Table/TimeSeries
-> typed simulation output TimeSeries when needed
-> metrics/validation
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

## Domain Adapter Boundary

Composite workflows often need weather APIs, standard weather files, external
simulators, laboratory tools, surrogate models, and databases. Those are
adapter domains, not the core product identity.

The core language should own the generic contracts:

```text
typed inputs and schemas
explicit network/file/process/database boundaries
case and output manifests
model-card and prediction metadata
report/review/IDE visibility
reproducibility and side-effect policy
```

Domain adapters should be built above those contracts. A weather API, EPW
writer, EnergyPlus-like runner, CFD adapter, FEM adapter, Modelica bridge, or
SQLite writer is valid evidence for the generic workflow layer only when its
inputs, outputs, assumptions, hashes, failures, and side effects remain
reviewable.

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

Use [side effect policy](../reference/language/side_effect_policy.md) for the detailed
general programming support plan.

## Completion Rule

A feature is not complete because an example parses. A feature is complete for
its stated supported scope only when these agree:

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
