# 12 Composite Workflow

## Goal

Understand how EngLang coordinates larger workflows without turning domain
adapters into core language features.

## What You Will Build

Run the current composite workflow examples:

```bat
eng.exe run examples/workflows/01_weather_api_to_standard_file/main.eng --save-artifacts
eng.exe run examples/workflows/02_external_simulation_surrogate/main.eng --save-artifacts
eng.exe run examples/workflows/03_uncertain_sensor_report/main.eng --save-artifacts
```

## Expected Artifacts

Each run should produce review evidence and workflow-specific generated
artifacts. The workflow programs under `examples/workflows/` do not call Python
or external processes; `process_results.json` should report
`process_count = 0`.

## Explanation

Composite workflows repeat the same generic contracts:

- typed input boundaries
- explicit network, cache, file, DB, and artifact boundary records
- generated artifacts with path and hash evidence
- review/report output
- deterministic fixture/native execution for smoke tests
- no hidden Python or `run command` adapter step

Current native modules such as `eng.net`, `eng.cache`, `eng.sampling`,
`eng.case`, `eng.template`, `eng.db`, and `eng.model` grow from these repeated
contracts. Weather APIs, standard-file writers, simulators, and surrogate
trainers remain adapters above the generic workflow layer.

## Common Mistakes

- Reading a workflow example as a domain-specific product claim.
- Hiding adapter failures inside scripts instead of returning manifests.
- Skipping review artifacts because generated files look plausible.

## What To Inspect

Read [Composite workflow guide](../../workflows/index.md), then compare each
run's review.json with the expected summaries under the example directory.
