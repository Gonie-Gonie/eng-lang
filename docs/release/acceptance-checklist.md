# Acceptance Checklist

Every development or release slice should answer these questions before commit
and release.

## General Slice Checklist

```text
[ ] Which public preview or development track does this change serve?
[ ] Does the change alter language syntax or semantics?
[ ] Does it alter artifact schemas or runtime behavior?
[ ] Does it need examples, diagnostics, IDE metadata, or docs?
[ ] Did docs/current/status.md remain accurate?
[ ] Did docs/current/feature_maturity_matrix.md remain accurate?
[ ] Did docs/current/tracks.md remain accurate?
[ ] Did user-facing docs avoid unsupported stability claims?
[ ] Did release notes change if package behavior changed?
[ ] Did ci/docs/artifact checks pass?
```

## Preview Release Gate

Before publishing `v0.1-preview`:

```text
[x] workspace package version is 0.1.0-preview
[x] public package label is v0.1-preview
[x] README shows current public line, active target, supported preview workflows, and future tracks
[x] version plan separates release versions from development tracks
[x] current status distinguishes supported preview, preview tooling, future tracks, and deferred scope
[x] feature maturity matrix does not present future tracks as public release versions
[x] release workflow uses v0.1-preview package labels
[x] release notes state that language and artifact formats are not stable
[x] package creates dist\englang-preview-v0.1-preview-windows-x64.zip
[x] package creates dist\englang-preview-v0.1-preview-windows-x64.zip.sha256
[x] package creates dist\englang-user-test-guide-v0.1-preview.pdf
[x] package includes eng.exe, eng-ide.exe, eng-lsp.exe, examples, stdlib, tools, README.txt, and curated PDF docs
[x] package docs folder does not include developer markdown files
[x] package-smoke extracts under a path with spaces and Korean characters
[x] packaged eng.exe doctor passes
[x] packaged eng-ide.exe --smoke passes
[x] packaged eng-lsp.exe --smoke passes
[x] official CSV+plot example runs and creates result/report/PlotSpec artifacts
[x] official simple system example runs and creates result/report artifacts
[x] official integrated HVAC example runs and creates policy, solver-boundary, report, and PlotSpec artifacts
[x] standalone packaged runner smoke passes
```

Before publishing `v0.2-preview`:

```text
[x] workspace package version is updated to 0.2.0-preview
[x] public package label is v0.2-preview
[x] release notes explain that language and artifact formats remain preview
[x] current status, maturity matrix, tracks, roadmap, and README agree on the v0.2 scope
[x] native IDE layout feels professional on clean desktop and smaller laptop sizes
[x] native IDE workspace explorer uses dense, scannable spacing
[x] native IDE supports opening, switching, and closing multiple files
[x] native IDE keeps result output from pushing variable/metadata side panels off screen
[x] native IDE variable panel updates after successful run
[x] top-level execution, args, const, function/file-import preview path is documented and tested
[x] imported files do not register or execute workflow roots
[x] imported module args and top-level `=` bindings are not imported
[x] unit-aware print and explicit summary CSV export have a mini official example
[x] data/table/TimeSeries expression-kernel scope is documented with supported examples and diagnostics
[x] first-class Summary object decision is explicitly recorded as deferred for v0.2 scope
[x] integrated language philosophy is documented as the active short-form policy
[x] side-effect/general programming policy is documented without claiming broad runtime support
[x] official examples 01-09 pass check/run smoke paths through release-check/artifact paths
[x] docs-check passes
[x] artifacts-check passes
[x] ide-check passes
[x] lsp-check passes
[x] package-smoke passes under a path with spaces and Korean characters
[x] release-check passes
```

## Local Verification

Run:

```bat
.\dev.bat release-check
```

`release-check` runs `ci`, `docs-check`, `artifacts-check`, IDE extension
checks, package assembly, package smoke, checksum verification, and
`dist\release-manifest.txt` generation.

Release workflow details live in [release-workflow.md](release-workflow.md).
