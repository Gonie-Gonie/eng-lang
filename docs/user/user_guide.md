# EngLang User Guide

EngLang is a native engineering language for workflows where units, physical
quantity kinds, data schemas, plots, reports, and provenance should be checked
as part of the program. This guide is for the Windows portable package.

The package is intentionally self-contained. A target PC does not need Rust,
Python, Node, Visual Studio Build Tools, or the repository-local development
toolchain.

## Start Here

Extract the zip, open a command prompt in the extracted folder, and run:

```bat
eng.exe doctor
eng-ide.exe --smoke
eng-lsp.exe --smoke
eng.exe run examples/official/01_csv_plot/main.eng ^
  --save-artifacts
```

After the final command, open:

```text
build/result/report.html
```

That report is the fastest way to confirm that the runtime, typed CSV import,
unit-aware calculations, PlotSpec/SVG generation, and report writer are all
working from the portable folder.

## Package Contents

The portable folder contains runtime binaries, curated documentation, examples,
standard library seeds, and optional editor tooling.

Runtime binaries:

- `eng.exe`: command-line checker, runner, viewer, formatter, and packager.
- `eng-ide.exe`: native Tauri/WebView IDE for local testing and inspection.
- `eng-lsp.exe`: language-server binary used by editor tooling and smoke checks.
- `WebView2Loader.dll`: required beside `eng-ide.exe`.

Examples:

- `examples/official/`: release-facing examples. Start here.
- `examples/compat/`: compatibility regression fixtures.
- `examples/internal/`: internal solver, uncertainty, ML, domain, and
  component fixtures. These are included for built-in smoke and inspection, but
  they are not public support.
- `examples/diagnostics/`: diagnostic and data-quality fixtures.

Documentation and tooling:

- `stdlib/`: packaged standard library source seeds.
- `docs/EngLang_User_Guide.pdf`: this guide.
- `docs/EngLang_Language_Grammar_Guide.pdf`: grammar and command policy.
- `tools/vscode-englang/`: optional VS Code extension source.
- `tools/englang-vscode-0.1.0.vsix`: optional installable VS Code extension
  package.
- `README.txt`: short package start page.
- `PACKAGE_ASSETS.txt`: portable asset inventory and support boundary.

The package does not include the full developer markdown tree, local build
outputs, repository toolchain caches, Rust sources, Python documentation tools,
or Node development dependencies.

## Official Examples

Use the official examples for user testing and demos. Start with these:

- `examples/official/01_csv_plot/main.eng`: typed CSV import, HeatRate
  calculation, TimeSeries statistics, line PlotSpec, and report output.
- `examples/official/09_command_where_with/main.eng`: command policy, `where`,
  and `with` syntax.
- `examples/official/19_class_object/main.eng`: class declarations, typed
  fields, object literals, and object summaries.
- `examples/official/20_multi_state_thermal/main.eng`: supported two-state
  source-equation fixed-step ODE simulation with TimeSeries input and
  `sim.T_air`/`sim.T_wall` outputs.
- `examples/official/34_three_state_source_ode/main.eng`: supported non-thermal
  three-state source-equation adaptive ODE simulation with a promoted
  TimeSeries input and `sim.x`/`sim.y`/`sim.z`/`sim.total` outputs.
- `examples/official/21_state_space_discrete/main.eng`: supported typed-block
  discrete state-space simulation.
- `examples/official/22_state_space_continuous/main.eng`: supported
  typed-block continuous state-space simulation with CSV TimeSeries input.
- `examples/official/23_thermal_component_assembly/main.eng`: supported
  constrained Thermal component boundary assembly with system-local component
  instances and a square dense linear residual solve artifact.
- `examples/official/24_linear_algebraic_thermal_node/main.eng`: supported
  source-to-solver linear Thermal algebraic node with named solution variables,
  residual norm, and largest-residual artifacts.
- `examples/official/25_fixed_point_loop/main.eng`: supported narrow
  fixed-point algebraic source solve using `solve component_graph` with
  `solver = fixed_point` and numeric tolerance/max-iteration/relaxation
  options. Runtime coverage also includes selected direct expression-mapped
  fixed-point residuals; this is not a general fixed-point partitioner.
- `examples/official/26_dynamic_component_room/main.eng`: supported narrow
  dynamic component source solve using `solve component_graph` with
  `solver = dynamic_component_semi_implicit_euler`, generated Thermal
  connection equations, a `der(port.T)` component-local equation, trajectories,
  and per-step algebraic diagnostics. This is a simple-linear component solve,
  not a nonlinear wall-conductance or production multi-domain simulator.
- `examples/official/27_nonlinear_algebraic/main.eng`: supported narrow
  source Newton solve using `solve component_graph` with `solver = newton` for
  coupled unitful HeatRate nonlinear residuals.
- `examples/official/28_small_dae/main.eng`: supported narrow source
  implicit-Euler DAE solve using `solve component_graph` with
  `solver = implicit_euler_dae`, source-derived multi-state/algebraic split,
  vector initial values, optional dimensionless `mass_matrix` coefficients, and
  state/algebraic trajectories.
- `examples/official/29_delay_component_solver/main.eng`: supported narrow
  source behavior solve using `solve component_graph` with
  `solver = dynamic_component_explicit_euler`, a component-local
  `delay(signal, duration)` expression, and integrated delay behavior artifacts.
- `examples/official/30_predictor_component_solver/main.eng`: supported narrow
  source behavior solve using a typed deterministic `predictor(signal)`
  identity-wrapper seed during explicit-Euler RHS evaluation.
