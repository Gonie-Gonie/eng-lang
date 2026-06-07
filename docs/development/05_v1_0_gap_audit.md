# v1.0 Gap Audit and Hardening Register

This audit compares the v9 master plan against the repository state after the
v1.0.0 release. Its purpose is not to move completed tags. It records the places
where the implementation is intentionally shallow, seed-only, or missing a gate
that should be hardened before or alongside v1.1.

## Status Terms

```text
Implemented
  The behavior exists, is covered by tests or smoke checks, and is documented.

Seed
  The public shape exists, but runtime semantics, materialized values, or full
  validation are intentionally partial.

Gap
  The plan expected this by the current stage, but the repo only has a weaker
  substitute or no automated gate.

Deferred
  The master plan explicitly places the feature after v1.0.
```

## Executive Summary

v1.0.0 correctly covers the four accepted v1.0 pillars:

```text
1. typed data boundary
2. plotting/report artifacts
3. minimal system/equation metadata
4. packaged standalone execution
```

The main issue is that several pillars are stable as artifact contracts, not as
complete numeric engines. This is acceptable if the docs keep saying so, but the
hardening queue must be explicit.

Highest priority hardening:

```text
P1  docs/spec code block gate for supported docs
P1  official examples directory and regression policy
P1  row-level typed table and TimeSeries value materialization
P1  real statistics kernels for mean/min/max/p95/integrate
P1  Args struct parsing and CLI help for standalone bundles
P2  plot block option execution and non-line plot seeds
P2  schema constraints and missing policy execution
P2  system/equation IR beyond residual metadata
P2  review/report schema validation snapshots
```

## Gap Register

### G-001 Docs Code Block Gate

Status: Implemented after v1.0.0 as a v1.0 hardening backfill.

Plan expectation:

```text
v1.0 release gate includes spec code block checks.
Accepted principle: spec code blocks are checked in CI.
```

Current state:

```text
- release-check runs ci, docs-check, and package-smoke
- docs-check extracts supported Eng snippets from README and supported docs roots
- docs-check validates current snippets and expected-failure snippets
- master-plan snippets remain excluded because they include future roadmap syntax
```

Risk:

```text
P1. Documentation can drift from supported syntax without CI catching it.
```

Hardening detail:

```text
1. [x] Add a docs-check command.
2. [x] Check only supported-doc roots first:
   - README.md
   - docs/specs
   - docs/reference
   - docs/guide
   - docs/tutorials
   - docs/architecture
   - docs/runtime
3. [x] Exclude docs/master-plan from executable snippet CI.
4. [x] Allow fenced blocks to opt out with an explicit marker such as:
   `eng future` or `eng partial`.
5. [x] Add docs-check to release-check once the current supported docs pass.
```

### G-002 Official Examples Layout

Status: Implemented after v1.0.0 as a v1.0 hardening backfill.

Plan expectation:

```text
examples/official/01_csv_plot/
examples/official/02_simple_system/
official examples are regression tested
```

Current state:

```text
- examples/official/01_csv_plot is the official CSV/report/plot example
- examples/official/02_simple_system is the official simple system example
- eng test examples runs these examples
- legacy numbered examples remain as compatibility smoke cases
```

Risk:

```text
P1. The examples are tested, but users and release automation do not have a
single official namespace to copy, package, or document.
```

Hardening detail:

```text
1. [x] Create examples/official/01_csv_plot.
2. [x] Create examples/official/02_simple_system.
3. [x] Keep numbered legacy examples or redirect docs to official examples.
4. [x] Update package-smoke to run official examples.
5. [x] Keep old examples as compatibility smoke cases until v1.1 or v1.2.
```

### G-003 Typed Table Runtime Values

Status: Implemented for the official CSV runtime path after v1.0.0 hardening.
General table-expression execution remains a follow-up.

Plan expectation:

```text
CSV -> schema -> typed Table object
row-level column parse
source hash provenance
```

Current state:

