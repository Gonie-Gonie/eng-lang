# Native External Simulation Surrogate

The executable native example is:

```text
examples/workflows/02_external_simulation_surrogate/main.eng
```

It uses native LHS sampling, template rendering, table-based regression,
prediction-table materialization, and SQLite writes. No Python process is
called; saved runs should show `process_count = 0`.

Expected review surfaces:

- `typed_payload.sample_tables[]`
- `typed_payload.case_manifests[]`
- `typed_payload.render_manifests[]`
- `typed_payload.model_specs[]`
- `typed_payload.model_cards[]`
- `typed_payload.prediction_manifests[]`
- `typed_payload.db_manifests[]`
- `output_manifest.json` records for rendered inputs, summary export, DB,
  model artifacts, and report artifacts
