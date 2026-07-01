# Native External Simulation Surrogate

Purpose: document the native generic module pattern for sample-driven external
simulation, case artifacts, model prediction, and DB writes without making any
simulator or surrogate framework part of the core language.

Source contract: this is the native module contract for the hybrid workflow at
`examples/workflows/02_external_simulation_surrogate_hybrid/main.eng`.

## Domain-Neutral Pattern

```text
sample table -> case inputs -> external process results -> typed result table -> model card/predictions -> DB write manifest
```

The domain adapter supplies the simulator, model-specific trainer, and optional
result parser. EngLang owns sample/case records, process boundaries, typed
tables, model-card/prediction manifests, DB manifests, output manifests, and
report/review artifacts.

## Source Code

```eng unchecked
schema DesignSample {
    case_id: String
    cooling_cop: Ratio [1]
}

schema SimulationResult {
    case_id: String
    annual_electricity: Energy [kWh]
}

samples = promote csv file("samples/design_samples.csv") as DesignSample
results = promote csv file("outputs/summary_results.csv") as SimulationResult

split = train_test_split(results.annual_electricity, target=results.annual_electricity, features=[cooling_cop], test=0.25, seed=7)
reg_model = regression(split, algorithm=linear)
predictions = predict reg_model using samples

db = open sqlite file("outputs/surrogate_results.sqlite")
write predictions to db.table("predictions")
with {
    mode = append
}
```

## Run Command

```bat
eng.exe run examples/workflows/02_external_simulation_surrogate_hybrid/main.eng --save-artifacts
```

For the native workflow module gate:

```bat
.\dev.bat workflows-test
```

## Expected Artifacts

- `review.json`
- `output_manifest.json`
- `run_log.json`
- `run_plan.json`
- `run_lock.json`
- `process_results.json`
- case `case_manifest.json` files when cases are materialized
- `model_card.json` or native model-card payload when model artifacts are used
- `db_write_manifest.json` or native `typed_payload.db_manifests[]` records
- SQLite database file when a native DB write is used
- prediction manifest records in `typed_payload.prediction_manifests[]`

## Review Checklist

- Sample rows have stable case IDs and row hashes.
- Case manifests show case directory, generated input, process status, result
  files, metrics, and failure reason.
- Process records include command, args, cwd, tool version, stdout/stderr
  hashes, expected outputs, and output hashes.
- Model records include features, target, split, metrics, model card, training
  hash, and model hash.
- Prediction records include schema, output quantity/unit, case IDs, row count,
  confidence column, and model/sample/output hashes when external.
- DB records include table name, mode, key, schema, row count, schema status,
  transaction status, and database hash before/after when available.

## Failure Modes

- invalid sample schema or duplicate case IDs
- missing generated input, result file, or case manifest
- external process timeout, nonzero exit, or missing expected output
- model card missing features, target, metrics, or hashes
- prediction schema mismatch or missing confidence column
- SQLite schema mismatch, missing upsert key, or rollback/transaction failure

## Extension Points For Domain Adapters

- Replace the fake simulator with EnergyPlus, CFD, FEM, Modelica, or lab-tool
  adapters behind the same `eng.process` and case artifact contract.
- Replace trainer/predictor fixtures with a model-specific adapter while
  preserving `eng.model` model-card and prediction-manifest records.
- Replace SQLite with a future DB adapter only if it emits the same typed table,
  manifest, transaction, and review contract.
