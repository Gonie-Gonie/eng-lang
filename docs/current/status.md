# Current Project Status

This page is the authoritative short-form status layer for contributors and LLM
agents. It describes the public package first. Internal implementation tracks
are summarized separately in [main_internal_status.md](main_internal_status.md).

## Release State

| Field | Value |
|---|---|
| Current public line | `v0.1.0` |
| Active target | `v0.1.x` cleanup and scoped additions |
| Workspace package version | `0.1.0` |
| Release channel | initial portable package plus unreleased main work |

EngLang `v0.1.0` is the current published portable release. The GitHub Release
page and attached assets are audited in
[release-state.md](../release/release-state.md).

EngLang is a semantic engineering workflow language. It is not a complete
engineering simulation solver. Later solver-centered commits on `main` are
unreleased implementation work until a new package/tag is published.

## Product Statement

EngLang helps engineers and LLM-generated code preserve units, quantities,
schemas, axes, provenance, plots, and review artifacts across typed data
analysis and simulation-result validation.

System simulation is a supporting capability when it produces typed TimeSeries
and reviewable residual/convergence artifacts. It is not the primary product
identity.

## Public Package Features

Public package behavior is documented, tested, usable through the package
workflow, and covered by the breaking-change policy.

### Core Language And Data Boundary

- Top-level file execution without `script main`.
- Root `args { ... }` for String/path/CsvFile/DirectoryPath and primitive
  Bool/Int/Count/Float/Duration values.
- Fast `=` bindings, explicit quantity declarations, and `:=` rejection.
- Top-level importable `const`, pure scalar `fn`, checked return dimensions,
  relative file imports, and no imported executable-body side effects.
- Built-in quantity/unit registry with `degC` as the canonical ASCII
  temperature spelling.
- Typed CSV promotion and JSON-record table promotion for schema/data boundaries.
- DateTime-indexed table metadata, row-level promoted-table runtime pages, source hash
  provenance, `typed_payload.table_diagnostics[]` summaries for promoted data,
  `typed_payload.table_selections[]` records for deterministic promoted-table row
  selection, `typed_payload.table_transforms[]` records for filter/select/derive/sort/require_one/join
  row counts, Date/DateTime predicate comparison, selected columns, derived
  columns, sort keys, predicate evidence, join key pair counts, and row-level diagnostics,
  `typed_payload.timeseries_coverage[]` records for DateTime-indexed
  promoted-table coverage, native `method = interpolate` fill outputs plus
  explicit non-mutating `record_only` policies in
  `typed_payload.timeseries_fill[]`, `typed_payload.timeseries_quality[]`
  coverage/fill summaries, filled TimeSeries outputs consumable by native
  statistics and HeatRate integration, `typed_payload.expectation_suites[]` lightweight expectation-suite
  records, `typed_payload.quality_results[]` common quality records for
  TimeSeries, validation, schema-constraint, and expectation-suite results,
  row/field failure details, report-facing `report_spec.quality_report`, HTML
  Quality Report tables, and IDE Quality inspector payloads,
  `typed_payload.time_alignments[]` comparison metadata plus native
  exact/nearest/linear alignment and resampling outputs,
  `typed_payload.sample_tables[]` summaries and row previews for generated
  and promoted sample/case tables, `typed_payload.case_tables[]` case summary rows with
  pending/succeeded/failed/skipped counts, collection status, scheduler hooks,
  and cache hit/miss counts,
  `typed_payload.case_manifests[]` case row manifests with sample row hashes and
  process-output enrichment, and `typed_payload.case_diagnostics[]` duplicate,
  output, step, directory, and cache-skip diagnostics.

### TimeSeries, Plot, Report, And Review

- Unit-aware TimeSeries calculation on the documented public paths.
- TimeSeries statistics: native mean, time-weighted mean, sum, median, standard
  deviation, percentile, `duration_above(series, threshold)` scalar and
  summary values, and trapezoidal integration.
- Explicit statistic, integration, and RMSE declarations materialize numeric
  values in their declared display units for artifact and print/export reuse.
- PlotSpec v1 line plots, measured-vs-simulated multi-series line plots, SVG
  output, plot manifests, report HTML, review JSON, report spec, and result
  artifacts.
- Unit-aware `print` and explicit one-row summary CSV export.

### Validation Workflow

