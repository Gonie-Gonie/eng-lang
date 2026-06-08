# Current Project Status

This page is the authoritative short-form status layer for contributors and LLM
agents. It separates public release versions from long-term development tracks.

## Release State

| Field | Value |
|---|---|
| Current public line | `v0.1-preview` |
| Active target | `v0.2-preview` IDE/documentation hardening |
| Workspace package version | `0.1.0-preview` |
| Release channel | `preview` |

EngLang is preview software. The language, runtime behavior, and artifact
formats are not stable. Earlier high-numbered release names are not part of the
current public version line.

Future capabilities are tracked in [development tracks](tracks.md), not as
public release versions. A track may have implementation seeds on `main` without
being part of the public release contract.

## Core Execution Invariants

- Core checking, running, plotting, report generation, and packaged execution
  do not depend on Python.
- Python may be used for optional documentation tooling only.
- The official execution path is `.eng -> typed semantic model -> bytecode ->
  native runtime/VM -> result/report/PlotSpec objects`; `--save-artifacts`
  writes `.engbc`, `.engres`, SVG/HTML/report/review artifacts.
- Fast declaration uses `=`. `:=` is rejected.
- Physical equations use `eq`. `==` is comparison syntax and is rejected in
  equation blocks.
- Public features need examples, tests, diagnostics or metadata where relevant,
  and reviewable artifacts.

## Supported Preview Features

Supported preview means the behavior is documented, tested, and usable through
the current public preview workflow, but it is not yet a stable contract.

- Fast `=` declarations in script/local expression contexts.
- Unit and quantity checking for supported arithmetic and official examples.
- Dimensionless plus physical quantity diagnostics.
- Ambiguous quantity warnings for unit-only declarations such as `power = 10 kW`.
- Typed CSV promote for the official typed schema import path.
- DateTime-indexed table metadata and row-level CSV runtime pages.
- TimeSeries statistics on the official HeatRate path, including mean,
  time-weighted mean, median, standard deviation, percentiles, and trapezoidal
  integration metadata.
- PlotSpec v1 line plot data, unit-aware axis labels, SVG export, and plot
  manifest artifacts.
- Report/review artifacts with variable tables, inferred declarations, unit
  conversion records, schema summaries, warnings, plot manifest data, and
  report spec hashing.
- Minimal `system`/`eq` parsing and unit diagnostics, with one-state thermal
  system metadata and fixed-step preview execution for official examples.
- Args string/path binding for `--input` style official examples, primitive
  Bool/Int/Count/Float/Duration normalization, and packaged runner help
  metadata.
- Standalone package output with `.engpkg`, bytecode, lock, source/dependency
  copy, dependency hashes, Args help, and reviewable report artifacts.
- Temperature spelling policy: `degC` remains the canonical ASCII spelling, and
  `°C` is supported as a user-facing alias for `AbsoluteTemperature`.
- Example taxonomy: `examples/official` is the user-test namespace; top-level
  numbered examples are compatibility regression paths; diagnostic and
  data-quality fixtures are separated by folder.

## Preview Tooling

- Native tester IDE (`eng-ide.exe`) for open/check/save/run, diagnostics,
  completions, source editing, variable/unit/schema/CSV inspection, PlotSpec
  preview, runtime summaries, UI settings, and artifact links.
- VS Code extension preview packaged as a secondary editor path.
- Integrated HVAC user-test example combining CSV promote, statistics, plotting,
  reporting, and system metadata.
- Data-quality examples for parse failures, missing-value interpolation,
  constraint violations, and unit conversion failures.
- TimeSeries raw-value histogram PlotSpec path through `plot histogram(...)`,
  with bin metadata shared with future-track distribution plots.

## Future Tracks On Main

The following tracks may have implementation seeds, examples, tests, and IDE
metadata on `main`, but they are not public release versions:

- Uncertainty track
- Data-driven modeling track
- IDE/LSP track
- Runtime optimization/JIT/AOT track
- Domain/component track

See [development tracks](tracks.md) for the current scope and limitations.

## Deferred / Known Limitations

- Arbitrary table formulas are not fully general.
- Arbitrary TimeSeries expressions are limited beyond the official typed CSV
  path.
- General quantity rules for all statistics are not complete.
- Plot semantics beyond current PlotSpec paths need multi-series, custom
  histogram bin counts, and grouped/stacked bar hardening.
- Multi-state, nonlinear, adaptive, or general equation-system solving is
  deferred.
- Numeric component graph solving and domain package registries are deferred.
- Quantity/unit-literal Args conversion beyond primitive typed Args is
  deferred.
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
| `eng_lsp` | Experimental editor-service smoke and snapshot paths |

Future crate splitting should be documented as planned work, not assumed as the
current architecture.
