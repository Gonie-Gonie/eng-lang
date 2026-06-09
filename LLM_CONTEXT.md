# LLM Context

Use this file as the first repo-local context document after `README.md`.
It is intentionally short so agents do not need to load every planning file.

## Current Target

- Current public line: `v0.4-preview`
- Active target: `v0.5-preview` write/export hardening and output manifest
- Workspace package version: `0.4.0-preview`
- EngLang is preview software. The language and artifact formats are not
  stable.
- Public release versions describe packages. Long-term capabilities are tracked
  as development tracks, not as high-numbered versions.
- `v1.0` is reserved for a genuinely stable core.

## Read First

1. `README.md`
2. `LLM_CONTEXT.md`
3. `docs/current/status.md`
4. `docs/current/version_plan.md`
5. `docs/current/feature_maturity_matrix.md`
6. `docs/current/tracks.md`
7. `docs/llm/load_map.yml`

## Current Public Preview

The current public preview supports:

- typed CSV promote
- top-level execution, args, const, pure scalar fn, and relative file imports
- command-style built-in workflow verbs with where/with policy
- unit-aware TimeSeries calculation
- statistics and integration metadata
- unit-aware print and explicit summary CSV export
- typed path helpers and provenance-visible `exists`
- read-only UTF-8 `read text/json/toml` with source hash provenance
- PlotSpec/SVG output
- review/report artifacts
- basic packaged execution
- native tester IDE user workflow
- curated user and language grammar PDFs

Implementation seeds for uncertainty, data-driven modeling, LSP, JIT/AOT,
domain/component, class/domain-object, and general programming/side-effect work
may exist on `main`, but they are future tracks unless the current status
documents explicitly promote a narrow preview scope.

## Core Invariants

- No Python in the core checking, running, plotting, report, or packaged
  execution path. Python is allowed only for optional documentation tooling.
- Official artifact flow:
  `.eng -> typed semantic model -> .engbc -> native runtime/VM -> .engres -> PlotSpec -> SVG/HTML/report/review artifacts`.
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
- Top-level side effects are disallowed for file run/build paths.
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
