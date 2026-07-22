# Feature Maturity Matrix

Use this file to avoid treating an implementation track as a public release
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
- Main status: `Stable` with internal syntax experiments
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
- Main status: `Stable` plus data-quality test coverage
- User-facing scope: schema/promote, CSV import, DateTime index metadata,
  source hashes, typed args paths, and missing-value policies.
- Evidence: `examples/official/01_csv_plot`, data-quality diagnostics,
  runtime artifacts, report/review metadata.
- Not included: arbitrary table formulas, richer data source types, general
  data policy DSL.
- Next cleanup action: keep schema/promote as the first data story.

### TimeSeries, Statistics, Plot, Report, Review

- Public package: `Stable`
- Main status: `Stable` with supported plot/report variants
- User-facing scope: unit-aware TimeSeries calculation; native summary and
  value-call statistics including `sum(series)` and
  `duration_above(series, threshold)`; integration; PlotSpec/SVG; report
  HTML; review JSON; report spec; and unit-correct numeric result artifacts for
  inferred and explicit computed declarations.
- Evidence: official CSV workflow, artifact schemas, report/plot guides,
  artifacts check.
- Not included: broad arbitrary TimeSeries expressions, rich interactive
  plots, grouped/stacked plots, full report layout system.
- Next cleanup action: benchmark reviewability and artifact completeness before
  runtime speed.

### Reviewability / Review IR

- Public package: `Stable` for the current artifact family; `Internal` for the
  normalized ReviewDocument projection.
- Main status: `Supported` for existing review/report/result/run-log/process/
  test/output-manifest artifacts; `Internal` for `eng review` summary and IDE
  Review inspector plus `eng review --against`/`eng review diff`
  and native IDE section-hash/item-level semantic diff.
- User-facing scope: review JSON, report HTML, report spec, result artifact,
  output manifest with artifact registry, run log, process results, test
  results, and native IDE
  inspection for the current package workflows. `review.json.review_document`
  now normalizes semantic/section hashes, inputs, schemas, units/quantities,
  time axes, symbols, derived values, calculations, table transforms, report
  outputs, validations, side effects, external boundaries, caches, fallbacks,
  and risk entries. Saved runs add source-matched runtime results and refresh
  only the section hashes whose normalized content changed. Core projection
  includes materialized tables, TimeSeries, explicit coverage checks, and
  source-derived time axes across matching normalized rows, plus generated-file
  and native SQLite write evidence in side-effect runtime rows. Native model,
  model-card, metric, and prediction bindings have discriminated runtime rows
  with computed metrics, coefficients, train/test counts, hashes, prediction
  schema/output/case IDs, and row counts.
  Runtime-generated report HTML validates the final saved ReviewDocument and
  projects its fingerprint, value/evidence rows, TimeSeries/coverage/
  side-effect/model/prediction counts, coverage sample summaries, file and DB
  paths/hashes, SQLite transaction/schema/table/row evidence, computed model
  metrics/hashes, prediction schema/row evidence, statuses, and full validation
  expressions.
- Evidence: official examples, artifact schemas, `artifacts-check`, report/
  review guide, `eng review`, shared compiler diff tests, and IDE smoke path
  covering normalized Review cockpit sections, semantic comparison, external
  boundaries, and side effects.
- Not included: complete runtime projection for every specialized solver,
  assembly, and non-write DB record family or a complete risk/fallback
  taxonomy across all tracks.
- Next cleanup action: route the remaining specialized solver, assembly,
  and non-write DB report panels through normalized rows as those record
  families gain complete runtime projection.

### Measured-Vs-Simulated Validation

- Public package: `Stable`
- Main status: `Stable`
- User-facing scope: documented weather/measured CSV promotion, explicit
  `TimeSeries[Time]` input, typed simulation TimeSeries, native
  `rmse(left, right)` metric, validation result, time-alignment metadata,
  and multi-series PlotSpec.
- Evidence: artifact checks, report/review metadata, IDE inspector payloads.
- Not included: broad calibration, resampling policy controls, general solver
  selection, production model calibration.
- Next cleanup action: describe this as semantic validation, not solver
  platform evidence.

