# Development Tracks

Tracks are long-term capability areas. They are not public release versions.
Each track below lists its purpose, current public scope, main-internal status,
and next cleanup action.

## T1 Core Language

- Purpose: keep engineering programs small, explicit, typed, and reviewable.
- Public scope: `=`, no `:=`, top-level execution, `args`, importable `const`,
  scalar `fn`, relative imports, unit diagnostics, and current formatter path.
- Main internal: broader expression and system syntax seeds exist.
- Next cleanup: keep syntax docs centered on workflow clarity, not feature
  volume.

## T2 Data Boundary

- Purpose: make source data explicit, typed, and provenance-visible.
- Public scope: schema/promote, CSV import, DateTime index metadata, source
  hashes, missing policy seeds, and typed path args.
- Main internal: data-quality diagnostics and policy fixtures.
- Next cleanup: keep schema/promote as the first public data story.

## T3 TimeSeries, Plot, Report, And Review

- Purpose: turn engineering calculations into inspectable artifacts.
- Public scope: TimeSeries statistics, integration, PlotSpec/SVG, report HTML,
  review JSON, report spec, and result artifacts.
- Main internal: bar/histogram variants, richer report metadata, and the
  normalized `review_document` projection.
- Next cleanup: route more report/IDE panels through ReviewDocument and add
  semantic diff after the risk/fallback taxonomy has runtime evidence.

## T4 System / Equation

- Purpose: produce typed TimeSeries and residual/convergence evidence for
  validation workflows.
- Public scope: measured-vs-simulated workflow and documented narrow
  system/equation examples.
- Main internal: dense linear, fixed-point, Newton, DAE, adaptive ODE,
  state-space, dynamic component, and behavior-node solver seeds.
- Next cleanup: keep detailed solver ledgers in [solver docs](../solver/README.md)
  and out of first-user docs.

## T5 IDE / LSP

- Purpose: give engineers a review cockpit for variables, units, schemas,
  TimeSeries, plots, reports, provenance, and side effects.
- Public scope: packaged Tauri/WebView tester IDE and smoke path.
- Main internal: solver/component inspector rows, LSP smoke/snapshot tooling,
  and optional VS Code extension source.
- Next cleanup: present TimeSeries/schema/unit/report panels before component
  graph and solver panels.

## T6 Uncertainty

- Purpose: make uncertainty sources and propagation reviewable.
- Public scope: none beyond internal examples unless explicitly documented.
- Main internal: deterministic samples, diagnostics, scalar runtime numeric
  payloads, narrow arithmetic propagation, validated propagation policy
  metadata, explicit statistic/probability validation type-checking, and
  pointwise TimeSeries sensor standard deviation review metadata, plus
  histogram artifacts.
- Next cleanup: keep as internal until TimeSeries uncertainty, runtime
  uncertainty validation artifacts, IDE projection, and tests align.

## T7 Data-Driven Modeling

- Purpose: make model training/evaluation metadata reviewable.
- Public scope: none beyond internal examples unless explicitly documented.
- Main internal: train/test metadata, deterministic metrics, model cards,
  parity plots, residual plots, and diagnostics.
- Next cleanup: describe as model/review artifacts, not physical system
  simulation.

## T8 Runtime Optimization / JIT / AOT

- Purpose: identify hot kernels without changing public semantics.
- Public scope: no native speedup claim.
- Main internal: `eng_jit`, `jit-plan`, `jit-bench`, interpreter kernel IR,
  benchmark catalog, and fallback metadata.
- Next cleanup: keep benchmark docs clear that timing is internal coverage until
  native speedup exists.

## T9 Domain / Component

- Purpose: represent engineering domains, ports, connections, and component
  metadata as typed review artifacts.
- Public scope: constrained examples only where current status says so.
- Main internal: domain declarations, ports, connection diagnostics, residual
  assembly metadata, constrained Thermal/Thermal-Fluid fixtures, and IDE/LSP
  metadata.
- Next cleanup: lead with semantic object/connection review, not production
  multi-domain solving.

## T10 Class / Domain Object

- Purpose: describe typed engineering objects and validation metadata.
- Public scope: typed fields/defaults, object literals, nested references,
  validation blocks, metadata methods, copy-with metadata, diagnostics, report
  artifacts, IDE summaries, and LSP metadata.
- Main internal: runtime object dispatch/lowering remains planned.
- Next cleanup: keep classes as reviewable objects, not replacements for
  systems/components.

## T11 General Programming / Side Effects

- Purpose: allow real workflow scripting while keeping effects explicit.
- Public scope: typed path helpers, read text/json/toml, explicit write
  text/json, constrained copy/move/delete, run logs, process results, test
  results, output manifests, and safe/normal/repro profiles.
- Main internal: broader filesystem/process policy plus `eng.net`, `eng.cache`,
  `eng.db`, and `eng.model` module boundaries are planned.
- Next cleanup: maintain artifact-first side-effect documentation and add
  module slices only with review/output-manifest evidence.

## T12 Composite Workflows

- Purpose: compose typed data, files, external tools, case manifests, model
  cards, database writes, and reports without making domain adapters core
  language identity.
- Public scope: supported side-effect primitives and workflow skeletons only.
- Main internal: weather API to standard file and external simulation surrogate
  hybrid examples define target contracts for future generic modules.
- Next cleanup: grow `eng.net`, `eng.cache`, `eng.table`, `eng.sampling`,
  `eng.case`, `eng.db`, and `eng.model` from generic workflow evidence.

## Internal Detail

Detailed implementation ledgers live outside this overview:

- [Main internal status](main_internal_status.md)
- [Solver docs](../solver/README.md)
- [Solver-centered plan](solver_centered_plan.md)
- [Generic solver completion plan](generic_solver_completion_plan.md)
- [Uncertainty track](uncertainty.md)
- [Reviewability track](reviewability.md)
- [Composite workflow modules](workflow_modules.md)
