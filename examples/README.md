# Examples

This folder is split by release role. Open `examples/official` first when using
EngLang as a user or release tester.

## Official User-Test Examples

`examples/official` is the release-facing namespace. These examples are copied
into portable packages, shown first in the native IDE, and exercised by release
smoke checks.

```text
official/01_csv_plot
  Supported CSV promote, HeatRate statistics, integration, PlotSpec, SVG,
  report, raw-value histogram variant, and standalone packaging path.

official/07_functions_imports
  Top-level execution, importable const, function-local binding,
  relative file imports, function-call inference, CLI print, and summary CSV
  export.

official/08_print_export_summary
  Mini scalar summary fixture for top-level args, reusable const, unit-aware
  print interpolation, and explicit one-row summary CSV export.

official/09_command_where_with
  Parenthesis-light command-style built-ins, scoped where locals, with option
  blocks, command-style statistics/integration, print/export output, and plot
  display options.

official/10_path_policy
  Typed path arguments, pure path helpers, runtime exists checks, and
  provenance-visible environment dependency metadata.

official/11_read_only_io
  Read-only text/json/toml inputs with source-hash provenance, combined with
  typed CSV data in one workflow.

official/12_write_output_manifest
  Explicit summary CSV export, write text/json outputs, overwrite policy, and
  output manifest generation for produced files.

official/13_file_operations
  Explicit copy/move/delete filesystem mutation seed, confirmation metadata,
  generated-output mutation boundaries, and output manifest records for file
  operations.

official/14_run_log
  Structured runtime message seed with print plus `log info/debug/warn/error`
  and generated `run_log.json` artifacts.

official/15_process_result
  External process seed with `run command`, `ProcessResult`, and generated
  `process_results.json` artifacts.

official/16_test_assert_golden
  Local workflow verification seed with named `test` blocks, unit-aware
  `assert` statements, golden artifact comparison, and generated
  `test_results.json` artifacts.

official/19_class_object
  Supported class/domain-object authoring fixture with typed fields, defaults,
  object literals, validation, field access metadata, immutable copy-with, and
  class/object artifacts.

official/20_multi_state_thermal
  Supported two-state source-equation thermal simulation with one
  `der(state)` equation per state, promoted CSV TimeSeries input binding,
  fixed-step RK4 execution, generated sim.T_air/sim.T_wall TimeSeries, and
  report/plot artifacts.

official/21_state_space_discrete
  Supported typed-block discrete state-space example with `StateVector[...]`,
  `InputVector[...]`, operator declarations, `next(x) eq A * x + B * u`, and
  generated sim.T_air/sim.T_wall TimeSeries.

official/22_state_space_continuous
  Supported typed-block continuous state-space example with promoted CSV
  TimeSeries input binding, fixed-step RK4 execution, and generated
  sim.T_air/sim.T_wall TimeSeries.

official/23_thermal_component_assembly
  Supported system-local component instance example with
  `name = Component(...)`, `connect instance.port to instance.port`, generated
  connection equations, one boundary seed, one direct component-local equation,
  and a square dense linear residual solve artifact for a constrained Thermal
  graph.

official/24_linear_algebraic_thermal_node
  Supported source-to-solver linear algebraic Thermal node example with
  system-local components, across/through boundary seeds, named solved
  variables, residual norm, and largest-residual artifacts.

official/25_fixed_point_loop
  Supported source-to-solver fixed-point loop using `solve component_graph`
  with `solver = fixed_point`, numeric tolerance/max-iteration/relaxation and
  initial-guess options, convergence metadata, named solved variables,
  residual norm, and largest-residual artifacts.

official/26_dynamic_component_room
  Supported source-to-solver dynamic component smoke using `solve
  component_graph` with `solver = dynamic_component_semi_implicit_euler`,
  generated Thermal connection equations, a `der(port.T)` component-local
  equation, state/algebraic trajectories, and per-step algebraic diagnostics.

official/27_nonlinear_algebraic
  Supported source-to-solver Newton smoke using `solve component_graph` with
  `solver = newton`, a unitful HeatRate nonlinear scalar residual, convergence
  history, named solved variables, residual norm, and largest-residual
  artifacts.

official/28_small_dae
  Supported source-to-solver implicit-Euler DAE smoke using `solve
  component_graph` with `solver = implicit_euler_dae`, source-derived
  state/algebraic split, algebraic initialization, state/algebraic
  trajectories, per-step Newton diagnostics, and largest-residual artifacts.

official/29_delay_component_solver
  Supported narrow source behavior smoke using `solve component_graph` with
  `solver = dynamic_component_explicit_euler`, a component-local
  `delay(signal, duration)` expression, typed behavior graph RHS evaluation,
  unitful temperature state trajectory output, and integrated delay behavior artifacts.

official/30_predictor_component_solver
  Supported narrow source behavior smoke using `solve component_graph` with
  `solver = dynamic_component_explicit_euler`, a typed deterministic
  `predictor(signal)` identity wrapper seed, behavior graph RHS evaluation,
  unitful temperature state trajectory output, and integrated Predictor contract artifacts.

official/31_external_behavior_solver
  Supported narrow source behavior smoke using `solve component_graph` with
  `solver = dynamic_component_explicit_euler`, a typed deterministic
  `adapter(signal)` identity wrapper seed, behavior graph RHS evaluation,
  unitful temperature state trajectory output, and integrated external behavior artifacts.

official/32_small_thermal_fluid_loop
  Supported constrained Thermal/Fluid[Water] algebraic residual solve with
  generated connection equations, component-local boundary seeds, simple pipe
  pressure/flow equations, named solved variables, residual norm, and
  largest-residual artifacts. The Fluid seed uses `Pressure [Pa]` with a fixed pipe pressure-drop seed.
```

