# Data-driven Modeling

v1.2 adds the first `eng.ml` preview surface. It is a deterministic seed for
user testing and artifact review, not a full production ML framework yet.

## Supported Forms

Use fast bindings after a data-derived `TimeSeries` such as `Q_coil`:

```eng
split = train_test_split(Q_coil, target=Q_coil, features=[T_supply, T_return, m_dot], test=0.5, seed=7)
reg_model = regression(split, algorithm=linear)
mlp_model = mlp(split, hidden=[4], epochs=20, seed=7)
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

The portable stdlib documents this preview surface at:

```text
stdlib/eng/ml.eng
```

## Metrics

Runtime artifacts include deterministic seed metrics:

```text
RMSE
MAE
R2
train_count
test_count
leakage_status
model_card
```

The metric values are computed from the promoted CSV path through the
`TimeSeries` test slice. The regression and MLP implementations are preview
seeds, so use them to test artifact flow and model review UX rather than to make
engineering decisions.

## Plots

Request a parity plot:

```eng
return report {
    plot parity(reg_eval) {
        title = "Regression parity"
    }
}
```

Request a residual plot:

```eng
return report {
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
  model_card
  parity_points
  residual_points
```

`review.json` includes `ml_info`. `report_spec.json` includes `ml`.
`report.html` includes an ML Models table.

## Official Example

Run:

```bat
.\target\debug\eng.exe run examples\official\05_data_driven_modeling\main.eng --entry main
```

or open this file in the native IDE:

```text
examples/official/05_data_driven_modeling/main.eng
```