### Explicit Side Effects

- Public package: `Stable`
- Main status: `Stable`
- User-facing scope: path helpers, provenance-visible `exists`, typed
  `url(...)`, native non-secret `env(...)` with fallback/missing behavior
  and run-lock invalidation, read text/json/toml, explicit write text/json,
  constrained copy/move/delete/mkdir,
  native text template rendering with render manifests, output manifest
  artifact registry, run log, process results with tool version,
  expected-output status, stdout/stderr hashes, test results, and
  safe/normal/repro profiles.
- Evidence: official examples 10 through 16, saved artifacts, side-effect
  policy docs.
- Not included: broad filesystem mutation, network/download, full process
  sandboxing, workspace-wide test discovery.
- Next cleanup action: keep every effect typed, explicit, and reviewable.
  Template rendering uses the command-only `render template ...` API;
  function-call spelling is rejected with direct recovery guidance.

### Package And Native IDE

- Public package: `Stable`
- Main status: `Stable` with internal inspector coverage
- User-facing scope: portable package, standalone runner, curated PDFs, package
  smoke, native IDE for check/run/inspect, PlotSpec/report opening, and
  side-effect panels.
- Evidence: release-check, package-smoke, IDE smoke.
- Not included: a public cross-release compatibility commitment for the
  persistent editor protocol or a production editor platform claim.
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

### Scoped Simulation-Output Workflows

- Public package: `Supported` only for documented validation and scoped
  examples
- Main status: `Supported` and `Internal` depending on the path
- User-facing scope: documented one-state thermal and multi-state
  source-equation ODE workflows with fixed-step Euler/RK4 or adaptive Heun,
  typed-block state-space workflows, and constrained component residual
  examples where current docs state evidence.
- Evidence: official and internal examples, runtime tests, report/review/IDE
  solver artifacts.
- Not included: a general nonlinear/DAE solver product, production multi-domain
  simulation, broad component coupling, event handling, or broad behavior graph
  solving. Dense linear, fixed-point, Newton, implicit-Euler DAE, dynamic
  component, and behavior-node source paths remain internal targeted support.
- Next cleanup action: keep solver detail in `docs/internal/solver/README.md` and
  avoid making it the README identity.

## Internal Tracks

### Solver Algorithm Tracks

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

- Public package: narrow `Native workflow support`
- Main status: narrow native support for current workflow/report artifacts
- User-facing scope: workflow 03 style uncertainty metadata, explicit scalar
  constructors, linear propagation metadata, probability/statistic validation,
  and report confidence-band artifacts.
- Evidence: workflow 03, diagnostics, deterministic samples,
  scalar `typed_payload.numeric_values`, narrow measured/interval arithmetic
  artifacts, propagation metadata, validated
  `with { uncertainty = ... }` review policy metadata, direct-compare
  diagnostics, explicit uncertainty statistic/probability validation
  type-checking, runtime pass/fail materialization for explicit statistic,
  probability, and `between` validations, `review.json` summary/propagation
  sections, pointwise TimeSeries `sensor_std` review metadata, runtime
  mean/percentile/integration/duration `sensor_std` propagation artifacts,
  `sensor_std` confidence-band
  PlotSpec rendering, internal IDE variable/inspector metadata, histogram
  artifacts, and the current uncertainty guide.
- Not included: stable Monte Carlo/Jacobian propagation contract, full
  probabilistic TimeSeries uncertainty propagation/statistics, broad
  deterministic scalar-binding value propagation, or public IDE support claim.
- Next cleanup action: route broader probabilistic TimeSeries uncertainty
  semantics into runtime/report/IDE artifacts before promoting the track beyond
  narrow native workflow support.

### Data-Driven Modeling Track

- Public package: narrow `Native workflow support` through
  `eng.model`; `eng.ml` remains `Internal`
- Main status: narrow native regression/model-card/prediction support plus an
  internal broader modeling track
- User-facing scope: workflow 02 deterministic `train regression`,
  `evaluate`, `model_card`, and
  `predict <model> using <table>` paths.