- Measured-vs-simulated workflow for the documented scope:
  weather/measured CSV promotion, explicit `TimeSeries[Time]` input contract,
  typed simulation TimeSeries output, native `rmse(left, right)` metric,
  validation result, time-alignment metadata, and multi-series PlotSpec.

This validation workflow demonstrates how simulation output can become typed
review material. It is not a broad solver claim.

### Explicit Side Effects

- Typed path helpers: `file`, `dir`, `join`, `parent`, `stem`, `extension`.
- Provenance-visible `exists`.
- Typed `url("https://...")` values and native non-secret
  `env("NAME", "fallback")` values with resolved/fallback/missing provenance,
  explicit missing-value failure, and run-lock dependency hashing.
- Read-only UTF-8 `read text`, `read json`, and `read toml` expressions with
  source-relative resolution and source hash provenance; `read json` payloads
  can be promoted to config contracts or JSON-record tables.
- Explicit `write text/json`, constrained copy/move/delete/mkdir file operations,
  native `render template` generated inputs with render manifests, CSV
  overwrite hardening, and `output_manifest.json`.
- Template rendering is exposed only as the `render template ...` command form;
  `render(...)` reports `E-RENDER-CALL-001` with the supported spelling.
- Structured `log debug/info/warn/error` and `run_log.json`.
- Explicit `run command`, `ProcessResult`, `env`, `timeout`, `retry`,
  `tool_version`, expected-output contracts, stdout/stderr hashes, and
  `process_results.json`.
- Named `test` blocks, checked assertions, golden artifact comparisons, and
  `test_results.json`.
- `eng run --profile safe|normal|repro` runtime policy basics.

### Package And IDE

- Standalone packaged runner with `.engpkg`, `.lock`, Args help, dependency
  copying, package smoke, curated PDF docs, release zip, and SHA256 checksum.
- Portable native IDE smoke path for open/check/save/run,
  diagnostics, variable summaries, schema/TimeSeries/metric/validation
  inspectors, checked-code role-aware highlight overlay, module registry
  browser, table transform inspector rows, PlotSpec viewing, report opening,
  and side-effect artifact panels.
- Packaged editor-tooling binary plus an internal persistent VS Code stdio
  client for document sync, diagnostics, semantic tokens, and editor requests.
  Exact-source editor requests share one compiler report and lazy snapshot;
  token-free comment/blank-line edits can retarget that report when every token
  and token-bearing line stays at the same absolute source location. A separate
  strict path can preserve a verified report prefix and reparse/semantically
  reanalyze only a final suffix of top-level fast, explicit, and pure scalar
  `const` declarations. Scalar-only documents may retain unchanged supported
  module or static imports. A clean cache-free/axis-free report may also retain
  an unchanged richer prefix, including root scalar helpers and `print`, when
  every preserved typed binding is registered and exact semantic-vector tail
  ownership matches isolated old-suffix analysis. The richer prefix itself is
  not reparsed or reanalyzed; edits inside it use full analysis. Suffix forms may
  interleave and switch style, and expressions may use earlier typed bindings or
  preserved registered, unit-consistent scalar calls directly, in arithmetic,
  or recursively inside scalar arguments. Explicit and `const` result
  dimensions must match their annotations.
  Full checks recursively validate those nested calls, including calls inside
  parenthesized scalar arithmetic and built-in arguments, underline the innermost
  invalid argument or unknown function name, ignore call-like string contents,
  and suppress a duplicate unresolved outer-argument diagnostic. Dimensionless
  math helpers and numeric percentile calls retain built-in status without hiding
  arbitrary unknown names that happen to begin with `p`. Shared argument and
  option parsing also keeps commas, semicolons, parentheses, and arithmetic
  symbols inside quoted or escaped string content, including process `env` values.
  The compiler-owned dimensionless math-function catalog now also drives LSP
  built-in tokens, completion, generated editor metadata, and TextMate first-paint
  scopes. Component `predictor(...)` and call-style `predict(...)` share the solver
  role without changing command-style model prediction.
  VS Code extension checks now load the installed TextMate engine, including
  ASAR-based installs, and compare role-bearing first-paint scopes against
  compiler semantic snapshots for every example. Report/test/solver block words,
  line-anchored `with` options, DB `.table(...)` methods, case-policy
  constants, and `apply ... over` operands retain the same role before and
  after semantic tokens arrive. Solver connections also keep `connect ... to ...`
  in the solver palette, while every standalone `with {` retains its structural
  workflow role alongside any command-domain modifier.
  Percentile calls now have one compiler-owned `p1` through `p100` integer
  contract shared by semantic analysis, runtime materialization, uncertainty
  propagation, JIT planning, LSP, native IDE first-paint, generated editor
  metadata, and TextMate. Leading zeroes such as `p05` are valid; `p0`, `p101`,
  decimal forms, and unrelated `p`-prefixed calls are not built-ins.
  Parser-lowered command expressions retain their command provenance, so the
  internal render lowering no longer makes user-written `render(...)` look like
  a callable built-in.
  This repairs absolute spans and line numbers
  after variable-width, inserted, or removed standalone trivia and suffix
  line-ending changes. Coordinated declaration and alias renames are accepted when
  names remain unique and every reference resolves backward in source order.
  Declaration additions/removals, complete clearing, and restart from trivia-only
  text update inferred, constant, and expected records, shared semantic vectors,
  syntax counts, and the first workflow line together. Imported constants and
  function contracts keep their source ownership; constants remain available to
  root aliases and arithmetic, eligible imported constants may use scalar calls in
  arithmetic, and eligible functions remain available to all root declaration forms.
  Full semantic analysis also carries the dimension of pure scalar arithmetic
  over typed aliases into inferred types, hover text, type information, and unit
  derivations even when the expression itself has no unit literal; declaration
  names disambiguate registered temperature, power, and dimensionless quantity
  families, with an operand type retained when the result remains ambiguous.
  Token-bearing
  non-declaration lines, incomplete or duplicate renames, forward or unresolved
  references,
  dimensionally incompatible arithmetic, invalid or unsupported calls, workflow
  expressions, diagnostics,
  static imports that contribute non-scalar functions or other definitions,
  import-line edits, imports inside the affected suffix, caches, and richer
  language use full analysis.
  Source changes invalidate recursive open import dependents while preserving
  the changed document as a candidate for these narrow reuse paths. This is not
  general partial parsing or a public cross-release protocol commitment.

