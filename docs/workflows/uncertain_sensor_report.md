# Uncertain Sensor Report

Source example: examples/workflows/03_uncertain_sensor_report/main.eng

This workflow demonstrates a narrow uncertainty-report pattern:

```text
typed sensor data -> derived TimeSeries -> measured uncertainty metadata -> reviewable report
```

## Run

```bat
eng.exe run examples/workflows/03_uncertain_sensor_report/main.eng --save-artifacts
```

## What It Proves

- typed sensor CSV promotion
- unit-aware TimeSeries heat-rate calculation
- pointwise measured standard-deviation metadata
- native mean, peak, integrated energy, and coverage bindings
- explicit sensor summary CSV and quality text artifacts
- summary statistics with threshold evidence
- confidence-band plot request
- report/review artifact generation
- `process_results.json` has `process_count = 0`
- static/runtime workflow plan and run-lock artifact generation

## What It Does Not Claim

This is not broad uncertainty propagation, seeded Monte Carlo, or a stable
public probabilistic programming API. It is a native measured-uncertainty
workflow that keeps uncertainty propagation deliberately narrow and reviewable.

## Review Points

Inspect uncertainty metadata, confidence-band report output, TimeSeries
statistics, coverage metadata, `static_run_plan.json`, `run_plan.json`,
`run_lock.json`, generated output artifacts, and the example's expected review
summary.
