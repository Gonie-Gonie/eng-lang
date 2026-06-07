# Official CSV Plot Example

This is the primary v1.0 data/report example. It covers:

```text
- schema and typed CSV promotion
- unit and quantity-aware coil heat-rate calculation
- row-level CSV runtime table pages
- schema constraint and missing policy execution status
- computed TimeSeries summary values for mean, time_weighted_mean, median, std,
  p90, p95, and duration_above
- trapezoidal integrate(HeatRate over Time) result value
- CSV-derived PlotSpec v1 points, SVG export, report_spec.json, report.html,
  and result.engres
- `histogram.eng` raw-value histogram PlotSpec bins for the same `Q_coil`
  TimeSeries
- standalone bundle packaging
```

Run from the repository root:

```bat
target\debug\eng.exe run examples\official\01_csv_plot\main.eng --entry main
target\debug\eng.exe run examples\official\01_csv_plot\histogram.eng --entry main
target\debug\eng.exe build examples\official\01_csv_plot\main.eng --entry main --standalone --profile repro
dist\main-standalone\run.bat --help
```

`struct Args` drives standalone help, and extra `run.bat --<field> <value>`
flags are forwarded to `eng.exe run` so Args values are recorded in generated
artifacts.
