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
| `eng.fs` | Explicit generated-output copy/move/delete/mkdir mutations. |
| `eng.log` | Structured runtime messages and run-log records. |
| `eng.process` | External process boundaries with args, env, cwd, timeout/retry, expected outputs, tool version, stdout/stderr hashes, and status. |
| `eng.test` | Local assertions, golden checks, and test-result artifacts. |
| `eng.artifact` | Generated artifact kinds, `write standard_text` table artifacts, hashes, manifests, and validation records. |
| `eng.table` | Promoted table diagnostics and schema-aware filter/select/derive/sort/require_one/join transform records with row-level diagnostics. |
| `eng.sampling` | Deterministic grid/random/LHS sample generation, promoted sample-table metadata, parameter ranges, duplicate case IDs, seeds, row hashes, and row-value previews. |
| `eng.case` | Native CaseTable/CaseOutput/CaseRunResult/CaseResultCollection stages, template and explicit sequential or bounded parallel calculation execution, verified local result-cache replay, manifests, diagnostics, and process-adapter enrichment. |
| `eng.timeseries` | Axis, coverage, fill, quality, alignment/resampling, and integration metadata above core TimeSeries semantics. |
| `eng.stats` | Compiler-checked TimeSeries summaries with native runtime values and report/review artifacts. |
| `eng.review` | Review IR, risk/fallback, and semantic diff vocabulary. |
| `eng.workflow` | Static/runtime RunPlan, run lock, dependency graph, rerun decisions, and workflow node status artifacts. |

## Additional Native, Planned, And Internal Boundaries

These names are taxonomy or target contracts. They are not supported importable
APIs unless a status document says so.

| Area | Status | Public meaning |
|---|---|---|
| `eng.plot` | Native workflow support | PlotSpec, plot manifest, and SVG artifacts from report directives; broader helper vocabulary remains planned. |
| `eng.report` | Native workflow support | Report block projection to report, review, and output artifacts; broader helper vocabulary remains planned. |
| `eng.config` | Supported narrow scope | Typed JSON/TOML file promotion with schema validation, optional field policy, source hashes, and config summaries. |
| `eng.net` | Supported narrow scope | Live HTTP(S) GET/download execution plus POST/PUT/PATCH string request bodies and pinned offline/cache HTTP(S) boundary records with redacted secret query/header values, SHA-256 checks, body hashes, and artifact summaries. |
| `eng.cache` | Supported narrow scope | Explicit cache keys, pinned network response replay, calculation-hash/result-SHA verified native case-result cache replay/repair, cache records, stale diagnostics, and manifest-path invalidation; process/model replay remains planned. |
| `eng.quality` | Supported narrow scope | Common quality result projection for validations, schema constraints, TimeSeries quality summaries, lightweight expectation suites, row/field failure details, report-facing quality tables, and IDE quality inspection. |
| `eng.template` | Supported narrow scope | Native text template rendering for generated inputs and adapter boundaries. |
| `eng.db` | Supported SQLite write scope | Native SQLite append/upsert/replace writes for typed tables with schema metadata, DB manifests, hash before/after records, transaction status, and safe-profile rejection. |
| `eng.table` | Planned broader APIs | Derived-value execution, fill operations, and richer schema-aware transforms. |
| `eng.sampling` | Planned broader APIs | Additional design-of-experiments strategies and richer sample manifests. |
| `eng.case` | Supported bounded native scope | Native template rendering and typed per-case calculations with explicit sequential or bounded parallel scheduling, deterministic worker slots, result/run manifests, calculation-hash/result-SHA resume, content-addressed local cache replay and repair, and fail/continue policy; automatic external-adapter dispatch remains planned. |
| `eng.model` | Supported model-spec and predict-table scope | ModelSpec, FeatureSpec, TargetSpec, model-card, native prediction table, prediction-manifest, confidence, metric, residual, and hash review vocabulary. |
| `eng.uncertainty` | Native workflow support | Narrow uncertainty constructors, linear propagation metadata, sensor_std TimeSeries review metadata, probability/statistic validation, and report confidence-band artifacts; broad probabilistic propagation remains planned. |
| `eng.building` | Planned | Building/Zone/Construction object vocabulary before simulation adapters. |
| `eng.system` | Internal target | Solver-facing adapters, not the public identity of stdlib. |
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
