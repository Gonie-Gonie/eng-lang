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
table filter/select/derive/sort/require_one/join operations now emit static
`review_document.table_transforms[]` and runtime `typed_payload.table_transforms[]` records with predicates, join
keys, matched pair counts, row counts, Date/DateTime predicate comparison, and
`row_diagnostics[]`. Promoted
`sample grid`, `sample random`, `sample lhs`, and DesignSample-style CSV tables
now emit `typed_payload.sample_tables[]` with case ID, parameter range,
duplicate-case checks, deterministic generation settings, row-hash review
metadata, and row-value previews, plus
`typed_payload.case_tables[]` summaries and `typed_payload.case_manifests[]`
case row manifests with pending/succeeded/failed/skipped status, sample row
hashes, collection manifest counts, case cache hit/miss counts, scheduler hook
contracts, and process-output enrichment when external processes are used. The
workflow examples now exercise native network/cache, sampling, template,
model-prediction, sample-table standard-text export, DB-write, and generated-artifact paths with zero external
processes in workflows 01, 02, and 03. Native network and cache records now
cover pinned offline response boundaries, live HTTP(S) response
materialization, and cache records; cache records now
share owner records across network, process, model, and case workflow surfaces,
materialize/replay pinned network response cache entries, enforce observed cache hashes
under the repro profile, and warn about stale cache entries. Native SQLite
append/upsert/replace writes now produce DB files,
DB manifests, schema diagnostics, hash before/after records, and transaction
status. Native `predict <model> using <table>` now materializes prediction
tables and manifests. Broader cache invalidation/reuse, case runner, broad DB
support, and broader model train syntax remain planned or internal until
concrete language/runtime/artifact slices land.

## Purpose

Composite engineering workflows often look domain-specific from the outside:
weather API to standard weather file, case input generation, optional external
solver integration, surrogate training, database writes, and report generation.

The core language should not become a weather, EPW, KMA, EnergyPlus, CFD, FEM,
or database-specific product. It should provide the generic workflow modules
that make those adapters typed, explicit, reproducible, and reviewable. The
workflow 01/02/03 smoke contract is native-only: no language-external
interpreter marker, interactive script artifact marker, or command-process
workflow step may appear in source/docs/run graph artifacts, and
`process_results.json` must report zero processes.

Network replay has two intentionally different public names: workflow examples
use args such as `args.pinned_response_file` for user-supplied saved response files,
while the language-level HTTP option is `offline_response`. The legacy `fixture`
option exists only as a migration alias that diagnostics and quick fixes rewrite
to `offline_response`.

## Module Map

The canonical machine-readable registry is `stdlib/eng/modules.toml`. The table
below is generated from that registry and checked by `dev.bat docs-check`.

