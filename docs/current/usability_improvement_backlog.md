# Usability Improvement Backlog

- Batch 829: Strengthened LSP syntax-catalog highlight coverage so every generated keyword and constant label must be surfaced as a keyword semantic token, and made workflow builtins fall back to keyword coloring when they are editor keyword-group words rather than call-style helpers.
- Batch 828: Documented VS Code highlight overlap rows in user-facing README wording and added a packaging gate so the inspector behavior stays discoverable.
- Batch 827: Exposed VS Code highlight range overlap counts and copy-ready overlap rows in the file and caret inspectors, matching the native IDE conflict visibility for confusing semantic-token colors.
- Batch 826: Added a native IDE Highlight-panel overlap summary so confusing semantic-token range conflicts are visible with source actions, selectors, and filter chips instead of only being implicit in rendered colors.
- Batch 825: Expanded VS Code saved-file diagnostic range parity for more live-LSP mapped warnings and errors, including sum calls, schema assignment markers, file mutation verbs, invalid URL literals, response field aliases, invalid log levels, and backtick payload fallback.
- Batch 824: Made VS Code local replacement quick fixes prefer the diagnostic range before falling back to the first same-line token, avoiding wrong edits when a line contains repeated `:=`, `==`, or migration tokens.
- Batch 823: Added VS Code saved-file diagnostic token-range fallbacks for syntax migration and network alias diagnostics so CLI Problems underline the same `:=`, `==`, `struct Args`, `script`, and `fixture` tokens as live LSP diagnostics.
- Batch 822: Added native IDE Highlight table row actions that open the matching inspector panels for each semantic token, extending the token-to-context routing beyond caret and hover summaries.
- Batch 821: Added VS Code highlight inspector panel hints to semantic-token rows and cursor copy-ready payloads so users can connect confusing colors to the matching Schema, Time, Workflow, Network, Case, Model, Modules, Units, Review, or Variables context.
- Batch 820: Routed native IDE Highlight/Caret action buttons for workflow-step, cache, case, and module/namespace semantic tokens to the matching Workflow, Network, Case, and Modules inspector panels.
- Batch 819: Fixed model target semantic-token placement so `train_test_split` source operands stay variables while named `target=` values are model properties, clearing conflicting token types across all 126 example sources.
- Batch 818: Added LSP semantic-token guards for workflow 01/02/03 and the previous overlap fixture so editor highlighting cannot emit conflicting token types for the same source range.
- Batch 817: Replaced the component residual graph Jacobian status placeholder with sparsity_metadata and cleaned internal component/domain docs so current structured artifact fields are not described as placeholders or seed-only paths.
- Batch 816: Archived current backlog batches 746-790 into the historical log, keeping active API/IDE/workflow cleanup docs focused on the latest implementation and open candidates.
- Batch 815: Promoted observed semantic-token fallback coverage into VS Code packaging, so every selector emitted by real examples and grammar fixtures must have at least one fallback scope before a local VSIX is built.
- Batch 814: Added explicit user-facing hover labels for compiler-owned coverage-result and generic table public-member fields in both VS Code and the native IDE, with dev gates preventing those new hover kinds from falling back to raw payload ids.
- Batch 813: Strengthened CLI native workflow smoke so workflow 01/02/03 must parse both static and executed run graphs and reject process, run-command, Python, notebook, or legacy helper markers in node and edge metadata, in addition to zero-process process_results.json.
- Batch 812: Added compiler-owned coverage result and generic table public-member editor catalogs, wired them through VS Code/native IDE completion and TextMate grammar fallback, and locked actual workflow examples such as `coverage.actual_count`, `weather.rows`, `db.tables_written`, and `sensor.rows` into grammar smoke.
- Batch 811: Strengthened workflow 03 smoke coverage from token presence to structured JSON assertions for propagated sensor uncertainty calculations, explicit p95 metadata-only status, report computed statistic/integration values, generated output validation, and PlotSpec confidence-band point arrays.
- Batch 810: Updated the workflow 01 native cache smoke contract to accept first-run `miss_materialized` cache records alongside cache hits and offline-response availability, restoring `dev.bat workflows-test` for the current native materialization behavior.
- Batch 809: Cleared cached VS Code review/highlight fallback state as soon as an EngLang buffer changes, preventing hover, completion, risk markers, or semantic symbol decorations from reusing an older snapshot after edits.
- Batch 808: Added VS Code highlighting pipeline and fallback scope-map coverage to `EngLang: Show Tooling Status` and the highlight-token inspector, so users can see TextMate first paint, compiler-backed semantic token routing, and missing selector coverage from one status view.
- Batch 807: Cleared the remaining fixture-level mixed-type semantic token overlaps by treating dotted calls as methods, avoiding lexical type overlays on `as` schema operands, skipping variable overlays for string write literals, preserving nested member field metadata without repainting nested receivers, and ignoring unit symbols in derived-column source fields.
- Batch 806: Suppressed generic variable and review-risk overlays on structural declarations such as schema columns, system parameters, and component declarations, added VS Code fallback scopes and theme colors for parameter modifiers surfaced by fixture coverage, and reduced mixed-type semantic token overlaps from 27 to 9 while keeping keyword/function conflicts at zero.
- Batch 805: Removed variable overlays from `export summary to csv`'s `summary` keyword and `plot A and B`'s `and` connector, clearing the remaining fixture-level keyword/variable conflicts.
- Batch 804: Stopped bare `model` value operands in `evaluate(model)`, `model_card(model)`, and `predict model using ...` from receiving lexical keyword overlays while preserving catalog keyword coverage.
- Batch 803: Reclassified `write text/json/standard_text` format selectors as side-effect keywords instead of helper functions, removing the last fixture-level function/keyword overlay.
- Batch 802: Stopped keyword overlays on call-style helper functions such as `integrate(...)`, `mlp(...)`, and `plot histogram(...)` while preserving command-style keyword coloring.
- Batch 801: Removed duplicate `function.defaultLibrary` semantic tokens from command-style verbs, keeping call-style helpers as functions and command phrases as keywords.
- Batch 800: Anchored `check coverage` command semantic keywords to the command phrase so a same-line `coverage = ...` binding stays colored as a variable.
- Batch 799: Stopped LSP keyword overlays on declaration, assignment, and named-argument labels such as `input:`, `model =`, `mode =`, and `test=...` while leaving comparison constants highlighted.
- Batch 798: Treated dotted `args.*` paths as parameter/property semantic tokens and stopped keyword overlays on their segments, aligning LSP coloring with VS Code TextMate first paint.
- Batch 797: Aligned LSP semantic colors for `print`, `log`, and log-level literals with the VS Code TextMate side-effect scopes instead of repainting them as report tokens.
- Batch 796: Colored `distribution(kind=normal|uniform)` literals as `uncertain` semantic tokens while preserving `sample uniform` as a workflow-step keyword.
- Batch 795: Gave the `empty` workflow status literal the same `workflowStep` semantic modifier as the other generated status literals, so VS Code TextMate first paint and LSP semantic coloring do not disagree.
- Batch 794: Reworded LSP hover Markdown kind/status lines to user-facing labels while keeping raw `kind` and `status` JSON fields stable for editor clients.
- Batch 793: Reworded hover status display in VS Code and Native IDE from raw ids such as `domain_compatible` into user-facing labels while keeping snapshot payloads unchanged.
- Batch 792: Aligned Native IDE hover titles and Highlight-panel Hover rows with VS Code by showing user-facing role labels instead of raw hover kind ids.
- Batch 791: Reworded VS Code hover kind display from raw payload ids such as `model_field` and `db_connection_field` into user-facing role labels while keeping payload matching unchanged.

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
