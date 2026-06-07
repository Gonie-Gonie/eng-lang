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

Completed v1.0 hardening backfills:

```text
P1  docs/spec code block gate for supported docs
P1  official examples directory and regression policy
P1  row-level typed table and TimeSeries value materialization for official CSV
P1  real statistics kernels for mean/min/max/p95/integrate on official TimeSeries
P1  time_weighted_mean, median, std, and pNN statistics for official TimeSeries
P1  Args struct parsing and CLI help for standalone bundles
P2  plot block option execution for official line plots
P2  schema constraints and missing policy execution for official CSV
P2  system/equation IR beyond residual metadata, with explicit solver boundary
P2  fixed-step ODE preview for the official one-state thermal system
P2  review/report schema validation snapshots
P2  per-cell CSV source-unit to canonical-unit conversion diagnostics
```

Remaining intentional deferrals:

```text
P2  adaptive, nonlinear, and multi-equation numeric system solver
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
- numeric quantity columns record canonical_unit, canonical_values, and per-cell conversion failures
- dedicated bad DateTime and bad numeric fixtures are exercised by eng test examples
- unsupported unit conversion fixture records source_unit, target_unit, and row-level failures
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
6. [x] Add dedicated bad DateTime and bad numeric cell fixtures.
7. [x] Add per-cell canonical conversion metadata and unit conversion failure reporting.
```

### G-004 Statistics Kernels

Status: Implemented for mean/time_weighted_mean/max/min/median/std/pNN,
duration_above, and trapezoidal integrate on the official HeatRate TimeSeries
path.

Plan expectation:

```text
mean, max/min, percentile, std, integrate, duration_above seed, lazy summary
```

Current state:

```text
- TimeSeries type metadata exists
- summary/integration metadata exists
- HeatRate sum lint exists
- result payload records computed statistics for Q_coil
- report_spec.json records computed statistics and integrations
- integrate(Q_coil, over=Time) records a trapezoidal Energy value
- time_weighted_mean records trapezoidal integral divided by elapsed seconds
- median, std, and pNN percentile kernels are computed for materialized points
- duration_above(5 kW) records seconds above threshold with linear crossing interpolation
```

Risk:

```text
P1 closed for the official data-analysis path. Remaining risk is limited to
arbitrary TimeSeries expressions, report-card presentation, and richer
quantity outputs such as std(AbsoluteTemperature) -> TemperatureDelta.
```

Hardening detail:

```text
1. [x] Build TimeSeries pages from RuntimeTable columns.
2. [x] Implement min/max/mean/p95 kernels for numeric series.
3. [x] Implement integrate(HeatRate over Time) with DateTime-derived seconds.
4. [x] Store computed values in result.engres and report_spec.json.
5. [x] Add duration_above as a v1.0 hardening backfill before uncertainty.
6. [x] Add time_weighted_mean using trapezoidal integral over elapsed seconds.
7. [x] Add median, std, and generic pNN percentile kernels.
```

### G-005 Plot Data Materialization

Status: Implemented for the official CSV line plot and v1.0 plot-type seeds.
Bar/histogram PlotSpec rendering seeds exist; full binning and multi-series
plot semantics remain deferred.

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
- plot type option can select line, bar, or histogram
- bar/histogram SVG seeds render PlotSpec points as rect-based plots
```

Risk:

```text
P2 closed for the official CSV line plot and bar/histogram seed rendering.
Remaining plot risk is around real histogram binning, multiple series, and
broader plot block semantics.
```

Hardening detail:

```text
1. [x] Generate PlotSpec points from runtime TimeSeries pages.
2. [x] Execute plot title and y-axis unit options from the plot block.
3. [x] Add golden checks for real CSV-derived points.
4. [x] Add bar/histogram seeds only after numeric pages exist.
```

### G-006 Args Struct and Standalone CLI Help

Status: Implemented after v1.0.0 as a v1.0 hardening backfill.

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
- review.json, report_spec.json, and result.engres record resolved arg_values
- standalone bundles include ARGS_HELP.txt
- run.bat --help prints Args metadata
- standalone run.bat still forwards extra args to eng.exe run
- eng run maps `--input data/sensor.csv` style flags into Args values
- promote csv args.input resolves through default or CLI-provided Args values
```

Risk:

```text
P1 closed for string-valued Args fields and CSV path binding. Rich typed
conversion for non-string Args values remains a later expansion.
```

