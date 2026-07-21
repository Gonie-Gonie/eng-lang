# 11 Native IDE Review

## Goal

Use the native IDE as a review surface for diagnostics and generated artifacts.

## What You Will Build

No new source file is required. Open a supported example in the IDE.

## Execute The Program

```bat
eng.exe ide examples/official/01_csv_plot/main.eng
```

## Expected Artifacts

When you run the file from the IDE, it should show diagnostics, runtime
objects, generated artifacts, schema metadata, unit conversions, TimeSeries
metadata, and report/review views.

## Compare Reviews

1. Run the current file and open the Review tab.
2. In Semantic Diff, choose Compare and select either a saved `review.json`
   artifact or a bare `review_document` JSON file.
3. Inspect the changed section hashes and the Added, Removed, and Changed item
   rows.
4. Run the file again after editing. The IDE recomputes the comparison against
   the selected baseline.

## Explanation

The IDE is not a separate language mode. It is a native inspection workflow for
the same files and artifacts produced by command-line runs.

## Common Mistakes

- Treating IDE success as separate from eng.exe run.
- Reviewing only the report panel and skipping diagnostics, semantic changes,
  or artifact paths.
- Editing an example from a read-only package location.

## What To Inspect

Use the Problems panel and the Variables, Units, Schema, Time, Review,
Highlight, and Artifacts tabs. For more detail, read
docs/user/howto/use_native_ide.md.
