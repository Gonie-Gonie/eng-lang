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
native mean/peak/integrated energy bindings
native time-axis coverage check
explicit sensor summary CSV and quality text artifacts
single `args.output` directory for generated artifacts
summary statistics with duration threshold linkage
PlotSpec confidence band request
report/review artifact generation
```

This example is intentionally narrow workflow evidence for uncertainty review,
not a public claim that arbitrary probabilistic TimeSeries propagation is stable.
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
write native CSV/text summary artifacts
record uncertainty review metadata
```

Expected saved-run properties:

```text
process_results.json has process_count = 0
review.json records timeseries_uncertainty metadata
output_manifest.json records outputs/sensor_summary.csv and outputs/sensor_quality_summary.txt
result.engres records the native coverage binding with status complete
result.engres records mean/integration/duration uncertainty propagation artifacts
report_spec.json and plot_spec.json record the confidence-band plot request
```
