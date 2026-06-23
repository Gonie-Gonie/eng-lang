# EngLang Standard Library

The standard library starts as a semantic engineering vocabulary, not a solver
library.

Its first responsibility is to provide names, constants, helper functions, and
artifact-facing vocabulary that make engineering computations easier to type,
check, review, and report.

## Positioning

| Area | Status | Public meaning |
|---|---|---|
| `prelude.eng` | Stable seed | Default imported vocabulary for current examples. |
| `units.eng` | Stable seed | Built-in unit vocabulary used by quantity/unit checks. |
| `eng.stats` | Planned | Semantic statistics helpers for TimeSeries and tables. |
| `eng.plot` | Planned | PlotSpec-oriented helper vocabulary. |
| `eng.report` | Planned | Report/review helper vocabulary. |
| `eng.path` / `eng.io` | Planned | Typed path and explicit IO helper vocabulary. |
| `eng.building` | Planned | Building/Zone/Construction object vocabulary before any simulation adapter. |
| `eng.system` | Internal/planned | Solver-facing adapters, not the public identity of stdlib. |
| `eng.ml` | Internal | Data-driven modeling review vocabulary and artifacts. |

## Building Vocabulary Rule

The first building examples should show typed objects and review artifacts:

```text
Building
Zone
Construction
summary report
validation
```

They should not lead with `simulate building`. Simulation lowering belongs in an
advanced adapter or future track after the object/report vocabulary is clear.

## Claim Boundary

Do not describe stdlib packages as production solvers unless the feature
maturity matrix and current status documents state that scope explicitly.