```text
- CSV header validation and source hash exist
- schema/promotion metadata appears in review/report/result
- VM table object is backed by RuntimeTable pages for promoted CSV data
- DateTime index values and numeric quantity columns are parsed into result.engres
- row count, column values, missing counts, parse failures, and source hash are recorded
```

Risk:

```text
P1 closed for the official CSV+coil path. Broad expression VM support is still
needed before arbitrary table formulas can claim full runtime execution.
```

Hardening detail:

```text
1. [x] Add a RuntimeTable value with rows, typed columns, and source provenance.
2. [x] Parse DateTime index values into stable seconds offsets for runtime use.
3. [x] Parse numeric/unit columns into typed numeric arrays for the official path.
4. [x] Report row count, parse failures, and missing values in result.engres.
5. [x] Add tests for the official CSV runtime page and computed values.
6. [ ] Add dedicated bad DateTime and bad numeric cell fixtures.
7. [ ] Add unit conversion failure reporting once per-cell unit conversion exists.
```

### G-004 Statistics Kernels

Status: Implemented for mean/max/min/p95 and trapezoidal integrate on the
official HeatRate TimeSeries path. duration_above remains deferred.

Plan expectation:

```text
mean, max/min, p95, integrate, duration_above seed, lazy summary
```

Current state:

```text
- TimeSeries type metadata exists
- summary/integration metadata exists
- HeatRate sum lint exists
- result payload records computed statistics for Q_coil
- report_spec.json records computed statistics and integrations
- integrate(Q_coil, over=Time) records a trapezoidal Energy value
- duration_above is not implemented
```

Risk:

```text
P1 closed for the official data-analysis path. Remaining risk is limited to
unsupported statistics and arbitrary TimeSeries expressions.
```

Hardening detail:

```text
1. [x] Build TimeSeries pages from RuntimeTable columns.
2. [x] Implement min/max/mean/p95 kernels for numeric series.
3. [x] Implement integrate(HeatRate over Time) with DateTime-derived seconds.
4. [x] Store computed values in result.engres and report_spec.json.
5. [ ] Add duration_above as a v1.0.x or v1.1 backfill if it is needed before uncertainty.
```

### G-005 Plot Data Materialization

Status: Implemented for the official CSV line plot. Bar/histogram remain
deferred.

Plan expectation:

```text
PlotSpec v1, line plot, bar plot seed, histogram seed, axis labels, SVG export
```

Current state:

```text
- PlotSpec v1, SVG, manifest, and unit-aware labels exist
- line plot is generated
- points are generated from runtime TimeSeries pages for the official CSV path
- plot title and y-axis unit options are applied
- bar/histogram remain deferred
```

Risk:

```text
P2 closed for the official CSV line plot. Remaining plot risk is around
additional plot types and broader plot block semantics.
```

Hardening detail:

```text
1. [x] Generate PlotSpec points from runtime TimeSeries pages.
2. [x] Execute plot title and y-axis unit options from the plot block.
3. [x] Add golden checks for real CSV-derived points.
4. [ ] Add bar/histogram seeds only after numeric pages exist.
```

### G-006 Args Struct and Standalone CLI Help

Status: Metadata/help base implemented after v1.0.0. Runtime flag binding is
still deferred.

Plan expectation:

```text
script main(args: Args) is the official entry point.
Args type drives CLI help and standalone interface.
```

Current state:

```text
- script entry metadata records arg name/type
- struct Args fields/defaults are parsed as Args metadata
- review.json, report_spec.json, result.engres, and .engpkg record Args metadata
- standalone bundles include ARGS_HELP.txt
- run.bat --help prints Args metadata
- standalone run.bat still forwards extra args to eng.exe run
- eng run does not map --input or other flags into args
```

Risk:

```text
P1. Packaged execution works for fixed-source examples, but not yet for
user-configurable model packages.
```

Hardening detail:

```text
1. [x] Parse struct declarations needed for Args.
2. [x] Record Args fields and defaults in review/result/package metadata.
3. Add eng run flag binding from Args fields.
4. [x] Generate standalone run.bat help or package help from Args metadata.
5. [partial] Add clean-folder tests for --help and a user-provided CSV path.
```

