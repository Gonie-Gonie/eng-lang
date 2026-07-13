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

## Recent Completed Batches

- Batch 571: Made the native IDE lexical fallback preserve compiler-owned keyword groups and reuse role colors before semantic-highlight data is available.
- Batch 572: Removed VS Code static snippets whose prefixes duplicate generated completion labels and added package guards so compiler-owned snippet insert text remains the single source for those suggestions.
- Batch 573: Made Native IDE Highlight panel category, detail, and selector chips clickable filters so users can inspect role-aware colors by token type or semantic modifier directly.
- Batch 574: Clarified current workflow docs so implemented CaseTable/CaseOutput materialization is not confused with the still-planned general run-case scheduler/resume/cache policy.
- Batch 575: Extended VS Code highlight inspection to report direct semantic selector coverage, so generic fallback coloring can be distinguished from fully mapped role colors.
- Batch 576: Documented the implemented native network request-body and secret redaction contract in public reference docs and CLI diagnostic code lists.
- Batch 577: Clarified cache invalidation wording across stdlib docs, side-effect policy, generated editor metadata, and current backlog so implemented manifest-path deletion is not described as planned.
- Batch 578: Added LSP semantic tokens for string interpolation variables, member fields, format precision, and format units so VS Code role-aware highlighting does not flatten workflow output strings.
- Batch 579: Added `row_preview` with case IDs, display values, numeric values, and units to `typed_payload.sample_tables[]` so native sampling exposes generated rows directly instead of only metadata and hashes.
- Batch 580: Exposed `samples.row_preview` as a runtime sample-table metadata binding and added matching compiler typing, LSP member completion, semantic-token coverage, and docs.
- Batch 581: Split VS Code TextMate dotted paths into path and member scopes, added first-paint coverage for `samples.row_preview`, and colored member segments in the bundled themes.
- Batch 582: Moved member-path TextMate fallback ahead of broad dotted-path regexes in interpolation, validation, function-call, and `with` expression contexts, with a grammar guard for ordering drift.
- Batch 583: Added `case_inputs.expected_count` as the preferred CaseOutput count binding, kept `planned_count` as a compatibility field, and guarded public workflow docs against stale planned-count wording.
- Batch 584: Added case/workflow status literals `planned`, `rendered`, and `collected` to the compiler-owned editor constant catalog, TextMate status-condition fallback, and grammar fixtures so native case statuses color consistently.
- Batch 585: Gave native workflow `status = planned/rendered/collected/missing` option values workflow-step semantic modifiers, with LSP snapshot coverage so semantic overlay keeps the same role-aware coloring after first paint.
- Batch 586: Aligned native case collection status literals `partial` and `empty` with the editor constant catalog, TextMate status-condition fallback, grammar fixtures, and workflow-step semantic-token coverage.
- Batch 587: Added LSP semantic-token context for `status ==`/`status !=` workflow conditions so condition keys and status literals keep workflow-step coloring after semantic overlay.
- Batch 588: Added TextMate grammar expected-token guards for `status == partial` and `status != empty` so first-paint workflow status condition coloring cannot drift from the fixture coverage.
- Batch 589: Moved workflow status-condition literal generation to LSP editor metadata via `syntax_catalog.workflow_status_literals`, removing the hardcoded TextMate status list and adding build/test guards against drift.
- Batch 590: Made the native IDE lexical fallback consume `syntax_catalog.workflow_status_literals` and color `status =`, `status ==`, and `status !=` literals as workflow-step tokens before semantic results arrive.
- Batch 591: Updated workflow 02 to expose native sampler `row_preview` bindings in console output, reports, and `sampling_summary.txt`, with runtime contract coverage so the workflow shows generated sample rows rather than only seed/count metadata.
- Batch 592: Documented `syntax_catalog.workflow_status_literals` in the editor token-scope contract and added a VS Code extension contract guard so workflow status literal coloring stays generated across TextMate and native IDE fallback paths.
- Batch 593: Removed the native IDE hardcoded workflow-status literal fallback list so `status =`, `status ==`, and `status !=` coloring uses only the LSP-generated `syntax_catalog.workflow_status_literals` catalog.
- Batch 594: Routed generated `syntax_catalog.units` labels into VS Code local quick fixes so missing-unit repairs only suggest compiler-owned units when editor metadata is available.
- Batch 595: Removed the native IDE hardcoded unit lexical fallback list so unit coloring uses generated `syntax_catalog.units` as the source of truth.
- Batch 596: Removed the native IDE hardcoded operator-word fallback list so operator-word coloring uses generated `syntax_catalog.operator_words`.
- Batch 597: Made native `collect results <CaseOutput>` require rendered CaseOutput evidence before reporting rows as collected; planned output paths now remain missing.
- Batch 598: Removed the native IDE hardcoded constant fallback list so constant coloring uses generated `syntax_catalog.constants`.
- Batch 599: Removed the native IDE hardcoded keyword fallback list so keyword coloring uses generated `syntax_catalog.keywords`, keyword groups, and workflow builtin catalogs.
- Batch 600: Archived compact summaries for batches 529-570 from the current usability backlog so the active file stays task-oriented.
- Batch 601: Promoted `unit x` and `unit y` into the generated workflow-option catalog and routed that catalog into VS Code/LSP alias quick fixes so plot option repairs cannot drift from public metadata.
- Batch 602: Routed VS Code and LSP option-value quick fixes through the generated workflow-option catalog so stale hardcoded option names are filtered before repair actions are shown.
- Batch 603: Clarified the workflow 01/02/03 native-only contract in public workflow docs, including the `workflows-test` guard against Python/notebook markers, `run command`, process-run artifacts, and process counts.

## Documentation Policy

- Public behavior changes update user docs, reference docs, examples, and release notes when applicable.
- Runtime artifact changes update the artifact reference and schemas when their shape changes.
- Historical implementation logs belong in `docs/archive`; internal unstable design work belongs in `docs/internal` or focused `docs/current/*_plan.md` files.
- `docs/README.md` remains the navigation entry point; avoid adding parallel indexes unless they serve a specific reader path.
