# Release Workflow

This document defines the repeatable EngLang preview release process.

## Version Policy

Current public release line:

```text
v0.6-preview
```

Cargo uses the SemVer-compatible workspace package version:

```text
0.6.0-preview
```

Release assets use the public label:

```text
dist\englang-preview-v0.6-preview-windows-x64.zip
dist\englang-preview-v0.6-preview-windows-x64.zip.sha256
dist\englang-user-test-guide-v0.6-preview.pdf
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

The package smoke verifies that the portable package can run without Rust or
Python installed on the target side.

## Package Contents

The unpacked portable folder contains:

```text
eng.exe
eng-ide.exe
eng-lsp.exe
examples/
stdlib/
docs/EngLang_User_Test_Guide.pdf
docs/EngLang_Language_Grammar_Guide.pdf
tools/vscode-englang/
tools/englang-vscode-preview-0.6.0-preview.vsix
README.txt
```

The package `docs\` folder is curated release documentation. It must not bundle
the full developer markdown tree.

## Tagging

After `release-check` passes and the worktree is clean:

```bat
git tag v0.6-preview
git push origin v0.6-preview
```

Do not reuse old high-numbered release names for the current public line. If a
preview needs a fix, create the next public preview label such as
`v0.7-preview` or a clearly scoped patch label only after updating
`docs/current/version_plan.md`.

## GitHub Release

Use `docs\release\v0.6-preview.md` as the public release note.

Attach:

```text
dist\englang-preview-v0.6-preview-windows-x64.zip
dist\englang-preview-v0.6-preview-windows-x64.zip.sha256
dist\englang-user-test-guide-v0.6-preview.pdf
dist\release-manifest.txt
```

## Manual Verification

Before publishing:

```text
[ ] release-check passed locally
[ ] worktree is clean
[ ] release note says this is preview software
[ ] portable package smoke passed in a clean folder
[ ] eng.exe doctor passes from the extracted package
[ ] eng-ide.exe --smoke passes from the extracted package
[ ] eng-lsp.exe --smoke passes from the extracted package
[ ] official CSV, simple system, and integrated HVAC examples run
[ ] package docs folder contains the curated PDF only
[ ] release assets match v0.6-preview public labels
```
