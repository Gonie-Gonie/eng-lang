# 07 Validation And Diagnostics

## Goal

Use diagnostics as a review tool instead of treating failures as opaque command
errors.

## What You Will Build

No new source is required. Run a valid example, then deliberately inspect how
schema and unit diagnostics are reported when source or input data is wrong.

## Run Commands

```bat
eng.exe check examples/official/01_csv_plot/main.eng
eng.exe run examples/official/01_csv_plot/main.eng --out build/runs/diagnostics
```

## Expected Artifacts

Successful runs produce review artifacts. Failed checks should report source
locations and diagnostic messages specific enough to fix the boundary that
failed.

## Explanation

EngLang diagnostics are part of reviewability. The goal is to find unit
mismatches, missing schema fields, invalid paths, and unsupported constructs
near the source of the problem.

## Common Mistakes

- Fixing downstream formulas before reading the first diagnostic.
- Removing type or unit annotations to make a diagnostic disappear.
- Treating internal-track examples as evidence that a public workflow is
  supported.

## What To Inspect

For a failed check, inspect the first diagnostic and its source span. For a
successful run, inspect the validation and schema sections of the review
artifact.
