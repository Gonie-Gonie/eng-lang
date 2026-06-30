# Composite Workflow Base Modules

Status: mixed. Existing path, IO, process, output-manifest, run-log, and test
features are supported in the current public package scope. Promoted CSV
tables now emit `typed_payload.table_diagnostics[]` with schema, row, column,
missing-cell, parse/conversion, time-axis coverage summaries, and
`typed_payload.timeseries_coverage[]` records with expected counts, missing
intervals, max gaps, and leap-year policy, plus
deterministic row-selection records in `typed_payload.table_selections[]`. Promoted
table filter/select/sort/require_one/join seeds now emit static `review_document.table_transforms[]`
and runtime `typed_payload.table_transforms[]` records with predicates, join
keys, matched pair counts, and row counts. Promoted
DesignSample-style CSV tables now emit `typed_payload.sample_tables[]` with
case ID, parameter range, duplicate-case, and row-hash preview metadata, plus
`typed_payload.case_manifests[]` case row manifests with sample row hashes and process-output enrichment. Hybrid
examples now emit process-generated weather, case, model-card, prediction, and
database side-effect artifacts. Native network and cache record seeds have
landed for offline/fixture boundaries and cache manifests; live network
execution, cache replay/invalidation, case runner, DB writer, and public model
syntax remain planned or internal until concrete language/runtime/artifact
slices land.

## Purpose

Composite engineering workflows often look domain-specific from the outside:
weather API to standard weather file, simulation input patching, external
solver runs, surrogate training, database writes, and report generation.

The core language should not become a weather, EPW, KMA, EnergyPlus, CFD, FEM,
or database-specific product. It should provide the generic workflow modules
that make those adapters typed, explicit, reproducible, and reviewable.

## Module Map

The canonical machine-readable registry is `stdlib/eng/modules.toml`. The table
below is a reader-facing summary and must stay consistent with that registry.

| Module | Current status | Purpose |
|---|---|---|
| `eng.path` | Supported through built-ins | typed paths, joins, names, existence checks, and generated-output root policy diagnostics |
| `eng.io` | Supported through built-ins | read/write text, JSON, TOML, source hashes, structured read parse summaries |
| `eng.fs` | Supported narrow scope | copy, move, delete under generated-output boundaries |
| `eng.config` | Supported narrow scope | typed JSON/TOML file promotion, config validation, source hashes, and config summaries |
| `eng.log` | Supported through built-ins | structured runtime messages and run logs |
| `eng.process` | Supported narrow scope | explicit command boundary, tool-version metadata, expected-output hashes, and process artifacts |
| `eng.test` | Supported narrow scope | local assertions, golden checks, test artifacts |
| `eng.table` | Supported diagnostics, row-selection, and filter/select/sort/require_one/join artifact seeds; planned broader APIs | promoted table row/column diagnostics, deterministic row selection, filter/select/sort/require_one row counts, selected columns, sort keys, join key pair counts; derived planned |
| `eng.timeseries` | Supported narrow scope plus coverage artifact seed | TimeSeries statistics, explicit `check coverage`, table time-axis coverage metadata, timeseries_coverage records, integration |
| `eng.sampling` | Supported promoted-table artifact seed; planned generators | sample table metadata, parameter ranges, row-hash previews; grid/random/LHS planned |
| `eng.case` | Supported case-manifest artifact seed; planned native runner | case IDs, sample row hashes, duplicate/missing diagnostics, case dirs, process/output links, result files, metrics, failure reasons |
| `eng.net` | Supported seed | offline/fixture HTTP GET and download boundary records with redacted query secrets and artifact summaries |
| `eng.cache` | Supported seed; planned reuse/invalidation | explicit cache keys, cache manifests, hit/miss lookup artifacts, pinned hash metadata |
| `eng.quality` | Planned | data expectations, quality summaries, and reportable validation results |
| `eng.template` | Planned | generated input rendering and template provenance |
| `eng.artifact` | Supported seed | output manifests, artifact_registry records, hashes, report/review links |
| `eng.db` | Supported DB-manifest artifact seed; planned native sqlite | DB side-effect manifest summaries; SQLite/open database writes with transaction artifacts planned |
| `eng.model` | Supported model-card artifact seed; planned public syntax | model cards, target quantity/unit, metrics, residual plots, training/model hashes |
| `eng.workflow` | Planned | RunPlan, run lock, dependency graph, rerun decisions, and node status |
| `eng.report` | Planned | report-facing helper vocabulary layered over report/review artifacts |

These names describe module boundaries. The current implementation may expose
some behavior as built-ins before it is factored into `.eng` stdlib modules.

## Stdlib Boundary Files

The current `stdlib/eng/` files are module boundary notes. They distinguish:

```text
compiler/runtime built-ins that are supported today
planned pure .eng helper vocabulary
internal vocabulary used by current examples or artifacts
```

The supported built-in seeds are now reflected as explicit files:

```text
stdlib/eng/path.eng
stdlib/eng/io.eng
stdlib/eng/fs.eng
```

`stdlib/eng/config.eng` is supported in narrow scope: `promote json file(...)`
and `promote toml file(...)` validate top-level config fields against schema
columns and emit source hashes plus config promotion summaries. Payload
promotion, nested/list fields, defaults, and unit mismatch policy remain planned.

IDE and LSP completions expose the module boundary names so users can discover
the current surface without implying that every planned helper is executable.

## Review Requirements

Every module that touches external state must produce review material:

```text
input path, URL, command, or database target
resolved value
source or output hash when available
schema or expected artifact shape
profile policy
status
diagnostics and warnings
source span
```