Hardening detail:

```text
1. [x] Parse struct declarations needed for Args.
2. [x] Record Args fields and defaults in review/result/package metadata.
3. [x] Add eng run flag binding from Args fields.
4. [x] Generate standalone run.bat help or package help from Args metadata.
5. [x] Add smoke tests for --help and a user-provided CSV path.
```

### G-007 Schema Constraint and Missing Policy Execution

Status: Implemented for the official CSV path after v1.0.0 hardening.
Numeric bound expression parsing now covers strict and inclusive upper/lower
bounds.

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
- result.engres and report_spec.json record policy_results with recorded/validated/executed status
- time monotonic, between, numeric bound, and missing error policies execute on runtime table pages
- interpolation policies execute on numeric runtime table pages
- constraint violation fixture records an upper-bound violation with row-level detail
```

Risk:

```text
P2 closed for the official CSV policy set and numeric bound expression seeds.
Arbitrary boolean row-expression execution remains outside the v1.0 stable
boundary.
```

Hardening detail:

```text
1. [x] Add policy status fields: recorded, validated, executed.
2. [x] Surface non-executed interpolation policies as warnings in report/review.
3. [x] Implement m_dot >= 0 style row checks.
4. [x] Implement missing value error policy before interpolation.
5. [x] Implement missing value interpolation.
6. [x] Add broader numeric bound constraint expression parsing.
```

### G-008 System/Eq IR and Solver Boundary

Status: Implemented after v1.0.0 as a v1.0 hardening backfill. Solver plan
metadata, source-order solve_order, symbolic Jacobian seed columns, and a
fixed-step ODE preview for the official one-state thermal system are recorded.

Plan expectation:

```text
minimal system/equation in v1.0
fixed time step simple ODE runner OR residual-only report
```

Current state:

```text
- system, parameter/state/input, equation, eq, der() parse/check path exists
- unit consistency diagnostics exist
- residual metadata appears in review/report/result
- a small equation IR is emitted separately from report-facing residual strings
- each residual records parameter/state/input dependencies
- derivative state mentions are recorded per equation
- review.json includes compiler-owned system_ir with solver_boundary.status = unsolved
- report_spec.json upgrades the official simple thermal ODE solver_boundary to computed during run
- result.engres includes typed_payload.solver_boundaries and typed_payload.system_ir
- review system_ir includes solver_plan.status = metadata_only
- report/result system_ir include solver_plan.status = computed for the official ODE preview
- source-order solve_order and symbolic Jacobian seed columns are recorded
- ODE runner status is computed for the official one-state thermal ODE preview
- result.engres records solver_result trajectory points, step count, and final state value
```

Risk:

```text
P2 closed for artifact review, dependency inspection, solver seed metadata, and
the official one-state thermal ODE preview. Remaining solver risk is limited to
adaptive, nonlinear, multi-state, and multi-equation solving.
```

Hardening detail:

```text
1. [x] Add a small symbolic equation IR separate from report strings.
2. [x] Record state/input/parameter dependencies for each residual.
3. [x] Add solver_boundary sections to review/report/result.
4. [x] Add source-order solve_order and symbolic Jacobian seed metadata.
5. [x] Execute a fixed-step ODE preview for the official one-state thermal system.
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

## v1.0 Hardening Closure

Completed before adding uncertainty semantics:

```text
1. G-001 docs-check command and supported-doc snippet policy
2. G-002 official examples namespace
3. G-009 artifact schema/golden validation baseline
4. G-006 Args metadata, resolved values, and CSV path flag binding
5. G-003 RuntimeTable values
6. G-004 real statistics kernels
7. G-005 real PlotSpec points
8. G-007 schema policies, missing policies, and numeric constraint bounds
9. G-008 system IR solver boundary and solver_plan seeds
10. G-003 per-cell unit conversion diagnostics for CSV quantity columns
11. G-008 fixed-step ODE preview for the official simple thermal system
```

Keep deferred until the appropriate later milestones:

```text
1. adaptive, nonlinear, and multi-equation numeric system solver
2. optimized AOT/model.exe
3. open domain/port and package ecosystem
```

## Release Note Correction

The v1.0.0 release remains valid as a stable artifact-contract release. Future
docs should avoid implying that v1.0 includes full numeric statistics, full
typed table runtime execution beyond the official CSV path, rich typed Args
conversion beyond string-valued flags, or numeric system simulation.
