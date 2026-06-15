# Official User-Test Examples

This is the release-facing example namespace. The portable package copies this
folder, the native IDE shows it first, and release smoke checks exercise these
paths before compatibility fixtures.

```text
01_csv_plot
  Typed CSV promote, unit-aware calculations, TimeSeries summary statistics,
  integrate metadata, line and histogram PlotSpec/SVG output, report, and
  standalone packaging smoke.

02_simple_system
  Minimal physical system/equation surface with residual metadata, solver-plan
  metadata, and fixed-step ODE preview output.

03_integrated_hvac
  Combined user-test path for Args, CSV policies, missing interpolation,
  statistics, integrate, PlotSpec/report, and simple system solver preview.

04_uncertainty_core
  Experimental uncertainty-track path for measured values, intervals,
  deterministic distributions/ensembles, source and argument diagnostics,
  propagation metadata, and in-report histogram output.

05_data_driven_modeling
  Experimental data-driven modeling track path for train/test split, linear
  regression, basic MLP, source and argument validation diagnostics,
  RMSE/MAE/R2 metrics, leakage lint, model card, parity plot output, and
  residual plot output via `residuals.eng`.

06_domain_port
  Experimental domain/component track path for user-defined domains,
  across/through variables, conservation metadata, component ports,
  structured generic parameters, domain-compatible connections, contract
  diagnostics, invalid connection diagnostics, metadata-only assembly
  connection sets, generated connection equations, equation/unknown counts, and
  residual graph placeholders.

07_functions_imports
  Preview top-level execution, static file import, importable const values,
  function-local bindings, unit-checked parameters, dimension-checked return
  expressions, function-call inference, CLI print, and explicit summary CSV
  export.

08_print_export_summary
  Mini scalar summary path for top-level `args`, reusable `const`,
  unit-aware print interpolation, and explicit one-row `export summary to csv`
  output with requested display units.

09_command_where_with
  Parenthesis-light command-style built-in workflow verbs, scoped `where`
  locals, `with` option blocks, command-style statistics/integration,
  print/export output, and plot display options.

10_path_policy
  Typed path arguments, pure path helpers (`join`, `parent`, `stem`,
  `extension`), runtime `exists` checks, and review/result/report-spec
  environment dependency provenance.

11_read_only_io
  Read-only text/json/toml inputs, source-hash provenance, and a multi-source
  workflow that combines typed CSV data with auxiliary configuration files.

12_write_output_manifest
  Explicit summary CSV export, write text/json outputs, overwrite policy, and
  output manifest generation for produced files.

13_file_operations
  Explicit copy/move/delete filesystem mutation seed, confirmation metadata,
  generated-output mutation boundaries, and output manifest records for file
  operations.

14_run_log
  Structured runtime message seed with print plus `log info/debug/warn/error`
  and generated `run_log.json` artifacts.

15_process_result
  External process seed with `run command`, `ProcessResult`, and generated
  `process_results.json` artifacts.

16_test_assert_golden
  Local workflow verification seed with named `test` blocks, unit-aware
  `assert` statements, golden artifact comparison, and generated
  `test_results.json` artifacts.

17_measured_vs_simulated
  Integrated typed data plus simulation workflow with weather/measured CSV
  promotion, fixed-step one-state thermal simulation output as `sim.T_zone`,
  RMSE calculation, threshold validation, time-alignment metadata, and a
  measured/simulated multi-series PlotSpec.

18_state_space_preview
  Typed StateVector/InputVector/OutputVector and LinearOperator metadata for
  the state-space preview surface. This is a review/IDE metadata seed, not a
  general matrix solver.

19_class_object_preview
  Typed class declarations, object literals, nested object references, simple
  class validation blocks, metadata methods, immutable copy-with, field access
  metadata, and class/object report sections for the class object preview
  surface. This is a review/IDE metadata seed, not runtime object dispatch.
```

Top-level numbered examples remain for compatibility and focused regression
tests. Diagnostic and data-quality fixtures live in their own top-level
folders; they are not the first user-facing examples.
