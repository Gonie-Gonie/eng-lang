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
| `eng.process` | External process boundaries with args, env, cwd, timeout/retry, expected outputs, tool version, stdout/stderr hashes, and status. |
| `eng.test` | Local assertions, golden checks, and test-result artifacts. |
| `eng.artifact` | Generated artifact kinds, `write standard_text` table artifacts, hashes, manifests, and validation records. |
| `eng.table` | Promoted table diagnostics, deterministic row selection, and schema-aware filter/select/derive/sort/require_one/join transform records with row-level diagnostics. |
| `eng.sampling` | Deterministic grid/random/LHS sample generation, promoted sample-table metadata, parameter ranges, duplicate case IDs, seeds, and row-hash previews. |
| `eng.case` | CaseTable summaries, per-case manifests, collection status, scheduler hooks, cache hit/miss metadata, diagnostics, and process-output enrichment. |
| `eng.timeseries` | Coverage metadata, statistics, and integration above core TimeSeries semantics. |
| `eng.review` | Review IR, risk/fallback, and semantic diff vocabulary. |
| `eng.workflow` | Static/runtime RunPlan, run lock, dependency graph, rerun decisions, and workflow node status artifacts. |

## Planned And Internal Boundaries

These names are taxonomy or target contracts. They are not supported importable
APIs unless a status document says so.

| Area | Status | Public meaning |
|---|---|---|
| `eng.stats` | Planned | Semantic statistics helpers for TimeSeries and tables. |
| `eng.plot` | Planned | PlotSpec-oriented helper vocabulary. |
| `eng.report` | Planned | Report/review helper vocabulary. |
| `eng.config` | Supported narrow scope | Typed JSON/TOML file promotion with schema validation, optional field policy, source hashes, and config summaries. |
| `eng.net` | Supported narrow scope | Pinned offline HTTP GET and download boundary records with redacted query secrets and artifact summaries. |
| `eng.cache` | Supported narrow scope | Explicit cache keys, pinned network response cache materialization/replay, cache records, and hit/miss lookup artifacts; broader reuse/invalidation remains planned. |
| `eng.quality` | Supported narrow scope | Common quality result projection for validations, schema constraints, TimeSeries quality summaries, lightweight expectation suites, row/field failure details, report-facing quality tables, and IDE quality inspection. |
| `eng.template` | Supported narrow scope | Native text template rendering for generated inputs and adapter boundaries. |
| `eng.db` | Supported SQLite write scope | Native SQLite append/upsert/replace writes for typed tables with schema metadata, DB manifests, hash before/after records, transaction status, and safe-profile rejection. |
| `eng.table` | Planned broader APIs | Derived-value execution, fill operations, and richer schema-aware transforms. |
| `eng.sampling` | Planned broader APIs | Additional design-of-experiments strategies and richer sample manifests. |
| `eng.case` | Planned broader runner | Native apply/run/collect syntax and parallel scheduler implementation. |
| `eng.model` | Supported model-spec and predict-table scope | ModelSpec, FeatureSpec, TargetSpec, model-card, native prediction table, prediction-manifest, confidence, metric, residual, and hash review vocabulary. |
| `eng.uncertainty` | Internal | Constructor, propagation, and uncertainty review vocabulary. |
| `eng.building` | Planned | Building/Zone/Construction object vocabulary before simulation adapters. |
| `eng.system` | Internal/planned | Solver-facing adapters, not the public identity of stdlib. |
| `eng.ml` | Internal | Data-driven modeling review vocabulary, target contracts, model specs, and model-card artifacts. |

## Module File Policy

`stdlib/eng/*.eng` files currently define module boundaries. Each file must say
whether its surface is supported compiler/runtime behavior, planned pure `.eng`
helper vocabulary, or internal vocabulary. A module file is not by itself a
claim that the behavior is importable or production-ready.

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
