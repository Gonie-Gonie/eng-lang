# TimeSeries Statistics Guide

v0.5-preview adds the first TimeSeries and statistics path. The current v1.0
hardening path computes numeric values for the official CSV coil example while
keeping broader TimeSeries expression execution deferred.

## Example

```eng partial
script main(args: Args) -> Report {
    sensor = promote csv "data/sensor.csv" as SensorData
    cp = 4180 J/kg/K
    Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
    E_coil = integrate(Q_coil, over=Time)

    return report {
        summarize Q_coil by [mean, max, p95, duration_above(5 kW)]
        show E_coil
        plot Q_coil over Time
    }
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

`summarize Q_coil by [mean, max, p95, duration_above(5 kW)]` creates:

```json
{
  "source": "Q_coil",
  "source_type": "TimeSeries[Time] of HeatRate",
  "quantity_kind": "HeatRate",
  "axis": "Time",
  "statistics": ["mean", "max", "p95", "duration_above(5 kW)"],
  "cache_key": "summary:Q_coil:Time"
}
```

The cache key marks the summary identity. For the official CSV coil path,
runtime pages materialize `Q_coil` and `result.engres` records computed
mean/max/p95 values plus `duration_above(...)` duration values in seconds.

`duration_above(<threshold>)` evaluates the threshold in the TimeSeries display
unit. When a threshold unit is supplied, the v1.0 hardening path supports the
same W/kW conversion used by plot display conversion. The duration kernel uses
Time-axis seconds and linearly interpolates threshold crossings between adjacent
points.

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

## Deferred

Later versions will add:

```text
- time-weighted mean
- broader TimeSeries expression execution
- non-uniform time handling
- statistics report cards
```
