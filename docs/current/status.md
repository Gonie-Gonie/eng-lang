# Current Project Status

This page is the authoritative short-form status layer for contributors and LLM
agents. It distinguishes supported behavior from preview and experimental
implementation seeds.

## Release State

| Field | Value |
|---|---|
| Latest stable baseline | `v1.0-stable` |
| Active release target | `v1.0.3` IDE/documentation hardening |
| Next planned targets | `v1.1` uncertainty, `v1.2` data-driven modeling, `v1.3` LSP/editor service, `v1.4` JIT start, `v1.5` standalone/AOT maturity, `v2.0` domain/component platform |
| Current package version | Workspace version `1.0.3` |

`v1.1`, `v1.2`, `v1.3`, `v1.4`, `v1.5`, and `v2.0` support code may exist on
`main`, but those features are not release-supported until their language
rules, runtime behavior, diagnostics, IDE metadata, examples, tests, and user
documentation are aligned.
The current v1.2 implementation gate is tracked in
[v1.2 data-driven modeling gate](v1_2_data_driven_modeling_gate.md).
The current v1.3 LSP gate is tracked in [v1.3 LSP gate](v1_3_lsp_gate.md).
The current v1.4 JIT gate is tracked in [v1.4 JIT gate](v1_4_jit_gate.md).
The current v1.5 standalone gate is tracked in
[v1.5 standalone/AOT gate](v1_5_standalone_gate.md).
The current v2.0 domain/component gate is tracked in
[v2.0 domain/component gate](v2_0_domain_component_gate.md).

## Core Execution Invariants

- Core checking, running, plotting, report generation, and packaged execution
  do not depend on Python.
- Python may be used for optional documentation tooling only.
- The official artifact path is `.eng -> typed semantic model -> .engbc ->
  native runtime/VM -> .engres -> PlotSpec -> SVG/HTML/report/review artifacts`.
- Fast declaration uses `=`. `:=` is rejected.
- Physical equations use `eq`. `==` is comparison syntax and is rejected in
  equation blocks.
- Public features need examples, tests, diagnostics or metadata where relevant,
  and reviewable artifacts.

## Supported Features

Supported means the behavior is documented, tested, and usable in the current
release-target path.

- Fast `=` declarations in script/local expression contexts.
- Unit and quantity checking for supported arithmetic and official examples.
- Dimensionless plus physical quantity diagnostics.
- Ambiguous quantity warnings for unit-only declarations such as `power = 10 kW`.
- Typed CSV promote for the official typed schema import path.
- DateTime-indexed table metadata and row-level CSV runtime pages.
- TimeSeries statistics on the supported official HeatRate path, including
  mean, time-weighted mean, median, standard deviation, percentiles, and
  trapezoidal integration metadata.
- PlotSpec v1 line plot data, unit-aware axis labels, SVG export, and plot
  manifest artifacts.
- Report/review artifacts with variable tables, inferred declarations, unit
  conversion records, schema summaries, warnings, plot manifest data, and
  report spec hashing.
- Minimal `system`/`eq` parsing and unit diagnostics, with one-state thermal
  system metadata and fixed-step preview execution for official examples.
- Args string/path binding for `--input` style official examples and packaged
  runner help metadata.
- Standalone package output with `.engpkg`, bytecode, lock, source/dependency
  copy, dependency hashes, Args help, and reviewable report artifacts.
- Temperature spelling policy: `degC` remains the canonical ASCII spelling, and
  `°C` is supported as a user-facing alias for `AbsoluteTemperature`.
- Example taxonomy: `examples/official` is the release-facing user-test
  namespace; top-level numbered examples are compatibility regression paths;
  diagnostic and data-quality fixtures are separated by folder and surfaced
  after official examples in the native IDE/CLI smoke path.

## Preview Features

Preview means official examples or package paths exercise the feature, but the
scope and limitations must remain explicit.

