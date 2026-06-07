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
Q_coil_ensemble = ensemble(Q_coil_dist, samples=31)
Q_total_unc = propagate(Q_coil_dist, method=linear)
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
  mean/stddev/lower/upper
  sample_count
  samples
  status
```

`review.json` includes `uncertainty_info`. `report_spec.json` includes
`uncertainty`. `report.html` includes an Uncertainty table.

The current propagation is a deterministic seed. It follows source bindings and
copies/resamples source samples where possible, but it is not yet a full
Jacobian or Monte Carlo propagation engine.

## Official Example

Run:

```bat
.\dev.bat run examples\official\04_uncertainty_core\main.eng
```

or open this file in the native IDE:

```text
examples/official/04_uncertainty_core/main.eng
```
