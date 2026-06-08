# Integrated HVAC User Test

This tutorial is the recommended public preview user test. It uses the official
integrated HVAC example to exercise the supported preview workflow and the native
tester IDE in one workflow.

## What This Covers

```text
- native tester IDE launch
- example browsing
- live diagnostics
- completion insertion
- typed CSV promotion
- missing-value interpolation
- schema constraint execution
- TimeSeries statistics
- trapezoidal integration
- PlotSpec/SVG/report output
- simple thermal system fixed-step ODE preview
```

## Portable Package Flow

Extract the portable zip and run:

```bat
eng-ide.exe
```

Open:

```text
examples/official/03_integrated_hvac/main.eng
```

In the IDE:

```text
1. Press Check.
2. Confirm diagnostics says 0 errors.
3. Type part of a quantity or unit name and press Ctrl+Space.
4. Insert a completion from the right panel.
5. Press Save if you changed the file.
6. Press Run.
7. Press Open Report.
```

For a command-line smoke of the same package:

```bat
eng-ide.exe --smoke
eng.exe run examples\official\03_integrated_hvac\main.eng
```

## Source Shape

The example has four sections:

```text
schema IntegratedHvacData
  Declares DateTime index, absolute temperature columns, mass-flow rate, bounds,
  monotonic time, and missing-value policy.

args
  Provides the default CSV path.

system RoomThermalPreview
  Declares the supported one-state first-order thermal ODE shape.

top-level executable workflow
  Promotes the CSV, computes Q_coil, integrates E_coil, summarizes statistics,
  and emits a PlotSpec-backed report.
```

## Expected Artifacts

After running, check:

```text
build/result/result.engres
build/result/review.json
build/result/report_spec.json
build/result/report.html
build/result/plots/plot_spec.json
build/result/plots/plot_manifest.json
build/result/plots/timeseries.svg
```

The result should include:

```text
- policy_results with interpolation executed
- computed statistics including p90, p95, median, std, and duration_above(5 kW)
- integration result for E_coil
- systems[0].solver_result.status = computed
```

The report should show the integrated HVAC plot title:

```text
Integrated HVAC coil heat rate
```

## Editing Tests

Good quick edits for user testing:

```text
- Change the plot title and run again.
- Change duration_above(5 kW) to duration_above(4.5 kW).
- Type Heat and press Ctrl+Space to insert HeatRate or HeatCapacity.
- Temporarily change m_dot <= 0.30 kg/s to m_dot <= 0.20 kg/s and check the
  policy diagnostics in the generated result/report.
```

If an edit breaks the language contract, the IDE diagnostics panel should show
the same compiler diagnostic that `eng.exe check` would produce.
