# Release Workflow

This document defines the repeatable EngLang release process.

For the current observed publication state and historical tag cleanup, see
[release-state.md](release-state.md).

## Publication Terms

Use these terms precisely:

```text
Release-ready: release-check passed and assets can be produced locally.
Tagged:        a git tag exists locally and on origin.
Published:     a GitHub Release page exists for the exact tag and the release
               assets are attached or intentionally documented as omitted.
```

Do not call a tag-only state a published release.

## Version Policy

Current public release line:

```text
v0.1.0
```

Cargo uses the SemVer-compatible workspace package version:

```text
0.1.0
```

Release assets use the public label:

```text
dist\englang-v0.1.0-windows-x64.zip
dist\englang-v0.1.0-windows-x64.zip.sha256
dist\englang-user-guide-v0.1.0.pdf
dist\release-manifest.txt
```

## Local Gate

Run:

```bat
.\dev.bat release-check
```

`release-check` performs:

```text
1. dev.bat ci
2. docs-check
3. IDE extension check
4. artifacts-check
5. package
6. package-smoke in a clean folder with spaces and Korean characters
7. checksum verification
8. release-manifest.txt generation
```

The package smoke verifies that the portable package can run without Rust,
Python, Node, or Visual Studio Build Tools on the target side.

## Package Contents

The unpacked portable folder contains:

```text
eng.exe
eng-ide.exe
eng-lsp.exe
WebView2Loader.dll
examples/
stdlib/
docs/EngLang_User_Guide.pdf
docs/EngLang_Language_Grammar_Guide.pdf
tools/vscode-englang/
tools/englang-vscode-0.1.0.vsix
README.txt
PACKAGE_ASSETS.txt
```

The package `docs\` folder is curated release documentation. It must not bundle
the full developer markdown tree.

## Tagging

After `release-check` passes and the worktree is clean:

```bat
git tag v0.1.0
git push origin v0.1.0
```

Do not reuse old preview, alpha, or high-numbered readiness tags for the current
public line.

## GitHub Release

Use `docs\release\v0.1.0.md` as the public release note.

Attach:

```text
dist\englang-v0.1.0-windows-x64.zip
dist\englang-v0.1.0-windows-x64.zip.sha256
dist\englang-user-guide-v0.1.0.pdf
dist\release-manifest.txt
```

After publication, update [release-state.md](release-state.md) with the exact
tag, GitHub Release URL, prerelease/stable flag, published timestamp, and any
asset omissions. If only a tag was pushed, record it as tagged but not
published.

## Manual Verification

Before publishing:

```text
[ ] release-check passed locally
[ ] worktree is clean
[ ] release note separates Stable, Supported, Internal, and Planned scope
[ ] release note states exact public scope and solver limitations
[ ] portable package smoke passed in a clean folder
[ ] eng.exe doctor passes from the extracted package
[ ] eng-ide.exe --smoke passes from the extracted package
[ ] eng-lsp.exe --smoke passes from the extracted package
[ ] official CSV, command where/with, and test/assert/golden examples run
[ ] package docs folder contains curated PDFs only
[ ] PACKAGE_ASSETS.txt documents official, compatibility, internal, and diagnostic examples
[ ] release assets match v0.1.0 public labels
```
