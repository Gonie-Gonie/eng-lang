# Usability Improvement Backlog

- Batch 860: Color-coded native IDE Highlight panel chips for token categories, details, selectors, and coverage domains so the inspector mirrors the editor's role colors instead of showing mostly generic chips.
- Batch 859: Reworded VS Code and native IDE user-facing highlighting settings/docs from compiler-backed implementation phrasing to checked-code and role-aware wording, with guards against the older wording returning.
- Batch 858: Split VS Code EngLang theme colors across unit, quantity, TimeSeries, workflow, validation, report, side-effect, external, solver, model, DB, and cache role families so role-aware highlighting is more colorful without collapsing each family into one color.
- Batch 857: Reworded VS Code highlighting docs away from TextMate/semantic implementation phrasing toward first-pass syntax colors and role-aware colors, with a guard against the older wording returning.
- Batch 856: Archived current backlog batches 821-840 into the historical log, keeping the active API/IDE/workflow cleanup backlog focused on the latest linter, highlighting, workflow, and docs work.
- Batch 855: Added VS Code highlight coverage summaries to Inspect Highlight Tokens and Tooling Status so keyword/workflow/option/unit/operator color gaps are visible from the editor JSON reports.
- Batch 854: Added a native IDE Highlight coverage summary with domain counts, unmatched source-word visibility, and copy-ready summary output so inconsistent keyword/workflow/unit coloring is easier to diagnose from the IDE.
- Batch 853: Added native IDE Workflow panel evidence for zero external processes, process-results status, run graph counts, graph hashes, and direct process-results artifact access.
- Batch 852: Added native workflow source/docs and latest zero-process artifact evidence to VS Code Tooling Status so workflow 01/02/03 native-only status is visible from the editor.
- Batch 851: Added `dev.bat workflow-native-status` so workflow 01/02/03 native-only source/docs guards and latest zero-process artifact evidence are visible without rerunning the full workflow smoke gate.
- Batch 850: Listed the VS Code Problems and Highlight copy commands in Tooling Status so the summary view exposes the full inspect/copy loop for linter and coloring reports.
- Batch 849: Added `EngLang: Copy Highlight Token at Cursor` so VS Code users can copy current or nearest same-line role-aware highlight token payloads directly from the editor context menu.
- Batch 848: Added `EngLang: Copy Problem at Cursor` so VS Code users can copy the current or nearest same-line Problems payload directly from the editor context menu.
- Batch 847: Added underlined source text and full source-line fields to VS Code Problems inspector rows and copy-ready payloads so linter reports identify the exact token/range without manual reconstruction.
- Batch 846: Made manual VS Code Problems refresh respect the selected diagnostics mode for dirty buffers, so file mode no longer switches to live-buffer diagnostics behind the user's back.
- Batch 845: Kept the VS Code EngLang Problems status bar scoped to the active editor when background `.eng` documents open, change, or save, so linter mode/counts do not jump to another file.
- Batch 844: Added `EngLang: Refresh Problems` as a VS Code command and `.eng` editor context-menu action so users can rerun the active-file linter without knowing that `Check Current File` refreshes Problems.
- Batch 843: Added a VS Code EngLang Problems status bar item that shows the active `.eng` file's diagnostics mode and current Problems counts, with a click-through to Tooling Status and package/doc coverage.
- Batch 842: Archived current backlog batches 791-820 into the historical log, keeping the active API/IDE/workflow cleanup backlog focused on recent linter, highlighting, workflow, and docs work.
- Batch 841: Reworded user-facing IDE/VS Code docs from executable/LSP and semantic-highlighting internals toward check/run tool, live editor tool, role-aware highlighting, and editor metadata wording.

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
