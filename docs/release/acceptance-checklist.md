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

Before publishing `v0.3-preview`:

```text
[x] workspace package version is updated to 0.3.0-preview
[x] public package label is v0.3-preview
[x] current status, maturity matrix, tracks, roadmap, README, and release workflow agree on the v0.3 scope
[x] path helpers are implemented for file/dir/join/parent/stem/extension
[x] exists is type-checked as Bool and resolved relative to the source file
[x] review.json records environment_dependencies
[x] result.engres records provenance.environment_dependencies
[x] report_spec.json records provenance.environment_dependencies
[x] artifact schemas include environment dependency fields
[x] official/10_path_policy mini example is present
[x] docs explain path helper scope and broader side-effect deferrals
[x] docs-check passes
[x] artifacts-check passes
[x] ide-check passes
[x] lsp-check passes
[x] package-smoke passes under a path with spaces and Korean characters
[x] release-check passes
```

Before publishing `v0.4-preview`:

```text
[x] workspace package version is updated to 0.4.0-preview
[x] public package label is v0.4-preview
[x] current status, maturity matrix, tracks, roadmap, README, and release workflow agree on the v0.4 scope
[x] read text/json/toml expressions are type-checked as String
[x] read text/json/toml resolve source-relative typed path values at runtime
[x] review.json records read-only I/O environment_dependencies
[x] result.engres records read-only I/O provenance.environment_dependencies
[x] report_spec.json records read-only I/O provenance.environment_dependencies
[x] artifact schemas include nullable source_hash fields
[x] official/11_read_only_io mini example is present
[x] docs explain raw UTF-8 read scope and structured JSON/TOML deferral
[x] docs-check passes
[x] artifacts-check passes
[x] ide-check passes
[x] lsp-check passes
[x] package-smoke passes under a path with spaces and Korean characters
[x] release-check passes
```

Before publishing `v0.5-preview`:

```text
[x] workspace package version is updated to 0.5.0-preview
[x] public package label is v0.5-preview
[x] current status, maturity matrix, tracks, roadmap, README, and release workflow agree on the v0.5 scope
[x] write text/json top-level statements are parsed and type-checked
[x] write text/json outputs are constrained under build/result
[x] review.json records writes metadata
[x] export summary to csv uses idempotent overwrite hardening
[x] changed write/export contents require with { overwrite = true }
[x] output_manifest.json records generated file kinds, paths, and hashes
[x] output manifest schema is documented
[x] official/12_write_output_manifest mini example is present
[x] docs explain write/export/output manifest scope and copy/move/delete deferral
[x] docs-check passes
[x] artifacts-check passes
[x] ide-check passes
[x] lsp-check passes
[x] package-smoke passes under a path with spaces and Korean characters
[x] release-check passes
```

Before publishing `v0.6-preview`:

```text
[x] workspace package version is updated to 0.6.0-preview
[x] public package label is v0.6-preview
[x] current status, maturity matrix, tracks, roadmap, README, and release workflow agree on the v0.6 scope
[x] copy/move/delete top-level statements are parsed and checked
[x] move/delete require with { confirm = true }
[x] delete dir(...) requires with { recursive = true }
[x] copy/move/delete operations are constrained to generated-output boundaries
[x] review.json records file_operations metadata
[x] output_manifest.json records copy_file/move_file/delete_file entries
[x] official/13_file_operations mini example is present
[x] docs explain copy/move/delete scope and process/network deferral
[x] docs-check passes
[x] artifacts-check passes
[x] ide-check passes
[x] lsp-check passes
[x] package-smoke passes under a path with spaces and Korean characters
[x] release-check passes
```

Before publishing `v0.7-preview`:

```text
[x] workspace package version is updated to 0.7.0-preview
[x] public package label is v0.7-preview
[x] current status, maturity matrix, tracks, roadmap, README, and release workflow agree on the v0.7 scope
[x] `log debug/info/warn/error` statements are parsed and checked
[x] invalid or missing log levels produce E-LOG-LEVEL-001
[x] log interpolation uses the same type/unit checks as print
[x] review.json records print/log entries with level metadata
[x] eng run stdout prefixes structured log messages with [level]
[x] saved runs write build/result/run_log.json
[x] output_manifest.json records the run_log artifact
[x] native IDE artifact panel shows run-log paths/objects
[x] official/14_run_log mini example is present
[x] docs explain print vs log vs durable report/export surfaces
[x] docs-check passes
[x] artifacts-check passes
[x] ide-check passes
[x] lsp-check passes
[x] package-smoke passes under a path with spaces and Korean characters
[x] release-check passes
```

Before publishing `v0.8-preview`:

```text
[x] workspace package version is updated to 0.8.0-preview
[x] public package label is v0.8-preview
[x] current status, maturity matrix, tracks, roadmap, README, and release workflow agree on the v0.8 scope
[x] `run command` statements are parsed and checked
[x] `run command` requires a bound `ProcessResult`
[x] invalid process declarations produce E-PROCESS-* diagnostics
[x] process options support args, cwd, and allow_failure
[x] non-zero exits fail unless allow_failure is true
[x] review.json records process_runs metadata
[x] saved runs write build/result/process_results.json
[x] output_manifest.json records the process_results artifact
[x] native IDE artifact panel shows process-results paths/objects
[x] official/15_process_result mini example is present
[x] docs explain external process execution as explicit, reviewable side effect
[x] docs-check passes
[x] artifacts-check passes
[x] ide-check passes
[x] lsp-check passes
[x] package-smoke passes under a path with spaces and Korean characters
[x] release-check passes
```

Before publishing `v0.9-preview`:

```text
[x] workspace package version is updated to 0.9.0-preview
[x] public package label is v0.9-preview
[x] current status, maturity matrix, tracks, roadmap, README, and release workflow agree on the v0.9 scope
[x] `test` blocks are parsed and checked
[x] `assert` is valid only inside a test block
[x] quantity assertions reject incompatible dimensions
[x] `golden "artifact" matches file("expected")` is parsed and checked
[x] review.json records tests metadata
[x] saved runs write build/result/test_results.json
[x] output_manifest.json records the test_results artifact
[x] native IDE artifact panel shows test-results paths/objects
[x] official/16_test_assert_golden mini example is present
[x] docs explain test/assert/golden as local workflow verification
[x] docs-check passes
[x] artifacts-check passes
[x] ide-check passes
[x] lsp-check passes
[x] package-smoke passes under a path with spaces and Korean characters
[x] release-check passes
```

Before publishing `v1.0.0`:

```text
[x] workspace package version is updated to 1.0.0
[x] public package label is v1.0.0
[x] stable core scope is documented
[x] breaking-change policy is documented
[x] current status, maturity matrix, tracks, roadmap, README, and release workflow agree on the v1.0.0 scope
[x] preview/experimental tracks remain explicitly outside the stable contract
[x] official/17_measured_vs_simulated integrated workflow is present
[x] RMSE metrics, validations, time alignments, and multi-series PlotSpec are schema/documented
[x] `eng run --profile safe|normal|repro` basics are implemented and documented
[x] package assets use stable names without preview labels
[x] official examples and compatibility fixtures pass
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
