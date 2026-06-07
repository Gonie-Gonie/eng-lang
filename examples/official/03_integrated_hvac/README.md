# Official Example 03: Integrated HVAC

This example is the v1.0.3 user-test scenario for the native tester IDE and
portable release package.

It exercises the stable core in one file:

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
- simple thermal system metadata and fixed-step ODE preview
```

Run from the repository root:

```bat
target\debug\eng.exe run examples\official\03_integrated_hvac\main.eng --entry main
```

From a portable package:

```bat
eng.exe run examples\official\03_integrated_hvac\main.eng --entry main
eng-ide.exe
```

In the native tester IDE, open
`examples/official/03_integrated_hvac/main.eng`, use `Check`, inspect
diagnostics/symbols/completions, then use `Run` and `Open Report`.