<!-- module-registry-table:start -->
| Module | Status | Backing | Artifacts | Diagnostics | Examples | Tests |
|---|---|---|---|---|---|---|
| `eng.path` | Supported | Compiler/runtime | `review.inputs`<br>`review.environment_dependencies` | `E-PATH-INVALID`<br>`E-PATH-TRAVERSAL`<br>`E-PATH-OUTSIDE-OUTPUT-ROOT` | `examples/official/10_path_policy` | `cargo test -p eng_compiler path_policy` |
| `eng.io` | Supported | Compiler/runtime | `review.inputs`<br>`review.side_effects`<br>`output_manifest` | `E-IO-JSON-PARSE`<br>`E-IO-TOML-PARSE`<br>`E-IO-JSON-FIELD-ACCESS-001`<br>`E-EXPORT-CSV-001`<br>`E-EXPORT-CSV-002`<br>`E-EXPORT-CSV-003`<br>`E-EXPORT-CSV-004`<br>`E-WRITE-001`<br>`E-WRITE-002`<br>`E-WRITE-003`<br>`E-WRITE-FMT-001`<br>`E-WRITE-FMT-002`<br>`E-WRITE-FMT-003`<br>`E-WRITE-FMT-004`<br>`E-WRITE-STANDARD-TEXT-001`<br>`E-WRITE-STANDARD-TEXT-OUTPUT` | `examples/official/11_read_only_io` | `cargo test -p eng_compiler read_only_io` |
| `eng.fs` | Supported narrow | Compiler/runtime | `review.side_effects`<br>`output_manifest`<br>`run_log` | `E-FS-001`<br>`E-FS-002`<br>`E-FS-003`<br>`E-FS-CONFIRM-001`<br>`E-FS-DELETE-001`<br>`E-PROFILE-SAFE-FS` | `examples/official/13_file_operations` | `cargo test -p eng_compiler file_operations` |
| `eng.log` | Supported | Compiler/runtime | `run_log` | `E-LOG-LEVEL-001` | `examples/official/15_process_result` | `cargo test -p eng_runtime run_file` |
| `eng.process` | Supported narrow | Compiler/runtime | `review.external_boundaries`<br>`process_results`<br>`output_manifest`<br>`run_log` | `E-PROCESS-001`<br>`E-PROCESS-BINDING-001`<br>`E-PROCESS-BINDING-002`<br>`E-PROCESS-CMD-001`<br>`E-PROCESS-ENV-001`<br>`E-PROCESS-CWD-001`<br>`E-PROCESS-TIMEOUT`<br>`E-PROCESS-RETRY-POLICY`<br>`E-PROCESS-ALLOW-FAILURE`<br>`E-PROFILE-SAFE-PROCESS` | `examples/official/15_process_result` | `cargo test -p eng_compiler process`<br>`cargo test -p eng_runtime process` |
| `eng.test` | Supported narrow | Compiler/runtime | `test_results`<br>`review.tests`<br>`output_manifest` | `E-ASSERT-001`<br>`E-ASSERT-002`<br>`E-ASSERT-EXPR-001`<br>`E-ASSERT-UNIT-001`<br>`E-ASSERT-TOL-001`<br>`E-ASSERT-TOL-002`<br>`E-GOLDEN-001`<br>`E-GOLDEN-002` | `examples/official/13_file_operations` | `cargo test -p eng_compiler records_test_assert_and_golden_metadata`<br>`cargo test -p eng_runtime run_file_executes_test_assert_and_golden_checks` |
| `eng.table` | Native workflow support | Compiler/runtime | `review.inputs`<br>`review_document.table_transforms`<br>`typed_payload.table_diagnostics`<br>`typed_payload.table_selections`<br>`typed_payload.table_transforms` | `E-TABLE-UNKNOWN-COLUMN`<br>`E-TABLE-PREDICATE-TYPE`<br>`E-TABLE-JOIN-KEY-MISMATCH`<br>`E-TABLE-SCHEMA-MISMATCH`<br>`W-TABLE-LEGACY-SELECT-FIRST-ROW` | `examples/workflows/01_weather_api_to_standard_file`<br>`examples/workflows/02_native_surrogate_case_workflow` | `cargo test -p eng_runtime table_`<br>`cargo test -p eng_compiler table_` |
| `eng.timeseries` | Native workflow support | Compiler/runtime | `typed_payload.timeseries_coverage`<br>`typed_payload.timeseries_fill`<br>`typed_payload.timeseries_quality`<br>`typed_payload.quality_results`<br>`typed_payload.time_alignments`<br>`review.fallbacks` | `E-TIMESERIES-COVERAGE-GAP`<br>`W-FALLBACK-USED` | `examples/workflows/01_weather_api_to_standard_file` | `cargo test -p eng_runtime run_file_records_timeseries_coverage_in_review`<br>`cargo test -p eng_runtime run_file_records_timeseries_fill_missing_in_artifacts`<br>`cargo test -p eng_runtime run_file_records_timeseries_alignment_and_resampling_hooks` |
| `eng.sampling` | Native workflow support | Compiler/runtime | `typed_payload.sample_tables`<br>`case_manifest` | `E-SAMPLING-COUNT-INVALID`<br>`E-SAMPLING-RANGE-UNIT`<br>`E-SAMPLING-SEED-INVALID`<br>`E-SAMPLING-SEED-MISSING`<br>`E-CASE-ID-DUPLICATE` | `examples/workflows/02_native_surrogate_case_workflow` | `cargo test -p eng_compiler sample_generation`<br>`cargo test -p eng_runtime sample` |
| `eng.case` | Native workflow support | Compiler/runtime | `typed_payload.case_tables`<br>`typed_payload.case_manifests`<br>`typed_payload.case_diagnostics`<br>`case_manifest`<br>`output_manifest` | `E-CASE-ID-DUPLICATE`<br>`E-CASE-DIR-COLLISION`<br>`E-CASE-OUTPUT-MISSING`<br>`E-CASE-STEP-FAILED`<br>`W-CASE-SKIPPED-CACHE` | `examples/workflows/02_native_surrogate_case_workflow` | `cargo test -p eng_runtime case_manifest` |
| `eng.artifact` | Native workflow support | Compiler/runtime | `output_manifest`<br>`review.side_effects`<br>`artifact_registry` | - | `examples/workflows/01_weather_api_to_standard_file`<br>`examples/workflows/02_native_surrogate_case_workflow` | `cargo test -p eng_runtime output_manifest`<br>`cargo test -p eng_runtime run_file_writes_standard_text_from_native_table` |
| `eng.review` | Native workflow support | Compiler/runtime | `review` | - | `eng review examples/workflows/01_weather_api_to_standard_file/main.eng` | `cargo test -p eng_compiler review_json_exposes_normalized_review_document`<br>`cargo test -p eng_cli review_semantic_diff_compares_workflow_modules` |
| `eng.model` | Native workflow support | Compiler/runtime | `typed_payload.model_specs`<br>`typed_payload.model_cards`<br>`typed_payload.prediction_manifests`<br>`typed_payload.model_diagnostics`<br>`model_card`<br>`model_metrics`<br>`output_manifest` | `E-MODEL-FEATURE-MISSING`<br>`E-MODEL-TARGET-MISSING`<br>`E-MODEL-CARD-MISSING`<br>`W-MODEL-EXTRAPOLATION` | `examples/workflows/02_native_surrogate_case_workflow`<br>`examples/internal/05_data_driven_modeling` | `cargo test -p eng_compiler records_data_driven_modeling_metadata`<br>`cargo test -p eng_runtime model`<br>`cargo test -p eng_runtime run_file_predicts_native_model_into_table_and_sqlite` |
| `eng.db` | Native workflow support | Compiler/runtime | `typed_payload.db_manifests`<br>`typed_payload.structured_reads`<br>`db_write_manifest`<br>`sqlite_database`<br>`review.external_boundaries` | `E-DB-CONNECT`<br>`E-DB-SCHEMA-MISMATCH`<br>`E-DB-READ-001`<br>`E-DB-KEY-MISSING`<br>`E-DB-TRANSACTION-FAILED`<br>`E-DB-SAFE-PROFILE`<br>`W-PROFILE-REPRO-DB` | `examples/workflows/02_native_surrogate_case_workflow` | `cargo test -p eng_compiler lowers_native_db_write_records`<br>`cargo test -p eng_compiler lowers_native_db_read_binding`<br>`cargo test -p eng_runtime run_file_reads_sqlite_table_after_native_write`<br>`cargo test -p eng_runtime sqlite`<br>`cargo test -p eng_runtime run_file_safe_profile_rejects_native_db_write`<br>`cargo test -p eng_runtime run_file_repro_profile_records_native_db_write` |
| `eng.config` | Supported narrow | Compiler/runtime | `typed_payload.config_promotions`<br>`review.config_promotions`<br>`output_manifest` | `E-CONFIG-SOURCE-001`<br>`E-CONFIG-MISSING-FIELD`<br>`E-CONFIG-UNKNOWN-FIELD`<br>`E-CONFIG-NULL-NOT-OPTIONAL`<br>`E-CONFIG-TYPE-MISMATCH` | `tests/runtime/config_optional_fields.eng` | `cargo test -p eng_compiler config_`<br>`cargo test -p eng_runtime config_` |
| `eng.net` | Native workflow support | Compiler/runtime | `review.external_boundaries`<br>`typed_payload.network_boundaries`<br>`run_log.network_events`<br>`output_manifest` | `E-NET-INVALID-URL`<br>`E-NET-RETRY-POLICY`<br>`E-NET-TIMEOUT`<br>`E-NET-BODY-METHOD`<br>`E-NET-BODY-POLICY`<br>`E-NET-BODY-SIZE-LIMIT`<br>`E-NET-HASH-MISMATCH`<br>`E-NET-UNPINNED-REPRO`<br>`E-NET-SECRET-LIVE`<br>`W-NET-FIXTURE-ALIAS`<br>`W-NET-RESPONSE-HASH-ALIAS`<br>`W-NET-RESPONSE-STATUS-ALIAS` | `examples/workflows/01_weather_api_to_standard_file` | `cargo test -p eng_compiler net_`<br>`cargo test -p eng_runtime network`<br>`cargo test -p eng_runtime secret_arg`<br>`cargo test -p eng_runtime run_file_executes_live_http_response_body_json_source`<br>`cargo test -p eng_runtime run_file_sends_live_http_request_body` |
| `eng.cache` | Native workflow support | Compiler/runtime | `cache_manifest`<br>`review.caches`<br>`run_log.cache_events`<br>`output_manifest` | `E-CACHE-KEY-NONDETERMINISTIC`<br>`E-CACHE-DIR`<br>`E-CACHE-TTL`<br>`E-CACHE-HASH-MISMATCH`<br>`E-CACHE-UNHASHED-REPRO`<br>`W-CACHE-STALE` | `examples/workflows/01_weather_api_to_standard_file` | `cargo test -p eng_compiler cache_`<br>`cargo test -p eng_runtime cache` |
| `eng.quality` | Native workflow support | Compiler/runtime | `typed_payload.expectation_suites`<br>`typed_payload.quality_results`<br>`typed_payload.validations`<br>`typed_payload.policy_results`<br>`review.expectation_suites`<br>`review.quality_results`<br>`review.validations`<br>`report_spec.quality_report`<br>`report_html.quality_report`<br>`ide.quality_inspector`<br>`output_manifest` | `E-TABLE-SCHEMA-MISMATCH`<br>`W-FALLBACK-USED` | `examples/diagnostics/data_quality` | `cargo test -p eng_compiler lowers_expectation_suite_records`<br>`cargo test -p eng_runtime run_file_records_common_quality_results_for_validation_and_schema_constraints` |
| `eng.template` | Native workflow support | Compiler/runtime | `typed_payload.render_manifests`<br>`template_render_manifest`<br>`review.render_manifests`<br>`output_manifest` | `E-TEMPLATE-MISSING-VALUE` | `examples/workflows/02_native_surrogate_case_workflow` | `cargo test -p eng_compiler render_template_command_lowers_with_template_contract`<br>`cargo test -p eng_runtime template` |
| `eng.workflow` | Native workflow support | Compiler/runtime | `run_plan`<br>`run_lock`<br>`output_manifest`<br>`run_log` | - | `examples/workflows/01_weather_api_to_standard_file`<br>`examples/workflows/02_native_surrogate_case_workflow` | `cargo test -p eng_runtime run_plan` |
| `eng.report` | Native workflow support | eng_report | `report_spec`<br>`report_html`<br>`review`<br>`output_manifest` | - | `examples/workflows/03_uncertain_sensor_report` | `cargo test -p eng_report` |
| `eng.stats` | Planned | No executable backing | `review.statistics` | `W-STATS-SUM-001` | `examples/official/01_csv_plot` | `cargo test -p eng_compiler stats` |
| `eng.plot` | Native workflow support | eng_report | `plot_spec`<br>`plot_manifest`<br>`plot_svg`<br>`output_manifest` | - | `examples/official/01_csv_plot`<br>`examples/workflows/03_uncertain_sensor_report` | `cargo test -p eng_report plot` |
| `eng.building` | Planned | No executable backing | `review.objects` | - | `planned_building_examples` | `planned_building_tests` |
| `eng.system` | Internal planned | Internal | `review.systems`<br>`system_ir` | - | `examples/advanced_solver/31_external_behavior_solver` | `cargo test -p eng_runtime system` |
| `eng.ml` | Internal | Compiler/runtime | `typed_payload.ml`<br>`typed_payload.model_specs`<br>`typed_payload.model_cards` | `E-MODEL-FEATURE-MISSING`<br>`E-MODEL-TARGET-MISSING`<br>`E-MODEL-CARD-MISSING` | `examples/internal/05_data_driven_modeling` | `cargo test -p eng_runtime model` |
| `eng.uncertainty` | Native workflow support | Compiler/runtime | `typed_payload.uncertainties`<br>`review.uncertainty`<br>`timeseries_uncertainty`<br>`report_spec.confidence_band`<br>`plot_spec.confidence_band` | `W-UNC-INDEPENDENCE-ASSUMED`<br>`W-WITH-UNCERTAINTY-SEED-001` | `examples/workflows/03_uncertain_sensor_report` | `cargo test -p eng_compiler uncertainty`<br>`cargo test -p eng_runtime uncertainty` |
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

