# Composite Workflow Base Modules

Status: mixed. Existing path, IO, process, output-manifest, run-log, and test
features are supported in the current public package scope. Promoted CSV
tables now emit `typed_payload.table_diagnostics[]` with schema, row, column,
missing-cell, parse/conversion, time-axis coverage summaries, and
`typed_payload.timeseries_coverage[]` records with expected counts, missing
intervals, max gaps, and leap-year policy, explicit
`typed_payload.timeseries_fill[]` records for `fill missing` interpolation
policies, `typed_payload.timeseries_quality[]` summaries that combine coverage
and fill outcomes, `typed_payload.expectation_suites[]` lightweight expectation
suite records, `typed_payload.quality_results[]` common quality records for
TimeSeries, validation, schema-constraint, and expectation-suite results,
row/field failure details, `report_spec.quality_report`, HTML Quality Report
tables, and IDE Quality inspector payloads,
explicit alignment/resampling hooks in
`typed_payload.time_alignments[]`, plus
deterministic row-selection records in `typed_payload.table_selections[]`. Promoted
table filter/select/derive/sort/require_one/join seeds now emit static `review_document.table_transforms[]`
and runtime `typed_payload.table_transforms[]` records with predicates, join
keys, matched pair counts, row counts, Date/DateTime predicate comparison, and
`row_diagnostics[]`. Promoted
`sample grid`, `sample random`, `sample lhs`, and DesignSample-style CSV tables
now emit `typed_payload.sample_tables[]` with case ID, parameter range,
duplicate-case, seed, generation, and row-hash preview metadata, plus
`typed_payload.case_tables[]` summaries and `typed_payload.case_manifests[]`
case row manifests with pending/succeeded/failed/skipped status, sample row
hashes, collection manifest counts, case cache hit/miss counts, scheduler hook
contracts, and process-output enrichment. Hybrid
examples now emit process-generated weather, case, model-card, prediction, and
database side-effect artifacts. Native network and cache record seeds have
landed for offline/fixture boundaries and cache manifests; cache manifests now
share owner records across network, process, model, and case workflow surfaces,
enforce observed cache hashes under the repro profile, and warn about stale
cache entries. Native SQLite append/upsert write seeds now produce DB files,
DB manifests, schema diagnostics, hash before/after records, and transaction
status. Native `predict <model> using <table>` now materializes prediction
tables and manifests. Live network execution, cache replay, case runner,
broad DB support, and broader model train syntax remain planned or internal until concrete
language/runtime/artifact slices land.

## Purpose

Composite engineering workflows often look domain-specific from the outside:
weather API to standard weather file, simulation input patching, external
solver runs, surrogate training, database writes, and report generation.

The core language should not become a weather, EPW, KMA, EnergyPlus, CFD, FEM,
or database-specific product. It should provide the generic workflow modules
that make those adapters typed, explicit, reproducible, and reviewable.

## Module Map

The canonical machine-readable registry is `stdlib/eng/modules.toml`. The table
below is generated from that registry and checked by `dev.bat docs-check`.

