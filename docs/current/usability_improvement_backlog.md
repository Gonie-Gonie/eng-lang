# Usability Improvement Backlog

- Batch 781: Added `dev.bat vscode-status` install freshness reporting so local VS Code users can see whether the built VSIX is newer than the installed EngLang extension before closing editor windows.
- Batch 780: Added a Native IDE per-token Copy Selector action in the Highlight panel so users can report or compare exact semantic selector mappings without copying the full highlight table.
- Batch 779: Expanded Native IDE lexical completion fallback coloring for property, method, variable, constant/value, operator, and module completion kinds so options and public fields stay colored before semantic tokens arrive.
- Batch 778: Split Native IDE semantic highlight colors for axis vs TimeSeries, validation vs report, and side-effect vs external roles, with a guard against regrouping those role colors.
- Batch 777: Clarified network replay wording so workflow args use pinned response files, language syntax uses `offline_response`, and legacy `fixture` is described only as a migration alias.
- Batch 776: Archived current backlog batches 701-745 into the historical log so active docs stay focused on recent workflow/API/IDE cleanup.
- Batch 775: Added direct VS Code `method.cache` selector coverage and tightened cache semantic fallback/theme guards so cache helpers, keys, option values, and records stay consistently colored.
- Batch 774: Replaced Workflow 01's token-only native API/cache smoke with structured JSON assertions for HTTP query metadata, response hashes, cache records, output-manifest network entries, and run-log events.
- Batch 773: Promoted `solver_plan.jacobian_sparsity` to the required artifact-schema field, kept `jacobian_seed` as an optional compatibility alias, and taught artifact golden checks to validate both names separately.
- Batch 772: Aligned LSP semantic highlighting with the generated VS Code syntax catalog, added `index` as a schema modifier completion, and guarded keyword/operator/constant catalog labels against semantic-token gaps.
- Batch 771: Expanded VS Code TextMate grammar smoke coverage so generated keyword, operator, and constant catalogs must have exact scoped token expectations, reducing first-paint highlighting gaps.
- Batch 770: Extended the preferred `jacobian_sparsity` solver-plan name into compiler/report Rust structs while retaining `jacobian_seed` as a compatibility field.
- Batch 769: Added `solver_plan.jacobian_sparsity` as the preferred solver-plan artifact field while keeping `jacobian_seed` as a compatibility alias, and reworded component residual source reasons away from seed-only terminology.
- Batch 768: Added LSP and VS Code local quick fixes for simple `E-CMD-UNKNOWN-VERB` diagnostics, converting clause-bearing command-style calls such as `median Q_plot over Time` into `median(Q_plot, over=Time)`.
- Batch 767: Exposed the legacy `select_first_row` compatibility warning through `eng.table` module metadata and public syntax diagnostics docs.
- Batch 766: Exposed write/export diagnostics through `eng.io` module metadata and filled the missing public syntax/CLI wording for those codes.
- Batch 765: Exposed the real file-operation syntax diagnostics `E-FS-001/002/003` through the `eng.fs` module registry, workflow-module table, and public CLI/syntax diagnostics docs.
- Batch 764: Added CLI reference coverage for every module-registry diagnostic and a docs-check guard so future registry diagnostics cannot lack public CLI wording.
- Batch 763: Removed placeholder/status words from module-registry diagnostic lists and added registry/docs-check guards so diagnostics must be real `E-`/`W-` codes.
- Batch 762: Added `E-GOLDEN-001` LSP/VS Code quick fixes and exposed the full `eng.test` assertion/golden diagnostic set in registry and public diagnostics docs.
- Batch 761: Added `E-PROCESS-BINDING-002` to the `eng.process` module registry and generated workflow-module table so process binding collision diagnostics are visible in docs and IDE module metadata.
- Batch 760: Exposed `E-LOG-LEVEL-001` through the `eng.log` module registry, current workflow-module table, and CLI diagnostics reference instead of hiding it as `none_current`.
- Batch 759: Added stdio, VS Code contract, and module-registry coverage for `W-NET-RESPONSE-STATUS-ALIAS` so `response.status` consistently quick-fixes to `response.response_source`.
- Batch 758: Added LSP and VS Code local quick fixes for `E-SAMPLING-RANGE-UNIT` when one `uniform(lower, upper)` endpoint is missing the other endpoint's unit.
- Batch 757: Added LSP and VS Code local quick fixes for `E-ML-SOURCE-001/002`, inserting native ML source-chain skeletons or split adapters when model workflows reference missing or wrong-type sources.
- Batch 756: Tightened `dev.bat workflows-test` so workflow 02 native LHS sample tables must expose generated row hash previews and row value previews with per-parameter numeric payloads.
- Batch 755: Added VS Code TextMate first-paint unit highlighting inside function parameter and return type annotations such as `Conductance [W/K]` and `HeatRate [W]`.
- Batch 754: Extended VS Code TextMate unit highlighting from ASCII-only labels to all compiler-owned unit labels, including degree-C aliases, with fixture/golden coverage.
- Batch 753: Restored the Celsius `degree-C` alias text in compiler tests and public syntax docs, and added a docs-check guard against recurring Celsius mojibake.
- Batch 752: Added LSP and VS Code local quick fixes for `E-WRITE-002`, replacing unsupported `write <format>` tokens with `text`, `json`, or `standard_text` without touching export-to-CSV syntax.
- Batch 751: Added LSP and VS Code local quick fixes for `E-WRITE-STANDARD-TEXT-001`, changing scalar `write standard_text` statements to `write text` when the writer target is not a typed table.
- Batch 750: Added LSP and VS Code local quick fixes for `E-WRITE-STANDARD-TEXT-OUTPUT`, inserting the native `output = join(args.output, "standard_weather_file.txt")` option when `write standard_text` lacks an output path.
- Batch 749: Expanded `dev.bat vscode-status` with built-VSIX version, size, update time, and installed extension package version summaries.
- Batch 748: Added `dev.bat vscode-status` so local VS Code extension install/package readiness can be checked without triggering a reinstall or failing on open VS Code windows.
- Batch 747: Added LSP and VS Code local quick fixes for `E-SOLVE-SOLVER-UNSUPPORTED`, using `solver = fixed_point` so solve-block solver diagnostics are actionable like simulation solver diagnostics.
- Batch 746: Added LSP and VS Code local quick fixes for `E-NET-BODY-POLICY` so unsupported secret request bodies can be replaced with an explicit string-literal body from either editor backend.

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
