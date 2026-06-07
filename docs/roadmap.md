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
v0.2-preview  IDE and documentation hardening
v0.3-preview  next focused preview scope
...
v1.0          stable core
```

`v0.x-preview` does not mean every implemented feature is stable. It means
users can download a package, run official examples, inspect artifacts, and
give feedback on the current preview scope.

## Current Public Line

`v0.1-preview` packages the current user-test workflow:

```text
- eng.exe doctor/check/run/build/view
- typed CSV promote
- unit-aware TimeSeries calculation
- TimeSeries statistics and integration metadata
- PlotSpec/SVG output
- review/report artifacts
- basic packaged execution
- native tester IDE
- curated user PDF
```

Known public-preview boundary:

```text
- language and artifact formats are not stable
- uncertainty and ML examples are future-track smoke paths
- LSP/VS Code is a secondary preview path
- JIT/AOT has planning metadata only, no speedup claim
- domain/component work is metadata-first, no numeric multi-domain solver
```

## Active Target

`v0.2-preview` focuses on hardening the user-test experience:

```text
- native IDE usability
- native IDE settings and layout quality
- curated user documentation
- clearer supported-preview vs future-track language
- package smoke in clean folders
- fewer stale version references in public docs
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