## Supported But Narrow Main Behavior

Some behavior on `main` is usable and tested in narrow scopes but is not part
of the stable public package contract. These entries must keep their limits
visible:

- Parenthesis-light command-style built-in verbs, owner-local `where`, and
  `with` blocks for documented built-in workflow commands.
- `eng fmt <file.eng>` source-preserving formatter for current syntax and
  official examples.
- Data-quality policies for documented examples.
- Bar and histogram plot paths used by current report/PlotSpec tests.
- Minimal `system`/`eq`, source-equation ODE, typed-block state-space, and
  constrained component residual examples when documented as narrow scopes.
- Class/domain object authoring for typed fields/defaults, object literals,
  validation, metadata methods, copy-with metadata, diagnostics, report/review
  serialization, IDE summaries, and LSP metadata.

Detailed solver, state-space, component, JIT, uncertainty, ML, and LSP tracks
are tracked in [main_internal_status.md](main_internal_status.md) and
[tracks.md](tracks.md).

## Checklist Track Scope Snapshot

The uncertainty, reviewability, and composite workflow checklist work should
move implementation tracks toward evidence-backed support without widening the
public package claim prematurely. The current test and CI mapping for these
tracks is recorded in [test_ci_gates.md](test_ci_gates.md) so fixture evidence
is not mistaken for public module support.

- Uncertainty / distribution numeric
  - Public package: `Internal`
  - Main status: `Internal`
  - Current handling: scalar runtime numeric payloads distinguish
    Certain/uncertain representations; narrow measured/interval arithmetic
    propagation,
    validated `with { uncertainty = ... }` policy metadata, review summary
    and propagation sections, direct-compare diagnostics, explicit
    statistic/probability validation type-checking, runtime pass/fail
    materialization for explicit statistic/probability/between validations,
    pointwise TimeSeries `sensor_std` review metadata, runtime mean/integration
    `sensor_std` propagation artifacts, confidence-band PlotSpec rendering, and
    internal IDE metadata are reviewable.
  - Keep internal until full probabilistic TimeSeries uncertainty
    propagation, full IDE projection, and tests align.
