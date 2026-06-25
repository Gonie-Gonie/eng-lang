# Composite Workflow Base Modules

Status: mixed. Existing path, IO, process, output-manifest, run-log, and test
features are supported in the current public package scope. Promoted CSV
tables now emit `typed_payload.table_diagnostics[]` with schema, row, column,
missing-cell, parse/conversion, and time-axis coverage summaries. Hybrid
examples now emit process-generated weather, case, model-card, prediction, and
database side-effect artifacts. Native network, cache, case, DB, and model
modules remain planned or internal until concrete language/runtime/artifact
slices land.

## Purpose

Composite engineering workflows often look domain-specific from the outside:
weather API to standard weather file, simulation input patching, external
solver runs, surrogate training, database writes, and report generation.

The core language should not become a weather, EPW, KMA, EnergyPlus, CFD, FEM,
or database-specific product. It should provide the generic workflow modules
that make those adapters typed, explicit, reproducible, and reviewable.

## Module Map

| Module | Current status | Purpose |
|---|---|---|
| `eng.path` | Supported through built-ins | typed paths, joins, names, existence checks |
| `eng.io` | Supported through built-ins | read/write text, JSON, TOML, source hashes |
| `eng.fs` | Supported narrow scope | copy, move, delete under generated-output boundaries |
| `eng.config` | Planned | typed JSON/TOML promotion and config validation |
| `eng.log` | Supported through built-ins | structured runtime messages and run logs |
| `eng.process` | Supported narrow scope | explicit external command boundary and process artifacts |
| `eng.test` | Supported narrow scope | local assertions, golden checks, test artifacts |
| `eng.table` | Supported diagnostic artifact seed; planned APIs | promoted table row/column diagnostics; filter/join/derived columns planned |
| `eng.timeseries` | Supported narrow scope | TimeSeries statistics, table time-axis coverage metadata, integration |
| `eng.sampling` | Planned | deterministic sample tables and design sweeps |
| `eng.case` | Planned native module; hybrid fixture evidence | per-case dirs, inputs, processes, results, metrics |
| `eng.net` | Planned | HTTP/download boundaries with cache and hash policy |
| `eng.cache` | Planned | reproducible cache keys, hit/miss artifacts, pinned downloads |
| `eng.artifact` | Supported seed | output manifests, hashes, report/review links |
| `eng.db` | Planned native module; hybrid manifest evidence | SQLite/open database writes with transaction artifacts |
| `eng.model` | Internal/planned; hybrid model evidence | model cards, metrics, residual plots, prediction schemas |

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

`stdlib/eng/config.eng` is intentionally planned: raw `read json` and
`read toml` exist today through `eng.io`, but typed config promotion is not yet
a supported workflow contract.

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

For generated files, `output_manifest.json` is the minimum public record. For
external processes, `process_results.json` is the minimum public record. For
promoted tables, `typed_payload.table_diagnostics[]` records the current
reviewable schema/row/coverage summary. Future network, cache, case, DB, and
model modules should follow the same artifact pattern.

## Hybrid Artifact Evidence

The current workflow examples are executable contract fixtures. They are not
native module claims.

`examples/workflows/01_weather_api_to_standard_file_hybrid` records:

```text
typed station and hourly weather schemas
explicit API fixture boundary
weather quality summary
standard text weather artifact
output manifest and report/review entries
```

`examples/workflows/02_external_simulation_surrogate_hybrid` records:

```text
typed design, result, and prediction schemas
three explicit fixture cases
per-case patched input, simulator output, and case_manifest.json
collected summary_results.csv
surrogate.json and model_metrics.json with hashes and residual metadata
predictions.csv
db_write_manifest.json with table names, modes, keys, schemas, row counts, and transaction status
process_results.json and output_manifest.json entries for every opaque boundary
```

These fixtures show the review contract that `eng.case`, `eng.db`, and
`eng.model` should eventually make native.

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

The planned `eng.case` module should record:

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
