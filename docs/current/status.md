# Current Project Status

This page is the authoritative short-form status layer for contributors and LLM
agents. It separates public release behavior from supported-but-non-stable
features, internal implementation seeds, and planned tracks.

## Release State

| Field | Value |
|---|---|
| Current public line | `v0.1.0` |
| Active target | solver-centered implementation hardening on `main` |
| Workspace package version | `0.1.0` |
| Release channel | initial portable package plus unreleased main-track work |

EngLang `v0.1.0` is the current published portable release. The GitHub Release
page and attached assets are audited in
[release-state.md](../release/release-state.md). Newer solver-centered work on
`main` may be implemented and tested without being part of those published
release assets. EngLang is not a complete engineering simulation solver.
Earlier high-numbered release names are historical planning labels, not the
current public version line.

The active language philosophy is recorded in
[Integrated Language Philosophy](philosophy.md):

```text
EngLang is a unit-safe engineering programming language for typed data
analysis, system simulation workflows, plotting, and reproducible review.
```

Future capabilities are tracked in [development tracks](tracks.md), not as
public release versions. A track may have implementation seeds on `main`
without being part of the public release contract.

## Core Execution Invariants

- Core checking, running, plotting, report generation, and packaged execution
  do not depend on Python.
- Python may be used for optional documentation tooling only.
- The official execution path is `.eng -> typed semantic model -> bytecode ->
  native runtime/VM -> result/report/PlotSpec objects`; `--save-artifacts`
  writes `.engbc`, `.engres`, SVG/HTML/report/review artifacts and
  `run_log.json`, `process_results.json`, `test_results.json`, and
  `output_manifest.json`.
- Fast declaration uses `=`. `:=` is rejected.
- Physical equations use `eq`. `==` is comparison syntax and is rejected in
  equation blocks.
- Public features need examples, tests, diagnostics or metadata where relevant,
  and reviewable artifacts.

## Public Package Features

Public package behavior is documented, tested, usable through the package
workflow, and covered by the breaking-change policy.

- Top-level file execution, root `args { ... }`, importable top-level `const`,
  pure scalar `fn` definitions, checked return dimensions, relative file
  imports, LSP function signature metadata, and no imported executable-body
  side effects.
- Fast `=` declarations, explicit quantity declarations, and `:=` rejection.
- Built-in quantity/unit registry, including canonical `degC` plus the `°C`
  alias for absolute temperature display.
- Unit and quantity checking for supported arithmetic, dimensionless plus
  physical quantity diagnostics, and ambiguous quantity warnings for unit-only
  declarations.
- Typed CSV promotion for the official schema/data boundary.
- DateTime-indexed table metadata, row-level CSV runtime pages, and source hash
  provenance for promoted data.
- TimeSeries statistics on the documented HeatRate path: mean, time-weighted
  mean, median, standard deviation, percentiles, duration-above metadata, and
  trapezoidal integration.
- PlotSpec v1 line plots, measured-vs-simulated multi-series line plots, SVG
  output, plot manifests, report HTML, review JSON, report spec, and result
  artifacts.
- Measured-vs-simulated workflow: weather/measured CSV promotion, explicit
  `TimeSeries[Time]` thermal input contract, one-state fixed-step thermal
  simulation output as `sim.T_zone`, RMSE metric, validation result,
  time-alignment metadata, and multi-series PlotSpec.
- Unit-aware `print`, structured `log debug/info/warn/error`, one-row summary
  CSV export, explicit write outputs, process results, local test/assert/golden
  checks, and their saved artifacts.
- Typed path helpers (`file`, `dir`, `join`, `parent`, `stem`, `extension`) and
  provenance-visible `exists`.
- Read-only UTF-8 `read text`, `read json`, and `read toml` expressions with
  source-relative resolution and source hash provenance.
- Explicit `write text/json`, summary CSV overwrite hardening, constrained
  copy/move/delete file operations, `output_manifest.json`, `run_log.json`,
  `process_results.json`, and `test_results.json`.
