# LLM Context

Use this file as the first repo-local context document after `README.md`.
It is intentionally short so agents do not need to load every planning file.

## Current Target

- Latest stable baseline: `v1.0-stable`
- Active release target: `v1.0.3` IDE and documentation hardening
- Next targets: `v1.1` uncertainty, then `v1.2` data-driven modeling
- `v1.1` and `v1.2` code on `main` is experimental unless the current status
  documents say otherwise.
- Current `v1.1` detail work includes deterministic uncertainty samples,
  source validation diagnostics, scale/offset propagation metadata, histogram
  artifacts, and IDE/report inspection.

## Read First

1. `README.md`
2. `LLM_CONTEXT.md`
3. `docs/current/status.md`
4. `docs/current/feature_maturity_matrix.md`
5. `docs/current/v1_0_3_hardening.md`
6. `docs/llm/load_map.yml`

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

## Status Terms

- `Prototype`: internal spike or seed.
- `Preview`: works through official examples or package paths with limitations.
- `Supported`: documented, tested, has diagnostics or IDE metadata where
  relevant, and is part of the release-target contract.
- `Stable`: public behavior with a breaking-change policy.
- `Experimental`: may exist on `main`, but is not release-supported.
- `Planned`: intended future work.

## Current Architecture

The current supported workspace is:

- `eng_cli`
- `eng_compiler`
- `eng_runtime`
- `eng_report`
- `eng_ide`

Do not split crates only because the long-term plan mentions future boundaries.
Use the current architecture unless a concrete task requires a split.

## Working Rule

A feature is not complete merely because an example passes. It is complete only
when the language rule, compiler check, runtime/check behavior, diagnostic,
IDE metadata, official example, and documentation are aligned for the stated
scope.
