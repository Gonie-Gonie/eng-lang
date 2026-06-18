# Internal 05 Data-driven Modeling

Purpose:

```text
Exercise the internal data-driven modeling path from typed CSV promotion to
deterministic model metrics, model cards, leakage lint metadata, and plots.
```

Language features shown:

```text
- typed CSV schema with DateTime index and constraints
- TimeSeries HeatRate derivation from promoted columns
- train_test_split(...) with target, feature list, test fraction, and seed
- regression(...), mlp(...), evaluate(...), model_card(...), leakage_lint(...)
- parity and residual plot commands with with { title = ... }
```

Run:

```bat
target\debug\eng.exe run examples\internal\05_data_driven_modeling\main.eng --save-artifacts
target\debug\eng.exe run examples\internal\05_data_driven_modeling\residuals.eng --save-artifacts
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
- deterministic small-sample modeling artifact path, not a production ML stack
- linear regression and small MLP metadata/training path only
- feature engineering, model persistence, and broad estimator selection are deferred
```

Related docs:

```text
docs/guide/data_driven_modeling.md
docs/guide/report_review.md
docs/current/feature_maturity_matrix.md
```
