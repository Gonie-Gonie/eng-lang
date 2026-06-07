# v1.2 Data-driven Modeling Gate

This page tracks the concrete gate for treating the v1.2 data-driven modeling
surface as more than a seed. It does not by itself make v1.2 release-supported;
the feature maturity matrix remains authoritative until the release target is
changed.

## Current Scope

The current v1.2 scope is intentionally narrow:

- Data source: a promoted CSV table with a derived `TimeSeries` target such as
  `Q_coil`.
- Split: `train_test_split(Q, target=Q, features=[...], test=<fraction>,
  seed=<integer>)`.
- Models: deterministic `regression(split, algorithm=linear)` and deterministic
  one-hidden-layer-or-more `mlp(split, hidden=[...], epochs=n, seed=n)`.
- Evaluation: `evaluate(model, split=split)` / `metrics(model, split=split)`.
- Review helpers: `model_card(model)` and `leakage_lint(split)`.
- Plots: one PlotSpec per run, with parity scatter and residual bar paths
  covered by separate official examples.

## Completed On Main

- [x] `stdlib/eng/ml.eng` documents the preview surface.
- [x] Compiler records `MlInfo` for split, regression, MLP/ANN, metrics,
  model-card, and leakage-lint bindings.
- [x] Compiler semantic types are recorded for `TrainTestSplit`,
  `Model[Regression]`, `Model[MLP]`, `ModelMetrics`, `ModelCard`, and
  `LeakageLint`.
- [x] Compiler source diagnostics validate the staged chain:
  TimeSeries -> TrainTestSplit -> Model -> evaluation/model-card.
- [x] Compiler argument diagnostics validate target/features/test fraction,
  supported regression algorithm, MLP hidden layers, epochs, and integer seeds.
- [x] Runtime builds a feature matrix from the source table behind the promoted
  TimeSeries.
- [x] Runtime trains deterministic standardized linear regression and a small
  deterministic tanh MLP.
- [x] Runtime records train/test counts, RMSE, MAE, R2, coefficients,
  intercept, loss history, model cards, leakage findings, parity points, and
  residual points.
- [x] Report/review/result artifacts expose ML metadata through `review.json`,
  `report_spec.json`, `result.engres`, and `report.html`.
- [x] Native IDE Runtime Summary shows ML metrics, coefficients, leakage
  status, and loss history from `result.engres`.
- [x] Official parity example:
  `examples/official/05_data_driven_modeling/main.eng`.
- [x] Official residual example:
  `examples/official/05_data_driven_modeling/residuals.eng`.
- [x] CLI `eng test examples` covers ML source diagnostics, argument
  diagnostics, parity plot artifacts, residual plot artifacts, and official
  ML result metadata.
- [x] `release-check` passes with the v1.2 examples included in package smoke
  and IDE smoke.

## Remaining Before Support Claim

- [ ] Decide whether v1.2 should be promoted from Experimental to Preview or
  Supported for a release target.
- [ ] Write release notes for the exact public scope and limitations.
- [ ] Keep multi-plot reports deferred unless the PlotSpec/report contract is
  extended intentionally; current parity/residual coverage uses separate runs.
- [ ] Keep cross-validation, hyperparameter search, model persistence, and
  general ML package import semantics explicitly deferred.
- [ ] Keep data-column existence and numeric-feature checks in runtime leakage
  lint/artifacts, not in the compiler source-order check.

## Diagnostic Contract

```text
E-ML-SOURCE-001  missing or unknown ML source/split/model reference
E-ML-SOURCE-002  referenced binding exists but has the wrong semantic type
E-ML-ARGS-001    missing or malformed required ML argument
E-ML-ARGS-002    invalid numeric/list ML argument
E-ML-ARGS-003    unsupported ML option for the current v1.2 scope
```

## Verification

```bat
.\dev.bat release-check
target\debug\eng.exe run examples\official\05_data_driven_modeling\main.eng --entry main
target\debug\eng.exe run examples\official\05_data_driven_modeling\residuals.eng --entry main
target\debug\eng.exe check examples\05_error_messages\missing_ml_source.eng --review
target\debug\eng.exe check examples\05_error_messages\invalid_ml_arguments.eng --review
```
