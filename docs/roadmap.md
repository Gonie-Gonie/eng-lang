# Roadmap

This roadmap follows the simplified version policy:

```text
Public release versions describe packages.
Long-term capabilities are managed as tracks.
v1.0 is reserved for a genuinely stable core.
```

Use [current status](current/status.md),
[version plan](current/version_plan.md), and the
[feature maturity matrix](current/feature_maturity_matrix.md) as the
authoritative state layer.

## Public Release Sequence

```text
v0.1-preview  first public preview
v0.2-preview  IDE/documentation hardening and integrated philosophy
v0.3-preview  syntax/dataflow unification and path-policy seed
v0.4-preview  read-only I/O and multi-source data policy
v0.5-preview  write/export hardening and output manifest
v0.6-preview  explicit copy/move/delete side-effect policy
v0.7-preview  structured log levels and run log artifacts
v0.8-preview  external process and ProcessResult policy seed
v0.9-preview  test/assert/golden support
...
v1.0          stable core
```

`v0.x-preview` does not mean every implemented feature is stable. It means
users can download a package, run official examples, inspect artifacts, and
give feedback on the current preview scope.

## Current Public Line

`v0.8-preview` packages the current user-test workflow:

```text
- eng.exe doctor/check/run/build/view
- top-level execution, args, const, fn, and file-import policy
- command-style built-in workflow verbs with where/with policy
- typed CSV promote
- unit-aware TimeSeries calculation
- TimeSeries statistics and integration metadata
- unit-aware print and explicit summary CSV export
- typed path helpers and provenance-visible `exists`
- read-only UTF-8 `read text/json/toml` with source hash provenance
- explicit `write text/json`, CSV overwrite hardening, and output manifest
- explicit `copy/move/delete` file operation seed with confirmation metadata
- `print` plus `log debug/info/warn/error` runtime messages with `run_log.json`
- explicit `run command` process execution with `ProcessResult` and
  `process_results.json`
- PlotSpec/SVG output
- review/report artifacts
- basic packaged execution
- native tester IDE
- curated user PDF
- language grammar PDF
```

Known public-preview boundary:

```text
- language and artifact formats are not stable
- uncertainty and ML examples are future-track smoke paths
- LSP/VS Code is a secondary preview path
- JIT/AOT has planning metadata only, no speedup claim
- domain/component work is metadata-first, no numeric multi-domain solver
```

## Integrated Direction

The active philosophy is:

```text
System modeling produces typed TimeSeries.
Data analysis validates, calibrates, summarizes, and explains those TimeSeries.
```

That gives the long-term workflow:

```text
schema/promote
-> typed Table/TimeSeries
-> system/component simulation input
-> typed simulation output TimeSeries
-> metrics/validation/calibration
-> PlotSpec/report/review artifacts
-> IDE visual inspection
-> standalone package
```

Use [integrated language philosophy](current/philosophy.md) as the short-form
policy source.

## Active Target

`v0.9-preview` focuses on test and golden support:

```text
- test/assert/golden syntax policy seed
- project test runner metadata
- golden artifact comparison workflow
- clearer boundary between examples, tests, and release acceptance
```

## Development Tracks

Tracks are described in [development tracks](current/tracks.md):

```text
T1 Core Language
T2 Data Boundary
T3 Statistics / Plot / Report
T4 System / Equation
T5 IDE / LSP
T6 Uncertainty
T7 Data-driven Modeling
T8 Runtime Optimization / JIT / AOT
T9 Domain / Component
T10 Class / Domain Object
T11 General Programming / Side Effects
```

A future preview may include early work from any track, but the track name
should remain separate from the release version.

## v1.0 Stable Core Gate

Do not use `v1.0` until the following are true:

```text
[ ] syntax and core semantics have a documented breaking-change policy
[ ] supported preview features are promoted or explicitly deferred
[ ] official examples pass on clean Windows package smoke
[ ] current status and maturity docs match implementation
[ ] portable zip works cleanly without Rust/Python on the target PC
[ ] tester IDE or CLI+report workflow is stable enough for users
[ ] bytecode/result/report/PlotSpec/package format headers are documented
[ ] release notes state exact stable scope and limitations
```

## Working Rule

Before claiming public support:

```text
1. Pick the public preview scope or development track.
2. Add examples and diagnostics.
3. Add runtime/report/IDE metadata where relevant.
4. Update status, maturity, and user docs.
5. Pass release-check.
```
