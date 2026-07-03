# Native Surrogate Case Workflow

Source example: `examples/workflows/02_native_surrogate_case_workflow/main.eng`

This workflow demonstrates a native sampling, case, model, prediction, and DB
workflow. Future simulator adapters can feed the same typed contracts, but the
current executable example has `process_count = 0` and does not launch a
simulator adapter:

```text
LHS training samples -> explicit CaseTable -> apply template over cases -> LHS prediction samples -> train regression -> predict -> SQLite
```

Run:

```bat
eng.exe run examples/workflows/02_native_surrogate_case_workflow/main.eng --save-artifacts
```

What it proves:

- deterministic native `sample lhs` tables for training and prediction inputs
- sampler metadata bindings such as `training_designs.method`,
  `training_designs.seed`, and `training_designs.sample_count`
- native result-column derivation before case/model/DB steps consume the table
- explicit native `materialize cases training_results` CaseTable materialization
- native `apply case_input_template over cases` CaseOutput materialization
- case metadata bindings such as `cases.pending_count` and
  `case_inputs.planned_count`
- native case_input artifact rendering plus summary values from case_001
- native table-based `train regression <table>` model training with explicit `with` options
- native `predict surrogate_model using designs` prediction table materialization
- native SQLite writes to `simulation_results` and `predictions`
- typed `args.output` routing for the sampling summary and workflow summary CSV
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
- `output_manifest.json` records for case_input artifacts, sampling summary,
  summary export, DB, model artifacts, and report artifacts

This is not an EnergyPlus, CFD, FEM, Modelica, or vendor ML framework adapter
in core. Real simulator or trainer adapters should layer above the same typed
table, model-card, prediction-manifest, DB-manifest, workflow, and artifact
contracts.
