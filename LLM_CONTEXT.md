# LLM Context

Use this file as the first repo-local context document after `README.md`.
It is intentionally short so agents do not need to load every planning file.

## Current Target

- Current public line: `v0.1.0`
- Active target: `v0.1.x` cleanup and scoped additions
- Workspace package version: `0.1.0`
- EngLang `v0.1.0` is an initial portable public package, not a general
  engineering solver release.
- The public package contract is the documented semantic workflow:
  typed data boundary, unit/quantity-aware TimeSeries work, report/review
  artifacts, explicit side effects, package smoke, and native tester IDE.
- Solver-centered work on `main` is internal or narrowly supported unless
  `docs/current/status.md` and `docs/current/feature_maturity_matrix.md`
  state a concrete public scope.
- Public release versions describe packages. Long-term capabilities are tracked
  as development tracks, not as high-numbered version ladders.

## Read First

1. `README.md`
2. `LLM_CONTEXT.md`
3. `docs/current/status.md`
4. `docs/current/philosophy.md`
5. `docs/current/feature_maturity_matrix.md`
6. `docs/current/tracks.md`
7. `docs/current/version_plan.md`
8. `docs/current/uncertainty.md`
9. `docs/current/reviewability.md`
10. `docs/current/workflow_modules.md`
11. `docs/llm/load_map.yml`

Open solver-specific documents only for solver implementation tasks:

- `docs/internal/solver/README.md`
- `docs/current/solver_centered_plan.md`
- `docs/current/generic_solver_completion_plan.md`

## Product Identity

EngLang is a semantic engineering workflow language.
It helps engineers and LLM-generated code preserve units, quantities, schemas,
axes, provenance, plots, and review artifacts across typed data analysis and
simulation-result validation.

What EngLang is:

- unit-safe typed data analysis
- TimeSeries semantics with explicit axes
- schema/promote data boundary
- report, review, and provenance artifacts
- LLM-reviewable engineering computation
- native IDE inspection for engineering artifacts

What EngLang is not:

- a solver-first language
- a Modelica or Simulink replacement
- a production multi-domain solver
- an EnergyPlus replacement
- a general nonlinear solver platform

## Current Public Package Scope

The public package centers on:

- typed CSV promote
- top-level execution, `args`, `const`, scalar `fn`, and relative imports
- command-style built-in workflow verbs with `where` and `with`
- unit-aware TimeSeries calculation
- statistics and integration metadata
- measured-vs-simulated validation artifacts for the documented scope
- unit-aware print and explicit summary CSV export
- typed path helpers and provenance-visible `exists`
- read-only UTF-8 `read text/json/toml` with source hash provenance
- explicit `write text/json`, constrained file operations, output manifest,
  run log, process results, and test results
- `eng run --profile safe|normal|repro` runtime policy basics
- PlotSpec/SVG output
- review/report artifacts
- basic packaged execution
- native tester IDE user workflow
- curated user and language grammar PDFs

## Internal / Narrow Tracks

Implementation evidence for uncertainty, data-driven modeling, LSP, JIT/AOT,
domain/component, state-space, class/domain-object, and solver work may exist on
`main`. Treat those tracks as `Internal` unless the status documents state a
narrow `Supported` scope and evidence.

Solvers are producers of typed TimeSeries and reviewable residual/convergence
artifacts. They are supporting capability, not the primary identity of EngLang.

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
- Temperature spelling: `degC` is the canonical ASCII spelling; the degree-C
  alias is user-facing compatibility only.
- Hidden imported side effects are disallowed for file run/build paths.
- Explicit top-level workflow effects must be typed, recorded, and reviewable.
- Public feature claims must match the feature maturity matrix.
- Composite workflow examples must stay generic. Weather, EPW, EnergyPlus-like
  simulation, database, and model-training adapters are layered examples unless
  the generic module and artifact contract is supported.

## Status Terms

- `Stable`: public behavior covered by the current public package scope and
  breaking-change policy.
- `Supported`: usable, documented, tested, and visible through diagnostics or
  artifacts for a stated narrow scope, but not covered by stable policy.
- `Internal`: may have code, tests, examples, or artifacts on `main`, but is
  not a public-supported release feature.
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
IDE metadata, official example or internal fixture, and documentation are
aligned for the stated scope.