- Evidence: native training/prediction tables, deterministic metrics, model
  specs/cards, target quantity/unit, prediction manifests, training/model
  hashes, SQLite prediction write/readback, plots, and diagnostics.
- Not included: broad ML package semantics, arbitrary estimators, distributed
  training, or a stable model interchange format.
- Next cleanup action: describe as model review artifacts, not physical system
  simulation.

### LSP / VS Code Track

- Public package: `Internal`
- Main status: `Internal`
- Evidence: `eng-lsp.exe` smoke/editor-request checks, a versioned persistent
  document cache with debounced diagnostics, exact-source `CheckReport` reuse,
  conservative report retargeting for token-free trivia edits with unchanged
  absolute token anchors, and a strict token-changing partial recheck exposed as
  `recheck_scalar_declaration_suffix_incrementally` that preserves the unchanged
  prefix and reparses/semantically reanalyzes from the first scalar declaration at
  or after the first changed declaration or standalone token-free trivia line.
  Clean scalar documents may interleave fast bindings, registered explicit
  annotations, and pure top-level scalar `const` declarations; unchanged
  supported `use/import eng.*` module declarations may remain in the preserved
  prefix, as may static imports whose recursively imported definitions are only
  pure registered scalar constants and functions. A clean report may also
  preserve an unchanged richer prefix before its final scalar suffix when every
  old and new suffix result is a registered scalar, each patched semantic vector
  has an exact independent tail, derived cache and axis metadata regenerate
  exactly, and isolated old-suffix analysis matches. This admits unchanged root
  scalar helpers, non-scalar file/path and TimeSeries bindings, cached boundaries,
  and command metadata such as `print`. Except for individually verified module
  declaration lines, richer prefix constructs are not reparsed or reanalyzed,
  and their non-scalar bindings cannot be used as suffix aliases or operands.
  Axis metadata is rebuilt and cache records are rekeyed
  with the new source hash after a successful patch. The richer contract remains
  root-source-only but may retain unchanged supported `eng.*` module imports
  whose compiler records exactly match their root source lines, spans, kinds,
  and statuses. Static file imports retain the stricter scalar-only import
  contract.
  Preserved imported definitions retain source ownership; constants can seed
  backward root aliases and arithmetic, and eligible constants may themselves
  use scalar function calls as values or arithmetic operands. Any suffix
  declaration form may use one or more preserved registered, unit-consistent
  function calls with exact arity and dimension-compatible scalar arguments,
  including recursively nested scalar calls. Explicit and `const` result
  dimensions must match the declaration annotation. Fast and explicit forms may
  switch style in the affected suffix. Other expressions may use numeric literals,
  backward aliases, or pure scalar arithmetic over registered-unit literals,
  parentheses, and earlier typed bindings. The path covers coordinated
  multi-line value/type/unit edits, renames, declaration additions/removals,
  complete clearing and trivia-only restart, variable-width/inserted/removed
  trivia, and suffix line-ending shifts while patching inferred, constant,
  expected, shared semantic, syntax-count, and workflow-line records together.
  Independently of report reuse, full semantic analysis projects a pure scalar
  alias-arithmetic result dimension into inferred declarations, typed bindings,
  hover/type metadata, and unit derivations. Registered quantity families use
  declaration-name disambiguation and otherwise preserve a compatible operand
  type when possible, so an expression does not need a redundant unit literal
  solely to remain visible to editor tooling. Nested user-function validation
  reports an invalid inner argument or unknown inner function at that innermost
  compiler-owned range without adding a duplicate outer unresolved-argument
  diagnostic, including when the call is nested in parenthesized scalar
  arithmetic or a built-in call argument. Call-like text inside strings is ignored;
  dimensionless math helpers and numeric percentile calls remain built-ins, while
  arbitrary `p`-prefixed calls are reported as unknown. Shared call, component,
  behavior, process-option, sampling, and scalar-expression splitting keeps quoted
  or escaped delimiters inert instead of producing false argument or `env` errors.
  The compiler's dimensionless math-function catalog feeds LSP semantic tokens,
  completion, generated metadata, and TextMate built-in scopes; component
  `predictor(...)` and call-style `predict(...)` use the solver role while the
  command-style prediction workflow retains its model role.
  Percentile statistics now use one compiler-owned parser across semantic checks,
  runtime materialization, uncertainty propagation, and JIT planning: `p1` through
  `p100` are valid, leading zeroes such as `p05` are accepted, and out-of-range
  forms are not built-ins. The same bounded pattern drives LSP, native IDE
  first-paint, generated metadata, and TextMate TimeSeries scopes.
  Parser-lowered command bindings carry explicit provenance; this keeps the
  internal template-render lowering valid without exposing `render(...)` as a
  callable built-in.
  The older fast-binding-only and explicit-declaration-only APIs are mode-limited
  compatibility wrappers over this engine; lazy shared editor snapshots,
  recursive import-dependent invalidation, a VS Code persistent stdio client
  with document sync and direct protocol semantic tokens, request-ID-scoped protocol
  cancellation with cooperative workspace scan interruption, compiler-owned
  declaration and explicit Outline selection ranges, import-source isolation
  across diagnostics/semantic overlays/hover fallback, source-origin-aware
  validation records, stdio tests, and optional VS Code source.
