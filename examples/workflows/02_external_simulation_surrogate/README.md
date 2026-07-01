# External Simulation Surrogate

This workflow demonstrates a native surrogate-workflow pattern without
claiming that EngLang core embeds any particular external simulator:

```text
native sample tables -> case input templates -> native regression model ->
native prediction table -> SQLite side effects -> report/review artifacts
```

The current executable workflow uses:

```text
eng.sampling  deterministic LHS training and prediction sample tables
eng.case      generated case manifests from sample-style tables
eng.template  native template rendering for three case input files
eng.model     regression_table(...) and predict model using samples
eng.db        native SQLite writes for training results and predictions
eng.artifact  output manifest records for rendered inputs, DB, model, and report
```

Expected saved-run properties:

```text
process_results.json has process_count = 0
typed_payload.sample_tables includes training_results and designs
typed_payload.model_cards/model_specs/prediction_manifests are native records
typed_payload.db_manifests records committed writes to simulation_results and predictions
output_manifest.json records rendered case inputs and workflow_summary.csv
```

The generated training table is synthetic fixture data produced by EngLang's
native sampler. Domain adapters can replace that source with real simulator
results later, but they should still enter EngLang through typed tables,
model cards, prediction manifests, and explicit side-effect records.

Run:

```text
target\debug\eng.exe run examples\workflows\02_external_simulation_surrogate\main.eng --save-artifacts
```
