# Uncertainty Core

v1.1 adds the first uncertainty surface for user testing. The implementation is
deterministic and review-oriented: every uncertainty expression is recorded in
compiler metadata, runtime result JSON, report spec JSON, HTML reports, and the
plot path when a distribution plot is requested.

## Supported Forms

Use fast bindings inside a `script`:

```eng
T_supply_meas = measured(12 degC, std=0.2 K)
T_return_band = interval(20 degC, 24 degC)
Q_coil_dist = normal(mean=5 kW, std=0.8 kW, samples=31)
Q_aux_band = uniform(0.3 kW, 0.7 kW, samples=21)
Q_coil_ensemble = ensemble(Q_coil_dist, samples=31)
Q_total_unc = propagate(Q_coil_dist, method=linear, scale=1.08, offset=0.4 kW)
```

The compiler records these semantic types:

```text
Measured[T]
Interval[T]
Distribution[T]
Ensemble[T]
```

`T` is inferred from the name and unit seed. For example, `Q_coil_dist` with
`kW` is treated as `Distribution[HeatRate]`.

## Runtime Semantics

The v1.1 implementation now materializes deterministic sample sets:

- `measured(value, std=...)` records the measured value and creates a small
  deterministic normal sample when a standard deviation is supplied.
- `interval(lower, upper)` records lower, midpoint, and upper samples.
- `normal(mean=..., std=..., samples=n)` uses deterministic quantile samples,
  so the same source always produces the same summary and histogram.
- `uniform(lower, upper, samples=n)` samples evenly inside the declared band.
- `ensemble(source, samples=n)` deterministically resamples a prior uncertainty.
- `propagate(source, method=linear, scale=..., offset=...)` resamples the source
  and applies the declared linear transform.

`ensemble(...)` and `propagate(...)` require the first argument to be an
uncertainty binding that was defined earlier in the same semantic pass. Unknown
sources produce `E-UNC-SOURCE-001`; deterministic bindings such as `Q = 5 kW`
produce `E-UNC-SOURCE-002` when used as uncertainty sources.

Each runtime uncertainty includes mean, standard deviation, lower/upper bounds,
`p05`, `p50`, `p95`, `distribution`, `method`, optional `scale`/`offset`
transform metadata, sample count, propagation count, and the generated sample
vector.

## Distribution Plot

Request a histogram in the report block:

```eng
return report {
    plot distribution(Q_coil_dist) {
        title = "Coil heat-rate uncertainty"
    }
}
```

The runtime writes a histogram `PlotSpec` and SVG under the normal build result
folder. In the native IDE, run the file and use the plot/report artifact buttons.

## Runtime Contract

`result.engres` includes:

```text
typed_payload.uncertainties
  binding
  kind
  quantity_kind
  display_unit
  source
  distribution
  method
  scale/offset
  mean/stddev/lower/upper
  p05/p50/p95
  sample_count
  samples
  status
```

`review.json` includes `uncertainty_info` with declared transform strings.
`result.engres` and runtime-updated `report_spec.json` include numeric
`scale`/`offset` values when they were declared. `report.html` includes an
Uncertainty table with a Transform column.

The current propagation is deterministic and supports explicit linear
scale/offset transforms with source validation. It is still not a full
Jacobian or Monte Carlo propagation engine, but it is concrete enough for
user-facing artifact review, histogram testing, and IDE inspection.

## Official Example

Run:

```bat
.\target\debug\eng.exe run examples\official\04_uncertainty_core\main.eng --entry main
```

or open this file in the native IDE:

```text
examples/official/04_uncertainty_core/main.eng
```
