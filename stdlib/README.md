# EngLang Standard Library

The standard library starts as a semantic engineering vocabulary, not a solver
library.

Its first responsibility is to provide names, constants, helper functions, and
artifact-facing vocabulary that make engineering computations easier to type,
check, review, and report.

Current `stdlib/eng/*.eng` files are declarative module boundary notes unless a
status document says otherwise. Compiler-recognized built-ins remain in the
compiler/runtime crates until importable stdlib execution is implemented.

## Positioning

| Area | Status | Public meaning |
|---|---|---|
| `prelude.eng` | Stable seed | Default imported vocabulary for current examples. |
| `units.eng` | Stable seed | Built-in unit vocabulary used by quantity/unit checks. |
| `eng.stats` | Planned | Semantic statistics helpers for TimeSeries and tables. |
| `eng.plot` | Planned | PlotSpec-oriented helper vocabulary. |
| `eng.report` | Planned | Report/review helper vocabulary. |
| `eng.path` | Supported built-in seed | Typed paths, joins, names, and review-visible `exists`. |
| `eng.io` | Supported built-in seed | Read text/json/toml, write text/json, exports, and hashes. |
| `eng.fs` | Supported narrow built-in seed | Explicit generated-output copy/move/delete mutations. |
| `eng.config` | Planned | Typed JSON/TOML configuration promotion and validation. |
| `eng.process` | Planned | Explicit external process boundary vocabulary. |
| `eng.artifact` | Planned | Generated artifact kinds, hashes, manifests, and validation records. |
| `eng.net` | Planned | HTTP/download boundary with cache and hash policy. |
| `eng.cache` | Planned | Reproducible cache keys and hit/miss artifacts. |
| `eng.table` | Planned | Table filters, joins, row diagnostics, and schema helpers. |
| `eng.timeseries` | Planned | Coverage, gap, and fill helpers above core TimeSeries semantics. |
| `eng.sampling` | Planned | Deterministic sample tables and design sweep helpers. |
| `eng.case` | Planned | Case manifests for sample-to-run workflows. |
| `eng.db` | Planned | SQLite/database side-effect helpers with transaction artifacts. |
| `eng.model` | Internal/planned | Model-card, prediction, and residual review vocabulary. |
| `eng.uncertainty` | Internal | Constructor, propagation, and uncertainty review vocabulary. |
| `eng.review` | Planned | Review IR, risk/fallback, and semantic diff vocabulary. |
| `eng.building` | Planned | Building/Zone/Construction object vocabulary before any simulation adapter. |
| `eng.system` | Internal/planned | Solver-facing adapters, not the public identity of stdlib. |
| `eng.ml` | Internal | Data-driven modeling review vocabulary and artifacts. |

## Module File Policy

`stdlib/eng/*.eng` files currently define module boundaries. Each file must say
whether its surface is compiler/runtime built-in, planned pure `.eng` helper
vocabulary, or internal vocabulary. A module file is not by itself a claim that
the behavior is importable or production-ready.

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