The supported and native-workflow built-ins are reflected as explicit files:

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
promotion from `read json` bindings is supported for config validation, and
`promote json records payload.records as SchemaName` materializes typed tables
from JSON record arrays. Broader unit mismatch policy remains planned.

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
commands, network/cache records, DB writes, model artifacts, and tests a
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
write manifests, and current network/cache records capture pinned offline
boundaries, live HTTP(S) response materialization, and cache hit/miss lookup
records, including materialized/replayed pinned network response cache entries.
Future broader cache invalidation/reuse, general run-case scheduling beyond
the current materialize/apply/collect case-table path, broad DB engines,
and model modules should follow the same artifact pattern.

## Native Artifact Evidence

The workflow programs under `examples/workflows/` are executable native
workflows. They declare native data, network/cache, model, DB, artifact, and
review records directly; they are not domain-adapter claims.

`examples/workflows/01_weather_api_to_standard_file` records:

```text
typed station schema and WeatherApiRecord JSON-record table schema
reviewable filter/require_one station transform from promoted station map
explicit pinned offline API response boundary
native network/cache record for the pinned API response, using the same
runtime-bound response-body path as live `http://` requests
native JSON schema promotion for the WeatherApiPayload API contract
native JSON records table promotion for api_contract.records
explicit generic DateTime coverage check
weather quality summary
standard text weather artifact generated by `write standard_text`
output manifest and report/review entries
process_results.json with process_count = 0
```

`examples/workflows/02_native_surrogate_case_workflow` records:

```text
deterministic LHS training and prediction sample tables
sample table artifacts with case IDs, parameter ranges, duplicate checks, row-hash records, and row-value previews
case manifest records for generated sample/case rows
rendered CaseOutput rows from `apply case_input_template over cases`
native case_input artifacts plus template_render_manifest records
preferred native `train regression` plus legacy-compatible `regression_table` model card/spec/diagnostic records with feature, target, metrics, training-hash, and model-hash metadata
native prediction table and typed_payload.prediction_manifests[] records with output quantity/unit, case IDs, row count, and confidence column
native SQLite db_manifests[] records with table names, modes, schemas, row counts, hashes, and committed transaction status
output_manifest.json entries for case inputs, sample table standard-text files, workflow_summary.csv, model artifacts, DB writes, and report artifacts
process_results.json with process_count = 0
standard-text files for full training and prediction sample table rows
```

`examples/workflows/03_uncertain_sensor_report` records:

```text
typed sensor CSV promotion
unit-aware TimeSeries derivation, integration, mean, peak, and coverage records
explicit measured uncertainty metadata from sensor_std
summary CSV and text quality artifacts
report_spec.json and plot_spec.json confidence-band records
output_manifest.json entries for generated sensor summaries and plot/report artifacts
process_results.json with process_count = 0
```

These workflow programs show the review contract that `eng.net`, `eng.cache`,
`eng.sampling`, `eng.template`, `eng.case`, `eng.db`, and `eng.model` preserve
inside native workflow modules. Workflow 03 adds the same native-artifact
evidence for `eng.timeseries`, `eng.uncertainty`, `eng.report`, and `eng.plot`.
Simulator and domain adapters can still be layered through `eng.process`, but
workflows 01, 02, and 03 do not require that boundary.
Broader model train syntax remains planned, and the internal `eng.ml` path
exposes matching model review artifacts without claiming a broad ML framework surface.

## Weather API To Standard File Pattern

Generic pattern:

```text
API data
-> typed schema
-> API JSON contract validation
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

