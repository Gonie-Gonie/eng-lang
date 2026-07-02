# Expected Review Summary

The saved run should show:

```text
native `sample lhs` tables for training_designs and designs with case IDs, parameter ranges, units, and row-hash previews
native `derive` transforms materializing annual_electricity, annual_cooling, peak_cooling, and unmet_hours from sampled design inputs
explicit native `materialize cases training_results` CaseTable rows with case directories and sample row hashes
native `apply case_input_template over cases` CaseOutput rows for per-case template inputs
native filter/require_one transforms selecting case_001 for summary metrics
summary metrics derived from the selected case_001 derived-result row rather than fixed literals
native `regression_table` model with feature, target, split, residual, training-hash, and model-hash metadata
native `model_card`, `evaluate`, and `predict ... using ...` records
PredictionResult schema with predicted_annual_electricity and confidence columns
eight generated case_input files plus render manifests
two native SQLite db_write manifests using args.database_target: simulation_results and predictions, both committed
output_manifest.json entries for case_input, template_render_manifest, sqlite_database, db_write_manifest, csv_export, and model:// artifacts
process_results.json with process_count = 0
report entries for sample, training, case, case-input, prediction row counts, model metrics, the DB target, and DB tables written
```
