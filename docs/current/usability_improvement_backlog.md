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
- Batch 541: Extended native workflow gates so public workflow docs and expected summaries reject Python/notebook markers and run-command wording for workflows 01/02/03.
- Batch 542: Expanded VS Code TextMate fallback highlighting for native workflow command verbs, validation verbs, and sampling method literals outside full phrase matches.
- Batch 543: Ran VS Code extension install through an ignored temporary working directory so CLI debug logs do not dirty the checkout, and documented the reinstall behavior.
- Batch 544: Added summary-first VS Code tooling status output with clearer executable source, fallback, availability, diagnostics, and role-coloring wording.
- Batch 545: Aligned VS Code TextMate fallback colors for TimeSeries quality verbs and added partial-edit fallback coverage for `require_one`.
- Batch 546: Renamed the top-level output manifest label to generated output list across CLI, VS Code commands, and docs.
- Batch 547: Made VS Code diagnostics output show the selected diagnostics source/tool path and point editor-JSON failures to Tooling Status.
- Batch 548: Added a Tooling Status action and output-panel hint when VS Code highlight inspection cannot obtain semantic highlight data.
- Batch 549: Scoped VS Code quick-fix computation to Quick Fix requests so refactor/source-action menus stay clean.
- Batch 550: Added VS Code and native IDE fallback coloring for solver Jacobian and uncertainty policy literals.
- Batch 551: Reworded advanced solver example docs so implemented narrow solve fixtures do not read as seed-only paths.
- Batch 552: Promoted editor constants and operator words into generated LSP metadata so VS Code grammar and native IDE lexical coloring can stay aligned.
- Batch 553: Switched VS Code TextMate operator-word fallback generation to the LSP syntax catalog with compatibility aliases for legacy clause words.
- Batch 554: Added TextMate grammar build and smoke guards so generated constants and operator words cannot silently drift from LSP editor metadata.
- Batch 555: Moved VS Code TextMate keyword-group fallback lists into generated LSP editor metadata so first-paint keyword scopes share the compiler-owned catalog.
- Batch 556: Added compiler-owned block-opener keyword groups so partial `args`, `where`, `with`, and `on` edits keep block-keyword coloring before full parse context is available.
- Batch 557: Mapped semantic keyword declaration/local fallbacks to block-opener TextMate scopes so `args` and `where` retain block coloring after semantic overlay.
- Batch 558: Made VS Code last-run artifact pickers and review-panel artifact links share availability state so existing outputs sort first and missing optional artifacts are labeled clearly.
- Batch 559: Added plain status summaries to VS Code highlight inspection JSON so token absence and missing fallback-scope coverage are visible without reading raw rows first.
- Batch 560: Aligned VS Code and native IDE caret-near highlight distance checks with half-open token ranges so token-end positions no longer read as inside the previous token.
- Batch 561: Added a plain Native IDE Highlight panel status summary so stale checks, empty highlight results, and filters that hide all ranges are visible before reading token tables.
- Batch 562: Added per-token copy actions to the Native IDE Highlight table so individual highlight text and source ranges can be copied without using the full visible-highlight export.
- Batch 563: Made the bundled VS Code EngLang Dark/Light themes directly color every semantic selector contributed by the extension and guarded that coverage against drift.
- Batch 564: Made native case-template apply outputs report rendered CaseOutput status/counts after files and render manifests are actually written, replacing workflow 02's planned-count surface with rendered-count evidence.
- Batch 565: Made CaseResultCollection aggregate status report collected/partial/missing/blocked/empty so workflow 02 no longer shows collected rows as status=complete.
- Batch 566: Extended native workflow smoke gates to reject process/run-command/Python metadata in saved static and runtime run graphs, not just source text and process_results.json.
- Batch 567: Made VS Code and native IDE fallback member completions recognize function-style `apply(..., over=...)` CaseOutput bindings and property-source `collect results` bindings.
- Batch 568: Aligned VS Code fallback member completions with the native IDE for CaseResultCollection-like receivers such as `case_results.` and `case_result_collection.`.
- Batch 569: Updated VS Code and native IDE CaseOutput fallback receiver heuristics to recognize rendered/blocked wording alongside planned/input/output/manifest names.
- Batch 570: Moved shared starter snippets such as top workflow, args block, schema csv, test block, promote csv, plot line, and log info into LSP-generated completion metadata so VS Code and the native IDE use the same catalog.
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

## Documentation Policy

- Public behavior changes update user docs, reference docs, examples, and release notes when applicable.
- Runtime artifact changes update the artifact reference and schemas when their shape changes.
- Historical implementation logs belong in `docs/archive`; internal unstable design work belongs in `docs/internal` or focused `docs/current/*_plan.md` files.
- `docs/README.md` remains the navigation entry point; avoid adding parallel indexes unless they serve a specific reader path.
