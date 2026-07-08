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

- Batch 470: Added VS Code local and eng-lsp unresolved interpolation quick fixes.
- Batch 471: Added LSP interpolation display-unit removal quick-fix parity.
- Batch 472: Added LSP unterminated interpolation close-brace quick-fix parity.
- Batch 473: Reworded workflow 03 docs from fixture-first CSV wording to checked CSV input wording.
- Batch 474: Made TimeSeries helper semantic tokens consistent and added VS Code `keyword.timeseries` fallback scope mapping.
- Batch 475: Broadened VS Code semantic fallback scopes for keyword workflow/model/timeseries clauses and added contract guards.
- Batch 476: Added VS Code highlight inspection semantic selector and TextMate fallback-scope debug output.
- Batch 477: Added observed base semantic selector fallback scopes for VS Code highlighting consistency.
- Batch 478: Added VS Code cursor-specific highlight token inspection.
- Batch 479: Added lsp-check coverage for observed VS Code semantic fallback selector mappings.
- Batch 480: Added structured workflow 02 native sampler/model/prediction/DB artifact guards.

## Documentation Policy

- Public behavior changes update user docs, reference docs, examples, and release notes when applicable.
- Runtime artifact changes update the artifact reference and schemas when their shape changes.
- Historical implementation logs belong in `docs/archive`; internal unstable design work belongs in `docs/internal` or focused `docs/current/*_plan.md` files.
- `docs/README.md` remains the navigation entry point; avoid adding parallel indexes unless they serve a specific reader path.