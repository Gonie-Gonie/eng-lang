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

official/02_simple_system
  Supported physical system/equation metadata and one-state fixed-step output.

official/03_integrated_hvac
  Recommended integration user test. Combines Args, CSV policies, missing-value
  interpolation, statistics, integration, plotting, reports, and system
  metadata.

official/04_uncertainty_core
  Internal uncertainty-track path for deterministic uncertainty summaries, propagation
  metadata/source terms, source and argument diagnostics, and histogram bin
  artifacts. It is tested on main but not release-supported yet.

official/05_data_driven_modeling
  Internal data-driven modeling track path for split/model/evaluation source diagnostics,
  argument diagnostics, deterministic metrics, leakage lint, model cards, and
  parity/residual plots. It is tested on main but not release-supported yet.

official/06_domain_port
  Internal domain/component track fixture for user-defined domain declarations,
  across/through variables, conservation metadata, components, ports, and
  domain-compatible connection review. It includes structured generic
  parameters such as Fluid[Medium M] and MechanicalNode[Frame F, Axis DOF].

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

official/17_measured_vs_simulated
  Stable measured-vs-simulated workflow with weather/measured CSV promotion,
  explicit TimeSeries input binding, one-state fixed-step simulation output,
  RMSE, validation, time alignment, and multi-series PlotSpec.

official/19_class_object
  Supported class/domain-object authoring fixture with typed fields, defaults,
  object literals, validation, field access metadata, immutable copy-with, and
  class/object artifacts.

official/20_multi_state_thermal
  Multi-state state-space thermal simulation with two state trajectories,
  TimeSeries input binding, fixed-step RK4 execution, and plot/report artifacts
  for sim.T_air and sim.T_wall. This is not a nonlinear, DAE, adaptive, or
  production component-graph solver.
```

## Internal Implementation Fixtures

`examples/internal` contains implementation fixtures that are checked by the
development smoke path but are not user-facing release workflows.

```text
internal/18_state_space_metadata
  StateVector/InputVector/OutputVector and LinearOperator metadata with a
  narrow internal fixed-step runtime seed that materializes a promoted
  TimeSeries input. This fixture is not a supported general state-space
  simulation workflow.
```

## Compatibility Regression Examples

The top-level numbered examples keep older paths alive and provide focused
regression coverage. They are intentionally not the first user-facing namespace.

```text
01_units
02_csv_plot
04_plotting
06_simple_system
```

## Diagnostic Fixtures

`05_error_messages` contains examples that are expected to produce specific
diagnostics or warnings. Use them when changing parser, semantic, unit, workflow,
equation, domain, port, or connection diagnostics.

## Data-Quality Fixtures

`07_data_quality` contains CSV policy and runtime data-quality fixtures. Some
files intentionally record parse failures, interpolation, constraint
violations, or unsupported conversion failures in generated artifacts.

## Scratch Files

The native IDE may create `examples/scratch/*.eng` during manual testing. Those
files are user work and are not part of the release contract unless explicitly
added and documented.
