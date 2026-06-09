# Stable Core Scope

EngLang `1.0.0` is a stable-core release, not a claim that every experimental
track is complete. The stable contract is intentionally narrow: the documented
data-to-report workflow, artifact family, package smoke path, and native tester
workflow are expected to remain compatible across `1.x` releases.

## Stable In 1.0.0

- Top-level file execution without `script main`.
- Root `args { ... }` for String/path/CsvFile/DirectoryPath and primitive
  Bool/Int/Count/Float/Duration values.
- Fast `=` bindings, explicit quantity declarations, and `:=` rejection.
- Built-in quantity/unit registry used by the official examples.
- `degC` plus the `°C` alias for absolute temperature display.
- Typed CSV promotion for the official schema/data boundary.
- DateTime-indexed table metadata and the supported HeatRate TimeSeries path.
- Supported TimeSeries statistics and trapezoidal integration for the official
  data path.
- Measured-vs-simulated workflow seed: CSV-derived measured TimeSeries, minimal
  fixed-step one-state thermal simulation output as `sim.T_zone`, RMSE metric,
  validation result, and time-alignment artifact metadata.
- Unit-aware `print`, structured `log <level>`, one-row summary CSV export,
  explicit write outputs, process results, and local test/assert/golden checks
  within their documented boundaries.
- PlotSpec v1 line and multi-series line plot, SVG output, report HTML, review
  JSON, report spec, run log, process results, test results, and output
  manifest artifacts.
- `eng run --profile safe|normal|repro` basics: safe rejects explicit workflow
  write/export/file-operation/process effects, normal is the default, and repro
  records profile diagnostics in result/run-log/output-manifest artifacts.
- Standalone packaged runner with `.engpkg`, `.lock`, Args help, dependency
  copying, package smoke, and curated PDF docs.
- Tauri/WebView tester IDE smoke path, terminal/variables/plot preview, and
  on-demand report/plot opening for the stable workflow.

## Shipped But Not Stable

The package still includes preview or experimental tracks. They may change in
`1.x` releases as long as the stable-core contract above is preserved:

- General nonlinear, adaptive, or multi-state physical solvers.
- Broad TimeSeries/table expression execution beyond the documented path.
- Uncertainty and data-driven modeling engines.
- LSP/VS Code as a persistent editor-service contract.
- Native JIT/AOT execution or speedup claims.
- Domain/component package ecosystem and numeric multi-domain solving.
- Full filesystem/network support and full process sandboxing.
- Workspace-wide test discovery, filtering, and fixtures.

## Stable Gate

Before a `1.0.0` package is published:

```text
.\dev.bat docs-check
.\dev.bat artifacts-check
.\dev.bat test
.\dev.bat release-check
```

The package must pass from a clean extracted folder without Rust, Python, Node,
or Visual Studio Build Tools on the target side.
