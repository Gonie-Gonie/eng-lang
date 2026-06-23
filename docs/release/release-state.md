# Release State Audit

Last checked: 2026-06-19, Asia/Seoul.

This file records the release-publication state that should be true for the
initial clean public package. It intentionally avoids treating historical tags,
local readiness notes, or solver-centered internal fixtures as released product
support.

## Public Release Line

`v0.1.0` is the only public release label for this cleanup.

Observed state:

```text
[x] Historical local and remote tags are deleted.
[x] The local and remote git tag list contains v0.1.0 only.
[x] A GitHub Release page exists for v0.1.0.
[x] The GitHub Release is published, not draft, and not prerelease.
[x] The portable zip, checksum, user guide PDF, and release manifest are attached.
```

GitHub Release URL:

```text
https://github.com/Gonie-Gonie/eng-lang/releases/tag/v0.1.0
```

Published timestamp:

```text
2026-06-18T09:34:34Z
```

The tag and release assets point at commit `504f062`. Later solver-centered
commits on `main` are unreleased implementation work until a new package/tag is
published.

## Historical Tags

The repository previously had preview, alpha, stable, and patch tags such as
`v0.1-preview`, `v0.7-alpha`, `v1.0-stable`, `v1.0.0`, `v1.0.1`, and `v1.0.2`.
Those labels are historical readiness markers, not the clean public package line.

For this release, remove those tags locally and remotely before publishing
`v0.1.0`.

## Solver Claim Boundary

Do not describe this release as a general engineering solver release.

The public package supports the workflows validated by the core official
examples and release smoke checks. Solver- and system-oriented examples live in
the source repository under `examples/advanced_solver` or `examples/internal`.
They are useful for regression coverage, but they are not portable package
tutorials or public claims of broad ODE, DAE, nonlinear, adaptive, state-space,
or multi-domain component solver support.

## Assets

Published package assets:

```text
dist\englang-v0.1.0-windows-x64.zip
dist\englang-v0.1.0-windows-x64.zip.sha256
dist\englang-user-guide-v0.1.0.pdf
dist\release-manifest.txt
```

The package may also include the VS Code extension bundle under
`dist\englang\tools\` when the local packaging environment can prepare it.
