# Version Plan

EngLang separates public release versions from long-term development tracks.

## Public Release Line

The current public line is:

```text
v0.8-preview
```

Existing high-numbered release names are not part of the current public line.
They may remain in git history, but current docs and release assets should not
ask users to understand them.

Recommended public sequence:

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
v1.0          stable core, only after the core behavior is ready
```

## Cargo Version

Cargo requires SemVer-compatible package versions, so the workspace package
version for `v0.8-preview` is:

```text
0.8.0-preview
```

Release assets use the shorter public label:

```text
englang-preview-v0.8-preview-windows-x64.zip
englang-user-test-guide-v0.8-preview.pdf
```

## Current Preview Goals

The version line follows the integrated language philosophy:

```text
typed data boundary
-> typed Table/TimeSeries
-> system/component simulation inputs and outputs
-> metrics, validation, calibration
-> PlotSpec/report/review
-> IDE inspection
-> standalone package
```

Current planning targets:

```text
v0.2-preview
  - native IDE and documentation hardening
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
  - preview project test runner metadata
  - golden artifact comparison workflow
```

## v1.0 Reservation

`v1.0` is reserved for a genuinely stable core. It should not be used until:

```text
- syntax and core semantics are unlikely to churn
- fast `=`, dimensionless policy, schema/promote, TimeSeries, PlotSpec, and
  report artifacts are documented and tested
- official examples pass
- current status and maturity docs match implementation
- portable zip works cleanly
- tester IDE or CLI+report workflow is stable enough for users
- artifact version headers are documented
- breaking-change policy is documented
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

A preview release may include early work from a track without making the whole
track stable or release-supported.

## README Rule

The README should show only:

```text
Current public line
Active target
Supported preview workflows
Future tracks
```

Detailed planning belongs in `docs/roadmap.md`, this file, and
`docs/current/philosophy.md`, `docs/current/tracks.md`, and focused reference
docs such as `docs/reference/side_effect_policy.md`.
