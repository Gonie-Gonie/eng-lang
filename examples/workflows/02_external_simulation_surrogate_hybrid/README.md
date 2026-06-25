# External Simulation Surrogate Hybrid

This workflow is not EnergyPlus-specific.

EnergyPlus-like tools are one example of the broader pattern:

```text
sample table -> input variants -> external runs -> typed results -> surrogate -> predictions -> database/report
```

The current `main.eng` stays within supported EngLang primitives:

```text
typed sample table promotion with PeopleDensity and power-density columns
schema constraint policy results for sample and prediction tables
typed result table promotion
typed prediction table promotion plus predictor output and prediction manifest
explicit external process boundaries
generated model artifact, metrics artifact, and self-contained model card
generated case manifests with case directories, process statuses, result files, metrics, and failure reasons
generated database side-effect manifest with schema diagnostics and transaction status
scalar report summary
```

Because native loop/case helpers are still planned, `main.eng` expands three
fixture cases explicitly. That keeps the case manifest and DB write contract
reviewable without claiming a native parameter-sweep abstraction yet.

The Python files in `tools/` are fake adapters. They document how a future
`eng.case`, `eng.process`, `eng.model`, and `eng.db` stack should make the
same contract native without turning EngLang into a solver wrapper.

Target contract:

```text
promote sample table
validate sample ranges
create case directories
run patcher per case with explicit base model, sample table, case id, and output path contract
run external simulator per case with explicit input, result, simulator log, and tool-version contract
collect typed simulation results into a CSV and result collection manifest
promote checked result rows with the SimulationResult schema
train surrogate through an explicit external trainer with model artifact, metrics, and model-card outputs
predict new samples through an explicit process boundary with CSV and prediction manifest outputs
write CSV or database side-effect manifests with table, schema, mode, key, row-count, and transaction records
report sample summary, process summary, result metrics, predictions, and model card
```
