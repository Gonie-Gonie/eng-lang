# 12 Composite Workflow

## Goal

Understand how EngLang coordinates larger workflows without turning domain
adapters into core language features.

## What You Will Build

Run the current composite workflow examples:

```bat
eng.exe run examples/workflows/01_weather_api_to_standard_file_hybrid/main.eng --out build/runs/weather_workflow
eng.exe run examples/workflows/02_external_simulation_surrogate_hybrid/main.eng --out build/runs/simulation_workflow
eng.exe run examples/workflows/03_uncertain_sensor_report/main.eng --out build/runs/uncertain_workflow
```

## Expected Artifacts

Each run should produce review evidence and workflow-specific generated
artifacts. Some examples call small fixture tools so that side-effect and
adapter contracts are visible but deterministic.

## Explanation

Composite workflows repeat the same generic contracts:

- typed input boundaries
- explicit external process boundaries
- generated artifacts with path and hash evidence
- review/report output
- deterministic fixture mode for smoke tests

Future modules such as eng.net, eng.cache, eng.case, eng.db, and eng.model
should grow from these repeated contracts. Weather APIs, standard-file writers,
simulators, and surrogate trainers remain adapters above the generic workflow
layer.

## Common Mistakes

- Reading a workflow example as a domain-specific product claim.
- Hiding adapter failures inside scripts instead of returning manifests.
- Skipping review artifacts because generated files look plausible.

## What To Inspect

Read [Composite workflow guide](../../workflows/index.md), then compare each
run's review.json with the expected summaries under the example directory.
