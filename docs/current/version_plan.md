# Version And Release Policy

EngLang separates public package labels from long-term development tracks. Do
not use old milestone names as feature claims.

## Current Public Line

The current published public line is:

```text
v0.1.0
```

Publication state is tracked in [release-state.md](../release/release-state.md).
As of the latest audit, the GitHub Release page for `v0.1.0` exists and has the
portable zip, checksum, user guide PDF, and release manifest attached. The tag
points to the release commit recorded there; newer solver-centered work on
`main` is not automatically part of those release assets.

## Cargo Version

Cargo uses the SemVer-compatible workspace package version:

```text
0.1.0
```

Release assets use the public label:

```text
englang-v0.1.0-windows-x64.zip
englang-user-guide-v0.1.0.pdf
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
Solver-centered implementation track
```

A future public package may include early work from a track without making the
whole track stable or release-supported.

## Historical Notes

Historical preview and high-numbered labels may appear in old planning notes.
Treat them as implementation slices unless [release-state.md](../release/release-state.md)
records a GitHub Release page for the exact tag.

The current docs should name concrete capabilities and maturity levels instead
of asking users to reason from old version ladders.

## README Rule

The repository root README should expose:

```text
Current public line
Active target
Public package scope
Development tracks
Quick start
Verification
```