- `eng run --profile safe|normal|repro`: `safe` rejects explicit workflow
  write/export/file-operation/process effects, `normal` is the default, and
  `repro` records profile diagnostics in result/run-log/output-manifest
  artifacts.
- Standalone packaged runner with `.engpkg`, `.lock`, Args help, dependency
  copying, package smoke under a sanitized Rust/Python-free child-process PATH,
  curated PDF docs, and SHA256 release checksum.
- Tauri/WebView tester IDE smoke path for open/check/save/run, diagnostics,
  variable summaries, schema/TimeSeries/metric/validation/time-alignment
  inspectors for the measured workflow, schema parse/conversion failure
  inspector coverage for data-quality runs, internal state-space trajectory and
  solver-inspector coverage, component solver trajectory inspector rows with
  failure code/reason metadata,
  dependency-graph inspector coverage, class object summary inspector coverage,
  side-effect
  artifact panels for output manifest, run log, process results, and test
  results, PlotSpec viewing, and on-demand report/plot opening for stable
  workflows.

## Supported Features

Supported features are usable and tested in a narrow scope, but are not covered
by the stable breaking-change policy.

- Parenthesis-light command-style built-in verbs, owner-local `where` blocks,
  LSP hover metadata for where locals, and `with` option/display blocks for
  documented built-in workflow commands. Arbitrary user-defined command syntax
  and project-wide display policy remain planned.
- `eng fmt <file.eng>` source-preserving formatter with stdout, `--check`, and
  `--write` modes. The formatter covers current block indentation,
  colon-label continuation indentation, trailing whitespace normalization,
  comment preservation, string-brace handling, idempotence tests, and the
  official-example formatter-clean gate. Full AST-aware style rewriting remains
  planned.
- Data-quality policies for the documented examples: missing-value handling,
  monotonic DateTime checks, constraint metadata, parse-failure artifacts, and
  unsupported unit conversion diagnostics. A general policy DSL remains
  planned.
- Bar and histogram plot paths used by uncertainty histograms, ML residual
  bars, and raw value histograms. Grouped/stacked bar semantics and custom
  multi-series histogram behavior remain planned.
- Minimal `system`/`eq` support for parsing, semantic/unit diagnostics,
  parameter/state/input metadata, `der(...)`, one-state fixed-step thermal
  execution, one-state `adaptive_heun` simulation, source-equation fixed-step
  and adaptive Heun ODE execution with scalar or Time-indexed TimeSeries inputs,
  and `sim.<state>` materialization for the documented workflows. General
  equation solving beyond that source-equation shape, nonlinear, DAE, broad
  adaptive, and component-coupled solving remain planned.
- Typed-block state-space workflows: top-level `states`/`inputs` blocks,
  `StateVector[...]` and `InputVector[...]` declarations, `operator A:` and
  `operator B:` declarations, shape/unit-checked A/B matrices, discrete
  `next(x) eq A * x + B * u`, continuous `der(x) eq A * x + B * u`,
  fixed-step explicit-Euler/RK4 execution, scalar or Time-indexed TimeSeries
  input materialization, and generated `sim.<state>` TimeSeries for
  `examples/official/21_state_space_discrete` and
  `examples/official/22_state_space_continuous`. Broad operator algebra,
  nonlinear, DAE, discrete adaptive, broad adaptive, and component-coupled
  state-space solving remain planned or internal.
