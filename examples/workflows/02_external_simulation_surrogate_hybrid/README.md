# External Simulation Surrogate Hybrid

This workflow is not EnergyPlus-specific.

EnergyPlus-like tools are one example of the broader pattern:

```text
sample table -> input variants -> external runs -> typed results -> surrogate -> predictions -> database/report
```

The current `main.eng` stays within supported EngLang primitives:

```text
typed sample table promotion
typed result table promotion
explicit external process boundaries
generated model-card artifact
scalar report summary
```

The Python files in `tools/` are fake adapters. They document how a future
`eng.case`, `eng.process`, `eng.model`, and `eng.db` stack should make the
same contract native without turning EngLang into a solver wrapper.

Target contract:

```text
promote sample table
validate sample ranges
create case directories
run patcher per case
run external simulator per case
collect results
promote results
train surrogate through external process or native model-card seed
predict new samples
write CSV or database outputs
report sample summary, process summary, result metrics, and model card
```