- Native tester IDE (`eng-ide.exe`) for open/check/save/run, diagnostics,
  completions, source editing, variable/unit/schema/CSV inspection, PlotSpec
  preview, runtime summaries, and artifact links.
- VS Code extension preview packaged as a secondary editor path.
- Integrated HVAC user-test example combining CSV promote, statistics, plotting,
  reporting, and system metadata.
- Data-quality examples for parse failures, missing-value interpolation,
  constraint violations, and unit conversion failures.

## Experimental / Unreleased

Experimental means code and examples may exist on `main`, but the feature is
not part of the supported release contract.

- `v1.1` uncertainty core: measured values, intervals, distributions,
  deterministic ensembles, source diagnostics, scale/offset propagation
  metadata, propagation source terms, deterministic samples, and distribution
  histogram bins.
- `v1.2` data-driven modeling: regression, basic MLP/ANN seed, train/test
  metadata, source and argument validation diagnostics, RMSE/MAE/R2, model
  cards, leakage lint, and parity/residual plots.
- `v1.3` LSP/editor service: experimental `eng-lsp.exe` smoke, packaged
  package-smoke inclusion, snapshot, optional VS Code snapshot backend,
  diagnostics, context-aware schema column completion, hover, and tested minimal
  stdio JSON-RPC paths.
- `v1.4` JIT start: experimental `eng_jit` crate, `eng.exe jit-plan`,
  `eng.exe jit-bench`, backend selection metadata, and native IDE Runtime
  Summary display for `eng-kernel-plan-v1` hot-kernel metadata covering
  TimeSeries arithmetic, integration, statistics fusion, and system residual
  interface seeds, including coarse row/source/operation/scan estimates.
  `eng-jit-bench-v1` records interpreter baseline timings while `jit.status`
  remains `not_available`. It does not provide native code generation or
  runtime acceleration yet.
- `v1.5` standalone/AOT maturity: packaged runner manifests and locks record
  runtime ABI, repro profile, dependency paths, byte-based dependency hashes,
  and the reserved executable-wrapper/AOT boundary. Optimized native
  `model.exe`/AOT is not implemented yet.
- `v2.0` domain/component platform start: user-defined `domain` declarations,
  across/through variables, conservation metadata, `component` ports,
  package/version metadata, structured generic domain parameters such as
  `Fluid[Medium M]` and `MechanicalNode[Frame F, Axis DOF]`, connection
  review/report metadata, native IDE Domain Graph inspection, LSP
  domain/component completion and hover metadata, invalid port-domain
  diagnostics, domain contract diagnostics, and medium/frame/axis compatibility
  diagnostics. It does not provide numeric multi-domain simulation yet.

## Deferred / Known Limitations

- Arbitrary table formulas are not fully general.
- Arbitrary TimeSeries expressions are limited beyond the official typed CSV
  path.
- General quantity rules for all statistics are not complete.
- Plot semantics beyond current PlotSpec seeds need multi-series and true
  histogram binning hardening.
- Multi-state, nonlinear, adaptive, or general equation-system solving is
  deferred.
- Numeric component graph solving and domain package registries are deferred.
- Typed Args conversion beyond string/path style bindings is deferred.
- Full Unicode unit spelling support beyond the supported `°C` alias is
  deferred.

## Current Crate Architecture

The supported current workspace structure is intentionally compact:

| Crate | Role |
|---|---|
| `eng_cli` | CLI commands, package/release smoke paths, user-facing execution |
| `eng_compiler` | Lexer, parser, AST, semantic checks, units, quantities, bytecode metadata |
| `eng_jit` | Experimental hot-kernel detection and numeric lowering-plan metadata |
| `eng_runtime` | Runtime execution, VM seed, CSV/data policies, `.engres` output |
| `eng_report` | PlotSpec/SVG/report/review rendering and artifact schemas |
| `eng_ide` | Native tester IDE and package smoke UI checks |

Future crate splitting should be documented as planned work, not assumed as the
current architecture.
