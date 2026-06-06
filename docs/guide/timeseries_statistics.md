# TimeSeries Statistics Guide

v0.5-preview adds the first TimeSeries and statistics metadata path.

The current implementation is a typed seed, not a full numeric statistics engine. It records enough information for review artifacts, future IDE completion, and the runtime result payload.

## Example

```eng
script main(args: Args) -> Report {
    sensor = promote csv "data/sensor.csv" as SensorData
    cp = 4180 J/kg/K
    Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
    E_coil = integrate(Q_coil, over=Time)

    return report {
        summarize Q_coil by [mean, max, p95]
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

`summarize Q_coil by [mean, max, p95]` creates:

```json
{
  "source": "Q_coil",
  "source_type": "TimeSeries[Time] of HeatRate",
  "quantity_kind": "HeatRate",
  "axis": "Time",
  "statistics": ["mean", "max", "p95"],
  "cache_key": "summary:Q_coil:Time"
}
```

The cache key marks a lazy summary. v0.5 does not compute numeric mean/max/p95 values yet.

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

```eng
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

## Deferred

Later versions will add:

```text
- row-level numeric TimeSeries values
- time-weighted mean
- min/max/p95 numeric kernels
- summary materialization
- non-uniform time handling
- statistics values in report cards
```
