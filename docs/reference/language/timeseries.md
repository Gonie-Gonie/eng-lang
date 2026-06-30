# TimeSeries Statistics Guide

The TimeSeries/statistics path computes numeric values for the official CSV
coil example while keeping broader TimeSeries expression execution deferred.

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

Promoted CSV tables also expose the schema index as an axis seed:

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

## Fill Missing Metadata

`fill missing <table>.<column>` records an explicit TimeSeries fill policy and,
with `method = interpolate`, materializes a filled TimeSeries without mutating
the promoted source table:

```eng partial
filled = fill missing weather.wind_speed
with {
    method = interpolate
    expected_step = 1 h
    max_gap = 3 h
}
```

Runtime artifacts include a `typed_payload.timeseries_fill[]` record with the
source table/column, time column, method, expected step, max gap, missing count,
fillable count, filled count, skipped count, status, and source line. The
interpolated output is also available in the VM object store under the fill
binding name. `typed_payload.timeseries_quality[]` summarizes the related
coverage/fill outcome with remaining missing count and a 0..1 quality score.
The same TimeSeries quality summary is also projected into
`typed_payload.quality_results[]` as a generic `timeseries_quality_result` bridge
record with target, subject, pass/warning/failure counts, score, status, reason,
and source line. That common array also carries non-TimeSeries validation,
schema-constraint, and expectation-suite quality results when a workflow emits
them.

## Alignment And Resampling Hooks

`align <series> with <series>` and `resample <series> to <series>` record
reviewable TimeSeries alignment intent without silently mutating source data:

```eng partial
aligned = align measured.T_zone with simulated.T_zone
resampled = resample measured.T_zone to simulated.T_zone
with {
    method = linear
    target_step = 1 h
    tolerance = 5 min
}
```

Runtime artifacts include `typed_payload.time_alignments[]` records with binding,
left/right series, strategy (`align`, `resample`, or `auto_pairwise`), method,
optional resample step/tolerance, overlap, step status, and matched counts.

## Deferred

Later versions will add:

```text
- broader TimeSeries expression execution
- statistics report cards
- richer output quantity handling for statistics such as `std(AbsoluteTemperature)`
```
