# Usability Improvement Backlog

- Batch 873: Split base VS Code EngLang theme colors for functions, methods, and properties so generic calls and member fields remain visually distinct before domain-specific role colors apply.
- Batch 872: Removed the remaining raw internal metadata wording from the native IDE how-to and guarded against that phrase returning in user-facing IDE docs.
- Batch 871: Reworded LSP/VS Code read text/json/toml completion details from raw read wording to direct read wording, regenerated editor metadata, and added guards against the older completion details returning.
- Batch 870: Reworded the native IDE highlight detail expander from raw JSON wording to Advanced highlight data, renamed the shared expander class/helper accordingly, and added IDE guards against the older raw label returning.
- Batch 869: Reworded the portable user guide away from raw artifact-file wording toward explicit JSON artifact files, with a user-docs guard against the older phrase returning.
- Batch 868: Reworded workflow 01 API docs and printed output away from generic payload wording toward native HTTP response and API JSON contract language, with docs guards against the older phrases returning.
- Batch 867: Archived current backlog batches 841-856 into the historical log so active API/IDE/workflow cleanup docs stay focused on the latest linter, highlighting, and native IDE usability work.
- Batch 866: Added native IDE Copy at cursor for Problems plus source-line diagnostic copy text so linter reports can be captured from the caret without reconstructing context manually.
- Batch 865: Reworded VS Code README copy/inspect guidance from implementation-detail copy wording to copy-ready diagnostic/highlight details and advanced highlight data.
- Batch 864: Added role-aware highlighting aliases to VS Code highlight inspector payloads and reworded remaining highlight warnings/status summaries away from semantic-highlighting implementation terms.
- Batch 863: Added native IDE status-label mappings for behavior graph not-connected solver states so advanced solver panels avoid leaking raw `*_not_integrated` artifact keys.
- Batch 862: Reworded VS Code Tooling Status and current docs away from implementation-detail highlighting internals toward checked-code role-aware color wording, with a package guard against the older phrasing returning.
- Batch 861: Moved the native IDE Highlight tab into the primary review flow immediately after Review, with side-tab order guards and user-doc wording so color/range debugging is easier to find.
- Batch 860: Color-coded native IDE Highlight panel chips for token categories, details, selectors, and coverage domains so the inspector mirrors the editor's role colors instead of showing mostly generic chips.
- Batch 859: Reworded VS Code and native IDE user-facing highlighting settings/docs from implementation-detail phrasing to checked-code and role-aware wording, with guards against the older wording returning.
- Batch 858: Split VS Code EngLang theme colors across unit, quantity, TimeSeries, workflow, validation, report, side-effect, external, solver, model, DB, and cache role families so role-aware highlighting is more colorful without collapsing each family into one color.
- Batch 857: Reworded VS Code highlighting docs away from TextMate/semantic implementation phrasing toward first-pass syntax colors and role-aware colors, with a guard against the older wording returning.

This file is the short current backlog for API clarity, native workflow usability,
editor/linter behavior, and documentation cleanup. The historical batch log was
archived to [usability_improvement_backlog_history.md](../archive/usability_improvement_backlog_history.md).

## Current Priorities

1. Keep workflow 01/02/03 native-only: no Python, `.py`, or `run command` path may re-enter those examples.
2. Replace remaining seed-only implementation paths with executable compiler/runtime behavior where the public docs imply support.
3. Improve VS Code and native IDE authoring quality: consistent TextMate first-paint highlighting, checked-code role-aware colors, precise diagnostics, hover, completion, and quick fixes.
4. Reduce API wording ambiguity in public examples, generated metadata, diagnostics, and command names.
5. Keep current docs compact and task-oriented; move implementation history and long-form plans to `docs/archive` or `docs/internal`.

## Open Candidates

- Cache replay and invalidation: network offline-response cache materialization/replay and `eng cache invalidate` manifest-path deletion are implemented with hash/path safety checks; broader process/model replay and cross-artifact invalidation design remain open.
- Live network execution: live HTTP(S) GET/download and POST/PUT/PATCH string request bodies are implemented with timeout, retry, body limits, SHA-256 verification, body hashes, cache replay, and redacted Secret query/header records; broader live secret injection and auth schemes still need a public contract.
- Model training surface: native `train regression <table>` feeds model-card, metrics, and prediction-table paths; broader algorithm coverage and clearer multi-model naming remain open.
- Case orchestration: current materialize cases, apply ... over cases, and
  collect results paths materialize CaseTable/CaseOutput/CaseResultCollection
  records; the remaining work is a general run-case scheduler/resume/cache/failure
  policy.
- DB query support: typed SQLite table readback is implemented; arbitrary query APIs, parameter binding, and query transaction policy remain open.
- Native IDE usability: keep improving token insight, source-range actions, and inspector flows for repeated debugging tasks.
- VS Code linter/highlighting: continue expanding checked-code role-aware color coverage as more source spans become first-class metadata.

## Documentation Policy

- Public behavior changes update user docs, reference docs, examples, and release notes when applicable.
- Runtime artifact changes update the artifact reference and schemas when their shape changes.
- Historical implementation logs belong in `docs/archive`; internal unstable design work belongs in `docs/internal` or focused `docs/current/*_plan.md` files.
- `docs/README.md` remains the navigation entry point; avoid adding parallel indexes unless they serve a specific reader path.
