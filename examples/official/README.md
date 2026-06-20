# Official User-Test Examples

This is the release-facing example namespace. The portable package copies this
folder, the native IDE shows it first, and release smoke checks exercise these
paths before compatibility fixtures.

```text
01_csv_plot
  Typed CSV promote, unit-aware calculations, TimeSeries summary statistics,
  integrate metadata, line and histogram PlotSpec/SVG output, report, and
  standalone packaging smoke.

07_functions_imports
  Top-level execution, static file import, importable const values,
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

19_class_object
  Typed class declarations, object literals, nested object references, simple
  class validation blocks, metadata methods, immutable copy-with, field access
  metadata, and class/object report sections for the supported class object
  authoring surface. This is not runtime object dispatch.

20_multi_state_thermal
  Two-state source-equation thermal simulation with one `der(state)` equation
  per state, promoted CSV TimeSeries input binding, fixed-step RK4 execution,
  generated state TimeSeries, and report/plot artifacts. This is not a general
  nonlinear, DAE, or component-graph solver.

21_state_space_discrete
  Discrete two-state state-space simulation using top-level typed state/input
  blocks, `StateVector[...]`, `InputVector[...]`, operator declarations, and
  `next(x) eq A * x + B * u` lowering to generated state TimeSeries.

22_state_space_continuous
  Continuous two-state state-space simulation using typed state/input blocks,
  operator declarations, promoted CSV TimeSeries input binding, fixed-step RK4
  execution, and generated state TimeSeries.

23_thermal_component_assembly
  Thermal component templates instantiated inside a system block with
  `name = Component(...)`, `connect instance.port to instance.port`, generated
  connection equations, component-local boundary/equation seeds, and a square
  dense linear residual solve artifact. This is not a production nonlinear,
  dynamic, DAE, or broad multi-domain component solver.

24_linear_algebraic_thermal_node
  Source-to-solver smoke for a steady Thermal algebraic graph with
  system-local components, across and through boundary seeds, generated
  connection equations, named solution variables, residual norm, and
  largest-residual dense linear solve artifacts.

25_fixed_point_loop
  Source-to-solver smoke for `solve component_graph` with
  `solver = fixed_point`, tolerance/max-iteration/relaxation/initial options,
  named solution variables, residual norm, largest-residual artifacts, and
  fixed-point convergence metadata for a narrow linear ResidualGraph loop.

26_dynamic_component_room
  Source-to-solver smoke for `solve component_graph` with
  `solver = dynamic_component_semi_implicit_euler`, generated Thermal
  connection equations, a `der(port.T)` component-local equation, state and
  algebraic trajectories, and per-step algebraic diagnostics for a simple
  linear dynamic component assembly.

27_nonlinear_algebraic
  Source-to-solver smoke for `solve component_graph` with `solver = newton`,
  finite-difference Jacobian by default, convergence history, named solved
  variables, residual norm, and largest-residual artifacts for a dimensionless
  nonlinear scalar residual.

28_small_dae
  Source-to-solver smoke for `solve component_graph` with
  `solver = implicit_euler_dae`, source-derived state/algebraic split,
  algebraic initialization, identity mass-matrix fallback, state/algebraic
  trajectories, and per-step Newton diagnostics for a small dimensionless DAE.

29_delay_component_solver
  Source-to-solver smoke for `solve component_graph` with
  `solver = dynamic_component_explicit_euler`, a component-local
  `delay(signal, duration)` behavior expression, typed behavior graph RHS
  evaluation, state trajectory output, and integrated delay behavior artifacts.

30_predictor_component_solver
  Source-to-solver smoke for `solve component_graph` with
  `solver = dynamic_component_explicit_euler`, a typed deterministic
  `predictor(signal)` identity wrapper seed, behavior graph RHS evaluation,
  state trajectory output, and integrated Predictor contract artifacts.

31_external_behavior_solver
  Source-to-solver smoke for `solve component_graph` with
  `solver = dynamic_component_explicit_euler`, a typed deterministic
  `adapter(signal)` identity function wrapper seed, behavior graph RHS
  evaluation, state trajectory output, and integrated external behavior
  contract/profile artifacts.

32_small_thermal_fluid_loop
  Source-to-solver smoke for a constrained Thermal/Fluid[Water] algebraic
  residual graph. Generated connection equations, component-local boundary
  seeds, and simple pipe pressure/flow equations form a square dense linear solve.
  This is a pressure-based fluid seed, not a production hydraulic network package or broad multi-domain solver.
```

Compatibility regression examples live under `examples/compat`. Diagnostic and
data-quality fixtures live under `examples/diagnostics`. They are not
release-facing examples.

Internal implementation fixtures that are not user-facing release workflows live
under `examples/internal`, including solver/system, uncertainty, data-driven
modeling, domain/component, state-space, adaptive, and component-solver seeds.
