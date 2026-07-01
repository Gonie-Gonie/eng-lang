# Composite Workflow Guide

Composite workflow examples show how current EngLang primitives coordinate
typed data, native workflow modules, generated artifacts, and review output.
They are user-facing workflow references, not first-run tutorials and not
domain-specific product claims.

This page is the workflow subguide linked from [docs/README.md](../README.md),
not a parallel documentation index. Keep global navigation in `docs/README.md`
and keep this page focused on workflow examples and their shared contracts.

Current examples:

- [Weather API to standard file](weather_api_to_standard_file.md)
- [External simulation surrogate](external_simulation_surrogate.md)
- [Uncertain sensor report](uncertain_sensor_report.md)

## Shared Contract

The examples repeat a small set of contracts:

- typed input boundary
- explicit network, file, DB, cache, and artifact boundary records
- generated artifact and hash path
- static/runtime workflow plan and run-lock artifacts
- report/review artifact
- deterministic fixture/native execution for smoke runs
- zero `run command` or Python process execution

These contracts define the generic workflow module surface for `eng.net`,
`eng.cache`, `eng.sampling`, `eng.case`, `eng.template`, `eng.db`,
`eng.model`, `eng.workflow`, and `eng.artifact`. Domain adapters such as
weather APIs, standard-file writers, external simulators, and surrogate
trainers stay layered above those generic modules instead of being hidden
Python/process steps inside the workflow examples.

## Run All Workflow Examples

```bat
eng.exe run examples/workflows/01_weather_api_to_standard_file/main.eng --save-artifacts
eng.exe run examples/workflows/02_external_simulation_surrogate/main.eng --save-artifacts
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
