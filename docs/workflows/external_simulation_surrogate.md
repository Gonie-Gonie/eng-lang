# External Simulation Surrogate

Source example: `examples/workflows/02_external_simulation_surrogate/main.eng`

This workflow demonstrates a native surrogate pattern:

```text
LHS training samples -> LHS prediction samples -> rendered case inputs -> regression_table -> predict -> SQLite
```

Run:

```bat
eng.exe run examples/workflows/02_external_simulation_surrogate/main.eng --save-artifacts
```

What it proves:

- deterministic native `sample lhs` tables for training and prediction inputs
- native template rendering and summary values from selected sample rows
- native table-based `regression_table(...)` model training
- native `predict surrogate_model using designs` prediction table materialization
- native SQLite writes to `simulation_results` and `predictions`
- `process_results.json` has `process_count = 0`

Expected review surfaces:

- `typed_payload.sample_tables[]`
- `typed_payload.case_manifests[]`
- `typed_payload.render_manifests[]`
- `typed_payload.model_specs[]`
- `typed_payload.model_cards[]`
- `typed_payload.prediction_manifests[]`
- `typed_payload.db_manifests[]`
- `output_manifest.json` records for rendered inputs, summary export, DB, model
  artifacts, and report artifacts

This is not an EnergyPlus, CFD, FEM, Modelica, or vendor ML framework adapter
in core. Real simulator or trainer adapters should layer above the same typed
table, model-card, prediction-manifest, DB-manifest, and artifact contracts.
