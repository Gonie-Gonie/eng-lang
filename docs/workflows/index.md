# Composite Workflow Guide

Composite workflow examples show how current EngLang primitives coordinate
typed data, external adapters, generated artifacts, and review output. They are
user-facing workflow references, not first-run tutorials and not
domain-specific product claims.

Current examples:

- [Weather API to standard file](weather_api_to_standard_file.md)
- [Native weather API to standard file contract](weather_api_to_standard_file_native.md)
- [External simulation surrogate](external_simulation_surrogate.md)
- [Native external simulation surrogate contract](external_simulation_surrogate_native.md)
- [Uncertain sensor report](uncertain_sensor_report.md)

## Shared Contract

The examples repeat a small set of contracts:

- typed input boundary
- explicit external boundary
- generated artifact and hash path
- report/review artifact
- deterministic fixture mode for smoke runs

These contracts define the generic workflow module surface for eng.net,
eng.cache, eng.case, eng.process, eng.db, eng.model, and eng.artifact. Domain
adapters such as weather APIs, standard-file writers, external simulators, and
surrogate trainers stay layered above those generic modules.

## Run All Workflow Examples

```bat
eng.exe run examples/workflows/01_weather_api_to_standard_file_hybrid/main.eng --save-artifacts
eng.exe run examples/workflows/02_external_simulation_surrogate_hybrid/main.eng --save-artifacts
eng.exe run examples/workflows/03_uncertain_sensor_report/main.eng --save-artifacts
```

After each run, compare `build/result/review.json` with the example's expected
summaries.

## Domain Adapter Acceptance

Adapters built after these generic modules must meet the same replacement
contract:

- adapter does not add core language syntax
- adapter uses generic artifact records
- adapter has a review contract
- adapter can be replaced by another domain adapter
- adapter docs clearly say domain-specific
