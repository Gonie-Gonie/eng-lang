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
- Batch 604: Extended the VS Code language word pattern to select the compiler-owned `latin-hypercube` workflow builtin as a whole word while preserving dotted EngLang symbol selection.
- Batch 605: Added a generated TextMate `status = <workflow-status>` pattern and expanded grammar fixtures plus LSP semantic-token snapshots so the full workflow status catalog colors consistently in `status =` and `status ==/!=` contexts.
- Batch 606: Moved the long solver implementation ledgers from `docs/current` into `docs/internal/solver` and updated current-track links so public current docs stay focused on status and reader-facing scope.
- Batch 607: Moved the standalone post-1.0 implementation issue backlog from `docs/current` to `docs/internal` and linked it from the internal documentation index.
- Batch 608: Added a VS Code grammar smoke guard that keeps generated `status = <workflow-status>` option highlighting ahead of generic with-block fallbacks and preserves its property/operator/literal captures.
- Batch 609: Made the VS Code metadata loader consume `completion_items` as the required runtime completion catalog and validate `completion_seed` only as an exact legacy compatibility alias, with package/readme guards to reduce generated API naming confusion.
- Batch 610: Deprecated the Rust `editor_completion_seed()` helper in favor of `editor_completion_items()`, kept an alias regression test, and clarified the static editor metadata reference so `completion_seed` reads as compatibility-only.
- Batch 611: Tightened workflow 01/02/03 native-only guards for `python2`/`python3`, `.ipynb`, and `jupyterlab` markers in CLI smoke and dev workflow checks, with a unit test for notebook/process marker detection.
- Batch 612: Expanded bundled VS Code theme TextMate first-paint colors for EngLang leaf scopes such as imports, deprecated keywords, operators, punctuation, interpolation markers, namespace/type declarations, and role variables, with a grammar smoke guard that keeps both bundled themes covering every non-meta expected scope.
- Batch 613: Reworded public release-note repository-only fixture bullets so implementation-track solver/model fixtures are not described as seeds, and extended docs-check wording guards to keep those stale seed phrases out of public workflow/release docs.
- Batch 614: Extended the VS Code language word pattern so slash/exponent engineering units such as `kg/s`, `J/kg/K`, `W/m^2`, and `people/m2` are selected as whole tokens alongside dotted EngLang symbols.
- Batch 615: Reworded generated sampling completion details from seeded-table language to deterministic sampling behavior and added a VS Code package guard so those seed-only phrases cannot return in public editor metadata.
- Batch 616: Moved TextMate-only unit aliases into generated `syntax_catalog.legacy_unit_aliases` and routed both VS Code grammar and native IDE lexical fallback through that catalog so `%`, byte-size, and compatibility unit colors do not drift.
- Batch 617: Clarified the VS Code README editor-metadata contract so `syntax_catalog.legacy_unit_aliases` is documented as highlight-only compatibility metadata, with a package guard to keep that wording from drifting.
- Batch 618: Moved legacy workflow builtin and option highlight aliases into generated `syntax_catalog.legacy_workflow_builtin_aliases` and `syntax_catalog.legacy_workflow_option_aliases`, then routed VS Code grammar and native IDE lexical fallback through those catalogs without re-adding the aliases to completions.
- Batch 619: Added `api_response.response_source` as the preferred HttpResponse field for live/cached/offline response origin, kept `status` as a compatibility alias, and updated workflow 01 plus editor metadata so HTTP status code/class are no longer mixed with response-origin wording.
- Batch 620: Added a compiler/LSP warning and rename action for `response.status` so users are guided toward `response.response_source` while `status_code` and `status_class` stay reserved for HTTP status semantics.
- Batch 621: Removed `response.status` from generated HTTP response member completion catalogs and package guards while preserving it as a typed compatibility alias with warning/code-action guidance toward `response.response_source`.
- Batch 622: Broadened VS Code `keyword.solver` semantic fallback scopes so solver command words, clause words such as `over`, and solver method literals such as `fixed_step` keep consistent keyword/operator/constant coloring across themes.
- Batch 623: Broadened VS Code `keyword.db` and `keyword.cache` semantic fallback scopes so DB/cache command words, clause words, and status literals keep consistent keyword/operator/constant coloring across themes.
- Batch 624: Added explicit package-contract guards for `keyword.defaultLibrary` semantic fallback scopes so compiler-owned builtin command words keep workflow/operator/validation/report/solver/constant coloring if the VS Code package map drifts.
- Batch 625: Added copy-ready text/range/selector fields to the VS Code cursor highlight inspection payload so users can report or compare the exact semantic token under the caret without manually reconstructing source ranges.
- Batch 626: Added role-aware semantic modifiers and VS Code fallback scopes for `state`, `input`, and `output` declaration keywords so semantic highlighting preserves the same modifier-style coloring as TextMate first paint.
- Batch 627: Hid the compatibility-only `case_inputs.planned_count` alias from generated editor case-output field suggestions while keeping `expected_count` and rendered/blocked/manifest fields visible.
- Batch 628: Hid compatibility-only model option aliases `x`, `y`, `test_fraction`, and `layers` from generated editor workflow-option suggestions while keeping canonical `features`, `target`, `test`, and `hidden` visible and preserving the aliases as highlight-only compatibility scopes.
- Batch 629: Aligned compiler, LSP, and VS Code local quick-fix comment stripping for `#` and `//` comments so legacy helper mentions inside comments or strings no longer produce row-selection warnings or migration actions.
- Batch 630: Shortened generated module completion details by using completion-only status labels such as `Native:` and removing planned-scope tails from native module purpose text, while keeping full registry status labels in docs/review payloads.
- Batch 631: Added a VS Code package-contract guard that keeps generated native module completions on the short `Native:` detail wording and prevents full status labels or planned-scope tails from returning.
- Batch 632: Routed native IDE module status helpers through compiler-owned `status_label`/`status_detail` fields and shortened VS Code/native fallback-only `native_preview` labels to `Native` so UI fallback wording does not drift from completion wording.
- Batch 633: Removed compatibility-only model option aliases from VS Code and LSP model quick-fix candidate tables so repairs keep using canonical `test` and `hidden` option names even if legacy aliases remain highlight-only metadata.
- Batch 634: Added the missing VS Code local rename action for `response.status` diagnostics so fallback quick fixes now match LSP guidance and rewrite the compatibility alias to `response.response_source`.
- Batch 635: Fixed VS Code TextMate `with { ... }` fallback ordering so option-block constants/operators keep their expected colors before generic local identifiers, including nested option maps.
- Batch 636: Extended `workflows-test` so workflow 01/02/03 runtime artifacts fail if Python/notebook markers or `run command` wording reappear after native execution, not just in source/docs and process counts.
- Batch 637: Reworded VS Code and native IDE network boundary tables to show `response_source` as response source instead of generic status, while preserving compatibility status fields in artifacts.
- Batch 638: Added operator-token fallback includes to VS Code workflow phrase scopes for print/log, standard text writes, run command, summarize, and HTTP request lines so operators color consistently across native workflow statements.
- Batch 639: Reworded current/internal docs that still described implemented tracks as implementation seeds, and added a docs-check guard so those stale implementation-seed phrases do not return outside archives.
- Batch 640: Renamed the compiler internal net request/download response-origin field from `status` to `response_source` while keeping emitted artifact `status` as a compatibility alias.
- Batch 641: Kept the VS Code last-run artifact picker in registry workflow order instead of resorting available artifacts alphabetically, so report, review, result, and run-graph artifacts stay in predictable positions.
- Batch 642: Reordered native IDE side tabs around the review workflow and added a dedicated Units tab that surfaces review unit records and unit conversions before Schema.
- Batch 643: Suppressed empty display-unit labels in LSP and VS Code hover markdown so string/API fields no longer render as blank unit metadata.
- Batch 644: Added native SQLite connection summary bindings such as `db.tables_written`, `db.table_count`, `db.row_count`, and `db.status`, then replaced workflow 02's hardcoded DB table summary with actual DB write/readback metadata.
- Batch 645: Propagated semantic token modifiers from typed workflow receivers onto their public member fields, so DB/sample/case/HTTP fields color consistently instead of falling back to generic property coloring.
- Batch 646: Added `db.summary` as the preferred native SQLite connection summary binding, keeping detailed table/count/row/status fields as drill-down APIs.
- Batch 647: Cleaned the public native surrogate workflow guide so it names `db.summary` alongside DB manifests instead of documenting only the lower-level DB write artifacts.
- Batch 648: Added TextMate first-paint coloring for compiler-catalog public workflow member fields, using generated HTTP/sample/DB/case field metadata instead of another hardcoded member list.
- Batch 649: Extended TextMate public workflow member coloring through dotted receivers such as `study.designs.row_preview` and `args.designs.row_preview`, so member fields do not fall back to generic property coloring when the table/model source is nested.
- Batch 650: Labeled VS Code Problems diagnostics by source as `eng/file` or `eng/live`, making saved-file and live-buffer linter results distinguishable without opening the EngLang output panel.
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

## Documentation Policy

- Public behavior changes update user docs, reference docs, examples, and release notes when applicable.
- Runtime artifact changes update the artifact reference and schemas when their shape changes.
- Historical implementation logs belong in `docs/archive`; internal unstable design work belongs in `docs/internal` or focused `docs/current/*_plan.md` files.
- `docs/README.md` remains the navigation entry point; avoid adding parallel indexes unless they serve a specific reader path.
