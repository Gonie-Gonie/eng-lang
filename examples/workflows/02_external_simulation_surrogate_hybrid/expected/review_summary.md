# Expected Review Summary

The saved run should show:

```text
DesignSample, SimulationResult, and PredictionResult schema promotion, including PeopleDensity sample units
promoted sample table artifact with case IDs, parameter ranges, and row-hash previews
schema constraint policy results for sample and prediction ranges
explicit process boundaries for input patching, simulation, collection, case manifests,
training, prediction, and DB write manifests
expected output contracts, tool-version metadata, stdout/stderr hashes, and hashed patched/simulator/collection outputs for every external process boundary
typed SimulationResult rows plus a result collection manifest with row count, case IDs, missing cases, failed cases, and summary metrics
three generated case manifests with case directories, process statuses, result files, metrics, and failure reasons
case input, case result, and case manifest artifact kinds in output_manifest.json
one generated workflow summary CSV
generated surrogate model, metrics, and self-contained model-card artifacts with feature, target, split, residual, training-hash, and model-hash metadata
generated prediction CSV plus prediction manifest with output quantity/unit, model hash, sample hash, case IDs, and row count
one generated database side-effect manifest summarized in typed_payload.db_manifests[] with schema diagnostics and transaction status
report entries for row counts, scalar metrics, predictions, and the DB target
```
