# TimeSeries Statistics Guide

The TimeSeries/statistics path computes numeric summary and scalar-call values
for materialized series while keeping broader arbitrary TimeSeries expression
execution outside this narrow contract.

## Example

```eng partial
sensor = promote csv "data/sensor.csv" as SensorData
cp = 4180 J/kg/K
Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
E_coil = integrate(Q_coil, over=Time)

report {
    summarize Q_coil by [mean, time_weighted_mean, max, median, std, p90, p95, duration_above(5 kW)]
    show E_coil
    plot Q_coil over Time
}
```

The compiler records:

```text
Q_coil
  type: TimeSeries[Time] of HeatRate
  display unit: W
  axis: Time

E_coil
  type: Energy
  derived from: integrate(Q_coil, over=Time)
```

## Axis Metadata

`review.json` includes `axis_info`:

```json
{
  "binding": "Q_coil",
  "axis": "Time",
  "role": "sample_axis",
  "source": "timeseries",
  "line": 11
}
```

Promoted CSV tables also expose the schema index as an axis source:

```json
{
  "binding": "sensor",
  "axis": "Time",
  "role": "index",
  "source": "schema"
}
```

## Summary Metadata

`summarize Q_coil by [mean, time_weighted_mean, max, median, std, p90, p95, duration_above(5 kW)]` creates:

```json
{
  "source": "Q_coil",
  "source_type": "TimeSeries[Time] of HeatRate",
  "quantity_kind": "HeatRate",
  "axis": "Time",
  "statistics": ["mean", "time_weighted_mean", "max", "median", "std", "p90", "p95", "duration_above(5 kW)"],
  "cache_key": "summary:Q_coil:Time"
}
```

The cache key marks the summary identity. For the official CSV coil path,
runtime pages materialize `Q_coil` and `result.engres` records computed
mean/time_weighted_mean/max/min/median/std/pNN values plus
`duration_above(...)` duration values in seconds.

`time_weighted_mean` uses the trapezoidal Time-axis integral divided by the
elapsed seconds. `median` sorts the finite point values and averages the middle
pair for even-length series. `std` is the population standard deviation for
the materialized point values. Percentile names of the form `pNN` use the
current nearest-rank percentile kernel, so `p95` keeps the same behavior as the
current artifact contract.

`duration_above(<threshold>)` evaluates the threshold in the TimeSeries display
unit. When a threshold unit is supplied, the current runtime supports the same
W/kW conversion used by plot display conversion. The duration kernel uses
Time-axis seconds and linearly interpolates threshold crossings between
adjacent points.

## Scalar Statistic Calls

The summary kernels also have value-returning forms:

```eng partial
sample_total = sum(Q_coil)
hot_duration = duration_above(Q_coil, 5 kW)
print "hot={hot_duration: .1 s}"
```

`sum(series)` returns the sum of materialized sample values in
the series display unit. It does not multiply by the Time step. For HeatRate to
Energy conversion, use `integrate(series, over=Time)`.

The compatibility spelling `sum(series, axis=Time)` is still parsed. It
retains the existing integration-confusion warning; new code should use
`sum(series)` when a sample-value reduction is intended.

`duration_above(series, threshold)` returns `Duration [s]`. Both calls
execute natively for print/export evaluation and their bound scalar results are
stored in `typed_payload.numeric_values[].value`; they do not depend on
Python-produced input.

Explicit computed declarations use the same native path and store values in
their declared display units:

```eng partial
occupied: Duration [min] = duration_above(data.Q, 1.5 kW)
sample_total: HeatRate [W] = sum(data.Q)
energy: Energy [kWh] = integrate(data.Q, over=Time)
error: HeatRate [W] = rmse(data.Q, data.reference_Q)
```

For example, a 5,400-second duration is stored as `90 min`, and an
integration kernel result in joules is converted to `kWh` before it
enters `typed_payload.numeric_values` or formatted output. Explicit
`duration_above` and `rmse` calls receive the same argument and unit
diagnostics as inferred `=` bindings.

## Integration Metadata

`integrate(Q_coil, over=Time)` creates:

```json
{
  "binding": "E_coil",
  "source": "Q_coil",
  "input_quantity": "HeatRate",
  "over_axis": "Time",
  "result_quantity": "Energy"
}
```

This records the physical rule:

```text
integrate(HeatRate over Time) -> Energy
```

## HeatRate Sum Lint

This is intentionally warned:

```eng partial
E_bad = sum(Q_coil, axis=Time)
```

Diagnostic:

```text
W-STATS-SUM-001
  Summing HeatRate over Time does not produce Energy.
  Use integrate(<heat_rate>, over=Time) to compute Energy.
```

## Runtime Result

`result.engres` includes:

```text
object_store.timeseries_count
object_store.objects[].points
typed_payload.statistics
typed_payload.integrations
typed_payload.timeseries_fill
typed_payload.timeseries_quality
typed_payload.quality_results
typed_payload.time_alignments
```

The VM object store records TimeSeries objects as:

```json
{
  "name": "Q_coil",
  "kind": "timeseries",
  "type": "TimeSeries[Time] of HeatRate",
  "axis": "Time",
  "display_unit": "W"
}
```

For the official CSV path, the object also carries stable point values and the
typed payload records computed statistic values plus trapezoidal integration
metadata.

## Fill Missing Values And Policies

`fill missing <table>.<column>` always records an explicit TimeSeries fill
policy. `method = interpolate` also materializes a filled TimeSeries without
mutating the promoted source table:

