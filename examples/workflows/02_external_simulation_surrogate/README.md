# External Simulation Surrogate

This workflow demonstrates a native surrogate-workflow pattern without
claiming that EngLang core embeds any particular external simulator:

```text
native input samples -> native result derivation -> case input templates ->
native regression model -> native prediction table -> SQLite side effects ->
report/review artifacts
```

The current executable workflow uses:

```text
eng.sampling  deterministic LHS training-design and prediction sample tables
eng.table     native derive transforms for surrogate simulation-result columns
eng.case      explicit `materialize cases` table plus generated case manifests from sample-style tables
eng.template  native template rendering for three case input files from selected derived result rows
eng.model     regression_table(...) and predict model using samples
eng.db        native SQLite writes for training results and predictions
eng.artifact  output manifest records for rendered inputs, DB, model, and report
```

Expected saved-run properties:

```text
process_results.json has process_count = 0
typed_payload.sample_tables includes training_designs and designs
object_store.tables includes the explicit CaseTable binding `cases`
typed_payload.table_transforms includes native derive records for annual_electricity, annual_cooling, peak_cooling, and unmet_hours
typed_payload.model_cards/model_specs/prediction_manifests are native records
typed_payload.db_manifests records committed writes to simulation_results and predictions
output_manifest.json records rendered case inputs and workflow_summary.csv
workflow_summary.csv records values pulled from the selected native derived-result row, not fixed literals
```

The training-design table is produced by EngLang's native sampler. The result
metrics are then calculated with native `derive` table transforms before the
model, case rendering, CSV export, and SQLite write steps consume them. Domain
adapters can replace the deterministic surrogate formulas later, but they
should still enter EngLang through typed tables, model cards, prediction
manifests, and explicit side-effect records.

Run:

```text
target\debug\eng.exe run examples\workflows\02_external_simulation_surrogate\main.eng --save-artifacts
```
