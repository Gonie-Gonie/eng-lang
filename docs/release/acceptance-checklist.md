# Acceptance Checklist

This checklist is for the current clean public package line. Historical preview
gates are archived under
[preview_acceptance_checklist.md](../archive/release-notes/preview_acceptance_checklist.md).

For observed publication state, see [release-state.md](release-state.md).

## General Change Gate

Before merging a development or release slice:

```text
[ ] Which public package workflow or internal track does this change serve?
[ ] Does it alter language syntax, semantics, artifact schemas, or runtime behavior?
[ ] Does it need examples, diagnostics, IDE metadata, docs, or golden artifacts?
[ ] Did README, status, maturity matrix, and tracks remain accurate?
[ ] Did user-facing docs avoid unsupported solver or stability claims?
[ ] Did release notes change if public package behavior changed?
[ ] Did docs/artifact/example checks pass in the dev environment?
```

## v0.1.0 Public Package Gate

Before publishing or republishing `v0.1.0` assets:

```text
[x] workspace package version is 0.1.0
[x] public package label is v0.1.0
[x] README, status, LLM context, release state, and release notes agree on v0.1.0
[x] public package scope is documented separately from main-internal work
[x] solver-heavy examples are not presented as first-user official workflows
[x] advanced solver and internal fixtures stay outside the portable package examples
[x] curated user guide focuses on semantic workflow, TimeSeries, reports, and IDE review
[x] package assets use v0.1.0 names without preview labels
[x] official examples and compatibility fixtures pass smoke checks
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