### G-007 Schema Constraint and Missing Policy Execution

Plan expectation:

```text
missing policy seed
constraint seed
row-level constraint execution eventually follows the typed boundary
```

Current state:

```text
- constraints and missing policies are parsed/recorded
- missing policy references are checked against schema columns
- row-level constraints and interpolation are not executed
```

Risk:

```text
P2. Users can write policies that appear in review artifacts but do not affect
runtime data yet.
```

Hardening detail:

```text
1. Add policy status fields: recorded, validated, executed.
2. Surface non-executed policies as warnings in report/review until implemented.
3. Implement m_dot >= 0 style row checks.
4. Implement missing value error policy before interpolation.
```

### G-008 System/Eq IR and Solver Boundary

Plan expectation:

```text
minimal system/equation in v1.0
full solver explicitly deferred
```

Current state:

```text
- system, parameter/state/input, equation, eq, der() parse/check path exists
- unit consistency diagnostics exist
- residual metadata appears in review/report/result
- no symbolic IR graph, solve order, ODE runner, or Jacobian seed yet
```

Risk:

```text
P2. v1.0 can represent a simple system, but cannot simulate it.
```

Hardening detail:

```text
1. Add a small symbolic equation IR separate from report strings.
2. Record state/input/parameter dependencies for each residual.
3. Add a solver_boundary section to result/review that explicitly says unsolved.
4. Keep numeric solver work deferred until the planned system/solver milestone.
```

### G-009 Review/Report Schema Validation

Status: Implemented after v1.0.0 as a v1.0 hardening backfill.

Plan expectation:

```text
reviewable artifacts, version/format headers, stable report/review path
```

Current state:

```text
- review.json, report_spec.json, report.html, result.engres exist
- unit tests assert key sections
- docs/schemas defines the v1.0 structural artifact baselines
- tests/golden/artifacts records official example golden expectations
- artifacts-check validates official CSV/plot, simple-system, PlotSpec, and engpkg artifacts
```

Risk:

```text
P2. Artifact contracts can drift while still passing loose contains-based tests.
```

Hardening detail:

```text
1. [x] Add docs/schemas for review/report/result/plotspec/engpkg.
2. [x] Add golden snapshots for official examples.
3. [x] Add hash-stable normalization for paths and generated timestamps.
4. [x] Add schema validation to release-check.
```

### G-010 Release/CI Matrix

Plan expectation:

```text
portable release package, release gate, downloadable artifacts
```

Current state:

```text
- release workflow exists and publishes v1.0.0 assets
- Windows release path is verified
- Linux/macOS are intentionally outside the initial release gate
- docs-check is in release-check
- artifacts-check schema/golden validation is in release-check
```

Risk:

```text
P2. Release automation is real, but release quality gates are still mostly
Windows smoke tests plus Rust tests.
```

Hardening detail:

```text
1. [x] Add docs-check after G-001.
2. [x] Add schema/golden artifact validation after G-009.
3. Keep Windows-only release gate until cross-platform policy changes.
4. Add a release post-check for downloaded zip checksum and doctor.
```

## Recommended Order Before v1.1

Do this before adding uncertainty semantics:

```text
1. G-001 docs-check command and supported-doc snippet policy
2. G-002 official examples namespace
3. G-009 artifact schema/golden validation baseline
4. G-006 Args metadata inventory, even if full flag binding waits
```

Then proceed with v1.1 while planning numeric/data backfill:

```text
5. G-003 RuntimeTable values
6. G-004 real statistics kernels
7. G-005 real PlotSpec points
```

Keep deferred until the appropriate later milestones:

```text
8. G-008 numeric solver, Jacobian, and ODE runner
9. optimized AOT/model.exe
10. open domain/port and package ecosystem
```

## Release Note Correction

The v1.0.0 release remains valid as a stable artifact-contract release. Future
docs should avoid implying that v1.0 includes full numeric statistics, full
typed table runtime execution, Args-derived flag binding, or numeric system
simulation.
