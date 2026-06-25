# External Simulation Surrogate

Source example: examples/workflows/02_external_simulation_surrogate_hybrid/main.eng

This workflow demonstrates a general adapter pattern:

```text
sample table -> input variants -> external runs -> typed results -> surrogate -> predictions -> database/report
```

## Run

```bat
eng.exe run examples/workflows/02_external_simulation_surrogate_hybrid/main.eng --out build/runs/simulation_workflow
```

## What It Proves

- typed sample table promotion
- explicit case directories and manifests
- external simulator process boundaries
- typed result and prediction table promotion
- model artifact, metrics artifact, and model-card generation
- database side-effect manifest as a reviewable boundary

## What It Does Not Claim

This is not EnergyPlus-specific and does not claim native parameter sweeps,
native surrogate training, or production database writes. The included tools
are fixture adapters that document the contract.

## Review Points

Inspect case manifests, process statuses, result collection manifest, model
card, prediction manifest, database side-effect manifest, and scalar report
summary.
