# Official Example 03: Integrated HVAC

This example is the release user-test scenario for the native tester IDE and
portable release package.

It exercises the supported workflow in one file:

```text
- Args default CSV path
- typed CSV promotion
- DateTime index parsing
- numeric missing-value interpolation
- schema constraint execution
- HeatRate calculation with unit conversion
- TimeSeries statistics and duration_above
- trapezoidal integrate provenance
- PlotSpec/SVG/report generation
- simple thermal system metadata and fixed-step one-state ODE result
```

What to inspect after `Run`:

```text
- review.json: schema constraints, missing-value policy, TimeSeries axis,
  integration provenance, and system equation metadata
- report.html: summary statistics, integrated energy, and plotted heat-rate
  series
- plots/timeseries.svg: generated SVG line chart for Q_coil over Time
- result.engres: runtime object records for the promoted table, computed
  HeatRate series, Energy integration, and solver-boundary metadata
```

This example is intentionally broad, but still within the stable-core boundary:
it demonstrates an integrated data-to-report workflow plus a narrow
system/equation surface. It does not claim a general nonlinear, DAE,
multi-state, or production component-graph solver.

Run from the repository root:

```bat
target\debug\eng.exe run examples\official\03_integrated_hvac\main.eng
```

From a portable package:

```bat
eng.exe run examples\official\03_integrated_hvac\main.eng
eng-ide.exe
```

In the native tester IDE, open
`examples/official/03_integrated_hvac/main.eng`, use `Check`, inspect
diagnostics/symbols/completions, then use `Run` and `Open Report`.