- Reviewability / Review IR
  - Public package: `Stable` artifact family, `Internal` ReviewDocument
  - Main status: `Supported` artifacts, `Internal` normalized IR slice
  - Current handling: keep current `review.json`/`report.html` public;
    `review.json.review_document`, `eng review`, and IDE Review inspector
    cover the first risk/fallback/external-boundary slice. Saved runs add
    nested runtime results for core inputs, schemas, scalar values,
    materialized tables, TimeSeries, explicit coverage checks, source-derived
    time axes, calculations, table transforms, outputs, validations,
    generated-file side effects, native SQLite write side effects, and native
    model/model-card/metric/prediction bindings, then refresh changed section
    hashes while preserving unchanged static hashes. Model rows include
    computed coefficients, metrics, train/test counts, and model/training
    hashes; prediction rows include input/model identity, output schema, case
    IDs, row count, and prediction hash.
    Runtime-generated `report.html` validates that final document and uses
    it for the review fingerprint, core runtime result/evidence table,
    TimeSeries/coverage/side-effect/model/prediction counts, sample summaries,
    file, DB, model, and prediction evidence, and full validation expressions.
  - One compiler-owned item-level semantic diff payload is available through
    `eng review --against`, `eng review diff`, and the native IDE Review panel.
    Specialized solver, assembly, and non-write DB runtime rows remain
    follow-up projection work.
- Composite workflow foundations
  - Public package: `Supported` side-effect primitives
  - Main status: `Supported` path/io/process/test/profile, promoted table
    diagnostics, native HTTP(S) network/cache boundaries with pinned
    offline-response and live/cache-replay paths, native JSON API payload
    contract promotion in the weather workflow, native sample-table artifacts,
    native template-rendered case input artifacts, optional JSON/TOML config
    field policy, native SQLite append/upsert/replace write records in
    `typed_payload.db_manifests[]`, typed SQLite table readback in
    `typed_payload.structured_reads[]`, output-manifest
    `artifact_registry` summaries, model specs/cards in
    `typed_payload.model_specs[]` and `typed_payload.model_cards[]`,
    native predict-table records and prediction manifests in `typed_payload.prediction_manifests[]`, model
    diagnostics in `typed_payload.model_diagnostics[]`, and native workflow
    artifact evidence for weather/case/model/prediction/DB manifest contracts.
    Native case execution includes calculation-hash/result-SHA output resume
    and verified local result-cache replay/repair. Planned work includes
    process/model cache replay, cross-artifact invalidation, shared or remote
    case caches, automatic external-adapter dispatch,
    broad DB query/engine support, and broader model train syntax.
  - Current native DB evidence includes schema diagnostics, transaction status,
    table names, modes, keys, row counts, source hashes, and report-visible
    DB table summaries. Current native model evidence includes preferred
    `train regression`, warning-producing compatibility aliases
    `regression_table(...)`/`train_regression(...)`, `model_card`,
    `evaluate`, `predict ... using ...`, ModelSpec/FeatureSpec/TargetSpec summaries,
    prediction schema/output metadata, confidence-column metadata, and hashes.
  - Keep domain adapters layered above generic module contracts and avoid
    treating domain-specific adapters as core language identity.

## Planned Tracks

- General table derived-value execution, fill transforms, and arbitrary TimeSeries expression execution.
- Quantity/unit-literal Args conversion and flag-only booleans.
- Multi-return functions, package/module imports, and full formatter policy.
- Native composite workflow modules beyond the current pinned/live
  network/cache boundary,
  sampling, table-regression, prediction, template, and SQLite write/readback
  support: process/model cache replay, cross-artifact invalidation, shared or
  remote case caches, automatic external-adapter
  dispatch, broad database query/engine support, and broader public model
  train/predict workflows.
- Broad nonlinear/DAE/adaptive/component solving beyond the documented narrow
  paths.
- Production multi-domain component simulation and pressure-drop packages.
- Stable public cross-release editor protocol compatibility commitment.
- Native JIT/AOT code generation and measured speedups.
- Broader network/download auth policy, broad filesystem mutation, and full
  process sandboxing.

## Current Crate Architecture

| Crate | Role |
|---|---|
| `eng_cli` | CLI commands, package/release smoke paths, user-facing execution |
| `eng_compiler` | Lexer, parser, AST, semantic checks, units, quantities, bytecode metadata |
| `eng_jit` | Internal hot-kernel detection and numeric lowering-plan metadata |
| `eng_runtime` | Runtime execution, VM, CSV/data policies, `.engres` output |
| `eng_report` | PlotSpec/SVG/report/review rendering and artifact schemas |
| `eng_ide` | Portable native IDE and package smoke UI checks |
| `eng_lsp` | Internal persistent editor service, compatibility CLI endpoints, smoke checks, and metadata JSON |

Future crate splitting should be documented as planned work, not assumed as the
current architecture.
