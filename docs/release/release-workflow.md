# Release Workflow

This document defines the repeatable EngLang release process.

## Release Ownership

Release work is separate from milestone implementation:

```text
1. milestone code and docs are completed on main
2. local release gate passes
3. release tag is created and pushed
4. GitHub Actions rebuilds from the tag
5. release assets are attached to the GitHub Release
```

Do not move an existing release tag. If a release needs a fix, create a new
patch tag such as `v1.0.3`. Do not repeat a release workflow while a hardening
slice is still being implemented; keep release notes as draft until the manual
IDE gate is clean.

## Local Release Gate

Run from the repository root:

```bat
.\dev.bat release-check
```

This command runs:

```text
1. dev.bat ci
2. dev.bat docs-check
3. dev.bat ide-check
4. dev.bat artifacts-check
5. dev.bat package-smoke
6. zip existence check
7. SHA256 checksum verification
8. dist/release-manifest.txt generation
```

`docs-check` extracts supported `eng` fenced code blocks from README and the
supported docs roots, checks current syntax snippets, and verifies snippets
marked `eng error` fail with compiler diagnostics. Design-only or future
fragments must be marked explicitly as `eng partial`, `eng future`, or
`eng unchecked`.

`artifacts-check` validates the schema files in `docs/schemas` and compares the
official CSV/plot and simple-system artifacts against
`tests/golden/artifacts`. It verifies stable format headers, version numbers,
release-critical counts, runtime table/statistics/integration values, policy
execution results, CSV canonical-unit conversion metadata, PlotSpec points,
system IR dependencies, solver-boundary status, Args resolved values, and
standalone `.engpkg` metadata.

Expected release files:

```text
dist\englang-preview-v<version>-windows-x64.zip
dist\englang-preview-v<version>-windows-x64.zip.sha256
dist\englang-user-test-guide-v<version>.pdf
dist\release-manifest.txt
```

The portable zip contains:

```text
eng.exe
eng-ide.exe
examples\
stdlib\
docs\
docs\EngLang_User_Test_Guide.pdf
tools\englang-vscode-preview-<version>.vsix
```

The `docs\` folder in the portable zip is curated release documentation. It
must not be a full copy of the repository's developer markdown tree.

`package-smoke` also verifies that the portable package can run
`eng-ide.exe --smoke` and build/run a standalone packaged runner without
requiring Rust or Python on the target side.

## Tag Release

After `release-check` passes and the worktree is clean:

```bat
git tag v<version-or-milestone>
git push origin v<version-or-milestone>
```

Examples:

```text
v1.0.0
v1.0.1
v1.1-alpha
```

Milestone tags such as `v1.0-stable` can remain as development markers. Public
GitHub Release tags should prefer SemVer tags such as `v1.0.0` or `v1.0.1`.

## GitHub Actions Release

`.github/workflows/release.yml` runs on:

```text
- tag push matching v*
- manual workflow_dispatch with tag_name
```

The workflow:

```text
1. checks out the tag
2. runs dev.bat setup
3. runs dev.bat release-check
4. uploads zip/checksum/manifest as workflow artifacts
5. publishes or updates the GitHub Release for the tag
```

For an existing tag, run the workflow manually with:

```text
tag_name = v1.0-stable
```

Use manual dispatch when a tag already existed before the release workflow was
added.

For a SemVer stable tag such as `v1.0.0`, the workflow first looks for
`docs\release\v1.0.0.md`. If it does not exist, it falls back to
`docs\release\v1.0-stable.md`.

## Release Notes

Release notes live in:

```text
docs\release\<tag>.md
```

Examples:

```text
docs\release\v1.0-stable.md
docs\release\v1.1-alpha.md
```

If no matching file exists, the workflow publishes a minimal fallback note. A
real release should add the matching release note before tag push.

## Post-Release Checks

After the workflow completes:

```text
[ ] GitHub Release exists for the tag
[ ] zip asset is attached
[ ] .sha256 asset is attached
[ ] release-manifest.txt is attached
[ ] checksum matches the zip
[ ] release notes render correctly
[ ] downloaded zip runs eng-ide.exe --smoke on a clean Windows folder
[ ] downloaded zip runs eng.exe doctor on a clean Windows folder
```
