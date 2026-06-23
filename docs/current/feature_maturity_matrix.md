# Feature Maturity Matrix

Use this file to avoid treating an implementation seed as a public release
claim. Public package status and main-internal status are intentionally
separate.

## Status Terms

| Status | Meaning |
|---|---|
| Stable | Public behavior covered by the current package scope and breaking-change policy. |
| Supported | Usable, documented, tested, and visible through artifacts for a narrow stated scope, but not stable. |
| Internal | Code, tests, examples, or artifacts may exist on `main`, but this is not public support. |
| Planned | Intended future work with no supported implementation contract yet. |

## Public Package Features

### Core Language

- Public package: `Stable`
- Main status: `Stable` plus internal syntax seeds
- User-facing scope: top-level execution, `args`, importable `const`, scalar
  `fn`, relative imports, fast `=`, `:=` rejection, unit diagnostics, and
  current formatter support.
- Evidence: official examples, compiler diagnostics, formatter gate, LSP
  metadata where relevant.
- Not included: package/module imports, multi-return functions, broad
  expression language, full AST-aware formatting policy.
- Next cleanup action: keep syntax docs centered on semantic workflow clarity.

### Typed Data Boundary

- Public package: `Stable`
- Main status: `Stable` plus data-quality fixtures
- User-facing scope: schema/promote, CSV import, DateTime index metadata,
  source hashes, typed args paths, missing policy seeds.
- Evidence: `examples/official/01_csv_plot`, data-quality diagnostics,
  runtime artifacts, report/review metadata.
- Not included: arbitrary table formulas, richer data source types, general
  data policy DSL.
- Next cleanup action: keep schema/promote as the first data story.

### TimeSeries, Statistics, Plot, Report, Review

- Public package: `Stable`
- Main status: `Stable` with supported plot/report variants
- User-facing scope: unit-aware TimeSeries calculation, statistics,
  integration, PlotSpec/SVG, report HTML, review JSON, report spec, and result
  artifacts.
- Evidence: official CSV workflow, artifact schemas, report/plot guides,
  artifacts check.
- Not included: broad arbitrary TimeSeries expressions, rich interactive
  plots, grouped/stacked plots, full report layout system.
- Next cleanup action: benchmark reviewability and artifact completeness before
  runtime speed.

### Measured-Vs-Simulated Validation

- Public package: `Stable`
- Main status: `Stable`
- User-facing scope: documented weather/measured CSV promotion, explicit
  `TimeSeries[Time]` input, typed simulation TimeSeries, RMSE metric,
  validation result, time-alignment metadata, and multi-series PlotSpec.
- Evidence: artifact checks, report/review metadata, IDE inspector payloads.
- Not included: broad calibration, resampling policy controls, general solver
  selection, production model calibration.
- Next cleanup action: describe this as semantic validation, not solver
  platform evidence.

### Explicit Side Effects

- Public package: `Stable`
- Main status: `Stable`
- User-facing scope: path helpers, provenance-visible `exists`, read
  text/json/toml, explicit write text/json, constrained copy/move/delete,
  output manifest, run log, process results, test results, and
  safe/normal/repro profiles.
- Evidence: official examples 10 through 16, saved artifacts, side-effect
  policy docs.
- Not included: broad filesystem mutation, network/download, full process
  sandboxing, workspace-wide test discovery.
- Next cleanup action: keep every effect typed, explicit, and reviewable.

### Package And Native Tester IDE

- Public package: `Stable`
- Main status: `Stable` with internal inspector coverage
- User-facing scope: portable package, standalone runner, curated PDFs, package
  smoke, native tester IDE for check/run/inspect, PlotSpec/report opening, and
  side-effect panels.
- Evidence: release-check, package-smoke, IDE smoke.
- Not included: persistent LSP editor contract, production editor platform.
- Next cleanup action: lead IDE docs with TimeSeries/schema/unit/report review.

## Supported Narrow Main Features

### Command-Style Verbs, `where`, And `with`

- Public package: `Supported`
- Main status: `Supported`
- User-facing scope: built-in workflow verbs only.
- Evidence: `examples/official/09_command_where_with`.
- Not included: arbitrary user-defined command syntax.
- Next cleanup action: keep examples formatter-clean and command policy narrow.