- Not included: a stable public compatibility guarantee across EngLang releases
  or general partial parse/semantic recomputation beyond the bounded scalar
  declaration contract, including forward/unresolved references, non-scalar
  suffix declarations, invalid, unregistered, non-unit-consistent, or
  non-scalar calls, workflow expressions in the affected suffix, changes inside
  a richer prefix, import-line edits, imports inside the affected suffix,
  token-bearing non-declaration lines in that suffix, and unverified report
  ownership.
- Next cleanup action: keep the implemented persistent service tested while its
  public maturity remains explicit.

### Runtime Optimization / JIT / AOT

- Public package: `Internal`
- Main status: `Internal`
- Evidence: `eng_jit`, `jit-plan`, `jit-bench`, interpreter kernel IR,
  benchmark catalog, and fallback metadata.
- Not included: native code generation or speedup claim.
- Next cleanup action: keep semantic benchmark strategy ahead of solver timing.

### Composite Workflow Foundations

- Public package: `Supported` for path, read/write, process, output manifest,
  run log, test, and profile primitives already listed under explicit side
  effects.
- Main status: `Supported` for those primitives plus promoted table
  diagnostics, deterministic promoted-table row-selection artifacts, promoted
  sample-table artifacts, typed config promotion optional field policy,
  promoted case table summaries with collection status and scheduler hooks,
  pending/succeeded/failed/skipped case manifests enriched from process
  outputs, case diagnostics, native SQLite DB write summaries in `typed_payload.db_manifests[]`, model
  specs/cards in `typed_payload.model_specs[]` and `typed_payload.model_cards[]`,
  prediction manifests in `typed_payload.prediction_manifests[]`, model diagnostics
  in `typed_payload.model_diagnostics[]`, DateTime-indexed
  `typed_payload.timeseries_coverage[]` records including explicit Gregorian-year coverage checks, native interpolation outputs and explicit non-mutating fill policies in `typed_payload.timeseries_fill[]`, `typed_payload.timeseries_quality[]` coverage/fill summaries, `typed_payload.expectation_suites[]` lightweight expectation-suite records, `typed_payload.quality_results[]` common quality records for TimeSeries, validation, schema-constraint, and expectation-suite results with row/field failure details, report-facing `report_spec.quality_report`, HTML Quality Report tables, IDE Quality inspector payloads, native exact/nearest/linear TimeSeries outputs with `typed_payload.time_alignments[]` materialization records, and time-axis coverage artifacts;
  native workflow examples now emit weather, template-rendered case input,
  model-card, prediction-manifest, and DB side-effect artifacts without external
  process adapters; deterministic grid/random/LHS sampling materializes
  `typed_payload.sample_tables[]`; native SQLite append/upsert/replace writes
  include manifests, typed table readback records, schema diagnostics,
  transaction status, and DB file hashes; preferred native
  `train regression`, warning-producing compatibility aliases
  `regression_table(...)`/`train_regression(...)`, and
  `predict <model> using <table>` materialize
  Table[Prediction] rows and `typed_payload.prediction_manifests[]`; live
  HTTP(S) GET/download execution materializes pinned response/download bodies
  with cache replay; native `materialize cases`, template `apply`, explicit
  sequential or bounded parallel `apply run_case`, and `collect results`
  materialize CaseTable, CaseOutput,
  CaseRunResult, and CaseResultCollection rows plus per-case result/run
  manifests. Calculation-hash/result-SHA output resume, content-addressed local
  cache replay/repair, overwrite, and fail/continue policies are implemented
  for the native expression runner; explicit parallel workers use deterministic
  static partitions and lifecycle hooks. Shared or remote case caches,
  automatic external-adapter dispatch, broad DB support, and
  broader model train syntax remain `Planned`.
