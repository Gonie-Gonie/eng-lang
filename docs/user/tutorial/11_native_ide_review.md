# 11 Native IDE Review

## Goal

Use the native IDE as a review surface for diagnostics and generated artifacts.

## What You Will Build

No new source file is required. Open a supported example in the IDE.

## Run Command

```bat
eng.exe ide examples/official/01_csv_plot/main.eng
```

## Expected Artifacts

When you run the file from the IDE, it should show diagnostics, runtime
objects, generated artifacts, schema metadata, unit conversions, TimeSeries
metadata, and report/review views.

## Explanation

The IDE is not a separate language mode. It is a native inspection workflow for
the same files and artifacts produced by command-line runs.

## Common Mistakes

- Treating IDE success as separate from eng.exe run.
- Reviewing only the report panel and skipping diagnostics or artifact paths.
- Editing an example from a read-only package location.

## What To Inspect

Use the Diagnostics, Symbols, Units, Schemas, TimeSeries, Report, and Artifacts
views. For more detail, read docs/guide/native_ide.md.
