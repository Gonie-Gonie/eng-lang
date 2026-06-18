# Version Plan

EngLang separates public release versions from long-term development tracks.

## Public Release Line

The current public line is:

```text
v1.0.0
```

Existing high-numbered release names are not part of the current public line.
Historical tags may remain for traceability; current public release line starts
from `v1.0.0`. Current docs and release assets should not ask users to
understand old high-numbered labels.

Publication status is tracked separately in
[release-state.md](../release/release-state.md). In particular, `v0.1.0` was
not a public release. The first public prerelease was `v0.1-preview`, using
workspace version `0.1.0-preview`.

Public sequence:

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
v1.0.0        stable core
```

Release notes and local checklists may describe historical readiness slices.
Treat a slice as published only when [release-state.md](../release/release-state.md)
records a GitHub Release page for the exact tag.

## Cargo Version

Cargo uses the SemVer-compatible workspace package version:

```text
1.0.0
```

Release assets use the public label:

```text
englang-v1.0.0-windows-x64.zip
englang-user-guide-v1.0.0.pdf
```

## Stable Core Goal

The stable line follows the integrated language philosophy:

```text
typed data boundary
-> typed Table/TimeSeries
-> metrics, validation, and summaries
-> PlotSpec/report/review
-> IDE inspection
-> standalone package
```

`1.0.0` stabilizes the documented data-to-report workflow, artifact family,
packaged runner, and Tauri tester path. It does not stabilize every track on
`main`; see [stable_core_scope.md](stable_core_scope.md).

## Historical Implementation Steps

These labels document historical implementation slices. They are not the active
public release sequence, and they should not be used as user-facing feature
names.

```text
v0.2-preview
  - tester IDE and documentation hardening
  - top-level/function/import/command/where/with policy documented
  - unit-aware print and explicit summary CSV export documented
  - integrated language philosophy documented
  - side-effect/general programming policy documented

v0.3-preview
  - syntax/dataflow unification
  - path type/helper seed implemented
  - first side-effect provenance seed for environment-dependent checks
  - IDE visibility for command lowering, where locals, and outputs
  - official path-policy mini example

v0.4-preview
  - read-only UTF-8 text/json/toml policy seed
  - multi-source data path using typed path args plus raw config/text reads
  - source hash provenance hardening

v0.5-preview
  - write text/json seed with explicit target policy
  - summary CSV overwrite hardening
  - output manifest for generated artifacts

v0.6-preview
  - copy/move/delete policy seed
  - destructive operations require explicit confirmation metadata
  - side-effect manifest grows beyond generated outputs

v0.7-preview
  - log debug/info/warn/error syntax seed
  - run log artifact for structured CLI/debug messages
  - print/log formatting policy hardening

v0.8-preview
  - external process policy seed
  - `ProcessResult` metadata object
  - command/cwd/args/exit/stdout/stderr review records

v0.9-preview
  - test/assert/golden policy seed
  - local workflow test metadata
  - golden artifact comparison workflow
```

## Stable Core Gate

`v1.0.0` requires:

```text
- syntax and core semantics have a documented breaking-change policy
- stable scope is explicitly documented
- Supported, Internal, and Planned work is separated from stable-core support
- official examples pass
- current status and maturity docs match implementation
- portable zip works cleanly
- tester IDE or CLI+report workflow is stable enough for users
- artifact version headers are documented
```

## Track Naming

Do not use public version numbers as feature names:

```text
Do not say: v1.1 = Uncertainty
Do not say: v1.2 = ML
Do not say: v2.0 = Domain/Component
```

Use tracks instead:

```text
Uncertainty track
Data-driven modeling track
IDE/LSP track
Runtime optimization/JIT/AOT track
Domain/component track
```

A future minor release may include early work from a track without making the
whole track stable or release-supported.

## README Rule

The repository root README should expose:

```text
Current public line
Active target
Stable core workflows
Future tracks
Quick start
Verification
```
