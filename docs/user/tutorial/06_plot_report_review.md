# 06 Plot, Report, And Review

## Goal

Generate a user-facing report and machine-readable review artifact from the
same source.

## What You Will Build

A report block that summarizes and plots a TimeSeries:

```eng partial
report {
    summarize Q_coil by [mean, max, p95]
    show E_coil
    plot Q_coil over Time
    with {
        unit y = kW
        title = "Coil heat rate"
    }
}
```

## Source File

Use examples/official/01_csv_plot/main.eng.

## Run Command

```bat
eng.exe run examples/official/01_csv_plot/main.eng --save-artifacts
```

## Expected Artifacts

Expected files include report.html, review.json, result.engres, and a plot
artifact.

## Explanation

Reports are for humans. Review artifacts are for repeatable inspection. The
same program should produce both so claims in the report can be traced back to
typed inputs, calculations, diagnostics, and generated files.

## Common Mistakes

- Treating the HTML report as the only source of evidence.
- Adding a plot without checking the axis unit and source TimeSeries.
- Ignoring review.json when a report looks visually correct.

## What To Inspect

Open report.html for presentation and review.json for evidence. In the IDE,
check the Report, Artifacts, Units, and TimeSeries views together.
