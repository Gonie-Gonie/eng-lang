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

- Batch 651: Applied the same `eng/file` and `eng/live` Problems source labels to fallback diagnostics emitted when editor JSON is unavailable.
- Batch 652: Added VS Code package-contract guards so file/live Problems source labels and fallback diagnostic source labels cannot silently drift out of `diagnosticsProvider.js`.
- Batch 653: Extended VS Code local member completions through dotted receivers, including generated field-map lookup by the terminal receiver segment, so nested sample/schema/workflow member APIs complete consistently with TextMate coloring.
- Batch 654: Extended LSP/native IDE member completion parsing through dotted receivers and terminal binding fallback, with sample-table coverage for nested receiver paths.
- Batch 655: Tightened LSP semantic-token coverage so nested sample-table member paths prove the specific dotted-receiver line receives workflow-step property highlighting.
- Batch 656: Added exact LSP hover metadata for compiler-catalog public workflow member accesses, including nested paths such as `study.samples.row_preview`, so hover does not fall back to unrelated same-label bindings.
- Batch 657: Aligned VS Code fallback and LSP option-value quick fixes to strip full EngLang line comments before computing replacement ranges, preserving trailing `#` and `//` notes while still allowing comment markers inside strings.
- Batch 658: Made `vscode-install` run the VS Code CLI with an ignored temporary user-data directory while explicitly targeting the normal user extension directory, reducing local install failures from VS Code AppData log permissions and keeping reinstall guidance accurate.
- Batch 659: Aligned raw `read json`/`read toml`/`read text` highlighting so TextMate first paint and LSP semantic tokens mark the phrase keywords as external boundaries while preserving workflow-step role metadata.
- Batch 660: Added LSP external-boundary semantic tokens for `promote csv/json/toml` source operands, including file helpers and dotted raw-source bindings, so typed promotions show where external workflow data enters the schema layer.
- Batch 661: Reworked TextMate `promote json records` highlighting to use phrase patterns with member-aware source-path fallbacks, and guarded all `promote csv/json/toml` phrase scopes so dotted sources stay colorful before LSP semantic tokens arrive.
- Batch 662: Improved `vscode-install` preflight guidance so an open VS Code window still reports the existing built VSIX path when available, making local linting/highlighting reinstall recovery less ambiguous.
- Batch 663: Reworked TimeSeries quality TextMate phrase scopes for `check coverage`, `fill missing`, `align`, and `resample` to use member-aware source-path fallbacks so dotted series operands stay colorful before semantic tokens arrive.
- Batch 664: Added member-aware TextMate fallbacks for command-style `integrate <series> over <axis>` and `mean/max/min <series> over <axis>` phrases while keeping call-style integrate/stat patterns separate.
- Batch 665: Reworked TextMate `plot distribution(...)` and `plot <series> over <axis>` phrase scopes to use member-aware report operand fallbacks, with grammar guards so dotted report paths stay colorful before LSP semantic tokens arrive.
- Batch 666: Extended the TextMate `plot` command fallback for multi-series and named plot functions with member-aware report operand fallbacks, guarded so dotted report paths do not collapse into broad property scopes.
- Batch 667: Reworked TextMate `summarize <series> by ...` and `show <value>` report phrase scopes to use member-aware fallbacks, so dotted report operands split into receiver/member scopes before semantic tokens arrive.
- Batch 668: Added member-aware TextMate fallbacks to I/O and external-boundary workflow phrases including `read`, `write`, `export`, `download`, `http`, `render template`, and file operations so dotted source/target operands stay split before semantic highlighting.
- Batch 669: Reworked DB/table TextMate phrase scopes for `open sqlite`, `read sqlite <db>.table(...)`, `write <table> to <db>.table(...)`, and `select <table> columns ...` so nested receivers use member-aware first-paint highlighting.
- Batch 670: Added member-aware TextMate fallbacks to model call phrase scopes including `train_test_split(...)`, `regression_table(...)`, `evaluate(...)`, and `model_card(...)`, keeping nested model/table operands split before semantic tokens arrive.
- Batch 671: Added member-aware TextMate fallbacks to uncertainty distribution calls such as `measured(...)`, `interval(...)`, `propagate(...)`, and `probability(...)`, completing the broad workflow fallback gap scan.
- Batch 672: Reworked table and case operation TextMate phrase scopes such as `filter`, `derive`, `sort`, `join`, `require_one`, `materialize cases`, and `collect results` so dotted operands use member-aware first-paint highlighting.
- Batch 673: Reworked `predict`, `train regression`, and `apply` TextMate phrase scopes so dotted model/table/step/case operands use member-aware first-paint highlighting while local workflow-step coloring is preserved.
- Batch 674: Reworked `integrate(...)`, statistical axis calls, and summary export field TextMate scopes so dotted series, axis, and summary operands use member-aware first-paint highlighting.
- Batch 675: Added a TextMate grammar regression guard requiring workflow property fallback scopes to include `#members`, with only fixed `status` key/literal scopes allowed as exceptions.
- Batch 676: Aligned VS Code local args/schema field completions with quick-fix parsing so `#` and `//` inside strings are not mistaken for comments.
- Batch 677: Added executable model and prediction public member fields such as `model.rmse` and `predictions.output_column` across compiler typing, runtime interpolation, LSP hover/completion/semantic tokens, generated VS Code metadata, and local completion fallback.
- Batch 678: Documented compiler-owned model and prediction public member catalogs in LSP/VS Code references and added a package guard so executable public member fields are not described as seed-only editor suggestions.
- Batch 679: Made TextMate `print` and `log` workflow phrase scopes member-aware so public fields such as `model.rmse` and `predictions.output_column` stay colorful in runtime message statements before semantic tokens arrive.
- Batch 680: Made TextMate `run command` phrase scopes member-aware so external-boundary command operands such as `args.simulator` split into parameter/member tokens before semantic highlighting arrives.
- Batch 681: Added a TextMate grammar guard requiring begin/end workflow phrase scopes to include `#members`, preventing print/log/run-style operand highlighting gaps from returning.
- Batch 682: Improved VS Code unavailable-diagnostics UX so Problems entries include a short `Tool failure:` reason when the selected checker exits without editor JSON, while detailed stderr/stdout stays in the EngLang output channel.
- Batch 683: Added a VS Code command registration guard so every package-exposed command has an `extension.js` handler and the only registered-only command is the legacy `englang.switchProblemsSource` compatibility alias.
- Batch 684: Added the Problems source label (`eng/file` or `eng/live`) to the VS Code tooling status payload so diagnostics mode and Problems-column source wording stay aligned.
- Batch 685: Standardized VS Code tooling status wording on `live editor checks` instead of `live editor requests`, keeping live Problems, hover, completion, and highlight wording aligned.
- Batch 686: Implemented `resample <series> by <duration>` as a native TimeSeries resampling hook, with runtime artifact coverage and VS Code TextMate first-paint highlighting aligned to the new step-only form.
- Batch 687: Aligned LSP semantic highlighting for `resample <series> by <duration>` so the `by` clause keeps workflowStep/timeseries/validation role coloring after semantic overlay, while duration units are not treated as workflow identifiers.
- Batch 688: Tightened LSP semantic highlighting for supported report `summarize <series> by [...]` statements so the source series keeps report/timeseries role coloring and statistics such as `duration_above(5 kW)` color only the statistic function, not the unit argument.
- Batch 689: Stopped VS Code TextMate report phrase scopes from highlighting unsupported bound report commands such as `arg_summary = summarize ...`, added compiler diagnostic `E-REPORT-BINDING-001`, and guarded grammar tests against those seed-only RHS forms.
- Batch 690: Replaced the report language reference placeholder example (`show summary`, `plot heat over Time`) with concrete supported `Q_coil` bindings and added a docs-check guard so abstract report examples do not return.
- Batch 691: Reworded the VS Code editor-metadata README so compiler-owned public member catalogs promise executable API fields, not `seed-only suggestions`, and added a package guard against that wording returning.
- Batch 692: Added `E-VALIDATE-BINDING-001` so `bad = validate ...` no longer falls through to unknown-function or ambiguous-quantity diagnostics when validation is used as a bound value.
- Batch 693: Anchored VS Code TextMate validation phrase scopes so `bad_validate = validate ...` no longer receives first-paint validation highlighting before the compiler reports `E-VALIDATE-BINDING-001`.
- Batch 694: Extended `E-VALIDATE-BINDING-001` to the full validation statement family so bound `assert` and `golden` forms no longer pass or degrade into unrelated warnings.
- Batch 695: Added `E-SIDE-EFFECT-BINDING-001` for statement-only side effects used as bound values and anchored matching VS Code TextMate scopes to top-level side-effect statements.
- Batch 696: Extended bound statement diagnostics and TextMate anchoring to `print`, `log`, and `report { ... }` so output/report blocks cannot masquerade as bound values.
- Batch 697: Added `E-BLOCK-BINDING-001` for declaration/block headers used as bound values and anchored declaration plus validation-block TextMate scopes to statement starts.
- Batch 698: Added `E-STATEMENT-BINDING-001` for `return`, `use`/`import`, and `connect` used as bound values, plus a TextMate guard for bound `return` statements.
- Batch 699: Extended `E-BLOCK-BINDING-001` and TextMate anchoring to block-member declarations such as `state`, `input`, `equation`, `port`, and `conservation` when they are incorrectly used as bound values.
- Batch 700: Anchored the dedicated TextMate `args { ... }` block scope to statement starts so bound `bad_args = args {` no longer receives args-block first-paint highlighting.
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

## Documentation Policy

- Public behavior changes update user docs, reference docs, examples, and release notes when applicable.
- Runtime artifact changes update the artifact reference and schemas when their shape changes.
- Historical implementation logs belong in `docs/archive`; internal unstable design work belongs in `docs/internal` or focused `docs/current/*_plan.md` files.
- `docs/README.md` remains the navigation entry point; avoid adding parallel indexes unless they serve a specific reader path.