## Internal Implementation Fixtures

`examples/internal` contains implementation fixtures that are checked by the
development smoke path but are not user-facing release workflows.

```text
internal/02_simple_system
  Minimal physical system/equation surface with residual metadata, solver-plan
  metadata, and one-state fixed-step output. This is an internal solver fixture,
  not a public release claim.

internal/03_integrated_hvac
  Combined integration path for Args, CSV policies, missing-value interpolation,
  statistics, integration, PlotSpec/report, and simple system metadata. This is
  an internal integration fixture, not the recommended release smoke path.

internal/04_uncertainty_core
  Internal uncertainty-track path for deterministic uncertainty summaries,
  propagation metadata/source terms, source and argument diagnostics, and
  histogram bin artifacts. It is tested on main but not release-supported yet.

internal/05_data_driven_modeling
  Internal data-driven modeling track path for split/model/evaluation source
  diagnostics, argument diagnostics, deterministic metrics, leakage lint,
  model cards, and parity/residual plots. It is tested on main but not
  release-supported yet.

internal/06_domain_port
  Internal domain/component track fixture for user-defined domain declarations,
  across/through variables, conservation metadata, components, ports, and
  domain-compatible connection review. It includes structured generic
  parameters such as Fluid[Medium M] and MechanicalNode[Frame F, Axis DOF].

internal/17_measured_vs_simulated
  Typed data plus simulation workflow with weather/measured CSV promotion,
  explicit TimeSeries thermal input binding, one-state fixed-step simulation
  output as sim.T_zone, RMSE calculation, threshold validation, time-alignment
  metadata, and a measured/simulated multi-series PlotSpec. It is not a public
  general solver claim.

internal/18_state_space_metadata
  StateVector/InputVector/OutputVector and LinearOperator metadata with a
  narrow internal fixed-step runtime seed that materializes a promoted
  TimeSeries input. This fixture is not a supported general state-space
  simulation workflow.

internal/20_multi_state_thermal
  Multi-state state-space thermal simulation with two state trajectories,
  TimeSeries input binding, fixed-step RK4 execution, and plot/report artifacts
  for sim.T_air and sim.T_wall. This remains an internal state-space fixture;
  the release-facing source-equation two-state ODE example is
  official/20_multi_state_thermal.

internal/21_thermal_component_assembly
  Focused thermal component assembly with generated connection equations,
  component-local boundary RHS equations, a square residual graph, and a dense
  linear residual solve artifact. This is not a production multi-domain
  component solver.

internal/22_component_boundary_solve
  Component-local boundary equations plus generated Thermal connection
  equations form a square linear residual graph and exercise the internal dense
  algebraic solver artifact path. This fixture is not a production
  multi-domain component graph solver.

internal/22_multi_domain_boundary_solve
  Small Thermal/Fluid/MechanicalNode boundary fixture whose generated
  connection equations plus component-local boundary equations form a square
  residual graph solved by the dense linear path. This is still a constrained
  algebraic seed, not a production physical multi-domain simulation engine.

internal/23_component_boundary_singular
  Square Thermal boundary residual graph whose equations are singular. The
  runtime must emit a `linear_solve_failed` component solution with
  `E-LINEAR-SINGULAR` instead of inventing solved variables.

internal/24_component_boundary_overdetermined
  Thermal boundary residual graph with more equations than unknowns. The
  runtime must emit `not_solved_overdetermined` with
  `E-ASSEMBLY-OVERDETERMINED` instead of attempting a dense solve.

internal/25_component_behavior_nodes
  Valid component-local delay, Predictor, and external adapter expressions.
  Review/report/IDE artifacts expose behavior-node metadata and explicit
  solver-integration limitations.

internal/26_state_space_discrete
  Discrete two-state state-space execution with `next(x) eq A * x + B * u`,
  scalar input materialization, canonical operator matrices, named nonzero
  entries, and two emitted state trajectories.

internal/27_adaptive_heun_thermal
  One-state thermal `simulate` fixture for `solver = adaptive_heun`, fixed
  report/output TimeGrid, explicit duration, tolerance, and adaptive internal
  substep diagnostics.

internal/28_adaptive_state_space
  Continuous state-space `der(x) eq A * x + B * u` fixture for the internal
  `solver = adaptive_heun` path, including TimeSeries input materialization,
  fixed output TimeGrid, and adaptive internal substep diagnostics.
```

## Compatibility Regression Examples

`examples/compat` keeps older focused examples alive as regression coverage.
They are intentionally not the first user-facing namespace and are not copied
as release-facing examples.

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

## Data-Quality Fixtures

`examples/diagnostics/data_quality` contains CSV policy and runtime
data-quality fixtures. Some files intentionally record parse failures,
interpolation, constraint violations, or unsupported conversion failures in
generated artifacts.

## Scratch Files

The native IDE may create `examples/scratch/*.eng` during manual testing. Those
files are user work and are not part of the release contract unless explicitly
added and documented.