### Class / Domain Objects

- Public package: `Supported`
- Main status: `Supported`
- User-facing scope: typed fields/defaults, object literals, nested references,
  validation blocks, metadata methods, copy-with metadata, diagnostics,
  report/review serialization, IDE summaries, and LSP metadata.
- Evidence: `examples/official/19_class_object`, diagnostics, report/IDE
  artifacts.
- Not included: runtime object dispatch, mutation, inheritance, method
  arguments, simulation lowering.
- Next cleanup action: present classes as reviewable engineering objects.

### Minimal System / Equation Workflows

- Public package: `Supported` only for documented validation and scoped
  examples
- Main status: `Supported` and `Internal` depending on the path
- User-facing scope: narrow source-equation ODE, one-state thermal,
  typed-block state-space, and constrained component residual examples where
  current docs state evidence.
- Evidence: official and internal examples, runtime tests, report/review/IDE
  solver artifacts.
- Not included: general nonlinear/DAE/adaptive/component-coupled solving,
  production multi-domain simulation, broad behavior graph solving.
- Next cleanup action: keep solver detail in `docs/solver/README.md` and
  avoid making it the README identity.

## Internal Tracks

### Solver Algorithm Seeds

- Public package: `Internal`
- Main status: `Internal`
- Evidence: dense linear, fixed-point, Newton, DAE, adaptive ODE, behavior,
  and dynamic component tests and artifacts.
- Not included: broad production solver support.
- Next cleanup action: keep algorithm tests independent until shared source
  syntax, artifacts, IDE, and docs align.

### Domain / Component Track

- Public package: `Internal` except documented narrow examples
- Main status: `Internal`
- Evidence: domain declarations, ports, connection diagnostics, assembly
  metadata, residual graph artifacts, IDE/LSP metadata.
- Not included: production numeric multi-domain solver, domain package
  registry, pressure-drop packages.
- Next cleanup action: lead with typed vocabulary and review artifacts.

### Uncertainty Track

- Public package: `Internal`
- Main status: `Internal`
- Evidence: deterministic samples, diagnostics, propagation metadata, and
  histogram artifacts.
- Not included: stable Monte Carlo/Jacobian propagation contract.
- Next cleanup action: keep internal until examples, artifacts, and guide align.

### Data-Driven Modeling Track

- Public package: `Internal`
- Main status: `Internal`
- Evidence: train/test split metadata, deterministic metrics, model cards,
  parity/residual plots, and diagnostics.
- Not included: broad ML package semantics.
- Next cleanup action: describe as model review artifacts, not physical system
  simulation.

### LSP / VS Code Track

- Public package: `Internal`
- Main status: `Internal`
- Evidence: `eng-lsp.exe` smoke/snapshot, stdio tests, optional VS Code source.
- Not included: stable persistent editor-service contract.
- Next cleanup action: keep LSP commands labeled as internal smoke.

### Runtime Optimization / JIT / AOT

- Public package: `Internal`
- Main status: `Internal`
- Evidence: `eng_jit`, `jit-plan`, `jit-bench`, interpreter kernel IR,
  benchmark catalog, and fallback metadata.
- Not included: native code generation or speedup claim.
- Next cleanup action: keep semantic benchmark strategy ahead of solver timing.

## Solver Vocabulary

Use these labels consistently in public docs:

| Term | Current meaning |
|---|---|
| Typed TimeSeries producer | Preferred product-facing description for scoped simulation paths. |
| Solver metadata | Review/result/report-spec metadata describing equations, residuals, methods, limitations, and convergence/failure evidence. |
| Narrow solver smoke | A testable implementation fixture with explicit limits. |
| General solver | Planned only. Do not use for current package claims. |
| Component graph solver | Narrow constrained scopes only unless a current status document says otherwise. |

## Completion Policy

A feature is not complete merely because an example passes. A feature is
complete only when its language rule, parser/AST support, semantic/type/unit
check, runtime/check behavior, diagnostics, IDE metadata or inspector support,
report/review artifact, official example or internal fixture, tests, and
documentation are aligned for the stated scope.