- Thermal component boundary assembly: component templates with ports,
  system-local `name = Component(...)` instances with empty constructors or
  declared numeric/importable-const/pure-arithmetic component parameter defaults
  plus named or declaration-order positional constructor overrides for
  boundary/equation seeds, machine-readable constructor and parameter
  provenance, `connect instance.port to instance.port`, generated Thermal
  across/through equations, component-local `name = port.signal = literal`
  boundary seeds, direct `port.signal eq literal`, and simple linear
  port-signal equations including unit-parameterized linear coefficient forms
  such as Conductance * TemperatureDelta across compatible residual display
  units, with compiler diagnostics for incompatible unitful constants. Square
  dense linear residual solve artifacts are covered by
  `examples/official/23_thermal_component_assembly` and the source-to-solver
  `examples/official/24_linear_algebraic_thermal_node`; a narrow explicit
  fixed-point source solve over linear ResidualGraph equations is covered by
  `examples/official/25_fixed_point_loop`; and a simple-linear dynamic
  component source solve is covered by
  `examples/official/26_dynamic_component_room`. Narrow coupled multi-variable
  unitful source Newton and multi-state unitful temperature implicit-Euler DAE
  component residual smokes are covered by
  `examples/official/27_nonlinear_algebraic` and
  `examples/official/28_small_dae`. Narrow unitful temperature explicit-Euler
  source behavior RHS smokes for delay, deterministic Predictor identity
  wrappers, and deterministic external adapter identity wrappers are covered by
  `examples/official/29_delay_component_solver`,
  `examples/official/30_predictor_component_solver`, and
  `examples/official/31_external_behavior_solver`. Broad args/object/non-arithmetic
  constructor bindings, broad nonlinear, derivative-rich, affine display-unit,
  or general compound-unit component-local equations, broad fixed-point/nonlinear
  source solving, behavior-node solving, broad nonlinear/DAE coupling, adaptive
  component timestepping, and production multi-domain solving remain planned or
  internal. A constrained Thermal/Fluid[Water] square algebraic residual graph
  solve is covered separately by
  `examples/official/32_small_thermal_fluid_loop`; it uses the public
  `Pressure [Pa]` quantity plus declared pump/pipe component parameters with
  numeric/importable-const/pure-arithmetic defaults plus named or
  declaration-order positional constructor overrides, but does not claim a broad
  or production multi-domain simulator. The source-to-solver
  unit-parameterized wall path is covered by
  `examples/official/33_unit_parameterized_wall`; it validates a Conductance
  [W/K] component parameter against a HeatRate residual and converts the
  coefficient into compatible residual display units without claiming broad
  nonlinear or general compound-unit component equations.
- Class/domain object authoring for typed fields/defaults, object literals,
  nested object references, field access metadata, simple validation blocks,
  zero-argument metadata methods, immutable copy-with metadata, diagnostics,
  report/review serialization, IDE object summary inspector, and LSP
  hover/member/object-literal field completion metadata with required/default
  details.
  Runtime object dispatch/lowering, method arguments, mutation, inheritance,
  and simulation lowering remain planned.

## Internal Implementation Seeds

Internal seeds may have code, tests, examples, or artifacts on `main`, but they
are not public stable workflows.

- Legacy/internal state-space metadata and runtime seeds: typed vector/operator declarations,
  vector-member diagnostics, operator quantity/unit summaries, review metadata,
  non-rectangular matrix diagnostics, non-numeric/non-finite matrix-entry
  diagnostics, unsupported unitful matrix-entry diagnostics, inverse-time
  coefficient checks and per-second canonicalization where source and
  derivative units are compatible, shape-checked
  continuous/discrete A/B execution, report/review/IDE canonical operator matrix
  and named-entry summaries, multi-state fixed-step Euler/RK4
  trajectories with TimeSeries input materialization, `OutputLayout` preserved
  across `SolverInput`/`SolverResult`, non-empty output layouts validated
  against state quantity/unit metadata, non-finite state/input/parameter
  numeric values rejected before solver execution, non-finite state-space
  matrix/input/derivative/update values rejected before trajectory emission,
  and report/review/result/IDE
  solver-inspector artifacts for state/input/parameter and output lists,
  timestep, tolerance, iteration count, convergence status, failure reason,
  and trajectory points.
  Continuous and discrete state-space fixture paths are covered by the example
  smoke gate.
  They are not a supported general nonlinear, DAE, adaptive, broad
  operator-algebra, or component-coupled state-space simulation workflow.
- System simulate diagnostics for missing inputs, non-TimeSeries bindings,
  axis/quantity mismatches, timestep/tolerance options, solver options, and
  unknown systems are covered by the CLI example smoke gate.
