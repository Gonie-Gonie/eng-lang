# Current Project Status

This page is the authoritative short-form status layer for contributors and LLM
agents. It distinguishes supported behavior from preview and experimental
implementation seeds.

## Release State

| Field | Value |
|---|---|
| Latest stable baseline | `v1.0-stable` |
| Active release target | `v1.0.3` IDE/documentation hardening |
| Next planned targets | `v1.1` uncertainty, `v1.2` data-driven modeling, `v1.3` LSP/editor service |
| Current package version | Workspace version `1.0.3` |

`v1.1`, `v1.2`, and `v1.3` support code may exist on `main`, but those
features are not release-supported until their language rules, runtime
behavior, diagnostics, IDE metadata, examples, tests, and user documentation
are aligned.
The current v1.2 implementation gate is tracked in
[v1.2 data-driven modeling gate](v1_2_data_driven_modeling_gate.md).
The current v1.3 LSP gate is tracked in [v1.3 LSP gate](v1_3_lsp_gate.md).

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
- Standalone package output with `.engpkg`, bytecode, lock, source copy, and
  reviewable report artifacts.
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
- `v1.3` LSP/editor service: experimental `eng-lsp.exe` smoke, snapshot,
  diagnostics, completion, hover, and minimal stdio JSON-RPC paths.

## Deferred / Known Limitations

- Arbitrary table formulas are not fully general.
- Arbitrary TimeSeries expressions are limited beyond the official typed CSV
  path.
- General quantity rules for all statistics are not complete.
- Plot semantics beyond current PlotSpec seeds need multi-series and true
  histogram binning hardening.
- Multi-state, nonlinear, adaptive, or general equation-system solving is
  deferred.
- Typed Args conversion beyond string/path style bindings is deferred.
- Full Unicode unit spelling support beyond the supported `°C` alias is
  deferred.

## Current Crate Architecture

The supported current workspace structure is intentionally compact:

| Crate | Role |
|---|---|
| `eng_cli` | CLI commands, package/release smoke paths, user-facing execution |
| `eng_compiler` | Lexer, parser, AST, semantic checks, units, quantities, bytecode metadata |
| `eng_runtime` | Runtime execution, VM seed, CSV/data policies, `.engres` output |
| `eng_report` | PlotSpec/SVG/report/review rendering and artifact schemas |
| `eng_ide` | Native tester IDE and package smoke UI checks |

Future crate splitting should be documented as planned work, not assumed as the
current architecture.