<!-- module-registry-table:start -->
| Module | Status | Backing | Artifacts | Diagnostics | Examples | Tests |
|---|---|---|---|---|---|---|
| `eng.path` | supported | compiler_runtime_builtin | `review.inputs`<br>`review.environment_dependencies` | `E-PATH-INVALID`<br>`E-PATH-TRAVERSAL`<br>`E-PATH-OUTSIDE-OUTPUT-ROOT` | `examples/official/10_path_policy` | `cargo test -p eng_compiler path_policy` |
| `eng.io` | supported | compiler_runtime_builtin | `review.inputs`<br>`review.side_effects`<br>`output_manifest` | `E-IO-JSON-PARSE`<br>`E-IO-TOML-PARSE`<br>`E-IO-JSON-FIELD-ACCESS-001` | `examples/official/11_read_only_io` | `cargo test -p eng_compiler read_only_io` |
| `eng.fs` | supported_narrow | compiler_runtime_builtin | `review.side_effects`<br>`output_manifest`<br>`run_log` | `E-FS-CONFIRM-001`<br>`E-FS-DELETE-001`<br>`E-PROFILE-SAFE-FS` | `examples/official/13_file_operations` | `cargo test -p eng_compiler file_operations` |
| `eng.log` | supported | compiler_runtime_builtin | `run_log` | `none_current` | `examples/official/15_process_result` | `cargo test -p eng_runtime run_file` |
| `eng.process` | supported_narrow | compiler_runtime_builtin | `review.external_boundaries`<br>`process_results`<br>`output_manifest`<br>`run_log` | `E-PROCESS-001`<br>`E-PROCESS-BINDING-001`<br>`E-PROCESS-CMD-001`<br>`E-PROCESS-ENV-001`<br>`E-PROCESS-CWD-001`<br>`E-PROCESS-TIMEOUT`<br>`E-PROCESS-RETRY-POLICY`<br>`E-PROCESS-ALLOW-FAILURE`<br>`E-PROFILE-SAFE-PROCESS` | `examples/official/15_process_result`<br>`examples/workflows/02_external_simulation_surrogate_hybrid` | `cargo test -p eng_compiler process`<br>`cargo test -p eng_runtime process` |
| `eng.test` | supported_narrow | compiler_runtime_builtin | `test_results`<br>`review.tests`<br>`output_manifest` | `E-ASSERT-001`<br>`E-ASSERT-002`<br>`E-ASSERT-UNIT-001`<br>`E-GOLDEN-002` | `examples/official/13_file_operations` | `cargo test -p eng_compiler records_test_assert_and_golden_metadata`<br>`cargo test -p eng_runtime run_file_executes_test_assert_and_golden_checks` |
| `eng.table` | supported_seed | compiler_runtime_builtin | `review.inputs`<br>`review_document.table_transforms`<br>`typed_payload.table_diagnostics`<br>`typed_payload.table_selections`<br>`typed_payload.table_transforms` | `E-TABLE-UNKNOWN-COLUMN`<br>`E-TABLE-PREDICATE-TYPE`<br>`E-TABLE-JOIN-KEY-MISMATCH`<br>`E-TABLE-SCHEMA-MISMATCH` | `examples/workflows/01_weather_api_to_standard_file_hybrid`<br>`examples/workflows/02_external_simulation_surrogate_hybrid` | `cargo test -p eng_runtime table_`<br>`cargo test -p eng_compiler table_` |
| `eng.timeseries` | supported_seed | compiler_runtime_builtin | `typed_payload.timeseries_coverage`<br>`typed_payload.timeseries_fill`<br>`typed_payload.timeseries_quality`<br>`typed_payload.quality_results`<br>`typed_payload.time_alignments`<br>`review.fallbacks` | `E-TIMESERIES-COVERAGE-GAP`<br>`W-FALLBACK-USED` | `examples/workflows/01_weather_api_to_standard_file_hybrid` | `cargo test -p eng_runtime run_file_records_timeseries_coverage_in_review`<br>`cargo test -p eng_runtime run_file_records_timeseries_fill_missing_in_artifacts`<br>`cargo test -p eng_runtime run_file_records_timeseries_alignment_and_resampling_hooks` |
| `eng.sampling` | supported_seed | compiler_runtime_builtin | `typed_payload.sample_tables`<br>`case_manifest` | `E-SAMPLING-COUNT-INVALID`<br>`E-SAMPLING-RANGE-UNIT`<br>`E-SAMPLING-SEED-MISSING`<br>`E-CASE-ID-DUPLICATE` | `examples/workflows/02_external_simulation_surrogate_hybrid` | `cargo test -p eng_compiler sample_generation`<br>`cargo test -p eng_runtime sample` |
| `eng.case` | supported_seed | compiler_runtime_builtin | `typed_payload.case_tables`<br>`typed_payload.case_manifests`<br>`typed_payload.case_diagnostics`<br>`case_manifest`<br>`output_manifest` | `E-CASE-ID-DUPLICATE`<br>`E-CASE-DIR-COLLISION`<br>`E-CASE-OUTPUT-MISSING`<br>`E-CASE-STEP-FAILED`<br>`W-CASE-SKIPPED-CACHE` | `examples/workflows/02_external_simulation_surrogate_hybrid` | `cargo test -p eng_runtime case_manifest` |
| `eng.artifact` | supported_seed | compiler_runtime_builtin | `output_manifest`<br>`review.side_effects`<br>`artifact_registry` | `artifact_validation_failed` | `examples/workflows/01_weather_api_to_standard_file_hybrid`<br>`examples/workflows/02_external_simulation_surrogate_hybrid` | `cargo test -p eng_runtime output_manifest` |
| `eng.review` | supported_seed | compiler_runtime_builtin | `review` | `semantic_diff_changed`<br>`review_risk` | `eng review examples/workflows/01_weather_api_to_standard_file_hybrid/main.eng` | `cargo test -p eng_compiler review_json_exposes_normalized_review_document`<br>`cargo test -p eng_cli review_semantic_diff_compares_workflow_modules` |
| `eng.model` | supported_seed | compiler_runtime_builtin | `typed_payload.model_specs`<br>`typed_payload.model_cards`<br>`typed_payload.prediction_manifests`<br>`typed_payload.model_diagnostics`<br>`model_card`<br>`model_metrics`<br>`output_manifest` | `E-MODEL-FEATURE-MISSING`<br>`E-MODEL-TARGET-MISSING`<br>`E-MODEL-CARD-MISSING`<br>`W-MODEL-EXTRAPOLATION` | `examples/workflows/02_external_simulation_surrogate_hybrid`<br>`examples/internal/05_data_driven_modeling` | `cargo test -p eng_compiler records_data_driven_modeling_metadata`<br>`cargo test -p eng_runtime model`<br>`cargo test -p eng_runtime run_file_predicts_native_model_into_table_and_sqlite` |
| `eng.db` | supported_seed | compiler_runtime_builtin | `typed_payload.db_manifests`<br>`db_write_manifest`<br>`sqlite_database`<br>`review.external_boundaries` | `E-DB-CONNECT`<br>`E-DB-SCHEMA-MISMATCH`<br>`E-DB-KEY-MISSING`<br>`E-DB-TRANSACTION-FAILED`<br>`E-DB-SAFE-PROFILE`<br>`W-PROFILE-REPRO-DB` | `examples/workflows/02_external_simulation_surrogate_hybrid` | `cargo test -p eng_compiler lowers_native_db_write_seed`<br>`cargo test -p eng_runtime sqlite`<br>`cargo test -p eng_runtime run_file_safe_profile_rejects_native_db_write`<br>`cargo test -p eng_runtime run_file_repro_profile_records_native_db_write` |
| `eng.config` | supported_narrow | compiler_runtime_builtin | `typed_payload.config_promotions`<br>`review.config_promotions`<br>`output_manifest` | `E-CONFIG-SOURCE-001`<br>`E-CONFIG-MISSING-FIELD`<br>`E-CONFIG-UNKNOWN-FIELD`<br>`E-CONFIG-NULL-NOT-OPTIONAL`<br>`E-CONFIG-TYPE-MISMATCH` | `tests/runtime/config_optional_fields.eng` | `cargo test -p eng_compiler config_`<br>`cargo test -p eng_runtime config_` |
| `eng.net` | supported_seed | compiler_runtime_builtin | `review.external_boundaries`<br>`typed_payload.network_boundaries`<br>`run_log.network_events`<br>`output_manifest` | `E-NET-INVALID-URL`<br>`E-NET-RETRY-POLICY`<br>`E-NET-TIMEOUT`<br>`E-NET-BODY-SIZE-LIMIT`<br>`E-NET-HASH-MISMATCH`<br>`E-NET-UNPINNED-REPRO` | `examples/workflows/01_weather_api_to_standard_file_hybrid` | `cargo test -p eng_compiler net_`<br>`cargo test -p eng_runtime network`<br>`cargo test -p eng_runtime secret_arg` |
| `eng.cache` | supported_seed | compiler_runtime_builtin | `cache_manifest`<br>`review.caches`<br>`run_log.cache_events`<br>`output_manifest` | `E-CACHE-KEY-NONDETERMINISTIC`<br>`E-CACHE-HASH-MISMATCH`<br>`E-CACHE-UNHASHED-REPRO`<br>`W-CACHE-STALE` | `examples/workflows/01_weather_api_to_standard_file_hybrid` | `cargo test -p eng_compiler cache_`<br>`cargo test -p eng_runtime cache` |
| `eng.quality` | supported_seed | compiler_runtime_builtin | `typed_payload.expectation_suites`<br>`typed_payload.quality_results`<br>`typed_payload.validations`<br>`typed_payload.policy_results`<br>`review.expectation_suites`<br>`review.quality_results`<br>`review.validations`<br>`report_spec.quality_report`<br>`report_html.quality_report`<br>`ide.quality_inspector`<br>`output_manifest` | `E-TABLE-SCHEMA-MISMATCH`<br>`W-FALLBACK-USED` | `examples/diagnostics/data_quality` | `cargo test -p eng_compiler lowers_expectation_suite_seed`<br>`cargo test -p eng_runtime run_file_records_common_quality_results_for_validation_and_schema_constraints` |
| `eng.template` | supported_seed | compiler_runtime_builtin | `typed_payload.render_manifests`<br>`template_render_manifest`<br>`review.render_manifests`<br>`output_manifest` | `E-TEMPLATE-MISSING-VALUE` | `examples/workflows/02_external_simulation_surrogate_hybrid` | `cargo test -p eng_compiler render_template_command_lowers_with_template_contract`<br>`cargo test -p eng_runtime template` |
| `eng.workflow` | planned | none | `run_plan`<br>`run_lock`<br>`output_manifest`<br>`run_log` | `run_lock_changed`<br>`artifact_hash_mismatch` | `examples/workflows/01_weather_api_to_standard_file_hybrid`<br>`examples/workflows/02_external_simulation_surrogate_hybrid` | `cargo test -p eng_runtime run_plan` |
| `eng.report` | planned | none | `report`<br>`review`<br>`output_manifest` | `none_current` | `examples/workflows/03_uncertain_sensor_report` | `cargo test -p eng_report` |
| `eng.stats` | planned | none | `review.statistics` | `W-STATS-SUM-001` | `examples/official/01_csv_plot` | `cargo test -p eng_compiler stats` |
| `eng.plot` | planned | none | `plotspec`<br>`plot_manifest`<br>`output_manifest` | `none_current` | `examples/official/01_csv_plot` | `cargo test -p eng_report plot` |
| `eng.building` | planned | none | `review.objects` | `planned` | `planned_building_examples` | `planned_building_tests` |
| `eng.system` | internal_planned | internal | `review.systems`<br>`system_ir` | `solver_or_numeric` | `examples/advanced_solver/31_external_behavior_solver` | `cargo test -p eng_runtime system` |
| `eng.ml` | internal | compiler_runtime_builtin | `typed_payload.ml`<br>`typed_payload.model_specs`<br>`typed_payload.model_cards` | `E-MODEL-FEATURE-MISSING`<br>`E-MODEL-TARGET-MISSING`<br>`E-MODEL-CARD-MISSING` | `examples/internal/05_data_driven_modeling` | `cargo test -p eng_runtime model` |
| `eng.uncertainty` | internal | compiler_runtime_builtin | `typed_payload.uncertainties`<br>`review.uncertainty` | `W-UNC-INDEPENDENCE-ASSUMED`<br>`W-WITH-UNCERTAINTY-SEED-001` | `examples/workflows/03_uncertain_sensor_report` | `cargo test -p eng_compiler uncertainty`<br>`cargo test -p eng_runtime uncertainty` |
<!-- module-registry-table:end -->

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
and `promote toml file(...)` validate top-level, nested object, and array/list
config fields against schema columns, allow optional missing/null fields
declared as `Optional[T]` or `T?`, apply schema defaults for missing config
fields, and emit source hashes plus config promotion summaries. Payload
promotion and unit mismatch policy remain planned.

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
external processes, `process_results.json` records command, args, env keys,
cwd, timeout, retry policy, attempt count, allow-failure policy, timed-out
state, tool version, stdout/stderr hashes, expected outputs,
expected-output kind, output hashes, validation status, duration, and status.
For
promoted tables, `typed_payload.table_diagnostics[]` records the current
reviewable schema/row/coverage summary, `typed_payload.table_selections[]`
records selected row, selected value, filters, match count, and selection
reason, `typed_payload.table_transforms[]` records filter/select/derive/sort/require_one/join row
counts, Date/DateTime predicate comparison evidence, selected columns,
derived columns, sort keys, predicates, join keys, matched pair counts, row
diagnostics, status, and reason,
`review_document.table_transforms[]` records the static transform contract,
`typed_payload.sample_tables[]` records deterministic generated and promoted
sample/case table metadata, and `typed_payload.case_manifests[]` records one case manifest per
sample row with process-output enrichment from generated `case_manifest.json`
files, `typed_payload.db_manifests[]` records generated and native SQLite DB
write manifests, and current network/cache seeds record fixture boundaries and
cache hit/miss lookup manifests. Future live network execution,
cache replay/invalidation, native case runner, broad DB engines, and model
modules should follow the same artifact pattern.

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
internal eng.ml artifacts and external model_card.json files promoted to typed_payload.model_specs[] and typed_payload.model_cards[] with model kind, features, target quantity/unit, train/test counts, metrics, residual point counts, training data hashes, and model artifact hashes
predictions.csv plus prediction_manifest.json promoted to typed_payload.prediction_manifests[] with prediction schema, output quantity/unit, model hash, sample hash, case IDs, row count, and confidence-column metadata
typed_payload.model_diagnostics[] for missing model cards, missing features/targets, and prediction schema warnings
db_write_manifest.json, promoted to typed_payload.db_manifests[] with table names, modes, keys, schemas, schema diagnostics, row counts, hashes, and transaction status
process_results.json and output_manifest.json entries for every opaque boundary
```

These fixtures show the review contract that `eng.case`, `eng.db`, and
`eng.model` should preserve as native slices grow. The current `eng.db` seed
adds native SQLite append/upsert writes for typed tables, while broad DB engines
and query APIs remain planned. The current `eng.model` seed makes external
model cards, model specs, prediction manifests, and model diagnostics
reviewable when they cross explicit process expected-output boundaries, and
native `predict <model> using <table>` materializes Table[Prediction] rows with
case IDs, predicted target values, confidence, and prediction-manifest metadata.
Broader model train syntax remains planned, and the internal `eng.ml` seed
exposes matching model review artifacts without claiming a broad ML framework surface.

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
number, sample row hash, default case directory, pending/succeeded/failed/skipped
status, result collection status, cache hit/miss counts, scheduler hooks,
duplicate diagnostics, and
process-enriched case materialization fields when matching expected outputs
exist. The planned native `eng.case` apply/run syntax should make this explicit:

```text
case_id
sample row hash
case directory
generated input files
process command and status
result files
metrics
failure reason
case summary table
case diagnostics
```

This keeps large parameter sweeps reviewable even when individual tools remain
opaque to EngLang.

## Diagnostics Target

Initial diagnostic families:

```text
E-NET-UNPINNED-REPRO
E-CACHE-HASH-MISMATCH
E-CACHE-UNHASHED-REPRO
W-CACHE-STALE
E-TABLE-SCHEMA-MISMATCH
E-TIMESERIES-COVERAGE-GAP
E-SAMPLING-SEED-MISSING
E-CASE-ID-DUPLICATE
E-CASE-DIR-COLLISION
E-CASE-OUTPUT-MISSING
E-CASE-STEP-FAILED
W-CASE-SKIPPED-CACHE
E-DB-CONNECT
E-DB-SCHEMA-MISMATCH
E-DB-KEY-MISSING
E-DB-TRANSACTION-FAILED
E-DB-SAFE-PROFILE
W-PROFILE-REPRO-DB
E-MODEL-FEATURE-MISSING
E-MODEL-TARGET-MISSING
E-MODEL-CARD-MISSING
W-MODEL-EXTRAPOLATION
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

### Hybrid Workflow Fixtures

The current executable workflow examples are hybrid fixtures. They use current
supported primitives where possible and document the target contracts for
planned modules:

```text
examples/workflows/01_weather_api_to_standard_file_hybrid
examples/workflows/02_external_simulation_surrogate_hybrid
```

These examples are not a claim that the core language includes weather APIs,
EPW writing, EnergyPlus adapters, SQLite, or ML frameworks.

### Native Workflow Targets

Native workflow examples should use generic `eng.*` modules directly once their
parser, semantic, runtime, artifact, and review contracts exist. They should be
stored separately from hybrid fixtures and named without the `_hybrid` suffix.
Until those examples land, the module registry table above is the source of
truth for which native workflow surfaces are supported, seed-only, planned, or
internal.
