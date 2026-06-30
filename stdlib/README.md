# EngLang Standard Library

The standard library starts as a semantic engineering vocabulary, not a solver
library.

Its first responsibility is to provide names, constants, helper functions, and
artifact-facing vocabulary that make engineering computations easier to type,
check, review, and report.

Current `stdlib/eng/*.eng` files are declarative module boundary notes unless a
status document says otherwise. Compiler-recognized built-ins remain in the
compiler/runtime crates until importable stdlib execution is implemented.
The machine-readable status registry is `stdlib/eng/modules.toml`; this README
must describe the same module names.

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
| `eng.log` | Structured runtime messages and run-log records. |
| `eng.process` | External process boundaries with args, cwd, expected outputs, tool version, stdout/stderr hashes, and status. |
| `eng.test` | Local assertions, golden checks, and test-result artifacts. |
| `eng.artifact` | Generated artifact kinds, hashes, manifests, and validation records. |
| `eng.table` | Promoted table diagnostics, deterministic row selection, and schema-aware filter/select/derive/sort/require_one/join transform records with row-level diagnostics. |
| `eng.sampling` | Promoted sample-table metadata, parameter ranges, duplicate case IDs, and row-hash previews. |
| `eng.case` | Promoted case manifests with case IDs, sample row hashes, row-level diagnostics, and process-output enrichment. |
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
| `eng.config` | Supported narrow scope | Typed JSON/TOML file promotion with schema validation, optional field policy, source hashes, and config summaries. |
| `eng.net` | Supported seed | Offline/fixture HTTP GET and download boundary records with redacted query secrets and artifact summaries. |
| `eng.cache` | Supported seed | Explicit cache keys, cache manifests, and hit/miss lookup artifacts; reuse/invalidation remains planned. |
| `eng.quality` | Supported seed | Common quality result projection for validations, schema constraints, TimeSeries quality summaries, lightweight expectation suites, row/field failure details, report-facing quality tables, and IDE quality inspection. |
| `eng.template` | Planned | Template rendering for generated inputs and adapter boundaries. |
| `eng.workflow` | Planned | RunPlan, run lock, dependency graph, rerun decisions, and workflow node status. |
| `eng.table` | Planned broader APIs | Derived-value execution, fill operations, and richer schema-aware transforms. |
| `eng.sampling` | Planned generators | Grid/random/LHS sample generation and seed policy. |
| `eng.case` | Planned runner | Native apply/run/collect, case directories, resume/cache status, and generated-output linkage. |
| `eng.db` | Planned native DB; supported manifest seed | DB side-effect manifest summaries; SQLite write/upsert helpers remain planned. |
| `eng.model` | Supported artifact seed; planned public syntax | Model-card, metric, residual, and hash review vocabulary. |
| `eng.uncertainty` | Internal | Constructor, propagation, and uncertainty review vocabulary. |
| `eng.building` | Planned | Building/Zone/Construction object vocabulary before simulation adapters. |
| `eng.system` | Internal/planned | Solver-facing adapters, not the public identity of stdlib. |
| `eng.ml` | Internal | Data-driven modeling review vocabulary, target contracts, and model-card artifacts. |

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
