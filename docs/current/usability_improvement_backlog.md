# Usability Improvement Backlog

This file is the short current backlog for API clarity, native workflow usability,
editor/linter behavior, and documentation cleanup. The historical batch log was
archived to [usability_improvement_backlog_history.md](../archive/usability_improvement_backlog_history.md).

## Current Priorities

1. Keep workflow 01/02/03 native-only: no Python, `.py`, or `run command` path may re-enter those examples.
2. Replace remaining seed-only implementation paths with executable compiler/runtime behavior where the public docs imply support.
3. Improve VS Code and native IDE authoring quality: consistent TextMate first-paint highlighting, compiler-backed semantic tokens, precise diagnostics, hover, completion, and quick fixes.
4. Reduce API wording ambiguity in public examples, generated metadata, diagnostics, and command names.
5. Keep current docs compact and task-oriented; move implementation history and long-form plans to `docs/archive` or `docs/internal`.

## Open Candidates

- Cache replay and invalidation: network offline-response cache materialization/replay is implemented with hash checks; broader process/model replay and explicit invalidation still need an artifact-safety design.
- Live network execution: live HTTP(S) GET/download is implemented with timeout, retry, body limit, SHA-256 verification, and cache replay; request body/auth policy still needs a broader public contract.
- Model training surface: native `train regression <table>` feeds model-card, metrics, and prediction-table paths; broader algorithm coverage and clearer multi-model naming remain open.
- Case orchestration: current case manifests are materialized through workflow records; a native `apply/run cases` surface still needs scheduler, resume, cache, and failure policy.
- DB query support: typed SQLite table readback is implemented; arbitrary query APIs, parameter binding, and query transaction policy remain open.
- Native IDE usability: keep improving token insight, source-range actions, and inspector flows for repeated debugging tasks.
- VS Code linter/highlighting: continue expanding compiler-backed semantic token coverage as more source spans become first-class metadata.

## Recent Completed Batches

- Batch 529: Added optional VS Code EngLang Dark/Light themes with explicit role-aware semantic colors.
- Batch 530: Reworded VS Code quick-fix docs away from skeleton language and added a wording guard.
- Batch 531: Reworded current workflow track scope away from skeleton language and added it to docs wording guards.
- Batch 532: Added portable package and VSIX smoke coverage for VS Code grammar, snippets, generated editor metadata, and EngLang color themes.
- Batch 533: Clarified Native IDE Problems and Terminal placeholder/help wording for supported commands and one-line EngLang statements.
- Batch 534: Added `completion_items` editor metadata as the preferred completion catalog API while retaining `completion_seed` as a compatibility alias.
- Batch 535: Added version-aware LSP document cache and versioned publishDiagnostics coverage for persistent editor sessions.
- Batch 536: Renamed VS Code extension completion provider wiring from completionSeed to completionItems while keeping generated `completion_seed` as a metadata-only legacy alias.
- Batch 537: Added explicit VS Code EngLang Dark/Light base semantic token colors so semantic highlighting does not fall back to host theme defaults for core token types.
- Batch 538: Archived compact summaries for batches 475-528 from the current usability backlog so the active file stays task-oriented.
- Batch 539: Guarded VS Code live stdin requests against stale document versions and fixed each request to the source text captured at launch.
- Batch 540: Added stale-document guards to VS Code completion, hover, symbols, definition, folding, semantic-token, formatting, and code-action providers before using live or cached editor results.

## Documentation Policy

- Public behavior changes update user docs, reference docs, examples, and release notes when applicable.
- Runtime artifact changes update the artifact reference and schemas when their shape changes.
- Historical implementation logs belong in `docs/archive`; internal unstable design work belongs in `docs/internal` or focused `docs/current/*_plan.md` files.
- `docs/README.md` remains the navigation entry point; avoid adding parallel indexes unless they serve a specific reader path.