- `examples/official/31_external_behavior_solver/main.eng`: supported narrow
  source behavior solve using a typed deterministic `adapter(signal)`
  identity-wrapper seed during explicit-Euler RHS evaluation.
- `examples/official/32_small_thermal_fluid_loop/main.eng`: supported
  constrained Thermal/Fluid[Water] algebraic residual solve with generated
  connection equations, pipe pressure/flow component equations, named solved
  variables, residual norm, and largest-residual artifacts. The Fluid seed uses the public `Pressure [Pa]` quantity with a fixed pipe pressure-drop seed.

The remaining official examples cover functions/imports, read-only IO, write
output manifests, file policy, structured runtime logs, process-result records,
local test-result records, and histogram PlotSpec output. The Newton and DAE
examples are scoped solver smokes, and the behavior examples are scoped
explicit-Euler RHS smokes. The Thermal/Fluid example is a scoped algebraic
pressure/flow residual smoke with a fixed pipe pressure-drop seed. The source ODE examples are scoped one-derivative-per-state workflows, not broad nonlinear/DAE/behavior or production hydraulic/multi-domain simulation support.

Recommended first user test:

```bat
eng.exe run examples/official/01_csv_plot/main.eng ^
  --save-artifacts
```

## Native IDE Workflow

Run:

```bat
eng-ide.exe
```

Open `examples/official/01_csv_plot/main.eng` from the Explorer, or
create a scratch `.eng` file. Use `Check` for compiler diagnostics and `Run` to
execute the current file. Successful runs update the terminal, Problems tab,
Variables table, PlotSpec preview, Runtime summary, and generated artifact
paths.

The IDE uses the same compiler and runtime crates as `eng.exe`. Diagnostics,
symbols, completions, run artifacts, and report generation therefore exercise
the real core path instead of duplicated editor-only logic.

The Runtime tab summarizes result metadata from `result.engres`, including
policy execution, statistics, generated plots, and package smoke-oriented
inspection data when present.

## Command-Line Workflow

Check the package environment:

```bat
eng.exe doctor
```

Check one file:

```bat
eng.exe check examples/official/01_csv_plot/main.eng
```

Run one file and save artifacts:

```bat
eng.exe run examples/official/01_csv_plot/main.eng ^
  --save-artifacts
```

View a result record:

```bat
eng.exe view build/result/result.engres
```

Build and run a standalone bundle:

```bat
eng.exe build examples/official/01_csv_plot/main.eng ^
  --standalone --profile repro
dist/main-standalone/run.bat
```

Generated run artifacts are written under `build/result` in the current working
folder.

## Expected Artifacts

A successful run can create:

- `build/result/result.engres`: main structured result record.
- `build/result/review.json`: review and policy metadata.
- `build/result/report_spec.json`: report input model.
- `build/result/report.html`: human-readable report.
- `build/result/plots/plot_spec.json`: PlotSpec data.
- `build/result/plots/plot_manifest.json`: plot file manifest and hashes.
- `build/result/plots/timeseries.svg`: rendered SVG plot.
- `build/result/run_log.json`: structured runtime log.
- `build/result/process_results.json`: process-result records when used.
- `build/result/test_results.json`: local test-result records when used.
- `build/result/output_manifest.json`: declared output files when used.

For the CSV/plot example, the result should include typed CSV provenance,
computed statistics, integration provenance, plot hashes, and report metadata.

## Standalone Bundle

Build:

```bat
eng.exe build examples/official/01_csv_plot/main.eng ^
  --standalone --profile repro
```

Run:

```bat
dist/main-standalone/run.bat --help
dist/main-standalone/run.bat
```

The standalone bundle contains the runner, compiled package files, lock/review
metadata, source snapshot, and required source data. It writes normal
`build/result` artifacts inside the standalone folder.

## Current Boundaries

The public user contract is intentionally narrow:

- Documented syntax and command policy in the language grammar guide.
- Official examples under `examples/official`.
- Typed CSV import, unit-aware calculations, TimeSeries statistics, local test
  records, process records, structured runtime logs, PlotSpec/SVG/report
  generation, and portable package smoke.
- Native IDE check/run/inspect workflow for the packaged examples.

These areas are present only as internal or future-facing seeds unless later
documentation marks them stable:

- Broad public solver support outside the official scoped examples.
- General nonlinear solving beyond the narrow `solver = newton` component
  residual smoke.
- General DAE solving beyond the narrow `solver = implicit_euler_dae`
  component residual smoke.
- Production multi-domain component graph solving.
- Native JIT execution or speedup claims.
- Broad uncertainty and ML public workflows.
- Full editor platform guarantees beyond the packaged smoke path.

Internal examples, including system/solver examples, remain in the portable
package because `eng-ide.exe --smoke`, `eng-lsp.exe --smoke`, diagnostics, and
regression checks use them. Treat those files as inspection fixtures, not as
supported user tutorials.

## Troubleshooting

If `eng.exe doctor` fails, run it from the extracted package root so it can find
`examples`, `stdlib`, and writeable output folders.

If an example run fails, run:

```bat
eng.exe check <path-to-file.eng>
```

Then inspect the Problems list in the IDE or the command-line diagnostics.

If a CSV path fails, keep relative paths anchored next to the source file, as in
the official examples.

If a report does not open automatically, open:

```text
build/result/report.html
```

If the IDE does not start, confirm that `WebView2Loader.dll` is next to
`eng-ide.exe` and that the extracted folder was not partially copied.
