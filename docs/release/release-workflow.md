# Release Workflow

This document defines the repeatable EngLang release process.

## Version Policy

Current public release line:

```text
v1.0.0
```

Cargo uses the SemVer-compatible workspace package version:

```text
1.0.0
```

Release assets use the public label:

```text
dist\englang-v1.0.0-windows-x64.zip
dist\englang-v1.0.0-windows-x64.zip.sha256
dist\englang-user-guide-v1.0.0.pdf
dist\release-manifest.txt
```

Preview versions still use preview asset names such as
`englang-preview-v0.9-preview-windows-x64.zip`.

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
tools/englang-vscode-1.0.0.vsix
README.txt
PACKAGE_ASSETS.txt
```

The package `docs\` folder is curated release documentation. It must not bundle
the full developer markdown tree.

## Tagging

After `release-check` passes and the worktree is clean:

```bat
git tag v1.0.0
git push origin v1.0.0
```

Do not reuse old high-numbered release names for the current public line. If a
stable release needs a fix, create a patch label such as `v1.0.1` only after
updating `docs/current/version_plan.md`.

## GitHub Release

Use `docs\release\v1.0.0.md` as the public release note.

Attach:

```text
dist\englang-v1.0.0-windows-x64.zip
dist\englang-v1.0.0-windows-x64.zip.sha256
dist\englang-user-guide-v1.0.0.pdf
dist\release-manifest.txt
```

## Manual Verification

Before publishing:

```text
[ ] release-check passed locally
[ ] worktree is clean
[ ] release note separates Stable, Supported, Internal, and Planned scope
[ ] release note states exact stable scope and limitations
[ ] portable package smoke passed in a clean folder
[ ] eng.exe doctor passes from the extracted package
[ ] eng-ide.exe --smoke passes from the extracted package
[ ] eng-lsp.exe --smoke passes from the extracted package
[ ] official CSV, simple system, and integrated HVAC examples run
[ ] package docs folder contains curated PDFs only
[ ] PACKAGE_ASSETS.txt documents official, compatibility, internal, and diagnostic examples
[ ] release assets match v1.0.0 public labels
```
