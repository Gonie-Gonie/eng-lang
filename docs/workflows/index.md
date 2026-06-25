# Composite Workflow Guide

Composite workflow examples show how current EngLang primitives coordinate
typed data, external adapters, generated artifacts, and review output. They are
user-facing workflow references, not first-run tutorials and not
domain-specific product claims.

Current examples:

- [Weather API to standard file](weather_api_to_standard_file.md)
- [External simulation surrogate](external_simulation_surrogate.md)
- [Uncertain sensor report](uncertain_sensor_report.md)

## Shared Contract

The examples repeat a small set of contracts:

- typed input boundary
- explicit external boundary
- generated artifact and hash path
- report/review artifact
- deterministic fixture mode for smoke runs

These contracts are the seed for generic workflow modules such as eng.net,
eng.cache, eng.case, eng.process, eng.db, eng.model, and eng.artifact. Domain
adapters such as weather APIs, standard-file writers, external simulators, and
surrogate trainers should remain layered above those generic modules.

## Run All Workflow Examples

```bat
eng.exe run examples/workflows/01_weather_api_to_standard_file_hybrid/main.eng --out build/runs/weather_workflow
eng.exe run examples/workflows/02_external_simulation_surrogate_hybrid/main.eng --out build/runs/simulation_workflow
eng.exe run examples/workflows/03_uncertain_sensor_report/main.eng --out build/runs/uncertain_workflow
```

After each run, compare review.json with the example's expected summaries.
