# Version Plan

EngLang separates public release versions from long-term development tracks.

## Public Release Line

The public line restarts from:

```text
v0.1-preview
```

Existing high-numbered release names are not part of the current public line.
They may remain in git history, but current docs and release assets should not
ask users to understand them.

Recommended public sequence:

```text
v0.1-preview  first public preview
v0.2-preview  IDE and documentation hardening
v0.3-preview  next focused preview scope
...
v1.0          stable core, only after the core behavior is ready
```

## Cargo Version

Cargo requires SemVer-compatible package versions, so the workspace package
version for `v0.1-preview` is:

```text
0.1.0-preview
```

Release assets use the shorter public label:

```text
englang-preview-v0.1-preview-windows-x64.zip
englang-user-test-guide-v0.1-preview.pdf
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
`docs/current/tracks.md`.