- Unsupported simulated system shapes are covered by an internal example smoke
  that requires an explicit `skipped_unsupported_shape` artifact instead of a
  fabricated trajectory.
- Solver algorithm seeds: dense linear solve with finite matrix/RHS/tolerance
  checks, solver-API fixed-point iteration plus the narrow
  `solve component_graph` fixed-point source path with nonconvergence
  diagnostics, and
  solver-API standalone damped Newton solve with finite initial-guess checks,
  finite-difference fallback, supplied analytic/JIT Jacobian hook,
  residual history, largest-residual summary, shared scaled residual-norm
  diagnostics, residual-assembly/evaluation finite-value checks, intermediate
  overflow rejection, and nonconvergence/singular-linear-solve/line-search
  failure artifacts. The same
  internal layer has a solver-API standalone implicit-Euler DAE seed over
  `F(x, xdot, z, u, t, p)` with optional mass matrix, finite
  derivative/mass-application checks, initial consistency checks,
  algebraic-variable initialization, explicit unsupported BDF policy, and a
  dynamic-component
  explicit-Euler seed that supports algebraic-free state updates plus
  fixed-point algebraic solves per timestep and returns state/algebraic
  trajectories through the common `SolverResult` output contract plus failure
  diagnostics. The dynamic-component seed can also drive algebraic-free RHS
  updates from derivative-form `ResidualGraph` equations, including input and
  parameter terms, through a count/name-validated residual-graph explicit-Euler
  entrypoint, and derivative plus linear algebraic residual graphs through a
  semi-implicit entrypoint with per-step dense linear algebraic solves. A
  source `EquationAssembly` bridge now validates dynamic component
  state/algebraic/input/parameter layouts, lowers arithmetic-linear derivative and
  algebraic residuals into those residual-graph entrypoints, preserves
  equation/unknown counts in component solver artifacts, and is covered by
  `examples/official/26_dynamic_component_room`,
  `tests/runtime/dynamic_component_explicit.eng`,
  `tests/runtime/dynamic_component_semi_implicit.eng`, and
  `tests/diagnostics/dynamic_component_nonconvergence.eng`. The runtime also
  has narrow source bridges from component `EquationAssembly` residuals to
  Newton and implicit-Euler DAE solves. The Newton bridge evaluates source
  residual expressions directly, scales residuals, uses finite-difference
  Jacobian by default, supports the `source_linear_terms` Jacobian hook for
  linear residual graphs, records residual history through step diagnostics,
  and is covered by `examples/official/27_nonlinear_algebraic`,
  `tests/runtime/nonlinear_residual_from_source.eng`, and
  `tests/diagnostics/newton_nonconvergence.eng`. The DAE bridge derives the
  state/algebraic split from assembly variables, builds `DaeInput`, applies
  Newton algebraic initialization, uses identity mass-matrix fallback, records
  state/algebraic trajectories and per-step Newton diagnostics, and is covered
  by `examples/official/28_small_dae`,
  `tests/runtime/small_dae_from_source.eng`, and
  `tests/diagnostics/dae_inconsistent_initial.eng`. The solver API also
  has an internal adaptive Heun/Euler ODE seed that preserves fixed output-grid
  trajectories while adapting internal substeps and reporting accepted/rejected
  substep diagnostics through system result/report/review artifacts; the
  one-state thermal `simulate` path wires
  `solver = adaptive_heun` to that seed and emits SolverResult artifacts, the
  source-equation `simulate` path now reuses the same adaptive seed through the
  shared `SourceRhsEvaluator`; its derivative coefficient and RHS expressions
  are pre-parsed through the shared arithmetic expression parser with
  unit-literal metadata, typed input symbols, and typed parameter symbols. The
  internal continuous state-space path now
  reuses the same adaptive seed for shape-checked `der(x) eq A * x + B * u`
  systems.
  Fixed-step ODE and
  dynamic-component seeds use the actual
  `TimeGrid` interval length for the final partial step when duration is not an
  exact multiple of timestep, and explicit Euler samples RHS values at the
  start of each interval. Fixed-step ODE, fixed-point, and dynamic-component
  seeds reject non-finite RHS/update values before they enter trajectories or
  algebraic artifacts. Component solver result artifacts can carry
  state/algebraic trajectory summaries, trajectory points, RuntimeTimeSeries
  materialization, tolerance, max-iteration, timestep diagnostics, per-step
  nonconvergence/failure artifacts, and IDE TimeSeries/solver-inspector rows
  from that `SolverResult` adapter. Broad adaptive solving beyond
  source-equation and state-space adaptive seeds, plus production
  nonlinear/DAE/component workflows beyond the narrow source residual smokes,
  are not supported.
