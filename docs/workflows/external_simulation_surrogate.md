# Native Surrogate Case Workflow

Source example: `examples/workflows/02_external_simulation_surrogate/main.eng`

This workflow demonstrates a native sampling, case, model, prediction, and DB
workflow. Future simulator adapters can feed the same typed contracts, but the
current executable example runs with zero external process adapters:

```text
LHS training samples -> explicit CaseTable -> apply template over cases -> LHS prediction samples -> train regression -> predict -> SQLite
```

Run:

```bat
eng.exe run examples/workflows/02_external_simulation_surrogate/main.eng --save-artifacts
```

What it proves:

- deterministic native `sample lhs` tables for training and prediction inputs
- explicit native `materialize cases training_results` CaseTable materialization
- native `apply case_input_template over cases` CaseOutput materialization
- native case_input artifact rendering plus summary values from case_001
- native table-based `train regression <table>` model training with explicit `with` options
- native `predict surrogate_model using designs` prediction table materialization
- native SQLite writes to `simulation_results` and `predictions`
- `process_results.json` has `process_count = 0`

Expected review surfaces:

- `typed_payload.sample_tables[]`
- `typed_payload.case_manifests[]`
- object-store `CaseOutput` table `case_inputs`
- `typed_payload.render_manifests[]`
- `typed_payload.model_specs[]`
- `typed_payload.model_cards[]`
- `typed_payload.prediction_manifests[]`
- `typed_payload.db_manifests[]`
- `static_run_plan.json`, `run_plan.json`, and `run_lock.json`
- `output_manifest.json` records for case_input artifacts, summary export, DB, model
  artifacts, and report artifacts

This is not an EnergyPlus, CFD, FEM, Modelica, or vendor ML framework adapter
in core. Real simulator or trainer adapters should layer above the same typed
table, model-card, prediction-manifest, DB-manifest, workflow, and artifact
contracts.
