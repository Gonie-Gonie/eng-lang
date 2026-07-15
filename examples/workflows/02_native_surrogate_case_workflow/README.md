# Native Surrogate Case Workflow

This workflow demonstrates a native sampling, case, model, prediction, and DB
workflow. It can act as the EngLang-side contract for future simulator adapters,
and the current executable example stays entirely inside native EngLang
execution:

```text
native input samples -> case input templates -> native per-case calculations ->
native result collection -> native regression model -> native prediction table ->
SQLite write/readback -> report/review artifacts
```

The current executable workflow uses:

```text
eng.sampling  deterministic LHS training-design and prediction sample tables
eng.table     native filter and require_one transforms over completed case results
eng.case      materialize, sequential run_case calculation, verified local cache replay, and collect stages with typed CaseRunResult rows
eng.template  native `apply ... over cases` template rendering for per-case input files
eng.model     train regression ... with { ... } and predict model using samples
eng.db        native SQLite writes plus typed readback for persisted predictions
eng.artifact  output manifest records for rendered inputs, sample table standard-text files, DB, model, and report
```

Expected saved-run properties:

```text
process_results.json has process_count = 0
typed_payload.sample_tables includes training_designs and designs with row previews
standard_text artifacts expose both generated LHS sample tables as reviewable files
report entries include native sample method, seed, count, parameter-count, and row-preview bindings
object_store.objects exposes `cases`, `case_inputs`, `case_runs`, and `case_result_collection` as table entries with their actual schemas and row counts
all four case-stage tables preserve sampled numeric columns; CaseRunResult and collection rows add calculated result columns with units and canonical values
the regression model, case_001 selection, and simulation_results SQLite write consume case_result_collection rather than bypassing the case pipeline
report entries separate initial CaseTable state, rendered CaseOutput state, succeeded CaseRunResult state, and final CaseResultCollection state
each training case has a native `result.json` and `case_run_manifest.json` with calculation hash, output fields, runner, scheduler, and success status
cache_manifest.json records per-case result misses, verified hits, replays, and repairs using calculation hashes and expected result SHA-256 values
typed_payload.model_cards/model_specs/prediction_manifests are native records
typed_payload.db_manifests records committed writes to simulation_results and predictions
db.summary exposes the actual SQLite write summary, while db.tables_written, db.table_count, db.row_count, and db.status keep the details available as EngLang bindings
typed_payload.structured_reads includes sqlite readback for persisted_predictions
typed args.database_target controls the SQLite output boundary
typed args.output controls the sampling summary and workflow summary export paths
output_manifest.json records case inputs, native case results and run manifests, training_designs_standard.txt, prediction_designs_standard.txt, sampling_summary.txt, and workflow_summary.csv
workflow_summary.csv records values pulled from the selected completed case-result row, not fixed literals
```

The training-design table is produced by EngLang's native sampler.
`materialize cases` and template `apply` create each case input. Sequential
`apply run_case` then evaluates the four typed result expressions for every
rendered case, writes its result and run manifest, and exposes a
`CaseRunResult` table for `collect results`. Model training, the selected summary row, and the
`simulation_results` SQLite write all consume the final
`case_result_collection`. The workflow reads sampler metadata through
`training_designs.method`, `training_designs.seed`,
`training_designs.sample_count`, and `training_designs.row_preview`, so the
native sampling contract is visible in normal bindings, standard-text sample table files, reports,
and the result JSON. It also exposes initial CaseTable counts, rendered input
counts, ready/succeeded/failed run counts, and collected/missing/blocked result
counts separately, so each scheduler stage is visible without inspecting JSON
artifacts. Future domain adapters can replace the native result expressions,
but they should still enter EngLang through typed case results, model cards,
prediction manifests, typed DB readback, DB connection summary bindings such as `db.summary`, and
explicit side-effect records.

With `resume = true`, each successful native result/manifest pair is also
stored in a content-addressed local case-result cache. A later run reuses a
current output only after calculation-hash and result-SHA verification; if the
output is missing or damaged, EngLang replays a verified cache entry, and if
both copies are invalid it recalculates the case and repairs the cache.

Run:

```text
target\debug\eng.exe run examples\workflows\02_native_surrogate_case_workflow\main.eng --save-artifacts
```
