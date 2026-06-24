# Examples

Examples are organized by user journey and release role. Open the core workflow
examples first when learning EngLang.

## Core Workflow Examples

These are the first-user examples and the package smoke baseline. They show the
semantic engineering workflow: data boundary, units, TimeSeries, artifacts,
explicit side effects, and local verification.

```text
official/01_csv_plot
  Typed CSV promote, HeatRate calculation, TimeSeries statistics, integration,
  PlotSpec/SVG, report, review, and standalone packaging path.

official/07_functions_imports
  Top-level execution, importable const, pure scalar functions, relative file
  imports, function-call inference, CLI print, and summary CSV export.

official/08_print_export_summary
  Scalar summary fixture for args, const, unit-aware print interpolation, and
  explicit one-row summary CSV export.

official/09_command_where_with
  Parenthesis-light built-in workflow commands, scoped where locals, with
  option blocks, statistics/integration, print/export output, and plot display
  options.

official/10_path_policy
  Typed path arguments, pure path helpers, runtime exists checks, and
  provenance-visible environment dependency metadata.

official/11_read_only_io
  Read-only text/json/toml inputs with source-hash provenance combined with
  typed CSV data.

official/12_write_output_manifest
  Explicit summary CSV export, write text/json outputs, overwrite policy, and
  output manifest generation.

official/14_run_log
  Structured runtime messages and generated run_log.json artifacts.

official/15_process_result
  External process seed with run command, ProcessResult, and
  process_results.json artifacts.

official/16_test_assert_golden
  Local workflow verification with named test blocks, unit-aware assert
  statements, golden artifact comparison, and test_results.json.
```

## Additional Review Examples

These are useful after the core workflow. They are still review/artifact
oriented, not a broad runtime platform claim.

```text
official/13_file_operations
  Explicit generated-output file mutation boundaries and output manifest
  records.

official/19_class_object
  Typed engineering objects with fields/defaults, validation, metadata methods,
  immutable copy-with, object summaries, diagnostics, and IDE/LSP metadata.
```

## Composite Workflow Examples

`examples/workflows` contains hybrid workflow skeletons that demonstrate
generic adapter boundaries. They are not first-user public package examples and
they are not domain-specific product claims.

```text
workflows/01_weather_api_to_standard_file_hybrid
  API data, typed station-map schema, fixture read, explicit process boundary,
  generated standard text artifact, and review/report path.

workflows/02_external_simulation_surrogate_hybrid
  Design sample table, typed result and prediction tables, explicit per-case
  process boundaries, case manifests, model-card/metrics artifacts, generated
  workflow summary, and DB side-effect manifest.

workflows/03_uncertain_sensor_report
  Typed sensor data, pointwise measured uncertainty metadata, duration summary,
  confidence-band PlotSpec, and report/review artifact path.
```

These examples should grow generic module contracts such as `eng.net`,
`eng.cache`, `eng.case`, `eng.db`, and `eng.model` before any domain-specific
weather, EPW, or solver adapter is treated as core language behavior.

## Advanced Solver Smoke Fixtures

The following directories live under `examples/advanced_solver`. Treat them as
advanced solver smoke fixtures, not the first-user product walkthrough.

```text
advanced_solver/20_multi_state_thermal
advanced_solver/21_state_space_discrete
advanced_solver/22_state_space_continuous
advanced_solver/23_thermal_component_assembly
advanced_solver/24_linear_algebraic_thermal_node
advanced_solver/25_fixed_point_loop
advanced_solver/26_dynamic_component_room
advanced_solver/27_nonlinear_algebraic
advanced_solver/28_small_dae
advanced_solver/29_delay_component_solver
advanced_solver/30_predictor_component_solver
advanced_solver/31_external_behavior_solver
advanced_solver/32_small_thermal_fluid_loop
advanced_solver/33_unit_parameterized_wall
advanced_solver/34_three_state_source_ode
```

These examples are valuable regression coverage. Their public meaning is
limited to typed TimeSeries and reviewable residual/convergence artifacts for
the documented narrow scopes. They do not claim a general nonlinear, DAE,
adaptive, behavior graph, or production multi-domain solver.

Package and IDE smoke paths may still include this directory for regression
coverage. It is not public tutorial content.

## Internal Regression Fixtures

`examples/internal` contains implementation fixtures checked by development
smoke paths. They are not user-facing release workflows.

Open these when working on a specific internal track:

```text
internal/02_simple_system
internal/03_integrated_hvac
internal/04_uncertainty_core
internal/05_data_driven_modeling
internal/06_domain_port
internal/17_measured_vs_simulated
internal/18_state_space_metadata
internal/20_multi_state_thermal
internal/21_thermal_component_assembly
internal/22_component_boundary_solve
internal/22_multi_domain_boundary_solve
internal/23_component_boundary_singular
internal/24_component_boundary_overdetermined
internal/25_component_behavior_nodes
internal/26_state_space_discrete
internal/27_adaptive_heun_thermal
internal/28_adaptive_state_space
```

## Compatibility Regression Examples

`examples/compat` keeps older focused examples alive as regression coverage.
They are intentionally not the first user-facing namespace and are not copied
as first-user walkthroughs.

```text
compat/01_units
compat/02_csv_plot
compat/04_plotting
compat/06_simple_system
```

## Diagnostic Fixtures

`examples/diagnostics/error_messages` contains examples that are expected to
produce specific diagnostics or warnings. Use them when changing parser,
semantic, unit, workflow, equation, domain, port, or connection diagnostics.

`examples/diagnostics/data_quality` contains CSV policy and runtime
data-quality fixtures. Some files intentionally record parse failures,
interpolation, constraint violations, or unsupported conversion failures in
generated artifacts.

## Scratch Files

The native IDE may create `examples/scratch/*.eng` during manual testing. Those
files are user work and are not part of the release contract unless explicitly
added and documented.
