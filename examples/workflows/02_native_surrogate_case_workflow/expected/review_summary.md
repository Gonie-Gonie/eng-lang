# Expected Review Summary

The saved run should show:

```text
native `sample lhs` tables for training_designs and designs with case IDs, parameter ranges, units, and row-hash previews
workflow bindings and generated sampling_summary.txt expose native sampler method, seed, sample count, and parameter count
native `derive` transforms materializing annual_electricity, annual_cooling, peak_cooling, and unmet_hours from sampled design inputs
explicit native `materialize cases training_results` CaseTable rows with case directories and sample row hashes
native `apply case_input_template over cases` CaseOutput rows for per-case template inputs
workflow bindings expose case pending/failed counts and case-input planned/blocked counts
native filter/require_one transforms selecting case_001 for summary metrics
summary metrics derived from the selected case_001 derived-result row rather than fixed literals
native `train regression` model with feature, target, split, residual, training-hash, and model-hash metadata
native `model_card`, `evaluate`, and `predict ... using ...` records
PredictionResult schema with predicted_annual_electricity and confidence columns
eight generated case_input files plus render manifests
two native SQLite db_write manifests using args.database_target: simulation_results and predictions, both committed
typed SQLite structured readback for persisted_predictions from the predictions table
output_manifest.json entries for case_input, template_render_manifest, sqlite_database, db_write_manifest, sampling_summary.txt, csv_export, and model:// artifacts
process_results.json with process_count = 0
report entries for sampler method/seed/count metadata, case status counts, training, case-input, prediction row counts, persisted prediction readback count, model metrics, the DB target, and DB tables written
```
