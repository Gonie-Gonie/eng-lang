# Expected Review Summary

The saved run should show:

```text
native `sample lhs` tables for training_designs and designs with case IDs, parameter ranges, units, row-hash previews, and row-value previews
workflow bindings, generated sampling_summary.txt, and standard-text sample table files expose native sampler method, seed, sample count, parameter count, row previews, and full row values
explicit native `materialize cases training_designs` CaseTable rows with case directories and sample row hashes
native `apply case_input_template over cases` CaseOutput rows for per-case template inputs
native `apply run_case over case_inputs` bounded parallel scheduler rows calculating annual_electricity, annual_cooling, peak_cooling, and unmet_hours per case with zero external processes
per-case `result.json` and `case_run_manifest.json` artifacts with expression output metadata, hashes, runner, scheduler, configured/effective worker counts, deterministic worker slot, lifecycle hooks, and success status
native `collect results case_runs` CaseResultCollection rows for completed native case results
workflow bindings expose initial case manifest pending/failed counts separately from case-input rendered/blocked counts, case-run ready/succeeded/failed counts, and collected/missing/blocked final result counts
native filter/require_one transforms selecting case_001 for summary metrics
summary metrics derived from the selected case_001 native case-run result row rather than fixed literals
native `train regression` model with feature, target, split, residual, training-hash, and model-hash metadata
native `model_card`, `evaluate`, and `predict ... using ...` records
PredictionResult schema with predicted_annual_electricity and confidence columns
normalized ReviewDocument symbol results for the model, model card, metrics, and prediction with computed coefficients/metrics, train/test counts, output schema/case IDs, and training/model/prediction hashes
eight generated case_input files plus render manifests, native result files, and run manifests
two native SQLite db_write manifests using args.database_target: simulation_results and predictions, both committed
typed SQLite structured readback for persisted_predictions from the predictions table
output_manifest.json entries for case_input, template_render_manifest, sqlite_database, db_write_manifest, sample table standard-text files, sampling_summary.txt, csv_export, and model:// artifacts
process_results.json with process_count = 0
report entries for sampler method/seed/count/row-preview metadata, initial case manifest status, case-input and final case-result-collection counts, prediction row counts, persisted prediction readback count, model metrics, the DB target, and native DB summary/table/count/status bindings
```