## Native Surrogate Adapter Pattern

Generic pattern:

```text
sample table
-> typed validation
-> case materialization
-> native input template rendering
-> native training/result table materialization
-> model-card or surrogate training
-> prediction/export/database write
-> report/review artifact
```

External simulators such as EnergyPlus, CFD, FEM, Modelica, laboratory
equipment, and legacy solvers can be adapters above this pattern. The current
workflow 02 example does not run those adapters; it proves the native table,
case, template, model-card, prediction, DB, and artifact contracts they should
feed into.

## Case Manifest Target

The current case artifact record contains `case_id`, source row, sample row
number, sample row hash, default case directory, pending/succeeded/failed/skipped
status, result collection status, cache hit/miss counts, scheduler hooks,
duplicate diagnostics, and
optional process-enriched case materialization fields only when a workflow uses
an `eng.process` adapter with matching expected outputs. Current native
`materialize cases`, `apply ... over cases`, and `collect results <CaseOutput>`
make the supported table/case/template path explicit by materializing CaseTable,
CaseOutput rows with expected, rendered, blocked, output, and manifest counts, and
CaseResultCollection rows. A collection row is `collected` only when the source
CaseOutput row has render evidence; planned output paths remain `missing` until
the native render step has materialized the input and render manifest. Broader
run-case scheduler policy should extend the same record shape:

```text
case_id
sample row hash
case directory
generated input files
optional adapter command and status
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
E-CACHE-DIR
E-CACHE-TTL
E-CACHE-UNHASHED-REPRO
W-CACHE-STALE
E-TABLE-SCHEMA-MISMATCH
E-TIMESERIES-COVERAGE-GAP
E-SAMPLING-SEED-INVALID
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

### Native Workflow Examples

The current executable workflow examples use supported native primitives:

```text
examples/workflows/01_weather_api_to_standard_file
examples/workflows/02_native_surrogate_case_workflow
examples/workflows/03_uncertain_sensor_report
```

These examples are not a claim that the core language includes KMA, EPW,
EnergyPlus, CFD, FEM, or ML framework adapters. Such adapters should be layered
above the generic native workflow modules listed in the registry table.
