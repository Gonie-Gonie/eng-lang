# Usability Improvement Backlog

- Batch 844: Added `EngLang: Refresh Problems` as a VS Code command and `.eng` editor context-menu action so users can rerun the active-file linter without knowing that `Check Current File` refreshes Problems.
- Batch 843: Added a VS Code EngLang Problems status bar item that shows the active `.eng` file's diagnostics mode and current Problems counts, with a click-through to Tooling Status and package/doc coverage.
- Batch 842: Archived current backlog batches 791-820 into the historical log, keeping the active API/IDE/workflow cleanup backlog focused on recent linter, highlighting, workflow, and docs work.
- Batch 841: Reworded user-facing IDE/VS Code docs from executable/LSP and semantic-highlighting internals toward check/run tool, live editor tool, role-aware highlighting, and editor metadata wording.
- Batch 840: Reworded public user diagnostics and standalone bundle docs away from internal snapshot phrasing, using live editor analysis and source file copy wording instead.
- Batch 839: Added `.eng` editor context menu entries for the Problems-at-cursor and Highlight-token-at-cursor inspectors, with package contract coverage so linter/highlight inspection is discoverable outside the Command Palette.
- Batch 838: Added the Problems cursor inspector to the top-level `EngLang: Show Tooling Status` command summary so the status JSON exposes the linter inspection path alongside highlight inspectors.
- Batch 837: Added `EngLang: Inspect Problem at Cursor` so VS Code users can inspect the diagnostic source, code, severity, exact range, nearest same-line Problems, and copy-ready payload for the caret position.
- Batch 836: Added current-file VS Code Problems probing to `EngLang: Show Tooling Status`, including diagnostic counts, source/severity counts, and range precision summaries so linter range behavior is inspectable from one status view.
- Batch 835: Aligned VS Code saved-file Problems ranges with the LSP option-value mapper so retry, timeout, process, sampling, cache, model, simulation, and solver option diagnostics underline the offending value instead of a broad fallback token.
- Batch 834: Made native IDE Problems row clicks select the exact diagnostic character range, falling back to column-aware line selection when range metadata is unavailable.
- Batch 833: Added precise native IDE Problems ranges from the shared LSP diagnostic range mapper, including range display, column-aware row jumps, filtering, and copy payloads.
- Batch 832: Added current-file highlight token and overlap probing to VS Code Tooling Status so the first diagnostic status view surfaces color-range conflicts without opening the raw highlight inspector first.
- Batch 831: Added native IDE caret-line overlap status so the cursor summary and caret highlight table flag overlapping highlight ranges and jump directly to the Highlight panel.
- Batch 830: Added VS Code highlight overlap status fields and status wording so file and caret inspectors call out overlapping ranges without requiring users to read the raw overlap arrays first.
- Batch 829: Strengthened LSP syntax-catalog highlight coverage so every generated keyword and constant label must be surfaced as a keyword semantic token, and made workflow builtins fall back to keyword coloring when they are editor keyword-group words rather than call-style helpers.
- Batch 828: Documented VS Code highlight overlap rows in user-facing README wording and added a packaging gate so the inspector behavior stays discoverable.
- Batch 827: Exposed VS Code highlight range overlap counts and copy-ready overlap rows in the file and caret inspectors, matching the native IDE conflict visibility for confusing semantic-token colors.
- Batch 826: Added a native IDE Highlight-panel overlap summary so confusing semantic-token range conflicts are visible with source actions, selectors, and filter chips instead of only being implicit in rendered colors.
- Batch 825: Expanded VS Code saved-file diagnostic range parity for more live-LSP mapped warnings and errors, including sum calls, schema assignment markers, file mutation verbs, invalid URL literals, response field aliases, invalid log levels, and backtick payload fallback.
- Batch 824: Made VS Code local replacement quick fixes prefer the diagnostic range before falling back to the first same-line token, avoiding wrong edits when a line contains repeated `:=`, `==`, or migration tokens.
- Batch 823: Added VS Code saved-file diagnostic token-range fallbacks for syntax migration and network alias diagnostics so CLI Problems underline the same `:=`, `==`, `struct Args`, `script`, and `fixture` tokens as live LSP diagnostics.
- Batch 822: Added native IDE Highlight table row actions that open the matching inspector panels for each semantic token, extending the token-to-context routing beyond caret and hover summaries.
- Batch 821: Added VS Code highlight inspector panel hints to semantic-token rows and cursor copy-ready payloads so users can connect confusing colors to the matching Schema, Time, Workflow, Network, Case, Model, Modules, Units, Review, or Variables context.

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

- Cache replay and invalidation: network offline-response cache materialization/replay and `eng cache invalidate` manifest-path deletion are implemented with hash/path safety checks; broader process/model replay and cross-artifact invalidation design remain open.
- Live network execution: live HTTP(S) GET/download and POST/PUT/PATCH string request bodies are implemented with timeout, retry, body limits, SHA-256 verification, body hashes, cache replay, and redacted Secret query/header records; broader live secret injection and auth schemes still need a public contract.
- Model training surface: native `train regression <table>` feeds model-card, metrics, and prediction-table paths; broader algorithm coverage and clearer multi-model naming remain open.
- Case orchestration: current materialize cases, apply ... over cases, and
  collect results paths materialize CaseTable/CaseOutput/CaseResultCollection
  records; the remaining work is a general run-case scheduler/resume/cache/failure
  policy.
- DB query support: typed SQLite table readback is implemented; arbitrary query APIs, parameter binding, and query transaction policy remain open.
- Native IDE usability: keep improving token insight, source-range actions, and inspector flows for repeated debugging tasks.
- VS Code linter/highlighting: continue expanding compiler-backed semantic token coverage as more source spans become first-class metadata.

## Documentation Policy

- Public behavior changes update user docs, reference docs, examples, and release notes when applicable.
- Runtime artifact changes update the artifact reference and schemas when their shape changes.
- Historical implementation logs belong in `docs/archive`; internal unstable design work belongs in `docs/internal` or focused `docs/current/*_plan.md` files.
- `docs/README.md` remains the navigation entry point; avoid adding parallel indexes unless they serve a specific reader path.
