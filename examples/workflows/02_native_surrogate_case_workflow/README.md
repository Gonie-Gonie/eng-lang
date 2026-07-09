# Native Surrogate Case Workflow

This workflow demonstrates a native sampling, case, model, prediction, and DB
workflow. It can act as the EngLang-side contract for future simulator adapters,
and the current executable example stays entirely inside native EngLang
execution:

```text
native input samples -> native result derivation -> case input templates ->
native result collection -> native regression model -> native prediction table ->
SQLite write/readback -> report/review artifacts
```

The current executable workflow uses:

```text
eng.sampling  deterministic LHS training-design and prediction sample tables
eng.table     native derive transforms for surrogate simulation-result columns
eng.case      explicit `materialize cases` table, generated case manifests, and `collect results` CaseResultCollection rows
eng.template  native `apply ... over cases` template rendering for per-case input files
eng.model     train regression ... with { ... } and predict model using samples
eng.db        native SQLite writes plus typed readback for persisted predictions
eng.artifact  output manifest records for rendered inputs, DB, model, and report
```

Expected saved-run properties:

```text
process_results.json has process_count = 0
typed_payload.sample_tables includes training_designs and designs
report entries include native sample method, seed, count, and parameter-count bindings
object_store.tables includes the explicit CaseTable binding `cases`
object_store.tables includes the CaseOutput binding `case_inputs`
object_store.tables includes the CaseResultCollection binding `case_result_collection`
report entries include `cases.pending_count`, `cases.failed_count`, `case_inputs.rendered_count`, `case_result_collection.collected_count`, and `case_result_collection.status`
typed_payload.table_transforms includes native derive records for annual_electricity, annual_cooling, peak_cooling, and unmet_hours
typed_payload.model_cards/model_specs/prediction_manifests are native records
typed_payload.db_manifests records committed writes to simulation_results and predictions
typed_payload.structured_reads includes sqlite readback for persisted_predictions
typed args.database_target controls the SQLite output boundary
typed args.output controls the sampling summary and workflow summary export paths
output_manifest.json records case_input artifacts, sampling_summary.txt, and workflow_summary.csv
workflow_summary.csv records values pulled from the selected native derived-result row, not fixed literals
```

The training-design table is produced by EngLang's native sampler. The result
metrics are then calculated with native `derive` table transforms before the
case table, case-input apply step, model, CSV export, and SQLite write steps
consume them. The workflow reads sampler metadata through
`training_designs.method`, `training_designs.seed`, and
`training_designs.sample_count`, so the native sampling contract is visible in
normal bindings and output files. It also reads `cases.pending_count`, `case_inputs.rendered_count`, and
`case_result_collection.collected_count`, and `case_result_collection.status`, so case materialization, case-input
rendering, and native result collection are visible without digging through JSON
artifacts. Domain adapters can replace the
deterministic surrogate formulas later, but they should still enter EngLang
through typed tables, model cards, prediction manifests, typed DB readback, and
explicit side-effect records.

Run:

```text
target\debug\eng.exe run examples\workflows\02_native_surrogate_case_workflow\main.eng --save-artifacts
```
