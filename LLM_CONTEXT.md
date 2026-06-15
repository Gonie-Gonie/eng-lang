# LLM Context

Use this file as the first repo-local context document after `README.md`.
It is intentionally short so agents do not need to load every planning file.

## Current Target

- Current public line: `v1.0.0`
- Active target: `v1.0.x` stable core maintenance and scoped additions
- Workspace package version: `1.0.0`
- EngLang 1.0.0 is a stable-core release. The documented data-to-report
  workflow, artifact family, packaged runner, and native tester path are stable;
  preview/experimental tracks remain outside that contract.
- Public release versions describe packages. Long-term capabilities are tracked
  as development tracks, not as high-numbered versions.
- Stable-core scope is documented in `docs/current/stable_core_scope.md`.

## Read First

1. `README.md`
2. `LLM_CONTEXT.md`
3. `docs/current/status.md`
4. `docs/current/version_plan.md`
5. `docs/current/feature_maturity_matrix.md`
6. `docs/current/stable_core_scope.md`
7. `docs/reference/breaking_change_policy.md`
8. `docs/current/tracks.md`
9. `docs/llm/load_map.yml`

## Current Stable Core

The current stable core supports:

- typed CSV promote
- top-level execution, args, const, pure scalar fn, and relative file imports
- command-style built-in workflow verbs with where/with policy
- unit-aware TimeSeries calculation
- statistics and integration metadata
- measured-vs-simulated workflow seed with typed simulation TimeSeries, RMSE,
  validation, time-alignment metadata, and multi-series PlotSpec
- unit-aware print and explicit summary CSV export
- typed path helpers and provenance-visible `exists`
- read-only UTF-8 `read text/json/toml` with source hash provenance
- explicit `write text/json`, CSV overwrite hardening, and output manifest
- explicit `copy/move/delete` file operation seed with confirmation metadata
- `print` plus `log debug/info/warn/error` runtime messages with `run_log.json`
- explicit `run command` process execution with `ProcessResult` and
  `process_results.json`
- named `test` blocks with checked assertions, golden artifact comparisons, and
  `test_results.json`
- `eng run --profile safe|normal|repro` runtime policy basics
- PlotSpec/SVG output
- review/report artifacts
- basic packaged execution
- native tester IDE user workflow
- curated user and language grammar PDFs

Implementation seeds for uncertainty, data-driven modeling, LSP, JIT/AOT,
domain/component, class/domain-object, and general programming/side-effect work
may exist on `main`, but they are future tracks unless the current status
documents explicitly promote a narrow preview scope.
The domain/component seed includes reviewable assembly metadata, domain plans,
and a homogeneous connection-constraint solver preview; it is not production
multi-domain physical solving.

## Core Invariants

- No Python in the core checking, running, plotting, report, or packaged
  execution path. Python is allowed only for optional documentation tooling.
- Official artifact flow:
  `.eng -> typed semantic model -> .engbc -> native runtime/VM -> .engres -> PlotSpec -> SVG/HTML/report/review/test artifacts`.
- Fast declaration uses `=`. `:=` is not EngLang syntax.
- Physical systems use `system`; prediction/data-driven work should use
  model/estimator language and remain clearly separated from physical systems.
- Physical equations use `eq`. `==` is comparison syntax and should not be used
  for physical equations.
- Temperature spelling: `degC` is the canonical ASCII spelling; `°C` is a
  supported user-facing alias for `AbsoluteTemperature`.
- Examples taxonomy: `examples/official` is the user-test/release namespace.
  Top-level numbered examples are compatibility regressions; diagnostic and
  data-quality fixtures are intentionally separate.
- Hidden imported side effects are disallowed for file run/build paths; explicit
  top-level workflow effects must be typed, recorded, and reviewable.
- Public feature claims must match the feature maturity matrix.
- General programming support must keep file/process/network effects typed,
  explicit, and reviewable.

## Status Terms

- `Prototype`: internal spike or seed.
- `Preview`: works through official examples or package paths with limitations.
- `Supported preview`: documented, tested, has diagnostics or IDE metadata
  where relevant, and is part of the current public preview contract.
- `Stable`: public behavior with a breaking-change policy.
- `Experimental`: may exist on `main`, but is not public-supported.
- `Planned`: intended future work.

## Current Architecture

The current workspace is:

- `eng_cli`
- `eng_compiler`
- `eng_runtime`
- `eng_report`
- `eng_ide`
- `eng_lsp`
- `eng_jit`

Do not split crates only because the long-term plan mentions future boundaries.
Use the current architecture unless a concrete task requires a split.

## Working Rule

A feature is not complete merely because an example passes. It is complete only
when the language rule, compiler check, runtime/check behavior, diagnostic,
IDE metadata, official example, and documentation are aligned for the stated
scope.
