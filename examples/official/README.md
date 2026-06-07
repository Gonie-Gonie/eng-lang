# Official User-Test Examples

This is the release-facing example namespace. The portable package copies this
folder, the native IDE shows it first, and release smoke checks exercise these
paths before compatibility fixtures.

```text
01_csv_plot
  Typed CSV promote, unit-aware calculations, TimeSeries summary statistics,
  integrate metadata, PlotSpec, SVG, report, and standalone packaging smoke.

02_simple_system
  Minimal physical system/equation surface with residual metadata, solver-plan
  metadata, and fixed-step ODE preview output.

03_integrated_hvac
  Combined user-test path for Args, CSV policies, missing interpolation,
  statistics, integrate, PlotSpec/report, and simple system solver preview.

04_uncertainty_core
  Experimental v1.1 uncertainty path for measured values, intervals,
  deterministic distributions/ensembles, propagation metadata, and in-report
  histogram output.

05_data_driven_modeling
  Experimental v1.2 data-driven modeling path for train/test split, linear
  regression, basic MLP, source and argument validation diagnostics,
  RMSE/MAE/R2 metrics, leakage lint, model card, parity plot output, and
  residual plot output via `residuals.eng`.
```

Top-level numbered examples remain for compatibility and focused regression
tests. Diagnostic and data-quality fixtures live in their own top-level
folders; they are not the first user-facing examples.