- User-facing scope: generic module boundaries only. Domain-specific KMA, EPW,
  EnergyPlus, CFD, FEM, or database adapters are examples layered above the
  core, not core language identity.
- Evidence: official side-effect examples, process artifacts, output manifests,
  `typed_payload.table_diagnostics[]`, `typed_payload.table_selections[]`,
  `typed_payload.table_transforms[]` with Date/DateTime predicate comparison
  evidence and row diagnostics,
  `review_document.table_transforms[]`,
  `typed_payload.config_promotions[]`,
  `typed_payload.timeseries_coverage[]`, `typed_payload.timeseries_quality[]`, `typed_payload.expectation_suites[]`, `typed_payload.quality_results[]`, `report_spec.quality_report`, HTML Quality Report tables, IDE Quality inspector payloads, `typed_payload.time_alignments[]`, `typed_payload.sample_tables[]`,
  `typed_payload.case_tables[]`, `typed_payload.case_manifests[]`,
  `typed_payload.case_diagnostics[]`, `typed_payload.db_manifests[]`,
  `typed_payload.structured_reads[]`,
  `typed_payload.model_specs[]`, `typed_payload.model_cards[]`,
  `typed_payload.prediction_manifests[]`, `typed_payload.model_diagnostics[]`,
  workflow examples under `examples/workflows`,
  data-quality diagnostics for invalid sample rows, IDE table transform
  inspector smoke coverage, and
  `docs/current/workflow_modules.md`.
  The native surrogate workflow now records native sample rows,
  CaseOutput rows from template `apply`, CaseRunResult rows from
  `apply run_case over case_inputs`, CaseResultCollection rows from
  `collect results case_runs`, and preserved typed source/result columns across
  every case stage, case_input artifacts, surrogate model specs/cards trained
  from the final collection,
  prediction manifests with output quantity/unit and
  confidence-column metadata, DB write manifests with schema diagnostics,
  table records, transaction status, and typed DB readback as reviewable native
  artifacts.
- Not included: request bodies/auth beyond `secret env` and broader cache
  invalidation/reuse API,
  general table derived-value execution/fill transforms,
  automatic external-adapter dispatch, domain weather
  adapters, EPW writer, EnergyPlus IDF parser, broad DB engines/query
  APIs/migrations, or ML framework
  support.
- Next cleanup action: grow `eng.cache` beyond network and native case-result
  materialization/replay into process/model and cross-artifact invalidation,
  then extend the existing native workflow artifact snapshots to cover
  external-adapter and shared/remote `eng.case` dispatch/cache policy, `eng.db`
  query/migration, and `eng.model` framework-adapter slices with artifacts and
  diagnostics.

## Solver Vocabulary

Use these labels consistently in public docs:

| Term | Current meaning |
|---|---|
| Typed TimeSeries producer | Preferred product-facing description for scoped simulation paths. |
| Solver metadata | Review/result/report-spec metadata for scoped equation evidence and failure diagnostics. |
| Narrow solver smoke | A testable validation example with explicit limits. |
| General solver | Planned only. Do not use for current package claims. |
| Component graph solver | Narrow constrained scopes only unless a current status document says otherwise. |

## Completion Policy

A feature is not complete merely because an example passes. A feature is
complete only when its language rule, parser/AST support, semantic/type/unit
check, runtime/check behavior, diagnostics, IDE metadata or inspector support,
report/review artifact, official example or internal validation example, tests, and
documentation are aligned for the stated scope.
