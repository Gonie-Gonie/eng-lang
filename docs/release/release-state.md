# Release State Audit

Last checked: 2026-06-18, Asia/Seoul.

This file records the release-publication state that should be true for the
initial clean public package. It intentionally avoids treating historical tags,
local readiness notes, or solver-centered internal fixtures as released product
support.

## Public Release Line

`v0.1.0` is the only public release label for this cleanup.

The release is complete only after all of these are true:

```text
1. .\dev.bat release-check passes.
2. Historical local and remote tags are deleted.
3. The local and remote git tag list contains v0.1.0.
4. A GitHub Release page exists for v0.1.0.
5. The portable zip, checksum, user guide PDF, and release manifest are attached.
```

## Historical Tags

The repository previously had preview, alpha, stable, and patch tags such as
`v0.1-preview`, `v0.7-alpha`, `v1.0-stable`, `v1.0.0`, `v1.0.1`, and `v1.0.2`.
Those labels are historical readiness markers, not the clean public package line.

For this release, remove those tags locally and remotely before publishing
`v0.1.0`.

## Solver Claim Boundary

Do not describe this release as a general engineering solver release.

The public package supports the workflows validated by the official examples and
release smoke checks. Solver- and system-oriented examples live under
`examples/internal` and are maintained as implementation fixtures. They are useful
for regression coverage, but they are not public claims of broad ODE, DAE,
nonlinear, adaptive, state-space, or multi-domain component solver support.

## Assets

Expected package assets:

```text
dist\englang-v0.1.0-windows-x64.zip
dist\englang-v0.1.0-windows-x64.zip.sha256
dist\englang-user-guide-v0.1.0.pdf
dist\release-manifest.txt
```

The package may also include the VS Code extension bundle under
`dist\englang\tools\` when the local packaging environment can prepare it.
