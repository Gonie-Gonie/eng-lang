# Internal 04 Uncertainty Core

Purpose:

```text
Exercise the internal deterministic uncertainty metadata path, including
measured, interval, distribution, ensemble, propagation, and histogram
artifacts.
```

Language features shown:

```text
- root args block with a simple String default
- measured(..., std=...), measured(..., error=...), interval(...),
  normal(...), uniform(...), ensemble(...)
- propagate(..., method=linear, scale=..., offset=...)
- deterministic arithmetic propagation from an uncertain source plus a scalar
- with { uncertainty = linear } review policy metadata
- report show entries for uncertainty values
- plot distribution(...) with a with { title = ... } option block
```

Run:

```bat
target\debug\eng.exe run examples\internal\04_uncertainty_core\main.eng --save-artifacts
```

Expected artifacts:

```text
build/result/result.engres
build/result/review.json
build/result/report_spec.json
build/result/report.html
build/result/plots/plot_spec.json
build/result/plots/timeseries.svg
```

Limitations:

```text
- deterministic review samples, not a production stochastic engine
- Internal linear propagation transform metadata and narrow arithmetic
  propagation
- no general Monte Carlo/Jacobian uncertainty solver claim
```

Related docs:

```text
docs/guide/uncertainty.md
docs/guide/report_review.md
docs/current/feature_maturity_matrix.md
```
