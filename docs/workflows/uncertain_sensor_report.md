# Uncertain Sensor Report

Source example: examples/workflows/03_uncertain_sensor_report/main.eng

This workflow demonstrates a narrow uncertainty-report pattern:

```text
typed sensor data -> derived TimeSeries -> measured uncertainty metadata -> reviewable report
```

## Run

```bat
eng.exe run examples/workflows/03_uncertain_sensor_report/main.eng --out build/runs/uncertain_workflow
```

## What It Proves

- typed sensor CSV promotion
- unit-aware TimeSeries heat-rate calculation
- pointwise measured standard-deviation metadata
- summary statistics with threshold evidence
- confidence-band plot request
- report/review artifact generation

## What It Does Not Claim

This is not broad uncertainty propagation, seeded Monte Carlo, or a stable
public probabilistic programming API. It is a workflow-shaped fixture for
reviewable measured uncertainty metadata.

## Review Points

Inspect uncertainty metadata, confidence-band report output, TimeSeries
statistics, and the example's expected review summary.
