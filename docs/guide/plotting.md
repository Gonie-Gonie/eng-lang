# Plotting Guide

The plot/report track emits PlotSpec v1 and SVG output from the native report
crate.

## Example

```eng partial
plot Q_coil over Time
with {
    unit y = kW
    type = line
    title = "Coil heat rate"
}
```

The current report path executes the supported plot options:

```text
unit y = <unit>
unit x = <unit>   # histogram value axis
type = line | bar | histogram
title = "<title>"
```

The PlotSpec planner infers the requested `TimeSeries[Time]` binding from
semantic metadata and the runtime materializes official CSV-derived TimeSeries
points. Multi-series line plots are supported with `plot A and B over Time`
when the series share the same Time axis and compatible display units.

For `examples/official/01_csv_plot/main.eng`, this produces:

```text
build/result/plots/plot_spec.json
build/result/plots/plot_manifest.json
build/result/plots/timeseries.svg
```

## PlotSpec v1

Example:

```json
{
  "format": "eng-plotspec-v1",
  "plot_spec_version": 1,
  "plot_type": "line",
  "title": "Q_coil over Time",
  "x_axis": { "name": "Time", "label": "Time", "unit": "sample" },
  "y_axis": { "name": "HeatRate", "label": "HeatRate", "unit": "W" },
  "series": [
    {
      "name": "Q_coil",
      "quantity_kind": "TimeSeries",
      "display_unit": "W",
      "points": [[0, 20], [1, 32]]
    }
  ]
}
```

The `points` array is the renderer-independent data model. The current runtime
uses runtime TimeSeries points for the official CSV example and deterministic
fallback points only when materialized runtime data is not available.

For multi-series line plots, PlotSpec stores one `series` object per line. The
native SVG renderer draws each line with a stable color and emits a compact
legend using the series names.

`plot_type = "bar"` consumes existing PlotSpec points and emits SVG rectangles.
`plot_type = "histogram"` bins TimeSeries y-values when requested through
`type = histogram` or the clearer `plot histogram(Q_coil)` header:

```eng partial
plot histogram(Q_coil) {
    unit x = kW
    title = "Coil heat-rate distribution"
}
```

Histogram PlotSpec series include `bins` with lower edge, upper edge, center,
and count metadata in addition to center/count `points`. The same bin contract
is used by `plot distribution(...)` for uncertainty summaries.

## SVG Export

The SVG renderer consumes PlotSpec and writes:

```text
build/result/plots/timeseries.svg
```

The SVG includes:

```text
title
line polyline when plot_type is line
bar/histogram rectangles when plot_type requests them
x-axis label with unit
y-axis label with unit
```

For the official plotting example:

```text
x-axis: Time (s)
y-axis: HeatRate (W)
```

## Plot Manifest

`plot_manifest.json` records the generated outputs:

```json
{
  "format": "eng-plot-manifest-v1",
  "plot_spec_version": 1,
  "plots": [
    {
      "title": "Q_coil over Time",
      "plot_type": "line",
      "plot_spec": "plot_spec.json",
      "plot_spec_hash": "...",
      "svg": "timeseries.svg",
      "svg_hash": "...",
      "x_axis_label": "Time (sample)",
      "y_axis_label": "HeatRate (W)",
      "series": ["Q_coil"]
    }
  ]
}
```

`eng view build/result/result.engres` prints the plot manifest path when it exists.

## Result Provenance

`result.engres` records:

```text
provenance.plot_spec_hash
```

This connects the typed result to the PlotSpec used for rendering.

## Tests

v0.6 includes:

```text
PlotSpec JSON smoke
SVG axis label smoke
official run creates plot_spec.json
official run creates plot_manifest.json
measured-vs-simulated run creates two PlotSpec series
SVG renderer emits one polyline and legend entry per line series
eng view lists plot manifest
official histogram run creates binned PlotSpec data
```

## Deferred

Later versions will add:

```text
- grouped/stacked bar semantics
- multiple histogram series and custom bin counts
- interactive viewer
- stricter PlotSpec validation schema
```
