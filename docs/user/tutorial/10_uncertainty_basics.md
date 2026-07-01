# 10 Uncertainty Basics

## Goal

Understand the current uncertainty track without overstating public support.

## What You Will Build

Use the workflow-shaped fixture:

```bat
eng.exe run examples/workflows/03_uncertain_sensor_report/main.eng --save-artifacts
```

## Expected Artifacts

The run should produce a report, review artifact, and uncertainty-related
metadata for the supported fixture.

## Explanation

The current user-facing message is narrow: uncertainty metadata can be attached
and reviewed in selected workflow examples, but broad probabilistic TimeSeries
propagation, seeded Monte Carlo workflows, and general uncertainty calculus are
not stable public features yet.

The language philosophy is still important: uncertainty should be explicit,
auditable, and connected to measurements or assumptions rather than hidden in
post-processing.

## Common Mistakes

- Assuming every deterministic TimeSeries operation automatically propagates
  uncertainty.
- Treating an internal fixture as a stable public API.
- Reporting a confidence band without recording where the uncertainty came
  from.

## What To Inspect

Inspect review.json, the generated plot/report, and
examples/workflows/03_uncertain_sensor_report/expected/review_summary.md. For
current scope, read docs/current/uncertainty.md.
