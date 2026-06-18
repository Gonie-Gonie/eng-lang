# Data-driven Modeling Track

This guide documents the current internal data-driven modeling track. It is
a deterministic path for user testing and artifact review, not a full
production ML framework yet.

## Supported Forms

Use fast bindings after a data-derived `TimeSeries` such as `Q_coil`:

```eng
cp = 4180 J/kg/K
Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
split = train_test_split(Q_coil, target=Q_coil, features=[T_supply, T_return, m_dot], test=0.5, seed=7)
reg_model = regression(split, algorithm=linear)
mlp_model = mlp(split, hidden=[4], epochs=80, seed=7)
reg_eval = evaluate(reg_model, split=split)
mlp_eval = evaluate(mlp_model, split=split)
reg_card = model_card(reg_model)
leakage = leakage_lint(split)
```

The compiler records these semantic types:

```text
TrainTestSplit
Model[Regression]
Model[MLP]
ModelMetrics
ModelCard
LeakageLint
```

The portable stdlib documents this internal surface at:

```text
stdlib/eng/ml.eng
```

## Source And Argument Validation

The compiler checks the staged ML binding chain before runtime:

```text
train_test_split(Q_coil, ...)      Q_coil must be a prior TimeSeries binding
regression(split, ...)             split must be a prior TrainTestSplit binding
mlp(split, ...) / ann(split, ...)  split must be a prior TrainTestSplit binding
evaluate(model, split=split)       model must be a prior Model[...] binding
model_card(model)                  model must be a prior Model[...] binding
leakage_lint(split)                split must be a prior TrainTestSplit binding
```

Unknown or missing ML references produce `E-ML-SOURCE-001`. References with the
wrong semantic type produce `E-ML-SOURCE-002`.

The compiler also checks the current data-driven modeling track argument contract:

```text
train_test_split(...)  requires target=<TimeSeriesName>, features=[...], and test=<fraction>
regression(...)        supports algorithm=linear
mlp(...) / ann(...)    requires hidden=[positive integers] and epochs=<positive integer>
seed=...               must be a non-negative integer when present
```

Missing or malformed required ML arguments produce `E-ML-ARGS-001` or
`E-ML-ARGS-002`. Unsupported ML options produce `E-ML-ARGS-003`.

## Runtime Semantics

The current runtime builds a real feature matrix from the promoted CSV table
behind the source `TimeSeries`.

- `train_test_split(...)` resolves the source series, source table, target, and
  feature columns, then records deterministic train/test counts.
- `leakage_lint(...)` records concrete findings such as target-in-features,
  missing feature columns, non-numeric features, or index features that require
  temporal review.
- `regression(split, algorithm=linear)` trains a deterministic standardized
  linear model and exports original-unit coefficients and intercept.
- `mlp(split, hidden=[n], epochs=m, seed=s)` trains a small deterministic
  one-hidden-layer tanh MLP from the same feature matrix.
- `evaluate(model, split=split)` carries forward metrics, parity points,
  residual points, coefficients, and loss history.
- `model_card(model)` carries forward the model review summary.

Runtime artifacts include:

```text
RMSE
MAE
R2
train_count
test_count
leakage_status
leakage_findings
coefficients
intercept
loss_history
model_card
```

Metric values are computed from the promoted CSV path through the `TimeSeries`
test slice. This path is deterministic and useful for artifact review,
plotting, IDE inspection, and leakage-test workflows. It is not yet a
production ML framework with cross-validation, hyperparameter search, or model
persistence.

## Plots

Request a parity plot:

```eng
report {
    plot parity(reg_eval) {
        title = "Regression parity"
    }
}
```

Request a residual plot:

```eng
report {
    plot residuals(reg_eval) {
        title = "Regression residuals"
    }
}
```

Parity plots use `scatter`. Residual plots use `bar`.

## Runtime Contract

`result.engres` includes:

```text
typed_payload.ml
  binding
  kind
  source
  target
  features
  algorithm
  hidden_layers
  epochs
  status
  train_count
  test_count
  rmse / mae / r2
  leakage_status
  leakage_findings
  coefficients
  intercept
  loss_history
  model_card
  parity_points
  residual_points
```

`review.json` includes `ml_info`. `report_spec.json` includes `ml`.
`report.html` includes an ML Models table.

## Official Example

Run:

```bat
.\target\debug\eng.exe run examples\internal\05_data_driven_modeling\main.eng --save-artifacts
```

or open this file in the Tauri IDE:

```text
examples/internal/05_data_driven_modeling/main.eng
```

The main example renders the parity scatter plot. The residual bar plot path is
available as:

```bat
.\target\debug\eng.exe run examples\internal\05_data_driven_modeling\residuals.eng --save-artifacts
```

or in the Tauri IDE:

```text
examples/internal/05_data_driven_modeling/residuals.eng
```
