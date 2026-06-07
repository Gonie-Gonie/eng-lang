# Official CSV Plot Example

This is the primary v1.0 data/report example. It covers:

```text
- schema and typed CSV promotion
- unit and quantity-aware coil heat-rate calculation
- row-level CSV runtime table pages
- computed TimeSeries summary values for mean, max, and p95
- trapezoidal integrate(HeatRate over Time) result value
- CSV-derived PlotSpec v1 points, SVG export, report_spec.json, report.html,
  and result.engres
- standalone bundle packaging
```

Run from the repository root:

```bat
target\debug\eng.exe run examples\official\01_csv_plot\main.eng --entry main
target\debug\eng.exe build examples\official\01_csv_plot\main.eng --entry main --standalone --profile repro
dist\main-standalone\run.bat --help
```

`struct Args` is currently used as metadata for standalone help. Runtime flag
binding from Args fields is deferred.
