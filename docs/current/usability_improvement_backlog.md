# Usability Improvement Backlog

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

## Recent Completed Batches

- Batch 701: Extended bound header diagnostics to metadata/member keywords such as `index`, `package`, and `version`, and narrowed TextMate `index` highlighting to schema-field positions.
- Batch 702: Added `E-OPTION-BINDING-001` for workflow option assignments such as `unit y = kW` used as bound values, and narrowed root TextMate unit-option highlighting to line-start option positions.
- Batch 703: Tightened native workflow guards so workflow 01/02/03 and their artifacts reject additional Python toolchain markers such as `.pyw`, `poetry`, `pyenv`, `tox`, `nox`, `mypy`, and `ruff`.
- Batch 704: Reworded the Plotting reference so runtime workflow PlotSpec points are described as materialized TimeSeries data, with sample renderer points limited to report-only smoke tests.
- Batch 705: Added VS Code semantic fallback coverage and a package-contract guard for the schema `index` modifier scope so TextMate and semantic-highlight inspection stay aligned.
- Batch 706: Aligned bundled VS Code semantic theme selectors with package fallback mappings by adding readonly/deprecated property and variable selectors plus a package guard against theme-only selectors.
- Batch 707: Clarified VS Code highlight-inspection status text so fallback-scope gaps and direct-selector mapping gaps are both named in the first summary line.
- Batch 708: Added VS Code local quick fixes for statement-only binding diagnostics so bound report, validation, side-effect, block/header, statement, and workflow-option forms can remove the invalid `name =` prefix directly from Problems.
- Batch 709: Added matching eng-lsp code actions and stdio coverage for statement-only binding diagnostics so native/LSP clients can remove invalid binding prefixes without relying on VS Code local fallbacks.
- Batch 710: Added a VS Code package contract guard comparing compiler lexer keywords against generated editor metadata so future keyword additions cannot lose TextMate/IDE catalog coverage silently.
- Batch 711: Renamed workflow 01 public HTTP replay arg from `offline_response_file` to `pinned_response_file` and guarded the native workflow smoke so API examples read as pinned native network/cache boundaries instead of fixture plumbing.
- Batch 712: Archived compact summaries for batches 571-650 from the current usability backlog so the active docs stay focused on recent IDE/workflow work and open cleanup candidates.
- Batch 713: Reworded the VS Code editor metadata public-member API docs and package guard so compiler-owned member catalogs are described as runtime-backed public fields, not editor-only placeholders.
- Batch 714: Aligned VS Code diagnostics output-panel source wording with the Problems source labels, so saved-file and live-buffer checks report `eng/file` and `eng/live` consistently.
- Batch 715: Added the `eng/file` and `eng/live` Problems source labels to VS Code diagnostics mode change messages and Tooling Status summaries so users see the active linter source without opening raw JSON fields.
- Batch 716: Split VS Code first-paint TextMate scopes for system/interface member keywords so `state`, `input`, `output`, `parameter`, `operator`, `port`, `across`, and `through` receive role-specific colors before semantic tokens arrive.
- Batch 717: Added VS Code string escape first-paint coverage so `\\n`, `\\"`, and `\\\\` escape sequences receive `constant.character.escape.englang` theme colors and grammar smoke protection.
- Batch 718: Added VS Code grammar smoke and theme coverage for `punctuation.separator.parameter.englang` so function/method parameter-list separators stay colored on first paint.
- Batch 719: Added a VS Code grammar-smoke guard requiring every non-meta TextMate leaf scope to appear in expected-token coverage, preventing future untested first-paint scopes.
- Batch 720: Removed the duplicated `completion_seed` alias from the editor metadata API, bumped the metadata format to v2, and kept VS Code/package guards on the single `completion_items` catalog.
- Batch 721: Aligned native IDE local member completions with generated DB/model/prediction public field catalogs and dotted-receiver fallback lookup.
- Batch 722: Added Native IDE source-column jumps for source-span buttons and extended docs-check guards for generated public field member completions.
- Batch 723: Preserved checked source lines in `CheckReport` and emitted first-token source-span columns for component graph IDE navigation.
- Batch 724: Made the VS Code review panel source-line buttons column-aware, matching Native IDE `L:C` navigation for `source_span.column` metadata.
- Batch 725: Added `source_span.column` metadata to normalized review-document fallback and risk rows so IDE review panels can jump to precise source columns.
- Batch 726: Archived compact summaries for batches 651-700 from the current usability backlog so the active file stays focused on recent IDE/workflow work.
- Batch 727: Made Native IDE source-column jumps interpret compiler/LSP columns as UTF-8 byte offsets, preserving precise `L:C` selection on non-ASCII source lines.
- Batch 728: Aligned VS Code review-panel source-column conversion with Native IDE byte-offset semantics for consistent non-ASCII source navigation.
- Batch 729: Made VS Code Problems diagnostics consume review `source_span`/source-column metadata and convert UTF-8 byte columns into precise editor ranges.
- Batch 730: Added top-level compiler review diagnostic `source_span` metadata so saved-file VS Code Problems can use precise source columns, not only line ranges.
- Batch 731: Added normalized review-document `source_span` metadata for Symbols and Units rows so IDE review tables can jump to precise source columns.
- Batch 732: Added normalized review-document `source_span` metadata for derived-value and binding-calculation rows, extending precise IDE review-table source jumps.
- Batch 733: Added normalized review-document `source_span` metadata for Inputs rows, including args, schemas, and environment dependency inputs.
- Batch 734: Added normalized review-document `source_span` metadata for Schemas rows so the VS Code review table opens schema declarations at precise columns.
- Batch 735: Added normalized review-document `source_span` metadata for Time Axes rows so review panels jump to TimeSeries declarations precisely.
- Batch 736: Added normalized review-document `source_span` metadata for Caches rows so review panels jump to cached boundary declarations precisely.
- Batch 737: Added normalized review-document `source_span` metadata for External Boundaries rows so boundary review actions open exact declarations.
- Batch 738: Added normalized review-document `source_span` metadata for Side Effects rows across exports, writes, file operations, and network downloads.
- Batch 739: Added normalized review-document `source_span` metadata for Table Transform rows so review-table clicks open transform declarations precisely.
- Batch 740: Added normalized review-document `source_span` metadata for TimeSeries calculation and Report Outputs rows covering summaries, integrations, and plot candidates.
- Batch 741: Added normalized review-document `source_span` metadata for Validation rows covering top-level validate commands and class validations.
- Batch 742: Added normalized review-document `source_span` metadata for remaining Calculation rows: uncertainty, modeling, and system equations.
- Batch 743: Tightened LSP semantic highlighting for process and network with-block options so process policy keys carry side-effect/external modifiers and network cache keys carry cache/external modifiers.
- Batch 744: Clarified workflow 01 docs so `args.pinned_response_file` is the public pinned-response API while `offline_response` remains the language-level HTTP replay option, with a guard against regressing to fixture-style wording.
- Batch 745: Made LSP semantic highlighting propagate network/process owner context onto `cache = true` option values, not only the cache option keys.

## Documentation Policy

- Public behavior changes update user docs, reference docs, examples, and release notes when applicable.
- Runtime artifact changes update the artifact reference and schemas when their shape changes.
- Historical implementation logs belong in `docs/archive`; internal unstable design work belongs in `docs/internal` or focused `docs/current/*_plan.md` files.
- `docs/README.md` remains the navigation entry point; avoid adding parallel indexes unless they serve a specific reader path.
