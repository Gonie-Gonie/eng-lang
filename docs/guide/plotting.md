# Plotting Guide

v0.6-preview adds PlotSpec v1 and SVG export from the native report crate.

## Example

```eng partial
plot Q_coil over Time {
    unit y = kW
    type = line
    title = "Coil heat rate"
}
```

The v1.0 hardening path executes the supported plot block options:

```text
unit y = <unit>
type = line | bar | histogram
title = "<title>"
```

The PlotSpec planner infers the requested `TimeSeries[Time]` binding from
semantic metadata and the runtime replaces preview points with official
CSV-derived TimeSeries points.

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

The `points` array is the renderer-independent data model. v0.6 used
deterministic sample points; the v1.0 hardening path uses runtime TimeSeries
points for the official CSV example.

`plot_type = "bar"` and `plot_type = "histogram"` are seed renderers. They
consume existing PlotSpec points and emit SVG rectangles. Histogram binning from
raw distributions is still a later plotting kernel.

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
      "y_axis_label": "HeatRate (W)"
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
eng view lists plot manifest
```

## Deferred

Later versions will add:

```text
- multiple series
- histogram binning from raw values
- grouped/stacked bar semantics
- interactive viewer
- PlotSpec validation schema
```