```eng partial
filled = fill missing weather.wind_speed
with {
    method = interpolate
    expected_step = 1 h
    max_gap = 3 h
}
```

The supported methods are:

- `interpolate`: linearly interpolate finite samples inside the source range,
  subject to `expected_step` and optional `max_gap`; no extrapolation is used
- `record_only`: acknowledge a non-mutating policy and emit review metadata
  without filling values

Omitting `method` remains a conservative compatibility path equivalent to
`record_only`, but the compiler warns because the command does not change any
values. Unsupported methods, non-positive durations, duplicate `step` and
`expected_step`, and unbound interpolation outputs are compiler errors.
Editor quick fixes present `interpolate` and `record_only` as separate explicit
choices; neither is silently preferred for scientific data.

Runtime artifacts include a `typed_payload.timeseries_fill[]` record with the
source table/column, time column, method, expected step, max gap, missing count,
fillable count, filled count, skipped count, status, and source line. The
interpolated output is also available in the VM object store under the fill
binding name. That binding is a normal runtime TimeSeries input for plots and
`summarize`; a filled `HeatRate` series can also be passed to
`integrate(<filled>, over=Time)`. Runtime resolves the source quantity and
display unit from the materialized series, and converts rate-time integrals to
J before publishing them in `typed_payload.integrations[]` and
`report_spec.computed_integrations[]`. A `record_only` or deferred policy does
not create a TimeSeries, so downstream calculations remain unavailable rather
than consuming the unfilled source implicitly.

`typed_payload.timeseries_quality[]` summarizes the related coverage/fill
outcome with remaining missing count and a 0..1 quality score.
The same TimeSeries quality summary is also projected into
`typed_payload.quality_results[]` as a generic `timeseries_quality_result` bridge
record with target, subject, pass/warning/failure counts, score, status, reason,
and source line. That common array also carries non-TimeSeries validation,
schema-constraint, and expectation-suite quality results when a workflow emits
them, including row/field failure details for schema constraints.
`report_spec.quality_report`, the HTML Quality Report table, and the IDE Quality
inspector summarize those common quality results for report consumers.

## Native Alignment And Resampling

Bound `align` and `resample` commands materialize a new TimeSeries. The source
series remains unchanged, while the result binding can be consumed directly by
RMSE, statistics, report summaries, and plots. `align` defaults to `exact`
sampling; `resample` defaults to `linear` sampling. The compiler also accepts
`to` for `align` and `with` for `resample`, but `align ... with ...` and
`resample ... to ...` are the preferred target-series spellings:

```eng partial
aligned = align measured.T_zone with simulated.T_zone
resampled = resample measured.T_zone to simulated.T_zone
with {
    method = linear
    target_step = 1 h
    tolerance = 5 min
}
resampled_hourly = resample measured.T_zone by 1 h
```

The preferred native RMSE form consumes two materialized TimeSeries paths:

```eng partial
rmse_T = rmse(resampled, simulated.T_zone)
validate rmse_T < 5 K
```

Runtime samples the right series on the left series timestamps, converts
compatible display units, computes the root-mean-square error in process, and
publishes the result through `typed_payload.metrics[]` and
`report_spec.computed_metrics[]`. A bound result also appears in
`typed_payload.numeric_values[]`, converted to an explicit declaration's
display unit when one is present. No Python or external process supplies the
metric. `rmse resampled vs simulated.T_zone` remains a command-style alias
for the same call.

The sampling methods are:

- `exact`: emit a value only when a source timestamp matches the target within
  the optional `tolerance`; without an explicit tolerance, runtime uses a small
  axis-precision tolerance
- `nearest`: emit the closest source value within the source range and optional
  `tolerance`
- `linear`: interpolate between finite source points; it never extrapolates
  outside the source range

Without a step option, target-series forms use the target TimeSeries timestamps.
`resample <series> by <duration>` creates a regular axis over the finite source
range. `target_step` or `step` on `resample ... to ...` creates a regular axis
over the finite source/target overlap. A regular-axis request is limited to
1,000,000 points. Target timestamps that cannot be sampled are omitted, making
the result `partial`; a request with no output points is `unavailable`.

`method`, `tolerance`, `step`, and `target_step` are compiler-checked. Steps and
tolerances must be positive finite durations, `align` does not accept step
options, and conflicting step sources are rejected. Repeating the same
`resample ... by ...` step in a `with` block produces a redundancy warning.

The result binding is an executable TimeSeries with attached public result
fields:

```text
materialized              Bool: at least one output point exists
complete                  Bool: every target point was materialized
materialization_status    materialized, partial, unavailable, or not_requested
materialization_reason    human-readable outcome
alignment_status          source/target overlap status
step_status               nominal-step comparison status
strategy, method          operation and sampling method
source_count              source point count
reference_count           target/reference point count
target_count              requested target-axis point count
output_count              emitted point count
matched_count             exact source/reference timestamp matches
resample_step, tolerance  optional Duration values
```

There is intentionally no ambiguous `.status` member; use
`.materialization_status` or `.alignment_status`. Runtime artifacts include the
new TimeSeries in `object_store.objects[]` and a detailed record in
`typed_payload.time_alignments[]`, `report_spec.time_alignments[]`, and the HTML
Time Alignment section. Automatic pairwise records remain comparison metadata
with `strategy = auto_pairwise` and do not create output series.

## Deferred

Later versions will add:

```text
- broader TimeSeries expression execution
- statistics report cards
- richer output quantity handling for statistics such as `std(AbsoluteTemperature)`
```
