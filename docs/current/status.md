# Current Project Status

This page is the authoritative short-form status layer for contributors and LLM
agents. It describes the public package first. Internal implementation seeds
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
- Typed CSV promotion for the official schema/data boundary.
- DateTime-indexed table metadata, row-level CSV runtime pages, source hash
  provenance, `typed_payload.table_diagnostics[]` summaries for promoted data,
  `typed_payload.table_selections[]` records for deterministic promoted-table row
  selection, `typed_payload.table_transforms[]` records for filter/select/derive/sort/require_one/join
  row counts, Date/DateTime predicate comparison, selected columns, derived
  columns, sort keys, predicate evidence, join key pair counts, and row-level diagnostics,
  `typed_payload.timeseries_coverage[]` records for DateTime-indexed
  promoted-table coverage, `typed_payload.timeseries_quality[]` coverage/fill
  summaries, `typed_payload.expectation_suites[]` lightweight expectation-suite
  records, `typed_payload.quality_results[]` common quality records for
  TimeSeries, validation, schema-constraint, and expectation-suite results,
  row/field failure details, report-facing `report_spec.quality_report`, HTML
  Quality Report tables, and IDE Quality inspector payloads,
  `typed_payload.time_alignments[]` alignment/resampling hook records,
  `typed_payload.sample_tables[]` summaries for generated and promoted
  sample/case tables,
  and `typed_payload.case_manifests[]` case row manifests with sample row hashes
  and process-output enrichment.

### TimeSeries, Plot, Report, And Review

- Unit-aware TimeSeries calculation on the documented public paths.
- TimeSeries statistics: mean, time-weighted mean, median, standard deviation,
  percentiles, duration-above metadata, and trapezoidal integration.
- PlotSpec v1 line plots, measured-vs-simulated multi-series line plots, SVG
  output, plot manifests, report HTML, review JSON, report spec, and result
  artifacts.
- Unit-aware `print` and explicit one-row summary CSV export.

### Validation Workflow

- Measured-vs-simulated workflow for the documented scope:
  weather/measured CSV promotion, explicit `TimeSeries[Time]` input contract,
  typed simulation TimeSeries output, RMSE metric, validation result,
  time-alignment metadata, and multi-series PlotSpec.

This validation workflow demonstrates how simulation output can become typed
review material. It is not a broad solver claim.

### Explicit Side Effects

- Typed path helpers: `file`, `dir`, `join`, `parent`, `stem`, `extension`.
- Provenance-visible `exists`.
- Read-only UTF-8 `read text`, `read json`, and `read toml` expressions with
  source-relative resolution and source hash provenance.
- Explicit `write text/json`, constrained copy/move/delete file operations,
  native `render template` generated inputs with render manifests, CSV
  overwrite hardening, and `output_manifest.json`.
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
- Native Tauri/WebView tester IDE smoke path for open/check/save/run,
  diagnostics, variable summaries, schema/TimeSeries/metric/validation
  inspectors, module registry browser, table transform inspector rows, PlotSpec
  viewing, report opening, and side-effect artifact panels.
- Packaged LSP smoke/snapshot binary for internal editor tooling. It is not a
  stable persistent editor-service contract.

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

Detailed solver, state-space, component, JIT, uncertainty, ML, and LSP seeds
are tracked in [main_internal_status.md](main_internal_status.md) and
[tracks.md](tracks.md).

## Checklist Track Scope Snapshot

The uncertainty, reviewability, and composite workflow checklist work should
move implementation seeds toward evidence-backed support without widening the
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
    cover the first risk/fallback/external-boundary slice.
  - CLI item-level semantic diff preview exists; runtime-updated
    ReviewDocument and native IDE diff panel remain planned.
- Composite workflow foundations
  - Public package: `Supported` side-effect primitives
  - Main status: `Supported` path/io/process/test/profile, promoted table
    diagnostics, promoted sample-table artifacts, promoted case manifests
    enriched from process outputs, optional JSON/TOML config field policy,
    DB manifest summaries in `typed_payload.db_manifests[]`, output-manifest
    `artifact_registry` summaries, internal model-card summaries in
    `typed_payload.model_cards[]`, and hybrid artifact fixtures
    for weather/case/model/prediction/DB manifest contracts; `Planned` native
    net/cache/sample generators/case runner/sqlite/model public syntax.
  - Current hybrid DB evidence includes schema diagnostics, transaction status,
    table names, modes, keys, row counts, source hashes, and report-visible
    DB table summaries.
  - Keep domain adapters layered above generic module contracts and avoid
    treating hybrid fixtures as native module support.

## Planned Tracks

- General table derived-value execution, fill transforms, and arbitrary TimeSeries expression execution.
- Quantity/unit-literal Args conversion and flag-only booleans.
- Multi-return functions, package/module imports, and full formatter policy.
- Native composite workflow modules beyond the current network/cache/sampling
  seeds: live network execution, cache replay/invalidation, native case runner,
  database writes, and model-card workflows.
- Broad nonlinear/DAE/adaptive/component solving beyond the documented narrow
  paths.
- Production multi-domain component simulation and pressure-drop packages.
- Stable persistent LSP/editor contract.
- Native JIT/AOT code generation and measured speedups.
- Network/download support, broad filesystem mutation, and full process
  sandboxing.

## Current Crate Architecture

| Crate | Role |
|---|---|
| `eng_cli` | CLI commands, package/release smoke paths, user-facing execution |
| `eng_compiler` | Lexer, parser, AST, semantic checks, units, quantities, bytecode metadata |
| `eng_jit` | Internal hot-kernel detection and numeric lowering-plan metadata |
| `eng_runtime` | Runtime execution, VM seed, CSV/data policies, `.engres` output |
| `eng_report` | PlotSpec/SVG/report/review rendering and artifact schemas |
| `eng_ide` | Tauri/WebView tester IDE and package smoke UI checks |
| `eng_lsp` | Internal editor-service smoke and snapshot paths |

Future crate splitting should be documented as planned work, not assumed as the
current architecture.
