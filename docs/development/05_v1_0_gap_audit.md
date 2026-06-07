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

Plan expectation:

```text
v1.0 release gate includes spec code block checks.
Accepted principle: spec code blocks are checked in CI.
```

Current state:

```text
- release-check runs ci and package-smoke
- there is no doc snippet extractor/checker
- master-plan snippets include future syntax and cannot all be checked as v1.0
```

Risk:

```text
P1. Documentation can drift from supported syntax without CI catching it.
```

Hardening detail:

```text
1. Add a docs-check command.
2. Check only supported-doc roots first:
   - README.md
   - docs/specs
   - docs/reference
   - docs/guide
   - docs/tutorials
   - docs/architecture
   - docs/runtime
3. Exclude docs/master-plan from executable snippet CI.
4. Allow fenced blocks to opt out with an explicit marker such as:
   `eng future` or `eng partial`.
5. Add docs-check to release-check once the current supported docs pass.
```

### G-002 Official Examples Layout

Plan expectation:

```text
examples/official/01_csv_plot/
examples/official/02_simple_system/
official examples are regression tested
```

Current state:

```text
- examples/02_csv_plot and examples/04_plotting act as official CSV/report/plot examples
- examples/06_simple_system acts as the official simple system example
- eng test examples runs these examples
- there is no examples/official namespace yet
```

Risk:

```text
P1. The examples are tested, but users and release automation do not have a
single official namespace to copy, package, or document.
```

Hardening detail:

```text
1. Create examples/official/01_csv_plot.
2. Create examples/official/02_simple_system.
3. Keep numbered legacy examples or redirect docs to official examples.
4. Update package-smoke to run official examples.
5. Keep old examples as compatibility smoke cases until v1.1 or v1.2.
```

### G-003 Typed Table Runtime Values

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
- VM table object is a seed object
- row values are not parsed into typed runtime pages
```

Risk:

```text
P1. Current data analysis is strongly typed at the boundary, but numeric
execution still relies on semantic metadata instead of runtime table values.
```

Hardening detail:

```text
1. Add a RuntimeTable value with rows, typed columns, and source span/provenance.
2. Parse DateTime index values into a stable internal representation.
3. Parse numeric/unit columns into typed Quantity arrays.
4. Report row count, parse failures, unit conversion failures, and missing values.
5. Add tests for wrong unit, bad DateTime, and bad numeric cell.
```

### G-004 Statistics Kernels

Plan expectation:

```text
mean, max/min, p95, integrate, duration_above seed, lazy summary
```

Current state:

```text
- TimeSeries type metadata exists
- summary/integration metadata exists
- HeatRate sum lint exists
- result payload marks summaries as lazy
- numeric mean/max/p95/integrate values are not materialized
- duration_above is not implemented
```

Risk:

```text
P1. Reports can describe the requested computation, but cannot yet verify or
display real computed statistics.
```

Hardening detail:

```text
1. Build TimeSeries pages from RuntimeTable columns.
2. Implement min/max/mean/p95 kernels for numeric series.
3. Implement integrate(HeatRate over Time) with unit-aware duration handling.
4. Add duration_above as a v1.0.x or v1.1 backfill if it is needed before uncertainty.
5. Store computed values in result.engres and report_spec.json.
```

### G-005 Plot Data Materialization

Plan expectation:

```text
PlotSpec v1, line plot, bar plot seed, histogram seed, axis labels, SVG export
```

Current state:

```text
- PlotSpec v1, SVG, manifest, and unit-aware labels exist
- line plot is generated
- points are deterministic preview points
- plot block options are not fully executed
- bar/histogram remain deferred
```

Risk:

```text
P2. Plot artifacts are stable, but the visual data is not yet sourced from real
TimeSeries values.
```

Hardening detail:

```text
1. Generate PlotSpec points from runtime TimeSeries pages.
2. Execute plot title and unit options from the plot block.
3. Add snapshot tests for real CSV-derived points.
4. Add bar/histogram seeds only after numeric pages exist.
```

### G-006 Args Struct and Standalone CLI Help

Plan expectation:

```text
script main(args: Args) is the official entry point.
Args type drives CLI help and standalone interface.
```

Current state:

```text
- script entry metadata records arg name/type
- standalone run.bat forwards extra args to eng.exe run
- struct Args is not parsed as a CLI schema
- eng run does not map --input or other flags into args
- standalone --help is not generated from Args
```

Risk:

```text
P1. Packaged execution works for fixed-source examples, but not yet for
user-configurable model packages.
```

Hardening detail:

```text
1. Parse struct declarations needed for Args.
2. Record Args fields and defaults in review/result/package metadata.
3. Add eng run flag binding from Args fields.
4. Generate standalone run.bat help or package help from Args metadata.
5. Add clean-folder tests for --help and a user-provided CSV path.
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

Plan expectation:

```text
reviewable artifacts, version/format headers, stable report/review path
```

Current state:

```text
- review.json, report_spec.json, report.html, result.engres exist
- unit tests assert key sections
- there is no JSON schema file or snapshot/golden validation suite
```

Risk:

```text
P2. Artifact contracts can drift while still passing loose contains-based tests.
```

Hardening detail:

```text
1. Add docs/schemas for review/report/result/plotspec/engpkg.
2. Add golden snapshots for official examples.
3. Add hash-stable normalization for paths and generated timestamps.
4. Add schema validation to release-check.
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
- docs-check and schema validation are not yet in release-check
```

Risk:

```text
P2. Release automation is real, but release quality gates are still mostly
Windows smoke tests plus Rust tests.
```

Hardening detail:

```text
1. Add docs-check after G-001.
2. Add schema/golden artifact validation after G-009.
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
typed table runtime execution, generated Args CLI help, or numeric system
simulation.