- Behavior graph seeds: solver-API runtime delay buffer with linear and
  previous-sample interpolation policies, explicit initial-history policy,
  relationship artifact, solver-style behavior-node evaluation, delay-driven
  fixed-step RHS adapter coverage, and an ordered `BehaviorGraphRhsAdapter`
  that can evaluate delay, Predictor, and external nodes from
  state/input/parameter/prior-node signals before passing the combined behavior
  evaluation into a solver-API RHS closure. Component-local
  `delay(signal, duration)` diagnostics cover unknown signals and invalid
  durations. Component-local behavior calls accept known `port.variable`
  signals and prior component-local expressions with resolved quantity/unit
  metadata, plus nested delay behavior expressions as typed signal inputs.
  Component-local Predictor calls and external behavior calls also validate
  their seed syntax and known component signal before becoming behavior-node
  metadata, with each component behavior diagnostic code covered by the CLI
  example smoke gate. Runtime also has solver-API Predictor and external
  behavior wrappers with contracts, range warnings, provenance, profile policy,
  adapter failure propagation, and graph-level invalid-source/non-finite-RHS
  failure coverage. A narrow source integration now evaluates component-local
  `delay(signal, duration)`, typed deterministic `predictor(signal)`, and typed
  deterministic `adapter(signal)` identity-wrapper behavior nodes during
  `solver = dynamic_component_explicit_euler` RHS evaluation for algebraic-free
  unitful temperature component state examples. Runtime, report-spec, report HTML,
  and IDE-visible component artifacts mark these nodes as integrated in
  `examples/official/29_delay_component_solver`,
  `examples/official/30_predictor_component_solver`, and
  `examples/official/31_external_behavior_solver`. Broader behavior graph
  solving, model loading, process backends, nonlinear/DAE behavior coupling,
  and production co-simulation remain planned.
- Domain/component assembly seeds beyond the supported Thermal and constrained
  Thermal/Fluid[Water] pressure/flow shapes include internal multi-domain boundary
  solves, singular solve failure artifacts, and overdetermined limitation
  artifacts.
  `examples/internal/22_multi_domain_boundary_solve` exercises a constrained
  Thermal/Fluid/MechanicalNode boundary solve. These remain internal algebraic
  assembly seeds, not a production multi-domain component graph solver.
- Domain/component graph metadata: domains, ports, connections, diagnostics,
  generated connection-equation metadata, residual graph metadata,
  structured residual evaluator input, normalized residual evaluation,
  generated-equation reasons, domain-plan metadata, dynamic-component
  state/algebraic/input/parameter split validation into solver layouts, IDE
  component graph and equation/residual/dependency source-line inspection, and
  connection constraint consistency artifacts.
  LSP port hovers expose
  type/base-domain and medium/frame/axis labels. It is not a production
  component-graph or multi-domain solver.
- Uncertainty track: deterministic samples, source/argument diagnostics,
  propagation metadata, and histogram artifacts.
- Data-driven modeling track: train/test split metadata, deterministic model
  metrics, model-card metadata, parity plots, and residual plots.
