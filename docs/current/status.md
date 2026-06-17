# Current Project Status

This page is the authoritative short-form status layer for contributors and LLM
agents. It separates public release behavior from supported-but-non-stable
features, internal implementation seeds, and planned tracks.

## Release State

| Field | Value |
|---|---|
| Current public line | `v1.0.0` |
| Active target | `v1.0.x` stable core maintenance and scoped additions |
| Workspace package version | `1.0.0` |
| Release channel | `stable-core` |

EngLang `1.0.0` is a stable-core release. The documented data-to-report
workflow, artifact family, packaged runner, and native tester path are stable.
Internal implementation seeds remain outside that contract. EngLang 1.0.0 is
not a complete engineering simulation solver. Earlier high-numbered release
names are not part of the current public version line.

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

## Stable Features

Stable behavior is documented, tested, usable through the package workflow, and
covered by the breaking-change policy.

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
  copying, package smoke, curated PDF docs, and SHA256 release checksum.
- Tauri/WebView tester IDE smoke path for open/check/save/run, diagnostics,
  variable summaries, schema/TimeSeries/metric/validation/time-alignment
  inspectors for the measured workflow, schema parse/conversion failure
  inspector coverage for data-quality runs, internal state-space trajectory and
  solver-inspector coverage, dependency-graph inspector coverage, class object
  summary inspector coverage, side-effect
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
- Data-quality policies for the documented examples: missing-value handling,
  monotonic DateTime checks, constraint metadata, parse-failure artifacts, and
  unsupported unit conversion diagnostics. A general policy DSL remains
  planned.
- Bar and histogram plot paths used by uncertainty histograms, ML residual
  bars, and raw value histograms. Grouped/stacked bar semantics and custom
  multi-series histogram behavior remain planned.
- Minimal `system`/`eq` support for parsing, semantic/unit diagnostics,
  parameter/state/input metadata, `der(...)`, one-state fixed-step thermal
  execution, and `sim.T_zone` materialization for the documented workflow.
  Multi-state, nonlinear, DAE, adaptive, and component-coupled solving remain
  planned.
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

- State-space metadata and runtime seeds: typed vector/operator declarations,
  vector-member diagnostics, operator quantity/unit summaries, review metadata,
  non-rectangular matrix diagnostics, unsupported unitful matrix-entry
  diagnostics, inverse-time coefficient checks and per-second canonicalization
  where source and derivative units are compatible, shape-checked
  continuous/discrete A/B execution, report/review/IDE canonical operator matrix
  and named-entry summaries, multi-state fixed-step Euler/RK4
  trajectories with TimeSeries input materialization, `OutputLayout` preserved
  across
  `SolverInput`/`SolverResult`, and report/review/result/IDE solver-inspector
  artifacts for state/input/parameter and output lists, timestep, tolerance,
  iteration count, convergence status, failure reason, and trajectory points.
  Continuous and discrete state-space fixture paths are covered by the example
  smoke gate.
  They are not a supported general nonlinear, DAE, adaptive, broad
  operator-algebra, or component-coupled state-space simulation workflow.
- System simulate diagnostics for missing inputs, non-TimeSeries bindings,
  axis/quantity mismatches, timestep options, solver options, and unknown
  systems are covered by the CLI example smoke gate.
- Unsupported simulated system shapes are covered by an internal example smoke
  that requires an explicit `skipped_unsupported_shape` artifact instead of a
  fabricated trajectory.
- Solver algorithm seeds: dense linear solve, solver-API fixed-point iteration
  with nonconvergence diagnostics, and solver-API standalone damped Newton
  solve with finite-difference fallback, supplied analytic/JIT Jacobian hook,
  residual history, largest-residual summary, and failure artifacts. The same
  internal layer has a solver-API standalone implicit-Euler DAE seed over
  `F(x, xdot, z, u, t, p)` with optional mass matrix, initial consistency
  checks, algebraic-variable initialization, and a dynamic-component
  explicit-Euler seed that supports algebraic-free state updates plus
  fixed-point algebraic solves per timestep and returns state/algebraic
  trajectories through the common `SolverResult` output contract plus failure
  diagnostics. Component solver result artifacts can carry state/algebraic
  trajectory summaries, trajectory points, and timestep diagnostics from that
  internal `SolverResult` adapter. Newton/DAE/dynamic component seeds are not
  wired into language-level nonlinear systems,
  component assembly, or report/IDE workflows.
- Behavior graph seeds: solver-API runtime delay buffer with linear and
  previous-sample interpolation policies, explicit initial-history policy,
  relationship artifact, solver-style behavior-node evaluation, and
  component-local
  `delay(signal, duration)` diagnostics for unknown signals and invalid
  durations. Component-local Predictor calls and external behavior calls also
  validate their seed syntax and known `port.variable` signal before becoming
  behavior-node metadata, with each component behavior diagnostic code covered
  by the CLI example smoke gate. Runtime also has a solver-API Predictor
  behavior contract wrapper with input/output quantity-unit metadata,
  valid-range warnings, model hash, differentiability flag, and solver Jacobian
  policy. Runtime also has a solver-API external function/process behavior
  wrapper with typed contracts, provenance hash, determinism metadata,
  safe/repro profile policy, and adapter failure propagation. Component
  artifacts distinguish delay/Predictor/external calls
  as runtime seeds through component graph, report, and IDE behavior nodes with
  inferred contract fields and diagnostic channels, but behavior nodes are not
  wired into language-level behavior graph solving. The valid behavior-node
  fixture is covered by the CLI example smoke path.
- Domain/component assembly seeds include component-local boundary equations
  for internal fixtures, dense linear residual solves when the residual graph is
  square, explicit RHS values in report specs, solved variable/residual
  artifacts, singular solve failure artifacts, and overdetermined limitation
  artifacts. `examples/official/21_thermal_component_assembly` exercises the
  focused Thermal assembly path, and
  `examples/official/22_multi_domain_boundary_solve` exercises a constrained
  Thermal/Fluid/MechanicalNode boundary solve. These remain internal algebraic
  assembly seeds, not a production multi-domain component graph solver.
- Domain/component graph metadata: domains, ports, connections, diagnostics,
  generated connection-equation metadata, residual graph metadata,
  structured residual evaluator input, normalized residual evaluation,
  generated-equation reasons, domain-plan metadata, IDE component graph and
  equation/residual/dependency source-line inspection, and connection constraint
  consistency artifacts.
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
  RHS, finite-difference Jacobian, and Newton-step kernels, per-candidate
  executor/fallback reasons, and no native speedup claim.
- LSP/VS Code track: smoke/snapshot tests, hover/completion metadata,
  conservative same-document go-to-definition, and packaged VS Code extension
  source. This is not a stable persistent editor-service contract.
- Integrated HVAC example: useful end-to-end user-test composition, not proof
  of general table, solver, or domain behavior.

## Planned Tracks

- General table formulas and arbitrary TimeSeries expression execution.
- Quantity/unit-literal Args conversion and flag-only booleans.
- Multi-return functions, package/module imports, and full formatter policy.
- General nonlinear, DAE, adaptive, and multi-state equation solving outside
  the current internal fixed-step state-space path and standalone runtime
  algorithm seeds.
- Stable-supported state-space workflow boundaries beyond the current internal
  fixed-step vector simulation path.
- Component graph solving with boundary conditions, component behavior
  equations, mixed algebraic/dynamic variables, and physical multi-domain
  coupling.
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
- Multi-state, nonlinear, adaptive, or general equation-system solving is
  deferred.
- Production numeric component graph solving, physical multi-domain solving,
  and domain package registries are deferred.
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
