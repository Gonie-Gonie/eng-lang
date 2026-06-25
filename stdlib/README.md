# EngLang Standard Library

The standard library starts as a semantic engineering vocabulary, not a solver
library.

Its first responsibility is to provide names, constants, helper functions, and
artifact-facing vocabulary that make engineering computations easier to type,
check, review, and report.

Current `stdlib/eng/*.eng` files are declarative module boundary notes unless a
status document says otherwise. Compiler-recognized built-ins remain in the
compiler/runtime crates until importable stdlib execution is implemented.

## Supported Built-In Declarations

These names are backed by current compiler/runtime behavior. The `.eng` files
record the supported vocabulary and review contract; importable stdlib execution
is still future work.

| Area | Public meaning |
|---|---|
| `prelude.eng` | Default imported vocabulary for current examples. |
| `units.eng` | Built-in unit vocabulary used by quantity/unit checks. |
| `eng.path` | Typed paths, joins, names, and review-visible `exists`. |
| `eng.io` | Read text/json/toml, write text/json, exports, and hashes. |
| `eng.fs` | Explicit generated-output copy/move/delete mutations. |
| `eng.process` | External process boundaries with args, cwd, expected outputs, tool version, stdout/stderr hashes, and status. |
| `eng.artifact` | Generated artifact kinds, hashes, manifests, and validation records. |
| `eng.table` | Promoted table diagnostics for schema rows, columns, and parse/conversion evidence. |
| `eng.sampling` | Promoted sample-table metadata, parameter ranges, duplicate case IDs, and row-hash previews. |
| `eng.case` | Promoted case manifest seeds with case IDs, sample row hashes, and row-level case diagnostics. |
| `eng.timeseries` | Coverage metadata, statistics, and integration above core TimeSeries semantics. |
| `eng.review` | Review IR, risk/fallback, and semantic diff vocabulary. |

## Planned And Internal Boundaries

These names are taxonomy or target contracts. They are not supported importable
APIs unless a status document says so.

| Area | Status | Public meaning |
|---|---|---|
| `eng.stats` | Planned | Semantic statistics helpers for TimeSeries and tables. |
| `eng.plot` | Planned | PlotSpec-oriented helper vocabulary. |
| `eng.report` | Planned | Report/review helper vocabulary. |
| `eng.config` | Planned | Typed JSON/TOML configuration promotion and validation. |
| `eng.net` | Planned | HTTP/download boundary with cache and hash policy. |
| `eng.cache` | Planned | Reproducible cache keys and hit/miss artifacts. |
| `eng.table` | Planned APIs | Table filters, joins, derived columns, and schema-aware transforms. |
| `eng.sampling` | Planned generators | Grid/random/LHS sample generation and seed policy. |
| `eng.case` | Planned runner | Native apply/run/collect, case directories, resume/cache status, and generated-output linkage. |
| `eng.db` | Planned | SQLite/database side-effect helpers with transaction artifacts. |
| `eng.model` | Internal/planned | Model-card, prediction, and residual review vocabulary. |
| `eng.uncertainty` | Internal | Constructor, propagation, and uncertainty review vocabulary. |
| `eng.building` | Planned | Building/Zone/Construction object vocabulary before simulation adapters. |
| `eng.system` | Internal/planned | Solver-facing adapters, not the public identity of stdlib. |
| `eng.ml` | Internal | Data-driven modeling review vocabulary and artifacts. |

## Module File Policy

`stdlib/eng/*.eng` files currently define module boundaries. Each file must say
whether its surface is a supported compiler/runtime built-in seed, planned pure
`.eng` helper vocabulary, or internal vocabulary. A module file is not by
itself a claim that the behavior is importable or production-ready.

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