For generated files, `output_manifest.json` is the minimum public record. Its
`artifact_registry` section gives source files, generated files, external
commands, network/cache seed records, DB writes, model artifacts, and tests a
shared review shape, including generated artifact validation records and
`standard_file` classifications for generic fixed-record/text outputs. For
external processes, `process_results.json` records command, args, cwd, tool
version, stdout/stderr hashes, expected outputs, expected-output kind, output
hashes, validation status, duration, and status. For
promoted tables, `typed_payload.table_diagnostics[]` records the current
reviewable schema/row/coverage summary, `typed_payload.table_selections[]`
records selected row, selected value, filters, match count, and selection
reason, `typed_payload.table_transforms[]` records filter/select/sort/require_one/join row
counts, selected columns, sort keys, predicates, join keys, matched pair counts, status, and reason,
`review_document.table_transforms[]` records the static transform contract,
`typed_payload.sample_tables[]` records
deterministic sample/case table metadata when a promoted table is
sample-like, and `typed_payload.case_manifests[]` records one case manifest per
sample row with process-output enrichment from generated `case_manifest.json`
files, `typed_payload.db_manifests[]` records generated DB write manifests,
and current network/cache seeds record fixture boundaries and cache hit/miss
lookup manifests. Future live network execution, cache replay/invalidation,
native case runner, native DB writer, and model modules should follow the same
artifact pattern.

## Hybrid Artifact Evidence

The current workflow examples are executable contract fixtures. They are not
native module claims.

`examples/workflows/01_weather_api_to_standard_file_hybrid` records:

```text
typed station and hourly weather schemas
reviewable station row selection from promoted station map
explicit API fixture boundary
explicit generic DateTime coverage check
weather quality summary
standard text weather artifact
output manifest and report/review entries
```

`examples/workflows/02_external_simulation_surrogate_hybrid` records:

```text
typed design, result, and prediction schemas, including PeopleDensity sample parameters
executed schema constraint policy_results for sample and prediction tables
promoted sample table artifact with case IDs, parameter ranges, duplicate checks, and row-hash previews
promoted case manifest seeds enriched with case directories, process statuses, generated inputs, result files, metrics, and failure reasons
three explicit fixture cases
per-case patched input, simulator output, simulator log, and case_manifest.json classified as case artifacts with expected-output hash and tool-version records
collected summary_results.csv plus result_collection_manifest.json with case IDs, missing/failed case lists, and summary metrics
surrogate.json, model_metrics.json, and model_card.json with feature, target, split, residual, training-hash, and model-hash metadata
internal eng.ml artifacts promoted to typed_payload.model_cards[] with model kind, features, target quantity/unit, train/test counts, metrics, residual point counts, training data hashes, and model artifact hashes
predictions.csv plus prediction_manifest.json with output quantity/unit, model hash, sample hash, case IDs, and row count
db_write_manifest.json, promoted to typed_payload.db_manifests[] with table names, modes, keys, schemas, schema diagnostics, row counts, hashes, and transaction status
process_results.json and output_manifest.json entries for every opaque boundary
```

These fixtures show the review contract that `eng.case`, `eng.db`, and
`eng.model` should eventually make native. The current internal `eng.ml`
seed already exposes model-card summaries as result artifacts, without claiming
a broad ML framework surface.

## Weather API To Standard File Pattern

Generic pattern:

```text
API data
-> typed schema
-> quality and coverage check
-> fallback or imputation policy
-> standard text artifact
-> output manifest
-> report/review artifact
```

Domain-specific adapters can build on this pattern:

```text
eng.weather.kma
eng.weather.epw
eng.weather.tmy
```

Those adapters should remain above the generic `eng.net`, `eng.cache`,
`eng.table`, `eng.timeseries`, and `eng.artifact` layers.

## External Simulation Surrogate Pattern

Generic pattern:

```text
sample table
-> typed validation
-> case materialization
-> input patching through an opaque boundary
-> external process runs
-> typed result collection
-> model-card or surrogate training
-> prediction/export/database write
-> report/review artifact
```

EnergyPlus-like workflows are one adapter of this pattern. The core modules
should also fit CFD, FEM, Modelica, laboratory equipment, and legacy solvers.

## Case Manifest Target

The current case artifact seed records `case_id`, source row, sample row
number, sample row hash, duplicate/missing case status, and process-enriched
case materialization fields when matching expected outputs exist. The planned
native `eng.case` runner should make this explicit syntax:

```text
case_id
sample row hash
case directory
generated input files
process command and status
result files
metrics
failure reason
```

This keeps large parameter sweeps reviewable even when individual tools remain
opaque to EngLang.

## Diagnostics Target

Initial diagnostic families:

```text
E-NET-UNPINNED
E-CACHE-HASH-MISMATCH
E-TABLE-SCHEMA-MISMATCH
E-TIMESERIES-COVERAGE-GAP
E-SAMPLING-SEED-MISSING
E-CASE-ID-DUPLICATE
E-PROCESS-OUTPUT-MISSING
E-DB-SCHEMA-MISMATCH
E-MODEL-CARD-MISSING
W-PROFILE-NONREPRO
W-FALLBACK-USED
```

These should be added only when the corresponding parser, semantic, runtime,
artifact, and test paths exist.

## Example Locations

Composite examples live under:

```text
examples/workflows/
```

The first examples are hybrid skeletons. They use current supported primitives
where possible and document the target contracts for planned modules:

```text
examples/workflows/01_weather_api_to_standard_file_hybrid
examples/workflows/02_external_simulation_surrogate_hybrid
```

These examples are not a claim that the core language includes weather APIs,
EPW writing, EnergyPlus adapters, SQLite, or ML frameworks.