- Runtime optimization/JIT/AOT track: `eng_jit`, `eng.exe jit-plan`,
  `eng.exe jit-bench`, backend selection metadata, interpreter kernel IR,
  interpreter executor correctness tests for arithmetic, integration, scalar
  residual, component residual Jacobian and Newton-step candidates, state-space
  RHS and explicit-Euler solver-step candidates, finite-difference Jacobian, and
  Newton-step kernels, per-candidate executor/fallback reasons, deterministic
  `jit-bench` interpreter-kernel sample executions across CSV, component, and
  state-space smoke targets, the `benchmarks/B01_*` through `B06_*` catalog
  checked by `dev.bat jit-check`, and no native speedup claim.
- LSP/VS Code track: smoke/snapshot tests, hover/completion metadata,
  conservative same-document go-to-definition, and packaged VS Code extension
  source. This is not a stable persistent editor-service contract.
- Integrated HVAC example: useful end-to-end user-test composition, not proof
  of general table, solver, or domain behavior.

## Planned Tracks

- General table formulas and arbitrary TimeSeries expression execution.
- Quantity/unit-literal Args conversion and flag-only booleans.
- Multi-return functions, package/module imports, and full formatter policy.
- General nonlinear/DAE simulation, broad adaptive, and general multi-state
  equation solving outside the current one-state thermal adaptive path,
  supported two-state source-equation path, narrow component residual
  Newton/DAE smokes, internal fixed-step/adaptive state-space paths, and
  standalone runtime algorithm seeds.
- Stable-supported state-space workflow boundaries beyond the current internal
  fixed-step vector simulation path.
- Component graph solving beyond the constrained Thermal boundary assembly and
  constrained Thermal/Fluid[Water] pressure/flow algebraic residual solve:
  broad args/object/non-arithmetic constructor bindings, nonlinear, derivative-rich, affine display-unit, or general compound-unit component behavior
  equations, mixed algebraic/dynamic variables, nonlinear/DAE coupling,
  production pressure-drop packages, and physical multi-domain coupling.
- Behavior graph integration for delay, Predictor, and external behavior
  wrappers.
- Domain package registry and open component ecosystem.
- Runtime object dispatch/lowering for class/domain objects.
- Persistent LSP editor contract and production editor integration.
- Native JIT/AOT code generation and measured speedups.
- Network/download support, broad filesystem mutation, and full process
  sandboxing.

## Deferred / Known Limitations

- Arbitrary table formulas are not fully general.
- Arbitrary TimeSeries expressions are limited beyond the documented typed CSV
  path.
- General quantity rules for all statistics are not complete.
- Plot semantics beyond current PlotSpec paths need custom histogram bin
  counts, grouped/stacked bar hardening, and broader multi-axis semantics.
- General multi-state equation-system, broad nonlinear/DAE simulation, broad
  adaptive, or general equation-system solving is deferred outside the
  source-equation fixed-step/adaptive path, the one-state thermal `adaptive_heun` path,
  the narrow component residual Newton/DAE smokes, and internal state-space
  seeds.
- Production numeric component graph solving beyond the constrained Thermal
  boundary assembly and constrained Thermal/Fluid[Water] pressure/flow algebraic
  residual solve, physical multi-domain solving, pressure-drop packages, and
  domain package registries are deferred.
- Full Unicode unit spelling support beyond the supported `°C` alias is
  deferred.
- First-class Summary objects are not part of the current scope; the v0.2
  decision is recorded in
  [summary_object_decision.md](../reference/summary_object_decision.md).

## Current Crate Architecture

The supported current workspace structure is intentionally compact:

| Crate | Role |
|---|---|
| `eng_cli` | CLI commands, package/release smoke paths, user-facing execution |
| `eng_compiler` | Lexer, parser, AST, semantic checks, units, quantities, bytecode metadata |
| `eng_jit` | Internal hot-kernel detection and numeric lowering-plan metadata |
| `eng_runtime` | Runtime execution, VM seed, CSV/data policies, `.engres` output |
| `eng_report` | PlotSpec/SVG/report/review rendering and artifact schemas |
| `eng_ide` | Tauri/WebView tester IDE and package smoke UI checks |
| `eng_lsp` | Internal editor-service smoke and snapshot paths |

Future crate splitting should be documented as planned work, not assumed as the
current architecture.
