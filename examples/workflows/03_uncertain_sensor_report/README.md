# Uncertain Sensor Report

This workflow is not building-energy-specific.

It demonstrates a general pattern:

```text
typed sensor data -> derived TimeSeries -> measured uncertainty metadata -> reviewable report
```

The current `main.eng` stays within supported and internal EngLang primitives:

```text
typed sensor CSV promotion
TimeSeries heat-rate calculation
pointwise measured standard-deviation metadata with `sensor_std`
summary statistics with duration threshold linkage
PlotSpec confidence band request
report/review artifact generation
```

This example is intentionally a workflow-shaped uncertainty fixture, not a
public claim that arbitrary probabilistic TimeSeries propagation is stable.
Future `eng.timeseries`, `eng.validate`, and `eng.report` modules should make
the same contract native while keeping domain-specific sensor models layered
above the generic workflow modules.

Target contract:

```text
promote typed sensor data
derive a unit-aware TimeSeries
attach explicit measurement uncertainty metadata
summarize statistics and threshold duration
integrate total quantity
render confidence-band plot
record uncertainty review metadata
```

Expected saved-run properties:

```text
process_results.json has process_count = 0
review.json records timeseries_uncertainty metadata
report_spec.json and plot_spec.json record the confidence-band plot request
```
