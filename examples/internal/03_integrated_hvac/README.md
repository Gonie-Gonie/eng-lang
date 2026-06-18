# Internal Integrated HVAC Fixture

This example is an internal integration fixture for the native tester IDE and
portable package smoke coverage.

It exercises several implemented paths in one file:

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

This example is intentionally broad. It is not a public release tutorial and it
does not claim public solver support, general nonlinear solving, DAE solving,
multi-state solving, or production component-graph solving.

Run from the repository root:

```bat
target\debug\eng.exe run examples\internal\03_integrated_hvac\main.eng
```

From a portable package:

```bat
eng.exe run examples\internal\03_integrated_hvac\main.eng
eng-ide.exe
```

In the native tester IDE, open
`examples/internal/03_integrated_hvac/main.eng`, use `Check`, inspect
diagnostics/symbols/completions, then use `Run` and `Open Report`.
