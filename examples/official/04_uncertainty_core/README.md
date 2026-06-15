# Official 04 Uncertainty Core

Purpose:

```text
Show the current deterministic uncertainty metadata path, including measured,
interval, distribution, ensemble, propagation, and histogram artifacts.
```

Language features shown:

```text
- root args block with a simple String default
- measured(...), interval(...), normal(...), uniform(...), ensemble(...)
- propagate(..., method=linear, scale=..., offset=...)
- report show entries for uncertainty values
- plot distribution(...) with a with { title = ... } option block
```

Run:

```bat
target\debug\eng.exe run examples\official\04_uncertainty_core\main.eng --save-artifacts
```

Expected artifacts:

```text
build/result/result.engres
build/result/report_spec.json
build/result/report.html
build/result/plots/plot_spec.json
build/result/plots/timeseries.svg
```

Limitations:

```text
- deterministic review samples, not a production stochastic engine
- linear propagation transform metadata only
- no general Monte Carlo/Jacobian uncertainty solver claim
```

Related docs:

```text
docs/guide/uncertainty.md
docs/guide/report_review.md
docs/current/feature_maturity_matrix.md
```
